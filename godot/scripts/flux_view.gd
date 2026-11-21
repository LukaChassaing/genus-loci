class_name TreeFluxView
extends Node3D
## Whole-tree flux view: every flow the core reports (TreeSim.get_flux, real units) drawn as luminous
## dashes travelling along its real path (TreeSim.build_flux_geometry: sampled root tip -> collar ->
## trunk -> branch -> leafy tip polylines, hyphae, gas-exchange sites).
##
## * Ribbons (xylem sap + minerals, phloem sugars, mycorrhizal hyphae): one quad per path segment and
##   strand, expanded in the vertex shader: camera-facing, pushed out to the near side of the bark (the
##   drawn wood radius is in UV2.y) so sap inside the trunk reads as a bundle of dashes on its surface;
##   phloem sits just outside the xylem strands (bark side), as in the real stem. A dim depth-test-off
##   pass keeps the crown's dashes readable through summer foliage (above ground only, so nothing shows
##   through opaque soil). Dashes are a shader pattern on the signed path distance (UV.x), their phase
##   integrated on the CPU per kind (speed follows the rate, never jumps): no per-dash CPU work.
## * Gas particles (one MultiMesh, one draw call): CO2 drawn into the leaves and O2 / water vapour
##   leaving them by day; respiration CO2 leaving bark and roots and O2 entering the roots.
## Dashes have a dark halo (blend_mix, not additive) so they read on the mid-blue sky, the pale horizon,
## green foliage and the dark soil alike. Densities and speeds follow the simulated rates per unit
## crown area, so the view changes with season, weather, drought and soil.

const KINDS: Array[String] = ["water", "minerals", "sugar", "o2", "co2", "vapour", "myco"]
const COLORS: Dictionary = {
	"water": Color(0.36, 0.82, 1.00),
	"minerals": Color(1.00, 0.78, 0.30),
	"sugar": Color(1.00, 0.56, 0.44),
	"o2": Color(0.86, 0.95, 1.00),
	"co2": Color(0.70, 0.60, 0.90),
	"vapour": Color(0.97, 0.98, 1.00),
	"myco": Color(0.84, 0.66, 1.00),
}
## Forme de chaque sorte de flux, dans la scène comme dans la légende (LegendGlyph) : la couleur n'est jamais
## seule à distinguer deux flux (WCAG 2.2, SC 1.4.1).
const SHAPES: Dictionary = {
	"water": "dash", "minerals": "dot", "sugar": "rail", "o2": "disc", "co2": "ring", "vapour": "puff", "myco": "short",
}
## Numéro de forme des rubans (uniforms shape_a / shape_b du shader) : 0 tiret long, 1 point, 2 double trait,
## 3 tirets courts fins.
const RIBBON_SHAPE: Dictionary = {"dash": 0.0, "dot": 1.0, "rail": 2.0, "short": 3.0}
## Rates read as "full" display level, per m2 of crown projection and per day (calibration of the view).
const FULL_WATER_MM: float = 3.0
const FULL_N_G_M2: float = 0.05
const FULL_SUGAR_GC_M2: float = 8.0
const FULL_GPP_GC_M2: float = 8.0
const FULL_RESP_GC_M2: float = 1.5
const FULL_MYCO_GC_M2: float = 0.8

