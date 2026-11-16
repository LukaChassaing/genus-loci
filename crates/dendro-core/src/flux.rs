//! Flows through the tree: water, mineral nutrients, sugars, gases and the
//! mycorrhizal exchange, in real units, for the current simulated day, plus
//! the geometry needed to draw them as moving dashes along their real paths.
//!
//! Everything here is *derived* from state the simulation already computes
//! (root water uptake per soil layer, phenology, the daily productivity
//! factors, soil aeration, the shoot and root graphs). The flux layer has no
//! state of its own and does not feed back into growth: it is a reading of
//! the simulation in physiological units, stylised where noted.
//!
//! Physical picture (the textbook model, e.g. Taiz & Zeiger, Plant
//! Physiology; Tyree & Zimmermann 2002, "Xylem Structure and the Ascent of
//! Sap", 2nd ed., Springer):
//! * **Xylem sap** rises passively from the fine roots to the leaves, pulled
//!   by the tension created by evaporation in the leaves (cohesion-tension
//!   theory: Dixon & Joly 1895, "On the ascent of sap", Phil Trans R Soc B
//!   186:563-576). Its daily volume equals root uptake; nearly all of it
//!   leaves the leaves as vapour (transpiration), a tiny part is built into
//!   new tissue. Stem water stores buffer the daytime peak and refill at
//!   night (Köcher, Horna & Leuschner 2013, Tree Physiol 33:817-832,
//!   doi:10.1093/treephys/tpt055: 10-22 % of daily transpiration in
//!   diffuse-porous species, much less in ring-porous ash), so their net
//!   daily change is ~0 outside drought onset (taken as 0 here).
//! * **Mineral nutrients** (N, P, K) enter with the water (mass flow and
//!   diffusion to the roots) and travel up in the xylem; demand is set by
//!   the stoichiometry of the tissue being built; in spring much of the N
//!   for new leaves comes from internal stores filled by autumn resorption
//!   (Millard & Grelet 2010, Tree Physiol 30:1083-1095,
//!   doi:10.1093/treephys/tpq042; Aerts 1996, J Ecol 84:597-608: ~50 % of
//!   leaf N and P resorbed before abscission).
//! * **Phloem sap** carries sugars from sources to sinks by the osmotically
//!   generated pressure flow of Münch (1930, "Die Stoffbewegungen in der
//!   Pflanze", Fischer, Jena): down from the leaves to the cambium, roots,
//!   reserves and fungal partners in summer; in spring the stored starch is
//!   mobilised and the flow runs *up* to the opening buds before the new
//!   leaves export (Barbaroux & Bréda 2002, Tree Physiol 22:1201-1210: oak
//!   stem reserves drop sharply at budburst and earlywood formation).
//!   Phloem moves ~0.25-0.4 mm/s, much slower than peak xylem flow (Windt
//!   et al. 2006, Plant Cell Environ 29:1715-1729,
//!   doi:10.1111/j.1365-3040.2006.01544.x).
//! * **Gas exchange**: photosynthesis takes up one CO2 per O2 released
//!   (photosynthetic quotient ~1); respiration of every living tissue takes
//!   up O2 and releases CO2 (respiratory quotient ~1 on carbohydrates). Roots
//!   in poorly aerated soil switch to fermentation, releasing CO2 without
//!   taking up O2 (Drew 1997, "Oxygen deficiency and root metabolism", Annu
//!   Rev Plant Physiol Plant Mol Biol 48:223-250).
//! * **Mycorrhizae**: the fungal partner explores the soil beyond the roots,
//!   delivers part of the P, N and (a little) water, and receives carbon
//!   (see `SymbiosisTraits` and the species RON files for sources).

use crate::fmath::DMath;
use crate::genome::MycorrhizaType;
use crate::growth::ShootSystem;
use crate::math::Vec3;
use crate::mesh::MeshData;
use crate::phenology::{Phase, Phenology, ReproStage};
use crate::rng;
use crate::roots::{self, RootKind, RootSystem};
use crate::sim::Simulation;
use crate::soil::SoilColumn;

/// Borrowed internal state of a [`Simulation`] (built by `Simulation::flux_view`).
pub(crate) struct FluxView<'a> {
    pub soil: &'a SoilColumn,
    pub phen: &'a Phenology,
    pub shoots: &'a ShootSystem,
    pub roots: &'a RootSystem,
    pub water_factor: f32,
    pub fertility: f32,
}

// ----------------------------------------------------------------------
// Constants
// ----------------------------------------------------------------------

