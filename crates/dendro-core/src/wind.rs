//! Wind against the tree, day after day (milestone M1, second step):
//! breakage, uprooting, storms on demand, acclimation of the stem.
//!
//! * **Daily test.** Every simulated day brings a maximum gust at 10 m
//!   (`crate::climate`, calibrated on Rennes). When it may endanger the
//!   tree, the full mechanics (`crate::mechanics`: weight, crown drag with
//!   reconfiguration, dynamic factor of the first sway mode, large
//!   deflections of the stem) is evaluated at that gust with today's
//!   foliage, and what fails fails:
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
//! * **Cost.** The mechanics is only evaluated on days whose gust reaches a
//!   lower bound of the tree's critical gust: the screen of
//!   `MechanicsModel::screen` (leafless and leafy crown, recomputed once a
//!   year, after a break or a cut, and whenever growth changes the set of
//!   breakable sections), tightened to the exact first critical gust once
//!   an evaluation found the tree safe (valid until the structure, the
//!   foliage or the year changes; saturated soil assumed). The bounds only
//!   skip evaluations that cannot break anything: a test checks that the
//!   tree is the same as when testing every day.
//! * **Determinism and replay.** Natural breaks are recomputed on replay
//!   (they follow from the seed and the log); only the player's storms are
//!   logged (`Action::Storm`), applied at the start of their day.
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

use crate::mechanics::{SegmentRole, WindLoad, BREAK_MIN_DIAMETER_M};
use crate::mesh::MeshData;
use crate::pruning::{Action, LogEvent};
use crate::sim::{Simulation, DAYS_PER_YEAR};

/// Gusts below this (m/s) never trigger an evaluation (calm days cost
/// nothing; far below any critical gust of the simulated trees).
pub const MIN_SCREEN_GUST_MS: f32 = 6.0;
/// The screened lower bound is used with this margin (it covers the growth
/// of the crown until the next screen).
pub const SCREEN_MARGIN: f32 = 0.97;
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
pub const BIG_TREE_SEGMENTS: usize = 8_000;
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
/// Floor of the reference speeds of the screen (m/s): keeps the bound
/// tight for a young tree that has met only light winds (the bound is
/// valid at any reference speed).
const SCREEN_MIN_SPEED_MS: f32 = 15.0;
/// Tolerance of the exact first critical gust used as the tight guard (m/s).
const TIGHT_TOLERANCE_MS: f32 = 0.25;

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

/// Exact first critical gust of the tree as it stood when an evaluation
/// found it safe: valid while nothing that changes the loads or the
/// strength changes.
#[derive(Clone, Copy, Debug)]
struct Tight {
    year: u32,
    wood_version: u64,
    root_version: u64,
    leaf: f32,
    gust: f32,
}

/// Derived data and front-end buffers, not part of the state hash: the
/// screen (a pure function of the state), the acclimation demand, the
/// debris of the last breaks.
#[derive(Clone, Debug, Default)]
pub(crate) struct WindCache {
    /// Lower bounds of the critical gust (m/s), (in leaf, leafless);
    /// `None` = to recompute; `Some(INFINITY)` = nothing can fail.
    pub(crate) guard: Option<(f32, f32)>,
    tight: Option<Tight>,
    /// Acclimation: (node id, stress / set-point, radius m when computed),
    /// by id.
    accl: Vec<(u32, f32, f32)>,
    /// Test switch: evaluate the mechanics on every day (no screen).
    pub(crate) always_evaluate: bool,
    /// Keep the meshes of broken parts (front end only).
    pub(crate) record_debris: bool,
    pub(crate) debris: Vec<WindDebris>,
    /// Mechanics evaluations run by the daily test, screens and exact
    /// critical-gust searches computed (diagnostics).
    pub(crate) evaluations: u64,
    pub(crate) screens: u64,
    pub(crate) searches: u64,
    /// Signature of the set of breakable sections when the guard was
    /// computed (see `Simulation::wind_structure_changed`).
    structure: u64,
    /// Diagnostics: the daily test is skipped (no natural damage; storms
    /// and acclimation still act). Not part of the state: tools only.
    pub(crate) damage_off: bool,
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

