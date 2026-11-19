//! Validation bench: confronts the engine with `data/validation/*.ron`.
//!
//! cargo run --release -p dendro-validate -- --seeds 5 --out docs/validation/report.md
//!
//! Options: `--seeds N` (default 5), `--out PATH` (markdown; a `.tsv` of
//! the verdicts is written next to it), `--data DIR` (reference files,
//! default `data/validation` of the workspace), `--species id,id` (default:
//! every species of `data/species` that has `data/validation/<id>.ron`),
//! `--years id=N` (simulated years of one species; `--oak-years N`,
//! `--cherry-years N` kept), `--climate-years N` (400 per seed),
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

/// Bench settings of one species: simulated years, reference "adult" age,
/// ages of the July transpiration and critical-gust reads. Bench choices,
/// not data: explicit for the two species the bench was written with,
/// derived from `typical_lifespan_years` otherwise.
fn bench_cfg(g: &Genome) -> (u32, u32, u32, u32) {
    match g.id.as_str() {
        "quercus_robur" => (400, 150, 100, 100),
        "prunus_avium" => (150, 70, 30, 60),
        _ => {
            let life = g.pruning.typical_lifespan_years.max(50.0);
            let years = ((1.4 * life / 50.0).round() as u32 * 50).clamp(100, 400);
            let mature = (((0.6 * life / 10.0).round() as u32) * 10).clamp(30, years);
            (years, mature, mature.min(100), mature.min(100))
        }
    }
}

