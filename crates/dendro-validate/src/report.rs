//! Markdown report (French) and a TSV of the verdicts (for diffs between branches).

use crate::data::Entry;
use crate::verdict::{judge, median, stars, weight, Cat, Engine, Measured, Verdict};
use std::collections::BTreeMap;
use std::fmt::Write;

pub struct Row {
    pub subject: String,
    pub entry: Entry,
    pub m: Measured,
    pub v: Verdict,
    pub w: f64,
}

pub fn row(subject: &str, e: &Entry, m: Measured) -> Row {
    let v = judge(e, &m);
    Row { subject: subject.to_string(), entry: e.clone(), w: weight(e), m, v }
}

pub fn domain(q: &str) -> &'static str {
    match q {
        "height_m" | "dbh_cm" | "crown_width_m" | "girth_cm" | "slenderness_hd" | "dbh_increment_mm_yr" | "shoot_length_cm" | "aboveground_biomass_t" | "stem_volume_m3"
        | "crown_diameter_vs_dbh_linear" | "crown_to_stem_ratio" | "shoot_flushes_per_year" => "Croissance et dimensions",
        "ring_width_mm" | "ring_width_trend" => "Cernes",
        "root_depth_m" | "root_spread_m" | "root_architecture" | "root_beta" | "root_fraction_top_10cm" | "root_fraction_top_30cm" | "root_fraction_top_40cm"
        | "root_shoot_ratio" | "sucker_distance_m" | "root_suckering_relative" | "min_soil_depth_cm" => "Racines",
        "budburst_doy" | "leaf_fall_doy" | "flowering_doy" | "flowering_duration_days" | "fruit_maturity_doy" | "budburst_model_form" | "budburst_rmse_d" | "chilling_hours" => {
            "Phénologie"
        }
        "age_first_flowering_years" | "mast_interval_years" | "acorn_mass_g" => "Reproduction",
        "lma_g_m2" | "lai" | "leaf_thickness_um" | "transpiration_l_day" | "transpiration_mm_day" | "transpiration_l_day_per_m2_canopy" | "rew_stress_threshold" => {
            "Feuilles et eau"
        }
        "dry_density_kg_m3" | "green_density_kg_m3" | "mor_mpa" | "moe_gpa" | "compressive_strength_mpa" | "tensile_strength_mpa" | "shrinkage_tangential_pct"
        | "sapwood_width_cm" | "sapwood_diameter_fraction" | "sapwood_rings" | "bark_thickness_mm" | "bark_area_fraction" => "Bois et écorce",
        "longevity_years" | "heart_rot_onset_years" | "heart_rot_fraction" | "dormant_bud_longevity_years" => "Longévité et santé",
        "shade_tolerance" | "drought_tolerance" | "waterlogging_tolerance" | "frost_tolerance" | "frost_kill_new_foliage_c" | "frost_kill_acorn_c" | "soil_ph_range"
        | "max_altitude_m" | "mycorrhiza_type" => "Tolérances, station, symbioses",
        "epicormic_response" | "dead_branch_persistence" | "stump_sprouting" | "pruning_season" | "wound_closure_mm_yr" | "pruning_wound_rot_fraction"
        | "max_occluded_branch_diameter_mm_4y" | "pruning_max_branch_diameter_mm" | "gummosis" | "pruning_crown_mass_threshold_pct" => "Taille, plaies, rejets",
        "basal_area_m2_ha" | "stem_density_per_ha" | "yield_class_m3_ha_yr" | "mai_m3_ha_yr" | "stand_volume_m3_ha" | "rotation_years" => "Peuplement et gestion",
        q if q.starts_with("temp_") || q.starts_with("frost_days") || q.starts_with("hot_days") || q.starts_with("record_") || q.starts_with("rain_") => "Climat",
        "diurnal_range_c" | "sunshine_h_yr" | "global_radiation_mj_m2_yr" | "etp_mm_yr" | "snow_days" | "wind_mean_ms" | "gust_days_ge_16ms" | "gust_days_ge_28ms"
        | "gust_record_ms" => "Climat",
        _ => "Mécanique et vent",
    }
}

