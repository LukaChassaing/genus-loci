//! Static mechanics (`dendro_core::mechanics`): analytic beam cases, then
//! simulated trees against published orders of magnitude.

use dendro_core::mechanics::{wind_profile, AIR_DENSITY, GRAVITY, WOOD_DRAG_COEFFICIENT};
use dendro_core::{FailureMode, Genome, MechSegment, MechSite, MechanicsModel, MechanicsTraits, Simulation, WindLoad};
use std::f32::consts::PI;
use std::sync::OnceLock;

const SEEDS: [u64; 3] = [42, 1, 1234];

fn median(mut v: Vec<f32>) -> f32 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[v.len() / 2]
}

fn traits() -> MechanicsTraits {
    MechanicsTraits::default()
}

fn bare_site(dbh_m: f32) -> MechSite {
    MechSite { leaf_area_m2: 0.0, leaf_fraction: 0.0, crown_cell_m: 0.5, dbh_m, root_depth_m: 1.0, saturated_root_fraction: 0.0 }
}

/// A straight beam from `a` along `dir` (unit) of length `len`, cut in `k`
/// equal segments of radius `r`, chained parent -> child.
fn beam(a: [f32; 3], dir: [f32; 3], len: f32, k: usize, r: f32) -> Vec<MechSegment> {
    (0..k)
        .map(|i| {
            let t0 = len * i as f32 / k as f32;
            let t1 = len * (i + 1) as f32 / k as f32;
            MechSegment {
                id: i as u32,
                parent: if i == 0 { None } else { Some(i - 1) },
                start: [a[0] + dir[0] * t0, a[1] + dir[1] * t0, a[2] + dir[2] * t0],
                end: [a[0] + dir[0] * t1, a[1] + dir[1] * t1, a[2] + dir[2] * t1],
                radius_m: r,
                decay_radius_m: 0.0,
                leaf_weight: 0.0,
                dead: false,
            }
        })
        .collect()
}

fn close(a: f32, b: f32, rel: f32) -> bool {
    (a - b).abs() <= rel * b.abs().max(1e-9)
}

// ----------------------------------------------------------------------
// Analytic cases
// ----------------------------------------------------------------------

#[test]
fn horizontal_cantilever_under_self_weight_matches_beam_theory() {
    // Uniform cantilever of length L under its weight w per metre: M(x) =
    // w (L - x)^2 / 2 at distance x from the clamp; sigma = 32 M / (pi d^3).
    let (len, r, k) = (4.0f32, 0.05f32, 16usize);
    let t = traits();
    let segs = beam([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], len, k, r);
    let s = MechanicsModel::from_segments(&segs, t.clone(), bare_site(0.1)).evaluate(&WindLoad::new(0.0));
    let w = t.green_density_kg_m3 * PI * r * r * GRAVITY;
    let d = 2.0 * r;
    for (i, &m) in s.segment_moment_nm.iter().enumerate() {
        let x = len * i as f32 / k as f32;
        let expect = w * (len - x) * (len - x) / 2.0;
        assert!(close(m, expect, 1e-4), "segment {i}: M {m} vs {expect}");
        let sigma = 32.0 * expect / (PI * d * d * d) * 1e-6;
        assert!(close(s.segment_stress_mpa[i], sigma, 1e-4), "segment {i}: sigma {} vs {sigma}", s.segment_stress_mpa[i]);
        assert!(close(s.segment_safety[i].min(999.0), (t.green_mor_mpa / sigma).min(999.0), 1e-4));
    }
    assert!(close(s.wood_mass_kg, t.green_density_kg_m3 * PI * r * r * len, 1e-5));
}

