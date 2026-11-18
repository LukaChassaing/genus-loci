//! Species trait genome.
//!
//! A [`Genome`] is a flat, serialisable set of measurable traits. Every
//! species-specific number used by the simulation lives here, so a new
//! species (e.g. *Prunus avium*) is obtained by writing a new RON file, not by
//! changing code. Units are given in each field's doc comment.

use crate::fmath::DMath;
use serde::{Deserialize, Serialize};
use std::fmt;

// Species registry: `SPECIES_FILES`, every `data/species/*.ron` embedded at
// compile time by build.rs (sorted by file name; id = file stem).
include!(concat!(env!("OUT_DIR"), "/species_registry.rs"));

/// RON source of the built-in pedunculate oak genome
/// (`data/species/quercus_robur.ron`).
pub const QUERCUS_ROBUR_RON: &str = include_str!("../../../data/species/quercus_robur.ron");
/// RON source of the built-in wild cherry genome
/// (`data/species/prunus_avium.ron`).
pub const PRUNUS_AVIUM_RON: &str = include_str!("../../../data/species/prunus_avium.ron");

/// Group of a species in the menus (no effect on the simulation).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SpeciesCategory {
    /// Forest broadleaf (« feuillu »).
    #[default]
    Broadleaf,
    /// Fruit tree (« fruitier »).
    FruitTree,
    /// Conifer (« résineux »).
    Conifer,
}

impl SpeciesCategory {
    /// French plural label for the menus.
    pub fn label_fr(self) -> &'static str {
        match self {
            SpeciesCategory::Broadleaf => "Feuillus",
            SpeciesCategory::FruitTree => "Fruitiers",
            SpeciesCategory::Conifer => "Résineux",
        }
    }
}

/// Complete trait set of a species.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Genome {
    /// Registry id: the file stem of `data/species/<id>.ron` (set when the
    /// registry loads the file; empty for a genome parsed from elsewhere).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
    /// Latin binomial.
    pub species: String,
    /// English common name.
    pub common_name: String,
    /// French common name (front-end labels).
    #[serde(default)]
    pub common_name_fr: String,
    /// Menu group (feuillu / fruitier / résineux). No effect on growth.
    #[serde(default)]
    pub category: SpeciesCategory,
    pub architecture: Architecture,
    pub leaf: LeafTraits,
    pub pigments: Pigments,
    pub phenology: PhenologyTraits,
    pub wood: WoodTraits,
    /// Bark appearance (renderer only).
    #[serde(default)]
    pub bark: BarkTraits,
    pub roots: RootTraits,
    pub tolerances: Tolerances,
    /// Flowering and fruiting.
    #[serde(default)]
    pub reproduction: ReproductionTraits,
    /// Lifespan: age-related decline of vigour.
    #[serde(default)]
    pub longevity: LongevityTraits,
    /// Root symbioses (mycorrhizae). Used by [`crate::flux`] only: the
    /// growth simulation itself is unaffected (neutral default).
    #[serde(default)]
    pub symbiosis: SymbiosisTraits,
    /// Response to pruning: wound closure, compartmentalisation, latent buds
    /// (see [`crate::pruning`]). Defaults are neutral-ish averages.
    #[serde(default)]
    pub pruning: crate::pruning::PruningTraits,
    /// Static mechanics: green wood strength, crown drag, anchorage (see
    /// [`crate::mechanics`]). Read-only: no effect on growth.
    #[serde(default)]
    pub mechanics: MechanicsTraits,
}

