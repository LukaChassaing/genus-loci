//! Verdict of one entry, from the published spread only (`spread_kind`).

use crate::data::Entry;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cat {
    Ok,
    Limit,
    Out,
    /// `single`: deviation reported, no tolerance published.
    Indicative,
    /// Conditions differ (stand, other species, wood state…).
    NotComparable,
    /// The engine has no such process or output.
    NotModeled,
    /// Modelled (or partly) but no measurement protocol in the bench, or
    /// a qualitative statement without an operational criterion.
    NotMeasurable,
    /// `absence_*` entries: they only record what was not found.
    Absence,
}

impl Cat {
    pub fn icon(self) -> &'static str {
        match self {
            Cat::Ok => "✅",
            Cat::Limit => "⚠️",
            Cat::Out => "❌",
            Cat::Indicative => "ℹ️",
            Cat::NotComparable => "➖",
            Cat::NotModeled => "⬜",
            Cat::NotMeasurable => "◻️",
            Cat::Absence => "∅",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Cat::Ok => "conforme",
            Cat::Limit => "limite",
            Cat::Out => "hors plage",
            Cat::Indicative => "indicatif",
            Cat::NotComparable => "non comparable",
            Cat::NotModeled => "non modélisé",
            Cat::NotMeasurable => "non mesurable",
            Cat::Absence => "entrée d'absence",
        }
    }
    pub const ALL: [Cat; 8] = [Cat::Ok, Cat::Limit, Cat::Out, Cat::Indicative, Cat::NotComparable, Cat::NotModeled, Cat::NotMeasurable, Cat::Absence];
    /// Counts in the conformity rate.
    pub fn judged(self) -> bool {
        matches!(self, Cat::Ok | Cat::Limit | Cat::Out)
    }
}

/// Nature of the engine value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nature {
    /// Emerges from the simulation.
    Emergent,
    /// A genome parameter read as such (the check is on the parameter).
    Param,
    /// A constant of the engine code.
    Constant,
    /// True by construction of the model.
    Construction,
}

impl Nature {
    pub fn label(self) -> &'static str {
        match self {
            Nature::Emergent => "simulé",
            Nature::Param => "paramètre",
            Nature::Constant => "constante",
            Nature::Construction => "par construction",
        }
    }
}

/// What the engine gives for an entry.
#[derive(Clone, Debug)]
pub enum Engine {
    /// One value per seed (or a single value for parameters / constants).
    Values(Vec<f64>),
    /// Qualitative rule checked on the engine.
    Rule { pass: bool, detail: String },
    None,
}

#[derive(Clone, Debug)]
pub struct Measured {
    pub engine: Engine,
    /// What was measured (French, short).
    pub what: String,
    pub nature: Nature,
    /// Some(reason): conditions differ, no verdict (value still shown).
    pub not_comparable: Option<String>,
    /// Forced category (not modelled / not measurable) with a reason.
    pub forced: Option<(Cat, String)>,
    /// The engine was tuned on this very source: a calibration check.
    pub calibration: bool,
    pub warn: Option<String>,
    /// Compare `-x` with the negated entry (a cold record: lower is more extreme).
    pub mirror: bool,
    /// One value per seed (false for a single aggregate value).
    pub per_seed: bool,
}

impl Measured {
    pub fn values(v: Vec<f64>, what: impl Into<String>, nature: Nature) -> Self {
        Measured { engine: Engine::Values(v), what: what.into(), nature, not_comparable: None, forced: None, calibration: false, warn: None, mirror: false, per_seed: true }
    }
    pub fn one(v: f64, what: impl Into<String>, nature: Nature) -> Self {
        let mut m = Self::values(vec![v], what, nature);
        m.per_seed = false;
        m
    }
    pub fn rule(pass: bool, detail: impl Into<String>, what: impl Into<String>, nature: Nature) -> Self {
        Measured { engine: Engine::Rule { pass, detail: detail.into() }, what: what.into(), nature, not_comparable: None, forced: None, calibration: false, warn: None, mirror: false, per_seed: true }
    }
    pub fn forced(cat: Cat, reason: impl Into<String>) -> Self {
        Measured { engine: Engine::None, what: String::new(), nature: Nature::Emergent, not_comparable: None, forced: Some((cat, reason.into())), calibration: false, warn: None, mirror: false, per_seed: true }
    }
    pub fn not_modeled(reason: impl Into<String>) -> Self {
        Self::forced(Cat::NotModeled, reason)
    }
    pub fn not_measurable(reason: impl Into<String>) -> Self {
        Self::forced(Cat::NotMeasurable, reason)
    }
    /// Conditions differ: no verdict, the engine value is shown.
    pub fn nc(mut self, reason: impl Into<String>) -> Self {
        self.not_comparable = Some(reason.into());
        self
    }
    pub fn calib(mut self) -> Self {
        self.calibration = true;
        self
    }
    pub fn warn(mut self, w: impl Into<String>) -> Self {
        self.warn = Some(w.into());
        self
    }
}

