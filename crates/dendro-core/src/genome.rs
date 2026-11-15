//! Species trait genome.
//!
//! A [`Genome`] is a flat, serialisable set of measurable traits. Every
//! species-specific number used by the simulation lives here, so a new
//! species (e.g. *Prunus avium*) is obtained by writing a new RON file, not by
//! changing code. Units are given in each field's doc comment.

use serde::{Deserialize, Serialize};
use std::fmt;

/// RON source of the built-in pedunculate oak genome
/// (`data/species/quercus_robur.ron`).
pub const QUERCUS_ROBUR_RON: &str = include_str!("../../../data/species/quercus_robur.ron");

/// Complete trait set of a species.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Genome {
    /// Latin binomial.
    pub species: String,
    /// English common name.
    pub common_name: String,
    pub architecture: Architecture,
    pub leaf: LeafTraits,
    pub pigments: Pigments,
    pub phenology: PhenologyTraits,
    pub wood: WoodTraits,
    pub roots: RootTraits,
    pub tolerances: Tolerances,
}

/// Above-ground architecture (bud/shoot model, after Palubicki et al. 2009).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Architecture {
    /// Apical control lambda of the Borchert-Honda resource split in young
    /// trees (0.5 = neutral, >0.5 favours the main axis).
    pub apical_dominance_young: f32,
    /// Apical control lambda reached at maturity (decurrent crowns -> lower).
    pub apical_dominance_mature: f32,
    /// Apical control lambda of branch axes (order >= 1), constant with
    /// age. The young/mature schedule above applies to the stem only.
    pub apical_dominance_branches: f32,
    /// Age (years) at which lambda reaches its mature value.
    pub maturity_age_years: f32,
    /// Age (years) until which lambda keeps its juvenile value (juvenile
    /// excurrent phase); it then declines smoothly to the mature value at
    /// `maturity_age_years`.
    pub juvenile_phase_years: f32,
    /// Sharpness of the vigour split among sibling laterals: each lateral
    /// (or dormant bud) receives a share proportional to `Q^k` of its light
    /// `Q`. 1 = Borchert-Honda (proportional); >1 = the best-lit branches
    /// take more than their share (strong competition between branches,
    /// few dominant scaffold limbs); <1 = even, many similar branches.
    pub lateral_competition: f32,
    /// Vigour (metamers) produced per unit of bud light exposure (alpha).
    pub vigour_per_light: f32,
    /// Minimum vigour (metamers) for a bud to break.
    pub bud_break_threshold: f32,
    /// Length of one metamer on a vigorous shoot (m).
    pub internode_length_m: f32,
    /// Longest annual shoot extension (m).
    pub max_shoot_length_m: f32,
    /// Maximum number of lateral buds formed on one annual shoot.
    pub lateral_buds_per_shoot: u8,
    /// Lateral buds formed per metamer of a shoot (before the cap above).
    pub lateral_buds_per_metamer: f32,
    /// Years a dormant bud stays viable.
    pub bud_lifetime_years: u32,
    /// Insertion angle of lateral shoots relative to the parent axis (deg).
    pub branching_angle_deg: f32,
    /// Divergence angle between successive buds (deg); 144 = 2/5 phyllotaxis.
    pub phyllotaxis_deg: f32,
    /// Maximum branching order (0 = trunk). Shoot units are lumped on old
    /// trees and orders accumulate over centuries, so use a large value
    /// unless a species really has a low branching order.
    pub max_order: u8,
    /// Upward tendency of the main axis (weight added to growth direction).
    pub gravitropism: f32,
    /// Strength of the gravitropic correction of lateral axes towards their
    /// set-point angle (weight per annual shoot).
    pub lateral_gravitropism: f32,
    /// Gravitropic set-point angle (deg from vertical) of first-order limbs
    /// inserted at the bottom / top of the crown (interpolated on relative
    /// insertion height). Plagiotropic limbs grow towards this angle rather
    /// than towards the vertical (Digby & Firn 1995). 0 = orthotropic.
    pub limb_setpoint_low_deg: f32,
    pub limb_setpoint_high_deg: f32,
    /// Set-point angle (deg from vertical) of higher-order branches/twigs.
    pub twig_setpoint_deg: f32,
    /// Typical open-grown crown width / height. Lateral shoots reaching
    /// beyond ~60 % of the corresponding crown radius progressively turn
    /// upward (their set-point goes to the vertical at 120 %). Stylised
    /// stand-in for the costs of long horizontal limbs; 0 disables.
    pub crown_spread_ratio: f32,
    /// Tendency to grow towards less shaded space (weight).
    pub phototropism: f32,
    /// Random deviation of each annual shoot (deg) -> sinuous limbs.
    pub sinuosity_deg: f32,
    /// Yearly probability that an axis loses its terminal bud and is
    /// continued by a subterminal lateral (sympodial-looking "elbow"; oak
    /// terminal buds are frequently lost to frost, insects or lammas
    /// growth).
    pub leader_loss_probability: f32,
    /// Direction change (deg) of the axis at such a relay.
    pub leader_loss_angle_deg: f32,
    /// Stylised sag: a branch segment is deflected downward (and the
    /// deflection locked in by new wood) by `sag_coefficient * kappa * L`
    /// rad, `kappa = M / (E I)` being its elastic curvature under the load
    /// beyond it and `L` its length. The coefficient lumps creep (x2-3 of
    /// the elastic deflection) and the lumping of shoot units.
    pub sag_coefficient: f32,
    /// Segments thinner than this radius (mm) keep their posture (active
    /// gravitropic correction by reaction wood is efficient in thin axes,
    /// Almeras & Fournier 2009) and do not sag.
    pub sag_min_radius_mm: f32,
    /// Maximum increase of the sag of one segment per year (deg).
    pub sag_max_deg_per_year: f32,
    /// Maximum cumulative sag of one segment relative to its parent (deg).
    pub sag_max_total_deg: f32,
    /// Sag never bends a segment further down than this angle below the
    /// horizontal (deg).
    pub sag_max_droop_deg: f32,
    /// Only segments inclined at least this much from the vertical (deg)
    /// sag; ascending limbs carry their load mostly axially.
    pub sag_min_inclination_deg: f32,
    /// Autotropic straightening (deg/year) of the main continuation of an
    /// axis thicker than `straightening_min_radius_mm` back towards its
    /// parent's direction, down to a residual bend of
    /// `straightening_keep_deg` (0 disables).
    #[serde(default)]
    pub straightening_deg_per_year: f32,
    #[serde(default)]
    pub straightening_keep_deg: f32,
    #[serde(default)]
    pub straightening_min_radius_mm: f32,
    /// Branches whose foliage mean light exposure (0..1) is below this are shed.
    pub shedding_threshold: f32,
    /// Years a branch whose foliage is all below `shedding_threshold`
    /// survives on its reserves before it is shed (1 = shed at once).
    #[serde(default = "one_u8")]
    pub shade_survival_years: u8,
    /// Branches whose foliage *mean* light exposure (0..1) is below this are
    /// shed too, even if some of their shoots are well lit: a branch that is
    /// mostly shaded is a net carbon sink (branch autonomy, Sprugel et al.
    /// 1991). Drives the rise of the crown base (clear bole).
    pub shedding_mean_threshold: f32,
    /// Natural pruning of the stem (stylised): laterals inserted on the stem
    /// below `clear_bole_fraction` x tree height (and below
    /// `clear_bole_max_m`) are shed while their base diameter is less than
    /// `clear_bole_limb_ratio` x the stem diameter. Stands for the death of
    /// the lowest, suppressed branches of the juvenile crown, which the
    /// coarse light model does not shade enough. Thick limbs are kept.
    pub clear_bole_fraction: f32,
    pub clear_bole_max_m: f32,
    pub clear_bole_limb_ratio: f32,
    /// Age (years) from which the crown retrenches (dieback of the farthest
    /// boughs, epicormic reiteration lower down; see `growth.rs`). 0 = never.
    #[serde(default)]
    pub retrenchment_onset_years: f32,
    /// Yearly probability that an eligible bough dies, reached 100 years
    /// after onset (linear ramp from 0).
    #[serde(default)]
    pub retrenchment_rate: f32,
    /// Boughs whose farthest shoot is beyond this fraction of
    /// `hydraulic_path_max_m` from the collar are eligible for dieback.
    #[serde(default = "one_f32")]
    pub retrenchment_reach: f32,
    /// Yearly probability that a thick, well-lit limb in the lower crown
    /// forms epicormic buds after retrenchment onset.
    #[serde(default)]
    pub epicormic_rate: f32,
    /// Transport distance (m, straight line collar -> bud) at which shoot
    /// extension stops; foliage photosynthetic efficiency also declines with
    /// it. Models the size limitation of water transport (Ryan & Yoder 1997)
    /// and sets the asymptotic height and crown size.
    pub hydraulic_path_max_m: f32,
    /// Shape of that decline: extension is scaled by
    /// `1 - (d / hydraulic_path_max_m)^hydraulic_exponent` (1 = linear;
    /// larger = vigorous height growth in mid-life, then a sharper plateau).
    #[serde(default = "one_f32")]
    pub hydraulic_exponent: f32,
}

