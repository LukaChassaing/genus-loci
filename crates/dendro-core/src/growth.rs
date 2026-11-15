//! Above-ground shoot system: bud-based growth with light competition.
//!
//! Model, after Palubicki et al. 2009 ("Self-organizing tree models"):
//! * The tree is a graph of **annual shoots** (one node = one year's
//!   extension of an axis, i.e. a lumped chain of metamers). Oak growth is
//!   rhythmic and largely preformed with buds clustered at the shoot tip, so
//!   one node per annual shoot is a reasonable lumping for young trees.
//! * Light: voxel shadow propagation from the leafy shoots, swept along five
//!   sky directions (see [`ShootSystem::compute_light`]); a shoot's exposure
//!   `Q` (0..1) also includes crowding by the shoots above it in its voxel.
//! * Vigour: Borchert-Honda allocation. Light is summed basipetally, the
//!   resulting vigour `alpha * sum(Q)` is distributed acropetally with apical
//!   control lambda between main axis and laterals.
//! * Bud fate: a bud breaks if its vigour exceeds a threshold; shoot length is
//!   proportional to vigour, capped, and reduced with the collar-to-bud
//!   distance (size-related decline of extension growth, Ryan & Yoder 1997).
//!   A lit terminal bud with too little vigour still makes a short shoot (oak
//!   brachyblast), so weak branches persist instead of dying at once.
//! * Shedding (self-pruning): a branch dies when none of its foliage is
//!   adequately lit, or when it carries no foliage at all.
//! * Secondary growth (Pressler's law / pipe model): each year, the
//!   cross-section increment of a segment is proportional to the
//!   (light-weighted) foliage it supports, scaled by the year's productivity.
//!   This yields the pipe model (Shinozaki et al. 1964) as a consequence while
//!   letting climate modulate ring width.
//!
//! Level of detail (a stylisation, not biology): the voxel size scales with
//! tree height, and a leafy shoot unit shorter than ~a voxel is extended in
//! place for up to three years before it branches. Young trees therefore
//! branch every year; on an old tree one node stands for a few years of twig
//! growth, so the node count stays bounded (~10-40k) and the sub-voxel twig
//! detail is implied by the foliage clusters.
//!
//! Crown architecture (see the `Architecture` traits; mostly stylised and
//! calibrated by eye, the concepts are from the literature):
//! * Apical control declines with age (juvenile plateau, then smooth decline
//!   below 0.5): excurrent sapling, decurrent mature crown.
//! * Sibling laterals compete for vigour with a sharpness exponent
//!   (`lateral_competition`), so a few scaffold limbs dominate.
//! * Lateral axes are corrected towards a gravitropic set-point angle
//!   (Digby & Firn 1995) that depends on the limb's insertion height; limb
//!   tips turn up at the crown edge.
//! * Terminal-bud loss with relay by a subterminal bud makes persistent
//!   "elbows"; simplification (`compact`) keeps them.
//! * Natural pruning of the lower stem (clear bole) and a bounded, locked-in
//!   sag of heavy inclined limbs (`sag`).

use crate::genome::Genome;
use crate::math::{Quat, Vec3};
use crate::rng::{self, S_BUD, S_RETRENCH, S_SHOOT};
use std::f32::consts::PI;

pub(crate) const NONE: u32 = u32::MAX;

/// Optical depth added by one leafy shoot unit of reference leaf area in
/// its voxel (Beer-Lambert: `tau = G * LAD * path`; with the level of
/// detail a leafy unit carries leaf area proportional to a voxel face, so
/// the per-unit value is size-independent).
const SHADOW_A: f32 = 2.5;
/// Weight of the shadow cast into a shoot's own voxel (side-by-side crowding).
/// Kept small: a voxel cannot resolve the horizontal arrangement of foliage,
/// and on old trees one voxel holds a lumped shoot cluster.
const SHADOW_SAME: f32 = 0.2;
/// Fraction of optical depth carried per voxel traversed (penumbra /
/// scattering: a shadow fades a few voxels away from its source). Oblique
/// rays traverse `path` voxels per layer and decay by `SHADOW_DECAY^path`.
const SHADOW_DECAY: f32 = 0.82;
/// Lateral blur weights (centre, each edge) of the separable 3-tap kernel.
const BLUR_C: f32 = 0.5;
const BLUR_E: f32 = 0.25;
/// Sky hemisphere: horizontal voxel shift per layer (x, z) and weight.
///
/// 13 directions in three elevation bands: the zenith (90 deg), 4 at 45
/// deg (shift 1) and 8 at ~24 deg (shift (2, 1), azimuths 26.6 deg apart
/// from the axes). Weights are the standard
/// overcast sky (radiance ~ (1 + 2 sin e) / 3, Moon & Spencer 1942)
/// integrated over the bands 60-90 / 35-60 / 10-35 deg: 0.21 / 0.40 / 0.39.
/// Open-grown trees receive most of their diffuse light from these low and
/// middle elevations, which is what keeps their crowns deep (Canham et al.
/// 1994 for side light in gaps; Valladares & Niinemets 2007).
const SKY_DIRS: [(i64, i64, f32); 13] = [
    (0, 0, 0.21),
    (1, 0, 0.10),
    (-1, 0, 0.10),
    (0, 1, 0.10),
    (0, -1, 0.10),
    (2, 1, 0.04875),
    (1, 2, 0.04875),
    (-1, 2, 0.04875),
    (-2, 1, 0.04875),
    (-2, -1, 0.04875),
    (-1, -2, 0.04875),
    (1, -2, 0.04875),
    (2, -1, 0.04875),
];
/// Planes are padded by this many cells (zero shadow = open sky): the
/// largest per-layer shift plus the blur radius.
const PAD: usize = 3;
/// Voxel size = tree height / this (bounded): the simulation's level of detail.
const VOXELS_PER_HEIGHT: f32 = 30.0;
/// Soft cap on shoot nodes: above it bud break gets harder (safety valve).
const NODE_SOFT_CAP: usize = 32_000;

