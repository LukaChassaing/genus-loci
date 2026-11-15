//! Integration tests for dendro-core.

use dendro_core::{Genome, MeshData, Phase, ShapeMetrics, Simulation, TreeStats, LEAF_TRI_BUDGET, ROOT_TRI_BUDGET, WOOD_TRI_BUDGET};
use std::sync::OnceLock;

const SEED: u64 = 42;

fn oak(seed: u64) -> Simulation {
    Simulation::new(Genome::quercus_robur(), seed)
}

/// One shared 200-year run (yearly stats on Jan 1 + a mid-summer clone).
struct LongRun {
    yearly: Vec<TreeStats>,
    shape_15: ShapeMetrics,
    shape_80: ShapeMetrics,
    shape_100: ShapeMetrics,
    shape_200: ShapeMetrics,
    summer_200: Simulation,
    winter_10: Simulation,
}

fn long_run() -> &'static LongRun {
    static RUN: OnceLock<LongRun> = OnceLock::new();
    RUN.get_or_init(|| {
        let mut sim = oak(SEED);
        let mut yearly = vec![sim.stats()];
        let mut winter_10 = None;
        let mut shapes = Vec::new();
        for y in 1..=200 {
            sim.advance_years(1);
            yearly.push(sim.stats());
            if y == 15 || y == 80 || y == 100 {
                shapes.push(sim.shape_metrics());
            }
            if y == 10 {
                let mut w = sim.clone();
                w.advance_days(20); // Jan 21, year 10
                winter_10 = Some(w);
            }
        }
        let mut summer = sim.clone();
        summer.advance_days(200);
        LongRun {
            yearly,
            shape_15: shapes[0],
            shape_80: shapes[1],
            shape_100: shapes[2],
            shape_200: sim.shape_metrics(),
            summer_200: summer,
            winter_10: winter_10.unwrap(),
        }
    })
}

fn stats_bits(s: &TreeStats) -> Vec<u32> {
    vec![
        s.age_years.to_bits(),
        s.height_m.to_bits(),
        s.dbh_cm.to_bits(),
        s.crown_radius_m.to_bits(),
        s.root_depth_m.to_bits(),
        s.root_spread_m.to_bits(),
        s.shoot_nodes as u32,
        s.leafy_shoots as u32,
        s.root_nodes as u32,
    ]
}

fn mesh_bits(m: &MeshData) -> Vec<u32> {
    let mut v: Vec<u32> = m.positions.iter().flatten().map(|f| f.to_bits()).collect();
    v.extend(m.colors.iter().flatten().map(|f| f.to_bits()));
    v.extend(m.indices.iter().copied());
    v
}

fn assert_same(a: &Simulation, b: &Simulation) {
    assert_eq!(stats_bits(&a.stats()), stats_bits(&b.stats()));
    assert_eq!(a.rings().len(), b.rings().len());
    for (ra, rb) in a.rings().iter().zip(b.rings()) {
        assert_eq!(ra.width_mm.to_bits(), rb.width_mm.to_bits());
        assert_eq!(ra.water_stress.to_bits(), rb.water_stress.to_bits());
    }
    assert_eq!(a.soil_water().to_bits(), b.soil_water().to_bits());
    assert_eq!(a.temperature_c().to_bits(), b.temperature_c().to_bits());
    assert_eq!(mesh_bits(&a.wood_mesh()), mesh_bits(&b.wood_mesh()));
    assert_eq!(mesh_bits(&a.leaf_mesh()), mesh_bits(&b.leaf_mesh()));
    assert_eq!(mesh_bits(&a.root_mesh()), mesh_bits(&b.root_mesh()));
}

#[test]
fn deterministic_same_seed() {
    let mut a = oak(7);
    let mut b = oak(7);
    a.advance_days(365 * 25 + 150);
    b.advance_days(365 * 25 + 150);
    assert_same(&a, &b);
    let mut c = oak(8);
    c.advance_days(365 * 25 + 150);
    assert_ne!(stats_bits(&a.stats()), stats_bits(&c.stats()), "different seeds should differ");
}

