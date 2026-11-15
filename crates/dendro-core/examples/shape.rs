//! Calibration and architecture table for several seeds.
//!
//! cargo run -p dendro-core --release --example shape -- [--seeds 42,1,1234] [--years 200]

use dendro_core::{ClimateParams, Genome, SoilProfile, Simulation};
use std::time::Instant;

fn main() {
    let mut seeds = vec![42u64, 1, 1234];
    let mut years = 200u32;
    let mut genome = Genome::quercus_robur();
    let mut soil = SoilProfile::default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--seeds" => seeds = args.next().unwrap().split(',').map(|s| s.parse().unwrap()).collect(),
            "--years" => years = args.next().unwrap().parse().unwrap(),
            "--soil" => {
                let v = args.next().unwrap();
                soil = match v.as_str() {
                    "loam" => SoilProfile::brown_forest_soil(),
                    "rendzina" | "shallow" => SoilProfile::shallow_rendzina(),
                    "gley" | "clay" => SoilProfile::gley_clay(),
                    "sand" => SoilProfile::sandy_podzol(),
                    path => SoilProfile::from_ron(&std::fs::read_to_string(path).expect("read soil")).expect("parse soil"),
                };
            }
            "--genome" => {
                let path = args.next().unwrap();
                genome = Genome::from_ron(&std::fs::read_to_string(&path).expect("read genome")).expect("parse genome");
            }
            other => panic!("unknown argument {other}"),
        }
    }
    let marks = [5u32, 15, 25, 40, 60, 70, 80, 100, 150, 200, 250, 300];
    for seed in seeds {
        println!("seed {seed}");
        println!(
            "{:>4} {:>6} {:>6} {:>5} {:>5} {:>5} {:>5} {:>5} {:>6} {:>6} {:>5}",
            "age", "H", "DBH", "W/H", "bole", "limbs", "maxR", "meanR", "nodes", "t(s)", "dead"
        );
        let mut sim = Simulation::with_site(genome.clone(), seed, ClimateParams::default(), soil.clone());
        let t0 = Instant::now();
        for y in 1..=years {
            sim.advance_years(1);
            if marks.contains(&y) {
                let s = sim.stats();
                let m = sim.shape_metrics();
                println!(
                    "{:>4} {:>6.2} {:>6.1} {:>5.2} {:>5.2} {:>5} {:>5.2} {:>5.2} {:>6} {:>6.2} {:>5}",
                    y,
                    s.height_m,
                    s.dbh_cm,
                    m.crown_width_to_height,
                    m.clear_bole_m,
                    m.scaffold_limbs,
                    m.max_limb_ratio,
                    m.mean_limb_ratio,
                    s.shoot_nodes,
                    t0.elapsed().as_secs_f64(),
                    s.dead_wood_nodes
                );
            }
        }
        if std::env::var("PROFILE").is_ok() {
            for (d, r, o, k) in sim.debug_limb_profile() {
                print!("{d:.1}:{:.0}cm/o{o}/k{k} ", r * 200.0);
            }
            println!();
        }
        let r = sim.rings();
        let n = r.len().max(1) as f32;
        println!(
            "mean ring {:.2} mm, mean water stress {:.3}, root depth {:.2} m, spread {:.2} m",
            r.iter().map(|x| x.width_mm).sum::<f32>() / n,
            r.iter().map(|x| x.water_stress).sum::<f32>() / n,
            sim.root_depth_m(),
            sim.root_spread_m()
        );
    }
}