#[derive(Clone, Debug)]
pub(crate) struct Node {
    /// Stable id (survives compaction), used for hashing.
    pub id: u32,
    pub parent: u32,
    pub start: Vec3,
    pub dir: Vec3,
    pub length: f32,
    /// Length before this year's extension (shoot units extended in place).
    pub grow_from: f32,
    /// Cross-section area at the base (m2).
    pub area: f32,
    /// Hydraulic path length from the collar to the end (m).
    pub path_end: f32,
    pub order: u8,
    /// Height (m) at which the first-order limb carrying this node leaves
    /// the stem (0 on the stem itself).
    pub limb_base_y: f32,
    /// Cumulative sag rotation of this segment (rad).
    pub sag: f32,
    /// Number of annual extensions lumped into this (LOD) shoot unit.
    pub unit_years: u8,
    /// Continues the parent's axis (from the terminal bud).
    pub terminal: bool,
    /// Current (last flushed) annual shoot: bears the leaves.
    pub leafy: bool,
    pub term_bud: bool,
    pub lat_buds: u8,
    pub birth_year: u32,
    /// Consecutive years in which this branch's foliage was all below the
    /// shedding light threshold (branch carbon reserves, see `mark_shed`).
    pub starve: u8,
    /// Year this branch died and became standing dead wood (retrenchment),
    /// `NONE` while alive. Dead wood keeps its geometry, carries no buds or
    /// leaves and breaks off piecewise (thin parts first).
    pub snag_since: u32,
    /// Year the current lateral buds were formed (epicormic buds are formed
    /// on old wood; bud viability counts from here).
    pub bud_year: u32,
    /// Light exposure (0..1) at the shoot tip, from the last light pass.
    pub light: f32,
    /// Direction of decreasing shadow at the tip.
    pub grad: Vec3,
    // --- scratch, recomputed every pass ---
    q_main: f32,
    q_lat: f32,
    /// Sum of `Q^k` over lateral children and dormant lateral buds.
    s_lat: f32,
    pub leaf_q: f32,
    pub leaf_n: f32,
    leaf_max: f32,
    v_main: f32,
    v_lat: f32,
    pub foliage: f32,
    dead: bool,
}

/// Fast `exp(-x)` for `x >= 0` (relative error < 2e-4): the light pass
/// evaluates it ~10^6 times per year. Deterministic (pure f32 arithmetic).
#[inline]
fn exp_neg(x: f32) -> f32 {
    let t = -x.clamp(0.0, 80.0) * std::f32::consts::LOG2_E;
    let fi = t.floor();
    let f = t - fi;
    // 2^f on [0, 1): minimax-like cubic.
    let p = 1.0 + f * (0.695_976_1 + f * (0.224_940_4 + f * 0.079_083_5));
    p * f32::from_bits((((fi as i32) + 127).max(0) as u32) << 23)
}

/// `x^k`, with fast paths for the small integer exponents used in genomes.
#[inline]
fn pow_k(x: f32, k: f32) -> f32 {
    if k == 3.0 {
        x * x * x
    } else if k == 2.0 {
        x * x
    } else if k == 1.0 {
        x
    } else {
        x.powf(k)
    }
}

/// Leaf area of a leafy shoot unit relative to the reference (a full-length
/// unit of about a voxel): short, weak units carry fewer leaves.
#[inline]
fn leaf_units(n: &Node, unit_len: f32) -> f32 {
    0.5 + 0.5 * (n.length / unit_len.max(1e-3)).min(1.0)
}

impl Node {
    #[inline]
    pub fn end(&self, ext: f32) -> Vec3 {
        let l = if self.leafy { self.grow_from + (self.length - self.grow_from) * ext } else { self.length };
        self.start + self.dir * l
    }
    #[inline]
    pub fn radius(&self) -> f32 {
        (self.area / PI).sqrt()
    }
    #[inline]
    pub fn is_snag(&self) -> bool {
        self.snag_since != NONE
    }
}

/// Years a piece of standing dead wood of radius `r` (m) persists before it
/// breaks off: twigs within a few years, boughs in decades for durable
/// heartwood (stylised; `WoodTraits::snag_years_per_m`, 500 for oak, whose
/// dead limbs stand for decades).
#[inline]
fn snag_life_years(r: f32, years_per_m: f32) -> u32 {
    (2.0 + years_per_m * r) as u32
}

#[derive(Clone, Debug)]
pub(crate) struct ShootSystem {
    pub nodes: Vec<Node>,
    next_id: u32,
    /// Extension 0..1 of the leafy (current-year) shoots.
    pub ext: f32,
    /// Voxel size (m) of the last light pass.
    pub voxel: f32,
    break_boost: f32,
    /// Tree height at the last flush (m).
    tree_h: f32,
    grid: Vec<f32>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FlushStats {
    pub new_shoots: usize,
    pub shed: usize,
    /// Boughs killed by crown retrenchment this year.
    pub retrenched: usize,
}

impl ShootSystem {
    pub fn new(g: &Genome) -> Self {
        let r = g.wood.twig_radius_mm * 0.001 * 0.6;
        let seedling = Node {
            id: 0,
            parent: NONE,
            start: Vec3::ZERO,
            dir: Vec3::Y,
            length: 0.03,
            grow_from: 0.0,
            area: PI * r * r,
            path_end: 0.03,
            order: 0,
            limb_base_y: 0.0,
            sag: 0.0,
            unit_years: 1,
            terminal: true,
            leafy: false,
            term_bud: true,
            lat_buds: 0,
            birth_year: 0,
            starve: 0,
            snag_since: NONE,
            bud_year: 0,
            light: 1.0,
            grad: Vec3::ZERO,
            q_main: 0.0,
            q_lat: 0.0,
            s_lat: 0.0,
            leaf_q: 0.0,
            leaf_n: 0.0,
            leaf_max: 0.0,
            v_main: 0.0,
            v_lat: 0.0,
            foliage: 0.0,
            dead: false,
        };
        ShootSystem { nodes: vec![seedling], next_id: 1, ext: 1.0, voxel: 0.1, break_boost: 1.0, tree_h: 0.03, grid: Vec::new() }
    }

    /// Basipetal sums of bud light (main/lateral) and of leafy-shoot light.
    fn accumulate(&mut self, k: f32) {
        for n in self.nodes.iter_mut() {
            n.q_main = if n.term_bud { n.light } else { 0.0 };
            n.q_lat = n.lat_buds as f32 * n.light;
            n.s_lat = n.lat_buds as f32 * pow_k(n.light, k);
            n.leaf_q = if n.leafy { n.light } else { 0.0 };
            n.leaf_n = if n.leafy { 1.0 } else { 0.0 };
            n.leaf_max = n.leaf_q;
        }
        for i in (1..self.nodes.len()).rev() {
            let n = &self.nodes[i];
            let (p, term, q, lq, ln, lm) = (n.parent as usize, n.terminal, n.q_main + n.q_lat, n.leaf_q, n.leaf_n, n.leaf_max);
            let pn = &mut self.nodes[p];
            if term {
                pn.q_main += q;
            } else {
                pn.q_lat += q;
                pn.s_lat += pow_k(q, k);
            }
            pn.leaf_q += lq;
            pn.leaf_n += ln;
            pn.leaf_max = pn.leaf_max.max(lm);
        }
    }