#[test]
fn chunking_does_not_matter() {
    let mut a = oak(3);
    let mut b = oak(3);
    a.advance_days(365 * 6);
    b.advance_days(365 * 6);
    // One call of 365 days vs 365 calls of one day (spans a full season).
    a.advance_days(365);
    for _ in 0..365 {
        b.advance_days(1);
    }
    assert_same(&a, &b);
    // Odd chunk sizes, including a drought toggled at the same day.
    a.set_drought(true);
    b.set_drought(true);
    a.advance_days(97);
    b.advance_days(40);
    b.advance_days(57);
    a.set_drought(false);
    b.set_drought(false);
    a.advance_days(300);
    for _ in 0..30 {
        b.advance_days(10);
    }
    assert_same(&a, &b);
}

#[test]
fn growth_is_monotonic() {
    let run = long_run();
    for w in run.yearly.windows(2) {
        let (p, n) = (&w[0], &w[1]);
        assert!(n.dbh_cm >= p.dbh_cm, "DBH decreased at {}: {} -> {}", n.age_years, p.dbh_cm, n.dbh_cm);
        // Height can only drop by the loss of a top shoot or, in the old
        // decurrent crown (no leader), of a shaded-out top branch: allow a
        // small yearly drop, but no lasting decline.
        assert!(n.height_m >= p.height_m - 0.5, "height decreased at {}: {} -> {}", n.age_years, p.height_m, n.height_m);
    }
    for w in run.yearly.windows(11) {
        assert!(w[10].height_m >= w[0].height_m - 0.2, "height declined over a decade at {}", w[10].age_years);
    }
    let first = &run.yearly[1];
    let last = &run.yearly[200];
    assert!(last.height_m > first.height_m * 5.0);
}

#[test]
fn calibration_open_grown_oak() {
    let y = &long_run().yearly;
    let check = |age: usize, what: &str, v: f32, lo: f32, hi: f32| {
        assert!(v >= lo && v <= hi, "{what} at {age} y = {v:.2}, expected {lo}..{hi}");
    };
    // Loose bounds around the targets 1.5-3 m @5, 8-12 m @25, 18-25 m @100,
    // 25-35 m @200; DBH 30-50 cm @60, 80-120 cm @200.
    check(5, "height", y[5].height_m, 1.0, 3.5);
    check(25, "height", y[25].height_m, 7.0, 14.0);
    check(100, "height", y[100].height_m, 17.0, 28.0);
    check(200, "height", y[200].height_m, 24.0, 38.0);
    check(60, "dbh", y[60].dbh_cm, 25.0, 55.0);
    check(200, "dbh", y[200].dbh_cm, 75.0, 130.0);
    check(200, "crown radius", y[200].crown_radius_m, 8.0, 22.0);
    // Deep taproot early, then laterals spread much wider than deep.
    check(5, "root depth", y[5].root_depth_m, 0.6, 2.5);
    check(100, "root depth", y[100].root_depth_m, 1.5, 3.6);
    assert!(y[100].root_spread_m > 2.0 * y[100].root_depth_m);
    // Bounded structure (level of detail keeps the model fast).
    assert!(y[200].shoot_nodes < 80_000, "{} nodes", y[200].shoot_nodes);
    assert!(y[200].root_nodes < 15_000, "{} root nodes", y[200].root_nodes);
}

