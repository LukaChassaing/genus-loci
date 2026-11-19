//! Diagnostics: carbon balance, reserves, vigour and death of old trees
//! (`crate::vitality`).
//!
//! cargo run -p dendro-core --release --example ageing -- --species quercus_robur [--seed N] [--years N] [--cost X] [--every N] [--quiet] [--stop]
//!
//! `--cost X` overrides `LongevityTraits::maintenance_per_m` (calibration).
//! Prints every `--every` years (default 10): height, DBH, collar BAI
//! (cm2/y, 5-year mean), crown radius, maintenance / production ratio, net
//! production, reserves, vigour, death probability, branches killed, dead
//! wood nodes; and the age of trunk death. `--quiet` prints the summary only; `--stop` stops at trunk death.

use dendro_core::{Genome, Simulation};

fn main() {
    let mut genome = Genome::quercus_robur();
    let (mut seed, mut years, mut every, mut quiet, mut stop) = (1u64, 400u32, 10u32, false, false);
    let mut cost: Option<f32> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--species" => {
                let id = args.next().unwrap_or_default();
                genome = Genome::find(&id).unwrap_or_else(|| panic!("unknown species {id}"));
            }
            "--seed" => seed = args.next().unwrap().parse().unwrap(),
            "--years" => years = args.next().unwrap().parse().unwrap(),
            "--every" => every = args.next().unwrap().parse().unwrap(),
            "--cost" => cost = Some(args.next().unwrap().parse().unwrap()),
            "--quiet" => quiet = true,
            "--stop" => stop = true,
            o => panic!("unknown {o}"),
        }
    }
    if let Some(c) = cost {
        genome.longevity.maintenance_per_m = c;
    }
    let mut sim = Simulation::new(genome.clone(), seed);
    if !quiet {
        println!("{} seed {seed} maintenance_per_m {}", genome.species, genome.longevity.maintenance_per_m);
        println!("{:>4} {:>5} {:>6} {:>6} {:>5} {:>5} {:>5} {:>5} {:>5} {:>6} {:>4} {:>6} {:>6}", "age", "H", "DBH", "BAI", "CR", "rho", "net", "res", "vig", "p%", "kill", "snags", "D/F");
    }
    let mut ba = Vec::new();
    let mut max_dbh = 0.0f32;
    for age in 1..=years {
        sim.advance_years(1);
        let s = sim.stats();
        let d = sim.collar_diameter_cm();
        ba.push(std::f32::consts::PI * 0.25 * d * d);
        max_dbh = max_dbh.max(s.dbh_cm);
        let v = sim.vitality();
        if !quiet && (age % every == 0 || age == years || v.death_age_years == Some(age)) {
            let n = ba.len();
            let bai = if n > 5 { (ba[n - 1] - ba[n - 6]) / 5.0 } else { 0.0 };
            println!(
                "{:>4} {:>5.1} {:>6.1} {:>6.1} {:>5.1} {:>5.2} {:>5.2} {:>5.2} {:>5.2} {:>6.2} {:>4} {:>6} {:>6.1}{}",
                age,
                s.height_m,
                s.dbh_cm,
                bai,
                s.crown_radius_m,
                v.cost_ratio,
                v.net_production,
                v.reserve,
                v.vigour,
                100.0 * v.death_probability,
                v.dieback_branches,
                s.dead_wood_nodes,
                if genome.longevity.maintenance_per_m > 0.0 { v.cost_ratio / genome.longevity.maintenance_per_m } else { 0.0 },
                if !sim.trunk_alive() { if v.standing_dead { " mort sur pied" } else { " effondré" } } else { "" }
            );
        }
        if (stop && !sim.trunk_alive()) || (!sim.trunk_alive() && v.collapsed && !sim.is_alive()) {
            break;
        }
    }
    let v = sim.vitality();
    println!(
        "{} seed {seed}: death {:?} ({:?}), max DBH {:.1} cm, resprouted {}",
        genome.id, v.death_age_years, v.cause, max_dbh, v.resprouted
    );
}
