//! Root system: a deliberately simpler, *stylised* mirror of the shoot model.
//!
//! Honest caveat: root architecture of mature oaks is poorly documented and
//! highly soil-dependent. What is grounded in literature: oak seedlings invest
//! in a strong taproot (often > 1 m within a few years), which later loses
//! dominance to a "heart" system of horizontal laterals in the top ~50 cm and
//! vertical sinkers near the trunk; laterals extend beyond the crown. For
//! mature Q. robur, Lucot & Bruckert 1992, "Organisation du systeme
//! racinaire du chene pedoncule (Quercus robur) developpe en conditions
//! edaphiques non contraignantes", Ann Sci For 49:465-479,
//! doi:10.1051/forest:19920503: surface system 0-0.6 m, intensive rooting
//! within ~3 m of the stem, sinkers below 4 m, roots ~20 m from 150-year-old
//! trees (species traits `lateral_spread`, `max_depth_m`). Everything else here (extension
//! rates, branching probabilities, soil crowding rule) is a plausible
//! stylisation chosen for visuals and robustness, not fitted data.
//!
//! Sources for the grounded part: Kutschera & Lichtenegger 2002, "Wurzelatlas
//! mitteleuropaeischer Waldbaeume und Straeucher", Leopold Stocker Verlag,
//! Graz (root-system drawings and morphology of central-European trees,
//! including Quercus robur and Prunus avium); Stone & Kalisz 1991, "On the
//! maximum extent of tree roots", For Ecol Manage 46:59-102,
//! doi:10.1016/0378-1127(91)90245-Q (maximum rooting depth and radial extent).
//! The spread is a multiple of the crown radius (`lateral_spread`), set per
//! species; the oak's is calibrated on Lucot & Bruckert's ~20 m.
//!
//! * Explicit axes: taproot, first-order laterals, sinkers and side roots up
//!   to `max_order`. Fine roots are implicit: a per-soil-layer density
//!   (see `update_layers`) used by the water balance and drawn as tufts.
//! * The soil steers growth: penetration resistance, bedrock and
//!   permanently waterlogged layers, moisture (see `grow`).
//! * Tip extension each year scaled by the tree's productivity, a soil
//!   crowding factor (coarse occupancy grid), depth/spread limits.
//! * Thickening: pipe model (Shinozaki et al. 1964, "A quantitative analysis
//!   of plant form - the pipe model theory. I. Basic analyses", Jpn J Ecol
//!   14:97-105; originally formulated for shoots, its use for roots is an
//!   extension made here). The collar receives the same cross-section
//!   increment as the stem base; each root segment gets a share equal to the
//!   fraction of root tips it supports.

use crate::genome::RootTraits;
use crate::soil::{Layer, SoilColumn};
use crate::math::Vec3;
use crate::rng::{self, S_ROOT};
use std::f32::consts::PI;

pub(crate) const NONE: u32 = u32::MAX;

