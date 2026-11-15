//! Layered soil: profile description (data) and a daily 1-D water balance.
//!
//! A [`SoilProfile`] lists horizons (O, A, B, C) with texture, bulk density,
//! penetration resistance, water retention and nutrient status, plus an
//! optional permanent water table and bedrock depth. Presets live in
//! `data/soils/*.ron`.
//!
//! The simulation discretises the profile into ~10 cm layers in the top
//! 60 cm and ~25 cm layers below ([`SoilLayerState`] exposes them). Daily
//! water balance, in the spirit of tipping-bucket models such as BILJOU
//! (Granier et al. 1999) and the cascade of SWAT/CERES:
//! * throughfall (90 % of rain) enters the top layer; water above
//!   saturation passes on at once, water above field capacity drains to the
//!   next layer at most at the horizon's conductivity per day;
//! * below the last layer water is lost (free drainage), held up (bedrock:
//!   perched water) or joins the water table; layers below the water table
//!   stay saturated and feed a small capillary rise into the layer above;
//! * soil evaporation comes from the top ~15 cm and declines as they dry;
//! * transpiration is shared among layers in proportion to root length
//!   density x relative extractable water, and stops in saturated
//!   (anoxic) layers. Plant water status is the relative extractable water
//!   of the *rooted* profile (BILJOU's REW), so deep roots buffer droughts.
//!
//! Honest limits: no Richards-equation flow, no hysteresis, no frozen soil,
//! no lateral flow, no seasonal water-table fluctuation (perched water in
//! winter on slowly draining clay does appear through the cascade).
//! Retention values are given per horizon in the data (typical values from
//! texture classes, e.g. Saxton & Rawls 2006 pedotransfer functions).

use serde::{Deserialize, Serialize};
use std::fmt;

/// RON source of the built-in default soil (`data/soils/brown_forest_soil.ron`).
pub const BROWN_FOREST_SOIL_RON: &str = include_str!("../../../data/soils/brown_forest_soil.ron");
/// Shallow soil over limestone (`data/soils/shallow_rendzina.ron`).
pub const SHALLOW_RENDZINA_RON: &str = include_str!("../../../data/soils/shallow_rendzina.ron");
/// Heavy, waterlogged clay (`data/soils/gley_clay.ron`).
pub const GLEY_CLAY_RON: &str = include_str!("../../../data/soils/gley_clay.ron");
/// Dry, deep sand (`data/soils/sandy_podzol.ron`).
pub const SANDY_PODZOL_RON: &str = include_str!("../../../data/soils/sandy_podzol.ron");

/// Soil horizon designation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HorizonKind {
    /// Organic layer / litter (O).
    Organic,
    /// Mineral topsoil enriched in humus (A; also E, eluvial, in podzols).
    Topsoil,
    /// Subsoil (B; Bt, Bw, Bg...).
    Subsoil,
    /// Weathered parent material (C).
    Parent,
}

/// One soil horizon (data).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Horizon {
    pub kind: HorizonKind,
    /// Conventional label, e.g. "A", "Bt", "Cg".
    pub name: String,
    /// Thickness (m).
    pub thickness_m: f32,
    /// Particle-size fractions (0..1, sum ~1) of the fine earth.
    pub sand: f32,
    pub silt: f32,
    pub clay: f32,
    /// Bulk density (g/cm3).
    pub bulk_density_g_cm3: f32,
    /// Penetration resistance at field capacity (MPa). Root elongation
    /// halves around 1-2 MPa and stops around 2.5-3 MPa (Bengough et al.
    /// 2011).
    pub penetration_resistance_mpa: f32,
    /// Volumetric water content at field capacity (-33 kPa), m3/m3.
    pub field_capacity: f32,
    /// Volumetric water content at the wilting point (-1500 kPa), m3/m3.
    pub wilting_point: f32,
    /// Volume fraction of stones (no water, no roots).
    #[serde(default)]
    pub stones: f32,
    /// Saturated drainage (mm/day) out of the horizon (~ conductivity).
    pub drainage_mm_day: f32,
    /// Organic matter (% mass).
    pub organic_matter_pct: f32,
    /// Relative nutrient availability 0..1 (fine roots proliferate where
    /// it is high; Hodge 2004).
    pub nutrient_index: f32,
}

