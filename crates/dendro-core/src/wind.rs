//! Wind against the tree, day after day (milestone M1, second step):
//! breakage, uprooting, storms on demand, acclimation of the stem.
//!
//! * **Daily wind, no damage.** Every simulated day brings a mean wind
//!   and a maximum gust at 10 m (`crate::climate`, calibrated on Rennes).
//!   They feed the reference winds of the acclimation (below), the sway of
//!   the front end and the display; they never break nor uproot anything
//!   (user decision, 2026-11-19: the sustained daily test made young
//!   conifers and many broadleaves fail far more often than published storm
//!   damage, see docs/sources.md, « Écarts connus »).
//! * **Storms asked for** (`Simulation::storm`, `Action::Storm`, logged and
//!   replayed, save format v2). The full mechanics (`crate::mechanics`:
//!   weight, crown drag with reconfiguration, dynamic factor of the first
//!   sway mode, large deflections of the stem) is evaluated at the storm's
//!   gust with today's foliage, and what fails fails:
//!   1. the root plate turns over (uprooting, « chablis »): the trunk dies
//!      at once and lies on the ground (`Simulation::uproot`, cause
//!      `DeathCause::Uprooted`); the stump may resprout with the species'
//!      probability. This is the **only** death the wind causes outright;
//!   2. else the stem snaps at its lowest failing section. This is **not a
//!      death**: through the pruning machinery (`Simulation::wind_break`,
//!      `CutKind::WindBreak`) the stump becomes a « chandelle » whose latent
//!      buds below the break sprout and rebuild a crown by reiteration, as
//!      after a topping; a break near the collar (below 0.6 m) resprouts
//!      like a coppice stool. « Tronc cassé » when the lost part carries
//!      most of the crown ([`BOLE_FOLIAGE_SHARE`]), else « cime cassée »;
//!   3. else every failing limb (structural branch, see
//!      `mechanics::BREAK_MIN_DIAMETER_M`) breaks at its most proximal
//!      failing section.
//!
//!   After each break the loads are recomputed (a lost limb unloads the
//!   stem), up to [`MAX_BREAK_ROUNDS`] times. A later decline can only come
//!   from the existing mechanisms (reserves, decay of the torn wound).
//! * **Acclimation** (thigmomorphogenesis). Once a year, at the end of the
//!   leaf expansion, the bending stress along the stem under the tree's
//!   reference winds is computed; sections above the stress of the stem at
//!   breast height (1.3 m) under the same wind lay down the extra wood that
//!   brings them back to it, at the next two thickenings (`growth.rs`,
//!   `adaptive_demand`): the uniform stress hypothesis for stems above the
//!   butt swell (Morgan & Cannell 1994, Tree Physiol 14:49-62, abstract
//!   read; Mattheck & Burkhardt 1990, not read). The set-point is the
//!   tree's own stress at 1.3 m, not a stress value in MPa (none was
//!   found). Reference winds: the strongest gust met each year in leaf and
//!   leafless, smoothed over [`ACCLIMATION_MEMORY_YEARS`] (estimated
//!   memory; trees respond to the strongest wind strains: Telewski 2006, Am
//!   J Bot 93:1466-1476; Moulia, Coutand & Julien 2015, Front Plant Sci
//!   6:52; Bonnesoeur et al. 2016, New Phytol 210:850-860; reviews /
//!   abstract, not read in full). Limbs keep the self-weight set-point of
//!   `adaptive_stress_mpa` only (no sourced wind set-point for limbs).

use crate::mechanics::{SegmentRole, WindLoad};
use crate::mesh::MeshData;
use crate::pruning::{Action, LogEvent};
use crate::sim::{Simulation, DAYS_PER_YEAR};

/// At most this many break / reload rounds per gust.
pub const MAX_BREAK_ROUNDS: usize = 4;
/// Memory of the acclimation reference winds (years, exponential smoothing
/// of the yearly maxima). Estimated.
pub const ACCLIMATION_MEMORY_YEARS: f32 = 5.0;
/// A day counts as "in leaf" for the reference winds above this leaf
/// fraction.
const LEAFY_DAY: f32 = 0.5;
/// The acclimation screen of big trees runs every `1 + segments /
/// BIG_TREE_SEGMENTS` years (cost); the demand is rescaled to the current
/// radii in between.
pub const BIG_TREE_SEGMENTS: usize = 2_000;
/// A stem break is a broken bole (« tronc cassé ») when the part lost
/// carries at least this share of the crown's foliage; below, it is a lost
/// top (« cime cassée »): the living branches left below the break take
/// over (estimated: three quarters).
pub const BOLE_FOLIAGE_SHARE: f32 = 0.75;
/// Torn wound radius / broken section radius (estimated).
pub const TORN_WOUND_FACTOR: f32 = 1.5;
/// Height of the stress reference of the stem acclimation (m): breast
/// height, where the stem is above the butt swell (Morgan & Cannell 1994).
pub const ACCLIMATION_REFERENCE_HEIGHT_M: f32 = 1.3;

