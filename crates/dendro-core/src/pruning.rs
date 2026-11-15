//! Pruning: cuts, wounds and the tree's response, on a replayable timeline.
//!
//! # Timeline
//! Every intervention (a cut, a forced drought toggled on or off) is recorded
//! as a [`LogEvent`] `(day, action)` in the simulation. Cuts target a shoot
//! node by its stable id (ids are deterministic for a given genome, seed and
//! log), so a fresh simulation given the same log ([`Simulation::set_prune_log`])
//! and advanced to the same day is bit-identical to the live one. Events
//! are applied at the start of their day, before that day is simulated.
//! A live cut made while later events exist (after rewinding) discards them.
//!
//! # Biology (stylised; numeric values are calibrations unless cited)
//! * **The cut** removes everything distal to it (wood, leaves, buds,
//!   flowers, fruit). Its type follows from where it lands
//!   ([`CutKind`]): at the branch collar (proper "natural target" cut),
//!   flush with the parent (collar removed: wider wound, weaker
//!   compartmentalisation), leaving a stub, shortening an axis (heading,
//!   reduction to a lateral), topping the stem, pollarding, coppicing.
//!   Shigo 1989, "Tree Pruning: A Worldwide Photo Guide"; Eisner, Gilman &
//!   Grabosky 2002, "Branch morphology impacts compartmentalization of
//!   pruning wounds", J Arboric 28(2):99-105 (a collar gives stronger
//!   compartmentalisation); Neely 1988, "Closure of branch pruning wounds
//!   with conventional and 'Shigo' cuts", J Arboric 14(11):261-264 (flush
//!   cuts expose much more wood than collar cuts).
//! * **Immediate consequences**: lost leaf area (less light intercepted,
//!   so less shoot vigour and radial growth through the existing light and
//!   Pressler models), reserves lost with the removed wood, a root system
//!   now too large for the crown (`root_cap` vs leaf area), more light inside
//!   the crown (light pass recomputed).
//! * **Response**: latent (epicormic) buds below the cut and on the host
//!   are released (`epicormic_potential`: strong in Quercus, weak in
//!   Prunus; Meier, Saunders & Michler 2012, "Epicormic buds in trees: a
//!   review of bud establishment, development and dormancy release", Tree
//!   Physiol 32:565-584; Fontaine et al. 2001, Ann For Sci 58:583-592 on
//!   the epicormic potential of Quercus petraea). Their vigour is fed by
//!   stored reserves (Loescher, McCamant & Keller 1990, "Carbohydrate
//!   reserves, translocation, and storage in woody plant roots",
//!   HortScience 25:274-281), so they grow as long, upright water sprouts. A
//!   compensatory growth multiplier follows the root:shoot imbalance. Fine
//!   roots die back towards the new balance (`root_balance_rate`); while
//!   the crown is too small to refill the reserves they run down, and a
//!   starving tree declines (growth capacity, crown dieback, death).
//!   Dormant-season cuts are the most invigorating, cuts during the flush
//!   (reserves just spent) and in summer less so: the dormant-pruning
//!   effect summarised by Mika 1986, "Physiological responses of fruit
//!   trees to pruning", Hort Rev 8:337-378.
//! * **Wounds**: woundwood rolls in from the edges at a rate proportional
//!   to the host's radial growth (the link between growth vigour and wound
//!   healing is classic, e.g. Neely 1970, J Amer Soc Hort Sci 95:536-540,
//!   known to us from secondary citations; wide wounds take longer to
//!   close, Neely 1988), slower for winter wounds (Dujesiefken, Liese,
//!   Shortle & Minocha 2005, "Response of beech and oaks to wounds made at
//!   different times of the year", Eur J For Res 124:113-117). While open,
//!   a wound can be colonised by decay fungi; decay is compartmentalised
//!   (CODIT, Shigo & Marx 1977; Shigo 1984, "Compartmentalization: a
//!   conceptual framework for understanding how trees grow and defend
//!   themselves", Annu Rev Phytopathol 22:189-214): it never spreads into
//!   the wood formed after wounding (wall 4), so a hollow stays inside the
//!   radius the host had when cut, while its column spreads along the axis
//!   (weak wall 1) at a rate set by `compartmentalization`. The residual
//!   sound shell t/R is a classic failure criterion (Mattheck & Breloer
//!   1994, "The Body Language of Trees", HMSO; failure becomes likely
//!   below t/R ~ 0.3): weakened parts break off.
//! * **Season and pathogens**: Prunus wounds made from September to May
//!   are the main entry of silver leaf (Chondrostereum purpureum, spores
//!   released in damp autumn/winter weather) and of bacterial canker
//!   (Pseudomonas syringae; Kennelly et al. 2007, "Pseudomonas syringae
//!   diseases of fruit trees: progress toward understanding and control",
//!   Plant Dis 91:4-17); hence the advice to prune cherries in summer (RHS
//!   advice "Silver leaf", rhs.org.uk/disease/silver-leaf). Modelled as
//!   traits x season x wound size, drawn deterministically from the seed.
//! * **Formative effects** come out of the same rules: crown raising
//!   (knots occluded inside a knotty core, [`BoleMetrics`]), thinning,
//!   reduction, pollard heads (repeated cuts at the same place build a
//!   knob, Read 2000, "Veteran Trees: A Guide to Good Management", English
//!   Nature), coppice stools.
//!
//! Not modelled: root suckers placed away from the stump (they are
//! approximated by stump sprouts), lammas regrowth after summer pruning,
//! wound dressings, natural self-pruning knots.

use crate::growth::NONE;
use crate::math::Vec3;
use crate::mesh::{self, MeshData, TubeSeg, TubeStyle};
use crate::phenology::Phase;
use crate::rng;
use crate::sim::{draw_radius, Simulation, DAYS_PER_YEAR};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Random stream of the pruning module.
const S_PRUNE: u64 = 40;

/// Species response to pruning (a genome section, `pruning: (...)` in the
/// RON files). Defaults are middling values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PruningTraits {
    /// Bank of viable latent (epicormic) buds on old wood, 0..1: how many
    /// buds a cut releases near the wound.
    pub epicormic_potential: f32,
    /// Vigour boost of released buds (water sprouts), at full reserves and
    /// a severe cut.
    pub sprout_vigour: f32,
    /// Ability of a stump to resprout after coppicing, 0..1.
    pub stump_sprouting: f32,
    /// Root suckering, 0..1 (approximated as extra stump sprouts after
    /// coppicing; not placed away from the stump).
    pub root_suckering: f32,
    /// Compensatory growth: extra shoot vigour per unit of root:shoot
    /// imbalance (root system sized for a larger crown).
    pub compensatory_growth: f32,
    /// Woundwood (callus) growth relative to the host's normal radial growth.
    pub woundwood_ratio: f32,
    /// Strength of compartmentalisation (CODIT walls), 0..1.
    pub compartmentalization: f32,
    /// Yearly probability that an open 10 cm wound is colonised by decay
    /// fungi (scaled by size, cut type and compartmentalisation).
    pub decay_infection_rate: f32,
    /// Axial spread of a decay column (m/year) with no compartmentalisation.
    pub decay_axial_m_per_year: f32,
    /// Radial progress of decay inside the barrier zone (mm/year).
    pub decay_radial_mm_per_year: f32,
    /// Wound pathogen infection risk (silver leaf, bacterial canker) of a
    /// 5 cm wound made from September to May, 0..1.
    pub pathogen_dormant_risk: f32,
    /// Same for a wound made in June-August.
    pub pathogen_summer_risk: f32,
    /// Years from infection to the dieback of the infected branch.
    pub pathogen_dieback_years: f32,
    /// Share of the tree's reserves held in the branches (lost with them).
    pub reserve_in_branches: f32,
    /// Yearly fraction of the root excess (fine roots) that dies back after
    /// the crown was reduced.
    pub root_balance_rate: f32,
    /// Sap bleeding of late-winter cuts, 0..1 (advice only).
    pub sap_bleeding: f32,
    /// Typical lifespan (years) of an open-grown tree, for the life
    /// expectancy estimate.
    pub typical_lifespan_years: f32,
}

impl Default for PruningTraits {
    fn default() -> Self {
        PruningTraits {
            epicormic_potential: 0.4,
            sprout_vigour: 1.5,
            stump_sprouting: 0.5,
            root_suckering: 0.0,
            compensatory_growth: 0.3,
            woundwood_ratio: 2.0,
            compartmentalization: 0.5,
            decay_infection_rate: 0.06,
            decay_axial_m_per_year: 0.08,
            decay_radial_mm_per_year: 6.0,
            pathogen_dormant_risk: 0.05,
            pathogen_summer_risk: 0.01,
            pathogen_dieback_years: 3.0,
            reserve_in_branches: 0.3,
            root_balance_rate: 0.35,
            sap_bleeding: 0.0,
            typical_lifespan_years: 250.0,
        }
    }
}

/// Where a cut lands, which decides how the wound behaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CutKind {
    /// Branch removed just outside its collar (proper cut).
    Collar,
    /// Branch removed flush with the parent (collar damaged).
    Flush,
    /// Branch cut beyond its collar, leaving a bare stub.
    Stub,
    /// Leader removed at a fork, back to a lateral at least a third of its
    /// diameter (drop-crotch / reduction cut).
    Reduction,
    /// Axis shortened between forks (heading cut).
    Heading,
    /// Stem cut in the crown (topping).
    Topping,
    /// Stem cut at 1.2-4 m, or sprouts recut on a pollard head.
    Pollard,
    /// Stem cut near the ground (coppicing).
    Coppice,
    /// Not a cut: a decayed part broke off, or a pathogen killed it.
    Natural,
}

impl CutKind {
    /// Stable integer code (front-end).
    pub fn code(self) -> u8 {
        self as u8
    }

