//! Godot mesh surfaces encoded in Rust, and the worker thread that builds them.
//!
//! `ArrayMesh.add_surface_from_arrays` converts every vertex in C++ (octahedral normals, a dummy
//! tangent per normal, 8-bit colours), which costs ~70 ms for the 640 000 leaf-card vertices of a
//! 150-year-old oak, on the main thread. Here the same bytes are produced in Rust (on a worker thread
//! when it matters) and handed to Godot as a ready surface (`ArrayMesh._surfaces`, the path Godot uses
//! to load a saved mesh), which only copies them to the GPU (~6 ms for the same leaves).
//!
//! The encoding reproduces Godot 4.7's `RenderingServer::mesh_create_surface_data_from_arrays` for
//! the attributes `MeshData` carries (uncompressed, format version 2); `TreeSim.debug_compare_surfaces`
//! checks it byte for byte against Godot's own conversion (UI self-test).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};

use dendro_core::{MeshData, Simulation};
use godot::classes::ArrayMesh;
use godot::prelude::*;

// RenderingServer::ArrayFormat bits.
const F_VERTEX: i64 = 1;
const F_NORMAL: i64 = 1 << 1;
const F_TANGENT: i64 = 1 << 2;
const F_COLOR: i64 = 1 << 3;
const F_TEX_UV: i64 = 1 << 4;
const F_TEX_UV2: i64 = 1 << 5;
const F_INDEX: i64 = 1 << 12;
const F_VERSION_2: i64 = 1 << 35;

/// Largest vertex count with 16-bit indices (asked to Godot once on the main thread, see
/// `init_index_threshold`; Godot 4.7 uses 2-byte indices up to 65 536 vertices).
static INDEX16_MAX_VERTS: AtomicUsize = AtomicUsize::new(65_536);

/// Reads the 16-bit index threshold from `RenderingServer` (main thread, once).
pub fn init_index_threshold() {
    use godot::classes::RenderingServer;
    let rs = RenderingServer::singleton();
    let fmt = (F_VERTEX | F_INDEX | F_VERSION_2) as u64;
    let fmt = godot::classes::rendering_server::ArrayFormat::from_ord(fmt);
    // Binary search between 1 and 2^20 for the last count with a 2-byte stride.
    let (mut lo, mut hi) = (1u32, 1u32 << 20);
    if rs.mesh_surface_get_format_index_stride(fmt, lo as i32) != 2 {
        return;
    }
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if rs.mesh_surface_get_format_index_stride(fmt, mid as i32) == 2 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    INDEX16_MAX_VERTS.store(lo as usize, Ordering::Relaxed);
}

/// One mesh surface in Godot's storage format.
#[derive(Default)]
pub struct RawSurface {
    pub format: i64,
    pub vertex: Vec<u8>,
    pub attribute: Vec<u8>,
    pub index: Vec<u8>,
    pub vertex_count: usize,
    pub index_count: usize,
    /// position x, y, z, size x, y, z.
    pub aabb: [f32; 6],
}

/// Godot's `Vector3::octahedron_encode` (f32, same operation order).
fn oct_encode(v: [f32; 3]) -> [f32; 2] {
    let s = v[0].abs() + v[1].abs() + v[2].abs();
    let n = [v[0] / s, v[1] / s, v[2] / s];
    let (mut x, mut y) = if n[2] >= 0.0 {
        (n[0], n[1])
    } else {
        (
            (1.0 - n[1].abs()) * if n[0] >= 0.0 { 1.0 } else { -1.0 },
            (1.0 - n[0].abs()) * if n[1] >= 0.0 { 1.0 } else { -1.0 },
        )
    };
    x = x * 0.5 + 0.5;
    y = y * 0.5 + 0.5;
    [x, y]
}

/// `(uint16_t)CLAMP(v * 65535, 0, 65535)`.
fn unorm16(v: f32) -> u16 {
    (v * 65535.0).clamp(0.0, 65535.0) as u16
}

