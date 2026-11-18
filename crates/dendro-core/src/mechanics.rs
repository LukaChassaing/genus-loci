//! Static mechanics of the tree: self-weight plus a steady wind, up to the
//! critical wind speed of each failure mode (first step of milestone M1,
//! « Physique du milieu »).
//!
//! Like [`crate::flux`], this is a **reading** of the simulation: no state,
//! no feedback on growth, deterministic (transcendental functions through
//! [`crate::fmath::DMath`]). Growth has its own copy of the self-weight case
//! (`ShootSystem::adaptive_demand` in growth.rs: same loads, same
//! `σ = 32 M / (π D³)`), which drives the adaptive radial growth of the
//! limbs; this module is not called by the simulation. Nothing breaks here; the outputs (safety factor
//! per segment, weakest point, failure mode, critical speeds) are what a
//! later step will turn into actual breakage logged as a `LogEvent`.
//!
//! # Model (sources in each constant's comment and in docs/sources.md)
//! * **Loads.** Every shoot segment carries its own weight (green wood,
//!   [`MechanicsTraits::green_density_kg_m3`]) at its midpoint; leafy shoots
//!   carry the fresh leaf mass at their tip. An optional extra mass per
//!   segment ([`WindLoad::extra_mass_kg`]) is the hook for snow and
//!   intercepted rain. Wind: a logarithmic profile over open ground scaled
//!   by the speed at 10 m; drag on the leafy crown `F = ½ ρ Cd(U) A0 U²`
//!   on its still-air frontal area `A0`, with reconfiguration
//!   `Cd ∝ U^B` (Vogel exponent B < 0, so drag ∝ U^(2+B)); in leafless
//!   parts, cross-flow drag on every woody segment taken as a rigid cylinder.
//! * **Moments.** Forces and moments are summed as vectors from the tips
//!   to the collar (`M_i = Σ_j (a_j − s_i) × F_j` over the subtree of i,
//!   computed in O(n) as `Σ a_j × F_j − s_i × Σ F_j`); the bending moment of
//!   a segment is the part perpendicular to its axis (the axial part is
//!   torsion, ignored). Branches in cantilever and an inclined trunk thus
//!   load their base through their weight, and opposite loads cancel.
//! * **Stress.** `σ = 32 M D / (π (D⁴ − Di⁴))` at the base of each
//!   segment: a solid circular section (`Di = 0`, i.e. `σ = 32 M / (π D³)`)
//!   or a hollow one where pruning decay (`pruning.rs`, CODIT) has reached
//!   it. Safety factor = green modulus of rupture / σ (axial stress from
//!   weight, ~0.4 MPa at the base of a 30 t oak, is neglected).
//! * **Uprooting.** Critical turning moment of the root-soil plate
//!   `M_crit = Creg × stem mass` (Gardiner, Peltola & Kellomäki 2000,
//!   Ecol Model 129:1-23; ForestGALES 2.5), reduced for shallow rooting and
//!   a saturated root zone, compared with the turning moment at the collar.
//! * **Critical speeds.** For each mode, the lowest 10 m wind speed at which
//!   its safety factor drops below 1: a 2 m/s scan from 0 to 80 m/s, then
//!   bisection in the first bracket (safety is not monotonic in U for every
//!   segment: on a branch leaning into the wind, wind and weight moments
//!   first cancel).
//!
//! # Interpretation of the wind speed
//! The speed is treated as a **gust** (a few seconds) at 10 m, applied
//! quasi-statically: no gust factor, no dynamic amplification (sway,
//! resonance), as in the static tree assessment of Sinn & Wessolly (1989,
//! Arboric J 13:45-65), which loads a tree with a 32 m/s Beaufort-12 wind.
//! Critical speeds are therefore gust speeds, comparable to the ~42 m/s
//! storm gusts above which almost all trunks break (Virot et al. 2016, Phys
//! Rev E 93:023001).
//!
//! Cost: one evaluation is O(n) in shoot segments (a few hundred µs for a
//! 150-year oak in release builds); the critical speeds take ~80
//! evaluations. Callers cache the result (see the Godot binding).

use crate::fmath::DMath;
use crate::genome::MechanicsTraits;
use crate::math::Vec3;
use crate::sim::Simulation;
use std::collections::HashSet;

// ----------------------------------------------------------------------
// Constants
// ----------------------------------------------------------------------

