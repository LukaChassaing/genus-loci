//! Engine-agnostic mesh output.
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
//!   (square texels for a tiling bark texture). Vertex colour = bark tint.
//! * Leaves: single-sided quads ("cards"); render with back-face culling
//!   disabled and alpha-scissor on a leaf-cluster texture. UVs cover 0..1 on
//!   each card (v = 0 at the card's distal end). Normals are bent outward
//!   from the crown centre (standard foliage-shading trick). Vertex colour RGB
//!   is the seasonal leaf colour, A = 1.

use crate::math::Vec3;
use crate::rng;

/// Indexed triangle mesh.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
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
}

const NONE: u32 = u32::MAX;

fn radial_segments(r: f32) -> usize {
    (3.0 + r * 70.0).clamp(3.0, 18.0) as usize
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
    let mut used = 0usize;
    let mut chain: Vec<usize> = Vec::new();
    let mut rings: Vec<Ring> = Vec::new();
    for &(r0, start, count) in &axes {
        let root_axis = segs[start as usize].parent == NONE;
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
        used += emit_rings(&mut mesh, &rings, radial, if root_axis { style } else { TubeStyle::default() });
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

fn emit_rings(mesh: &mut MeshData, rings: &[Ring], radial: usize, flare: TubeStyle) -> usize {
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
            let (s, c) = th.sin_cos();
            let dirv = normal * c + binormal * s;
            let mut r = ring.radius;
            if w > 0.0 {
                let lobe = if flare.buttress_lobes > 0 {
                    let x = 0.5 + 0.5 * (th * flare.buttress_lobes as f32).cos();
                    0.35 + 0.65 * x * x
                } else {
                    1.0
                };
                r *= 1.0 + flare.root_flare * w * lobe;
            }
            mesh.vert(ring.center + dirv * r, dirv, [j as f32 / radial as f32, v], ring.color);
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
    pub pos: Vec3,
    pub dir: Vec3,
    pub size: f32,
    pub color: [f32; 4],
    pub key: u64,
}

/// Crossed cards per leaf cluster.
pub(crate) const CARDS_PER_CLUSTER: usize = 3;

/// Build leaf cards; at most `tri_budget` triangles (clusters are thinned
/// deterministically and enlarged to compensate).
pub(crate) fn build_leaves(clusters: &[LeafCluster], tri_budget: usize) -> MeshData {
    let mut mesh = MeshData::default();
    if clusters.is_empty() {
        return mesh;
    }
    let max_clusters = tri_budget / (CARDS_PER_CLUSTER * 2);
    let keep = if clusters.len() > max_clusters { max_clusters as f32 / clusters.len() as f32 } else { 1.0 };
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
        if tris + CARDS_PER_CLUSTER * 2 > tri_budget {
            break;
        }
        let s = c.size * scale;
        let outward = (c.pos - center).normalize_or(Vec3::Y);
        let dir = c.dir.normalize_or(Vec3::Y);
        let perp = dir.any_perpendicular();
        let perp2 = dir.cross(perp);
        let spin = rng::unit(rng::mix(c.key)) * std::f32::consts::PI;
        for k in 0..CARDS_PER_CLUSTER {
            let ang = spin + k as f32 * std::f32::consts::PI / CARDS_PER_CLUSTER as f32;
            let (sa, ca) = ang.sin_cos();
            let b = perp * ca + perp2 * sa; // card width axis
            // Card length axis: along the shoot, tilted by a varied per-card
            // amount so the cluster reads as a volume from all sides.
            let tilt = 0.35 * (k as f32 - 1.0) + 0.5 * (rng::unit(rng::mix(c.key ^ (0x71 + k as u64))) - 0.5);
            let a = (dir + b.cross(dir) * tilt).normalize_or(dir);
            let mid = c.pos + a * (s * 0.25);
            let mut face = a.cross(b).normalize_or(outward);
            let flipped = face.dot(outward) < 0.0;
            if flipped {
                face = -face;
            }
            let nrm = (face * 0.35 + outward * 0.65).normalize_or(face);
            let ha = a * (s * 0.5);
            let hb = b * (s * 0.5);
            let base = mesh.positions.len() as u32;
            let q = [mid - ha - hb, mid + ha - hb, mid + ha + hb, mid - ha + hb];
            let uv = [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];
            for (p, t) in q.iter().zip(uv) {
                mesh.vert(*p, nrm, t, c.color);
            }
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
        TubeSeg { parent, start, end, radius, continues, color: [0.5; 4], tip_taper: 0.5 }
    }

    /// A thin twig on a thick stem must stay thin at its collar (no wide
    /// cone/sheet hidden inside, or poking out of, the stem).
    #[test]
    fn collar_ring_bounded_by_branch_radius() {
        let style = TubeStyle { collar_flare: 0.25, root_flare: 0.7, root_flare_height: 1.5, buttress_lobes: 5 };
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
