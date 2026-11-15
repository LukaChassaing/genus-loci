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
pub mod genome;
mod growth;
mod math;
pub mod mesh;
pub mod phenology;
pub mod rings;
mod rng;
mod roots;
mod sim;
pub mod soil;

pub use climate::{ClimateParams, Weather};
pub use genome::{
    Architecture, Genome, GenomeError, LeafTraits, PhenologyTraits, Pigments, RootTraits, Tolerances, WoodTraits,
};
pub use mesh::MeshData;
pub use phenology::{Phase, Season};
pub use rings::Ring;
pub use soil::{Horizon, HorizonKind, SoilError, SoilLayerState, SoilProfile};
pub use sim::{ShapeMetrics, Simulation, TreeStats, DAYS_PER_YEAR, FINE_ROOT_TRI_BUDGET, LEAF_TRI_BUDGET, ROOT_TRI_BUDGET, WOOD_TRI_BUDGET};
