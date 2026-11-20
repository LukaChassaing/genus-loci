//! Saving and loading a tree.
//!
//! A save is not a dump of the simulation state: it is the *recipe* that
//! rebuilds it — species, seed, climate, soil, the replayable intervention
//! log ([`LogEvent`]s: cuts and drought toggles) and the current day. The
//! simulation is deterministic (same inputs and call sequence => same state
//! on the same build), so loading re-simulates from germination ([`Replay`],
//! about 1 to 2 s per simulated century) and lands on the saved state. The
//! metadata carries a [`Simulation::state_hash`] taken when saving, so a
//! loader can tell whether the replay reproduced the saved tree (it will not
//! when the engine's calibration changed between the two builds: the file
//! is still valid, the tree is regrown under the new rules).
//!
//! Format: RON, versioned. Version 2 (this build) adds the player's storms
//! to the log (`SavedAction::Storm`); version 1 files (no storms) are still
//! read. Unknown versions and unknown species / soils are reported as
//! [`SaveError`]s, never panics. A v1 file replayed by this build grows
//! under the new rules (daily wind, breakage, acclimation): its saved hash
//! will not match, as after any calibration change.
//!
//! ```no_run
//! use dendro_core::{save::SaveFile, Genome, Simulation};
//! let mut sim = Simulation::new(Genome::quercus_robur(), 7);
//! sim.advance_years(20);
//! let file = SaveFile::capture(&sim, "Mon chêne", 0);
//! let text = file.to_ron();
//! let back = SaveFile::from_ron(&text).unwrap().replay().unwrap();
//! assert_eq!(back.state_hash(), sim.state_hash());
//! ```

use crate::climate::ClimateParams;
use crate::genome::Genome;
use crate::pruning::{Action, CutSpec, LogEvent};
use crate::sim::Simulation;
use crate::soil::SoilProfile;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

/// Version of the save format written by this build.
pub const SAVE_VERSION: u32 = 2;
/// Oldest version this build still reads.
pub const SAVE_VERSION_MIN: u32 = 1;

/// Longest history accepted from a file (years): a guard against corrupt
/// days that would make the replay run for hours.
const MAX_YEARS: u64 = 2000;

/// Why a save could not be read or replayed. `Display` gives a French
/// message meant for the player.
#[derive(Clone, Debug, PartialEq)]
pub enum SaveError {
    /// The file could not be read or written.
    Io(String),
    /// Not a valid RON save.
    Parse(String),
    /// Written by another version of the format.
    UnsupportedVersion(u32),
    /// The species is not known to this build.
    UnknownSpecies(String),
    /// The soil preset is not known to this build.
    UnknownSoil(String),
    /// Well-formed but inconsistent content (unsorted log, absurd day...).
    Invalid(String),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::Io(e) => write!(f, "fichier illisible ou inaccessible ({e})"),
            SaveError::Parse(e) => write!(f, "sauvegarde corrompue ({e})"),
            SaveError::UnsupportedVersion(v) => {
                write!(f, "format de sauvegarde v{v} non pris en charge (cette version lit les v{SAVE_VERSION_MIN} à v{SAVE_VERSION})")
            }
            SaveError::UnknownSpecies(s) => write!(f, "espèce inconnue : « {s} »"),
            SaveError::UnknownSoil(s) => write!(f, "sol inconnu : « {s} »"),
            SaveError::Invalid(e) => write!(f, "sauvegarde incohérente ({e})"),
        }
    }
}

impl std::error::Error for SaveError {}

/// The species of a saved tree: a built-in species by its scientific name.
/// `genome` is reserved for embedding a full (e.g. bred or modded) genome
/// later; when present it takes precedence over the name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpeciesRef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genome: Option<Genome>,
}

/// One intervention of the log, in the file's own (stable) vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum SavedAction {
    /// Cut of node `node` (stable id) at `t` along it, flush or at the collar.
    Cut { node: u32, t: f32, flush: bool },
    /// Forced drought switched on / off.
    Drought(bool),
    /// A storm asked for by the player: gust at 10 m (m/s). Format v2.
    Storm { gust_ms: f32 },
}

/// A logged intervention: applied at the start of day `day`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedEvent {
    pub day: u64,
    pub action: SavedAction,
}