    /// French label.
    pub fn label_fr(self) -> &'static str {
        match self {
            CutKind::Collar => "Coupe au col",
            CutKind::Flush => "Coupe rase",
            CutKind::Stub => "Chicot",
            CutKind::Reduction => "Réduction sur tire-sève",
            CutKind::Heading => "Rabattage",
            CutKind::Topping => "Étêtage",
            CutKind::Pollard => "Têtard",
            CutKind::Coppice => "Recépage",
            CutKind::Natural => "Rupture",
        }
    }

    /// Woundwood closure speed relative to a collar cut. Calibration: the
    /// numbers are not measured; the ranking follows Shigo 1989 (collar cuts
    /// close best, stubs and topping cuts barely) and Neely 1988 (flush cuts
    /// expose a much wider wound).
    fn closure_factor(self) -> f32 {
        match self {
            CutKind::Collar => 1.0,
            CutKind::Reduction => 0.85,
            CutKind::Pollard => 0.7,
            CutKind::Flush => 0.6,
            CutKind::Stub => 1.0, // once the stub is gone (until then: none)
            CutKind::Heading => 0.5,
            CutKind::Natural => 0.4,
            CutKind::Topping => 0.35,
            CutKind::Coppice => 0.3,
        }
    }

    /// Relative decay infection risk (calibration; Eisner et al. 2002 for
    /// collar vs flush, Shigo 1989 for stubs and topping).
    fn infection_factor(self) -> f32 {
        match self {
            CutKind::Collar => 1.0,
            CutKind::Pollard => 1.1,
            CutKind::Reduction => 1.3,
            CutKind::Heading => 1.5,
            CutKind::Coppice => 1.5,
            CutKind::Stub => 1.6,
            CutKind::Flush => 1.8,
            CutKind::Topping => 2.0,
            CutKind::Natural => 2.0,
        }
    }

    fn from_code(c: u8) -> CutKind {
        [
            CutKind::Collar,
            CutKind::Flush,
            CutKind::Stub,
            CutKind::Reduction,
            CutKind::Heading,
            CutKind::Topping,
            CutKind::Pollard,
            CutKind::Coppice,
            CutKind::Natural,
        ][(c as usize).min(8)]
    }
}

/// One atomic cut: node (stable id), position along its visible length
/// (0 = base, 1 = tip), flush with the parent instead of at the collar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CutSpec {
    pub node_id: u32,
    pub t: f32,
    pub flush: bool,
}

/// An intervention.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Cut(CutSpec),
    /// Forced drought switched on / off.
    Drought(bool),
}

/// A logged intervention, applied at the start of day `day` (days since
/// the start of the simulation).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogEvent {
    pub day: u64,
    pub action: Action,
}

/// State of a wound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WoundState {
    /// Fresh, no woundwood yet.
    Open,
    /// Woundwood rolling in.
    Closing,
    /// Occluded (a knot remains inside).
    Closed,
    /// Decay column behind it.
    Decayed,
    /// Stub still standing on it (closure blocked).
    Stub,
    /// The part carrying it is gone (shed, broken, cut further down).
    Gone,
}

impl WoundState {
    pub fn code(self) -> u8 {
        self as u8
    }
    pub fn label_fr(self) -> &'static str {
        match self {
            WoundState::Open => "ouverte",
            WoundState::Closing => "en cicatrisation",
            WoundState::Closed => "refermée",
            WoundState::Decayed => "cariée",
            WoundState::Stub => "chicot",
            WoundState::Gone => "disparue",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Wound {
    id: u32,
    kind: CutKind,
    /// Host node (stable id); for a stub, the stub itself until it falls.
    host: u32,
    /// Parent of a stub (the wound moves there once the stub is gone).
    stub_parent: u32,
    /// On the side of the host at its end (branch removed) or the host's end
    /// face (axis cut).
    side: bool,
    dir: Vec3,
    /// Wound radius (m).
    radius: f32,
    /// Radius of the wood present when cut, that decay may occupy (m).
    core_r: f32,
    /// Host radius when cut (m): the knotty core around a knot.
    host_r0: f32,
    host_r_prev: f32,
    /// Woundwood width rolled in (m).
    callus: f32,
    close_k: f32,
    closed_year: u32,
    r_at_close: f32,
    infected: bool,
    decay_r: f32,
    decay_len: f32,
    /// Pathogen progress 0..1 (< 0: not infected).
    pathogen: f32,
    pathogen_done: bool,
    year: u32,
    recuts: u16,
    last_recut: u64,
    last_recut_year: u32,
    gone: bool,
    y: f32,
}

impl Wound {
    fn state(&self) -> WoundState {
        if self.gone {
            WoundState::Gone
        } else if self.stub_parent != NONE {
            WoundState::Stub
        } else if self.infected && self.decay_r > 0.15 * self.core_r.max(1e-4) {
            WoundState::Decayed
        } else if self.closed_year != NONE {
            WoundState::Closed
        } else if self.callus > 0.1 * self.radius {
            WoundState::Closing
        } else {
            WoundState::Open
        }
    }

    fn closure(&self) -> f32 {
        if self.closed_year != NONE {
            1.0
        } else {
            (self.callus / self.radius.max(1e-5)).clamp(0.0, 1.0)
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CutRecord {
    day: u64,
    kind: CutKind,
    diameter_cm: f32,
    foliage_share: f32,
    wound: u32,
    natural: bool,
}

/// Pruning state carried by a [`Simulation`]. Inactive (and without any
/// effect on growth) until the first cut.
#[derive(Clone, Debug, Default)]
pub struct PruneState {
    log: Vec<LogEvent>,
    /// Events before this index have been applied.
    cursor: usize,
    /// Light / foliage must be recomputed before the next simulated day.
    light_dirty: bool,
    pub(crate) active: bool,
    wounds: Vec<Wound>,
    records: Vec<CutRecord>,
    next_wound: u32,
    /// Reserve status, 1 = full for the tree's size.
    reserve: f32,
    /// Leaf area (proxy) the root system and reserves were built for.
    root_cap: f32,
    /// Fine-root fraction lost last year.
    fine_root_loss: f32,
    /// Vigour multiplier applied at the last flush.
    last_vigour: f32,
    /// New shoots formed at the last flush (also tracked when unpruned).
    pub(crate) last_new_shoots: u32,
    starving_years: u32,
    dead: bool,
    /// Share of the crown removed since the last end of season.
    removed_this_year: f32,
    /// Lasting injury from heavy pruning (removal beyond ~25 % of the
    /// foliage in a year, the limit of the ANSI A300 pruning standard and of
    /// Lilly, Gilman & Smiley 2019, "Best Management Practices: Pruning",
    /// 3rd ed., ISA). Decays slowly (calibration). Feeds the life
    /// expectancy heuristic only.
    injury: f32,
}

impl PruneState {
    pub(crate) fn has_pending(&self) -> bool {
        self.cursor < self.log.len() || self.light_dirty
    }

    pub(crate) fn record_drought(&mut self, day: u64, on: bool) {
        self.log.truncate(self.cursor);
        self.log.push(LogEvent { day, action: Action::Drought(on) });
        self.cursor = self.log.len();
    }

    /// Growth capacity factor (1 unless starving).
    pub(crate) fn capacity(&self) -> f32 {
        if self.dead {
            return 0.0;
        }
        let x = (self.reserve / 0.4).clamp(0.0, 1.0);
        0.3 + 0.7 * x * x * (3.0 - 2.0 * x)
    }

    /// Root extension factor: roots stop growing while they exceed what the
    /// crown can feed (functional balance).
    pub(crate) fn root_growth_factor(&self) -> f32 {
        1.0 - self.fine_root_loss.clamp(0.0, 0.9)
    }
}

/// What a cut would do (tooltip), see [`Simulation::cut_preview`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CutPreview {
    pub kind: CutKind,
    /// Diameter of the cut (cm).
    pub diameter_cm: f32,
    /// Share of the leaf area removed (0..1).
    pub foliage_share: f32,
    /// Share of the wood volume removed (0..1).
    pub wood_share: f32,
    /// Expected years to closure (99 = will not close).
    pub closure_years: f32,
    /// Probability that decay sets in before closure (0..1).
    pub decay_risk: f32,
    /// Wound pathogen infection risk now (silver leaf, canker) (0..1).
    pub infection_risk: f32,
    /// Season advice: 0 good, 1 acceptable, 2 bad.
    pub season: u8,
    /// Expected epicormic sprouts at the cut.
    pub sprouts: f32,
    /// Cut point, axis direction and drawn radius (m) there.
    pub point: [f32; 3],
    pub dir: [f32; 3],
    pub radius_m: f32,
    /// Bleeding expected (late-winter cut on a bleeding species).
    pub bleeding: bool,
}

/// A ray hit on a branch, see [`Simulation::pick_branch`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickHit {
    pub node_id: u32,
    /// Position along the visible segment, snapped to its base (collar)
    /// when the hit was close to it.
    pub t: f32,
    pub snapped: bool,
    /// Distance along the ray (m).
    pub ray_t: f32,
}

/// A shoot node, see [`Simulation::shoot_nodes`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShootInfo {
    pub id: u32,
    /// `u32::MAX` for the collar node.
    pub parent_id: u32,
    pub order: u8,
    pub radius_m: f32,
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub terminal: bool,
    pub leafy: bool,
    pub dead: bool,
}

/// One line of the cut journal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JournalEntry {
    pub day: u64,
    pub kind: CutKind,
    pub diameter_cm: f32,
    pub foliage_share: f32,
    pub state: WoundState,
    /// Closure 0..1.
    pub closure: f32,
    /// Decayed share of the wound core's cross-section (0..1).
    pub decay: f32,
    /// Pathogen infection (silver leaf / canker) took hold.
    pub pathogen: bool,
    /// Not a cut (breakage, pathogen dieback).
    pub natural: bool,
}

/// Measurements for comparison and objectives.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PruneMetrics {
    pub age_years: f32,
    pub height_m: f32,
    pub crown_radius_m: f32,
    pub dbh_cm: f32,
    /// Leaf area proxy (m2 of leafy crown cells).
    pub leaf_area_m2: f32,
    /// Reserve status (1 = full).
    pub reserve: f32,
    /// Shoot vigour multiplier of the last flush (1 = normal).
    pub vigour: f32,
    /// New shoots at the last flush.
    pub new_shoots: u32,
    /// Live shoots growing directly from cut hosts (epicormic / water
    /// sprouts, coppice and pollard shoots).
    pub sprouts: u32,
    /// Decayed wood volume (m3).
    pub decay_volume_m3: f32,
    pub open_wounds: u32,
    pub closed_wounds: u32,
    pub decayed_wounds: u32,
    /// Fine roots lost last year (fraction).
    pub fine_root_loss: f32,
    /// Estimated lifespan (years, total age at death). Heuristic.
    pub life_expectancy_years: f32,
    pub alive: bool,
}

