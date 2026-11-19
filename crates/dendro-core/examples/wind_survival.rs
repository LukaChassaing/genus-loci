//! Survival under the calibrated wind climate: share of trees alive and
//! standing (not uprooted) at a given age, breaks and reiterations.
//!
//! cargo run -p dendro-core --release --example wind_survival -- [--species a,b] [--seeds N] [--years 100]

use dendro_core::{DamageKind, Genome, Simulation};

fn main() {
    let mut species: Vec<String> = ["quercus_robur", "prunus_avium", "fagus_sylvatica", "picea_abies", "pinus_sylvestris", "malus_domestica"].iter().map(|s| s.to_string()).collect();
    let mut seeds = 20u64;
    let mut years = 100u32;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--species" => species = args.next().unwrap().split(',').map(|s| s.to_string()).collect(),
            "--seeds" => seeds = args.next().unwrap().parse().unwrap(),
            "--years" => years = args.next().unwrap().parse().unwrap(),
            _ => panic!("unknown argument {a}"),
        }
    }
    println!("| Espèce | debout et vivant | déracinés (âges) | troncs/cimes cassés (arbres) | dont avant 25 ans | réitérés (vivants après cassure) | charpentières | mort autre cause | H médiane | DBH médian |");
    println!("|---|---|---|---|---|---|---|---|---|---|");
    for sp in &species {
        let g = Genome::by_id(sp).unwrap();
        let (mut ok, mut up, mut broke_trees, mut young, mut reit, mut limbs, mut other) = (0, Vec::new(), 0, 0, 0, 0, 0);
        let (mut hs, mut ds) = (Vec::new(), Vec::new());
        for seed in 1..=seeds {
            let mut s = Simulation::new(g.clone(), seed);
            s.advance_years(years);
            let h = s.wind_history();
            let stem_breaks: Vec<_> = h.iter().filter(|d| matches!(d.kind, DamageKind::Stem | DamageKind::Top)).collect();
            young += stem_breaks.iter().filter(|d| d.day < 25 * 365).count();
            limbs += h.iter().filter(|d| d.kind == DamageKind::Limb).count();
            let uprooted = s.uprooted();
            if let Some((day, _)) = uprooted {
                up.push(day / 365);
            }
            if !stem_breaks.is_empty() {
                broke_trees += 1;
                if s.is_alive() && uprooted.is_none() {
                    reit += 1;
                }
            }
            if s.trunk_alive() && uprooted.is_none() {
                ok += 1;
                hs.push(s.height_m());
                ds.push(s.dbh_cm());
            } else if uprooted.is_none() {
                other += 1;
            }
        }
        let med = |mut v: Vec<f32>| {
            if v.is_empty() {
                return f32::NAN;
            }
            v.sort_by(|a, b| a.total_cmp(b));
            v[v.len() / 2]
        };
        up.sort();
        println!("| {sp} | {ok}/{seeds} | {} {:?} | {broke_trees} | {young} | {reit} | {limbs} | {other} | {:.1} | {:.1} |", up.len(), up, med(hs), med(ds));
    }
}
