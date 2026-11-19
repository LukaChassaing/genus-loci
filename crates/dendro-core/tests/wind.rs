//! Wind against the tree (`dendro_core::wind`): the daily wind breaks
//! nothing; storms asked for break through the pruning machinery, uproot
//! through `vitality.rs`; sway of simulated trees.

use dendro_core::{Action, CutKind, DamageKind, DeathCause, Genome, Simulation, WindLoad};

const YEAR: u32 = 365;

fn load(s: &Simulation) -> WindLoad {
    WindLoad { azimuth_deg: s.climate_params().wind_towards_deg, ..WindLoad::new(0.0) }
}

/// The daily wind of the calibrated climate never breaks nor uproots
/// anything (user decision, 2026-11-19): 100 years of oak and of spruce, no
/// entry in the wind history, while gusts >= 28 m/s did occur.
#[test]
fn daily_wind_breaks_nothing() {
    for (id, seed) in [("quercus_robur", 1u64), ("picea_abies", 1)] {
        let mut s = Simulation::new(Genome::by_id(id).unwrap(), seed);
        let mut strong = 0;
        for _ in 0..100 * YEAR {
            s.advance_days(1);
            strong += (s.wind_today().1 >= 28.0) as u32;
        }
        assert!(strong > 10, "{id}: only {strong} days >= 28 m/s in 100 years");
        assert!(s.wind_history().is_empty(), "{id}: {:?}", s.wind_history());
        assert!(s.uprooted().is_none());
    }
}

/// A storm just below the tree's first critical gust breaks nothing; just
/// above, something breaks, through the pruning machinery (a torn wound,
/// `CutKind::WindBreak`, in the cut journal) or the tree is uprooted. On
/// several seeds: 20-year oaks in summer, whose first failure mode is the
/// stem (docs/sources.md). A snapped stem is not a death: ten years later
/// the tree is alive and taller than the break (reiteration from the latent
/// buds below it).
#[test]
fn storm_breaks_above_the_critical_gust_and_not_below_then_reiterates() {
    let mut stem_breaks = 0;
    for seed in [1u64, 2, 3, 4, 5] {
        let mut s = Simulation::new(Genome::quercus_robur(), seed);
        s.advance_days(20 * YEAR + 196); // mid-July of the 21st season, full crown
        let crit = s.mechanics_model().first_critical_speed(&load(&s), 0.05).expect("a critical gust below 80 m/s");
        assert!((15.0..80.0).contains(&crit), "seed {seed}: critical gust {crit}");
        let mut below = s.clone();
        let n = below.shoot_nodes().len();
        assert!(below.storm(crit - 1.0).is_empty(), "seed {seed}: broke below the critical gust {crit}");
        assert_eq!(below.shoot_nodes().len(), n);
        assert!(below.prune_journal().is_empty());
        let dmg = s.storm(crit + 0.5);
        assert!(!dmg.is_empty(), "seed {seed}: nothing broke at {} m/s (critical {crit})", crit + 0.5);
        assert!(dmg.iter().all(|d| d.storm && d.gust_ms == crit + 0.5 && d.diameter_cm > 0.0));
        assert!(s.prune_log().iter().any(|e| matches!(e.action, Action::Storm(_))));
        let first = dmg[0];
        if first.kind == DamageKind::Uprooted {
            continue;
        }
        assert!(s.shoot_nodes().iter().all(|x| x.id != first.node_id), "seed {seed}: the broken part is still there");
        let j = s.prune_journal();
        assert!(j.iter().any(|e| e.kind == CutKind::WindBreak && e.natural && e.diameter_cm > 0.0), "seed {seed}: no torn wound");
        if matches!(first.kind, DamageKind::Stem | DamageKind::Top) {
            stem_breaks += 1;
            s.advance_years(10);
            println!("seed {seed}: {:?} at {:.1} m ({:.1} cm) in {:.1} m/s; 10 years later H {:.1} m, DBH {:.1} cm", first.kind, first.height_m, first.diameter_cm, first.gust_ms, s.height_m(), s.dbh_cm());
            assert!(s.is_alive() && s.trunk_alive(), "seed {seed}: a snapped stem is not a death");
            assert!(s.height_m() > first.height_m + 1.0, "seed {seed}: no reiteration above the break at {} m: H {}", first.height_m, s.height_m());
            assert!(s.dbh_cm() > 0.0, "seed {seed}: DBH lost after the break");
        }
    }
    assert!(stem_breaks >= 3, "only {stem_breaks} of 5 storms snapped the stem of a 20-year oak in summer");
}