const RIBBON_GLSL: String = """
shader_type spatial;
render_mode unshaded, blend_mix, depth_draw_never, cull_disabled, shadows_disabled%s;
uniform float width = 0.02;
uniform float lift = 1.06;
uniform float lateral = 0.0;
uniform float spacing = 0.3;
uniform float phase = 0.0;
uniform float dir = 1.0;
uniform float level = 0.0;
uniform float level_b = 0.0;
uniform vec3 col_a : source_color = vec3(0.36, 0.82, 1.0);
uniform vec3 col_b : source_color = vec3(1.0, 0.78, 0.3);
uniform float on_a = 1.0;
uniform float on_b = 1.0;
uniform float shape_a = 0.0;
uniform float shape_b = 0.0;
uniform float base_line = 0.0;
uniform float alpha_scale = 1.0;
uniform float ghost = 0.0;
uniform float clip_radius = 1000.0;
uniform float clip_depth = 1000.0;
uniform int up_n = 0;
uniform float up_bottom[24];
uniform float up_rel[24];
varying float v_side;
varying float v_s;
varying float v_w;
varying float v_key;
varying float v_part;
varying vec3 v_wpos;
float local_uptake(float d) {
	float v = 1.0;
	for (int i = 0; i < 24; i++) {
		if (i >= up_n) break;
		v = up_rel[i];
		if (d < up_bottom[i]) break;
	}
	return v;
}
float fhash(vec2 p) {
	vec3 q = fract(vec3(p.xyx) * 0.1031);
	q += dot(q, q.yzx + 33.33);
	return fract((q.x + q.y) * q.z);
}
void vertex() {
	vec3 p = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	vec3 t = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
	vec3 cam = INV_VIEW_MATRIX[3].xyz;
	vec3 tc = cam - p;
	vec3 o = tc - t * dot(tc, t);
	float ol = length(o);
	o = ol > 1e-5 ? o / ol : vec3(0.0, 1.0, 0.0);
	vec3 side = normalize(cross(t, o) + 1e-6);
	float n = COLOR.a * 8.0;
	float th = (COLOR.r - 0.5) * (n > 1.5 ? 1.7 : 0.0) + lateral;
	float rad = UV2.y * lift + width * 0.8;
	vec3 radial = o * cos(th) + side * sin(th);
	vec3 across = normalize(cross(t, radial) + 1e-6);
	float wd = width * mix(0.75, 1.35, sqrt(clamp(UV2.x, 0.0, 1.0)));
	vec3 c = p + radial * rad + across * UV.y * wd;
	if (COLOR.g > 0.25) {
		// Roots and hyphae stay under the ground surface (the push toward the camera must not lift a
		// shallow root's dashes onto the grass).
		c.y = min(c.y, -0.012);
	}
	v_side = UV.y;
	v_s = UV.x;
	v_w = UV2.x;
	v_key = COLOR.b;
	v_part = COLOR.g;
	v_wpos = c;
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(c, 1.0);
}
void fragment() {
	if (length(v_wpos.xz) > clip_radius || -v_wpos.y > clip_depth) {
		discard;
	}
	if (ghost > 0.5 && v_wpos.y < 0.02) {
		discard;
	}
	float x = v_s / spacing - phase * dir + v_key * 7.31;
	float cell = floor(x);
	float f = fract(x);
	float local = v_part > 0.25 && v_part < 0.75 ? 0.35 + 0.65 * local_uptake(-v_wpos.y) : 1.0;
	float carry = (0.50 + 0.50 * sqrt(clamp(v_w, 0.0, 1.0))) * local;
	float isb = step(0.5, fhash(vec2(cell + 7.0, v_key * 97.0 + 3.0)));
	float lvl = mix(level, level_b, isb);
	float on = mix(on_a, on_b, isb);
	float lit = step(fhash(vec2(cell, v_key * 131.0)), clamp(lvl * carry * 1.5, 0.0, 0.85)) * on;
	float d = abs(f - 0.5);
	float a = abs(v_side);
	// Forme du trait (SHAPES) : 0 tiret long, 1 point, 2 double trait, 3 tirets courts fins.
	float shp = mix(shape_a, shape_b, isb);
	float core;
	float halo;
	if (shp < 0.5) {
		core = (1.0 - smoothstep(0.13, 0.23, d)) * (1.0 - smoothstep(0.22, 0.48, a));
		halo = (1.0 - smoothstep(0.19, 0.36, d)) * (1.0 - smoothstep(0.35, 1.0, a));
	} else if (shp < 1.5) {
		core = (1.0 - smoothstep(0.035, 0.065, d)) * (1.0 - smoothstep(0.30, 0.55, a));
		halo = (1.0 - smoothstep(0.06, 0.11, d)) * (1.0 - smoothstep(0.45, 1.0, a));
	} else if (shp < 2.5) {
		float rail = 1.0 - smoothstep(0.07, 0.15, abs(a - 0.55));
		core = (1.0 - smoothstep(0.20, 0.30, d)) * rail;
		halo = (1.0 - smoothstep(0.26, 0.40, d)) * (1.0 - smoothstep(0.75, 1.0, a));
	} else {
		core = (1.0 - smoothstep(0.06, 0.10, d)) * (1.0 - smoothstep(0.12, 0.30, a));
		halo = (1.0 - smoothstep(0.09, 0.16, d)) * (1.0 - smoothstep(0.25, 0.7, a));
	}
	vec3 col = mix(col_a, col_b, isb);
	float line = base_line * (1.0 - smoothstep(0.1, 0.6, a)) * on_a;
	float alpha = max(max(halo * 0.55, core) * lit, line);
	ALBEDO = mix(col * 0.16, col * 1.15, max(core * lit, line * 0.7));
	ALPHA = clamp(alpha * alpha_scale, 0.0, 1.0);
}
"""