    /// Mark poorly lit or leafless branches for removal.
    ///
    /// A branch survives `survival` years of insufficient light on its
    /// reserves before it dies (branch autonomy, Sprugel et al. 1991;
    /// shaded oak branches are not shed within a single season), so one
    /// dark year or a transient neighbour does not strip the lower crown.
    fn mark_shed(&mut self, threshold: f32, mean_thr: f32, year: u32, bole: (f32, f32), survival: u8, snag_k: f32) -> usize {
        let mut count = 0;
        self.nodes[0].dead = false;
        let mut on_stem = vec![false; self.nodes.len()];
        on_stem[0] = true;
        let (bole_h, bole_ratio) = bole;
        // Live nodes carrying standing dead wood become dead wood themselves
        // when they lose their last leaves (instead of vanishing with it).
        let len = self.nodes.len();
        let mut has_snag = vec![false; len];
        for i in (1..len).rev() {
            let n = &self.nodes[i];
            if n.is_snag() || has_snag[i] {
                has_snag[n.parent as usize] = true;
            }
        }
        for i in 1..self.nodes.len() {
            let p = self.nodes[i].parent as usize;
            let n = &self.nodes[i];
            on_stem[i] = on_stem[p] && n.terminal;
            let dead = if self.nodes[p].dead {
                true
            } else if n.is_snag() {
                year > n.snag_since + snag_life_years(n.radius(), snag_k)
            } else if n.leaf_n <= 0.0 && has_snag[i] && year > n.birth_year + 1 {
                self.nodes[i].snag_since = year;
                self.nodes[i].lat_buds = 0;
                self.nodes[i].term_bud = false;
                false
            } else if on_stem[p] && !n.terminal && n.start.y < bole_h && n.area < bole_ratio * self.nodes[p].area {
                // Stylised natural pruning of the lower stem (clear bole).
                true
            } else if n.leaf_n > 0.0 {
                // Hierarchical self-pruning: a branch dies when none of its
                // foliage is adequately lit (shaded twigs go first, whole
                // limbs only when entirely overtopped).
                let starving = n.order > 0 && (n.leaf_max < threshold || n.leaf_q < mean_thr * n.leaf_n);
                let st = if starving { n.starve.saturating_add(1) } else { 0 };
                self.nodes[i].starve = st;
                st >= survival.max(1)
            } else {
                year > n.birth_year + 1
            };
            self.nodes[i].dead = dead;
            if dead {
                count += 1;
            }
        }
        count
    }

    /// Remove dead nodes and merge old, bud-less, nearly collinear chains so
    /// the node count tracks branch length rather than age. Preserves the
    /// invariant `parent index < child index`.
    fn compact(&mut self, year: u32, max_seg: f32) {
        let n = self.nodes.len();
        let mut child_count = vec![0u16; n];
        for i in 1..n {
            if !self.nodes[i].dead {
                child_count[self.nodes[i].parent as usize] += 1;
            }
        }
        let mut remap = vec![NONE; n];
        let mut out: Vec<Node> = Vec::with_capacity(n);
        for i in 0..n {
            if self.nodes[i].dead {
                continue;
            }
            let mut node = self.nodes[i].clone();
            if i == 0 {
                remap[0] = 0;
                out.push(node);
                continue;
            }
            let op = node.parent as usize;
            let np = remap[op] as usize;
            let par = &out[np];
            let mergeable = op != 0
                && child_count[op] == 1
                && node.terminal
                && !node.leafy
                && !par.leafy
                && !par.term_bud
                && par.lat_buds == 0
                && par.is_snag() == node.is_snag()
                && year >= node.birth_year + 4
                && par.length + node.length <= max_seg
                && {
                    // Merge only if the joint lies close to the chord, so
                    // the elbows of sinuous limbs survive simplification.
                    let chord = node.end(1.0) - par.start;
                    let cl = chord.length();
                    let j = par.start + par.dir * par.length - par.start;
                    let t = if cl > 1e-6 { j.dot(chord) / (cl * cl) } else { 0.0 };
                    (j - chord * t).length() <= 0.06 * cl + 0.005
                };
            if mergeable {
                let start = par.start;
                let end = node.end(1.0);
                let d = end - start;
                let len = d.length();
                node.parent = par.parent;
                node.start = start;
                node.dir = d.normalize_or(par.dir);
                node.length = len;
                node.area = par.area.max(node.area);
                node.order = par.order;
                node.terminal = par.terminal;
                out[np] = node;
                remap[i] = np as u32;
            } else {
                node.parent = np as u32;
                remap[i] = out.len() as u32;
                out.push(node);
            }
        }
        self.nodes = out;
    }

    /// Late-life crown retrenchment and reiteration (stylised; concepts from
    /// the veteran-tree literature: Lonsdale 2013 "Ancient and other
    /// veteran trees", Read 2000; hydraulic basis: Ryan & Yoder 1997).
    ///
    /// * **Dieback**: each year after `retrenchment_onset_years`, every
    ///   bough (a branch at least half as thin as its parent, 2-12 cm
    ///   radius, based in the outer crown) whose farthest shoot lies beyond `retrenchment_reach` x
    ///   `hydraulic_path_max_m` from the collar dies with a probability
    ///   rising from 0 at onset to `retrenchment_rate` after
    ///   `retrenchment_ramp_years`. It
    ///   becomes standing dead wood (stag-headed top, dead limbs), whose
    ///   thin parts break off first (`snag_life_years`).
    /// * **Reiteration**: live limbs thicker than 5 cm in the lower 65 % of
    ///   the crown, with good light, form epicormic buds with probability
    ///   `epicormic_rate` per year, so a lower inner crown rebuilds.
    ///
    /// Returns the number of boughs killed.
    fn retrench(&mut self, g: &Genome, year: u32, age: f32, seed: u64) -> usize {
        let a = &g.architecture;
        let n = self.nodes.len();
        // Farthest live shoot-tip distance from the collar in each subtree.
        let mut reach = vec![0.0f32; n];
        for i in (0..n).rev() {
            let nd = &self.nodes[i];
            if !nd.is_snag() {
                reach[i] = reach[i].max(nd.end(1.0).length());
            }
            if i > 0 {
                let p = nd.parent as usize;
                reach[p] = reach[p].max(reach[i]);
            }
        }
        let ramp = ((age - a.retrenchment_onset_years) / a.retrenchment_ramp_years.max(1.0)).clamp(0.0, 1.0);
        let p_die = a.retrenchment_rate * ramp;
        let limit = a.retrenchment_reach * a.hydraulic_path_max_m;
        let h = self.height().max(1.0);
        let mut killed = 0usize;
        let mut kill = vec![false; n];
        for i in 1..n {
            let p = self.nodes[i].parent as usize;
            if kill[p] {
                kill[i] = true;
                continue;
            }
            let nd = &self.nodes[i];
            if nd.is_snag() {
                continue;
            }
            let r = nd.radius();
            // Boughs of the outer/upper crown only: the base itself must be
            // far out, so dieback takes the top and the limb ends, not the
            // scaffold limbs at their base.
            let bough = nd.order > 0
                && (0.02..0.12).contains(&r)
                && self.nodes[p].area >= 2.0 * nd.area
                && nd.start.length() > 0.6 * limit;
            if bough && reach[i] > limit && rng::unit(rng::hash(seed, S_RETRENCH, nd.id as u64, year as u64)) < p_die {
                kill[i] = true;
                killed += 1;
            }
        }
        for (nd, &k) in self.nodes.iter_mut().zip(&kill).skip(1) {
            if k {
                nd.snag_since = year;
                nd.leafy = false;
                nd.term_bud = false;
                nd.lat_buds = 0;
            } else if !nd.is_snag()
                && a.epicormic_rate > 0.0
                && nd.order > 0
                && nd.lat_buds == 0
                && nd.radius() >= 0.05
                && nd.end(1.0).y < 0.65 * h
                && nd.light >= 0.25
                && rng::unit(rng::hash(seed, S_RETRENCH, nd.id as u64, (year as u64) << 20 | 1)) < a.epicormic_rate
            {
                nd.lat_buds = 2;
                nd.bud_year = year;
            }
        }
        killed
    }