    /// The screened lower bounds of the critical gust (in leaf, leafless;
    /// m/s), if computed.
    pub fn wind_screen(&self) -> Option<(f32, f32)> {
        self.wind_cache.guard
    }

    /// Diagnostics: (full evaluations run by the daily test, screens,
    /// exact critical-gust searches).
    pub fn wind_costs(&self) -> (u64, u64, u64) {
        (self.wind_cache.evaluations, self.wind_cache.screens, self.wind_cache.searches)
    }

    /// Test / diagnostics: evaluate the mechanics every day with a gust of
    /// at least [`MIN_SCREEN_GUST_MS`] instead of screening (same results,
    /// much slower).
    pub fn set_wind_always_evaluate(&mut self, on: bool) {
        self.wind_cache.always_evaluate = on;
    }

    /// Front end: keep the meshes of the parts the wind breaks off (see
    /// [`Self::take_wind_debris`]). Off by default (costs a mesh per break).
    pub fn set_record_wind_debris(&mut self, on: bool) {
        self.wind_cache.record_debris = on;
        if !on {
            self.wind_cache.debris.clear();
        }
    }

    /// Diagnostics (examples, calibration, growth tests): switch the daily
    /// wind test off (no natural breakage or uprooting; storms asked for
    /// and the acclimation still act). A tree grown this way is not the
    /// tree of the simulation.
    pub fn set_wind_damage_enabled(&mut self, on: bool) {
        self.wind_cache.damage_off = !on;
    }

