//! Simulation backend for the Godot front-end.
//!
//! `Backend` adapts `dendro_core::Simulation` to the units and conventions the Godot side
//! expects. Only `mesh_data_to_array_mesh` knows about Godot types.

use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::ArrayMesh;
use godot::prelude::*;

pub use dendro_core::MeshData;
use dendro_core::{ClimateParams, Genome, HorizonKind, Phase, Simulation, SoilProfile};

/// Pruning timeline (undo, rewind, control tree).
#[path = "prune.rs"]
mod prune;
/// Save / load (recipe files, replay on a worker thread).
#[path = "save_load.rs"]
pub(crate) mod save_load;

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
    if data.uv2.len() == n {
        let v: PackedVector2Array = data.uv2.iter().map(|p| Vector2::new(p[0], p[1])).collect();
        arrays.set(ArrayType::TEX_UV2.ord() as usize, &v.to_variant());
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
    /// Index of the soil preset used at the next reset.
    soil: usize,
    /// Index of the species (`Genome::builtin()`) used at the next reset.
    species: usize,
    /// Pruning timeline: checkpoints and control tree (see `prune.rs`).
    timeline: prune::Timeline,
    /// Load in progress / last load outcome (see `save_load.rs`).
    load: save_load::LoadState,
}

impl Backend {
    pub fn new(seed: u64) -> Self {
        Self::with_site(seed, 0, 0)
    }

    /// `soil` indexes `SoilProfile::presets()`, `species` indexes `Genome::builtin()`;
    /// out-of-range values fall back to the first entry.
    pub fn with_site(seed: u64, soil: usize, species: usize) -> Self {
        let mut presets = SoilProfile::presets();
        let soil = if soil < presets.len() { soil } else { 0 };
        let profile = presets.swap_remove(soil);
        let mut genomes = Genome::builtin();
        let species = if species < genomes.len() { species } else { 0 };
        let genome = genomes.swap_remove(species);
        let sim = Simulation::with_site(genome, seed, ClimateParams::default(), profile);
        let mut b = Self { sim, carry_days: 0.0, drought: false, soil, species, timeline: prune::Timeline::default(), load: save_load::LoadState::default() };
        b.timeline_reset();
        b
    }

    pub fn reset(&mut self, seed: u64) {
        *self = Self::with_site(seed, self.soil, self.species);
    }

    pub fn set_soil(&mut self, soil: usize) {
        self.soil = soil;
    }

    pub fn soil_index(&self) -> usize {
        self.soil
    }

    /// Species used from the next reset on (like the soil).
    pub fn set_species(&mut self, species: usize) {
        self.species = species;
    }

    /// Index of the species selected for the next reset.
    pub fn species_index(&self) -> usize {
        self.species
    }

    /// Scientific names of the built-in species, in index order.
    pub fn species_names() -> Vec<String> {
        Genome::builtin().into_iter().map(|g| g.species).collect()
    }

    /// French common names of the built-in species, in index order.
    pub fn species_common_names() -> Vec<String> {
        Genome::builtin().into_iter().map(|g| g.common_name_fr).collect()
    }

    pub fn soil_names() -> Vec<String> {
        SoilProfile::presets().into_iter().map(|p| p.name).collect()
    }

    /// Depth of rock under the current soil, or a negative value when there is none.
    pub fn bedrock_depth_m(&self) -> f32 {
        self.sim.soil_profile().bedrock_depth_m.unwrap_or(-1.0)
    }

    /// Depth of the permanent water table, or a negative value when there is none.
    pub fn water_table_depth_m(&self) -> f32 {
        self.sim.soil_profile().water_table_depth_m.unwrap_or(-1.0)
    }