#[test]
fn vertical_pole_in_wind_matches_hand_computation() {
    // Vertical pole (the stem: rigid, full cross-flow drag): no bending from
    // its weight; wind moment at the base = sum of segment forces x height.
    let (h, r, k) = (12.0f32, 0.1f32, 24usize);
    let segs = beam([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], h, k, r);
    let model = MechanicsModel::from_segments(&segs, traits(), bare_site(0.2));
    let calm = model.evaluate(&WindLoad::new(0.0));
    assert!(calm.segment_moment_nm.iter().all(|&m| m < 1e-3), "weight bends a vertical pole");
    let u = 30.0;
    let s = model.evaluate(&WindLoad::new(u));
    let dz = h / k as f32;
    let mut m0 = 0.0f32;
    let mut f0 = 0.0f32;
    for i in 0..k {
        let z = (i as f32 + 0.5) * dz;
        let uz = u * wind_profile(z);
        let f = 0.5 * AIR_DENSITY * WOOD_DRAG_COEFFICIENT * 2.0 * r * dz * uz * uz;
        m0 += f * z;
        f0 += f;
    }
    assert!(close(s.segment_moment_nm[0], m0, 1e-4), "base moment {} vs {m0}", s.segment_moment_nm[0]);
    assert!(close(s.base_moment_nm, m0, 1e-4));
    assert!(close(s.drag_n, f0, 1e-4));
    assert!(close(s.centre_of_pressure_m, m0 / f0, 1e-4));
}

#[test]
fn symmetric_branches_do_not_bend_the_stem() {
    // Two equal horizontal branches on opposite sides of a vertical stem:
    // their weights load the fork but cancel in the stem below (vector sum).
    let mut segs = beam([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], 4.0, 4, 0.12);
    for (j, sx) in [1.0f32, -1.0].iter().enumerate() {
        let mut b = beam([0.0, 4.0, 0.0], [*sx, 0.0, 0.0], 3.0, 3, 0.05);
        let base = segs.len();
        for (i, s) in b.iter_mut().enumerate() {
            s.id = 100 + 10 * j as u32 + i as u32;
            s.parent = Some(if i == 0 { 3 } else { base + i - 1 });
        }
        segs.extend(b);
    }
    let s = MechanicsModel::from_segments(&segs, traits(), bare_site(0.24)).evaluate(&WindLoad::new(0.0));
    for i in 0..4 {
        assert!(s.segment_moment_nm[i] < 1e-2, "stem segment {i} bent by {} N m", s.segment_moment_nm[i]);
    }
    assert!(s.segment_moment_nm[4] > 100.0, "the branches themselves are loaded");
}

#[test]
fn hollow_section_raises_stress_by_the_section_modulus_ratio() {
    // sigma = 32 M D / (pi (D^4 - Di^4)): with Di = 0.7 D (shell t/R = 0.3,
    // the failure threshold of Mattheck & Breloer 1994) the stress rises by
    // 1 / (1 - 0.7^4) = 1.316 for the same moment.
    let segs = beam([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], 3.0, 6, 0.08);
    let mut hollow = segs.clone();
    hollow[0].decay_radius_m = 0.7 * 0.08;
    let a = MechanicsModel::from_segments(&segs, traits(), bare_site(0.16)).evaluate(&WindLoad::new(0.0));
    let b = MechanicsModel::from_segments(&hollow, traits(), bare_site(0.16)).evaluate(&WindLoad::new(0.0));
    let ratio = b.segment_stress_mpa[0] / a.segment_stress_mpa[0];
    assert!(close(ratio, 1.0 / (1.0 - 0.7f32.powi(4)), 1e-4), "ratio {ratio}");
    assert!(b.segment_safety[0] < a.segment_safety[0]);
    assert!(close(b.segment_stress_mpa[1], a.segment_stress_mpa[1], 1e-6), "only the hollow segment changes");
    assert!(b.weakest.unwrap().decayed);
}