fn three_f32() -> f32 {
    3.0
}

fn quarter_f32() -> f32 {
    0.25
}

fn one_f32() -> f32 {
    1.0
}

fn one_u8() -> u8 {
    1
}

/// Leaf traits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeafTraits {
    /// Leaf blade length (cm).
    pub length_cm: f32,
    /// Leaf blade width (cm) (informational: for the front-end's leaf texture).
    pub width_cm: f32,
    /// Number of lobes, both sides (informational: for the leaf texture).
    pub lobes: u8,
    /// Deciduous (true) or evergreen (false).
    pub deciduous: bool,
    /// Fraction (0..1) of dead leaves retained over winter by juvenile trees.
    pub marcescence: f32,
    /// Age (years) after which marcescence fades out.
    pub marcescence_max_age_years: f32,
    /// Leaves borne by one annual shoot (informational for now).
    pub leaves_per_shoot: f32,
    /// Rendering: size of one leaf-cluster card, in leaf lengths.
    pub cluster_size_leaves: f32,
    /// Rendering: leaf-card area per lit shoot unit, in voxel faces (crown
    /// density; ~1 sparse, ~3 dense).
    pub foliage_density: f32,
}

/// Relative leaf pigment contents (0..1), driving seasonal colour.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pigments {
    pub chlorophyll: f32,
    /// Carotenoids (yellow/orange, unmasked when chlorophyll degrades).
    pub carotenoids: f32,
    /// Anthocyanins (red, synthesised during senescence).
    pub anthocyanins: f32,
    /// Tannins / oxidised phenolics (brown of dead leaves).
    pub tannins: f32,
}