fn num(x: f64) -> String {
    if !x.is_finite() {
        return "—".into();
    }
    let a = x.abs();
    if a >= 100.0 {
        format!("{x:.0}")
    } else if a >= 10.0 {
        format!("{x:.1}")
    } else if a >= 1.0 {
        format!("{x:.2}")
    } else if a >= 0.01 {
        format!("{x:.3}")
    } else if a == 0.0 {
        "0".into()
    } else {
        format!("{x:.2e}")
    }
}

pub fn published(e: &Entry) -> String {
    let u = if e.unit == "-" { String::new() } else { format!(" {}", e.unit) };
    let core = match (e.value, e.min, e.max) {
        (Some(v), Some(a), Some(b)) => format!("{} [{}–{}]", num(v), num(a), num(b)),
        (Some(v), None, None) => num(v),
        (None, Some(a), Some(b)) => format!("{}–{}", num(a), num(b)),
        (Some(v), Some(a), None) => format!("{} (≥ {})", num(v), num(a)),
        (Some(v), None, Some(b)) => format!("{} (≤ {})", num(v), num(b)),
        (None, Some(a), None) => format!("≥ {}", num(a)),
        (None, None, Some(b)) => format!("≤ {}", num(b)),
        (None, None, None) => "qualitatif".into(),
    };
    format!("{core}{u} ({})", e.spread_kind)
}

/// Number of seeds of the run (to flag medians over fewer trees, e.g. after
/// wind deaths on the `vent` branch).
pub static SEEDS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub fn engine_cell(m: &Measured) -> String {
    let n = SEEDS.load(std::sync::atomic::Ordering::Relaxed);
    let few = |k: usize| if m.per_seed && m.nature == crate::verdict::Nature::Emergent && k < n { format!(" ({k} graine{} debout)", if k > 1 { "s" } else { "" }) } else { String::new() };
    match &m.engine {
        Engine::Values(v) if v.len() > 1 => {
            let lo = v.iter().copied().filter(|x| x.is_finite()).fold(f64::MAX, f64::min);
            let hi = v.iter().copied().filter(|x| x.is_finite()).fold(f64::MIN, f64::max);
            format!("{} [{}–{}]{}", num(median(v)), num(lo), num(hi), few(v.len()))
        }
        Engine::Values(v) if v.len() == 1 => format!("{}{}", num(v[0]), few(1)),
        Engine::Values(_) if m.per_seed && m.nature == crate::verdict::Nature::Emergent && n > 0 => "— (aucune graine debout)".into(),
        Engine::Rule { detail, .. } => detail.clone(),
        _ => "—".into(),
    }
}

pub fn engine_median(m: &Measured) -> Option<f64> {
    match &m.engine {
        Engine::Values(v) if !v.is_empty() => Some(median(v)).filter(|x| x.is_finite()),
        _ => None,
    }
}

fn short_source(s: &str) -> String {
    let s = s.replace('|', "/");
    let mut cut = s.chars().take(60).collect::<String>();
    if s.chars().count() > 60 {
        cut.push('…');
    }
    cut
}

fn esc(s: &str) -> String {
    s.replace('|', "/").replace('\n', " ")
}

fn conditions(e: &Entry) -> String {
    let c = &e.conditions;
    let mut s = String::new();
    if let Some(a) = c.age_years {
        if a > 1900.0 {
            let _ = write!(s, "année {a:.0} ; ");
        } else {
            let _ = write!(s, "{a:.0} ans ; ");
        }
    }
    if let Some(site) = &c.site {
        s.push_str(&site.chars().take(70).collect::<String>());
        if site.chars().count() > 70 {
            s.push('…');
        }
    }
    if let Some(x) = &c.extra {
        let _ = write!(s, " ({})", x.chars().take(50).collect::<String>());
    }
    esc(&s)
}

#[derive(Default, Clone)]
struct Tally {
    n: BTreeMap<Cat, usize>,
    w_ok: f64,
    w_lim: f64,
    w_judged: f64,
    n_cal: usize,
}

