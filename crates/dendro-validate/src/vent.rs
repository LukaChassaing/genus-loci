//! Measurements that need the dynamic-wind API of the `m1-vent-dynamique`
//! branch (daily wind and gusts, natural frequency and damping, wind damage,
//! wind acclimation). Compiled only with `--features vent`, i.e. in a
//! checkout of that branch; main has none of these functions.

use crate::data::Entry;
use crate::eval::{Ctx, SpeciesCtx};
use crate::measure::{self, TreeRun, MONTH_START};
use crate::verdict::{median, Measured, Nature};
use dendro_core::{DamageKind, Genome, Simulation, WindLoad};
use std::fmt::Write;

/// Sway and storm response at a sample age.
#[derive(Clone, Debug, Default)]
pub struct AgeDyn {
    pub f0_leafy: f64,
    pub f0_leafless: f64,
    pub damping_leafy: f64,
    pub damping_leafless: f64,
    /// DBH / H² (m / m²).
    pub d_over_h2: f64,
    /// Damage of a single leafless storm gust on a copy of the tree (35 and
    /// 38 m/s: Lothar and the October 1987 storm at Rennes).
    pub storm35: Vec<&'static str>,
    pub storm38: Vec<&'static str>,
}

#[derive(Clone, Debug, Default)]
pub struct RunDyn {
    pub history: Vec<dendro_core::WindDamage>,
}

/// `--no-wind-damage`: grow sound trees (no breakage, no uprooting; the
/// acclimation still runs), as the growth calibration tests of the branch do.
pub static NO_DAMAGE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn configure(sim: &mut Simulation) {
    // Wind damage is on by default on the branch: the trees can break and
    // be uprooted while they grow, as in the game.
    if NO_DAMAGE.load(std::sync::atomic::Ordering::Relaxed) {
        sim.set_wind_damage_enabled(false);
    }
}

fn kind(k: DamageKind) -> &'static str {
    match k {
        DamageKind::Uprooted => "uproot",
        DamageKind::Stem => "stem",
        _ => "limb",
    }
}

pub fn age_dyn(sim: &Simulation) -> AgeDyn {
    let model = sim.mechanics_model();
    let az = sim.climate_params().wind_towards_deg;
    let d = |lf: f32| model.dynamics(&WindLoad { leaf_fraction: Some(lf), azimuth_deg: az, ..WindLoad::new(0.0) });
    let (a, b) = (d(1.0), d(0.0));
    let h = sim.height_m() as f64;
    let storm = |g: f32| {
        let mut c = sim.clone();
        c.storm(g).into_iter().map(|x| kind(x.kind)).collect::<Vec<_>>()
    };
    AgeDyn {
        f0_leafy: a.natural_frequency_hz as f64,
        f0_leafless: b.natural_frequency_hz as f64,
        damping_leafy: a.damping_ratio as f64,
        damping_leafless: b.damping_ratio as f64,
        d_over_h2: sim.dbh_cm() as f64 / 100.0 / (h * h).max(1e-9),
        storm35: storm(35.0),
        storm38: storm(38.0),
    }
}

pub fn year_end(sim: &Simulation, run: &mut TreeRun, age: u32) {
    if sim.wind_history().len() != run.vent.history.len() {
        run.vent.history = sim.wind_history().to_vec();
    }
    if run.death_age.is_none() && (sim.uprooted().is_some() || !sim.is_alive()) {
        run.death_age = Some(age);
    }
}

#[derive(Clone, Debug, Default)]
pub struct WindAcc {
    mean: f64,
    jan: f64,
    aug: f64,
    g16: f64,
    g28: f64,
    year_max: Vec<f64>,
    month_max: Vec<[f64; 12]>,
}

impl WindAcc {
    pub fn day(&mut self, w: &dendro_core::Weather, m: usize, y: u32) {
        let (u, g) = (w.wind_mean_ms as f64, w.gust_ms as f64);
        self.mean += u;
        if m == 0 {
            self.jan += u;
        }
        if m == 7 {
            self.aug += u;
        }
        self.g16 += (g >= 16.0) as u8 as f64;
        self.g28 += (g >= 28.0) as u8 as f64;
        while self.year_max.len() <= y as usize {
            self.year_max.push(0.0);
            self.month_max.push([0.0; 12]);
        }
        let yi = y as usize;
        self.year_max[yi] = self.year_max[yi].max(g);
        self.month_max[yi][m] = self.month_max[yi][m].max(g);
    }