/// Above-ground architecture (bud/shoot model, after Palubicki et al. 2009,
/// ACM Trans Graph 28(3):58, doi:10.1145/1531326.1531364; see growth.rs for
/// the other references and docs/sources.md).
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
    /// mostly shaded is a net carbon sink (branch autonomy, Sprugel, Hinckley
    /// & Schaap 1991, Annu Rev Ecol Syst; carbon autonomy of branches).
    /// Thresholds per species are calibration, not measurements. Drives the rise of the crown base (clear bole).
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
    /// Years after onset over which the dieback probability ramps up
    /// linearly from 0 to `retrenchment_rate`.
    #[serde(default = "hundred_f32")]
    pub retrenchment_ramp_years: f32,
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
    /// Shape of the decline of foliage efficiency (stomatal conductance)
    /// with the collar-to-leaf distance: efficiency
    /// `1 - (d / hydraulic_path_max_m)^e` (floor 0.08). Small values = early,
    /// gradual decline (oak 0.65); large = efficient until close to the
    /// size limit (fast-growing pioneers).
    #[serde(default = "foliage_exponent_default")]
    pub hydraulic_foliage_exponent: f32,
    /// Whorled branching: the lateral buds of an annual shoot are spread
    /// evenly around it (360 deg / n, with a random rotation per shoot)
    /// instead of following `phyllotaxis_deg`. The shoot model already
    /// starts all laterals at the tip of the annual shoot, so this gives the
    /// regular annual whorls (verticilles) of Pinus, Picea, Abies
    /// (Hallé, Oldeman & Tomlinson 1978: rhythmic growth of the trunk in
    /// the models of Rauh and Massart). false = phyllotaxis (default).
    #[serde(default)]
    pub whorled: bool,
    /// Plagiotropy of branch laterals (0..1): the laterals of an axis of
    /// order >= 1 are turned towards the horizontal plane of that axis, so
    /// branches spread as flat sprays (Massart's and Troll's models:
    /// Picea, Abies, Fagus, Carpinus; Hallé et al. 1978). 0 = laterals all
    /// around the axis (default), 1 = all in the plane.
    #[serde(default)]
    pub lateral_planarity: f32,
}

fn afp_default() -> f32 {
    0.10
}

fn foliage_exponent_default() -> f32 {
    0.65
}

fn hundred_f32() -> f32 {
    100.0
}

fn five_hundred_f32() -> f32 {
    500.0
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
    /// Margin serration 0..1 (0 = entire or lobed margin, 1 = coarsely,
    /// doubly serrate). Informational: for the leaf texture.
    #[serde(default)]
    pub serration: f32,
    /// Deciduous (true) or evergreen (false).
    pub deciduous: bool,
    /// Fraction (0..1) of dead leaves retained over winter by juvenile trees.
    pub marcescence: f32,
    /// Age (years) after which marcescence fades out.
    pub marcescence_max_age_years: f32,
    /// Leaves borne by one annual shoot. Rendering: one leaf card depicts a
    /// shoot with this many leaves (front-end texture), so a young tree
    /// gets about one card per annual shoot.
    pub leaves_per_shoot: f32,
    /// Rendering: size of one leaf-cluster card, in leaf lengths.
    pub cluster_size_leaves: f32,
    /// Rendering: leaf-card area per lit shoot unit, in voxel faces (crown
    /// density; ~1 sparse, ~3 dense).
    pub foliage_density: f32,
    /// Leaf arrangement on the shoot (front-end texture): 0 = leaves evenly
    /// spaced along the shoot, 1 = bunched at the shoot tip (oak).
    #[serde(default)]
    pub tip_bunching: f32,
    /// Petiole length relative to the blade (front-end texture; oak ~0.05,
    /// long-stalked cherry ~0.3).
    #[serde(default = "petiole_default")]
    pub petiole_ratio: f32,
    /// How far the blades hang from the shoot, 0 (held forward) .. 1
    /// (drooping back along the shoot; front-end texture).
    #[serde(default)]
    pub leaf_droop: f32,
    /// Rendering: foliage of short shoots and spurs along older living wood
    /// (the leafy shoot units only stand for the current year's long
    /// shoots): leaf-card area per metre of bearing wood, in voxel sizes,
    /// at full light. 0 = none.
    #[serde(default)]
    pub spur_foliage: f32,
    /// Rendering: thickest wood (radius, mm) still carrying short shoots.
    #[serde(default = "spur_radius_default")]
    pub spur_max_radius_mm: f32,
    /// Rendering: light exposure (0..1) below which wood carries no short
    /// shoots (they fade in over the next 0.2).
    #[serde(default = "spur_light_default")]
    pub spur_min_light: f32,
    /// Rendering: spread of the autumn colouring onset between leaf
    /// clusters, as a fraction of the colouring period (outer, sunlit
    /// leaves turn first; whole boughs vary together). 0 = uniform.
    #[serde(default = "colour_spread_default")]
    pub colour_spread: f32,
    /// Leaf form: broad leaf, needle or scale. Drives the foliage texture
    /// and, through `lma_g_m2` / `crown_lai`, the leaf mass and area.
    #[serde(default)]
    pub leaf_type: LeafType,
    /// Evergreens (`deciduous: false`): mean lifespan of a leaf (needle)
    /// cohort, years. Each annual shoot keeps its foliage for about this
    /// long; the oldest cohort is shed in autumn, progressively over
    /// `max(1, L/3)` years around age `L` (see [`LeafTraits::cohort_survival`]).
    /// Ignored for deciduous species. Picea abies 4-7, Abies alba ~8-10,
    /// Pinus 2-4, Taxus 4-8 (see the species files for sources).
    #[serde(default = "one_f32")]
    pub leaf_longevity_years: f32,
    /// Leaf mass per area (g dry mass per m2 of one-sided / projected leaf
    /// area), used for the leaf mass of the mechanics, the carbon of the
    /// flux view and the litter. Default 75 = the former code constant
    /// (Poorter et al. 2009, New Phytol 182:565-588: deciduous trees
    /// ~50-100 g/m2).
    #[serde(default = "lma_default")]
    pub lma_g_m2: f32,
    /// Leaf area index of the crown (m2 of one-sided / projected leaf area
    /// per m2 of crown projection) of a full, healthy crown. Default 4 =
    /// the former code constant. Must use the same area basis as `lma_g_m2`.
    #[serde(default = "lai_default")]
    pub crown_lai: f32,
}

