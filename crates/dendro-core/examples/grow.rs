//! Grow a tree and print its dimensions every 10 years.
//!
//! cargo run -p dendro-core --release --example grow -- --years 100 [--seed N]
//!     [--obj out.obj] [--drought-year Y] [--rings]

use dendro_core::{Genome, Simulation};
use std::time::Instant;

fn main() {
    let mut years = 100u32;
    let mut seed = 42u64;
    let mut obj: Option<String> = None;
    let mut drought_year: Option<u32> = None;
    let mut print_rings = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--years" => years = args.next().and_then(|v| v.parse().ok()).expect("--years N"),
            "--seed" => seed = args.next().and_then(|v| v.parse().ok()).expect("--seed N"),
            "--obj" => obj = Some(args.next().expect("--obj PATH")),
            "--drought-year" => drought_year = args.next().and_then(|v| v.parse().ok()),
            "--rings" => print_rings = true,
            "-h" | "--help" => {
                println!("usage: grow [--years N] [--seed N] [--obj PATH] [--drought-year Y] [--rings]");
                return;
            }
            other => panic!("unknown argument {other}"),
        }
    }

    let genome = Genome::quercus_robur();
    println!("{} ({}), seed {seed}, {years} years", genome.species, genome.common_name);
    let mut sim = Simulation::new(genome, seed);
    println!(
        "{:>4} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>8} {:>8} {:>8} {:>7}",
        "age", "H(m)", "DBH(cm)", "crownR", "rootD", "rootR", "nodes", "leafy", "woodTri", "rootTri", "leafTri", "ring"
    );
    let t0 = Instant::now();
    let mut mesh_time = 0.0f64;
    for y in 0..years {
        if Some(y) == drought_year {
            sim.set_drought(true);
        }
        sim.advance_days(365);
        if Some(y) == drought_year {
            sim.set_drought(false);
        }
        let age = y + 1;
        if age % 10 == 0 || age == 5 || age == 25 || age == years {
            // Meshes measured mid-summer of the next year would differ; use the
            // winter state here (leaf mesh then = marcescent only), plus a
            // summer leaf count from a cloned sim.
            let tm = Instant::now();
            let wood = sim.wood_mesh();
            let roots = sim.root_mesh();
            mesh_time += tm.elapsed().as_secs_f64();
            let mut summer = sim.clone();
            summer.advance_days(200);
            let leaves = summer.leaf_mesh();
            let s = sim.stats();
            let ring = sim.rings().last().map(|r| r.width_mm).unwrap_or(0.0);
            println!(
                "{:>4} {:>7.2} {:>7.1} {:>7.2} {:>7.2} {:>7.2} {:>7} {:>7} {:>8} {:>8} {:>8} {:>7.2}",
                age,
                s.height_m,
                s.dbh_cm,
                s.crown_radius_m,
                s.root_depth_m,
                s.root_spread_m,
                s.shoot_nodes,
                s.leafy_shoots,
                wood.triangle_count(),
                roots.triangle_count(),
                leaves.triangle_count(),
                ring
            );
        }
    }
    let total = t0.elapsed().as_secs_f64();
    println!("simulation+reporting: {:.2} s total ({:.3} s spent in wood/root meshing)", total, mesh_time);

    // Pure simulation timing on a fresh run.
    let t = Instant::now();
    let mut fresh = Simulation::new(Genome::quercus_robur(), seed);
    fresh.advance_days(365 * years);
    println!("pure simulation of {years} years: {:.3} s", t.elapsed().as_secs_f64());

    // Mesh build timing in full leaf (mid-summer of the next year).
    fresh.advance_days(200);
    let t = Instant::now();
    let wood = fresh.wood_mesh();
    let tw = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let leaves = fresh.leaf_mesh();
    let tl = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let roots = fresh.root_mesh();
    let tr = t.elapsed().as_secs_f64();
    println!(
        "mesh build (summer, age {:.1}): wood {} tris {:.1} ms, leaves {} tris {:.1} ms, roots {} tris {:.1} ms",
        fresh.age_years(),
        wood.triangle_count(),
        tw * 1e3,
        leaves.triangle_count(),
        tl * 1e3,
        roots.triangle_count(),
        tr * 1e3
    );
    println!("leaf colour {:?}, density {:.2}, phase {:?}", fresh.leaf_color(), fresh.leaf_density(), fresh.phase());

    if print_rings {
        println!("year  width  early  late  ef    stress drought");
        for r in sim.rings() {
            println!(
                "{:>4} {:>6.2} {:>6.2} {:>5.2} {:>5.2} {:>6.2} {}",
                r.year, r.width_mm, r.earlywood_mm, r.latewood_mm, r.earlywood_fraction, r.water_stress, r.forced_drought
            );
        }
    }

    if let Some(path) = obj {
        let mut s = String::new();
        let mut off = 0;
        for (name, m) in [("wood", &wood), ("leaves", &leaves), ("roots", &roots)] {
            s.push_str(&m.to_obj(name, off));
            off += m.vertex_count();
        }
        std::fs::write(&path, s).expect("write OBJ");
        println!("wrote {path}");
    }
}
