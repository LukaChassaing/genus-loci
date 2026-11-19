//! Pruning (« Taille ») API of `TreeSim`: picking, cut preview, cuts, undo, rewinding, journal,
//! metrics and the "what if" control tree. Coordinates are the tree's (Y up, metres, collar at the
//! origin); days are days since germination; cut types are `dendro_core::CutKind` codes.

use crate::backend::{mesh_data_to_array_mesh, mesh_data_to_array_mesh_named, prof};
use crate::TreeSim;
use dendro_core::{CutKind, PruneMetrics, Simulation};
use godot::classes::ArrayMesh;
use godot::prelude::*;

fn metrics_dict(m: &PruneMetrics, s: &Simulation) -> VarDictionary {
    let b = s.bole_metrics();
    let mut d = VarDictionary::new();
    let mut put = |k: &str, v: f64| d.set(&k.to_variant(), &v.to_variant());
    put("day", s.days_elapsed() as f64);
    put("age", m.age_years as f64);
    put("height_m", m.height_m as f64);
    put("crown_radius_m", m.crown_radius_m as f64);
    put("dbh_cm", m.dbh_cm as f64);
    put("leaf_area_m2", m.leaf_area_m2 as f64);
    put("reserve", m.reserve as f64);
    put("vigour", m.vigour as f64);
    put("new_shoots", m.new_shoots as f64);
    put("sprouts", m.sprouts as f64);
    put("decay_volume_m3", m.decay_volume_m3 as f64);
    put("open_wounds", m.open_wounds as f64);
    put("closed_wounds", m.closed_wounds as f64);
    put("decayed_wounds", m.decayed_wounds as f64);
    put("fine_root_loss", m.fine_root_loss as f64);
    put("life_expectancy", m.life_expectancy_years as f64);
    put("alive", if m.alive { 1.0 } else { 0.0 });
    put("clear_bole_m", b.clear_bole_m as f64);
    put("knot_free_bole_m", b.knot_free_bole_m as f64);
    put("knotty_core_cm", b.knotty_core_cm as f64);
    put("clear_wood_mm", b.clear_wood_mm as f64);
    put("pollard_heads", b.pollard_heads as f64);
    put("pollard_max_recuts", b.pollard_max_recuts as f64);
    d
}

#[godot_api(secondary)]
impl TreeSim {
    /// Branch under a ray: `{hit: bool, node_id, t, snapped, ray_t}`. `tol` = angular tolerance (rad).
    #[func]
    fn prune_pick(&self, origin: Vector3, dir: Vector3, tol: f64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let hit = self.backend.live().pick_branch([origin.x, origin.y, origin.z], [dir.x, dir.y, dir.z], tol as f32);
        d.set("hit", hit.is_some());
        if let Some(h) = hit {
            d.set("node_id", h.node_id as i64);
            d.set("t", h.t as f64);
            d.set("snapped", h.snapped);
            d.set("ray_t", h.ray_t as f64);
        }
        d
    }

