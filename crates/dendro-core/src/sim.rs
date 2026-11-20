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
//!
//! Flowering and fruiting run alongside (see [`crate::phenology`]); they do
//! not feed back into growth (no reproductive cost, a simplification).

use crate::fmath::DMath;
use crate::climate::{Climate, ClimateParams, Weather};
use crate::genome::Genome;
use crate::growth::ShootSystem;
use crate::mesh::{self, LeafCluster, MeshData, TubeSeg, TubeStyle};
use crate::phenology::{self, PhenoEvent, Phase, Phenology, ReproStage, Reproduction, Season};
use crate::rings::{self, Ring};
use crate::rng::{self, S_FLOWER, S_LEAF};
use crate::roots::RootSystem;
use crate::soil::{SoilColumn, SoilLayerState, SoilProfile};

/// Days per simulated year (no leap years).
pub const DAYS_PER_YEAR: u32 = 365;

/// Triangle budget of the wood (trunk + branches) mesh.
pub const WOOD_TRI_BUDGET: usize = 220_000;
/// Triangle budget of the root mesh.
pub const ROOT_TRI_BUDGET: usize = 55_000;
/// Triangle budget of the fine-root tufts (wood + roots + fine roots <= 290k).
pub const FINE_ROOT_TRI_BUDGET: usize = 15_000;
/// Triangle budget of the leaf mesh.
pub const LEAF_TRI_BUDGET: usize = 380_000;
/// Triangle budget of the flower / fruit mesh (cards of 4 vertices and 2
/// triangles: at most `2 * FLOWER_TRI_BUDGET` = 60k vertices).
pub const FLOWER_TRI_BUDGET: usize = 30_000;

/// Productivity of an average year (sum of daily index), used to normalise.
const P_YEAR_REF: f32 = 100.0;
/// Productivity of an average summer (expansion complete -> senescence).
const P_SUMMER_REF: f32 = 70.0;

/// Drawn (mesh) radius of a branch of simulated radius `rs` (see
/// `Simulation::wood_mesh`); shared with the pruning picker and wound mesh.
pub(crate) fn draw_radius(rs: f32, r_ref: f32, e: f32, boost_max: f32) -> f32 {
    let pipe = (r_ref / rs.max(1e-6)).dpowf(1.0 - 2.0 / e);
    let fade = ((rs - 0.005) / 0.035).clamp(0.0, 1.0);
    let cap = 1.0 + (boost_max - 1.0) * fade * fade * (3.0 - 2.0 * fade);
    rs * pipe.min(cap).max(1.0)
}

/// Critical relative extractable water (REWc) below which transpiration and
/// carbon gain decline linearly. Granier et al. 1999 (Ecol Model
/// 116:269-283, Table 1) found REWc "quite constant" at ~0.4 across species
/// (0.3 for one Douglas-fir study). Calibration: the `drought` trait moves it
/// within 0.35-0.45 (0.45 - 0.1 x drought, i.e. 0.40 for both built-in
/// species), a small, unsourced species effect.
pub(crate) fn rew_critical(g: &Genome) -> f32 {
    0.45 - 0.1 * g.tolerances.drought.clamp(0.0, 1.0)
}

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
    /// Shoot nodes that are standing dead wood (crown retrenchment).
    pub dead_wood_nodes: usize,
}

/// Crown architecture metrics (diagnostics and tests).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeMetrics {
    /// Height (m) of the lowest living branch whose base diameter is at least
    /// 10 % of the stem diameter where it leaves the stem (clear bole).
    pub clear_bole_m: f32,
    /// First-order limbs (leaving the stem, followed along its thickest
    /// child) with a base diameter of at least 20 % of DBH.
    pub scaffold_limbs: u32,
    /// Largest limb/stem diameter ratio among those limbs (local stem).
    pub max_limb_ratio: f32,
    /// Mean limb/stem diameter ratio among those limbs.
    pub mean_limb_ratio: f32,
    /// Crown width (2 x mean sector radius) over height.
    pub crown_width_to_height: f32,
}

/// A growing tree in its environment.
#[derive(Clone, Debug)]
pub struct Simulation {
    pub(crate) genome: Genome,
    pub(crate) seed: u64,
    climate: Climate,
    soil: SoilColumn,
    /// Mean water stress of the last completed season (0..1).
    pub(crate) stress_prev: f32,
    /// Root-weighted soil hypoxia summed since the last budburst, and its
    /// mean over the previous budburst-to-budburst year (0..1).
    hyp_sum: f32,
    hyp_days: f32,
    hyp_prev: f32,
    /// Root-weighted nutrient availability of the site (0..1).
    fertility: f32,
    pub(crate) phen: Phenology,
    pub(crate) shoots: ShootSystem,
    pub(crate) roots: RootSystem,
    pub(crate) rings: Vec<Ring>,
    pub(crate) day: u64,
    pub(crate) drought: bool,
    drought_in_season: bool,
    pub(crate) today: Weather,
    water_factor: f32,
    p_year: f32,
    p_summer: f32,
    pub(crate) p_prev: f32,
    in_summer: bool,
    stress_sum: f32,
    stress_days: f32,
    pending_early_mm: f32,
    pending_measure: f32,
    marcescent: f32,
    pub(crate) wood_version: u64,
    pub(crate) leaf_version: u64,
    pub(crate) root_version: u64,
    pub(crate) flower_version: u64,
    reported_version: u64,
    leaf_key: u64,
    flower_key: u64,
    repro: Reproduction,
    /// Pruning state: intervention log, wounds, reserves (see `crate::pruning`).
    pub(crate) prune: crate::pruning::PruneState,
    /// Carbon status, ageing and death (see `crate::vitality`).
    pub(crate) vit: crate::vitality::Vitality,
    /// Wind damage and acclimation (see `crate::wind`); the cache is derived
    /// data and front-end buffers, outside the state hash.
    pub(crate) wind: crate::wind::WindState,
    pub(crate) wind_cache: crate::wind::WindCache,
    /// Collar area increment (m2) of this season's earlywood.
    pending_collar: f32,
}

impl Simulation {
    /// New seedling (seed about to germinate) on Jan 1 of year 0, with the
    /// default temperate-oceanic climate.
    pub fn new(genome: Genome, seed: u64) -> Self {
        Self::with_climate(genome, seed, ClimateParams::default())
    }

    /// New seedling with custom climate parameters (default soil).
    pub fn with_climate(genome: Genome, seed: u64, climate: ClimateParams) -> Self {
        Self::with_site(genome, seed, climate, SoilProfile::default())
    }

