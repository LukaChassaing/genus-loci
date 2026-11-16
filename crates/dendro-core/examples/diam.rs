//! Diagnostics: stem and limb diameters against age.
//!
//! cargo run -p dendro-core --release --example diam -- [--species oak|cherry] [--seed N] [--years N]
//!
//! Per reporting age: height, DBH, girth, collar diameter, mean ring width
//! of the last 5 years, basal-area increment (CAI), crown diameter / DBH,
//! simulated and drawn limb/stem diameter ratios, and the base diameter of
//! first-order limbs reaching ~5 m and ~10 m from the stem.

use dendro_core::{Genome, Simulation};

fn main() {
    let mut genome = Genome::quercus_robur();
    let (mut seed, mut years) = (42u64, 200u32);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--species" => {
                genome = match args.next().as_deref() {
                    Some("cherry") => Genome::prunus_avium(),
                    _ => Genome::quercus_robur(),
                }
            }
            "--seed" => seed = args.next().unwrap().parse().unwrap(),
            "--years" => years = args.next().unwrap().parse().unwrap(),
            o => panic!("unknown {o}"),
        }
    }
    let e = genome.wood.pipe_exponent.max(2.0);
    let boost = genome.wood.limb_draw_boost.max(1.0);
    // Same law as `sim::draw_radius` (mesh only).
    let draw = |rs: f32, r_ref: f32| {
        let pipe = (r_ref / rs.max(1e-6)).powf(1.0 - 2.0 / e);
        let fade = ((rs - 0.005) / 0.035).clamp(0.0, 1.0);
        let cap = 1.0 + (boost - 1.0) * fade * fade * (3.0 - 2.0 * fade);
        rs * pipe.min(cap).max(1.0)
    };
    let mut sim = Simulation::new(genome.clone(), seed);
    println!("{} seed {seed}", genome.species);
    println!(
        "{:>4} {:>6} {:>6} {:>6} {:>6} {:>5} {:>6} {:>6} {:>5} {:>5} {:>5} {:>5} {:>3} {:>9} {:>9} {:>7} {:>5}",
        "age", "H", "DBH", "girth", "collar", "ring", "CAI", "CD/D", "rMx", "rMn", "dMx", "dMn", "nL", "d|reach5", "d|reach10", "fol", "path"
    );
    let ages = [5u32, 10, 15, 20, 30, 40, 60, 80, 100, 150, 200, 250, 300];
    let mut prev_ba = 0.0f32;
    let mut prev_age = 0u32;
    for age in 1..=years {
        sim.advance_years(1);
        if !ages.contains(&age) && age != years {
            continue;
        }
        let s = sim.stats();
        let m = sim.shape_metrics();
        let dbh = s.dbh_cm;
        let ba = std::f32::consts::PI * 0.25 * dbh * dbh;
        let cai = if age > prev_age && prev_age > 0 { (ba - prev_ba) / (age - prev_age) as f32 } else { 0.0 };
        prev_ba = ba;
        prev_age = age;
        let rings = sim.rings();
        let n = rings.len().clamp(1, 5);
        let ring = rings.iter().rev().take(5).map(|r| r.width_mm).sum::<f32>() / n as f32;
        let collar = sim.collar_diameter_cm();
        let r_ref = collar * 0.005;
        // Drawn ratio: drawn limb radius / drawn stem radius (the stem is
        // drawn at its simulated radius since its pipe factor is ~1).
        let d_ratio = |ratio: f32| {
            let rl = ratio * dbh * 0.005;
            if rl <= 0.0 {
                0.0
            } else {
                draw(rl, r_ref) / (dbh * 0.005)
            }
        };
        let limbs = sim.debug_limbs();
        let mut d5: Vec<f32> = Vec::new();
        let mut d10: Vec<f32> = Vec::new();
        for (_, r, reach, _, _, _) in &limbs {
            if (4.0..6.0).contains(reach) {
                d5.push(200.0 * r);
            } else if (8.5..11.5).contains(reach) {
                d10.push(200.0 * r);
            }
        }
        let mean = |v: &Vec<f32>| if v.is_empty() { 0.0 } else { v.iter().sum::<f32>() / v.len() as f32 };
        let maxv = |v: &Vec<f32>| v.iter().copied().fold(0.0f32, f32::max);
        let (fol, path) = sim.debug_foliage();
        println!(
            "{:>4} {:>6.1} {:>6.1} {:>6.0} {:>6.1} {:>5.2} {:>6.1} {:>6.1} {:>5.2} {:>5.2} {:>5.2} {:>5.2} {:>3} {:>4.1}/{:<4.1} {:>4.1}/{:<4.1} {:>7.2} {:>5.1}",
            age,
            s.height_m,
            dbh,
            dbh * std::f32::consts::PI,
            collar,
            ring,
            cai,
            if dbh > 0.0 { 2.0 * s.crown_radius_m / (dbh * 0.01) } else { 0.0 },
            m.max_limb_ratio,
            m.mean_limb_ratio,
            d_ratio(m.max_limb_ratio),
            d_ratio(m.mean_limb_ratio),
            m.scaffold_limbs,
            mean(&d5),
            maxv(&d5),
            mean(&d10),
            maxv(&d10),
            fol,
            path
        );
    }
}