/// Phenology parameters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhenologyTraits {
    /// Chilling days (daily mean within the chilling range) needed for full
    /// dormancy release.
    pub chilling_requirement_days: f32,
    pub chill_temp_min_c: f32,
    pub chill_temp_max_c: f32,
    /// Base temperature of growing degree days (degC).
    pub gdd_base_c: f32,
    /// Forcing (degC.day since Jan 1) to budburst once chilling is met.
    pub gdd_budburst: f32,
    /// Forcing after budburst to full leaf expansion (degC.day).
    pub leaf_expansion_gdd: f32,
    /// Photoperiod (h) below which senescence can start (after midsummer).
    pub senescence_photoperiod_h: f32,
    /// Base temperature of the cold-degree-day senescence sum (degC).
    pub senescence_temp_base_c: f32,
    /// Cold degree days needed from senescence onset to full colouring.
    pub senescence_cdd: f32,
    /// Duration of leaf fall (days).
    pub leaf_fall_days: f32,
}

/// Wood / secondary growth traits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WoodTraits {
    /// Basic wood density (kg/m3) (branch load for the sag model).
    pub density_kg_m3: f32,
    /// Modulus of elasticity of green wood (GPa) (branch sag).
    pub elasticity_gpa: f32,
    /// Ring-porous wood (earlywood from reserves before leaf-out).
    pub ring_porous: bool,
    /// Share of annual radial growth formed as earlywood in an average year.
    pub earlywood_share: f32,
    /// Pressler coefficient: m2 of new sapwood per m2 of light-weighted
    /// foliage above, per year, in an average year.
    pub growth_coefficient: f32,
    /// Radius of a new current-year shoot (mm).
    pub twig_radius_mm: f32,
    /// Physiological maximum ring width (mm).
    pub max_ring_width_mm: f32,
    /// Branching (da Vinci / pipe-model) exponent used to draw branch radii
    /// relative to the stem: `r_parent^e = sum r_child^e`. 2 = area
    /// preserving (what the simulated Pressler areas give); larger = thicker
    /// limbs. Mesh only.
    pub pipe_exponent: f32,
    /// Mesh only: the largest factor by which the pipe exponent may widen a
    /// branch over its simulated radius (twigs below ~5 mm are never
    /// widened).
    #[serde(default = "one_f32")]
    pub limb_draw_boost: f32,
    /// Mesh only: out-of-roundness (fluting) of thick wood, relative.
    #[serde(default)]
    pub bark_irregularity: f32,
    /// Mesh stylisation: widening of a branch base ring where it leaves its
    /// parent (branch collar), as a fraction of the branch radius.
    pub collar_flare: f32,
    /// Mesh stylisation: relative widening of the stem at ground level
    /// (root flare / buttressing). DBH and rings are measured on the
    /// simulated stem, not on this flare.
    pub root_flare: f32,
    /// Height of the flared zone, in collar radii.
    pub root_flare_height_radii: f32,
    /// Number of buttress lobes of the flare (0 = round).
    pub buttress_lobes: u8,
}

