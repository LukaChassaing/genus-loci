//! Engine-agnostic mesh output.
//!
//! Sources: geometry is a rendering stylisation. Where radii follow the pipe
//! model, see Shinozaki et al. 1964 (Jpn J Ecol 14:97-105) and the Leonardo
//! exponent note in `sim.rs` (Eloy 2011).
//!
//! Conventions (all meshes):
//! * Right-handed, **Y up**, metres, tree base (root collar) at the origin.
//! * Triangles are wound **clockwise when seen from the front face** (the side
//!   the vertex normals point to), i.e. `cross(b - a, c - a)` points *against*
//!   the normal. This is Godot's front-face convention. Use
//!   [`MeshData::flip_winding`] for counter-clockwise engines/formats
//!   (`to_obj` does it for you).
//! * Wood/roots: closed-around generalised cylinders (open ends), one seam
//!   vertex column duplicated for UVs. `u` = around the circumference (0..1),
//!   `v` = arc length along the axis divided by the axis base circumference
//!   (square texels for a tiling bark texture). Vertex colour = bark tint;
//!   dead wood (retrenchment) is tinted silver-grey. Roots: alpha = 1.0 on
//!   structural roots (`root_mesh`), 0.5 on fine-root tufts
//!   (`fine_root_mesh`), so one material can style both.
//! * Leaves: single-sided quads ("cards"); render with back-face culling
//!   disabled and alpha-scissor on a leaf texture atlas of two tiles side
//!   by side: u in 0..0.5 = one leafy shoot (the current year's long shoot),
//!   u in 0.5..1 = a twig spray (lumped twigs, short shoots and spurs); a
//!   card may be mirrored (u reversed). v = 0 at the card's distal end.
//!   Normals are bent outward from the crown centre (standard
//!   foliage-shading trick). Vertex colour RGB is the seasonal leaf colour
//!   (linear, as a summer green albedo); A = the card's autumn senescence
//!   0..1 (0 in spring, summer and on winter's dead leaves), so a renderer
//!   can treat autumn hues (picked as display colours) differently.

use crate::fmath::DMath;
use crate::math::Vec3;
use crate::rng;

/// Indexed triangle mesh.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    /// Optional second UV channel: empty, or one entry per vertex. Roots
    /// (`root_mesh`, `fine_root_mesh`): x = path length (m) from the root
    /// collar along the root to this vertex's ring; y = an axis key (an
    /// integer 0..63, constant along one drawn axis, for per-root variation
    /// in a shader) + the drawn tube radius there in m, capped at 0.99 (a
    /// proxy for the sap flow it carries, pipe model): key = floor(y),
    /// radius = fract(y).
    pub uv2: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

impl MeshData {
    /// Number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Reserve room for `verts` more vertices and `indices` more indices.
    pub fn reserve(&mut self, verts: usize, indices: usize) {
        self.positions.reserve(verts);
        self.normals.reserve(verts);
        self.uvs.reserve(verts);
        self.colors.reserve(verts);
        self.indices.reserve(indices);
    }

    /// Reverse the winding of every triangle (CW <-> CCW).
    pub fn flip_winding(&mut self) {
        for t in self.indices.as_chunks_mut::<3>().0 {
            t.swap(1, 2);
        }
    }

    /// Append another mesh.
    pub fn append(&mut self, other: &MeshData) {
        let base = self.positions.len() as u32;
        self.positions.extend_from_slice(&other.positions);
        self.normals.extend_from_slice(&other.normals);
        self.uvs.extend_from_slice(&other.uvs);
        self.colors.extend_from_slice(&other.colors);
        self.uv2.extend_from_slice(&other.uv2);
        self.indices.extend(other.indices.iter().map(|i| i + base));
    }