/// What the wind did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageKind {
    /// The stem (bole) snapped, taking most of the crown with it (see
    /// [`BOLE_FOLIAGE_SHARE`]). Not a death: the stump (« chandelle »)
    /// resprouts from its latent buds, like a topped or coppiced stem.
    Stem,
    /// The top of the stem snapped: the branches below remain and one of
    /// them takes over as leader.
    Top,
    /// A structural limb broke (diameter >= 20 % DBH and >= 5 cm).
    Limb,
    /// The root plate turned over: the trunk is down and dead.
    Uprooted,
}

impl DamageKind {
    pub fn code(self) -> u8 {
        self as u8
    }
    pub fn label_fr(self) -> &'static str {
        match self {
            DamageKind::Stem => "tronc cassé, rejets",
            DamageKind::Top => "cime cassée",
            DamageKind::Limb => "charpentière cassée",
            DamageKind::Uprooted => "arbre déraciné",
        }
    }
}

/// One entry of the wind history.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindDamage {
    /// Day of the gust (days since germination).
    pub day: u64,
    /// Gust at 10 m (m/s).
    pub gust_ms: f32,
    /// A storm asked for by the player (else the day's weather).
    pub storm: bool,
    pub kind: DamageKind,
    /// Stable id of the broken node (the collar's for an uprooting).
    pub node_id: u32,
    /// Height of the break (m) and diameter of the broken section (cm;
    /// collar diameter for an uprooting).
    pub height_m: f32,
    pub diameter_cm: f32,
    /// Leaf fraction of the crown that day.
    pub leaf_fraction: f32,
}

/// A part torn off by the wind, kept for the front end (falling branch).
#[derive(Clone, Debug)]
pub struct WindDebris {
    pub day: u64,
    /// Break point (tree coordinates).
    pub pivot: [f32; 3],
    /// The lost wood (same conventions as the wood mesh).
    pub mesh: MeshData,
    pub leafy_shoots: u32,
}

/// Wind state of the tree (part of the state hash).
#[derive(Clone, Debug, Default)]
pub(crate) struct WindState {
    pub(crate) history: Vec<WindDamage>,
    /// Day the tree was uprooted, and the direction it fell towards.
    pub(crate) fallen_day: Option<u64>,
    pub(crate) fallen_azimuth_deg: f32,
    year_max_leaf: f32,
    year_max_bare: f32,
    /// Acclimation reference gusts (m/s), in leaf and leafless.
    pub(crate) ref_leaf: f32,
    pub(crate) ref_bare: f32,
}

/// Derived data and front-end buffers, not part of the state hash: the
/// acclimation demand, the debris of the last breaks.
#[derive(Clone, Debug, Default)]
pub(crate) struct WindCache {
    /// Acclimation: (node id, stress / set-point, radius m when computed),
    /// by id.
    accl: Vec<(u32, f32, f32)>,
    /// Year of the last acclimation demand computed (None: never).
    accl_year: Option<u32>,
    /// Keep the meshes of broken parts (front end only).
    pub(crate) record_debris: bool,
    pub(crate) debris: Vec<WindDebris>,
    /// Mechanics evaluations run by storms and acclimation screens
    /// computed (diagnostics).
    pub(crate) evaluations: u64,
    pub(crate) screens: u64,
    /// Diagnostics: no acclimation to the wind.
    pub(crate) acclimation_off: bool,
}

impl Simulation {
    /// Today's wind: (daily mean at 10 m, maximum gust at 10 m, both m/s;
    /// direction the wind blows towards, deg). The weather of the last
    /// simulated day.
    pub fn wind_today(&self) -> (f32, f32, f32) {
        (self.today.wind_mean_ms, self.today.gust_ms, self.today.wind_azimuth_deg)
    }

    /// The wind history (breaks and uprooting), oldest first.
    pub fn wind_history(&self) -> &[WindDamage] {
        &self.wind.history
    }