fn main() {
    let t0 = Instant::now();
    let mut n_seeds = 5u64;
    let mut out = PathBuf::from("docs/validation/report.md");
    let mut data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/validation");
    let mut clim_years = 400u32;
    let mut years_override: Vec<(String, u32)> = Vec::new();
    let mut only: Option<Vec<String>> = None;
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
            "--oak-years" => years_override.push(("quercus_robur".into(), val().parse().expect("years"))),
            "--cherry-years" => years_override.push(("prunus_avium".into(), val().parse().expect("years"))),
            "--years" => {
                let v = val();
                let (id, n) = v.split_once('=').expect("--years id=N");
                years_override.push((id.to_string(), n.parse().expect("years")));
            }
            "--species" => only = Some(val().split(',').map(|s| s.trim().to_string()).collect()),
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

    // Species of the run: every registry species with a genome AND a
    // reference file `data/validation/<id>.ron` (optionally filtered by
    // --species). A reference file with errors stops `--check`; in a normal
    // run it is reported and the species skipped, so one half-written file
    // does not stop the bench for the others.
    let mut species: Vec<(Genome, dendro_validate::data::RefFile)> = Vec::new();
    let mut bad = false;
    for g in Genome::registry() {
        if only.as_ref().is_some_and(|o| !o.contains(&g.id)) {
            continue;
        }
        let path = data_dir.join(format!("{}.ron", g.id));
        if !path.exists() {
            eprintln!("{}: pas de fichier de références, espèce ignorée", g.id);
            continue;
        }
        match data::load(&path) {
            Ok(f) => {
                let errs = data::check(&f);
                if errs.is_empty() {
                    eprintln!("{}: {} entries OK", g.id, f.entries.len());
                    species.push((g.clone(), f));
                } else {
                    bad = true;
                    for e in &errs {
                        eprintln!("{}: {e}", g.id);
                    }
                    eprintln!("{}: fichier de références invalide, espèce ignorée", g.id);
                }
            }
            Err(e) => {
                bad = true;
                eprintln!("{e}\n{}: fichier illisible, espèce ignorée", g.id);
            }
        }
    }
    // physics.ron reads the oak (and the cherry when present).
    let with_physics = species.iter().any(|(g, _)| g.id == "quercus_robur");
    let mut physics = None;
    if with_physics {
        let f = data::load(&data_dir.join("physics.ron")).unwrap_or_else(|e| panic!("{e}"));
        let errs = data::check(&f);
        if !errs.is_empty() {
            for e in &errs {
                eprintln!("physics: {e}");
            }
            std::process::exit(1);
        }
        eprintln!("physics: {} entries OK", f.entries.len());
        physics = Some(f);
    } else {
        eprintln!("physics: ignoré (le chêne ne fait pas partie du passage)");
    }
    if check_only {
        // Also every reference file without a genome yet (written ahead of
        // its species file): structure only.
        let mut extra: Vec<PathBuf> = std::fs::read_dir(&data_dir)
            .map(|d| d.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "ron")).collect())
            .unwrap_or_default();
        extra.sort();
        for p in extra {
            let stem = p.file_stem().unwrap().to_string_lossy().into_owned();
            if stem == "physics" || species.iter().any(|(g, _)| g.id == stem) {
                continue;
            }
            match data::load(&p) {
                Ok(f) => {
                    let errs = data::check(&f);
                    for e in &errs {
                        eprintln!("{stem}: {e}");
                    }
                    if f.subject != stem {
                        eprintln!("{stem}: subject {:?} differs from the file name", f.subject);
                        bad = true;
                    }
                    bad |= !errs.is_empty();
                    eprintln!("{stem}: {} entries{} (pas de génome data/species/{stem}.ron)", f.entries.len(), if errs.is_empty() { " OK" } else { "" });
                }
                Err(e) => {
                    bad = true;
                    eprintln!("{e}");
                }
            }
        }
        if bad {
            std::process::exit(1);
        }
        return;
    }
    if species.is_empty() {
        eprintln!("aucune espèce à évaluer");
        std::process::exit(1);
    }

    let seeds: Vec<u64> = (1..=n_seeds).collect();
    report::SEEDS.store(seeds.len(), std::sync::atomic::Ordering::Relaxed);
    let gley = SoilProfile::gley_clay();
    let cfgs: Vec<(u32, u32, u32, u32)> = species
        .iter()
        .map(|(g, _)| {
            let mut c = bench_cfg(g);
            if let Some((_, y)) = years_override.iter().rev().find(|(id, _)| *id == g.id) {
                c.0 = *y;
                c.1 = c.1.min(*y);
            }
            c
        })
        .collect();
    let det_genome = species.iter().find(|(g, _)| g.id == "prunus_avium").unwrap_or(&species[0]).0.clone();

    // All runs in parallel (independent simulations, deterministic each).
    let (runs, gleys, climate, determinism) = std::thread::scope(|s| {
        let tr: Vec<Vec<_>> = species
            .iter()
            .zip(&cfgs)
            .map(|((g, _), c)| seeds.iter().map(|&sd| s.spawn({ let g = g.clone(); let y = c.0; move || measure::run_tree(&g, sd, y) })).collect())
            .collect();
        let gl: Vec<Vec<_>> = species
            .iter()
            .map(|(g, _)| {
                // Gley runs only where the waterlogging rule reads them.
                let want = g.roots.waterlogging_tolerance < 0.3;
                seeds
                    .iter()
                    .filter(|_| want)
                    .map(|&sd| s.spawn({ let g = g.clone(); let soil = gley.clone(); move || measure::run_tree_on(&g, sd, 30, soil, false) }))
                    .collect()
            })
            .collect();
        let cl: Vec<_> = seeds.iter().map(|&sd| s.spawn(move || measure::climate_stats(sd, clim_years))).collect();
        let det = s.spawn(move || determinism_check(&det_genome));
        (
            tr.into_iter().map(|v| v.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>()).collect::<Vec<_>>(),
            gl.into_iter().map(|v| v.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>()).collect::<Vec<_>>(),
            cl.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>(),
            det.join().unwrap(),
        )
    });
    eprintln!("simulations done in {:.1} s", t0.elapsed().as_secs_f64());

    let mut files = Vec::new();
    let mut ctxs = Vec::new();
    for (((g, f), c), (r, gl)) in species.into_iter().zip(&cfgs).zip(runs.into_iter().zip(gleys)) {
        ctxs.push(SpeciesCtx { id: g.id.clone(), genome: g, runs: r, gley: gl, mature_age: c.1, transp_age: c.2, crit_age: c.3 });
        files.push(f);
    }
    files.extend(physics);
    let cx = Ctx {
        species: ctxs,
        climate,
        #[cfg(feature = "vent")]
        vent: dendro_validate::vent::extra(&seeds),
    };
    let mut rows: Vec<Row> = Vec::new();
    for f in &files {
        for e in &f.entries {
            let m = match cx.get(&f.subject) {
                Some(sp) => eval::species_entry(sp, &cx, e),
                None => eval::physics_entry(&cx, e),
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
    let years = cx
        .species
        .iter()
        .map(|s| format!("{} {} ans (adulte : {} ans)", s.genome.common_name_fr.to_lowercase(), s.last_age(), s.mature_age))
        .chain(cx.species.iter().filter(|s| !s.gley.is_empty()).map(|s| format!("+ 30 ans sur argile à gley ({})", s.genome.common_name_fr.to_lowercase())))
        .chain(std::iter::once(format!("climat {clim_years} ans par graine")))
        .collect::<Vec<_>>()
        .join(", ");
    let meta = Meta {
        title,
        command: cmd,
        seeds: seeds.clone(),
        years,
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
        format!("identique bit à bit ({}, graine 3, {years} ans : hauteur, DBH, {} cernes)", g.common_name_fr.to_lowercase(), rings_plain.len())
    } else {
        format!("**DIFFÉRENT** (hauteur {} contre {}, DBH {} contre {})", p.height_m, plain.height_m(), p.dbh_cm, plain.dbh_cm())
    }
}