pub fn median(v: &[f64]) -> f64 {
    let mut v: Vec<f64> = v.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

/// Verdict and a short explanation; `dev` is the signed relative deviation
/// from the published reference point (value, or band centre), when defined.
#[derive(Clone, Debug)]
pub struct Verdict {
    pub cat: Cat,
    pub why: String,
    pub dev: Option<f64>,
    /// Distance outside the band in units of the band half-width (0 inside).
    pub excess: Option<f64>,
}

/// Published band `[lo, hi]` (either side may be open) per `spread_kind`.
pub fn band(e: &Entry) -> (Option<f64>, Option<f64>) {
    match e.spread_kind.as_str() {
        "sd" => match (e.value, e.min, e.max) {
            (Some(v), Some(a), Some(b)) => {
                let sd = 0.5 * (b - a);
                (Some(v - 2.0 * sd), Some(v + 2.0 * sd))
            }
            _ => (None, None),
        },
        _ => (e.min, e.max),
    }
}

/// Reference value of a `max_record` entry: the record (upper) and the
/// lower bound of its class when given ("over 40 m").
fn record(e: &Entry) -> Option<f64> {
    e.value.or(e.max).or(e.min)
}

/// Bench conventions for records (not published tolerances, stated in the report).
pub const RECORD_EXCEED: f64 = 1.10;
pub const RECORD_FAR_BELOW: f64 = 0.5;

pub fn judge(e: &Entry, m: &Measured) -> Verdict {
    if m.mirror {
        let mut e2 = e.clone();
        e2.value = e.value.map(|v| -v);
        e2.min = e.max.map(|v| -v);
        e2.max = e.min.map(|v| -v);
        let mut m2 = m.clone();
        m2.mirror = false;
        if let Engine::Values(v) = &mut m2.engine {
            v.iter_mut().for_each(|x| *x = -*x);
        }
        let mut out = judge(&e2, &m2);
        out.dev = out.dev.map(|d| -d);
        return out;
    }
    if e.id.starts_with("absence_") {
        return Verdict { cat: Cat::Absence, why: "entrée d'absence (rien de publié trouvé)".into(), dev: None, excess: None };
    }
    if let Some((c, r)) = &m.forced {
        return Verdict { cat: *c, why: r.clone(), dev: None, excess: None };
    }
    let x = match &m.engine {
        Engine::Values(v) if !v.is_empty() => median(v),
        Engine::Rule { pass, detail } => {
            if let Some(r) = &m.not_comparable {
                return Verdict { cat: Cat::NotComparable, why: format!("{r} ; règle : {detail}"), dev: None, excess: None };
            }
            let cat = if *pass { Cat::Ok } else { Cat::Out };
            return Verdict { cat, why: detail.clone(), dev: None, excess: None };
        }
        _ => return Verdict { cat: Cat::NotMeasurable, why: "aucune valeur moteur".into(), dev: None, excess: None },
    };
    let reference = e.value.or_else(|| match (e.min, e.max) {
        (Some(a), Some(b)) => Some(0.5 * (a + b)),
        (a, b) => a.or(b),
    });
    let dev = reference.filter(|r| r.abs() > 1e-12).map(|r| (x - r) / r.abs());
    if let Some(r) = &m.not_comparable {
        return Verdict { cat: Cat::NotComparable, why: r.clone(), dev, excess: None };
    }
    if !x.is_finite() {
        return Verdict { cat: Cat::NotMeasurable, why: "mesure indisponible (événement jamais observé)".into(), dev, excess: None };
    }
    match e.spread_kind.as_str() {
        "max_record" => {
            let Some(rec) = record(e) else {
                return Verdict { cat: Cat::NotMeasurable, why: "record sans valeur".into(), dev, excess: None };
            };
            // A record given as a range (atlas_dbh_max 90-120): upper side
            // for exceeding, lower side for "far below".
            let top = e.max.unwrap_or(rec).max(rec);
            let low = e.min.unwrap_or(rec).min(rec);
            let dev = Some((x - top) / top.abs());
            if x > RECORD_EXCEED * top {
                Verdict { cat: Cat::Out, why: format!("dépasse le record de plus de {:.0} %", (RECORD_EXCEED - 1.0) * 100.0), dev, excess: Some((x / top - 1.0) / (RECORD_EXCEED - 1.0)) }
            } else if x > top {
                Verdict { cat: Cat::Limit, why: "au-dessus du record, de moins de 10 %".into(), dev, excess: Some(0.5) }
            } else if x < RECORD_FAR_BELOW * low {
                Verdict { cat: Cat::Limit, why: format!("très en dessous du record (< {:.0} %)", RECORD_FAR_BELOW * 100.0), dev, excess: Some(0.5) }
            } else {
                Verdict { cat: Cat::Ok, why: "sous le record, du même ordre".into(), dev, excess: Some(0.0) }
            }
        }
        "single" if e.min.is_none() && e.max.is_none() => Verdict { cat: Cat::Indicative, why: "valeur unique publiée : écart rapporté".into(), dev, excess: None },
        "sd" if e.min.is_none() || e.max.is_none() => Verdict { cat: Cat::Indicative, why: "écart-type non saisi : traité comme valeur unique".into(), dev, excess: None },
        _ => {
            // range / ci95 / quantiles / sd (band at ±2 sd) / single with one bound.
            let (lo, hi) = band(e);
            match (lo, hi) {
                (Some(a), Some(b)) => {
                    let c = 0.5 * (a + b);
                    let h = (0.5 * (b - a)).max(1e-12);
                    let d = (x - c).abs();
                    let excess = ((d - h) / h).max(0.0);
                    let kind = if e.spread_kind == "sd" { "±2 sd" } else { "plage" };
                    if d <= h {
                        Verdict { cat: Cat::Ok, why: format!("dans la {kind} publiée"), dev, excess: Some(0.0) }
                    } else if d <= 1.5 * h {
                        Verdict { cat: Cat::Limit, why: format!("hors {kind}, dans 1,5 fois sa demi-largeur"), dev, excess: Some(excess) }
                    } else {
                        Verdict { cat: Cat::Out, why: format!("hors {kind} ({:.1} demi-largeurs au-delà du bord)", excess), dev, excess: Some(excess) }
                    }
                }
                (Some(a), None) => one_sided(x, a, true, dev),
                (None, Some(b)) => one_sided(x, b, false, dev),
                (None, None) => Verdict { cat: Cat::Indicative, why: "pas de bornes publiées".into(), dev, excess: None },
            }
        }
    }
}

/// One-sided bound: conforme on the right side; "limite" within a factor
/// 1.5 of the bound (bench convention, stated in the report).
fn one_sided(x: f64, bound: f64, lower: bool, dev: Option<f64>) -> Verdict {
    let ok = if lower { x >= bound } else { x <= bound };
    let near = if lower { x >= bound / 1.5 } else { x <= bound * 1.5 };
    let side = if lower { "≥" } else { "≤" };
    let excess = if ok { 0.0 } else { ((x - bound).abs() / (0.5 * bound.abs())).max(0.0) };
    if ok {
        Verdict { cat: Cat::Ok, why: format!("borne {side} {bound} respectée"), dev, excess: Some(0.0) }
    } else if near {
        Verdict { cat: Cat::Limit, why: format!("borne {side} {bound} manquée, à moins d'un facteur 1,5"), dev, excess: Some(excess) }
    } else {
        Verdict { cat: Cat::Out, why: format!("borne {side} {bound} manquée de plus d'un facteur 1,5"), dev, excess: Some(excess) }
    }
}

/// Confidence weight of an entry: source kind x reading status.
pub fn weight(e: &Entry) -> f64 {
    let s = match e.source_kind.as_str() {
        "primary" => 1.0,
        "review" => 0.9,
        "handbook" => 0.8,
        "secondary" => 0.6,
        "wikipedia" => 0.4,
        _ => 0.5,
    };
    let r = match e.read.as_str() {
        "full" | "table" => 1.0,
        "abstract" => 0.7,
        "via_wikipedia" => 0.6,
        "snippet" => 0.5,
        _ => 0.5,
    };
    s * r
}

pub fn stars(w: f64) -> &'static str {
    if w >= 0.85 {
        "●●●"
    } else if w >= 0.5 {
        "●●○"
    } else {
        "●○○"
    }
}
