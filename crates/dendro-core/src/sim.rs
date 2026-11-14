//! The simulation driver: daily climate, water, phenology and productivity;
//! structural events a few times per year.
//!
//! Yearly event sequence (driven by phenology):
//! 1. **Budburst** -> spring flush: shedding, vigour allocation, bud break,
//!    new annual shoots created unextended; they extend over the following
//!    weeks (geometry updated in 5 steps).
//! 2. **Leaf expansion complete** -> light pass on the new canopy, earlywood
//!    (from last year's reserves), root growth.
//! 3. **Senescence onset** -> latewood (from this summer's productivity),
//!    ring recorded.
//! 4. **Leaf fall complete** -> dormancy (young oaks keep some dead leaves).

use crate::climate::{Climate, ClimateParams, SoilWater, Weather};
use crate::genome::Genome;
use crate::growth::ShootSystem;
use crate::mesh::{self, LeafCluster, MeshData, TubeSeg};
use crate::phenology::{self, PhenoEvent, Phase, Phenology, Season};
use crate::rings::{self, Ring};
use crate::rng::{self, S_LEAF};
use crate::roots::RootSystem;

/// Days per simulated year (no leap years).
pub const DAYS_PER_YEAR: u32 = 365;

/// Triangle budget of the wood (trunk + branches) mesh.
pub const WOOD_TRI_BUDGET: usize = 210_000;
/// Triangle budget of the root mesh.
pub const ROOT_TRI_BUDGET: usize = 80_000;
/// Triangle budget of the leaf mesh.
pub const LEAF_TRI_BUDGET: usize = 380_000;

/// Productivity of an average year (sum of daily index), used to normalise.
const P_YEAR_REF: f32 = 100.0;
/// Productivity of an average summer (expansion complete -> senescence).
const P_SUMMER_REF: f32 = 70.0;

/// Snapshot of the main tree dimensions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TreeStats {
    pub age_years: f32,
    pub height_m: f32,
    /// Diameter at breast height (1.3 m), cm; 0 while shorter than 1.3 m.
    pub dbh_cm: f32,
    pub crown_radius_m: f32,
    pub root_depth_m: f32,
    pub root_spread_m: f32,
    pub shoot_nodes: usize,
    pub leafy_shoots: usize,
    pub root_nodes: usize,
}

/// A growing tree in its environment.
#[derive(Clone, Debug)]
pub struct Simulation {
    genome: Genome,
    seed: u64,
    climate: Climate,
    soil: SoilWater,
    phen: Phenology,
    shoots: ShootSystem,
    roots: RootSystem,
    rings: Vec<Ring>,
    day: u64,
    drought: bool,
    drought_in_season: bool,
    today: Weather,
    water_factor: f32,
    p_year: f32,
    p_summer: f32,
    p_prev: f32,
    in_summer: bool,
    stress_sum: f32,
    stress_days: f32,
    pending_early_mm: f32,
    pending_measure: f32,
    marcescent: f32,
    wood_version: u64,
    leaf_version: u64,
    root_version: u64,
    reported_version: u64,
    leaf_key: u64,
}

impl Simulation {
    /// New seedling (acorn about to germinate) on Jan 1 of year 0, with the
    /// default temperate-oceanic climate.
    pub fn new(genome: Genome, seed: u64) -> Self {
        Self::with_climate(genome, seed, ClimateParams::default())
    }

    /// New seedling with custom climate parameters.
    pub fn with_climate(genome: Genome, seed: u64, climate: ClimateParams) -> Self {
        let chill = genome.phenology.chilling_requirement_days * 0.5;
        let shoots = ShootSystem::new(&genome);
        Simulation {
            climate: Climate::new(climate, seed),
            soil: SoilWater::new(60.0),
            phen: Phenology::new().with_initial_chill(chill),
            shoots,
            roots: RootSystem::new(),
            rings: Vec::new(),
            day: 0,
            drought: false,
            drought_in_season: false,
            today: Weather::default(),
            water_factor: 1.0,
            p_year: 0.0,
            p_summer: 0.0,
            p_prev: 1.0,
            in_summer: false,
            stress_sum: 0.0,
            stress_days: 0.0,
            pending_early_mm: 0.0,
            pending_measure: 0.0,
            marcescent: 0.0,
            wood_version: 1,
            leaf_version: 1,
            root_version: 1,
            reported_version: 0,
            leaf_key: 0,
            genome,
            seed,
        }
    }

