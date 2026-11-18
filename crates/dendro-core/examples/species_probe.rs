//! Quick look at any registry species: size, crown, foliage, phenology and
//! mechanics at a few ages (calibration aid for new genomes).
//!
//!   cargo run -p dendro-core --release --example species_probe -- picea_abies [--seed 1] [--ages 20,40,60,100]

use dendro_core::{Genome, Simulation, WindLoad};

fn main() {
    let mut args = std::env::args().skip(1);
    let id = args.next().unwrap_or_else(|| "quercus_robur".into());
    let mut seed = 1u64;
    let mut ages: Vec<u32> = vec![10, 20, 40, 60, 100];
    while let Some(a) = args.next() {
        match a.as_str() {
            "--seed" => seed = args.next().unwrap().parse().unwrap(),
            "--ages" => ages = args.next().unwrap().split(',').map(|s| s.parse().unwrap()).collect(),
            o => panic!("unknown {o}"),
        }
    }
    let g = Genome::find(&id).unwrap_or_else(|| panic!("unknown species {id}"));
    println!("{} ({}), category {:?}, evergreen {}, retained winter foliage {:.2}", g.species, g.id, g.category, g.leaf.evergreen(), g.leaf.retained_share());
    let mut sim = Simulation::new(g, seed);
    let mut y = 0;
    for &a in &ages {
        sim.advance_years(a - y);
        y = a;
        let winter = sim.leaf_density();
        let m = sim.mechanics_model();
        let bare = m.critical_speeds(&WindLoad { leaf_fraction: Some(0.0), ..WindLoad::new(0.0) }).first().map_or(f32::NAN, |f| f.1);
        let crit = m.critical_speeds(&WindLoad { leaf_fraction: Some(1.0), ..WindLoad::new(0.0) });
        let leafy = crit.first().map_or(f32::NAN, |f| f.1);
        let e20 = m.evaluate(&WindLoad { leaf_fraction: Some(1.0), ..WindLoad::new(20.0) });
        let crit0 = m.critical_speeds(&WindLoad { leaf_fraction: Some(0.0), ..WindLoad::new(0.0) });
        let e40b = m.evaluate(&WindLoad { leaf_fraction: Some(0.0), ..WindLoad::new(45.0) });
        let e40l = m.evaluate(&WindLoad { leaf_fraction: Some(1.0), ..WindLoad::new(45.0) });
        println!("    leafless: {crit0:?}; at 45 m/s drag leafless {:.0} N / in leaf {:.0} N, base moment {:.0} / {:.0} kNm, limb safety {:.2} / {:.2}, weakest leafless {:?}",
            e40b.drag_n, e40l.drag_n, e40b.base_moment_nm / 1000.0, e40l.base_moment_nm / 1000.0, e40b.limb_safety, e40l.limb_safety, e40b.weakest);
        println!("    weakest at 20 m/s: {:?}", e20.weakest);
        if let Some(w) = e20.weakest {
            let (segs, site) = sim.mechanics_inputs();
            let tot: f32 = segs.iter().map(|s| s.leaf_weight).sum();
            let above: f32 = segs.iter().filter(|s| s.end[1] > w.height_m).map(|s| s.leaf_weight).sum();
            let lever: f32 = segs.iter().filter(|s| s.end[1] > w.height_m).map(|s| s.leaf_weight * (s.end[1] - w.height_m)).sum::<f32>() / above.max(1e-6);
            println!("    foliage share above the weak point {:.2}, mean lever {:.1} m, leaf area {:.0} m2, segments {}", above / tot, lever, site.leaf_area_m2, segs.len());
        }
        println!("    in leaf: {crit:?}; at 20 m/s: drag {:.0} N, base moment {:.0} kNm, crown area {:.1} m2, leaf mass {:.0} kg, safety stem {:.2} limb {:.2} anchor {:.2}, crit moment {:.0} kNm",
            e20.drag_n, e20.base_moment_nm / 1000.0, e20.crown_frontal_area_m2, e20.leaf_mass_kg, e20.stem_safety, e20.limb_safety, e20.anchorage_safety, e20.critical_turning_moment_nm / 1000.0);
        let sh = sim.shape_metrics();
        println!(
            "age {a:3}: H {:5.1} m DBH {:5.1} cm crown W/H {:.2} clear bole {:.1} m limbs {} | Jan leaf density {winter:.2} | crit gust leafless {bare:.1} in leaf {leafy:.1} m/s | nodes {}",
            sim.height_m(),
            sim.dbh_cm(),
            sh.crown_width_to_height,
            sh.clear_bole_m,
            sh.scaffold_limbs,
            sim.stats().shoot_nodes
        );
    }
}
