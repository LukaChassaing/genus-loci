//! Static mechanics (`dendro_core::mechanics`): analytic beam cases, then
//! simulated trees against published orders of magnitude.

use dendro_core::mechanics::dynamic_factor;
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
    MechSite { leaf_area_m2: 0.0, leaf_fraction: 0.0, crown_cell_m: 0.5, dbh_m, root_depth_m: 1.0, saturated_root_fraction: 0.0, leaf_lma_g_m2: 75.0 }
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
    // Updated (M1 step 2): the hand computation is the quasi-static,
    // small-deflection case (`quasi_static`); the dynamic factor and the
    // large deflections are checked by their own tests below.
    let (h, r, k) = (12.0f32, 0.1f32, 24usize);
    let segs = beam([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], h, k, r);
    let model = MechanicsModel::from_segments(&segs, traits(), bare_site(0.2));
    let calm = model.evaluate(&WindLoad::new(0.0));
    assert!(calm.segment_moment_nm.iter().all(|&m| m < 1e-3), "weight bends a vertical pole");
    let u = 30.0;
    let s = model.evaluate(&WindLoad { quasi_static: true, ..WindLoad::new(u) });
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
    for (i, m) in s.segment_moment_nm.iter().take(4).enumerate() {
        assert!(*m < 1e-2, "stem segment {i} bent by {m} N m");
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

#[test]
fn uniform_cantilever_frequency_matches_beam_theory() {
    // Fundamental frequency of a uniform cantilever (Euler-Bernoulli):
    // f1 = (1.8751² / 2π) √(E I / (m L⁴)), m mass per length (any vibration
    // textbook, e.g. Clough & Penzien). Rayleigh's quotient with the static
    // deflection shape under a uniform lateral load gives 3.530 instead of
    // the exact 3.516 (+0.4 %), plus the discretisation: within 1.5 %.
    let (h, r, k) = (10.0f32, 0.1f32, 200usize);
    let segs = beam([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], h, k, r);
    let t = traits();
    let model = MechanicsModel::from_segments(&segs, t.clone(), bare_site(0.2));
    let d = model.dynamics(&WindLoad { leaf_fraction: Some(0.0), ..WindLoad::new(0.0) });
    let e = t.green_moe_gpa * 1e9;
    let i = PI * (2.0 * r).powi(4) / 64.0;
    let m = t.green_density_kg_m3 * PI * r * r;
    let exact = 1.875_104f32.powi(2) / (2.0 * PI) * (e * i / (m * h.powi(4))).sqrt();
    assert!(close(d.natural_frequency_hz, exact, 0.015), "f {} Hz vs {exact} Hz", d.natural_frequency_hz);
    assert!(d.natural_frequency_hz >= exact * 0.999, "Rayleigh is an upper bound");
    // Damping in still air: the leafless value of the traits (no leaves).
    assert!(close(d.damping_ratio, t.damping_leafless, 1e-6));
}

#[test]
fn dynamic_factor_follows_en_1991_1_4() {
    // Hand computation of EN 1991-1-4 Annex B for h = 20 m, b = 15 m, n =
    // 0.4 Hz, ζ = 0.1 in a 10-min mean wind of 20 m/s at 10 m (z0 = 0.03 m).
    let (h, b, n, zeta, v10) = (20.0f32, 15.0f32, 0.4f32, 0.1f32, 20.0f32);
    let z0 = 0.03f32;
    let zs = 0.6 * h;
    let iv = 1.0 / (zs / z0).ln();
    let vm = v10 * (zs / z0).ln() / (10.0f32 / z0).ln();
    let alpha = 0.67 + 0.05 * z0.ln();
    let l = 300.0 * (zs / 200.0).powf(alpha);
    let fl = n * l / vm;
    let sl = 6.8 * fl / (1.0 + 10.2 * fl).powf(5.0 / 3.0);
    let b2 = 1.0 / (1.0 + 0.9 * ((b + h) / l).powf(0.63));
    let rr = |eta: f32| 1.0 / eta - (1.0 - (-2.0 * eta).exp()) / (2.0 * eta * eta);
    let r2 = PI * PI / (2.0 * 2.0 * PI * zeta) * sl * rr(4.6 * h * fl / l) * rr(4.6 * b * fl / l);
    let nu = (n * (r2 / (b2 + r2)).sqrt()).max(0.08);
    let x = (2.0 * (nu * 600.0).ln()).sqrt();
    let kp = (x + 0.6 / x).max(3.0);
    let cd = (1.0 + 2.0 * kp * iv * (b2 + r2).sqrt()) / (1.0 + 7.0 * iv * b2.sqrt());
    let got = dynamic_factor(h, b, n, zeta, v10);
    assert!(close(got.cd, cd, 1e-3), "c_d {} vs {cd}", got.cd);
    assert!(close(got.resonant, r2, 1e-3) && close(got.background, b2, 1e-3));
    // Less damping, more resonance; a stiffer (higher n) tree, less.
    assert!(dynamic_factor(h, b, n, 0.02, v10).cd > got.cd);
    assert!(dynamic_factor(h, b, 2.0, zeta, v10).resonant < got.resonant);
    // For trees (n 0.2-1 Hz, ζ 0.05-0.4) the factor stays near 1: Sellier &
    // Fourcaud 2009 (Am J Bot 96:885-896, via the review of James et al.
    // 2014, Arboric Urban For 40:1-15) report 0.98-1.19 for a pine.
    for (n, z) in [(0.2, 0.05), (0.3, 0.1), (0.6, 0.3), (1.0, 0.4)] {
        let c = dynamic_factor(25.0, 15.0, n, z, 25.0).cd;
        assert!((0.9..1.25).contains(&c), "n {n} ζ {z}: c_d {c}");
    }
}

#[test]
fn flexible_stem_bows_and_sheds_load() {
    // A slender pole with a leafy crown at its top: with large deflections
    // the stem bows downwind, the crown sinks and turns its sail, so the
    // base moment is lower than in the small-deflection statics once the
    // top moves by a large part of the height; for moderate deflections the
    // weight of the displaced crown (P-Δ) offsets the gain.
    let crown = |base: usize, top: f32| -> Vec<MechSegment> {
        (0..20u32)
            .map(|j| MechSegment {
                id: 1000 + j,
                parent: Some(base),
                start: [0.0, top, 0.0],
                end: [0.0, top + 0.5, 1.5 * (j as f32 / 19.0 - 0.5)],
                radius_m: 0.004,
                decay_radius_m: 0.0,
                leaf_weight: 1.0,
                dead: false,
            })
            .collect()
    };
    let run = |r: f32| {
        let mut segs = beam([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], 6.0, 30, r);
        segs.extend(crown(29, 6.0));
        let site = MechSite { leaf_area_m2: 20.0, leaf_fraction: 1.0, ..bare_site(2.0 * r) };
        let m = MechanicsModel::from_segments(&segs, traits(), site);
        let load = |qs: bool| WindLoad { leaf_fraction: Some(1.0), quasi_static: qs, ..WindLoad::new(30.0) };
        let (a, b) = (m.evaluate(&load(true)), m.evaluate(&load(false)));
        (a.base_moment_nm, b.base_moment_nm / b.dynamic_factor, b.top_deflection_m)
    };
    for r in [0.02f32, 0.025, 0.03, 0.04] {
        let (qs, ld, defl) = run(r);
        println!("r {r}: quasi-static {qs:.0} N m, large deflection {ld:.0} N m, top {defl:.2} m");
    }
    // Very flexible (top displaced by most of the height): the load drops.
    let (qs, ld, defl) = run(0.02);
    assert!(defl > 3.0, "a 4 cm, 6 m pole in 30 m/s should bow: {defl} m");
    assert!(ld < 0.6 * qs, "flexible: {ld} vs {qs} N m");
    // Moderate deflection (< 10 % of the height): the weight's P-Δ moment
    // and the shorter lever nearly cancel, the statics hold within 6 %.
    let (qs, ld, defl) = run(0.04);
    assert!(defl < 0.1 * 6.0 && close(ld, qs, 0.06), "stiff: {ld} vs {qs} N m, deflection {defl} m");
}

// ----------------------------------------------------------------------
// Simulated trees
// ----------------------------------------------------------------------

/// Trees on the reference soil (deep loam), seeds `SEEDS`, at 60 and 150
/// years: `(species id, [age][seed])`, in registry order. Tests name the
/// species they check by id (`tree("quercus_robur")`): the registry order
/// changes whenever a species file is added.
/// `(species id, typical lifespan in years, [age][seed])`.
type SpeciesTrees = (String, f32, Vec<Vec<Simulation>>);

fn trees() -> &'static Vec<SpeciesTrees> {
    static T: OnceLock<Vec<SpeciesTrees>> = OnceLock::new();
    T.get_or_init(|| {
        // Every deciduous species of the registry: the claims below are
        // about crowns that are bare in winter. Evergreens are excluded on
        // purpose (their winter crown keeps its needles, and the in-leaf
        // stem of a monopodial conifer is still too weak, docs/sources.md
        // « Écarts connus »). No other exception: the apple was excluded
        // until the bare crown wood was sheltered and capped by the leafy
        // envelope (mechanics.rs, `CROWN_WOOD_LEAF_CAP`).
        Genome::builtin()
            .into_iter()
            .filter(|g| !g.leaf.evergreen())
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
                (g.id.clone(), g.pruning.typical_lifespan_years, vec![at60, at150])
            })
            .collect()
    })
}