    /// Advance the simulation by `n` days. Returns `true` if any mesh
    /// (wood, leaves or roots) changed since the previous call; use the
    /// `*_version` counters to know which one.
    pub fn advance_days(&mut self, n: u32) -> bool {
        for _ in 0..n {
            self.step_day();
        }
        let v = self.wood_version + self.leaf_version + self.root_version;
        let changed = v != self.reported_version;
        self.reported_version = v;
        changed
    }

    /// Advance whole years (convenience).
    pub fn advance_years(&mut self, years: u32) -> bool {
        self.advance_days(years * DAYS_PER_YEAR)
    }

    /// Force (or end) a drought: no rain and +40 % evaporative demand while
    /// active.
    pub fn set_drought(&mut self, on: bool) {
        self.drought = on;
    }

    pub fn drought(&self) -> bool {
        self.drought
    }

    fn step_day(&mut self) {
        let year = (self.day / DAYS_PER_YEAR as u64) as u32;
        let doy = (self.day % DAYS_PER_YEAR as u64) as u32;
        if doy == 0 && self.day > 0 {
            self.p_prev = (self.p_year / P_YEAR_REF).clamp(0.2, 1.6);
            self.p_year = 0.0;
        }
        let w = self.climate.day(year, doy, self.drought);
        self.today = w;

        // Water balance.
        let leaf = self.phen.leaf_area();
        let demand = w.pet_mm * (0.3 + 0.7 * leaf);
        self.soil.step(w.precip_mm, demand);
        let thr = 0.5 - 0.2 * self.genome.tolerances.drought; // REW of stomatal closure onset
        self.water_factor = (self.soil.rew() / thr).min(1.0);

        // Phenology and structural events.
        match self.phen.step(&w, doy, &self.genome) {
            PhenoEvent::Budburst => self.on_budburst(year),
            PhenoEvent::ExpansionComplete => self.on_expansion_complete(year),
            PhenoEvent::SenescenceStart => self.on_senescence(year),
            PhenoEvent::LeafFallComplete => self.on_leaf_fall(),
            PhenoEvent::None => {}
        }
        // Shoot extension in 5 visible steps.
        let ext_q = (self.phen.shoot_ext * 5.0).floor() / 5.0;
        if ext_q > self.shoots.ext {
            self.shoots.ext = ext_q;
            self.wood_version += 1;
        }

        // Daily productivity index (light x temperature x water).
        let t = w.temp_c;
        let f_t = ((t - 4.0) / 14.0).clamp(0.0, 1.0) * ((38.0 - t) / 8.0).clamp(0.0, 1.0);
        let p_day = leaf * f_t * self.water_factor * (w.photoperiod_h / 14.0);
        self.p_year += p_day;
        if self.in_summer {
            self.p_summer += p_day;
        }
        if leaf > 0.3 {
            self.stress_sum += 1.0 - self.water_factor;
            self.stress_days += 1.0;
            if self.drought {
                self.drought_in_season = true;
            }
        }

        // Leaf appearance key (quantised so the leaf mesh changes every few days).
        let key = (self.phen.phase as u64)
            | ((self.phen.expansion * 8.0).round() as u64) << 4
            | ((self.phen.colouring * 12.0).round() as u64) << 8
            | ((self.phen.fallen * 10.0).round() as u64) << 12
            | ((self.phen.frost_damage * 4.0).round() as u64) << 16
            | (self.wood_version & 0xFFFF_FFFF) << 24;
        if key != self.leaf_key {
            self.leaf_key = key;
            self.leaf_version += 1;
        }
        self.day += 1;
    }

    fn on_budburst(&mut self, year: u32) {
        self.marcescent = 0.0;
        let vigour = (0.35 + 0.65 * self.p_prev).clamp(0.3, 1.3);
        let age = self.age_years();
        self.shoots.flush(&self.genome, year, age, vigour, self.seed);
        self.stress_sum = 0.0;
        self.stress_days = 0.0;
        self.drought_in_season = false;
        self.wood_version += 1;
    }