impl Horizon {
    /// Total porosity (saturated water content), from bulk density.
    pub fn porosity(&self) -> f32 {
        (1.0 - self.bulk_density_g_cm3 / 2.65).clamp(self.field_capacity + 0.02, 0.9)
    }

    /// Aeration: air-filled porosity at field capacity (m3/m3); below
    /// ~0.10 roots suffer from poor aeration (Grable & Siemer 1968).
    pub fn aeration(&self) -> f32 {
        (self.porosity() - self.field_capacity).max(0.0)
    }

    /// USDA texture class name (approximate, from the triangle).
    pub fn texture_class(&self) -> &'static str {
        let (s, c, si) = (self.sand, self.clay, self.silt);
        if c >= 0.40 {
            if si >= 0.40 {
                "silty clay"
            } else if s > 0.45 {
                "sandy clay"
            } else {
                "clay"
            }
        } else if c >= 0.27 {
            if s > 0.45 {
                "sandy clay loam"
            } else if si >= 0.40 {
                "silty clay loam"
            } else {
                "clay loam"
            }
        } else if s >= 0.85 {
            "sand"
        } else if s >= 0.70 {
            "loamy sand"
        } else if si >= 0.80 {
            "silt"
        } else if si >= 0.50 {
            "silt loam"
        } else if s > 0.52 {
            "sandy loam"
        } else {
            "loam"
        }
    }
}

/// A soil profile (data): horizons from the surface down.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoilProfile {
    pub name: String,
    pub description: String,
    pub horizons: Vec<Horizon>,
    /// Depth (m) of a permanent water table, if any (layers below are
    /// saturated, i.e. anoxic for roots).
    pub water_table_depth_m: Option<f32>,
    /// Depth (m) of hard, unfissured bedrock, if any (roots and water stop).
    pub bedrock_depth_m: Option<f32>,
}

/// Error parsing a soil file.
#[derive(Debug, Clone)]
pub struct SoilError(pub String);

impl fmt::Display for SoilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "soil parse error: {}", self.0)
    }
}
impl std::error::Error for SoilError {}

impl SoilProfile {
    /// Parse a profile from RON text.
    pub fn from_ron(text: &str) -> Result<SoilProfile, SoilError> {
        let p: SoilProfile = ron::from_str(text).map_err(|e| SoilError(e.to_string()))?;
        if p.horizons.is_empty() || p.horizons.iter().any(|h| h.thickness_m <= 0.0 || h.field_capacity <= h.wilting_point) {
            return Err(SoilError("horizons must be non-empty, thickness > 0, field capacity > wilting point".into()));
        }
        Ok(p)
    }

    /// Serialise to pretty RON.
    pub fn to_ron(&self) -> String {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).expect("soil serialisation cannot fail")
    }

    /// Default site: deep, well-drained loamy brown forest soil (cambisol /
    /// luvisol).
    pub fn brown_forest_soil() -> SoilProfile {
        SoilProfile::from_ron(BROWN_FOREST_SOIL_RON).expect("built-in soil must parse")
    }
    /// Shallow stony soil over hard limestone at 45 cm (rendzina / leptosol).
    pub fn shallow_rendzina() -> SoilProfile {
        SoilProfile::from_ron(SHALLOW_RENDZINA_RON).expect("built-in soil must parse")
    }
    /// Heavy clay with a permanent water table at 70 cm (stagnogley).
    pub fn gley_clay() -> SoilProfile {
        SoilProfile::from_ron(GLEY_CLAY_RON).expect("built-in soil must parse")
    }
    /// Deep, coarse, nutrient-poor sand (podzol / arenosol).
    pub fn sandy_podzol() -> SoilProfile {
        SoilProfile::from_ron(SANDY_PODZOL_RON).expect("built-in soil must parse")
    }

    /// All built-in presets (name, profile).
    pub fn presets() -> Vec<SoilProfile> {
        vec![Self::brown_forest_soil(), Self::shallow_rendzina(), Self::gley_clay(), Self::sandy_podzol()]
    }

    /// Total depth described by the horizons (m).
    pub fn described_depth_m(&self) -> f32 {
        self.horizons.iter().map(|h| h.thickness_m).sum()
    }

    /// Depth (m) of the simulated column: down to bedrock, else the
    /// described horizons (the last one extends) down to at least 5 m.
    pub fn column_depth_m(&self) -> f32 {
        match self.bedrock_depth_m {
            Some(b) => b.max(0.1),
            None => self.described_depth_m().max(5.0),
        }
    }

    /// Horizon at a depth (m, positive down); the last horizon extends
    /// downward.
    pub fn horizon_at(&self, depth: f32) -> &Horizon {
        let mut z = 0.0;
        for h in &self.horizons {
            z += h.thickness_m;
            if depth < z {
                return h;
            }
        }
        self.horizons.last().expect("non-empty")
    }
}