    /// New seedling on a given site: climate and soil profile.
    pub fn with_site(genome: Genome, seed: u64, climate: ClimateParams, soil: SoilProfile) -> Self {
        // Chill days of a typical Nov-Dec before the first spring (the
        // default Rennes climate has ~7 days <= 5 degC by Dec 31).
        // Calibration: a constant, the year before germination is not simulated.
        let chill = 7.0;
        let shoots = ShootSystem::new(&genome);
        // Individual variation of the juvenile phase: +-1 year.
        let first_age = genome.reproduction.first_flowering_age_years + 2.0 * (rng::unit(rng::hash(seed, S_FLOWER, 0, 0)) - 0.5);
        let mut soil = SoilColumn::new(soil);
        soil.track_hypoxia = genome.roots.hypoxia_sensitivity > 0.0;
        soil.hypoxia_afp = genome.roots.hypoxia_air_porosity;
        soil.rew_critical = rew_critical(&genome);
        Simulation {
            climate: Climate::new(climate, seed),
            soil,
            stress_prev: 0.0,
            hyp_sum: 0.0,
            hyp_days: 0.0,
            hyp_prev: 0.0,
            fertility: 0.8,
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
            flower_version: 1,
            reported_version: 0,
            leaf_key: 0,
            flower_key: 0,
            repro: Reproduction::new(first_age, seed),
            prune: crate::pruning::PruneState::default(),
            vit: crate::vitality::Vitality::default(),
            wind: crate::wind::WindState::default(),
            wind_cache: crate::wind::WindCache::default(),
            pending_collar: 0.0,
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
        let v = self.wood_version + self.leaf_version + self.root_version + self.flower_version;
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
        // Logged so that a replay of the pruning timeline (and the unpruned
        // control tree) sees the same weather.
        if on != self.drought {
            self.prune.record_drought(self.day, on);
        }
        self.drought = on;
    }

    pub fn drought(&self) -> bool {
        self.drought
    }

    fn step_day(&mut self) {
        if self.prune.has_pending() {
            self.apply_due_events();
        }
        let year = (self.day / DAYS_PER_YEAR as u64) as u32;
        let doy = (self.day % DAYS_PER_YEAR as u64) as u32;
        if doy == 0 && self.day > 0 {
            self.p_prev = (self.p_year / P_YEAR_REF).clamp(0.2, 1.6);
            self.p_year = 0.0;
            self.wind_new_year();
        }
        let w = self.climate.day(year, doy, self.drought);
        self.today = w;

        // Water balance of the layered soil.
        let leaf = self.phen.leaf_area(self.genome.leaf.retained_share());
        let wd = self.soil.step(w.precip_mm, w.pet_mm, leaf, leaf > 0.3);
        // REW of stomatal closure onset (same threshold as the soil's
        // transpiration, see `rew_critical`).
        let thr = self.soil.rew_critical;
        // Roots in saturated (anoxic) soil stop taking up water and signal
        // stomatal closure (Kozlowski 1997); tolerant species suffer less.
        let tol = self.genome.roots.waterlogging_tolerance.clamp(0.0, 1.0);
        self.water_factor = (wd.rew / thr).min(1.0) * (1.0 - (1.0 - tol) * wd.anoxic);
        // Poorly aerated (heavy, wet) soil impairs the roots of sensitive
        // species even where it is not saturated.
        let hs = self.genome.roots.hypoxia_sensitivity;
        if hs > 0.0 {
            self.water_factor *= 1.0 - hs.min(1.0) * wd.hypoxic;
            // All year round: sensitive roots die back in waterlogged winter
            // and spring soil, not only while the leaves transpire.
            self.hyp_sum += wd.hypoxic;
            self.hyp_days += 1.0;
        }

        // Phenology and structural events.
        match self.phen.step(&w, doy, &self.genome) {
            PhenoEvent::Budburst => self.on_budburst(year),
            PhenoEvent::ExpansionComplete => self.on_expansion_complete(year),
            PhenoEvent::SenescenceStart => self.on_senescence(year),
            PhenoEvent::LeafFallComplete => self.on_leaf_fall(),
            PhenoEvent::None => {}
        }
        // The day's strongest gust against the tree (crate::wind).
        self.wind_day();
        let age = self.day as f32 / DAYS_PER_YEAR as f32;
        self.repro.step(&w, doy, age, &self.phen, &self.genome);
        // Shoot extension in 5 visible steps.
        let ext_q = (self.phen.shoot_ext * 5.0).floor() / 5.0;
        if ext_q > self.shoots.ext {
            self.shoots.ext = ext_q;
            self.wood_version += 1;
        }

        // Daily productivity index (light x temperature x water).
        let t = w.temp_c;
        let f_t = ((t - 4.0) / 14.0).clamp(0.0, 1.0) * ((38.0 - t) / 8.0).clamp(0.0, 1.0);
        // Nutrient availability of the rooted soil scales productivity
        // mildly (stylised: poor sand ~ -25 %).
        let f_nut = (0.6 + 0.5 * self.fertility).min(1.0);
        let p_day = leaf * f_t * self.water_factor * f_nut * (w.photoperiod_h / 14.0);
        self.p_year += p_day;
        if self.in_summer {
            self.p_summer += p_day;
        }
        // Water stress of the growing season only (an evergreen's winter
        // foliage would otherwise dilute it; for a deciduous tree the leaf
        // area is 0 while dormant, so this is the same test).
        if leaf > 0.3 && self.phen.phase != Phase::Dormant {
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
        // Flower appearance key: constant (0) while nothing is displayed, so
        // the flower mesh only changes when there is something to draw.
        let (fl, fr) = (self.flower_density(), self.fruit_density());
        let fkey = if fl > 0.0 || fr > 0.0 {
            let r = &self.genome.reproduction;
            1 | (self.repro.stage as u64) << 1
                | ((fl * 12.0).round() as u64) << 4
                | ((fr * 10.0).round() as u64) << 9
                | ((self.repro.ripeness(r) * 8.0).round() as u64) << 14
                | ((self.repro.frost_loss * 4.0).round() as u64) << 18
                | (self.wood_version & 0xFFFF_FFFF) << 24
        } else {
            0
        };
        if fkey != self.flower_key {
            self.flower_key = fkey;
            self.flower_version += 1;
        }
        self.day += 1;
    }

    fn on_budburst(&mut self, year: u32) {
        self.marcescent = 0.0;
        let vigour = (0.35 + 0.65 * self.p_prev).clamp(0.3, 1.3);
        // Functional balance: after a stressed season more goes to roots,
        // less to shoots (uses last season's stress only, so this year's
        // weather cannot leak into earlywood).
        let rt = &self.genome.roots;
        let rs0 = rt.root_shoot_ratio.clamp(0.0, 0.9);
        let rs = (rs0 * (1.0 + rt.root_shoot_stress_gain * self.stress_prev)).min(0.95);
        let mut vigour = vigour * (1.0 - rs) / (1.0 - rs0);
        let age = self.age_years();
        if self.genome.roots.hypoxia_sensitivity > 0.0 {
            self.hyp_prev = if self.hyp_days > 0.0 { self.hyp_sum / self.hyp_days } else { 0.0 };
            self.hyp_sum = 0.0;
            self.hyp_days = 0.0;
        }
        let decline = self.growth_capacity(age);
        if decline < 1.0 {
            vigour *= decline;
        }
        if self.prune.active {
            vigour *= self.prune_flush_vigour();
        }
        let fs = self.shoots.flush(&self.genome, year, age, vigour, self.seed);
        self.prune.last_new_shoots = fs.new_shoots as u32;
        self.stress_sum = 0.0;
        self.stress_days = 0.0;
        self.drought_in_season = false;
        self.wood_version += 1;
    }

    /// Whole-tree growth capacity (0..1]: age-related decline (longevity
    /// traits) x root health after a year in poorly aerated soil
    /// (`hypoxia_sensitivity`). Exactly 1.0 when neither applies.
    ///
    /// With the carbon balance on (`LongevityTraits::maintenance_per_m >
    /// 0`, see `crate::vitality`), the age-scheduled decline is replaced by
    /// the reserve status: ageing then follows from the tree's state, not
    /// from its age.
    fn growth_capacity(&self, age: f32) -> f32 {
        let mut f = if self.ageing() { self.vit.capacity() } else { self.genome.longevity.vigour(age) };
        let hs = self.genome.roots.hypoxia_sensitivity;
        if hs > 0.0 && self.hyp_prev > 0.0 {
            f *= (1.0 - hs.min(1.0) * self.hyp_prev).max(0.3);
        }
        if self.prune.active {
            f *= self.prune.capacity();
        }
        f
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
        let a = &self.genome.architecture;
        self.shoots.compute_foliage(a.hydraulic_path_max_m, a.hydraulic_foliage_exponent);
        self.update_cost_ratio();
        // Earlywood from reserves (last year's productivity): ring-porous
        // oak builds earlywood from stored starch (Barbaroux & Breda 2002,
        // Tree Physiol 22:1201-1210); the 0.4..1.3 clamp is a calibration.
        let w = &self.genome.wood;
        let share = w.earlywood_share;
        let mut factor = share * self.p_prev.clamp(0.4, 1.3);
        let decline = self.growth_capacity(self.age_years());
        if decline < 1.0 {
            factor *= decline;
        }
        let (mi, mh) = self.measure_node();
        let a0 = self.shoots.nodes[mi].area;
        let max_dr = w.max_ring_width_mm * 0.001 * share * 1.5;
        let wind = self.wind_stress_now();
        let collar = self.shoots.thicken(&self.genome, factor, max_dr, &wind);
        // The yearly wind acclimation demand of the new structure
        // (crate::wind).
        self.wind_yearly_screen();
        self.pending_early_mm = rings::width_mm(a0, self.shoots.nodes[mi].area);
        self.pending_measure = mh;
        self.pending_collar = collar;
        self.roots.thicken(collar);
        if !self.is_alive() {
            // A dead tree: no root growth.
            self.wood_version += 1;
            return;
        }
        // Root growth.
        let cr = self.shoots.crown_radius();
        let age = self.age_years();
        let prod = if decline < 1.0 { self.p_prev * decline } else { self.p_prev };
        // After a heavy cut the roots stop extending (functional balance).
        let prod = if self.prune.active { prod * self.prune.root_growth_factor() } else { prod };
        self.roots.grow(&self.genome.roots, year, age, cr, prod, self.stress_prev, &self.soil, self.seed);
        self.roots.update_layers(&self.genome.roots, &mut self.soil);
        let (mut nsum, mut lsum) = (0.0, 0.0);
        for l in &self.soil.layers {
            nsum += l.root_len * l.nutrient;
            lsum += l.root_len;
        }
        if lsum > 0.0 {
            self.fertility = nsum / lsum;
        }
        self.wood_version += 1;
        self.root_version += 1;
    }

    fn on_senescence(&mut self, year: u32) {
        self.in_summer = false;
        let w = &self.genome.wood;
        let share = w.earlywood_share;
        let p_summer = (self.p_summer / P_SUMMER_REF).clamp(0.03, 1.5);
        let mut factor = (1.0 - share) * p_summer;
        let decline = self.growth_capacity(self.age_years());
        if decline < 1.0 {
            factor *= decline;
        }
        let (mi, _) = self.measure_node();
        let a0 = self.shoots.nodes[mi].area;
        let max_dr = w.max_ring_width_mm * 0.001 * (1.0 - share) * 1.5;
        let wind = self.wind_stress_now();
        let collar = self.shoots.thicken(&self.genome, factor, max_dr, &wind);
        let late = rings::width_mm(a0, self.shoots.nodes[mi].area);
        self.roots.thicken(collar);
        self.shoots.sag(&self.genome);
        // Captured before this season's deaths (pruning starvation here,
        // ageing below): the ring of the season a trunk dies in is kept.
        let trunk_alive = self.vit.death_year == crate::growth::NONE;
        if self.prune.active {
            self.prune_yearly(year);
        }
        let early = self.pending_early_mm;
        let total = early + late;
        self.stress_prev = if self.stress_days > 0.0 { self.stress_sum / self.stress_days } else { 0.0 };
        self.vitality_yearly(year, p_summer, self.pending_collar + collar);
        self.pending_collar = 0.0;
        // A dead trunk lays no ring (sprouts of a stool are new stems).
        if !trunk_alive {
            self.wood_version += 1;
            self.root_version += 1;
            return;
        }
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
        self.soil.close_season();
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

    /// Climate parameters of the site (saved with the tree, see `crate::save`).
    pub fn climate_params(&self) -> &ClimateParams {
        self.climate.params()
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
        // An uprooted trunk lies on the ground: only its stump sprouts can
        // be measured at breast height.
        let node = if self.vit.uprooted { self.shoots.living_stem_node_at(1.3) } else { self.shoots.trunk_node_at(1.3) };
        match node {
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

    /// Polar occupancy grid of the root system for a display envelope:
    /// max root depth (m) per (azimuth sector, radial bin) out to `r_max`,
    /// sector-major; 0 = no root. See [`crate::roots::RootSystem::envelope`].
    pub fn root_envelope(&self, sectors: usize, bins: usize, r_max: f32) -> Vec<f32> {
        self.roots.envelope(sectors, bins, r_max)
    }

    /// Root water uptake (= actual transpiration) of the last simulated day,
    /// mm/day; per layer in [`Self::soil_layers`] (`uptake_mm`).
    pub fn water_uptake_mm(&self) -> f32 {
        self.soil.last.uptake
    }

    /// Mean leaf colour of the canopy today (RGB 0..1). Evergreens stay
    /// green all year (only the cast cohort, a small share, yellows).
    pub fn leaf_color(&self) -> [f32; 3] {
        let ret = self.genome.leaf.retained_share();
        if ret > 0.0 {
            let green = phenology::leaf_colour(&self.genome.pigments, 1.0, 0.0);
            if self.phen.phase == Phase::Dormant {
                return green;
            }
            let now = phenology::leaf_colour(&self.genome.pigments, self.phen.expansion, self.phen.colouring);
            let k = 1.0 - ret;
            return [0, 1, 2].map(|i| green[i] * ret + now[i] * k);
        }
        if self.phen.phase == Phase::Dormant {
            return phenology::MARCESCENT_BROWN;
        }
        phenology::leaf_colour(&self.genome.pigments, self.phen.expansion, self.phen.colouring)
    }

    /// Leaf density factor 0..1 (visible foliage relative to full summer
    /// canopy; dead marcescent leaves count half; an evergreen keeps its
    /// older cohorts in winter).
    pub fn leaf_density(&self) -> f32 {
        let ret = self.genome.leaf.retained_share();
        match self.phen.phase {
            Phase::Dormant if ret <= 0.0 => self.marcescent * 0.5,
            _ => self.phen.leaf_area(ret),
        }
    }

    /// The species keeps its foliage over winter (evergreen, several leaf cohorts).
    pub fn evergreen(&self) -> bool {
        self.genome.leaf.evergreen()
    }

    /// Stage of this year's flowering / fruiting cycle.
    pub fn repro_stage(&self) -> ReproStage {
        self.repro.stage
    }

    /// Visible bloom 0..1: flowers opening, full bloom, petal fall (0 before
    /// first flowering, and for species whose flowers are not displayed:
    /// scaled by `ReproductionTraits::flower_display`; a late frost browns
    /// part of the bloom).
    pub fn flower_density(&self) -> f32 {
        let r = &self.genome.reproduction;
        // A dead trunk does not flower (the flowering of stump sprouts is
        // not modelled).
        if r.flower_display <= 0.0 || !self.trunk_alive() {
            return 0.0;
        }
        self.repro.flower_amount() * r.flower_display.min(1.0)
    }

    /// Visible fruit 0..1 (fruit set after late frost x swelling x removal
    /// once ripe), scaled by `ReproductionTraits::fruit_display`.
    pub fn fruit_density(&self) -> f32 {
        let r = &self.genome.reproduction;
        if r.fruit_display <= 0.0 || !self.trunk_alive() {
            return 0.0;
        }
        self.repro.fruit_amount(r) * (1.0 - self.repro.frost_loss) * r.fruit_display.min(1.0)
    }

    /// Monotonic bloom progress 0..1 this year (0 before bloom onset, 1 once
    /// petal fall is complete; tells opening from petal fall, which
    /// `flower_density` does not).
    pub fn bloom_progress(&self) -> f32 {
        match self.repro.stage {
            ReproStage::Waiting => 0.0,
            ReproStage::Blooming => self.repro.bloom,
            ReproStage::Fruiting | ReproStage::Done => 1.0,
        }
    }

    /// Fraction 0..1 of this year's flowers killed by late frost.
    pub fn flower_frost_loss(&self) -> f32 {
        self.repro.frost_loss
    }

    /// Fruit ripeness 0..1.
    pub fn fruit_ripeness(&self) -> f32 {
        self.repro.ripeness(&self.genome.reproduction)
    }

    /// Mean petal colour today (RGB 0..1); frost-killed flowers brown.
    pub fn flower_color(&self) -> [f32; 3] {
        const FROSTED: [f32; 3] = [0.48, 0.36, 0.24];
        let c = self.genome.reproduction.flower_color;
        let f = 0.7 * self.repro.frost_loss;
        [0, 1, 2].map(|k| c[k] + (FROSTED[k] - c[k]) * f)
    }

    /// Mean fruit colour today (RGB 0..1), unripe -> ripe.
    pub fn fruit_color(&self) -> [f32; 3] {
        let r = &self.genome.reproduction;
        let t = self.repro.ripeness(r);
        [0, 1, 2].map(|k| r.fruit_color_unripe[k] + (r.fruit_color_ripe[k] - r.fruit_color_unripe[k]) * t)
    }

    /// Relative extractable soil water 0..1 of the rooted profile (layers
    /// weighted by root presence, see [`crate::soil`]).
    pub fn soil_water(&self) -> f32 {
        self.soil.last.rew
    }

    /// The site's soil profile (data).
    pub fn soil_profile(&self) -> &SoilProfile {
        &self.soil.profile
    }

    /// Simulated soil layers, top down (water, root density...).
    pub fn soil_layers(&self) -> &[SoilLayerState] {
        self.soil.states()
    }

    /// Fraction of root length above `depth_m`: `(structural roots only,
    /// all roots including the implicit fine roots)`.
    pub fn root_length_fraction_above(&self, depth_m: f32) -> (f32, f32) {
        self.roots.length_fraction_above(&self.soil, depth_m)
    }

    /// Mean water stress (0..1) of the last completed growing season.
    pub fn last_season_water_stress(&self) -> f32 {
        self.stress_prev
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
    pub fn flower_version(&self) -> u64 {
        self.flower_version
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
            dead_wood_nodes: self.shoots.nodes.iter().filter(|n| n.is_snag()).count(),
        }
    }

    /// Crown architecture metrics.
    pub fn shape_metrics(&self) -> ShapeMetrics {
        let (clear_bole_m, scaffold_limbs, max_limb_ratio, mean_limb_ratio) =
            self.shoots.shape_metrics(self.dbh_cm() * 0.01);
        ShapeMetrics {
            clear_bole_m,
            scaffold_limbs,
            max_limb_ratio,
            mean_limb_ratio,
            crown_width_to_height: 2.0 * self.crown_radius_m() / self.height_m().max(0.01),
        }
    }

    /// Diagnostics: per first-order limb on the stem, `(base height m,
    /// base radius m, farthest horizontal reach m, leafy tips, mean tip light,
    /// limb age y)`.
    #[doc(hidden)]
    pub fn debug_limbs(&self) -> Vec<(f32, f32, f32, u32, f32, u32)> {
        let nodes = &self.shoots.nodes;
        let n = nodes.len();
        let mut limb = vec![usize::MAX; n];
        let mut out: Vec<(f32, f32, f32, u32, f32, u32)> = Vec::new();
        let year = self.year();
        for i in 1..n {
            let nd = &nodes[i];
            let p = nd.parent as usize;
            if nd.order == 1 && nodes[p].order == 0 {
                limb[i] = out.len();
                out.push((nd.start.y, nd.radius(), 0.0, 0, 0.0, year.saturating_sub(nd.birth_year)));
            } else if nd.order >= 1 {
                limb[i] = limb[p];
            }
            if limb[i] != usize::MAX {
                let l = &mut out[limb[i]];
                l.2 = l.2.max(nd.end(1.0).horizontal_len());
                if nd.leafy {
                    l.3 += 1;
                    l.4 += nd.light;
                }
            }
        }
        for l in out.iter_mut() {
            if l.3 > 0 {
                l.4 /= l.3 as f32;
            }
        }
        out
    }

    /// Diagnostics: `(light-weighted foliage carried by the collar, in m2 of
    /// crown cells, foliage-weighted mean path length m)` from the last
    /// foliage pass.
    #[doc(hidden)]
    pub fn debug_foliage(&self) -> (f32, f32) {
        let f = self.shoots.nodes[0].foliage;
        (f, if f > 0.0 { self.shoots.area_per_foliage(1.0).recip() } else { 0.0 })
    }

    /// Diagnostics: da Vinci area ratio `sum(child areas) / parent area` at
    /// living branch points whose parent radius is at least `min_r` (m):
    /// `(mean, count)`. 1 = area preserving (pipe exponent 2).
    #[doc(hidden)]
    pub fn debug_area_ratio(&self, min_r: f32) -> (f32, usize) {
        let nodes = &self.shoots.nodes;
        let mut sum = vec![0.0f32; nodes.len()];
        let mut kids = vec![0u32; nodes.len()];
        for nd in nodes.iter().skip(1).filter(|n| !n.is_snag()) {
            sum[nd.parent as usize] += nd.area;
            kids[nd.parent as usize] += 1;
        }
        let (mut acc, mut n) = (0.0f32, 0usize);
        for (i, nd) in nodes.iter().enumerate() {
            if kids[i] >= 2 && nd.radius() >= min_r && !nd.is_snag() {
                acc += sum[i] / nd.area;
                n += 1;
            }
        }
        (if n > 0 { acc / n as f32 } else { 0.0 }, n)
    }

    /// Diagnostics: along the thickest first-order limb (followed through
    /// its thickest child), `(distance along the axis m, simulated radius m,
    /// order of the node, number of children)` per node.
    #[doc(hidden)]
    pub fn debug_limb_profile(&self) -> Vec<(f32, f32, u8, u32)> {
        let nodes = &self.shoots.nodes;
        let n = nodes.len();
        let mut thickest = vec![u32::MAX; n];
        let mut kids = vec![0u32; n];
        for (i, nd) in nodes.iter().enumerate().skip(1) {
            let p = nd.parent as usize;
            kids[p] += 1;
            let t = thickest[p];
            if t == u32::MAX || nd.area > nodes[t as usize].area {
                thickest[p] = i as u32;
            }
        }
        // Trunk, then the thickest non-trunk child of any trunk node.
        let mut on = vec![false; n];
        let mut cur = 0usize;
        on[0] = true;
        while thickest[cur] != u32::MAX {
            cur = thickest[cur] as usize;
            on[cur] = true;
        }
        let mut best = (0.0f32, usize::MAX);
        for (i, nd) in nodes.iter().enumerate().skip(1) {
            if on[nd.parent as usize] && !on[i] && nd.area > best.0 {
                best = (nd.area, i);
            }
        }
        let mut out = Vec::new();
        let mut i = best.1;
        let mut d = 0.0;
        while i != usize::MAX && i < n {
            let nd = &nodes[i];
            out.push((d, nd.radius(), nd.order, kids[i]));
            d += nd.length;
            i = if thickest[i] == u32::MAX { usize::MAX } else { thickest[i] as usize };
        }
        out
    }

    // ------------------------------------------------------------------
    // Meshes
    // ------------------------------------------------------------------

    /// Trunk and branches. See [`crate::mesh`] for conventions.
    pub fn wood_mesh(&self) -> MeshData {
        self.wood_mesh_with(None)
    }

    /// The wood mesh with one vertex colour per shoot segment (node order,
    /// e.g. [`crate::mechanics::safety_colors`]) instead of the bark colours.
    pub fn wood_mesh_tinted(&self, colors: &[[f32; 4]]) -> MeshData {
        self.wood_mesh_with(Some(colors))
    }

    fn wood_mesh_with(&self, tint: Option<&[[f32; 4]]>) -> MeshData {
        let year = self.year();
        let ext = self.shoots.ext;
        let nodes = &self.shoots.nodes;
        // The mesh axis follows the terminal child, unless a lateral has
        // clearly overtaken it (a fork or a relay), in which case it follows
        // the thickest child so radii never drop abruptly along a tube.
        // (Following the thickest child blindly would hop between near-equal
        // siblings and draw zigzags.)
        let mut thickest = vec![u32::MAX; nodes.len()];
        let mut term = vec![u32::MAX; nodes.len()];
        for (i, n) in nodes.iter().enumerate().skip(1) {
            let p = n.parent as usize;
            let t = thickest[p];
            if t == u32::MAX || n.area > nodes[t as usize].area {
                thickest[p] = i as u32;
            }
            if n.terminal {
                term[p] = i as u32;
            }
        }
        for p in 0..nodes.len() {
            let (t, k) = (term[p], thickest[p]);
            if t != u32::MAX && k != u32::MAX && nodes[k as usize].area < 1.5 * nodes[t as usize].area {
                thickest[p] = t;
            }
        }
        let r_ref = nodes[0].radius().max(1e-6);
        // Eloy 2011, Phys Rev Lett 107:258101, doi:10.1103/PhysRevLett.107.258101:
        // Leonardo exponents of ~1.8-2.3 across species. The species
        // `pipe_exponent` values (2.3 oak, 2.1 cherry) lie in that range;
        // the per-species choice within it is a calibration (no Q. robur or
        // P. avium measurement found). Values below 2 are drawn as 2, the
        // exponent of the simulated (Pressler) areas.
        let e = self.genome.wood.pipe_exponent.max(2.0);
        let boost_max = self.genome.wood.limb_draw_boost.max(1.0);
        let bark = &self.genome.bark;
        let segs: Vec<TubeSeg> = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let age = year.saturating_sub(n.birth_year) as f32;
                // Rendered radius. The simulated areas follow Pressler's law
                // (a branching exponent of exactly 2) plus, on loaded limbs, the
                // adaptive radial growth (growth.rs: area ratio ~1.1 at the old
                // oak's limb insertions); measured exponents are
                // ~1.8-2.3 (Eloy 2011), i.e. limbs and boughs may be somewhat
                // thicker relative to the stem than area preservation gives. Drawn
                // radius = simulated radius x (r_collar / r)^(1 - 2/e),
                // capped at `limb_draw_boost` and faded in from 0.5 to 4 cm
                // so twigs keep their simulated size (otherwise the fine
                // spray turns into a tangle of thick wires). Mesh only: DBH,
                // rings and the simulation use the simulated areas.
                let rs = n.radius();
                let r = draw_radius(rs, r_ref, e, boost_max);
                // Young shoots green-brown, then grey-brown bark, darker on old trunks.
                let t = (age / 4.0).min(1.0);
                let dark = (r / 0.4).min(1.0);
                let col = if let Some(c) = tint.and_then(|t| t.get(i)) {
                    *c
                } else if n.is_snag() {
                    // Dead wood: bark sloughs off, silver-grey.
                    [0.56, 0.54, 0.50, 1.0]
                } else {
                    let c = |k: usize| (bark.young_color[k] + (bark.color[k] - bark.young_color[k]) * t - bark.old_darkening[k] * dark).clamp(0.0, 1.0);
                    [c(0), c(1), c(2), 1.0]
                };
                TubeSeg {
                    parent: n.parent,
                    start: n.start,
                    end: n.end(ext),
                    radius: r,
                    // Sprouts on a pruning cut face start their own axes, so the
                    // cut stem keeps its girth up to the cut (see `crate::pruning`).
                    continues: i > 0 && thickest[n.parent as usize] == i as u32 && !nodes[n.parent as usize].cut_end,
                    color: col,
                    // Dead limbs end in broken, blunt stubs.
                    tip_taper: if n.is_snag() {
                        0.85
                    } else if n.cut_end {
                        // Pruning cut face (see `crate::pruning`): blunt end.
                        1.0
                    } else {
                        0.5
                    },
                    path0: 0.0,
                }
            })
            .collect();
        let w = &self.genome.wood;
        // Buttress lobes sit over the thickest main lateral roots.
        let mut lobes = [0.0f32; 8];
        let az = self.roots.main_lateral_azimuths((w.buttress_lobes as usize).min(8));
        let lobe_count = if az.len() >= 3 { az.len() as u32 } else { 0 };
        for (l, a) in lobes.iter_mut().zip(&az) {
            *l = *a;
        }
        let style = TubeStyle {
            collar_flare: w.collar_flare,
            root_flare: w.root_flare,
            root_flare_height: w.root_flare_height_radii * nodes[0].radius(),
            buttress_lobes: w.buttress_lobes as u32,
            smooth_min_radius: 0.04,
            irregularity: w.bark_irregularity,
            lobe_azimuths: lobes,
            lobe_count,
            twig_radius: 0.006,
            twig_min_len: (0.6 * self.shoots.voxel).max(0.25),
            path_uv2: false,
        };
        mesh::build_tubes(&segs, WOOD_TRI_BUDGET, 0.0008, style)
    }

    /// Root system (below y = 0).
    pub fn root_mesh(&self) -> MeshData {
        // Keep thick shallow roots under the surface: sink each tube centre
        // to at least ~its radius below y = 0 (except the collar ring).
        let sink = |p: crate::math::Vec3, r: f32| crate::math::Vec3::new(p.x, p.y.min(-1.1 * r), p.z);
        // Zone of rapid taper (Eis 1974; Wilson 1964): structural laterals
        // are much thicker within 1-2 m of the stem than the pipe model
        // alone gives, merging into the buttresses. Mesh only.
        let k = self.genome.roots.rapid_taper.max(0.0);
        let r_max = 0.8 * self.shoots.nodes[0].radius();
        let path = self.roots.path_starts();
        let segs: Vec<TubeSeg> = self
            .roots
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let d = n.start.horizontal_len();
                let r = if i == 0 { n.radius() } else { (n.radius() * (1.0 + k * (-d).dexp())).min(r_max.max(n.radius())) };
                TubeSeg {
                    parent: n.parent,
                    start: if i == 0 { n.start } else { sink(n.start, r) },
                    end: sink(n.end(), r),
                    radius: r,
                    continues: n.continues,
                    color: [0.47, 0.37, 0.27, 1.0],
                    tip_taper: 0.3,
                    path0: path[i],
                }
            })
            .collect();
        let style = TubeStyle { smooth_min_radius: 0.01, path_uv2: true, ..Default::default() };
        mesh::build_tubes(&segs, ROOT_TRI_BUDGET, 0.0008, style)
    }

    /// Fine roots, drawn as sparse fibrous tufts (fans of thin rootlets) at
    /// the ends of distal structural roots. Tufts are placed in proportion
    /// to the fine-root proliferation of the soil layer they sit in (denser
    /// in the topsoil; none in saturated soil), deterministically, within
    /// [`FINE_ROOT_TRI_BUDGET`]. Same conventions as [`Self::root_mesh`];
    /// vertex colour alpha is 0.5 here (1.0 on structural roots) so a
    /// renderer can style the two classes differently.
    pub fn fine_root_mesh(&self) -> MeshData {
        use crate::math::Vec3;
        let rt = &self.genome.roots;
        let cand: Vec<(usize, f32)> = self
            .roots
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind != crate::roots::RootKind::Tap && n.radius() < 0.012)
            .filter_map(|(i, n)| {
                let l = self.soil.layer_at(-n.end().y)?;
                let w = crate::roots::proliferation(&self.soil.layers[l], rt);
                (w > 0.0).then_some((i, w))
            })
            .collect();
        let total: f32 = cand.iter().map(|c| c.1).sum();
        const ROOTLETS: usize = 5;
        let per_tuft = ROOTLETS * 2 * 3 * 2;
        let want = (FINE_ROOT_TRI_BUDGET / per_tuft) as f32;
        let mut segs: Vec<TubeSeg> = Vec::new();
        let col = [0.80, 0.71, 0.56, 0.5];
        let path = self.roots.path_starts();
        for &(i, w) in &cand {
            let n = &self.roots.nodes[i];
            let key = rng::hash(self.seed, S_LEAF ^ 0xF1E, n.id as u64, 0);
            if rng::unit(key) >= want * w / total.max(1e-6) {
                continue;
            }
            let e = n.end();
            let e = Vec3::new(e.x, e.y.min(-0.03), e.z);
            for k in 0..ROOTLETS {
                let u = |s: u64| rng::unit(rng::mix(key ^ ((k as u64) << 8 | s)));
                let rand = Vec3::new(u(1) - 0.5, (u(2) - 0.5) * 0.8 - 0.2, u(3) - 0.5) * 2.0;
                let d1 = (n.dir * 0.6 + rand).normalize_or(n.dir);
                let len = 0.2 + 0.3 * u(4);
                let mut mid = e + d1 * (0.5 * len);
                mid.y = mid.y.min(-0.03);
                let d2 = (d1 + Vec3::new(u(5) - 0.5, u(6) - 0.6, u(7) - 0.5)).normalize_or(d1);
                let mut tip = mid + d2 * (0.5 * len);
                tip.y = tip.y.min(-0.02);
                let a = segs.len() as u32;
                let r = 0.001 + 0.0008 * u(8);
                // Path coordinate continues from the parent root's end.
                let path0 = path[i] + n.length;
                segs.push(TubeSeg { parent: u32::MAX, start: e, end: mid, radius: r, continues: false, color: col, tip_taper: 0.3, path0 });
                segs.push(TubeSeg { parent: a, start: mid, end: tip, radius: r * 0.7, continues: true, color: col, tip_taper: 0.3, path0 });
            }
        }
        mesh::build_tubes(&segs, FINE_ROOT_TRI_BUDGET, 0.0, TubeStyle { path_uv2: true, ..Default::default() })
    }