/// Molar masses (g/mol).
pub const M_C: f32 = 12.011;
pub const M_CO2: f32 = 44.009;
pub const M_O2: f32 = 31.998;
/// Carbon fraction of dry plant tissue. Thomas & Martin 2012, "Carbon content
/// of tree tissues: a synthesis", Forests 3:332-352 (~47-50 %).
const C_FRAC: f32 = 0.48;
/// Gross photosynthesis under one m2 of crown projection on an ideal day
/// (full leaf, 14 h day, warm, moist, fertile), g C m-2 d-1. Calibration:
/// gives ~1100-1600 g C m-2 yr-1 under the crown, the order of magnitude of
/// temperate deciduous forest GPP in eddy-covariance syntheses (Luyssaert et
/// al. 2007, Glob Change Biol 13:2509-2537).
const GPP_MAX: f32 = 11.0;
/// Q10 of maintenance respiration. Ryan 1991, "Effects of climate change on
/// plant respiration", Ecol Appl 1:157-167 (Q10 ~2 commonly used).
const Q10: f32 = 2.0;
/// Growth respiration per unit of carbon built into new tissue. Amthor 2000,
/// "The McCree-de Wit-Penning de Vries-Thornley respiration paradigms: 30
/// years later", Ann Bot 86:1-20 (growth respiration ~25 % of the carbon
/// in new biomass, Ryan 1991).
const GROWTH_RESP: f32 = 0.25;
/// Maintenance respiration at 15 degC. Calibration (no direct source):
/// chosen so that annual autotrophic respiration is ~half of GPP for a
/// mid-aged tree, NPP/GPP ~0.47 (Waring, Landsberg & Williams 1998, "Net
/// primary production of forests: a constant fraction of gross primary
/// production?", Tree Physiol 18:129-134).
/// Leaves, g C per m2 of crown projection per day (full canopy).
const RM_LEAF: f32 = 1.3;
/// Fine roots, g C per m2 of crown projection per day.
const RM_FINE_ROOT: f32 = 0.8;
/// Living sapwood (stem, branches, coarse roots), g C per m3 per day.
const RM_SAPWOOD: f32 = 25.0;
/// Depth of the living, conducting sapwood band (m). Calibration: oak
/// keeps its reserves in the outer sapwood rings (Barbaroux & Bréda 2002).
const SAPWOOD_DEPTH: f32 = 0.04;
/// Leaf mass per area (g dry m-2 leaf) and leaf area index of a full crown.
/// Poorter et al. 2009, "Causes and consequences of variation in leaf mass
/// per area (LMA): a meta-analysis", New Phytol 182:565-588 (deciduous
/// trees ~50-100 g m-2); LAI 4 is a calibration for an open-grown crown.
const LMA: f32 = 75.0;
const LAI_FULL: f32 = 4.0;
/// Days over which the year's leaves and shoots are built (budburst to full
/// expansion), calibration.
const FLUSH_DAYS: f32 = 35.0;
/// Shoot wood built with the leaves, as a fraction of leaf carbon (calibration).
const SHOOT_WOOD_PER_LEAF: f32 = 0.3;
/// Tissue nutrient concentrations (mg per g dry mass): [N, P, K].
/// Leaves: the order of the global means of Reich & Oleksyn 2004, "Global
/// patterns of plant leaf N and P in relation to temperature and latitude",
/// PNAS 101:11001-11006 (~20 mg N, ~1.8 mg P per g); K, wood, fine roots and
/// fruits are typical orders of magnitude (Marschner 2012, "Mineral
/// Nutrition of Higher Plants", 3rd ed.), not species measurements.
const NUT_LEAF: [f32; 3] = [20.0, 1.8, 10.0];
const NUT_WOOD: [f32; 3] = [2.0, 0.2, 1.5];
const NUT_ROOT: [f32; 3] = [10.0, 1.0, 4.0];
const NUT_FRUIT: [f32; 3] = [15.0, 2.0, 15.0];
/// Share of spring leaf N/P/K demand met from internal stores (Millard &
/// Grelet 2010; calibration value).
const SPRING_REMOB: f32 = 0.5;
/// Fraction of leaf N/P (and K) resorbed before leaf fall (Aerts 1996, ~50 %).
const RESORPTION: [f32; 3] = [0.5, 0.5, 0.4];
/// Days of autumn colouring + leaf fall over which resorption / litterfall
/// are spread (calibration).
const AUTUMN_DAYS: f32 = 30.0;
/// Rain intercepted by the crown and evaporated (same ~10 % as the soil
/// water balance, see `soil.rs`).
const INTERCEPTION: f32 = 0.10;
/// Mean soil temperature (degC) the root zone relaxes to; soil damps the
/// daily air temperature (calibration).
const SOIL_T_MEAN: f32 = 10.0;

// ----------------------------------------------------------------------
// Snapshot
// ----------------------------------------------------------------------

/// Where a flow comes from / goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compartment {
    Soil,
    FineRoots,
    Roots,
    Trunk,
    Crown,
    Leaves,
    /// Opening buds and growing shoots.
    Shoots,
    Fruits,
    /// Starch / N stores in the sapwood rays of stem and roots.
    Reserves,
    Fungus,
    Atmosphere,
    Litter,
}

/// Unit of a flow rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowUnit {
    LitresPerDay,
    GramsCarbonPerDay,
    GramsPerDay,
}

/// One typed flow of the day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flow {
    pub name: &'static str,
    pub source: Compartment,
    pub sink: Compartment,
    pub rate: f32,
    pub unit: FlowUnit,
}

/// All flows of the last simulated day, in real units (whole tree).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FluxSnapshot {
    /// Crown projection area (m2) the per-area fluxes are scaled by.
    pub crown_area_m2: f32,
    /// Phenological phase today.
    pub phase: Option<Phase>,

    // --- water (L/day) ---
    /// Root water uptake (soil -> fine roots), total and per soil layer.
    pub uptake_l: f32,
    pub uptake_layers_l: Vec<f32>,
    /// Sap through the collar (roots -> trunk -> crown).
    pub xylem_l: f32,
    /// Water built into new tissue.
    pub growth_water_l: f32,
    /// Leaves -> atmosphere as vapour.
    pub transpiration_l: f32,
    /// Net change of stem water storage over the day (0, see module doc).
    pub storage_change_l: f32,
    /// Water drawn from stem storage during the day and refilled at night.
    pub storage_buffer_l: f32,
    /// Rain on the crown, evaporated from wet leaves (interception), and
    /// reaching the ground.
    pub rain_l: f32,
    pub interception_l: f32,
    pub throughfall_l: f32,

    // --- nutrients (g/day): [N, P, K] ---
    /// Soil -> roots (part of it through hyphae, see `myco_nutrients_g`).
    pub nutrient_uptake_g: [f32; 3],
    /// Carried up in the xylem (uptake minus what the roots keep).
    pub nutrient_xylem_g: [f32; 3],
    /// Stores -> growing shoots (spring).
    pub nutrient_remob_g: [f32; 3],
    /// Senescing leaves -> stores (autumn, phloem).
    pub nutrient_resorb_g: [f32; 3],
    /// Fungus -> roots.
    pub myco_nutrients_g: [f32; 3],
    /// Unmet share of the soil nutrient demand (0..1).
    pub nutrient_limitation: f32,

    // --- carbon (g C/day) ---
    pub gpp_gc: f32,
    pub resp_leaf_gc: f32,
    pub resp_wood_gc: f32,
    pub resp_root_gc: f32,
    pub resp_growth_gc: f32,
    pub alloc_shoots_gc: f32,
    pub alloc_wood_gc: f32,
    pub alloc_roots_gc: f32,
    pub alloc_fruits_gc: f32,
    pub alloc_reserves_gc: f32,
    pub alloc_myco_gc: f32,
    /// Reserves drawn on (spring flush, ring-porous earlywood, respiration
    /// when photosynthesis cannot cover it).
    pub reserves_used_gc: f32,
    /// Phloem: leaves -> trunk, roots, fungus, reserves.
    pub phloem_down_gc: f32,
    /// Phloem: reserves -> buds and growing shoots (spring reversal).
    pub phloem_up_gc: f32,
    /// Ethanol carbon from root fermentation in hypoxic soil.
    pub ethanol_gc: f32,
    /// Leaf carbon returning to the soil (litterfall).
    pub litter_gc: f32,

    // --- gases (g/day) ---
    /// Taken up by photosynthesis / released by it.
    pub co2_in_g: f32,
    pub o2_out_g: f32,
    /// Respiration (leaves, wood, roots): O2 taken up, CO2 released
    /// (fermentation CO2 included).
    pub o2_in_g: f32,
    pub co2_out_g: f32,
    /// Split of respiration CO2 by organ (g/day): leaves, wood, roots.
    pub co2_out_organs_g: [f32; 3],

    // --- state ---
    /// Root-weighted soil hypoxia 0..1 (own computation, all species).
    pub root_hypoxia: f32,
    /// Water stress 0..1 (1 - the simulation's water factor).
    pub water_stress: f32,
    pub mycorrhiza: MycorrhizaType,
    /// Mycorrhizal activity 0..1 (soil temperature, aeration).
    pub myco_activity: f32,
    pub myco_water_l: f32,
}