    /// [`Self::set_wind_damage_enabled`]`(false)` as a builder.
    pub fn without_wind_damage(mut self) -> Self {
        self.set_wind_damage_enabled(false);
        self
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

    /// The daily wind test (called after the day's phenology).
    pub(crate) fn wind_day(&mut self) {
        let gust = self.today.gust_ms;
        let az = self.today.wind_azimuth_deg;
        self.note_gust(gust);
        // An uprooted tree is no longer tested (its stump sprouts grow
        // among the lying stem; their loads are not modelled).
        if self.wind.fallen_day.is_some() || !self.is_alive() || self.wind_cache.damage_off {
            return;
        }
        if 2.0 * self.shoots.nodes[0].radius() < BREAK_MIN_DIAMETER_M || gust < MIN_SCREEN_GUST_MS {
            // Nothing can break or overturn yet / a calm day.
            return;
        }
        let leaf = self.leaf_density().clamp(0.0, 1.0);
        let year = self.year();
        if !self.wind_cache.always_evaluate {
            if let Some(t) = self.wind_cache.tight {
                if t.year == year && t.wood_version == self.wood_version && t.root_version == self.root_version && t.leaf == leaf && gust < t.gust {
                    return;
                }
            }
            let (lb_leaf, lb_bare) = match self.wind_cache.guard {
                Some(g) => g,
                None => {
                    let g = self.compute_guard();
                    self.wind_cache.guard = Some(g);
                    self.wind_cache.structure = self.breakable_signature();
                    g
                }
            };
            let lb = if leaf >= 0.99 {
                lb_leaf
            } else if leaf <= 0.01 {
                lb_bare
            } else {
                lb_leaf.min(lb_bare)
            };
            if gust < SCREEN_MARGIN * lb {
                return;
            }
        }
        let broke = self.wind_event(gust, az, false);
        if !broke && !self.wind_cache.always_evaluate && self.is_alive() {
            // Safe today: the exact first critical gust of this tree and
            // foliage (soil assumed saturated, the weakest anchorage) is
            // the guard until something changes.
            self.wind_cache.searches += 1;
            let (segments, mut site) = self.mechanics_inputs();
            site.saturated_root_fraction = 1.0;
            let model = crate::mechanics::MechanicsModel::from_segments(&segments, self.genome.mechanics.clone(), site);
            let load = WindLoad { azimuth_deg: az, leaf_fraction: Some(leaf), ..WindLoad::default() };
            let crit = model.first_safe_speed(&load, TIGHT_TOLERANCE_MS);
            self.wind_cache.tight = Some(Tight { year, wood_version: self.wood_version, root_version: self.root_version, leaf, gust: crit.max(gust) });
        }
    }

    /// Signature (FNV-1a over node ids) of the sections the mechanics may
    /// break today: live, leafless, at least max(5 cm, 20 % DBH) across, or
    /// on the stem from `STEM_BREAK_MIN_DIAMETER_M`
    /// (`mechanics::MechanicsModel::from_segments`). O(n), no mechanics.
    fn breakable_signature(&self) -> u64 {
        let nodes = &self.shoots.nodes;
        let n = nodes.len();
        // Stem path: from the collar along the thickest child (as the mechanics).
        let mut thickest = vec![usize::MAX; n];
        for (i, nd) in nodes.iter().enumerate().skip(1) {
            let p = nd.parent as usize;
            if thickest[p] == usize::MAX || nd.radius() > nodes[thickest[p]].radius() {
                thickest[p] = i;
            }
        }
        let mut on = vec![false; n];
        let mut c = 0;
        on[0] = true;
        while thickest[c] != usize::MAX {
            c = thickest[c];
            on[c] = true;
        }
        let thr = crate::mechanics::LIMB_MIN_DIAMETER_M.max(crate::mechanics::LIMB_DBH_FRACTION * 0.01 * self.dbh_cm());
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for (nd, &t) in nodes.iter().zip(&on) {
            let d = 2.0 * nd.radius();
            if !nd.is_snag() && !nd.leafy && (d >= thr || (t && d >= crate::mechanics::STEM_BREAK_MIN_DIAMETER_M)) {
                h ^= nd.id as u64;
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
        }
        h
    }

    /// After growth (flush, thickening): a section that became breakable is
    /// not covered by the screen, which is then dropped (recomputed on the
    /// next windy day).
    pub(crate) fn wind_structure_changed(&mut self) {
        if self.wind_cache.guard.is_some() && self.breakable_signature() != self.wind_cache.structure {
            self.wind_cache.guard = None;
        }
    }

    /// The guard alone (no acclimation demand: one pass less).
    fn compute_guard(&mut self) -> (f32, f32) {
        self.compute_screen_with(false).1
    }

    /// Screen of the current tree at the reference winds: per-node stem
    /// acclimation demand (stress / stress at 1.3 m, the larger of the two
    /// foliage states; 0 off the stem) and the lower bounds of the
    /// critical gust (in leaf, leafless).
    fn compute_screen_with(&mut self, demand: bool) -> (Vec<f32>, (f32, f32)) {
        self.wind_cache.screens += 1;
        // This year's shoots at full extension: the bound then holds while
        // they lengthen (no new screen at each extension step).
        let (segments, mut site) = self.mechanics_inputs_at(1.0);
        // Weakest anchorage (saturated soil): the bound holds whatever the
        // soil water of the days to come.
        site.saturated_root_fraction = 1.0;
        let heights: Vec<f32> = segments.iter().map(|s| s.start[1]).collect();
        let model = crate::mechanics::MechanicsModel::from_segments(&segments, self.genome.mechanics.clone(), site);
        let az = self.climate_params().wind_towards_deg;
        let u_leaf = self.wind.ref_leaf.max(SCREEN_MIN_SPEED_MS);
        let u_bare = self.wind.ref_bare.max(SCREEN_MIN_SPEED_MS);
        let case = |leaf: f32, u: f32| WindLoad { speed_10m_ms: u, azimuth_deg: az, leaf_fraction: Some(leaf), ..WindLoad::default() };
        // The leafless case keeps the dead leaves of a young marcescent
        // tree and the needles of an evergreen (its winter crown).
        let l = &self.genome.leaf;
        let juvenile = (1.0 - self.age_years() / l.marcescence_max_age_years.max(1.0)).clamp(0.0, 1.0);
        let winter = if l.deciduous { (l.marcescence * juvenile).clamp(0.0, 1.0) } else { self.genome.leaf.retained_share().clamp(0.0, 1.0) };
        let (s_leaf, trunk, lb_leaf) = model.screen_with(&case(1.0, u_leaf), demand);
        let (s_bare, _, lb_bare) = model.screen_with(&case(winter, u_bare), demand);
        let guard = (lb_leaf.unwrap_or(f32::INFINITY), lb_bare.unwrap_or(f32::INFINITY));
        if !demand {
            return (Vec::new(), guard);
        }
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
        let have_leaf = self.wind.ref_leaf > 0.0 && ref_leaf > 0.0;
        let have_bare = self.wind.ref_bare > 0.0 && ref_bare > 0.0;
        // The stem path above the reference height (butt swell below).
        // Tried on every bending axis from 2.5 cm (limbs and crown
        // branches too, no source for that extension): the wood taken from
        // the stem lowered the anchorage (Creg x stem mass) and 9 oaks out
        // of 10 were uprooted within 100 years, against 4 with the stem
        // only (docs/sources.md).
        let ratio: Vec<f32> = trunk
            .iter()
            .zip(&heights)
            .zip(s_leaf.iter().zip(&s_bare))
            .map(|((&t, &h), (&a, &b))| {
                if !t || h <= ACCLIMATION_REFERENCE_HEIGHT_M {
                    return 0.0;
                }
                let x = if have_leaf { a / ref_leaf } else { 0.0 };
                let y = if have_bare { b / ref_bare } else { 0.0 };
                x.max(y)
            })
            .collect();
        (ratio, guard)
    }

    /// Once a year at the end of leaf expansion, after the earlywood: the
    /// screen of the tree as it now stands (the guard of the year to come)
    /// and the acclimation demand applied at the next thickenings (the
    /// latewood of this year, the earlywood of the next: trees respond to
    /// the winds they have met).
    pub(crate) fn wind_yearly_screen(&mut self) {
        self.wind_cache.tight = None;
        if self.wind.fallen_day.is_some() || !self.is_alive() {
            self.wind_cache.accl.clear();
            return;
        }
        let year = self.year();
        let collar_ok = 2.0 * self.shoots.nodes[0].radius() >= BREAK_MIN_DIAMETER_M && !self.wind_cache.damage_off;
        let on = !self.wind_cache.acclimation_off && (self.wind.ref_leaf > 0.0 || self.wind.ref_bare > 0.0) && self.dbh_cm() > 0.0;
        // Big trees (cost): the demand every `1 + segments / BIG_TREE_SEGMENTS`
        // years, rescaled to the current radii in between
        // (`wind_stress_now`); the guard is recomputed on the next windy
        // day. Independent of the guard, which the daily test may have
        // dropped: growth must not depend on how the screen was used.
        let every = 1 + (self.shoots.nodes.len() / BIG_TREE_SEGMENTS) as u32;
        if on && every > 1 && !self.wind_cache.accl.is_empty() && !year.is_multiple_of(every) {
            self.wind_cache.guard = None;
            return;
        }
        self.wind_cache.accl.clear();
        if !on && !collar_ok {
            return;
        }
        let (demand, guard) = self.compute_screen_with(on);
        self.wind_cache.guard = if collar_ok { Some(guard) } else { None };
        self.wind_cache.structure = self.breakable_signature();
        if on {
            let mut accl: Vec<(u32, f32, f32)> = self.shoots.nodes.iter().zip(&demand).filter(|(_, &s)| s > 1.0).map(|(n, &s)| (n.id, s, n.radius())).collect();
            accl.sort_unstable_by_key(|a| a.0);
            self.wind_cache.accl = accl;
        }
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

    /// Test the tree against a gust and apply what fails (module header).
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
                self.wind_cache.guard = None;
                self.wind_cache.tight = None;
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
            self.wind_cache.tight = None;
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
