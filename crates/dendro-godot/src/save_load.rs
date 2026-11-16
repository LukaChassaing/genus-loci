//! Save / load of the tree for the front-end (child module of `backend`).
//!
//! Saving writes the recipe (`dendro_core::save::SaveFile`). Loading resets the backend to the
//! saved site and replays the saved history on a worker thread; the front-end polls the progress
//! every frame (the window never freezes) and the result is adopted on the main thread, with the
//! replay's checkpoints installed in the pruning timeline (undo right after a load is not a full
//! replay from germination).

use super::Backend;
use dendro_core::{Genome, Replay, ReplayResult, SaveError, SaveFile};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Instant;

/// Checkpoints kept from a replay (a 200-year oak checkpoint is ~8 MB).
const LOAD_CHECKPOINTS: usize = 8;
/// Days simulated between two progress updates / cancel checks.
const SLICE_DAYS: u32 = 60;

struct Loader {
    handle: JoinHandle<Option<(ReplayResult, u64)>>,
    cancel: Arc<AtomicBool>,
    day: Arc<AtomicU64>,
    target: u64,
    saved_hash: Option<u64>,
    name: String,
    started: Instant,
}

/// Loading state held by the backend.
#[derive(Default)]
pub struct LoadState {
    loader: Option<Loader>,
    /// Message of the last failed load ("" when none).
    pub error: String,
    /// Outcome of the last finished load.
    pub info: Option<LoadInfo>,
}

impl Drop for LoadState {
    fn drop(&mut self) {
        if let Some(l) = self.loader.take() {
            l.cancel.store(true, Ordering::Relaxed);
            let _ = l.handle.join();
        }
    }
}

#[derive(Clone, Debug)]
pub struct LoadInfo {
    pub name: String,
    pub saved_hash: Option<u64>,
    pub hash: u64,
    pub millis: u64,
    pub day: u64,
}

/// One entry of the save browser.
pub struct SaveEntry {
    pub path: String,
    pub file: SaveFileOrError,
    pub modified_unix: i64,
}

pub enum SaveFileOrError {
    Ok(Box<SaveFile>),
    Err(String),
}

/// French common name of a built-in species, or the given name.
pub fn common_name(species: &str) -> String {
    Genome::builtin().into_iter().find(|g| g.species == species).map_or_else(|| species.to_string(), |g| g.common_name_fr)
}

impl Backend {
    /// Write the current tree to `path`. `written_unix`: current time from the caller.
    pub fn save_to(&self, path: &Path, name: &str, written_unix: i64) -> Result<(), SaveError> {
        SaveFile::capture(&self.sim, name, written_unix).write(path)
    }

    /// Start loading `path`: on success the backend is reset to the saved species, soil and seed
    /// (a seedling, until the replay finishes) and the replay runs in the background.
    pub fn begin_load(&mut self, path: &Path) -> Result<(), SaveError> {
        let file = SaveFile::read(path)?;
        let replay = Replay::new(&file, LOAD_CHECKPOINTS)?; // resolves species and soil: fails early
        let species = file.builtin_species_index().ok_or_else(|| SaveError::UnknownSpecies(file.species.name.clone()))?;
        let soil = file.soil_index().ok_or_else(|| SaveError::UnknownSoil(file.soil.clone()))?;
        *self = Backend::with_site(file.seed, soil, species);
        let cancel = Arc::new(AtomicBool::new(false));
        let day = Arc::new(AtomicU64::new(0));
        let (flag, progress) = (cancel.clone(), day.clone());
        let handle = std::thread::spawn(move || {
            let mut r = replay;
            while !r.step(SLICE_DAYS) {
                progress.store(r.day(), Ordering::Relaxed);
                if flag.load(Ordering::Relaxed) {
                    return None;
                }
            }
            progress.store(r.day(), Ordering::Relaxed);
            let res = r.finish();
            // Hashing a 200-year oak takes ~0.2 s: here, not on the frame.
            let hash = res.sim.state_hash();
            Some((res, hash))
        });
        self.load.loader = Some(Loader {
            handle,
            cancel,
            day,
            target: file.day,
            saved_hash: file.saved_hash(),
            name: file.meta.name.clone(),
            started: Instant::now(),
        });
        self.load.error.clear();
        self.load.info = None;
        Ok(())
    }

    pub fn is_loading(&self) -> bool {
        self.load.loader.is_some()
    }

    /// Progress 0..1 of the load in progress; adopts the result when the replay is done (returns
    /// 1). 1 when nothing is loading.
    pub fn load_poll(&mut self) -> f64 {
        let Some(l) = &self.load.loader else { return 1.0 };
        if !l.handle.is_finished() {
            let p = if l.target == 0 { 1.0 } else { l.day.load(Ordering::Relaxed) as f64 / l.target as f64 };
            return p.clamp(0.0, 0.999);
        }
        let l = self.load.loader.take().expect("loader");
        match l.handle.join() {
            Ok(Some((res, hash))) => {
                let millis = l.started.elapsed().as_millis() as u64;
                let day = res.sim.days_elapsed();
                self.sim = res.sim;
                self.carry_days = 0.0;
                self.drought = self.sim.drought();
                self.timeline_install(res.base, res.checkpoints);
                self.load.info = Some(LoadInfo { name: l.name, saved_hash: l.saved_hash, hash, millis, day });
                1.0
            }
            Ok(None) => 1.0, // cancelled
            Err(_) => {
                self.load.error = "le rejeu a échoué (erreur interne)".into();
                1.0
            }
        }
    }

    /// Message of the last failed load ("" when none).
    pub fn load_error(&self) -> &str {
        &self.load.error
    }

    /// Outcome of the last finished load.
    pub fn load_info(&self) -> Option<&LoadInfo> {
        self.load.info.as_ref()
    }

    pub fn cancel_load(&mut self) {
        if let Some(l) = self.load.loader.take() {
            l.cancel.store(true, Ordering::Relaxed);
            let _ = l.handle.join();
        }
    }
}

/// The `.ron` saves of `dir`, newest first (unreadable ones included, with their error).
pub fn list_saves(dir: &Path) -> Vec<SaveEntry> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<SaveEntry> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "ron"))
        .map(|p| read_entry(&p))
        .collect();
    let when = |e: &SaveEntry| match &e.file {
        SaveFileOrError::Ok(f) => f.meta.written_unix,
        SaveFileOrError::Err(_) => e.modified_unix,
    };
    out.sort_by(|a, b| when(b).cmp(&when(a)).then_with(|| a.path.cmp(&b.path)));
    out
}

/// One save file with its metadata (or why it cannot be read).
pub fn read_entry(p: &Path) -> SaveEntry {
    let modified_unix = std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs() as i64);
    let file = match SaveFile::read(p) {
        Ok(f) => SaveFileOrError::Ok(Box::new(f)),
        Err(e) => SaveFileOrError::Err(e.to_string()),
    };
    SaveEntry { path: p.to_string_lossy().into_owned(), file, modified_unix }
}

/// Delete a save (only `.ron` files). True when removed.
pub fn delete_save(path: &Path) -> bool {
    path.extension().is_some_and(|x| x == "ron") && std::fs::remove_file(path).is_ok()
}
