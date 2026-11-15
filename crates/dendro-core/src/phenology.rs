//! Leaf phenology: dormancy, budburst, leaf expansion, summer, autumn
//! colouring and leaf fall.
//!
//! * Budburst: sequential/parallel chilling-forcing model (Cannell & Smith
//!   1983 style): growing degree days from Jan 1, with the forcing
//!   requirement raised when chilling is incomplete.
//! * Senescence: photoperiod-gated cold-degree-day sum (after Delpierre et al.
//!   2009, simplified).
//! * Late frost on expanding leaves reduces leaf area, recovered by re-flushing.
//! * Flowering and fruiting ([`Reproduction`]): bloom by its own forcing
//!   sum (same chilling penalty as budburst), so a species may flower
//!   before, with or after leaf-out; late frost on open flowers reduces the
//!   year's fruit set; fruits develop, ripen and are gone after a few weeks.

use crate::climate::Weather;
use crate::genome::{Genome, Pigments, ReproductionTraits};

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
    /// Forcing (degC.day) since Jan 1.
    pub gdd: f32,
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

    /// Relative chilling deficit 0..1 (0 = requirement met). Meaningful
    /// while dormant; chilling restarts at budburst.
    pub fn chill_lack(&self, g: &Genome) -> f32 {
        1.0 - (self.chill / g.phenology.chilling_requirement_days).min(1.0)
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

/// Stage of the yearly flowering / fruiting cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReproStage {
    /// Before bloom (or a juvenile tree that does not flower this year).
    Waiting,
    /// Flowers opening, open, petals falling.
    Blooming,
    /// Fruits developing, ripe, then taken by birds / dropped.
    Fruiting,
    /// Cycle over for this year.
    Done,
}

/// Yearly flowering and fruiting state (stylised).
///
/// Bloom onset follows the forcing sum since Jan 1 (`bloom_gdd`, raised by
/// the same chilling deficit as budburst, which is captured before the
/// leaves break so that a species flowering after leaf-out is not
/// penalised). Bloom then progresses with forcing over `bloom_duration_gdd`
/// (opening, full bloom, petal fall). A daily minimum below
/// `flower_frost_c` while flowers are open kills a share of them (late
/// frost; flower buds and open flowers of Prunus are killed around -2 degC,
/// e.g. Rodrigo 2000), which reduces that year's fruit set. Fruits swell
/// and ripen over `fruit_ripening_days`, then disappear over
/// `fruit_persistence_days`.
#[derive(Clone, Debug)]
pub(crate) struct Reproduction {
    pub stage: ReproStage,
    /// The tree flowers this year (past its juvenile phase).
    pub mature: bool,
    /// Chilling deficit captured while dormant.
    lack: f32,
    gdd_onset: f32,
    /// Bloom progress 0..1 (1 = petal fall complete).
    pub bloom: f32,
    /// Days since the end of petal fall.
    pub fruit_days: f32,
    /// Fraction of this year's flowers killed by frost 0..1.
    pub frost_loss: f32,
    /// Age (years) of first flowering of this individual.
    first_age: f32,
}

impl Reproduction {
    pub fn new(first_age: f32) -> Self {
        Reproduction {
            stage: ReproStage::Waiting,
            mature: false,
            lack: 0.0,
            gdd_onset: 0.0,
            bloom: 0.0,
            fruit_days: 0.0,
            frost_loss: 0.0,
            first_age,
        }
    }

