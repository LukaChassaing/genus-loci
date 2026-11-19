//! Probe: what becomes of a tree whose stem is cut (topping at mid-height,
//! coppice at the collar) over the next decades: vitality, DBH, height.
//!
//! cargo run -p dendro-core --release --example break_probe -- <species> <age> <seed>

use dendro_core::{Genome, Simulation};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let g = Genome::by_id(&a[0]).expect("species");
    let age: u32 = a[1].parse().unwrap();
    let seed: u64 = a[2].parse().unwrap();
    for mode in ["top", "coppice"] {
        let mut s = Simulation::new(g.clone(), seed);
        s.advance_years(age);
        let h = s.height_m();
        let k = if mode == "top" { s.top_at(0.5 * h) } else { s.coppice() };
        println!("{mode}: {k:?} at {age} y (H {h:.1}, DBH {:.1})", s.dbh_cm());
        for y in 1..=40 {
            s.advance_years(1);
            if y % 5 == 0 {
                let v = s.vitality();
                println!(
                    "  +{y}: alive {} trunk {} H {:.1} DBH {:.1} reserve {:.2} vigour {:.2} hazard {:.3} cause {:?}",
                    s.is_alive(),
                    s.trunk_alive(),
                    s.height_m(),
                    s.dbh_cm(),
                    v.reserve,
                    v.vigour,
                    v.death_probability,
                    v.cause
                );
            }
        }
    }
}
