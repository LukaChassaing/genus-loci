//! Wind history of one simulated tree.
//!
//! cargo run -p dendro-core --release --example wind_history -- <species> <seed> <years>

use dendro_core::{Genome, Simulation};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut s = Simulation::new(Genome::by_id(&a[0]).expect("species"), a[1].parse().unwrap());
    let years: u32 = a[2].parse().unwrap();
    let mut last = 0;
    for y in 0..years {
        s.advance_years(1);
        let h = s.wind_history();
        for d in &h[last..] {
            println!(
                "year {:3} doy {:3} {:<22} gust {:4.1} m/s at {:4.1} m, d {:4.1} cm, leaf {:.2} | tree H {:.1} DBH {:.1}",
                d.day / 365,
                d.day % 365,
                d.kind.label_fr(),
                d.gust_ms,
                d.height_m,
                d.diameter_cm,
                d.leaf_fraction,
                s.height_m(),
                s.dbh_cm()
            );
        }
        last = h.len();
        let _ = y;
    }
    println!("end: alive {} trunk {} H {:.1} DBH {:.1} costs {:?}", s.is_alive(), s.trunk_alive(), s.height_m(), s.dbh_cm(), s.wind_costs());
}
