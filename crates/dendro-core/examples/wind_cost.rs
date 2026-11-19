//! Cost of the wind acclimation: an oak grown 200 years with and without it.
//!
//! cargo run -p dendro-core --release --example wind_cost

use dendro_core::{Genome, Simulation};
use std::time::Instant;

fn main() {
    for accl in [false, true] {
        let mut s = Simulation::new(Genome::quercus_robur(), 42);
        s.set_wind_acclimation_enabled(accl);
        let t = Instant::now();
        s.advance_years(200);
        println!("acclimation {accl}: {:.2} s, screens {:?}, DBH {:.1} H {:.1} segments {}", t.elapsed().as_secs_f32(), s.wind_costs(), s.dbh_cm(), s.height_m(), s.shoot_nodes().len());
    }
    let s = Simulation::new(Genome::quercus_robur(), 42);
    let mut s = s;
    s.advance_years(200);
    let t = Instant::now();
    for _ in 0..10 {
        let _ = s.mechanics_inputs();
    }
    println!("mechanics_inputs: {:.1} ms", t.elapsed().as_secs_f32() * 100.0);
    let (seg, site) = s.mechanics_inputs();
    let t = Instant::now();
    let m = dendro_core::MechanicsModel::from_segments(&seg, Genome::quercus_robur().mechanics, site);
    let _ = m.stem_stresses(&dendro_core::WindLoad { leaf_fraction: Some(1.0), ..dendro_core::WindLoad::new(28.0) });
    println!("model + stresses: {:.1} ms", t.elapsed().as_secs_f32() * 1000.0);
}