    /// Day the tree was uprooted (None while standing), and the direction
    /// it fell towards (deg, `WindLoad` azimuth).
    pub fn uprooted(&self) -> Option<(u64, f32)> {
        self.wind.fallen_day.map(|d| (d, self.wind.fallen_azimuth_deg))
    }

    /// Acclimation reference gusts (m/s at 10 m): (in leaf, leafless).
    pub fn wind_reference_gusts(&self) -> (f32, f32) {
        (self.wind.ref_leaf, self.wind.ref_bare)
    }

    /// Diagnostics: (mechanics evaluations run by storms, acclimation
    /// screens).
    pub fn wind_costs(&self) -> (u64, u64) {
        (self.wind_cache.evaluations, self.wind_cache.screens)
    }

    /// Front end: keep the meshes of the parts the wind breaks off (see
    /// [`Self::take_wind_debris`]). Off by default (costs a mesh per break).
    pub fn set_record_wind_debris(&mut self, on: bool) {
        self.wind_cache.record_debris = on;
        if !on {
            self.wind_cache.debris.clear();
        }
    }

    /// Diagnostics: switch the stem acclimation to the wind off (the
    /// growth of the simulation has it on).
    pub fn set_wind_acclimation_enabled(&mut self, on: bool) {
        self.wind_cache.acclimation_off = !on;
        if !on {
            self.wind_cache.accl.clear();
        }
    }

    /// The parts broken off since the last call (oldest first).
    pub fn take_wind_debris(&mut self) -> Vec<WindDebris> {
        std::mem::take(&mut self.wind_cache.debris)
    }

    /// A storm now: a gust of `gust_ms` (m/s at 10 m) from the prevailing
    /// direction tests the tree at once, with today's foliage, and is logged
    /// (replayed at the start of this day). Later logged events (after a
    /// rewind) are dropped, as for a cut. Returns the damage it caused.
    pub fn storm(&mut self, gust_ms: f32) -> Vec<WindDamage> {
        let gust = gust_ms.clamp(0.0, crate::mechanics::MAX_SEARCH_SPEED_MS);
        let before = self.wind.history.len();
        self.apply_storm(gust);
        self.prune.log.truncate(self.prune.cursor);
        self.prune.log.push(LogEvent { day: self.day, action: Action::Storm(gust) });
        self.prune.cursor = self.prune.log.len();
        self.wind.history[before..].to_vec()
    }

    /// A logged storm, applied at the start of its day.
    pub(crate) fn apply_storm(&mut self, gust: f32) {
        if self.wind.fallen_day.is_some() || !self.is_alive() {
            return;
        }
        self.note_gust(gust);
        let az = self.climate_params().wind_towards_deg;
        self.wind_event(gust, az, true);
    }

    fn note_gust(&mut self, gust: f32) {
        if self.leaf_density() > LEAFY_DAY {
            self.wind.year_max_leaf = self.wind.year_max_leaf.max(gust);
        } else {
            self.wind.year_max_bare = self.wind.year_max_bare.max(gust);
        }
    }

    /// New year: the reference winds take in last year's maxima.
    pub(crate) fn wind_new_year(&mut self) {
        let w = &mut self.wind;
        let k = 1.0 / ACCLIMATION_MEMORY_YEARS;
        for (r, m) in [(&mut w.ref_leaf, w.year_max_leaf), (&mut w.ref_bare, w.year_max_bare)] {
            if m > 0.0 {
                *r = if *r <= 0.0 { m } else { *r + (m - *r) * k };
            }
        }
        w.year_max_leaf = 0.0;
        w.year_max_bare = 0.0;
    }

    /// The day's wind (called after the day's phenology): it only feeds
    /// the reference winds of the acclimation (no damage, module header).
    pub(crate) fn wind_day(&mut self) {
        let gust = self.today.gust_ms;
        self.note_gust(gust);
    }