impl From<&LogEvent> for SavedEvent {
    fn from(e: &LogEvent) -> Self {
        let action = match e.action {
            Action::Cut(c) => SavedAction::Cut { node: c.node_id, t: c.t, flush: c.flush },
            Action::Drought(on) => SavedAction::Drought(on),
            Action::Storm(g) => SavedAction::Storm { gust_ms: g },
        };
        SavedEvent { day: e.day, action }
    }
}

impl From<&SavedEvent> for LogEvent {
    fn from(e: &SavedEvent) -> Self {
        let action = match e.action {
            SavedAction::Cut { node, t, flush } => Action::Cut(CutSpec { node_id: node, t, flush }),
            SavedAction::Drought(on) => Action::Drought(on),
            SavedAction::Storm { gust_ms } => Action::Storm(gust_ms),
        };
        LogEvent { day: e.day, action }
    }
}

/// Information for the save browser; not needed to rebuild the tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SaveMeta {
    /// Name given by the player.
    pub name: String,
    /// When the file was written (Unix seconds, UTC).
    pub written_unix: i64,
    pub age_years: f32,
    pub height_m: f32,
    pub dbh_cm: f32,
    /// Cuts in the log.
    pub cuts: u32,
    /// [`Simulation::state_hash`] at save time, 16 hex digits.
    pub state_hash: String,
}

/// A saved tree (format v2; v1 is the same without storms).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SaveFile {
    pub version: u32,
    pub meta: SaveMeta,
    pub species: SpeciesRef,
    pub seed: u64,
    pub climate: ClimateParams,
    /// Soil preset name (`SoilProfile::presets()`).
    pub soil: String,
    /// Days simulated since germination.
    pub day: u64,
    /// The whole intervention log, oldest first. After a rewind it may
    /// hold events later than `day` (re-applied when their day comes).
    pub events: Vec<SavedEvent>,
}

/// Only the version, read first so that another format fails cleanly.
#[derive(Deserialize)]
struct VersionProbe {
    version: u32,
}

impl SaveFile {
    /// The recipe of `sim` as it is now. `written_unix`: current time
    /// (Unix seconds) supplied by the caller.
    pub fn capture(sim: &Simulation, name: &str, written_unix: i64) -> SaveFile {
        let log = sim.prune_log();
        let cuts = log.iter().filter(|e| matches!(e.action, Action::Cut(_))).count() as u32;
        SaveFile {
            version: SAVE_VERSION,
            meta: SaveMeta {
                name: name.to_string(),
                written_unix,
                age_years: sim.age_years(),
                height_m: sim.height_m(),
                dbh_cm: sim.dbh_cm(),
                cuts,
                state_hash: format!("{:016x}", sim.state_hash()),
            },
            species: SpeciesRef { name: sim.genome().species.clone(), genome: None },
            seed: sim.seed(),
            climate: sim.climate_params().clone(),
            soil: sim.soil_profile().name.clone(),
            day: sim.days_elapsed(),
            events: log.iter().map(SavedEvent::from).collect(),
        }
    }

    /// Pretty RON text.
    pub fn to_ron(&self) -> String {
        let cfg = ron::ser::PrettyConfig::new().depth_limit(3).indentor("  ".to_string());
        // Serialising plain data with no maps cannot fail.
        ron::ser::to_string_pretty(self, cfg).unwrap_or_default()
    }

    /// Parse and validate (version, log order, day range). Does not resolve
    /// the species or soil: see [`SaveFile::genome`], [`SaveFile::soil_profile`].
    pub fn from_ron(text: &str) -> Result<SaveFile, SaveError> {
        let probe: VersionProbe = ron::from_str(text).map_err(|e| SaveError::Parse(e.to_string()))?;
        if !(SAVE_VERSION_MIN..=SAVE_VERSION).contains(&probe.version) {
            return Err(SaveError::UnsupportedVersion(probe.version));
        }
        let file: SaveFile = ron::from_str(text).map_err(|e| SaveError::Parse(e.to_string()))?;
        file.validate()?;
        Ok(file)
    }

