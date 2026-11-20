//! Cost of the wind acclimation: a tree grown with and without it.
//!
//! cargo run -p dendro-core --release --example wind_cost -- [species] [years] [seed]

use dendro_core::{Genome, Simulation};
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let id = a.first().cloned().unwrap_or_else(|| "quercus_robur".into());
    let years: u32 = a.get(1).map(|s| s.parse().unwrap()).unwrap_or(200);
    let seed: u64 = a.get(2).map(|s| s.parse().unwrap()).unwrap_or(42);
    for accl in [false, true] {
        let mut s = Simulation::new(Genome::by_id(&id).unwrap(), seed);
        s.set_wind_acclimation_enabled(accl);
        let t = Instant::now();
        s.advance_years(years);
        println!("{id} acclimation {accl}: {:.2} s, screens {:?}, DBH {:.1} H {:.1} segments {}", t.elapsed().as_secs_f32(), s.wind_costs(), s.dbh_cm(), s.height_m(), s.shoot_nodes().len());
    }
}