/// Fine-root proliferation factor of a soil layer (stylised): nutrients
/// (sqrt), last season's moisture, and avoidance of saturated soil.
/// Qualitative basis: roots proliferate in nutrient-rich patches (Hodge 2004,
/// New Phytol 162:9-24). Calibration: the sqrt, the 0.4/0.6 moisture weights
/// and the waterlogging term are invented functional forms, not fitted.
pub(crate) fn proliferation(l: &Layer, rt: &RootTraits) -> f32 {
    let tol = rt.waterlogging_tolerance.clamp(0.0, 1.0);
    l.nutrient.max(0.0).sqrt() * (0.4 + 0.6 * l.season_rew) * (1.0 - (1.0 - tol) * l.season_wet)
}
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

    /// Annual root growth in the layered soil.
    ///
    /// `stress` is last season's mean water stress (0..1): a dry year raises
    /// the allocation to roots and sends sinkers and laterals deeper
    /// (drought-deepening plasticity), a basis in e.g. Brunner et al. 2015,
    /// "How tree roots respond to drought", Front Plant Sci 6:547,
    /// doi:10.3389/fpls.2015.00547 (review) and the functional-equilibrium
    /// allocation theory (Poorter et al. 2012, New Phytol 193:30-50). Penetration slows with soil
    /// strength and stops at bedrock or in permanently saturated (anoxic)
    /// soil, with laterals deflected along the barrier: shallow "plate"
    /// root systems on shallow or waterlogged soils, deep rooting on loose
    /// dry sand (Stone & Kalisz 1991, maximum rooting depths; Kutschera &
    /// Lichtenegger 2002 for oak root profiles).
    #[allow(clippy::too_many_arguments)]
    pub fn grow(
        &mut self,
        rt: &RootTraits,
        year: u32,
        age: f32,
        crown_radius: f32,
        productivity: f32,
        stress: f32,
        soil: &SoilColumn,
        seed: u64,
    ) {
        let r_new = 0.0012f32;
        let a_new = PI * r_new * r_new;
        let barrier = soil.root_barrier(rt.waterlogging_tolerance).min(rt.max_depth_m);
        let stress = stress.clamp(0.0, 1.0);
        // Emerge first-order laterals from the root collar region, a couple
        // per year; they leave the stem base just below the surface (where
        // the buttresses are) and then run out at their preferred depth.
        let target = ((age + 1.0) * 2.0).min(rt.lateral_count as f32) as u32;
        while self.laterals < target {
            let k = self.laterals;
            let want_depth = (0.04 + 0.2 * (k as f32 + 0.5) / rt.lateral_count as f32).min(0.8 * barrier);
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
            let pref = (0.3 + 0.7 * rng::unit(rng::hash(seed, S_ROOT, k as u64, 2))) * rt.lateral_depth_m;
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
                pref_depth: pref,
                tips_sub: 0.0,
            });
            self.laterals += 1;
        }

        // Coarse soil occupancy (0.5 m cells, horizontal x depth) for crowding.
        const CELL: f32 = 0.5;
        const HALF: i32 = 64;
        const DEPTH_CELLS: i32 = 12;
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
        // Soil queries at a depth.
        let layer = |depth: f32| soil.layer_at(depth.max(0.0)).map(|i| &soil.layers[i]);
        let resist = |depth: f32| layer(depth).map(|l| l.resistance).unwrap_or(10.0);
        let moist = |depth: f32| layer(depth).map(|l| if l.season_wet > 0.5 { 0.3 } else { l.season_rew }).unwrap_or(0.0);

        let spread_limit = rt.lateral_spread * crown_radius.max(0.4) + 0.4;
        // Allocation to roots rises with water stress (functional balance:
        // Poorter et al. 2012, "Biomass allocation to leaves, stems and roots:
        // meta-analyses of interspecific variation and environmental control",
        // New Phytol 193:30-50, doi:10.1111/j.1469-8137.2011.03952.x).
        // Calibration: the gain `root_shoot_stress_gain` is a game value.
        let alloc = 1.0 + rt.root_shoot_stress_gain * stress;
        let prod = ((0.3 + 0.7 * productivity) * alloc).clamp(0.2, 1.8);
        // Drought deepening of sinkers / taproot.
        let deepen = 1.0 + rt.drought_deepening * stress * 2.0;
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
            // Soil strength slows elongation (Bengough et al. 2011, J Exp Bot
            // 62:59-68: halved at ~0.8-2 MPa, stopped at the species'
            // maximum). Calibration: the squared linear ramp is a
            // smooth stand-in for that threshold behaviour.
            let r_soil = resist(depth + 0.05);
            let f_soil = (1.0 - r_soil / rt.max_penetration_mpa.max(0.1)).clamp(0.0, 1.0).powi(2);
            let limit = match n.kind {
                RootKind::Tap => {
                    rt.taproot_dominance * (-age / rt.taproot_decline_years).exp().max(0.08) * (1.0 - depth / barrier) * f_soil * deepen
                }
                RootKind::Lateral => {
                    // Pioneer laterals keep most of their speed until close
                    // to the spread limit (sqrt), side roots slow linearly.
                    let reach = (1.0 - rh / spread_limit).max(0.0);
                    let reach = if n.order == 1 { reach.sqrt() } else { reach };
                    reach.min(1.0 - n.axis_len / order_len)
                        * 0.75f32.powi(n.order as i32 - 1)
                        * (0.5 + 0.5 * f_soil)
                }
                RootKind::Sinker => 0.5 * (1.0 - depth / barrier) * f_soil * deepen,
            };
            if limit <= 0.0 {
                // Reached its limit. First-order laterals may resume when the
                // crown (and thus the spread limit) grows.
                if !(n.kind == RootKind::Lateral && n.order == 1) {
                    self.nodes[i].tip = false;
                }
                continue;
            }
            // Crowding slows side roots; the first-order "pioneer" laterals
            // that explore beyond the crown are much less affected.
            let crowd = cell_of(e).map(|c| 1.0 / (1.0 + occ[c] as f32 / 6.0)).unwrap_or(1.0);
            let crowd = if n.kind == RootKind::Lateral && n.order == 1 { crowd.powf(0.25) } else { crowd };
            let ext = base * limit.min(1.0) * crowd;
            if ext < 0.005 {
                continue;
            }
            let h = |s: u64| rng::hash(seed, S_ROOT, n.id as u64, (year as u64) << 8 | s);
            // Sinuous paths: soil heterogeneity (stones, old channels).
            let wig = Vec3::new(rng::normal(h(1)), rng::normal(h(2)) * 0.5, rng::normal(h(3))) * 0.25;
            let mut d = (n.dir + wig).normalize_or(n.dir);
            match n.kind {
                RootKind::Tap | RootKind::Sinker => d = (d + Vec3::new(0.0, -1.5, 0.0)).normalize_or(n.dir),
                RootKind::Lateral => {
                    // Plagiotropic: steer towards the preferred depth, moved
                    // down in dry years, and towards moister soil
                    // (hydrotropism, stylised as a vertical moisture gradient).
                    let pref = (n.pref_depth * (1.0 + rt.drought_deepening * stress)).min(barrier - 0.08).max(0.05);
                    let grad = moist(depth + 0.25) - moist((depth - 0.25).max(0.02));
                    let want = ((-pref - e.y) * 0.8 - rt.hydrotropism * grad).clamp(-0.35, 0.35);
                    let hz = Vec3::new(d.x, 0.0, d.z).normalize_or(Vec3::new(1.0, 0.0, 0.0));
                    d = (hz + Vec3::new(0.0, want + d.y * 0.2, 0.0)).normalize_or(hz);
                }
            }
            // Stay below the surface and above the barrier (deflection).
            if e.y + d.y * ext > -0.04 {
                d.y = d.y.min(-0.05);
                d = d.normalize_or(n.dir);
            }
            if depth + (-d.y) * ext > barrier - 0.02 {
                d.y = d.y.max(0.0);
                d = d.normalize_or(n.dir);
            }
            if n.length + ext <= max_seg * 0.6 && depth + (-n.dir.y).max(0.0) * ext <= barrier {
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
                    pref_depth: (n.pref_depth + 0.3 * (rng::unit(h(8)) - 0.3)).max(0.05),
                    tips_sub: 0.0,
                });
            }
            // Sinkers drop from laterals near the trunk (heart-sinker
            // system), more often after dry years.
            if n.kind == RootKind::Lateral
                && n.order <= 2
                && rh < 0.35 * spread_limit
                && depth < barrier - 0.15
                && rng::unit(h(7)) < rt.sinker_tendency * 0.35 * (1.0 + rt.drought_deepening * stress)
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

    /// Root length per soil layer: structural (explicit axes) and fine roots.
    ///
    /// Fine roots are not simulated individually. Their length in a layer
    /// is `fine_root_per_m` metres per metre of structural root there
    /// (Calibration: the ratio is tuned to give topsoil root length
    /// densities of order km/m3; it is not a measured ratio),
    /// times a proliferation factor: nutrient-rich (Hodge 2004) and moist
    /// layers get more, seasonally saturated (anoxic) layers fewer
    /// (stylised). This density drives water uptake and is drawn as sparse
    /// tufts (`fine_root_tufts`).
    pub fn update_layers(&self, rt: &RootTraits, soil: &mut SoilColumn) {
        for l in soil.layers.iter_mut() {
            l.struct_len = 0.0;
            l.root_len = 0.0;
        }
        for n in &self.nodes {
            let mid = -(n.start.y + n.end().y) * 0.5;
            if let Some(i) = soil.layer_at(mid.max(0.0)) {
                soil.layers[i].struct_len += n.length;
            }
        }
        for l in soil.layers.iter_mut() {
            l.root_len = l.struct_len * (1.0 + rt.fine_root_per_m * proliferation(l, rt));
        }
        soil.root_zone_m2 = PI * self.spread().max(0.5).powi(2);
        soil.refresh_states();
    }

    /// Fraction of root length above `depth` (m): (structural, total with
    /// fine roots).
    pub fn length_fraction_above(&self, soil: &SoilColumn, depth: f32) -> (f32, f32) {
        let (mut s_up, mut s_all, mut t_up, mut t_all) = (0.0, 0.0, 0.0, 0.0);
        for l in &soil.layers {
            s_all += l.struct_len;
            t_all += l.root_len;
            let f = ((depth - l.top) / l.thick().max(1e-6)).clamp(0.0, 1.0);
            s_up += l.struct_len * f;
            t_up += l.root_len * f;
        }
        (s_up / s_all.max(1e-6), t_up / t_all.max(1e-6))
    }

    /// Path length (m) from the root collar to the start of each node,
    /// along the root (`parent < index` for every node).
    pub fn path_starts(&self) -> Vec<f32> {
        let mut p = vec![0.0f32; self.nodes.len()];
        for i in 1..self.nodes.len() {
            let par = self.nodes[i].parent as usize;
            p[i] = p[par] + self.nodes[par].length;
        }
        p
    }

    /// Space occupied by the root system, for a display envelope: the
    /// maximum depth (m, positive, including the root radius) reached in
    /// each cell of a polar grid of `sectors` azimuth sectors (sector k
    /// covers world azimuth atan2(z, x) in [k, k+1) * 2pi/sectors) by `bins`
    /// equal radial bins out to `r_max` (m); 0 = no root there. Segments
    /// are sampled every ~10 % of a bin, so long segments mark every cell
    /// they cross.
    pub fn envelope(&self, sectors: usize, bins: usize, r_max: f32) -> Vec<f32> {
        let (sectors, bins) = (sectors.max(1), bins.max(1));
        let mut out = vec![0.0f32; sectors * bins];
        let r_max = r_max.max(1e-3);
        let step = 0.1 * r_max / bins as f32;
        for n in &self.nodes {
            let rad = n.radius();
            let k = ((n.length / step).ceil() as usize).clamp(1, 64);
            for s in 0..=k {
                let p = n.start + n.dir * (n.length * s as f32 / k as f32);
                let r = p.horizontal_len();
                let az = p.z.atan2(p.x).rem_euclid(2.0 * PI);
                let si = ((az / (2.0 * PI) * sectors as f32) as usize).min(sectors - 1);
                let bi = ((r / r_max * bins as f32) as usize).min(bins - 1);
                let c = &mut out[si * bins + bi];
                *c = c.max(-p.y + rad);
            }
        }
        out
    }

    /// World azimuths (atan2(z, x)) of the `n` thickest first-order laterals.
    pub fn main_lateral_azimuths(&self, n: usize) -> Vec<f32> {
        let mut v: Vec<(f32, f32)> = self
            .nodes
            .iter()
            .filter(|r| r.kind == RootKind::Lateral && r.order == 1 && !r.continues)
            .map(|r| (r.area, r.dir.z.atan2(r.dir.x)))
            .collect();
        v.sort_by(|a, b| b.0.total_cmp(&a.0));
        v.into_iter().take(n).map(|x| x.1).collect()
    }

    /// Pipe-model thickening: the collar gets `collar_da` (m2), each segment
    /// the share of root ends it supports (Shinozaki et al. 1964; one unit
    /// cross-section per root tip, a stylisation of the pipe model).
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
