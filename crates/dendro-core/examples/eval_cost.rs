//! Cost of building the mechanics model and of one evaluation (150-year
//! oak, the tree of `performance_mechanics_old_oak`).
//!
//! cargo run -p dendro-core --release --example eval_cost

use dendro_core::{Genome, Simulation, WindLoad};
use std::time::Instant;

fn main() {
    let mut s = Simulation::new(Genome::quercus_robur(), 42);
    s.advance_years(150);
    let t = Instant::now();
    let (seg, site) = s.mechanics_inputs();
    let t_in = t.elapsed();
    let t = Instant::now();
    let m = dendro_core::MechanicsModel::from_segments(&seg, Genome::quercus_robur().mechanics, site);
    let t_model = t.elapsed();
    for u in [0.0f32, 30.0] {
        for qs in [true, false] {
            let t = Instant::now();
            let _ = m.evaluate(&WindLoad { quasi_static: qs, ..WindLoad::new(u) });
            println!("evaluate u {u} quasi_static {qs}: {:.2} ms", t.elapsed().as_secs_f32() * 1e3);
        }
    }
    println!("inputs {:.2} ms, model {:.2} ms ({} segments)", t_in.as_secs_f32() * 1e3, t_model.as_secs_f32() * 1e3, m.segment_count());
}