impl Default for SoilProfile {
    fn default() -> Self {
        Self::brown_forest_soil()
    }
}

/// State of one simulated soil layer (for the UI and diagnostics).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoilLayerState {
    /// Top and bottom depth (m, positive down).
    pub top_m: f32,
    pub bottom_m: f32,
    pub kind: HorizonKind,
    /// Relative extractable water 0..1 ((theta - wilting) / (field capacity
    /// - wilting)); 1 also when wetter than field capacity.
    pub water: f32,
    /// Fraction of pore space filled with water 0..1 (1 = saturated).
    pub saturation: f32,
    /// Saturated (anoxic for roots) today.
    pub waterlogged: bool,
    /// Root length density index 0..1 (fine + structural, relative to the
    /// densest layer).
    pub root_density: f32,
    /// Absolute root length density (km of root per m3 of soil, fine roots
    /// included; order-of-magnitude, see `roots.rs`).
    pub root_length_km_m3: f32,
    /// Water taken up by the roots from this layer on the last simulated
    /// day (mm), i.e. its share of the actual transpiration.
    pub uptake_mm: f32,
}

/// Internal per-layer data.
#[derive(Clone, Debug)]
pub(crate) struct Layer {
    pub top: f32,
    pub bottom: f32,
    pub kind: HorizonKind,
    /// mm of water at saturation / field capacity / wilting point.
    sat: f32,
    fc: f32,
    wp: f32,
    /// Current water (mm).
    water: f32,
    drain: f32,
    pub resistance: f32,
    pub nutrient: f32,
    /// Below the permanent water table.
    pub below_table: bool,
    /// Root length (m) of the tree in this layer, fine roots included.
    pub root_len: f32,
    /// Structural (explicit) root length (m) in this layer.
    pub struct_len: f32,
    /// Mean relative extractable water over the last season (for roots).
    pub season_rew: f32,
    season_sum: f32,
    season_n: f32,
    /// Days saturated during the last growing season (fraction).
    pub season_wet: f32,
    wet_sum: f32,
    /// Root water uptake from this layer on the last simulated day (mm).
    pub uptake: f32,
}

impl Layer {
    #[inline]
    pub fn rew(&self) -> f32 {
        ((self.water - self.wp) / (self.fc - self.wp).max(1e-3)).clamp(0.0, 1.0)
    }
    #[inline]
    pub fn saturated(&self) -> bool {
        self.below_table || self.water >= self.fc + 0.85 * (self.sat - self.fc)
    }
    #[inline]
    pub fn thick(&self) -> f32 {
        self.bottom - self.top
    }
    /// Oxygen shortage of the layer 0..1 from its air-filled porosity
    /// (volume fraction): 0 at >= 10 %, 1 when no air is left.
    #[inline]
    pub fn hypoxia(&self) -> f32 {
        let afp = (self.sat - self.water).max(0.0) / (self.thick() * 1000.0).max(1e-3);
        (1.0 - afp / 0.10).clamp(0.0, 1.0)
    }
}

/// Daily outputs of the water balance.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WaterDay {
    /// Relative extractable water of the rooted profile 0..1.
    pub rew: f32,
    /// Root-weighted fraction of the root system in saturated layers.
    pub anoxic: f32,
    /// Root-weighted hypoxia 0..1 (saturated layers count fully, others by
    /// their air-filled porosity, see `Layer::hypoxia`).
    pub hypoxic: f32,
    /// Actual transpiration taken up by the roots (mm/day).
    pub uptake: f32,
}