    /// Leaf clusters with seasonal colour; empty in winter except for
    /// marcescent (dead, retained) leaves on young trees.
    ///
    /// Two kinds of foliage (see the texture atlas in [`crate::mesh`]):
    /// * **Leafy shoot units** (the current year's long shoots). Each carries
    ///   one card cluster drawn as a leafy shoot along the unit (card size
    ///   follows the shoot and leaf length, so a sapling shows single
    ///   shoots, not tufts), plus, where the unit stands for a lumped twig
    ///   spray (old trees, large voxels), extra "spray" clusters scattered
    ///   over the unit's voxel. Card area per unit scales with the voxel
    ///   face (`foliage_density`), at least one card per lumped annual
    ///   shoot (`leaves_per_shoot` leaves each).
    /// * **Short shoots and spurs** along older living wood up to
    ///   `spur_max_radius_mm`, where light allows (`spur_min_light`):
    ///   `spur_foliage` card area per metre of wood, so the limbs and the
    ///   inner crown read as leafy instead of bare wire between the shoot
    ///   tips.
    ///
    /// When the canopy exceeds [`LEAF_TRI_BUDGET`], the spur fill is
    /// thinned first (leafy units keep at least 70 % of the budget), with
    /// cards enlarged to keep the covered area. Autumn colouring starts
    /// per cluster (`colour_spread`): sunlit clusters first, whole boughs
    /// varying together, and the first to turn are the first to fall.
    pub fn leaf_mesh(&self) -> MeshData {
        use crate::math::Vec3;
        let phase = self.phen.phase;
        let dormant = phase == Phase::Dormant;
        let g = &self.genome;
        let lt = &g.leaf;
        // Evergreens keep their older cohorts: green in winter, only the
        // oldest cohort yellows and falls in autumn.
        let evergreen = lt.evergreen();
        if dormant && self.marcescent <= 0.0 && !evergreen {
            return MeshData::default();
        }
        let year = self.year();
        let ext = self.shoots.ext;
        let height = self.shoots.height().max(0.1);
        let voxel = self.shoots.voxel;
        let leaf_len = lt.length_cm * 0.01;
        let size_factor = if dormant {
            0.8
        } else {
            (0.2 + 0.8 * self.phen.expansion) * (1.0 - 0.5 * self.phen.frost_damage)
        };
        // Evergreens: only the expanding flush grows; the older cohorts (and
        // the whole crown outside the flush) are drawn full size.
        let flushing = matches!(phase, Phase::BudBurst | Phase::LeafExpansion);
        let size_of = |n: &crate::growth::Node| -> f32 {
            if !evergreen {
                size_factor
            } else if n.leafy && flushing {
                let fresh = (0.2 + 0.8 * self.phen.expansion) * (1.0 - 0.5 * self.phen.frost_damage);
                // A lumped unit carrying older needles keeps them full size.
                (fresh + n.old_leaf).min(1.0)
            } else {
                1.0
            }
        };
        // Share of a unit's foliage cast this autumn (oldest cohort).
        let cast_share = |n: &crate::growth::Node| -> f32 {
            let w = n.leaf_w();
            if w <= 0.0 {
                return 0.0;
            }
            let a = year.saturating_sub(n.birth_year).max(1) as f32;
            let next = if n.leafy { lt.cohort_survival(1.0) } else { 0.0 } + if n.old_leaf > 0.0 { lt.cohort_survival(a + 1.0) } else { 0.0 };
            (1.0 - next / w).clamp(0.0, 1.0)
        };
        let nodes = &self.shoots.nodes;
        let spur_r = lt.spur_max_radius_mm * 0.001;
        let spur_on = lt.spur_foliage > 0.0 && !dormant;
        // Light of a stretch of wood: its own voxel's, raised towards the
        // mean light of the foliage it carries (a voxel cannot resolve
        // that short shoots sit on the lit side of a limb).
        let spur_light = |n: &crate::growth::Node| {
            let carried = if n.leaf_n > 0.0 { n.leaf_q / n.leaf_n } else { n.light };
            (0.5 * (n.light + carried)).max(n.light)
        };
        let mut leafy: Vec<usize> = Vec::new();
        let mut spurs: Vec<usize> = Vec::new();
        for (i, n) in nodes.iter().enumerate() {
            if n.leaf_w() > 0.0 {
                leafy.push(i);
            } else if spur_on && !n.is_snag() && spur_light(n) > lt.spur_min_light && n.radius() <= spur_r {
                spurs.push(i);
            }
        }
        if leafy.is_empty() {
            return MeshData::default();
        }
        let mut center = Vec3::ZERO;
        for &i in &leafy {
            center += nodes[i].end(ext);
        }
        center = center * (1.0 / leafy.len() as f32);
        center.x = 0.0;
        center.z = 0.0;
        // Foliage lobes: every leafy shoot belongs to the "bough" it hangs
        // from (the first node below 10 % of the collar radius on its path
        // from the stem). Each bough's foliage is shaped and shaded around
        // its own centroid, so the crown reads as rounded masses separated
        // by darker gaps rather than as one uniform ball.
        let r_b = 0.1 * nodes[0].radius();
        let mut bough = vec![u32::MAX; nodes.len()];
        for (i, n) in nodes.iter().enumerate().skip(1) {
            let pb = bough[n.parent as usize];
            bough[i] = if pb != u32::MAX { pb } else if n.radius() < r_b { i as u32 } else { u32::MAX };
        }
        // Per-bough foliage sums, indexed by the bough's node index.
        let mut lobe_sum: Vec<(Vec3, u32)> = vec![(Vec3::ZERO, 0); nodes.len()];
        for &i in &leafy {
            if bough[i] != u32::MAX {
                let e = &mut lobe_sum[bough[i] as usize];
                e.0 += nodes[i].end(ext);
                e.1 += 1;
            }
        }
        let lobe_center = |i: usize| -> Vec3 {
            match bough[i] {
                u32::MAX => center,
                b => match lobe_sum[b as usize] {
                    (sum, k) if k >= 6 => sum * (1.0 / k as f32),
                    _ => center,
                },
            }
        };
        let s_pref = leaf_len * lt.cluster_size_leaves;
        let s_max = (voxel * 1.2).max(s_pref);
        let spread = (0.5 * voxel).max(leaf_len);
        let spur_spread = (0.3 * voxel).max(0.6 * leaf_len);
        let unit = |i: usize, s: u64| rng::unit(rng::hash(self.seed, S_LEAF, nodes[i].id as u64, s));

        // Pass 1: cards wanted per site.
        let leafy_cards: Vec<(f32, f32)> = leafy
            .iter()
            .map(|&i| {
                let n = &nodes[i];
                let lit = n.light.clamp(0.0, 1.0);
                // Shade shoots keep a fair share (shade leaves are larger and
                // thinner, Niinemets 2010).
                // Evergreens: one unit carries several needle cohorts.
                let area = lt.foliage_density * voxel * voxel * (0.55 + 0.45 * lit) * if evergreen { n.leaf_w() } else { 1.0 };
                let want = (area / (s_pref * s_pref)).max(n.unit_years as f32).min(60.0);
                // Small units: one shoot card sized by the shoot itself.
                let size = if want <= 3.0 {
                    // `max` then `min`, not `clamp`: a small `cluster_size_leaves` (< 2.2)
                    // puts s_pref below the floor and `clamp` would panic.
                    (n.length + 1.5 * leaf_len).max(2.2 * leaf_len).min(s_pref)
                } else {
                    (area / want.round()).sqrt().clamp(0.8 * s_pref, s_max)
                };
                (want.round().max(1.0), size)
            })
            .collect();
        let spur_cards: Vec<f32> = spurs
            .iter()
            .map(|&i| {
                let n = &nodes[i];
                let l = spur_light(n);
                let x = ((l - lt.spur_min_light) / 0.2).clamp(0.0, 1.0);
                let lf = x * x * (3.0 - 2.0 * x) * (0.6 + 0.4 * l.min(1.0));
                let want = lt.spur_foliage * voxel * n.length * lf / (s_pref * s_pref);
                // Stochastic rounding keeps the expected area.
                (want.floor() + if want.fract() > unit(i, 0x5B) { 1.0 } else { 0.0 }).min(30.0)
            })
            .collect();
        let want_l: f32 = leafy_cards.iter().map(|c| c.0).sum();
        let want_s: f32 = spur_cards.iter().sum();
        // Kept below the hard budget: mesh build time (old oak) and fill rate.
        let budget = LEAF_TRI_BUDGET as f32 * 0.84 / 2.0;
        let keep_l = ((budget - want_s).max(0.7 * budget) / want_l.max(1.0)).min(1.0);
        let keep_s = ((budget - keep_l * want_l) / want_s.max(1.0)).clamp(0.0, 1.0);

        // Per-cluster colour: autumn onset delay (sunlit first, boughs vary
        // together), fall order, brightness. None = fallen / not shown.
        let spread_c = lt.colour_spread.clamp(0.0, 0.9);
        let colouring = self.phen.colouring;
        let summer = phenology::leaf_colour(&g.pigments, self.phen.expansion, 0.0);
        let evergreen_green = phenology::leaf_colour(&g.pigments, 1.0, 0.0);
        let cluster_colour = |i: usize, lit: f32, ck: u64, pos: Vec3, winter_share: f32| -> Option<[f32; 4]> {
            let u = |s: u64| rng::unit(rng::mix(ck ^ s));
            let n = &nodes[i];
            // Evergreen clusters that are not part of the cast cohort (and
            // every cluster in winter): green; the flush is fresh green.
            let keep_green = evergreen && (dormant || u(24) >= cast_share(n));
            let (rgb, sen) = if keep_green {
                if n.leafy && flushing && n.old_leaf <= 0.0 {
                    (phenology::leaf_colour(&g.pigments, self.phen.expansion, 0.0), 0.0)
                } else {
                    // Older needles a little darker.
                    let age = year.saturating_sub(n.birth_year) as f32;
                    let d = 1.0 - 0.04 * age.min(5.0);
                    ([evergreen_green[0] * d, evergreen_green[1] * d, evergreen_green[2] * d], 0.0)
                }
            } else if dormant {
                // Lower crown keeps its dead leaves more often.
                let rel = pos.y / height;
                if u(20) >= self.marcescent * (1.5 - rel) * winter_share {
                    return None;
                }
                let v = 0.85 + 0.3 * u(21);
                let b = phenology::MARCESCENT_BROWN;
                ([b[0] * v, b[1] * v, b[2] * v], 0.0)
            } else {
                let hb = rng::unit(rng::mix(self.seed ^ (bough[i] as u64).wrapping_mul(0x9E37_79B9)));
                let delay = spread_c * (0.45 * (1.0 - lit) + 0.35 * u(22) + 0.2 * hb).clamp(0.0, 1.0);
                let rank = if spread_c > 0.0 { 0.6 * delay / spread_c + 0.4 * u(23) } else { u(23) };
                if rank < self.phen.fallen {
                    return None;
                }
                if colouring > 0.0 {
                    let c = ((colouring - delay) / (1.0 - spread_c)).clamp(0.0, 1.0);
                    (phenology::leaf_colour(&g.pigments, self.phen.expansion, c), c)
                } else {
                    (summer, 0.0)
                }
            };
            // Per-cluster brightness and slight hue variation; shaded
            // clusters darker.
            let v = (0.78 + 0.3 * u(6)) * (0.8 + 0.2 * lit);
            Some([
                (rgb[0] * v * (0.94 + 0.12 * u(7))).min(1.0),
                (rgb[1] * v).min(1.0),
                (rgb[2] * v * (0.85 + 0.3 * u(8))).min(1.0),
                sen,
            ])
        };
        const SHOOT_TILE: [f32; 2] = [0.004, 0.496];
        const SPRAY_TILE: [f32; 2] = [0.504, 0.996];

        // Pass 2: clusters.
        let cap = (budget / 2.5) as usize + leafy.len();
        let mut clusters = Vec::with_capacity(cap);
        for (&li, &(want, size0)) in leafy.iter().zip(&leafy_cards) {
            let n = &nodes[li];
            let lc = lobe_center(li);
            let lit = n.light.clamp(0.0, 1.0);
            // Thin a unit's cards, never the unit itself; enlarge the rest.
            let kept = if keep_l < 1.0 {
                let x = want * keep_l;
                (x.floor() + if x.fract() > unit(li, 0x6B) { 1.0 } else { 0.0 }).max(1.0)
            } else {
                want
            };
            let size = size0 * (want / kept).sqrt().min(1.6) * size_of(n);
            let end = n.end(ext);
            let crown_out = (end - center).normalize_or(Vec3::Y);
            let lobe_out = (end - lc).normalize_or(crown_out);
            let outward = (lobe_out * 0.6 + crown_out * 0.4).normalize_or(crown_out);
            let len = (end - n.start).length();
            let mut left = kept as usize;
            let mut k = 0u64;
            while left > 0 {
                let ck = rng::hash(self.seed, S_LEAF, n.id as u64, 1 + k);
                let u = |s: u64| rng::unit(rng::mix(ck ^ s));
                let rv = Vec3::new(u(9) - 0.5, u(10) - 0.5, u(11) - 0.5);
                let (pos, dir, cards, anchor, tile, s) = if k == 0 {
                    // The shoot itself: card base on the shoot, tip beyond it.
                    let dir = (n.dir + outward * 0.25 + rv * 0.3).normalize_or(n.dir);
                    let s = size.min(s_max);
                    let pos = end - dir * len.min(0.55 * s);
                    (pos, dir, if dormant { 3 } else { left.clamp(2, 3) }, 0.08, SHOOT_TILE, s)
                } else {
                    // Lumped twigs: sprays over the unit's voxel, towards
                    // the tip and outward.
                    let along = n.start + n.dir * (len * (1.0 - 0.75 * u(1) * u(1)));
                    let rv2 = Vec3::new(u(2) - 0.5, u(3) - 0.5, u(4) - 0.5) * 2.0;
                    let off = (rv2 + outward * 0.7).normalize_or(outward) * (spread * u(5).sqrt());
                    let dir = (n.dir + outward * 0.6 + rv).normalize_or(n.dir);
                    (along + off, dir, left.min(3), 0.3, SPRAY_TILE, size * (0.8 + 0.4 * u(12)))
                };
                left = left.saturating_sub(cards);
                k += 1;
                let Some(color) = cluster_colour(li, lit, ck, pos, if k == 1 { 1.0 } else { 0.33 }) else { continue };
                clusters.push(LeafCluster { pos, dir, size: s, color, key: ck, center: lc, cards: cards as u8, anchor, tile });
            }
        }
        let spur_scale = (1.0 / keep_s.max(1e-3)).sqrt().min(1.4);
        for (&si, &want) in spurs.iter().zip(&spur_cards) {
            if want <= 0.0 || keep_s <= 0.0 {
                continue;
            }
            let n = &nodes[si];
            let lc = lobe_center(si);
            let lit = spur_light(n).clamp(0.0, 1.0);
            let start = n.start;
            let end = n.end(ext);
            let len = (end - start).length();
            let crown_out = (end - center).normalize_or(Vec3::Y);
            let lobe_out = (end - lc).normalize_or(crown_out);
            let outward = (lobe_out * 0.6 + crown_out * 0.4).normalize_or(crown_out);
            let mut left = want as usize;
            let mut k = 0u64;
            while left > 0 {
                let cards = left.min(3);
                left -= cards;
                let ck = rng::hash(self.seed, S_LEAF ^ 0x5B0, n.id as u64, k);
                k += 1;
                let u = |s: u64| rng::unit(rng::mix(ck ^ s));
                if keep_s < 1.0 && u(30) >= keep_s {
                    continue;
                }
                let along = start + n.dir * (len * (0.1 + 0.9 * u(1)));
                let rv = Vec3::new(u(2) - 0.5, u(3) - 0.5, u(4) - 0.5) * 2.0;
                let off = (rv + outward * 0.5).normalize_or(outward) * (spur_spread * (0.3 + 0.7 * u(5).sqrt()));
                let dir = (outward * 0.6 + rv * 0.5 + n.dir * 0.3 + Vec3::Y * 0.15).normalize_or(outward);
                let pos = along + off;
                let size = s_pref * (0.8 + 0.3 * u(12)) * spur_scale * size_factor;
                let Some(color) = cluster_colour(si, lit, ck, pos, 0.33) else { continue };
                clusters.push(LeafCluster { pos, dir, size, color, key: ck, center: lc, cards: cards as u8, anchor: 0.2, tile: SPRAY_TILE });
            }
        }
        mesh::build_leaves(&clusters, LEAF_TRI_BUDGET)
    }