/// Open-grown oak architecture: clear bole, a few massive scaffold limbs,
/// broad crown.
#[test]
fn oak_architecture() {
    let run = long_run();
    let (s80, s100, s200) = (run.shape_80, run.shape_100, run.shape_200);
    // Clear bole ~10-20 % of height, 2-4(5) m on a mature tree.
    assert!((2.0..5.0).contains(&s80.clear_bole_m), "clear bole at 80 y: {s80:?}");
    assert!((2.0..5.0).contains(&s200.clear_bole_m), "clear bole at 200 y: {s200:?}");
    // About 5-9 scaffold limbs (>= 20 % of DBH) leave the stem.
    for s in [s80, s100, s200] {
        assert!((3..=10).contains(&s.scaffold_limbs), "scaffold limbs: {s:?}");
        // First-order limbs 25-50 % of the stem diameter (and some a bit more).
        assert!(s.max_limb_ratio >= 0.35 && s.max_limb_ratio <= 0.9, "limb/stem ratio: {s:?}");
        assert!(s.mean_limb_ratio >= 0.25, "limb/stem ratio: {s:?}");
    }
    // Broad crown about as wide as tall at maturity.
    assert!((0.8..1.3).contains(&s100.crown_width_to_height), "crown W/H at 100 y: {s100:?}");
    assert!((0.8..1.3).contains(&s200.crown_width_to_height), "crown W/H at 200 y: {s200:?}");
    // A young tree has a single dominant stem: no basal co-dominant fork.
    let s15 = run.shape_15;
    assert!(s15.max_limb_ratio < 0.75, "juvenile stem not dominant: {s15:?}");
}

#[test]
fn other_seeds_do_not_collapse() {
    for seed in [1u64, 1234] {
        let mut sim = oak(seed);
        for y in 1..=110u32 {
            sim.advance_years(1);
            let s = sim.stats();
            if y >= 20 {
                assert!(s.leafy_shoots > 1000, "seed {seed}: canopy collapsed at {y} y ({} shoots)", s.leafy_shoots);
            }
            if y == 25 {
                assert!((7.0..14.0).contains(&s.height_m), "seed {seed}: H25 {}", s.height_m);
            }
            if y == 60 {
                assert!((25.0..55.0).contains(&s.dbh_cm), "seed {seed}: DBH60 {}", s.dbh_cm);
            }
            if y == 100 {
                assert!((17.0..28.0).contains(&s.height_m), "seed {seed}: H100 {}", s.height_m);
                // Distinct individuals, same species architecture.
                let m = sim.shape_metrics();
                assert!((2.0..5.0).contains(&m.clear_bole_m), "seed {seed}: {m:?}");
                assert!((3..=10).contains(&m.scaffold_limbs), "seed {seed}: {m:?}");
                assert!((0.8..1.35).contains(&m.crown_width_to_height), "seed {seed}: {m:?}");
            }
        }
    }
}

#[test]
fn drought_gives_narrow_ring() {
    let mut ctrl = oak(SEED);
    ctrl.advance_years(40);
    let mut dry = ctrl.clone();
    dry.set_drought(true);
    dry.advance_years(1);
    dry.set_drought(false);
    ctrl.advance_years(1);
    let rc = ctrl.rings().last().unwrap();
    let rd = dry.rings().last().unwrap();
    assert_eq!(rc.year, 40);
    assert_eq!(rd.year, 40);
    assert!(rd.forced_drought && !rc.forced_drought);
    assert!(rd.water_stress > rc.water_stress);
    assert!(rd.width_mm < rc.width_mm, "drought ring {} vs control {}", rd.width_mm, rc.width_mm);
    assert!(rd.width_mm < 0.7 * rc.width_mm, "drought ring {} not clearly narrower than {}", rd.width_mm, rc.width_mm);
    // Ring-porous: earlywood comes from reserves and is unaffected the same year.
    assert_eq!(rd.earlywood_mm.to_bits(), rc.earlywood_mm.to_bits());
    assert!(rd.latewood_mm < rc.latewood_mm);
}

#[test]
fn ring_trend_is_plausible() {
    let run = long_run();
    let rings = run.summer_200.rings();
    assert_eq!(rings.len(), 200);
    let mean = |a: usize, b: usize| rings[a..b].iter().map(|r| r.width_mm).sum::<f32>() / (b - a) as f32;
    let juvenile = mean(0, 5);
    let prime = mean(20, 60);
    let old = mean(160, 200);
    assert!(prime > juvenile, "juvenile increase: {juvenile} -> {prime}");
    assert!(old < prime, "age decline: {prime} -> {old}");
    assert!((1.0..6.0).contains(&prime), "prime ring width {prime}");
    for r in rings {
        assert!(r.width_mm.is_finite() && r.width_mm >= 0.0);
        assert!((0.0..=1.0).contains(&r.earlywood_fraction));
        assert!((0.0..=1.0).contains(&r.water_stress));
    }
}