/// The simulated soil column.
#[derive(Clone, Debug)]
pub(crate) struct SoilColumn {
    pub profile: SoilProfile,
    pub layers: Vec<Layer>,
    /// Depth (m) below which roots cannot grow (bedrock), if any.
    pub bedrock: Option<f32>,
    pub last: WaterDay,
    /// Ground area (m2) occupied by the root system (for densities).
    pub root_zone_m2: f32,
    /// Compute root hypoxia (only species sensitive to it need it).
    pub track_hypoxia: bool,
    states: Vec<SoilLayerState>,
}

impl SoilColumn {
    pub fn new(profile: SoilProfile) -> Self {
        let depth = profile.column_depth_m();
        let table = profile.water_table_depth_m;
        // Layer boundaries: horizon boundaries, 10 cm steps to 0.6 m, then
        // 25 cm steps.
        let mut bounds = vec![0.0f32];
        let mut z = 0.0f32;
        let mut hz_ends = Vec::new();
        let mut acc = 0.0;
        for h in &profile.horizons {
            acc += h.thickness_m;
            hz_ends.push(acc);
        }
        if let Some(t) = table {
            hz_ends.push(t);
        }
        while z < depth - 1e-4 {
            let step = if z < 0.6 - 1e-4 { 0.1 } else { 0.25 };
            let mut next = (z + step).min(depth);
            for &e in &hz_ends {
                if e > z + 0.02 && e < next - 0.02 {
                    next = e;
                }
            }
            bounds.push(next);
            z = next;
        }
        let layers = bounds
            .windows(2)
            .map(|w| {
                let (top, bottom) = (w[0], w[1]);
                let h = profile.horizon_at(0.5 * (top + bottom));
                let mm = (bottom - top) * 1000.0 * (1.0 - h.stones);
                let below_table = table.is_some_and(|t| top >= t - 1e-4);
                let sat = h.porosity() * mm;
                let fc = h.field_capacity * mm;
                Layer {
                    top,
                    bottom,
                    kind: h.kind,
                    sat,
                    fc,
                    wp: h.wilting_point * mm,
                    water: if below_table { sat } else { fc },
                    drain: h.drainage_mm_day,
                    resistance: h.penetration_resistance_mpa,
                    nutrient: h.nutrient_index,
                    below_table,
                    root_len: 0.0,
                    struct_len: 0.0,
                    season_rew: 1.0,
                    season_sum: 0.0,
                    season_n: 0.0,
                    season_wet: if below_table { 1.0 } else { 0.0 },
                    wet_sum: 0.0,
                    uptake: 0.0,
                }
            })
            .collect();
        let mut c = SoilColumn {
            bedrock: profile.bedrock_depth_m,
            profile,
            layers,
            last: WaterDay { rew: 1.0, anoxic: 0.0, hypoxic: 0.0, uptake: 0.0 },
            root_zone_m2: 1.0,
            track_hypoxia: false,
            states: Vec::new(),
        };
        c.refresh_states();
        c
    }

    /// Index of the layer containing `depth` (m, positive down), if inside.
    pub fn layer_at(&self, depth: f32) -> Option<usize> {
        if depth < 0.0 {
            return None;
        }
        self.layers.iter().position(|l| depth < l.bottom)
    }