/// Standard gravity (m/s²), CGPM 1901.
pub const GRAVITY: f32 = 9.806_65;
/// Air density (kg/m³) of the ICAO standard atmosphere at sea level, 15 °C.
pub const AIR_DENSITY: f32 = 1.225;
/// Height of the reference wind speed (m): the WMO standard anemometer
/// height (WMO-No. 8, Guide to Meteorological Instruments and Methods of
/// Observation).
pub const REFERENCE_HEIGHT_M: f32 = 10.0;
/// Aerodynamic roughness length (m) of open, flat country with low
/// vegetation ("open" class of the Davenport classification as revised by
/// Wieringa, tabulated in WMO-No. 8: value quoted from memory, reference
/// not re-checked; see docs/sources.md). The tree stands alone in a field.
pub const ROUGHNESS_LENGTH_M: f32 = 0.03;
/// Height (m) below which the logarithmic profile is held constant
/// (estimated: the log law is not meant for the first metre, where it
/// would go to zero at z0).
pub const PROFILE_MIN_HEIGHT_M: f32 = 1.0;
/// Speed at which `MechanicsTraits::crown_drag_coefficient` is defined
/// (m/s): the highest wind-tunnel speed of Vollsinger et al. 2005.
pub const CROWN_CD_REFERENCE_MS: f32 = 20.0;
/// Reconfiguration range (m/s): below, the crown drag coefficient keeps its
/// value at this speed; above `RECONF_MAX_MS` the crown is fully
/// streamlined and drag grows as U² again. As in ForestGALES 2.5 (Forest
/// Research, user manual, technical appendix eq. 12: "maximum value of drag
/// coefficient set at u = 10 m/s and a minimum value at u = 25 m/s").
pub const RECONF_MIN_MS: f32 = 10.0;
pub const RECONF_MAX_MS: f32 = 25.0;
/// Drag coefficient of a woody segment taken as a smooth circular cylinder
/// in cross flow at subcritical Reynolds numbers (~1.2 for 10^4 < Re <
/// 2·10^5; textbook value, e.g. Hoerner 1965, "Fluid-Dynamic Drag", not
/// re-read). Twigs and limbs in
/// storm winds fall in that range; a large trunk at high speed is
/// supercritical (lower Cd): overestimate there, minor in the total.
/// Cross-flow principle: normal force ∝ (U sin θ)² (Hoerner 1965).
pub const WOOD_DRAG_COEFFICIENT: f32 = 1.2;
/// Reconfiguration of leafless branches (Vogel exponent, same speed range
/// as the crown): bare branches bend and streamline too, less than leaves.
/// No measurement on leafless broadleaf crowns was found: -0.5 is
/// estimated, between a rigid cylinder (0) and leafy hardwood crowns (-1,
/// Vollsinger et al. 2005), and of the order of the ForestGALES 2.5 fit for
/// Sitka spruce crowns (n = 0.51), whose streamlining is mainly the bending
/// of branches. Not applied to the stem.
pub const BARE_WOOD_VOGEL_EXPONENT: f32 = -0.5;
/// Leaf dry matter content (dry mass / fresh mass), to turn the dry leaf
/// mass per area of `flux.rs` (LMA) into a fresh mass. Order of magnitude
/// for tree leaves (0.3-0.45; LDMC as defined by Wilson, Thompson &
/// Hodgson 1999, New Phytol 143:155-162): 0.4 is estimated.
pub const LEAF_DRY_MATTER_CONTENT: f32 = 0.4;
/// Rooting depth classes of Nicoll et al. 2006 (Can J For Res 36:1871-1883):
/// shallow < 0.4 m, deep > 0.8 m.
pub const SHALLOW_ROOTING_M: f32 = 0.4;
pub const DEEP_ROOTING_M: f32 = 0.8;
/// Anchorage of shallow relative to deep rooting: ForestGALES 2.5 Sitka
/// spruce coefficients on freely draining soil, 153.2 / 178.1 N m/kg (the
/// abstract of Nicoll et al. 2006: deep rooting adds 10-15 %). Linear
/// between the two depth classes (estimated interpolation).
pub const SHALLOW_ROOTING_FACTOR: f32 = 153.2 / 178.1;
/// Anchorage on a gleyed (seasonally waterlogged) mineral soil relative to
/// a freely draining one: ForestGALES 2.5 Sitka spruce, shallow rooting,
/// 135.8 / 153.2 N m/kg (the deep-rooting ratio is the same, 157.9 /
/// 178.1). Applied here in proportion to the share of the roots in
/// saturated layers today: the soil class of GALES is read as a soil state
/// (estimated). Tree-pulling on soil just watered confirms that water
/// below the root plate lowers the critical turning moment (Kamimura et
/// al. 2012, Eur J For Res 131:219-227; not quantified here).
pub const WET_SOIL_FACTOR: f32 = 135.8 / 153.2;
/// A segment counts as a structural limb (and as part of the stem for the
/// stem mode) from this diameter, and from `LIMB_DBH_FRACTION` x DBH:
/// the limb definition of `ShapeMetrics` (2r >= 0.2 DBH). Thinner wood,
/// leafy shoots and dead wood are reported but never decide the failure
/// mode (twigs break in every gale).
pub const LIMB_MIN_DIAMETER_M: f32 = 0.05;
pub const LIMB_DBH_FRACTION: f32 = 0.2;
/// Upper end of the critical speed search (m/s). Gusts beyond ~80 m/s are
/// unknown in western Europe.
pub const MAX_SEARCH_SPEED_MS: f32 = 80.0;
const SCAN_STEP_MS: f32 = 2.0;
const BISECT_STEPS: u32 = 14;
/// Reported safety factors are capped (an unloaded segment has infinite
/// safety).
pub const SAFETY_CAP: f32 = 999.0;
/// Greenhill's constant for the self-weight buckling height of a uniform
/// cylindrical column, `H_crit = 0.792 (E / (ρ g))^(1/3) D^(2/3)`
/// (Greenhill 1881, Proc Camb Philos Soc, not read: constant as quoted by
/// McMahon 1973, Science 179:1201-1204, who found trees ~4x below it).
const GREENHILL: f32 = 0.792;

// ----------------------------------------------------------------------
// Public types
// ----------------------------------------------------------------------

/// Role of a segment in the failure modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentRole {
    /// On the stem: the path from the collar along the thickest child.
    Trunk,
    /// A structural limb (see [`LIMB_MIN_DIAMETER_M`]).
    Limb,
    /// Minor wood, leafy shoots, dead wood.
    Branch,
}

impl SegmentRole {
    pub fn code(self) -> u8 {
        self as u8
    }
}