    fn measure_node(&self) -> (usize, f32) {
        match self.shoots.trunk_node_at(1.3) {
            Some(i) if self.shoots.height() > 1.3 => (i, 1.3),
            _ => (0, 0.0),
        }
    }

    fn on_expansion_complete(&mut self, year: u32) {
        self.in_summer = true;
        self.p_summer = 0.0;
        self.shoots.ext = 1.0;
        self.shoots.compute_light();
        self.shoots.compute_foliage(self.genome.architecture.hydraulic_path_max_m);
        // Earlywood from reserves (last year's productivity).
        let w = &self.genome.wood;
        let share = if w.ring_porous { w.earlywood_share } else { w.earlywood_share * 0.6 };
        let factor = share * self.p_prev.clamp(0.4, 1.3);
        let (mi, mh) = self.measure_node();
        let a0 = self.shoots.nodes[mi].area;
        let collar = self.shoots.thicken(w.growth_coefficient, factor, w.max_ring_width_mm * 0.001 * share * 1.5);
        self.pending_early_mm = rings::width_mm(a0, self.shoots.nodes[mi].area);
        self.pending_measure = mh;
        self.roots.thicken(collar);
        // Root growth.
        let cr = self.shoots.crown_radius();
        let age = self.age_years();
        self.roots.grow(&self.genome.roots, year, age, cr, self.p_prev, self.seed);
        let depth = self.roots.depth();
        self.soil.set_capacity(60.0 + 120.0 * (depth / 1.5).min(1.0));
        self.wood_version += 1;
        self.root_version += 1;
    }

    fn on_senescence(&mut self, year: u32) {
        self.in_summer = false;
        let w = &self.genome.wood;
        let share = if w.ring_porous { w.earlywood_share } else { w.earlywood_share * 0.6 };
        let p_summer = (self.p_summer / P_SUMMER_REF).clamp(0.03, 1.5);
        let factor = (1.0 - share) * p_summer;
        let (mi, _) = self.measure_node();
        let a0 = self.shoots.nodes[mi].area;
        let collar = self.shoots.thicken(w.growth_coefficient, factor, w.max_ring_width_mm * 0.001 * (1.0 - share) * 1.5);
        let late = rings::width_mm(a0, self.shoots.nodes[mi].area);
        self.roots.thicken(collar);
        let early = self.pending_early_mm;
        let total = early + late;
        self.rings.push(Ring {
            year,
            width_mm: total,
            earlywood_mm: early,
            latewood_mm: late,
            earlywood_fraction: if total > 0.0 { early / total } else { 0.0 },
            water_stress: if self.stress_days > 0.0 { self.stress_sum / self.stress_days } else { 0.0 },
            forced_drought: self.drought_in_season,
            measured_at_m: self.pending_measure,
        });
        self.wood_version += 1;
        self.root_version += 1;
    }

    fn on_leaf_fall(&mut self) {
        let l = &self.genome.leaf;
        let juvenile = (1.0 - self.age_years() / l.marcescence_max_age_years.max(1.0)).clamp(0.0, 1.0);
        self.marcescent = if l.deciduous { l.marcescence * juvenile } else { 0.0 };
    }

    // ------------------------------------------------------------------
    // Queries
    // ------------------------------------------------------------------

