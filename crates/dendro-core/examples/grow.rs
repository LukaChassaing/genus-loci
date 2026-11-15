//! Grow a tree and print its dimensions every 10 years.
//!
//! cargo run -p dendro-core --release --example grow -- --years 100 [--seed N]
//!     [--species oak|cherry] [--soil N] [--obj out.obj] [--drought-year Y]
//!     [--rings] [--pheno]

use dendro_core::{ClimateParams, Genome, Phase, ReproStage, Simulation, SoilProfile};
use std::time::Instant;

/// One year's phenology: (year, budburst, bloom onset, end of leaf fall,
/// fraction of flowers lost to late frost).
type YearPheno = (u32, Option<u32>, Option<u32>, Option<u32>, f32);

fn main() {
    let mut years = 100u32;
    let mut seed = 42u64;
    let mut obj: Option<String> = None;
    let mut drought_year: Option<u32> = None;
    let mut print_rings = false;
    let mut print_pheno = false;
    let mut species = "oak".to_string();
    let mut soil = 0usize;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--years" => years = args.next().and_then(|v| v.parse().ok()).expect("--years N"),
            "--seed" => seed = args.next().and_then(|v| v.parse().ok()).expect("--seed N"),
            "--species" => species = args.next().expect("--species oak|cherry"),
            "--soil" => soil = args.next().and_then(|v| v.parse().ok()).expect("--soil N (preset index)"),
            "--obj" => obj = Some(args.next().expect("--obj PATH")),
            "--drought-year" => drought_year = args.next().and_then(|v| v.parse().ok()),
            "--rings" => print_rings = true,
            "--pheno" => print_pheno = true,
            "-h" | "--help" => {
                println!(
                    "usage: grow [--years N] [--seed N] [--species oak|cherry] [--soil N] [--obj PATH] [--drought-year Y] [--rings] [--pheno]"
                );
                let names: Vec<String> = SoilProfile::presets().into_iter().map(|p| p.name).collect();
                println!("soil presets: {names:?}");
                return;
            }
            other => panic!("unknown argument {other}"),
        }
    }

    let genome = match species.as_str() {
        "oak" | "quercus" | "quercus_robur" => Genome::quercus_robur(),
        "cherry" | "prunus" | "prunus_avium" => Genome::prunus_avium(),
        other => panic!("unknown species {other} (oak|cherry)"),
    };
    let mut presets = SoilProfile::presets();
    assert!(soil < presets.len(), "soil index out of range (0..{})", presets.len());
    let profile = presets.swap_remove(soil);
    println!("{} ({}), seed {seed}, {years} years, soil: {}", genome.species, genome.common_name, profile.name);
    let new_sim = || Simulation::with_site(genome.clone(), seed, ClimateParams::default(), profile.clone());
    let mut sim = new_sim();
    println!(
        "{:>4} {:>7} {:>7} {:>7} {:>5} {:>7} {:>7} {:>7} {:>7} {:>6} {:>8} {:>8} {:>8} {:>7}",
        "age", "H(m)", "DBH(cm)", "crownR", "W/H", "rootD", "rootR", "nodes", "leafy", "dead", "woodTri", "rootTri", "leafTri", "ring"
    );
    let t0 = Instant::now();
    let mut mesh_time = 0.0f64;
    let mut pheno: Vec<YearPheno> = Vec::new();
    for y in 0..years {
        if Some(y) == drought_year {
            sim.set_drought(true);
        }
        if print_pheno {
            // Day by day: budburst, bloom onset and end of leaf fall.
            let (mut bb, mut bloom, mut fall, mut frost) = (None, None, None, 0.0f32);
            let mut prev = sim.phase();
            for _ in 0..365 {
                sim.advance_days(1);
                let d = sim.day_of_year();
                if bb.is_none() && sim.phase() == Phase::BudBurst {
                    bb = Some(d);
                }
                if bloom.is_none() && sim.repro_stage() == ReproStage::Blooming {
                    bloom = Some(d);
                }
                if prev == Phase::LeafFall && sim.phase() == Phase::Dormant {
                    fall = Some(d);
                }
                prev = sim.phase();
                frost = frost.max(sim.flower_frost_loss());
            }
            pheno.push((y, bb, bloom, fall, frost));
        } else {
            sim.advance_days(365);
        }
        if Some(y) == drought_year {
            sim.set_drought(false);
        }
        let age = y + 1;
        if age % 10 == 0 || age == 5 || age == 15 || age == 25 || age == years {
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
                "{:>4} {:>7.2} {:>7.1} {:>7.2} {:>5.2} {:>7.2} {:>7.2} {:>7} {:>7} {:>6} {:>8} {:>8} {:>8} {:>7.2}",
                age,
                s.height_m,
                s.dbh_cm,
                s.crown_radius_m,
                sim.shape_metrics().crown_width_to_height,
                s.root_depth_m,
                s.root_spread_m,
                s.shoot_nodes,
                s.leafy_shoots,
                s.dead_wood_nodes,
                wood.triangle_count(),
                roots.triangle_count(),
                leaves.triangle_count(),
                ring
            );
        }
    }
    let total = t0.elapsed().as_secs_f64();
    println!("simulation+reporting: {:.2} s total ({:.3} s spent in wood/root meshing)", total, mesh_time);

    if print_pheno {
        println!("year budburst bloom leaf-fall-end frost-loss (0-based day of year)");
        let fmt = |d: Option<u32>| d.map(|v| v.to_string()).unwrap_or_else(|| "-".into());
        for (y, bb, bl, fa, fl) in &pheno {
            println!("{y:>4} {:>8} {:>5} {:>13} {fl:>10.2}", fmt(*bb), fmt(*bl), fmt(*fa));
        }
    }

    // Pure simulation timing on a fresh run.
    let t = Instant::now();
    let mut fresh = new_sim();
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