#[test]
fn crown_drag_follows_the_vogel_exponent() {
    // A crown of leafy tips all at 10 m (profile factor 1) on a hair-thin
    // stem (negligible wood drag): between 10 and 25 m/s drag ~ U^(2+B);
    // with B = -1, doubling U from 12 to 24 m/s doubles the drag (x4 for a
    // rigid body); above 25 m/s, fully streamlined, ~U^2 again.
    let mut segs = beam([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], 10.0, 10, 1e-5);
    for j in 0..20u32 {
        segs.push(MechSegment {
            id: 1000 + j,
            parent: Some(9),
            start: [0.0, 10.0, 0.0],
            end: [0.0, 10.0, 1.5 * (j as f32 / 19.0 - 0.5)],
            radius_m: 1e-5,
            decay_radius_m: 0.0,
            leaf_weight: 1.0,
            dead: false,
        });
    }
    let site = MechSite { leaf_area_m2: 10.0, leaf_fraction: 1.0, ..bare_site(0.2) };
    let model = MechanicsModel::from_segments(&segs, traits(), site);
    let drag = |u: f32| model.evaluate(&WindLoad { leaf_fraction: Some(1.0), ..WindLoad::new(u) }).drag_n;
    let ratio = drag(24.0) / drag(12.0);
    assert!(close(ratio, 2.0f32.powf(2.0 + traits().vogel_exponent), 2e-3), "ratio {ratio}");
    let ratio_hi = drag(60.0) / drag(30.0);
    assert!(close(ratio_hi, 4.0, 2e-3), "ratio above the reconfiguration range {ratio_hi}");
    // Drag = 1/2 rho Cd A0 U^2 with A0 the rasterised frontal area.
    let s = model.evaluate(&WindLoad { leaf_fraction: Some(1.0), ..WindLoad::new(20.0) });
    let expect = 0.5 * AIR_DENSITY * traits().crown_drag_coefficient * s.crown_frontal_area_m2 * 400.0;
    assert!(close(s.drag_n, expect, 2e-3), "drag {} vs {expect}", s.drag_n);
}

// ----------------------------------------------------------------------
// Simulated trees
// ----------------------------------------------------------------------

/// Trees on the reference soil (deep loam), seeds `SEEDS`, at 60 and 150
/// years: `[species][age][seed]`.
fn trees() -> &'static Vec<Vec<Vec<Simulation>>> {
    static T: OnceLock<Vec<Vec<Vec<Simulation>>>> = OnceLock::new();
    T.get_or_init(|| {
        Genome::builtin()
            .into_iter()
            .map(|g| {
                let mut at60 = Vec::new();
                let mut at150 = Vec::new();
                for &seed in &SEEDS {
                    let mut s = Simulation::new(g.clone(), seed);
                    s.advance_years(60);
                    at60.push(s.checkpoint());
                    s.advance_years(90);
                    at150.push(s);
                }
                vec![at60, at150]
            })
            .collect()
    })
}

const AGES: [u32; 2] = [60, 150];

fn leafless() -> WindLoad {
    WindLoad { leaf_fraction: Some(0.0), ..WindLoad::new(0.0) }
}

fn in_leaf(u: f32) -> WindLoad {
    WindLoad { leaf_fraction: Some(1.0), ..WindLoad::new(u) }
}

#[test]
fn more_wind_means_less_safety() {
    for (sp, by_age) in trees().iter().enumerate() {
        for (a, sims) in by_age.iter().enumerate() {
            let model = sims[0].mechanics_model();
            let mut last = f32::INFINITY;
            for u in [0.0, 10.0, 20.0, 30.0, 40.0, 50.0] {
                let s = model.evaluate(&in_leaf(u));
                let worst = s.stem_safety.min(s.limb_safety).min(s.anchorage_safety);
                assert!(worst <= last + 1e-4, "species {sp} age {}: safety rose from {last} to {worst} at {u} m/s", AGES[a]);
                last = worst;
                if u > 0.0 {
                    assert!(s.drag_n > 0.0);
                }
            }
        }
    }
}

