//! Diagnostics: geometry and self-weight safety of the scaffold limbs.
//!
//! cargo run -p dendro-core --release --example limb_diag -- [--species oak|cherry] [--genome PATH] [--seed N] [--ages 20,60,150] [--all]
//!
//! For each structural first-order limb (a non-stem child of a stem node,
//! diameter >= 0.2 DBH): insertion height, base diameter, stem diameter
//! below the insertion and their ratio, area ratio at the insertion
//! (stem above + laterals) / stem below, length of the limb axis (followed
//! through the thickest child), chord angle from the vertical, horizontal
//! reach, diameter at 25/50/75 % of the axis, self-weight safety (in leaf,
//! calm) at the base and its minimum along the axis (and where), share of
//! the tree's leafy shoots it carries.

use dendro_core::{Genome, Simulation, WindLoad};

fn main() {
    let mut genome = Genome::quercus_robur();
    let mut seed = 42u64;
    let mut ages: Vec<u32> = vec![20, 60, 150];
    let mut all = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--species" => {
                genome = match args.next().as_deref() {
                    Some("cherry") => Genome::prunus_avium(),
                    _ => Genome::quercus_robur(),
                }
            }
            "--genome" => genome = Genome::from_ron(&std::fs::read_to_string(args.next().unwrap()).unwrap()).unwrap(),
            "--seed" => seed = args.next().unwrap().parse().unwrap(),
            "--ages" => ages = args.next().unwrap().split(',').map(|s| s.parse().unwrap()).collect(),
            "--all" => all = true,
            o => panic!("unknown {o}"),
        }
    }
    let mut sim = Simulation::new(genome.clone(), seed);
    let mut done = 0;
    for &age in &ages {
        sim.advance_years(age - done);
        done = age;
        report(&sim, age, all);
    }
}