/// The trees of one species, by id: `[age][seed]`.
fn tree(id: &str) -> &'static Vec<Vec<Simulation>> {
    &trees().iter().find(|(i, _, _)| i == id).unwrap_or_else(|| panic!("no deciduous species {id}")).2
}

/// Every deciduous species, with only the ages (indices into `AGES`) that
/// fall within its typical lifespan (`pruning.typical_lifespan_years`): a
/// 150-year hazel (a stem lives 60-90 years) or plum is not a tree the
/// claims below are about. Decided with the user (2026-11-18) when the
/// 150-year hazel's limb safety rose from 2.25 to 2.54 at 10 m/s (the wind
/// moment partly cancelling a limb's self-weight moment).
fn living_trees() -> impl Iterator<Item = (&'static str, usize, &'static Vec<Simulation>)> {
    trees().iter().flat_map(|(id, life, by_age)| {
        by_age.iter().enumerate().filter(move |(a, _)| AGES[*a] as f32 <= *life).map(move |(a, sims)| (id.as_str(), a, sims))
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
    // From 20 m/s on, more wind means less safety, and a 50 m/s gust leaves
    // the tree less safe than still air. Below 20 m/s a light wind may
    // relieve a limb: its moment partly cancels the limb's self-weight
    // moment (150-year hazel: limb safety 2.25 -> 2.54 at 10 m/s; 150-year
    // pear: 6.4 -> 7.5). Decided with the user (2026-11-18); the test used
    // to require a monotonic fall from 0 m/s.
    // Updated (user decision, 2026-11-20): the monotonic fall is checked on
    // the quasi-static statics, where it follows from the physics (drag
    // grows with speed on a fixed geometry). With large deflections a
    // flexible stem may shed load at high speed (60-year beech: safety 0.79
    // at 40 m/s, 0.87 at 50 m/s); that is tested on its own below
    // (`large_deflections_unload_a_flexible_tree_at_high_speed`).
    for (sp, a, sims) in living_trees() {
        let model = sims[0].mechanics_model();
        let worst_at = |u: f32| {
            let s = model.evaluate(&WindLoad { quasi_static: true, ..in_leaf(u) });
            if u > 0.0 {
                assert!(s.drag_n > 0.0);
            }
            s.stem_safety.min(s.limb_safety).min(s.anchorage_safety)
        };
        let calm = worst_at(0.0);
        let _ = worst_at(10.0);
        let mut last = f32::INFINITY;
        for u in [20.0, 30.0, 40.0, 50.0] {
            let worst = worst_at(u);
            assert!(worst <= last + 1e-4, "species {sp} age {}: safety rose from {last} to {worst} at {u} m/s", AGES[a]);
            last = worst;
        }
        assert!(last < calm, "species {sp} age {}: safety at 50 m/s {last} not below still air {calm}", AGES[a]);
    }
}

#[test]
fn large_deflections_unload_a_flexible_tree_at_high_speed() {
    // Reconfiguration of flexible bodies: a stem that bends in the wind
    // turns its crown away and lowers it into slower air, so its load grows
    // more slowly than on the rigid geometry (Vogel 1989, J Exp Bot
    // 40:941-948; de Langre 2008, Annu Rev Fluid Mech 40:141-168, both cited
    // in mechanics.rs). Decision of the user (2026-11-20): this replaces the
    // monotonic safety of the dynamic model in `more_wind_means_less_safety`.
    // On every simulated tree at 50 m/s in leaf, the moment at the collar
    // with large deflections (divided by the dynamic factor, which the
    // quasi-static case does not apply) is not above the quasi-static one by
    // more than the P-Delta allowance of `flexible_stem_bows_and_sheds_load`
    // (6 %; the weight of the displaced crown, P-Delta, can offset the
    // shorter lever), and the most flexible trees shed a clear share of it
    // (at least one tree below 0.9). The top displacement alone does not
    // tell which trees unload: a thin leader may swing several metres
    // without unloading the collar (150-year oak 5.5 m of 28.9 m, ratio
    // 1.013; 150-year sycamore 7.7 m, 1.013).
    let mut flexible = 0;
    for (sp, a, sims) in living_trees() {
        let model = sims[0].mechanics_model();
        let h = sims[0].height_m();
        let qs = model.evaluate(&WindLoad { quasi_static: true, ..in_leaf(50.0) });
        let ld = model.evaluate(&in_leaf(50.0));
        let ratio = ld.base_moment_nm / ld.dynamic_factor / qs.base_moment_nm;
        println!("{sp} {} y: top deflection {:.2} m of {h:.1} m, collar moment large deflection / quasi-static {ratio:.3}", AGES[a], ld.top_deflection_m);
        assert!(ratio < 1.06, "species {sp} age {}: {ratio}", AGES[a]);
        if ratio < 0.9 {
            flexible += 1;
        }
    }
    assert!(flexible > 0, "no simulated tree sheds 10 % of its collar moment at 50 m/s");
}

#[test]
fn leafless_crown_withstands_more_wind_than_crown_in_leaf() {
    // Broadleaves catch much more wind in leaf (Vollsinger et al. 2005 for
    // the crown drag; winter storms meet bare crowns).
    for (sp, a, sims) in living_trees() {
        {
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
    let oak = &tree("quercus_robur")[0][0];
    let (segs, site) = oak.mechanics_inputs();
    let t = oak.genome().mechanics.clone();
    let storm = WindLoad { leaf_fraction: Some(0.0), ..WindLoad::new(30.0) };
    let sound = MechanicsModel::from_segments(&segs, t.clone(), site.clone());
    let a = sound.evaluate(&storm);
    let weak = a.weakest.expect("structural sections");
    let stem_idx = (0..segs.len())
        // Updated (M1 wind): the stem mode now covers every stem section a
        // storm can break (from 5 cm, `segment_breakable`), no longer only
        // those of at least 20 % of the DBH; the weakest one is hollowed.
        .filter(|&i| a.segment_role[i] == dendro_core::SegmentRole::Trunk && a.segment_breakable[i])
        .min_by(|&i, &j| a.segment_safety[i].total_cmp(&a.segment_safety[j]))
        .expect("stem sections");
    let mut hollow = segs.clone();
    hollow[stem_idx].decay_radius_m = 0.7 * segs[stem_idx].radius_m;
    let rotten = MechanicsModel::from_segments(&hollow, t, site);
    let b = rotten.evaluate(&storm);
    assert!(b.stem_safety < a.stem_safety, "stem safety {} -> {} (weakest {weak:?})", a.stem_safety, b.stem_safety);
    // Updated (M1 step 2): compared on the quasi-static statics. With large
    // deflections, a hollowed (less stiff) stem bows more at its ~70 m/s
    // critical gust and sheds a little load, which hides the loss of
    // strength in this comparison (seen: 70.77 -> 70.80 m/s).
    let qs = WindLoad { quasi_static: true, ..leafless() };
    let (ca, cb) = (sound.critical_speeds(&qs), rotten.critical_speeds(&qs));
    assert!(cb.stem_break.unwrap_or(80.0) < ca.stem_break.unwrap_or(80.0), "critical stem speed {ca:?} -> {cb:?}");
    // (2) Decay from the pruning model reaches the mechanics: topping a
    // 60-year oak at half height leaves a decay column in the stem (see
    // tests/pruning.rs, big_wound_decays_small_one_closes). Infection is a
    // random event of the pruning model, and the wound can later leave the
    // tree with its host (seen: decay from year 8, the wound record "gone"
    // at year 12 with the current genome): the check uses the first year
    // (within 20) in which a decay column stands in the stem. It used to
    // check year 20 only.
    let mut topped = Simulation::new(Genome::quercus_robur(), 42);
    topped.advance_days(60 * 365 + 20);
    let h = topped.height_m();
    topped.top_at(0.5 * h);
    let mut found = false;
    for _ in 0..20 {
        topped.advance_years(1);
        if topped.mechanics_inputs().0.iter().any(|s| s.decay_radius_m > 0.0) {
            found = true;
            break;
        }
    }
    assert!(found, "no decayed segment within 20 years of topping");
    let (segs, site) = topped.mechanics_inputs();
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
    let oak = &tree("quercus_robur")[0][0];
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
    // Primary branches of 40 woody species carry their own weight at a
    // bending stress of 11.1 MPa on average (species means 5.2-18.9 MPa;
    // Evans et al. 2008, Trees 22:283-289, abstract), i.e. a safety of
    // ~3-11 for a green MOR of 55-57 MPa: the >= 2 floor is loose. Every
    // species and age, the 150-year oak included (its limbs were too
    // slender, ~1.6, before the adaptive radial growth of growth.rs).
    for (sp, a, sims) in living_trees() {
        {
            let snaps: Vec<_> = sims.iter().map(|s| s.mechanics_model().evaluate(&in_leaf(0.0))).collect();
            let stem = median(snaps.iter().map(|s| s.stem_safety).collect());
            let limb = median(snaps.iter().map(|s| s.limb_safety).collect());
            let buckling = median(snaps.iter().map(|s| s.buckling_safety).collect());
            assert!(stem >= 2.0, "species {sp} age {}: stem safety {stem:.2}", AGES[a]);
            assert!(buckling > 1.0, "species {sp} age {}: buckles under its own weight ({buckling:.2})", AGES[a]);
            assert!(limb >= 2.0, "species {sp} age {}: limb safety {limb:.2}", AGES[a]);
            let mass = median(snaps.iter().map(|s| s.above_ground_mass_kg).collect());
            assert!(mass > 0.0);
        }
    }
}

/// Per tree: the stem path (collar, thickest child), and for every
/// structural first-order limb (non-stem child of a stem segment, living,
/// diameter >= 0.2 DBH) its segment index and the area ratio at its
/// insertion, `(stem above + all living children) / stem below`.
fn scaffold_insertions(sim: &Simulation) -> Vec<(usize, f32)> {
    let (segs, site) = sim.mechanics_inputs();
    let n = segs.len();
    let mut thickest = vec![usize::MAX; n];
    let mut kids = vec![0.0f32; n];
    for (i, s) in segs.iter().enumerate().skip(1) {
        let p = s.parent.unwrap();
        if !s.dead {
            kids[p] += s.radius_m * s.radius_m;
        }
        if thickest[p] == usize::MAX || s.radius_m > segs[thickest[p]].radius_m {
            thickest[p] = i;
        }
    }
    let mut on = vec![false; n];
    let mut c = 0;
    on[0] = true;
    while thickest[c] != usize::MAX {
        c = thickest[c];
        on[c] = true;
    }
    (1..n)
        .filter(|&i| {
            let p = segs[i].parent.unwrap();
            on[p] && !on[i] && !segs[i].dead && 2.0 * segs[i].radius_m >= 0.2 * site.dbh_m
        })
        .map(|i| {
            let p = segs[i].parent.unwrap();
            (i, kids[p] / (segs[p].radius_m * segs[p].radius_m))
        })
        .collect()
}

#[test]
fn scaffold_insertions_add_cross_section() {
    // Leonardo's rule says the daughters' cross-sections sum to the
    // mother's (ratio 1). For a branch loaded by its own weight, the
    // uniform-stress model needs more: 1.04 for a lateral of a tenth of the
    // main axis' weight, 1.24 for equal weights, more at wide angles
    // (Minamino & Tateno 2014, PLoS ONE 9:e93535, Figs 2-3); measured fork
    // ratios of Fagus crenata are "a little larger" than 1, closest to the
    // uniform-stress prediction (same paper, Fig. 6). Here: the median
    // ratio at the scaffold-limb insertions of the 60- and 150-year oak and
    // cherry (all seeds pooled) lies in [0.95, 1.25] (below 1: the area of
    // shed branches is gone from the sum), and for the 150-year oak, whose
    // limbs are loaded most, above 1.02 (pure Pressler allocation gives
    // 0.99-1.00). The band is ours, built on the two model limits.
    for (sp, a, sims) in living_trees() {
        {
            let ratios: Vec<f32> = sims.iter().flat_map(|s| scaffold_insertions(s).into_iter().map(|x| x.1)).collect();
            if ratios.is_empty() {
                continue;
            }
            let m = median(ratios.clone());
            assert!((0.95..=1.25).contains(&m), "species {sp} age {}: median area ratio at limb insertions {m:.3} ({} limbs)", AGES[a], ratios.len());
            if sp == "quercus_robur" && AGES[a] == 150 {
                assert!(m > 1.02, "old oak: median area ratio at limb insertions {m:.3} (pure pipe model)");
            }
        }
    }
}

#[test]
fn old_oak_limbs_bear_crown_like_stress() {
    // Bending stress under self-weight is roughly constant along primary
    // branches and similar across species (~11 MPa; Evans et al. 2008,
    // Trees 22:283-289, abstract). In the 150-year oak, the base of the
    // scaffold limbs must not carry much more stress than the smaller
    // branches of 2-10 cm (median over seeds of the per-tree median ratio
    // <= 1.5; it was ~2.3 with the pure pipe model), and stays inside the
    // range of species means, 5.2-18.9 MPa. The 1.5 factor is ours.
    let mut ratio = Vec::new();
    let mut limb_mpa = Vec::new();
    for sim in &tree("quercus_robur")[1] {
        let snap = sim.mechanics_model().evaluate(&in_leaf(0.0));
        let (segs, _) = sim.mechanics_inputs();
        let limbs: Vec<f32> = scaffold_insertions(sim).into_iter().map(|(i, _)| snap.segment_stress_mpa[i]).collect();
        let small: Vec<f32> = (0..segs.len())
            .filter(|&i| snap.segment_role[i] != dendro_core::SegmentRole::Trunk && !segs[i].dead && segs[i].leaf_weight <= 0.0 && (0.01..0.05).contains(&segs[i].radius_m))
            .map(|i| snap.segment_stress_mpa[i])
            .collect();
        let (l, s) = (median(limbs), median(small));
        ratio.push(l / s);
        limb_mpa.push(l);
    }
    let (r, l) = (median(ratio.clone()), median(limb_mpa.clone()));
    println!("old oak: limb-base / 2-10 cm stress ratios {ratio:?}, limb-base stress {limb_mpa:?} MPa");
    assert!(r <= 1.5, "old oak: limb-base stress / 2-10 cm branch stress = {r:.2} ({ratio:?})");
    assert!((5.2..=18.9).contains(&l), "old oak: median limb-base stress {l:.1} MPa ({limb_mpa:?})");
}

#[test]
fn adult_critical_gusts_in_published_range() {
    // A sound adult broadleaf, leafless (the European winter storms), must
    // not fail below a 10 m gust of 32 m/s: the static tree assessment of
    // Sinn & Wessolly (1989) loads trees with a Beaufort-12 wind of 32 m/s
    // that a sound tree must resist (median of three seeds).
    // Changed with the user (2026-11-18): the former band 30-45 m/s was our
    // own estimate around 32 m/s and the ~42 m/s of Virot et al. 2016 (Phys
    // Rev E 93:023001), which Albrecht et al. 2016 dispute; with the
    // frontal-area drag the 60-year oak and cherry give 47.4 and 45.8 m/s.
    // Virot's value stays a comparison of the validation bench
    // (`virot_critical_wind`, data/validation/physics.ron), not a test bound.
    // Checked: oak at 60 and 150 years, cherry at 60.
    let cases = [("quercus_robur", 0usize), ("quercus_robur", 1), ("prunus_avium", 0)];
    for (sp, a) in cases {
        let first: Vec<f32> = tree(sp)[a].iter().map(|s| s.mechanics_model().critical_speeds(&leafless()).first().map_or(80.0, |f| f.1)).collect();
        let m = median(first.clone());
        assert!(m >= 32.0, "species {sp} age {}: first failure at {m:.1} m/s ({first:?}), below the 32 m/s of Sinn & Wessolly 1989", AGES[a]);
    }
}

#[test]
fn snapshot_is_deterministic_and_reports_modes() {
    let s = &tree("prunus_avium")[0][0];
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
    let s = &tree("quercus_robur")[1][0];
    let t0 = std::time::Instant::now();
    let m = s.mechanics_model();
    let _ = m.evaluate(&WindLoad::new(30.0));
    let one = t0.elapsed().as_secs_f32();
    let t1 = std::time::Instant::now();
    let _ = m.critical_speeds(&leafless());
    let crit = t1.elapsed().as_secs_f32();
    let t2 = std::time::Instant::now();
    let _ = m.critical_speeds(&WindLoad { quasi_static: true, ..leafless() });
    let qs = t2.elapsed().as_secs_f32();
    let t3 = std::time::Instant::now();
    let first = m.first_critical_speed(&leafless(), 0.25);
    let fc = t3.elapsed().as_secs_f32();
    println!("model + 1 evaluation {:.1} ms, critical speeds {:.1} ms (quasi-static {:.1} ms; first only {:.1} ms -> {first:?}) ({} segments)", one * 1e3, crit * 1e3, qs * 1e3, fc * 1e3, m.segment_count());
    assert!(one < 0.02 && crit < 0.2);
}