    fn validate(&self) -> Result<(), SaveError> {
        let max_day = MAX_YEARS * crate::sim::DAYS_PER_YEAR as u64;
        if self.day > max_day {
            return Err(SaveError::Invalid(format!("jour {} au-delà de {MAX_YEARS} ans", self.day)));
        }
        let mut prev = 0;
        for e in &self.events {
            if e.day < prev {
                return Err(SaveError::Invalid("journal non chronologique".into()));
            }
            if e.day > max_day {
                return Err(SaveError::Invalid(format!("événement au jour {}", e.day)));
            }
            if let SavedAction::Cut { t, .. } = e.action {
                if !(0.0..=1.0).contains(&t) {
                    return Err(SaveError::Invalid(format!("position de coupe {t}")));
                }
            }
            if let SavedAction::Storm { gust_ms } = e.action {
                if self.version < 2 {
                    return Err(SaveError::Invalid("tempête dans un fichier v1".into()));
                }
                if !(0.0..=crate::mechanics::MAX_SEARCH_SPEED_MS).contains(&gust_ms) {
                    return Err(SaveError::Invalid(format!("rafale de tempête {gust_ms} m/s")));
                }
            }
            prev = e.day;
        }
        Ok(())
    }

    /// Read and parse a save file.
    pub fn read(path: &Path) -> Result<SaveFile, SaveError> {
        let text = std::fs::read_to_string(path).map_err(|e| SaveError::Io(e.to_string()))?;
        SaveFile::from_ron(&text)
    }

    /// Write atomically (temporary file, then rename), creating the folder.
    pub fn write(&self, path: &Path) -> Result<(), SaveError> {
        let io = |e: std::io::Error| SaveError::Io(e.to_string());
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(io)?;
        }
        let tmp = path.with_extension("ron.tmp");
        std::fs::write(&tmp, self.to_ron()).map_err(io)?;
        std::fs::rename(&tmp, path).map_err(io)
    }

    /// The genome: the embedded one, else the built-in species by name.
    pub fn genome(&self) -> Result<Genome, SaveError> {
        if let Some(g) = &self.species.genome {
            return Ok(g.clone());
        }
        Genome::find(&self.species.name).ok_or_else(|| SaveError::UnknownSpecies(self.species.name.clone()))
    }

    /// Index of the species in `Genome::registry()` (None for an embedded or
    /// unknown genome). Saves name the species by its latin binomial (older
    /// saves too); a registry id is accepted as well.
    pub fn builtin_species_index(&self) -> Option<usize> {
        if self.species.genome.is_some() {
            return None;
        }
        Genome::index_of(&self.species.name)
    }

    /// The soil profile, by preset name.
    pub fn soil_profile(&self) -> Result<SoilProfile, SaveError> {
        SoilProfile::presets().into_iter().find(|p| p.name == self.soil).ok_or_else(|| SaveError::UnknownSoil(self.soil.clone()))
    }

    /// Index of the soil in `SoilProfile::presets()`.
    pub fn soil_index(&self) -> Option<usize> {
        SoilProfile::presets().iter().position(|p| p.name == self.soil)
    }

    /// The intervention log in the simulation's terms.
    pub fn log(&self) -> Vec<LogEvent> {
        self.events.iter().map(LogEvent::from).collect()
    }

    /// The fresh seedling (day 0) this save grows from.
    pub fn fresh_simulation(&self) -> Result<Simulation, SaveError> {
        Ok(Simulation::with_site(self.genome()?, self.seed, self.climate.clone(), self.soil_profile()?))
    }

    /// Rebuild the tree by replay, in one go (see [`Replay`] for a sliced,
    /// cancellable version with progress).
    pub fn replay(&self) -> Result<Simulation, SaveError> {
        let mut r = Replay::new(self, 0)?;
        while !r.step(u32::MAX) {}
        Ok(r.finish().sim)
    }

    /// The saved state hash as a number (None if malformed).
    pub fn saved_hash(&self) -> Option<u64> {
        u64::from_str_radix(&self.meta.state_hash, 16).ok()
    }
}

/// A replay in progress: call [`Replay::step`] until it returns `true`,
/// e.g. on a worker thread or a few hundred days per frame.
pub struct Replay {
    base: Simulation,
    sim: Simulation,
    target: u64,
    /// Days (sorted, unique) on which a cut was made, up to the target.
    cut_days: Vec<u64>,
    next_cut: usize,
    keep_checkpoints: usize,
    checkpoints: Vec<Simulation>,
    finished: bool,
}

/// What a finished replay yields.
pub struct ReplayResult {
    /// The rebuilt tree, on the saved day.
    pub sim: Simulation,
    /// The fresh seedling (day 0) it grew from.
    pub base: Simulation,
    /// Copies at the start of the last cut days, before that day's first
    /// cut, oldest first (for undo / rewind without a full replay).
    pub checkpoints: Vec<Simulation>,
}