/// How the tree is expected to fail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FailureMode {
    /// Stem breakage (« volis »).
    #[default]
    StemBreak,
    /// Breakage of a structural limb (« charpentière »).
    LimbBreak,
    /// Uprooting: the root plate turns over (« chablis »).
    Uprooting,
}

impl FailureMode {
    pub fn code(self) -> u8 {
        self as u8
    }
    pub fn label_fr(self) -> &'static str {
        match self {
            FailureMode::StemBreak => "rupture du tronc",
            FailureMode::LimbBreak => "rupture d'une charpentière",
            FailureMode::Uprooting => "déracinement",
        }
    }
}

/// One shoot segment as seen by the mechanics. Segments are ordered with
/// `parent < index`; index 0 is the collar.
#[derive(Clone, Debug, PartialEq)]
pub struct MechSegment {
    /// Stable node id (`ShootInfo::id`).
    pub id: u32,
    pub parent: Option<usize>,
    pub start: [f32; 3],
    pub end: [f32; 3],
    /// Outer wood radius at the base of the segment (m).
    pub radius_m: f32,
    /// Radius of the decayed core at the base (m), 0 if sound.
    pub decay_radius_m: f32,
    /// Relative leaf area carried at the tip (0 = no leaves). Normalised
    /// over the tree.
    pub leaf_weight: f32,
    /// Standing dead wood.
    pub dead: bool,
}

/// Site and canopy state the mechanics needs besides the segments.
#[derive(Clone, Debug, PartialEq)]
pub struct MechSite {
    /// One-sided leaf area of the full summer canopy (m²).
    pub leaf_area_m2: f32,
    /// Share of that canopy present today (phenology, 0..1).
    pub leaf_fraction: f32,
    /// Cell size (m) of the frontal-area raster (the light voxel).
    pub crown_cell_m: f32,
    /// Stem diameter at 1.3 m (m), for the limb threshold.
    pub dbh_m: f32,
    /// Maximum rooting depth (m).
    pub root_depth_m: f32,
    /// Share of the root length in saturated soil layers today (0..1).
    pub saturated_root_fraction: f32,
    /// Leaf mass per area (g dry / m², `LeafTraits::lma_g_m2`): with
    /// `LEAF_DRY_MATTER_CONTENT`, the fresh leaf mass of the crown.
    pub leaf_lma_g_m2: f32,
}

/// The load case: a steady wind (plus, later, extra masses).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WindLoad {
    /// Gust speed at 10 m over open ground (m/s), applied statically.
    pub speed_10m_ms: f32,
    /// Direction the wind blows **towards**, in degrees from +x towards +z
    /// in the horizontal plane.
    pub azimuth_deg: f32,
    /// Overrides the phenological leaf fraction (`Some(0.0)`: leafless,
    /// `Some(1.0)`: full summer crown); `None` = today's.
    pub leaf_fraction: Option<f32>,
    /// Extra mass per segment (kg, segment order; missing = 0), applied at
    /// the segment midpoint: the slot for snow and intercepted water.
    pub extra_mass_kg: Vec<f32>,
}

impl WindLoad {
    /// A wind of `speed` m/s along +x, today's foliage, no extra load.
    pub fn new(speed_10m_ms: f32) -> Self {
        WindLoad { speed_10m_ms, ..Default::default() }
    }
}

/// Lowest 10 m gust speed (m/s) at which each mode occurs; `None` beyond
/// [`MAX_SEARCH_SPEED_MS`] (or no structure for that mode, e.g. no limb).
/// `Some(0.0)` means failure under self-weight alone.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CriticalSpeeds {
    pub stem_break: Option<f32>,
    pub limb_break: Option<f32>,
    pub uprooting: Option<f32>,
}

impl CriticalSpeeds {
    /// The mode reached first as the wind rises, and its speed.
    pub fn first(&self) -> Option<(FailureMode, f32)> {
        [(FailureMode::StemBreak, self.stem_break), (FailureMode::LimbBreak, self.limb_break), (FailureMode::Uprooting, self.uprooting)]
            .into_iter()
            .filter_map(|(m, v)| v.map(|v| (m, v)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }
}

/// The segment with the lowest safety factor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeakPoint {
    /// Index in the segment list (shoot node order).
    pub index: usize,
    pub node_id: u32,
    pub role: SegmentRole,
    pub safety: f32,
    /// Height of the section (segment base, m) and its diameter (cm).
    pub height_m: f32,
    pub diameter_cm: f32,
    /// The section is hollowed by decay.
    pub decayed: bool,
}

/// Result of one load case.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MechanicsSnapshot {
    /// Load case (10 m gust speed, m/s) and the leaf fraction used.
    pub wind_speed_ms: f32,
    pub leaf_fraction: f32,
    /// Per segment (shoot node order): safety factor (capped at
    /// [`SAFETY_CAP`]), bending moment at the base (N m), bending stress
    /// (MPa), role, stable node id.
    pub segment_safety: Vec<f32>,
    pub segment_moment_nm: Vec<f32>,
    pub segment_stress_mpa: Vec<f32>,
    pub segment_role: Vec<SegmentRole>,
    pub segment_ids: Vec<u32>,
    /// Lowest safety factor of each mode (capped).
    pub stem_safety: f32,
    pub limb_safety: f32,
    pub anchorage_safety: f32,
    /// Weakest structural section (stem or limb) and weakest section of all.
    pub weakest: Option<WeakPoint>,
    pub weakest_any: Option<WeakPoint>,
    /// Mode with the lowest safety factor at this speed.
    pub expected_failure: FailureMode,
    /// Critical speeds (filled by [`Simulation::mechanics_snapshot`] and
    /// [`MechanicsModel::critical_speeds`]; default when not computed).
    pub critical: CriticalSpeeds,
    /// Turning moment at the collar (N m, horizontal axes) and critical
    /// turning moment of the root plate (N m).
    pub base_moment_nm: f32,
    pub critical_turning_moment_nm: f32,
    /// Masses (kg): above ground (wood + leaves + extra), wood, fresh
    /// leaves, stem (the anchorage regressor), extra load.
    pub above_ground_mass_kg: f32,
    pub wood_mass_kg: f32,
    pub leaf_mass_kg: f32,
    pub stem_mass_kg: f32,
    pub extra_mass_kg: f32,
    /// Total horizontal wind force (N) and height of its resultant (m).
    pub drag_n: f32,
    pub centre_of_pressure_m: f32,
    /// Still-air frontal area of the leafy crown (m², with today's leaves).
    pub crown_frontal_area_m2: f32,
    /// Crown drag coefficient at the wind speed at mid-crown height.
    pub crown_drag_coefficient: f32,
    /// Self-weight elastic buckling height / tree height (Greenhill).
    pub buckling_safety: f32,
}