impl Tally {
    fn add(&mut self, r: &Row) {
        *self.n.entry(r.v.cat).or_default() += 1;
        if r.v.cat.judged() {
            self.w_judged += r.w;
            if r.m.calibration {
                self.n_cal += 1;
            }
            match r.v.cat {
                Cat::Ok => self.w_ok += r.w,
                Cat::Limit => self.w_lim += r.w,
                _ => {}
            }
        }
    }
    fn rate(&self) -> String {
        if self.w_judged <= 0.0 {
            "—".into()
        } else {
            format!("{:.0} %", 100.0 * (self.w_ok + 0.5 * self.w_lim) / self.w_judged)
        }
    }
    fn line(&self, name: &str) -> String {
        let mut s = format!("| {name} |");
        for c in Cat::ALL {
            let _ = write!(s, " {} |", self.n.get(&c).copied().unwrap_or(0));
        }
        let _ = write!(s, " {} | {} |", self.rate(), self.n_cal);
        s
    }
}

fn tally_header(first: &str) -> String {
    let mut h = format!("| {first} |");
    let mut sep = String::from("|---|");
    for c in Cat::ALL {
        let _ = write!(h, " {} {} |", c.icon(), c.label());
        sep.push_str("---:|");
    }
    h.push_str(" conformité pondérée | dont calage |");
    sep.push_str("---:|---:|");
    format!("{h}\n{sep}\n")
}

/// Severity used to sort the major gaps: confidence x distance (band
/// half-widths beyond the edge, or relative deviation for indicative rows).
pub fn severity(r: &Row) -> f64 {
    // Parameters, constants and rules whose criterion is the bench's own
    // count half: they say less about the model than a simulated value
    // judged against a published spread.
    let rule = matches!(r.m.engine, Engine::Rule { .. });
    let k = if r.m.nature == crate::verdict::Nature::Emergent && !rule { 1.0 } else { 0.5 };
    k * match r.v.cat {
        Cat::Out => r.w * (1.0 + r.v.excess.unwrap_or(if rule { 0.5 } else { 1.0 })),
        Cat::Limit => r.w * 0.5,
        Cat::Indicative => r.w * r.v.dev.map(|d| d.abs()).unwrap_or(0.0),
        _ => 0.0,
    }
}

pub struct Meta {
    pub title: String,
    pub command: String,
    pub seeds: Vec<u64>,
    pub years: String,
    pub duration_s: f64,
    pub determinism: String,
    pub extra_sections: Vec<String>,
    /// Hand-written observations, placed before the major gaps.
    pub notes: String,
}

pub fn subject_title(s: &str) -> String {
    if s == "physics" {
        return "Physique, climat de Rennes, sol".to_string();
    }
    match dendro_core::Genome::by_id(s) {
        Some(g) => format!("{} ({})", g.common_name_fr, g.species),
        None => s.to_string(),
    }
}

/// Short lower-case tag of a subject for the per-domain table.
fn subject_tag(s: &str) -> String {
    match s {
        "quercus_robur" => "chêne".to_string(),
        "physics" => "physique".to_string(),
        _ => dendro_core::Genome::by_id(s).map_or_else(|| s.to_string(), |g| g.common_name_fr.to_lowercase()),
    }
}

