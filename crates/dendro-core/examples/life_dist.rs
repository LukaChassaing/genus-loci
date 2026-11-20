//! Distribution of the trunk's death age over many seeds (longevity of
//! `vitality.rs`): age, cause, lifetime maximum DBH, and the vitality state
//! the year before death.
//!
//! cargo run -p dendro-core --release --example life_dist -- <species> <seeds> [years]

use dendro_core::{Genome, Simulation};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let g = Genome::by_id(&a[0]).expect("species");
    let n: u64 = a[1].parse().unwrap();
    let years: u32 = a.get(2).map(|s| s.parse().unwrap()).unwrap_or(400);
    let mut ages = Vec::new();
    for seed in 1..=n {
        let mut s = Simulation::new(g.clone(), seed);
        let mut dbh = 0.0f32;
        let mut before = None;
        let mut low_reserve_years = 0;
        for _ in 0..years {
            let v = s.vitality();
            before = Some((v.reserve, v.cost_ratio, v.vigour, v.death_probability));
            s.advance_years(1);
            dbh = dbh.max(s.dbh_cm());
            if s.vitality().reserve < 0.2 {
                low_reserve_years += 1;
            }
            if !s.trunk_alive() {
                break;
            }
        }
        let v = s.vitality();
        let (r, rho, vig, hz) = before.unwrap();
        println!(
            "seed {seed}: death {:?} {:?}, max DBH {dbh:.0} cm, years with reserve < 0.2: {low_reserve_years}; year before death: reserve {r:.2} rho {rho:.3} vigour {vig:.2} hazard {hz:.4}",
            v.death_age_years, v.cause
        );
        ages.push(v.death_age_years.unwrap_or(years + 1));
    }
    ages.sort();
    let q = |p: f32| ages[((ages.len() - 1) as f32 * p).round() as usize];
    println!("{} death ages (>{years} = alive): {ages:?}; quartiles {} / {} / {}", a[0], q(0.25), q(0.5), q(0.75));
}
