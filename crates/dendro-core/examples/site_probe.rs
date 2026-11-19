//! Height and DBH of one tree on a soil preset, with and without the wind
//! acclimation of the stem (diagnostic of the growth tests).
//!
//! cargo run -p dendro-core --release --example site_probe -- <species> <soil index> <age> [seed]

use dendro_core::{ClimateParams, Genome, Simulation, SoilProfile};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let g = Genome::by_id(&a[0]).expect("species");
    let soil = SoilProfile::presets().into_iter().nth(a[1].parse().unwrap()).expect("soil");
    let age: u32 = a[2].parse().unwrap();
    let seed: u64 = a.get(3).map(|s| s.parse().unwrap()).unwrap_or(42);
    for accl in [false, true] {
        let mut s = Simulation::with_site(g.clone(), seed, ClimateParams::default(), soil.clone());
        s.set_wind_acclimation_enabled(accl);
        s.advance_years(age);
        println!("{} acclimation {accl}: H {:.2} m, DBH {:.1} cm, screens {:?}", soil.name, s.height_m(), s.dbh_cm(), s.wind_costs());
    }
}
