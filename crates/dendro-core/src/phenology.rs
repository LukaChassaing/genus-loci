//! Leaf phenology: dormancy, budburst, leaf expansion, summer, autumn
//! colouring and leaf fall.
//!
//! * Budburst: sequential/parallel chilling-forcing model (Cannell & Smith
//!   1983 style): growing degree days from Jan 1, with the forcing
//!   requirement raised when chilling is incomplete.
//! * Senescence: photoperiod-gated cold-degree-day sum (after Delpierre et al.
//!   2009, simplified).
//! * Late frost on expanding leaves reduces leaf area, recovered by re-flushing.

use crate::climate::Weather;
use crate::genome::{Genome, Pigments};

/// Phenological phase of the foliage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Winter dormancy, no (live) leaves.
    Dormant,
    /// Buds opening, first leaves emerging.
    BudBurst,
    /// Leaves and shoots expanding.
    LeafExpansion,
    /// Fully expanded green canopy (summer).
    Mature,
    /// Autumn colouring (chlorophyll breakdown).
    Colouring,
    /// Leaves falling.
    LeafFall,
}

/// Calendar season (northern hemisphere, meteorological).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Season {
    Winter,
    Spring,
    Summer,
    Autumn,
}

impl Season {
    /// Meteorological season of a 0-based day of year.
    pub fn from_doy(doy: u32) -> Season {
        match doy {
            59..=150 => Season::Spring,
            151..=242 => Season::Summer,
            243..=333 => Season::Autumn,
            _ => Season::Winter,
        }
    }
}

/// Events emitted by the daily phenology update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PhenoEvent {
    None,
    Budburst,
    ExpansionComplete,
    SenescenceStart,
    LeafFallComplete,
}

#[derive(Clone, Debug)]
pub(crate) struct Phenology {
    pub phase: Phase,
    chill: f32,
    gdd: f32,
    gdd_since_bb: f32,
    /// Leaf expansion 0..1.
    pub expansion: f32,
    /// Shoot extension 0..1 (current-year shoots).
    pub shoot_ext: f32,
    cdd: f32,
    /// Autumn colouring 0..1.
    pub colouring: f32,
    /// Fraction of leaves fallen 0..1.
    pub fallen: f32,
    fall_days: f32,
    /// Fraction of leaf area lost to late frost 0..1 (recovers).
    pub frost_damage: f32,
}

#[inline]
fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

impl Phenology {
    pub fn new() -> Self {
        Phenology {
            phase: Phase::Dormant,
            chill: 0.0,
            gdd: 0.0,
            gdd_since_bb: 0.0,
            expansion: 0.0,
            shoot_ext: 1.0,
            cdd: 0.0,
            colouring: 0.0,
            fallen: 0.0,
            fall_days: 0.0,
            frost_damage: 0.0,
        }
    }

    /// Start a model chilling state as if autumn had passed normally, so the
    /// first spring behaves like any other.
    pub fn with_initial_chill(mut self, chill: f32) -> Self {
        self.chill = chill;
        self
    }

    /// Live (photosynthetic) leaf area fraction 0..1.
    pub fn leaf_area(&self) -> f32 {
        match self.phase {
            Phase::Dormant => 0.0,
            _ => self.expansion * (1.0 - self.fallen) * (1.0 - self.frost_damage),
        }
    }