/// Godot's `Vector3::normalized` (f32).
fn normalized(v: [f32; 3]) -> [f32; 3] {
    let l2 = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    if l2 == 0.0 {
        return [0.0; 3];
    }
    let l = l2.sqrt();
    [v[0] / l, v[1] / l, v[2] / l]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// Normal (octahedral) and the dummy tangent Godot derives from it when none is given
/// (`Vector3(n.z, -n.x, n.y).cross(n.normalized()).normalized()`, sign +1), 8 bytes.
fn encode_normal_tangent(n: [f32; 3], out: &mut Vec<u8>) {
    let e = oct_encode(n);
    out.extend_from_slice(&unorm16(e[0]).to_le_bytes());
    out.extend_from_slice(&unorm16(e[1]).to_le_bytes());
    let t = normalized(cross([n[2], -n[0], n[1]], normalized(n)));
    // Vector3::octahedron_tangent_encode(+1).
    let mut e = oct_encode(t);
    e[1] = e[1].max(1.0 / 32767.0) * 0.5 + 0.5;
    let mut v = [unorm16(e[0]), unorm16(e[1])];
    if v[0] == 0 && v[1] == 65535 {
        v[0] = 65535; // Godot's sanitising of the (0, 1) / (1, 1) ambiguity
    }
    out.extend_from_slice(&v[0].to_le_bytes());
    out.extend_from_slice(&v[1].to_le_bytes());
}

/// `uint8_t(CLAMP(c * 255.0, 0.0, 255.0))` (double product, truncation).
fn unorm8(c: f32) -> u8 {
    (c as f64 * 255.0).clamp(0.0, 255.0) as u8
}

/// Encodes `data` (non-empty, triangles) as Godot stores it.
pub fn encode(data: &MeshData) -> RawSurface {
    let n = data.positions.len();
    let has_n = data.normals.len() == n;
    let has_c = data.colors.len() == n;
    let has_uv = data.uvs.len() == n;
    let has_uv2 = data.uv2.len() == n;
    let mut format = F_VERTEX | F_INDEX | F_VERSION_2;
    if has_n {
        format |= F_NORMAL | F_TANGENT;
    }
    if has_c {
        format |= F_COLOR;
    }
    if has_uv {
        format |= F_TEX_UV;
    }
    if has_uv2 {
        format |= F_TEX_UV2;
    }
    let mut vertex = Vec::with_capacity(n * if has_n { 20 } else { 12 });
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for p in &data.positions {
        for k in 0..3 {
            vertex.extend_from_slice(&p[k].to_le_bytes());
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    // Godot's AABB is the min / max box, with a flat axis given CMP_EPSILON of thickness.
    let size = [0, 1, 2].map(|k| if hi[k] - lo[k] == 0.0 { 1e-5 } else { hi[k] - lo[k] });
    if has_n {
        for &nm in &data.normals {
            encode_normal_tangent(nm, &mut vertex);
        }
    }
    let stride = 4 * has_c as usize + 8 * has_uv as usize + 8 * has_uv2 as usize;
    let mut attribute = Vec::with_capacity(n * stride);
    for i in 0..n {
        if has_c {
            attribute.extend(data.colors[i].iter().map(|&c| unorm8(c)));
        }
        if has_uv {
            attribute.extend_from_slice(&data.uvs[i][0].to_le_bytes());
            attribute.extend_from_slice(&data.uvs[i][1].to_le_bytes());
        }
        if has_uv2 {
            attribute.extend_from_slice(&data.uv2[i][0].to_le_bytes());
            attribute.extend_from_slice(&data.uv2[i][1].to_le_bytes());
        }
    }
    let index = if n <= INDEX16_MAX_VERTS.load(Ordering::Relaxed) {
        data.indices.iter().flat_map(|&i| (i as u16).to_le_bytes()).collect()
    } else {
        data.indices.iter().flat_map(|&i| i.to_le_bytes()).collect()
    };
    RawSurface {
        format,
        vertex,
        attribute,
        index,
        vertex_count: n,
        index_count: data.indices.len(),
        aabb: [lo[0], lo[1], lo[2], size[0], size[1], size[2]],
    }
}

/// The surface as the dictionary `ArrayMesh._surfaces` / `RenderingServer.mesh_add_surface` take.
fn surface_dict(s: &RawSurface) -> VarDictionary {
    let mut d = VarDictionary::new();
    d.set("format", s.format);
    d.set("primitive", 3i64); // PRIMITIVE_TRIANGLES
    d.set("vertex_data", &PackedByteArray::from(s.vertex.as_slice()));
    d.set("vertex_count", s.vertex_count as i64);
    if !s.attribute.is_empty() {
        d.set("attribute_data", &PackedByteArray::from(s.attribute.as_slice()));
    }
    let a = s.aabb;
    d.set("aabb", Aabb::new(Vector3::new(a[0], a[1], a[2]), Vector3::new(a[3], a[4], a[5])));
    d.set("uv_scale", Vector4::ZERO);
    d.set("index_data", &PackedByteArray::from(s.index.as_slice()));
    d.set("index_count", s.index_count as i64);
    d
}

/// An `ArrayMesh` holding the surface (none when `s` is `None` or empty).
pub fn to_array_mesh(s: Option<&RawSurface>) -> Gd<ArrayMesh> {
    let mut mesh = ArrayMesh::new_gd();
    if let Some(s) = s.filter(|s| s.vertex_count > 0) {
        let mut a = VarArray::new();
        a.push(&surface_dict(s).to_variant());
        mesh.set("_surfaces", &a.to_variant());
    }
    mesh
}

/// The meshes `main.gd` keeps in step with the simulation (bit `1 << kind` in the masks).
pub const WOOD: usize = 0;
pub const LEAF: usize = 1;
pub const ROOT: usize = 2; // structural roots + fine-root tufts
pub const FLOWER: usize = 3;
pub const KINDS: usize = 4;

/// Builds the meshes of `kinds` (bit mask) from `sim` and encodes them: one surface per kind, plus the
/// fine roots (index `KINDS`).
fn build_all(sim: &Simulation, kinds: u32) -> Vec<Option<RawSurface>> {
    let enc = |m: MeshData| if m.is_empty() { None } else { Some(encode(&m)) };
    let mut out: Vec<Option<RawSurface>> = (0..=KINDS).map(|_| None).collect();
    if kinds & (1 << WOOD) != 0 {
        out[WOOD] = enc(sim.wood_mesh());
    }
    if kinds & (1 << LEAF) != 0 {
        out[LEAF] = enc(sim.leaf_mesh());
    }
    if kinds & (1 << ROOT) != 0 {
        out[ROOT] = enc(sim.root_mesh());
        out[KINDS] = enc(sim.fine_root_mesh());
    }
    if kinds & (1 << FLOWER) != 0 {
        out[FLOWER] = enc(sim.flower_mesh());
    }
    out
}

struct Job {
    generation: u64,
    kinds: u32,
    versions: [i64; KINDS],
    sim: Box<Simulation>,
}

/// A finished build: the encoded surfaces of `kinds` and the mesh versions of the snapshot they show.
pub struct Built {
    generation: u64,
    pub kinds: u32,
    pub versions: [i64; KINDS],
    pub surfaces: Vec<Option<RawSurface>>,
    /// Milliseconds the worker spent (build + encode).
    pub millis: f64,
}

/// Builds meshes from a snapshot (clone) of the simulation on one worker thread, one job at a time,
/// so the main thread only uploads the result. The simulation itself is never touched by the
/// worker: it reads its own copy, so threading cannot change simulation results.
#[derive(Default)]
pub struct MeshWorker {
    tx: Option<Sender<Job>>,
    /// Joined on drop: the thread runs code of this library, which Godot unloads at exit.
    thread: Option<std::thread::JoinHandle<()>>,
    rx: Option<Receiver<Built>>,
    busy: bool,
    /// Bumped whenever the live tree is replaced or edited out of sequence (reset, load, cut, undo):
    /// a job started before is discarded when it lands.
    generation: u64,
    ready: Option<Built>,
}

impl MeshWorker {
    fn start(&mut self) {
        let (job_tx, job_rx) = channel::<Job>();
        let (res_tx, res_rx) = channel::<Built>();
        let thread = std::thread::Builder::new()
            .name("dendro-meshes".into())
            .spawn(move || {
                // Ends when the TreeSim (and so the job sender) is dropped.
                while let Ok(job) = job_rx.recv() {
                    let t0 = std::time::Instant::now();
                    let surfaces = build_all(&job.sim, job.kinds);
                    let built = Built {
                        generation: job.generation,
                        kinds: job.kinds,
                        versions: job.versions,
                        surfaces,
                        millis: t0.elapsed().as_secs_f64() * 1000.0,
                    };
                    if res_tx.send(built).is_err() {
                        break;
                    }
                }
            })
            .expect("mesh worker thread");
        self.thread = Some(thread);
        self.tx = Some(job_tx);
        self.rx = Some(res_rx);
    }

    /// Starts building `kinds` from a copy of `sim` unless a build is already running or a result is
    /// waiting to be taken. Returns whether a job was started.
    pub fn request(&mut self, sim: &Simulation, kinds: u32, versions: [i64; KINDS]) -> bool {
        if self.busy || self.ready.is_some() || kinds == 0 {
            return false;
        }
        if self.tx.is_none() {
            self.start();
        }
        let job = Job { generation: self.generation, kinds, versions, sim: Box::new(sim.clone()) };
        if self.tx.as_ref().is_some_and(|tx| tx.send(job).is_ok()) {
            self.busy = true;
            true
        } else {
            false
        }
    }

    /// Collects a finished job (stale ones are dropped). Returns the mask of kinds ready to take.
    pub fn poll(&mut self) -> u32 {
        use std::sync::mpsc::TryRecvError;
        if self.busy {
            match self.rx.as_ref().map(|rx| rx.try_recv()) {
                Some(Ok(b)) => {
                    self.busy = false;
                    if b.generation == self.generation {
                        self.ready = Some(b);
                    }
                }
                Some(Err(TryRecvError::Empty)) => {}
                // The worker died (a panic in a mesh builder): forget it, the next request starts a new one.
                _ => {
                    godot_warn!("dendro: mesh worker thread stopped; restarting it");
                    self.busy = false;
                    self.tx = None;
                    self.rx = None;
                    if let Some(t) = self.thread.take() {
                        let _ = t.join();
                    }
                }
            }
        }
        self.ready.as_ref().map_or(0, |b| b.kinds)
    }

    pub fn busy(&self) -> bool {
        self.busy
    }

    /// Takes the kinds of `mask` out of the finished build (after `poll` reported it); the rest stays
    /// waiting, so a big build can be installed over several frames.
    pub fn take(&mut self, mask: u32) -> Option<Built> {
        let b = self.ready.as_mut()?;
        let mask = mask & b.kinds;
        if mask == 0 {
            return None;
        }
        let mut part = Built {
            generation: b.generation,
            kinds: mask,
            versions: b.versions,
            surfaces: (0..=KINDS).map(|_| None).collect(),
            millis: std::mem::take(&mut b.millis),
        };
        for k in 0..KINDS {
            if mask & (1 << k) != 0 {
                part.surfaces[k] = b.surfaces[k].take();
                if k == ROOT {
                    part.surfaces[KINDS] = b.surfaces[KINDS].take();
                }
            }
        }
        b.kinds &= !mask;
        if b.kinds == 0 {
            self.ready = None;
        }
        Some(part)
    }

    /// Discards the result of the job in flight and any result waiting (the tree changed).
    pub fn invalidate(&mut self) {
        self.generation += 1;
        self.ready = None;
    }
}

impl Drop for MeshWorker {
    fn drop(&mut self) {
        // Closing the job channel ends the thread after the job in flight (~0.1 s at most).
        self.tx = None;
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