pub fn markdown(rows: &[Row], meta: &Meta) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "# {}\n", meta.title);
    let _ = writeln!(
        o,
        "Rapport généré par `dendro-validate` (ne pas éditer à la main). Régénérer :\n\n```\n{}\n```\n",
        meta.command
    );
    let _ = writeln!(
        o,
        "Graines : {:?} ; durées simulées : {} ; durée du banc : {:.0} s. Contrôle « le banc ne change rien » : {}.\n",
        meta.seeds, meta.years, meta.duration_s, meta.determinism
    );
    o.push_str(METHOD);

    // Summary by subject.
    let _ = writeln!(o, "## Résumé par jeu de référence\n");
    o.push_str(&tally_header("jeu"));
    let mut by_subj: BTreeMap<&str, Tally> = BTreeMap::new();
    let mut all = Tally::default();
    for r in rows {
        by_subj.entry(r.subject.as_str()).or_default().add(r);
        all.add(r);
    }
    for (s, t) in &by_subj {
        let _ = writeln!(o, "{}", t.line(&subject_title(s)));
    }
    let _ = writeln!(o, "{}\n", all.line("**total**"));
    let _ = writeln!(
        o,
        "Conformité pondérée = Σ poids(✅) + ½ Σ poids(⚠️), divisé par Σ poids des entrées jugées (✅ ⚠️ ❌). « dont calage » : entrées jugées qui ne font que contrôler une source ayant servi au calage du moteur.\n"
    );
    // Summary independent of calibration.
    let mut indep = Tally::default();
    for r in rows.iter().filter(|r| !r.m.calibration && r.m.nature == crate::verdict::Nature::Emergent) {
        indep.add(r);
    }
    let _ = writeln!(o, "Entrées **indépendantes et simulées** seulement (ni calage, ni paramètre lu, ni constante) :\n");
    o.push_str(&tally_header("sélection"));
    let _ = writeln!(o, "{}\n", indep.line("indépendantes, simulées"));

    // By domain.
    let _ = writeln!(o, "## Résumé par domaine\n");
    o.push_str(&tally_header("domaine"));
    let mut by_dom: BTreeMap<(&str, &str), Tally> = BTreeMap::new();
    for r in rows {
        by_dom.entry((domain(&r.entry.quantity), r.subject.as_str())).or_default().add(r);
    }
    for ((d, s), t) in &by_dom {
        let _ = writeln!(o, "{}", t.line(&format!("{d} — {}", subject_tag(s))));
    }
    o.push('\n');

    o.push_str(&meta.notes);
    o.push('\n');

    // Major gaps.
    let _ = writeln!(o, "## Écarts majeurs\n");
    let _ = writeln!(
        o,
        "Triés par gravité = poids de confiance x distance (demi-largeurs de plage au-delà du bord pour ❌, écart relatif pour ℹ️), divisée par deux pour les paramètres, constantes et règles dont le critère est un choix du banc. Les valeurs uniques (ℹ️) n'ont pas de tolérance publiée : elles figurent ici dès que l'écart relatif dépasse 30 % (seuil de tri du banc, pas une tolérance).\n"
    );
    let mut gaps: Vec<&Row> = rows
        .iter()
        .filter(|r| matches!(r.v.cat, Cat::Out | Cat::Limit) || (r.v.cat == Cat::Indicative && r.v.dev.map(|d| d.abs() > 0.3).unwrap_or(false)))
        .collect();
    gaps.sort_by(|a, b| severity(b).partial_cmp(&severity(a)).unwrap());
    let _ = writeln!(o, "| # | jeu | entrée | publié | moteur | verdict | écart | confiance | nature | remarque |\n|---:|---|---|---|---|---|---:|---|---|---|");
    for (i, r) in gaps.iter().enumerate() {
        let _ = writeln!(
            o,
            "| {} | {} | `{}` {} | {} | {} | {} {} | {} | {} {:.2} ({}/{}) | {}{} | {} |",
            i + 1,
            r.subject,
            r.entry.id,
            esc(&r.m.what),
            esc(&published(&r.entry)),
            esc(&engine_cell(&r.m)),
            r.v.cat.icon(),
            r.v.cat.label(),
            r.v.dev.map(|d| format!("{:+.0} %", 100.0 * d)).unwrap_or_default(),
            stars(r.w),
            r.w,
            r.entry.source_kind,
            r.entry.read,
            r.m.nature.label(),
            if r.m.calibration { ", calage" } else { "" },
            esc(&[r.v.why.clone(), r.m.warn.clone().unwrap_or_default()].join(" ; "))
        );
    }
    o.push('\n');

    // Divergent sources.
    let _ = writeln!(o, "## Sources divergentes\n");
    let _ = writeln!(o, "Même jeu et même observable, verdicts opposés (au moins un ✅ et un ❌) : la divergence entre sources est montrée telle quelle.\n");
    let mut groups: BTreeMap<(String, String), Vec<&Row>> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.v.cat.judged()) {
        groups.entry((r.subject.clone(), r.entry.quantity.clone())).or_default().push(r);
    }
    let mut any = false;
    for ((s, q), g) in &groups {
        if g.iter().any(|r| r.v.cat == Cat::Ok) && g.iter().any(|r| r.v.cat == Cat::Out) {
            any = true;
            let items: Vec<String> = g.iter().map(|r| format!("`{}` {} {}", r.entry.id, esc(&published(&r.entry)), r.v.cat.icon())).collect();
            let _ = writeln!(o, "- **{s} / {q}** : {}", items.join(" ; "));
        }
    }
    if !any {
        let _ = writeln!(o, "Aucune.");
    }
    o.push('\n');

    // Not modelled.
    let _ = writeln!(o, "## Ce que le moteur ne modélise pas encore\n");
    let mut nm: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.v.cat == Cat::NotModeled) {
        nm.entry(r.v.why.clone()).or_default().push(format!("`{}`", r.entry.id));
    }
    for (why, ids) in &nm {
        let _ = writeln!(o, "- {} : {}", why, ids.join(", "));
    }
    let _ = writeln!(o, "\n### Non mesuré par le banc\n");
    let mut ms: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.v.cat == Cat::NotMeasurable) {
        ms.entry(r.v.why.clone()).or_default().push(format!("`{}`", r.entry.id));
    }
    for (why, ids) in &ms {
        let _ = writeln!(o, "- {} : {}", why, ids.join(", "));
    }
    o.push('\n');

    // Blocking-test candidates.
    let _ = writeln!(o, "## Candidats à des tests bloquants\n");
    let _ = writeln!(
        o,
        "Entrées ✅ dont la valeur est simulée (pas un paramètre ni une constante), indépendantes du calage, de confiance ≥ 0,7, à plage publiée (ni valeur unique, ni record : la règle des records est une convention du banc). « marge » = distance au bord le plus proche en part de la demi-largeur ; les entrées de marge < 20 % sont écartées (test fragile). Les dates de débourrement comparent une moyenne climatique de Rennes à des années précises d'un site plus frais : à garder comme garde-fou large, pas comme calage.\n"
    );
    let _ = writeln!(o, "| jeu | entrée | publié | moteur | marge | confiance |\n|---|---|---|---|---:|---:|");
    for r in rows.iter().filter(|r| {
        r.v.cat == Cat::Ok
            && r.m.nature == crate::verdict::Nature::Emergent
            && !r.m.calibration
            && r.w >= 0.7
            && !matches!(r.entry.spread_kind.as_str(), "single" | "max_record")
    }) {
        let mval = match (crate::verdict::band(&r.entry), engine_median(&r.m)) {
            ((Some(a), Some(b)), Some(x)) => {
                let h = 0.5 * (b - a);
                Some((h - (x - 0.5 * (a + b)).abs()) / h.max(1e-12))
            }
            _ => None,
        };
        if mval.is_some_and(|m| m < 0.2) {
            continue;
        }
        let margin = mval.map(|m| format!("{:.0} %", 100.0 * m)).unwrap_or("—".into());
        let _ = writeln!(o, "| {} | `{}` | {} | {} | {} | {:.2} |", r.subject, r.entry.id, esc(&published(&r.entry)), esc(&engine_cell(&r.m)), margin, r.w);
    }
    o.push('\n');

    for s in &meta.extra_sections {
        o.push_str(s);
        o.push('\n');
    }

    // Detailed tables.
    let _ = writeln!(o, "## Tableau détaillé\n");
    let subjects: Vec<&str> = by_subj.keys().copied().collect();
    for s in subjects {
        let _ = writeln!(o, "### {}\n", subject_title(s));
        let _ = writeln!(o, "| entrée | observable | conditions | publié | moteur médiane [min–max graines] | verdict | nature | source | lecture (poids) |\n|---|---|---|---|---|---|---|---|---|");
        for r in rows.iter().filter(|r| r.subject == s) {
            let mut verdict = format!("{} {}", r.v.cat.icon(), r.v.cat.label());
            if let Some(d) = r.v.dev {
                let _ = write!(verdict, " ({:+.0} %)", 100.0 * d);
            }
            let note = [r.v.why.clone(), r.m.warn.clone().unwrap_or_default()].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" ; ");
            let what = if r.m.what.is_empty() { String::new() } else { format!(" — {}", r.m.what) };
            let _ = writeln!(
                o,
                "| `{}` | {} | {} | {} | {}{} | {} — {} | {}{} | {} | {}/{} {} ({:.2}) |",
                r.entry.id,
                r.entry.quantity,
                conditions(&r.entry),
                esc(&published(&r.entry)),
                esc(&engine_cell(&r.m)),
                esc(&what),
                verdict,
                esc(&note),
                if r.m.forced.is_some() { "—" } else { r.m.nature.label() },
                if r.m.calibration { ", calage" } else { "" },
                esc(&short_source(&r.entry.source)),
                r.entry.source_kind,
                r.entry.read,
                stars(r.w),
                r.w
            );
        }
        o.push('\n');
    }
    o
}

