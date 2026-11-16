//! Checks of the literature-aligned behaviours (see docs/sources.md):
//! budburst model, PET, pipe exponent, age-dependent shade tolerance,
//! waterlogging thresholds, oak root extent, stem diameter against age and
//! limb/stem proportions.

use dendro_core::climate::{extraterrestrial_radiation_mj, hargreaves_pet_mm, Climate};
use dendro_core::{ClimateParams, Genome, Phase, ReproStage, ShapeMetrics, Simulation};
use std::sync::OnceLock;

/// Mean (budburst day, bloom-onset day) over `years` years after a 10-year
/// establishment (0-based days of year).
fn spring_dates(g: Genome, seed: u64, years: u32) -> (f32, f32, Vec<u32>) {
    let mut sim = Simulation::new(g, seed);
    sim.advance_years(10);
    let (mut bb, mut bl) = (Vec::new(), Vec::new());
    for _ in 0..years {
        let (mut b, mut f) = (None, None);
        for _ in 0..365 {
            sim.advance_days(1);
            if b.is_none() && sim.phase() == Phase::BudBurst {
                b = Some(sim.day_of_year());
            }
            if f.is_none() && sim.repro_stage() == ReproStage::Blooming {
                f = Some(sim.day_of_year());
            }
        }
        bb.push(b.expect("no budburst"));
        if let Some(f) = f {
            bl.push(f);
        }
    }
    let mean = |v: &[u32]| v.iter().sum::<u32>() as f32 / v.len().max(1) as f32;
    (mean(&bb), mean(&bl), bb)
}

#[test]
fn budburst_and_bloom_dates_are_realistic() {
    // Oak: mid/late April (Lebourgeois et al. 2010: French oak stands leaf
    // out from mid-April; Eaton et al. 2016: late April - early May across
    // Europe). Day 105 = 16 April, 122 = 3 May (0-based).
    let (oak_bb, _, oak_all) = spring_dates(Genome::quercus_robur(), 42, 25);
    assert!((105.0..122.0).contains(&oak_bb), "oak mean budburst day {oak_bb}");
    // Never on the doy-170 fallback nor at the window edge (one generated
    // spring in 25 is ~4 degC colder than normal until mid-May, so a late
    // year near day 150 is accepted).
    assert!(oak_all.iter().all(|&d| (85..160).contains(&d)), "oak budburst days {oak_all:?}");
    // Wild cherry: bloom early-mid April at Rennes (German network mean
    // 17 April, day 106, in a cooler climate), with or before leaf-out,
    // about a week before the oak.
    let (ch_bb, ch_bloom, _) = spring_dates(Genome::prunus_avium(), 42, 25);
    assert!((90.0..108.0).contains(&ch_bloom), "cherry mean bloom day {ch_bloom}");
    assert!(ch_bloom <= ch_bb + 1.0, "bloom {ch_bloom} after leaf-out {ch_bb}");
    assert!(ch_bb < oak_bb - 4.0, "cherry leaf-out {ch_bb} vs oak {oak_bb}");
}

#[test]
fn warm_winter_delays_budburst() {
    // Cannell & Smith 1983 / Murray et al. 1989: fewer chill days -> more
    // thermal time needed, exponentially.
    for g in Genome::builtin() {
        let p = &g.phenology;
        let warm = p.budburst_requirement_gdd(5.0);
        let normal = p.budburst_requirement_gdd(30.0);
        let cold = p.budburst_requirement_gdd(80.0);
        assert!(warm > normal + 60.0 && normal > cold, "{}: {warm} {normal} {cold}", g.species);
        // Asymptote reached after ample chilling.
        assert!((cold - p.gdd_budburst).abs() < 0.05 * p.chill_forcing_gdd.max(1.0), "{}", g.species);
        assert_eq!((p.chill_start_doy, p.forcing_start_doy), (304, 31), "{}: Nov 1 / Feb 1", g.species);
        assert!(p.chill_temp_max_c <= 5.0, "{}: chill days are days <= 5 degC", g.species);
    }
    // Whole simulation: a uniformly warmer climate (+3 degC) has fewer chill
    // days, which offsets part of the earlier forcing.
    let warm = ClimateParams { mean_temp_c: 15.4, ..ClimateParams::default() };
    let mut a = Simulation::new(Genome::quercus_robur(), 3);
    let mut b = Simulation::with_climate(Genome::quercus_robur(), 3, warm);
    a.advance_years(3);
    b.advance_years(3);
    // Just check it runs and leafs out within the safety window every year.
    for sim in [&mut a, &mut b] {
        let mut seen = false;
        for _ in 0..365 {
            sim.advance_days(1);
            seen |= sim.phase() == Phase::BudBurst && sim.day_of_year() < 170;
        }
        assert!(seen);
    }
}