// ----------------------------------------------------------------------
// Model
// ----------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Seg {
    id: u32,
    parent: usize,
    start: Vec3,
    end: Vec3,
    mid: Vec3,
    dir: Vec3,
    len: f32,
    d: f32,
    di: f32,
    /// Section modulus π (D⁴ − Di⁴) / (32 D) (m³).
    zmod: f32,
    /// Green wood mass (kg).
    mass: f32,
    leaf_share: f32,
    role: SegmentRole,
    /// Counts for the stem (resp. limb) mode.
    stem_mode: bool,
    limb_mode: bool,
}

const NO_PARENT: usize = usize::MAX;

/// Geometry and traits of one tree, prepared for repeated evaluation.
#[derive(Clone, Debug)]
pub struct MechanicsModel {
    traits: MechanicsTraits,
    site: MechSite,
    segs: Vec<Seg>,
    wood_mass: f32,
    stem_mass: f32,
    height: f32,
    crown_mid_y: f32,
}

/// Direction- and foliage-dependent coefficients of one load case.
struct Prepared {
    wind: Vec3,
    leaf_fraction: f32,
    crown_area: f32,
    /// Per segment: wood drag `½ ρ Cd d L sin²θ × exposure` (N per (m/s)²),
    /// whether that wood reconfigures (not the stem), crown drag
    /// `½ ρ A0 share` (m² kg/m³), leaf mass and extra mass (kg), wind
    /// profile factor at the midpoint and at the tip.
    wood_k: Vec<f32>,
    wood_bends: Vec<bool>,
    crown_k: Vec<f32>,
    leaf_mass: Vec<f32>,
    extra: Vec<f32>,
    prof_mid: Vec<f32>,
    prof_end: Vec<f32>,
    /// Profile factors raised to the bare-wood and crown Vogel exponents.
    prof_mid_b: Vec<f32>,
    prof_end_b: Vec<f32>,
    wood_reconf: Reconf,
    crown_reconf: Reconf,
}

struct PassOut {
    stem: f32,
    limb: f32,
    anchorage: f32,
    base_moment: f32,
    drag: f32,
    drag_height: f32,
    /// (bending moment N m, stress Pa, safety) per segment, when asked.
    detail: Vec<(f32, f32, f32)>,
}

/// `U(z) / U(10 m)` of the logarithmic profile over open ground:
/// `ln(z / z0) / ln(10 / z0)` (neutral surface layer; e.g. Stull 1988, "An
/// Introduction to Boundary Layer Meteorology", Kluwer, ch. 9), held
/// constant below [`PROFILE_MIN_HEIGHT_M`].
#[inline]
pub fn wind_profile(z: f32) -> f32 {
    profile(z)
}

#[inline]
fn profile(z: f32) -> f32 {
    let z = z.max(PROFILE_MIN_HEIGHT_M);
    (z / ROUGHNESS_LENGTH_M).dln() / (REFERENCE_HEIGHT_M / ROUGHNESS_LENGTH_M).dln()
}

/// `x^b` for the reconfiguration laws, factorised so that a pass costs one
/// `dpowf` per load case instead of one per segment: with local speed
/// `u_loc = u x k` (k = profile factor), `(u_loc / u_ref)^b` inside the
/// reconfiguration range is `(u / u_ref)^b x k^b`; outside it is constant.
struct Reconf {
    b: f32,
    u_ref: f32,
    /// Values at the clamped ends of the range.
    lo: f32,
    hi: f32,
}

impl Reconf {
    fn new(b: f32, u_ref: f32) -> Self {
        Reconf { b, u_ref, lo: (RECONF_MIN_MS / u_ref).dpowf(b), hi: (RECONF_MAX_MS / u_ref).dpowf(b) }
    }
    /// `(clamp(u x k) / u_ref)^b` given `ub = (u / u_ref)^b` and `kb = k^b`.
    #[inline]
    fn at(&self, u_loc: f32, ub: f32, kb: f32) -> f32 {
        if u_loc <= RECONF_MIN_MS {
            self.lo
        } else if u_loc >= RECONF_MAX_MS {
            self.hi
        } else {
            ub * kb
        }
    }
}