/// Leaf form (see [`LeafTraits::leaf_type`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeafType {
    /// Flat leaf blade (all broadleaves; default).
    #[default]
    Broad,
    /// Needle (Pinaceae, Taxus).
    Needle,
    /// Scale leaves pressed to the shoot (Cupressaceae).
    Scale,
}

fn lma_default() -> f32 {
    75.0
}

fn lai_default() -> f32 {
    4.0
}

impl LeafTraits {
    /// Evergreen foliage: the leaves of an annual shoot live several years.
    pub fn evergreen(&self) -> bool {
        !self.deciduous && self.leaf_longevity_years > 1.0
    }

    /// Share (0..1) of a cohort of age `age_years` (0 = this year's shoot)
    /// still on the shoot. Exactly 0 for every older cohort of a deciduous
    /// species. Evergreens: a linear ramp from 1 to 0 over
    /// `w = max(1, L/3)` years centred on `L` (`leaf_longevity_years`), so
    /// the mean lifespan (integral of the curve) is `L`. Stylised: needle
    /// survival curves are not linear, but cohort retention data are
    /// usually summarised by the mean retention only.
    pub fn cohort_survival(&self, age_years: f32) -> f32 {
        if !self.evergreen() {
            return if age_years <= 0.0 { 1.0 } else { 0.0 };
        }
        let l = self.leaf_longevity_years;
        let w = (l / 3.0).max(1.0);
        (0.5 + (l - age_years) / w).clamp(0.0, 1.0)
    }

    /// Share (0..1) of an evergreen's foliage carried by the older cohorts
    /// just before the spring flush (`(S - 1) / S`, `S` = sum of the
    /// cohort survivals): the leaf area left in winter. 0 for deciduous
    /// species.
    pub fn retained_share(&self) -> f32 {
        if !self.evergreen() {
            return 0.0;
        }
        let mut sum = 1.0;
        let mut a = 1.0;
        while a < self.leaf_longevity_years * 2.0 + 4.0 {
            sum += self.cohort_survival(a);
            a += 1.0;
        }
        (sum - 1.0) / sum
    }
}

fn petiole_default() -> f32 {
    0.1
}

fn spur_radius_default() -> f32 {
    15.0
}

fn spur_light_default() -> f32 {
    0.1
}

fn colour_spread_default() -> f32 {
    0.5
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
    /// Share (0..1) of the mid-senescence anthocyanin peak still present
    /// in fully coloured leaves. 0 = transient red (it fades as the leaf
    /// dies, e.g. oak); high = leaves that stay red until they fall
    /// (anthocyanins keep being made in senescing leaves of Prunus, Acer,
    /// Hoch et al. 2001).
    #[serde(default)]
    pub anthocyanin_persistence: f32,
    /// Colour (RGB 0..1) of the anthocyanin-rich leaf. The hue of the
    /// anthocyanin red varies between species with vacuolar pH and
    /// co-pigments, from dull brick-red to scarlet.
    #[serde(default = "anthocyanin_color_default")]
    pub anthocyanin_color: [f32; 3],
}