    /// Flowers (bloom) and fruits, drawn as crossed cards like the leaves
    /// (same conventions as [`Self::leaf_mesh`]); vertex colour alpha is 1.0
    /// on flower cards and 0.5 on fruit cards so a renderer can texture them
    /// differently. Empty when there is nothing to show.
    ///
    /// Flowers sit on short spurs along living, well-lit branch wood at
    /// least `min_wood_age_years` old and thinner than
    /// `max_wood_radius_mm` (never on the stem), scattered within ~0.4 voxel
    /// of the axis (on old trees a shoot unit lumps several years of twigs).
    /// The card area per flowering unit scales with the voxel face
    /// (`flower_density`) and its light, so a crown in full bloom reads as a
    /// white cloud at any size; clusters open and drop progressively. Fruit
    /// clusters are the subset of flower clusters that set fruit, hanging.
    /// At most [`FLOWER_TRI_BUDGET`] triangles.
    pub fn flower_mesh(&self) -> MeshData {
        use crate::math::Vec3;
        let fl_amount = self.flower_density();
        let fr_amount = self.fruit_density();
        if fl_amount <= 0.0 && fr_amount <= 0.0 {
            return MeshData::default();
        }
        let r = &self.genome.reproduction;
        let year = self.year();
        let ext = self.shoots.ext;
        let voxel = self.shoots.voxel;
        let nodes = &self.shoots.nodes;
        let max_r = r.max_wood_radius_mm * 0.001;
        // Light of the part of the crown a branch belongs to: the mean light
        // of the foliage it carries (from the last flush), else its own.
        let branch_light = |n: &crate::growth::Node| if n.leaf_n > 0.0 { n.leaf_q / n.leaf_n } else { n.light };
        let cand: Vec<usize> = (1..nodes.len())
            .filter(|&i| {
                let n = &nodes[i];
                if n.is_snag() || n.order == 0 || branch_light(n) < r.min_light || n.radius() > max_r {
                    return false;
                }
                // A lumped (LOD) unit holds wood up to `unit_years` older
                // than its last extension.
                let wood_age = year.saturating_sub(n.birth_year) + (n.unit_years as u32).saturating_sub(1);
                wood_age >= r.min_wood_age_years
            })
            .collect();
        if cand.is_empty() {
            return MeshData::default();
        }
        let mut center = Vec3::ZERO;
        for &i in &cand {
            center += nodes[i].end(ext);
        }
        center = center * (1.0 / cand.len() as f32);
        center.x = 0.0;
        center.z = 0.0;
        let cards = mesh::CARDS_PER_CLUSTER;
        let n_max = ((FLOWER_TRI_BUDGET as f32 * 0.95) / (2.0 * cards as f32 * cand.len() as f32)).floor().max(1.0) as usize;
        let s_pref = r.flower_cluster_cm * 0.01;
        let s_max = (voxel * 0.8).max(s_pref);
        let spread = (0.4 * voxel).max(0.05);
        let fruit_ratio = r.fruit_cluster_cm / r.flower_cluster_cm.max(0.1);
        let fl_open = self.repro.flower_amount();
        let remaining = self.repro.fruit_remaining(r);
        let swell = self.repro.fruit_swell();
        let set = r.fruit_set * (1.0 - self.repro.frost_loss) * self.repro.crop;
        let fcol = self.flower_color();
        let rcol = self.fruit_color();
        let mut clusters = Vec::new();
        for &i in &cand {
            let n = &nodes[i];
            let lit = branch_light(n).clamp(0.0, 1.0);
            let area = r.flower_density * voxel * voxel * lit;
            let count = ((area / (cards as f32 * s_pref * s_pref)).round() as usize).clamp(1, n_max);
            let size = (area / (cards as f32 * count as f32)).sqrt().clamp(s_pref, s_max);
            let start = n.start;
            let len = (n.end(ext) - start).length();
            for k in 0..count {
                let ck = rng::hash(self.seed, S_FLOWER, n.id as u64, 1 + k as u64);
                let u = |s: u64| rng::unit(rng::mix(ck ^ s));
                let along = start + n.dir * (len * (0.15 + 0.85 * u(1)));
                let rv = Vec3::new(u(2) - 0.5, u(3) - 0.5, u(4) - 0.5) * 2.0;
                let pos = along + rv.normalize_or(Vec3::Y) * (spread * u(5).sqrt());
                let v = 0.9 + 0.15 * u(6);
                // Flowers open (and drop) cluster by cluster.
                if fl_amount > 0.0 && u(7) < fl_open {
                    let dir = (n.dir + Vec3::new(u(8) - 0.5, u(9) - 0.5, u(10) - 0.5)).normalize_or(n.dir);
                    let col = [(fcol[0] * v).min(1.0), (fcol[1] * v).min(1.0), (fcol[2] * v).min(1.0), 1.0];
                    clusters.push(LeafCluster { pos, dir, size: size * (0.8 + 0.4 * u(11)), color: col, key: ck, center, cards: 3, anchor: 0.25, tile: [0.0, 1.0] });
                }
                // Fruits: the clusters that set fruit, hanging on pedicels.
                if fr_amount > 0.0 && u(12) < set && u(13) < remaining {
                    let dir = (Vec3::new(0.0, -1.0, 0.0) + Vec3::new(u(14) - 0.5, u(15) - 0.5, u(16) - 0.5) * 0.8).normalize_or(-Vec3::Y);
                    let col = [(rcol[0] * v).min(1.0), (rcol[1] * v).min(1.0), (rcol[2] * v).min(1.0), 0.5];
                    let s = size * fruit_ratio * (0.4 + 0.6 * swell) * (0.8 + 0.4 * u(17));
                    clusters.push(LeafCluster { pos, dir, size: s, color: col, key: ck ^ 0xF4E1, center, cards: 3, anchor: 0.25, tile: [0.0, 1.0] });
                }
            }
        }
        mesh::build_leaves(&clusters, FLOWER_TRI_BUDGET)
    }

}

// Read-only view of the internal state for `crate::flux` (a sibling module
// cannot read these private fields; flux.rs owns the logic).
impl Simulation {
    pub(crate) fn flux_view(&self) -> crate::flux::FluxView<'_> {
        crate::flux::FluxView {
            soil: &self.soil,
            phen: &self.phen,
            shoots: &self.shoots,
            roots: &self.roots,
            water_factor: self.water_factor,
            fertility: self.fertility,
        }
    }
}

