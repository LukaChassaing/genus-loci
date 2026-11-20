//! Entry by entry: which engine measure answers which published value.
//!
//! Explicit per-id table (with a few per-quantity defaults): the conditions
//! of an entry (age, stand or isolated tree, species, wood state) decide
//! how the engine is read and whether the comparison is meaningful.

use crate::data::Entry;
use crate::measure::{AgeRec, ClimateStats, TreeRun, YearRec};
use crate::verdict::{median, Measured, Nature};
use dendro_core::Genome;

pub struct SpeciesCtx {
    /// Registry id (`data/species/<id>.ron`, `data/validation/<id>.ron`).
    pub id: String,
    pub genome: Genome,
    pub runs: Vec<TreeRun>,
    /// Same seeds on the gley clay preset (waterlogging rule), 30 years.
    pub gley: Vec<TreeRun>,
    /// Reference age for "mature" sizes given without an age (bench choice).
    pub mature_age: u32,
    /// Age of the July transpiration read for `transpiration_l_day` without
    /// an age (bench choice).
    pub transp_age: u32,
    /// Age of the critical gust read for `critical_wind_ms` (bench choice).
    pub crit_age: u32,
    /// Runs of the seeds after those of `runs` (seeds `runs.len() + 1` ..),
    /// simulated only for the entries that ask for more seeds than the run
    /// (`Entry::seeds`).
    pub extra_runs: Vec<TreeRun>,
}

impl SpeciesCtx {
    /// The same species read on the first `n` seeds (`runs` then
    /// `extra_runs`), for an entry with `seeds: Some(n)`.
    fn with_seeds(&self, n: usize) -> SpeciesCtx {
        SpeciesCtx {
            id: self.id.clone(),
            genome: self.genome.clone(),
            runs: self.runs.iter().chain(&self.extra_runs).take(n).cloned().collect(),
            gley: self.gley.clone(),
            mature_age: self.mature_age,
            transp_age: self.transp_age,
            crit_age: self.crit_age,
            extra_runs: Vec::new(),
        }
    }
}

pub struct Ctx {
    /// Every species simulated in this run (registry order).
    pub species: Vec<SpeciesCtx>,
    pub climate: Vec<ClimateStats>,
    pub vent: crate::vent::Extra,
}

impl Ctx {
    pub fn get(&self, id: &str) -> Option<&SpeciesCtx> {
        self.species.iter().find(|s| s.id == id)
    }
    /// The oak: reference species of `physics.ron` (that file is only
    /// evaluated when the oak is part of the run).
    pub fn oak(&self) -> &SpeciesCtx {
        self.get("quercus_robur").expect("physics.ron needs the oak in the run")
    }
    /// The cherry, when simulated (secondary values of a few physics rows).
    pub fn cherry(&self) -> Option<&SpeciesCtx> {
        self.get("prunus_avium")
    }
}