#[test]
fn leafless_crown_withstands_more_wind_than_crown_in_leaf() {
    // Broadleaves catch much more wind in leaf (Vollsinger et al. 2005 for
    // the crown drag; winter storms meet bare crowns).
    for (sp, by_age) in trees().iter().enumerate() {
        for (a, sims) in by_age.iter().enumerate() {
            let bare: Vec<f32> = sims.iter().map(|s| s.mechanics_model().critical_speeds(&leafless()).first().map_or(80.0, |f| f.1)).collect();
            let leafy: Vec<f32> = sims.iter().map(|s| s.mechanics_model().critical_speeds(&in_leaf(0.0)).first().map_or(80.0, |f| f.1)).collect();
            let (b, l) = (median(bare.clone()), median(leafy.clone()));
            assert!(b > l, "species {sp} age {}: leafless {b:.1} m/s vs in leaf {l:.1} m/s ({bare:?} / {leafy:?})", AGES[a]);
            // Same drag at the same speed: the leafless crown is less loaded.
            let s0 = &sims[0];
            let m = s0.mechanics_model();
            assert!(m.evaluate(&WindLoad { leaf_fraction: Some(0.0), ..WindLoad::new(30.0) }).base_moment_nm < m.evaluate(&in_leaf(30.0)).base_moment_nm);
        }
    }
}

#[test]
fn decay_reduces_safety() {
    // (1) Injected decay: hollowing (shell t/R = 0.3) the weakest stem
    // section of a sound 60-year oak lowers its stem safety and its
    // critical stem-breaking speed.
    let oak = &trees()[0][0][0];
    let (segs, site) = oak.mechanics_inputs();
    let t = oak.genome().mechanics.clone();
    let storm = WindLoad { leaf_fraction: Some(0.0), ..WindLoad::new(30.0) };
    let sound = MechanicsModel::from_segments(&segs, t.clone(), site.clone());
    let a = sound.evaluate(&storm);
    let weak = a.weakest.expect("structural sections");
    let stem_idx = (0..segs.len())
        .filter(|&i| a.segment_role[i] == dendro_core::SegmentRole::Trunk && 2.0 * segs[i].radius_m >= 0.2 * site.dbh_m)
        .min_by(|&i, &j| a.segment_safety[i].total_cmp(&a.segment_safety[j]))
        .expect("stem sections");
    let mut hollow = segs.clone();
    hollow[stem_idx].decay_radius_m = 0.7 * segs[stem_idx].radius_m;
    let rotten = MechanicsModel::from_segments(&hollow, t, site);
    let b = rotten.evaluate(&storm);
    assert!(b.stem_safety < a.stem_safety, "stem safety {} -> {} (weakest {weak:?})", a.stem_safety, b.stem_safety);
    let (ca, cb) = (sound.critical_speeds(&leafless()), rotten.critical_speeds(&leafless()));
    assert!(cb.stem_break.unwrap_or(80.0) < ca.stem_break.unwrap_or(80.0), "critical stem speed {ca:?} -> {cb:?}");
    // (2) Decay from the pruning model reaches the mechanics: topping a
    // 60-year oak at half height leaves a decay column in the stem (see
    // tests/pruning.rs, big_wound_decays_small_one_closes); 20 years later
    // it is still standing (later, the decayed head breaks off).
    let mut topped = Simulation::new(Genome::quercus_robur(), 42);
    topped.advance_days(60 * 365 + 20);
    let h = topped.height_m();
    topped.top_at(0.5 * h);
    topped.advance_years(20);
    let (segs, site) = topped.mechanics_inputs();
    assert!(segs.iter().any(|s| s.decay_radius_m > 0.0), "no decayed segment after topping");
    let t = topped.genome().mechanics.clone();
    let mut cured = segs.clone();
    for s in cured.iter_mut() {
        s.decay_radius_m = 0.0;
    }
    let real = MechanicsModel::from_segments(&segs, t.clone(), site.clone()).evaluate(&storm);
    let healthy = MechanicsModel::from_segments(&cured, t, site).evaluate(&storm);
    let mut lower = 0;
    for (i, (x, y)) in real.segment_stress_mpa.iter().zip(&healthy.segment_stress_mpa).enumerate() {
        assert!(x >= y, "segment {i}: decay lowered the stress {y} -> {x}");
        assert!(real.segment_safety[i] <= healthy.segment_safety[i]);
        if x > y {
            lower += 1;
        }
    }
    assert!(lower > 0, "decay changed no section");
}

