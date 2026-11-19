//! Context sections of the report: climate series and simulated trajectories.

use crate::eval::{Ctx, SpeciesCtx};
use crate::verdict::median;
use std::fmt::Write;

fn mm(v: &[f64]) -> String {
    let f: Vec<f64> = v.iter().copied().filter(|x| x.is_finite()).collect();
    if f.is_empty() {
        return "—".into();
    }
    let lo = f.iter().copied().fold(f64::MAX, f64::min);
    let hi = f.iter().copied().fold(f64::MIN, f64::max);
    let p = |x: f64| if x.abs() >= 100.0 { format!("{x:.0}") } else { format!("{x:.1}") };
    format!("{} [{}–{}]", p(median(&f)), p(lo), p(hi))
}

pub fn climate_section(cx: &Ctx) -> String {
    let c = &cx.climate;
    let mut o = String::from("## Climat simulé, mois par mois\n\n");
    let _ = writeln!(o, "Médiane [min–max] des {} séries de {} ans. Normales de Rennes-St Jacques 1991-2020 (fiche Météo-France lue, notes des entrées `rennes_*`) pour comparaison.\n", c.len(), c.first().map(|s| s.years).unwrap_or(0));
    let t_norm = [6.2, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, 19.2, 19.3, f64::NAN, f64::NAN, f64::NAN, f64::NAN];
    let r_norm = [66.6, 51.6, 48.9, 51.2, 58.1, 50.9, 44.0, 43.5, f64::NAN, 73.1, 73.2, 73.3];
    let names = ["janv.", "févr.", "mars", "avril", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."];
    let _ = writeln!(o, "| mois | T moyenne °C | normale | Tx-Tn °C | pluie mm | normale |\n|---|---:|---:|---:|---:|---:|");
    for m in 0..12 {
        let t: Vec<f64> = c.iter().map(|s| s.t_month[m]).collect();
        let d: Vec<f64> = c.iter().map(|s| s.dtr_month[m]).collect();
        let r: Vec<f64> = c.iter().map(|s| s.rain_month[m]).collect();
        let f = |x: f64| if x.is_finite() { format!("{x:.1}") } else { "—".into() };
        let _ = writeln!(o, "| {} | {} | {} | {} | {} | {} |", names[m], mm(&t), f(t_norm[m]), mm(&d), mm(&r), f(r_norm[m]));
    }
    o.push_str("\n(Normales mensuelles de pluie relevées dans les notes des entrées ; septembre non lu dans la note tronquée.)\n");
    o
}

fn species_block(o: &mut String, name: &str, sp: &SpeciesCtx) {
    let _ = writeln!(o, "### {name}\n");
    let n = sp.last_age();
    // Trunks dead before the end of the run (dendro_core::vitality): living
    // years only, as in the verdicts.
    let (dead, censored, _) = sp.death_ages();
    let max_dbh: Vec<f64> = sp.runs.iter().map(|r| crate::eval::living(r).iter().map(|y| y.dbh_cm).fold(0.0, f64::max)).collect();
    let mut ages: Vec<u32> = dead.clone();
    ages.sort_unstable();
    let _ = writeln!(
        o,
        "Mort du tronc (émergente) : {}/{} graines mortes en {n} ans, âges {:?}{} ; DBH maximal sur la vie {} cm.\n",
        dead.len(),
        dead.len() + censored,
        ages,
        if censored > 0 { format!(", {censored} vivante(s) à {n} ans") } else { String::new() },
        mm(&max_dbh)
    );
    let _ = writeln!(
        o,
        "| âge | H m | DBH cm | houppier m | cerne mm (5 ans) | prof. racines m | 1re rafale critique défeuillé m/s | feuillé m/s | transpiration juillet L/j | biomasse ligneuse sèche t |\n|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    );
    for &a in crate::measure::sample_ages(n).iter() {
        let living: Vec<&crate::measure::YearRec> = sp.runs.iter().filter_map(|r| crate::eval::living(r).get(a as usize - 1)).collect();
        if living.is_empty() {
            continue;
        }
        let col = |f: &dyn Fn(&crate::measure::YearRec) -> f64| -> String { mm(&living.iter().map(|y| f(y)).collect::<Vec<_>>()) };
        let ring: Vec<f64> = sp
            .runs
            .iter()
            .map(|r| {
                let v: Vec<f64> = r.years.iter().filter(|y| y.age + 5 > a && y.age <= a).filter_map(|y| y.ring_mm).collect();
                v.iter().sum::<f64>() / v.len().max(1) as f64
            })
            .collect();
        let at = |f: &dyn Fn(&crate::measure::AgeRec) -> f64| -> String { mm(&sp.runs.iter().filter_map(|r| r.ages.get(&a).map(f)).collect::<Vec<_>>()) };
        let _ = writeln!(
            o,
            "| {a} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            col(&|y| y.height_m),
            col(&|y| y.dbh_cm),
            col(&|y| 2.0 * y.crown_radius_m),
            mm(&ring),
            col(&|y| y.root_depth_m),
            at(&|r| r.first_crit_leafless.unwrap_or(f64::NAN)),
            at(&|r| r.first_crit_leafy.unwrap_or(f64::NAN)),
            at(&|r| r.july_transp_l_day),
            at(&|r| r.agb_dry_t),
        );
    }
    let date = |f: &dyn Fn(&crate::measure::YearRec) -> Option<u32>| -> String {
        mm(&sp
            .runs
            .iter()
            .map(|r| {
                let v: Vec<f64> = r.years.iter().filter(|y| y.age >= 5).filter_map(f).map(|d| d as f64 + 1.0).collect();
                v.iter().sum::<f64>() / v.len().max(1) as f64
            })
            .collect::<Vec<_>>())
    };
    let sd = |f: &dyn Fn(&crate::measure::YearRec) -> Option<u32>| -> String {
        mm(&sp
            .runs
            .iter()
            .map(|r| {
                let v: Vec<f64> = r.years.iter().filter(|y| y.age >= 5).filter_map(f).map(|d| d as f64).collect();
                let m = v.iter().sum::<f64>() / v.len().max(1) as f64;
                (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len().max(1) as f64).sqrt()
            })
            .collect::<Vec<_>>())
    };
    let _ = writeln!(
        o,
        "\nPhénologie (jour de l'année moyen [min–max des graines], écart-type interannuel) : débourrement {} (σ {}) ; début de coloration {} ; fin de coloration {} ; arbre nu {} ; pleine floraison {} (σ {}) ; maturité des fruits {} ; première floraison à {} ans.\n",
        date(&|y| y.budburst),
        sd(&|y| y.budburst),
        date(&|y| y.colouring),
        date(&|y| y.fall_start),
        date(&|y| y.bare),
        date(&|y| y.bloom_full),
        sd(&|y| y.bloom_full),
        date(&|y| y.ripe),
        mm(&sp.runs.iter().map(|r| r.first_flowering_age.map(|a| a as f64).unwrap_or(f64::NAN)).collect::<Vec<_>>()),
    );
}

pub fn trees_section(cx: &Ctx) -> String {
    let mut o = String::from("## Trajectoires simulées (arbre isolé, site de référence)\n\nMédiane [min–max] des graines.\n\n");
    for sp in &cx.species {
        species_block(&mut o, &sp.genome.common_name_fr, sp);
    }
    o
}
