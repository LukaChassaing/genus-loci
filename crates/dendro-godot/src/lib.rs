//! Thin gdext layer: exposes `TreeSim` to Godot. All behaviour lives in `backend.rs`.
mod backend;

use backend::{mesh_data_to_array_mesh, Backend};
use godot::classes::ArrayMesh;
use godot::prelude::*;

struct DendroExtension;

#[gdextension]
unsafe impl ExtensionLibrary for DendroExtension {}

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct TreeSim {
    backend: Backend,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for TreeSim {
    fn init(base: Base<RefCounted>) -> Self {
        Self { backend: Backend::new(1), base }
    }
}

#[godot_api]
impl TreeSim {
    #[func]
    fn reset(&mut self, seed: i64) {
        self.backend.reset(seed as u64);
    }
    /// Soil preset used from the next `reset` on.
    #[func]
    fn set_soil(&mut self, index: i64) {
        self.backend.set_soil(index.max(0) as usize);
    }
    #[func]
    fn get_soil(&self) -> i64 {
        self.backend.soil_index() as i64
    }
    #[func]
    fn get_soil_names(&self) -> PackedStringArray {
        Backend::soil_names().iter().map(|n| GString::from(n.as_str())).collect()
    }
    #[func]
    fn get_bedrock_depth_m(&self) -> f64 {
        self.backend.bedrock_depth_m() as f64
    }
    #[func]
    fn get_water_table_depth_m(&self) -> f64 {
        self.backend.water_table_depth_m() as f64
    }
    #[func]
    fn get_horizon_bottoms(&self) -> PackedFloat32Array {
        self.backend.soil_horizons().iter().map(|h| h.0).collect()
    }
    #[func]
    fn get_horizon_kinds(&self) -> PackedInt32Array {
        self.backend.soil_horizons().iter().map(|h| h.1).collect()
    }
    #[func]
    fn get_horizon_water(&self) -> PackedFloat32Array {
        self.backend.soil_horizons().iter().map(|h| h.2).collect()
    }
    #[func]
    fn get_dry_front_m(&self) -> f64 {
        self.backend.dry_front_m() as f64
    }
    #[func]
    fn build_fine_root_mesh(&self) -> Gd<ArrayMesh> {
        mesh_data_to_array_mesh(&self.backend.build_fine_root_mesh())
    }
    #[func]
    fn advance_days(&mut self, days: f64) -> bool {
        self.backend.advance_days(days)
    }
    #[func]
    fn set_drought(&mut self, active: bool) {
        self.backend.set_drought(active);
    }
    #[func]
    fn get_age_years(&self) -> f64 {
        self.backend.age_years()
    }
    #[func]
    fn get_day_of_year(&self) -> i64 {
        self.backend.day_of_year() as i64
    }
    #[func]
    fn get_year(&self) -> i64 {
        self.backend.year() as i64
    }
    #[func]
    fn get_phase(&self) -> i64 {
        self.backend.phase() as i64
    }
    #[func]
    fn get_height_m(&self) -> f64 {
        self.backend.height_m()
    }
    #[func]
    fn get_trunk_diameter_cm(&self) -> f64 {
        self.backend.trunk_diameter_cm()
    }
    #[func]
    fn get_crown_radius_m(&self) -> f64 {
        self.backend.crown_radius_m()
    }
    #[func]
    fn get_root_depth_m(&self) -> f64 {
        self.backend.root_depth_m()
    }
    #[func]
    fn get_temperature_c(&self) -> f64 {
        self.backend.temperature_c()
    }
    #[func]
    fn get_soil_water(&self) -> f64 {
        self.backend.soil_water()
    }
    #[func]
    fn get_leaf_color(&self) -> Color {
        let c = self.backend.leaf_color();
        Color::from_rgba(c[0], c[1], c[2], c[3])
    }
    #[func]
    fn build_wood_mesh(&self) -> Gd<ArrayMesh> {
        mesh_data_to_array_mesh(&self.backend.build_wood_mesh())
    }
    #[func]
    fn build_leaf_mesh(&self) -> Gd<ArrayMesh> {
        mesh_data_to_array_mesh(&self.backend.build_leaf_mesh())
    }
    #[func]
    fn build_root_mesh(&self) -> Gd<ArrayMesh> {
        mesh_data_to_array_mesh(&self.backend.build_root_mesh())
    }
    #[func]
    fn get_ring_widths(&self) -> PackedFloat32Array {
        PackedFloat32Array::from(self.backend.ring_widths_mm().as_slice())
    }
    #[func]
    fn get_ring_stress(&self) -> PackedFloat32Array {
        PackedFloat32Array::from(self.backend.ring_stress().as_slice())
    }
    #[func]
    fn get_ring_earlywood_fraction(&self) -> PackedFloat32Array {
        PackedFloat32Array::from(self.backend.ring_earlywood_fraction().as_slice())
    }
    #[func]
    fn get_ring_forced_drought(&self) -> PackedByteArray {
        PackedByteArray::from(self.backend.ring_forced_drought().as_slice())
    }
    #[func]
    fn get_root_spread_m(&self) -> f64 {
        self.backend.root_spread_m()
    }
    #[func]
    fn get_leaf_density(&self) -> f64 {
        self.backend.leaf_density()
    }
    #[func]
    fn get_wood_version(&self) -> i64 {
        self.backend.wood_version()
    }
    #[func]
    fn get_leaf_version(&self) -> i64 {
        self.backend.leaf_version()
    }
    #[func]
    fn get_root_version(&self) -> i64 {
        self.backend.root_version()
    }
}