impl Replay {
    /// Prepare the replay of `file`, keeping at most `keep_checkpoints`
    /// checkpoints (the latest cut days).
    pub fn new(file: &SaveFile, keep_checkpoints: usize) -> Result<Replay, SaveError> {
        let base = file.fresh_simulation()?;
        let log = file.log();
        let mut cut_days: Vec<u64> = log.iter().filter(|e| matches!(e.action, Action::Cut(_)) && e.day <= file.day).map(|e| e.day).collect();
        cut_days.dedup();
        let mut sim = base.checkpoint();
        sim.set_prune_log(log);
        Ok(Replay { base, sim, target: file.day, cut_days, next_cut: 0, keep_checkpoints, checkpoints: Vec::new(), finished: false })
    }

    /// Simulate at most `max_days` more days. Returns `true` when the saved day is reached.
    pub fn step(&mut self, max_days: u32) -> bool {
        let mut budget = max_days as u64;
        loop {
            // Checkpoint at the start of a cut day, before its cuts are applied.
            while self.next_cut < self.cut_days.len() && self.cut_days[self.next_cut] <= self.sim.days_elapsed() {
                if self.cut_days[self.next_cut] == self.sim.days_elapsed() && self.keep_checkpoints > 0 {
                    self.checkpoints.push(self.sim.checkpoint());
                    if self.checkpoints.len() > self.keep_checkpoints {
                        self.checkpoints.remove(0);
                    }
                }
                self.next_cut += 1;
            }
            let day = self.sim.days_elapsed();
            if day >= self.target {
                if !self.finished {
                    // Events of the saved day itself (a cut made just before saving).
                    self.sim.apply_due_events();
                    self.finished = true;
                }
                return true;
            }
            if budget == 0 {
                return false;
            }
            let stop = self.cut_days.get(self.next_cut).copied().unwrap_or(self.target).min(self.target);
            let n = (stop - day).min(budget).min(3650);
            self.sim.advance_days(n as u32);
            budget -= n;
        }
    }

    /// Days simulated so far.
    pub fn day(&self) -> u64 {
        self.sim.days_elapsed()
    }

    /// The saved day the replay is heading to.
    pub fn target_day(&self) -> u64 {
        self.target
    }

    /// 0..1.
    pub fn progress(&self) -> f32 {
        if self.target == 0 {
            1.0
        } else {
            (self.sim.days_elapsed() as f64 / self.target as f64).min(1.0) as f32
        }
    }

    /// The result (call once [`Replay::step`] returned `true`; earlier, the
    /// tree is simply younger than saved).
    pub fn finish(self) -> ReplayResult {
        ReplayResult { sim: self.sim, base: self.base, checkpoints: self.checkpoints }
    }
}

/// FNV-1a over the text written into it (fed by `Debug` formatting).
struct Fnv(u64);

impl fmt::Write for Fnv {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
        Ok(())
    }
}

impl Simulation {
    /// A stable hash of the tree's state: day, seed, drought flag, the
    /// shoot node tree (geometry, radii, buds, light...), the root system,
    /// the rings, the pruning state (log, wounds, reserves), phenology.
    /// Excludes what depends on how the simulation was driven rather than
    /// on what happened (mesh version counters, scratch buffers). Pending
    /// work of the day (a cut's deferred light pass) is settled on a copy
    /// first, so a tree saved right after a cut hashes like its replay.
    ///
    /// Stable for one build on one platform; not meant to match across
    /// engine versions (any calibration change alters it).
    pub fn state_hash(&self) -> u64 {
        if self.prune.has_pending() {
            let mut s = self.checkpoint();
            s.apply_due_events();
            return s.hash_settled();
        }
        self.hash_settled()
    }

    fn hash_settled(&self) -> u64 {
        use fmt::Write;
        let mut h = Fnv(0xcbf2_9ce4_8422_2325);
        // `Debug` prints every f32 as its shortest round-trip form, i.e.
        // one string per bit pattern, and covers private fields of the
        // sub-systems without widening their visibility.
        let _ = write!(
            h,
            "{}|{}|{}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
            self.day,
            self.seed,
            self.drought,
            self.shoots.nodes,
            self.shoots.ext,
            self.roots,
            self.rings,
            self.prune,
            self.phen,
            self.p_prev.to_bits(),
            self.stress_prev.to_bits(),
            self.vit,
            self.wind,
        );
        h.0
    }
}
