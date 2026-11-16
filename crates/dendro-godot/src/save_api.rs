//! Save / load API of `TreeSim` (« Sauvegarder », « Charger »). Paths are absolute file-system
//! paths (GDScript globalizes `user://saves/...`). Errors come back as French messages ("" = ok).

use crate::backend::save_load::{self, SaveFileOrError};
use crate::TreeSim;
use godot::prelude::*;
use std::path::Path;

fn hex(h: u64) -> String {
    format!("{h:016x}")
}

fn entry_dict(e: save_load::SaveEntry) -> VarDictionary {
    let mut d = VarDictionary::new();
    let stem = Path::new(&e.path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    d.set("path", e.path.as_str());
    d.set("file", stem.as_str());
    match e.file {
        SaveFileOrError::Ok(f) => {
            d.set("ok", true);
            d.set("error", "");
            d.set("name", f.meta.name.as_str());
            d.set("species", f.species.name.as_str());
            d.set("common", save_load::common_name(&f.species.name).as_str());
            d.set("age", f.meta.age_years as f64);
            d.set("height_m", f.meta.height_m as f64);
            d.set("dbh_cm", f.meta.dbh_cm as f64);
            d.set("cuts", f.meta.cuts as i64);
            d.set("written", f.meta.written_unix);
            d.set("day", f.day as i64);
            d.set("version", f.version as i64);
        }
        SaveFileOrError::Err(msg) => {
            d.set("ok", false);
            d.set("error", msg.as_str());
            d.set("name", stem.as_str());
            d.set("written", e.modified_unix);
        }
    }
    d
}

#[godot_api(secondary)]
impl TreeSim {
    /// Write the current tree (its recipe: species, seed, site, log, day) to `path`, named `name`;
    /// `written_unix` = now (Unix seconds). Returns "" or an error message.
    #[func]
    fn save_to(&self, path: GString, name: GString, written_unix: i64) -> GString {
        match self.backend.save_to(Path::new(&path.to_string()), &name.to_string(), written_unix) {
            Ok(()) => GString::new(),
            Err(e) => GString::from(e.to_string().as_str()),
        }
    }

    /// Start loading `path`: resets to the saved species / soil / seed at once and replays the
    /// history in the background (poll with `load_poll`). Returns "" or an error message (the
    /// current tree is kept on error).
    #[func]
    fn begin_load(&mut self, path: GString) -> GString {
        match self.backend.begin_load(Path::new(&path.to_string())) {
            Ok(()) => GString::new(),
            Err(e) => GString::from(e.to_string().as_str()),
        }
    }

    /// Progress 0..1 of the load in progress; the loaded tree is adopted when it returns 1.
    #[func]
    fn load_poll(&mut self) -> f64 {
        self.backend.load_poll()
    }

    #[func]
    fn is_loading(&self) -> bool {
        self.backend.is_loading()
    }

    /// Abandon the load in progress (the seedling of the saved site stays).
    #[func]
    fn cancel_load(&mut self) {
        self.backend.cancel_load();
    }

    /// Error of the last load that failed after starting ("" when none).
    #[func]
    fn get_load_error(&self) -> GString {
        GString::from(self.backend.load_error())
    }

    /// Last finished load: `{name, hash, saved_hash, match, millis, day}` (empty when none).
    #[func]
    fn get_load_info(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        if let Some(i) = self.backend.load_info() {
            d.set("name", i.name.as_str());
            d.set("hash", hex(i.hash).as_str());
            d.set("saved_hash", i.saved_hash.map(hex).unwrap_or_default().as_str());
            d.set("match", i.saved_hash == Some(i.hash));
            d.set("millis", i.millis as i64);
            d.set("day", i.day as i64);
        }
        d
    }

    /// State hash of the live tree, 16 hex digits (see `Simulation::state_hash`).
    #[func]
    fn get_state_hash(&self) -> GString {
        GString::from(hex(self.backend.live().state_hash()).as_str())
    }

    #[func]
    fn get_seed(&self) -> i64 {
        self.backend.live().seed() as i64
    }

    #[func]
    fn get_drought(&self) -> bool {
        self.backend.live().drought()
    }

    /// The saves in `dir`, newest first: `{path, file, ok, error, name, species, common, age,
    /// height_m, dbh_cm, cuts, written, day, version}`.
    #[func]
    fn list_saves(dir: GString) -> VarArray {
        let mut out = VarArray::new();
        for e in save_load::list_saves(Path::new(&dir.to_string())) {
            out.push(&entry_dict(e).to_variant());
        }
        out
    }

    /// Metadata of one save file (same keys as `list_saves`).
    #[func]
    fn read_save_info(path: GString) -> VarDictionary {
        entry_dict(save_load::read_entry(Path::new(&path.to_string())))
    }

    /// Delete a save file (`.ron` only). True when removed.
    #[func]
    fn delete_save(path: GString) -> bool {
        save_load::delete_save(Path::new(&path.to_string()))
    }
}