impl MechanicsModel {
    /// Builds the model from explicit segments (tests, tools); see
    /// [`Simulation::mechanics_model`] for a simulated tree. Segments must
    /// be ordered with `parent < index`.
    pub fn from_segments(segments: &[MechSegment], traits: MechanicsTraits, site: MechSite) -> MechanicsModel {
        let n = segments.len();
        let v = |a: [f32; 3]| Vec3::new(a[0], a[1], a[2]);
        // Stem: from the collar along the thickest child (as ShapeMetrics).
        let mut thickest = vec![NO_PARENT; n];
        for (i, s) in segments.iter().enumerate() {
            if let Some(p) = s.parent {
                assert!(p < i, "mechanics segments must be ordered parent < child");
                let t = thickest[p];
                if t == NO_PARENT || s.radius_m > segments[t].radius_m {
                    thickest[p] = i;
                }
            }
        }
        let mut on_trunk = vec![false; n];
        if n > 0 {
            let mut cur = 0;
            on_trunk[0] = true;
            while thickest[cur] != NO_PARENT {
                cur = thickest[cur];
                on_trunk[cur] = true;
            }
        }
        let structural_d = LIMB_MIN_DIAMETER_M.max(LIMB_DBH_FRACTION * site.dbh_m);
        let leaf_total: f32 = segments.iter().map(|s| s.leaf_weight.max(0.0)).sum();
        let rho = traits.green_density_kg_m3;
        let mut segs = Vec::with_capacity(n);
        let (mut wood_mass, mut stem_mass, mut height) = (0.0f32, 0.0f32, 0.0f32);
        let (mut leaf_y, mut leaf_w) = (0.0f32, 0.0f32);
        for (i, s) in segments.iter().enumerate() {
            let (start, end) = (v(s.start), v(s.end));
            let axis = end - start;
            let len = axis.length();
            let dir = axis.normalize_or(Vec3::Y);
            let r = s.radius_m.max(1e-5);
            let d = 2.0 * r;
            let di = (2.0 * s.decay_radius_m).clamp(0.0, 0.98 * d);
            let zmod = std::f32::consts::PI * (d * d * d * d - di * di * di * di) / (32.0 * d);
            // Decayed wood keeps most of its (wet) mass: full section.
            let mass = rho * std::f32::consts::PI * r * r * len;
            let leaf_share = if leaf_total > 0.0 { s.leaf_weight.max(0.0) / leaf_total } else { 0.0 };
            let role = if on_trunk[i] {
                SegmentRole::Trunk
            } else if !s.dead && s.leaf_weight <= 0.0 && d >= structural_d {
                SegmentRole::Limb
            } else {
                SegmentRole::Branch
            };
            let structural = !s.dead && s.leaf_weight <= 0.0 && d >= structural_d;
            wood_mass += mass;
            if on_trunk[i] {
                stem_mass += mass;
            }
            height = height.max(end.y);
            if leaf_share > 0.0 {
                leaf_y += leaf_share * end.y;
                leaf_w += leaf_share;
            }
            segs.push(Seg {
                id: s.id,
                parent: s.parent.unwrap_or(NO_PARENT),
                start,
                end,
                mid: start + axis * 0.5,
                dir,
                len,
                d,
                di,
                zmod: zmod.max(1e-12),
                mass,
                leaf_share,
                role,
                stem_mode: on_trunk[i] && structural,
                limb_mode: role == SegmentRole::Limb,
            });
        }
        let crown_mid_y = if leaf_w > 0.0 { leaf_y / leaf_w } else { 0.5 * height };
        MechanicsModel { traits, site, segs, wood_mass, stem_mass, height, crown_mid_y }
    }

    pub fn segment_count(&self) -> usize {
        self.segs.len()
    }

    /// Crown drag coefficient at local wind speed `u` (m/s), on the
    /// still-air frontal area: `Cd_ref (u / 20)^B`, `u` clamped to the
    /// reconfiguration range (see [`RECONF_MIN_MS`]).
    pub fn crown_cd(&self, u: f32) -> f32 {
        let t = &self.traits;
        let uc = u.clamp(RECONF_MIN_MS, RECONF_MAX_MS);
        t.crown_drag_coefficient * (uc / CROWN_CD_REFERENCE_MS).dpowf(t.vogel_exponent)
    }

    /// Critical turning moment of the root plate (N m): `Creg x stem mass`
    /// (Gardiner et al. 2000; ForestGALES 2.5 eq. 1), scaled for rooting
    /// depth and saturated soil.
    pub fn critical_turning_moment_nm(&self) -> f32 {
        let s = &self.site;
        let t = ((s.root_depth_m - SHALLOW_ROOTING_M) / (DEEP_ROOTING_M - SHALLOW_ROOTING_M)).clamp(0.0, 1.0);
        let depth = SHALLOW_ROOTING_FACTOR + (1.0 - SHALLOW_ROOTING_FACTOR) * t;
        let wet = 1.0 - (1.0 - WET_SOIL_FACTOR) * s.saturated_root_fraction.clamp(0.0, 1.0);
        self.traits.anchorage_nm_per_kg * self.stem_mass * depth * wet
    }