fn anthocyanin_color_default() -> [f32; 3] {
    [0.58, 0.12, 0.07]
}

/// Phenology parameters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhenologyTraits {
    /// Chill days (daily mean within `chill_temp_min_c..=chill_temp_max_c`,
    /// counted from `chill_start_doy`) after which the extra forcing due to
    /// incomplete chilling has fallen to 5 % of `chill_forcing_gdd` (three
    /// e-folding lengths of the exponential of Cannell & Smith 1983 /
    /// Murray et al. 1989, see `phenology.rs`).
    pub chilling_requirement_days: f32,
    pub chill_temp_min_c: f32,
    pub chill_temp_max_c: f32,
    /// Base temperature of growing degree days (degC).
    pub gdd_base_c: f32,
    /// Forcing (degC.day above `gdd_base_c` from `forcing_start_doy`) to
    /// budburst with ample chilling (the asymptote `a` of
    /// `T = a + b exp(r C)`).
    pub gdd_budburst: f32,
    /// Extra forcing needed after a winter without any chill day (`b`).
    #[serde(default)]
    pub chill_forcing_gdd: f32,
    /// Day of year (0-based) from which forcing is summed (31 = Feb 1, as
    /// in Cannell & Smith 1983).
    #[serde(default = "forcing_start_default")]
    pub forcing_start_doy: u32,
    /// Day of year (0-based) from which chill days are counted (304 =
    /// Nov 1, as in Cannell & Smith 1983).
    #[serde(default = "chill_start_default")]
    pub chill_start_doy: u32,
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

fn forcing_start_default() -> u32 {
    31
}

fn chill_start_default() -> u32 {
    304
}

impl PhenologyTraits {
    /// Extra forcing (degC.day) required after `chill_days` chill days:
    /// `b exp(r C)` with `r = -3 / chilling_requirement_days` (Cannell &
    /// Smith 1983; Murray, Cannell & Smith 1989).
    pub fn chill_penalty_gdd(&self, chill_days: f32) -> f32 {
        let r = 3.0 / self.chilling_requirement_days.max(1.0);
        self.chill_forcing_gdd * (-r * chill_days.max(0.0)).dexp()
    }

