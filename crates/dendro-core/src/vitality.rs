//! Whole-tree carbon status, ageing and death (no programmed lifespan).
//!
//! Why: before this module the annual wood budget was `coef x F` (foliage
//! carried, light- and hydraulics-weighted, see `growth.rs`) with no cost
//! that grows with the living structure. An open-grown crown keeps
//! widening for centuries, so the basal-area increment stayed constant (or
//! rose) and the stem kept thickening like `sqrt(age)`; nothing could kill
//! an unpruned tree. Here the longevity *emerges* from three mechanisms;
//! there is no death date and `PruningTraits::typical_lifespan_years` is
//! not read.
//!
//! 1. **Carbon balance.** Each season the foliage pays the maintenance of
//!    the living wood that supplies it. Under the pipe model (Shinozaki et
//!    al. 1964) that sapwood scales with `sum(leaf area x collar distance)`
//!    (`ShootSystem::sap_demand`), whereas production scales with the
//!    light- and hydraulics-weighted foliage `F` (hydraulic limitation of
//!    far foliage: Ryan & Yoder 1997, BioScience 47:235-242; Koch et al.
//!    2004, Nature 428:851-854, cited). The cost / production ratio
//!    `rho = maintenance_per_m x D / F` therefore rises steeply once the
//!    crown nears the species' hydraulic reach, and stays small in a young
//!    tree. The yearly net production is `p - rho` (`p` = the season's
//!    productivity, 1 in an average year). It is **not** subtracted from
//!    the wood budget: `WoodTraits::growth_coefficient` is calibrated on
//!    measured diameters, i.e. already net of what real trees of those ages
//!    respire, and subtracting a cost again would shift every calibrated
//!    diameter. The balance acts through the reserves (2.), so it is
//!    neutral as long as they stay full. **Model choice, not an
//!    established cause**: the "respiration hypothesis" of age-related decline is
//!    contested (Ryan, Binkley & Fownes 1997, Adv Ecol Res 27:213-262, cited;
//!    Ryan, Phillips & Bond 2006, Plant Cell Environ 29:367-381, summary of
//!    the abstract seen on the USDA Treesearch page: hydraulic limitation is "common, but not universal" and does not
//!    alone explain the decline of wood production). See docs/sources.md,
//!    « Écarts connus ».
//! 2. **Reserves and retrenchment.** Non-structural reserves (0..1, full
//!    for the tree's size) refill while the net production exceeds what
//!    storage needs and run down otherwise (drought years too, through the
//!    productivity index). Low reserves cut growth (same capacity curve as
//!    after a heavy pruning, `pruning.rs`) and kill the branches whose own
//!    balance is negative (`ShootSystem::starve_dieback`): the outer crown
//!    dies back, the tree "grows downwards" and may recover with a smaller
//!    crown, reiterating epicormic shoots (veteran phase: Fay 2002;
//!    Lonsdale (ed.) 2013; Read 2000, cited).
//! 3. **Growth-dependent mortality.** Every year the tree dies with a
//!    probability that rises as its vigour falls: a logistic function of a
//!    vigour index (functional form after the growth-based mortality models
//!    of Bigler & Bugmann 2004, Ecol Appl 14:902-914, excerpts of the
//!    abstract seen through a search engine, paper not read: their
//!    best Picea abies models use relative growth and a short-term growth
//!    trend; Hülsmann et al. 2017, Can J For Res 47:890-900, cited). The
//!    index here is the lowest of three, all relative to the tree itself:
//!    the 5-year mean collar basal-area increment over its own best 5-year
//!    mean; the reserve level; the carbon margin `1 - load` (`load` = the
//!    ~5-year running mean of maintenance / production), the analogue of
//!    a growth-efficiency vigour index (stem growth per unit of leaf area,
//!    Waring, Thies & Muscato 1980, Forest Sci 26:112-117, cited). **The coefficients are estimated** (none could be read), chosen
//!    for a low background rate at full vigour. A tree whose reserves stay
//!    exhausted for `STARVE_YEARS` dies of carbon starvation. Draws come
//!    from the seed (`S_MORTALITY`), so a given seed always dies the same
//!    year.
//!
//! A dead trunk is not removed: it stands as a snag ("chandelle"), loses its
//! twigs within a few years and its limbs over decades (the existing dead
//! wood rule, `WoodTraits::snag_years_per_m`), then the stem collapses (see
//! [`collapse_years`]). Species that resprout from the stump or the roots
//! (`PruningTraits::stump_sprouting`, `root_suckering`) may keep the collar
//! alive with latent buds: the stem dies, the individual goes on.
//!
//! Cost: O(n) once a year (one pass for the ratio, one for the dieback
//! when reserves are low).

