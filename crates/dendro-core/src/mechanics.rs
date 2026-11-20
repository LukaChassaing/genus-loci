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
//!   `Cd ∝ U^B` (Vogel exponent B < 0, so drag ∝ U^(2+B)). `A0` is the
//!   silhouette of the leafy tips on the plane normal to the wind, one
//!   light voxel per cell, and the force is **distributed by frontal
//!   area**: each cell carries `½ ρ Cd S U(z)²`, shared among the tips of
//!   that cell (tips behind one another along the wind shelter each
//!   other), not by the foliage each shoot carries. Leafless crown wood:
//!   cross-flow drag of rigid cylinders, with the mutual hiding of the
//!   elements of a cell (random overlap) and, inside the leafy envelope,
//!   capped by the drag the same cell has in full leaf
//!   ([`CROWN_WOOD_LEAF_CAP`]). The stem is always fully exposed.
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
use crate::math::{Quat, Vec3};
use crate::sim::Simulation;

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
/// Cap on the drag of the bare crown wood of one envelope cell, as a
/// fraction of the drag of the same cell in full leaf at the same speed:
/// a crown out of leaf does not catch more wind than in leaf. The only
/// measurement found is on three field-grown poplars: drag coefficient of
/// the defoliated crown 0.553-0.770 x that of the leafy crown at 10 m/s, on
/// the same (ellipse) envelope area, the ratio increasing with wind speed
/// (Koizumi et al. 2010, J Wood Sci 56:189-193, author manuscript read).
/// Above 10 m/s the ratio is not known: 1 (the bound itself, no margin
/// taken from the 10 m/s measurement) is estimated.
pub const CROWN_WOOD_LEAF_CAP: f32 = 1.0;
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
/// Below this diameter at the collar nothing breaks in the simulation.
pub const BREAK_MIN_DIAMETER_M: f32 = 0.05;
/// The sections the simulation actually breaks in a gale
/// (`crate::wind`) are the structural ones (stem and limbs, see
/// [`LIMB_MIN_DIAMETER_M`]): crown branches below 20 % of the DBH carry
/// the crown drag of their leafy tips without mutual sheltering, which
/// overestimates their load (docs/sources.md); their loss, like the
/// stripping of twigs and leaves in every gale, is not modelled.
///
/// Uprooting is only checked from this DBH (m). Estimated: the anchorage
/// law `Creg x stem mass` is a regression on pulled trees of plantation
/// size (Nicoll et al. 2006), and applied to open-grown saplings of 5-10 cm
/// it overturns every one of them in the first gale above ~25 m/s, while
/// saplings bend down rather than overturn. Young trees can still snap.
pub const UPROOT_MIN_DBH_M: f32 = 0.2;
/// The stem can break from this diameter up (m; its leafless sections,
/// whatever the DBH). Tried at 2.5 cm (so that a young tree's leader, not
/// the 5 cm threshold, decides where the stem fails): the thin leaders of
/// 10-year oaks then snapped in 17-30 m/s summer gusts and the trees turned
/// into bushes of resprouts breaking again and again (seed 42: 6 m tall at
/// 50 years, 500 breaks in 200 years); kept at the limb threshold, 5 cm,
/// until the load on young crowns is better known (docs/sources.md).
pub const STEM_BREAK_MIN_DIAMETER_M: f32 = LIMB_MIN_DIAMETER_M;
/// Upper end of the critical speed search (m/s). Gusts beyond ~80 m/s are
/// unknown in western Europe.
pub const MAX_SEARCH_SPEED_MS: f32 = 80.0;
const SCAN_STEP_MS: f32 = 2.0;
/// Precision of the critical speeds (m/s): a hundredth of the 0.1 m/s
/// shown, far below the uncertainty of the loads (it was ~1e-4 m/s, 14
/// bisections of a 2 m/s bracket; values unchanged at the 0.1 m/s shown).
const CRITICAL_SPEED_TOL_MS: f32 = 0.01;
/// Slack of the scan's skip-ahead (`MechanicsModel::critical_speeds`):
/// from a speed where the lowest safety is s, the scan jumps to `u sqrt(s /
/// SCAN_SLACK)`. Estimated (the former wind screen used 1.05 for the same
/// bound); checked against the plain 2 m/s scan on 24 cases (oak, cherry,
/// spruce, Scots pine at 20, 60 and 150 years, both foliage states).
const SCAN_SLACK: f32 = 1.15;
/// Reported safety factors are capped (an unloaded segment has infinite
/// safety).
pub const SAFETY_CAP: f32 = 999.0;
/// Greenhill's constant for the self-weight buckling height of a uniform
/// cylindrical column, `H_crit = 0.792 (E / (ρ g))^(1/3) D^(2/3)`
/// (Greenhill 1881, Proc Camb Philos Soc, not read: constant as quoted by
/// McMahon 1973, Science 179:1201-1204, who found trees ~4x below it).
const GREENHILL: f32 = 0.792;

/// Woody segments at least this thick bend in the dynamics (Rayleigh mode
/// shape), and the stem segments among them in the large-deflection model
/// (limbs are not bowed: a cost choice, see `MechanicsModel::pass`); thinner
/// wood, leafy shoots and dead wood move rigidly with their parent axis. Estimated: the
/// reconfiguration of leaves and twigs is already in the measured crown
/// drag and its Vogel exponent (Vollsinger et al. 2005 tested whole
/// juvenile crowns), so letting twigs bend too would count it twice.
pub const FLEX_MIN_DIAMETER_M: f32 = 0.025;
/// Large-deflection iterations: at most this many fixed-point steps, run
/// only when the axes turn by more than `LARGE_DEFLECTION_MIN_RAD`; stop
/// when no rotation changes by more than `LARGE_DEFLECTION_TOL_RAD`;
/// under-relaxation factor; cap on one segment's own rotation (numerical
/// guards, not physics). The cap was 4: the iterations then stopped before
/// convergence on slender leaders bent by several metres (150-year oak in
/// leaf, critical stem gust 64.9 m/s with 4 iterations, 58.9 converged;
/// 130-year cherry 49.4 -> 51.6). Speed searches start each pass from the
/// rotations of the previous one (`Prepared::warm`); their critical speeds
/// then differ from cold evaluations by up to 0.4 m/s (24 cases: oak,
/// cherry, spruce, Scots pine at 20, 60 and 150 years, both foliage
/// states), the effect of the convergence tolerance.
const LARGE_DEFLECTION_ITERATIONS: usize = 16;
pub const LARGE_DEFLECTION_MIN_RAD: f32 = 0.15;
const LARGE_DEFLECTION_TOL_RAD: f32 = 0.005;
const LARGE_DEFLECTION_RELAX: f32 = 0.6;
const LARGE_DEFLECTION_MAX_SEG_RAD: f32 = 0.35;
/// EN 1991-1-4:2005: minimum height of the profile laws (Table 4.1,
/// terrain category II, z0 = 0.05 m; our z0 is 0.03 m) and averaging period
/// of the peak factor (B.2, T = 600 s).
const EC_Z_MIN_M: f32 = 2.0;
const EC_PEAK_PERIOD_S: f32 = 600.0;

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
    /// Quasi-static small-deflection statics only (no dynamic factor, no
    /// large deflections): the first, static M1 model, for hand-checked
    /// cases.
    pub quasi_static: bool,
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
    /// The section can break in the simulation (stem and limb sections, see
    /// [`BREAK_MIN_DIAMETER_M`]).
    pub segment_breakable: Vec<bool>,
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
    /// First sway mode: natural frequency (Hz), total damping ratio in this
    /// wind (still-air + aerodynamic), dynamic factor applied to the wind
    /// loads, horizontal displacement of the stem top (m).
    pub natural_frequency_hz: f32,
    pub damping_ratio: f32,
    pub dynamic_factor: f32,
    pub top_deflection_m: f32,
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
    /// Bending stiffness E I (N m²) of the segments that bend in the
    /// dynamics and large-deflection model, 0 for the rigid ones (leafy
    /// shoots, dead wood, wood thinner than [`FLEX_MIN_DIAMETER_M`]).
    ei: f32,
    /// Bends in the large-deflection model: a flexible stem section of at
    /// least 20 % of the DBH (the thin leader in the crown of a big tree
    /// moves with it, like the crown branches).
    bows: bool,
    /// Green wood mass (kg).
    mass: f32,
    leaf_share: f32,
    role: SegmentRole,
    /// Counts for the stem (resp. limb) mode.
    stem_mode: bool,
    limb_mode: bool,
    /// Can break in the simulation: stem or limb mode (see
    /// [`BREAK_MIN_DIAMETER_M`]).
    breakable: bool,
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
    /// Highest segment of the stem path (top deflection).
    trunk_top: usize,
    /// The stem path, collar first.
    trunk: Vec<usize>,
}

