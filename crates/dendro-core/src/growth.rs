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
//! detail is implied by the foliage clusters. No mechanics (branch sag) yet.

use crate::genome::Genome;
use crate::math::Vec3;
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
            leaf_q: 0.0,
            leaf_n: 0.0,
            leaf_max: 0.0,
            v_main: 0.0,
            v_lat: 0.0,
            foliage: 0.0,
            dead: false,
        };
        ShootSystem { nodes: vec![seedling], next_id: 1, ext: 1.0, voxel: 0.1, break_boost: 1.0, grid: Vec::new() }
    }

    /// Basipetal sums of bud light (main/lateral) and of leafy-shoot light.
    fn accumulate(&mut self) {
        for n in self.nodes.iter_mut() {
            n.q_main = if n.term_bud { n.light } else { 0.0 };
            n.q_lat = n.lat_buds as f32 * n.light;
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
            }
            pn.leaf_q += lq;
            pn.leaf_n += ln;
            pn.leaf_max = pn.leaf_max.max(lm);
        }
    }

    /// Mark poorly lit or leafless branches for removal.
    fn mark_shed(&mut self, threshold: f32, year: u32) -> usize {
        let mut count = 0;
        self.nodes[0].dead = false;
        for i in 1..self.nodes.len() {
            let p = self.nodes[i].parent as usize;
            let n = &self.nodes[i];
            let dead = if self.nodes[p].dead {
                true
            } else if n.leaf_n > 0.0 {
                // Hierarchical self-pruning: a branch dies when none of its
                // foliage is adequately lit (shaded twigs go first, whole
                // limbs only when entirely overtopped).
                n.order > 0 && n.leaf_max < threshold
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
                && par.dir.dot(node.dir) > if year >= node.birth_year + 12 { 0.8 } else { 0.94 }
                && par.length + node.length <= max_seg;
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
    pub fn flush(&mut self, g: &Genome, year: u32, age: f32, vigour_factor: f32, seed: u64) -> FlushStats {
        let a = &g.architecture;
        let mut stats = FlushStats::default();
        for n in self.nodes.iter_mut() {
            if year > n.birth_year + a.bud_lifetime_years {
                n.lat_buds = 0;
                n.term_bud = false;
            }
        }
        self.accumulate();
        let thr = a.shedding_threshold * (1.3 - g.tolerances.shade);
        if year > 0 {
            stats.shed = self.mark_shed(thr, year);
            let max_seg = 0.3 + 1.2 * self.voxel;
            self.compact(year, max_seg);
            self.accumulate();
        }

        // Safety valve on structure size.
        if self.nodes.len() > NODE_SOFT_CAP {
            self.break_boost = (self.break_boost * 1.2).min(6.0);
        } else {
            self.break_boost = (self.break_boost * 0.95).max(1.0);
        }
        let threshold = a.bud_break_threshold * (1.3 - g.tolerances.shade) * self.break_boost;
        let mat = (age / a.maturity_age_years).clamp(0.0, 1.0);
        let lambda = a.apical_dominance_young + (a.apical_dominance_mature - a.apical_dominance_young) * mat;
        let n0 = self.nodes.len();
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
                } else if p.q_lat > 0.0 {
                    p.v_lat * q / p.q_lat
                } else {
                    0.0
                }
            };
            let n = &mut self.nodes[i];
            let denom = lambda * n.q_main + (1.0 - lambda) * n.q_lat;
            if denom > 0.0 {
                n.v_main = v_in * lambda * n.q_main / denom;
                n.v_lat = v_in * (1.0 - lambda) * n.q_lat / denom;
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
            if n.lat_buds > 0 && n.q_lat > 0.0 {
                let share = n.v_lat * n.light / n.q_lat;
                let mut broken = 0u8;
                for k in 0..n.lat_buds {
                    let jitter = 0.85 + 0.3 * rng::unit(rng::hash(seed, S_BUD, n.id as u64, (year as u64) << 8 | k as u64));
                    let v = share * (1.0 - 0.12 * k as f32) * jitter;
                    if v >= threshold {
                        let node = &self.nodes[i];
                        new_nodes.push(self.spawn(g, i, node, v, false, k, year, seed));
                        broken += 1;
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
        // Random deviation (sinuosity): random unit vector scaled by tan(angle).
        let n_max = a.max_shoot_length_m / a.internode_length_m;
        let metamers = vigour.min(n_max);
        let wig = Vec3::new(rng::normal(h(3)), rng::normal(h(4)), rng::normal(h(5))).normalize_or(Vec3::ZERO)
            * (a.sinuosity_deg.to_radians().tan() * (metamers / 3.0).clamp(0.25, 1.0));
        let grav = if order == 0 { a.gravitropism } else { a.lateral_gravitropism };
        d = (d + wig + Vec3::Y * grav + parent.grad * a.phototropism).normalize_or(Vec3::Y);
        // Don't grow into the ground.
        if d.y < -0.3 {
            d = Vec3::new(d.x, -0.3, d.z).normalize_or(Vec3::Y);
        }
        // Straight-line collar-to-bud distance is used as the transport
        // distance (the summed path would also count sinuosity, which keeps
        // growing even when the crown no longer expands).
        let dist = parent.end(1.0).length();
        let hydraulic = (1.0 - dist / a.hydraulic_path_max_m).clamp(0.03, 1.0);
        let length = (a.internode_length_m * metamers * hydraulic).max(0.015);
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
