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
    /// Age (years) over which lambda moves from young to mature value.
    pub maturity_age_years: f32,
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
    /// Upward tendency of lateral axes (small = plagiotropic, spreading).
    pub lateral_gravitropism: f32,
    /// Tendency to grow towards less shaded space (weight).
    pub phototropism: f32,
    /// Random deviation of each annual shoot (deg) -> sinuous limbs.
    pub sinuosity_deg: f32,
    /// Branches whose foliage mean light exposure (0..1) is below this are shed.
    pub shedding_threshold: f32,
    /// Transport distance (m, straight line collar -> bud) at which shoot
    /// extension stops; foliage photosynthetic efficiency also declines with
    /// it. Models the size limitation of water transport (Ryan & Yoder 1997)
    /// and sets the asymptotic height and crown size.
    pub hydraulic_path_max_m: f32,
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
    /// Basic wood density (kg/m3). Informational for now.
    pub density_kg_m3: f32,
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