impl FluxSnapshot {
    /// Respiration total (g C/day): maintenance + growth.
    pub fn respiration_gc(&self) -> f32 {
        self.resp_leaf_gc + self.resp_wood_gc + self.resp_root_gc + self.resp_growth_gc
    }

    /// Carbon balance residual (g C/day): sources - sinks; ~0.
    pub fn carbon_residual_gc(&self) -> f32 {
        let src = self.gpp_gc + self.reserves_used_gc;
        let sink = self.respiration_gc()
            + self.alloc_shoots_gc
            + self.alloc_wood_gc
            + self.alloc_roots_gc
            + self.alloc_fruits_gc
            + self.alloc_reserves_gc
            + self.alloc_myco_gc;
        src - sink
    }

    /// Water balance residual (L/day): uptake - (transpiration + growth
    /// water + storage change); ~0.
    pub fn water_residual_l(&self) -> f32 {
        self.uptake_l - (self.transpiration_l + self.growth_water_l + self.storage_change_l)
    }

    /// The day's flows as a typed list (zero flows omitted).
    pub fn flows(&self) -> Vec<Flow> {
        use Compartment::*;
        use FlowUnit::*;
        let f = |name, source, sink, rate, unit| Flow {
            name,
            source,
            sink,
            rate,
            unit,
        };
        let all = [
            f("water uptake", Soil, FineRoots, self.uptake_l, LitresPerDay),
            f(
                "hyphal water",
                Fungus,
                FineRoots,
                self.myco_water_l,
                LitresPerDay,
            ),
            f("xylem sap", Roots, Trunk, self.xylem_l, LitresPerDay),
            f(
                "xylem sap",
                Trunk,
                Leaves,
                self.xylem_l - self.growth_water_l,
                LitresPerDay,
            ),
            f(
                "transpiration",
                Leaves,
                Atmosphere,
                self.transpiration_l,
                LitresPerDay,
            ),
            f(
                "interception",
                Leaves,
                Atmosphere,
                self.interception_l,
                LitresPerDay,
            ),
            f(
                "throughfall",
                Atmosphere,
                Soil,
                self.throughfall_l,
                LitresPerDay,
            ),
            f(
                "N uptake",
                Soil,
                Roots,
                self.nutrient_uptake_g[0],
                GramsPerDay,
            ),
            f(
                "P uptake",
                Soil,
                Roots,
                self.nutrient_uptake_g[1],
                GramsPerDay,
            ),
            f(
                "K uptake",
                Soil,
                Roots,
                self.nutrient_uptake_g[2],
                GramsPerDay,
            ),
            f(
                "N in xylem",
                Roots,
                Crown,
                self.nutrient_xylem_g[0],
                GramsPerDay,
            ),
            f(
                "N remobilised",
                Reserves,
                Shoots,
                self.nutrient_remob_g[0],
                GramsPerDay,
            ),
            f(
                "N resorbed",
                Leaves,
                Reserves,
                self.nutrient_resorb_g[0],
                GramsPerDay,
            ),
            f(
                "hyphal N",
                Fungus,
                Roots,
                self.myco_nutrients_g[0],
                GramsPerDay,
            ),
            f(
                "hyphal P",
                Fungus,
                Roots,
                self.myco_nutrients_g[1],
                GramsPerDay,
            ),
            f(
                "photosynthesis",
                Atmosphere,
                Leaves,
                self.gpp_gc,
                GramsCarbonPerDay,
            ),
            f(
                "phloem to sinks",
                Leaves,
                Trunk,
                self.phloem_down_gc,
                GramsCarbonPerDay,
            ),
            f(
                "phloem from reserves",
                Reserves,
                Shoots,
                self.phloem_up_gc,
                GramsCarbonPerDay,
            ),
            f(
                "wood growth",
                Trunk,
                Trunk,
                self.alloc_wood_gc,
                GramsCarbonPerDay,
            ),
            f(
                "root growth",
                Trunk,
                Roots,
                self.alloc_roots_gc,
                GramsCarbonPerDay,
            ),
            f(
                "shoot growth",
                Leaves,
                Shoots,
                self.alloc_shoots_gc,
                GramsCarbonPerDay,
            ),
            f(
                "fruit filling",
                Leaves,
                Fruits,
                self.alloc_fruits_gc,
                GramsCarbonPerDay,
            ),
            f(
                "storage",
                Trunk,
                Reserves,
                self.alloc_reserves_gc,
                GramsCarbonPerDay,
            ),
            f(
                "carbon to fungus",
                Roots,
                Fungus,
                self.alloc_myco_gc,
                GramsCarbonPerDay,
            ),
            f(
                "litterfall",
                Leaves,
                Litter,
                self.litter_gc,
                GramsCarbonPerDay,
            ),
            f("CO2 uptake", Atmosphere, Leaves, self.co2_in_g, GramsPerDay),
            f("O2 release", Leaves, Atmosphere, self.o2_out_g, GramsPerDay),
            f(
                "O2 respiration",
                Atmosphere,
                Trunk,
                self.o2_in_g,
                GramsPerDay,
            ),
            f(
                "CO2 respiration",
                Trunk,
                Atmosphere,
                self.co2_out_g,
                GramsPerDay,
            ),
        ];
        all.into_iter().filter(|x| x.rate > 0.0).collect()
    }
}