/// Root system traits (largely a stylisation, see `roots.rs`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RootTraits {
    /// Relative vigour of the taproot (0..1) in the seedling.
    pub taproot_dominance: f32,
    /// e-folding time (years) of taproot vigour.
    pub taproot_decline_years: f32,
    /// Maximum rooting depth (m).
    pub max_depth_m: f32,
    /// Horizontal spread of laterals as a multiple of crown radius.
    pub lateral_spread: f32,
    /// Number of first-order lateral roots.
    pub lateral_count: u32,
    /// Depth range (m) where first-order laterals leave the taproot.
    pub lateral_depth_m: f32,
    /// Probability that a lateral segment near the trunk emits a sinker.
    pub sinker_tendency: f32,
    /// Probability that a new root segment emits a side root.
    pub branching_probability: f32,
    /// Maximum explicit root order (finer roots are implicit).
    pub max_order: u8,
    /// Maximum annual extension of a root tip (m).
    pub annual_extension_m: f32,
    /// Soil penetration resistance (MPa) at which root elongation stops.
    #[serde(default = "three_f32")]
    pub max_penetration_mpa: f32,
    /// Waterlogging tolerance 0..1: how far roots enter permanently
    /// saturated soil (x 0.4 m), and how much anoxia hurts growth.
    #[serde(default)]
    pub waterlogging_tolerance: f32,
    /// Drought-deepening plasticity: relative deepening of laterals and
    /// boost of sinker/taproot growth per unit of last year's water stress.
    #[serde(default)]
    pub drought_deepening: f32,
    /// Strength of growth towards moister soil (laterals).
    #[serde(default)]
    pub hydrotropism: f32,
    /// Baseline fraction of growth allocated below ground (root:shoot
    /// partition, informational baseline of the allocation below).
    #[serde(default = "quarter_f32")]
    pub root_shoot_ratio: f32,
    /// Increase of root allocation per unit water stress (root growth x
    /// (1 + gain x stress); shoot vigour reduced accordingly).
    #[serde(default)]
    pub root_shoot_stress_gain: f32,
    /// Fine-root length (m) per metre of structural root in a layer of
    /// proliferation factor 1.
    #[serde(default)]
    pub fine_root_per_m: f32,
    /// Mesh only: thickening of structural laterals near the stem ("zone of
    /// rapid taper"): drawn radius x (1 + k exp(-distance / 1 m)).
    #[serde(default)]
    pub rapid_taper: f32,
}

/// Environmental tolerances.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tolerances {
    /// Daily minimum (degC) that damages expanding leaves.
    pub frost_c: f32,
    /// Drought tolerance 0..1 (shifts stomatal closure to drier soil).
    pub drought: f32,
    /// Shade tolerance 0..1 (lower bud break / shedding thresholds).
    pub shade: f32,
}

/// Error parsing a genome file.
#[derive(Debug, Clone)]
pub struct GenomeError(pub String);

impl fmt::Display for GenomeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "genome parse error: {}", self.0)
    }
}
impl std::error::Error for GenomeError {}

impl Genome {
    /// Built-in pedunculate oak (*Quercus robur*).
    pub fn quercus_robur() -> Genome {
        Genome::from_ron(QUERCUS_ROBUR_RON).expect("built-in Quercus robur genome must parse")
    }

    /// Parse a genome from RON text.
    pub fn from_ron(text: &str) -> Result<Genome, GenomeError> {
        ron::from_str(text).map_err(|e| GenomeError(e.to_string()))
    }

    /// Serialise to pretty RON.
    pub fn to_ron(&self) -> String {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .expect("genome serialisation cannot fail")
    }
}