## Gas particles. INSTANCE_CUSTOM: x = particle kind (0 CO2 into leaf, 1 O2 out of leaf, 2 vapour,
## 3 CO2 out of bark, 4 O2 into root, 5 CO2 out of root), y = random 0..1, z = site weight 0..1.
const GAS_GLSL: String = """
shader_type spatial;
render_mode unshaded, blend_mix, depth_draw_never, cull_disabled, shadows_disabled;
uniform float rate[6];
uniform float on[6];
uniform vec3 colors[6];
uniform float phase = 0.0;
uniform float size = 0.05;
uniform float reach = 0.5;
uniform float clip_radius = 1000.0;
uniform float clip_depth = 1000.0;
varying float glow;
varying vec3 v_col;
varying vec3 v_wpos;
varying float v_soft;
varying float v_ring;
float fhash(vec2 p) {
	vec3 q = fract(vec3(p.xyx) * 0.1031);
	q += dot(q, q.yzx + 33.33);
	return fract((q.x + q.y) * q.z);
}
void vertex() {
	int k = int(INSTANCE_CUSTOM.x + 0.5);
	float rnd = INSTANCE_CUSTOM.y;
	float w = INSTANCE_CUSTOM.z;
	vec3 site = MODEL_MATRIX[3].xyz;
	float speed = k == 2 ? 0.10 : 0.22;
	float cyc = phase * speed + rnd * 7.31;
	float t = fract(cyc);
	vec3 off = normalize(vec3(fhash(vec2(rnd * 91.0, 1.0)) - 0.5, fhash(vec2(rnd * 57.0, 2.0)) - 0.5,
		fhash(vec2(rnd * 33.0, 3.0)) - 0.5) + 1e-4);
	bool inward = k == 0 || k == 4;
	float r = reach * (0.5 + 0.5 * fhash(vec2(rnd * 13.0, 4.0)));
	vec3 p;
	if (k == 2) {
		// Vapour: rises and spreads above the leaves.
		p = site + vec3(off.x * 0.6, 0.6 + abs(off.y), off.z * 0.6) * r * 1.6 * t;
	} else if (inward) {
		p = site + off * r * (1.0 - t);
	} else {
		p = site + (off + vec3(0.0, k == 1 ? 0.5 : 0.25, 0.0)) * r * t;
	}
	if (k >= 4) {
		p.y = min(p.y, -0.02);
	}
	float alive = step(fhash(vec2(rnd * 71.0, floor(cyc))), rate[k] * (0.35 + 0.65 * w) * 1.2) * on[k];
	glow = alive * sin(3.14159 * t);
	v_col = colors[k];
	v_soft = k == 2 ? 1.0 : 0.0;
	v_ring = (k == 0 || k == 3 || k == 5) ? 1.0 : 0.0;  // CO₂ : anneau creux ; O₂ : disque plein
	v_wpos = p;
	vec3 right = INV_VIEW_MATRIX[0].xyz;
	vec3 up = INV_VIEW_MATRIX[1].xyz;
	float s = size * (k == 2 ? 2.6 : 1.0) * (alive > 0.5 ? 1.0 : 0.0);
	vec3 wp = p + (right * VERTEX.x + up * VERTEX.y) * s;
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(wp, 1.0);
	UV = VERTEX.xy + 0.5;
}
void fragment() {
	if (v_wpos.y < 0.0 && (length(v_wpos.xz) > clip_radius || -v_wpos.y > clip_depth)) {
		discard;
	}
	float r = length(UV - 0.5) * 2.0;
	float core = 1.0 - smoothstep(v_soft > 0.5 ? 0.0 : 0.25, v_soft > 0.5 ? 1.0 : 0.55, r);
	float halo = 1.0 - smoothstep(0.5, 1.0, r);
	if (v_ring > 0.5) {
		core = 1.0 - smoothstep(0.10, 0.20, abs(r - 0.55));
		halo = 1.0 - smoothstep(0.22, 0.40, abs(r - 0.55));
	}
	float a = v_soft > 0.5 ? core * 0.35 : max(core, halo * 0.5);
	ALBEDO = mix(v_col * 0.18, v_col * 1.15, v_soft > 0.5 ? 1.0 : core);
	ALPHA = clamp(a * glow, 0.0, 1.0);
}
"""