    /// The simulated thin layers merged into runs of the same horizon kind, top to bottom:
    /// (bottom depth m, kind 0 litter / 1 topsoil / 2 subsoil / 3 parent material / 4 rock, mean water 0..1).
    pub fn soil_horizons(&self) -> Vec<(f32, i32, f32)> {
        let mut out: Vec<(f32, i32, f32, f32)> = Vec::new(); // bottom, kind, water*thickness, thickness
        for l in self.sim.soil_layers() {
            let kind = match l.kind {
                HorizonKind::Organic => 0,
                HorizonKind::Topsoil => 1,
                HorizonKind::Subsoil => 2,
                HorizonKind::Parent => 3,
            };
            let t = (l.bottom_m - l.top_m).max(0.0);
            match out.last_mut() {
                Some(last) if last.1 == kind => {
                    last.0 = l.bottom_m;
                    last.2 += l.water * t;
                    last.3 += t;
                }
                _ => out.push((l.bottom_m, kind, l.water * t, t)),
            }
        }
        let mut merged: Vec<(f32, i32, f32)> =
            out.into_iter().map(|(b, k, w, t)| (b, k, if t > 0.0 { w / t } else { 0.0 })).collect();
        // Rock under a shallow soil is shown as its own dry layer (kind 4).
        if let Some(rock) = self.sim.soil_profile().bedrock_depth_m {
            merged.retain(|h| h.0 <= rock + 0.01);
            merged.push((rock + 100.0, 4, 0.0));
        }
        merged
    }

    /// Depth down to which the soil is dry (water below 0.15) without interruption from the surface.
    pub fn dry_front_m(&self) -> f32 {
        let mut depth = 0.0;
        for l in self.sim.soil_layers() {
            if l.water >= 0.15 {
                break;
            }
            depth = l.bottom_m;
        }
        depth
    }