const METHOD: &str = "## Méthode\n\n\
- **Arbre isolé** sur le site de référence (climat de Rennes par défaut, sol par défaut), plusieurs graines ; on compare la **médiane** des graines et on montre leur dispersion [min–max]. Une entrée « en peuplement », d'une autre espèce ou d'un autre état du bois est ➖ non comparable : la valeur du moteur est tout de même affichée.\n\
- **Tolérance tirée uniquement de la dispersion publiée** (`spread_kind`) : `range`/`ci95`/`quantiles` → dans la plage ; `sd` → à ± 2 sd (les fichiers saisissent min/max = valeur ± 1 sd) ; ⚠️ limite = hors plage mais à moins de 1,5 fois la demi-largeur du centre ; `single` → ℹ️ indicatif, écart relatif rapporté sans verdict ; borne unique (« ≥ 10 m ») → respectée ✅, manquée d'un facteur < 1,5 ⚠️, au-delà ❌ (convention du banc).\n\
- **Records** (`max_record`) : le moteur (maximum sur la vie simulée, sauf mention) ne doit pas dépasser le record de plus de 10 % (❌ ; entre 0 et 10 % ⚠️), ni rester sous 50 % de la borne basse du record (⚠️). Ces deux seuils sont des conventions du banc, pas des tolérances publiées.\n\
- **Âge de référence « adulte »** pour les tailles publiées sans âge : 150 ans (chêne), 70 ans (merisier, maturité 60-80 ans d'EUFORGEN). C'est un choix du banc.\n\
- **Climat** : statistiques sur de longues séries (une série par graine), records = médiane des maxima de fenêtres de 80 ans (durée des séries de records de la station).\n\
- **Phénologie** : dates moyennes sur toutes les années (à partir de 5 ans pour les feuilles), jour de l'année compté à partir de 1.\n\
- **Nature de la valeur moteur** : *simulé* (émerge de la simulation), *paramètre* (valeur du génome lue telle quelle : le verdict juge le paramètre, pas le modèle), *constante* (du code), *par construction*. **calage** : la source a servi à régler le moteur ; un ✅ y est attendu et ne valide rien d'indépendant.\n\
- **Confiance** = poids(source_kind) x poids(read) : primary 1,0, review 0,9, handbook 0,8, secondary 0,6, wikipedia 0,4 ; full/table 1,0, abstract 0,7, via_wikipedia 0,6, snippet 0,5. ●●● ≥ 0,85, ●●○ ≥ 0,5, ●○○ en dessous.\n\
- Catégories : ✅ conforme, ⚠️ limite, ❌ hors plage, ℹ️ indicatif (valeur unique), ➖ non comparable (conditions), ⬜ non modélisé, ◻️ non mesurable (modélisé mais sans protocole de mesure dans le banc, ou énoncé sans critère opérationnel), ∅ entrée d'absence.\n\
- Les règles qualitatives vérifiées par le banc énoncent leur critère dans la cellule « moteur » ; quand ce critère est un choix du banc, c'est écrit.\n\n";