var enabled: bool = true
## Per-kind toggles (KINDS order).
var kind_on: Dictionary = {}
## Filtre d'un calque (Hud) : si non vide, seules ces sortes sont montrées (en plus des cases du panneau).
var only: PackedStringArray = PackedStringArray()
## Last values from TreeSim.get_flux (real units), for the panel.
var values: Dictionary = {}
var levels: Dictionary = {}
var phases: Dictionary = {"xylem": 0.0, "phloem": 0.0, "myco": 0.0, "gas": 0.0}
var sugar_dir: float = -1.0
var geometry_dirty: bool = true
var _geo_age: float = 10.0
var _val_age: float = 10.0
var _shoot_mi: MeshInstance3D
var _root_mi: MeshInstance3D
var _hypha_mi: MeshInstance3D
var _gas_mi: MultiMeshInstance3D
var _mats: Dictionary = {}   # name -> ShaderMaterial
var _gas_mat: ShaderMaterial


func _ready() -> void:
	for k: String in KINDS:
		kind_on[k] = true
		levels[k] = 0.0
	var ribbon: Shader = Shader.new()
	ribbon.code = RIBBON_GLSL % ""
	var ghost_sh: Shader = Shader.new()
	ghost_sh.code = RIBBON_GLSL % ", depth_test_disabled"
	# Pass chain on each path mesh: xylem -> phloem -> dim x-ray xylem -> dim x-ray phloem.
	for name: String in ["xylem", "phloem", "xylem_x", "phloem_x", "hypha", "hypha_in"]:
		var m: ShaderMaterial = ShaderMaterial.new()
		m.shader = ghost_sh if name.ends_with("_x") else ribbon
		m.render_priority = 2 if name.ends_with("_x") else 1
		_mats[name] = m
	for name: String in ["xylem", "xylem_x"]:
		_mats[name].set_shader_parameter("col_a", COLORS["water"])
		_mats[name].set_shader_parameter("col_b", COLORS["minerals"])
		_mats[name].set_shader_parameter("shape_a", RIBBON_SHAPE[SHAPES["water"]])
		_mats[name].set_shader_parameter("shape_b", RIBBON_SHAPE[SHAPES["minerals"]])
		_mats[name].set_shader_parameter("lift", 1.04)
	for name: String in ["phloem", "phloem_x"]:
		_mats[name].set_shader_parameter("col_a", COLORS["sugar"])
		_mats[name].set_shader_parameter("col_b", COLORS["sugar"])
		_mats[name].set_shader_parameter("shape_a", RIBBON_SHAPE[SHAPES["sugar"]])
		_mats[name].set_shader_parameter("shape_b", RIBBON_SHAPE[SHAPES["sugar"]])
		_mats[name].set_shader_parameter("lift", 1.16)
		_mats[name].set_shader_parameter("lateral", 0.55)
	for name: String in ["xylem_x", "phloem_x"]:
		_mats[name].set_shader_parameter("ghost", 1.0)
		_mats[name].set_shader_parameter("alpha_scale", 0.38)
	_mats["xylem"].next_pass = _mats["phloem"]
	_mats["phloem"].next_pass = _mats["xylem_x"]
	_mats["xylem_x"].next_pass = _mats["phloem_x"]
	var hm: ShaderMaterial = _mats["hypha"]
	hm.set_shader_parameter("col_a", COLORS["myco"])
	hm.set_shader_parameter("col_b", Color(0.97, 0.94, 1.0))
	hm.set_shader_parameter("lift", 0.0)
	hm.set_shader_parameter("base_line", 0.22)
	for m: ShaderMaterial in [hm, _mats["hypha_in"]]:
		m.set_shader_parameter("shape_a", RIBBON_SHAPE[SHAPES["myco"]])
		m.set_shader_parameter("shape_b", RIBBON_SHAPE[SHAPES["myco"]])
	var hin: ShaderMaterial = _mats["hypha_in"]
	hin.set_shader_parameter("col_a", Color(0.97, 0.94, 1.0))
	hin.set_shader_parameter("col_b", Color(0.97, 0.94, 1.0))
	hin.set_shader_parameter("lift", 0.0)
	hin.set_shader_parameter("lateral", 1.2)
	hm.next_pass = hin
	_shoot_mi = _ribbon_instance("FluxShoots", _mats["xylem"])
	_root_mi = _ribbon_instance("FluxRoots", _mats["xylem"])
	_hypha_mi = _ribbon_instance("FluxHyphae", hm)

	_gas_mi = MultiMeshInstance3D.new()
	_gas_mi.name = "FluxGas"
	var mm: MultiMesh = MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_custom_data = true
	var q: QuadMesh = QuadMesh.new()
	q.size = Vector2(1.0, 1.0)
	mm.mesh = q
	_gas_mi.multimesh = mm
	var gs: Shader = Shader.new()
	gs.code = GAS_GLSL
	_gas_mat = ShaderMaterial.new()
	_gas_mat.shader = gs
	_gas_mat.render_priority = 3
	var cols: PackedVector3Array = PackedVector3Array()
	for k: String in ["co2", "o2", "vapour", "co2", "o2", "co2"]:
		var c: Color = COLORS[k]
		cols.append(Vector3(c.r, c.g, c.b))
	_gas_mat.set_shader_parameter("colors", cols)
	_gas_mi.material_override = _gas_mat
	_gas_mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(_gas_mi)
	var env: String = OS.get_environment("DENDRO_FLUX_KINDS")
	if env != "":
		set_kinds_from_string(env)