    pub fn build_fine_root_mesh(&self) -> MeshData {
        self.sim.fine_root_mesh()
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

    pub fn build_flower_mesh(&self) -> MeshData {
        self.sim.flower_mesh()
    }

    pub fn flower_version(&self) -> i64 {
        self.sim.flower_version() as i64
    }

    pub fn flower_density(&self) -> f64 {
        self.sim.flower_density() as f64
    }

    pub fn bloom_progress(&self) -> f64 {
        self.sim.bloom_progress() as f64
    }

    pub fn fruit_density(&self) -> f64 {
        self.sim.fruit_density() as f64
    }

    pub fn flower_color(&self) -> [f32; 4] {
        let c = self.sim.flower_color();
        [c[0], c[1], c[2], 1.0]
    }

    pub fn fruit_color(&self) -> [f32; 4] {
        let c = self.sim.fruit_color();
        [c[0], c[1], c[2], 1.0]
    }

    /// Bark traits of the species being simulated (renderer only).
    pub fn bark_color(&self) -> [f32; 4] {
        let c = self.sim.genome().bark.color;
        [c[0], c[1], c[2], 1.0]
    }

    pub fn bark_lenticels(&self) -> f32 {
        self.sim.genome().bark.lenticels
    }

    pub fn bark_fissures(&self) -> f32 {
        self.sim.genome().bark.fissures
    }

    pub fn bark_gloss(&self) -> f32 {
        self.sim.genome().bark.gloss
    }

    /// Leaf shape of the species being simulated (for the leaf texture).
    pub fn leaf_length_m(&self) -> f32 {
        self.sim.genome().leaf.length_cm * 0.01
    }

    pub fn leaf_width_m(&self) -> f32 {
        self.sim.genome().leaf.width_cm * 0.01
    }

    pub fn leaf_lobes(&self) -> i32 {
        self.sim.genome().leaf.lobes as i32
    }

    pub fn leaf_serration(&self) -> f32 {
        self.sim.genome().leaf.serration
    }

    /// Leaf arrangement on the shoot (for the leaf texture): leaves per
    /// shoot, tip bunching 0..1, petiole / blade length, droop 0..1.
    pub fn leaf_arrangement(&self) -> [f32; 4] {
        let l = &self.sim.genome().leaf;
        [l.leaves_per_shoot, l.tip_bunching, l.petiole_ratio, l.leaf_droop]
    }

    /// Earlywood / latewood contrast 0..1 for the ring view (1 = ring-porous).
    pub fn ring_contrast(&self) -> f32 {
        self.sim.genome().wood.ring_contrast
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

    /// Root envelope for the soil display: `[r_max, then sectors x bins max depths]` (see
    /// `Simulation::root_envelope`), with `r_max` = root spread + 0.5 m (fine-root reach).
    pub fn root_envelope(&self, sectors: usize, bins: usize) -> Vec<f32> {
        let r_max = self.sim.root_spread_m() + 0.5;
        let mut v = vec![r_max];
        v.extend(self.sim.root_envelope(sectors, bins, r_max));
        v
    }

    /// Root water uptake of the last simulated day (mm/day).
    pub fn water_uptake_mm(&self) -> f64 {
        self.sim.water_uptake_mm() as f64
    }

    /// Per simulated soil layer, top down: (bottom depth m, uptake mm/day, root density 0..1).
    pub fn uptake_profile(&self) -> Vec<f32> {
        self.sim.soil_layers().iter().flat_map(|l| [l.bottom_m, l.uptake_mm, l.root_density]).collect()
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

    /// The day's flows (see `dendro_core::flux`), as named values in real units.
    pub fn flux_values(&self) -> Vec<(&'static str, f32)> {
        let f = self.sim.flux_snapshot();
        let myco = match f.mycorrhiza {
            dendro_core::MycorrhizaType::None => 0.0,
            dendro_core::MycorrhizaType::Arbuscular => 1.0,
            dendro_core::MycorrhizaType::Ecto => 2.0,
        };
        vec![
            ("crown_area_m2", f.crown_area_m2),
            ("uptake_l", f.uptake_l),
            ("xylem_l", f.xylem_l),
            ("transpiration_l", f.transpiration_l),
            ("growth_water_l", f.growth_water_l),
            ("storage_buffer_l", f.storage_buffer_l),
            ("rain_l", f.rain_l),
            ("interception_l", f.interception_l),
            ("n_uptake_g", f.nutrient_uptake_g[0]),
            ("p_uptake_g", f.nutrient_uptake_g[1]),
            ("k_uptake_g", f.nutrient_uptake_g[2]),
            ("n_xylem_g", f.nutrient_xylem_g[0]),
            ("n_remob_g", f.nutrient_remob_g[0]),
            ("n_resorb_g", f.nutrient_resorb_g[0]),
            ("myco_n_g", f.myco_nutrients_g[0]),
            ("myco_p_g", f.myco_nutrients_g[1]),
            ("nutrient_limitation", f.nutrient_limitation),
            ("gpp_gc", f.gpp_gc),
            ("resp_leaf_gc", f.resp_leaf_gc),
            ("resp_wood_gc", f.resp_wood_gc),
            ("resp_root_gc", f.resp_root_gc),
            ("resp_growth_gc", f.resp_growth_gc),
            ("alloc_shoots_gc", f.alloc_shoots_gc),
            ("alloc_wood_gc", f.alloc_wood_gc),
            ("alloc_roots_gc", f.alloc_roots_gc),
            ("alloc_fruits_gc", f.alloc_fruits_gc),
            ("alloc_reserves_gc", f.alloc_reserves_gc),
            ("alloc_myco_gc", f.alloc_myco_gc),
            ("reserves_used_gc", f.reserves_used_gc),
            ("phloem_down_gc", f.phloem_down_gc),
            ("phloem_up_gc", f.phloem_up_gc),
            ("ethanol_gc", f.ethanol_gc),
            ("litter_gc", f.litter_gc),
            ("co2_in_g", f.co2_in_g),
            ("o2_out_g", f.o2_out_g),
            ("o2_in_g", f.o2_in_g),
            ("co2_out_g", f.co2_out_g),
            ("co2_out_leaf_g", f.co2_out_organs_g[0]),
            ("co2_out_wood_g", f.co2_out_organs_g[1]),
            ("co2_out_root_g", f.co2_out_organs_g[2]),
            ("root_hypoxia", f.root_hypoxia),
            ("water_stress", f.water_stress),
            ("mycorrhiza", myco),
            ("myco_activity", f.myco_activity),
            ("myco_water_l", f.myco_water_l),
        ]
    }

    /// Flow path geometry (`dendro_core::flux::FluxGeometry`).
    pub fn flux_geometry(&self) -> dendro_core::FluxGeometry {
        self.sim.flux_geometry(dendro_core::FluxBudget::default())
    }
}
