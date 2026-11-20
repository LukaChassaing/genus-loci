//! Critical speeds of a few trees (seed 42, 20/60/150 years), leafless and
//! in leaf, with the search time: before / after comparisons of the search.
//!
//! cargo run -p dendro-core --release --example crit_dump

use dendro_core::{Genome, Simulation, WindLoad};
use std::time::Instant;

fn main() {
    let mut total = 0.0f32;
    for id in ["quercus_robur", "prunus_avium", "picea_abies", "pinus_sylvestris"] {
        let mut s = Simulation::new(Genome::by_id(id).unwrap(), 42);
        let mut done = 0;
        for age in [20u32, 60, 150] {
            s.advance_years(age - done);
            done = age;
            let m = s.mechanics_model();
            for lf in [0.0f32, 1.0] {
                let t = Instant::now();
                let c = m.critical_speeds(&WindLoad { leaf_fraction: Some(lf), ..WindLoad::new(0.0) });
                let dt = t.elapsed().as_secs_f32();
                total += dt;
                println!("{id} {age} leaf {lf}: {:?} {:?} {:?} ({:.1} ms)", c.stem_break, c.limb_break, c.uprooting, dt * 1e3);
            }
        }
    }
    println!("total search time {:.0} ms", total * 1e3);
}