func _ribbon_instance(n: String, m: Material) -> MeshInstance3D:
	var mi: MeshInstance3D = MeshInstance3D.new()
	mi.name = n
	mi.material_override = m
	mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	mi.extra_cull_margin = 4.0
	add_child(mi)
	return mi


## "all", "none" or a comma list of KINDS (e.g. "water,sugar,o2").
func set_kinds_from_string(s: String) -> void:
	var want: PackedStringArray = s.to_lower().replace(" ", "").split(",", false)
	for k: String in KINDS:
		kind_on[k] = s == "all" or want.has(k)


func set_kind(kind: String, on: bool) -> void:
	if kind_on.has(kind):
		kind_on[kind] = on


## La sorte `kind` est-elle montrée (case du panneau et filtre du calque) ?
func is_shown(kind: String) -> bool:
	return bool(kind_on.get(kind, true)) and (only.is_empty() or only.has(kind))


func set_enabled(on: bool) -> void:
	enabled = on
	visible = on


func set_clip(radius: float, depth: float) -> void:
	for m: ShaderMaterial in _all_mats():
		m.set_shader_parameter("clip_radius", radius)
		m.set_shader_parameter("clip_depth", depth)


func set_uptake_profile(n: int, bottoms: PackedFloat32Array, rel: PackedFloat32Array) -> void:
	for name: String in ["xylem", "xylem_x"]:
		_mats[name].set_shader_parameter("up_n", n)
		_mats[name].set_shader_parameter("up_bottom", bottoms)
		_mats[name].set_shader_parameter("up_rel", rel)


func _all_mats() -> Array[ShaderMaterial]:
	var out: Array[ShaderMaterial] = [_gas_mat]
	for m: ShaderMaterial in _mats.values():
		out.append(m)
	return out


## Per frame. Values are re-read ~10 times per second; geometry when the tree changed (at most twice a s).
func update(sim: TreeSim, delta: float, playing: bool, view_dist: float) -> void:
	_val_age += delta
	if _val_age >= 0.1:
		_val_age = 0.0
		_read_values(sim, false)
	_geo_age += delta
	if geometry_dirty and _geo_age >= (1.5 if playing else 0.3) and enabled:
		rebuild_geometry(sim)
	if playing:
		# Display speeds: phloem much slower than xylem (Windt et al. 2006: ~0.3 mm/s phloem vs much
		# faster midday xylem flow); compressed for legibility.
		phases["xylem"] += delta * (0.25 + 1.2 * sqrt(levels["water"]))
		phases["phloem"] += delta * (0.15 + 0.35 * sqrt(levels["sugar"]))
		phases["myco"] += delta * (0.12 + 0.25 * sqrt(levels["myco"]))
		phases["gas"] += delta * 1.0
	_push(view_dist)