#[inline]
fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Funds spent on growth -> (carbon built in, growth respiration).
#[inline]
fn grow(funds: f32) -> (f32, f32) {
    let g = funds / (1.0 + GROWTH_RESP);
    (g, funds - g)
}

impl Simulation {
    /// Root-weighted hypoxia 0..1 of the rooted soil (saturated layers count
    /// fully, others by their air-filled porosity, see `soil.rs`). Computed
    /// for every species (the simulation only tracks it for sensitive ones).
    fn flux_root_hypoxia(v: &FluxView) -> f32 {
        let (mut hy, mut tot) = (0.0, 0.0);
        for l in &v.soil.layers {
            if l.root_len <= 0.0 {
                continue;
            }
            tot += l.root_len;
            hy += l.root_len * if l.saturated() { 1.0 } else { l.hypoxia() };
        }
        if tot > 0.0 {
            hy / tot
        } else {
            0.0
        }
    }

    /// Living sapwood volume (m3) of the shoots (stem + branches) and of the
    /// structural roots.
    fn flux_sapwood_m3(v: &FluxView) -> (f32, f32) {
        let ring = |r: f32| {
            let inner = (r - SAPWOOD_DEPTH).max(0.0);
            std::f32::consts::PI * (r * r - inner * inner)
        };
        let shoot: f32 = v
            .shoots
            .nodes
            .iter()
            .filter(|n| !n.is_snag())
            .map(|n| ring(n.radius()) * n.length)
            .sum();
        let root: f32 = v
            .roots
            .nodes
            .iter()
            .map(|n| ring(n.radius()) * n.length)
            .sum();
        (shoot, root)
    }