    fn prepare(&self, load: &WindLoad) -> Prepared {
        let n = self.segs.len();
        let f = load.leaf_fraction.unwrap_or(self.site.leaf_fraction).clamp(0.0, 1.0);
        let (sa, ca) = load.azimuth_deg.to_radians().dsin_cos();
        let wind = Vec3::new(ca, 0.0, sa);
        let across = Vec3::new(-sa, 0.0, ca);
        // Still-air frontal area of the leafy crown: leafy shoot tips
        // rasterised on the plane normal to the wind, one light voxel per
        // cell (the resolution at which the simulation knows its crown).
        let cell = self.site.crown_cell_m.max(0.05);
        let mut cells: HashSet<(i32, i32)> = HashSet::new();
        for s in self.segs.iter().filter(|s| s.leaf_share > 0.0) {
            let u = (s.end.dot(across) / cell).floor() as i32;
            let w = (s.end.y / cell).floor() as i32;
            cells.insert((u, w));
        }
        let full_area = cells.len() as f32 * cell * cell;
        let crown_area = full_area * f;
        // Fresh leaf mass (kg/m²): the species' LMA (g dry / m²) / LDMC.
        let leaf_kg_m2 = self.site.leaf_lma_g_m2 * 1e-3 / LEAF_DRY_MATTER_CONTENT;
        let leaf_kg = self.site.leaf_area_m2 * f * leaf_kg_m2;
        let mut p = Prepared {
            wind,
            leaf_fraction: f,
            crown_area,
            wood_k: Vec::with_capacity(n),
            wood_bends: Vec::with_capacity(n),
            crown_k: Vec::with_capacity(n),
            leaf_mass: Vec::with_capacity(n),
            extra: Vec::with_capacity(n),
            prof_mid: Vec::with_capacity(n),
            prof_end: Vec::with_capacity(n),
            prof_mid_b: Vec::with_capacity(n),
            prof_end_b: Vec::with_capacity(n),
            // Bare wood: rigid (1) up to RECONF_MIN_MS, then (u / 10)^B.
            wood_reconf: Reconf::new(BARE_WOOD_VOGEL_EXPONENT, RECONF_MIN_MS),
            crown_reconf: Reconf::new(self.traits.vogel_exponent, CROWN_CD_REFERENCE_MS),
        };
        for (i, s) in self.segs.iter().enumerate() {
            let c = s.dir.dot(wind);
            let sin2 = (1.0 - c * c).max(0.0);
            // In leaf, the crown drag coefficient (measured on whole crowns)
            // already includes the branches inside the crown: their own drag
            // only counts for the leafless share. The stem is always exposed.
            let exposure = if s.role == SegmentRole::Trunk { 1.0 } else { 1.0 - f };
            p.wood_k.push(0.5 * AIR_DENSITY * WOOD_DRAG_COEFFICIENT * s.d * s.len * sin2 * exposure);
            p.wood_bends.push(s.role != SegmentRole::Trunk);
            p.crown_k.push(0.5 * AIR_DENSITY * crown_area * s.leaf_share);
            p.leaf_mass.push(leaf_kg * s.leaf_share);
            p.extra.push(load.extra_mass_kg.get(i).copied().unwrap_or(0.0).max(0.0));
            let (km, ke) = (profile(s.mid.y), profile(s.end.y));
            p.prof_mid.push(km);
            p.prof_end.push(ke);
            p.prof_mid_b.push(if p.wood_bends[i] { km.dpowf(BARE_WOOD_VOGEL_EXPONENT) } else { 1.0 });
            p.prof_end_b.push(if s.leaf_share > 0.0 { ke.dpowf(self.traits.vogel_exponent) } else { 1.0 });
        }
        p
    }

    /// One O(n) pass at 10 m wind speed `u`.
    fn pass(&self, p: &Prepared, u: f32, detail: bool) -> PassOut {
        let n = self.segs.len();
        let mut fsum = vec![Vec3::ZERO; n];
        let mut qsum = vec![Vec3::ZERO; n];
        let (mut drag, mut drag_h) = (0.0f32, 0.0f32);
        let (ub_wood, ub_crown) = if u > 0.0 {
            ((u / p.wood_reconf.u_ref).dpowf(p.wood_reconf.b), (u / p.crown_reconf.u_ref).dpowf(p.crown_reconf.b))
        } else {
            (1.0, 1.0)
        };
        let cd_ref = self.traits.crown_drag_coefficient;
        for (i, s) in self.segs.iter().enumerate() {
            let down = Vec3::new(0.0, -GRAVITY, 0.0);
            let fw = down * (s.mass + p.extra[i]);
            let mut f = fw;
            let mut q = s.mid.cross(fw);
            if p.leaf_mass[i] > 0.0 {
                let fl = down * p.leaf_mass[i];
                f += fl;
                q += s.end.cross(fl);
            }
            if u > 0.0 {
                let um = u * p.prof_mid[i];
                let reconf = if p.wood_bends[i] { p.wood_reconf.at(um, ub_wood, p.prof_mid_b[i]) } else { 1.0 };
                let fwood = p.wood_k[i] * reconf * um * um;
                if fwood > 0.0 {
                    let v = p.wind * fwood;
                    f += v;
                    q += s.mid.cross(v);
                    drag += fwood;
                    drag_h += fwood * s.mid.y;
                }
                if p.crown_k[i] > 0.0 {
                    let ue = u * p.prof_end[i];
                    let fc = p.crown_k[i] * cd_ref * p.crown_reconf.at(ue, ub_crown, p.prof_end_b[i]) * ue * ue;
                    let v = p.wind * fc;
                    f += v;
                    q += s.end.cross(v);
                    drag += fc;
                    drag_h += fc * s.end.y;
                }
            }
            fsum[i] = f;
            qsum[i] = q;
        }
        for i in (1..n).rev() {
            let pi = self.segs[i].parent;
            if pi != NO_PARENT {
                let (f, q) = (fsum[i], qsum[i]);
                fsum[pi] += f;
                qsum[pi] += q;
            }
        }
        let mor = self.traits.green_mor_mpa * 1e6;
        let (mut stem, mut limb) = (f32::INFINITY, f32::INFINITY);
        let mut out = Vec::with_capacity(if detail { n } else { 0 });
        let mut base_moment = 0.0;
        for (i, s) in self.segs.iter().enumerate() {
            let m = qsum[i] - s.start.cross(fsum[i]);
            if i == 0 {
                base_moment = (m.x * m.x + m.z * m.z).sqrt();
            }
            if !(detail || s.stem_mode || s.limb_mode) {
                continue;
            }
            let mb = m - s.dir * m.dot(s.dir);
            let bend = mb.length();
            let sigma = bend / s.zmod;
            let sf = if sigma > 0.0 { mor / sigma } else { f32::INFINITY };
            if s.stem_mode {
                stem = stem.min(sf);
            }
            if s.limb_mode {
                limb = limb.min(sf);
            }
            if detail {
                out.push((bend, sigma, sf));
            }
        }
        let mcrit = self.critical_turning_moment_nm();
        let anchorage = if base_moment > 0.0 { mcrit / base_moment } else { f32::INFINITY };
        PassOut { stem, limb, anchorage, base_moment, drag, drag_height: drag_h, detail: out }
    }