use crate::fmath::DMath;
use crate::growth::NONE;
use crate::rng::{self, S_MORTALITY};
use crate::sim::Simulation;

/// Net production share (of an average year's production) that keeps the
/// reserves level; above it they refill, below it they run down.
/// Estimated (stylised; no source gives such a threshold).
const STORAGE_NEED: f32 = 0.2;
/// Reserve change per unit of net production above / below `STORAGE_NEED`
/// (per year). Estimated: an exhausted crown empties full reserves in a
/// few years to a couple of decades depending on the deficit.
const RESERVE_RATE: f32 = 0.25;
/// Reserve level below which branches with a negative balance die.
const DIEBACK_RESERVE: f32 = 0.6;
/// Yearly death probability of such a branch at zero reserves (estimated).
const DIEBACK_MAX: f32 = 0.4;
/// Years with exhausted reserves after which the tree dies (as the
/// starving pruned tree of `pruning.rs`, 4 years; calibration).
const STARVE_YEARS: u32 = 4;
/// Logistic mortality `p = 1 / (1 + exp(-(A + B v)))`, `v` = vigour 0..1
/// (vigour above `MORT_FULL` counts as full: year-to-year climate swings
/// of the growth record are not decline). Estimated: p ~ 0.02 %/year at
/// full vigour, ~0.8 % at v = 0.5, ~5 % at v = 0.2, 25 % at v = 0.
/// Functional form after Bigler & Bugmann 2004.
const MORT_A: f32 = -1.1;
const MORT_B: f32 = -7.4;
const MORT_FULL: f32 = 0.7;
/// Weight of the season in the running mean of the cost / production ratio.
const LOAD_SMOOTH: f32 = 0.2;
/// Carbon margin (1 - maintenance / production) above which the margin
/// does not lower the vigour index (estimated).
const MARGIN_FULL: f32 = 0.5;
/// No mortality draw before this age (seedling mortality is not modelled).
const MORT_MIN_AGE: f32 = 10.0;
/// Ground contact speeds the decay of the dead stem base relative to
/// aerial dead limbs (`snag_years_per_m`; European durability standards
/// separate above-ground and ground-contact use classes). Estimated.
const STEM_BASE_DECAY: f32 = 0.25;
/// Latent buds released at the stump of a resprouting dead tree (as a
/// coppice cut, `pruning.rs`: 16 x potential).
const STUMP_BUDS: f32 = 16.0;

/// Why the trunk died.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DeathCause {
    #[default]
    None,
    /// Low vigour: growth-dependent mortality draw.
    Decline,
    /// Reserves exhausted for several years.
    Starvation,
}

impl DeathCause {
    pub fn code(self) -> u8 {
        self as u8
    }
    pub fn label_fr(self) -> &'static str {
        match self {
            DeathCause::None => "",
            DeathCause::Decline => "dépérissement",
            DeathCause::Starvation => "réserves épuisées",
        }
    }
}