    /// Daily water balance. `precip` and `pet` in mm; `leaf` = canopy leaf
    /// area 0..1 of full summer canopy. Returns rooted-profile REW and
    /// anoxic root fraction.
    pub fn step(&mut self, precip: f32, pet: f32, leaf: f32, season: bool) -> WaterDay {
        let n = self.layers.len();
        // 1. Infiltration and drainage cascade (one pass, top-down).
        let mut carry = precip * 0.9; // ~10 % interception loss
        for i in 0..n {
            let l = &mut self.layers[i];
            if l.below_table {
                // Water reaching the table is lost to groundwater flow.
                carry = 0.0;
                l.water = l.sat;
                continue;
            }
            l.water += carry;
            carry = 0.0;
            if l.water > l.sat {
                carry += l.water - l.sat;
                l.water = l.sat;
            }
            if l.water > l.fc {
                let d = (l.water - l.fc).min(l.drain);
                l.water -= d;
                carry += d;
            }
        }
        // Bottom boundary: bedrock holds the water back (perched water
        // table filling the column from below); otherwise free drainage.
        if self.bedrock.is_some() && carry > 0.0 {
            for l in self.layers.iter_mut().rev() {
                let room = l.sat - l.water;
                let a = room.min(carry);
                l.water += a;
                carry -= a;
                if carry <= 0.0 {
                    break;
                }
            }
            // A small leak through fissures keeps it from flooding forever.
            if let Some(l) = self.layers.last_mut() {
                let leak = (l.water - l.fc).clamp(0.0, 1.5);
                l.water -= leak;
            }
        }
        // 2. Capillary rise from the water table into the layer above it
        // (up to ~2 mm/day when that layer is dry).
        if let Some(k) = self.layers.iter().position(|l| l.below_table) {
            if k > 0 {
                let l = &mut self.layers[k - 1];
                let deficit = (l.fc - l.water).max(0.0);
                l.water += deficit.min(2.0 * (1.0 - l.rew()) + 0.2);
            }
        }
        // 3. Soil evaporation from the top ~15 cm (stage-2 decline).
        let e_soil = pet * 0.3 * (1.0 - 0.5 * leaf);
        let mut want = e_soil;
        for l in self.layers.iter_mut() {
            if l.top >= 0.15 || want <= 0.0 {
                break;
            }
            let w = (l.water - 0.5 * l.wp).max(0.0);
            let e = (want * (l.rew() * 0.8 + 0.2 * (l.water > l.fc) as u8 as f32)).min(w);
            l.water -= e;
            want -= e;
        }
        // 4. Transpiration, shared by root length x extractable water.
        let rew = self.rooted_rew();
        let t_pot = pet * 0.7 * leaf;
        let t_act = t_pot * (rew / 0.4).min(1.0);
        let mut wsum = 0.0;
        for l in &self.layers {
            if !l.saturated() {
                wsum += l.root_len * l.rew();
            }
        }
        let mut uptake = 0.0;
        for l in self.layers.iter_mut() {
            l.uptake = 0.0;
        }
        if t_act > 0.0 && wsum > 0.0 {
            for l in self.layers.iter_mut() {
                if l.saturated() {
                    continue;
                }
                let share = t_act * l.root_len * l.rew() / wsum;
                let taken = share.min((l.water - l.wp).max(0.0));
                l.water -= taken;
                l.uptake = taken;
                uptake += taken;
            }
        }
        // Rooted fraction in saturated layers (anoxia), and root-weighted
        // hypoxia: air-filled porosity below ~10 % (Grable & Siemer 1968)
        // starves roots of oxygen even short of saturation (heavy clays).
        let total_root: f32 = self.layers.iter().map(|l| l.root_len).sum();
        let (anoxic, hypoxic) = if total_root > 0.0 && !self.track_hypoxia {
            (self.layers.iter().filter(|l| l.saturated()).map(|l| l.root_len).sum::<f32>() / total_root, 0.0)
        } else if total_root > 0.0 {
            let mut an = 0.0;
            let mut hy = 0.0;
            for l in &self.layers {
                if l.root_len <= 0.0 {
                    continue;
                }
                if l.saturated() {
                    an += l.root_len;
                    hy += l.root_len;
                } else {
                    hy += l.root_len * l.hypoxia();
                }
            }
            (an / total_root, hy / total_root)
        } else {
            (0.0, 0.0)
        };
        if season {
            for l in self.layers.iter_mut() {
                l.season_sum += l.rew();
                l.season_n += 1.0;
                l.wet_sum += l.saturated() as u8 as f32;
            }
        }
        self.last = WaterDay { rew: self.rooted_rew(), anoxic, hypoxic, uptake };
        self.refresh_states();
        self.last
    }