/// Direction- and foliage-dependent coefficients of one load case.
struct Prepared {
    wind: Vec3,
    leaf_fraction: f32,
    crown_area: f32,
    /// Crown width across the wind (m), for the dynamic factor.
    crown_width: f32,
    /// Per segment: wood drag `½ ρ Cd d L × exposure` without the cross-flow
    /// factor (`wood_base`, for the deformed geometry), and with it:
    /// wood drag `½ ρ Cd d L sin²θ × exposure` (N per (m/s)²),
    /// whether that wood reconfigures (not the stem), crown drag
    /// `½ ρ A0 share` (m² kg/m³), leaf mass and extra mass (kg), wind
    /// profile factor at the midpoint and at the tip.
    wood_base: Vec<f32>,
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
    /// Dynamics of the first sway mode (see [`Dynamics`]).
    dynamics: Dynamics,
    /// Mode shape (displacement along the wind, normalised to 1 at its
    /// maximum) at each segment's midpoint and tip.
    shape_mid: Vec<f32>,
    shape_end: Vec<f32>,
    /// Bending moment vectors at rest (self-weight, undeformed): the
    /// large-deflection model bends the axes by the wind's share only, the
    /// rest geometry being the equilibrium under weight (the sag model of
    /// growth.rs already drew it).
    m0: Vec<Vec3>,
    quasi_static: bool,
    table: ProfileTable,
    /// Rotations of the last large-deflection pass (speed, per segment):
    /// the warm start of the next one (see `MechanicsModel::pass`).
    warm: std::cell::RefCell<Option<(f32, Vec<Vec3>)>>,
}

/// First sway mode of the tree (Rayleigh estimate, see
/// [`MechanicsModel::dynamics`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Dynamics {
    /// Natural frequency of the fundamental sway mode (Hz).
    pub natural_frequency_hz: f32,
    /// Damping ratio in still air (structural + leaves, Jackson et al.
    /// 2019), without the aerodynamic part of a mean wind.
    pub damping_ratio: f32,
    /// Generalised (modal) mass, mode shape normalised to 1 at its
    /// maximum (kg).
    pub modal_mass_kg: f32,
}

/// Deformed geometry of the large-deflection model.
struct Geom {
    start: Vec<Vec3>,
    end: Vec<Vec3>,
    dir: Vec<Vec3>,
    /// Share of the leafy sail still facing the wind: `|R n · n|` with n the
    /// wind direction and R the rotation of the segment.
    sail: Vec<f32>,
}

struct PassOut {
    stem: f32,
    limb: f32,
    anchorage: f32,
    base_moment: f32,
    drag: f32,
    drag_height: f32,
    /// Dynamic factor and total damping ratio of this load case.
    dynamic_factor: f32,
    damping: f32,
    /// Bending moment vector at the base of every segment (N m), kept for
    /// the large-deflection iterations.
    moments: Vec<Vec3>,
    /// (bending moment N m, stress Pa, safety) per segment, when asked.
    detail: Vec<(f32, f32, f32)>,
    /// Horizontal displacement of the stem top (m), deformed geometry.
    top_deflection: f32,
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

/// Turbulence intensity `I_v(z) = 1 / ln(z / z0)` over flat open terrain
/// (EN 1991-1-4:2005, eq. 4.7, turbulence and orography factors 1), with
/// `z` floored at `z_min` = 2 m (Table 4.1, terrain category II).
pub fn turbulence_intensity(z: f32) -> f32 {
    1.0 / (z.max(EC_Z_MIN_M) / ROUGHNESS_LENGTH_M).dln()
}

/// Peak gust over the 10-minute mean wind at height `z`:
/// `√(1 + 7 I_v(z))`, from the peak velocity pressure `q_p = (1 + 7 I_v)
/// q_m` of EN 1991-1-4:2005, eq. 4.8. At 10 m over open ground (z0 =
/// 0.03 m) it is 1.49: a daily maximum gust of 30 m/s comes with a mean
/// wind of ~20 m/s.
pub fn gust_factor(z: f32) -> f32 {
    (1.0 + 7.0 * turbulence_intensity(z)).sqrt()
}

/// Result of [`dynamic_factor`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DynamicFactor {
    /// Dynamic factor `c_d` (peak load with resonance / quasi-static peak
    /// gust load).
    pub cd: f32,
    /// Size factor `c_s` (non-simultaneous gusts over the crown); reported,
    /// not applied (see the module documentation).
    pub cs: f32,
    /// Background and resonant response parts B², R².
    pub background: f32,
    pub resonant: f32,
    /// Peak factor.
    pub peak_factor: f32,
}

