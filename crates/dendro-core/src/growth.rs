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
use crate::rng::{self, S_BUD, S_SHOOT};
use std::f32::consts::PI;

pub(crate) const NONE: u32 = u32::MAX;

/// Shadow added by one leafy shoot to the light passing through its voxel.
const SHADOW_A: f32 = 1.0;
/// Weight of the shadow cast into a shoot's own voxel (side-by-side crowding).
/// Kept small: a voxel cannot resolve the horizontal arrangement of foliage,
/// and on old trees one voxel holds a lumped shoot cluster.
const SHADOW_SAME: f32 = 0.2;
/// Fraction of shadow carried from one voxel layer to the next (penumbra /
/// scattering: shadow fades a few voxels below its source).
const SHADOW_DECAY: f32 = 0.82;
/// Lateral blur weights (centre, each edge) of the separable 3-tap kernel.
const BLUR_C: f32 = 0.5;
const BLUR_E: f32 = 0.25;
/// Sky directions: horizontal voxel shift per layer (x, z) and weight.
/// The oblique ones shift 2 voxels per layer (~27 deg elevation), which lets
/// the lower crown flanks receive light under the overhang of the upper crown.
const SKY_DIRS: [(i64, i64, f32); 5] =
    [(0, 0, 0.36), (2, 0, 0.16), (-2, 0, 0.16), (0, 2, 0.16), (0, -2, 0.16)];
