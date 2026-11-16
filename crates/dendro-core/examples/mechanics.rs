//! Static mechanics of simulated trees: masses, safety under self-weight,
//! critical gust speeds (leafless winter crown and full summer crown).
//!
//! cargo run -p dendro-core --release --example mechanics -- [--soil N] [--seeds 42,1,1234] [--ages 20,60,150]

use dendro_core::{ClimateParams, FailureMode, Genome, Simulation, SoilProfile, WindLoad};
use std::time::Instant;

fn fmt(v: Option<f32>) -> String {
    v.map(|v| format!("{v:5.1}")).unwrap_or_else(|| "  >80".into())
}

fn main() {
    let mut soil = 0usize;
    let mut seeds: Vec<u64> = vec![42, 1, 1234];
    let mut ages: Vec<u32> = vec![20, 60, 150];
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--soil" => soil = args.next().and_then(|v| v.parse().ok()).expect("--soil N"),
            "--seeds" => seeds = args.next().expect("--seeds a,b").split(',').map(|s| s.parse().expect("seed")).collect(),
            "--ages" => ages = args.next().expect("--ages a,b").split(',').map(|s| s.parse().expect("age")).collect(),
            _ => panic!("unknown argument {a}"),
        }
    }
    let profile = SoilProfile::presets().into_iter().nth(soil).expect("soil preset");
    println!("soil: {}", profile.name);
    println!(
        "{:<18} {:>4} {:>5} | {:>5} {:>5} {:>8} {:>7} | {:>6} {:>6} {:>5} | leafless: stem  limb  uproot | in leaf: stem  limb  uproot | first (leafless) | ms",
        "species", "age", "seed", "H m", "DBH", "mass t", "stem t", "SFstem", "SFlimb", "buckl"
    );
    for g in Genome::builtin() {
        for &seed in &seeds {
            let mut sim = Simulation::with_site(g.clone(), seed, ClimateParams::default(), profile.clone());
            let mut done = 0u32;
            for &age in &ages {
                sim.advance_years(age - done);
                done = age;
                let t0 = Instant::now();
                let model = sim.mechanics_model();
                let rest = model.evaluate(&WindLoad { leaf_fraction: Some(1.0), ..WindLoad::new(0.0) });
                let bare = WindLoad { leaf_fraction: Some(0.0), ..WindLoad::new(0.0) };
                let leafy = WindLoad { leaf_fraction: Some(1.0), ..WindLoad::new(0.0) };
                let cb = model.critical_speeds(&bare);
                let cl = model.critical_speeds(&leafy);
                let ms = t0.elapsed().as_secs_f32() * 1000.0;
                let first = cb.first().map(|(m, v)| format!("{} {v:.1}", match m {
                    FailureMode::StemBreak => "stem",
                    FailureMode::LimbBreak => "limb",
                    FailureMode::Uprooting => "uproot",
                }));
                println!(
                    "{:<18} {:>4} {:>5} | {:>5.1} {:>5.1} {:>8.2} {:>7.2} | {:>6.1} {:>6.1} {:>5.2} |          {} {} {}  |         {} {} {} | {:<16} | {:.0} ({} seg)",
                    g.species,
                    age,
                    seed,
                    sim.height_m(),
                    sim.dbh_cm(),
                    rest.above_ground_mass_kg / 1000.0,
                    rest.stem_mass_kg / 1000.0,
                    rest.stem_safety,
                    rest.limb_safety,
                    rest.buckling_safety,
                    fmt(cb.stem_break),
                    fmt(cb.limb_break),
                    fmt(cb.uprooting),
                    fmt(cl.stem_break),
                    fmt(cl.limb_break),
                    fmt(cl.uprooting),
                    first.unwrap_or_else(|| "none".into()),
                    ms,
                    model.segment_count()
                );
                let s32 = model.evaluate(&WindLoad { leaf_fraction: Some(1.0), ..WindLoad::new(32.0) });
                let b32 = model.evaluate(&WindLoad { leaf_fraction: Some(0.0), ..WindLoad::new(32.0) });
                println!(
                    "      32 m/s in leaf: A0 {:.0} m2, Cd {:.2}, drag {:.1} kN at {:.1} m, base {:.0} kNm / Mcrit {:.0} kNm, weakest {:?} | leafless drag {:.1} kN at {:.1} m",
                    s32.crown_frontal_area_m2,
                    s32.crown_drag_coefficient,
                    s32.drag_n / 1000.0,
                    s32.centre_of_pressure_m,
                    s32.base_moment_nm / 1000.0,
                    s32.critical_turning_moment_nm / 1000.0,
                    s32.weakest.map(|w| (w.role, (w.height_m * 10.0).round() / 10.0, (w.diameter_cm * 10.0).round() / 10.0, (w.safety * 100.0).round() / 100.0)),
                    b32.drag_n / 1000.0,
                    b32.centre_of_pressure_m,
                );
            }
        }
    }
}