/// Stem quality, for timber objectives.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct BoleMetrics {
    /// Height of the stem free of live branches (m).
    pub clear_bole_m: f32,
    /// Height of the stem free of live branches and of open wounds (m):
    /// the "knot-free bole" once its knots are occluded.
    pub knot_free_bole_m: f32,
    /// Diameter of the knotty core of that bole (cm): the stem diameter
    /// when its last pruning wound closed.
    pub knotty_core_cm: f32,
    /// Clear (knot-free) wood around the core at 1.3 m (mm of radius).
    pub clear_wood_mm: f32,
    /// Open pruning wounds on the bole.
    pub open_wounds: u32,
    /// Pollard heads (cut stems carrying sprouts) and the most recuts on one.
    pub pollard_heads: u32,
    pub pollard_max_recuts: u32,
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Reserve availability by phenological phase: the seasonal cycle of
/// non-structural carbohydrates, lowest just after the spring flush, refilled
/// from late summer (Loescher et al. 1990). Values are a calibration.
fn season_reserve(phase: Phase) -> f32 {
    match phase {
        Phase::Dormant => 1.0,
        Phase::BudBurst => 0.6,
        Phase::LeafExpansion => 0.55,
        Phase::Mature => 0.75,
        Phase::Colouring => 0.9,
        Phase::LeafFall => 0.95,
    }
}

/// Wound pathogen season: spores of silver leaf are released from September
/// to May (RHS), bacterial canker enters autumn/winter wounds (Kennelly et
/// al. 2007); June-August wounds are safest. `doy` is 0-based.
fn pathogen_season(t: &PruningTraits, doy: u32) -> f32 {
    if (151..243).contains(&doy) {
        t.pathogen_summer_risk
    } else {
        t.pathogen_dormant_risk
    }
}

/// Closest approach of a ray (o + s d, |d| = 1) and a segment [a, b]:
/// (ray parameter s, segment parameter u in 0..1, distance).
fn ray_segment(o: Vec3, d: Vec3, a: Vec3, b: Vec3) -> (f32, f32, f32) {
    let e = b - a;
    let ee = e.dot(e);
    let w = o - a;
    let de = d.dot(e);
    let dw = d.dot(w);
    let ew = e.dot(w);
    let denom = ee - de * de;
    let mut u = if denom > 1e-9 { ((ew - de * dw) / denom).clamp(0.0, 1.0) } else { 0.0 };
    if ee <= 1e-12 {
        u = 0.0;
    }
    let p = a + e * u;
    let s = (p - o).dot(d).max(0.0);
    // Re-project onto the segment from the ray point.
    if ee > 1e-12 {
        let q = o + d * s;
        u = ((q - a).dot(e) / ee).clamp(0.0, 1.0);
    }
    let p = a + e * u;
    let s = (p - o).dot(d).max(0.0);
    let dist = (o + d * s - p).length();
    (s, u, dist)
}

/// Internal result of analysing a cut.
struct CutPlan {
    idx: usize,
    kind: CutKind,
    /// Remove the whole node (cut at its base) vs keep a remnant.
    at_base: bool,
    cut_len: f32,
    r: f32,
    host: usize,
    point: Vec3,
    dir: Vec3,
}

impl Simulation {
    // ------------------------------------------------------------------
    // Timeline
    // ------------------------------------------------------------------

    /// The intervention log (cuts and drought toggles), oldest first.
    pub fn prune_log(&self) -> &[LogEvent] {
        &self.prune.log
    }

    /// Install an intervention log. Events dated before the current day are
    /// taken as already applied (install on a fresh simulation, or on a
    /// checkpoint taken at the start of the day of an event); events of
    /// today are applied now, later ones when their day comes.
    pub fn set_prune_log(&mut self, log: Vec<LogEvent>) {
        let day = self.day;
        self.prune.cursor = log.iter().position(|e| e.day >= day).unwrap_or(log.len());
        self.prune.log = log;
        self.apply_due_events();
    }

    /// A copy for checkpointing (scratch buffers dropped).
    pub fn checkpoint(&self) -> Simulation {
        let mut c = self.clone();
        c.shoots.strip_scratch();
        c
    }

    /// Re-simulate `base` (a fresh simulation or a checkpoint) with `log`
    /// up to day `to_day`.
    pub fn replayed(base: &Simulation, log: &[LogEvent], to_day: u64) -> Simulation {
        let mut s = base.checkpoint();
        s.set_prune_log(log.to_vec());
        if to_day > s.day {
            let mut left = to_day - s.day;
            while left > 0 {
                let n = left.min(3650) as u32;
                s.advance_days(n);
                left -= n as u64;
            }
        }
        s
    }

    /// Apply the logged events due today (and the deferred light pass).
    pub(crate) fn apply_due_events(&mut self) {
        while self.prune.cursor < self.prune.log.len() && self.prune.log[self.prune.cursor].day <= self.day {
            let ev = self.prune.log[self.prune.cursor];
            self.prune.cursor += 1;
            match ev.action {
                Action::Cut(c) => {
                    self.apply_cut(c, false);
                }
                Action::Drought(on) => self.drought = on,
            }
        }
        if self.prune.light_dirty {
            self.prune.light_dirty = false;
            self.shoots.compute_light();
            let a = &self.genome.architecture;
            self.shoots.compute_foliage(a.hydraulic_path_max_m, a.hydraulic_foliage_exponent);
        }
    }

    /// Cut node `node_id` at `t` (0 = base .. 1 = tip of its visible
    /// segment) now, and log it. `flush`: cut flush with the parent instead
    /// of at the collar (only meaningful at a branch base). Returns the cut
    /// type, or `None` when the node does not exist.
    pub fn cut(&mut self, node_id: u32, t: f32, flush: bool) -> Option<CutKind> {
        let spec = CutSpec { node_id, t: t.clamp(0.0, 1.0), flush };
        let kind = self.apply_cut(spec, true)?;
        // A new cut while later events exist (after rewinding) branches the
        // timeline: the later events are dropped.
        self.prune.log.truncate(self.prune.cursor);
        self.prune.log.push(LogEvent { day: self.day, action: Action::Cut(spec) });
        self.prune.cursor = self.prune.log.len();
        Some(kind)
    }

    // ------------------------------------------------------------------
    // Cutting
    // ------------------------------------------------------------------

    fn draw_r(&self, rs: f32) -> f32 {
        let nodes = &self.shoots.nodes;
        draw_radius(rs, nodes[0].radius().max(1e-6), self.genome.wood.pipe_exponent.max(2.0), self.genome.wood.limb_draw_boost.max(1.0))
    }

    /// Length (m) from a branch base within which a cut is a collar cut.
    fn collar_zone(&self, idx: usize) -> f32 {
        let n = &self.shoots.nodes[idx];
        (2.0 * self.draw_r(n.radius())).max(0.05)
    }

    fn plan(&self, spec: CutSpec) -> Option<CutPlan> {
        let idx = self.shoots.index_of(spec.node_id)?;
        let nodes = &self.shoots.nodes;
        let ext = self.shoots.ext;
        let n = &nodes[idx];
        let vis = (n.end(ext) - n.start).length().max(1e-4);
        let mut cut_len = spec.t.clamp(0.0, 1.0) * vis;
        let r = n.radius();
        let p = n.parent as usize;
        let near_base = idx != 0 && cut_len <= self.collar_zone(idx);
        // A shoot on a pollard head (a cut stem end) is cut at its base even
        // when it took over as the stem.
        let on_head = idx != 0 && nodes[p].cut_end && self.prune.wounds.iter().any(|w| w.host == nodes[p].id && !w.side);
        let stem = n.order == 0 && !(near_base && on_head);
        let at_base = !stem && near_base;
        let kind = if stem {
            cut_len = cut_len.max(if idx == 0 { 0.02 } else { 0.005 }).min(vis);
            let y = n.start.y + n.dir.y * cut_len;
            if y < 0.6 {
                CutKind::Coppice
            } else if (1.2..=4.0).contains(&y) {
                CutKind::Pollard
            } else {
                CutKind::Topping
            }
        } else if at_base {
            cut_len = 0.0;
            let head = on_head;
            if n.terminal {
                let lateral = nodes.iter().skip(p + 1).any(|c| c.parent as usize == p && !c.terminal && !c.is_snag() && c.radius() >= r / 3.0);
                if head {
                    CutKind::Pollard
                } else if lateral {
                    CutKind::Reduction
                } else {
                    CutKind::Heading
                }
            } else if spec.flush {
                CutKind::Flush
            } else if head {
                CutKind::Pollard
            } else {
                CutKind::Collar
            }
        } else {
            cut_len = cut_len.max(0.005);
            if n.terminal {
                CutKind::Heading
            } else {
                CutKind::Stub
            }
        };
        let point = n.start + n.dir * cut_len;
        Some(CutPlan { idx, kind, at_base, cut_len, r, host: if at_base { p } else { idx }, point, dir: n.dir })
    }