    /// Acclimation demand of the current tree at the reference winds: per
    /// node, the stem's stress / stress at 1.3 m (the larger of the two
    /// foliage states; 0 off the stem and below 1.3 m).
    fn acclimation_demand(&mut self) -> Vec<f32> {
        self.wind_cache.screens += 1;
        let (segments, site) = self.mechanics_inputs_at(1.0);
        let heights: Vec<f32> = segments.iter().map(|s| s.start[1]).collect();
        let model = crate::mechanics::MechanicsModel::from_segments(&segments, self.genome.mechanics.clone(), site);
        let az = self.climate_params().wind_towards_deg;
        // Quasi-static loads (cost): the set-point is a stress ratio along
        // the stem, in which the dynamic factor (one scalar for the whole
        // tree) cancels; the large deflections matter for slender leaders,
        // not for the structural stem sections that respond.
        let case = |leaf: f32, u: f32| WindLoad { speed_10m_ms: u, azimuth_deg: az, leaf_fraction: Some(leaf), quasi_static: true, ..WindLoad::default() };
        // The leafless case keeps the dead leaves of a young marcescent
        // tree and the needles of an evergreen (its winter crown).
        let l = &self.genome.leaf;
        let juvenile = (1.0 - self.age_years() / l.marcescence_max_age_years.max(1.0)).clamp(0.0, 1.0);
        let winter = if l.deciduous { (l.marcescence * juvenile).clamp(0.0, 1.0) } else { self.genome.leaf.retained_share().clamp(0.0, 1.0) };
        let n = segments.len();
        let none = || (vec![0.0; n], Vec::new());
        let (s_leaf, t_leaf) = if self.wind.ref_leaf > 0.0 { model.stem_stresses(&case(1.0, self.wind.ref_leaf)) } else { none() };
        let (s_bare, t_bare) = if self.wind.ref_bare > 0.0 { model.stem_stresses(&case(winter, self.wind.ref_bare)) } else { none() };
        let trunk = if t_leaf.is_empty() { t_bare } else { t_leaf };
        // Uniform stress along the stem (module header): stress of each stem
        // section over the stress of the stem at 1.3 m, under the same wind.
        let reference = |s: &[f32]| {
            trunk
                .iter()
                .zip(&heights)
                .zip(s)
                .filter(|((&t, &h), _)| t && h <= ACCLIMATION_REFERENCE_HEIGHT_M)
                .map(|(_, &x)| x)
                .next_back()
                .unwrap_or(0.0)
        };
        let (ref_leaf, ref_bare) = (reference(&s_leaf), reference(&s_bare));
        // The stem path above the reference height (butt swell below).
        // Tried on every bending axis from 2.5 cm (limbs and crown
        // branches too, no source for that extension): the wood taken from
        // the stem lowered the anchorage (Creg x stem mass) and 9 oaks out
        // of 10 were uprooted within 100 years under the former daily
        // damage, against 4 with the stem only (docs/sources.md).
        // Only the structural stem: order-0 sections of that path (the
        // leader axis) at least max(5 cm, 20 % of the DBH) across (the
        // structural threshold of `crate::mechanics`). In a decurrent crown
        // the path follows the dominant limb, and the thin leader inside it
        // is crown wood: acclimating them as the stem kept the stem path
        // dominant higher up (12-13 scaffold limbs on the 100- and 200-year
        // oak of `oak_architecture`, band 3-10; 9-10 without).
        let dmin = crate::mechanics::LIMB_MIN_DIAMETER_M.max(crate::mechanics::LIMB_DBH_FRACTION * 0.01 * self.dbh_cm());
        let order0: Vec<bool> = self.shoots.nodes.iter().map(|n| n.order == 0 && 2.0 * n.radius() >= dmin).collect();
        trunk
            .iter()
            .zip(&order0)
            .map(|(&t, &o)| t && o)
            .zip(&heights)
            .zip(s_leaf.iter().zip(&s_bare))
            .map(|((t, &h), (&a, &b))| {
                if !t || h <= ACCLIMATION_REFERENCE_HEIGHT_M {
                    return 0.0;
                }
                let x = if ref_leaf > 0.0 { a / ref_leaf } else { 0.0 };
                let y = if ref_bare > 0.0 { b / ref_bare } else { 0.0 };
                x.max(y)
            })
            .collect()
    }

    /// Once a year at the end of leaf expansion, after the earlywood: the
    /// acclimation demand applied at the next thickenings (the latewood of
    /// this year, the earlywood of the next: trees respond to the winds
    /// they have met).
    pub(crate) fn wind_yearly_screen(&mut self) {
        let on = !self.wind_cache.acclimation_off && (self.wind.ref_leaf > 0.0 || self.wind.ref_bare > 0.0) && self.dbh_cm() > 0.0;
        if !on || self.wind.fallen_day.is_some() || !self.is_alive() {
            self.wind_cache.accl.clear();
            return;
        }
        // Big trees (cost): every `1 + segments / BIG_TREE_SEGMENTS` years,
        // the demand being rescaled to the current radii in between
        // (`wind_stress_now`).
        let year = self.year();
        let every = 1 + (self.shoots.nodes.len() / BIG_TREE_SEGMENTS) as u32;
        if self.wind_cache.accl_year.is_some_and(|y| year < y + every) {
            return;
        }
        self.wind_cache.accl_year = Some(year);
        let demand = self.acclimation_demand();
        let mut accl: Vec<(u32, f32, f32)> = self.shoots.nodes.iter().zip(&demand).filter(|(_, &s)| s > 1.0).map(|(n, &s)| (n.id, s, n.radius())).collect();
        accl.sort_unstable_by_key(|a| a.0);
        self.wind_cache.accl = accl;
    }