    pub fn finish(self, years: u32) -> WindStats {
        let yf = years as f64;
        let days = |m: usize| (MONTH_START[m + 1] - MONTH_START[m]) as f64 * yf;
        // Station records 1981-2025: windows of 45 years.
        let win = |v: Vec<f64>| -> f64 { median(&v.chunks(45).filter(|c| c.len() == 45).map(|c| c.iter().copied().fold(0.0, f64::max)).collect::<Vec<_>>()) };
        let month = |m: usize| win(self.month_max.iter().map(|a| a[m]).collect());
        WindStats {
            mean: self.mean / (yf * 365.0),
            jan: self.jan / days(0),
            aug: self.aug / days(7),
            g16: self.g16 / yf,
            g28: self.g28 / yf,
            record_45y: win(self.year_max.clone()),
            record_month_45y: std::array::from_fn(month),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct WindStats {
    pub mean: f64,
    pub jan: f64,
    pub aug: f64,
    pub g16: f64,
    pub g28: f64,
    pub record_45y: f64,
    pub record_month_45y: [f64; 12],
}

/// Extra protocol runs of the branch: the same oaks without wind acclimation.
#[derive(Clone, Debug, Default)]
pub struct Extra {
    /// (height m, DBH cm) at 40 years, with and without acclimation, per seed.
    pub with: Vec<(f64, f64)>,
    pub without: Vec<(f64, f64)>,
}

pub fn extra(seeds: &[u64]) -> Extra {
    let g = Genome::quercus_robur();
    let mut off = g.clone();
    off.wood.wind_acclimation_stress_mpa = 0.0;
    off.wood.wind_acclimation_stem_stress_mpa = 0.0;
    off.wood.wind_elongation_reduction = 0.0;
    let grow = |g: &Genome, s: u64| {
        // Without wind damage, so that breakage does not confound the
        // comparison (the acclimation still runs, see `without_wind_damage`).
        let mut sim = Simulation::new(g.clone(), s).without_wind_damage();
        sim.advance_years(40);
        (sim.height_m() as f64, sim.dbh_cm() as f64)
    };
    std::thread::scope(|sc| {
        let a: Vec<_> = seeds.iter().map(|&s| sc.spawn({ let g = g.clone(); move || grow(&g, s) })).collect();
        let b: Vec<_> = seeds.iter().map(|&s| sc.spawn({ let g = off.clone(); move || grow(&g, s) })).collect();
        Extra { with: a.into_iter().map(|h| h.join().unwrap()).collect(), without: b.into_iter().map(|h| h.join().unwrap()).collect() }
    })
}

fn pearson(p: &[(f64, f64)]) -> f64 {
    let n = p.len() as f64;
    let (mx, my) = (p.iter().map(|a| a.0).sum::<f64>() / n, p.iter().map(|a| a.1).sum::<f64>() / n);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (x, y) in p {
        sxy += (x - mx) * (y - my);
        sxx += (x - mx).powi(2);
        syy += (y - my).powi(2);
    }
    sxy / (sxx * syy).sqrt().max(1e-12)
}

fn f0_points(sp: &SpeciesCtx) -> Vec<(f64, f64)> {
    let mut p = Vec::new();
    for r in &sp.runs {
        for (a, rec) in &r.ages {
            if *a >= 10 && *a <= 150 && r.death_age.is_none_or(|d| *a < d) {
                p.push((rec.vent.d_over_h2, rec.vent.f0_leafless));
            }
        }
    }
    p
}

pub fn physics_entry(cx: &Ctx, e: &Entry) -> Measured {
    let w = |f: fn(&crate::measure::ClimateStats) -> f64, what: &str| Measured::values(cx.climate.iter().map(f).collect(), what.to_string(), Nature::Emergent);
    let oak = &cx.oak;
    let v = |f: fn(&measure::AgeRec) -> f64| oak.at(100, f);
    match e.id.as_str() {
        "rennes_wind_mean_annual" => w(|s| s.wind.mean, "vent moyen journalier à 10 m").calib(),
        "rennes_wind_mean_jan" => w(|s| s.wind.jan, "vent moyen de janvier").calib(),
        "rennes_wind_mean_aug" => w(|s| s.wind.aug, "vent moyen d'août").calib(),
        "rennes_gust_days_16" => w(|s| s.wind.g16, "jours avec rafale ≥ 16 m/s").calib(),
        "rennes_gust_days_28" => w(|s| s.wind.g28, "jours avec rafale ≥ 28 m/s").calib(),
        "rennes_gust_record" => w(|s| s.wind.record_45y, "rafale maximale sur 45 ans (médiane des fenêtres)").calib(),
        "rennes_gust_record_dec" => w(|s| s.wind.record_month_45y[11], "rafale maximale de décembre sur 45 ans"),
        "rennes_gust_record_oct" => w(|s| s.wind.record_month_45y[9], "rafale maximale d'octobre sur 45 ans"),
        "rennes_gust_record_jul" => w(|s| s.wind.record_month_45y[6], "rafale maximale de juillet sur 45 ans"),
        "rennes_gust_lothar_wiki" => w(|s| s.wind.record_month_45y[11], "rafale maximale de décembre sur 45 ans"),
        "conifers_f0_linear_dbh_h2" | "dfir_f0_linear_dbh_h2_both_states" => {
            let p = f0_points(oak);
            let r = pearson(&p);
            let rc = pearson(&f0_points(&cx.cherry));
            Measured::rule(r > 0.8, format!("corrélation f0 (défeuillé) / (DBH/H²), 10-150 ans, toutes graines : chêne r = {r:.2}, merisier r = {rc:.2} (critère de physics.md : r > 0,8, pente > 0)"), "f0 linéaire en DBH/H²", Nature::Emergent)
                .warn("conifères ; aucune pente feuillue publiée")
        }
        "broadleaf_f0_not_cantilever" => Measured::not_measurable("énoncé qualitatif (variance résiduelle due à l'architecture)"),
        "conifers_f0_debranched_higher" | "dfir_damping_debranched" | "dfir_pruning_80pct_threshold" => {
            Measured::not_measurable("protocole d'élagage du houppier (ébranchage, 50 et 80 % de la masse) non écrit dans le banc")
        }
        "leaves_lower_f0_damping" => {
            let a = median(&v(|r| r.vent.f0_leafy));
            let b = median(&v(|r| r.vent.f0_leafless));
            Measured::rule(a < b, format!("chêne 100 ans : f0 feuillé {a:.3} Hz, défeuillé {b:.3} Hz (seule la partie « f0 » de l'énoncé est testée)"), "les feuilles baissent f0", Nature::Emergent)
        }
        "jackson_damping_summer" => Measured::values(v(|r| r.vent.damping_leafy), "amortissement en air calme, houppier feuillé (chêne 100 ans)", Nature::Param).calib(),
        "jackson_damping_winter" => Measured::values(v(|r| r.vent.damping_leafless), "amortissement en air calme, défeuillé (chêne 100 ans)", Nature::Param).calib(),
        "internal_damping_lt_005" => Measured::values(v(|r| r.vent.damping_leafless), "amortissement défeuillé (chêne 100 ans)", Nature::Param).warn("conifères"),
        "dfir_damping_unpruned" => Measured::values(v(|r| r.vent.damping_leafy), "amortissement feuillé (chêne 100 ans)", Nature::Param).warn("douglas, 9 arbres"),
        "jackson_f0_winter_gain" => Measured::values(v(|r| (r.vent.f0_leafless / r.vent.f0_leafy - 1.0) * 100.0), "gain de f0 hiver / été (%), chêne 100 ans", Nature::Emergent)
            .warn("valeur recopiée de docs/sources.md, non relue"),
        "thigmo_jaffe1973" => thigmo(cx, |x| x.0, "hauteur", "la stimulation freine l'élongation").warn("plantes herbacées (via Wikipédia)"),
        "thigmo_beech_guyed" => thigmo(cx, |x| 100.0 * x.0 / x.1, "H/D (test 13 de physics.md : plus faible)", "contrôle mécanique de la croissance radiale"),
        "thigmo_beech_stem_vs_roots" => Measured::not_measurable("le moteur n'acclimate pas l'ancrage : renforcement tige / racines non comparable"),
        "thigmo_hydrangea_dose" => Measured::not_modeled("pas de nombre de flexions par jour"),
        _ => Measured::not_measurable("entrée de vent non reliée"),
    }
}

/// Acclimated vs non-acclimated oaks at 40 years, seed by seed: the rule
/// passes when the acclimated value is lower for every seed.
fn thigmo(cx: &Ctx, f: fn(&(f64, f64)) -> f64, what: &str, rule: &str) -> Measured {
    let a: Vec<f64> = cx.vent.with.iter().map(f).collect();
    let b: Vec<f64> = cx.vent.without.iter().map(f).collect();
    let lower = a.iter().zip(&b).filter(|(x, y)| x < y).count();
    let rel = (median(&a) - median(&b)) / median(&b).abs().max(1e-12);
    let null = if rel.abs() < 0.03 { " ; écart < 3 % : pas d'effet détectable" } else { "" };
    Measured::rule(
        lower == a.len() && !a.is_empty(),
        format!("chêne 40 ans, {what} : avec acclimatation {:.2}, sans {:.2} ({:+.1} %) ; plus faible avec acclimatation pour {lower} graine(s) sur {}{null}", median(&a), median(&b), 100.0 * rel, a.len()),
        rule,
        Nature::Emergent,
    )
}

/// Lothar-like gust on the leafless 100-year-old oak (copy of the tree).
pub fn storm_fraction(sp: &SpeciesCtx) -> Measured {
    let a = if sp.genome.species.starts_with("Quercus") { 100 } else { 60 };
    let v = sp.at(a, |r| if r.vent.storm35.iter().any(|k| *k == "stem" || *k == "uproot") { 1.0 } else { 0.0 });
    let any = sp.at(a, |r| if r.vent.storm35.is_empty() { 0.0 } else { 1.0 });
    let n = v.len();
    let frac = v.iter().sum::<f64>() / n.max(1) as f64;
    Measured::one(frac, format!("arbres isolés de {a} ans cassés au tronc ou déracinés par une rafale de 35 m/s défeuillée ({} graines ; tout dégât compris : {:.0} %)", n, 100.0 * any.iter().sum::<f64>() / n.max(1) as f64), Nature::Emergent)
        .nc("arbres de peuplement (inventaire) contre arbre isolé")
}

type StatFn = fn(&crate::measure::ClimateStats) -> f64;

/// Report section: wind climate, sway, storms and survival.
pub fn section(cx: &Ctx) -> String {
    let mut o = String::from("## Branche vent : climat de vent, oscillation, tenue mécanique\n\n");
    let c = &cx.climate;
    let mm = |v: Vec<f64>| {
        let lo = v.iter().copied().fold(f64::MAX, f64::min);
        let hi = v.iter().copied().fold(f64::MIN, f64::max);
        format!("{:.2} [{:.2}–{:.2}]", median(&v), lo, hi)
    };
    let _ = writeln!(o, "### Climat de vent ({} séries de {} ans)\n", c.len(), c.first().map(|s| s.years).unwrap_or(0));
    let _ = writeln!(o, "| statistique | moteur | Rennes-St Jacques |\n|---|---:|---:|");
    let rows: [(&str, StatFn, &str); 9] = [
        ("vent moyen (m/s)", |s| s.wind.mean, "3,6"),
        ("vent moyen janvier", |s| s.wind.jan, "3,9"),
        ("vent moyen août", |s| s.wind.aug, "3,2"),
        ("jours rafale ≥ 16 m/s", |s| s.wind.g16, "41,7"),
        ("jours rafale ≥ 28 m/s", |s| s.wind.g28, "0,6"),
        ("rafale max. sur 45 ans", |s| s.wind.record_45y, "38 (1987)"),
        ("rafale max. d'octobre, 45 ans", |s| s.wind.record_month_45y[9], "38"),
        ("rafale max. de décembre, 45 ans", |s| s.wind.record_month_45y[11], "35"),
        ("rafale max. de juillet, 45 ans", |s| s.wind.record_month_45y[6], "23"),
    ];
    for (n, f, r) in rows {
        let _ = writeln!(o, "| {n} | {} | {r} |", mm(c.iter().map(f).collect()));
    }
    let _ = writeln!(o, "\nMaxima mensuels de rafale sur 45 ans (médiane des fenêtres) : {}.\n", (0..12).map(|m| format!("{:.1}", median(&c.iter().map(|s| s.wind.record_month_45y[m]).collect::<Vec<_>>()))).collect::<Vec<_>>().join(" ; "));
    for (name, sp) in [("Chêne", &cx.oak), ("Merisier", &cx.cherry)] {
        let dmg = if NO_DAMAGE.load(std::sync::atomic::Ordering::Relaxed) { "dégâts au vent désactivés (--no-wind-damage)" } else { "dégâts au vent activés, comme dans le jeu" };
        let _ = writeln!(o, "### {name} : tenue mécanique pendant la croissance ({dmg})\n");
        let _ = writeln!(o, "| graine | debout en fin de simulation | déraciné à (ans) | casses de branches | casses du tronc | plus gros diamètre cassé (cm) | rafale la plus forte ayant cassé (m/s) |\n|---:|---|---:|---:|---:|---:|---:|");
        for r in &sp.runs {
            let h = &r.vent.history;
            let limb = h.iter().filter(|d| kind(d.kind) == "limb").count();
            let stem = h.iter().filter(|d| kind(d.kind) == "stem").count();
            let dmax = h.iter().map(|d| d.diameter_cm as f64).fold(0.0, f64::max);
            let gmax = h.iter().map(|d| d.gust_ms as f64).fold(0.0, f64::max);
            let _ = writeln!(
                o,
                "| {} | {} | {} | {} | {} | {:.1} | {:.1} |",
                r.seed,
                if r.death_age.is_none() { "oui" } else { "non" },
                r.death_age.map(|a| a.to_string()).unwrap_or("—".into()),
                limb,
                stem,
                dmax,
                gmax
            );
        }
        let _ = writeln!(o, "\nAprès la mort d'un arbre, ses années ne sont plus utilisées par les verdicts (médianes sur les graines encore debout).\n");
        let _ = writeln!(o, "| âge | f0 feuillé Hz | f0 défeuillé Hz | gain hiver % | amortissement feuillé | défeuillé | DBH/H² | rafale 35 m/s (défeuillé) | 38 m/s |\n|---:|---:|---:|---:|---:|---:|---:|---|---|");
        for &a in measure::sample_ages(&sp.genome.species).iter().filter(|&&a| a >= 10) {
            let f = |g: fn(&measure::AgeRec) -> f64| {
                let v = sp.at(a, g);
                if v.is_empty() {
                    "—".to_string()
                } else {
                    let m = median(&v);
                    if m.abs() < 0.01 {
                        format!("{m:.2e}")
                    } else {
                        format!("{m:.3}")
                    }
                }
            };
            let st = |g: fn(&AgeDyn) -> &Vec<&'static str>| {
                sp.runs
                    .iter()
                    .filter(|r| r.death_age.is_none_or(|d| a < d))
                    .filter_map(|r| r.ages.get(&a))
                    .map(|x| {
                        let k = g(&x.vent);
                        if k.is_empty() {
                            "—".to_string()
                        } else {
                            k.join("+")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let _ = writeln!(
                o,
                "| {a} | {} | {} | {} | {} | {} | {} | {} | {} |",
                f(|r| r.vent.f0_leafy),
                f(|r| r.vent.f0_leafless),
                f(|r| (r.vent.f0_leafless / r.vent.f0_leafy - 1.0) * 100.0),
                f(|r| r.vent.damping_leafy),
                f(|r| r.vent.damping_leafless),
                f(|r| r.vent.d_over_h2),
                st(|d| &d.storm35),
                st(|d| &d.storm38)
            );
        }
        o.push('\n');
    }
    let hd = |v: &[(f64, f64)]| median(&v.iter().map(|x| 100.0 * x.0 / x.1).collect::<Vec<_>>());
    let _ = writeln!(
        o,
        "### Acclimatation au vent (chêne, 40 ans)\n\nAvec : H {:.2} m, DBH {:.1} cm, H/D {:.1}. Sans (génome copié en mémoire, `wind_acclimation_*` et `wind_elongation_reduction` à 0) : H {:.2} m, DBH {:.1} cm, H/D {:.1}. Les deux séries sont cultivées sans dégâts au vent (`without_wind_damage`) pour que la casse ne brouille pas la comparaison.\n",
        median(&cx.vent.with.iter().map(|x| x.0).collect::<Vec<_>>()),
        median(&cx.vent.with.iter().map(|x| x.1).collect::<Vec<_>>()),
        hd(&cx.vent.with),
        median(&cx.vent.without.iter().map(|x| x.0).collect::<Vec<_>>()),
        median(&cx.vent.without.iter().map(|x| x.1).collect::<Vec<_>>()),
        hd(&cx.vent.without)
    );
    o
}
