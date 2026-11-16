//! Save / load: the recipe (species, seed, site, log, day) round-trips through
//! RON and the replay lands on the same state hash as the live tree.
//!
//! Never compares against a constant hash (the calibration changes between
//! builds): always two runs of this build.

use dendro_core::save::{SaveError, SaveFile, SavedAction, SAVE_VERSION};
use dendro_core::{ClimateParams, Genome, Simulation, SoilProfile};

const YEAR: u32 = 365;

/// A live 18-year oak with the kind of history a player builds: a drought
/// toggled on and off, a crown raising and a thinning, single cuts, and a
/// last cut made on the save day itself (pending light pass).
fn pruned_oak() -> Simulation {
    let mut presets = SoilProfile::presets();
    let soil = presets.swap_remove(1.min(presets.len() - 1));
    let mut s = Simulation::with_site(Genome::quercus_robur(), 2024, ClimateParams::default(), soil);
    // Uneven chunks, like the front-end's per-frame stepping.
    for _ in 0..(8 * YEAR / 37) {
        s.advance_days(37);
    }
    s.advance_days(8 * YEAR - (8 * YEAR / 37) * 37 + 150);
    s.set_drought(true);
    s.advance_days(70);
    s.set_drought(false);
    s.advance_days(200);
    assert!(s.raise_crown(0.8) > 0, "crown raising cut nothing");
    s.advance_days(3 * YEAR + 40);
    s.thin_crown(0.3);
    s.advance_days(YEAR);
    let limb = s.shoot_nodes().into_iter().filter(|n| n.order == 1 && !n.dead).max_by(|a, b| a.radius_m.total_cmp(&b.radius_m));
    if let Some(l) = limb {
        assert!(s.cut(l.id, 0.3, false).is_some());
    }
    s.advance_days(2 * YEAR + 11);
    // Cut today, saved before any further day is simulated.
    let twig = s.shoot_nodes().into_iter().filter(|n| n.order >= 2 && !n.dead).max_by(|a, b| a.radius_m.total_cmp(&b.radius_m));
    if let Some(t) = twig {
        s.cut(t.id, 0.0, true);
    }
    s
}

#[test]
fn save_then_load_by_replay_gives_the_same_state_hash() {
    let live = pruned_oak();
    let log = live.prune_log().to_vec();
    assert!(log.iter().any(|e| matches!(e.action, dendro_core::Action::Drought(true))));
    assert!(log.iter().filter(|e| matches!(e.action, dendro_core::Action::Cut(_))).count() >= 3);

    // (a) Replay equivalence, without any file.
    let file = SaveFile::capture(&live, "Chêne de test", 1_700_000_000);
    let fresh = file.fresh_simulation().expect("fresh");
    let replayed = Simulation::replayed(&fresh, &log, live.days_elapsed());
    assert_eq!(replayed.state_hash(), live.state_hash(), "replaying the log does not rebuild the live tree");

    // (b) Through the RON text and the sliced replay.
    let text = file.to_ron();
    assert!(text.contains("version: 1"), "{text}");
    let back = SaveFile::from_ron(&text).expect("parse");
    assert_eq!(back, file, "RON round-trip");
    let mut r = dendro_core::Replay::new(&back, 4).expect("replay");
    let mut slices = 0;
    let mut last = 0.0;
    while !r.step(500) {
        slices += 1;
        assert!(r.progress() >= last);
        last = r.progress();
    }
    assert!(slices > 10);
    let res = r.finish();
    assert_eq!(res.sim.days_elapsed(), live.days_elapsed());
    assert_eq!(res.sim.state_hash(), live.state_hash(), "loaded tree differs from the saved one");
    assert_eq!(back.saved_hash(), Some(live.state_hash()));
    assert_eq!(res.sim.prune_log(), live.prune_log());
    assert_eq!(res.sim.drought(), live.drought());
    assert_eq!(res.base.days_elapsed(), 0);
    assert!(!res.checkpoints.is_empty() && res.checkpoints.len() <= 4);

    // A checkpoint from the replay restarts the same history.
    let cp = res.checkpoints.last().expect("checkpoint");
    let again = Simulation::replayed(cp, live.prune_log(), live.days_elapsed());
    assert_eq!(again.state_hash(), live.state_hash(), "checkpoint replay differs");

    // And the loaded tree keeps growing exactly like the live one.
    let mut a = live.clone();
    let mut b = res.sim;
    a.advance_days(400);
    b.advance_days(400);
    assert_eq!(a.state_hash(), b.state_hash(), "trees diverge after loading");
}

/// After a rewind the log holds cuts later than the current day: they are saved and re-applied when
/// their day comes again, in the loaded tree as in the live one.
#[test]
fn rewound_tree_with_pending_cuts_round_trips() {
    let live = pruned_oak();
    let log = live.prune_log().to_vec();
    let cut_days: Vec<u64> = log.iter().filter(|e| matches!(e.action, dendro_core::Action::Cut(_))).map(|e| e.day).collect();
    let last = *cut_days.last().expect("cuts");
    let d = *cut_days.iter().find(|&&d| d < last).expect("an earlier cut day");
    let fresh = SaveFile::capture(&live, "x", 0).fresh_simulation().expect("fresh");
    // Rewound onto an earlier cut day (that day's cuts applied, later ones pending).
    let rewound = Simulation::replayed(&fresh, &log, d);
    assert!(rewound.prune_log().iter().any(|e| e.day > d), "nothing pending after the rewind");
    let file = SaveFile::capture(&rewound, "rewound", 0);
    assert!(file.events.iter().any(|e| e.day > file.day));
    let back = SaveFile::from_ron(&file.to_ron()).expect("parse");
    let mut r = dendro_core::Replay::new(&back, 3).expect("replay");
    while !r.step(700) {}
    let mut loaded = r.finish().sim;
    assert_eq!(loaded.prune_log(), rewound.prune_log());
    assert_eq!(loaded.state_hash(), rewound.state_hash(), "rewound save does not reload identically");
    // Both run past the pending cuts and stay identical.
    let mut a = rewound.clone();
    let n = (last - d + 30) as u32;
    a.advance_days(n);
    loaded.advance_days(n);
    assert_eq!(loaded.state_hash(), a.state_hash(), "pending cuts re-applied differently");
}

