//! Death ages of seeds 1-5 with and without the wind acclimation of the
//! stem (diagnostic of the ageing tests).
//!
//! cargo run -p dendro-core --release --example life_probe -- <species> [years]

use dendro_core::{Genome, Simulation};

fn main() {
    let id = std::env::args().nth(1).expect("species");
    let years: u32 = std::env::args().nth(2).map(|s| s.parse().unwrap()).unwrap_or(200);
    for accl in [false, true] {
        let mut out = Vec::new();
        for seed in 1..=5u64 {
            let mut s = Simulation::new(Genome::by_id(&id).unwrap(), seed);
            s.set_wind_acclimation_enabled(accl);
            let mut dbh = 0.0f32;
            for _ in 0..years {
                s.advance_years(1);
                dbh = dbh.max(s.dbh_cm());
                if !s.trunk_alive() {
                    break;
                }
            }
            out.push((s.vitality().death_age_years, s.vitality().cause, dbh.round()));
        }
        println!("{id} acclimation {accl}: {out:?}");
    }
}
