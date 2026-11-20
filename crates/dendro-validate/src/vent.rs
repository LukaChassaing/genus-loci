//! Wind measurements (milestone M1, second step): daily wind and gusts,
//! natural frequency and damping, storms asked for on a copy of the tree,
//! stem acclimation to the wind. The daily wind never breaks anything in
//! the engine (only storms asked for do, `dendro_core::wind`).

use crate::data::Entry;
use crate::eval::{Ctx, SpeciesCtx};
use crate::measure::{self, TreeRun, MONTH_START};
use crate::verdict::{median, Measured, Nature};
use dendro_core::{DamageKind, Genome, Simulation, WindLoad};
use std::fmt::Write;

/// Ages at which storms are tried on a copy of the tree (cost).
const STORM_AGES: [u32; 4] = [20, 60, 100, 150];

/// Sway and storm response at a sample age.
#[derive(Clone, Debug, Default)]
pub struct AgeDyn {
    pub f0_leafy: f64,
    pub f0_leafless: f64,
    pub damping_leafy: f64,
    pub damping_leafless: f64,
    /// DBH / H² (m / m²).
    pub d_over_h2: f64,
    /// Damage of a single storm gust asked for on a copy of the tree, in
    /// winter (leafless; the needles of an evergreen) and in summer (full
    /// crown), 35 and 38 m/s (Lothar and the October 1987 storm at Rennes).
    /// Empty when not tried at this age.
    pub storm35: Vec<&'static str>,
    pub storm38: Vec<&'static str>,
    pub storm35_leafy: Vec<&'static str>,
    pub tried: bool,
}

#[derive(Clone, Debug, Default)]
pub struct RunDyn {
    pub history: Vec<dendro_core::WindDamage>,
}

/// Nothing to configure: the daily wind does not damage trees.
pub fn configure(_sim: &mut Simulation) {}

fn kind(k: DamageKind) -> &'static str {
    match k {
        DamageKind::Uprooted => "uproot",
        DamageKind::Stem => "stem",
        DamageKind::Top => "top",
        DamageKind::Limb => "limb",
    }
}

/// The damage of a storm of `gust` m/s on a copy of `sim`, with the given
/// leaf fraction (the storm uses today's foliage: the copy is moved to a
/// winter or a summer day of the same year without growing).
fn storm_on_copy(sim: &Simulation, gust: f32, leafy: bool) -> Vec<&'static str> {
    let mut c = sim.clone();
    // Storms test the crown as it is today; the sample ages are taken at the
    // end of the season (leafless for a deciduous tree). For the summer
    // case, a copy is advanced to mid-July of the next year.
    if leafy {
        let doy = c.day_of_year() as u64;
        let to = if doy < 196 { 196 - doy } else { 365 - doy + 196 };
        c.advance_days(to as u32);
    }
    c.storm(gust).into_iter().map(|x| kind(x.kind)).collect()
}