    /// Spring flush: shed, allocate vigour, break buds, create the new
    /// annual shoots (initially unextended, `ext = 0`).
    #[allow(clippy::needless_range_loop)] // index-based: nodes are mutated in the loop
    pub fn flush(&mut self, g: &Genome, year: u32, age: f32, vigour_factor: f32, seed: u64) -> FlushStats {
        let a = &g.architecture;
        let mut stats = FlushStats::default();
        for n in self.nodes.iter_mut() {
            if year > n.birth_year.max(n.bud_year) + a.bud_lifetime_years {
                n.lat_buds = 0;
                n.term_bud = false;
            }
        }
        self.tree_h = self.nodes.iter().map(|n| n.end(1.0).y).fold(0.03, f32::max);
        let kc = a.lateral_competition;
        self.accumulate(kc);
        let thr = a.shedding_threshold * (1.3 - g.tolerances.shade);
        if year > 0 {
            let mean_thr = a.shedding_mean_threshold * (1.3 - g.tolerances.shade);
            let bole_h = (a.clear_bole_fraction * self.height()).min(a.clear_bole_max_m);
            let ratio = a.clear_bole_limb_ratio * a.clear_bole_limb_ratio;
            stats.shed = self.mark_shed(thr, mean_thr, year, (bole_h, ratio), a.shade_survival_years, g.wood.snag_years_per_m);
            let max_seg = 0.3 + 1.2 * self.voxel;
            self.compact(year, max_seg);
            if a.retrenchment_onset_years > 0.0 && age >= a.retrenchment_onset_years {
                stats.retrenched = self.retrench(g, year, age, seed);
            }
            self.accumulate(kc);
        }

        // Safety valve on structure size.
        if self.nodes.len() > NODE_SOFT_CAP {
            self.break_boost = (self.break_boost * 1.2).min(6.0);
        } else {
            self.break_boost = (self.break_boost * 0.95).max(1.0);
        }
        let threshold = a.bud_break_threshold * (1.3 - g.tolerances.shade) * self.break_boost;
        // Juvenile plateau, then a smooth decline to the mature value.
        let span = (a.maturity_age_years - a.juvenile_phase_years).max(1.0);
        let t = ((age - a.juvenile_phase_years) / span).clamp(0.0, 1.0);
        let mat = t * t * (3.0 - 2.0 * t);
        let lambda = a.apical_dominance_young + (a.apical_dominance_mature - a.apical_dominance_young) * mat;
        let n0 = self.nodes.len();
        let mut has_term = vec![false; n0];
        for n in self.nodes.iter().skip(1) {
            if n.terminal {
                has_term[n.parent as usize] = true;
            }
        }
        let root_q = self.nodes[0].q_main + self.nodes[0].q_lat;
        let v_root = a.vigour_per_light * root_q * vigour_factor;
        let mut new_nodes: Vec<Node> = Vec::new();
        let mut extended: Vec<(usize, f32)> = Vec::new();
        let unit_len = 0.75 * self.voxel - 0.06;
        for i in 0..n0 {
            let v_in = if i == 0 {
                v_root
            } else {
                let n = &self.nodes[i];
                let p = &self.nodes[n.parent as usize];
                let q = n.q_main + n.q_lat;
                if n.terminal {
                    if p.q_main > 0.0 { p.v_main * q / p.q_main } else { 0.0 }
                } else if p.s_lat > 0.0 {
                    p.v_lat * pow_k(q, kc) / p.s_lat
                } else {
                    0.0
                }
            };
            let n = &mut self.nodes[i];
            // The age schedule applies to the stem; branch axes keep their
            // own apical control (otherwise every axis turns sympodial).
            let lam = if n.order == 0 { lambda } else { a.apical_dominance_branches };
            let denom = lam * n.q_main + (1.0 - lam) * n.q_lat;
            if denom > 0.0 {
                n.v_main = v_in * lam * n.q_main / denom;
                n.v_lat = v_in * (1.0 - lam) * n.q_lat / denom;
            } else {
                n.v_main = 0.0;
                n.v_lat = 0.0;
            }
            if n.light <= 0.0 {
                continue;
            }
            // Terminal bud (only exists when there is no terminal child).
            // Weak but lit terminal buds still make a short shoot (oak
            // brachyblast), which keeps the branch alive without branching.
            // Level of detail: a leafy tip unit shorter than the voxel scale
            // is extended in place instead of spawning a new node, and only
            // branches once it spans ~a voxel. On young trees (small voxels)
            // every annual shoot is a node; on old trees a node stands for a
            // few years of twig growth whose detail is implied by the foliage.
            let complete = !n.leafy || n.length >= unit_len || n.unit_years >= 3;
            if n.term_bud && (n.v_main >= threshold || n.light >= thr) {
                let v = n.v_main;
                if complete {
                    n.term_bud = false;
                    has_term[i] = true;
                    let node = &self.nodes[i];
                    new_nodes.push(self.spawn(g, i, node, v, true, 0, year, seed));
                } else {
                    let node = &self.nodes[i];
                    let sh = self.spawn(g, i, node, v, true, 0, year, seed);
                    let n = &mut self.nodes[i];
                    let d = (n.dir * n.length + sh.dir * sh.length).normalize_or(n.dir);
                    n.grow_from = n.length;
                    // Keep the end continuous: re-aim the unit to the new tip.
                    let tip = n.start + n.dir * n.length + sh.dir * sh.length;
                    n.length = (tip - n.start).length();
                    n.grow_from = n.grow_from.min(n.length);
                    n.dir = d;
                    n.path_end += sh.length;
                    n.lat_buds = n.lat_buds.max(sh.lat_buds);
                    n.birth_year = year;
                    n.unit_years += 1;
                    extended.push((i, n.grow_from));
                    continue;
                }
            }
            if !complete {
                continue;
            }
            // Lateral buds: equal light, acrotonic weighting (distal buds stronger).
            let n = &self.nodes[i];
            if n.lat_buds > 0 && n.s_lat > 0.0 {
                let share = n.v_lat * pow_k(n.light, kc) / n.s_lat;
                let (nid, nbuds) = (n.id, n.lat_buds);
                let mut broken = 0u8;
                for k in 0..nbuds {
                    let jitter = 0.85 + 0.3 * rng::unit(rng::hash(seed, S_BUD, nid as u64, (year as u64) << 8 | k as u64));
                    let v = share * (1.0 - 0.12 * k as f32) * jitter;
                    if v >= threshold {
                        // Sympodial relay: on an axis whose terminal bud did
                        // not grow, the most distal lateral that breaks takes
                        // over the axis (oak's subterminal bud cluster), with
                        // only an elbow-sized deviation instead of the full
                        // branching angle.
                        let relay = !has_term[i];
                        has_term[i] = true;
                        let node = &self.nodes[i];
                        new_nodes.push(self.spawn(g, i, node, v, relay, k, year, seed));
                        broken += 1;
                        if relay {
                            self.nodes[i].term_bud = false;
                        }
                    }
                }
                self.nodes[i].lat_buds -= broken;
            }
        }
        for n in self.nodes.iter_mut() {
            n.leafy = false;
            n.grow_from = n.length;
        }
        for &(i, gf) in &extended {
            self.nodes[i].leafy = true;
            self.nodes[i].grow_from = gf;
        }
        stats.new_shoots = new_nodes.len();
        for mut n in new_nodes {
            n.id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1);
            self.nodes.push(n);
        }
        self.ext = 0.0;
        stats
    }