#[test]
fn pet_matches_station_penman_total() {
    // Rennes-St Jacques 1991-2020: Penman PET 856 mm/yr (Meteo-France).
    let mut c = Climate::new(ClimateParams::default(), 11);
    let years = 20;
    let mut pet = 0.0f64;
    let mut summer = 0.0f64;
    for y in 0..years {
        for d in 0..365 {
            let w = c.day(y, d, false);
            pet += w.pet_mm as f64;
            if (151..243).contains(&d) {
                summer += (w.pet_mm - w.precip_mm) as f64;
            }
            assert!(w.tmin_c < w.temp_c && w.temp_c < w.tmax_c);
        }
    }
    let pet = pet / years as f64;
    let summer = summer / years as f64;
    assert!((760.0..950.0).contains(&pet), "annual PET {pet:.0} mm");
    // Station June-August: PET 389 mm, rain 138 mm -> deficit ~250 mm.
    assert!((180.0..330.0).contains(&summer), "summer climatic deficit {summer:.0} mm");
    // FAO-56 eq. 21 at 48 degN: ~41 MJ/m2/day at the June solstice, ~9 in
    // late December.
    let june = extraterrestrial_radiation_mj(48.07, 171);
    let dec = extraterrestrial_radiation_mj(48.07, 354);
    assert!((40.0..43.0).contains(&june) && (7.5..10.5).contains(&dec), "Ra {june} {dec}");
    assert!(hargreaves_pet_mm(20.0, 11.0, 41.0) > hargreaves_pet_mm(20.0, 6.0, 41.0));
}

#[test]
fn pipe_exponents_in_published_range() {
    // Eloy 2011: Leonardo exponents ~1.8-2.3.
    for g in Genome::builtin() {
        let e = g.wood.pipe_exponent;
        assert!((1.8..=2.3).contains(&e), "{}: pipe exponent {e}", g.species);
    }
}

#[test]
fn juvenile_shade_tolerance() {
    // Wild cherry: shade tolerant when young, light-demanding later; the
    // oak keeps one value.
    let c = Genome::prunus_avium().tolerances;
    assert!(c.shade_at(3.0) > c.shade_at(40.0) + 0.2, "{} vs {}", c.shade_at(3.0), c.shade_at(40.0));
    assert!(c.shade_at(14.0) < c.shade_at(5.0) && c.shade_at(14.0) > c.shade_at(30.0));
    assert_eq!(c.shade_at(40.0), c.shade);
    let o = Genome::quercus_robur().tolerances;
    assert_eq!(o.shade_at(2.0), o.shade_at(150.0));
}

#[test]
fn waterlogging_thresholds() {
    // Prunus roots react to poor aeration earlier (wetter) than the generic
    // 10 % air-filled porosity; Q. robur tolerates hypoxia.
    let (o, c) = (Genome::quercus_robur().roots, Genome::prunus_avium().roots);
    assert!(c.hypoxia_air_porosity > o.hypoxia_air_porosity);
    assert!(c.hypoxia_sensitivity > o.hypoxia_sensitivity);
    assert!(o.waterlogging_tolerance > c.waterlogging_tolerance);
}

#[test]
fn oak_root_extent_at_150_years() {
    // Lucot & Bruckert 1992: 150-year-old Q. robur, roots reaching ~20 m
    // from the stem; most root length in the top 0.6 m.
    let mut sim = Simulation::new(Genome::quercus_robur(), 42);
    sim.advance_years(150);
    let s = sim.stats();
    assert!((15.0..28.0).contains(&s.root_spread_m), "root spread {} m", s.root_spread_m);
    // On the default loam the compact C2 horizon (3 MPa from 2.1 m) stops
    // the sinkers; on loose soil they would go deeper (max_depth_m 4.5).
    assert!((1.5..3.0).contains(&s.root_depth_m), "root depth {} m", s.root_depth_m);
    assert!(s.root_spread_m > 5.0 * s.root_depth_m);
    let (_, total) = sim.root_length_fraction_above(0.6);
    assert!(total > 0.7, "{:.0} % of root length in the top 0.6 m", total * 100.0);
}