    /// Relative extractable water of the rooted profile: layers count by
    /// root presence (a layer with >= 10 % of the densest layer's root
    /// length counts fully), as in BILJOU's rooted-zone REW. Saturated
    /// layers count as full.
    pub fn rooted_rew(&self) -> f32 {
        let max = self.layers.iter().map(|l| l.root_len / l.thick().max(1e-3)).fold(0.0, f32::max);
        let (mut num, mut den) = (0.0, 0.0);
        for l in &self.layers {
            let p = if max > 0.0 { (l.root_len / l.thick().max(1e-3) / (0.1 * max)).min(1.0) } else if l.top < 0.3 { 1.0 } else { 0.0 };
            if p <= 0.0 {
                continue;
            }
            num += p * (l.water.min(l.fc) - l.wp).max(0.0);
            den += p * (l.fc - l.wp);
        }
        if den > 0.0 {
            (num / den).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    /// Close the season's averages (call once a year, e.g. at leaf fall).
    pub fn close_season(&mut self) {
        for l in self.layers.iter_mut() {
            if l.season_n > 0.0 {
                l.season_rew = l.season_sum / l.season_n;
                l.season_wet = l.wet_sum / l.season_n;
            }
            l.season_sum = 0.0;
            l.season_n = 0.0;
            l.wet_sum = 0.0;
        }
    }

    /// Depth (m) where roots stop: bedrock, or the top of the permanently
    /// waterlogged zone plus what the species tolerates (`tolerance` 0..1
    /// lets roots enter it by up to 0.4 m; Q. robur roots survive some
    /// waterlogging, Q. petraea much less: Parelle et al. 2006).
    pub fn root_barrier(&self, tolerance: f32) -> f32 {
        let mut b = self.profile.column_depth_m();
        if let Some(r) = self.bedrock {
            b = b.min(r);
        }
        // Permanently (>= 85 % of the season) saturated layers.
        if let Some(l) = self.layers.iter().find(|l| l.below_table || l.season_wet >= 0.85) {
            b = b.min(l.top + 0.4 * tolerance.clamp(0.0, 1.0));
        }
        b
    }

    pub fn refresh_states(&mut self) {
        let max = self.layers.iter().map(|l| l.root_len / l.thick().max(1e-3)).fold(0.0, f32::max);
        self.states = self
            .layers
            .iter()
            .map(|l| {
                let dens = l.root_len / l.thick().max(1e-3);
                SoilLayerState {
                    top_m: l.top,
                    bottom_m: l.bottom,
                    kind: l.kind,
                    water: l.rew(),
                    saturation: (l.water / l.sat.max(1e-3)).clamp(0.0, 1.0),
                    waterlogged: l.saturated(),
                    root_density: if max > 0.0 { (dens / max).clamp(0.0, 1.0) } else { 0.0 },
                    // Root length per m3 of soil within the root zone.
                    root_length_km_m3: dens / self.root_zone_m2.max(1e-3) * 1e-3,
                    uptake_mm: l.uptake,
                }
            })
            .collect();
    }

    pub fn states(&self) -> &[SoilLayerState] {
        &self.states
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_parse_and_roundtrip() {
        for p in SoilProfile::presets() {
            let back = SoilProfile::from_ron(&p.to_ron()).unwrap();
            assert_eq!(p, back);
            let c = SoilColumn::new(p.clone());
            assert!(!c.layers.is_empty());
            for w in c.layers.windows(2) {
                assert!((w[0].bottom - w[1].top).abs() < 1e-5, "{}: layers must tile", p.name);
            }
            assert!((c.layers.last().unwrap().bottom - p.column_depth_m()).abs() < 1e-4);
        }
        assert_eq!(SoilProfile::brown_forest_soil().horizons[1].texture_class(), "loam");
    }

    #[test]
    fn rain_infiltrates_and_drains() {
        let mut c = SoilColumn::new(SoilProfile::sandy_podzol());
        // Dry it out, then rain.
        for _ in 0..60 {
            c.step(0.0, 4.0, 1.0, true);
        }
        for l in c.layers.iter_mut() {
            l.root_len = if l.top < 1.0 { 1.0 } else { 0.0 };
        }
        let dry = c.layers[0].rew();
        c.step(30.0, 0.0, 0.0, false);
        assert!(c.layers[0].rew() > dry);
        // Free-draining sand never holds water above field capacity long.
        for _ in 0..10 {
            c.step(0.0, 0.0, 0.0, false);
        }
        assert!(c.layers.iter().all(|l| !l.saturated()));
        // The water table keeps the gley's lower layers saturated.
        let g = SoilColumn::new(SoilProfile::gley_clay());
        assert!(g.layers.last().unwrap().saturated());
        assert!(g.root_barrier(0.5) < 1.0);
        let r = SoilColumn::new(SoilProfile::shallow_rendzina());
        assert!(r.root_barrier(1.0) <= 0.46);
    }
}