    /// The acclimation demand (stress / set-point) of the last update, mapped
    /// on today's nodes (by id) and rescaled to their current radius (`σ ∝
    /// 1 / r³` at a given moment). Empty when there is none.
    pub(crate) fn wind_stress_now(&self) -> Vec<f32> {
        let a = &self.wind_cache.accl;
        if a.is_empty() {
            return Vec::new();
        }
        self.shoots
            .nodes
            .iter()
            .map(|n| match a.binary_search_by_key(&n.id, |e| e.0) {
                Ok(k) => {
                    let (_, s, r0) = a[k];
                    let q = r0 / n.radius().max(1e-6);
                    s * q * q * q
                }
                Err(_) => 0.0,
            })
            .collect()
    }

    /// Test the tree against a storm's gust and apply what fails (module
    /// header).
    /// Returns whether anything failed.
    fn wind_event(&mut self, gust: f32, azimuth: f32, storm: bool) -> bool {
        self.wind_cache.evaluations += 1;
        let mut any = false;
        for _ in 0..MAX_BREAK_ROUNDS {
            let model = self.mechanics_model();
            let leaf = self.leaf_density().clamp(0.0, 1.0);
            let load = WindLoad { speed_10m_ms: gust, azimuth_deg: azimuth, ..WindLoad::default() };
            let snap = model.evaluate(&load);
            let nodes = &self.shoots.nodes;
            if model.can_uproot() && snap.anchorage_safety < 1.0 {
                let d = 200.0 * nodes[0].radius();
                self.wind.history.push(WindDamage { day: self.day, gust_ms: gust, storm, kind: DamageKind::Uprooted, node_id: nodes[0].id, height_m: 0.0, diameter_cm: d, leaf_fraction: leaf });
                self.wind.fallen_day = Some(self.day);
                self.wind.fallen_azimuth_deg = azimuth;
                let year = self.year();
                self.uproot(year, azimuth);
                return true;
            }
            let n = nodes.len();
            let failing: Vec<bool> = snap.segment_breakable.iter().zip(&snap.segment_safety).map(|(&b, &s)| b && s < 1.0).collect();
            // 1. The stem: lowest failing stem section.
            let stem = (0..n).find(|&i| failing[i] && snap.segment_role[i] == SegmentRole::Trunk);
            let mut targets: Vec<(u32, DamageKind)> = Vec::new();
            if let Some(i) = stem {
                // A broken bole when the lost part carries most of the crown;
                // else the top is lost and a lateral below takes over as
                // leader (estimated share, see BOLE_FOLIAGE_SHARE).
                let total = nodes[0].foliage.max(1e-9);
                let bole = nodes[i].foliage / total >= BOLE_FOLIAGE_SHARE;
                targets.push((nodes[i].id, if bole { DamageKind::Stem } else { DamageKind::Top }));
            } else {
                // 2. Limbs: the most proximal failing section of each path.
                let mut below = vec![false; n];
                for i in 1..n {
                    let p = nodes[i].parent as usize;
                    below[i] = below[p] || failing[p];
                    if failing[i] && !below[i] {
                        targets.push((nodes[i].id, DamageKind::Limb));
                    }
                }
            }
            if targets.is_empty() {
                return any;
            }
            any = true;
            for (id, kind) in targets {
                let Some(idx) = self.shoots.index_of(id) else { continue };
                let (h, d) = self.wind_break(idx);
                self.wind.history.push(WindDamage { day: self.day, gust_ms: gust, storm, kind, node_id: id, height_m: h, diameter_cm: d, leaf_fraction: leaf });
            }
            if !self.is_alive() {
                return any;
            }
        }
        any
    }

    /// Days per year (for front ends composing storm days).
    pub fn wind_days_per_year() -> u64 {
        DAYS_PER_YEAR as u64
    }
}