fn report(sim: &Simulation, age: u32, all: bool) {
    let (segs, site) = sim.mechanics_inputs();
    let model = sim.mechanics_model();
    let snap = model.evaluate(&WindLoad { leaf_fraction: Some(1.0), ..WindLoad::new(0.0) });
    let n = segs.len();
    let v = |a: [f32; 3]| (a[0], a[1], a[2]);
    let len = |i: usize| {
        let (a, b) = (v(segs[i].start), v(segs[i].end));
        ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2) + (b.2 - a.2).powi(2)).sqrt()
    };
    let mut thickest = vec![usize::MAX; n];
    let mut kids: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 1..n {
        let p = segs[i].parent.unwrap();
        kids[p].push(i);
        if thickest[p] == usize::MAX || segs[i].radius_m > segs[thickest[p]].radius_m {
            thickest[p] = i;
        }
    }
    let mut on_trunk = vec![false; n];
    let mut cur = 0;
    on_trunk[0] = true;
    while thickest[cur] != usize::MAX {
        cur = thickest[cur];
        on_trunk[cur] = true;
    }
    // Leafy shoots per subtree.
    let mut leafy = vec![0u32; n];
    for i in (0..n).rev() {
        if segs[i].leaf_weight > 0.0 {
            leafy[i] += 1;
        }
        if let Some(p) = segs[i].parent {
            leafy[p] += leafy[i];
        }
    }
    let dbh = site.dbh_m;
    println!(
        "== {} age {age}: H {:.1} m, DBH {:.1} cm, limb safety {:.2}, stem safety {:.2}, segments {n}",
        sim.genome().species,
        sim.height_m(),
        dbh * 100.0,
        snap.limb_safety,
        snap.stem_safety
    );
    println!(
        "{:>5} {:>6} {:>6} {:>5} {:>5} | {:>5} {:>5} {:>5} | {:>5} {:>5} {:>5} | {:>5} {:>6} {:>5} | {:>5}",
        "y", "dL cm", "dT cm", "dL/dT", "Aratio", "L m", "ang", "reach", "d25", "d50", "d75", "SFb", "SFmin", "@frac", "leaf%"
    );
    let total_leafy = leafy[0].max(1) as f32;
    let mut rows = Vec::new();
    for i in 1..n {
        let p = segs[i].parent.unwrap();
        if !on_trunk[p] || on_trunk[i] || segs[i].dead {
            continue;
        }
        let d = 2.0 * segs[i].radius_m;
        if !all && d < 0.2 * dbh.max(0.25) {
            continue;
        }
        if d < 0.03 {
            continue;
        }
        let dt = 2.0 * segs[p].radius_m;
        let a_kids: f32 = kids[p].iter().filter(|&&k| !segs[k].dead).map(|&k| segs[k].radius_m.powi(2)).sum();
        let a_ratio = a_kids / segs[p].radius_m.powi(2);
        // Axis.
        let mut axis = vec![i];
        let mut c = i;
        while thickest[c] != usize::MAX {
            c = thickest[c];
            axis.push(c);
        }
        let mut cum = vec![0.0f32];
        for &k in &axis {
            let l = *cum.last().unwrap();
            cum.push(l + len(k));
        }
        let total = *cum.last().unwrap();
        let tip = v(segs[*axis.last().unwrap()].end);
        let base = v(segs[i].start);
        let (dx, dy, dz) = (tip.0 - base.0, tip.1 - base.1, tip.2 - base.2);
        let chord = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-6);
        let ang = (dy / chord).clamp(-1.0, 1.0).acos().to_degrees();
        let reach = (tip.0 * tip.0 + tip.2 * tip.2).sqrt();
        let d_at = |f: f32| {
            let t = f * total;
            let k = cum.iter().position(|&x| x > t).unwrap_or(cum.len() - 1).saturating_sub(1).min(axis.len() - 1);
            200.0 * segs[axis[k]].radius_m
        };
        let sfb = snap.segment_safety[i];
        let (mut sfmin, mut at) = (f32::INFINITY, 0.0);
        for (j, &k) in axis.iter().enumerate() {
            if 2.0 * segs[k].radius_m < 0.05 {
                break;
            }
            if snap.segment_safety[k] < sfmin {
                sfmin = snap.segment_safety[k];
                at = cum[j] / total;
            }
        }
        rows.push((segs[i].start[1], d, dt, a_ratio, total, ang, reach, d_at(0.25), d_at(0.5), d_at(0.75), sfb, sfmin, at, 100.0 * leafy[i] as f32 / total_leafy));
    }
    for r in &rows {
        println!(
            "{:>5.1} {:>6.1} {:>6.1} {:>5.2} {:>5.2} | {:>5.1} {:>5.0} {:>5.1} | {:>5.1} {:>5.1} {:>5.1} | {:>5.2} {:>6.2} {:>5.2} | {:>5.1}",
            r.0,
            100.0 * r.1,
            100.0 * r.2,
            r.1 / r.2,
            r.3,
            r.4,
            r.5,
            r.6,
            r.7,
            r.8,
            r.9,
            r.10,
            r.11,
            r.12,
            r.13
        );
    }
    let mut ratios: Vec<f32> = rows.iter().map(|r| r.1 / r.2).collect();
    ratios.sort_by(f32::total_cmp);
    let mut sfs: Vec<f32> = rows.iter().map(|r| r.10).collect();
    sfs.sort_by(f32::total_cmp);
    if !ratios.is_empty() {
        println!("limbs {}: median dL/dT {:.2}, median SF base {:.2}", rows.len(), ratios[ratios.len() / 2], sfs[sfs.len() / 2]);
    }
    // Self-weight bending stress of living, leafless (woody) branch
    // segments by diameter class: median and 90th percentile (MPa), and
    // the class's share of the wood volume.
    let classes = [0.0f32, 0.01, 0.02, 0.05, 0.1, 0.2, 0.4, 10.0];
    let vol_total: f32 = (0..n).map(|i| segs[i].radius_m.powi(2) * len(i)).sum();
    for w in classes.windows(2) {
        let mut s: Vec<f32> = Vec::new();
        let mut vol = 0.0;
        for i in 1..n {
            let d = 2.0 * segs[i].radius_m;
            if on_trunk[i] || segs[i].dead || segs[i].leaf_weight > 0.0 || d < w[0] || d >= w[1] {
                continue;
            }
            s.push(snap.segment_stress_mpa[i]);
            vol += segs[i].radius_m.powi(2) * len(i);
        }
        if s.is_empty() {
            continue;
        }
        s.sort_by(f32::total_cmp);
        println!(
            "  d {:>4.0}-{:<4.0} cm: n {:>5}, stress median {:>5.1} p90 {:>5.1} max {:>5.1} MPa, wood {:>4.1} %",
            100.0 * w[0],
            100.0 * w[1].min(9.99),
            s.len(),
            s[s.len() / 2],
            s[s.len() * 9 / 10],
            s[s.len() - 1],
            100.0 * vol / vol_total
        );
    }
    let (ar, cnt) = sim.debug_area_ratio(0.05);
    let (ar2, cnt2) = sim.debug_area_ratio(0.01);
    println!("area ratio sum(children)/parent: r>=5cm {ar:.3} ({cnt}), r>=1cm {ar2:.3} ({cnt2})");
}
