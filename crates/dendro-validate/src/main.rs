//! Validation bench: confronts the engine with `data/validation/*.ron`.
//!
//! cargo run --release -p dendro-validate -- --seeds 5 --out docs/validation/report.md
//!
//! Options: `--seeds N` (default 5), `--out PATH` (markdown; a `.tsv` of
//! the verdicts is written next to it), `--data DIR` (reference files,
//! default `data/validation` of the workspace), `--oak-years N` (400),
//! `--cherry-years N` (150), `--climate-years N` (400 per seed),
//! `--compare-with TSV` (append a verdict diff against an earlier run),
//! `--title TEXT`, `--check` (only load and check the reference files),
//! `--no-wind-damage` (feature `vent` only: sound trees, no breakage).

use dendro_core::{Genome, Simulation, SoilProfile};
use dendro_validate::data;
use dendro_validate::eval::{self, Ctx, SpeciesCtx};
use dendro_validate::measure;
use dendro_validate::report::{self, Meta, Row};
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let t0 = Instant::now();
    let mut n_seeds = 5u64;
    let mut out = PathBuf::from("docs/validation/report.md");
    let mut data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/validation");
    let (mut oak_years, mut cherry_years, mut clim_years) = (400u32, 150u32, 400u32);
    let mut compare: Option<(PathBuf, String)> = None;
    let mut title = String::from("Banc de validation du moteur — branche main");
    let mut check_only = false;
    let args: Vec<String> = std::env::args().collect();
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        let mut val = || it.next().unwrap_or_else(|| panic!("{a} needs a value")).clone();
        match a.as_str() {
            "--seeds" => n_seeds = val().parse().expect("--seeds N"),
            "--out" => out = PathBuf::from(val()),
            "--data" => data_dir = PathBuf::from(val()),
            "--oak-years" => oak_years = val().parse().expect("years"),
            "--cherry-years" => cherry_years = val().parse().expect("years"),
            "--climate-years" => clim_years = val().parse().expect("years"),
            "--compare-with" => {
                let p = PathBuf::from(val());
                let name = val();
                compare = Some((p, name));
            }
            "--title" => title = val(),
            "--check" => check_only = true,
            #[cfg(feature = "vent")]
            "--no-wind-damage" => dendro_validate::vent::NO_DAMAGE.store(true, std::sync::atomic::Ordering::Relaxed),
            _ => panic!("unknown argument {a}"),
        }
    }

    // Reference files.
    let mut files = Vec::new();
    for name in ["quercus_robur", "prunus_avium", "physics"] {
        let f = data::load(&data_dir.join(format!("{name}.ron"))).unwrap_or_else(|e| panic!("{e}"));
        let errs = data::check(&f);
        if !errs.is_empty() {
            for e in &errs {
                eprintln!("{name}: {e}");
            }
            std::process::exit(1);
        }
        eprintln!("{name}: {} entries OK", f.entries.len());
        files.push(f);
    }
    if check_only {
        return;
    }

    let seeds: Vec<u64> = (1..=n_seeds).collect();
    report::SEEDS.store(seeds.len(), std::sync::atomic::Ordering::Relaxed);
    let oak_g = Genome::quercus_robur();
    let cherry_g = Genome::prunus_avium();
    let gley = SoilProfile::gley_clay();

    // All runs in parallel (independent simulations, deterministic each).
    let (oak_runs, cherry_runs, gley_runs, climate, determinism) = std::thread::scope(|s| {
        let oak: Vec<_> = seeds.iter().map(|&sd| s.spawn({ let g = oak_g.clone(); move || measure::run_tree(&g, sd, oak_years) })).collect();
        let ch: Vec<_> = seeds.iter().map(|&sd| s.spawn({ let g = cherry_g.clone(); move || measure::run_tree(&g, sd, cherry_years) })).collect();
        let gl: Vec<_> = seeds
            .iter()
            .map(|&sd| s.spawn({ let g = cherry_g.clone(); let soil = gley.clone(); move || measure::run_tree_on(&g, sd, 30, soil, false) }))
            .collect();
        let cl: Vec<_> = seeds.iter().map(|&sd| s.spawn(move || measure::climate_stats(sd, clim_years))).collect();
        let det = s.spawn({
            let g = cherry_g.clone();
            move || determinism_check(&g)
        });
        (
            oak.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>(),
            ch.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>(),
            gl.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>(),
            cl.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>(),
            det.join().unwrap(),
        )
    });
    eprintln!("simulations done in {:.1} s", t0.elapsed().as_secs_f64());

    let cx = Ctx {
        oak: SpeciesCtx { genome: oak_g, runs: oak_runs, gley: Vec::new(), mature_age: 150 },
        cherry: SpeciesCtx { genome: cherry_g, runs: cherry_runs, gley: gley_runs, mature_age: 70 },
        climate,
        #[cfg(feature = "vent")]
        vent: dendro_validate::vent::extra(&seeds),
    };
    let mut rows: Vec<Row> = Vec::new();
    for f in &files {
        for e in &f.entries {
            let m = match f.subject.as_str() {
                "quercus_robur" => eval::species_entry(&cx.oak, &cx.cherry, e),
                "prunus_avium" => eval::species_entry(&cx.cherry, &cx.oak, e),
                _ => eval::physics_entry(&cx, e),
            };
            rows.push(report::row(&f.subject, e, m));
        }
    }

    #[allow(unused_mut)]
    let mut extra = vec![dendro_validate::summary::climate_section(&cx), dendro_validate::summary::trees_section(&cx)];
    #[cfg(feature = "vent")]
    extra.push(dendro_validate::vent::section(&cx));
    let tsv = report::tsv(&rows);
    if let Some((p, name)) = &compare {
        let before = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        extra.push(report::diff(&before, &tsv, name, "cette branche"));
    }
    let cmd = std::iter::once("cargo run --release -p dendro-validate --".to_string())
        .chain(args.iter().skip(1).cloned())
        .collect::<Vec<_>>()
        .join(" ");
    let meta = Meta {
        title,
        command: cmd,
        seeds: seeds.clone(),
        years: format!("chêne {oak_years} ans, merisier {cherry_years} ans (+ 30 ans sur argile à gley), climat {clim_years} ans par graine"),
        duration_s: t0.elapsed().as_secs_f64(),
        determinism,
        extra_sections: extra,
        #[cfg(not(feature = "vent"))]
        notes: dendro_validate::notes::MAIN.to_string(),
        #[cfg(feature = "vent")]
        notes: dendro_validate::notes::VENT.to_string(),
    };
    let md = report::markdown(&rows, &meta);
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    std::fs::write(&out, md).expect("write report");
    let tsv_path = out.with_extension("tsv");
    std::fs::write(&tsv_path, tsv).expect("write tsv");
    eprintln!("wrote {} and {} in {:.1} s", out.display(), tsv_path.display(), t0.elapsed().as_secs_f64());
}

/// The bench steps day by day and reads the state: that must not change the
/// tree. Same seed, plain `advance_years` against the probed run.
fn determinism_check(g: &Genome) -> String {
    let years = 40;
    let probed = measure::run_tree(g, 3, years);
    let mut plain = Simulation::new(g.clone(), 3);
    plain.advance_years(years);
    let p = probed.years.last().unwrap();
    let rings_plain: Vec<f32> = plain.rings().iter().map(|r| r.width_mm).collect();
    let rings_probed: Vec<f64> = probed.years.iter().filter_map(|y| y.ring_mm).collect();
    let same = p.height_m == plain.height_m() as f64
        && p.dbh_cm == plain.dbh_cm() as f64
        && rings_plain.len() == rings_probed.len()
        && rings_plain.iter().zip(&rings_probed).all(|(a, b)| *a as f64 == *b);
    if same {
        format!("identique bit à bit (merisier, graine 3, {years} ans : hauteur, DBH, {} cernes)", rings_plain.len())
    } else {
        format!("**DIFFÉRENT** (hauteur {} contre {}, DBH {} contre {})", p.height_m, plain.height_m(), p.dbh_cm, plain.dbh_cm())
    }
}
