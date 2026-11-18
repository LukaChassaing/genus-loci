//! Size and position of the flower / fruit mesh of a species on a given day
//! (checking that cones or fruits are drawn, and where).
//!
//!   cargo run -p dendro-core --release --example fruit_mesh -- picea_abies 3 61 272
use dendro_core::{Genome, Simulation};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let g = Genome::find(&a[0]).expect("species id");
    let seed: u64 = a[1].parse().unwrap();
    let years: u32 = a[2].parse().unwrap();
    let doy: u32 = a[3].parse().unwrap();
    let mut sim = Simulation::new(g, seed);
    sim.advance_days(years * 365 + doy);
    let m = sim.flower_mesh();
    let ys: Vec<f32> = m.positions.iter().map(|p| p[1]).collect();
    let (lo, hi) = ys.iter().fold((f32::MAX, f32::MIN), |(a, b), &y| (a.min(y), b.max(y)));
    println!("day {doy} of year {years}: fruit density {:.2}, ripeness {:.2}, {} vertices between {lo:.1} and {hi:.1} m (tree {:.1} m)",
        sim.fruit_density(), sim.fruit_ripeness(), m.positions.len(), sim.height_m());
}
