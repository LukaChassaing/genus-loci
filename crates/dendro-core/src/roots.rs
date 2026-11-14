//! Root system: a deliberately simpler, *stylised* mirror of the shoot model.
//!
//! Honest caveat: root architecture of mature oaks is poorly documented and
//! highly soil-dependent. What is grounded in literature: oak seedlings invest
//! in a strong taproot (often > 1 m within a few years), which later loses
//! dominance to a "heart" system of horizontal laterals in the top ~50 cm and
//! vertical sinkers near the trunk; laterals extend well beyond the crown
//! (commonly quoted 1.5-3x crown radius). Everything else here (extension
//! rates, branching probabilities, soil crowding rule) is a plausible
//! stylisation chosen for visuals and robustness, not fitted data.
//!
//! * Explicit axes: taproot, first-order laterals, sinkers and side roots up
//!   to `max_order`. Fine roots are implicit (not simulated).
//! * Tip extension each year scaled by the tree's productivity, a soil
//!   crowding factor (coarse occupancy grid), depth/spread limits.
//! * Thickening: pipe model. The collar receives the same cross-section
//!   increment as the stem base; each root segment gets a share equal to the
//!   fraction of root tips it supports.

use crate::genome::RootTraits;
use crate::math::Vec3;
use crate::rng::{self, S_ROOT};
use std::f32::consts::PI;

pub(crate) const NONE: u32 = u32::MAX;
/// Soft cap on explicit root nodes (side roots stop being added above it).
const ROOT_NODE_CAP: usize = 9_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RootKind {
    Tap,
    Lateral,
    Sinker,
}

#[derive(Clone, Debug)]
pub(crate) struct RootNode {
    pub id: u32,
    pub parent: u32,
    pub start: Vec3,
    pub dir: Vec3,
    pub length: f32,
    pub area: f32,
    pub order: u8,
    pub kind: RootKind,
    /// Active growing tip.
    pub tip: bool,
    /// Continues the parent's axis.
    pub continues: bool,
    /// Length of the axis from its base to this node's end (m).
    pub axis_len: f32,
    /// Preferred depth (m) of a lateral root.
    pub pref_depth: f32,
    tips_sub: f32,
}