    /// Wavefront OBJ text (counter-clockwise, as OBJ viewers expect), as a
    /// single named object. Vertex colours are written as `v x y z r g b`.
    pub fn to_obj(&self, name: &str, vertex_offset: usize) -> String {
        use std::fmt::Write;
        let mut s = String::with_capacity(self.positions.len() * 80);
        let _ = writeln!(s, "o {name}");
        for (p, c) in self.positions.iter().zip(&self.colors) {
            let _ = writeln!(s, "v {:.4} {:.4} {:.4} {:.3} {:.3} {:.3}", p[0], p[1], p[2], c[0], c[1], c[2]);
        }
        for t in &self.uvs {
            let _ = writeln!(s, "vt {:.4} {:.4}", t[0], t[1]);
        }
        for n in &self.normals {
            let _ = writeln!(s, "vn {:.4} {:.4} {:.4}", n[0], n[1], n[2]);
        }
        for t in self.indices.as_chunks::<3>().0 {
            // Flip CW -> CCW.
            let (a, b, c) = (t[0] as usize + 1 + vertex_offset, t[2] as usize + 1 + vertex_offset, t[1] as usize + 1 + vertex_offset);
            let _ = writeln!(s, "f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}");
        }
        s
    }

    #[inline]
    fn vert(&mut self, p: Vec3, n: Vec3, uv: [f32; 2], c: [f32; 4]) {
        self.positions.push(p.to_array());
        self.normals.push(n.to_array());
        self.uvs.push(uv);
        self.colors.push(c);
    }
}

/// One segment of a branching tube skeleton.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TubeSeg {
    pub parent: u32,
    pub start: Vec3,
    pub end: Vec3,
    pub radius: f32,
    /// Continues the parent's axis (otherwise starts a new axis).
    pub continues: bool,
    pub color: [f32; 4],
    /// End radius factor for axis tips (relative to own radius).
    pub tip_taper: f32,
    /// Path length (m) from the root collar to `start`, written to `uv2`
    /// when `TubeStyle::path_uv2` is set (only read on axis starts).
    pub path0: f32,
}

/// Stylisation of the tube junctions (mesh only: never feeds back into the
/// simulation, so DBH and rings are unaffected).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TubeStyle {
    /// Branch collar: each side axis starts inside its parent and its base
    /// ring is widened by this fraction (0 = plain tube start).
    pub collar_flare: f32,
    /// Root flare at the collar of the root axis: relative radius increase
    /// at y = 0 (fading out quadratically up to `root_flare_height`).
    pub root_flare: f32,
    /// Height (m) of the flared zone.
    pub root_flare_height: f32,
    /// Number of buttress lobes in the flared zone (0 = round).
    pub buttress_lobes: u32,
    /// Azimuths (rad, atan2(z, x) in world space) of the buttress lobes;
    /// empty = evenly spaced. Lets the lobes sit over the main lateral roots.
    pub lobe_azimuths: [f32; 8],
    /// Number of valid entries in `lobe_azimuths`.
    pub lobe_count: u32,
    /// Axes with a drawn radius above this are smoothed (Catmull-Rom); 0
    /// disables.
    pub smooth_min_radius: f32,
    /// Relative cross-section irregularity of thick side axes (0 = round).
    pub irregularity: f32,
    /// Axes thinner than `twig_radius` (m) are simplified to rings at least
    /// `twig_min_len` (m) apart (0 disables).
    pub twig_radius: f32,
    pub twig_min_len: f32,
    /// Fill `MeshData::uv2` with (path length from the collar, radius).
    pub path_uv2: bool,
}

const NONE: u32 = u32::MAX;

/// Radial resolution: 3 sides for fine twigs (< ~4 mm), 4-6 for small
/// branches, up to 28 on massive wood so big limbs read round.
fn radial_segments(r: f32) -> usize {
    if r < 0.004 {
        3
    } else {
        (4.0 + r * 45.0).clamp(4.0, 24.0) as usize
    }
}

/// Thin twig axes: drop interior rings closer than `min_len` (m) to the
/// last kept one. Straightens the kinks between short lumped shoot units
/// into a fine, nearly straight spray and halves the triangle cost of the
/// twig layer. Side twigs keep their (original) attachment points, which
/// then lie at most a few centimetres off the simplified tube.
fn decimate_axis(rings: &mut Vec<Ring>, first: usize, min_len: f32) {
    if rings.len() <= first + 2 {
        return;
    }
    let src = std::mem::take(rings);
    let last = src.len() - 1;
    let mut out = Vec::with_capacity(src.len());
    out.extend_from_slice(&src[..=first]);
    for (k, r) in src.iter().enumerate().skip(first + 1) {
        let prev = out[out.len() - 1].center;
        if k == last || (r.center - prev).length() >= min_len {
            out.push(*r);
        }
    }
    // Re-aim tangents along the simplified polyline.
    for k in first..out.len() {
        let a = out[k.saturating_sub(1).max(first)].center;
        let b = out[(k + 1).min(out.len() - 1)].center;
        out[k].tangent = (b - a).normalize_or(out[k].tangent);
    }
    *rings = out;
}