    pub fn step(&mut self, w: &Weather, doy: u32, g: &Genome) -> PhenoEvent {
        let p = &g.phenology;
        if doy == 0 {
            self.gdd = 0.0;
        }
        let forcing = (w.temp_c - p.gdd_base_c).max(0.0);
        self.gdd += forcing;
        self.frost_damage = (self.frost_damage - 0.012).max(0.0);
        let mut ev = PhenoEvent::None;
        match self.phase {
            Phase::Dormant => {
                if w.temp_c >= p.chill_temp_min_c && w.temp_c <= p.chill_temp_max_c {
                    self.chill += 1.0;
                }
                let lack = 1.0 - (self.chill / p.chilling_requirement_days).min(1.0);
                let req = p.gdd_budburst * (1.0 + 1.2 * lack);
                if ((40..200).contains(&doy) && self.gdd >= req) || doy == 170 {
                    self.phase = Phase::BudBurst;
                    self.gdd_since_bb = 0.0;
                    self.expansion = 0.0;
                    self.shoot_ext = 0.0;
                    self.colouring = 0.0;
                    self.fallen = 0.0;
                    self.cdd = 0.0;
                    self.chill = 0.0;
                    ev = PhenoEvent::Budburst;
                }
            }
            Phase::BudBurst | Phase::LeafExpansion => {
                self.gdd_since_bb += forcing;
                self.shoot_ext = smoothstep(self.gdd_since_bb / (p.leaf_expansion_gdd * 0.9));
                self.expansion = smoothstep((self.gdd_since_bb - 10.0) / p.leaf_expansion_gdd);
                // Daily minimum approximated as mean - 5 degC (spring diurnal range).
                let tmin = w.temp_c - 5.0;
                if tmin < g.tolerances.frost_c && self.expansion > 0.02 {
                    let dmg = (0.3 + (g.tolerances.frost_c - tmin) * 0.15).min(0.8);
                    self.frost_damage = self.frost_damage.max(dmg);
                }
                self.phase = if self.expansion < 0.12 { Phase::BudBurst } else { Phase::LeafExpansion };
                if self.expansion >= 1.0 {
                    self.expansion = 1.0;
                    self.shoot_ext = 1.0;
                    self.phase = Phase::Mature;
                    ev = PhenoEvent::ExpansionComplete;
                }
            }
            Phase::Mature => {
                let gate = doy >= 172 && w.photoperiod_h < p.senescence_photoperiod_h;
                if gate {
                    self.cdd += (p.senescence_temp_base_c - w.temp_c).max(0.0);
                }
                if (g.leaf.deciduous && self.cdd > 0.03 * p.senescence_cdd) || doy >= 300 {
                    self.phase = Phase::Colouring;
                    ev = PhenoEvent::SenescenceStart;
                } else if !g.leaf.deciduous && doy == 280 {
                    // Evergreens: close the ring at a fixed date, keep leaves.
                    ev = PhenoEvent::SenescenceStart;
                }
            }
            Phase::Colouring => {
                self.cdd += (p.senescence_temp_base_c - w.temp_c).max(0.0);
                self.colouring = (self.cdd / p.senescence_cdd).min(1.0);
                if self.colouring >= 1.0 || doy >= 335 {
                    self.colouring = 1.0;
                    self.phase = Phase::LeafFall;
                    self.fall_days = 0.0;
                }
            }
            Phase::LeafFall => {
                // Frost and wind speed up abscission.
                self.fall_days += 1.0 + (-w.temp_c).max(0.0) * 0.4;
                self.fallen = (self.fall_days / p.leaf_fall_days).min(1.0);
                if self.fallen >= 1.0 || doy == 364 {
                    self.fallen = 1.0;
                    self.phase = Phase::Dormant;
                    self.expansion = 0.0;
                    ev = PhenoEvent::LeafFallComplete;
                }
            }
        }
        ev
    }
}

/// Leaf RGB (linear-ish, 0..1) from pigment contents and the leaf state.
///
/// Stylised additive pigment mixing: chlorophyll masks carotenoids until it
/// breaks down in autumn; anthocyanins peak mid-senescence; oxidised tannins
/// give the final brown. Young leaves are pale yellow-green with a reddish tint.
pub fn leaf_colour(p: &Pigments, expansion: f32, colouring: f32) -> [f32; 3] {
    const GREEN: [f32; 3] = [0.16, 0.33, 0.08];
    const YOUNG: [f32; 3] = [0.55, 0.62, 0.22];
    const YELLOW: [f32; 3] = [0.80, 0.62, 0.14];
    const RED: [f32; 3] = [0.58, 0.12, 0.07];
    const BROWN: [f32; 3] = [0.40, 0.26, 0.12];
    let c = colouring.clamp(0.0, 1.0);
    let e = expansion.clamp(0.0, 1.0);
    let chl = p.chlorophyll * (1.0 - c).powi(2) * (0.3 + 0.7 * e);
    let young = (1.0 - e) * 0.8;
    let car = p.carotenoids * (0.08 + 1.4 * c * (1.0 - c) + 0.3 * c);
    let anth = p.anthocyanins * (3.0 * c * (1.0 - c) + 0.6 * (1.0 - e));
    let brown = p.tannins * c.powf(2.5) * 1.6;
    let w = [chl * 2.0, young, car, anth * 1.5, brown];
    let cols = [GREEN, YOUNG, YELLOW, RED, BROWN];
    let total: f32 = w.iter().sum::<f32>().max(1e-6);
    let mut out = [0.0; 3];
    for (wi, col) in w.iter().zip(cols.iter()) {
        for k in 0..3 {
            out[k] += wi * col[k] / total;
        }
    }
    out
}

/// Colour of dead marcescent leaves.
pub(crate) const MARCESCENT_BROWN: [f32; 3] = [0.43, 0.29, 0.15];