/// A stem cut at the collar (coppice, the path of a wind break near the
/// ground): the stool resprouts and its thickest shoot carries the DBH. On
/// main the DBH stayed at 1.4 cm on a 16 m coppice shoot 40 years after a
/// coppice cut of a 60-year oak (seed 42), and the collapse of the collar
/// increment read as a loss of vigour (5.5 % death hazard 5 years after).
#[test]
fn stool_shoots_carry_the_dbh_and_stay_vigorous() {
    let mut s = Simulation::new(Genome::quercus_robur(), 42);
    s.advance_years(60);
    assert!(s.coppice().is_some());
    let mut hazard_max = 0.0f32;
    for _ in 0..30 {
        s.advance_years(1);
        hazard_max = hazard_max.max(s.vitality().death_probability);
    }
    println!("30 years after the cut: H {:.1} m, DBH {:.1} cm, max hazard {hazard_max:.4}", s.height_m(), s.dbh_cm());
    assert!(s.is_alive() && s.trunk_alive());
    // Plausible coppice stem: a 30-year shoot of a vigorous stool, at least
    // the ~1 cm per 3 m of height of a very slender stem, and a hazard at the
    // full-vigour level of `vitality.rs` (~0.02 %/year; 1 % = bench bound).
    assert!(s.dbh_cm() > s.height_m() / 3.0, "DBH {} cm for {} m", s.dbh_cm(), s.height_m());
    assert!(hazard_max < 0.01, "death hazard {hazard_max} after coppicing");
}

/// Uprooting: the only outright death by the wind. Through `vitality.rs`:
/// cause « déracinement », the trunk lies on the ground (not a standing
/// snag), it grows no more; the stump may resprout (species ability).
#[test]
fn uprooting_kills_the_trunk_through_vitality() {
    let mut s = Simulation::new(Genome::quercus_robur(), 7);
    s.advance_days(60 * YEAR + 20); // winter
    let c = s.mechanics_model().critical_speeds(&load(&s));
    let up = c.uprooting.expect("uprooting below 80 m/s");
    let first = c.first().unwrap();
    println!("60-year oak, winter: {c:?}");
    let h0 = s.height_m();
    let mut g = up;
    while s.uprooted().is_none() && g <= 80.0 {
        s.storm(g);
        g += 1.0;
    }
    assert!(s.uprooted().is_some(), "not uprooted up to 80 m/s (first mode {first:?})");
    let v = s.vitality();
    assert!(!s.trunk_alive() && v.cause == DeathCause::Uprooted && v.uprooted && !v.standing_dead, "{v:?}");
    assert!(s.height_m() < 0.7 * h0, "the stem does not lie down: {} -> {} m", h0, s.height_m());
    let last = *s.wind_history().last().unwrap();
    assert_eq!(last.kind, DamageKind::Uprooted);
    // No more rings on the dead trunk; the history ends with the fall.
    let rings = s.rings().len();
    s.advance_years(3);
    assert_eq!(s.rings().len(), rings);
    assert_eq!(s.wind_history().last().unwrap().kind, DamageKind::Uprooted);
    // Stump sprouts as for any trunk death (oak: stump_sprouting > 0.5).
    println!("3 years after: alive {} (resprouted {})", s.is_alive(), s.vitality().resprouted);
    assert_eq!(s.is_alive(), s.vitality().resprouted);
}

/// Sway: natural frequency of simulated trees, leafless above in leaf
/// (leaves add mass), within the range of measured trees (0.1-3 Hz; Moore
/// & Maguire 2004, Trees 18:195-203, abstract). For the adult broadleaves
/// (60 and 150 years) the winter gain stays within twice the +18-19 %
/// measured by Jackson et al. 2019 (band ours). Damping: still-air traits.
#[test]
fn sway_frequency_is_plausible() {
    for id in ["quercus_robur", "prunus_avium", "fagus_sylvatica", "malus_domestica", "picea_abies", "pinus_sylvestris"] {
        let g = Genome::by_id(id).unwrap();
        let deciduous = g.leaf.deciduous;
        let mut s = Simulation::new(g, 42);
        for age in [20u32, 60, 150] {
            s.advance_years(age - s.year());
            if !s.trunk_alive() {
                break;
            }
            let m = s.mechanics_model();
            let leaf = m.dynamics(&WindLoad { leaf_fraction: Some(1.0), ..WindLoad::new(0.0) });
            let bare = m.dynamics(&WindLoad { leaf_fraction: Some(0.0), ..WindLoad::new(0.0) });
            let gain = bare.natural_frequency_hz / leaf.natural_frequency_hz - 1.0;
            println!("{id} {age} y: f {:.3} Hz in leaf, {:.3} Hz leafless (+{:.0} %)", leaf.natural_frequency_hz, bare.natural_frequency_hz, 100.0 * gain);
            assert!((0.1..3.0).contains(&leaf.natural_frequency_hz), "{id} {age}");
            assert!(bare.natural_frequency_hz > leaf.natural_frequency_hz);
            assert!(bare.damping_ratio < leaf.damping_ratio);
            if deciduous && age >= 60 {
                assert!(gain < 0.38, "{id} {age} y: winter gain {gain}");
            }
        }
    }
}