/// Smooth thick axes: insert Catmull-Rom points between ring centres
/// (the curve passes through the original centres, so side axes stay
/// attached). Segments are subdivided to about 2.5 radii per span, at most
/// 3 pieces; axes thinner than `min_r` are left as they are. Rings below
/// `skip_below_y` (the subdivided root-flare zone) are not touched.
fn smooth_axis(rings: &mut Vec<Ring>, first: usize, min_r: f32, skip_below_y: f32) {
    if rings.len() < first + 3 || rings[first].radius < min_r {
        return;
    }
    let src = std::mem::take(rings);
    let n = src.len();
    let mut out = Vec::with_capacity(n * 2);
    out.extend_from_slice(&src[..=first]);
    for k in first..n - 1 {
        let (a, b) = (src[k], src[k + 1]);
        let p0 = if k > 0 { src[k - 1].center } else { a.center * 2.0 - b.center };
        let p3 = if k + 2 < n { src[k + 2].center } else { b.center * 2.0 - a.center };
        let len = (b.center - a.center).length();
        let r = a.radius.max(b.radius);
        let m = if r >= min_r && a.center.y >= skip_below_y { ((len / (6.0 * r)).ceil() as usize).clamp(1, 3) } else { 1 };
        for j in 1..m {
            let t = j as f32 / m as f32;
            let (t2, t3) = (t * t, t * t * t);
            let c = (a.center * 2.0
                + (b.center - p0) * t
                + (p0 * 2.0 - a.center * 5.0 + b.center * 4.0 - p3) * t2
                + (a.center * 3.0 - p0 - b.center * 3.0 + p3) * t3)
                * 0.5;
            let d = ((b.center - p0)
                + (p0 * 2.0 - a.center * 5.0 + b.center * 4.0 - p3) * (2.0 * t)
                + (a.center * 3.0 - p0 - b.center * 3.0 + p3) * (3.0 * t2))
                * 0.5;
            out.push(Ring {
                center: c,
                radius: a.radius + (b.radius - a.radius) * t,
                tangent: d.normalize_or(a.tangent),
                color: a.color,
            });
        }
        out.push(b);
    }
    *rings = out;
}