    /// Build a new annual shoot from a bud of `parent`.
    #[allow(clippy::too_many_arguments)]
    fn spawn(&self, g: &Genome, pi: usize, parent: &Node, vigour: f32, terminal: bool, k: u8, year: u32, seed: u64) -> Node {
        let a = &g.architecture;
        let key = (parent.id as u64) << 12 | (k as u64) << 1 | terminal as u64;
        let h = |s: u64| rng::hash(seed, S_SHOOT, key, (year as u64) << 4 | s);
        let order = if terminal { parent.order } else { parent.order.saturating_add(1) };
        let mut d = parent.dir;
        if !terminal {
            let u = parent.dir.any_perpendicular();
            let w = parent.dir.cross(u);
            let az = (a.phyllotaxis_deg * (k as f32 + (parent.id % 5) as f32)).to_radians()
                + 0.6 * (rng::unit(h(1)) - 0.5);
            let beta = a.branching_angle_deg.to_radians() * (0.85 + 0.3 * rng::unit(h(2)));
            d = parent.dir * beta.cos() + (u * az.cos() + w * az.sin()) * beta.sin();
        }
        let limb_base_y = if order == 0 {
            0.0
        } else if parent.order == 0 {
            parent.end(1.0).y
        } else {
            parent.limb_base_y
        };
        let n_max = a.max_shoot_length_m / a.internode_length_m;
        let metamers = vigour.min(n_max);
        // Straight-line collar-to-bud distance is used as the transport
        // distance (the summed path would also count sinuosity, which keeps
        // growing even when the crown no longer expands).
        let dist = parent.end(1.0).length();
        let hydraulic = (1.0 - (dist / a.hydraulic_path_max_m).max(0.0).powf(a.hydraulic_exponent)).clamp(0.03, 1.0);
        let length = (a.internode_length_m * metamers * hydraulic).max(0.015);
        // All direction changes below are per shoot; scale them with the
        // shoot length so curvature per metre stays similar when shoots get
        // short (old trees), instead of turning fine twigs into corkscrews.
        let bend = (length / a.max_shoot_length_m).sqrt().clamp(0.25, 1.0);
        // Relay of a lost terminal bud by a subterminal one: a persistent
        // direction change at the annual shoot boundary (oak "elbows").
        if terminal && rng::unit(h(6)) < a.leader_loss_probability {
            let u = d.any_perpendicular();
            let w = d.cross(u);
            let az = std::f32::consts::TAU * rng::unit(h(7));
            let side = u * az.cos() + w * az.sin();
            // Weak (short) shoots make smaller elbows, so fine twigs do not
            // zigzag at the scale of the crown.
            let ang = a.leader_loss_angle_deg.to_radians() * (0.6 + 0.8 * rng::unit(h(8))) * bend;
            d = d * ang.cos() + side * ang.sin();
        }
        // Random deviation (sinuosity): random unit vector scaled by tan(angle).
        let wig = Vec3::new(rng::normal(h(3)), rng::normal(h(4)), rng::normal(h(5))).normalize_or(Vec3::ZERO)
            * (a.sinuosity_deg.to_radians().tan() * bend);
        // Gravitropic correction towards the set-point angle of the axis.
        let (grav, setpoint) = if order == 0 {
            (a.gravitropism, 0.0)
        } else if order == 1 {
            let rel = (limb_base_y / self.tree_h.max(0.1)).clamp(0.0, 1.0);
            (a.lateral_gravitropism, a.limb_setpoint_low_deg + (a.limb_setpoint_high_deg - a.limb_setpoint_low_deg) * rel)
        } else {
            (a.lateral_gravitropism, a.twig_setpoint_deg)
        };
        // Upturn at the crown edge (stylised): beyond a horizontal reach of
        // `crown_spread_ratio * height / 2`, the set-point rises towards the
        // vertical, standing in for the mechanical and hydraulic costs of
        // ever longer horizontal limbs.
        let setpoint = if order > 0 && a.crown_spread_ratio > 0.0 {
            let reach = parent.end(1.0).horizontal_len() / (0.5 * a.crown_spread_ratio * self.tree_h.max(0.5));
            let f = ((reach - 0.6) / 0.6).clamp(0.0, 1.0);
            setpoint * (1.0 - f * f * (3.0 - 2.0 * f))
        } else {
            setpoint
        };
        let target = if setpoint <= 0.0 {
            Vec3::Y
        } else {
            let hz = Vec3::new(d.x, 0.0, d.z).normalize_or(parent.dir.any_perpendicular());
            let sp = setpoint.to_radians();
            hz * sp.sin() + Vec3::Y * sp.cos()
        };
        // Light comes from above the horizon: the shadow gradient may point
        // sideways but never downward.
        let photo = Vec3::new(parent.grad.x, parent.grad.y.max(0.0), parent.grad.z);
        d = (d + wig + (target * grav + photo * a.phototropism) * bend).normalize_or(Vec3::Y);
        // Don't grow into the ground.
        if d.y < -0.3 {
            d = Vec3::new(d.x, -0.3, d.z).normalize_or(Vec3::Y);
        }
        let lat_buds = if order < a.max_order && metamers >= 1.0 {
            ((metamers * a.lateral_buds_per_metamer).round() as u8).clamp(1, a.lateral_buds_per_shoot)
        } else {
            0
        };
        let r = g.wood.twig_radius_mm * 0.001;
        Node {
            id: 0,
            parent: pi as u32,
            start: parent.end(1.0),
            dir: d,
            length,
            grow_from: 0.0,
            area: PI * r * r,
            path_end: parent.path_end + length,
            order,
            limb_base_y,
            sag: 0.0,
            unit_years: 1,
            terminal,
            leafy: true,
            term_bud: true,
            lat_buds,
            birth_year: year,
            starve: 0,
            snag_since: NONE,
            bud_year: year,
            light: parent.light,
            grad: parent.grad,
            q_main: 0.0,
            q_lat: 0.0,
            s_lat: 0.0,
            leaf_q: 0.0,
            leaf_n: 0.0,
            leaf_max: 0.0,
            v_main: 0.0,
            v_lat: 0.0,
            foliage: 0.0,
            dead: false,
        }
    }