    /// Safety factors, moments and masses for one load case (the critical
    /// speeds are left at their default; see [`Self::critical_speeds`]).
    pub fn evaluate(&self, load: &WindLoad) -> MechanicsSnapshot {
        let p = self.prepare(load);
        let u = load.speed_10m_ms.max(0.0);
        let o = self.pass(&p, u, true);
        let cap = |x: f32| x.min(SAFETY_CAP);
        let mut snap = MechanicsSnapshot {
            wind_speed_ms: u,
            leaf_fraction: p.leaf_fraction,
            stem_safety: cap(o.stem),
            limb_safety: cap(o.limb),
            anchorage_safety: cap(o.anchorage),
            base_moment_nm: o.base_moment,
            critical_turning_moment_nm: self.critical_turning_moment_nm(),
            wood_mass_kg: self.wood_mass,
            stem_mass_kg: self.stem_mass,
            leaf_mass_kg: p.leaf_mass.iter().sum(),
            extra_mass_kg: p.extra.iter().sum(),
            drag_n: o.drag,
            centre_of_pressure_m: if o.drag > 0.0 { o.drag_height / o.drag } else { 0.0 },
            crown_frontal_area_m2: p.crown_area,
            crown_drag_coefficient: self.crown_cd(u * profile(self.crown_mid_y)),
            ..Default::default()
        };
        snap.above_ground_mass_kg = snap.wood_mass_kg + snap.leaf_mass_kg + snap.extra_mass_kg;
        let modes = [(FailureMode::StemBreak, o.stem), (FailureMode::LimbBreak, o.limb), (FailureMode::Uprooting, o.anchorage)];
        snap.expected_failure = modes.iter().min_by(|a, b| a.1.total_cmp(&b.1)).map(|m| m.0).unwrap_or_default();
        let n = self.segs.len();
        snap.segment_safety.reserve(n);
        let mut weak: Option<WeakPoint> = None;
        let mut weak_any: Option<WeakPoint> = None;
        for (i, (s, &(bend, sigma, sf))) in self.segs.iter().zip(o.detail.iter()).enumerate() {
            snap.segment_safety.push(cap(sf));
            snap.segment_moment_nm.push(bend);
            snap.segment_stress_mpa.push(sigma * 1e-6);
            snap.segment_role.push(s.role);
            snap.segment_ids.push(s.id);
            let wp = WeakPoint { index: i, node_id: s.id, role: s.role, safety: cap(sf), height_m: s.start.y, diameter_cm: 100.0 * s.d, decayed: s.di > 0.0 };
            if (s.stem_mode || s.limb_mode) && weak.is_none_or(|w| sf < w.safety) {
                weak = Some(wp);
            }
            if weak_any.is_none_or(|w| sf < w.safety) {
                weak_any = Some(wp);
            }
        }
        snap.weakest = weak;
        snap.weakest_any = weak_any;
        // Elastic buckling under self-weight (uniform column, collar diameter).
        if let Some(s0) = self.segs.first() {
            let e = self.traits.green_moe_gpa * 1e9;
            let h_crit = GREENHILL * (e / (self.traits.green_density_kg_m3 * GRAVITY)).dpowf(1.0 / 3.0) * s0.d.dpowf(2.0 / 3.0);
            snap.buckling_safety = cap(h_crit / self.height.max(1e-3));
        }
        snap
    }