#[test]
fn phenology_cycle() {
    let mut sim = oak(11);
    sim.advance_years(15);
    let mut budburst = None;
    let mut saw = std::collections::HashSet::new();
    for _ in 0..365 {
        sim.advance_days(1);
        saw.insert(sim.phase());
        if budburst.is_none() && sim.phase() == Phase::BudBurst {
            budburst = Some(sim.day_of_year());
        }
        if sim.day_of_year() == 200 {
            assert_eq!(sim.phase(), Phase::Mature);
            assert!(sim.leaf_density() > 0.9);
            let c = sim.leaf_color();
            assert!(c[1] > c[0] && c[1] > c[2], "summer leaves should be green: {c:?}");
        }
        if sim.day_of_year() == 40 {
            assert_eq!(sim.phase(), Phase::Dormant);
        }
    }
    let bb = budburst.expect("no budburst");
    assert!((80..140).contains(&bb), "budburst doy {bb}");
    for p in [Phase::Dormant, Phase::BudBurst, Phase::LeafExpansion, Phase::Mature, Phase::Colouring, Phase::LeafFall] {
        assert!(saw.contains(&p), "phase {p:?} never reached");
    }
}

fn check_mesh(m: &MeshData, name: &str, budget: usize, check_winding: bool) {
    let n = m.positions.len();
    assert_eq!(m.normals.len(), n, "{name}");
    assert_eq!(m.uvs.len(), n, "{name}");
    assert_eq!(m.colors.len(), n, "{name}");
    assert_eq!(m.indices.len() % 3, 0, "{name}");
    assert!(m.triangle_count() <= budget, "{name}: {} tris > {budget}", m.triangle_count());
    for &i in &m.indices {
        assert!((i as usize) < n, "{name}: index out of range");
    }
    for (p, nr) in m.positions.iter().zip(&m.normals) {
        assert!(p.iter().all(|v| v.is_finite()), "{name}: NaN position");
        let l = (nr[0] * nr[0] + nr[1] * nr[1] + nr[2] * nr[2]).sqrt();
        assert!((l - 1.0).abs() < 1e-3, "{name}: normal length {l}");
    }
    for c in &m.colors {
        assert!(c.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)), "{name}: colour {c:?}");
    }
    if check_winding {
        // Clockwise front faces: geometric normal opposes the vertex normals.
        let (mut cw, mut total) = (0usize, 0usize);
        for t in m.indices.as_chunks::<3>().0 {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| m.positions[i as usize]);
            let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let g = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0]];
            let gl = (g[0] * g[0] + g[1] * g[1] + g[2] * g[2]).sqrt();
            if gl < 1e-9 {
                continue;
            }
            let nsum: [f32; 3] = [0, 1, 2].map(|k| t.iter().map(|&i| m.normals[i as usize][k]).sum());
            total += 1;
            if g[0] * nsum[0] + g[1] * nsum[1] + g[2] * nsum[2] < 0.0 {
                cw += 1;
            }
        }
        assert!(total > 0);
        assert!(cw as f32 / total as f32 > 0.99, "{name}: only {cw}/{total} triangles clockwise");
    }
}