/// Carbon status and fate of the tree (see the module docs).
#[derive(Clone, Debug)]
pub(crate) struct Vitality {
    /// Non-structural reserves, 0..1 (1 = full for the tree's size).
    pub reserve: f32,
    /// Maintenance / production ratio of the last light pass.
    pub cost_ratio: f32,
    /// Net production of the last season (units of an average year).
    pub net: f32,
    /// Running mean (exponential, ~5 years) of the maintenance / production
    /// ratio of the crown (average-year production).
    pub load: f32,
    /// Consecutive seasons with exhausted reserves.
    starve_years: u32,
    /// Collar basal-area increments (m2) of the last 5 seasons.
    bai: [f32; 5],
    bai_n: u32,
    /// Best 5-year mean collar BAI so far (m2/year).
    bai_peak: f32,
    /// Vigour index 0..1 and death probability of the last season.
    pub vigour: f32,
    pub hazard: f32,
    /// Branches killed by starvation dieback last season.
    pub dieback: u32,
    /// Year the (first) trunk died, `NONE` while it lives.
    pub death_year: u32,
    pub cause: DeathCause,
    /// Year the dead stem collapses (`NONE` = not dead).
    pub collapse_year: u32,
    pub collapsed: bool,
    /// The stump / roots resprouted after the trunk died.
    pub resprouted: bool,
}

impl Default for Vitality {
    fn default() -> Self {
        Vitality {
            reserve: 1.0,
            cost_ratio: 0.0,
            net: 1.0,
            load: 0.0,
            starve_years: 0,
            bai: [0.0; 5],
            bai_n: 0,
            bai_peak: 0.0,
            vigour: 1.0,
            hazard: 0.0,
            dieback: 0,
            death_year: NONE,
            cause: DeathCause::None,
            collapse_year: NONE,
            collapsed: false,
            resprouted: false,
        }
    }
}

impl Vitality {
    /// Growth capacity from the reserves: a tree whose reserves are not
    /// full refills them before it grows (storage as a sink competing with
    /// growth: Sala, Woodruff & Meinzer 2012, Tree Physiol 32:764-775,
    /// cited). Smooth from 0.25 at empty to 1 at 80 % (estimated shape).
    pub fn capacity(&self) -> f32 {
        let x = (self.reserve / 0.8).clamp(0.0, 1.0);
        0.25 + 0.75 * x * x * (3.0 - 2.0 * x)
    }
}

/// Key of the species in the mortality stream, so that two species grown
/// with the same seed (same weather) do not share their death draws.
fn species_key(g: &crate::genome::Genome) -> u64 {
    g.species.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3)) << 1
}

/// Years a dead stem of collar radius `r` (m) stands before it collapses:
/// the dead-wood rule of the limbs (`2 + snag_years_per_m x r`, growth.rs)
/// with the base decaying faster in contact with the soil
/// (`STEM_BASE_DECAY`). E.g. oak (500 y/m) of 1 m DBH: ~64 years; Norway
/// spruce (250 y/m) of 80 cm: ~27 years; birch (40 y/m): ~5 years.
/// Stylised and estimated: no snag-fall data was read.
pub fn collapse_years(r: f32, snag_years_per_m: f32) -> u32 {
    (2.0 + snag_years_per_m * r * STEM_BASE_DECAY).round() as u32
}

impl Simulation {
    /// The carbon balance is modelled (`LongevityTraits::maintenance_per_m > 0`).
    pub(crate) fn ageing(&self) -> bool {
        self.genome.longevity.maintenance_per_m > 0.0
    }

    /// Update the cost / production ratio after a light + foliage pass.
    pub(crate) fn update_cost_ratio(&mut self) {
        let c = self.genome.longevity.maintenance_per_m;
        let f = self.shoots.nodes[0].foliage;
        self.vit.cost_ratio = if c > 0.0 && f > 0.0 { (c * self.shoots.sap_demand / f).min(10.0) } else { 0.0 };
    }

