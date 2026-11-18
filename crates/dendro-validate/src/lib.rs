//! Validation bench of Dendrologia: confronts engine outputs with published
//! values (`data/validation/*.ron`, format in `data/validation/FORMAT.md`).
//! The generated report is `docs/validation/report.md`.
//!
//! The `vent` feature compiles the measurements that need the dynamic-wind
//! API of the `m1-vent-dynamique` branch; it does not build on main.

pub mod data;
pub mod eval;
pub mod measure;
pub mod notes;
pub mod report;
pub mod summary;
pub mod verdict;
#[cfg(feature = "vent")]
pub mod vent;