    /// Thermal time to budburst (degC.day from `forcing_start_doy`) after
    /// `chill_days` chill days: `a + b exp(r C)`.
    pub fn budburst_requirement_gdd(&self, chill_days: f32) -> f32 {
        self.gdd_budburst + self.chill_penalty_gdd(chill_days)
    }
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
    /// Share of annual radial growth formed as earlywood in an average year
    /// (formed at the end of leaf expansion, from last year's reserves).
    pub earlywood_share: f32,
    /// Renderer: visual contrast 0..1 between earlywood and latewood of a
    /// ring (1 = ring-porous, large earlywood vessels; ~0.3 = diffuse- to
    /// semi-ring-porous).
    #[serde(default = "one_f32")]
    pub ring_contrast: f32,
    /// Persistence of standing dead wood: a dead piece of radius `r` (m)
    /// breaks off after `2 + snag_years_per_m * r` years (durable heartwood
    /// -> large values).
    #[serde(default = "five_hundred_f32")]
    pub snag_years_per_m: f32,
    /// Annual wood budget: m3 of new wood per m2 of light-weighted foliage,
    /// in an average year. The tree's budget (this x total foliage) is
    /// spread over all segments by Pressler's law, area increment
    /// proportional to the foliage above (`ShootSystem::area_per_foliage`;
    /// Larson 1963, "Stem form development of forest trees", Forest Science
    /// Monographs 5; whole-tree budget as in LIGNUM, Perttunen et al. 1996,
    /// Ann Bot 77:87-98). Calibration: fitted to published open-grown
    /// DBH-age curves (White 1998 for oak, Pryor 1988 for cherry), not a
    /// published constant; the proportional foliage-to-sapwood idea is the
    /// pipe model (Shinozaki et al. 1964).
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
    /// Adaptive (mechanical) radial growth: set-point of the self-weight
    /// bending stress (MPa, full summer crown, calm air) above which a
    /// woody segment lays down extra wood until its stress is back at the
    /// set-point, taken from the year's wood budget (see
    /// `ShootSystem::thicken`). Uniform-stress axiom (Mattheck & Burkhardt
    /// 1990; Morgan & Cannell 1994; branches: Evans et al. 2008, Minamino &
    /// Tateno 2014). 0 = off (pure Pressler allocation, the behaviour of
    /// genomes written before this trait).
    #[serde(default)]
    pub adaptive_stress_mpa: f32,
    /// Adaptive growth: segments thinner than this (cm) do not respond
    /// (flexible twigs deflect rather than thicken).
    #[serde(default)]
    pub adaptive_min_diameter_cm: f32,
    /// Adaptive growth: the largest share (0..1) of a growth phase's wood
    /// budget that the mechanical response may take from Pressler's law.
    #[serde(default)]
    pub adaptive_max_budget_share: f32,
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
    /// Sensitivity 0..1 of root function to poor soil aeration (air-filled
    /// porosity < 10 %, short of saturation): water uptake is reduced by
    /// `sensitivity x` the root-weighted hypoxia. 0 = tolerant (roots with
    /// aerenchyma / hypertrophied lenticels), 1 = very sensitive (root
    /// asphyxia). Independent of `waterlogging_tolerance`, which only sets
    /// how far roots enter permanently saturated soil.
    #[serde(default)]
    pub hypoxia_sensitivity: f32,
    /// Air-filled porosity (m3/m3) below which the roots suffer from
    /// hypoxia (see `soil.rs`). ~0.10 is the usual threshold (Grable &
    /// Siemer 1968, maize); sensitive species react from wetter (higher)
    /// values.
    #[serde(default = "afp_default")]
    pub hypoxia_air_porosity: f32,
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

/// Bark appearance, for the renderer (no effect on the simulation).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BarkTraits {
    /// Bark colour (RGB 0..1) of established wood (from ~4 years old).
    pub color: [f32; 3],
    /// Colour of current-year shoots.
    pub young_color: [f32; 3],
    /// Colour subtracted on the thickest wood (radius >= 0.4 m, linear
    /// from 0): old trunks are darker.
    pub old_darkening: [f32; 3],
    /// Strength 0..1 of horizontal lenticel bands (cherry, birch).
    pub lenticels: f32,
    /// Strength 0..1 of deep vertical fissures (old oak).
    pub fissures: f32,
    /// Glossiness 0..1 (smooth, shiny periderm).
    pub gloss: f32,
}

impl Default for BarkTraits {
    /// Grey-brown, fissured bark (the oak's).
    fn default() -> Self {
        BarkTraits {
            color: [0.40, 0.32, 0.28],
            young_color: [0.40, 0.38, 0.22],
            old_darkening: [0.10, 0.08, 0.06],
            lenticels: 0.0,
            fissures: 0.8,
            gloss: 0.0,
        }
    }
}