/// No degenerate or oversized wood triangles (e.g. a branch collar or root
/// flare ring blown up into a sheet): every triangle is a thin slab of a
/// tube, so its longest edge is bounded by the longest simplified segment
/// and its width (2 x area / longest edge) by a fraction of the collar radius.
fn check_wood_triangles(sim: &Simulation, name: &str) {
    let m = sim.wood_mesh();
    let collar_r = sim.collar_diameter_cm() * 0.005;
    let d = |a: [f32; 3], b: [f32; 3]| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
    for t in m.indices.as_chunks::<3>().0 {
        let [a, b, c] = [t[0], t[1], t[2]].map(|i| m.positions[i as usize]);
        let e = d(a, b).max(d(b, c)).max(d(a, c));
        assert!(e <= 1.75, "{name}: wood triangle edge {e} m at {a:?}");
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let x = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        let area = 0.5 * (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt();
        let width = 2.0 * area / e.max(1e-6);
        assert!(width <= 0.5 * collar_r + 0.02, "{name}: wood triangle {width} m wide at {a:?} (collar r {collar_r})");
    }
}

#[test]
fn wood_triangles_bounded() {
    check_wood_triangles(&long_run().summer_200, "200 y");
    check_wood_triangles(&long_run().winter_10, "10 y");
    let mut sim = oak(1);
    sim.advance_days(365 * 60 + 200);
    check_wood_triangles(&sim, "seed 1, 60 y");
}

#[test]
fn mesh_sanity_old_tree() {
    let sim = &long_run().summer_200;
    let wood = sim.wood_mesh();
    let leaves = sim.leaf_mesh();
    let roots = sim.root_mesh();
    check_mesh(&wood, "wood", WOOD_TRI_BUDGET, true);
    check_mesh(&leaves, "leaves", LEAF_TRI_BUDGET, true);
    check_mesh(&roots, "roots", ROOT_TRI_BUDGET, true);
    assert!(wood.triangle_count() + roots.triangle_count() <= 300_000);
    assert!(leaves.triangle_count() <= 400_000);
    assert!(!leaves.is_empty(), "summer canopy must have leaves");
    // Roots below ground, wood above (allow a small overlap at the collar).
    let root_max_y = roots.positions.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
    assert!(root_max_y < 0.05, "roots reach y = {root_max_y}");
    let wood_min_y = wood.positions.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
    assert!(wood_min_y > -0.6, "wood reaches y = {wood_min_y}");
    let h = wood.positions.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
    assert!((h - sim.height_m()).abs() < 1.0);
}

#[test]
fn winter_leaves() {
    let young = &long_run().winter_10;
    assert_eq!(young.phase(), Phase::Dormant);
    let m = young.leaf_mesh();
    check_mesh(&m, "marcescent leaves", LEAF_TRI_BUDGET, true);
    assert!(!m.is_empty(), "young oak should keep some dead leaves (marcescence)");
    // An old oak has no marcescent leaves.
    let mut old = long_run().summer_200.clone();
    old.advance_days(165 + 30); // Jan 30 of the next year
    assert_eq!(old.phase(), Phase::Dormant);
    assert!(old.leaf_mesh().is_empty());
    assert!(!old.wood_mesh().is_empty());
}

#[test]
fn change_flags() {
    let mut sim = oak(5);
    assert!(sim.advance_days(1), "first call reports the initial geometry");
    sim.advance_days(400);
    let w = sim.wood_version();
    // Mid-winter: nothing grows.
    let mut s2 = oak(5);
    s2.advance_days(365 * 3 + 5);
    s2.advance_days(1);
    assert!(!s2.advance_days(3), "no change expected in early January");
    assert!(w > 1);
}

#[test]
fn genome_roundtrip() {
    let g = Genome::quercus_robur();
    let text = g.to_ron();
    let back = Genome::from_ron(&text).unwrap();
    assert_eq!(g, back);
    assert!(Genome::from_ron("(species: \"x\")").is_err());
}

/// Performance smoke test (run with `cargo test --release -- --ignored`).
#[test]
#[ignore]
fn performance_200_years() {
    let t = std::time::Instant::now();
    let mut sim = oak(SEED);
    sim.advance_years(200);
    let sim_s = t.elapsed().as_secs_f64();
    sim.advance_days(200);
    let t = std::time::Instant::now();
    let w = sim.wood_mesh();
    let l = sim.leaf_mesh();
    let r = sim.root_mesh();
    let mesh_s = t.elapsed().as_secs_f64();
    println!("200 years: {sim_s:.2} s; meshes {:.1} ms ({} + {} + {} tris)", mesh_s * 1e3, w.triangle_count(), l.triangle_count(), r.triangle_count());
    assert!(sim_s < 6.0);
    assert!(mesh_s < 0.2);
}