impl RootNode {
    #[inline]
    pub fn end(&self) -> Vec3 {
        self.start + self.dir * self.length
    }
    #[inline]
    pub fn radius(&self) -> f32 {
        (self.area / PI).sqrt()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RootSystem {
    pub nodes: Vec<RootNode>,
    next_id: u32,
    laterals: u32,
}

impl RootSystem {
    pub fn new() -> Self {
        let r = 0.0012f32;
        RootSystem {
            nodes: vec![RootNode {
                id: 0,
                parent: NONE,
                start: Vec3::ZERO,
                dir: Vec3::new(0.0, -1.0, 0.0),
                length: 0.04,
                area: PI * r * r,
                order: 0,
                kind: RootKind::Tap,
                tip: true,
                continues: true,
                axis_len: 0.04,
                pref_depth: 0.0,
                tips_sub: 0.0,
            }],
            next_id: 1,
            laterals: 0,
        }
    }

    fn push(&mut self, mut n: RootNode) -> usize {
        n.id = self.next_id;
        self.next_id += 1;
        self.nodes.push(n);
        self.nodes.len() - 1
    }

    pub fn depth(&self) -> f32 {
        self.nodes.iter().map(|n| -n.end().y).fold(0.0, f32::max)
    }

    pub fn spread(&self) -> f32 {
        self.nodes.iter().map(|n| n.end().horizontal_len()).fold(0.0, f32::max)
    }

    /// Annual root growth.
    pub fn grow(&mut self, rt: &RootTraits, year: u32, age: f32, crown_radius: f32, productivity: f32, seed: u64) {
        let r_new = 0.0012f32;
        let a_new = PI * r_new * r_new;
        // Emerge first-order laterals from the taproot, a couple per year.
        let target = ((age + 1.0) * 2.0).min(rt.lateral_count as f32) as u32;
        while self.laterals < target {
            let k = self.laterals;
            let want_depth = 0.05 + rt.lateral_depth_m * (k as f32 + 0.5) / rt.lateral_count as f32;
            // Find the taproot node closest to that depth.
            let mut best = 0usize;
            let mut cur = 0usize;
            loop {
                if -self.nodes[cur].end().y <= want_depth + 0.05 {
                    best = cur;
                }
                let next = (cur + 1..self.nodes.len())
                    .find(|&j| self.nodes[j].parent as usize == cur && self.nodes[j].continues && self.nodes[j].kind == RootKind::Tap);
                match next {
                    Some(j) => cur = j,
                    None => break,
                }
            }
            let az = (137.5f32 * k as f32).to_radians() + 0.4 * rng::unit(rng::hash(seed, S_ROOT, k as u64, 1));
            let dir = Vec3::new(az.cos(), -0.18, az.sin()).normalize_or(Vec3::new(1.0, 0.0, 0.0));
            let start = self.nodes[best].end();
            self.push(RootNode {
                id: 0,
                parent: best as u32,
                start,
                dir,
                length: 0.03,
                area: a_new,
                order: 1,
                kind: RootKind::Lateral,
                tip: true,
                continues: false,
                axis_len: 0.03,
                pref_depth: (0.25 + 0.5 * rng::unit(rng::hash(seed, S_ROOT, k as u64, 2))) * rt.lateral_depth_m * 2.0,
                tips_sub: 0.0,
            });
            self.laterals += 1;
        }

        // Coarse soil occupancy (0.5 m cells, horizontal x depth) for crowding.
        const CELL: f32 = 0.5;
        const HALF: i32 = 64;
        const DEPTH_CELLS: i32 = 10;
        let mut occ = vec![0u16; (2 * HALF * 2 * HALF * DEPTH_CELLS) as usize];
        let cell_of = |p: Vec3| -> Option<usize> {
            let x = (p.x / CELL).floor() as i32 + HALF;
            let z = (p.z / CELL).floor() as i32 + HALF;
            let y = ((-p.y) / CELL).floor() as i32;
            if !(0..2 * HALF).contains(&x) || !(0..2 * HALF).contains(&z) || !(0..DEPTH_CELLS).contains(&y) {
                None
            } else {
                Some(((y * 2 * HALF + z) * 2 * HALF + x) as usize)
            }
        };
        for n in &self.nodes {
            if let Some(c) = cell_of(n.end()) {
                occ[c] = occ[c].saturating_add(1);
            }
        }

        let spread_limit = rt.lateral_spread * crown_radius.max(0.4) + 0.4;
        let prod = (0.3 + 0.7 * productivity).clamp(0.2, 1.4);
        // Segment length grows with the root system (level of detail).
        let max_seg = (0.3 + 0.012 * age).min(1.2);
        let allow_new_axes = self.nodes.len() < ROOT_NODE_CAP;
        let n0 = self.nodes.len();
        for i in 0..n0 {
            if !self.nodes[i].tip {
                continue;
            }
            let n = self.nodes[i].clone();
            let e = n.end();
            let depth = -e.y;
            let rh = e.horizontal_len();
            let base = rt.annual_extension_m * prod;
            // Maximum axis length of side roots relative to the system size.
            let order_len = match n.order {
                0 | 1 => f32::MAX,
                2 => (0.25 * spread_limit).max(0.8),
                _ => (0.08 * spread_limit).max(0.3),
            };
            let limit = match n.kind {
                RootKind::Tap => {
                    rt.taproot_dominance * (-age / rt.taproot_decline_years).exp().max(0.08)
                        * (1.0 - depth / rt.max_depth_m)
                }
                RootKind::Lateral => {
                    (1.0 - rh / spread_limit).min(1.0 - n.axis_len / order_len) * 0.75f32.powi(n.order as i32 - 1)
                }
                RootKind::Sinker => 0.5 * (1.0 - depth / rt.max_depth_m),
            };
            if limit <= 0.0 {
                // Reached its limit. First-order laterals may resume when the
                // crown (and thus the spread limit) grows.
                if !(n.kind == RootKind::Lateral && n.order == 1) {
                    self.nodes[i].tip = false;
                }
                continue;
            }
            let crowd = cell_of(e).map(|c| 1.0 / (1.0 + occ[c] as f32 / 6.0)).unwrap_or(1.0);
            let ext = base * limit.min(1.0) * crowd;
            if ext < 0.005 {
                continue;
            }
            let h = |s: u64| rng::hash(seed, S_ROOT, n.id as u64, (year as u64) << 8 | s);
            let wig = Vec3::new(rng::normal(h(1)), rng::normal(h(2)) * 0.5, rng::normal(h(3))) * 0.25;
            let mut d = (n.dir + wig).normalize_or(n.dir);
            match n.kind {
                RootKind::Tap | RootKind::Sinker => d = (d + Vec3::new(0.0, -1.5, 0.0)).normalize_or(n.dir),
                RootKind::Lateral => {
                    // Plagiotropic: steer towards the root's preferred depth.
                    let want = ((-n.pref_depth - e.y) * 0.8).clamp(-0.35, 0.35);
                    let hz = Vec3::new(d.x, 0.0, d.z).normalize_or(Vec3::new(1.0, 0.0, 0.0));
                    d = (hz + Vec3::new(0.0, want + d.y * 0.2, 0.0)).normalize_or(hz);
                }
            }
            // Stay below the surface and above the maximum rooting depth.
            if e.y + d.y * ext > -0.04 {
                d.y = d.y.min(-0.05);
                d = d.normalize_or(n.dir);
            }
            if depth + (-d.y) * ext > rt.max_depth_m {
                d.y = d.y.max(0.0);
                d = d.normalize_or(n.dir);
            }
            if n.length + ext <= max_seg * 0.6 && depth + (-n.dir.y).max(0.0) * ext <= rt.max_depth_m {
                // Lengthen the current tip segment.
                self.nodes[i].length += ext;
                self.nodes[i].axis_len += ext;
                continue;
            }
            self.nodes[i].tip = false;
            let seg = ext.min(max_seg);
            let child = self.push(RootNode {
                id: 0,
                parent: i as u32,
                start: e,
                dir: d,
                length: seg,
                area: a_new,
                order: n.order,
                kind: n.kind,
                tip: true,
                continues: true,
                axis_len: n.axis_len + seg,
                pref_depth: n.pref_depth,
                tips_sub: 0.0,
            });
            if !allow_new_axes {
                continue;
            }
            // Side roots.
            if n.order < rt.max_order && n.kind != RootKind::Tap && rng::unit(h(4)) < rt.branching_probability {
                let side = if rng::unit(h(5)) < 0.5 { -1.0 } else { 1.0 };
                let ang = side * (0.9 + 0.5 * rng::unit(h(6)));
                let (s, c) = ang.sin_cos();
                let sd = Vec3::new(d.x * c - d.z * s, d.y.min(-0.05), d.x * s + d.z * c).normalize_or(d);
                self.push(RootNode {
                    id: 0,
                    parent: child as u32,
                    start: e,
                    dir: sd,
                    length: 0.03,
                    area: a_new,
                    order: n.order + 1,
                    kind: RootKind::Lateral,
                    tip: true,
                    continues: false,
                    axis_len: 0.03,
                    pref_depth: (n.pref_depth + 0.3 * (rng::unit(h(8)) - 0.3)).max(0.1),
                    tips_sub: 0.0,
                });
            }
            // Sinkers drop from laterals close to the trunk.
            if n.kind == RootKind::Lateral
                && n.order <= 2
                && rh < 0.35 * spread_limit
                && rng::unit(h(7)) < rt.sinker_tendency * 0.35
            {
                self.push(RootNode {
                    id: 0,
                    parent: child as u32,
                    start: e,
                    dir: Vec3::new(d.x * 0.2, -1.0, d.z * 0.2).normalize_or(Vec3::new(0.0, -1.0, 0.0)),
                    length: 0.03,
                    area: a_new,
                    order: n.order + 1,
                    kind: RootKind::Sinker,
                    tip: true,
                    continues: false,
                    axis_len: 0.03,
                    pref_depth: 0.0,
                    tips_sub: 0.0,
                });
            }
        }
    }

    /// Pipe-model thickening: the collar gets `collar_da` (m2), each segment
    /// the share of root ends it supports.
    pub fn thicken(&mut self, collar_da: f32) {
        if collar_da <= 0.0 {
            return;
        }
        for n in self.nodes.iter_mut() {
            n.tips_sub = 0.0;
        }
        // A node with no children is an axis end.
        let len = self.nodes.len();
        let mut has_child = vec![false; len];
        for i in 1..len {
            has_child[self.nodes[i].parent as usize] = true;
        }
        for (n, &hc) in self.nodes.iter_mut().zip(&has_child) {
            if !hc {
                n.tips_sub = 1.0;
            }
        }
        for i in (1..len).rev() {
            let (p, t) = (self.nodes[i].parent as usize, self.nodes[i].tips_sub);
            self.nodes[p].tips_sub += t;
        }
        let total = self.nodes[0].tips_sub.max(1.0);
        for n in self.nodes.iter_mut() {
            n.area += collar_da * n.tips_sub / total;
        }
    }
}