    /// The flows of the last simulated day. Pure function of the state.
    pub fn flux_snapshot(&self) -> FluxSnapshot {
        let v = self.flux_view();
        let g = self.genome();
        let w = self.weather();
        let phase = v.phen.phase;
        let leaf = v.phen.leaf_area();
        let doy = (self.day_of_year() + 364) % 365; // the day just simulated
        let cr = self.crown_radius_m().max(0.05);
        let area = std::f32::consts::PI * cr * cr;
        let mut s = FluxSnapshot {
            crown_area_m2: area,
            phase: Some(phase),
            mycorrhiza: g.symbiosis.mycorrhiza,
            ..Default::default()
        };

        // --- water ---
        // Uptake is simulated in mm/day over the rooted soil; over the crown
        // projection 1 mm = 1 L/m2. Stylised: stand transpiration of oak on
        // a sunny day with moist soil is ~2-4 mm/day (Bréda & Granier;
        // BILJOU documentation, INRAE): ~100 L/day for a 40-year oak, a few
        // thousand for a 200-year open-grown oak with a ~35 m crown spread
        // (crown-projection scaling: probably high for such a tree).
        s.uptake_layers_l = v.soil.layers.iter().map(|l| l.uptake * area).collect();
        s.uptake_l = s.uptake_layers_l.iter().sum();
        s.xylem_l = s.uptake_l;
        s.storage_buffer_l = s.uptake_l * if g.wood.ring_porous { 0.04 } else { 0.12 };
        s.rain_l = w.precip_mm * area;
        s.interception_l = s.rain_l * INTERCEPTION * leaf.max(0.3);
        s.throughfall_l = s.rain_l - s.interception_l;
        s.water_stress = 1.0 - v.water_factor.clamp(0.0, 1.0);

        // --- soil air and temperature ---
        let hyp = Self::flux_root_hypoxia(&v);
        s.root_hypoxia = hyp;
        let t_air = w.temp_c;
        let t_soil = 0.4 * t_air + 0.6 * SOIL_T_MEAN;
        let q = |t: f32| Q10.dpowf((t - 15.0) / 10.0);

        // --- photosynthesis ---
        // Same factors as the simulation's daily productivity index (light
        // x temperature x water x nutrients x day length; `sim.rs`), so the
        // sugar flow responds to drought, waterlogging and poor soil like
        // growth does.
        let f_t = ((t_air - 4.0) / 14.0).clamp(0.0, 1.0) * ((38.0 - t_air) / 8.0).clamp(0.0, 1.0);
        let f_nut = (0.6 + 0.5 * v.fertility).min(1.0);
        let photo = leaf * f_t * v.water_factor.clamp(0.0, 1.0) * f_nut * (w.photoperiod_h / 14.0);
        s.gpp_gc = GPP_MAX * area * photo;

        // --- maintenance respiration ---
        let (sap_shoot, sap_root) = Self::flux_sapwood_m3(&v);
        s.resp_leaf_gc = RM_LEAF * area * leaf * q(t_air);
        s.resp_wood_gc = RM_SAPWOOD * sap_shoot * q(t_air);
        // Roots: fine roots (crown-area proxy, only where roots exist) and
        // coarse-root sapwood, at soil temperature.
        let has_roots = v.soil.layers.iter().any(|l| l.root_len > 0.0);
        let root_demand =
            (if has_roots { RM_FINE_ROOT * area } else { 0.0 } + RM_SAPWOOD * sap_root) * q(t_soil);
        s.resp_root_gc = root_demand;
        let rm = s.resp_leaf_gc + s.resp_wood_gc + s.resp_root_gc;

        // --- allocation of the day's assimilates (phloem sinks) ---
        let leaf_c_full = LMA * LAI_FULL * area * C_FRAC;
        let mut avail = s.gpp_gc - rm;
        if avail < 0.0 {
            // Respiration exceeding photosynthesis is paid from local reserves.
            s.reserves_used_gc += -avail;
            avail = 0.0;
        }
        // Share of the root and wood respiration fed by current assimilates
        // through the phloem (the rest: local reserves).
        let fed = if rm > 0.0 {
            (s.gpp_gc / rm).min(1.0)
        } else {
            0.0
        };
        let mut grow_wood = 0.0;
        let mut grow_root = 0.0;
        let mut grow_shoot = 0.0;
        let mut grow_fruit = 0.0;
        let myco_on = g.symbiosis.mycorrhiza != MycorrhizaType::None;
        // Fungi are aerobic and slow in cold soil (calibration).
        s.myco_activity = if myco_on && has_roots {
            smooth(3.0, 10.0, t_soil) * (1.0 - hyp)
        } else {
            0.0
        };
        let rs0 = g.roots.root_shoot_ratio.clamp(0.05, 0.9);
        match phase {
            Phase::BudBurst | Phase::LeafExpansion => {
                // Leaves and shoots are built over ~5 weeks, first from
                // reserves, increasingly from the new leaves' own export.
                let demand =
                    leaf_c_full * (1.0 + SHOOT_WOOD_PER_LEAF) / FLUSH_DAYS * (1.0 + GROWTH_RESP);
                let from_assim = avail.min(demand);
                avail -= from_assim;
                let from_res = demand - from_assim;
                s.reserves_used_gc += from_res;
                s.phloem_up_gc += from_res;
                grow_shoot += demand;
                // Ring-porous species build the earlywood vessels from
                // reserves at budburst (Barbaroux & Bréda 2002).
                if g.wood.ring_porous {
                    let ew = 0.08 * leaf_c_full / FLUSH_DAYS * (1.0 + GROWTH_RESP);
                    s.reserves_used_gc += ew;
                    grow_wood += ew;
                }
                // Surplus of the young canopy: wood and roots.
                grow_wood += avail * (1.0 - rs0);
                grow_root += avail * rs0;
                avail = 0.0;
            }
            Phase::Mature => {
                let myco = g.symbiosis.carbon_cost.clamp(0.0, 0.5) * avail * s.myco_activity;
                s.alloc_myco_gc = myco;
                avail -= myco;
                let fruit_share = if self.repro_stage() == ReproStage::Fruiting {
                    0.3 * self.fruit_density().min(1.0)
                } else {
                    0.0
                };
                // Storage builds up from midsummer (Barbaroux & Bréda 2002:
                // reserve accumulation continues after growth stops).
                let store_share =
                    0.15 + 0.35 * smooth(185.0, 250.0, doy as f32) + 0.3 * s.water_stress;
                let store_share = store_share.min(0.8);
                s.alloc_reserves_gc = avail * store_share;
                let rest = avail * (1.0 - store_share);
                grow_fruit += rest * fruit_share;
                let rest = rest * (1.0 - fruit_share);
                grow_root += rest * rs0;
                grow_wood += rest * (1.0 - rs0);
                avail = 0.0;
            }
            Phase::Colouring | Phase::LeafFall => {
                let myco = g.symbiosis.carbon_cost.clamp(0.0, 0.5) * avail * s.myco_activity;
                s.alloc_myco_gc = myco;
                avail -= myco;
                s.alloc_reserves_gc = avail * 0.7;
                grow_root += avail * 0.3;
                avail = 0.0;
            }
            Phase::Dormant => {
                // Ring-porous species start the earlywood vessels from
                // reserves before budburst, once forcing has begun.
                if g.wood.ring_porous && doy < 170 && v.phen.gdd > 30.0 {
                    let ew = 0.04 * leaf_c_full / FLUSH_DAYS * (1.0 + GROWTH_RESP);
                    s.reserves_used_gc += ew;
                    grow_wood += ew;
                }
                s.alloc_reserves_gc += avail;
                avail = 0.0;
            }
        }
        debug_assert!(avail == 0.0);
        let (gw, rw) = grow(grow_wood);
        let (gr, rr) = grow(grow_root);
        let (gs, rsh) = grow(grow_shoot);
        let (gf, rf) = grow(grow_fruit);
        s.alloc_wood_gc = gw;
        s.alloc_roots_gc = gr;
        s.alloc_shoots_gc = gs;
        s.alloc_fruits_gc = gf;
        s.resp_growth_gc = rw + rr + rsh + rf;
        s.phloem_down_gc = if phase == Phase::Dormant {
            0.0
        } else {
            (grow_wood
                + grow_root
                + s.alloc_reserves_gc
                + s.alloc_myco_gc
                + (s.resp_wood_gc + s.resp_root_gc) * fed)
                .max(0.0)
                - if matches!(phase, Phase::BudBurst | Phase::LeafExpansion) && g.wood.ring_porous {
                    // Earlywood from reserves is fed locally, not from leaves.
                    0.08 * leaf_c_full / FLUSH_DAYS * (1.0 + GROWTH_RESP)
                } else {
                    0.0
                }
        }
        .max(0.0);

        // --- litter (autumn) ---
        if phase == Phase::LeafFall {
            s.litter_gc = leaf_c_full * (1.0 - 0.1) / AUTUMN_DAYS * 0.6;
        }

        // --- respiration gases; root fermentation under hypoxia ---
        // Aerobic: O2 in = CO2 out (RQ ~1). Hypoxic roots ferment: glucose
        // -> 2 ethanol + 2 CO2, so 1/3 of the carbon leaves as CO2 and 2/3
        // as ethanol, with no O2 consumed (Drew 1997).
        let mol = |gc: f32| gc / M_C;
        let leaf_r = s.resp_leaf_gc + rsh + rf;
        let wood_r = s.resp_wood_gc + rw;
        let root_r = s.resp_root_gc + rr;
        let root_aer = root_r * (1.0 - hyp);
        let root_ferm = root_r * hyp;
        s.ethanol_gc = root_ferm * 2.0 / 3.0;
        // Ethanol carbon is part of root respiration's carbon use but not
        // released as CO2.
        s.co2_out_organs_g = [
            mol(leaf_r) * M_CO2,
            mol(wood_r) * M_CO2,
            mol(root_aer + root_ferm / 3.0) * M_CO2,
        ];
        s.co2_out_g = s.co2_out_organs_g.iter().sum();
        s.o2_in_g = mol(leaf_r + wood_r + root_aer) * M_O2;
        s.co2_in_g = mol(s.gpp_gc) * M_CO2;
        s.o2_out_g = mol(s.gpp_gc) * M_O2;

        // --- water built into new tissue (~1 g water per g dry mass) ---
        let dry_new = (gw + gr + gs + gf) / C_FRAC;
        s.growth_water_l = (dry_new * 1.0e-3).min(s.xylem_l * 0.5);
        s.transpiration_l = s.xylem_l - s.growth_water_l;
        s.storage_change_l = 0.0;

        // --- nutrients ---
        let mut demand = [0.0f32; 3];
        let mut remob = [0.0f32; 3];
        for k in 0..3 {
            let leaf_part = gs / (1.0 + SHOOT_WOOD_PER_LEAF);
            let shoot_d =
                (leaf_part * NUT_LEAF[k] + (gs - leaf_part) * NUT_WOOD[k]) / C_FRAC * 1e-3;
            remob[k] = shoot_d * SPRING_REMOB;
            demand[k] = shoot_d - remob[k]
                + (gw * NUT_WOOD[k] + gr * NUT_ROOT[k] + gf * NUT_FRUIT[k]) / C_FRAC * 1e-3;
        }
        // Supply: mass flow + diffusion need transpiring, moist soil and a
        // fertile profile (stylised; Marschner 2012).
        // (Uptake scales with the sap flow: little before the leaves transpire;
        // spring growth then relies on stored N, see `SPRING_REMOB`.)
        let flow_f =
            ((s.uptake_l / area) / 1.5).min(1.0) * (0.3 + 0.7 * v.water_factor.clamp(0.0, 1.0));
        let supply_f = (0.35 + 0.65 * v.fertility.clamp(0.0, 1.0)) * flow_f;
        let shares = [
            g.symbiosis.nitrogen_share,
            g.symbiosis.phosphorus_share,
            0.0,
        ];
        let mut lim = 0.0;
        for k in 0..3 {
            s.nutrient_uptake_g[k] = demand[k] * supply_f.min(1.0);
            s.nutrient_remob_g[k] = remob[k];
            s.myco_nutrients_g[k] =
                s.nutrient_uptake_g[k] * shares[k].clamp(0.0, 1.0) * s.myco_activity;
            let kept = (gr * NUT_ROOT[k] / C_FRAC * 1e-3).min(s.nutrient_uptake_g[k]);
            s.nutrient_xylem_g[k] = s.nutrient_uptake_g[k] - kept;
            if matches!(phase, Phase::Colouring | Phase::LeafFall) {
                s.nutrient_resorb_g[k] =
                    LMA * LAI_FULL * area * NUT_LEAF[k] * 1e-3 * RESORPTION[k] / AUTUMN_DAYS;
            }
        }
        if demand[0] > 0.0 {
            lim = 1.0 - s.nutrient_uptake_g[0] / demand[0];
        }
        s.nutrient_limitation = lim.clamp(0.0, 1.0);
        s.myco_water_l = s.uptake_l * g.symbiosis.water_share.clamp(0.0, 1.0) * s.myco_activity;
        s
    }
}