/// Flowering and fruiting (see `phenology::Reproduction`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReproductionTraits {
    /// Age (years) of first flowering (end of the juvenile phase); each
    /// tree varies by +-1 year (seeded).
    pub first_flowering_age_years: f32,
    /// Rendering weight 0..1 of the flowers (0 = inconspicuous, e.g. oak
    /// catkins: no flower cards and `flower_density() == 0`).
    pub flower_display: f32,
    /// Rendering weight 0..1 of the fruits.
    pub fruit_display: f32,
    /// Forcing (degC.day above `gdd_base_c` from `forcing_start_doy`) to
    /// bloom onset with ample chilling (plus the same chilling penalty
    /// `PhenologyTraits::chill_penalty_gdd` as budburst).
    pub bloom_gdd: f32,
    /// Forcing from bloom onset to the end of petal fall (degC.day).
    pub bloom_duration_gdd: f32,
    /// Flowers are borne on wood at least this old (years; spurs).
    pub min_wood_age_years: u32,
    /// ... in parts of the crown at least this well lit (0..1).
    pub min_light: f32,
    /// ... on axes thinner than this (mm radius).
    pub max_wood_radius_mm: f32,
    /// Flower-cluster card area per flowering shoot unit, in voxel faces
    /// (bloom density; cf. `LeafTraits::foliage_density`).
    pub flower_density: f32,
    /// Size of one flower cluster (umbel / catkin bunch) card (cm).
    pub flower_cluster_cm: f32,
    /// Size of one fruit cluster card (cm).
    pub fruit_cluster_cm: f32,
    /// Petal colour (RGB 0..1).
    pub flower_color: [f32; 3],
    /// Fraction of flower clusters that set fruit in a frost-free year.
    pub fruit_set: f32,
    /// Daily minimum (degC) that kills open flowers (late frost).
    pub flower_frost_c: f32,
    /// Days from the end of petal fall to ripe fruit.
    pub fruit_ripening_days: f32,
    /// Days ripe fruit stays on the tree (eaten, dropped).
    pub fruit_persistence_days: f32,
    /// Fruit colour unripe / ripe (RGB 0..1).
    pub fruit_color_unripe: [f32; 3],
    pub fruit_color_ripe: [f32; 3],
    /// Kind of fruit, for the renderer (texture and how the card hangs).
    #[serde(default)]
    pub fruit_kind: FruitKind,
    /// Mean fresh mass of one fruit / cone (g). Informational (no carbon
    /// cost of reproduction in the growth model).
    #[serde(default)]
    pub fruit_mass_g: f32,
    /// Mean interval (years) between crop years: 1 = a crop every year
    /// (default); > 1 = masting (Fagus, Picea) or alternate bearing
    /// (Malus): each year after first flowering is a crop year with
    /// probability `1 / crop_interval_years`, never two in a row; other
    /// years flower and fruit at `off_year_crop` of the normal amount.
    /// Stylised (no resource-budget model of masting).
    #[serde(default = "one_f32")]
    pub crop_interval_years: f32,
    /// Flower and fruit amount of a non-crop year, relative (0..1).
    #[serde(default = "off_year_default")]
    pub off_year_crop: f32,
}

fn off_year_default() -> f32 {
    0.15
}

/// Kind of fruit (renderer only, see [`ReproductionTraits::fruit_kind`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FruitKind {
    /// Round fleshy fruit, drupe or pome (cherry, apple, plum; default).
    #[default]
    Fleshy,
    /// Nut or acorn (Quercus, Juglans, Corylus, Castanea, Fagus).
    Nut,
    /// Winged fruit (samara: Acer, Fraxinus; Betula, Carpinus, Tilia nutlets with a wing or bract).
    Samara,
    /// Pendent woody cone (Picea, Pinus, Pseudotsuga, Larix).
    ConePendent,
    /// Erect cone, disintegrating on the tree (Abies).
    ConeErect,
    /// Fleshy aril (Taxus).
    Aril,
}

impl Default for ReproductionTraits {
    /// Inconspicuous, wind-pollinated flowers (not displayed).
    fn default() -> Self {
        ReproductionTraits {
            first_flowering_age_years: 30.0,
            flower_display: 0.0,
            fruit_display: 0.0,
            bloom_gdd: 250.0,
            bloom_duration_gdd: 120.0,
            min_wood_age_years: 0,
            min_light: 0.3,
            max_wood_radius_mm: 30.0,
            flower_density: 1.0,
            flower_cluster_cm: 5.0,
            fruit_cluster_cm: 3.0,
            flower_color: [0.65, 0.62, 0.30],
            fruit_set: 0.3,
            flower_frost_c: -2.5,
            fruit_ripening_days: 120.0,
            fruit_persistence_days: 20.0,
            fruit_color_unripe: [0.35, 0.45, 0.15],
            fruit_color_ripe: [0.45, 0.32, 0.15],
            fruit_kind: FruitKind::Fleshy,
            fruit_mass_g: 0.0,
            crop_interval_years: 1.0,
            off_year_crop: off_year_default(),
        }
    }
}

/// Lifespan traits: whole-tree, age-related decline of vigour (heart rot,
/// pathogen load and hydraulic dysfunction of short-lived pioneer trees;
/// stylised). The crown dieback itself is `Architecture::retrenchment_*`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LongevityTraits {
    /// Age (years) at which vigour starts to decline (0 = never).
    pub decline_onset_years: f32,
    /// Years from onset to the minimum vigour (smooth ramp).
    pub decline_years: f32,
    /// Remaining fraction of shoot, cambial and root growth at the end of
    /// the decline.
    pub decline_min_vigour: f32,
}

impl Default for LongevityTraits {
    fn default() -> Self {
        LongevityTraits { decline_onset_years: 0.0, decline_years: 50.0, decline_min_vigour: 0.3 }
    }
}