/// Structural factor of EN 1991-1-4:2005 (§6.3.1 and Annex B, procedure 1:
/// Davenport's gust response factor for one mode) for a structure of height
/// `h` and width `b` (m), natural frequency `n` (Hz), damping ratio `zeta`
/// (logarithmic decrement δ = 2π ζ) in a mean wind `v_mean_10` (m/s, 10-min
/// mean at 10 m over open ground):
/// * reference height `z_s = 0.6 h` (Fig. 6.1), mean wind there from the
///   logarithmic profile, `I_v(z_s)`;
/// * integral length scale `L = 300 (z_s / 200)^α`, `α = 0.67 + 0.05 ln
///   z0` (eq. B.1);
/// * spectrum `S_L = 6.8 f_L / (1 + 10.2 f_L)^(5/3)`, `f_L = n L / v_m`
///   (eq. B.2);
/// * background `B² = 1 / (1 + 0.9 ((b + h) / L)^0.63)` (eq. B.3);
/// * resonance `R² = π² / (2 δ) S_L R_h R_b`, `R_ℓ = 1/η − (1 − e^(−2η)) /
///   (2 η²)`, `η_h = 4.6 h f_L / L`, `η_b = 4.6 b f_L / L` (eqs. B.6-B.8);
/// * peak factor `k_p = √(2 ln(ν T)) + 0.6 / √(2 ln(ν T))` ≥ 3, `ν = n
///   √(R² / (B² + R²))` ≥ 0.08 Hz, T = 600 s (eqs. B.4-B.5);
/// * `c_s = (1 + 7 I_v B) / (1 + 7 I_v)`, `c_d = (1 + 2 k_p I_v √(B² +
///   R²)) / (1 + 7 I_v B)` (eqs. 6.2-6.3).
///
/// The standard is written for buildings and chimneys; trees enter it as
/// one vertical cantilever with the mode, frequency and damping of
/// [`MechanicsModel::dynamics`]. The formulas are quoted from the standard
/// as we know it (text not re-read in this session: see docs/sources.md).
pub fn dynamic_factor(h: f32, b: f32, n: f32, zeta: f32, v_mean_10: f32) -> DynamicFactor {
    let h = h.max(0.5);
    let b = b.max(0.1);
    let zs = (0.6 * h).max(EC_Z_MIN_M);
    let iv = turbulence_intensity(zs);
    let vm = (v_mean_10 * profile(zs)).max(0.5);
    let alpha = 0.67 + 0.05 * ROUGHNESS_LENGTH_M.dln();
    let l = 300.0 * (zs / 200.0).dpowf(alpha);
    let fl = n.max(0.01) * l / vm;
    let sl = 6.8 * fl / (1.0 + 10.2 * fl).dpowf(5.0 / 3.0);
    let b2 = 1.0 / (1.0 + 0.9 * ((b + h) / l).dpowf(0.63));
    let r = |eta: f32| if eta < 1e-4 { 1.0 } else { 1.0 / eta - (1.0 - (-2.0 * eta).dexp()) / (2.0 * eta * eta) };
    let delta = 2.0 * std::f32::consts::PI * zeta.max(0.005);
    let r2 = std::f32::consts::PI * std::f32::consts::PI / (2.0 * delta) * sl * r(4.6 * h * fl / l) * r(4.6 * b * fl / l);
    let nu = (n * (r2 / (b2 + r2)).sqrt()).max(0.08);
    let x = (2.0 * (nu * EC_PEAK_PERIOD_S).dln()).max(1e-3).sqrt();
    let kp = (x + 0.6 / x).max(3.0);
    let bb = b2.sqrt();
    let cs = (1.0 + 7.0 * iv * bb) / (1.0 + 7.0 * iv);
    let cd = (1.0 + 2.0 * kp * iv * (b2 + r2).sqrt()) / (1.0 + 7.0 * iv * bb);
    DynamicFactor { cd, cs, background: b2, resonant: r2, peak_factor: kp }
}

/// Unit quaternion of the rotation vector `v` (axis × angle), angle below
/// ~0.35 rad: `(cos(a/2), sin(a/2) v/a)` by its Taylor series to third
/// order, renormalised (relative error < 1e-5 at 0.35 rad), without the
/// libm calls of `Quat::from_axis_angle` in the inner loop.
#[inline]
fn small_rotation(v: Vec3) -> Quat {
    let a2 = v.dot(v);
    let w = 1.0 - a2 / 8.0;
    let k = 0.5 * (1.0 - a2 / 24.0);
    let (x, y, z) = (v.x * k, v.y * k, v.z * k);
    let norm = (w * w + x * x + y * y + z * z).sqrt();
    Quat { w: w / norm, x: x / norm, y: y / norm, z: z / norm }
}

/// Wind profile factor and its powers for the two Vogel exponents,
/// tabulated every `PROFILE_TABLE_STEP_M` from the ground up (linear
/// interpolation, a few hundred libm calls per load case instead of
/// several per segment in the large-deflection passes).
struct ProfileTable {
    k: Vec<f32>,
    kb_wood: Vec<f32>,
    kb_crown: Vec<f32>,
}

const PROFILE_TABLE_STEP_M: f32 = 0.25;

