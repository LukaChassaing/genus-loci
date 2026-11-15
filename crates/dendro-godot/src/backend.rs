//! Simulation backend for the Godot front-end.
//!
//! `Backend` adapts `dendro_core::Simulation` to the units and conventions the Godot side
//! expects. Only `mesh_data_to_array_mesh` knows about Godot types.

use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::ArrayMesh;
use godot::prelude::*;

pub use dendro_core::MeshData;
use dendro_core::{Genome, Phase, Simulation};

/// Converts mesh data to an `ArrayMesh` (one surface, or none when the mesh is empty).
/// Packed arrays are built in bulk.
pub fn mesh_data_to_array_mesh(data: &MeshData) -> Gd<ArrayMesh> {
    let mut mesh = ArrayMesh::new_gd();
    if data.is_empty() {
        return mesh;
    }
    let n = data.positions.len();

    let verts: PackedVector3Array = data.positions.iter().map(|p| Vector3::new(p[0], p[1], p[2])).collect();
    let mut arrays = VarArray::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(ArrayType::VERTEX.ord() as usize, &verts.to_variant());
    if data.normals.len() == n {
        let v: PackedVector3Array = data.normals.iter().map(|p| Vector3::new(p[0], p[1], p[2])).collect();
        arrays.set(ArrayType::NORMAL.ord() as usize, &v.to_variant());
    }
    if data.uvs.len() == n {
        let v: PackedVector2Array = data.uvs.iter().map(|p| Vector2::new(p[0], p[1])).collect();
        arrays.set(ArrayType::TEX_UV.ord() as usize, &v.to_variant());
    }
    if data.colors.len() == n {
        let v: PackedColorArray = data.colors.iter().map(|c| Color::from_rgba(c[0], c[1], c[2], c[3])).collect();
        arrays.set(ArrayType::COLOR.ord() as usize, &v.to_variant());
    }
    let idx = PackedInt32Array::from_iter(data.indices.iter().map(|&i| i as i32));
    arrays.set(ArrayType::INDEX.ord() as usize, &idx.to_variant());

    mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
    mesh
}

/// Wraps the simulation of one tree.
pub struct Backend {
    sim: Simulation,
    /// Fraction of a day not yet simulated (the core steps in whole days).
    carry_days: f64,
    drought: bool,
}

impl Backend {
    pub fn new(seed: u64) -> Self {
        Self { sim: Simulation::new(Genome::quercus_robur(), seed), carry_days: 0.0, drought: false }
    }

    pub fn reset(&mut self, seed: u64) {
        *self = Self::new(seed);
    }

    pub fn advance_days(&mut self, days: f64) -> bool {
        self.carry_days += days.max(0.0);
        let whole = self.carry_days.floor();
        self.carry_days -= whole;
        if whole < 1.0 {
            return false;
        }
        self.sim.advance_days(whole as u32)
    }

    pub fn set_drought(&mut self, active: bool) {
        self.drought = active;
        self.sim.set_drought(active);
    }

    pub fn age_years(&self) -> f64 {
        self.sim.age_years() as f64
    }

    /// 1..=365 (the core counts from 0).
    pub fn day_of_year(&self) -> u32 {
        self.sim.day_of_year() + 1
    }

    pub fn year(&self) -> u32 {
        self.sim.year()
    }

    pub fn phase(&self) -> u8 {
        match self.sim.phase() {
            Phase::Dormant => 0,
            Phase::BudBurst => 1,
            Phase::LeafExpansion => 2,
            Phase::Mature => 3,
            Phase::Colouring => 4,
            Phase::LeafFall => 5,
        }
    }

    pub fn height_m(&self) -> f64 {
        self.sim.height_m() as f64
    }

    pub fn trunk_diameter_cm(&self) -> f64 {
        self.sim.dbh_cm() as f64
    }

    pub fn crown_radius_m(&self) -> f64 {
        self.sim.crown_radius_m() as f64
    }

    pub fn root_depth_m(&self) -> f64 {
        self.sim.root_depth_m() as f64
    }

    pub fn temperature_c(&self) -> f64 {
        self.sim.temperature_c() as f64
    }

    pub fn soil_water(&self) -> f64 {
        self.sim.soil_water() as f64
    }

    pub fn leaf_color(&self) -> [f32; 4] {
        let c = self.sim.leaf_color();
        [c[0], c[1], c[2], 1.0]
    }

    pub fn build_wood_mesh(&self) -> MeshData {
        self.sim.wood_mesh()
    }

    pub fn build_leaf_mesh(&self) -> MeshData {
        self.sim.leaf_mesh()
    }

    pub fn build_root_mesh(&self) -> MeshData {
        self.sim.root_mesh()
    }

    pub fn ring_widths_mm(&self) -> Vec<f32> {
        self.sim.rings().iter().map(|r| r.width_mm).collect()
    }

    pub fn ring_stress(&self) -> Vec<f32> {
        self.sim.rings().iter().map(|r| r.water_stress).collect()
    }

    pub fn ring_earlywood_fraction(&self) -> Vec<f32> {
        self.sim.rings().iter().map(|r| r.earlywood_fraction).collect()
    }

    pub fn ring_forced_drought(&self) -> Vec<u8> {
        self.sim.rings().iter().map(|r| r.forced_drought as u8).collect()
    }

    pub fn root_spread_m(&self) -> f64 {
        self.sim.root_spread_m() as f64
    }

    pub fn leaf_density(&self) -> f64 {
        self.sim.leaf_density() as f64
    }

    pub fn wood_version(&self) -> i64 {
        self.sim.wood_version() as i64
    }

    pub fn leaf_version(&self) -> i64 {
        self.sim.leaf_version() as i64
    }

    pub fn root_version(&self) -> i64 {
        self.sim.root_version() as i64
    }
}