/// Upward component of the sky vector of an unshaded bud (sum of w * y / |dir|).
const ISO_UP: f32 = 0.36 + 4.0 * 0.16 * 0.447_213_6;
/// Voxel size = tree height / this (bounded): the simulation's level of detail.
const VOXELS_PER_HEIGHT: f32 = 40.0;
/// Soft cap on shoot nodes: above it bud break gets harder (safety valve).
const NODE_SOFT_CAP: usize = 70_000;

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
            n.s_lat = n.lat_buds as f32 * n.light.powf(k);
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
                pn.s_lat += q.powf(k);
            }
            pn.leaf_q += lq;
            pn.leaf_n += ln;
            pn.leaf_max = pn.leaf_max.max(lm);
        }
    }

    /// Mark poorly lit or leafless branches for removal.
    fn mark_shed(&mut self, threshold: f32, mean_thr: f32, year: u32, bole: (f32, f32)) -> usize {
        let mut count = 0;
        self.nodes[0].dead = false;
        let mut on_stem = vec![false; self.nodes.len()];
        on_stem[0] = true;
        let (bole_h, bole_ratio) = bole;
        for i in 1..self.nodes.len() {
            let p = self.nodes[i].parent as usize;
            let n = &self.nodes[i];
            on_stem[i] = on_stem[p] && n.terminal;
            let dead = if self.nodes[p].dead {
                true
            } else if on_stem[p] && !n.terminal && n.start.y < bole_h && n.area < bole_ratio * self.nodes[p].area {
                // Stylised natural pruning of the lower stem (clear bole).
                true
            } else if n.leaf_n > 0.0 {
                // Hierarchical self-pruning: a branch dies when none of its
                // foliage is adequately lit (shaded twigs go first, whole
                // limbs only when entirely overtopped).
                n.order > 0 && (n.leaf_max < threshold || n.leaf_q < mean_thr * n.leaf_n)
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

    /// Spring flush: shed, allocate vigour, break buds, create the new
    /// annual shoots (initially unextended, `ext = 0`).
    #[allow(clippy::needless_range_loop)] // index-based: nodes are mutated in the loop
    pub fn flush(&mut self, g: &Genome, year: u32, age: f32, vigour_factor: f32, seed: u64) -> FlushStats {
        let a = &g.architecture;
        let mut stats = FlushStats::default();
        for n in self.nodes.iter_mut() {
            if year > n.birth_year + a.bud_lifetime_years {
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
            stats.shed = self.mark_shed(thr, mean_thr, year, (bole_h, ratio));
            let max_seg = 0.3 + 1.2 * self.voxel;
            self.compact(year, max_seg);
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
                    p.v_lat * q.powf(kc) / p.s_lat
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
                let share = n.v_lat * n.light.powf(kc) / n.s_lat;
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
        let hydraulic = (1.0 - dist / a.hydraulic_path_max_m).clamp(0.03, 1.0);
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

    /// Shadow-propagation light pass using the leafy shoots as shadow
    /// sources. Sets `light` and `grad` on every node.
    ///
    /// Shadow is swept downward through a voxel grid along five sky
    /// directions (zenith + four at 45 deg elevation), with attenuation and a
    /// small lateral blur per layer. A Palubicki-style single vertical
    /// pyramid lights only from above and kills the crown sides; the oblique
    /// sweeps approximate diffuse side light so open-grown crowns keep foliage
    /// down their flanks while the interior stays dark.
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
        // Planes are padded by 3 cells on each side (zero shadow = open sky).
        const PAD: usize = 3;
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
        // Leafy shoot density per voxel; nodes bucketed by layer.
        self.grid.clear();
        self.grid.resize(ny * plane, 0.0);
        let dens = &mut self.grid;
        let mut node_cell: Vec<(u32, u32, u32)> = Vec::with_capacity(self.nodes.len());
        for (i, n) in self.nodes.iter().enumerate() {
            let (y, c) = cell(n.end(1.0));
            if n.leafy {
                dens[y * plane + c] += 1.0;
            }
            node_cell.push((y as u32, c as u32, i as u32));
        }
        node_cell.sort_unstable();
        let mut layer_start = vec![0usize; ny + 1];
        for &(y, _, _) in &node_cell {
            layer_start[y as usize + 1] += 1;
        }
        for y in 0..ny {
            layer_start[y + 1] += layer_start[y];
        }
        let n_nodes = self.nodes.len();
        let mut light = vec![0.0f32; n_nodes];
        let mut gvec = vec![Vec3::ZERO; n_nodes];
        let mut above = vec![0.0f32; plane];
        let mut tmp = vec![0.0f32; plane];
        let mut cur = vec![0.0f32; plane];
        for &(dx, dz, w) in SKY_DIRS.iter() {
            above.iter_mut().for_each(|v| *v = 0.0);
            let to_sky = Vec3::new(-dx as f32, 1.0, -dz as f32).normalize_or(Vec3::Y);
            for y in (0..ny).rev() {
                // Separable blur, shifted by (dx, dz) per layer for oblique rays.
                for z in PAD..PAD + nz {
                    let row = z * px;
                    let src = &above[(row as isize - dx as isize) as usize..];
                    let dst = &mut tmp[row..row + px];
                    for x in PAD..PAD + nx {
                        dst[x] = BLUR_C * src[x] + BLUR_E * (src[x - 1] + src[x + 1]);
                    }
                }
                for z in PAD..PAD + nz {
                    let zs = (z as isize - dz as isize) as usize;
                    let (r0, r1, r2) = ((zs - 1) * px, zs * px, (zs + 1) * px);
                    let row = z * px;
                    for x in PAD..PAD + nx {
                        cur[row + x] = SHADOW_DECAY * (BLUR_C * tmp[r1 + x] + BLUR_E * (tmp[r0 + x] + tmp[r2 + x]));
                    }
                }
                // Stylisation: low-elevation sky light reaching the bottom of
                // the crown of a sizeable tree is attenuated (surrounding
                // vegetation, horizon, the tree's own skirt not resolved by
                // the coarse grid). This produces the clear bole of oak
                // instead of a crown skirting the ground.
                let wy = if dx == 0 && dz == 0 {
                    w
                } else {
                    let rel = (origin.y + (y as f32 + 0.5) * vs) / height;
                    let skirt = ((rel - 0.04) / 0.22).clamp(0.2, 1.0);
                    let ramp = ((height - 4.0) / 8.0).clamp(0.0, 1.0);
                    w * (1.0 - ramp * (1.0 - skirt))
                };
                for &(_, c, i) in &node_cell[layer_start[y]..layer_start[y + 1]] {
                    let q = (1.0 - cur[c as usize]).clamp(0.0, 1.0) * wy;
                    light[i as usize] += q;
                    gvec[i as usize] += to_sky * q;
                }
                let d = &dens[y * plane..(y + 1) * plane];
                for z in PAD..PAD + nz {
                    let row = z * px;
                    for x in row + PAD..row + PAD + nx {
                        above[x] = cur[x] + SHADOW_A * d[x];
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
        for &(y, c, i) in &node_cell {
            let n = &mut self.nodes[i as usize];
            let i = i as usize;
            let others = if n.leafy { rank[i] as f32 } else { dens[y as usize * plane + c as usize] };
            n.light = (light[i] - SHADOW_SAME * others).clamp(0.0, 1.0);
            // Phototropism towards the brightest sky sector, relative to an
            // isotropic (fully lit) bud whose vector would point straight up.
            n.grad = (gvec[i] - Vec3::Y * (light[i] * ISO_UP)).normalize_or(Vec3::ZERO);
        }
    }

    /// Light-weighted foliage supported by each node (basipetal sum), in
    /// m2 of intercepting area. Call after `compute_light`.
    ///
    /// Foliage far from the collar is weighted down (hydraulic limitation of
    /// stomatal conductance in tall trees, Ryan & Yoder 1997), which produces
    /// the age-related decline of basal area increment.
    pub fn compute_foliage(&mut self, hydraulic_path_max: f32) -> f32 {
        let face = self.voxel * self.voxel;
        for n in self.nodes.iter_mut() {
            n.foliage = if n.leafy {
                let eff = (1.0 - 0.75 * n.end(1.0).length() / hydraulic_path_max).max(0.1);
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
    pub fn sag(&mut self, g: &Genome) {
        let a = &g.architecture;
        if a.sag_coefficient <= 0.0 {
            return;
        }
        let n = self.nodes.len();
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
            let qp = rot[p];
            let mut q = qp;
            let nd = &self.nodes[i];
            let room = max_total - path[p] - nd.sag;
            // Upright segments are loaded mostly axially and do not sag.
            let inclined = qp.rotate(nd.dir).y <= max_up;
            if nd.order > 0 && !nd.leafy && inclined && room > 1e-5 && nd.radius() >= min_r && nd.end(1.0).y >= 1.0 && mass[i] > 0.0 {
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
        for n in &self.nodes {
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