impl LongevityTraits {
    /// Vigour factor (0..1] at `age` years: exactly 1.0 before onset.
    pub fn vigour(&self, age: f32) -> f32 {
        if self.decline_onset_years <= 0.0 || age <= self.decline_onset_years {
            return 1.0;
        }
        let t = ((age - self.decline_onset_years) / self.decline_years.max(1.0)).clamp(0.0, 1.0);
        let s = t * t * (3.0 - 2.0 * t);
        1.0 - (1.0 - self.decline_min_vigour.clamp(0.0, 1.0)) * s
    }
}

/// Environmental tolerances.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tolerances {
    /// Daily minimum (degC) that damages expanding leaves.
    pub frost_c: f32,
    /// Drought tolerance 0..1 (shifts stomatal closure to drier soil).
    pub drought: f32,
    /// Shade tolerance 0..1 (lower bud break / shedding thresholds) of the
    /// established tree.
    pub shade: f32,
    /// Shade tolerance of the young tree, if different (e.g. pioneers that
    /// tolerate shade as seedlings and become light-demanding later). Kept
    /// until `shade_juvenile_years`, then blended smoothly into `shade` by
    /// `shade_mature_years`. None = `shade` at all ages.
    #[serde(default)]
    pub shade_juvenile: Option<f32>,
    #[serde(default)]
    pub shade_juvenile_years: f32,
    #[serde(default)]
    pub shade_mature_years: f32,
}

impl Tolerances {
    /// Shade tolerance 0..1 at `age` years (see `shade_juvenile`).
    pub fn shade_at(&self, age: f32) -> f32 {
        let Some(young) = self.shade_juvenile else {
            return self.shade;
        };
        let span = (self.shade_mature_years - self.shade_juvenile_years).max(1.0);
        let t = ((age - self.shade_juvenile_years) / span).clamp(0.0, 1.0);
        let s = t * t * (3.0 - 2.0 * t);
        young + (self.shade - young) * s
    }
}

/// Mycorrhizal association of a species.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MycorrhizaType {
    /// No mycorrhiza (default).
    #[default]
    None,
    /// Arbuscular mycorrhiza (Glomeromycota hyphae inside root cortex
    /// cells; short extraradical hyphae).
    Arbuscular,
    /// Ectomycorrhiza (fungal mantle and Hartig net around root tips;
    /// extramatrical mycelium, sometimes rhizomorphs).
    Ecto,
}

/// Root symbiosis traits (see [`crate::flux`]). Defaults are neutral (no
/// fungus), so genomes without this section behave as before.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SymbiosisTraits {
    pub mycorrhiza: MycorrhizaType,
    /// Share of net primary production given to the fungal partner (0..1).
    pub carbon_cost: f32,
    /// Share of the tree's P uptake delivered by fungal hyphae (0..1).
    pub phosphorus_share: f32,
    /// Share of the tree's N uptake delivered by fungal hyphae (0..1).
    pub nitrogen_share: f32,
    /// Share of the root water uptake that passes through hyphae (0..1).
    pub water_share: f32,
    /// Extent of the extraradical mycelium beyond the root surface (m),
    /// as drawn.
    pub hyphal_reach_m: f32,
}

impl Default for SymbiosisTraits {
    fn default() -> Self {
        SymbiosisTraits {
            mycorrhiza: MycorrhizaType::None,
            carbon_cost: 0.0,
            phosphorus_share: 0.0,
            nitrogen_share: 0.0,
            water_share: 0.0,
            hyphal_reach_m: 0.0,
        }
    }
}