/// Build generalised cylinders along the axes of a segment tree. Axes are
/// chains of `continues` segments; they are emitted thickest first until
/// `tri_budget` is exhausted, skipping axes thinner than `min_radius`.
/// Requires `parent < index` for every segment.
pub(crate) fn build_tubes(segs: &[TubeSeg], tri_budget: usize, min_radius: f32, style: TubeStyle) -> MeshData {
    let n = segs.len();
    let mut next = vec![NONE; n];
    for (i, s) in segs.iter().enumerate() {
        if s.continues && s.parent != NONE && next[s.parent as usize] == NONE {
            next[s.parent as usize] = i as u32;
        }
    }
    // Axis starts with their segment count.
    let mut axes: Vec<(f32, u32, u32)> = Vec::new();
    for (i, s) in segs.iter().enumerate() {
        let is_start = s.parent == NONE || next[s.parent as usize] != i as u32;
        if !is_start || s.radius < min_radius {
            continue;
        }
        let mut count = 0u32;
        let mut j = i as u32;
        while j != NONE {
            count += 1;
            j = next[j as usize];
        }
        axes.push((s.radius, i as u32, count));
    }
    axes.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));

    let mut mesh = MeshData::default();
    mesh.reserve(tri_budget * 3 / 4, tri_budget * 3);
    let mut used = 0usize;
    // Segments actually drawn: an axis whose parent axis was dropped for
    // budget is dropped too (no floating fragments).
    let mut drawn = vec![false; n];
    let mut chain: Vec<usize> = Vec::new();
    let mut rings: Vec<Ring> = Vec::new();
    for &(r0, start, count) in &axes {
        let root_axis = segs[start as usize].parent == NONE;
        if !root_axis && !drawn[segs[start as usize].parent as usize] {
            continue;
        }
        let mut radial = radial_segments(r0);
        if root_axis && style.buttress_lobes > 0 {
            radial = radial.max(6 * style.buttress_lobes as usize).min(36);
        }
        let extra = if root_axis { 8 } else { 1 };
        let tris = (count as usize + extra) * radial * 2;
        if used + tris > tri_budget {
            if tris <= 2 * radial * 2 {
                break; // even tiny axes no longer fit
            }
            continue;
        }
        chain.clear();
        let mut j = start;
        while j != NONE {
            let s = &segs[j as usize];
            if (s.end - s.start).length() < 1e-4 {
                break;
            }
            chain.push(j as usize);
            j = next[j as usize];
        }
        if chain.is_empty() {
            continue;
        }
        axis_rings(segs, &chain, &style, &mut rings);
        let first = if !root_axis && style.collar_flare > 0.0 { 1 } else { 0 };
        let flare_top = if root_axis { style.root_flare_height } else { f32::MIN };
        smooth_axis(&mut rings, first, style.smooth_min_radius.max(1e-4), flare_top);
        if style.twig_min_len > 0.0 && r0 < style.twig_radius {
            decimate_axis(&mut rings, first, style.twig_min_len);
        }
        let tris = (rings.len() - 1) * radial * 2;
        if used + tris > tri_budget {
            continue;
        }
        for &c in &chain {
            drawn[c] = true;
        }
        let irregular = if root_axis { 0.0 } else { style.irregularity };
        let phase = rng::unit(rng::mix(start as u64 ^ 0xBA4C)) * std::f32::consts::TAU;
        let path0 = style.path_uv2.then(|| (segs[start as usize].path0, (rng::mix(start as u64 ^ 0xF10) % 64) as f32));
        let flare = if root_axis { style } else { TubeStyle::default() };
        used += emit_rings(&mut mesh, &rings, radial, flare, irregular, phase, path0);
    }
    mesh
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Ring {
    pub center: Vec3,
    pub radius: f32,
    tangent: Vec3,
    color: [f32; 4],
}

/// Rings along one axis, with the branch collar (side axes) or the
/// subdivided root-flare zone (root axis).
pub(crate) fn axis_rings(segs: &[TubeSeg], chain: &[usize], style: &TubeStyle, rings: &mut Vec<Ring>) {
    rings.clear();
    let m = chain.len();
    let s0 = &segs[chain[0]];
    let first_dir = (s0.end - s0.start).normalize_or(Vec3::Y);
    if s0.parent != NONE && style.collar_flare > 0.0 {
        // Branch collar: start inside the parent so no gap shows, flare the
        // base ring to blend into the parent's surface.
        // The hidden first ring never exceeds the flared branch radius (a
        // thin twig on a thick stem must not start as a wide cone).
        let pr = segs[s0.parent as usize].radius.max(s0.radius);
        let r1 = (s0.radius * (1.0 + style.collar_flare)).min(pr);
        let back = 0.8 * pr;
        rings.push(Ring { center: s0.start - first_dir * back, radius: (0.9 * pr).min(r1), tangent: first_dir, color: s0.color });
        rings.push(Ring { center: s0.start, radius: r1, tangent: first_dir, color: s0.color });
    } else {
        rings.push(Ring { center: s0.start, radius: s0.radius, tangent: first_dir, color: s0.color });
    }
    for k in 1..=m {
        let s = &segs[chain[k - 1]];
        let d0 = (s.end - s.start).normalize_or(Vec3::Y);
        let ring = if k < m {
            let s2 = &segs[chain[k]];
            let d1 = (s2.end - s2.start).normalize_or(d0);
            Ring { center: s.end, radius: s2.radius, tangent: (d0 + d1).normalize_or(d0), color: s2.color }
        } else {
            Ring { center: s.end, radius: s.radius * s.tip_taper, tangent: d0, color: s.color }
        };
        rings.push(ring);
    }
    if s0.parent == NONE && style.root_flare > 0.0 && style.root_flare_height > 0.0 {
        // Subdivide the flared zone so the flare curve is smooth.
        let h = style.root_flare_height;
        let step = h / 6.0;
        let mut out: Vec<Ring> = Vec::with_capacity(rings.len() + 8);
        out.push(rings[0]);
        for w in rings.windows(2) {
            let (a, b) = (w[0], w[1]);
            if a.center.y < h {
                let span = (b.center.y.min(h) - a.center.y).max(0.0);
                let k = ((span / step).ceil() as usize).clamp(1, 8);
                for i in 1..k {
                    let t = i as f32 / k as f32 * (span / (b.center.y - a.center.y).max(1e-6)).min(1.0);
                    out.push(Ring {
                        center: a.center + (b.center - a.center) * t,
                        radius: a.radius + (b.radius - a.radius) * t,
                        tangent: (a.tangent + (b.tangent - a.tangent) * t).normalize_or(a.tangent),
                        color: a.color,
                    });
                }
            }
            out.push(b);
        }
        *rings = out;
    }
}