impl ProfileTable {
    fn new(top: f32, b_crown: f32) -> Self {
        let n = ((top.max(1.0) + 20.0) / PROFILE_TABLE_STEP_M) as usize + 2;
        let mut t = ProfileTable { k: Vec::with_capacity(n), kb_wood: Vec::with_capacity(n), kb_crown: Vec::with_capacity(n) };
        for i in 0..n {
            let k = profile(i as f32 * PROFILE_TABLE_STEP_M);
            t.k.push(k);
            t.kb_wood.push(k.dpowf(BARE_WOOD_VOGEL_EXPONENT));
            t.kb_crown.push(k.dpowf(b_crown));
        }
        t
    }
    /// (k, k^B_wood, k^B_crown) at height `y`.
    #[inline]
    fn at(&self, y: f32) -> (f32, f32, f32) {
        let x = (y.max(0.0) / PROFILE_TABLE_STEP_M).min((self.k.len() - 2) as f32);
        let i = x as usize;
        let f = x - i as f32;
        let l = |v: &[f32]| v[i] + (v[i + 1] - v[i]) * f;
        (l(&self.k), l(&self.kb_wood), l(&self.kb_crown))
    }
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
        let mut trunk_top = 0;
        if n > 0 {
            let mut cur = 0;
            on_trunk[0] = true;
            while thickest[cur] != NO_PARENT {
                cur = thickest[cur];
                on_trunk[cur] = true;
            }
            trunk_top = cur;
        }
        let structural_d = LIMB_MIN_DIAMETER_M.max(LIMB_DBH_FRACTION * site.dbh_m);
        let leaf_total: f32 = segments.iter().map(|s| s.leaf_weight.max(0.0)).sum();
        let rho = traits.green_density_kg_m3;
        let e = traits.green_moe_gpa * 1e9;
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
            let d4 = d * d * d * d - di * di * di * di;
            let zmod = std::f32::consts::PI * d4 / (32.0 * d);
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
            let flexible = !s.dead && s.leaf_weight <= 0.0 && d >= FLEX_MIN_DIAMETER_M && len > 0.0;
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
                ei: if flexible { e * std::f32::consts::PI * d4 / 64.0 } else { 0.0 },
                bows: flexible && on_trunk[i],
                mass,
                leaf_share,
                role,
                // The stem mode covers every stem section the storms can break
                // (from STEM_BREAK_MIN_DIAMETER_M), so that the critical speeds
                // are those at which a storm breaks the stem: with the 20 %-of-DBH
                // threshold of the limbs, the thin leader of a 60-year spruce
                // (seed 42) broke at 14 m/s while its critical stem gust read 31.6.
                stem_mode: on_trunk[i] && !s.dead && s.leaf_weight <= 0.0 && d >= STEM_BREAK_MIN_DIAMETER_M,
                limb_mode: role == SegmentRole::Limb,
                breakable: (on_trunk[i] && !s.dead && s.leaf_weight <= 0.0 && d >= STEM_BREAK_MIN_DIAMETER_M) || role == SegmentRole::Limb,
            });
        }
        let crown_mid_y = if leaf_w > 0.0 { leaf_y / leaf_w } else { 0.5 * height };
        let trunk = (0..n).filter(|&i| on_trunk[i]).collect();
        MechanicsModel { traits, site, segs, wood_mass, stem_mass, height, crown_mid_y, trunk_top, trunk }
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

    /// Fundamental sway mode for the foliage and extra masses of `load`
    /// (its speed is ignored).
    ///
    /// Rayleigh's method with the static deflection shape: the tree is
    /// loaded sideways (along the wind) by the weight of each of its masses
    /// (wood at the segment midpoints, fresh leaves at the leafy tips,
    /// extra loads), the deflection `u_j` follows from the curvature `M /
    /// (E I)` integrated from the collar along every axis (small rotations,
    /// green MOE), and `ω² = g Σ m u / Σ m u²` (Rayleigh 1877, "The Theory
    /// of Sound"; e.g. Clough & Penzien, "Dynamics of Structures"; for a
    /// uniform cantilever this shape gives a frequency 0.4 % above the exact
    /// one, checked in the tests). Only the stem (its whole path, from
    /// 2.5 cm) and the structural limbs (at least max(5 cm, 20 % of the DBH)
    /// across) bend in this shape: the root-soil plate, crown
    /// branches, twigs, leafy shoots and dead wood are rigid bodies carried
    /// by them, which makes the estimate an upper bound. Why: with every
    /// axis from 2.5 cm flexible, the static shape is dominated by the tips
    /// of slender crown branches (their own, local modes), not by the sway
    /// of the stem that pull-and-release tests measure (Jackson et al. 2019);
    /// the leaves at those tips then weighed far too much in the mode
    /// (winter / summer frequency +30 to +48 % for the oak and the cherry,
    /// against +18 to +19 % measured; +14 % once only the structural axes
    /// bend: docs/sources.md). Damping in still air: `damping_leafless + leaf
    /// fraction x damping_leaves` (traits, Jackson et al. 2019).
    pub fn dynamics(&self, load: &WindLoad) -> Dynamics {
        self.prepare(load).dynamics
    }

    /// Mode shape (unnormalised displacement along `wind` at midpoints and
    /// tips) and Rayleigh frequency for the given leaf and extra masses.
    fn rayleigh(&self, wind: Vec3, leaf_mass: &[f32], extra: &[f32]) -> (Vec<f32>, Vec<f32>, f32, f32) {
        let n = self.segs.len();
        let mut fsum = vec![Vec3::ZERO; n];
        let mut qsum = vec![Vec3::ZERO; n];
        for (i, s) in self.segs.iter().enumerate() {
            let fw = wind * (GRAVITY * (s.mass + extra[i]));
            let fl = wind * (GRAVITY * leaf_mass[i]);
            fsum[i] = fw + fl;
            qsum[i] = s.mid.cross(fw) + s.end.cross(fl);
        }
        for i in (1..n).rev() {
            let pi = self.segs[i].parent;
            if pi != NO_PARENT {
                let (f, q) = (fsum[i], qsum[i]);
                fsum[pi] += f;
                qsum[pi] += q;
            }
        }
        // Small-rotation kinematics: rotation vector θ accumulated along the
        // axes, displacement u of a point = u(base) + θ(base) × (p − base).
        let mut theta = vec![Vec3::ZERO; n];
        let mut ustart = vec![Vec3::ZERO; n];
        let mut mid = vec![0.0f32; n];
        let mut end = vec![0.0f32; n];
        for (i, s) in self.segs.iter().enumerate() {
            let (tp, up, sp) = if s.parent == NO_PARENT { (Vec3::ZERO, Vec3::ZERO, s.start) } else { (theta[s.parent], ustart[s.parent], self.segs[s.parent].start) };
            let u0 = up + tp.cross(s.start - sp);
            let mut t = tp;
            if s.ei > 0.0 && (s.role == SegmentRole::Trunk || s.limb_mode) {
                let m = qsum[i] - s.start.cross(fsum[i]);
                let mb = m - s.dir * m.dot(s.dir);
                t += mb * (s.len / s.ei);
            }
            let ue = u0 + t.cross(s.end - s.start);
            let um = (u0 + ue) * 0.5;
            theta[i] = t;
            ustart[i] = u0;
            mid[i] = um.dot(wind);
            end[i] = ue.dot(wind);
        }
        let (mut a, mut b) = (0.0f64, 0.0f64);
        for (i, s) in self.segs.iter().enumerate() {
            let (mw, ml) = ((s.mass + extra[i]) as f64, leaf_mass[i] as f64);
            a += mw * mid[i] as f64 + ml * end[i] as f64;
            b += mw * (mid[i] as f64) * (mid[i] as f64) + ml * (end[i] as f64) * (end[i] as f64);
        }
        let freq = if a > 0.0 && b > 0.0 { ((GRAVITY as f64 * a / b).sqrt() / std::f64::consts::TAU) as f32 } else { 0.0 };
        let umax = mid.iter().chain(end.iter()).fold(0.0f32, |m, &x| m.max(x.abs())).max(1e-30);
        let mut mgen = 0.0f64;
        for (i, s) in self.segs.iter().enumerate() {
            let (pm, pe) = ((mid[i] / umax) as f64, (end[i] / umax) as f64);
            mgen += (s.mass + extra[i]) as f64 * pm * pm + leaf_mass[i] as f64 * pe * pe;
        }
        for x in mid.iter_mut().chain(end.iter_mut()) {
            *x /= umax;
        }
        (mid, end, freq, mgen as f32)
    }

    fn prepare(&self, load: &WindLoad) -> Prepared {
        self.prepare_with(load, true)
    }

    /// [`Self::prepare`]; without `sway`, the first mode is not estimated
    /// (quasi-static loads only: frequency 0, dynamic factor 1).
    fn prepare_with(&self, load: &WindLoad, sway: bool) -> Prepared {
        let n = self.segs.len();
        let f = load.leaf_fraction.unwrap_or(self.site.leaf_fraction).clamp(0.0, 1.0);
        let (sa, ca) = load.azimuth_deg.to_radians().dsin_cos();
        let wind = Vec3::new(ca, 0.0, sa);
        let across = Vec3::new(-sa, 0.0, ca);
        // Silhouette of the crown on the plane normal to the wind, one light
        // voxel per cell (the resolution at which the simulation knows its
        // crown): a cell belongs to the leafy envelope when a leafy shoot
        // tip projects into it. Every segment is binned once (leafy shoots
        // by their tip, crown wood by its midpoint; the stem is not binned)
        // on a dense grid over their bounding box, sums taken in segment
        // order (deterministic). O(n + cells).
        let cell = self.site.crown_cell_m.max(0.05);
        let binned = |s: &Seg| s.leaf_share > 0.0 || s.role != SegmentRole::Trunk;
        let key = |s: &Seg| {
            let p = if s.leaf_share > 0.0 { s.end } else { s.mid };
            ((p.dot(across) / cell).floor() as i32, (p.y / cell).floor() as i32)
        };
        let (mut u0, mut u1, mut w0, mut w1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for s in self.segs.iter().filter(|s| binned(s)) {
            let (u, w) = key(s);
            (u0, u1, w0, w1) = (u0.min(u), u1.max(u), w0.min(w), w1.max(w));
        }
        let nu = if u1 >= u0 { (u1 - u0 + 1) as usize } else { 0 };
        let nw = if w1 >= w0 { (w1 - w0 + 1) as usize } else { 0 };
        let cell_of: Vec<usize> = self
            .segs
            .iter()
            .map(|s| {
                if binned(s) {
                    let (u, w) = key(s);
                    (u - u0) as usize * nw + (w - w0) as usize
                } else {
                    usize::MAX
                }
            })
            .collect();
        // Per cell: leaf share of the tips, projected area of the crown wood
        // (`d L |sin θ|`, m²) and its rigid cross-flow drag `½ ρ Cd d L sin²θ`.
        let mut leaf_in = vec![0.0f32; nu * nw];
        let mut wood_area = vec![0.0f32; nu * nw];
        let mut wood_drag = vec![0.0f32; nu * nw];
        for (s, &c) in self.segs.iter().zip(&cell_of) {
            if c == usize::MAX {
                continue;
            }
            if s.leaf_share > 0.0 {
                leaf_in[c] += s.leaf_share;
            }
            if s.role != SegmentRole::Trunk {
                let cth = s.dir.dot(wind);
                let sin2 = (1.0 - cth * cth).max(0.0);
                wood_area[c] += s.d * s.len * sin2.sqrt();
                wood_drag[c] += 0.5 * AIR_DENSITY * WOOD_DRAG_COEFFICIENT * s.d * s.len * sin2;
            }
        }
        // Still-air frontal area of the leafy crown: the envelope cells.
        let cell_area = cell * cell;
        let full_area = leaf_in.iter().filter(|&&l| l > 0.0).count() as f32 * cell_area;
        let crown_area = full_area * f;
        // Width of the leafy silhouette across the wind (dynamic factor).
        let (mut lu0, mut lu1) = (i32::MAX, i32::MIN);
        for (k, &l) in leaf_in.iter().enumerate() {
            if l > 0.0 {
                let u = (k / nw.max(1)) as i32;
                (lu0, lu1) = (lu0.min(u), lu1.max(u));
            }
        }
        let crown_width = if lu1 >= lu0 { (lu1 - lu0 + 1) as f32 * cell } else { self.segs.iter().map(|s| s.d).fold(0.0, f32::max) };
        // Crown wood: its exposure per cell (see `CROWN_WOOD_*` and
        // docs/sources.md). (1) Mutual hiding of the elements of one cell,
        // random overlap (Beer-Lambert): projected area a on a cell of area
        // S hides `S (1 − e^(−a/S))`, i.e. a factor `(1 − e^(−x)) / x`
        // (estimated: real twigs are clumped, not random, and optical hiding
        // is not the aerodynamic shelter of wakes).
        // (2) Inside the leafy envelope, the drag of the bare wood of a cell
        // is capped by the drag of the same cell in full leaf at the same
        // speed (`CROWN_WOOD_LEAF_CAP`); the cap is taken at the speed where
        // the ratio of the two reconfiguration laws is largest, so it holds
        // at every speed. Wood outside the envelope (limbs below the leaves,
        // wood in gaps of the crown) is never sheltered by leaves.
        let rw = |u: f32| (u / RECONF_MIN_MS).dpowf(BARE_WOOD_VOGEL_EXPONENT);
        let rc = |u: f32| (u / CROWN_CD_REFERENCE_MS).dpowf(self.traits.vogel_exponent);
        let worst = (rw(RECONF_MIN_MS) / rc(RECONF_MIN_MS)).max(rw(RECONF_MAX_MS) / rc(RECONF_MAX_MS));
        let leaf_cell_drag = 0.5 * AIR_DENSITY * self.traits.crown_drag_coefficient * cell_area;
        let wood_factor: Vec<f32> = (0..nu * nw)
            .map(|c| {
                let x = wood_area[c] / cell_area;
                let hide = if x > 1e-6 { (1.0 - (-x).dexp()) / x } else { 1.0 };
                let drag = wood_drag[c] * hide * worst;
                if leaf_in[c] > 0.0 && drag > 0.0 {
                    hide * (CROWN_WOOD_LEAF_CAP * leaf_cell_drag / drag).min(1.0)
                } else {
                    hide
                }
            })
            .collect();
        // Fresh leaf mass (kg/m²): the species' LMA (g dry / m²) / LDMC.
        let leaf_kg_m2 = self.site.leaf_lma_g_m2 * 1e-3 / LEAF_DRY_MATTER_CONTENT;
        let leaf_kg = self.site.leaf_area_m2 * f * leaf_kg_m2;
        let mut p = Prepared {
            wind,
            leaf_fraction: f,
            crown_area,
            crown_width,
            wood_base: Vec::with_capacity(n),
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
            dynamics: Dynamics::default(),
            shape_mid: Vec::new(),
            shape_end: Vec::new(),
            m0: Vec::new(),
            quasi_static: load.quasi_static,
            table: ProfileTable::new(if load.quasi_static { 0.0 } else { self.height }, self.traits.vogel_exponent),
            warm: std::cell::RefCell::new(None),
        };
        for (i, s) in self.segs.iter().enumerate() {
            let c = s.dir.dot(wind);
            let sin2 = (1.0 - c * c).max(0.0);
            // In leaf, the crown drag coefficient (measured on whole crowns)
            // already includes the branches inside the crown: the wood of an
            // envelope cell only counts for the leafless share. The stem is
            // always exposed; crown wood outside the envelope always.
            let cid = cell_of[i];
            let exposure = if s.role == SegmentRole::Trunk {
                1.0
            } else if leaf_in[cid] > 0.0 {
                (1.0 - f) * wood_factor[cid]
            } else {
                wood_factor[cid]
            };
            let base = 0.5 * AIR_DENSITY * WOOD_DRAG_COEFFICIENT * s.d * s.len * exposure;
            p.wood_base.push(base);
            p.wood_k.push(base * sin2);
            p.wood_bends.push(s.role != SegmentRole::Trunk);
            // Crown drag by frontal area: each envelope cell carries
            // `½ ρ Cd(U) S f`, shared among the leafy tips of the cell in
            // proportion to their foliage (tips behind one another along the
            // wind shelter each other), at the height of each tip.
            let ck = if s.leaf_share > 0.0 && cid != usize::MAX { 0.5 * AIR_DENSITY * cell_area * f * s.leaf_share / leaf_in[cid] } else { 0.0 };
            p.crown_k.push(ck);
            p.leaf_mass.push(leaf_kg * s.leaf_share);
            p.extra.push(load.extra_mass_kg.get(i).copied().unwrap_or(0.0).max(0.0));
            // Exact profile for the quasi-static model (hand-checked tests),
            // the interpolated table otherwise (cost).
            let (km, kmw, ke, kec) = if load.quasi_static {
                let (km, ke) = (profile(s.mid.y), profile(s.end.y));
                (km, km.dpowf(BARE_WOOD_VOGEL_EXPONENT), ke, ke.dpowf(self.traits.vogel_exponent))
            } else {
                let (km, kmw, _) = p.table.at(s.mid.y);
                let (ke, _, kec) = p.table.at(s.end.y);
                (km, kmw, ke, kec)
            };
            p.prof_mid.push(km);
            p.prof_end.push(ke);
            p.prof_mid_b.push(if p.wood_bends[i] { kmw } else { 1.0 });
            p.prof_end_b.push(if s.leaf_share > 0.0 { kec } else { 1.0 });
        }
        let (mid, end, freq, mgen) = if sway { self.rayleigh(p.wind, &p.leaf_mass, &p.extra) } else { (vec![0.0; n], vec![0.0; n], 0.0, 0.0) };
        let t = &self.traits;
        p.dynamics = Dynamics { natural_frequency_hz: freq, damping_ratio: t.damping_leafless + f * t.damping_leaves, modal_mass_kg: mgen };
        p.shape_mid = mid;
        p.shape_end = end;
        if !p.quasi_static {
            p.m0 = self.pass_on(&p, 0.0, None, false).moments;
        }
        p
    }

    /// Dynamic factor and total damping at 10 m gust speed `u`, given the
    /// quasi-static modal drag `s1 = Σ F_j φ_j²` (N) of the pass.
    fn dynamic_at(&self, p: &Prepared, u: f32, s1: f32) -> (f32, f32) {
        let dynm = &p.dynamics;
        if p.quasi_static || u <= 0.0 || dynm.natural_frequency_hz <= 0.0 || dynm.modal_mass_kg <= 0.0 {
            return (1.0, dynm.damping_ratio);
        }
        // Mean wind of the gust (EN 1991-1-4 eq. 4.8, see `gust_factor`).
        let vm = u / gust_factor(REFERENCE_HEIGHT_M);
        // Aerodynamic damping of the sway in that mean wind: the drag of a
        // body moving at q̇ in a mean wind V varies by dF/dV q̇, i.e. a
        // modal damping `c = Σ dF_j/dV φ_j²` and `ζ_a = c / (4π n M)` (the
        // reasoning of EN 1991-1-4 Annex F, eq. F.18, for one mode). Drag
        // grows as V^(2+B) (B: Vogel exponent of the reconfiguration range,
        // leaf-weighted; 0 outside it), so `dF/dV = (2 + B) F / V`, with F
        // the modal drag scaled from the gust to the mean wind. Estimated
        // reduction of a measured quantity: no tree measurement of ζ_a in
        // storms was found.
        let uc = vm * profile(self.crown_mid_y);
        let b = if (RECONF_MIN_MS..RECONF_MAX_MS).contains(&uc) { p.leaf_fraction * self.traits.vogel_exponent + (1.0 - p.leaf_fraction) * BARE_WOOD_VOGEL_EXPONENT } else { 0.0 };
        let scale = (vm / u).dpowf(2.0 + b);
        let za = (2.0 + b) * s1 * scale / uc.max(0.5) / (4.0 * std::f32::consts::PI * dynm.natural_frequency_hz * dynm.modal_mass_kg);
        let zeta = (dynm.damping_ratio + za).min(1.0);
        let df = dynamic_factor(self.height, p.crown_width, dynm.natural_frequency_hz, zeta, vm);
        (df.cd, zeta)
    }

    /// One O(n) pass at 10 m wind speed `u` on the rest geometry (`geom`
    /// None) or a deformed one.
    fn pass_on(&self, p: &Prepared, u: f32, geom: Option<&Geom>, detail: bool) -> PassOut {
        let n = self.segs.len();
        let mut fsum = vec![Vec3::ZERO; n];
        let mut qsum = vec![Vec3::ZERO; n];
        let mut wf = vec![Vec3::ZERO; if u > 0.0 { n } else { 0 }];
        let mut wq = vec![Vec3::ZERO; if u > 0.0 { n } else { 0 }];
        let (mut drag, mut drag_h, mut s1) = (0.0f32, 0.0f32, 0.0f32);
        let (ub_wood, ub_crown) = if u > 0.0 {
            ((u / p.wood_reconf.u_ref).dpowf(p.wood_reconf.b), (u / p.crown_reconf.u_ref).dpowf(p.crown_reconf.b))
        } else {
            (1.0, 1.0)
        };
        let cd_ref = self.traits.crown_drag_coefficient;
        let down = Vec3::new(0.0, -GRAVITY, 0.0);
        for (i, s) in self.segs.iter().enumerate() {
            let (start, end, dir) = match geom {
                Some(g) => (g.start[i], g.end[i], g.dir[i]),
                None => (s.start, s.end, s.dir),
            };
            let mid = (start + end) * 0.5;
            let fw = down * (s.mass + p.extra[i]);
            let mut f = fw;
            let mut q = mid.cross(fw);
            if p.leaf_mass[i] > 0.0 {
                let fl = down * p.leaf_mass[i];
                f += fl;
                q += end.cross(fl);
            }
            fsum[i] = f;
            qsum[i] = q;
            if u > 0.0 {
                let (km, ke, kmb, keb, wood_k, sail) = match geom {
                    None => (p.prof_mid[i], p.prof_end[i], p.prof_mid_b[i], p.prof_end_b[i], p.wood_k[i], 1.0),
                    Some(g) => {
                        let (km, kmw, _) = p.table.at(mid.y);
                        let (ke, _, kec) = p.table.at(end.y);
                        let c = dir.dot(p.wind);
                        let kmb = if p.wood_bends[i] { kmw } else { 1.0 };
                        let keb = if p.crown_k[i] > 0.0 { kec } else { 1.0 };
                        (km, ke, kmb, keb, p.wood_base[i] * (1.0 - c * c).max(0.0), g.sail[i])
                    }
                };
                let um = u * km;
                let reconf = if p.wood_bends[i] { p.wood_reconf.at(um, ub_wood, kmb) } else { 1.0 };
                let fwood = wood_k * reconf * um * um;
                let (mut vf, mut vq) = (Vec3::ZERO, Vec3::ZERO);
                if fwood > 0.0 {
                    let v = p.wind * fwood;
                    vf += v;
                    vq += mid.cross(v);
                    drag += fwood;
                    drag_h += fwood * mid.y;
                    s1 += fwood * p.shape_mid[i] * p.shape_mid[i];
                }
                if p.crown_k[i] > 0.0 {
                    let ue = u * ke;
                    let fc = p.crown_k[i] * cd_ref * p.crown_reconf.at(ue, ub_crown, keb) * ue * ue * sail;
                    let v = p.wind * fc;
                    vf += v;
                    vq += end.cross(v);
                    drag += fc;
                    drag_h += fc * end.y;
                    s1 += fc * p.shape_end[i] * p.shape_end[i];
                }
                wf[i] = vf;
                wq[i] = vq;
            }
        }
        let (dyn_f, damping) = if u > 0.0 { self.dynamic_at(p, u, s1) } else { (1.0, p.dynamics.damping_ratio) };
        if u > 0.0 {
            for ((f, q), (a, b)) in fsum.iter_mut().zip(qsum.iter_mut()).zip(wf.iter().zip(wq.iter())) {
                *f += *a * dyn_f;
                *q += *b * dyn_f;
            }
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
        let mut moments = Vec::with_capacity(if p.quasi_static { 0 } else { n });
        let mut base_moment = 0.0;
        for (i, s) in self.segs.iter().enumerate() {
            let start = geom.map_or(s.start, |g| g.start[i]);
            let m = qsum[i] - start.cross(fsum[i]);
            if !p.quasi_static {
                moments.push(m);
            }
            if i == 0 {
                base_moment = (m.x * m.x + m.z * m.z).sqrt();
            }
            if !(detail || s.stem_mode || s.limb_mode) {
                continue;
            }
            let dir = geom.map_or(s.dir, |g| g.dir[i]);
            let mb = m - dir * m.dot(dir);
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
        let anchorage = if base_moment > 0.0 && self.can_uproot() { mcrit / base_moment } else { f32::INFINITY };
        let top_deflection = geom.map_or(0.0, |g| {
            let t = self.trunk_top.min(n.saturating_sub(1));
            let d = g.end[t] - self.segs[t].end;
            (d.x * d.x + d.z * d.z).sqrt()
        });
        PassOut { stem, limb, anchorage, base_moment, drag, drag_height: drag_h, dynamic_factor: dyn_f, damping, moments, detail: out, top_deflection }
    }

    /// Deformed geometry for the bending moments of a pass: every flexible
    /// segment turns by `(M − M_rest)_bending L / (E I)` (the wind's share
    /// of its curvature), rotations compose from the collar outwards,
    /// rigidly carrying everything beyond. Under-relaxed against the
    /// previous iteration's rotations (`theta`, updated). Returns the
    /// geometry, the largest change of rotation (rad) and the largest
    /// accumulated rotation (rad).
    fn deform(&self, p: &Prepared, moments: &[Vec3], theta: &mut [Vec3]) -> (Geom, f32, f32) {
        self.deform_relaxed(p, moments, theta, LARGE_DEFLECTION_RELAX)
    }

    /// [`Self::deform`] with the under-relaxation factor `relax` (0: the
    /// geometry of the rotations `theta` as they are, `moments` unused).
    fn deform_relaxed(&self, p: &Prepared, moments: &[Vec3], theta: &mut [Vec3], relax: f32) -> (Geom, f32, f32) {
        let n = self.segs.len();
        let mut g = Geom { start: Vec::with_capacity(n), end: Vec::with_capacity(n), dir: Vec::with_capacity(n), sail: Vec::with_capacity(n) };
        let mut rot = vec![Quat::IDENTITY; n];
        let (mut change, mut total) = (0.0f32, 0.0f32);
        for (i, s) in self.segs.iter().enumerate() {
            let mut th = Vec3::ZERO;
            if s.bows && relax > 0.0 {
                let dm = moments[i] - p.m0[i];
                let mb = dm - s.dir * dm.dot(s.dir);
                th = mb * (s.len / s.ei);
                let l = th.length();
                if l > LARGE_DEFLECTION_MAX_SEG_RAD {
                    th = th * (LARGE_DEFLECTION_MAX_SEG_RAD / l);
                }
            }
            let new = theta[i] * (1.0 - relax) + th * relax;
            change = change.max((new - theta[i]).length());
            theta[i] = new;
            let (rp, sp_new, sp_old) = if s.parent == NO_PARENT { (Quat::IDENTITY, s.start, s.start) } else { (rot[s.parent], g.start[s.parent], self.segs[s.parent].start) };
            let q = if new.dot(new) > 1e-18 { small_rotation(new).mul(rp) } else { rp };
            let start = sp_new + rp.rotate(s.start - sp_old);
            let end = start + q.rotate(s.end - s.start);
            let nw = q.rotate(p.wind);
            // Rotation from the rest orientation: 2 |sin(angle/2)| ~ angle.
            total = total.max(2.0 * (q.x * q.x + q.y * q.y + q.z * q.z).sqrt());
            rot[i] = q;
            g.start.push(start);
            g.end.push(end);
            g.dir.push(q.rotate(s.dir));
            g.sail.push(nw.dot(p.wind).abs());
        }
        (g, change, total)
    }

    /// The pass at `u` with large deflections: the rest-geometry pass, then,
    /// when the axes turn by more than [`LARGE_DEFLECTION_MIN_RAD`], a few
    /// fixed-point iterations "loads → bending → deformed geometry →
    /// loads" (geometrically nonlinear statics). A flexible young stem
    /// bows: its crown drops into slower air, its lever arm shortens and
    /// its leafy sail turns away from the wind (the drag reduction of
    /// flexible bodies by reconfiguration: de Langre 2008, "Effects of wind
    /// on plants", Annu Rev Fluid Mech 40:141-168; Alben, Shelley & Zhang
    /// 2002, Nature 420:479-481), while the weight of the displaced crown
    /// adds its own moment (P-Δ). The net gain is large only when the top
    /// moves by a large part of the height (a 6 m, 4 cm pole: base moment
    /// halved); for moderate deflections the two effects nearly cancel
    /// (tests). Stiff adult stems hardly move and keep the rest-geometry
    /// result.
    fn pass(&self, p: &Prepared, u: f32, detail: bool) -> PassOut {
        let first = self.pass_on(p, u, None, detail);
        if p.quasi_static || u <= 0.0 {
            return first;
        }
        // Cheap check along the stem path only (small rotations): the
        // accumulated rotation and the top displacement.
        let (mut rot, mut disp, mut total) = (Vec3::ZERO, Vec3::ZERO, 0.0f32);
        for &i in &self.trunk {
            let s = &self.segs[i];
            if s.bows {
                let dm = first.moments[i] - p.m0[i];
                rot += (dm - s.dir * dm.dot(s.dir)) * (s.len / s.ei);
            }
            disp += rot.cross(s.end - s.start);
            total = total.max(rot.length());
        }
        if total < LARGE_DEFLECTION_MIN_RAD {
            // Small deflection: rest geometry.
            let mut out = first;
            out.top_deflection = (disp.x * disp.x + disp.z * disp.z).sqrt();
            return out;
        }
        // Warm start: the rotations of the last large-deflection pass of
        // this load case (a speed search evaluates nearby speeds one after
        // the other), scaled by (u / u_last)^2, the growth of the wind's
        // moment; the iterations then start near their fixed point.
        let warm = p.warm.borrow().clone();
        let (mut theta, mut g) = match warm {
            Some((u_last, th)) if th.len() == self.segs.len() && u_last > 0.0 => {
                let k = ((u / u_last) * (u / u_last)).min(4.0);
                let mut th: Vec<Vec3> = th.into_iter().map(|t| t * k).collect();
                let (g, _, _) = self.deform_relaxed(p, &first.moments, &mut th, 0.0);
                (th, g)
            }
            _ => {
                let mut th = vec![Vec3::ZERO; self.segs.len()];
                let (g, _, _) = self.deform_relaxed(p, &first.moments, &mut th, 1.0);
                (th, g)
            }
        };
        let mut out = first;
        for _ in 0..LARGE_DEFLECTION_ITERATIONS {
            out = self.pass_on(p, u, Some(&g), detail);
            let (g2, change, _) = self.deform(p, &out.moments, &mut theta);
            g = g2;
            if change < LARGE_DEFLECTION_TOL_RAD {
                break;
            }
        }
        *p.warm.borrow_mut() = Some((u, theta));
        out
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
            natural_frequency_hz: p.dynamics.natural_frequency_hz,
            damping_ratio: o.damping,
            dynamic_factor: o.dynamic_factor,
            top_deflection_m: o.top_deflection,
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
            snap.segment_breakable.push(s.breakable);
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

    /// Whether uprooting is checked for this tree (collar of at least
    /// [`BREAK_MIN_DIAMETER_M`]).
    pub fn can_uproot(&self) -> bool {
        self.site.dbh_m >= UPROOT_MIN_DBH_M
    }

    /// Bending stress (Pa) of every bending axis (from
    /// [`FLEX_MIN_DIAMETER_M`]; 0 elsewhere) under `load` (weight + wind,
    /// with dynamics and large deflections unless `quasi_static`, then
    /// without estimating the sway mode), and whether each segment lies
    /// on the stem path: the load case of the wind acclimation of growth.rs
    /// (`crate::wind`).
    pub fn stem_stresses(&self, load: &WindLoad) -> (Vec<f32>, Vec<bool>) {
        let p = self.prepare_with(load, !load.quasi_static);
        let o = self.pass(&p, load.speed_10m_ms.max(0.0), true);
        let stress = self.segs.iter().zip(&o.detail).map(|(s, d)| if s.ei > 0.0 { d.1 } else { 0.0 }).collect();
        let trunk = self.segs.iter().map(|s| s.role == SegmentRole::Trunk).collect();
        (stress, trunk)
    }

    /// Lowest safety factor over the three modes at speed `u` (one pass).
    pub fn min_safety_at(&self, load: &WindLoad) -> f32 {
        let p = self.prepare(load);
        let o = self.pass(&p, load.speed_10m_ms.max(0.0), false);
        o.stem.min(o.limb).min(o.anchorage)
    }

    /// Lowest 10 m gust speed (m/s) at which any mode fails, to `tol` m/s,
    /// for the direction, foliage and extra masses of `load`: a scan in
    /// steps of 4 m/s then a bisection (≈ 15 passes instead of the ≈ 80 of
    /// [`Self::critical_speeds`]). `None` beyond [`MAX_SEARCH_SPEED_MS`].
    pub fn first_critical_speed(&self, load: &WindLoad, tol: f32) -> Option<f32> {
        let p = self.prepare(load);
        let eval = |u: f32| {
            let o = self.pass(&p, u, false);
            o.stem.min(o.limb).min(o.anchorage)
        };
        if eval(0.0) < 1.0 {
            return Some(0.0);
        }
        let mut u0 = 0.0f32;
        while u0 < MAX_SEARCH_SPEED_MS {
            let u1 = (u0 + 4.0).min(MAX_SEARCH_SPEED_MS);
            if eval(u1) < 1.0 {
                let (mut lo, mut hi) = (u0, u1);
                while hi - lo > tol.max(0.01) {
                    let mid = 0.5 * (lo + hi);
                    if eval(mid) < 1.0 {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                return Some(hi);
            }
            u0 = u1;
        }
        None
    }

    /// Lowest speed in `[lo, hi]` where the safety `f(u)` drops below 1,
    /// given `f(lo) >= 1 > f(hi)`, to `CRITICAL_SPEED_TOL_MS`: regula falsi
    /// on ln f with
    /// the Illinois modification (safety varies smoothly, roughly as a
    /// power of the speed), falling back to a bisection step whenever the
    /// interpolated point is not well inside the bracket. Returns the
    /// failing end of the final bracket, as the plain bisection did.
    fn root_in(f: impl Fn(f32) -> f32, mut lo: f32, f_lo: f32, mut hi: f32, f_hi: f32) -> f32 {
        let tol = CRITICAL_SPEED_TOL_MS;
        let g = |x: f32| if x.is_finite() { x.max(1e-6).ln() } else { 10.0 };
        let (mut glo, mut ghi) = (g(f_lo), g(f_hi));
        // Consecutive updates of the same end: after two, a bisection step
        // (the safeguard that keeps regula falsi from crawling on one side).
        let (mut side, mut same) = (0i8, 0u32);
        for _ in 0..60 {
            if hi - lo <= tol {
                break;
            }
            let w = hi - lo;
            let falsi = if glo > ghi { lo + w * glo / (glo - ghi) } else { lo + 0.5 * w };
            let mut x = if same >= 2 || !(falsi > lo && falsi < hi) { lo + 0.5 * w } else { falsi };
            // Never closer to an end than a fraction of the tolerance.
            x = x.clamp(lo + 0.1 * tol.min(w), hi - 0.1 * tol.min(w));
            let gx = g(f(x));
            let s = if gx < 0.0 { -1 } else { 1 };
            same = if s == side { same + 1 } else { 0 };
            side = s;
            if gx < 0.0 {
                hi = x;
                ghi = gx;
            } else {
                lo = x;
                glo = gx;
            }
        }
        hi
    }

    /// Critical 10 m gust speed of each failure mode for the direction,
    /// foliage and extra masses of `load` (its speed is ignored).
    pub fn critical_speeds(&self, load: &WindLoad) -> CriticalSpeeds {
        let p = self.prepare(load);
        // Every evaluation is kept: the search of one mode narrows the
        // brackets of the others.
        let memo: std::cell::RefCell<Vec<(f32, [f32; 3])>> = std::cell::RefCell::new(Vec::new());
        let eval = |u: f32| {
            let o = self.pass(&p, u, false);
            let r = [o.stem, o.limb, o.anchorage];
            memo.borrow_mut().push((u, r));
            r
        };
        let mut found: [Option<f32>; 3] = [None; 3];
        let first = eval(0.0);
        for k in 0..3 {
            if first[k] < 1.0 {
                found[k] = Some(0.0);
            }
        }
        // Scan upwards, by steps of SCAN_STEP_MS or more: from a speed u
        // where every mode still to find has safety >= s, no mode can fail
        // below `u sqrt(s / SCAN_SLACK)` (the wind's moment grows at most as
        // U^2, the Vogel exponents being <= 0, the weight's moment does not
        // grow; SCAN_SLACK covers the growth of the dynamic factor and the
        // P-Delta moment of the large deflections with speed). Then a
        // bisection in the bracket of each mode.
        let mut u0 = 0.0f32;
        let mut s0 = first;
        while u0 < MAX_SEARCH_SPEED_MS && found.iter().any(|f| f.is_none()) {
            let s_min = (0..3).filter(|&k| found[k].is_none()).map(|k| s0[k]).fold(f32::INFINITY, f32::min);
            let jump = if u0 > 0.0 && s_min.is_finite() && s_min > SCAN_SLACK { u0 * (s_min / SCAN_SLACK).sqrt() } else { 0.0 };
            let u1 = (u0 + SCAN_STEP_MS).max(jump).min(MAX_SEARCH_SPEED_MS);
            let cur = eval(u1);
            for k in 0..3 {
                if found[k].is_none() && cur[k] < 1.0 {
                    let (mut lo, mut flo, mut hi, mut fhi) = (u0, s0[k], u1, cur[k]);
                    for &(u, r) in memo.borrow().iter() {
                        if u > lo && u < hi {
                            if r[k] >= 1.0 {
                                (lo, flo) = (u, r[k]);
                            } else {
                                (hi, fhi) = (u, r[k]);
                            }
                        }
                    }
                    found[k] = Some(Self::root_in(|u| eval(u)[k], lo, flo, hi, fhi));
                }
            }
            u0 = u1;
            s0 = cur;
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
        self.mechanics_inputs_at(self.shoots.ext)
    }

    /// [`Self::mechanics_inputs`] with the year's new shoots drawn at
    /// extension `ext` (0..1) instead of today's.
    pub(crate) fn mechanics_inputs_at(&self, ext: f32) -> (Vec<MechSegment>, MechSite) {
        let nodes = &self.shoots.nodes;
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
