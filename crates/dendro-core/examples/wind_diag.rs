//! Diagnostics of the wind mechanics of one simulated tree: stem profile,
//! weakest section, crown silhouette, modal masses.
//!
//! cargo run -p dendro-core --release --example wind_diag -- <species> <age> [seed] [gust]

use dendro_core::{Genome, MechSegment, MechanicsModel, SegmentRole, Simulation, WindLoad};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let g = Genome::by_id(&a[0]).expect("species");
    let age: u32 = a[1].parse().unwrap();
    let seed: u64 = a.get(2).map(|s| s.parse().unwrap()).unwrap_or(42);
    let gust: f32 = a.get(3).map(|s| s.parse().unwrap()).unwrap_or(20.0);
    let mut sim = Simulation::new(g.clone(), seed);
    if std::env::var("NOACCL").is_ok() {
        sim.set_wind_acclimation_enabled(false);
    }
    sim.advance_years(age);
    if let Ok(d) = std::env::var("DOY") {
        sim.advance_days(d.parse().unwrap());
    }
    let model = sim.mechanics_model();
    let (segs, site) = sim.mechanics_inputs();
    println!("H {:.1} m, DBH {:.1} cm, crown radius {:.1} m, leaf area {:.0} m2, voxel {:.2} m", sim.height_m(), sim.dbh_cm(), sim.crown_radius_m(), site.leaf_area_m2, site.crown_cell_m);
    for (lf, name) in [(1.0f32, "in leaf"), (0.0, "leafless")] {
        let load = WindLoad { leaf_fraction: Some(lf), ..WindLoad::new(gust) };
        let s = model.evaluate(&load);
        let d = model.dynamics(&load);
        println!(
            "{name} @ {gust} m/s: f0 {:.3} Hz, modal mass {:.0} kg, zeta {:.3}, cd {:.3}, A0 {:.1} m2, drag {:.0} N at {:.1} m, top defl {:.2} m, SF stem {:.2} limb {:.2} anch {:.2}",
            d.natural_frequency_hz, d.modal_mass_kg, s.damping_ratio, s.dynamic_factor, s.crown_frontal_area_m2, s.drag_n, s.centre_of_pressure_m, s.top_deflection_m, s.stem_safety, s.limb_safety, s.anchorage_safety
        );
        if let Some(w) = s.weakest {
            println!("   weakest: {:?} h {:.1} m, d {:.1} cm, SF {:.2}", w.role, w.height_m, w.diameter_cm, w.safety);
        }
        if lf > 0.5 {
            // Stem profile.
            let mut line = String::new();
            for (i, r) in s.segment_role.iter().enumerate() {
                if *r == SegmentRole::Trunk {
                    let sg = &segs[i];
                    let h = sg.start[1];
                    let k = (h / 2.0).floor() as i32;
                    if line.len() < 4000 && (i == 0 || (h - 2.0 * k as f32) < 0.6 || std::env::var("FULL").is_ok_and(|v| h > v.parse::<f32>().unwrap_or(0.0))) {
                        line += &format!(" {:.0}m:{:.1}cm/SF{:.1}", h, 200.0 * sg.radius_m, s.segment_safety[i]);
                    }
                }
            }
            println!("   stem:{line}");
        }
    }
    // Modal mass split: leaves removed from the inputs (leafless, wood only)
    // and a model where only stem and limbs flex.
    let strip = |segs: &[MechSegment], thr: f32| -> Vec<MechSegment> {
        segs.iter()
            .map(|s| {
                let mut s = s.clone();
                if 2.0 * s.radius_m < thr && s.leaf_weight <= 0.0 {
                    s.dead = true; // rigid in the dynamics (and no breakage)
                }
                s
            })
            .collect()
    };
    for thr in [0.05f32, 0.10, 0.2 * site.dbh_m] {
        let m2 = MechanicsModel::from_segments(&strip(&segs, thr), g.mechanics.clone(), site.clone());
        let fl = m2.dynamics(&WindLoad { leaf_fraction: Some(1.0), ..WindLoad::new(0.0) }).natural_frequency_hz;
        let fb = m2.dynamics(&WindLoad { leaf_fraction: Some(0.0), ..WindLoad::new(0.0) }).natural_frequency_hz;
        println!("only axes >= {:.1} cm flex: f0 leaf {:.3} bare {:.3} ratio {:+.0} %", thr * 100.0, fl, fb, 100.0 * (fb / fl - 1.0));
    }
    let total_leaf: f32 = segs.iter().map(|s| s.leaf_weight).sum();
    let leaf_kg = site.leaf_area_m2 * site.leaf_lma_g_m2 * 1e-3 / 0.4;
    let m = model.evaluate(&WindLoad { leaf_fraction: Some(1.0), ..WindLoad::new(0.0) });
    println!("leaf mass {:.0} kg ({} leafy tips, weight {:.1}), wood {:.0} kg", leaf_kg, segs.iter().filter(|s| s.leaf_weight > 0.0).count(), total_leaf, m.wood_mass_kg);
}
