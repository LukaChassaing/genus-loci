//! Checks of the literature-aligned behaviours (see docs/sources.md):
//! budburst model, PET, pipe exponent, age-dependent shade tolerance,
//! waterlogging thresholds and oak root extent.

use dendro_core::climate::{extraterrestrial_radiation_mj, hargreaves_pet_mm, Climate};
use dendro_core::{ClimateParams, Genome, Phase, ReproStage, Simulation};

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