// ----------------------------------------------------------------------
// Geometry
// ----------------------------------------------------------------------

/// A drawn flow path. `dist` is the signed path length from the root collar
/// (m): positive up the shoots, negative down the roots; for hyphae, the
/// distance from the root it leaves. `weight` is the share 0..1 of the
/// whole-tree flow passing there (pipe model: everything downstream);
/// `radius` the drawn wood radius (m) the line sits on.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FluxPolyline {
    pub points: Vec<[f32; 3]>,
    pub dist: Vec<f32>,
    pub weight: Vec<f32>,
    pub radius: Vec<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GasSiteKind {
    /// Leafy shoot: photosynthesis, transpiration, leaf respiration.
    Leaf,
    /// Bark of stem / branches: wood respiration.
    Wood,
    /// Fine roots: root respiration.
    Root,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GasSite {
    pub pos: [f32; 3],
    pub kind: GasSiteKind,
    /// Relative intensity 0..1 (leaf light, bark area, root density).
    pub weight: f32,
}

/// Sampled flow paths of the tree (deterministic for a seed).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FluxGeometry {
    /// Collar -> leafy shoot tips (each polyline starts where it joins an
    /// earlier one, so shared wood is drawn once).
    pub shoot_paths: Vec<FluxPolyline>,
    /// Collar -> fine-root-bearing root ends.
    pub root_paths: Vec<FluxPolyline>,
    /// Mycorrhizal hyphae from fine roots into the soil.
    pub hyphae: Vec<FluxPolyline>,
    pub gas_sites: Vec<GasSite>,
}

/// Limits of [`Simulation::flux_geometry`].
#[derive(Clone, Copy, Debug)]
pub struct FluxBudget {
    pub shoot_paths: usize,
    pub root_paths: usize,
    pub hyphae: usize,
    pub leaf_sites: usize,
    pub wood_sites: usize,
    pub root_sites: usize,
}

impl Default for FluxBudget {
    fn default() -> Self {
        FluxBudget {
            shoot_paths: 260,
            root_paths: 160,
            hyphae: 140,
            leaf_sites: 700,
            wood_sites: 70,
            root_sites: 110,
        }
    }
}

const S_FLUX: u64 = 0xF1_0C5;

/// Deterministic weighted sampling without replacement (exponential keys).
fn sample(
    cands: &[(usize, f32)],
    k: usize,
    seed: u64,
    stream: u64,
    id: impl Fn(usize) -> u64,
) -> Vec<usize> {
    let mut keyed: Vec<(f32, usize)> = cands
        .iter()
        .filter(|c| c.1 > 0.0)
        .map(|&(i, w)| (rng::exponential(rng::hash(seed, stream, id(i), 0)) / w, i))
        .collect();
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    keyed.truncate(k);
    keyed.into_iter().map(|x| x.1).collect()
}