    pub fn genome(&self) -> &Genome {
        &self.genome
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Days simulated since the start (Jan 1 of year 0).
    pub fn days_elapsed(&self) -> u64 {
        self.day
    }

    /// Age in years (fractional), counted from Jan 1 of the germination year.
    pub fn age_years(&self) -> f32 {
        self.day as f32 / DAYS_PER_YEAR as f32
    }

    /// Current day of year, 0..365 (0 = Jan 1). This is the next day to be
    /// simulated.
    pub fn day_of_year(&self) -> u32 {
        (self.day % DAYS_PER_YEAR as u64) as u32
    }

    /// Calendar year index (0 = first year).
    pub fn year(&self) -> u32 {
        (self.day / DAYS_PER_YEAR as u64) as u32
    }

    pub fn phase(&self) -> Phase {
        self.phen.phase
    }

    pub fn season(&self) -> Season {
        Season::from_doy(self.day_of_year())
    }

    /// Tree height (m).
    pub fn height_m(&self) -> f32 {
        self.shoots.height()
    }

    /// Stem diameter at 1.3 m (cm), 0 if the tree is shorter.
    pub fn dbh_cm(&self) -> f32 {
        if self.shoots.height() <= 1.3 {
            return 0.0;
        }
        match self.shoots.trunk_node_at(1.3) {
            Some(i) => self.shoots.nodes[i].radius() * 200.0,
            None => 0.0,
        }
    }

    /// Stem diameter at the collar (cm).
    pub fn collar_diameter_cm(&self) -> f32 {
        self.shoots.nodes[0].radius() * 200.0
    }

    /// Mean crown radius (m).
    pub fn crown_radius_m(&self) -> f32 {
        self.shoots.crown_radius()
    }

    /// Maximum rooting depth (m, positive).
    pub fn root_depth_m(&self) -> f32 {
        self.roots.depth()
    }

    /// Maximum horizontal root spread from the stem (m).
    pub fn root_spread_m(&self) -> f32 {
        self.roots.spread()
    }

    /// Mean leaf colour of the canopy today (RGB 0..1).
    pub fn leaf_color(&self) -> [f32; 3] {
        if self.phen.phase == Phase::Dormant {
            return phenology::MARCESCENT_BROWN;
        }
        phenology::leaf_colour(&self.genome.pigments, self.phen.expansion, self.phen.colouring)
    }

    /// Leaf density factor 0..1 (visible foliage relative to full summer
    /// canopy; dead marcescent leaves count half).
    pub fn leaf_density(&self) -> f32 {
        match self.phen.phase {
            Phase::Dormant => self.marcescent * 0.5,
            _ => self.phen.leaf_area(),
        }
    }

    /// Relative extractable soil water 0..1.
    pub fn soil_water(&self) -> f32 {
        self.soil.rew()
    }

    /// Weather of the last simulated day.
    pub fn weather(&self) -> Weather {
        self.today
    }

    /// Daily mean temperature of the last simulated day (degC).
    pub fn temperature_c(&self) -> f32 {
        self.today.temp_c
    }

    /// Completed annual rings, oldest first.
    pub fn rings(&self) -> &[Ring] {
        &self.rings
    }

    /// Version counters, bumped whenever the corresponding mesh would change.
    pub fn wood_version(&self) -> u64 {
        self.wood_version
    }
    pub fn leaf_version(&self) -> u64 {
        self.leaf_version
    }
    pub fn root_version(&self) -> u64 {
        self.root_version
    }

    pub fn stats(&self) -> TreeStats {
        TreeStats {
            age_years: self.age_years(),
            height_m: self.height_m(),
            dbh_cm: self.dbh_cm(),
            crown_radius_m: self.crown_radius_m(),
            root_depth_m: self.root_depth_m(),
            root_spread_m: self.root_spread_m(),
            shoot_nodes: self.shoots.nodes.len(),
            leafy_shoots: self.shoots.nodes.iter().filter(|n| n.leafy).count(),
            root_nodes: self.roots.nodes.len(),
        }
    }

    // ------------------------------------------------------------------
    // Meshes
    // ------------------------------------------------------------------

    /// Trunk and branches. See [`crate::mesh`] for conventions.
    pub fn wood_mesh(&self) -> MeshData {
        let year = self.year();
        let ext = self.shoots.ext;
        let segs: Vec<TubeSeg> = self
            .shoots
            .nodes
            .iter()
            .map(|n| {
                let age = year.saturating_sub(n.birth_year) as f32;
                let r = n.radius();
                // Young shoots green-brown, then grey-brown bark, darker on old trunks.
                let t = (age / 4.0).min(1.0);
                let dark = (r / 0.4).min(1.0);
                let col = [
                    0.40 + 0.0 * t - 0.10 * dark,
                    0.38 - 0.06 * t - 0.08 * dark,
                    0.22 + 0.06 * t - 0.06 * dark,
                    1.0,
                ];
                TubeSeg {
                    parent: n.parent,
                    start: n.start,
                    end: n.end(ext),
                    radius: r,
                    continues: n.terminal,
                    color: col,
                    tip_taper: 0.5,
                }
            })
            .collect();
        mesh::build_tubes(&segs, WOOD_TRI_BUDGET, 0.0008)
    }

    /// Root system (below y = 0).
    pub fn root_mesh(&self) -> MeshData {
        // Keep thick shallow roots under the surface: sink each tube centre
        // to at least ~its radius below y = 0 (except the collar ring).
        let sink = |p: crate::math::Vec3, r: f32| crate::math::Vec3::new(p.x, p.y.min(-0.9 * r), p.z);
        let segs: Vec<TubeSeg> = self
            .roots
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| TubeSeg {
                parent: n.parent,
                start: if i == 0 { n.start } else { sink(n.start, n.radius()) },
                end: sink(n.end(), n.radius()),
                radius: n.radius(),
                continues: n.continues,
                color: [0.47, 0.37, 0.27, 1.0],
                tip_taper: 0.3,
            })
            .collect();
        mesh::build_tubes(&segs, ROOT_TRI_BUDGET, 0.0008)
    }