    /// Latent buds released by a cut and their vigour boost.
    fn release(&self, plan: &CutPlan, severity: f32, key: u64) -> (u8, f32) {
        let tr = &self.genome.pruning;
        let u = rng::unit(rng::hash(self.seed, S_PRUNE, key, 7));
        let mut potential = match plan.kind {
            CutKind::Coppice => (tr.stump_sprouting + 0.5 * tr.root_suckering).min(1.0),
            _ => tr.epicormic_potential,
        };
        // The latent bud bank of a host is used up by repeated releases (lost
        // buds are not replaced: Fontaine et al. 2001); pollard heads keep
        // forming new buds in their callus.
        if self.shoots.nodes[plan.host].wounded && plan.kind != CutKind::Pollard {
            potential *= 0.4;
        }
        // Latent buds per cut (calibration: a topped oak stem bears a dense
        // tuft of sprouts; a collar cut releases a few around the wound).
        let per = match plan.kind {
            CutKind::Coppice => 16.0,
            // A pollard head re-forms its tuft of buds at each recut (Read 2000).
            CutKind::Topping | CutKind::Pollard => 14.0,
            CutKind::Heading => 8.0,
            CutKind::Reduction | CutKind::Stub => 2.0,
            _ => 1.5,
        };
        // Light reaching the wound region helps latent buds break.
        // An end cut opens the stump of the axis to the sky; a branch removal
        // releases buds only where light reaches the host.
        let light = if plan.at_base { self.shoots.nodes[plan.host].light.clamp(0.4, 1.0) } else { 1.0 };
        let n = (potential * per * (0.75 + 0.5 * u) * light).round().clamp(0.0, 20.0) as u8;
        let reserve = if self.prune.active { self.prune.reserve.clamp(0.0, 1.0) } else { 1.0 };
        let boost = tr.sprout_vigour * season_reserve(self.phen.phase) * reserve * (0.4 + severity.min(1.0));
        (n, boost.min(3.0))
    }

    /// Apply a cut. `live`: made by the player now (vs replayed): only used
    /// to decide nothing differently; kept for clarity.
    fn apply_cut(&mut self, spec: CutSpec, _live: bool) -> Option<CutKind> {
        let plan = self.plan(spec)?;
        Some(self.execute(plan, false))
    }

    fn execute(&mut self, plan: CutPlan, natural: bool) -> CutKind {
        let year = self.year();
        let doy = self.day_of_year();
        let kind = if natural { CutKind::Natural } else { plan.kind };
        let total_leaf = self.shoots.leaf_area_proxy().max(1e-6);
        let total_wood = self.shoots.wood_volume().max(1e-9);
        if !self.prune.active {
            self.prune.active = true;
            self.prune.reserve = 1.0;
            self.prune.root_cap = total_leaf;
            self.prune.last_vigour = 1.0;
        }
        let idx = plan.idx;
        let node_id = self.shoots.nodes[idx].id;
        let host_id = self.shoots.nodes[plan.host].id;
        let host_r = self.shoots.nodes[plan.host].radius();
        let (leaf, wood) = if plan.at_base {
            self.shoots.remove_subtree(idx, false)
        } else {
            let unit_len = 0.75 * self.shoots.voxel - 0.06;
            let face = self.shoots.voxel * self.shoots.voxel;
            let (l, w) = self.shoots.remove_subtree(idx, true);
            let n = &mut self.shoots.nodes[idx];
            let cut = n.length - plan.cut_len.min(n.length);
            let mut l2 = 0.0;
            if n.leafy {
                l2 = 0.5 * face + 0.5 * face * (n.length / unit_len.max(1e-3)).min(1.0);
            }
            let w2 = n.area * cut;
            n.length = plan.cut_len.min(n.length).max(0.002);
            n.grow_from = n.grow_from.min(n.length);
            n.path_end -= cut;
            n.leafy = false;
            n.term_bud = false;
            n.lat_buds = 0;
            n.cut_end = true;
            (l + l2, w + w2)
        };
        let severity = (leaf / total_leaf).clamp(0.0, 1.0);
        if !natural {
            // Pollarding and coppicing are sustainable on species that
            // resprout well (old pollards outlive maiden trees, Read 2000):
            // they count towards the injury in proportion to the species'
            // lack of resprouting ability (calibration).
            let tolerance = match plan.kind {
                CutKind::Pollard => self.genome.pruning.epicormic_potential,
                CutKind::Coppice => (self.genome.pruning.stump_sprouting + 0.5 * self.genome.pruning.root_suckering).min(1.0),
                _ => 0.0,
            };
            let s = severity * (1.0 - tolerance);
            self.prune.removed_this_year += s * (1.0 - self.prune.removed_this_year);
        }
        let wood_share = (wood / total_wood).clamp(0.0, 1.0);
        // Reserves stored in the removed wood are lost; a cut during the
        // flush also loses what was just invested in the new leaves.
        let tr = self.genome.pruning.clone();
        let mut lost = tr.reserve_in_branches * wood_share;
        if matches!(self.phen.phase, Phase::BudBurst | Phase::LeafExpansion) {
            lost += 0.15 * severity;
        }
        self.prune.reserve = (self.prune.reserve - lost).max(0.0);

        // Host after the removal (indices shift when removing at the base).
        let host = self.shoots.index_of(host_id).unwrap_or(0);
        let wid = self.prune.next_wound;
        self.prune.next_wound += 1;
        {
            let key = (node_id as u64) << 20 ^ self.day;
            let (mut buds, boost) = self.release(&CutPlan { host, ..plan }, severity, key);
            if natural {
                buds /= 2;
            }
            let h = &mut self.shoots.nodes[host];
            if buds > 0 && boost > 0.0 {
                h.lat_buds = h.lat_buds.max(buds);
                h.bud_year = year;
                h.sprout = h.sprout.max(boost);
            } else if !plan.at_base && h.lat_buds == 0 {
                // No bud to rely on: the remnant is dead wood at once.
                h.snag_since = year;
                h.sprout = 0.0;
            }
        }
        self.shoots.nodes[host].wounded = true;
        if !plan.at_base && host != 0 {
            let p = self.shoots.nodes[host].parent as usize;
            self.shoots.nodes[p].wounded = true;
        }

        // The wound.
        let dormant_winter = self.phen.phase == Phase::Dormant && !(59..335).contains(&doy);
        let mut radius = plan.r;
        if kind == CutKind::Flush {
            // Flush cuts expose much more wood than the branch section
            // (Neely 1988).
            radius *= 1.35;
        }
        let core_r = match kind {
            CutKind::Collar | CutKind::Pollard if plan.at_base => 0.5 * plan.r,
            CutKind::Stub => 0.5 * plan.r,
            CutKind::Reduction => 0.8 * plan.r,
            CutKind::Flush => host_r.max(plan.r),
            _ => host_r.max(plan.r),
        };
        let p_path = pathogen_season(&tr, doy) * (2.0 * radius / 0.05).clamp(0.3, 2.5);
        let infected_path = !natural && rng::unit(rng::hash(self.seed, S_PRUNE, wid as u64, 0x9A7)) < p_path;
        // A remnant that is dead wood at once is a stub: its wound only
        // starts closing on the parent once it has broken off.
        let stub_parent = if !plan.at_base && self.shoots.nodes[host].is_snag() && host != 0 {
            self.shoots.nodes[self.shoots.nodes[host].parent as usize].id
        } else {
            NONE
        };
        // Pollard recut: count the recuts on the head.
        if plan.at_base && kind == CutKind::Pollard {
            let day = self.day;
            if let Some(hw) = self.prune.wounds.iter_mut().find(|w| w.host == host_id && !w.side && !w.gone) {
                if hw.last_recut != day {
                    hw.last_recut = day;
                    hw.last_recut_year = year;
                    hw.recuts += 1;
                }
            }
        }
        self.prune.wounds.push(Wound {
            id: wid,
            kind,
            host: host_id,
            stub_parent,
            side: plan.at_base,
            dir: plan.dir,
            radius,
            core_r,
            host_r0: host_r,
            host_r_prev: self.shoots.nodes[host].radius(),
            callus: 0.0,
            close_k: if dormant_winter { 0.8 } else { 1.0 },
            closed_year: NONE,
            r_at_close: 0.0,
            infected: false,
            decay_r: 0.0,
            decay_len: 0.0,
            pathogen: if infected_path { 0.0 } else { -1.0 },
            pathogen_done: false,
            year,
            recuts: 0,
            last_recut: u64::MAX,
            last_recut_year: year,
            gone: false,
            y: plan.point.y,
        });
        self.prune.records.push(CutRecord { day: self.day, kind, diameter_cm: 200.0 * plan.r, foliage_share: severity, wound: wid, natural });
        self.prune.light_dirty = true;
        self.wood_version += 1;
        self.leaf_version += 1;
        self.flower_version += 1;
        kind
    }

    // ------------------------------------------------------------------
    // Yearly response
    // ------------------------------------------------------------------

    /// Shoot vigour multiplier for the coming flush: compensatory growth
    /// from the root:shoot imbalance, paid from the reserves.
    pub(crate) fn prune_flush_vigour(&mut self) -> f32 {
        if self.prune.dead {
            self.prune.last_vigour = 0.0;
            return 0.0;
        }
        let tr = &self.genome.pruning;
        let leaf = self.shoots.leaf_area_proxy().max(1e-4);
        // Calibration: the multiplier saturates at 1 + 8 x the trait (a
        // coppice stool regrows from a root system many times too large).
        let imbalance = (self.prune.root_cap / leaf - 1.0).clamp(0.0, 8.0);
        let m = (1.0 + tr.compensatory_growth * imbalance * self.prune.reserve.min(1.0)).clamp(0.3, 1.0 + 8.0 * tr.compensatory_growth);
        // Regrowth draws on the reserves (calibration).
        self.prune.reserve = (self.prune.reserve - 0.03 * (m - 1.0)).max(0.0);
        self.prune.last_vigour = m * self.prune.capacity();
        m
    }