/// Lower-case text of an entry (conditions, quote, note): qualitative
/// entries carry their claim there.
fn entry_text(e: &Entry) -> String {
    [e.conditions.extra.as_deref(), e.quote.as_deref(), e.note.as_deref()].into_iter().flatten().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Text of the conditions only (site + extra), lower case. Unlike
/// `entry_text` it leaves out the quote and the note, which often talk about
/// the neighbouring entry ("60-70 years in closed stands (next entry)").
fn conditions_text(e: &Entry) -> String {
    [e.conditions.site.as_deref(), e.conditions.extra.as_deref()].into_iter().flatten().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// The conditions describe trees grown in a stand (closed, dense, mature
/// stand, plantation crop, field hedge), not an isolated tree.
fn stand_condition(e: &Entry) -> bool {
    let t = conditions_text(e);
    if ["free-standing", "open-grown", "solitary", "isolé", "isolated"].iter().any(|k| t.contains(k)) {
        return false;
    }
    ["dense stand", "closed stand", "in stands", "mature stand", "peuplement", "im bestand", "dense canopies", "haies de"].iter().any(|k| t.contains(k))
}

/// The conditions announce a maximum ("maximum", "up to 2 m"), not a typical
/// size: the engine is read at its largest size over the simulated life.
fn maximum_condition(e: &Entry) -> bool {
    let t = conditions_text(e);
    ["maximum", "maximal", "up to", "jusqu'à", "exceptionally"].iter().any(|k| t.contains(k))
}

/// Occurrences of `kw` at the start of a word that no negation precedes
/// ("not ectomycorrhiza", "pas ectomycorhize", "contre l'arbuscule").
fn affirmed(txt: &str, kws: &[&str]) -> bool {
    const NEG: [&str; 9] = [" not ", " no ", " non ", " pas ", " contre ", " unlike ", " nicht ", " rather than ", " instead of "];
    kws.iter().any(|kw| {
        txt.match_indices(kw).any(|(i, _)| {
            let word_start = txt[..i].chars().next_back().is_none_or(|c| !c.is_alphabetic());
            // The 24 characters before, punctuation read as spaces.
            let tail: String = txt[..i].chars().rev().take(24).collect::<Vec<_>>().into_iter().rev().collect();
            let before = format!(" {}", tail.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { ' ' }).collect::<String>());
            let before = format!("{before} ");
            word_start && !(NEG.iter().any(|n| before.contains(n)) || before.trim_end().ends_with("non-"))
        })
    })
}

/// Years of a run while the tree is standing (all of them on main).
pub fn living(r: &TreeRun) -> &[YearRec] {
    match r.death_age {
        Some(d) => &r.years[..(d as usize).saturating_sub(1).min(r.years.len())],
        None => &r.years,
    }
}

fn alive_at(r: &TreeRun, age: u32) -> bool {
    r.death_age.is_none_or(|d| age < d)
}

impl SpeciesCtx {
    pub fn year(&self, age: u32, f: impl Fn(&YearRec) -> f64) -> Vec<f64> {
        self.runs.iter().filter(|r| alive_at(r, age)).filter_map(|r| r.years.get(age as usize - 1).map(&f)).collect()
    }
    /// Detailed record at `age`; when no run sampled that exact age (the
    /// reference "adult" age is not always in `sample_ages`), the nearest
    /// sampled age stands in.
    pub fn at(&self, age: u32, f: impl Fn(&AgeRec) -> f64) -> Vec<f64> {
        let sampled = self.runs.iter().any(|r| r.ages.contains_key(&age));
        let age = if sampled {
            age
        } else {
            self.runs.first().and_then(|r| r.ages.keys().copied().min_by_key(|k| k.abs_diff(age))).unwrap_or(age)
        };
        self.runs.iter().filter(|r| alive_at(r, age)).filter_map(|r| r.ages.get(&age).map(&f)).collect()
    }
    fn lifetime_max(&self, f: impl Fn(&YearRec) -> f64) -> Vec<f64> {
        self.runs.iter().map(|r| living(r).iter().map(&f).fold(f64::MIN, f64::max)).collect()
    }
    pub fn last_age(&self) -> u32 {
        self.runs.iter().map(|r| r.run_years.max(r.years.len() as u32)).max().unwrap_or(0)
    }
    /// Emergent longevity: age of trunk death per seed (`dendro_core::
    /// vitality`); a seed still alive at the end of the run is censored.
    /// `(death ages, censored seeds, run length)`.
    pub fn death_ages(&self) -> (Vec<u32>, usize, u32) {
        let dead: Vec<u32> = self.runs.iter().filter_map(|r| r.death_age).collect();
        (dead.clone(), self.runs.len() - dead.len(), self.last_age())
    }
    /// Mean of a yearly date over the years from `from_age` on (1-based DOY).
    fn mean_date(&self, from_age: u32, f: impl Fn(&YearRec) -> Option<u32>) -> Vec<f64> {
        self.runs
            .iter()
            .map(|r| {
                let v: Vec<f64> = living(r).iter().filter(|y| y.age >= from_age).filter_map(&f).map(|d| d as f64 + 1.0).collect();
                if v.is_empty() {
                    f64::NAN
                } else {
                    v.iter().sum::<f64>() / v.len() as f64
                }
            })
            .collect()
    }
    /// Mean ring width at 1.3 m over a window of cambial ages (rings
    /// counted from the first ring measured at breast height, as on a core).
    fn ring_mean(&self, a: u32, b: u32) -> Vec<f64> {
        self.runs
            .iter()
            .map(|r| {
                let v: Vec<f64> = living(r)
                    .iter()
                    .filter(|y| y.ring_at_m >= 1.29)
                    .filter_map(|y| y.ring_mm)
                    .enumerate()
                    .filter(|(k, _)| (k + 1) as u32 >= a && (k + 1) as u32 <= b)
                    .map(|(_, w)| w)
                    .collect();
                v.iter().sum::<f64>() / v.len().max(1) as f64
            })
            .collect()
    }
    fn at_dbh(&self, d: u32, f: impl Fn(&crate::measure::AtDbh) -> f64) -> Vec<f64> {
        self.runs.iter().filter_map(|r| r.at_dbh.get(&d).map(&f)).collect()
    }
    fn first_flowering(&self) -> Vec<f64> {
        self.runs.iter().map(|r| r.first_flowering_age.map(|a| a as f64).unwrap_or(f64::NAN)).collect()
    }
}

/// Emergent longevity (median age of trunk death over the seeds) against a
/// longevity entry. Censoring: seeds alive at the end of the run count as
/// "dies later than the run"; with at most half of them censored the
/// median is still a death age (censored seeds are put at the run length,
/// above every death age). Otherwise the median is only known to exceed
/// the run: the entry is judged only when that bound already settles it
/// (beyond the top of the published band or record: out), else it is not
/// measurable. Calibration: `LongevityTraits::maintenance_per_m` was tuned
/// per species so that trees die within the published ranges, so every
/// comparable longevity verdict is a calibration check.
fn longevity(sp: &SpeciesCtx, e: &Entry) -> Measured {
    let (dead, censored, n) = sp.death_ages();
    let total = dead.len() + censored;
    let what = format!("âge médian de mort du tronc, émergent ({}/{} graines mortes en {n} ans)", dead.len(), total);
    if dead.len() * 2 > total {
        let mut v: Vec<f64> = dead.iter().map(|&a| a as f64).collect();
        v.extend(std::iter::repeat_n(n as f64, censored));
        let m = Measured::values(v, what, Nature::Emergent).calib();
        return if censored > 0 { m.warn(format!("{censored} graine(s) encore vivante(s) à {n} ans, comptée(s) à {n} ans")) } else { m };
    }
    let top = e.max.or(e.value).or(e.min);
    let m = Measured::one(n as f64, format!("{what} : médiane > {n} ans (censurée)"), Nature::Emergent).calib();
    match top {
        Some(t) if (n as f64) > t * if e.spread_kind == "max_record" { crate::verdict::RECORD_EXCEED } else { 1.0 } => m.warn("borne inférieure : la médiane dépasse déjà la borne haute publiée"),
        _ => Measured::not_measurable(format!("{what} : médiane > {n} ans (censurée), sous la longévité publiée : run trop court pour conclure")),
    }
}

fn age_of(e: &Entry) -> Option<u32> {
    e.conditions.age_years.map(|a| a.round() as u32)
}

const STAND: &str = "peuplement (le moteur simule un arbre isolé)";

/// Measurement for an entry of a species file (`cx`: the other species of
/// the run, for the few rules that compare with the oak).
///
/// Fruit-tree files tag each entry `FRANC/plein vent`, `VERGER greffé
/// <porte-greffe>` or `MIXTE` at the start of `conditions.extra`: the engine
/// grows an own-rooted seedling tree (no grafting, no rootstock), so the
/// grafted-orchard entries are not comparable.
pub fn species_entry(sp: &SpeciesCtx, cx: &Ctx, e: &Entry) -> Measured {
    if let Some(n) = e.seeds.map(|n| n as usize).filter(|&n| n > sp.runs.len()) {
        let wide = sp.with_seeds(n);
        let mut m = species_entry_seeded(&wide, cx, e);
        m.what = format!("{} — sur {} graines (1-{}, champ `seeds` de l'entrée)", m.what, wide.runs.len(), wide.runs.len());
        return m;
    }
    species_entry_seeded(sp, cx, e)
}

fn species_entry_seeded(sp: &SpeciesCtx, cx: &Ctx, e: &Entry) -> Measured {
    let m = species_entry_raw(sp, cx, e);
    let extra = e.conditions.extra.as_deref().map(|x| x.trim_start().to_lowercase()).unwrap_or_default();
    if extra.starts_with("verger greffé") && m.comparable() {
        return m.nc("verger greffé sur porte-greffe : le moteur simule un arbre franc de semis, sans greffe ni taille");
    }
    if extra.starts_with("cultivar") && m.comparable() {
        return m.nc("cultivar de verger : le moteur simule un arbre franc de semis, sans greffe ni conduite de verger");
    }
    if e.note.as_deref().is_some_and(|n| n.to_lowercase().contains("arbres de verger")) && m.comparable() {
        return m.nc("la source décrit des arbres de verger (greffés, conduits), pas un franc de pied");
    }
    m
}

fn species_entry_raw(sp: &SpeciesCtx, cx: &Ctx, e: &Entry) -> Measured {
    let mat = sp.mature_age;
    let id = e.id.as_str();
    let q = e.quantity.as_str();
    let g = &sp.genome;
    let age = age_of(e);
    let at_age_h = |a: u32| Measured::values(sp.year(a, |y| y.height_m), format!("hauteur à {a} ans"), Nature::Emergent);
    let at_age_d = |a: u32| Measured::values(sp.year(a, |y| y.dbh_cm), format!("DBH à {a} ans"), Nature::Emergent);
    let mature_h = || Measured::values(sp.year(mat, |y| y.height_m), format!("hauteur à {mat} ans (âge de référence « adulte » du banc)"), Nature::Emergent);
    let mature_d = || Measured::values(sp.year(mat, |y| y.dbh_cm), format!("DBH à {mat} ans (âge de référence « adulte » du banc)"), Nature::Emergent);
    let life_h = || Measured::values(sp.lifetime_max(|y| y.height_m), format!("hauteur maximale sur {} ans simulés", sp.last_age()), Nature::Emergent);
    let life_d = || Measured::values(sp.lifetime_max(|y| y.dbh_cm), format!("DBH maximal sur {} ans simulés", sp.last_age()), Nature::Emergent);
    let individual = "arbre remarquable individuel, pas une valeur typique";
    let lifespan = || longevity(sp, e);

    match q {
        "height_m" => {
            if id.contains("ivenack") {
                return life_h().nc(format!("{individual} (500-800 ans)"));
            }
            if id == "wp_sv_prins_eugen" {
                return at_age_h(350.min(sp.last_age())).nc(individual);
            }
            if id.starts_with("cienciala_stand") || id.starts_with("tomczak") || id.starts_with("pryor_topheight") || id.starts_with("pretzsch_topheight") || id == "stoj_height_59y" || id == "wp_sv_height_50y" {
                let a = age.unwrap_or(mat).min(sp.last_age());
                return at_age_h(a).nc(STAND);
            }
            if id == "wp_nl_height_forest" || id == "wppl_height" {
                return mature_h().nc(STAND);
            }
            if id.starts_with("pryor_freegrowth") {
                return at_age_h(age.unwrap()).calib().warn("modèle préliminaire de Pryor (92 arbres d'alignement et de lisière) ; cible de calage du génome selon docs/validation/prunus_avium.md");
            }
            if id == "pretzsch_free_height_cap" {
                return life_h();
            }
            // Spruce minus fir at equal age: a difference of heights between
            // two genomes, not a height.
            if id == "utschig_gap" {
                return height_gap(sp, cx, 100);
            }
            let ctext = conditions_text(e);
            // Clear bole length (a stand quantity), not the tree height.
            if ctext.contains("bole") || ctext.contains("fût") {
                let a = age.unwrap_or(mat).min(sp.last_age());
                let m = Measured::values(sp.at(a, |r| r.clear_bole_m), format!("hauteur du fût sans grosse branche, {a} ans"), Nature::Emergent);
                return if stand_condition(e) { m.nc(STAND) } else { m };
            }
            if ctext.contains("dominant height") || ctext.contains("hauteur dominante") {
                let a = age.unwrap_or(mat).min(sp.last_age());
                return at_age_h(a).nc("hauteur dominante de peuplement (potentiel de station) : le moteur simule un arbre isolé");
            }
            if ctext.contains("maximum mai") {
                return mature_h().nc("table de production de plantation, âge du MAI maximal non donné dans l'entrée");
            }
            if e.spread_kind == "max_record" {
                return life_h();
            }
            if age.is_none() && maximum_condition(e) {
                return life_h().warn("l'entrée annonce un maximum : lu au maximum de vie simulé, pas à l'âge de référence");
            }
            if let Some(a) = age {
                return at_age_h(a.min(sp.last_age()));
            }
            if stand_condition(e) {
                return mature_h().nc(STAND);
            }
            let m = mature_h();
            if id == "atlas_height_range" {
                return m.calib();
            }
            m
        }
        "dbh_cm" => {
            if id == "atlas_dbh_veteran" {
                return life_d().nc("classe d'âge > 1000 ans, hors de la durée simulée");
            }
            if id == "atlas_dbh_usual" {
                return mature_d().warn("« jusqu'à 1 m » pour les arbres usuels : lu au DBH de l'âge de référence, pas au maximum de vie");
            }
            if id.starts_with("tomczak") || id.starts_with("pryor_stand") || id == "stoj_dbh_59y" || id == "wp_sv_dbh_50y" {
                let a = age.unwrap_or(mat).min(sp.last_age());
                return at_age_d(a).nc(STAND);
            }
            if id.starts_with("pryor_freegrowth") {
                return at_age_d(age.unwrap()).calib().warn("cible de calage du génome (borne basse d'un arbre isolé selon sources.md)");
            }
            if id == "pretzsch_dbh_70y_condition" {
                return at_age_d(70).warn("arbres d'avenir fortement éclaircis : proches d'un arbre libre, pas identiques");
            }
            if conditions_text(e).contains("maximum mai") {
                return mature_d().nc("table de production de plantation (DBH à l'âge du MAI maximal, non donné dans l'entrée) : le moteur simule un arbre isolé");
            }
            if e.spread_kind == "max_record" {
                let m = life_d();
                return if id == "atlas_dbh_max" { m.calib() } else { m };
            }
            if age.is_none() && maximum_condition(e) {
                return life_d().warn("l'entrée annonce un maximum : lu au maximum de vie simulé, pas à l'âge de référence");
            }
            if let Some(a) = age {
                return at_age_d(a.min(sp.last_age()));
            }
            if stand_condition(e) {
                return mature_d().nc(STAND);
            }
            mature_d()
        }
        "crown_width_m" => {
            match id {
                "pryor_crown_dbh25" | "pryor_crown_dbh50" => {
                    let d = if id.ends_with("25") { 25 } else { 50 };
                    // Crown width when the DBH first reaches the threshold.
                    let v: Vec<f64> = sp
                        .runs
                        .iter()
                        .filter_map(|r| r.years.iter().find(|y| y.dbh_cm >= d as f64).map(|y| 2.0 * y.crown_radius_m))
                        .collect();
                    Measured::values(v, format!("largeur de houppier quand le DBH atteint {d} cm"), Nature::Emergent).warn("régression sur arbres de peuplement ET libres")
                }
                "stoj_crown_dominant_59y" => Measured::values(sp.year(59, |y| 2.0 * y.crown_radius_m), "largeur de houppier à 59 ans", Nature::Emergent).nc(STAND),
                _ => {
                    let a = age.unwrap_or(mat);
                    let m = Measured::values(sp.year(a, |y| 2.0 * y.crown_radius_m), format!("largeur de houppier à {a} ans"), Nature::Emergent);
                    if stand_condition(e) {
                        m.nc(STAND)
                    } else {
                        m
                    }
                }
            }
        }
        "ring_width_mm" => match id {
            "tomczak_ring_width_mean" => Measured::values(sp.ring_mean(1, 144), "cerne moyen à 1,3 m, âges cambiaux 1 à 144", Nature::Emergent).nc(STAND),
            "germ008_ring_all" => Measured::values(sp.ring_mean(1, 140), "cerne moyen à 1,3 m, âges cambiaux 1 à 140", Nature::Emergent).nc(STAND).warn("série forestière ; écart-type de la série saisi en sd"),
            "wp_nl_ring_range" => Measured::values(sp.ring_mean(1, sp.last_age()), "cerne moyen à 1,3 m sur la vie simulée", Nature::Emergent),
            "albrecht_ring" => Measured::values(sp.ring_mean(1, 60), "cerne moyen à 1,3 m, âges cambiaux 1 à 60", Nature::Emergent),
            "pryor_ring_veneer" => Measured::values(sp.ring_mean(1, 60), "cerne moyen à 1,3 m, âges cambiaux 1 à 60", Nature::Emergent).nc("exigence commerciale du placage, pas une mesure de croissance"),
            _ if id.starts_with("germ008_ring_") => {
                let a = age.unwrap_or(10);
                let (lo, hi) = (a.saturating_sub(9).max(1), a + 10);
                Measured::values(sp.ring_mean(lo, hi), format!("cerne moyen à 1,3 m, âges cambiaux {lo} à {hi}"), Nature::Emergent).nc(STAND)
            }
            _ => Measured::not_measurable("entrée de cerne non reliée"),
        },
        "aboveground_biomass_t" => {
            let (d, what) = match id {
                "cienciala_agb_D30" => (30, "biomasse ligneuse sèche au DBH 30 cm"),
                "cienciala_agb_D40" => (40, "biomasse ligneuse sèche au DBH 40 cm"),
                "cienciala_agb_D50" => (50, "biomasse ligneuse sèche au DBH 50 cm"),
                _ => (25, "biomasse ligneuse sèche au DBH 25 cm (échantillon : D 26 cm, H 21 m)"),
            };
            Measured::values(sp.at_dbh(d, |a| a.agb_dry_t), what, Nature::Emergent)
                .nc("équation d'arbres de peuplement : un arbre isolé porte plus de branches au même DBH")
                .warn("masse verte du moteur / densité verte x densité basale du génome")
        }
        "root_depth_m" => match id {
            "wp_pl_root_depth" => Measured::values(sp.year(40, |y| y.root_depth_m), "profondeur racinaire à 40 ans", Nature::Emergent),
            "lucot_bruckert_surface_root" => Measured::values(sp.year(150, |y| y.root_depth_m), "profondeur racinaire maximale à 150 ans", Nature::Emergent)
                .nc("profondeur du seul système « intensif » de surface ; le moteur donne la profondeur maximale"),
            "stoj_rootdepth_erlbeck" => Measured::values(sp.lifetime_max(|y| y.root_depth_m), "profondeur racinaire maximale sur la vie (sol de référence)", Nature::Emergent).warn("source de seconde main ; sol « facilement pénétrable » vs sol brun de référence"),
            _ => Measured::not_measurable("qualitatif, sans profondeur chiffrée"),
        },
        "root_spread_m" => {
            let a = 150.min(sp.last_age());
            let m = Measured::values(sp.year(a, |y| y.root_spread_m), format!("rayon d'extension racinaire à {a} ans"), Nature::Emergent);
            if stand_condition(e) {
                m.nc("racines mesurées en peuplement (voisins) : le moteur simule un arbre isolé")
            } else {
                m
            }
        }
        "budburst_doy" => {
            let m = Measured::values(sp.mean_date(5, |y| y.budburst), "date moyenne de débourrement (jour, ans 5 et +), climat de Rennes", Nature::Emergent);
            if id.starts_with("eaton") || id.starts_with("cole") || id.starts_with("quick") || id == "wp_nl_budburst" {
                return m.warn("site plus frais que Rennes (Angleterre / Pays-Bas) ; année précise contre moyenne climatique");
            }
            m
        }
        "leaf_fall_doy" => match id {
            "quick_first_tint" => Measured::values(sp.mean_date(5, |y| y.colouring), "début de la coloration (jour moyen)", Nature::Emergent),
            "quick_first_bare" => Measured::values(sp.mean_date(5, |y| y.bare), "arbre nu (jour moyen)", Nature::Emergent),
            _ => {
                let bare = median(&sp.mean_date(5, |y| y.bare));
                Measured::values(sp.mean_date(5, |y| y.fall_start), "fin de la coloration / début de chute (jour moyen) ; stade 4 « couleur ou chute » lu ainsi", Nature::Emergent)
                    .warn(format!("arbre nu en moyenne au jour {bare:.0} ; Angleterre du Sud, année précise"))
            }
        },
        "flowering_doy" => {
            let m = Measured::values(sp.mean_date(1, |y| y.bloom_full), "pleine floraison (30 % de la floraison), jour moyen des années fleuries", Nature::Emergent);
            if id == "atlas_flowering" {
                return m.calib();
            }
            m
        }
        "flowering_duration_days" => {
            let v: Vec<f64> = sp
                .runs
                .iter()
                .map(|r| {
                    let d: Vec<f64> = r.years.iter().filter(|y| y.bloom_days > 0).map(|y| y.bloom_days as f64).collect();
                    d.iter().sum::<f64>() / d.len().max(1) as f64
                })
                .collect();
            Measured::values(v, "durée moyenne de la floraison (ouverture -> chute des pétales)", Nature::Emergent).calib()
        }
        "fruit_maturity_doy" => {
            let m = Measured::values(sp.mean_date(1, |y| y.ripe), "maturité des fruits (jour moyen)", Nature::Emergent);
            match id {
                "wpfr_fruit_cultivars" => m.nc("variétés fruitières greffées"),
                "wppl_fruit" => m.warn("formes sauvages et cultivées mêlées"),
                "wp_nl_acorn_fall" => m.warn("chute des glands, lue comme maturité"),
                _ => m,
            }
        }
        "age_first_flowering_years" => {
            let m = Measured::values(sp.first_flowering(), "âge de la première floraison simulée", Nature::Emergent)
                .warn(format!("piloté par le paramètre first_flowering_age_years = {} (± 1 an)", g.reproduction.first_flowering_age_years));
            let ctext = conditions_text(e);
            if stand_condition(e) {
                return m.nc(STAND);
            }
            if ["rejets", "sprout", "coppice", "taillis"].iter().any(|k| ctext.contains(k)) {
                return m.nc("arbres issus de rejets de souche : le moteur simule un semis");
            }
            match id {
                "wp_pl_first_fruit_stand" | "stoj_maturity" => m.nc(STAND),
                "euf_first_flowering" | "pryor_first_flowering" | "atlas_first_fruiting" => m.calib(),
                _ => m,
            }
        }
        "longevity_years" => {
            let ind = ["zizkuv", "kongeegen", "ivenack", "doornenburg", "kvilleken"];
            if ind.iter().any(|k| id.contains(k)) {
                return lifespan().nc(individual);
            }

            // Age found in a stand or a sample of trees, not a longevity of the species.
            if conditions_text(e).contains("stand") && !conditions_text(e).contains("free-standing") {
                return lifespan().nc("âge observé dans un peuplement ou un échantillon, pas la longévité de l'espèce");
            }
            lifespan()
        }
        "girth_cm" => {
            // Last living year of each run: end of the simulation, or the
            // year before the trunk died (dendro_core::vitality).
            let v: Vec<f64> = sp.runs.iter().filter_map(|r| living(r).last().map(|y| y.dbh_cm * std::f64::consts::PI)).collect();
            Measured::values(v, format!("circonférence en fin de vie simulée (≤ {} ans, ou dernière année avant la mort du tronc)", sp.last_age()), Nature::Emergent).nc(individual)
        }
        "lma_g_m2" => {
            let m = Measured::values(sp.at(mat, |a| a.lma_g_m2), "masse surfacique foliaire (constante flux::LMA, la même pour toutes les feuilles)", Nature::Constant);
            if id.ends_with("shade") {
                return m.warn("feuilles d'ombre : le moteur n'a pas de LMA d'ombre");
            }
            m
        }
        "lai" => Measured::values(sp.at(mat, |a| a.lai_crown), "surface foliaire / projection du houppier (constante LAI_FULL)", Nature::Constant).nc("peuplement dépérissant"),
        "transpiration_mm_day" => Measured::values(sp.at(100, |a| a.july_transp_l_day / (std::f64::consts::PI * (a.crown_width_m / 2.0).powi(2))), "transpiration de juillet par m² de projection du houppier (mm/j), 100 ans", Nature::Emergent).nc("peuplement dépérissant"),
        "transpiration_l_day" => {
            let a = age.unwrap_or(sp.transp_age);
            let m = Measured::values(sp.at(a, |r| r.july_transp_l_day), format!("transpiration moyenne de juillet (L/j), {a} ans, moyenne de 5 ans"), Nature::Emergent);
            match id {
                "quick_tree_water_use" => m.nc("chênes de plantation (houppiers plus petits qu'un arbre isolé)"),
                "heatpulse_transp" => m.nc("verger de cultivar greffé, hémisphère sud"),
                _ => m,
            }
        }
        "transpiration_l_day_per_m2_canopy" => Measured::values(
            sp.at(100, |a| a.july_transp_l_day / (std::f64::consts::PI * (a.crown_width_m / 2.0).powi(2))),
            "transpiration de juillet par m² de projection du houppier, 100 ans",
            Nature::Emergent,
        )
        .nc("chênes de plantation ; jours de juillet seulement"),
        "shade_tolerance" | "drought_tolerance" | "waterlogging_tolerance" => {
            let t = &g.tolerances;
            let p = match q {
                "shade_tolerance" => t.shade,
                "drought_tolerance" => t.drought,
                _ => g.roots.waterlogging_tolerance,
            } as f64;
            if e.unit == "0-5 scale" {
                return Measured::one(p * 5.0, format!("paramètre du génome x 5 ({p:.2} sur 0-1)"), Nature::Param)
                    .nc("échelles différentes (0-5 de Niinemets & Valladares, 0-1 du génome) : correspondance non établie")
                    .warn("le fichier dit sd mais la source donne moyenne ± erreur-type");
            }
            if e.unit.starts_with("Ellenberg") {
                return Measured::one(p, "paramètre shade du génome (0-1)", Nature::Param).nc("indice d'Ellenberg (lumière), pas une tolérance");
            }
            // Species the genome declares intolerant of waterlogging: the
            // growth on gley clay must show it (bench rule).
            if q == "waterlogging_tolerance" && g.roots.waterlogging_tolerance < 0.3 {
                return waterlogging_rule(sp);
            }
            if id == "pryor_shade_young" {
                return Measured::not_modeled("pas de couvert concurrent (arbre isolé) : l'ombrage d'un jeune plant n'est pas simulé");
            }
            Measured::not_measurable(format!("énoncé qualitatif sans critère opérationnel (paramètre du génome : {p:.2})"))
        }
        "dry_density_kg_m3" => {
            let m = Measured::one(g.wood.density_kg_m3 as f64, "densité basale du génome (wood.density_kg_m3)", Nature::Param);
            if id == "cienciala_basic_density" {
                return m;
            }
            m.nc("état d'humidité différent (12 % ou non précisé) de la densité basale du génome")
        }
        "green_density_kg_m3" => Measured::one(g.mechanics.green_density_kg_m3 as f64, "densité verte du génome (mécanique)", Nature::Param),
        "mor_mpa" => Measured::one(g.mechanics.green_mor_mpa as f64, "MOR du bois vert (génome)", Nature::Param).nc("bois sec (≈ 12 %) ou de structure contre bois vert du moteur"),
        "moe_gpa" => Measured::one(g.mechanics.green_moe_gpa as f64, "MOE du bois vert (génome)", Nature::Param).nc("bois sec (≈ 12 %) ou de structure contre bois vert du moteur"),
        "critical_wind_ms" if g.leaf.evergreen() => {
            // An evergreen meets winter storms with most of its foliage.
            let a = sp.crit_age;
            Measured::values(sp.at(a, |r| r.first_crit_leafy.unwrap_or(f64::NAN)), format!("première rafale critique, houppier en feuilles (persistant), {a} ans"), Nature::Emergent)
        }
        "critical_wind_ms" => {
            let a = sp.crit_age;
            Measured::values(sp.at(a, |r| r.first_crit_leafless.unwrap_or(f64::NAN)), format!("première rafale critique, houppier défeuillé, {a} ans"), Nature::Emergent)
                .warn("valeur de Virot et al. contestée (Albrecht et al. 2016)")
        }
        "mycorrhiza_type" => {
            // No explicit field in the reference format (value is empty,
            // unit "category"): the expected type is read from the entry's
            // text, ignoring negated mentions ("not ectomycorrhiza") and
            // sources that do not state the type.
            let txt = entry_text(e);
            if ["type not stated", "not stated by", "non précisé", "pas détaillé", "not detailed"].iter().any(|k| txt.contains(k)) && !txt.contains("inferred") {
                return Measured::not_measurable("la source ne donne pas le type de mycorhize (seulement l'existence de la symbiose)");
            }
            let ecto = affirmed(&txt, &["ecto", "ecm"]);
            let arb = affirmed(&txt, &["arbusc", "endomy", "(va)", "am "]);
            let (want, label) = match (ecto, arb) {
                (true, false) => ("Ecto", "ectomycorhize"),
                (false, true) => ("Arbusc", "mycorhize arbusculaire"),
                (true, true) => return Measured::not_measurable("le texte de l'entrée affirme les deux types de mycorhize"),
                (false, false) => return Measured::not_measurable("type mycorhizien non lisible dans l'entrée"),
            };
            let t = format!("{:?}", g.symbiosis.mycorrhiza);
            let m = Measured::rule(t.contains(want), format!("type du génome : {t} ; type attendu lu dans l'entrée : {label}"), "type mycorhizien", Nature::Param);
            if txt.contains("inferred") || txt.contains("déduit") {
                m.warn("type déduit par l'auteur de l'entrée (genre fongique), non énoncé par la source")
            } else {
                m
            }
        }
        "root_architecture" => {
            if id == "wpde_rootsystem" {
                return Measured::not_measurable("« kräftig und weitläufig » : pas de critère opérationnel");
            }
            let txt = entry_text(e);
            let shallow = ["shallow", "flach", "superficiel", "traçant"].iter().any(|k| txt.contains(k));
            let Some(oak) = cx.get("quercus_robur").filter(|o| shallow && o.id != sp.id) else {
                return Measured::not_measurable("architecture racinaire qualitative : pas de critère du banc (seul « superficiel » est comparé au chêne)");
            };
            let c = median(&sp.year(60, |y| y.root_depth_m));
            let o = median(&oak.year(60, |y| y.root_depth_m));
            let name = sp.genome.common_name_fr.to_lowercase();
            Measured::rule(c < o, format!("profondeur à 60 ans : {name} {c:.2} m, chêne {o:.2} m (critère du banc : enracinement moins profond que le chêne)"), "enracinement superficiel relatif", Nature::Emergent)
        }
        "ring_width_trend" => {
            let a = median(&sp.ring_mean(4, 6));
            let b = median(&sp.ring_mean(12, 20));
            Measured::rule(a > b, format!("cerne à 1,3 m, âge cambial 4-6 : {a:.2} mm, 12-20 : {b:.2} mm (critère du banc : déclin)"), "déclin du cerne après 4-5 ans", Nature::Emergent)
        }
        "frost_tolerance" if e.unit == "degC" => {
            let txt = entry_text(e) + " " + &e.id;
            if ["blossom", "flower", "fleur", "bloom", "floraison"].iter().any(|k| txt.contains(k)) {
                Measured::one(g.reproduction.flower_frost_c as f64, "paramètre reproduction.flower_frost_c (gel des fleurs ouvertes)", Nature::Param)
            } else {
                Measured::not_modeled("endurcissement au froid du bois en hiver non modélisé")
            }
        }
        "frost_tolerance" => {
            // Qualitative statement. Only a statement about flowers is read
            // with the flower-frost rule; a statement on seedlings or on
            // frost in general is not a flowering claim.
            let txt = entry_text(e) + " " + &e.id;
            let flowers = ["flower", "fleur", "blossom", "bloom", "floraison", "blüher"].iter().any(|k| txt.contains(k));
            if !flowers {
                if ["seedling", "plantule", "jeunes plants"].iter().any(|k| txt.contains(k)) {
                    return Measured::not_modeled("gel des plantules : pas de régénération simulée (l'énoncé ne porte pas sur les fleurs)");
                }
                return Measured::not_measurable("énoncé général sur le gel (feuilles, bourgeons, pousses), sans critère du banc : seul le gel des fleurs est lu, et l'énoncé ne les concerne pas");
            }
            let v: Vec<f64> = sp
                .runs
                .iter()
                .map(|r| {
                    let fl: Vec<&YearRec> = r.years.iter().filter(|y| y.bloom_start.is_some()).collect();
                    fl.iter().filter(|y| y.flower_frost_loss > 0.0).count() as f64 / fl.len().max(1) as f64
                })
                .collect();
            let f = median(&v);
            Measured::rule(f > 0.0, format!("part des années fleuries avec des fleurs gelées : {:.1} %", 100.0 * f), "gel des fleurs certaines années", Nature::Emergent)
        }
        "dbh_increment_mm_yr" => {
            if id == "stoj_dbh_increment" {
                let v: Vec<f64> = sp.runs.iter().filter_map(|r| Some((living(r).get(58)?.dbh_cm - living(r).get(52)?.dbh_cm) / 6.0 * 10.0)).collect();
                return Measured::values(v, "accroissement du DBH de 53 à 59 ans (mm/an)", Nature::Emergent).nc(STAND);
            }
            let v: Vec<f64> = sp.runs.iter().map(|r| living(r).windows(2).map(|w| (w[1].dbh_cm - w[0].dbh_cm) * 10.0).fold(0.0, f64::max)).collect();
            Measured::values(v, "accroissement annuel maximal du DBH (mm/an)", Nature::Emergent)
        }
        "slenderness_hd" => Measured::values(sp.year(59, |y| 100.0 * y.height_m / y.dbh_cm.max(1e-6)), "H/D à 59 ans", Nature::Emergent).nc(STAND),
        "stem_volume_m3" => {
            let d = if id.contains("_30_") { 30 } else { 40 };
            Measured::values(sp.at_dbh(d, |a| a.stem_vol_7cm_m3), format!("volume du tronc jusqu'à 7 cm de diamètre, au DBH {d} cm"), Nature::Emergent)
                .warn("longueur de grume du moteur différente de celle de la table (12 ou 14 m)")
        }
        "shoot_length_cm" => Measured::one(g.architecture.max_shoot_length_m as f64 * 100.0, "paramètre max_shoot_length_m", Nature::Param),
        "frost_kill_new_foliage_c" => Measured::one(g.tolerances.frost_c as f64, "paramètre tolerances.frost_c", Nature::Param).calib(),
        // Bark beetles and stand mortality (spruce, 2026-11-20): the inputs
        // of `BioticTraits` and the shape of the response; compared off the
        // bench (examples/life_dist.rs, docs/sources.md « Scolytes »).
        "annual_mortality_pct" | "annual_removal_pct" | "bark_beetle_crisis_year_fraction" | "bark_beetle_predisposition_weight" => Measured::not_measurable(
            "donnée d'inventaire ou d'indice de peuplement, entrée du génome (biotic) ; taux de mort par an simulés en années de crise et hors crise : examples/life_dist.rs",
        ),
        "bark_beetle_attacked_stand_pct" | "bark_beetle_successful_attack_fraction" | "bark_beetle_risk_qmd_plateau_cm" | "bark_beetle_host_age_years" => {
            Measured::not_modeled("ni population de scolytes, ni attaques, ni peuplement : seule la mort de l'hôte est tirée (vitality.rs)")
        }
        // Stand-scale quantities.
        "basal_area_m2_ha" | "stem_density_per_ha" | "yield_class_m3_ha_yr" | "mai_m3_ha_yr" | "stand_volume_m3_ha" | "rotation_years" => {
            Measured::not_modeled("grandeur de peuplement ou de gestion : le moteur simule un arbre isolé")
        }
        "storm_damage_fraction" => storm_fraction(sp),
        "shoot_flushes_per_year" => Measured::not_modeled("une seule pousse par an (pas de pousse de la Saint-Jean)"),
        "mast_interval_years" => Measured::not_modeled("pas de cycle de glandée (fructification chaque année)"),
        "acorn_mass_g" | "leaf_thickness_um" | "frost_kill_acorn_c" | "dormant_bud_longevity_years" | "chilling_hours" | "compressive_strength_mpa" | "tensile_strength_mpa"
        | "shrinkage_tangential_pct" | "sapwood_width_cm" | "sapwood_diameter_fraction" | "sapwood_rings" | "bark_thickness_mm" | "bark_area_fraction" | "heart_rot_onset_years"
        | "heart_rot_fraction" | "min_soil_depth_cm" | "sucker_distance_m" | "root_suckering_relative" | "soil_ph_range" | "max_altitude_m" | "gummosis" | "natural_frequency_hz" => {
            if q == "chilling_hours" {
                return Measured::not_modeled("froid compté en jours ≤ 5 °C (Cannell & Smith), pas en heures ; variétés fruitières").nc("variétés fruitières");
            }
            Measured::not_modeled("grandeur ou processus absent du moteur")
        }
        "epicormic_response" | "dead_branch_persistence" | "stump_sprouting" | "pruning_season" | "wound_closure_mm_yr" | "pruning_wound_rot_fraction"
        | "max_occluded_branch_diameter_mm_4y" | "pruning_max_branch_diameter_mm" => {
            Measured::not_measurable("modélisé (taille, plaies, rejets) mais le banc n'a pas encore de protocole d'élagage pour cette entrée")
        }
        // --- Observables introduced with the 20 new species files ---
        "needle_longevity_years" => {
            if g.leaf.evergreen() {
                Measured::one(g.leaf.leaf_longevity_years as f64, "paramètre leaf.leaf_longevity_years (durée de vie moyenne d'une cohorte)", Nature::Param)
            } else {
                Measured::not_modeled("génome caduc : une seule cohorte de feuilles par an")
            }
        }
        "leaf_length_cm" | "needle_length_cm" => Measured::one(g.leaf.length_cm as f64, "paramètre leaf.length_cm (texture du feuillage)", Nature::Param),
        "height_increment_cm_yr" | "height_increment_m_yr" => {
            let k = if q.ends_with("cm_yr") { 100.0 } else { 1.0 };
            let unit = if k > 1.0 { "cm/an" } else { "m/an" };
            let v: Vec<f64> = sp.runs.iter().map(|r| living(r).windows(2).map(|w| (w[1].height_m - w[0].height_m) * k).fold(0.0, f64::max)).collect();
            Measured::values(v, format!("accroissement annuel maximal de la hauteur ({unit}) sur la vie simulée"), Nature::Emergent)
        }
        "basic_density_kg_m3" => Measured::one(g.wood.density_kg_m3 as f64, "densité basale du génome (wood.density_kg_m3)", Nature::Param),
        "oven_dry_density_kg_m3" => Measured::one(g.wood.density_kg_m3 as f64, "densité basale du génome", Nature::Param).nc("densité anhydre contre densité basale"),
        "chilling_requirement_days" => Measured::one(g.phenology.chilling_requirement_days as f64, "paramètre phenology.chilling_requirement_days (jours ≤ 5 °C, Cannell & Smith)", Nature::Param),
        "fruit_mass_g" | "nut_mass_g" => {
            if g.reproduction.fruit_mass_g > 0.0 {
                Measured::one(g.reproduction.fruit_mass_g as f64, "paramètre reproduction.fruit_mass_g (informatif, sans coût carboné)", Nature::Param)
            } else {
                Measured::not_modeled("masse du fruit non renseignée dans le génome")
            }
        }
        "root_fraction_top_10cm" | "root_fraction_top_30cm" | "root_fraction_top_40cm" => {
            let (f, d): (fn(&AgeRec) -> f64, u32) = match q {
                "root_fraction_top_10cm" => (|r| r.root_frac_10, 10),
                "root_fraction_top_30cm" => (|r| r.root_frac_30, 30),
                _ => (|r| r.root_frac_40, 40),
            };
            let a = mat.min(sp.last_age());
            Measured::values(sp.at(a, f), format!("fraction de la longueur racinaire au-dessus de {d} cm, {a} ans"), Nature::Emergent).warn("longueur de racines (moteur) contre biomasse (source)")
        }
        "clear_bole_height_m" => {
            let a = age.unwrap_or(mat).min(sp.last_age());
            Measured::values(sp.at(a, |r| r.clear_bole_m), format!("hauteur du fût sans grosse branche, {a} ans"), Nature::Emergent)
        }
        "cone_length_cm" | "fruit_diameter_cm" | "nut_size_cm" | "catkin_length_cm" | "flower_diameter_cm" | "flowers_per_cluster" => {
            Measured::not_modeled("taille des organes reproducteurs non simulée (seule une carte d'affichage, reproduction.*_cluster_cm)")
        }
        "frost_kill_10pct_c" | "frost_kill_90pct_c" | "frost_hardiness_c" => Measured::not_modeled("endurcissement au froid hivernal non modélisé (seul le gel des jeunes feuilles et des fleurs l'est)"),
        "cone_cycle_years" | "aril_maturity_years" => Measured::not_modeled("cycle de fructification sur une seule saison : pas de cône de deux ans"),
        "fruit_yield_t_per_tree" | "biennial_bearing_ratio" | "yield_comparison" | "fruiting_peak_age_years" | "productive_lifespan_years" | "fruit_set" => {
            Measured::not_modeled("pas de rendement en fruits (affichage seulement, sans coût carboné)")
        }
        "rootstock_vigour" | "size_vs_seedling_pct" => Measured::not_modeled("pas de porte-greffe : l'arbre simulé est franc de pied (semis)"),
        "rain_mm_yr" | "rain_mm_month" | "rain_mm_day_record" | "rain_days_ge_1mm" | "rain_days_ge_5mm" | "rain_days_ge_10mm" | "temp_mean_c" | "temp_min_mean_c"
        | "temp_max_mean_c" | "summer_temp_mean_c" | "record_low_c" | "record_high_c" | "diurnal_range_c" | "hot_days_tx_ge_30" | "hot_days_tx_ge_25"
        | "frost_days" | "frost_days_tx_le_0" | "frost_days_tn_le_m5" | "frost_days_tn_le_m10" | "sunshine_h_yr" | "global_radiation_mj_m2_yr" | "etp_mm_yr"
        | "snow_days" | "wind_mean_ms" | "gust_record_ms" | "gust_days_ge_16ms" | "gust_days_ge_28ms" => {
            Measured::not_measurable("climat d'une station de l'aire de l'espèce : le banc simule le climat de Rennes")
        }
        "damping_ratio" | "f0_vs_d_over_h2" | "f0_winter_gain_pct" | "f0_debranched_vs_intact" | "anchorage_moment_knm" | "tilt_at_critical_moment_deg"
        | "drag_coefficient" | "anchorage_moment_gain_deep_roots_pct" | "thigmomorphogenesis_effect" | "bending_dose_response" | "stem_vs_anchorage_strengthening"
        | "radial_growth_mechanical_control" | "failure_mode" | "leaf_on_damage_effect" | "pruning_crown_mass_threshold_pct" | "leonardo_rule" => {
            Measured::not_modeled("mécanique dynamique ou essai de traction : pas de protocole par espèce dans le banc (voir physics.ron)")
        }
        "storm_damage_predictors" | "storm_damaged_forest_fraction" | "storm_damage_plot_fraction" | "storm_damage_return_time_years" | "uprooted_share_of_damaged_pct"
        | "windthrown_volume_m3" | "forest_destroyed_fraction" | "wind_tolerance" | "wind_exposure_tolerance" | "salt_wind_tolerance" => {
            Measured::not_modeled("dégâts de tempête à l'échelle du peuplement ou du paysage")
        }
        "site_index" | "height_mai_culmination_years" | "height_cai_culmination_years" | "dbh_mai_culmination_years" | "dbh_cai_culmination_years" | "cai_peak_age_years"
        | "exploitability_age_years" | "coppice_rotation_years" | "stems_per_stool" | "height_growth_end_years" => {
            Measured::not_modeled("grandeur de peuplement ou de gestion : le moteur simule un arbre isolé")
        }
        "flowering_date_shift_days" | "flowering_date_sd_days" | "budburst_trend_days_per_yr" | "flowering_elevation_gradient_d_per_100m" | "budburst_rmse_d"
        | "budburst_model_form" | "chill_portions" => Measured::not_measurable("tendance, gradient ou statistique de modèle : pas de protocole du banc"),
        "seed_dormancy_years" | "stratification_days" | "seed_dispersal_m" | "sex_system" | "ploidy" | "clonal_age_years" | "allelopathy_juglone" | "tannin_content_pct"
        | "pathogen_ink_disease" | "pathogen_blight" | "ash_dieback_mortality_fraction" | "heartwood_onset_years" | "bark_thickness_cm" | "sinker_root_onset_years"
        | "litter_g_m2_yr" | "photosynthesis_optimum_c" | "wound_closure" | "marcescence" | "root_shoot_ratio" | "root_beta" | "rew_stress_threshold"
        | "crown_to_stem_ratio" | "crown_diameter_vs_dbh_linear" => Measured::not_modeled("grandeur ou processus absent du moteur, ou sans protocole par espèce"),
        _ => Measured::not_measurable(format!("observable « {q} » non relié par le banc")),
    }
}

/// Spruce minus fir mean height at equal age (paired seeds), the quantity of
/// a yield-table comparison (Utschig 2004). Needs the spruce in the run.
fn height_gap(sp: &SpeciesCtx, cx: &Ctx, age: u32) -> Measured {
    let a = age.min(sp.last_age());
    let Some(spruce) = cx.get("picea_abies").filter(|s| s.id != sp.id) else {
        return Measured::not_measurable("l'épicéa n'est pas dans le run : écart de hauteur non mesurable");
    };
    let v: Vec<f64> = spruce
        .runs
        .iter()
        .zip(sp.runs.iter())
        .filter(|(s, f)| alive_at(s, a) && alive_at(f, a))
        .filter_map(|(s, f)| Some(s.years.get(a as usize - 1)?.height_m - f.years.get(a as usize - 1)?.height_m))
        .collect();
    Measured::values(v, format!("hauteur de l'épicéa moins hauteur du {} à {a} ans (même graine, arbres isolés)", sp.genome.common_name_fr.to_lowercase()), Nature::Emergent)
        .warn("tables de production de peuplement (Wiedemann, Assmann & Franz contre Hausser) comparées à deux arbres isolés ; âge de 100 ans choisi par le banc")
}

/// Cherry on gley clay vs the reference soil (waterlogging intolerance).
fn waterlogging_rule(sp: &SpeciesCtx) -> Measured {
    let r: Vec<f64> = sp
        .runs
        .iter()
        .zip(sp.gley.iter())
        .map(|(a, b)| b.years.last().map(|y| y.dbh_cm).unwrap_or(0.0) / a.years.get(b.years.len() - 1).map(|y| y.dbh_cm).unwrap_or(1.0).max(1e-6))
        .collect();
    let m = median(&r);
    let n = sp.gley.first().map(|r| r.years.len()).unwrap_or(0);
    Measured::rule(m < 0.7, format!("DBH à {n} ans sur argile à gley / sol de référence = {m:.2} (critère du banc : < 0,7)"), "croissance sur sol engorgé", Nature::Emergent)
}

/// Physics file (climate of Rennes, mechanics, roots, wood).
pub fn physics_entry(cx: &Ctx, e: &Entry) -> Measured {
    let c = &cx.climate;
    let cl = |f: fn(&ClimateStats) -> f64, what: &str| Measured::values(c.iter().map(f).collect(), what.to_string(), Nature::Emergent);
    let oak = cx.oak();
    let id = e.id.as_str();
    match id {
        "rennes_temp_mean_annual" => cl(|s| s.t_mean, "température moyenne annuelle").calib(),
        "rennes_temp_mean_jan" => cl(|s| s.t_month[0], "température moyenne de janvier").calib(),
        "rennes_temp_mean_jul" => cl(|s| s.t_month[6], "température moyenne de juillet").calib(),
        "rennes_temp_mean_aug" => cl(|s| s.t_month[7], "température moyenne d'août").calib(),
        "rennes_temp_max_mean_annual" => cl(|s| s.tx_mean, "Tx moyenne annuelle").calib(),
        "rennes_temp_min_mean_annual" => cl(|s| s.tn_mean, "Tn moyenne annuelle").calib(),
        "rennes_dtr_jan" => cl(|s| s.dtr_month[0], "amplitude Tx-Tn de janvier").calib(),
        "rennes_dtr_jul" => cl(|s| s.dtr_month[6], "amplitude Tx-Tn de juillet").calib(),
        // Tn <= 0, Tx >= 25 and Tx <= 0: monthly counts of the fiche used to fit
        // the standard deviations of the temperature generator (tempgen.rs).
        "rennes_frost_days_tn0" => cl(|s| s.frost_days, "jours Tn ≤ 0 °C par an").calib(),
        "rennes_frost_days_tn_m5" => cl(|s| s.tn_le_m5, "jours Tn ≤ -5 °C par an"),
        "rennes_frost_days_tn_m10" => cl(|s| s.tn_le_m10, "jours Tn ≤ -10 °C par an"),
        "rennes_ice_days_tx0" => cl(|s| s.tx_le_0, "jours Tx ≤ 0 °C par an").calib(),
        "rennes_hot_days_tx30" => cl(|s| s.tx_ge_30, "jours Tx ≥ 30 °C par an"),
        "rennes_summer_days_tx25" => cl(|s| s.tx_ge_25, "jours Tx ≥ 25 °C par an").calib().warn("calé sur les comptages mensuels d'avril-juin et septembre-octobre, pas juillet-août"),
        "rennes_record_high" | "rennes_record_high_wiki" => cl(|s| s.record_tx_80y, "Tx maximale sur 80 ans (médiane des fenêtres de 80 ans)"),
        "rennes_record_high_jun" => cl(|s| s.record_tx_june_80y, "Tx maximale de juin sur 80 ans (médiane des fenêtres)"),
        "rennes_record_low" => {
            let mut m = cl(|s| s.record_tn_80y, "Tn minimale sur 80 ans (médiane des fenêtres)");
            m.mirror = true;
            m
        }
        "rennes_etp_penman_annual" => cl(|s| s.pet_yr, "ETP annuelle (Hargreaves-Samani)").warn("Hargreaves contre Penman ; période 2001-2020").calib(),
        "rennes_rain_annual" => cl(|s| s.rain_yr, "pluie annuelle").calib(),
        "rennes_rain_driest_month" => cl(|s| s.rain_month.iter().copied().fold(f64::MAX, f64::min), "pluie du mois le plus sec (moyenne)"),
        "rennes_rain_wettest_month" => cl(|s| s.rain_month.iter().copied().fold(f64::MIN, f64::max), "pluie du mois le plus humide (moyenne)"),
        "rennes_rain_days_1mm" => cl(|s| s.rain_ge_1, "jours de pluie ≥ 1 mm").calib(),
        "rennes_rain_days_5mm" => cl(|s| s.rain_ge_5, "jours de pluie ≥ 5 mm"),
        "rennes_rain_days_10mm" => cl(|s| s.rain_ge_10, "jours de pluie ≥ 10 mm"),
        "rennes_rain_daily_max_record" => cl(|s| s.record_rain_80y, "pluie quotidienne maximale sur 80 ans (médiane des fenêtres)"),
        "rennes_sunshine_annual" | "rennes_global_radiation_annual" => Measured::not_modeled("pas de rayonnement ni d'insolation générés (seul Ra extraterrestre pour l'ETP)"),
        "rennes_snow_days" => Measured::not_modeled("pas de neige"),
        _ if id.starts_with("rennes_wind") || id.starts_with("rennes_gust") => vent_or(cx, e, "pas de vent quotidien sur main (M1 statique)"),
        "conifers_f0_linear_dbh_h2" | "dfir_f0_linear_dbh_h2_both_states" | "broadleaf_f0_not_cantilever" | "conifers_f0_debranched_higher" | "leaves_lower_f0_damping"
        | "internal_damping_lt_005" | "dfir_damping_unpruned" | "dfir_damping_debranched" | "jackson_damping_summer" | "jackson_damping_winter" | "jackson_f0_winter_gain"
        | "dfir_pruning_80pct_threshold" => vent_or(cx, e, "pas de fréquence propre ni d'amortissement sur main (mécanique statique)"),
        "virot_critical_wind" => Measured::values(oak.at(100, |r| r.first_crit_leafless.unwrap_or(f64::NAN)), "chêne de 100 ans : première rafale critique, houppier défeuillé", Nature::Emergent)
            .warn("valeur contestée (Albrecht et al. 2016)"),
        "albrecht_comment_virot" => Measured::not_measurable("contestation publiée, pas une valeur"),
        "rigid_model_tree_cd" => Measured::values(oak.at(100, |r| r.crown_cd_5ms), "Cd du houppier à 5 m/s (chêne 100 ans ; constant sous RECONF_MIN_MS = 10 m/s)", Nature::Param)
            .warn("modèles rigides en soufflerie, « bon accord avec les arbres naturels à faible vitesse »"),
        "anchorage_root_depth_gain" => Measured::one(
            (1.0 / dendro_core::mechanics::SHALLOW_ROOTING_FACTOR as f64 - 1.0) * 100.0,
            "gain du moment d'ancrage, racines profondes / superficielles (SHALLOW_ROOTING_FACTOR)",
            Nature::Constant,
        )
        .warn("conifères (Nicoll) transposés aux feuillus"),
        "stem_mass_best_predictor" => Measured::rule(true, "moment critique d'ancrage = coefficient x masse de tige (ForestGALES)", "moment d'ancrage ∝ masse de tige", Nature::Construction).warn("épicéa noir ; vrai par construction"),
        "anchorage_sitka_wet" => Measured::not_measurable("effet qualitatif d'une nappe sur la plaque racinaire : protocole non écrit"),
        "sim_curve_tilt_1deg" => Measured::not_modeled("pas d'inclinaison de la souche"),
        "leonardo_rule_eloy" => {
            let v = oak.at(100, |r| r.area_ratio);
            let m = median(&v);
            let ch = cx.cherry().map_or(f64::NAN, |c| median(&c.at(60, |r| r.area_ratio)));
            Measured::rule(
                (0.8..=1.2).contains(&m),
                format!("Σ sections filles / section mère aux fourches (rayon ≥ 1 cm) : chêne 100 ans {m:.3}, merisier 60 ans {ch:.3} (critère de travail 0,8-1,2, non publié)"),
                "règle de Léonard",
                Nature::Emergent,
            )
        }
        "windthrow_age_mode" => {
            let young: Vec<&str> = oak.runs.iter().filter(|r| alive_at(r, 40)).filter_map(|r| r.ages.get(&40).and_then(|a| a.first_mode_leafless)).collect();
            let old: Vec<&str> = oak.runs.iter().filter(|r| alive_at(r, 200)).filter_map(|r| r.ages.get(&200).and_then(|a| a.first_mode_leafless)).collect();
            let ys = young.iter().filter(|m| **m == "stem").count();
            let ou = old.iter().filter(|m| **m == "uproot").count();
            let pass = 2 * ys > young.len() && 2 * ou > old.len();
            Measured::rule(pass, format!("chêne, premier mode défeuillé : 40 ans {young:?}, 200 ans {old:?} (critère : tronc chez les jeunes, déracinement chez les vieux, majorité des graines)"), "mode de rupture selon l'âge", Nature::Emergent)
                .warn("Wikipédia sans valeur chiffrée")
        }
        "lothar_vivian_swiss_plots" | "lothar_vivian_swiss_vivian" | "black_forest_lothar_area" | "lothar_wood_volume_france" | "great_storm_1987_brittany" | "bock_storm_acid_soils" => {
            Measured::not_modeled("dégâts à l'échelle du peuplement ou du paysage")
        }
        "leaf_on_effect_1987" => {
            let a = median(&oak.at(100, |r| r.first_crit_leafy.unwrap_or(f64::NAN)));
            let b = median(&oak.at(100, |r| r.first_crit_leafless.unwrap_or(f64::NAN)));
            Measured::one(a / b, format!("chêne 100 ans : rafale critique feuillé {a:.1} / défeuillé {b:.1} m/s"), Nature::Emergent).nc("affirmation non confirmée, pas de règle testable")
        }
        "thigmo_jaffe1973" | "thigmo_beech_guyed" | "thigmo_beech_stem_vs_roots" | "thigmo_hydrangea_dose" => vent_or(cx, e, "pas d'acclimatation au vent sur main"),
        "wood_oak_density" => Measured::one(oak.genome.wood.density_kg_m3 as f64, "densité basale du génome (chêne)", Nature::Param).nc("densité à 12 % contre densité basale"),
        "wood_oak_mor" => Measured::one(oak.genome.mechanics.green_mor_mpa as f64, "MOR vert du génome (chêne)", Nature::Param).nc("bois sec à 12 % contre bois vert"),
        "wood_oak_moe" => Measured::one(oak.genome.mechanics.green_moe_gpa as f64, "MOE vert du génome (chêne)", Nature::Param).nc("bois sec à 12 % contre bois vert"),
        _ if id.starts_with("wood_") => Measured::not_modeled("espèce absente du moteur (hêtre, frêne)"),
        "root_beta_trees" => {
            let v = oak.at(100, |r| (1.0 - r.root_frac_30).max(1e-9).powf(1.0 / 30.0));
            Measured::values(v, "β de Gale & Grigal tiré de la fraction au-dessus de 30 cm (chêne 100 ans)", Nature::Emergent).warn("longueur de racines (moteur) contre biomasse (Jackson)")
        }
        "root_frac_top10" | "root_frac_top30" | "root_global_profile" => {
            let (f, d): (fn(&AgeRec) -> f64, u32) = match id {
                "root_frac_top10" => (|r| r.root_frac_10, 10),
                "root_frac_top30" => (|r| r.root_frac_30, 30),
                _ => (|r| r.root_frac_40, 40),
            };
            let ch = cx.cherry().map_or(f64::NAN, |c| median(&c.at(60, f)));
            Measured::values(oak.at(100, f), format!("fraction de la longueur racinaire au-dessus de {d} cm, chêne 100 ans (merisier 60 ans : {ch:.2})"), Nature::Emergent)
                .warn("longueur de racines (moteur) contre biomasse (Jackson)")
        }
        "root_shoot_forests" => Measured::one(oak.genome.roots.root_shoot_ratio as f64, "paramètre root_shoot_ratio (part d'allocation, pas un rapport de biomasses)", Nature::Param)
            .nc("le moteur n'expose pas la biomasse racinaire"),
        "rew_threshold_04" => Measured::one(0.45 - 0.1 * oak.genome.tolerances.drought.clamp(0.0, 1.0) as f64, "REW critique du moteur (0,45 - 0,1 x drought, formule de sim.rs)", Nature::Param),
        "cs83_model_form" => Measured::rule(true, "débourrement par T = a + b exp(r C), jours-degrés depuis forcing_start_doy, froid depuis le 1er novembre", "forme du modèle", Nature::Construction),
        "cs83_budburst_dates" => Measured::not_modeled("épicéa de Sitka (espèce absente), dates modélisées").nc("autre espèce"),
        "chuine_budburst_rmse" => Measured::not_measurable("RMSE d'un modèle contre des observations : pas de série observée à Rennes dans le banc"),
        "hemery_linear_range" => {
            let r2: Vec<f64> = oak.runs.iter().map(|r| r2_crown_dbh(&r.years)).collect();
            let m = median(&r2);
            let ch = cx.cherry().map_or(f64::NAN, |c| median(&c.runs.iter().map(|r| r2_crown_dbh(&r.years)).collect::<Vec<_>>()));
            Measured::rule(m > 0.8, format!("r² de la largeur de houppier sur le DBH entre 20 et 50 cm : chêne {m:.3}, merisier {ch:.3} (publié : r² > 0,8)"), "houppier linéaire en DBH", Nature::Emergent)
        }
        "hemery_walnut_crown" | "hemery_walnut_z_ratio" => {
            let v: Vec<f64> = oak.runs.iter().filter_map(|r| r.years.iter().find(|y| y.dbh_cm >= 60.0).map(|y| if id.ends_with("crown") { 2.0 * y.crown_radius_m } else { 200.0 * y.crown_radius_m / y.dbh_cm })).collect();
            Measured::values(v, "chêne au DBH 60 cm (largeur de houppier ou rapport houppier/tige)", Nature::Emergent).nc("noyer, autre espèce")
        }
        _ => Measured::not_measurable(format!("entrée « {} » non reliée par le banc", e.quantity)),
    }
}

fn r2_crown_dbh(ys: &[YearRec]) -> f64 {
    let p: Vec<(f64, f64)> = ys.iter().filter(|y| (20.0..=50.0).contains(&y.dbh_cm)).map(|y| (y.dbh_cm, 2.0 * y.crown_radius_m)).collect();
    let n = p.len() as f64;
    if n < 3.0 {
        return f64::NAN;
    }
    let (mx, my) = (p.iter().map(|a| a.0).sum::<f64>() / n, p.iter().map(|a| a.1).sum::<f64>() / n);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (x, y) in &p {
        sxy += (x - mx) * (y - my);
        sxx += (x - mx).powi(2);
        syy += (y - my).powi(2);
    }
    sxy * sxy / (sxx * syy).max(1e-12)
}


fn storm_fraction(sp: &SpeciesCtx) -> Measured {
    crate::vent::storm_fraction(sp)
}


fn vent_or(cx: &Ctx, e: &Entry, _reason: &str) -> Measured {
    crate::vent::physics_entry(cx, e)
}

#[cfg(test)]
mod tests {
    use super::affirmed;

    #[test]
    fn negated_mycorrhiza_type_is_not_affirmed() {
        let t = "arbuscular mycorrhiza (not ectomycorrhiza)";
        assert!(!affirmed(t, &["ecto", "ecm"]));
        assert!(affirmed(t, &["arbusc"]));
        let t = "ectomycorhize : confirme le type ecm, contre l'arbuscule du merisier.";
        assert!(affirmed(t, &["ecto", "ecm"]));
        assert!(!affirmed(t, &["arbusc"]));
        assert!(!affirmed("non-ectomycorrhizal", &["ecto"]));
        assert!(!affirmed("hector", &["ecto"]));
        assert!(affirmed("ectomycorrhizal with a large number of fungi", &["ecto"]));
    }
}
