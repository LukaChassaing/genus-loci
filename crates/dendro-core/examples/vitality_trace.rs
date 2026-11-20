//! Yearly trace of the carbon status of one tree (`vitality.rs`): reserves,
//! cost / production ratio, vigour, death hazard, dieback, DBH, height,
//! wood volume and foliage. Diagnostic of longevity changes.
//!
//! cargo run -p dendro-core --release --example vitality_trace -- <species> <seed> <years> [every]

use dendro_core::{Genome, Simulation, WindLoad};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut s = Simulation::new(Genome::by_id(&a[0]).expect("species"), a[1].parse().unwrap());
    let years: u32 = a[2].parse().unwrap();
    let every: u32 = a.get(3).map(|x| x.parse().unwrap()).unwrap_or(10);
    println!("age H DBH reserve cost_ratio net vigour hazard dieback wood_t leaf_kg segments");
    for y in 1..=years {
        s.advance_years(1);
        if y % every == 0 || !s.trunk_alive() {
            let v = s.vitality();
            let m = s.mechanics_model().evaluate(&WindLoad { leaf_fraction: Some(1.0), quasi_static: true, ..WindLoad::new(0.0) });
            println!(
                "{y} {:.1} {:.1} {:.3} {:.3} {:.3} {:.3} {:.4} {} {:.2} {:.0} {}",
                s.height_m(),
                s.dbh_cm(),
                v.reserve,
                v.cost_ratio,
                v.net_production,
                v.vigour,
                v.death_probability,
                v.dieback_branches,
                m.wood_mass_kg / 1000.0,
                m.leaf_mass_kg,
                s.shoot_nodes().len()
            );
        }
        if !s.trunk_alive() {
            println!("trunk died: {:?}", s.vitality());
            break;
        }
    }
}
