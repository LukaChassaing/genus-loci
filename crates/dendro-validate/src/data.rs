//! Reference files `data/validation/*.ron` (format: `data/validation/FORMAT.md`).

use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;

#[derive(Clone, Debug, Deserialize)]
pub struct Conditions {
    pub age_years: Option<f64>,
    pub site: Option<String>,
    pub extra: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Entry {
    pub id: String,
    pub quantity: String,
    pub conditions: Conditions,
    pub value: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub unit: String,
    pub spread_kind: String,
    pub source: String,
    pub url: Option<String>,
    pub accessed: String,
    pub source_kind: String,
    pub read: String,
    pub language: String,
    pub quote: Option<String>,
    pub note: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RefFile {
    pub subject: String,
    pub entries: Vec<Entry>,
}

pub const SPREAD_KINDS: &[&str] = &["range", "sd", "ci95", "quantiles", "single", "max_record"];
pub const SOURCE_KINDS: &[&str] = &["primary", "review", "handbook", "wikipedia", "secondary"];
pub const READ_KINDS: &[&str] = &["full", "abstract", "table", "via_wikipedia", "snippet"];

/// Units in use in the reference files. A new unit must be added here (and
/// its meaning made clear in FORMAT.md) so that a typo is caught.
pub const KNOWN_UNITS: &[&str] = &[
    "-", "%", "% crown mass", "0-5 scale", "Ellenberg L (1-9)", "GPa", "Hz", "L/day", "L/day/m2", "MJ/m2/yr", "MPa", "bendings/day",
    "category", "cm", "cm DBH", "contestation", "d", "day of year", "days", "days/yr", "degC", "deg", "doy", "flushes", "fraction",
    "fraction of plots", "fraction of sample trees", "g", "g/m2", "h/yr", "hours", "kg/m3", "l/day", "m", "m/s", "m2/ha", "m2/m2", "m3",
    "m3/ha", "m3/ha/yr", "mm", "mm/day", "mm/yr", "pH", "qualitative", "ratio", "relation (pente non obtenue)", "relation qualitative",
    "rings", "t", "trees/ha", "um", "years",
];

pub fn load(path: &Path) -> Result<RefFile, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    ron::from_str::<RefFile>(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Structural checks; returns the list of problems (empty if the file is valid).
pub fn check(f: &RefFile) -> Vec<String> {
    let mut errs = Vec::new();
    let mut ids = HashSet::new();
    for e in &f.entries {
        let id = &e.id;
        if !ids.insert(id.clone()) {
            errs.push(format!("{id}: duplicate id"));
        }
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            errs.push(format!("{id}: id has characters outside [A-Za-z0-9_]"));
        }
        if !SPREAD_KINDS.contains(&e.spread_kind.as_str()) {
            errs.push(format!("{id}: unknown spread_kind {}", e.spread_kind));
        }
        if !SOURCE_KINDS.contains(&e.source_kind.as_str()) {
            errs.push(format!("{id}: unknown source_kind {}", e.source_kind));
        }
        if !READ_KINDS.contains(&e.read.as_str()) {
            errs.push(format!("{id}: unknown read {}", e.read));
        }
        if !KNOWN_UNITS.contains(&e.unit.as_str()) {
            errs.push(format!("{id}: unknown unit {:?}", e.unit));
        }
        for v in [e.value, e.min, e.max].into_iter().flatten() {
            if !v.is_finite() {
                errs.push(format!("{id}: non-finite number"));
            }
        }
        if let (Some(a), Some(b)) = (e.min, e.max) {
            if a > b {
                errs.push(format!("{id}: min {a} > max {b}"));
            }
        }
        if let (Some(v), Some(a)) = (e.value, e.min) {
            if v < a {
                errs.push(format!("{id}: value {v} < min {a}"));
            }
        }
        if let (Some(v), Some(b)) = (e.value, e.max) {
            if v > b {
                errs.push(format!("{id}: value {v} > max {b}"));
            }
        }
        if e.source.trim().is_empty() {
            errs.push(format!("{id}: empty source"));
        }
        if e.quantity.trim().is_empty() {
            errs.push(format!("{id}: empty quantity"));
        }
    }
    errs
}