## Jumps to the current state (start-up, screenshots, tests).
func snap(sim: TreeSim) -> void:
	_read_values(sim, true)
	rebuild_geometry(sim)


func rebuild_geometry(sim: TreeSim) -> void:
	var t0: int = Time.get_ticks_usec()
	_rebuild_geometry(sim)
	Prof.add("gd.flux_geometry", t0)


func _rebuild_geometry(sim: TreeSim) -> void:
	geometry_dirty = false
	_geo_age = 0.0
	var g: Array = sim.build_flux_geometry()
	_shoot_mi.mesh = g[0]
	_root_mi.mesh = g[1]
	_hypha_mi.mesh = g[2]
	var pos: PackedVector3Array = g[3]
	var info: PackedFloat32Array = g[4]
	var mm: MultiMesh = _gas_mi.multimesh
	# Instances: leaf sites x3 (CO2 in, O2 out, vapour), bark x1 (CO2 out), roots x2 (O2 in, CO2 out).
	# Filled as one buffer (12 floats of transform + 4 of custom data per instance).
	var buf: PackedFloat32Array = PackedFloat32Array()
	var count: int = 0
	var bb: AABB = AABB()
	var h: int = 0
	for i: int in pos.size():
		var kind: int = int(info[i * 2])
		var w: float = info[i * 2 + 1]
		var subs: Array = [0, 1, 2] if kind == 0 else ([3] if kind == 1 else [4, 5])
		var p: Vector3 = pos[i]
		bb = AABB(p, Vector3.ZERO) if i == 0 else bb.expand(p)
		for sk: int in subs:
			h = (h * 1103515245 + 12345) & 0x7fffffff
			buf.append_array([1.0, 0.0, 0.0, p.x, 0.0, 1.0, 0.0, p.y, 0.0, 0.0, 1.0, p.z,
				float(sk), float(h % 10007) / 10007.0, w, 0.0])
			count += 1
	mm.instance_count = 0
	mm.instance_count = count
	if count > 0:
		mm.buffer = buf
	_gas_mi.custom_aabb = bb.grow(4.0)


func _read_values(sim: TreeSim, snap_now: bool) -> void:
	values = sim.get_flux()
	var area: float = maxf(float(values.get("crown_area_m2", 1.0)), 0.01)
	var target: Dictionary = {
		"water": clampf(float(values["xylem_l"]) / area / FULL_WATER_MM, 0.0, 1.0),
		"minerals": clampf(float(values["n_xylem_g"]) / area / FULL_N_G_M2, 0.0, 1.0),
		"sugar": clampf((float(values["phloem_down_gc"]) + float(values["phloem_up_gc"])) / area / FULL_SUGAR_GC_M2, 0.0, 1.0),
		"o2": clampf(float(values["o2_out_g"]) * 12.011 / 31.998 / area / FULL_GPP_GC_M2, 0.0, 1.0),
		"co2": clampf(float(values["co2_in_g"]) * 12.011 / 44.009 / area / FULL_GPP_GC_M2, 0.0, 1.0),
		"vapour": clampf(float(values["transpiration_l"]) / area / FULL_WATER_MM, 0.0, 1.0),
		"myco": clampf(float(values["alloc_myco_gc"]) / area / FULL_MYCO_GC_M2, 0.0, 1.0),
		"resp_wood": clampf(float(values["co2_out_wood_g"]) * 12.011 / 44.009 / area / FULL_RESP_GC_M2, 0.0, 1.0),
		"resp_root": clampf(float(values["co2_out_root_g"]) * 12.011 / 44.009 / area / FULL_RESP_GC_M2, 0.0, 1.0),
	}
	var k: float = 1.0 if snap_now else 0.35
	for key: String in target:
		levels[key] = lerpf(float(levels.get(key, 0.0)), target[key], k)
		if target[key] == 0.0 and levels[key] < 0.004:
			levels[key] = 0.0
	var up: float = float(values["phloem_up_gc"])
	var dn: float = float(values["phloem_down_gc"])
	if up + dn > 0.0:
		sugar_dir = 1.0 if up > dn else -1.0