impl Simulation {
    /// Flow paths, gas-exchange sites and hyphae for drawing, within `budget`.
    pub fn flux_geometry(&self, budget: FluxBudget) -> FluxGeometry {
        let v = self.flux_view();
        let seed = self.seed();
        let g = self.genome();
        let mut out = FluxGeometry::default();
        let shoots = v.shoots;
        let nodes = &shoots.nodes;
        let ext = shoots.ext;
        let n = nodes.len();

        // ---- shoots ----
        // Drawn radius: same stylisation as `Simulation::wood_mesh`.
        let r_ref = nodes[0].radius().max(1e-6);
        let e = g.wood.pipe_exponent.max(2.0);
        let boost_max = g.wood.limb_draw_boost.max(1.0);
        let drawn = |rs: f32| {
            let pipe = (r_ref / rs.max(1e-6)).dpowf(1.0 - 2.0 / e);
            let fade = ((rs - 0.005) / 0.035).clamp(0.0, 1.0);
            let cap = 1.0 + (boost_max - 1.0) * fade * fade * (3.0 - 2.0 * fade);
            rs * pipe.min(cap).max(1.0)
        };
        let mut down = vec![0.0f32; n];
        let mut s_end = vec![0.0f32; n];
        for (i, nd) in nodes.iter().enumerate() {
            if nd.leafy && !nd.is_snag() {
                down[i] = nd.light.max(0.05);
            }
            let s0 = if i == 0 {
                0.0
            } else {
                s_end[nd.parent as usize]
            };
            s_end[i] = s0 + (nd.end(ext) - nd.start).length();
        }
        for i in (1..n).rev() {
            let p = nodes[i].parent as usize;
            down[p] += down[i];
        }
        let total = down[0].max(1e-6);
        let leafy: Vec<(usize, f32)> = (0..n)
            .filter(|&i| nodes[i].leafy && !nodes[i].is_snag())
            .map(|i| (i, down[i]))
            .collect();
        let mut tips = sample(&leafy, budget.shoot_paths, seed, S_FLUX, |i| {
            nodes[i].id as u64
        });
        tips.sort_by(|a, b| down[*b].total_cmp(&down[*a]).then(a.cmp(b)));
        let mut inc = vec![false; n];
        for &t in &tips {
            let mut chain = Vec::new();
            let mut i = t;
            loop {
                if inc[i] {
                    break;
                }
                inc[i] = true;
                chain.push(i);
                if i == 0 {
                    break;
                }
                i = nodes[i].parent as usize;
            }
            if chain.is_empty() {
                continue;
            }
            chain.reverse();
            let mut pl = FluxPolyline::default();
            let first = &nodes[chain[0]];
            let s0 = if chain[0] == 0 {
                0.0
            } else {
                s_end[first.parent as usize]
            };
            pl.points.push(first.start.to_array());
            pl.dist.push(s0);
            pl.weight.push(down[chain[0]] / total);
            pl.radius.push(drawn(first.radius()));
            for &c in &chain {
                let nd = &nodes[c];
                pl.points.push(nd.end(ext).to_array());
                pl.dist.push(s_end[c]);
                pl.weight.push(down[c] / total);
                pl.radius.push(drawn(nd.radius()));
            }
            out.shoot_paths.push(pl);
        }

        // ---- roots ----
        let rnodes = &v.roots.nodes;
        let rn = rnodes.len();
        let rt = &g.roots;
        let path = v.roots.path_starts();
        let k_taper = rt.rapid_taper.max(0.0);
        let r_max = 0.8 * nodes[0].radius();
        let root_r = |i: usize| {
            let nd = &rnodes[i];
            let d = nd.start.horizontal_len();
            if i == 0 {
                nd.radius()
            } else {
                (nd.radius() * (1.0 + k_taper * (-d).dexp())).min(r_max.max(nd.radius()))
            }
        };
        let sink = |p: Vec3, r: f32| Vec3::new(p.x, p.y.min(-1.1 * r), p.z);
        let mut rw = vec![0.0f32; rn];
        let mut fine: Vec<(usize, f32)> = Vec::new();
        for (i, nd) in rnodes.iter().enumerate() {
            if nd.kind == RootKind::Tap && i == 0 && rn > 1 {
                continue;
            }
            if nd.radius() < 0.012 || nd.tip {
                let pr = v
                    .soil
                    .layer_at(-nd.end().y)
                    .map(|l| roots::proliferation(&v.soil.layers[l], rt))
                    .unwrap_or(0.0);
                let w = nd.length * (0.05 + pr);
                rw[i] = w;
                fine.push((i, w));
            }
        }
        let own = rw.clone();
        for i in (1..rn).rev() {
            let p = rnodes[i].parent as usize;
            rw[p] += rw[i];
        }
        let rtotal = rw[0].max(1e-6);
        let mut rtips = sample(&fine, budget.root_paths, seed, S_FLUX + 1, |i| {
            rnodes[i].id as u64
        });
        rtips.sort_by(|a, b| rw[*b].total_cmp(&rw[*a]).then(a.cmp(b)));
        let mut rinc = vec![false; rn];
        for &t in &rtips {
            let mut chain = Vec::new();
            let mut i = t;
            loop {
                if rinc[i] {
                    break;
                }
                rinc[i] = true;
                chain.push(i);
                if i == 0 {
                    break;
                }
                i = rnodes[i].parent as usize;
            }
            if chain.is_empty() {
                continue;
            }
            chain.reverse();
            let mut pl = FluxPolyline::default();
            let c0 = chain[0];
            let r0 = root_r(c0);
            let p0 = if c0 == 0 {
                rnodes[0].start
            } else {
                sink(rnodes[c0].start, r0)
            };
            pl.points.push(p0.to_array());
            pl.dist.push(-path[c0]);
            pl.weight.push(rw[c0] / rtotal);
            pl.radius.push(r0);
            for &c in &chain {
                let r = root_r(c);
                pl.points.push(sink(rnodes[c].end(), r).to_array());
                pl.dist.push(-(path[c] + rnodes[c].length));
                pl.weight.push(rw[c] / rtotal);
                pl.radius.push(r);
            }
            out.root_paths.push(pl);
        }

        // ---- hyphae ----
        let reach = g.symbiosis.hyphal_reach_m;
        if g.symbiosis.mycorrhiza != MycorrhizaType::None && reach > 0.0 {
            let hroots = sample(&fine, budget.hyphae, seed, S_FLUX + 2, |i| {
                rnodes[i].id as u64
            });
            for (k, &i) in hroots.iter().enumerate() {
                let nd = &rnodes[i];
                let h = rng::hash(seed, S_FLUX + 3, nd.id as u64, k as u64);
                let u = |s: u64| rng::unit(rng::mix(h ^ s.wrapping_mul(0x9E37)));
                let start = sink(nd.end(), root_r(i));
                let out_dir =
                    Vec3::new(start.x, 0.0, start.z).normalize_or(Vec3::new(1.0, 0.0, 0.0));
                let mut dir = (out_dir * 0.5
                    + Vec3::new(u(1) - 0.5, (u(2) - 0.6) * 0.8, u(3) - 0.5) * 1.6)
                    .normalize_or(out_dir);
                let len = reach * (0.6 + 0.4 * u(4));
                let mut p = start;
                let mut pl = FluxPolyline::default();
                pl.points.push(p.to_array());
                pl.dist.push(0.0);
                pl.weight.push(1.0);
                pl.radius.push(0.0);
                const STEPS: usize = 4;
                for st in 0..STEPS {
                    let jit = Vec3::new(
                        u(10 + st as u64) - 0.5,
                        u(20 + st as u64) - 0.5,
                        u(30 + st as u64) - 0.5,
                    );
                    dir = (dir + jit * 0.9).normalize_or(dir);
                    p += dir * (len / STEPS as f32);
                    p.y = p.y.min(-0.02);
                    pl.points.push(p.to_array());
                    pl.dist.push(len * (st + 1) as f32 / STEPS as f32);
                    pl.weight.push(1.0 - 0.2 * (st + 1) as f32 / STEPS as f32);
                    pl.radius.push(0.0);
                }
                out.hyphae.push(pl);
            }
        }

        // ---- gas exchange sites ----
        let lights: Vec<(usize, f32)> = leafy
            .iter()
            .map(|&(i, _)| (i, nodes[i].light.max(0.05)))
            .collect();
        for i in sample(&lights, budget.leaf_sites, seed, S_FLUX + 4, |i| {
            nodes[i].id as u64
        }) {
            out.gas_sites.push(GasSite {
                pos: nodes[i].end(ext).to_array(),
                kind: GasSiteKind::Leaf,
                weight: nodes[i].light.clamp(0.05, 1.0),
            });
        }
        let bark: Vec<(usize, f32)> = (0..n)
            .filter(|&i| !nodes[i].is_snag() && !nodes[i].leafy)
            .map(|i| (i, nodes[i].radius() * nodes[i].length))
            .collect();
        let bmax = bark.iter().map(|b| b.1).fold(0.0, f32::max).max(1e-9);
        for i in sample(&bark, budget.wood_sites, seed, S_FLUX + 5, |i| {
            nodes[i].id as u64
        }) {
            let nd = &nodes[i];
            let h = rng::hash(seed, S_FLUX + 6, nd.id as u64, 0);
            let u = |s: u64| rng::unit(rng::mix(h ^ s));
            let a = nd.dir.any_perpendicular();
            let b = nd.dir.cross(a);
            let ang = u(1) * std::f32::consts::TAU;
            let along = nd.start + nd.dir * ((nd.end(ext) - nd.start).length() * u(2));
            let p = along + (a * ang.dcos() + b * ang.dsin()) * drawn(nd.radius());
            out.gas_sites.push(GasSite {
                pos: p.to_array(),
                kind: GasSiteKind::Wood,
                weight: (nd.radius() * nd.length / bmax).sqrt(),
            });
        }
        let omax = own.iter().copied().fold(0.0, f32::max).max(1e-9);
        for i in sample(&fine, budget.root_sites, seed, S_FLUX + 7, |i| {
            rnodes[i].id as u64
        }) {
            let p = sink(rnodes[i].end(), root_r(i));
            out.gas_sites.push(GasSite {
                pos: p.to_array(),
                kind: GasSiteKind::Root,
                weight: (own[i] / omax).sqrt().max(0.2),
            });
        }
        out
    }
}

