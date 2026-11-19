//! Sway frequencies and critical gusts of simulated trees, median over
//! seeds (the table of docs/sources.md, « Vent dynamique »).
//!
//! cargo run -p dendro-core --release --example wind_table -- [--species a,b] [--seeds 42,1,1234] [--ages 20,60,150] [--static] [--damage]
//!
//! By default the trees grow without wind damage (sound trees; the wind
//! still acclimates their growth), as in the growth calibration tests.
//! `--damage`: the trees of the simulation (natural wind damage on).
//! `--no-accl`: without the stem acclimation to the wind.
//! `--static`: the quasi-static small-deflection model (no dynamic factor,
//! no large deflections) for the critical gusts.

use dendro_core::{CriticalSpeeds, Genome, Simulation, WindLoad};

fn median(mut v: Vec<f32>) -> f32 {
    if v.is_empty() {
        return f32::NAN;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

fn fmt(v: f32) -> String {
    if v.is_nan() {
        "  —  ".into()
    } else if v >= 79.9 {
        " >80 ".into()
    } else {
        format!("{v:5.1}")
    }
}

fn crit(c: &CriticalSpeeds) -> [f32; 3] {
    let f = |o: Option<f32>, exists: bool| if exists { o.unwrap_or(80.0) } else { f32::NAN };
    [f(c.stem_break, true), f(c.limb_break, c.limb_break.is_some() || true), f(c.uprooting, true)]
}

fn main() {
    let mut species: Vec<String> = ["quercus_robur", "prunus_avium", "fagus_sylvatica", "picea_abies", "pinus_sylvestris", "malus_domestica"].iter().map(|s| s.to_string()).collect();
    let mut seeds: Vec<u64> = vec![42, 1, 1234];
    let mut ages: Vec<u32> = vec![20, 60, 150];
    let mut quasi = false;
    let mut damage = false;
    let mut accl = true;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--species" => species = args.next().expect("--species a,b").split(',').map(|s| s.to_string()).collect(),
            "--seeds" => seeds = args.next().expect("--seeds a,b").split(',').map(|s| s.parse().expect("seed")).collect(),
            "--ages" => ages = args.next().expect("--ages a,b").split(',').map(|s| s.parse().expect("age")).collect(),
            "--static" => quasi = true,
            "--damage" => damage = true,
            "--no-accl" => accl = false,
            _ => panic!("unknown argument {a}"),
        }
    }
    println!("| Espèce | Âge | H m | DBH cm | f0 feuilles / hors feuilles (Hz) | rapport | Hors feuilles : tronc / charp. / déracin. | En feuilles : tronc / charp. / déracin. |");
    println!("|---|---|---|---|---|---|---|---|");
    for sp in &species {
        let g = Genome::by_id(sp).unwrap_or_else(|| panic!("unknown species {sp}"));
        let mut rows: Vec<Vec<[f32; 10]>> = vec![Vec::new(); ages.len()];
        for &seed in &seeds {
            let mut sim = Simulation::new(g.clone(), seed);
            sim.set_wind_damage_enabled(damage);
            sim.set_wind_acclimation_enabled(accl);
            let mut done = 0u32;
            for (k, &age) in ages.iter().enumerate() {
                sim.advance_years(age - done);
                done = age;
                let model = sim.mechanics_model();
                let leafy = WindLoad { leaf_fraction: Some(1.0), quasi_static: quasi, ..WindLoad::new(0.0) };
                let bare = WindLoad { leaf_fraction: Some(0.0), quasi_static: quasi, ..WindLoad::new(0.0) };
                let fl = model.dynamics(&leafy).natural_frequency_hz;
                let fb = model.dynamics(&bare).natural_frequency_hz;
                let cb = crit(&model.critical_speeds(&bare));
                let cl = crit(&model.critical_speeds(&leafy));
                rows[k].push([sim.height_m(), sim.dbh_cm(), fl, fb, cb[0], cb[1], cb[2], cl[0], cl[1], cl[2]]);
            }
        }
        for (k, &age) in ages.iter().enumerate() {
            let m = |j: usize| median(rows[k].iter().map(|r| r[j]).filter(|v| !v.is_nan()).collect());
            let limb_none = |j: usize| rows[k].iter().all(|r| r[j] >= 79.9);
            let l = |j: usize| if limb_none(j) { " >80 ".to_string() } else { fmt(m(j)) };
            println!(
                "| {sp} | {age} | {:.1} | {:.1} | {:.2} / {:.2} | {:+.0} % | {} / {} / {} | {} / {} / {} |",
                m(0),
                m(1),
                m(2),
                m(3),
                100.0 * (m(3) / m(2) - 1.0),
                fmt(m(4)),
                l(5),
                fmt(m(6)),
                fmt(m(7)),
                l(8),
                fmt(m(9))
            );
        }
    }
}
