//! Validation bench of Dendrologia: confronts engine outputs with published
//! values (`data/validation/*.ron`, format in `data/validation/FORMAT.md`).
//! The generated report is `docs/validation/report.md`.
//!
//! `vent`: wind climate, sway, storms asked for, stem acclimation.

pub mod data;
pub mod eval;
pub mod measure;
pub mod notes;
pub mod report;
pub mod summary;
pub mod verdict;
pub mod vent;