#[test]
fn saturated_soil_lowers_the_uprooting_speed() {
    // Gleyed vs freely draining soil (ForestGALES 2.5): x0.886 on the
    // critical turning moment; turning moment ~ U^2 above the
    // reconfiguration range, so the critical speed drops by ~sqrt(0.886).
    let oak = &trees()[0][0][0];
    let (segs, site) = oak.mechanics_inputs();
    let t = oak.genome().mechanics.clone();
    let wet_site = MechSite { saturated_root_fraction: 1.0, ..site.clone() };
    let dry = MechanicsModel::from_segments(&segs, t.clone(), MechSite { saturated_root_fraction: 0.0, ..site });
    let wet = MechanicsModel::from_segments(&segs, t, wet_site);
    assert!(close(wet.critical_turning_moment_nm() / dry.critical_turning_moment_nm(), 135.8 / 153.2, 1e-5));
    let (d, w) = (dry.critical_speeds(&leafless()).uprooting.unwrap(), wet.critical_speeds(&leafless()).uprooting.unwrap());
    assert!(w < d, "uprooting {d:.1} -> {w:.1} m/s on saturated soil");
}

#[test]
fn self_weight_safety_is_plausible() {
    // Target (estimated, no direct measurement of stress safety under
    // self-weight found): >= 2 for the stem and the structural limbs of a
    // sound tree in full leaf, calm air. McMahon 1973 (Science 179:1201)
    // found trees ~4x below their elastic buckling height (Greenhill), a
    // comparable margin; Wessolly's static assessment asks >= 1.5 against
    // a Beaufort 12 wind on top of the weight.
    // Known gap (docs/sources.md, « Charpentières »): the limbs of the old
    // oak are too slender; that case is checked separately below.
    for (sp, by_age) in trees().iter().enumerate() {
        for (a, sims) in by_age.iter().enumerate() {
            let snaps: Vec<_> = sims.iter().map(|s| s.mechanics_model().evaluate(&in_leaf(0.0))).collect();
            let stem = median(snaps.iter().map(|s| s.stem_safety).collect());
            let limb = median(snaps.iter().map(|s| s.limb_safety).collect());
            let buckling = median(snaps.iter().map(|s| s.buckling_safety).collect());
            assert!(stem >= 2.0, "species {sp} age {}: stem safety {stem:.2}", AGES[a]);
            assert!(buckling > 1.0, "species {sp} age {}: buckles under its own weight ({buckling:.2})", AGES[a]);
            let old_oak = sp == 0 && AGES[a] == 150;
            if !old_oak {
                assert!(limb >= 2.0, "species {sp} age {}: limb safety {limb:.2}", AGES[a]);
            }
            let mass = median(snaps.iter().map(|s| s.above_ground_mass_kg).collect());
            assert!(mass > 0.0);
        }
    }
}

#[test]
fn old_oak_limbs_known_gap() {
    // Sentinel of a known gap, not a target: the 150-year oak's structural
    // limbs carry their weight with a safety of only ~1.5-1.7 (median of
    // three seeds), below the >= 2 expected of sound trees, because the
    // simulated limbs are too slender (docs/sources.md, « Charpentières » and
    // « Mécanique »). When the limb allometry is fixed this test will fail:
    // move the case into `self_weight_safety_is_plausible`.
    let snaps: Vec<_> = trees()[0][1].iter().map(|s| s.mechanics_model().evaluate(&in_leaf(0.0))).collect();
    let limb = median(snaps.iter().map(|s| s.limb_safety).collect());
    assert!(limb > 1.0, "old oak limbs break under their own weight: {limb:.2}");
    assert!(limb < 2.0, "old oak limbs now reach the target ({limb:.2}): fold this case into self_weight_safety_is_plausible");
}