    /// Daily update, after the leaf phenology step of the same day.
    pub fn step(&mut self, w: &Weather, doy: u32, age: f32, phen: &Phenology, g: &Genome) {
        let r = &g.reproduction;
        if doy == 0 {
            self.stage = ReproStage::Waiting;
            self.mature = age + 0.5 >= self.first_age;
            self.bloom = 0.0;
            self.fruit_days = 0.0;
            self.frost_loss = 0.0;
        }
        if phen.phase == Phase::Dormant {
            self.lack = phen.chill_lack(g);
        }
        match self.stage {
            ReproStage::Waiting => {
                let req = r.bloom_gdd * (1.0 + 1.2 * self.lack);
                if self.mature && (((40..200).contains(&doy) && phen.gdd >= req) || doy == 170) {
                    self.stage = ReproStage::Blooming;
                    self.gdd_onset = phen.gdd;
                    self.bloom = 0.0;
                }
            }
            ReproStage::Blooming => {
                self.bloom = ((phen.gdd - self.gdd_onset) / r.bloom_duration_gdd.max(1.0)).min(1.0);
                // Open flowers (before petal fall) are killed by late frost.
                let tmin = w.temp_c - 5.0;
                if self.bloom > 0.05 && self.bloom < 0.8 && tmin < r.flower_frost_c {
                    let dmg = (0.4 + 0.25 * (r.flower_frost_c - tmin)).min(0.95);
                    self.frost_loss = self.frost_loss.max(dmg);
                }
                if self.bloom >= 1.0 {
                    self.stage = ReproStage::Fruiting;
                    self.fruit_days = 0.0;
                }
            }
            ReproStage::Fruiting => {
                self.fruit_days += 1.0;
                if self.fruit_days > r.fruit_ripening_days + r.fruit_persistence_days {
                    self.stage = ReproStage::Done;
                }
            }
            ReproStage::Done => {}
        }
    }

    /// Open-flower density 0..1 (opening, full bloom, petal fall), before
    /// the display weight.
    pub fn flower_amount(&self) -> f32 {
        if self.stage != ReproStage::Blooming {
            return 0.0;
        }
        let b = self.bloom;
        smoothstep(b / 0.3) * (1.0 - smoothstep((b - 0.55) / 0.45))
    }

    /// Fruit presence 0..1 (swelling, then removal), before frost loss and
    /// the display weight.
    pub fn fruit_amount(&self, r: &ReproductionTraits) -> f32 {
        if self.stage != ReproStage::Fruiting {
            return 0.0;
        }
        self.fruit_swell() * self.fruit_remaining(r)
    }

    /// Fruit size 0..1 (young fruits swell over ~12 days after petal fall).
    pub fn fruit_swell(&self) -> f32 {
        smoothstep(self.fruit_days / 12.0)
    }

    /// Share 0..1 of the fruits still on the tree (taken once ripe).
    pub fn fruit_remaining(&self, r: &ReproductionTraits) -> f32 {
        1.0 - smoothstep((self.fruit_days - r.fruit_ripening_days) / r.fruit_persistence_days.max(1.0))
    }

    /// Fruit ripeness 0..1 (colour turns in the last ~45 % of ripening).
    pub fn ripeness(&self, r: &ReproductionTraits) -> f32 {
        if self.stage != ReproStage::Fruiting {
            return 0.0;
        }
        smoothstep((self.fruit_days / r.fruit_ripening_days.max(1.0) - 0.55) / 0.45)
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
    const BROWN: [f32; 3] = [0.40, 0.26, 0.12];
    let c = colouring.clamp(0.0, 1.0);
    let e = expansion.clamp(0.0, 1.0);
    let chl = p.chlorophyll * (1.0 - c).powi(2) * (0.3 + 0.7 * e);
    let young = (1.0 - e) * 0.8;
    let car = p.carotenoids * (0.08 + 1.4 * c * (1.0 - c) + 0.3 * c);
    // Senescence anthocyanins peak at c = 0.5; past the peak a share
    // `anthocyanin_persistence` of it is kept until the leaf falls.
    let mut sen = 3.0 * c * (1.0 - c);
    if p.anthocyanin_persistence > 0.0 && c > 0.5 {
        sen += p.anthocyanin_persistence.min(1.0) * (0.75 - sen);
    }
    let anth = p.anthocyanins * (sen + 0.6 * (1.0 - e));
    let brown = p.tannins * c.powf(2.5) * 1.6;
    let w = [chl * 2.0, young, car, anth * 1.5, brown];
    let cols = [GREEN, YOUNG, YELLOW, p.anthocyanin_color, BROWN];
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