    /// End-of-season update: reserves, root balance, wounds, decline.
    pub(crate) fn prune_yearly(&mut self, year: u32) {
        let tr = self.genome.pruning.clone();
        let leaf = self.shoots.leaf_area_proxy();
        // Reserves refill when the crown is large enough for the living
        // tissue it supports, run down otherwise (calibration: break-even at
        // 25 % of the leaf area the roots were built for).
        let q = (leaf / self.prune.root_cap.max(1e-4)).min(1.2);
        self.prune.reserve = (self.prune.reserve + 0.3 * (q - 0.25)).clamp(0.0, 1.0);
        // Fine roots die back towards the new balance.
        if leaf < self.prune.root_cap {
            let lost = (self.prune.root_cap - leaf) * tr.root_balance_rate;
            self.prune.fine_root_loss = lost / self.prune.root_cap.max(1e-6);
            self.prune.root_cap -= lost;
        } else {
            self.prune.fine_root_loss = 0.0;
            self.prune.root_cap = leaf;
        }
        self.prune.injury = 0.97 * self.prune.injury + (self.prune.removed_this_year - 0.25).max(0.0);
        self.prune.removed_this_year = 0.0;
        self.update_wounds(year, &tr);
        // Decline of a starving tree: outer boughs die, then the whole tree.
        if self.prune.reserve < 0.15 {
            self.prune.starving_years += 1;
            let p = 0.3 * (1.0 - self.prune.reserve / 0.15);
            let seed = self.seed;
            for n in self.shoots.nodes.iter_mut().skip(1) {
                if !n.is_snag() && n.order > 0 && n.radius() < 0.08 && n.leaf_n > 0.0 && rng::unit(rng::hash(seed, S_PRUNE, n.id as u64, (year as u64) << 8 | 3)) < p {
                    n.snag_since = year;
                    n.leafy = false;
                    n.term_bud = false;
                    n.lat_buds = 0;
                }
            }
            if self.prune.starving_years >= 4 && self.prune.reserve <= 0.0 {
                self.prune.dead = true;
                for n in self.shoots.nodes.iter_mut() {
                    if !n.is_snag() {
                        n.snag_since = year;
                    }
                    n.leafy = false;
                    n.term_bud = false;
                    n.lat_buds = 0;
                }
            }
            self.wood_version += 1;
        } else {
            self.prune.starving_years = 0;
        }
    }

    fn update_wounds(&mut self, year: u32, tr: &PruningTraits) {
        let map: HashMap<u32, usize> = self.shoots.nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
        let mut breaks: Vec<(usize, bool)> = Vec::new();
        let mut kills: Vec<usize> = Vec::new();
        let seed = self.seed;
        let mut reserve_hit = 0.0f32;
        for w in self.prune.wounds.iter_mut() {
            if w.gone {
                continue;
            }
            let mut h = map.get(&w.host).copied();
            if h.is_none() && w.stub_parent != NONE {
                // The stub broke off: the collar wound on the parent can
                // now close.
                w.host = w.stub_parent;
                w.stub_parent = NONE;
                w.side = true;
                h = map.get(&w.host).copied();
                if let Some(hi) = h {
                    w.host_r_prev = self.shoots.nodes[hi].radius();
                }
                continue;
            }
            let Some(hi) = h else {
                w.gone = true;
                continue;
            };
            let host = &self.shoots.nodes[hi];
            if host.is_snag() && !w.side && w.stub_parent == NONE && hi != 0 {
                // The remnant died back (no bud broke): it is now a stub.
                w.stub_parent = self.shoots.nodes[host.parent as usize].id;
                continue;
            }
            let r_h = host.radius();
            let dr = (r_h - w.host_r_prev).max(0.0);
            w.host_r_prev = r_h;
            let open = w.closed_year == NONE;
            let on_stub = w.stub_parent != NONE;
            if open && !on_stub {
                w.callus += dr * tr.woundwood_ratio * w.kind.closure_factor() * w.close_k;
                if w.callus >= w.radius {
                    w.closed_year = year;
                    w.r_at_close = r_h;
                }
            }
            let wid = w.id as u64;
            let hkey = |s: u64| rng::unit(rng::hash(seed, S_PRUNE, wid << 16 | s, year as u64));
            if open && !w.infected {
                let size = (2.0 * w.radius / 0.1).clamp(0.1, 4.0);
                let p = tr.decay_infection_rate * size * w.kind.infection_factor() * (1.0 - 0.7 * tr.compartmentalization);
                if hkey(1) < p {
                    w.infected = true;
                }
            }
            if w.infected {
                let slow = if open { 1.0 } else { 0.5 };
                w.decay_r = (w.decay_r + tr.decay_radial_mm_per_year * 0.001 * slow).min(w.core_r);
                w.decay_len += tr.decay_axial_m_per_year * (1.0 - tr.compartmentalization) * slow;
            }
            if w.pathogen >= 0.0 && !w.pathogen_done {
                w.pathogen += 1.0 / tr.pathogen_dieback_years.max(0.5);
                if w.pathogen >= 1.0 {
                    w.pathogen_done = true;
                    w.infected = true;
                    if host.order > 0 {
                        kills.push(hi);
                    } else {
                        reserve_hit += 0.25;
                    }
                }
            }
            // Residual sound shell (Mattheck & Breloer 1994).
            if w.decay_r > 0.0 && host.leaf_n > 0.0 && r_h > 0.0 {
                let t_r = (r_h - w.decay_r) / r_h;
                if t_r < 0.32 && hkey(2) < 0.04 + 0.4 * (0.32 - t_r) / 0.32 {
                    breaks.push((hi, w.side));
                }
            }
        }
        self.prune.reserve = (self.prune.reserve - reserve_hit).max(0.0);
        for &i in &kills {
            let n = self.shoots.nodes.len();
            let mut inside = vec![false; n];
            inside[i] = true;
            for j in i..n {
                if j > i {
                    let p = self.shoots.nodes[j].parent as usize;
                    if p >= n || !inside[p] {
                        continue;
                    }
                    inside[j] = true;
                }
                let nd = &mut self.shoots.nodes[j];
                if !nd.is_snag() {
                    nd.snag_since = year;
                }
                nd.leafy = false;
                nd.term_bud = false;
                nd.lat_buds = 0;
                nd.sprout = 0.0;
            }
            let id = self.shoots.nodes[i].id;
            if let Some(w) = self.prune.wounds.iter().find(|w| w.host == id) {
                let wid = w.id;
                self.prune.records.push(CutRecord { day: self.day, kind: CutKind::Natural, diameter_cm: 200.0 * self.shoots.nodes[i].radius(), foliage_share: 0.0, wound: wid, natural: true });
            }
            self.wood_version += 1;
        }
        // Breakage: the part beyond the decayed host end falls (ids first,
        // indices shift).
        let ids: Vec<(u32, bool)> = breaks.iter().map(|&(i, side)| (self.shoots.nodes[i].id, side)).collect();
        for (id, side) in ids {
            let Some(i) = self.shoots.index_of(id) else { continue };
            if side {
                // Decay in a limb at a fork: the limb breaks there.
                let n = &self.shoots.nodes[i];
                if i == 0 {
                    continue;
                }
                let len = n.length;
                let plan = CutPlan { idx: i, kind: CutKind::Natural, at_base: false, cut_len: len, r: n.radius(), host: i, point: n.start + n.dir * len, dir: n.dir };
                self.execute(plan, true);
            } else {
                // A decayed cut end (topping, pollard head): the heaviest
                // shoot anchored in it tears out.
                let nodes = &self.shoots.nodes;
                let Some((c, _)) = nodes.iter().enumerate().skip(i + 1).filter(|(_, c)| c.parent as usize == i && !c.is_snag()).max_by(|a, b| a.1.area.total_cmp(&b.1.area)) else { continue };
                let n = &nodes[c];
                let plan = CutPlan { idx: c, kind: CutKind::Natural, at_base: true, cut_len: 0.0, r: n.radius(), host: i, point: n.start, dir: n.dir };
                self.execute(plan, true);
            }
        }
    }

    // ------------------------------------------------------------------
    // Queries
    // ------------------------------------------------------------------

    /// A read-only list of the shoot nodes (for tests, scripts and tools).
    pub fn shoot_nodes(&self) -> Vec<ShootInfo> {
        let nodes = &self.shoots.nodes;
        let ext = self.shoots.ext;
        nodes
            .iter()
            .map(|n| ShootInfo {
                id: n.id,
                parent_id: if n.parent == NONE { u32::MAX } else { nodes[n.parent as usize].id },
                order: n.order,
                radius_m: n.radius(),
                start: n.start.to_array(),
                end: n.end(ext).to_array(),
                terminal: n.terminal,
                leafy: n.leafy,
                dead: n.is_snag(),
            })
            .collect()
    }

    /// True once the tree has been cut at least once.
    pub fn is_pruned(&self) -> bool {
        self.prune.active
    }

    /// Branch under a ray (tree coordinates: Y up, metres, collar at the
    /// origin), front-most hit. `tol_rad`: angular pick tolerance added to
    /// the drawn radius (pixels x radians per pixel). Hits close to a branch
    /// base snap to its collar.
    pub fn pick_branch(&self, origin: [f32; 3], dir: [f32; 3], tol_rad: f32) -> Option<PickHit> {
        let o = Vec3::new(origin[0], origin[1], origin[2]);
        let d = Vec3::new(dir[0], dir[1], dir[2]).normalize_or(Vec3::Y);
        let ext = self.shoots.ext;
        let nodes = &self.shoots.nodes;
        let r_ref = nodes[0].radius().max(1e-6);
        let e = self.genome.wood.pipe_exponent.max(2.0);
        let boost = self.genome.wood.limb_draw_boost.max(1.0);
        let mut best: Option<(f32, usize, f32)> = None;
        for (i, n) in nodes.iter().enumerate() {
            let a = n.start;
            let b = n.end(ext);
            if (b - a).length() < 1e-4 {
                continue;
            }
            // Cheap reject: distance from the ray to the segment's bounding sphere.
            let c = (a + b) * 0.5;
            let rad = 0.5 * (b - a).length();
            let oc = c - o;
            let s0 = oc.dot(d);
            let perp = (oc - d * s0).length();
            let rd = draw_radius(n.radius(), r_ref, e, boost);
            let tol = rd + tol_rad * s0.max(0.0) + 0.01;
            if perp > rad + tol || s0 < -rad {
                continue;
            }
            let (s, u, dist) = ray_segment(o, d, a, b);
            if dist <= rd + tol_rad * s + 0.005 && best.is_none_or(|bb| s < bb.0) {
                best = Some((s, i, u));
            }
        }
        let (s, i, u) = best?;
        let n = &nodes[i];
        let vis = (n.end(ext) - n.start).length();
        let snap = n.order > 0 && u * vis <= self.collar_zone(i);
        Some(PickHit { node_id: n.id, t: if snap { 0.0 } else { u }, snapped: snap, ray_t: s })
    }