    /// End-of-season update (called once the ring is laid down):
    /// reserves, starvation dieback, mortality, the dead stem.
    pub(crate) fn vitality_yearly(&mut self, year: u32, p_year: f32, collar_bai: f32) {
        // Dead stem (any cause, carbon balance modelled or not): it stands
        // until it collapses.
        if self.vit.collapse_year != NONE && !self.vit.collapsed && year >= self.vit.collapse_year {
            self.vit.collapsed = true;
            self.shoots.collapse_dead_stem();
            self.prune.light_dirty = true;
            self.wood_version += 1;
            self.leaf_version += 1;
            self.flower_version += 1;
        }
        if !self.ageing() || !self.is_alive() {
            return;
        }
        let v = &mut self.vit;
        v.net = p_year - v.cost_ratio;
        // Structural load only (crown geometry): weather acts through the
        // reserves (`net`), not here.
        v.load += LOAD_SMOOTH * (v.cost_ratio.min(2.0) - v.load);
        v.reserve = (v.reserve + RESERVE_RATE * (v.net - STORAGE_NEED)).clamp(0.0, 1.0);
        v.starve_years = if v.reserve <= 0.0 { v.starve_years + 1 } else { 0 };
        v.bai[(v.bai_n % 5) as usize] = collar_bai.max(0.0);
        v.bai_n += 1;
        let k = v.bai_n.min(5);
        let mean = v.bai.iter().take(k as usize).sum::<f32>() / k as f32;
        if v.bai_n >= 5 {
            v.bai_peak = v.bai_peak.max(mean);
        }
        let rel = if v.bai_peak > 0.0 && v.bai_n >= 5 { (mean / v.bai_peak).clamp(0.0, 1.0) } else { 1.0 };
        // Carbon margin left after maintenance (1 - load), full above
        // `MARGIN_FULL`.
        let margin = ((1.0 - v.load) / MARGIN_FULL).clamp(0.0, 1.0);
        v.vigour = rel.min((v.reserve / DIEBACK_RESERVE).clamp(0.0, 1.0)).min(margin);
        let x = (v.vigour / MORT_FULL).min(1.0);
        v.hazard = 1.0 / (1.0 + (-(MORT_A + MORT_B * x)).dexp());
        // Starvation dieback of the branches that cost more than they return.
        v.dieback = 0;
        if v.reserve < DIEBACK_RESERVE {
            let p = DIEBACK_MAX * (1.0 - v.reserve / DIEBACK_RESERVE);
            let c = self.genome.longevity.maintenance_per_m;
            let thr = self.vit.cost_ratio.min(1.0);
            let n = self.shoots.starve_dieback(&self.genome, c, thr, p, year, self.seed);
            self.vit.dieback = n as u32;
            if n > 0 {
                self.prune.light_dirty = true;
                self.wood_version += 1;
                self.leaf_version += 1;
                self.flower_version += 1;
            }
        }
        // Death of the trunk.
        let age = self.age_years();
        let starved = self.vit.starve_years >= STARVE_YEARS;
        let drawn = age >= MORT_MIN_AGE && rng::unit(rng::hash(self.seed, S_MORTALITY, species_key(&self.genome), year as u64)) < self.vit.hazard;
        if starved || drawn {
            self.trunk_dies(year, if starved { DeathCause::Starvation } else { DeathCause::Decline });
        }
    }

    /// Record the death of a tree killed by pruning starvation
    /// (`pruning.rs`, whose own state already turned every node into dead
    /// wood): age and cause, and the collapse of the dead stem. No
    /// resprouting is drawn (the pruning model decides that), and it applies
    /// whether or not the carbon balance is modelled.
    pub(crate) fn record_starvation_death(&mut self, year: u32) {
        let r = self.shoots.nodes[0].radius();
        let v = &mut self.vit;
        if v.death_year == NONE {
            v.death_year = year;
            v.cause = DeathCause::Starvation;
        }
        if v.collapse_year == NONE {
            v.collapse_year = year + collapse_years(r, self.genome.wood.snag_years_per_m).max(1);
            v.collapsed = false;
        }
        v.reserve = 0.0;
        v.vigour = 0.0;
    }

