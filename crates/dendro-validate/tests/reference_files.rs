//! Fast checks of the reference files `data/validation/*.ron` (syntax,
//! unique ids, min <= value <= max, known units and enums) and of the
//! bench's promise not to change the simulated tree. No verdict is tested
//! here: the bench is a measuring instrument (docs/validation/report.md).

use dendro_validate::{data, measure};
use std::path::PathBuf;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/validation")
}

#[test]
fn reference_files_load_and_are_consistent() {
    // Every `data/validation/*.ron` (one per species, plus physics.ron):
    // the subject is the file stem, the structure passes `data::check`.
    let mut names: Vec<String> = std::fs::read_dir(dir())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "ron"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert!(names.iter().any(|n| n == "physics") && names.iter().any(|n| n == "quercus_robur"), "{names:?}");
    let mut total = 0;
    let mut problems = Vec::new();
    for name in &names {
        let f = match data::load(&dir().join(format!("{name}.ron"))) {
            Ok(f) => f,
            Err(e) => {
                problems.push(e);
                continue;
            }
        };
        if f.subject != *name {
            problems.push(format!("{name}: subject {:?} differs from the file name", f.subject));
        }
        if f.entries.is_empty() {
            problems.push(format!("{name}: no entry"));
        }
        problems.extend(data::check(&f).into_iter().map(|e| format!("{name}: {e}")));
        total += f.entries.len();
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert!(total >= 393, "{total} entries");
}

#[test]
fn check_catches_inconsistent_entries() {
    let text = std::fs::read_to_string(dir().join("physics.ron")).unwrap();
    let mut f: data::RefFile = ron::from_str(&text).unwrap();
    let mut bad = f.entries[0].clone();
    bad.min = Some(100.0);
    bad.unit = "degK".into();
    f.entries.push(bad);
    let errs = data::check(&f);
    assert!(errs.iter().any(|e| e.contains("duplicate id")), "{errs:?}");
    assert!(errs.iter().any(|e| e.contains("unknown unit")), "{errs:?}");
    assert!(errs.iter().any(|e| e.contains("< min")), "{errs:?}");
}

/// The bench steps day by day and reads the state (phases, flux, mechanics):
/// the tree must be the same as one advanced in whole years.
#[test]
fn measuring_does_not_change_the_tree() {
    let g = dendro_core::Genome::prunus_avium();
    let years = 22;
    let probed = measure::run_tree(&g, 4, years);
    let mut plain = dendro_core::Simulation::new(g, 4);
    plain.advance_years(years);
    let last = probed.years.last().unwrap();
    assert_eq!(last.height_m, plain.height_m() as f64);
    assert_eq!(last.dbh_cm, plain.dbh_cm() as f64);
    let a: Vec<f64> = probed.years.iter().filter_map(|y| y.ring_mm).collect();
    let b: Vec<f64> = plain.rings().iter().map(|r| r.width_mm as f64).collect();
    assert_eq!(a, b);
}