    /// What cutting `node_id` at `t` would do now (no change).
    pub fn cut_preview(&self, node_id: u32, t: f32, flush: bool) -> Option<CutPreview> {
        let plan = self.plan(CutSpec { node_id, t, flush })?;
        let tr = &self.genome.pruning;
        let nodes = &self.shoots.nodes;
        let unit_len = 0.75 * self.shoots.voxel - 0.06;
        let face = self.shoots.voxel * self.shoots.voxel;
        // Removed leaf area and wood, and the host's remaining foliage.
        let n = nodes.len();
        let mut inside = vec![false; n];
        inside[plan.idx] = true;
        let (mut leaf, mut wood, mut fol) = (0.0f32, 0.0f32, 0.0f32);
        for j in plan.idx..n {
            if j > plan.idx {
                let p = nodes[j].parent as usize;
                if p >= n || !inside[p] {
                    continue;
                }
                inside[j] = true;
            }
            let nd = &nodes[j];
            let len = if j == plan.idx && !plan.at_base { nd.length - plan.cut_len } else { nd.length };
            wood += nd.area * len.max(0.0);
            if nd.leafy {
                leaf += (0.5 + 0.5 * (nd.length / unit_len.max(1e-3)).min(1.0)) * face;
            }
            if j == plan.idx {
                fol += nd.foliage;
            }
        }
        let total_leaf = self.shoots.leaf_area_proxy().max(1e-6);
        let severity = (leaf / total_leaf).clamp(0.0, 1.0);
        // A stub's wound closes on its parent once the stub has broken off.
        let closing_on = if plan.kind == CutKind::Stub && plan.idx != 0 { nodes[plan.idx].parent as usize } else { plan.host };
        let host = &nodes[closing_on];
        let host_fol = if plan.at_base || closing_on != plan.host { (host.foliage - fol).max(0.0) } else { 0.0 };
        // Expected radial growth of the host (Pressler, average year).
        let w = &self.genome.wood;
        let r_h = host.radius().max(1e-4);
        let dr = (w.growth_coefficient * host_fol / (std::f32::consts::TAU * r_h)).min(w.max_ring_width_mm * 0.001);
        let mut radius = plan.r;
        if plan.kind == CutKind::Flush {
            radius *= 1.35;
        }
        let rate = dr * tr.woundwood_ratio * plan.kind.closure_factor();
        let mut closure_years = if rate > 1e-6 { (radius / rate).min(99.0) } else { 99.0 };
        if plan.kind == CutKind::Stub {
            // The stub must die and break off first.
            closure_years = (closure_years + 2.0 + self.genome.wood.snag_years_per_m * plan.r).min(99.0);
        }
        let size = (2.0 * radius / 0.1).clamp(0.1, 4.0);
        let p_year = (tr.decay_infection_rate * size * plan.kind.infection_factor() * (1.0 - 0.7 * tr.compartmentalization)).min(1.0);
        let decay_risk = 1.0 - (1.0 - p_year).powf(closure_years.min(60.0));
        let doy = self.day_of_year();
        let infection_risk = (pathogen_season(tr, doy) * (2.0 * radius / 0.05).clamp(0.3, 2.5)).min(1.0);
        let phase = self.phen.phase;
        let season = if tr.pathogen_dormant_risk > 3.0 * tr.pathogen_summer_risk && tr.pathogen_dormant_risk >= 0.15 {
            // Prunus: summer only (silver leaf, canker).
            if (151..243).contains(&doy) {
                0
            } else {
                2
            }
        } else {
            match phase {
                Phase::Dormant => 0,
                Phase::BudBurst | Phase::LeafExpansion => 2,
                _ => 1,
            }
        };
        let potential = if plan.kind == CutKind::Coppice { (tr.stump_sprouting + 0.5 * tr.root_suckering).min(1.0) } else { tr.epicormic_potential };
        let per = match plan.kind {
            CutKind::Coppice => 16.0,
            // A pollard head re-forms its tuft of buds at each recut (Read 2000).
            CutKind::Topping | CutKind::Pollard => 14.0,
            CutKind::Heading => 8.0,
            CutKind::Reduction | CutKind::Stub => 2.0,
            _ => 1.5,
        };
        let sprouts = potential * per * if plan.at_base { host.light.clamp(0.4, 1.0) } else { 1.0 };
        let bleeding = tr.sap_bleeding > 0.3 && (32..120).contains(&doy);
        Some(CutPreview {
            kind: plan.kind,
            diameter_cm: 200.0 * plan.r,
            foliage_share: severity,
            wood_share: (wood / self.shoots.wood_volume().max(1e-9)).clamp(0.0, 1.0),
            closure_years,
            decay_risk,
            infection_risk,
            season,
            sprouts,
            point: plan.point.to_array(),
            dir: plan.dir.to_array(),
            radius_m: self.draw_r(plan.r),
            bleeding,
        })
    }

    /// Wood that a cut would remove (the distal part), as tubes slightly
    /// fatter than the wood mesh, for a highlight overlay. Same conventions
    /// as [`Self::wood_mesh`]; vertex colour is plain white.
    pub fn prune_preview_mesh(&self, node_id: u32, t: f32, flush: bool) -> MeshData {
        let Some(plan) = self.plan(CutSpec { node_id, t, flush }) else { return MeshData::default() };
        let nodes = &self.shoots.nodes;
        let ext = self.shoots.ext;
        let n = nodes.len();
        let mut remap = vec![NONE; n];
        let mut segs: Vec<TubeSeg> = Vec::new();
        for j in plan.idx..n {
            let nd = &nodes[j];
            let parent = if j == plan.idx {
                NONE
            } else {
                let p = nd.parent as usize;
                if p >= n || remap[p] == NONE {
                    continue;
                }
                remap[p]
            };
            let start = if j == plan.idx { plan.point } else { nd.start };
            let end = nd.end(ext);
            remap[j] = segs.len() as u32;
            segs.push(TubeSeg {
                parent,
                start,
                end,
                radius: self.draw_r(nd.radius()) * 1.12 + 0.003,
                continues: parent != NONE && nd.terminal,
                color: [1.0, 1.0, 1.0, 1.0],
                tip_taper: 0.6,
                path0: 0.0,
            });
        }
        mesh::build_tubes(&segs, 80_000, 0.0, TubeStyle::default())
    }

    /// Cut faces, woundwood rolls, decay, stubs and pollard knobs. Vertex
    /// colours carry the look (fresh wood, weathered wood, decay, bark-
    /// coloured woundwood); render double-sided. Same conventions as
    /// [`Self::wood_mesh`].
    pub fn wound_mesh(&self) -> MeshData {
        let mut m = MeshData::default();
        if !self.prune.active {
            return m;
        }
        let nodes = &self.shoots.nodes;
        let ext = self.shoots.ext;
        let map: HashMap<u32, usize> = nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
        let bark = self.genome.bark.color;
        // Woundwood and scars: smooth young bark, a lighter grey-brown than the
        // species' bark tint (display colours, sRGB).
        let callus_c = [bark[0] * 0.9 + 0.12, bark[1] * 0.9 + 0.12, bark[2] * 0.9 + 0.11, 1.0];
        let scar_c = [bark[0] * 0.8 + 0.14, bark[1] * 0.8 + 0.13, bark[2] * 0.8 + 0.12, 1.0];
        let year = self.year();
        let mut ends_done: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for w in &self.prune.wounds {
            if w.gone {
                continue;
            }
            let Some(&hi) = map.get(&w.host) else { continue };
            let h = &nodes[hi];
            let age = year.saturating_sub(w.year) as f32;
            let weather = (age / 4.0).min(1.0);
            let fresh = [0.68, 0.55, 0.36];
            let old = [0.42, 0.39, 0.35];
            let wood = [
                fresh[0] + (old[0] - fresh[0]) * weather,
                fresh[1] + (old[1] - fresh[1]) * weather,
                fresh[2] + (old[2] - fresh[2]) * weather,
                1.0,
            ];
            let heart = [wood[0] * 0.78, wood[1] * 0.70, wood[2] * 0.62, 1.0];
            let decay_c = [0.20, 0.14, 0.09, 1.0];
            let end = h.end(ext);
            let r_host_draw = self.draw_r(h.radius());
            // Side wounds: an oval on the host's surface, elongated along the
            // host (a branch leaving at an angle cuts an oblique ellipse).
            let (center, normal, rw, stretch) = if w.side {
                let hd = h.dir;
                let perp = (w.dir - hd * w.dir.dot(hd)).normalize_or(hd.any_perpendicular());
                let along = w.dir.dot(hd).clamp(-1.0, 1.0);
                let sin = (1.0 - along * along).sqrt();
                if sin < 0.5 {
                    // Continuation removed: an end face.
                    ends_done.insert(h.id);
                    (end, w.dir, self.draw_r(w.radius).min(r_host_draw), 1.0)
                } else {
                    (end + perp * r_host_draw * 0.97, perp, self.draw_r(w.radius).min(0.9 * r_host_draw), (1.0 / sin).min(1.6))
                }
            } else {
                ends_done.insert(h.id);
                (end, h.dir, r_host_draw, 1.0)
            };
            let closure = w.closure();
            let exposed = rw * (1.0 - closure);
            let lip = 0.12 * rw * (closure * 4.0).min(1.0);
            let decay_frac = if w.infected { (w.decay_r / w.core_r.max(1e-4)).min(1.0) } else { 0.0 };
            let mut rings: Vec<(f32, f32, [f32; 4])> = Vec::with_capacity(9);
            if !w.side && w.recuts > 0 && !h.is_snag() {
                // Pollard head: a bulbous knob of woundwood and bud clusters
                // that grows with each cycle (Read 2000).
                // Overgrown by the limbs once the pollard has lapsed.
                let lapse = (year.saturating_sub(w.last_recut_year) as f32 / 20.0).min(1.0);
                let k = 1.0 + 0.07 * (w.recuts as f32).min(6.0) * (1.0 - 0.7 * lapse);
                let dark = [bark[0] * 0.8, bark[1] * 0.78, bark[2] * 0.76, 1.0];
                for &(fr, fh, c) in &[(0.0, 0.5, dark), (0.55, 0.42, callus_c), (0.95, 0.18, callus_c), (1.12, -0.05, callus_c), (1.08, -0.3, callus_c), (0.9, -0.55, dark)] {
                    rings.push((fr * rw * k * (1.0 - 0.1 * lapse), fh * rw * k * (1.0 - 0.6 * lapse), c));
                }
            } else if h.is_snag() || self.prune.dead {
                // Dead stub / dead tree: weathered grey face, no woundwood.
                rings.push((0.0, 0.0, old_c(old)));
                rings.push((rw * 0.98, 0.0, old_c(old)));
                rings.push((rw * 1.02, -0.02 * rw, [0.56, 0.54, 0.50, 1.0]));
            } else if closure >= 1.0 {
                // Occluded: a bark-covered scar, a small knob on pollard heads.
                let knob = 1.0 + 0.18 * (w.recuts as f32).min(8.0);
                let dome = if w.side { 0.08 } else { 0.25 * knob };
                rings.push((0.0, dome * rw, scar_c));
                rings.push((0.5 * rw * knob, 0.7 * dome * rw, callus_c));
                rings.push((rw * knob, 0.0, scar_c));
            } else {
                let dr = decay_frac * exposed;
                rings.push((0.0, -0.01 * rw, if decay_frac > 0.0 { decay_c } else { heart }));
                if decay_frac > 0.0 {
                    rings.push((dr, -0.01 * rw, decay_c));
                }
                rings.push((dr.max(0.55 * exposed), 0.0, heart));
                rings.push((dr.max(0.6 * exposed), 0.0, wood));
                rings.push((exposed, 0.0, wood));
                rings.push((exposed + 0.02 * rw, lip, callus_c));
                rings.push((rw * 1.08, lip * 0.6, callus_c));
                let knob = 1.0 + 0.18 * (w.recuts as f32).min(8.0);
                rings.push((rw * 1.15 * knob, -0.02 * rw, scar_c));
            }
            push_disc(&mut m, center + normal * (0.003 + 0.01 * rw), normal, &rings, (w.id as u64) ^ 0xC0DE, h.dir, stretch);
        }
        // Other cut ends (stubs whose wound moved, broken tops): plain faces.
        for n in nodes.iter().filter(|n| n.cut_end && !ends_done.contains(&n.id)) {
            let rw = self.draw_r(n.radius());
            let c = if n.is_snag() { [0.52, 0.50, 0.46, 1.0] } else { [0.62, 0.55, 0.44, 1.0] };
            let rings = [(0.0, 0.0, c), (rw, 0.0, c)];
            push_disc(&mut m, n.end(ext) + n.dir * 0.003, n.dir, &rings, n.id as u64, n.dir, 1.0);
        }
        m
    }