/// State at one age: (age, shape metrics, height m, (base radius m,
/// horizontal reach m) of each first-order limb).
type Snapshot = (u32, ShapeMetrics, f32, Vec<(f32, f32)>);

/// One open-grown run on deep loam (seed 42): yearly DBH, ring widths and
/// snapshots at chosen ages.
struct Growth {
    dbh: Vec<f32>,
    rings: Vec<f32>,
    at: Vec<Snapshot>,
}

fn grow_open(g: Genome, years: u32, ages: &[u32]) -> Growth {
    let mut sim = Simulation::new(g, 42);
    let mut dbh = vec![0.0];
    let mut at = Vec::new();
    for age in 1..=years {
        sim.advance_years(1);
        dbh.push(sim.dbh_cm());
        if ages.contains(&age) {
            let limbs = sim.debug_limbs().into_iter().map(|l| (l.1, l.2)).collect();
            at.push((age, sim.shape_metrics(), sim.height_m(), limbs));
        }
    }
    Growth { dbh, rings: sim.rings().iter().map(|r| r.width_mm).collect(), at }
}

fn oak_200() -> &'static Growth {
    static G: OnceLock<Growth> = OnceLock::new();
    G.get_or_init(|| grow_open(Genome::quercus_robur(), 200, &[100, 200]))
}

fn cherry_100() -> &'static Growth {
    static G: OnceLock<Growth> = OnceLock::new();
    G.get_or_init(|| grow_open(Genome::prunus_avium(), 100, &[60, 100]))
}

fn mean(v: &[f32]) -> f32 {
    v.iter().sum::<f32>() / v.len().max(1) as f32
}

/// Basal-area increment (cm2/y) between two ages.
fn bai(g: &Growth, a: usize, b: usize) -> f32 {
    std::f32::consts::FRAC_PI_4 * (g.dbh[b] * g.dbh[b] - g.dbh[a] * g.dbh[a]) / (b - a) as f32
}

#[test]
fn open_grown_oak_dbh_follows_white_1998() {
    // White 1998, "Estimating the age of large and veteran trees in
    // Britain", Forestry Commission Information Note 12, Tables 1a and 2
    // (common/sessile oak): even "core" rings until the crown reaches its
    // optimum size, then a constant basal-area increment (CAI).
    //   average site, parkland  100 y x 3.5 mm, then CAI  76.6 cm2/y
    //   good site, open grown    80 y x 4 mm,   then CAI  79.9 cm2/y
    //   champion potential       70 y x 5 mm,   then CAI 109.2 cm2/y
    // giving DBH (cm) 14/16/20 @20, 28/32/40 @40, 42/48/60 @60,
    // 70/78/95 @100, 99/106/127 @150, 121/128/152 @200. Mitchell's rule
    // (2.5 cm of girth a year, open-grown broadleaves with a full crown) is
    // an upper bound: 64 cm @80, 159 cm @200.
    let g = oak_200();
    for (age, lo, hi) in [(20, 10.0, 22.0), (40, 26.0, 42.0), (60, 40.0, 62.0), (100, 66.0, 96.0), (150, 92.0, 132.0), (200, 112.0, 156.0)] {
        let d = g.dbh[age];
        assert!((lo..=hi).contains(&d), "oak DBH at {age} y = {d:.1} cm, expected {lo}-{hi}");
        assert!(d * std::f32::consts::PI <= 2.5 * age as f32 + 10.0, "oak girth at {age} y beyond Mitchell's rule");
    }
    // Rings: wide and even in the formative phase (3.5-5 mm in White's
    // table; ring widths of open-grown oak are typically 2-5 mm), narrower
    // once the constant CAI is spread over a large girth.
    let young = mean(&g.rings[10..60]);
    let old = mean(&g.rings[150..200]);
    assert!((3.0..6.0).contains(&young), "oak rings 10-60 y: {young:.2} mm");
    assert!((1.2..3.0).contains(&old), "oak rings 150-200 y: {old:.2} mm");
    // CAI plateau instead of a CAI that keeps climbing with crown size.
    let (mid, late) = (bai(g, 80, 100), bai(g, 150, 200));
    assert!((60.0..120.0).contains(&late), "oak CAI 150-200 y: {late:.1} cm2/y");
    assert!(late > 0.7 * mid && late < 1.4 * mid, "oak CAI 80-100 y {mid:.1} -> 150-200 y {late:.1} cm2/y");
}

