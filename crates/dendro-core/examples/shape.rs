//! Calibration and architecture table for several seeds.
//!
//! cargo run -p dendro-core --release --example shape -- [--seeds 42,1,1234] [--years 200]

use dendro_core::{Genome, Simulation};
use std::time::Instant;

fn main() {
    let mut seeds = vec![42u64, 1, 1234];
    let mut years = 200u32;
    let mut genome = Genome::quercus_robur();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--seeds" => seeds = args.next().unwrap().split(',').map(|s| s.parse().unwrap()).collect(),
            "--years" => years = args.next().unwrap().parse().unwrap(),
            "--genome" => {
                let path = args.next().unwrap();
                genome = Genome::from_ron(&std::fs::read_to_string(&path).expect("read genome")).expect("parse genome");
            }
            other => panic!("unknown argument {other}"),
        }
    }
    let marks = [5u32, 15, 25, 40, 60, 80, 100, 150, 200];
    for seed in seeds {
        println!("seed {seed}");
        println!(
            "{:>4} {:>6} {:>6} {:>5} {:>5} {:>5} {:>5} {:>5} {:>6} {:>6}",
            "age", "H", "DBH", "W/H", "bole", "limbs", "maxR", "meanR", "nodes", "t(s)"
        );
        let mut sim = Simulation::new(genome.clone(), seed);
        let t0 = Instant::now();
        for y in 1..=years {
            sim.advance_years(1);
            if marks.contains(&y) {
                let s = sim.stats();
                let m = sim.shape_metrics();
                println!(
                    "{:>4} {:>6.2} {:>6.1} {:>5.2} {:>5.2} {:>5} {:>5.2} {:>5.2} {:>6} {:>6.2}",
                    y,
                    s.height_m,
                    s.dbh_cm,
                    m.crown_width_to_height,
                    m.clear_bole_m,
                    m.scaffold_limbs,
                    m.max_limb_ratio,
                    m.mean_limb_ratio,
                    s.shoot_nodes,
                    t0.elapsed().as_secs_f64()
                );
            }
        }
    }
}