    /// Position (tree coordinates) of each journal entry's wound, `None` when it is gone.
    pub fn wound_positions(&self) -> Vec<Option<[f32; 3]>> {
        let ext = self.shoots.ext;
        self.prune
            .records
            .iter()
            .map(|r| {
                let w = self.prune.wounds.iter().find(|w| w.id == r.wound)?;
                if w.gone {
                    return None;
                }
                let h = &self.shoots.nodes[self.shoots.index_of(w.host)?];
                let mut p = h.end(ext);
                if w.side {
                    let perp = (w.dir - h.dir * w.dir.dot(h.dir)).normalize_or(Vec3::ZERO);
                    p += perp * self.draw_r(h.radius());
                }
                Some(p.to_array())
            })
            .collect()
    }

    /// The cut journal (cuts and natural breakages), oldest first.
    pub fn prune_journal(&self) -> Vec<JournalEntry> {
        self.prune
            .records
            .iter()
            .map(|r| {
                let w = self.prune.wounds.iter().find(|w| w.id == r.wound);
                let (state, closure, decay, pathogen) = match w {
                    Some(w) => (w.state(), w.closure(), if w.infected { (w.decay_r / w.core_r.max(1e-4)).min(1.0) } else { 0.0 }, w.pathogen_done),
                    None => (WoundState::Gone, 0.0, 0.0, false),
                };
                JournalEntry { day: r.day, kind: r.kind, diameter_cm: r.diameter_cm, foliage_share: r.foliage_share, state, closure, decay, pathogen, natural: r.natural }
            })
            .collect()
    }

    /// Is the tree alive (some foliage or a bud on living wood)?
    pub fn is_alive(&self) -> bool {
        !self.prune.dead && self.shoots.nodes.iter().any(|n| !n.is_snag() && (n.leafy || n.term_bud || n.lat_buds > 0))
    }

    /// Estimated total lifespan (years). Heuristic: the species' typical
    /// lifespan, shortened by low reserves, by decay weakening the stem
    /// (t/R) and by stem pathogens. Not a validated prediction.
    pub fn life_expectancy_years(&self) -> f32 {
        let tr = &self.genome.pruning;
        let age = self.age_years();
        if !self.is_alive() {
            return age;
        }
        if !self.prune.active {
            return tr.typical_lifespan_years.max(age);
        }
        let map: HashMap<u32, usize> = self.shoots.nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
        let mut structural = 1.0f32;
        let mut pathogen = 1.0f32;
        for w in self.prune.wounds.iter().filter(|w| !w.gone) {
            let Some(&hi) = map.get(&w.host) else { continue };
            let h = &self.shoots.nodes[hi];
            if h.order == 0 {
                if !w.side && w.closed_year == NONE {
                    // A large open cut on the stem invites decay into the trunk.
                    structural *= 1.0 - 0.15 * (w.radius / 0.1).min(1.0);
                }
                if w.decay_r > 0.0 {
                    let t_r = (h.radius() - w.decay_r) / h.radius().max(1e-4);
                    structural = structural.min(0.3 + 0.7 * smooth((t_r - 0.2) / 0.4));
                }
                if w.pathogen_done {
                    pathogen *= 0.6;
                }
            }
        }
        let health = (0.25 + 0.75 * smooth(self.prune.reserve / 0.6)) * structural * pathogen / (1.0 + 1.5 * self.prune.injury);
        let remaining = (tr.typical_lifespan_years - age).max(0.0) * health;
        age + remaining
    }

    /// Comparison / objective measurements.
    pub fn prune_metrics(&self) -> PruneMetrics {
        let mut m = PruneMetrics {
            age_years: self.age_years(),
            height_m: self.height_m(),
            crown_radius_m: self.crown_radius_m(),
            dbh_cm: self.dbh_cm(),
            leaf_area_m2: self.shoots.leaf_area_proxy(),
            reserve: if self.prune.active { self.prune.reserve } else { 1.0 },
            vigour: if self.prune.active { self.prune.last_vigour } else { 1.0 },
            new_shoots: self.prune.last_new_shoots,
            fine_root_loss: self.prune.fine_root_loss,
            life_expectancy_years: self.life_expectancy_years(),
            alive: self.is_alive(),
            ..Default::default()
        };
        let nodes = &self.shoots.nodes;
        let first = self.prune.records.first().map_or(u32::MAX, |r| (r.day / DAYS_PER_YEAR as u64) as u32);
        m.sprouts = nodes.iter().skip(1).filter(|n| n.birth_year >= first && !n.is_snag() && nodes[n.parent as usize].wounded && n.leaf_n + if n.leafy { 1.0 } else { 0.0 } > 0.0).count() as u32;
        for w in self.prune.wounds.iter().filter(|w| !w.gone) {
            match w.state() {
                WoundState::Closed => m.closed_wounds += 1,
                WoundState::Decayed => m.decayed_wounds += 1,
                _ => {
                    if w.closed_year == NONE {
                        m.open_wounds += 1
                    } else {
                        m.closed_wounds += 1
                    }
                }
            }
            if w.infected {
                m.decay_volume_m3 += std::f32::consts::PI * w.decay_r * w.decay_r * (w.decay_len + 2.0 * w.decay_r);
            }
        }
        m
    }

    /// Stem quality metrics (timber and pollard objectives).
    pub fn bole_metrics(&self) -> BoleMetrics {
        let nodes = &self.shoots.nodes;
        let n = nodes.len();
        let mut on_stem = vec![false; n];
        on_stem[0] = true;
        for i in 1..n {
            let p = nodes[i].parent as usize;
            on_stem[i] = on_stem[p] && nodes[i].terminal && nodes[i].order == 0;
        }
        // Lowest live branch on the stem.
        let mut clear = self.height_m();
        for i in 1..n {
            let nd = &nodes[i];
            if on_stem[nd.parent as usize] && !on_stem[i] && !nd.is_snag() && nd.leaf_n > 0.0 {
                clear = clear.min(nd.start.y);
            }
        }
        let map: HashMap<u32, usize> = nodes.iter().enumerate().map(|(i, nd)| (nd.id, i)).collect();
        let mut b = BoleMetrics { clear_bole_m: clear, knot_free_bole_m: clear, ..Default::default() };
        let mut core = 0.0f32;
        for w in self.prune.wounds.iter().filter(|w| !w.gone) {
            let Some(&hi) = map.get(&w.host) else { continue };
            let h = &nodes[hi];
            if !w.side && h.cut_end && h.order == 0 {
                let kids = nodes.iter().skip(hi + 1).any(|c| c.parent as usize == hi && !c.is_snag());
                if kids && !h.is_snag() {
                    b.pollard_heads += 1;
                    b.pollard_max_recuts = b.pollard_max_recuts.max(w.recuts as u32);
                }
            }
            if !on_stem[hi] || w.y > clear {
                continue;
            }
            if w.closed_year == NONE {
                b.open_wounds += 1;
                b.knot_free_bole_m = b.knot_free_bole_m.min(w.y);
                core = core.max(h.radius());
            } else {
                core = core.max(w.r_at_close);
            }
        }
        b.knotty_core_cm = 200.0 * core;
        let r13 = self.dbh_cm() * 0.005;
        b.clear_wood_mm = if core > 0.0 { ((r13 - core) * 1000.0).max(0.0) } else { r13 * 1000.0 };
        b
    }