#[test]
fn open_grown_cherry_dbh_follows_free_growth() {
    // Pryor 1988, "The silviculture and yield of wild cherry", Forestry
    // Commission Bulletin 75, Table 7 (free growth, crowns kept isolated,
    // 110-690 stems/ha): mean DBH 14 cm @15, 23 @30, 30 @40, 44 @60,
    // 50 @70. A lone tree grows at least as fast; upper bound 60 cm in
    // 50-60 y on favourable sites (Evans 1984, as cited by the Future Trees
    // Trust monograph). Old cherries reach 90-120 cm at most (Welk et al.
    // 2016, European Atlas of Forest Tree Species).
    let g = cherry_100();
    for (age, lo, hi) in [(15, 10.0, 20.0), (30, 22.0, 36.0), (40, 28.0, 46.0), (60, 44.0, 64.0), (100, 60.0, 100.0)] {
        let d = g.dbh[age];
        assert!((lo..=hi).contains(&d), "cherry DBH at {age} y = {d:.1} cm, expected {lo}-{hi}");
    }
    // Wide juvenile rings (Pryor: ~4.7 mm a year to 15 y).
    let young = mean(&g.rings[10..30]);
    assert!((4.0..7.5).contains(&young), "cherry rings 10-30 y: {young:.2} mm");
}

#[test]
fn limb_to_stem_ratios_and_elastic_similarity() {
    // Simulated areas obey Leonardo's rule with exponent 2 (Pressler), so
    // the limb/stem ratio follows the share of foliage a limb carries:
    // scaffold limbs of open-grown trees are about a third to a half of the
    // stem diameter where they leave it, none matching it (Gilman 2003,
    // "Branch-to-stem diameter ratio affects strength of attachment",
    // J Arboric 29(5):291-294: attachment strength falls as the ratio
    // rises and co-dominant stems split far more easily, red maple).
    // Calibration (regression guard, no measured band found for these
    // species): mean 0.25-0.6, max < 0.9 (oak) / 0.6 (excurrent cherry).
    // Elastic similarity (McMahon & Kronauer 1976, "Tree structures:
    // deducing the principle of mechanical design", J Theor Biol
    // 59:443-466: length ~ diameter^(2/3)): the limb reaching farthest
    // should have about the diameter the stem would have at that length,
    // DBH x (reach / height)^1.5, within a factor of two.
    for (g, species, max_r) in [(oak_200(), "oak", 0.9), (cherry_100(), "cherry", 0.6)] {
        for (age, m, h, limbs) in &g.at {
            assert!((0.25..0.6).contains(&m.mean_limb_ratio), "{species} {age} y: {m:?}");
            assert!(m.max_limb_ratio < max_r, "{species} {age} y: {m:?}");
            let (r, reach) = limbs.iter().copied().fold((0.0, 0.0), |b, l| if l.1 > b.1 { l } else { b });
            let d = 2.0 * r * 100.0;
            let expect = g.dbh[*age as usize] * (reach / h).powf(1.5);
            assert!(d > 0.5 * expect && d < 2.0 * expect, "{species} {age} y: limb reaching {reach:.1} m is {d:.1} cm, elastic similarity {expect:.1} cm");
        }
    }
}

#[test]
fn ring_view_matches_stem_diameter() {
    // The ring panel draws the recorded ring widths: twice their sum must
    // give the stem diameter (rings of the first years are measured at the
    // collar until the stem passes 1.3 m, hence the 10 % tolerance).
    for (g, species, ages) in [(oak_200(), "oak", [100usize, 200]), (cherry_100(), "cherry", [60, 100])] {
        for age in ages {
            let d_rings = 2.0 * g.rings[..age].iter().sum::<f32>() * 0.1;
            let d = g.dbh[age];
            assert!((d_rings - d).abs() < 0.1 * d, "{species} {age} y: rings give {d_rings:.1} cm, DBH {d:.1} cm");
        }
    }
}