    /// Leaf clusters with seasonal colour; empty in winter except for
    /// marcescent (dead, retained) leaves on young trees.
    pub fn leaf_mesh(&self) -> MeshData {
        let phase = self.phen.phase;
        let dormant = phase == Phase::Dormant;
        if dormant && self.marcescent <= 0.0 {
            return MeshData::default();
        }
        let g = &self.genome;
        let ext = self.shoots.ext;
        let height = self.shoots.height().max(0.1);
        let leaf_len = g.leaf.length_cm * 0.01;
        let base_size = (leaf_len * 2.2).max(self.shoots.voxel * 1.05);
        let size_factor = if dormant {
            0.8
        } else {
            (0.2 + 0.8 * self.phen.expansion) * (1.0 - 0.5 * self.phen.frost_damage)
        };
        let mut clusters = Vec::new();
        for n in self.shoots.nodes.iter().filter(|n| n.leafy) {
            let key = rng::hash(self.seed, S_LEAF, n.id as u64, 0);
            let h1 = rng::unit(key);
            let h2 = rng::unit(rng::mix(key ^ 1));
            let end = n.end(ext);
            let color = if dormant {
                // Lower crown keeps its dead leaves more often.
                let rel = end.y / height;
                if h1 >= self.marcescent * (1.5 - rel) {
                    continue;
                }
                let v = 0.85 + 0.3 * h2;
                [phenology::MARCESCENT_BROWN[0] * v, phenology::MARCESCENT_BROWN[1] * v, phenology::MARCESCENT_BROWN[2] * v]
            } else {
                if h1 < self.phen.fallen {
                    continue;
                }
                // Shaded leaves colour later; per-cluster variation.
                let c = (self.phen.colouring + 0.3 * (h2 - 0.5) + 0.15 * (n.light - 0.5)).clamp(0.0, 1.0);
                let c = if self.phen.colouring > 0.0 { c } else { 0.0 };
                let mut col = phenology::leaf_colour(&g.pigments, self.phen.expansion, c);
                let v = 0.88 + 0.24 * h2 * (0.6 + 0.4 * n.light);
                for x in col.iter_mut() {
                    *x = (*x * v).min(1.0);
                }
                col
            };
            let size = base_size * size_factor * (0.85 + 0.3 * h2);
            clusters.push(LeafCluster { pos: end, dir: n.dir, size, color: [color[0], color[1], color[2], 1.0], key });
            // Long shoots get a second cluster half-way.
            let len = n.length * ext;
            if len > 2.0 * size {
                let mid = n.start + n.dir * (len * 0.5);
                clusters.push(LeafCluster {
                    pos: mid,
                    dir: n.dir,
                    size: size * 0.8,
                    color: [color[0], color[1], color[2], 1.0],
                    key: rng::mix(key ^ 7),
                });
            }
        }
        mesh::build_leaves(&clusters, LEAF_TRI_BUDGET)
    }

}