#[test]
fn adult_critical_gusts_in_published_range() {
    // Target: the first failure of a sound adult broadleaf, leafless (the
    // European winter storms), at a 10 m gust speed of 30-45 m/s (median of
    // three seeds). Anchors: the static tree assessment of Sinn & Wessolly
    // (1989) loads trees with a 32 m/s Beaufort-12 wind that a sound tree
    // must resist; storm data show almost all trunks break above ~42 m/s,
    // whatever the species and size (Virot et al. 2016, Phys Rev E
    // 93:023001).
    // The band around the two anchors is our choice (estimated).
    // Checked: oak at 60 and 150 years, cherry at 60. The 150-year cherry
    // (beyond its usual lifespan, a small senescent crown) comes out above
    // the band (~56-59 m/s): reported in docs/sources.md, not tested.
    let cases = [(0usize, 0usize), (0, 1), (1, 0)];
    for (sp, a) in cases {
        let first: Vec<f32> = trees()[sp][a].iter().map(|s| s.mechanics_model().critical_speeds(&leafless()).first().map_or(80.0, |f| f.1)).collect();
        let m = median(first.clone());
        assert!((30.0..=45.0).contains(&m), "species {sp} age {}: first failure at {m:.1} m/s ({first:?})", AGES[a]);
    }
}

#[test]
fn snapshot_is_deterministic_and_reports_modes() {
    let s = &trees()[1][0][0];
    let a = s.mechanics_snapshot(&WindLoad::new(35.0));
    let b = s.mechanics_snapshot(&WindLoad::new(35.0));
    assert_eq!(a, b);
    assert_eq!(a.segment_safety.len(), s.shoot_nodes().len());
    assert!(a.weakest.is_some() && a.above_ground_mass_kg > a.wood_mass_kg * 0.99);
    let (mode, speed) = a.critical.first().expect("some failure below 80 m/s");
    assert!(speed > 0.0);
    // At the critical speed of the first mode, that mode is (about) the
    // expected one.
    let at = s.mechanics_model().evaluate(&WindLoad::new(speed + 0.5));
    let sf = match mode {
        FailureMode::StemBreak => at.stem_safety,
        FailureMode::LimbBreak => at.limb_safety,
        FailureMode::Uprooting => at.anchorage_safety,
    };
    assert!(sf < 1.0, "{mode:?} at {speed} m/s: safety {sf}");
    // The mechanics is a reading: the tree is untouched.
    let h = s.state_hash();
    let _ = s.mechanics_snapshot(&WindLoad::new(50.0));
    assert_eq!(s.state_hash(), h);
}

#[test]
#[ignore]
fn performance_mechanics_old_oak() {
    // Cost budget: one evaluation is O(n); the critical speeds (~80 passes)
    // of a 150-year oak (~24 000 segments) stay well under 0.2 s (release).
    let s = &trees()[0][1][0];
    let t0 = std::time::Instant::now();
    let m = s.mechanics_model();
    let _ = m.evaluate(&WindLoad::new(30.0));
    let one = t0.elapsed().as_secs_f32();
    let t1 = std::time::Instant::now();
    let _ = m.critical_speeds(&leafless());
    let crit = t1.elapsed().as_secs_f32();
    println!("model + 1 evaluation {:.1} ms, critical speeds {:.1} ms ({} segments)", one * 1e3, crit * 1e3, m.segment_count());
    assert!(one < 0.02 && crit < 0.2);
}