impl FluxGeometry {
    /// Ribbon quads for a shader: one quad per segment and strand, expanded
    /// in the vertex shader (camera-facing, pushed out to the bark).
    /// Per vertex: position = segment end; normal = unit segment direction;
    /// uv = (signed path distance m, side -1/+1); uv2 = (flow weight 0..1,
    /// drawn wood radius m); colour = (strand position 0..1 across the
    /// bundle, `part` (0 shoot, 0.5 root, 1 hypha), per-strand key 0..1,
    /// strands / 8). Thick, heavily used wood gets up to 4 strands.
    pub fn ribbon_mesh(paths: &[FluxPolyline], part: f32) -> MeshData {
        let mut m = MeshData::default();
        for (pi, pl) in paths.iter().enumerate() {
            for k in 1..pl.points.len() {
                let a = Vec3::new(
                    pl.points[k - 1][0],
                    pl.points[k - 1][1],
                    pl.points[k - 1][2],
                );
                let b = Vec3::new(pl.points[k][0], pl.points[k][1], pl.points[k][2]);
                let d = b - a;
                if d.length() < 1e-5 {
                    continue;
                }
                let dir = d.normalize_or(Vec3::Y).to_array();
                let wseg = pl.weight[k - 1].max(pl.weight[k]);
                let strands = if part > 0.9 {
                    1
                } else {
                    (1.0 + 3.0 * wseg.sqrt()).round().clamp(1.0, 4.0) as usize
                };
                for st in 0..strands {
                    let frac = if strands == 1 {
                        0.5
                    } else {
                        st as f32 / (strands - 1) as f32
                    };
                    let key = rng::unit(rng::hash(0x5EED, S_FLUX + 8, pi as u64, st as u64));
                    let base = m.positions.len() as u32;
                    for (p, s, w, r) in [
                        (a, pl.dist[k - 1], pl.weight[k - 1], pl.radius[k - 1]),
                        (b, pl.dist[k], pl.weight[k], pl.radius[k]),
                    ] {
                        for side in [-1.0f32, 1.0] {
                            m.positions.push(p.to_array());
                            m.normals.push(dir);
                            m.uvs.push([s, side]);
                            m.uv2.push([w, r]);
                            m.colors.push([frac, part, key, strands as f32 / 8.0]);
                        }
                    }
                    m.indices.extend_from_slice(&[
                        base,
                        base + 2,
                        base + 1,
                        base + 1,
                        base + 2,
                        base + 3,
                    ]);
                }
            }
        }
        m
    }
}