    /// The stem dies: it becomes a standing snag; the stump may resprout.
    fn trunk_dies(&mut self, year: u32, cause: DeathCause) {
        let first = self.vit.death_year == NONE;
        let tr = &self.genome.pruning;
        let ability = tr.stump_sprouting.max(tr.root_suckering).clamp(0.0, 1.0);
        let sprouts = rng::unit(rng::hash(self.seed, S_MORTALITY, species_key(&self.genome) ^ 1, year as u64)) < ability;
        let r = self.shoots.nodes[0].radius();
        self.shoots.kill_all(year, sprouts);
        if sprouts {
            // The root system and its reserves (Loescher et al. 1990) feed the
            // released latent buds of the collar, as after coppicing.
            let n = (ability * STUMP_BUDS).round().clamp(1.0, 20.0) as u8;
            let boost = (self.genome.pruning.sprout_vigour * 0.5).min(3.0);
            let st = &mut self.shoots.nodes[0];
            st.lat_buds = n;
            st.bud_year = year;
            st.sprout = boost;
            st.wounded = true;
        }
        let v = &mut self.vit;
        if first {
            v.death_year = year;
            v.cause = cause;
        }
        v.collapse_year = year + collapse_years(r, self.genome.wood.snag_years_per_m).max(1);
        v.collapsed = false;
        v.resprouted = sprouts;
        // A resprouting stool starts a new stem: half-full reserves, a new
        // growth record.
        v.reserve = if sprouts { 0.5 } else { 0.0 };
        v.starve_years = 0;
        v.bai = [0.0; 5];
        v.bai_n = 0;
        v.bai_peak = 0.0;
        v.vigour = if sprouts { 0.5 } else { 0.0 };
        self.prune.light_dirty = true;
        self.wood_version += 1;
        self.leaf_version += 1;
        self.flower_version += 1;
        self.root_version += 1;
    }
}

/// Read-only snapshot of the carbon status and fate of the tree
/// ([`Simulation::vitality`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VitalityInfo {
    /// The carbon balance is modelled for this species.
    pub modelled: bool,
    /// Reserves 0..1 (1 = full).
    pub reserve: f32,
    /// Maintenance / production ratio of the current crown.
    pub cost_ratio: f32,
    /// Net production of the last season (units of an average year).
    pub net_production: f32,
    /// Vigour index 0..1 and yearly death probability of the last season.
    pub vigour: f32,
    pub death_probability: f32,
    /// Branches killed by carbon-starvation dieback last season.
    pub dieback_branches: u32,
    /// Age (years) at which the first trunk died.
    pub death_age_years: Option<u32>,
    pub cause: DeathCause,
    /// A dead stem is still standing (snag).
    pub standing_dead: bool,
    /// Years left before the standing dead stem collapses.
    pub collapse_in_years: Option<u32>,
    /// The dead stem has collapsed.
    pub collapsed: bool,
    /// The stump or the roots resprouted after the trunk died.
    pub resprouted: bool,
}

impl Simulation {
    /// Carbon status, vigour and fate of the tree (see [`crate::vitality`]).
    pub fn vitality(&self) -> VitalityInfo {
        let v = &self.vit;
        let dead = v.death_year != NONE;
        let year = self.year();
        let standing = v.collapse_year != NONE && !v.collapsed;
        VitalityInfo {
            modelled: self.ageing(),
            reserve: v.reserve,
            cost_ratio: v.cost_ratio,
            net_production: v.net,
            vigour: v.vigour,
            death_probability: v.hazard,
            dieback_branches: v.dieback,
            // The trunk dies at the end of a season, after laying its ring:
            // dying in calendar year y it has lived y + 1 seasons (rings).
            death_age_years: dead.then(|| v.death_year + 1),
            cause: v.cause,
            standing_dead: standing,
            collapse_in_years: standing.then(|| v.collapse_year.saturating_sub(year)),
            collapsed: v.collapsed,
            resprouted: v.resprouted,
        }
    }

    /// The (first) trunk is alive: false once it has died, even when the
    /// stump has resprouted (see [`Simulation::is_alive`] for the organism).
    pub fn trunk_alive(&self) -> bool {
        self.vit.death_year == NONE
    }
}
