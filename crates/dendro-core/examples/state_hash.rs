//! Cross-platform determinism probe (roadmap M0).
//!
//! Grows every registry species (oak and cherry first) for N years (default 100) with a fixed seed and prints
//! one FNV-1a 64-bit hash per tree over the bit patterns of the public output
//! (tree stats, rings, wood/leaf/root/fine-root meshes). CI runs it on Linux and
//! Windows and fails if the lines differ. No golden value is stored: the
//! calibration changes, only equality between platforms matters.
//!
//!   cargo run -p dendro-core --release --example state_hash -- [--years 100] [--seed 42]
//!
//! Two hashes per tree: `hash` (self-contained, over the public output) and
//! `state` (`Simulation::state_hash()` from save.rs, over the internal state).

use dendro_core::{Genome, MeshData, Simulation};

struct Fnv(u64);
impl Fnv {
    fn new() -> Self {
        Fnv(0xcbf29ce484222325)
    }
    fn bytes(&mut self, b: &[u8]) {
        for &x in b {
            self.0 ^= x as u64;
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
    fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.u32(v.to_bits());
    }
    fn mesh(&mut self, m: &MeshData) {
        self.u32(m.positions.len() as u32);
        for p in &m.positions {
            p.iter().for_each(|&v| self.f32(v));
        }
        for n in &m.normals {
            n.iter().for_each(|&v| self.f32(v));
        }
        for u in &m.uvs {
            u.iter().for_each(|&v| self.f32(v));
        }
        for c in &m.colors {
            c.iter().for_each(|&v| self.f32(v));
        }
        for u in &m.uv2 {
            u.iter().for_each(|&v| self.f32(v));
        }
        m.indices.iter().for_each(|&i| self.u32(i));
    }
}

fn run(name: &str, genome: Genome, seed: u64, years: u32) {
    let mut sim = Simulation::new(genome, seed);
    sim.advance_days(365 * years);
    let s = sim.stats();
    let mut h = Fnv::new();
    for v in [s.age_years, s.height_m, s.dbh_cm, s.crown_radius_m, s.root_depth_m, s.root_spread_m] {
        h.f32(v);
    }
    for n in [s.shoot_nodes, s.leafy_shoots, s.root_nodes, s.dead_wood_nodes] {
        h.u32(n as u32);
    }
    for r in sim.rings() {
        h.u32(r.year);
        for v in [r.width_mm, r.earlywood_mm, r.latewood_mm, r.earlywood_fraction, r.water_stress] {
            h.f32(v);
        }
    }
    let (w, l, r, f) = (sim.wood_mesh(), sim.leaf_mesh(), sim.root_mesh(), sim.fine_root_mesh());
    for m in [&w, &l, &r, &f] {
        h.mesh(m);
    }
    println!(
        "{name} seed={seed} years={years} height={:.4} dbh={:.4} rings={} wood_tris={} hash={:016x} state={:016x}",
        s.height_m,
        s.dbh_cm,
        sim.rings().len(),
        w.triangle_count(),
        h.0,
        sim.state_hash()
    );
}

fn main() {
    let mut years = 100u32;
    let mut seed = 42u64;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--years" => years = args.next().unwrap().parse().unwrap(),
            "--seed" => seed = args.next().unwrap().parse().unwrap(),
            other => panic!("unknown argument {other}"),
        }
    }
    // Oak and cherry first (their lines are the historical ones), then every
    // other registry species.
    run("quercus_robur", Genome::quercus_robur(), seed, years);
    run("prunus_avium", Genome::prunus_avium(), seed, years);
    for g in Genome::builtin() {
        if g.id != "quercus_robur" && g.id != "prunus_avium" {
            run(&g.id.clone(), g, seed, years);
        }
    }
}
