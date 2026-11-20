//! # dendro-core
//!
//! Engine-agnostic, deterministic tree growth simulation for *Dendrologia*.
//!
//! A [`Simulation`] grows one tree from a [`Genome`] (species traits, see
//! `data/species/*.ron`) in a procedural temperate-oceanic [`climate`]:
//! daily weather, a layered [`soil`] profile with its water balance,
//! phenology and productivity; bud-based shoot
//! growth with light competition (after Palubicki et al. 2009), Pressler /
//! pipe-model secondary growth recorded as annual [`Ring`]s, a root system
//! that responds to the soil (stylised where noted), and mesh output
//! ([`MeshData`]) for wood, leaves, structural and fine roots.
//!
//! ```no_run
//! use dendro_core::{Genome, Simulation};
//! let mut sim = Simulation::new(Genome::quercus_robur(), 42);
//! sim.advance_days(365 * 25);
//! println!("height {:.1} m, dbh {:.1} cm", sim.height_m(), sim.dbh_cm());
//! let wood = sim.wood_mesh();
//! # let _ = wood;
//! ```
//!
//! Determinism: same genome + seed + same call sequence => bit-identical
//! state on the same platform/build. All randomness is hashed from
//! `(seed, entity, year)`; the simulation is single-threaded.

pub mod climate;
pub mod flux;
pub mod fmath;
pub mod genome;
mod growth;
mod math;
pub mod mechanics;
pub mod mesh;
pub mod phenology;
pub mod pruning;
pub mod rings;
mod rng;
mod roots;
pub mod save;
mod sim;
pub mod soil;
pub mod tempgen;
pub mod vitality;
pub mod wind;

pub use climate::{ClimateParams, Weather};
pub use flux::{Compartment, Flow, FlowUnit, FluxBudget, FluxGeometry, FluxPolyline, FluxSnapshot, GasSite, GasSiteKind};
pub use genome::{FruitKind, LeafType, MechanicsTraits, MycorrhizaType, SpeciesCategory, SymbiosisTraits};
pub use genome::{
    Architecture, BarkTraits, BioticTraits, Genome, GenomeError, LeafTraits, LongevityTraits, PhenologyTraits, Pigments, ReproductionTraits, RootTraits,
    Tolerances, WoodTraits,
};
pub use mechanics::{CriticalSpeeds, DynamicFactor, Dynamics, FailureMode, MechSegment, MechSite, MechanicsModel, MechanicsSnapshot, SegmentRole, WeakPoint, WindLoad};
pub use mesh::MeshData;
pub use phenology::{Phase, ReproStage, Season};
pub use pruning::{
    Action, BoleMetrics, CutKind, CutPreview, CutSpec, JournalEntry, LogEvent, PickHit, PruneMetrics, PruningTraits, ShootInfo, WoundState,
};
pub use rings::Ring;
pub use save::{Replay, ReplayResult, SaveError, SaveFile, SaveMeta, SAVE_VERSION_MIN, SAVE_VERSION};
pub use vitality::{DeathCause, VitalityInfo};
pub use wind::{DamageKind, WindDamage, WindDebris};
pub use soil::{Horizon, HorizonKind, SoilError, SoilLayerState, SoilProfile};
pub use sim::{
    ShapeMetrics, Simulation, TreeStats, DAYS_PER_YEAR, FINE_ROOT_TRI_BUDGET, FLOWER_TRI_BUDGET, LEAF_TRI_BUDGET, ROOT_TRI_BUDGET,
    WOOD_TRI_BUDGET,
};
