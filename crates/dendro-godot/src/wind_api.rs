//! The daily wind never breaks anything: only storms asked for (`storm`) do.
//! Wind API of `TreeSim` (« Tempête », wind history, sway of the shader, falling branches). Days
//! are days since germination; speeds are m/s at 10 m over open ground; azimuths are those of
//! `dendro_core::WindLoad` (degrees from +x towards +z, the direction the wind blows towards).

use crate::backend::mesh_data_to_array_mesh_named;
use crate::TreeSim;
use godot::prelude::*;

#[godot_api(secondary)]
impl TreeSim {
    /// Today's wind: x = daily mean wind, y = maximum gust (m/s at 10 m), z = direction it blows
    /// towards (deg). The weather of the last simulated day.
    #[func]
    fn get_wind_today(&self) -> Vector3 {
        let (m, g, a) = self.backend.live().wind_today();
        Vector3::new(m, g, a)
    }

    /// A storm now: a gust of `gust_ms` tests the tree with today's foliage (logged and replayed like
    /// a cut). Returns the number of damage entries it caused.
    #[func]
    fn storm(&mut self, gust_ms: f64) -> i64 {
        self.meshes.invalidate();
        self.backend.storm(gust_ms.clamp(0.0, 80.0) as f32) as i64
    }

    /// The wind history, oldest first: {day, year, doy, gust_ms, storm, kind (0 stem, 1 top, 2 limb,
    /// 3 uprooted), label, height_m, diameter_cm, leaf_fraction}.
    #[func]
    fn get_wind_history(&self) -> VarArray {
        let mut a = VarArray::new();
        for e in self.backend.live().wind_history() {
            let mut d = VarDictionary::new();
            d.set("day", e.day as i64);
            d.set("year", (e.day / 365) as i64);
            d.set("doy", (e.day % 365 + 1) as i64);
            d.set("gust_ms", e.gust_ms as f64);
            d.set("storm", e.storm);
            d.set("kind", e.kind.code() as i64);
            d.set("label", e.kind.label_fr());
            d.set("height_m", e.height_m as f64);
            d.set("diameter_cm", e.diameter_cm as f64);
            d.set("leaf_fraction", e.leaf_fraction as f64);
            a.push(&d.to_variant());
        }
        a
    }

    /// x = day the tree was uprooted (-1 while standing), y = direction it fell towards (deg).
    #[func]
    fn get_uprooted(&self) -> Vector2 {
        match self.backend.live().uprooted() {
            Some((d, az)) => Vector2::new(d as f32, az),
            None => Vector2::new(-1.0, 0.0),
        }
    }

    /// Year of the last stem break (« tronc cassé, rejets »), -1 when none.
    #[func]
    fn get_last_stem_break_year(&self) -> i64 {
        self.backend.live().wind_history().iter().rev().find(|e| e.kind == dendro_core::DamageKind::Stem).map_or(-1, |e| (e.day / 365) as i64)
    }

    /// Stem diameter at the collar (cm): the size of the root plate of a windthrown tree.
    #[func]
    fn get_collar_diameter_cm(&self) -> f64 {
        self.backend.live().collar_diameter_cm() as f64
    }

    /// Acclimation reference gusts (m/s at 10 m): x in leaf, y leafless.
    #[func]
    fn get_wind_reference_gusts(&self) -> Vector2 {
        let (a, b) = self.backend.live().wind_reference_gusts();
        Vector2::new(a, b)
    }

    /// Sway of the first mode for the shader: {freq_hz, damping, top_per_10ms (static top
    /// displacement under a 10 m/s gust, m), height_m}. Recomputed once a year and when the leaves
    /// come or go.
    #[func]
    fn get_sway(&mut self) -> VarDictionary {
        let s = self.backend.sway();
        let mut d = VarDictionary::new();
        d.set("freq_hz", s.freq_hz as f64);
        d.set("damping", s.damping as f64);
        d.set("top_per_10ms", s.top_per_10ms as f64);
        d.set("height_m", s.height as f64);
        d
    }

    /// The parts the wind broke off since the last call, oldest first: [{day, pivot (Vector3, break
    /// point), mesh (ArrayMesh, the lost wood), leafy}].
    #[func]
    fn take_wind_debris(&mut self) -> VarArray {
        let mut a = VarArray::new();
        for d in self.backend.take_wind_debris() {
            let mut e = VarDictionary::new();
            e.set("day", d.day as i64);
            e.set("pivot", Vector3::new(d.pivot[0], d.pivot[1], d.pivot[2]));
            e.set("mesh", &mesh_data_to_array_mesh_named(&d.mesh, "other").to_variant());
            e.set("leafy", d.leafy_shoots as i64);
            a.push(&e.to_variant());
        }
        a
    }
}