    // ------------------------------------------------------------------
    // High-level operations (each expands into logged atomic cuts)
    // ------------------------------------------------------------------

    /// Ids of the stem nodes (collar to top, following terminal children).
    fn stem_path(&self) -> Vec<usize> {
        let nodes = &self.shoots.nodes;
        let mut on = vec![false; nodes.len()];
        on[0] = true;
        let mut out = vec![0usize];
        for i in 1..nodes.len() {
            let p = nodes[i].parent as usize;
            if on[p] && nodes[i].terminal && nodes[i].order == 0 {
                on[i] = true;
                out.push(i);
            }
        }
        out
    }

    /// Crown raising: remove every live branch leaving the stem below
    /// `height_m`, at its collar. Returns the number of cuts.
    pub fn raise_crown(&mut self, height_m: f32) -> usize {
        let stem = self.stem_path();
        let nodes = &self.shoots.nodes;
        let mut on = vec![false; nodes.len()];
        for &i in &stem {
            on[i] = true;
        }
        let ids: Vec<u32> = nodes
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(i, nd)| on[nd.parent as usize] && !on[*i] && nd.start.y < height_m)
            .map(|(_, nd)| nd.id)
            .collect();
        ids.into_iter().filter(|&id| self.cut(id, 0.0, false).is_some()).count()
    }

    /// Cut the stem at `height_m` (topping / pollarding / coppicing).
    pub fn top_at(&mut self, height_m: f32) -> Option<CutKind> {
        let stem = self.stem_path();
        let nodes = &self.shoots.nodes;
        let ext = self.shoots.ext;
        let &i = stem.iter().find(|&&i| nodes[i].end(ext).y >= height_m)?;
        let n = &nodes[i];
        let vis = (n.end(ext) - n.start).length().max(1e-4);
        let t = ((height_m - n.start.y) / (n.dir.y.max(0.05) * vis)).clamp(0.0, 1.0);
        let id = n.id;
        self.cut(id, t, false)
    }

    /// Coppice: cut the stem 10 cm above the ground.
    pub fn coppice(&mut self) -> Option<CutKind> {
        let ext = self.shoots.ext;
        let n = &self.shoots.nodes[0];
        let vis = (n.end(ext) - n.start).length().max(1e-4);
        let id = n.id;
        self.cut(id, (0.1 / vis).min(1.0), false)
    }

    /// Pollard cycle: cut every shoot growing on the cut stem ends (heads)
    /// and on the bole below them, at their base. Returns the number of cuts.
    pub fn pollard_recut(&mut self) -> usize {
        let nodes = &self.shoots.nodes;
        let ext = self.shoots.ext;
        let stem = self.stem_path();
        let mut on = vec![false; nodes.len()];
        for &i in &stem {
            on[i] = true;
        }
        let heads: Vec<usize> = self
            .prune
            .wounds
            .iter()
            .filter(|w| !w.side && !w.gone)
            .filter_map(|w| self.shoots.index_of(w.host))
            .filter(|&i| nodes[i].cut_end && nodes[i].order == 0)
            .collect();
        let Some(top) = heads.iter().map(|&i| nodes[i].end(ext).y).reduce(f32::max) else { return 0 };
        let ids: Vec<u32> = nodes
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(i, c)| {
                let p = c.parent as usize;
                !c.is_snag() && (heads.contains(&p) || (on[p] && !on[*i] && c.start.y <= top + 0.01))
            })
            .map(|(_, c)| c.id)
            .collect();
        ids.into_iter().filter(|&id| self.cut(id, 0.0, false).is_some()).count()
    }

    /// Crown thinning: remove about `fraction` of the secondary branches
    /// (1-6 cm across, not on the stem) at their collar, chosen
    /// deterministically. Returns the number of cuts.
    pub fn thin_crown(&mut self, fraction: f32) -> usize {
        let stem = self.stem_path();
        let nodes = &self.shoots.nodes;
        let mut on = vec![false; nodes.len()];
        for &i in &stem {
            on[i] = true;
        }
        let day = self.day;
        let seed = self.seed;
        let ids: Vec<u32> = nodes
            .iter()
            .skip(1)
            .filter(|nd| {
                let r = nd.radius();
                !on[nd.parent as usize] && !nd.terminal && !nd.is_snag() && (0.005..0.03).contains(&r) && rng::unit(rng::hash(seed, S_PRUNE, nd.id as u64, day ^ 0x7417)) < fraction
            })
            .map(|nd| nd.id)
            .collect();
        ids.into_iter().filter(|&id| self.cut(id, 0.0, false).is_some()).count()
    }

    /// Crown reduction: shorten every branch reaching beyond `1 - fraction`
    /// of the crown radius, where it crosses that radius. Returns the
    /// number of cuts.
    pub fn reduce_crown(&mut self, fraction: f32) -> usize {
        let limit = self.crown_radius_m() * (1.0 - fraction.clamp(0.0, 0.8));
        let ext = self.shoots.ext;
        let nodes = &self.shoots.nodes;
        let mut cuts: Vec<(u32, f32)> = Vec::new();
        let mut cut_below = vec![false; nodes.len()];
        for (i, nd) in nodes.iter().enumerate() {
            if i > 0 && cut_below[nd.parent as usize] {
                cut_below[i] = true;
                continue;
            }
            if nd.order == 0 || nd.is_snag() {
                continue;
            }
            let (a, b) = (nd.start.horizontal_len(), nd.end(ext).horizontal_len());
            if a < limit && b >= limit {
                let t = ((limit - a) / (b - a).max(1e-4)).clamp(0.05, 1.0);
                cuts.push((nd.id, t));
                cut_below[i] = true;
            }
        }
        cuts.into_iter().filter(|&(id, t)| self.cut(id, t, false).is_some()).count()
    }
}

fn old_c(c: [f32; 3]) -> [f32; 4] {
    [c[0], c[1], c[2], 1.0]
}

/// A flat (or domed) disc of concentric rings `(radius, height along the
/// normal, colour)` around `center`; clockwise seen from the normal side.
#[allow(clippy::too_many_arguments)]
fn push_disc(m: &mut MeshData, center: Vec3, normal: Vec3, rings: &[(f32, f32, [f32; 4])], key: u64, axis: Vec3, stretch: f32) {
    const SEG: usize = 14;
    let nrm = normal.normalize_or(Vec3::Y);
    let u = nrm.any_perpendicular();
    let v = nrm.cross(u);
    let phase = rng::unit(rng::mix(key)) * std::f32::consts::TAU;
    let base = m.positions.len() as u32;
    for (k, &(r, hgt, c)) in rings.iter().enumerate() {
        // Slope of the dome for the normal.
        let slope = if k + 1 < rings.len() {
            let (r2, h2, _) = rings[k + 1];
            (hgt - h2) / (r2 - r).abs().max(1e-4)
        } else {
            0.0
        };
        for j in 0..=SEG {
            let th = phase + std::f32::consts::TAU * j as f32 / SEG as f32;
            let (s, co) = th.sin_cos();
            let radial = u * co + v * s;
            // Slightly irregular outline.
            let wob = 1.0 + 0.06 * (3.0 * th + phase).sin();
            // Stretch the outline along `axis` (projected into the disc plane).
            let ax = (axis - nrm * axis.dot(nrm)).normalize_or(u);
            let radial_s = radial + ax * (radial.dot(ax) * (stretch - 1.0));
            let p = center + radial_s * (r * wob) + nrm * hgt;
            let nn = (nrm + radial * slope.clamp(-2.0, 2.0)).normalize_or(nrm);
            m.positions.push(p.to_array());
            m.normals.push(nn.to_array());
            m.uvs.push([j as f32 / SEG as f32, k as f32]);
            m.colors.push(c);
        }
    }
    let row = SEG as u32 + 1;
    for k in 0..rings.len().saturating_sub(1) as u32 {
        for j in 0..SEG as u32 {
            let i0 = base + k * row + j;
            let i1 = i0 + 1;
            let o0 = i0 + row;
            let o1 = o0 + 1;
            m.indices.extend_from_slice(&[i0, o1, o0, i0, i1, o1]);
        }
    }
}

// Integer codes for the front-end (see `CutKind::code`).
impl From<u8> for CutKind {
    fn from(c: u8) -> Self {
        CutKind::from_code(c)
    }
}

/// Days per year (re-export for front-ends composing `LogEvent` days).
pub const PRUNE_DAYS_PER_YEAR: u64 = DAYS_PER_YEAR as u64;

/// Total stem section defects at height `y` (m): decayed radius fraction
/// of the stem there (0..1), for the cross-section view.
impl Simulation {
    pub fn stem_decay_at(&self, y: f32) -> f32 {
        if !self.prune.active {
            return 0.0;
        }
        let Some(si) = self.shoots.trunk_node_at(y) else { return 0.0 };
        let r = self.shoots.nodes[si].radius().max(1e-4);
        let map: HashMap<u32, usize> = self.shoots.nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
        let mut best = 0.0f32;
        for w in self.prune.wounds.iter().filter(|w| w.infected && !w.gone) {
            let Some(&hi) = map.get(&w.host) else { continue };
            if self.shoots.nodes[hi].order != 0 {
                continue;
            }
            if (w.y - y).abs() <= w.decay_len + w.decay_r {
                best = best.max(w.decay_r / r);
            }
        }
        best.min(1.0)
    }

    /// Pruning knots in the stem section at height `y`: `(year of the cut,
    /// stem radius when cut (m), year closed or u32::MAX)` for wounds on the
    /// stem within one stem radius of `y`.
    pub fn stem_knots_at(&self, y: f32) -> Vec<(u32, f32, u32)> {
        let mut out = Vec::new();
        if !self.prune.active {
            return out;
        }
        let map: HashMap<u32, usize> = self.shoots.nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
        for w in self.prune.wounds.iter().filter(|w| w.side) {
            let Some(&hi) = map.get(&w.host) else { continue };
            let h = &self.shoots.nodes[hi];
            if h.order == 0 && (w.y - y).abs() < 1.0 {
                out.push((w.year, w.host_r0, if w.closed_year == NONE { u32::MAX } else { w.closed_year }));
            }
        }
        out
    }
}