/// Wood and crown mechanics (see [`crate::mechanics`]). Read by the static
/// mechanics; the green density also sets the self-weight load of the
/// adaptive radial growth (`WoodTraits::adaptive_stress_mpa`). The sag
/// model keeps using `WoodTraits::density_kg_m3` and `elasticity_gpa`. Every value is sourced in the species RON files and
/// `docs/sources.md`. Defaults: a generic temperate hardwood (the mean of the
/// two built-in species' sources, for genomes written before this block).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MechanicsTraits {
    /// Density of green (fresh, living) wood: oven-dry mass + water per
    /// green volume (kg/m3) = specific gravity (green volume) x (1 + green
    /// moisture content). Self-weight load of the mechanics.
    pub green_density_kg_m3: f32,
    /// Modulus of rupture of green clear wood in static bending (MPa).
    pub green_mor_mpa: f32,
    /// Modulus of elasticity of green clear wood in static bending (GPa).
    /// Used for the elastic buckling (Greenhill) check; the deflection and
    /// sway of the dynamic step will use it too.
    pub green_moe_gpa: f32,
    /// Crown drag coefficient at the reference speed of 20 m/s, defined on
    /// the still-air frontal area of the leafy crown (as in Vollsinger et
    /// al. 2005, Can J For Res 35:1238-1249).
    pub crown_drag_coefficient: f32,
    /// Vogel exponent B of crown reconfiguration (< 0): drag grows as
    /// U^(2+B) instead of U^2 between `mechanics::RECONF_MIN_MS` and
    /// `RECONF_MAX_MS` (Vogel 1989, J Exp Bot 40:941-948).
    pub vogel_exponent: f32,
    /// Anchorage coefficient (N m per kg of green stem mass): critical
    /// turning moment of the root-soil plate per kg of stem, for deep
    /// rooting on a freely draining soil (`Creg` of Gardiner et al. 2000
    /// and ForestGALES; Nicoll et al. 2006, Can J For Res 36:1871-1883).
    pub anchorage_nm_per_kg: f32,
}

impl Default for MechanicsTraits {
    fn default() -> Self {
        MechanicsTraits {
            green_density_kg_m3: 885.0,
            green_mor_mpa: 56.0,
            green_moe_gpa: 8.8,
            crown_drag_coefficient: 0.2,
            vogel_exponent: -1.0,
            anchorage_nm_per_kg: 178.1,
        }
    }
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

/// Accent-insensitive sort key of a French name (stable menu order).
fn sort_key(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'É' | 'È' => 'E',
            c => c,
        })
        .flat_map(char::to_lowercase)
        .collect()
}

/// The parsed registry, in menu order (see [`Genome::registry`]).
fn registry_cache() -> &'static [Genome] {
    static CACHE: std::sync::OnceLock<Vec<Genome>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| {
        let mut all: Vec<Genome> = SPECIES_FILES
            .iter()
            .map(|(id, text)| {
                let mut g = Genome::from_ron(text).unwrap_or_else(|e| panic!("data/species/{id}.ron: {e}"));
                g.id = (*id).to_string();
                g
            })
            .collect();
        all.sort_by(|a, b| (a.category, sort_key(&a.common_name_fr), &a.id).cmp(&(b.category, sort_key(&b.common_name_fr), &b.id)));
        all
    })
}

impl Genome {
    /// Built-in pedunculate oak (*Quercus robur*).
    pub fn quercus_robur() -> Genome {
        Genome::by_id("quercus_robur").expect("built-in Quercus robur genome")
    }

    /// Built-in wild cherry (*Prunus avium*).
    pub fn prunus_avium() -> Genome {
        Genome::by_id("prunus_avium").expect("built-in Prunus avium genome")
    }

    /// Every species of `data/species/*.ron` (embedded at compile time), in
    /// menu order: by category (broadleaves, fruit trees, conifers), then by
    /// French name. Indices into this list are the species indices of the
    /// front-end; they shift when a species is added, so persist the id or
    /// the latin name, never an index.
    pub fn registry() -> &'static [Genome] {
        registry_cache()
    }

    /// All built-in species (owned copies of [`Genome::registry`]).
    pub fn builtin() -> Vec<Genome> {
        registry_cache().to_vec()
    }

    /// The species shown first when nothing is chosen (the oak).
    pub fn default_index() -> usize {
        Genome::index_of("quercus_robur").unwrap_or(0)
    }

    /// Built-in species by registry id (`picea_abies`).
    pub fn by_id(id: &str) -> Option<Genome> {
        registry_cache().iter().find(|g| g.id == id).cloned()
    }

    /// Registry index of a species given by id or latin binomial.
    pub fn index_of(id_or_latin: &str) -> Option<usize> {
        registry_cache().iter().position(|g| g.id == id_or_latin || g.species == id_or_latin)
    }

    /// Built-in species by id or latin binomial.
    pub fn find(id_or_latin: &str) -> Option<Genome> {
        Genome::index_of(id_or_latin).map(|i| registry_cache()[i].clone())
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