    /// What a cut would do: `{ok, kind, label, diameter_cm, foliage_share, wood_share, closure_years,
    /// decay_risk, infection_risk, season (0 good / 1 acceptable / 2 bad), sprouts, point, dir,
    /// radius_m, bleeding}`.
    #[func]
    fn prune_preview(&self, node_id: i64, t: f64, flush: bool) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(p) = self.backend.live().cut_preview(node_id as u32, t as f32, flush) else {
            d.set("ok", false);
            return d;
        };
        d.set("ok", true);
        d.set("kind", p.kind.code() as i64);
        d.set("label", p.kind.label_fr());
        d.set("diameter_cm", p.diameter_cm as f64);
        d.set("foliage_share", p.foliage_share as f64);
        d.set("wood_share", p.wood_share as f64);
        d.set("closure_years", p.closure_years as f64);
        d.set("decay_risk", p.decay_risk as f64);
        d.set("infection_risk", p.infection_risk as f64);
        d.set("season", p.season as i64);
        d.set("sprouts", p.sprouts as f64);
        d.set("point", Vector3::new(p.point[0], p.point[1], p.point[2]));
        d.set("dir", Vector3::new(p.dir[0], p.dir[1], p.dir[2]));
        d.set("radius_m", p.radius_m as f64);
        d.set("bleeding", p.bleeding);
        d
    }

    /// The wood a cut would remove (highlight overlay).
    #[func]
    fn build_prune_preview_mesh(&self, node_id: i64, t: f64, flush: bool) -> Gd<ArrayMesh> {
        mesh_data_to_array_mesh(&self.backend.live().prune_preview_mesh(node_id as u32, t as f32, flush))
    }

    /// Cut faces, woundwood, decay, stubs and pollard knobs (vertex colours; render double-sided).
    #[func]
    fn build_wound_mesh(&self) -> Gd<ArrayMesh> {
        mesh_data_to_array_mesh_named(&prof::time("wound.build", || self.backend.live().wound_mesh()), "wound")
    }

    /// Cut now (logged). Returns the cut type code, -1 if nothing was cut.
    #[func]
    fn prune_cut(&mut self, node_id: i64, t: f64, flush: bool) -> i64 {
        self.meshes.invalidate();
        self.backend.prune_cut(node_id as u32, t as f32, flush) as i64
    }

    /// High-level operation ("raise" m, "top" m, "coppice", "pollard", "thin" fraction, "reduce"
    /// fraction), expanded into logged cuts. Returns the number of cuts.
    #[func]
    fn prune_op(&mut self, op: GString, arg: f64) -> i64 {
        self.meshes.invalidate();
        self.backend.prune_op(&op.to_string(), arg as f32) as i64
    }

    /// Undo the last cut (re-simulates from its day to now).
    #[func]
    fn prune_undo(&mut self) -> bool {
        self.meshes.invalidate();
        self.backend.prune_undo()
    }

    /// Jump to a day (backwards replays the timeline; later cuts come back when their day comes).
    #[func]
    fn prune_seek_day(&mut self, day: i64) {
        self.meshes.invalidate();
        self.backend.prune_seek_day(day.max(0) as u64);
    }

    #[func]
    fn get_days_elapsed(&self) -> i64 {
        self.backend.live().days_elapsed() as i64
    }

    #[func]
    fn get_prune_cut_count(&self) -> i64 {
        self.backend.prune_cut_count() as i64
    }

    /// French label of a cut type code.
    #[func]
    fn prune_kind_label(&self, kind: i64) -> GString {
        GString::from(CutKind::from(kind.clamp(0, 9) as u8).label_fr())
    }

    /// Cut journal, oldest first: `[{day, year, doy (1-based), kind, label, diameter_cm,
    /// foliage_share, state (WoundState code), state_label, closure, decay, pathogen, natural}]`.
    #[func]
    fn get_prune_journal(&self) -> VarArray {
        let mut a = VarArray::new();
        for e in self.backend.live().prune_journal() {
            let mut d = VarDictionary::new();
            d.set("day", e.day as i64);
            d.set("year", (e.day / 365) as i64);
            d.set("doy", (e.day % 365 + 1) as i64);
            d.set("kind", e.kind.code() as i64);
            d.set("label", e.kind.label_fr());
            d.set("diameter_cm", e.diameter_cm as f64);
            d.set("foliage_share", e.foliage_share as f64);
            d.set("state", e.state.code() as i64);
            d.set("state_label", e.state.label_fr());
            d.set("closure", e.closure as f64);
            d.set("decay", e.decay as f64);
            d.set("pathogen", e.pathogen);
            d.set("natural", e.natural);
            a.push(&d.to_variant());
        }
        a
    }

    /// Wound positions of the journal entries (tree coordinates; NaN when the wound is gone).
    #[func]
    fn get_wound_positions(&self) -> PackedVector3Array {
        self.backend
            .live()
            .wound_positions()
            .iter()
            .map(|p| match p {
                Some(p) => Vector3::new(p[0], p[1], p[2]),
                None => Vector3::new(f32::NAN, f32::NAN, f32::NAN),
            })
            .collect()
    }

    /// Measurements of the live tree (comparison and objectives), see `PruneMetrics` / `BoleMetrics`.
    #[func]
    fn get_prune_metrics(&self) -> VarDictionary {
        let s = self.backend.live();
        metrics_dict(&s.prune_metrics(), s)
    }

    /// Decayed share (0..1) of the stem radius at height `y` (cross-section view).
    #[func]
    fn get_stem_decay(&self, y: f64) -> f64 {
        self.backend.live().stem_decay_at(y as f32) as f64
    }

    /// Pruning knots near height `y` on the stem: flat `[cut year, stem radius at the cut (m),
    /// closed year (-1 open)]` triplets.
    #[func]
    fn get_stem_knots(&self, y: f64) -> PackedFloat32Array {
        self.backend.live().stem_knots_at(y as f32).iter().flat_map(|k| [k.0 as f32, k.1, if k.2 == u32::MAX { -1.0 } else { k.2 as f32 }]).collect()
    }

    /// The control ("what if") tree keeps the first `n` cuts (0 = never pruned).
    #[func]
    fn control_set_cuts(&mut self, n: i64) {
        self.backend.control_set_cuts(n.max(0) as usize);
    }

    #[func]
    fn control_get_cuts(&self) -> i64 {
        self.backend.control_cuts() as i64
    }

    /// Advance the control tree in the background (call each frame while shown). True when up to date.
    #[func]
    fn control_poll(&mut self, want_mesh: bool) -> bool {
        self.backend.control_poll(want_mesh)
    }

    /// Control metrics (same keys as `get_prune_metrics`), empty while being computed.
    #[func]
    fn get_control_metrics(&self) -> VarDictionary {
        match self.backend.control_sim() {
            Some(s) => metrics_dict(&s.prune_metrics(), s),
            None => VarDictionary::new(),
        }
    }

    /// Wood of the control tree (ghost silhouette); empty when none.
    #[func]
    fn build_control_wood_mesh(&self) -> Gd<ArrayMesh> {
        match self.backend.control_mesh() {
            Some(m) => mesh_data_to_array_mesh_named(m, "control"),
            None => ArrayMesh::new_gd(),
        }
    }

    /// A point `frac` along a live first-order limb leaving the stem whose radius is closest to
    /// `radius_m` (verification hooks: aim the tool without knowing node ids). `[x, y, z, node_id]`,
    /// node_id -1 when none.
    #[func]
    fn prune_hint_point(&self, radius_m: f64, frac: f64) -> PackedFloat32Array {
        let nodes = self.backend.live().shoot_nodes();
        let stem: std::collections::HashSet<u32> = nodes.iter().filter(|n| n.order == 0).map(|n| n.id).collect();
        let best = nodes
            .iter()
            .filter(|n| n.order == 1 && stem.contains(&n.parent_id) && !n.dead)
            .min_by(|a, b| (a.radius_m - radius_m as f32).abs().total_cmp(&(b.radius_m - radius_m as f32).abs()));
        let f = frac as f32;
        match best {
            Some(n) => PackedFloat32Array::from(
                &[n.start[0] + (n.end[0] - n.start[0]) * f, n.start[1] + (n.end[1] - n.start[1]) * f, n.start[2] + (n.end[2] - n.start[2]) * f, n.id as f32][..],
            ),
            None => PackedFloat32Array::from(&[0.0, 0.0, 0.0, -1.0][..]),
        }
    }

    #[func]
    fn get_control_version(&self) -> i64 {
        self.backend.control_version()
    }
}
