//! Distribution of the trunk's death age over many seeds (longevity of
//! `vitality.rs`): age, cause, lifetime maximum DBH, and the vitality state
//! the year before death; the distribution of the reserves over the living
//! years from 60 y on (share of years below each level, all seeds pooled),
//! with which the reserve level of a "crisis year" of the bark-beetle
//! mortality (`BioticTraits::crisis_reserve`) was set; the mean yearly
//! death probabilities (decline draw, bark beetle) of those years, in and
//! out of crisis years (reserves at or below `crisis_reserve`, 0.2 for a
//! non-host).
//!
//! cargo run -p dendro-core --release --example life_dist -- <species> <seeds> [years]

use dendro_core::{Genome, Simulation};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let g = Genome::by_id(&a[0]).expect("species");
    let n: u64 = a[1].parse().unwrap();
    let years: u32 = a.get(2).map(|s| s.parse().unwrap()).unwrap_or(400);
    let mut ages = Vec::new();
    // Reserve levels of the living years from 60 y on, all seeds pooled.
    let mut adult_reserves: Vec<f32> = Vec::new();
    let crisis = if g.biotic.host() { g.biotic.crisis_reserve } else { 0.2 };
    // (years, sum of decline probabilities, sum of beetle probabilities), out of / in crisis.
    let mut hz = [(0u32, 0.0f64, 0.0f64); 2];
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
            if s.age_years() >= 60.0 {
                let v = s.vitality();
                adult_reserves.push(v.reserve);
                let h = &mut hz[(v.reserve <= crisis) as usize];
                h.0 += 1;
                h.1 += v.death_probability as f64;
                h.2 += v.bark_beetle_probability as f64;
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
    let n_adult = adult_reserves.len().max(1) as f32;
    let shares: Vec<String> = [0.0f32, 0.05, 0.1, 0.2, 0.3, 0.4, 0.6]
        .iter()
        .map(|&t| format!("<= {t:.2}: {:.1} %", 100.0 * adult_reserves.iter().filter(|&&r| r <= t).count() as f32 / n_adult))
        .collect();
    println!("living years from 60 y: {}; share with reserves {}", adult_reserves.len(), shares.join(", "));
    for (k, (n, d, b)) in hz.iter().enumerate() {
        let n1 = (*n).max(1) as f64;
        println!(
            "{} (reserves {} {crisis}): {n} years, mean yearly death probability: decline {:.2} %, bark beetle {:.2} %",
            if k == 1 { "crisis years" } else { "other years" },
            if k == 1 { "<=" } else { ">" },
            100.0 * d / n1,
            100.0 * b / n1
        );
    }
    println!("{} death ages (>{years} = alive): {ages:?}; quartiles {} / {} / {}", a[0], q(0.25), q(0.5), q(0.75));
}