    /// Critical 10 m gust speed of each failure mode for the direction,
    /// foliage and extra masses of `load` (its speed is ignored).
    pub fn critical_speeds(&self, load: &WindLoad) -> CriticalSpeeds {
        let p = self.prepare(load);
        let eval = |u: f32| {
            let o = self.pass(&p, u, false);
            [o.stem, o.limb, o.anchorage]
        };
        let mut found: [Option<f32>; 3] = [None; 3];
        let first = eval(0.0);
        for k in 0..3 {
            if first[k] < 1.0 {
                found[k] = Some(0.0);
            }
        }
        let mut u0 = 0.0f32;
        while u0 < MAX_SEARCH_SPEED_MS && found.iter().any(|f| f.is_none()) {
            let u1 = (u0 + SCAN_STEP_MS).min(MAX_SEARCH_SPEED_MS);
            let cur = eval(u1);
            for k in 0..3 {
                if found[k].is_none() && cur[k] < 1.0 {
                    let (mut lo, mut hi) = (u0, u1);
                    for _ in 0..BISECT_STEPS {
                        let mid = 0.5 * (lo + hi);
                        if eval(mid)[k] < 1.0 {
                            hi = mid;
                        } else {
                            lo = mid;
                        }
                    }
                    found[k] = Some(hi);
                }
            }
            u0 = u1;
        }
        CriticalSpeeds { stem_break: found[0], limb_break: found[1], uprooting: found[2] }
    }
}

/// Display colour of a safety factor: red at or below 1 (fails), orange
/// around 1.5, yellow at 2, green from 4 up (display convention, not a
/// standard).
pub fn safety_color(sf: f32) -> [f32; 4] {
    const STOPS: [(f32, [f32; 3]); 4] = [(1.0, [0.86, 0.12, 0.10]), (1.5, [0.95, 0.50, 0.10]), (2.0, [0.95, 0.85, 0.20]), (4.0, [0.25, 0.72, 0.30])];
    if sf <= STOPS[0].0 {
        let c = STOPS[0].1;
        return [c[0], c[1], c[2], 1.0];
    }
    for w in STOPS.windows(2) {
        let ((a, ca), (b, cb)) = (w[0], w[1]);
        if sf <= b {
            let t = (sf - a) / (b - a);
            return [ca[0] + (cb[0] - ca[0]) * t, ca[1] + (cb[1] - ca[1]) * t, ca[2] + (cb[2] - ca[2]) * t, 1.0];
        }
    }
    let c = STOPS[3].1;
    [c[0], c[1], c[2], 1.0]
}

/// Per-segment colours of a snapshot for [`Simulation::wood_mesh_tinted`]:
/// structural sections (stem, limbs) in full colour, minor wood faded
/// towards grey so the load path reads first.
pub fn safety_colors(s: &MechanicsSnapshot) -> Vec<[f32; 4]> {
    s.segment_safety
        .iter()
        .zip(&s.segment_role)
        .map(|(&sf, &role)| {
            let c = safety_color(sf);
            if role == SegmentRole::Branch {
                let g = 0.55;
                [0.5 * (c[0] + g), 0.5 * (c[1] + g), 0.5 * (c[2] + g), 1.0]
            } else {
                c
            }
        })
        .collect()
}

// ----------------------------------------------------------------------
// Simulation glue
// ----------------------------------------------------------------------

impl Simulation {
    /// The tree as it is today, ready for mechanics evaluations (see the
    /// module documentation). Pure function of the state.
    pub fn mechanics_model(&self) -> MechanicsModel {
        let (segments, site) = self.mechanics_inputs();
        MechanicsModel::from_segments(&segments, self.genome.mechanics.clone(), site)
    }

    /// The inputs of [`Self::mechanics_model`]: one segment per shoot node
    /// (same order as [`Self::shoot_nodes`]) and the site state. Public so
    /// tools and tests can alter them (e.g. inject decay) and rebuild a
    /// model with [`MechanicsModel::from_segments`].
    pub fn mechanics_inputs(&self) -> (Vec<MechSegment>, MechSite) {
        let nodes = &self.shoots.nodes;
        let ext = self.shoots.ext;
        let decay = self.decay_radius_by_node();
        let unit_len = 0.75 * self.shoots.voxel - 0.06;
        let segments: Vec<MechSegment> = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| MechSegment {
                id: n.id,
                parent: if i == 0 { None } else { Some(n.parent as usize) },
                start: n.start.to_array(),
                end: n.end(ext).to_array(),
                radius_m: n.radius(),
                decay_radius_m: decay[i],
                leaf_weight: if n.leaf_w() > 0.0 { crate::growth::leaf_units(n, unit_len) * n.leaf_w() } else { 0.0 },
                dead: n.is_snag(),
            })
            .collect();
        // Leaf area: crown projection x LAI of a full open-grown crown, as
        // in the flux view (flux.rs, `LAI_FULL`).
        let cr = self.crown_radius_m().max(0.05);
        let soil = self.flux_view().soil;
        let (mut wet, mut tot) = (0.0f32, 0.0f32);
        for l in &soil.layers {
            tot += l.root_len;
            if l.saturated() {
                wet += l.root_len;
            }
        }
        let site = MechSite {
            leaf_area_m2: std::f32::consts::PI * cr * cr * self.genome.leaf.crown_lai,
            leaf_lma_g_m2: self.genome.leaf.lma_g_m2,
            leaf_fraction: self.leaf_density().clamp(0.0, 1.0),
            crown_cell_m: self.shoots.voxel,
            dbh_m: self.dbh_cm() * 0.01,
            root_depth_m: self.root_depth_m(),
            saturated_root_fraction: if tot > 0.0 { wet / tot } else { 0.0 },
        };
        (segments, site)
    }

    /// Static mechanics of the tree today under `load`, with the critical
    /// speeds of every failure mode (same direction and foliage).
    pub fn mechanics_snapshot(&self, load: &WindLoad) -> MechanicsSnapshot {
        let m = self.mechanics_model();
        let mut s = m.evaluate(load);
        s.critical = m.critical_speeds(load);
        s
    }
}
