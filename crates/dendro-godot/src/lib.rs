//! Thin gdext layer: exposes `TreeSim` to Godot. All behaviour lives in `backend.rs`.
mod backend;
/// Pruning (« Taille ») functions of `TreeSim`.
mod prune_api;
/// Save / load (« Sauvegarder », « Charger ») functions of `TreeSim`.
mod save_api;
/// Wind (« Tempête », history, sway) functions of `TreeSim`.
mod wind_api;
/// Godot surfaces encoded in Rust; mesh worker thread.
mod surface;

use backend::{mesh_data_to_array_mesh_godot, mesh_data_to_array_mesh_named, prof, Backend};
use surface::MeshWorker;
use godot::classes::ArrayMesh;
use godot::prelude::*;

struct DendroExtension;

#[gdextension]
unsafe impl ExtensionLibrary for DendroExtension {}

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct TreeSim {
    backend: Backend,
    /// Background mesh builds (`mesh_request` / `mesh_poll` / `mesh_take`).
    meshes: MeshWorker,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for TreeSim {
    fn init(base: Base<RefCounted>) -> Self {
        static INDEX_THRESHOLD: std::sync::Once = std::sync::Once::new();
        INDEX_THRESHOLD.call_once(surface::init_index_threshold);
        Self { backend: Backend::new(1), meshes: MeshWorker::default(), base }
    }
}

#[godot_api]
impl TreeSim {
    #[func]
    fn reset(&mut self, seed: i64) {
        self.backend.reset(seed as u64);
        self.meshes.invalidate();
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
    /// Species used from the next `reset` on (index into `get_species_names`).
    #[func]
    fn set_species(&mut self, index: i64) {
        self.backend.set_species(index.max(0) as usize);
    }
    #[func]
    fn get_species(&self) -> i32 {
        self.backend.species_index() as i32
    }
    /// Scientific names of the built-in species.
    #[func]
    fn get_species_names(&self) -> PackedStringArray {
        Backend::species_names().iter().map(|n| GString::from(n.as_str())).collect()
    }
    /// French common names of the built-in species.
    #[func]
    fn get_species_common_names(&self) -> PackedStringArray {
        Backend::species_common_names().iter().map(|n| GString::from(n.as_str())).collect()
    }
    /// Registry ids of the built-in species (`data/species/<id>.ron`), in index order.
    #[func]
    fn get_species_ids(&self) -> PackedStringArray {
        Backend::species_ids().iter().map(|n| GString::from(n.as_str())).collect()
    }
    /// French menu group of each species (« Feuillus », « Fruitiers », « Résineux »), in index order.
    #[func]
    fn get_species_categories(&self) -> PackedStringArray {
        Backend::species_categories().iter().map(|n| GString::from(n.as_str())).collect()
    }
    /// Index of a species given by registry id or latin binomial, -1 if unknown. Indices shift when a
    /// species file is added: persist ids, not indices.
    #[func]
    fn find_species(&self, id_or_latin: GString) -> i32 {
        dendro_core::Genome::index_of(&id_or_latin.to_string()).map_or(-1, |i| i as i32)
    }
    /// Index of the species shown when nothing is chosen (the oak).
    #[func]
    fn get_default_species(&self) -> i32 {
        dendro_core::Genome::default_index() as i32
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
        mesh_data_to_array_mesh_named(&prof::time("fine_root.build", || self.backend.build_fine_root_mesh()), "fine_root")
    }
    #[func]
    fn advance_days(&mut self, days: f64) -> bool {
        prof::time("sim", || self.backend.advance_days(days))
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
    /// Vitality (« État » of the HUD, see `dendro_core::vitality`): `modelled` (bool), `reserve`,
    /// `vigour`, `death_probability` (per year), `cost_ratio`, `dieback` (branches killed last season),
    /// `trunk_alive`, `alive` (stump sprouts count), `death_age` (years, -1 while alive), `cause` (French
    /// label, "" while alive), `standing_dead`, `collapse_in` (years, -1), `collapsed`, `resprouted`,
    /// `bark_beetle_probability` (per year; > 0 only for a host in a crisis year, see `BioticTraits`).
    #[func]
    fn get_vitality(&self) -> VarDictionary {
        let v = self.backend.vitality();
        let mut d = VarDictionary::new();
        let mut put = |k: &str, x: Variant| d.set(&k.to_variant(), &x);
        put("modelled", v.modelled.to_variant());
        put("reserve", (v.reserve as f64).to_variant());
        put("vigour", (v.vigour as f64).to_variant());
        put("death_probability", (v.death_probability as f64).to_variant());
        put("bark_beetle_probability", (v.bark_beetle_probability as f64).to_variant());
        put("cost_ratio", (v.cost_ratio as f64).to_variant());
        put("dieback", (v.dieback_branches as i64).to_variant());
        put("trunk_alive", self.backend.trunk_alive().to_variant());
        put("alive", self.backend.is_alive().to_variant());
        put("death_age", v.death_age_years.map_or(-1i64, |a| a as i64).to_variant());
        put("cause", v.cause.label_fr().to_variant());
        put("standing_dead", v.standing_dead.to_variant());
        put("collapse_in", v.collapse_in_years.map_or(-1i64, |a| a as i64).to_variant());
        put("collapsed", v.collapsed.to_variant());
        put("resprouted", v.resprouted.to_variant());
        d
    }
    /// The first trunk is alive (cheap check for the time-lapse: it pauses when the trunk dies).
    #[func]
    fn get_trunk_alive(&self) -> bool {
        self.backend.trunk_alive()
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
        mesh_data_to_array_mesh_named(&prof::time("wood.build", || self.backend.build_wood_mesh()), "wood")
    }
    #[func]
    fn build_leaf_mesh(&self) -> Gd<ArrayMesh> {
        mesh_data_to_array_mesh_named(&prof::time("leaf.build", || self.backend.build_leaf_mesh()), "leaf")
    }
    #[func]
    fn build_root_mesh(&self) -> Gd<ArrayMesh> {
        mesh_data_to_array_mesh_named(&prof::time("root.build", || self.backend.build_root_mesh()), "root")
    }
    /// Flower and fruit cards (alpha 1.0 = flower, 0.5 = fruit); empty when nothing shows.
    #[func]
    fn build_flower_mesh(&self) -> Gd<ArrayMesh> {
        mesh_data_to_array_mesh_named(&prof::time("flower.build", || self.backend.build_flower_mesh()), "flower")
    }
    #[func]
    fn get_flower_version(&self) -> i64 {
        self.backend.flower_version()
    }
    /// Visible bloom 0..1 (opening, full bloom, petal fall).
    #[func]
    fn get_flower_density(&self) -> f64 {
        self.backend.flower_density()
    }
    /// Monotonic bloom progress 0..1 this year (0 before bloom, 1 after petal fall).
    #[func]
    fn get_bloom_progress(&self) -> f64 {
        self.backend.bloom_progress()
    }
    /// Visible fruit 0..1.
    #[func]
    fn get_fruit_density(&self) -> f64 {
        self.backend.fruit_density()
    }
    #[func]
    fn get_flower_color(&self) -> Color {
        let c = self.backend.flower_color();
        Color::from_rgba(c[0], c[1], c[2], c[3])
    }
    /// Fruit colour today (green -> ripe red).
    #[func]
    fn get_fruit_color(&self) -> Color {
        let c = self.backend.fruit_color();
        Color::from_rgba(c[0], c[1], c[2], c[3])
    }
    #[func]
    fn get_bark_color(&self) -> Color {
        let c = self.backend.bark_color();
        Color::from_rgba(c[0], c[1], c[2], c[3])
    }
    /// Strength 0..1 of horizontal lenticel bands.
    #[func]
    fn get_bark_lenticels(&self) -> f32 {
        self.backend.bark_lenticels()
    }
    /// Strength 0..1 of deep vertical fissures.
    #[func]
    fn get_bark_fissures(&self) -> f32 {
        self.backend.bark_fissures()
    }
    /// Bark glossiness 0..1.
    #[func]
    fn get_bark_gloss(&self) -> f32 {
        self.backend.bark_gloss()
    }
    #[func]
    fn get_leaf_length_m(&self) -> f32 {
        self.backend.leaf_length_m()
    }
    #[func]
    fn get_leaf_width_m(&self) -> f32 {
        self.backend.leaf_width_m()
    }
    /// Number of leaf lobes (both sides; 0 = unlobed).
    #[func]
    fn get_leaf_lobes(&self) -> i32 {
        self.backend.leaf_lobes()
    }
    /// Leaf margin serration 0..1.
    #[func]
    fn get_leaf_serration(&self) -> f32 {
        self.backend.leaf_serration()
    }
    /// Leaf arrangement for the leaf texture: (leaves per shoot, tip
    /// bunching 0..1, petiole / blade length ratio, droop 0..1).
    #[func]
    fn get_leaf_arrangement(&self) -> Vector4 {
        let a = self.backend.leaf_arrangement();
        Vector4::new(a[0], a[1], a[2], a[3])
    }
    /// Earlywood / latewood contrast 0..1 for the ring view (1 = ring-porous oak).
    /// Leaf form of the species: 0 broad leaf, 1 needle, 2 scale (foliage texture).
    #[func]
    fn get_leaf_type(&self) -> i32 {
        self.backend.leaf_type()
    }
    /// Foliage card size in leaf (needle) lengths.
    #[func]
    fn get_leaf_cluster_size(&self) -> f32 {
        self.backend.leaf_cluster_size()
    }
    /// The species keeps green foliage over winter (several leaf cohorts).
    #[func]
    fn get_evergreen(&self) -> bool {
        self.backend.evergreen()
    }
    /// Fruit kind: 0 fleshy, 1 nut, 2 samara, 3 pendent cone, 4 erect cone, 5 aril (fruit texture).
    #[func]
    fn get_fruit_kind(&self) -> i32 {
        self.backend.fruit_kind()
    }
    /// Size of one fruit card (cm): large fleshy fruits (apple, pear) get a single-fruit texture.
    #[func]
    fn get_fruit_cluster_cm(&self) -> f32 {
        self.backend.fruit_cluster_cm()
    }
    #[func]
    fn get_ring_contrast(&self) -> f32 {
        self.backend.ring_contrast()
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
    /// `[r_max, depth(sector 0, bin 0), ...]`: max root depth (m, 0 = none) on a polar grid of
    /// `sectors` azimuth sectors (atan2(z, x), sector-major) x `bins` radial bins out to r_max.
    #[func]
    fn get_root_envelope(&self, sectors: i64, bins: i64) -> PackedFloat32Array {
        PackedFloat32Array::from(self.backend.root_envelope(sectors.clamp(1, 256) as usize, bins.clamp(1, 256) as usize).as_slice())
    }
    /// Root water uptake (= transpiration) of the last simulated day, mm/day.
    #[func]
    fn get_water_uptake_mm(&self) -> f64 {
        self.backend.water_uptake_mm()
    }
    /// Triplets per soil layer, top down: bottom depth (m), uptake (mm/day), root density 0..1.
    #[func]
    fn get_uptake_profile(&self) -> PackedFloat32Array {
        PackedFloat32Array::from(self.backend.uptake_profile().as_slice())
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
    /// The day's flows in real units (L/day, g C/day, g/day), keyed by name (see backend `flux_values`).
    #[func]
    fn get_flux(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        for (k, v) in self.backend.flux_values() {
            d.set(&k.to_variant(), &(v as f64).to_variant());
        }
        d
    }
    /// Flow paths: `[shoot ribbons, root ribbons, hypha ribbons (ArrayMesh, see FluxGeometry::ribbon_mesh),
    /// gas site positions (PackedVector3Array), gas site (kind 0 leaf / 1 wood / 2 root, weight) pairs
    /// (PackedFloat32Array)]`.
    #[func]
    fn build_flux_geometry(&self) -> VarArray {
        use dendro_core::{FluxGeometry, GasSiteKind};
        let g = prof::time("flux.build", || self.backend.flux_geometry());
        let mut a = VarArray::new();
        a.push(&mesh_data_to_array_mesh_named(&FluxGeometry::ribbon_mesh(&g.shoot_paths, 0.0), "flux").to_variant());
        a.push(&mesh_data_to_array_mesh_named(&FluxGeometry::ribbon_mesh(&g.root_paths, 0.5), "flux").to_variant());
        a.push(&mesh_data_to_array_mesh_named(&FluxGeometry::ribbon_mesh(&g.hyphae, 1.0), "flux").to_variant());
        let pos: PackedVector3Array = g.gas_sites.iter().map(|s| Vector3::new(s.pos[0], s.pos[1], s.pos[2])).collect();
        let info: PackedFloat32Array = g
            .gas_sites
            .iter()
            .flat_map(|s| {
                let k = match s.kind {
                    GasSiteKind::Leaf => 0.0,
                    GasSiteKind::Wood => 1.0,
                    GasSiteKind::Root => 2.0,
                };
                [k, s.weight]
            })
            .collect();
        a.push(&pos.to_variant());
        a.push(&info.to_variant());
        a
    }
    /// Profiler of the binding (bench, `DENDRO_PROFILE=1`): `{key: Vector3(total ms, calls, longest call ms)}` accumulated
    /// since the last call (sim stepping, `<mesh>.build` / `.convert` / `.upload`, `flux.build`).
    #[func]
    fn prof_take(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        for (k, ms, n, max) in prof::take() {
            d.set(&k.to_variant(), &Vector3::new(ms as f32, n as f32, max as f32).to_variant());
        }
        d
    }

    /// Starts building the meshes of `kinds` (bit mask: 1 wood, 2 leaves, 4 roots + fine roots, 8 flowers)
    /// on the worker thread, from a copy of the tree as it is now. Returns false (nothing started) while a
    /// build is running or its result has not been taken yet.
    #[func]
    fn mesh_request(&mut self, kinds: i64) -> bool {
        let b = &self.backend;
        let versions = [b.wood_version(), b.leaf_version(), b.root_version(), b.flower_version()];
        let sim = self.backend.live();
        prof::time("async.request", || self.meshes.request(sim, kinds as u32, versions))
    }

    /// Mask of the kinds of a finished build waiting to be taken (0 = none yet).
    #[func]
    fn mesh_poll(&mut self) -> i64 {
        self.meshes.poll() as i64
    }

    /// True while a build runs on the worker thread.
    #[func]
    fn mesh_busy(&self) -> bool {
        self.meshes.busy()
    }

    /// The kinds of `mask` from the finished build (the others stay waiting, see `mesh_poll`): `[kinds mask,
    /// versions (PackedInt64Array: wood, leaf, root, flower, of the snapshot), wood, leaf, roots, fine roots,
    /// flowers]`, the meshes as `ArrayMesh` (null for kinds not taken). Empty when none.
    #[func]
    fn mesh_take(&mut self, mask: i64) -> VarArray {
        let mut a = VarArray::new();
        let Some(b) = self.meshes.take(mask as u32) else { return a };
        if b.millis > 0.0 {
            prof::add_ms("async.worker", b.millis);
        }
        a.push(&(b.kinds as i64).to_variant());
        a.push(&PackedInt64Array::from(&b.versions[..]).to_variant());
        let t0 = std::time::Instant::now();
        let built = |k: usize| b.kinds & (1 << k) != 0;
        let kind_of_slot = [surface::WOOD, surface::LEAF, surface::ROOT, surface::ROOT, surface::FLOWER];
        let slot_surface = [surface::WOOD, surface::LEAF, surface::ROOT, surface::KINDS, surface::FLOWER];
        const KEYS: [&str; 5] = ["install.wood", "install.leaf", "install.root", "install.fine_root", "install.flower"];
        for (slot, &k) in kind_of_slot.iter().enumerate() {
            if built(k) {
                let m = prof::time(KEYS[slot], || surface::to_array_mesh(b.surfaces[slot_surface[slot]].as_ref()));
                a.push(&m.to_variant());
            } else {
                a.push(&Variant::nil());
            }
        }
        prof::add("async.install", t0);
        a
    }

    /// Drops the build in flight and any build waiting (call when the shown tree is replaced).
    #[func]
    fn mesh_invalidate(&mut self) {
        self.meshes.invalidate();
    }

    /// Self-test: encodes the current wood, leaf, root, fine-root and flower meshes and the flux ribbons
    /// both in Rust and through Godot's `add_surface_from_arrays`, and compares the stored surfaces
    /// (format, vertex / attribute / index bytes, counts, AABB). Returns "" when identical, else what
    /// differs; "skipped" when the rendering server keeps no surface data (headless).
    #[func]
    fn debug_compare_surfaces(&self) -> GString {
        use godot::classes::RenderingServer;
        let live = self.backend.live();
        let g = live.flux_geometry(dendro_core::FluxBudget::default());
        let meshes = [
            ("wood", live.wood_mesh()),
            ("leaf", live.leaf_mesh()),
            ("root", live.root_mesh()),
            ("fine_root", live.fine_root_mesh()),
            ("flower", live.flower_mesh()),
            ("wound", live.wound_mesh()),
            ("flux", dendro_core::FluxGeometry::ribbon_mesh(&g.root_paths, 0.5)),
            // Layouts without normals (no normal / tangent block), without UVs, without colours.
            ("bare", dendro_core::MeshData {
                positions: vec![[0.0, 0.0, 0.0], [1.0, 0.5, -2.0], [0.25, 3.0, 1.0]],
                colors: vec![[1.0, 0.6, 0.2, 0.5]; 3],
                indices: vec![0, 1, 2],
                ..Default::default()
            }),
            ("normals_only", dendro_core::MeshData {
                positions: vec![[0.0, 0.0, 0.0], [1.0, 0.5, -2.0], [0.25, 3.0, 1.0]],
                normals: vec![[0.0, 0.0, -1.0], [-0.6, -0.8, 0.0], [0.0, -1.0, 0.0]],
                indices: vec![0, 2, 1],
                ..Default::default()
            }),
        ];
        let mut rs = RenderingServer::singleton();
        let mut report = Vec::new();
        let mut compared = 0;
        for (name, m) in meshes.iter() {
            if m.is_empty() {
                continue;
            }
            let ours = surface::to_array_mesh(Some(&surface::encode(m)));
            let godot = mesh_data_to_array_mesh_godot(m);
            let a = rs.mesh_get_surface(ours.get_rid(), 0);
            let b = rs.mesh_get_surface(godot.get_rid(), 0);
            if b.is_empty() || b.get("vertex_data").is_none() {
                continue;
            }
            compared += 1;
            for key in ["format", "primitive", "vertex_count", "index_count", "vertex_data", "attribute_data", "index_data"] {
                if a.get(key) != b.get(key) {
                    report.push(format!("{name}.{key}"));
                }
            }
            if ours.get_aabb() != godot.get_aabb() {
                report.push(format!("{name}.aabb {:?} vs {:?}", ours.get_aabb(), godot.get_aabb()));
            }
        }
        if compared == 0 {
            return GString::from("skipped");
        }
        GString::from(report.join(", ").as_str())
    }

    /// Static mechanics (« Mécanique ») under a steady gust of `wind_speed_ms` at 10 m, today's foliage:
    /// safety factors per failure mode, weakest section, expected failure, critical speeds (m/s, -1 when
    /// above 80 m/s), masses, moments, drag. See `dendro_core::mechanics`; cached per day / wood version.
    #[func]
    fn get_mechanics(&mut self, wind_speed_ms: f64) -> VarDictionary {
        let s = prof::time("mech", || self.backend.mechanics(wind_speed_ms.clamp(0.0, 80.0) as f32).clone());
        let mut d = VarDictionary::new();
        let mut put = |k: &str, v: Variant| {
            d.set(&k.to_variant(), &v);
        };
        let crit = |v: Option<f32>| v.map_or(-1.0, |x| x as f64);
        put("wind_ms", (s.wind_speed_ms as f64).to_variant());
        put("leaf_fraction", (s.leaf_fraction as f64).to_variant());
        put("stem_safety", (s.stem_safety as f64).to_variant());
        put("limb_safety", (s.limb_safety as f64).to_variant());
        put("anchorage_safety", (s.anchorage_safety as f64).to_variant());
        put("min_safety", (s.stem_safety.min(s.limb_safety).min(s.anchorage_safety) as f64).to_variant());
        put("expected_mode", (s.expected_failure.code() as i64).to_variant());
        put("expected_label", GString::from(s.expected_failure.label_fr()).to_variant());
        put("critical_stem", crit(s.critical.stem_break).to_variant());
        put("critical_limb", crit(s.critical.limb_break).to_variant());
        put("critical_uproot", crit(s.critical.uprooting).to_variant());
        match s.critical.first() {
            Some((m, v)) => {
                put("first_mode", (m.code() as i64).to_variant());
                put("first_label", GString::from(m.label_fr()).to_variant());
                put("first_speed", (v as f64).to_variant());
            }
            None => {
                put("first_mode", (-1i64).to_variant());
                put("first_label", GString::from("").to_variant());
                put("first_speed", (-1.0f64).to_variant());
            }
        }
        if let Some(w) = s.weakest {
            put("weak_role", (w.role.code() as i64).to_variant());
            put("weak_height_m", (w.height_m as f64).to_variant());
            put("weak_diameter_cm", (w.diameter_cm as f64).to_variant());
            put("weak_safety", (w.safety as f64).to_variant());
            put("weak_decayed", w.decayed.to_variant());
        }
        put("base_moment_knm", (s.base_moment_nm as f64 / 1000.0).to_variant());
        put("critical_moment_knm", (s.critical_turning_moment_nm as f64 / 1000.0).to_variant());
        put("mass_kg", (s.above_ground_mass_kg as f64).to_variant());
        put("wood_mass_kg", (s.wood_mass_kg as f64).to_variant());
        put("leaf_mass_kg", (s.leaf_mass_kg as f64).to_variant());
        put("stem_mass_kg", (s.stem_mass_kg as f64).to_variant());
        put("drag_kn", (s.drag_n as f64 / 1000.0).to_variant());
        put("pressure_height_m", (s.centre_of_pressure_m as f64).to_variant());
        put("crown_area_m2", (s.crown_frontal_area_m2 as f64).to_variant());
        put("crown_cd", (s.crown_drag_coefficient as f64).to_variant());
        put("buckling_safety", (s.buckling_safety as f64).to_variant());
        put("natural_frequency_hz", (s.natural_frequency_hz as f64).to_variant());
        put("damping_ratio", (s.damping_ratio as f64).to_variant());
        put("dynamic_factor", (s.dynamic_factor as f64).to_variant());
        put("top_deflection_m", (s.top_deflection_m as f64).to_variant());
        put("critical_pending", self.backend.mechanics_pending().to_variant());
        d
    }
    /// Safety factor of every shoot segment (node order) of the last `get_mechanics` speed.
    #[func]
    fn get_mechanics_safety(&mut self, wind_speed_ms: f64) -> PackedFloat32Array {
        PackedFloat32Array::from(self.backend.mechanics(wind_speed_ms.clamp(0.0, 80.0) as f32).segment_safety.as_slice())
    }
    /// The wood coloured by safety factor under `wind_speed_ms` (red <= 1, green >= 4; minor wood faded).
    #[func]
    fn build_mechanics_mesh(&mut self, wind_speed_ms: f64) -> Gd<ArrayMesh> {
        let m = prof::time("mech.build", || self.backend.mechanics_mesh(wind_speed_ms.clamp(0.0, 80.0) as f32));
        mesh_data_to_array_mesh_named(&m, "other")
    }
}