/// TSV of verdicts: subject, id, quantity, category, engine median, weight.
pub fn tsv(rows: &[Row]) -> String {
    let mut o = String::from("subject\tid\tquantity\tcat\tengine\tweight\tdev\n");
    for r in rows {
        let _ = writeln!(
            o,
            "{}\t{}\t{}\t{}\t{}\t{:.3}\t{}",
            r.subject,
            r.entry.id,
            r.entry.quantity,
            r.v.cat.label(),
            engine_median(&r.m).map(|x| format!("{x}")).unwrap_or_default(),
            r.w,
            r.v.dev.map(|d| format!("{d:.4}")).unwrap_or_default()
        );
    }
    o
}

/// Diff of two TSV files (from -> to): changed categories, sorted.
pub fn diff(from: &str, to: &str, from_name: &str, to_name: &str) -> String {
    let parse = |s: &str| -> BTreeMap<(String, String), (String, String, String, String)> {
        s.lines()
            .skip(1)
            .filter_map(|l| {
                let f: Vec<&str> = l.split('\t').collect();
                (f.len() >= 7).then(|| ((f[0].to_string(), f[1].to_string()), (f[3].to_string(), f[4].to_string(), f[2].to_string(), f[6].to_string())))
            })
            .collect()
    };
    let a = parse(from);
    let b = parse(to);
    let rank = |c: &str| match c {
        "conforme" => 0,
        "limite" => 1,
        "hors plage" => 2,
        _ => 3,
    };
    let (mut better, mut worse, mut other) = (Vec::new(), Vec::new(), Vec::new());
    let mut moved = Vec::new();
    for (k, (ca, ea, q, _)) in &a {
        let Some((cb, eb, _, _)) = b.get(k) else { continue };
        let line = format!("| {} | `{}` | {} | {} → {} | {} → {} |", k.0, k.1, q, ca, cb, short(ea), short(eb));
        if ca != cb {
            let (ra, rb) = (rank(ca), rank(cb));
            if ra < 3 && rb < 3 {
                if rb < ra {
                    better.push(line);
                } else {
                    worse.push(line);
                }
            } else {
                other.push(line);
            }
        } else if ea != eb {
            let (fa, fb) = (ea.parse::<f64>().unwrap_or(f64::NAN), eb.parse::<f64>().unwrap_or(f64::NAN));
            let rel = (fb - fa) / fa.abs().max(1e-9);
            if rel.abs() > 0.05 {
                moved.push((rel.abs(), format!("| {} | `{}` | {} | {} | {} → {} ({:+.0} %) |", k.0, k.1, q, ca, short(ea), short(eb), 100.0 * rel)));
            }
        }
    }
    moved.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
    let mut o = format!("## Diff des verdicts {from_name} → {to_name}\n\n");
    let head = "| jeu | entrée | observable | verdict | moteur (médiane) |\n|---|---|---|---|---|\n";
    let _ = writeln!(o, "### Ce qui s'améliore ({})\n", better.len());
    if !better.is_empty() {
        o.push_str(head);
        o.push_str(&better.join("\n"));
        o.push('\n');
    }
    let _ = writeln!(o, "\n### Ce qui se dégrade ({})\n", worse.len());
    if !worse.is_empty() {
        o.push_str(head);
        o.push_str(&worse.join("\n"));
        o.push('\n');
    }
    let _ = writeln!(o, "\n### Changements impliquant une entrée non jugée (➖ ⬜ ◻️ ℹ️) ({})\n", other.len());
    if !other.is_empty() {
        o.push_str(head);
        o.push_str(&other.join("\n"));
        o.push('\n');
    }
    let _ = writeln!(o, "\n### Même verdict, valeur du moteur changée de plus de 5 % ({})\n", moved.len());
    if !moved.is_empty() {
        o.push_str("| jeu | entrée | observable | verdict | moteur (médiane) |\n|---|---|---|---|---|\n");
        for (_, l) in &moved {
            o.push_str(l);
            o.push('\n');
        }
    }
    o
}

fn short(s: &str) -> String {
    s.parse::<f64>().map(num).unwrap_or_else(|_| if s.is_empty() { "—".into() } else { s.to_string() })
}