/// `path0`: (path length at the axis start, axis key); when set, `uv2` =
/// (path0 + arc length along the axis, key + ring radius), see `MeshData::uv2`.
fn emit_rings(mesh: &mut MeshData, rings: &[Ring], radial: usize, flare: TubeStyle, irregular: f32, phase: f32, path0: Option<(f32, f32)>) -> usize {
    let m = rings.len() - 1;
    let base_r = rings[0].radius.max(1e-4);
    let circumference = std::f32::consts::TAU * base_r;
    let mut normal = rings[0].tangent.any_perpendicular();
    let mut arclen = 0.0f32;
    let base = mesh.positions.len() as u32;
    let mut prev_center = rings[0].center;
    for ring in rings {
        arclen += (ring.center - prev_center).length();
        prev_center = ring.center;
        let tangent = ring.tangent;
        // Parallel transport of the frame.
        normal = (normal - tangent * normal.dot(tangent)).normalize_or(tangent.any_perpendicular());
        let binormal = tangent.cross(normal);
        let v = arclen / circumference;
        // Root flare weight at this height (root axis only).
        let w = if flare.root_flare > 0.0 && flare.root_flare_height > 0.0 {
            let t = (1.0 - ring.center.y / flare.root_flare_height).clamp(0.0, 1.0);
            t * t
        } else {
            0.0
        };
        for j in 0..=radial {
            let th = std::f32::consts::TAU * j as f32 / radial as f32;
            let (s, c) = th.dsin_cos();
            let dirv = normal * c + binormal * s;
            let mut r = ring.radius;
            if w > 0.0 {
                let lobe = if flare.lobe_count > 0 {
                    // Lobes centred on the given world azimuths.
                    let az = dirv.z.datan2(dirv.x);
                    let mut best = 0.0f32;
                    for &la in &flare.lobe_azimuths[..flare.lobe_count as usize] {
                        let mut d = (az - la).abs() % std::f32::consts::TAU;
                        if d > std::f32::consts::PI {
                            d = std::f32::consts::TAU - d;
                        }
                        let x = (1.0 - d / 0.7).max(0.0);
                        best = best.max(x * x * (3.0 - 2.0 * x));
                    }
                    0.35 + 0.65 * best
                } else if flare.buttress_lobes > 0 {
                    let x = 0.5 + 0.5 * (th * flare.buttress_lobes as f32).dcos();
                    0.35 + 0.65 * x * x
                } else {
                    1.0
                };
                r *= 1.0 + flare.root_flare * w * lobe;
            }
            if irregular > 0.0 && ring.radius > 0.03 {
                // Fluted, slightly out-of-round old wood (low harmonics, fixed
                // per axis so the bumps run along it as ridges).
                let k = ((ring.radius - 0.03) / 0.1).min(1.0);
                r *= 1.0 + irregular * k * (0.6 * (2.0 * th + phase).dsin() + 0.4 * (3.0 * th + 1.7 * phase).dsin());
            }
            mesh.vert(ring.center + dirv * r, dirv, [j as f32 / radial as f32, v], ring.color);
        }
        if let Some((p0, key)) = path0 {
            let e = [p0 + arclen, key + ring.radius.min(0.99)];
            mesh.uv2.extend(std::iter::repeat_n(e, radial + 1));
        }
    }
    let row = radial as u32 + 1;
    for k in 0..m as u32 {
        for j in 0..radial as u32 {
            let a = base + k * row + j;
            let b = a + 1;
            let c = a + row;
            let d = c + 1;
            // Clockwise seen from outside (see module docs).
            mesh.indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    m * radial * 2
}

/// A leaf cluster to be drawn as crossed cards.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LeafCluster {
    /// Attachment point: the card spans from `anchor * size` behind it to
    /// `(1 - anchor) * size` ahead of it along `dir`.
    pub pos: Vec3,
    pub dir: Vec3,
    pub size: f32,
    pub color: [f32; 4],
    pub key: u64,
    /// Centre of the foliage lobe this cluster belongs to (normals are bent
    /// away from it).
    pub center: Vec3,
    /// Number of crossed cards (1..=3), rotated 180/cards deg apart about
    /// `dir`.
    pub cards: u8,
    /// Fraction of the card length behind `pos`.
    pub anchor: f32,
    /// Horizontal texture range `[u0, u1]` of the cards (texture atlas
    /// tile); a random half of the cards is mirrored.
    pub tile: [f32; 2],
}

/// Crossed cards per (full) leaf cluster.
pub(crate) const CARDS_PER_CLUSTER: usize = 3;

/// Build leaf cards; at most `tri_budget` triangles (clusters are thinned
/// deterministically and enlarged to compensate).
pub(crate) fn build_leaves(clusters: &[LeafCluster], tri_budget: usize) -> MeshData {
    let mut mesh = MeshData::default();
    if clusters.is_empty() {
        return mesh;
    }
    let total_cards: usize = clusters.iter().map(|c| c.cards.max(1) as usize).sum();
    let max_cards = tri_budget / 2;
    let keep = if total_cards > max_cards { max_cards as f32 / total_cards as f32 } else { 1.0 };
    let cap = total_cards.min(max_cards + max_cards / 8);
    mesh.reserve(cap * 4, cap * 6);
    let scale = 1.0 / keep.sqrt();
    let mut center = Vec3::ZERO;
    for c in clusters {
        center += c.pos;
    }
    center = center * (1.0 / clusters.len() as f32);
    center.x = 0.0;
    center.z = 0.0;
    let mut tris = 0usize;
    for c in clusters {
        if keep < 1.0 && rng::unit(rng::mix(c.key ^ 0x5EED)) >= keep {
            continue;
        }
        let cards = c.cards.clamp(1, 3) as usize;
        if tris + cards * 2 > tri_budget {
            break;
        }
        let s = c.size * scale;
        let crown_out = (c.pos - center).normalize_or(Vec3::Y);
        let outward = ((c.pos - c.center).normalize_or(crown_out) * 0.65 + crown_out * 0.35).normalize_or(crown_out);
        let dir = c.dir.normalize_or(Vec3::Y);
        let perp = dir.any_perpendicular();
        let perp2 = dir.cross(perp);
        let spin = rng::unit(rng::mix(c.key)) * std::f32::consts::PI;
        // Card k is rotated by k x 180/cards deg: rotate (sin, cos) instead
        // of evaluating them per card (this loop runs ~10^5 times).
        let (mut sa, mut ca) = spin.dsin_cos();
        let (sr, cr) = (std::f32::consts::PI / cards as f32).dsin_cos();
        let mid_k = 0.5 * (cards as f32 - 1.0);
        for k in 0..cards {
            if k > 0 {
                let s2 = sa * cr + ca * sr;
                ca = ca * cr - sa * sr;
                sa = s2;
            }
            let h = rng::mix(c.key ^ (0x71 + k as u64));
            let b = perp * ca + perp2 * sa; // card width axis
            // Card length axis: along the shoot, tilted by a varied per-card
            // amount so the cluster reads as a volume from all sides.
            let tilt = 0.35 * (k as f32 - mid_k) + 0.5 * (rng::unit(h) - 0.5);
            let a = (dir + b.cross(dir) * tilt).normalize_or(dir);
            let mid = c.pos + a * (s * (0.5 - c.anchor));
            // a and b are orthonormal: their cross product is a unit vector.
            let mut face = a.cross(b);
            let flipped = face.dot(outward) < 0.0;
            if flipped {
                face = -face;
            }
            let nrm = (face * 0.35 + outward * 0.65).normalize_or(face).to_array();
            let ha = a * (s * 0.5);
            let hb = b * (s * 0.5);
            let base = mesh.positions.len() as u32;
            let q = [mid - ha - hb, mid + ha - hb, mid + ha + hb, mid - ha + hb];
            mesh.positions.extend_from_slice(&q.map(|p| p.to_array()));
            mesh.normals.extend_from_slice(&[nrm; 4]);
            let (u0, u1) = if h & (1 << 40) != 0 { (c.tile[1], c.tile[0]) } else { (c.tile[0], c.tile[1]) };
            mesh.uvs.extend_from_slice(&[[u0, 1.0], [u0, 0.0], [u1, 0.0], [u1, 1.0]]);
            mesh.colors.extend_from_slice(&[c.color; 4]);
            // q0,q1,q2 is CCW around a x b. Emit CW around the chosen face normal.
            if flipped {
                mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            } else {
                mesh.indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
            }
            tris += 2;
        }
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(parent: u32, start: Vec3, end: Vec3, radius: f32, continues: bool) -> TubeSeg {
        TubeSeg { parent, start, end, radius, continues, color: [0.5; 4], tip_taper: 0.5, path0: 0.0 }
    }

    /// Buttress lobes peak at the supplied world azimuths (over the main
    /// lateral roots).
    #[test]
    fn buttress_lobes_follow_roots() {
        let mut lobe_azimuths = [0.0; 8];
        lobe_azimuths[0] = 0.0; // +x
        lobe_azimuths[1] = std::f32::consts::FRAC_PI_2; // +z
        lobe_azimuths[2] = std::f32::consts::PI; // -x
        let style = TubeStyle { root_flare: 0.8, root_flare_height: 1.0, buttress_lobes: 3, lobe_azimuths, lobe_count: 3, ..Default::default() };
        let segs = [seg(NONE, Vec3::ZERO, Vec3::new(0.0, 4.0, 0.0), 0.5, false)];
        let m = build_tubes(&segs, 100_000, 0.0, style);
        // Widest ground-level vertex per direction.
        let reach = |dx: f32, dz: f32| {
            m.positions
                .iter()
                .filter(|p| p[1] < 0.05)
                .map(|p| p[0] * dx + p[2] * dz)
                .fold(0.0, f32::max)
        };
        let on_lobe = reach(1.0, 0.0).min(reach(0.0, 1.0)).min(reach(-1.0, 0.0));
        let off_lobe = reach(0.0, -1.0);
        assert!(on_lobe > off_lobe * 1.2, "lobes {on_lobe} vs gap {off_lobe}");
    }

    /// A thin twig on a thick stem must stay thin at its collar (no wide
    /// cone/sheet hidden inside, or poking out of, the stem).
    #[test]
    fn collar_ring_bounded_by_branch_radius() {
        let style = TubeStyle { collar_flare: 0.25, root_flare: 0.7, root_flare_height: 1.5, buttress_lobes: 5, ..Default::default() };
        let segs = [
            seg(NONE, Vec3::ZERO, Vec3::new(0.0, 3.0, 0.0), 0.5, false),
            seg(0, Vec3::new(0.0, 3.0, 0.0), Vec3::new(0.0, 4.0, 0.0), 0.45, true),
            seg(0, Vec3::new(0.0, 3.0, 0.0), Vec3::new(1.0, 3.2, 0.0), 0.004, false),
        ];
        let mut rings = Vec::new();
        axis_rings(&segs, &[2], &style, &mut rings);
        for r in &rings {
            assert!(r.radius <= 0.004 * 1.25 + 1e-6, "collar ring radius {}", r.radius);
        }
        let mesh = build_tubes(&segs, 100_000, 0.0, style);
        // Every vertex lies within the flared radius of the root axis or
        // within reach of the segments.
        for p in &mesh.positions {
            let h = (p[0] * p[0] + p[2] * p[2]).sqrt();
            assert!(h <= 0.5 * 1.7 + 1.01, "vertex too far out: {p:?}");
        }
    }
}