func _push(view_dist: float) -> void:
	var spacing: float = pow(2.0, roundf(log(clampf(view_dist * 0.02, 0.05, 2.4)) / log(2.0)))
	var width: float = clampf(view_dist * 0.003, 0.005, 0.12)
	var on_w: float = 1.0 if is_shown("water") else 0.0
	var on_m: float = 1.0 if is_shown("minerals") else 0.0
	var on_s: float = 1.0 if is_shown("sugar") else 0.0
	for name: String in ["xylem", "xylem_x"]:
		var m: ShaderMaterial = _mats[name]
		m.set_shader_parameter("spacing", spacing)
		m.set_shader_parameter("width", width)
		m.set_shader_parameter("phase", phases["xylem"])
		m.set_shader_parameter("dir", 1.0)
		m.set_shader_parameter("level", levels["water"])
		# Minerals ride with the water: their dashes need moving sap.
		m.set_shader_parameter("level_b", minf(levels["minerals"], levels["water"] * 2.0 + 0.05) * (1.0 if levels["water"] > 0.0 else 0.0))
		m.set_shader_parameter("on_a", on_w)
		m.set_shader_parameter("on_b", on_m)
	for name: String in ["phloem", "phloem_x"]:
		var m: ShaderMaterial = _mats[name]
		m.set_shader_parameter("spacing", spacing * 0.8)
		m.set_shader_parameter("width", width * 0.85)
		m.set_shader_parameter("phase", phases["phloem"])
		m.set_shader_parameter("dir", sugar_dir)
		m.set_shader_parameter("level", levels["sugar"])
		m.set_shader_parameter("level_b", levels["sugar"])
		m.set_shader_parameter("on_a", on_s)
		m.set_shader_parameter("on_b", on_s)
	var hm: ShaderMaterial = _mats["hypha"]
	var on_y: float = 1.0 if is_shown("myco") else 0.0
	var act: float = float(values.get("myco_activity", 0.0))
	hm.set_shader_parameter("spacing", spacing * 0.35)
	hm.set_shader_parameter("width", width * 0.6)
	hm.set_shader_parameter("phase", phases["myco"])
	# Outward: carbon to the fungus (violet); inward, on the same filament: P, N and a little water to
	# the root (pale dashes, second pass).
	hm.set_shader_parameter("dir", 1.0)
	hm.set_shader_parameter("level", levels["myco"])
	hm.set_shader_parameter("level_b", levels["myco"])
	hm.set_shader_parameter("on_a", on_y)
	hm.set_shader_parameter("on_b", on_y)
	hm.set_shader_parameter("base_line", 0.22 * clampf(0.3 + act, 0.0, 1.0))
	var hin: ShaderMaterial = _mats["hypha_in"]
	var feed: float = clampf(act * 0.7, 0.0, 1.0) * (1.0 if float(values.get("p_uptake_g", 0.0)) > 0.0 else 0.0)
	hin.set_shader_parameter("spacing", spacing * 0.35)
	hin.set_shader_parameter("width", width * 0.55)
	hin.set_shader_parameter("phase", phases["myco"] * 1.3)
	hin.set_shader_parameter("dir", -1.0)
	hin.set_shader_parameter("level", feed)
	hin.set_shader_parameter("level_b", feed)
	hin.set_shader_parameter("on_a", on_y)
	hin.set_shader_parameter("on_b", on_y)
	var rates: PackedFloat32Array = PackedFloat32Array([levels["co2"], levels["o2"], levels["vapour"], levels["resp_wood"],
		levels["resp_root"] * (1.0 - float(values.get("root_hypoxia", 0.0))), levels["resp_root"]])
	var ons: PackedFloat32Array = PackedFloat32Array()
	for kk: String in ["co2", "o2", "vapour", "co2", "o2", "co2"]:
		ons.append(1.0 if is_shown(kk) else 0.0)
	_gas_mat.set_shader_parameter("rate", rates)
	_gas_mat.set_shader_parameter("on", ons)
	_gas_mat.set_shader_parameter("phase", phases["gas"])
	_gas_mat.set_shader_parameter("size", clampf(view_dist * 0.006, 0.03, 0.35))
	_gas_mat.set_shader_parameter("reach", clampf(view_dist * 0.012, 0.15, 1.6))
