//! Crop years of a species (masting / alternate bearing): the years whose
//! bloom reaches a given density, for choosing screenshot dates.
//!
//!   cargo run -p dendro-core --release --example crop_years -- picea_abies 3 [80]
use dendro_core::{Genome, Simulation};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let g = Genome::find(a.first().map_or("quercus_robur", |s| s.as_str())).expect("species id");
    let seed: u64 = a.get(1).map_or(1, |s| s.parse().unwrap());
    let years: u32 = a.get(2).map_or(80, |s| s.parse().unwrap());
    let mut sim = Simulation::new(g, seed);
    for y in 0..years {
        let (mut fl, mut fr) = (0.0f32, 0.0f32);
        for _ in 0..365 {
            sim.advance_days(1);
            fl = fl.max(sim.flower_density());
            fr = fr.max(sim.fruit_density());
        }
        if fl > 0.0 {
            println!("year {y}: peak bloom {fl:.2}, peak fruit {fr:.2}");
        }
        let _ = y;
    }
}
