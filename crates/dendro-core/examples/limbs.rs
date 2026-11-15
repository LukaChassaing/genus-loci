//! Diagnostics: first-order limbs of a tree at a given age.
//!
//! cargo run -p dendro-core --release --example limbs -- [--genome PATH] [--seed N] [--years N]

use dendro_core::{Genome, Simulation};

fn main() {
    let mut genome = Genome::prunus_avium();
    let (mut seed, mut years) = (42u64, 40u32);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--genome" => genome = Genome::from_ron(&std::fs::read_to_string(args.next().unwrap()).unwrap()).unwrap(),
            "--oak" => genome = Genome::quercus_robur(),
            "--seed" => seed = args.next().unwrap().parse().unwrap(),
            "--years" => years = args.next().unwrap().parse().unwrap(),
            o => panic!("unknown {o}"),
        }
    }
    let mut sim = Simulation::new(genome, seed);
    sim.advance_years(years);
    sim.advance_days(200);
    let s = sim.stats();
    println!("H {:.2} dbh {:.1} crownR {:.2} leafy {}", s.height_m, s.dbh_cm, s.crown_radius_m, s.leafy_shoots);
    println!("{:>6} {:>6} {:>6} {:>5} {:>5} {:>4}", "baseY", "r(cm)", "reach", "tips", "light", "age");
    let mut l = sim.debug_limbs();
    l.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (y, r, reach, tips, light, age) in l {
        println!("{:>6.2} {:>6.2} {:>6.2} {:>5} {:>5.2} {:>4}", y, r * 100.0, reach, tips, light, age);
    }
}