    /// Light pass: Beer-Lambert attenuation through a voxel grid of leaf
    /// area, swept along the 13 sky directions of [`SKY_DIRS`]. Sets `light`
    /// (0..1, fraction of the unobstructed overcast sky) and `grad` on every
    /// node.
    ///
    /// Each leafy unit adds optical depth to its voxel in proportion to its
    /// leaf area (a vigorous, full-length unit carries more leaves than a
    /// short brachyblast); along an oblique ray the depth per layer is scaled
    /// by the path length through the layer. Nothing here depends on the
    /// previous pass's light (no feedback loop). There is no ground/horizon
    /// attenuation: an open-grown tree receives the full side light, and the
    /// interior and lower inner crown are darkened only by the tree's own
    /// foliage.
    pub fn compute_light(&mut self) {
        let mut lo = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut hi = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
        for n in &self.nodes {
            let e = n.end(1.0);
            lo = Vec3::new(lo.x.min(e.x), lo.y.min(e.y), lo.z.min(e.z));
            hi = Vec3::new(hi.x.max(e.x), hi.y.max(e.y), hi.z.max(e.z));
        }
        let height = hi.y.max(0.1);
        let mut vs = (height / VOXELS_PER_HEIGHT).clamp(0.15, 1.0);
        let (mut nx, mut ny, mut nz);
        loop {
            nx = ((hi.x - lo.x) / vs) as usize + 1;
            ny = ((hi.y - lo.y.min(0.0)) / vs) as usize + 1;
            nz = ((hi.z - lo.z) / vs) as usize + 1;
            if nx * ny * nz <= 1_000_000 {
                break;
            }
            vs *= 1.15;
        }
        self.voxel = vs;
        let px = nx + 2 * PAD;
        let pz = nz + 2 * PAD;
        let plane = px * pz;
        let origin = Vec3::new(lo.x, lo.y.min(0.0), lo.z);
        let inv = 1.0 / vs;
        let cell = |p: Vec3| -> (usize, usize) {
            let x = (((p.x - origin.x) * inv) as usize).min(nx - 1) + PAD;
            let y = (((p.y - origin.y) * inv) as usize).min(ny - 1);
            let z = (((p.z - origin.z) * inv) as usize).min(nz - 1) + PAD;
            (y, z * px + x)
        };
        // Leaf area per voxel (in reference units); nodes bucketed by layer.
        let unit_len = 0.75 * vs;
        self.grid.clear();
        self.grid.resize(ny * plane, 0.0);
        let dens = &mut self.grid;
        let mut node_cell: Vec<(u32, u32, u32)> = Vec::with_capacity(self.nodes.len());
        for (i, n) in self.nodes.iter().enumerate() {
            let (y, c) = cell(n.end(1.0));
            if n.leafy {
                dens[y * plane + c] += leaf_units(n, unit_len);
            }
            node_cell.push((y as u32, c as u32, i as u32));
        }
        node_cell.sort_unstable();
        let n_nodes = self.nodes.len();
        // Distinct occupied cells (nodes are sorted by (layer, cell)): light
        // is evaluated once per cell, not once per node.
        let mut ucell: Vec<u32> = Vec::with_capacity(n_nodes);
        let mut node_u: Vec<u32> = Vec::with_capacity(n_nodes);
        let mut layer_u = vec![0usize; ny + 1];
        for &(y, c, _) in &node_cell {
            let new = match ucell.last() {
                Some(&lc) => lc != c || node_u.is_empty() || node_cell[node_u.len() - 1].0 != y,
                None => true,
            };
            if new {
                ucell.push(c);
                layer_u[y as usize + 1] = ucell.len();
            }
            node_u.push(ucell.len() as u32 - 1);
        }
        for y in 0..ny {
            layer_u[y + 1] = layer_u[y + 1].max(layer_u[y]);
        }
        let mut light = vec![0.0f32; ucell.len()];
        let mut gvec = vec![Vec3::ZERO; ucell.len()];
        let mut above = vec![0.0f32; plane];
        let mut tmp = vec![0.0f32; plane];
        let mut cur = vec![0.0f32; plane];
        let mut iso_up = 0.0f32;
        for &(dx, dz, w) in SKY_DIRS.iter() {
            above.iter_mut().for_each(|v| *v = 0.0);
            let path = ((1 + dx * dx + dz * dz) as f32).sqrt();
            let decay = SHADOW_DECAY.powf(path);
            let to_sky = Vec3::new(-dx as f32, 1.0, -dz as f32) * (1.0 / path);
            iso_up += w * to_sky.y;
            let src_off = dz as isize * px as isize + dx as isize;
            for y in (0..ny).rev() {
                // Shift by (dx, dz) per layer for oblique rays, then a
                // separable 3-tap blur (penumbra). Written as zipped slices
                // so the compiler can vectorise them.
                for z in PAD..PAD + nz {
                    let row = z * px;
                    let s0 = (row as isize - src_off) as usize + PAD;
                    let (sl, sc, sr) = (&above[s0 - 1..s0 - 1 + nx], &above[s0..s0 + nx], &above[s0 + 1..s0 + 1 + nx]);
                    let dst = &mut tmp[row + PAD..row + PAD + nx];
                    for (((d, &l), &c), &r) in dst.iter_mut().zip(sl).zip(sc).zip(sr) {
                        *d = BLUR_C * c + BLUR_E * (l + r);
                    }
                }
                for z in PAD..PAD + nz {
                    let row = z * px + PAD;
                    let (t0, t1, t2) = (&tmp[row - px..row - px + nx], &tmp[row..row + nx], &tmp[row + px..row + px + nx]);
                    let dst = &mut cur[row..row + nx];
                    for (((d, &a), &b), &c) in dst.iter_mut().zip(t0).zip(t1).zip(t2) {
                        *d = decay * (BLUR_C * b + BLUR_E * (a + c));
                    }
                }
                let (k0, k1) = (layer_u[y], layer_u[y + 1]);
                for (k, &c) in ucell[k0..k1].iter().enumerate() {
                    let q = exp_neg(cur[c as usize]) * w;
                    light[k0 + k] += q;
                    gvec[k0 + k] += to_sky * q;
                }
                let d = &dens[y * plane..(y + 1) * plane];
                let a = SHADOW_A * path;
                for z in PAD..PAD + nz {
                    let row = z * px + PAD;
                    for ((o, &c), &dd) in above[row..row + nx].iter_mut().zip(&cur[row..row + nx]).zip(&d[row..row + nx]) {
                        *o = c + a * dd;
                    }
                }
            }
        }
        // Same-voxel crowding, ranked by height so that crowded clusters thin
        // out from below instead of dying all at once (a voxel holds a lumped
        // cluster whose upper shoots shade the lower ones).
        let mut ranked: Vec<(u32, u32, f32, u32)> = node_cell
            .iter()
            .filter(|&&(_, _, i)| self.nodes[i as usize].leafy)
            .map(|&(y, c, i)| (y, c, self.nodes[i as usize].end(1.0).y, i))
            .collect();
        ranked.sort_unstable_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)).then(b.2.total_cmp(&a.2)).then(a.3.cmp(&b.3)));
        let mut rank = vec![0u16; n_nodes];
        let mut k = 0;
        while k < ranked.len() {
            let key = (ranked[k].0, ranked[k].1);
            let mut r = 0u16;
            while k < ranked.len() && (ranked[k].0, ranked[k].1) == key {
                rank[ranked[k].3 as usize] = r;
                r += 1;
                k += 1;
            }
        }
        for (k, &(y, c, i)) in node_cell.iter().enumerate() {
            let k = node_u[k] as usize;
            let n = &mut self.nodes[i as usize];
            let i = i as usize;
            let others = if n.leafy { rank[i] as f32 } else { dens[y as usize * plane + c as usize] };
            n.light = (light[k] * exp_neg(SHADOW_SAME * others)).clamp(0.0, 1.0);
            // Phototropism towards the brightest sky sector, relative to an
            // isotropic (fully lit) bud whose vector would point straight up.
            n.grad = (gvec[k] - Vec3::Y * (light[k] * iso_up)).normalize_or(Vec3::ZERO);
        }
    }

    /// Light-weighted foliage supported by each node (basipetal sum), in
    /// m2 of intercepting area. Call after `compute_light`.
    ///
    /// Foliage far from the collar is weighted down (hydraulic limitation of
    /// stomatal conductance in tall trees, Ryan & Yoder 1997), which produces
    /// the age-related decline of basal area increment.
    pub fn compute_foliage(&mut self, hydraulic_path_max: f32, exponent: f32) -> f32 {
        let face = self.voxel * self.voxel;
        for n in self.nodes.iter_mut() {
            n.foliage = if n.leafy {
                let eff = (1.0 - (n.end(1.0).length() / hydraulic_path_max).powf(exponent)).max(0.08);
                n.light * face * eff
            } else {
                0.0
            };
        }
        for i in (1..self.nodes.len()).rev() {
            let (p, f) = (self.nodes[i].parent as usize, self.nodes[i].foliage);
            self.nodes[p].foliage += f;
        }
        self.nodes[0].foliage
    }

    /// Secondary growth: add `coef * foliage * factor` m2 of wood to every
    /// segment (Pressler), capping the radial increment. Returns the area
    /// increment at the collar.
    pub fn thicken(&mut self, coef: f32, factor: f32, max_dr: f32) -> f32 {
        let mut collar = 0.0;
        for (i, n) in self.nodes.iter_mut().enumerate() {
            let r0 = n.radius();
            let a1 = n.area + coef * n.foliage * factor;
            let r1 = (a1 / PI).sqrt().min(r0 + max_dr);
            let new_area = PI * r1 * r1;
            if i == 0 {
                collar = new_area - n.area;
            }
            n.area = new_area;
        }
        collar
    }

    /// Stylised branch sag, applied once a year after secondary growth.
    ///
    /// Each branch segment carries the wood (and a nominal foliage load) of
    /// everything beyond it; its elastic curvature under that load is
    /// `kappa = M / (E I)` with `M` the moment of the load about the segment
    /// base (horizontal lever to the load's centre of mass) and
    /// `I = pi r^4 / 4`. The segment is rotated downward each year by
    /// `sag_coefficient * kappa * length` (capped), and its whole subtree
    /// follows rigidly. This is not mechanics (no stress relaxation, no
    /// reaction wood): it only reproduces the progressive drooping of old,
    /// long, heavy limbs while thick stems stay put. The stem (order 0) never
    /// sags.
    ///
    /// The same pass applies **autotropic straightening** (Moulia et al.
    /// 2006; Bastien et al. 2013: plant organs sense their own curvature and
    /// straighten it with reaction wood): where a thick axis is continued by
    /// a lateral that took over (relay, or the terminal shed), the
    /// continuation is turned back towards its parent's direction by up to
    /// `straightening_deg_per_year`, leaving `straightening_keep_deg` of
    /// residual bend. Big limbs thus read as sinuous axes instead of chains
    /// of 50-degree kinks; thin twigs keep their zigzag.
    pub fn sag(&mut self, g: &Genome) {
        let a = &g.architecture;
        if a.sag_coefficient <= 0.0 && a.straightening_deg_per_year <= 0.0 {
            return;
        }
        let n = self.nodes.len();
        let mut thickest = vec![NONE; n];
        for i in 1..n {
            let p = self.nodes[i].parent as usize;
            let t = thickest[p];
            if t == NONE || self.nodes[i].area > self.nodes[t as usize].area {
                thickest[p] = i as u32;
            }
        }
        let st_rate = a.straightening_deg_per_year.to_radians();
        let st_keep = a.straightening_keep_deg.to_radians();
        let st_min_r = a.straightening_min_radius_mm * 0.001;
        let rho = g.wood.density_kg_m3;
        let e = g.wood.elasticity_gpa * 1e9;
        // Nominal fresh foliage load: ~0.5 kg per m2 of crown surface cell.
        let leaf_kg = 0.5 * self.voxel * self.voxel;
        let mut mass = vec![0.0f32; n];
        let mut moment = vec![Vec3::ZERO; n];
        for (i, nd) in self.nodes.iter().enumerate() {
            let m = rho * nd.area * nd.length + if nd.leafy { leaf_kg } else { 0.0 };
            mass[i] = m;
            moment[i] = (nd.start + nd.dir * (0.5 * nd.length)) * m;
        }
        for i in (1..n).rev() {
            let p = self.nodes[i].parent as usize;
            mass[p] += mass[i];
            let mi = moment[i];
            moment[p] += mi;
        }
        let max_rot = a.sag_max_deg_per_year.to_radians();
        let max_total = a.sag_max_total_deg.to_radians();
        let min_r = a.sag_min_radius_mm * 0.001;
        let min_y = -a.sag_max_droop_deg.to_radians().sin();
        let max_up = a.sag_min_inclination_deg.to_radians().cos();
        // Single forward pass (parent < child): decide each segment's sag
        // increment, compose rotations down the tree and move the nodes.
        // `path` = sag accumulated from the stem to the segment; the total
        // is capped so that nested sagging segments cannot curl a limb.
        let mut rot = vec![Quat::IDENTITY; n];
        let mut path = vec![0.0f32; n];
        for i in 1..n {
            let p = self.nodes[i].parent as usize;
            let mut qp = rot[p];
            if st_rate > 0.0 && p > 0 && thickest[p] == i as u32 && self.nodes[p].radius() >= st_min_r && !self.nodes[i].leafy {
                let rd = qp.rotate(self.nodes[i].dir);
                // Straighten the azimuth only (side-to-side kinks); the
                // inclination stays under gravitropic control.
                let ph = Vec3::new(self.nodes[p].dir.x, 0.0, self.nodes[p].dir.z);
                let pd = if ph.length() > 0.2 {
                    (ph.normalize_or(Vec3::Y) * Vec3::new(rd.x, 0.0, rd.z).length() + Vec3::Y * rd.y).normalize_or(rd)
                } else {
                    rd
                };
                let ang = rd.dot(pd).clamp(-1.0, 1.0).acos();
                if ang > st_keep {
                    let axis = rd.cross(pd);
                    if axis.length() > 1e-6 {
                        let turn = (ang - st_keep).min(st_rate);
                        qp = Quat::from_axis_angle(axis.normalize_or(Vec3::Y), turn).mul(qp);
                    }
                }
            }
            let mut q = qp;
            let nd = &self.nodes[i];
            let room = max_total - path[p] - nd.sag;
            // Upright segments are loaded mostly axially and do not sag.
            let inclined = qp.rotate(nd.dir).y <= max_up;
            if a.sag_coefficient > 0.0 && nd.order > 0 && !nd.leafy && inclined && room > 1e-5 && nd.radius() >= min_r && nd.end(1.0).y >= 1.0 && mass[i] > 0.0 {
                let com = moment[i] * (1.0 / mass[i]);
                let lever = Vec3::new(com.x - nd.start.x, 0.0, com.z - nd.start.z);
                let l = lever.length();
                if l >= 1e-3 {
                    let r = nd.radius();
                    let inertia = 0.25 * PI * r * r * r * r;
                    let kappa = 9.81 * mass[i] * l / (e * inertia);
                    // Locked deflection: new wood is laid down on the bent
                    // limb, so the deflection never springs back.
                    let target = a.sag_coefficient * kappa * nd.length;
                    let ang = (target - nd.sag).min(max_rot).min(room);
                    if ang > 1e-5 {
                        // Lever direction after the ancestors' rotation.
                        let lw = qp.rotate(lever * (1.0 / l));
                        let lw = Vec3::new(lw.x, 0.0, lw.z).normalize_or(lever * (1.0 / l));
                        let local = Quat::from_axis_angle(Vec3::Y.cross(lw), ang);
                        let cand = local.mul(qp);
                        if cand.rotate(nd.dir).y >= min_y {
                            q = cand;
                            self.nodes[i].sag += ang;
                        }
                    }
                }
            }
            path[i] = path[p] + self.nodes[i].sag;
            rot[i] = q;
            let start = self.nodes[p].end(1.0);
            let nd = &mut self.nodes[i];
            nd.start = start;
            if !q.is_identity() {
                nd.dir = q.rotate(nd.dir).normalize_or(nd.dir);
                // Keep limbs off the ground.
                if nd.start.y + nd.dir.y * nd.length < 0.4 && nd.dir.y < 0.0 {
                    nd.dir = Vec3::new(nd.dir.x, 0.0, nd.dir.z).normalize_or(nd.dir);
                }
            }
        }
    }

    /// Index of the trunk node spanning height `y`, following terminal children.
    pub fn trunk_node_at(&self, y: f32) -> Option<usize> {
        let mut cur = 0usize;
        if self.nodes[0].end(self.ext).y >= y {
            return Some(0);
        }
        for i in 1..self.nodes.len() {
            let n = &self.nodes[i];
            if n.parent as usize == cur && n.terminal {
                cur = i;
                if n.end(self.ext).y >= y {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Architecture metrics (see [`crate::sim::ShapeMetrics`]).
    ///
    /// The trunk is followed from the collar along the *thickest* child
    /// (when the stem dissolves into limbs, the dominant limb). Every other
    /// child of a trunk node is a first-order limb.
    pub fn shape_metrics(&self, dbh_m: f32) -> (f32, u32, f32, f32) {
        let n = self.nodes.len();
        let mut thickest = vec![NONE; n];
        for i in 1..n {
            let p = self.nodes[i].parent as usize;
            let t = thickest[p];
            if t == NONE || self.nodes[i].area > self.nodes[t as usize].area {
                thickest[p] = i as u32;
            }
        }
        let mut on_trunk = vec![false; n];
        let mut cur = 0usize;
        on_trunk[0] = true;
        while thickest[cur] != NONE {
            cur = thickest[cur] as usize;
            on_trunk[cur] = true;
        }
        let mut bole = f32::MAX;
        let mut limbs = 0u32;
        let mut ratios: Vec<f32> = Vec::new();
        for i in 1..n {
            let c = &self.nodes[i];
            let p = c.parent as usize;
            if !on_trunk[p] || on_trunk[i] {
                continue;
            }
            let ratio = c.radius() / self.nodes[p].radius().max(1e-6);
            if ratio >= 0.1 {
                bole = bole.min(c.start.y);
            }
            if dbh_m > 0.0 && 2.0 * c.radius() >= 0.2 * dbh_m {
                limbs += 1;
                ratios.push(ratio);
            }
        }
        if bole == f32::MAX {
            bole = self.height();
        }
        let max_ratio = ratios.iter().copied().fold(0.0, f32::max);
        let mean_ratio = if ratios.is_empty() { 0.0 } else { ratios.iter().sum::<f32>() / ratios.len() as f32 };
        (bole, limbs, max_ratio, mean_ratio)
    }

    pub fn height(&self) -> f32 {
        self.nodes.iter().map(|n| n.end(self.ext).y).fold(0.0, f32::max)
    }

    /// Mean over 8 azimuth sectors of the farthest leafy shoot tip (m).
    pub fn crown_radius(&self) -> f32 {
        let mut sectors = [0.0f32; 8];
        for n in self.nodes.iter().filter(|n| !n.is_snag()) {
            let e = n.end(self.ext);
            let r = e.horizontal_len();
            if r <= 0.0 {
                continue;
            }
            let ang = e.z.atan2(e.x) + PI;
            let s = ((ang / (2.0 * PI) * 8.0) as usize).min(7);
            sectors[s] = sectors[s].max(r);
        }
        sectors.iter().sum::<f32>() / 8.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_exp_is_accurate() {
        for i in 0..2000 {
            let x = i as f32 * 0.01;
            let (a, b) = (exp_neg(x), (-x).exp());
            assert!((a - b).abs() <= 2e-4 * b + 1e-7, "exp(-{x}): {a} vs {b}");
        }
    }
}