pub fn age_dyn(sim: &Simulation) -> AgeDyn {
    let model = sim.mechanics_model();
    let az = sim.climate_params().wind_towards_deg;
    let d = |lf: f32| model.dynamics(&WindLoad { leaf_fraction: Some(lf), azimuth_deg: az, ..WindLoad::new(0.0) });
    let (a, b) = (d(1.0), d(0.0));
    let h = sim.height_m() as f64;
    let age = sim.age_years().round() as u32;
    let tried = STORM_AGES.contains(&age) && sim.is_alive();
    let st = |g: f32, leafy: bool| if tried { storm_on_copy(sim, g, leafy) } else { Vec::new() };
    AgeDyn {
        f0_leafy: a.natural_frequency_hz as f64,
        f0_leafless: b.natural_frequency_hz as f64,
        damping_leafy: a.damping_ratio as f64,
        damping_leafless: b.damping_ratio as f64,
        d_over_h2: sim.dbh_cm() as f64 / 100.0 / (h * h).max(1e-9),
        storm35: st(35.0, false),
        storm38: st(38.0, false),
        storm35_leafy: st(35.0, true),
        tried,
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

/// Extra protocol runs: the same oaks without wind acclimation.
#[derive(Clone, Debug, Default)]
pub struct Extra {
    /// (height m, DBH cm, stem diameter at half height / DBH) at 40 years,
    /// with and without acclimation, per seed.
    pub with: Vec<(f64, f64, f64)>,
    pub without: Vec<(f64, f64, f64)>,
}

/// Stem diameter at half the height over the DBH.
fn taper_half(sim: &Simulation) -> f64 {
    let (segs, _) = sim.mechanics_inputs();
    let h = sim.height_m();
    let model = sim.mechanics_model();
    let s = model.evaluate(&WindLoad::new(0.0));
    let mut best = (f32::MAX, 0.0f32);
    for (seg, role) in segs.iter().zip(&s.segment_role) {
        if *role == dendro_core::SegmentRole::Trunk {
            let dh = (seg.start[1] - 0.5 * h).abs();
            if dh < best.0 {
                best = (dh, 2.0 * seg.radius_m);
            }
        }
    }
    let dbh = sim.dbh_cm() as f64 / 100.0;
    if dbh > 0.0 {
        best.1 as f64 / dbh
    } else {
        0.0
    }
}

pub fn extra(seeds: &[u64]) -> Extra {
    let g = Genome::quercus_robur();
    let grow = |g: &Genome, s: u64, accl: bool| {
        let mut sim = Simulation::new(g.clone(), s);
        sim.set_wind_acclimation_enabled(accl);
        sim.advance_years(40);
        (sim.height_m() as f64, sim.dbh_cm() as f64, taper_half(&sim))
    };
    std::thread::scope(|sc| {
        let a: Vec<_> = seeds.iter().map(|&s| sc.spawn({ let g = g.clone(); move || grow(&g, s, true) })).collect();
        let b: Vec<_> = seeds.iter().map(|&s| sc.spawn({ let g = g.clone(); move || grow(&g, s, false) })).collect();
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
    let oak = cx.oak();
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
            let rc = cx.cherry().map(|c| pearson(&f0_points(c))).unwrap_or(f64::NAN);
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
        "thigmo_jaffe1973" => thigmo(cx, |x| x.0, "hauteur", "la stimulation freine l'élongation").warn("plantes herbacées (via Wikipédia) ; le moteur ne réduit pas l'élongation"),
        "thigmo_beech_guyed" => thigmo(cx, |x| 100.0 * x.0 / x.1, "H/D (test 13 de physics.md : plus faible)", "contrôle mécanique de la croissance radiale"),
        "thigmo_beech_stem_vs_roots" => Measured::not_measurable("le moteur n'acclimate pas l'ancrage : renforcement tige / racines non comparable"),
        "thigmo_hydrangea_dose" => Measured::not_modeled("pas de nombre de flexions par jour"),
        _ => Measured::not_measurable("entrée de vent non reliée"),
    }
}

/// Acclimated vs non-acclimated oaks at 40 years, seed by seed: the rule
/// passes when the acclimated value is lower for every seed.
fn thigmo(cx: &Ctx, f: fn(&(f64, f64, f64)) -> f64, what: &str, rule: &str) -> Measured {
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

/// A Lothar-class storm (35 m/s, winter) asked for on a copy of the tree at
/// 60 years (100 for the oaks): share of seeds whose stem broke or which
/// were uprooted.
pub fn storm_fraction(sp: &SpeciesCtx) -> Measured {
    let a = if sp.genome.species.starts_with("Quercus") { 100 } else { 60 };
    let v = sp.at(a, |r| if r.vent.storm35.iter().any(|k| *k == "stem" || *k == "uproot") { 1.0 } else { 0.0 });
    let any = sp.at(a, |r| if r.vent.storm35.is_empty() { 0.0 } else { 1.0 });
    let n = v.len();
    let frac = v.iter().sum::<f64>() / n.max(1) as f64;
    Measured::one(frac, format!("arbres isolés de {a} ans cassés au tronc ou déracinés par une tempête de 35 m/s demandée en hiver ({} graines ; tout dégât compris : {:.0} %)", n, 100.0 * any.iter().sum::<f64>() / n.max(1) as f64), Nature::Emergent)
        .nc("arbres de peuplement (inventaire) contre arbre isolé")
}

type StatFn = fn(&crate::measure::ClimateStats) -> f64;

/// Report section: wind climate, sway, storms, acclimation.
pub fn section(cx: &Ctx) -> String {
    let mut o = String::from("## Vent : climat, oscillation, tempêtes demandées\n\nLe vent quotidien ne casse rien (seules les tempêtes demandées cassent ou déracinent, décision de l'utilisateur du 2026-11-19) ; il nourrit l'acclimatation de la tige.\n\n");
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
    let _ = writeln!(o, "### Oscillation et tempêtes demandées (copie de l'arbre, rafale à 10 m depuis l'OSO)\n");
    let _ = writeln!(o, "Médianes des graines vivantes. Vitesses critiques (premier mode, m/s) avec facteur dynamique et grandes flèches. Tempêtes : dégâts par graine (`stem` tronc cassé, `top` cime cassée, `limb` charpentière, `uproot` déracinement, `—` rien) ; hiver = défeuillé (aiguilles des persistants), été = mi-juillet de l'année suivante.\n");
    let _ = writeln!(o, "| espèce | âge | f0 feuillé Hz | f0 défeuillé Hz | gain hiver % | 1re ruine hors feuilles | 1re ruine en feuilles | 35 m/s hiver | 38 m/s hiver | 35 m/s été |\n|---|---:|---:|---:|---:|---:|---:|---|---|---|");
    for sp in &cx.species {
        for &a in STORM_AGES.iter() {
            let alive = |r: &&TreeRun| r.death_age.is_none_or(|d| a < d);
            if !sp.runs.iter().filter(alive).any(|r| r.ages.get(&a).is_some_and(|x| x.vent.tried)) {
                continue;
            }
            let f = |g: fn(&measure::AgeRec) -> f64| {
                let v: Vec<f64> = sp.runs.iter().filter(alive).filter_map(|r| r.ages.get(&a)).map(g).filter(|x| x.is_finite()).collect();
                if v.is_empty() {
                    "—".to_string()
                } else {
                    format!("{:.2}", median(&v))
                }
            };
            let st = |g: fn(&AgeDyn) -> &Vec<&'static str>| {
                sp.runs
                    .iter()
                    .filter(alive)
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
                "| {} | {a} | {} | {} | {} | {} | {} | {} | {} | {} |",
                sp.id,
                f(|r| r.vent.f0_leafy),
                f(|r| r.vent.f0_leafless),
                f(|r| (r.vent.f0_leafless / r.vent.f0_leafy - 1.0) * 100.0),
                f(|r| r.first_crit_leafless.unwrap_or(80.0)),
                f(|r| r.first_crit_leafy.unwrap_or(80.0)),
                st(|d| &d.storm35),
                st(|d| &d.storm38),
                st(|d| &d.storm35_leafy)
            );
        }
    }
    o.push('\n');
    let hd = |v: &[(f64, f64, f64)]| median(&v.iter().map(|x| 100.0 * x.0 / x.1).collect::<Vec<_>>());
    let col = |v: &[(f64, f64, f64)], k: usize| median(&v.iter().map(|x| [x.0, x.1, x.2][k]).collect::<Vec<_>>());
    let _ = writeln!(
        o,
        "### Acclimatation de la tige au vent (chêne, 40 ans)\n\nAvec : H {:.2} m, DBH {:.1} cm, H/D {:.1}, diamètre de la tige à mi-hauteur / DBH {:.3}. Sans (`set_wind_acclimation_enabled(false)`) : H {:.2} m, DBH {:.1} cm, H/D {:.1}, {:.3}.\n",
        col(&cx.vent.with, 0),
        col(&cx.vent.with, 1),
        hd(&cx.vent.with),
        col(&cx.vent.with, 2),
        col(&cx.vent.without, 0),
        col(&cx.vent.without, 1),
        hd(&cx.vent.without),
        col(&cx.vent.without, 2)
    );
    o
}
