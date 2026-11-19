//! Oak shape metrics at 80, 100 and 200 years, with and without the wind
//! acclimation of the stem (diagnostic of `oak_architecture`).
//!
//! cargo run -p dendro-core --release --example shape_probe -- [seed]

use dendro_core::{Genome, Simulation};

fn main() {
    let seed: u64 = std::env::args().nth(1).map(|s| s.parse().unwrap()).unwrap_or(42);
    for accl in [false, true] {
        let mut s = Simulation::new(Genome::quercus_robur(), seed);
        s.set_wind_acclimation_enabled(accl);
        let mut done = 0;
        for age in [80u32, 100, 200] {
            s.advance_years(age - done);
            done = age;
            println!("acclimation {accl} {age} y: H {:.2} DBH {:.1} {:?}", s.height_m(), s.dbh_cm(), s.shape_metrics());
        }
    }
}