#[test]
fn state_hash_sees_a_cut_and_ignores_stepping() {
    let mut a = Simulation::new(Genome::prunus_avium(), 5);
    a.advance_days(6 * YEAR);
    let mut b = Simulation::new(Genome::prunus_avium(), 5);
    for _ in 0..6 {
        b.advance_years(1);
    }
    assert_eq!(a.state_hash(), b.state_hash(), "chunking changed the hash");
    let stem = a.shoot_nodes().into_iter().find(|n| n.order == 1 && !n.dead).expect("a branch");
    a.cut(stem.id, 0.0, false);
    assert_ne!(a.state_hash(), b.state_hash(), "a cut is invisible to the hash");
    let mut c = Simulation::new(Genome::prunus_avium(), 6);
    c.advance_days(6 * YEAR);
    assert_ne!(c.state_hash(), b.state_hash(), "seed is invisible to the hash");
}

#[test]
fn unknown_version_fails_cleanly() {
    let s = Simulation::new(Genome::quercus_robur(), 1);
    let text = SaveFile::capture(&s, "x", 0).to_ron().replacen(&format!("version: {SAVE_VERSION}"), "version: 99", 1);
    assert_eq!(SaveFile::from_ron(&text), Err(SaveError::UnsupportedVersion(99)));
    // A future format with fields this build does not know still reports the version.
    assert_eq!(SaveFile::from_ron("(version: 2, tree: (dna: \"ACGT\"))"), Err(SaveError::UnsupportedVersion(2)));
    assert!(SaveError::UnsupportedVersion(99).to_string().contains("v99"));
}

#[test]
fn missing_species_or_soil_fails_cleanly() {
    let mut s = Simulation::new(Genome::quercus_robur(), 1);
    s.advance_days(30);
    let mut file = SaveFile::capture(&s, "x", 0);
    file.species.name = "Ginkgo biloba".into();
    let back = SaveFile::from_ron(&file.to_ron()).expect("parses");
    assert_eq!(back.replay().err(), Some(SaveError::UnknownSpecies("Ginkgo biloba".into())));
    assert!(dendro_core::Replay::new(&back, 0).is_err());
    let mut file = SaveFile::capture(&s, "x", 0);
    file.soil = "Lune".into();
    assert_eq!(file.replay().err(), Some(SaveError::UnknownSoil("Lune".into())));
}

#[test]
fn garbage_and_inconsistent_files_fail_cleanly() {
    for text in ["", "not ron at all", "(version: 1)", "(version: \"one\")", "[1, 2, 3]"] {
        assert!(SaveFile::from_ron(text).is_err(), "{text:?} accepted");
    }
    let s = Simulation::new(Genome::quercus_robur(), 1);
    let mut file = SaveFile::capture(&s, "x", 0);
    file.events.push(dendro_core::save::SavedEvent { day: 10, action: SavedAction::Drought(true) });
    file.events.push(dendro_core::save::SavedEvent { day: 5, action: SavedAction::Drought(false) });
    assert!(matches!(SaveFile::from_ron(&file.to_ron()), Err(SaveError::Invalid(_))));
    let mut file = SaveFile::capture(&s, "x", 0);
    file.day = u64::MAX / 2;
    assert!(matches!(SaveFile::from_ron(&file.to_ron()), Err(SaveError::Invalid(_))));
    let missing = SaveFile::read(std::path::Path::new("/nonexistent/dendro/save.ron"));
    assert!(matches!(missing, Err(SaveError::Io(_))));
}

#[test]
fn write_and_read_a_file() {
    let mut s = Simulation::new(Genome::prunus_avium(), 77);
    s.advance_days(3 * YEAR);
    let dir = std::env::temp_dir().join(format!("dendro-save-test-{}", std::process::id()));
    let path = dir.join("merisier.ron");
    let file = SaveFile::capture(&s, "Merisier « été »", 42);
    file.write(&path).expect("write");
    let back = SaveFile::read(&path).expect("read");
    assert_eq!(back, file);
    assert_eq!(back.builtin_species_index(), Some(1));
    assert_eq!(back.replay().expect("replay").state_hash(), s.state_hash());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Load time of a 200-year pruned oak (report figure):
/// `cargo test -p dendro-core --release --test save -- --ignored --nocapture`.
#[test]
#[ignore]
fn load_time_200_year_pruned_oak() {
    let mut s = Simulation::new(Genome::quercus_robur(), 11);
    for y in 0..200u32 {
        s.advance_days(YEAR);
        if y % 25 == 20 {
            s.raise_crown((y as f32 / 40.0).min(6.0));
            s.thin_crown(0.15);
        }
    }
    let file = SaveFile::capture(&s, "Vieux chêne", 0);
    let t = std::time::Instant::now();
    let back = SaveFile::from_ron(&file.to_ron()).expect("parse").replay().expect("replay");
    let dt = t.elapsed();
    println!("200-year oak, {} cuts: replay {:.2} s, hash match {}", file.meta.cuts, dt.as_secs_f32(), back.state_hash() == s.state_hash());
    assert_eq!(back.state_hash(), s.state_hash());
}
