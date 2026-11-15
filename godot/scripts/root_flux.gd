class_name RootFlux
extends Node3D
## Roots: the bark material of the structural and fine roots, and the water / nutrient flux drawn on them.
##
## Flux = small luminous dashes travelling along every root toward the trunk base (an emissive term of
## the root shader, using the along-root path coordinate the core writes into UV2, see MeshData::uv2),
## plus a few motes drifting from the surrounding soil into fine-root tufts (one MultiMesh, one draw
## call). Everything follows the simulation: brightness, dash density and speed scale with the day's
## root water uptake (TreeSim.get_water_uptake_mm: 0 in winter dormancy, strong on a warm day in full
## leaf, weak in drought), and per depth with the uptake per unit root length of each soil layer
## (TreeSim.get_uptake_profile: moist layers deliver more). The dash phase only advances while the
## simulation plays.

const MAX_LAYERS: int = 24
const MOTES: int = 90
const STRONG_UPTAKE_MM: float = 3.0   # mm/day read as "full flux"

## Shared by the root shader and the mote shader: per-depth uptake lookup.
const UPTAKE_GLSL: String = """
uniform float flux_on = 1.0;
uniform float flux_level = 0.0;     // 0..1 (smoothed daily uptake)
uniform float flux_phase = 0.0;     // dash phase (dashes travelled), advances only while playing
uniform float flux_spacing = 0.3;   // dash spacing (m), follows the view distance
uniform int up_n = 0;
uniform float up_bottom[24];
uniform float up_rel[24];
float local_uptake(float d) {
	float v = 0.0;
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
"""

const ROOT_GLSL: String = """
shader_type spatial;
render_mode cull_disabled;
uniform sampler2D albedo_tex : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D normal_tex : hint_normal, filter_linear_mipmap_anisotropic, repeat_enable;
uniform float clip_radius = 1000.0;
uniform float clip_depth = 1000.0;
uniform float fine_style = 0.0;
uniform vec3 root_tint = vec3(1.40, 1.18, 0.92);
uniform vec4 cap_color : source_color = vec4(0.36, 0.25, 0.15, 1.0);
varying vec3 wpos;
%s
void vertex() {
	wpos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}
vec3 flux(float path, float key, float rad) {
	float lvl = flux_level * flux_on;
	if (lvl < 0.004) {
		return vec3(0.0);
	}
	float local = local_uptake(-wpos.y);
	// Pipe model: thick roots carry the water of everything beyond them.
	float carry = fine_style > 0.5 ? 0.25 + 0.30 * local : clamp(0.45 + 0.55 * sqrt(rad / 0.04), 0.45, 1.0) * (0.45 + 0.55 * local);
	float sp = flux_spacing * (fine_style > 0.5 ? 0.45 : 1.0);
	float s = path / sp + flux_phase * (fine_style > 0.5 ? 1.6 : 1.0) + key * 0.618;
	float cell = floor(s);
	float f = fract(s);
	float h = fhash(vec2(cell, key));
	float lit = step(h, clamp(lvl * carry * 1.1, 0.0, 0.75));
	float dash = fine_style > 0.5 ? 1.0 - smoothstep(0.10, 0.22, abs(f - 0.5)) : 1.0 - smoothstep(0.16, 0.30, abs(f - 0.5));
	float mineral = step(0.86, fhash(vec2(cell + 7.0, key + 3.0)));
	vec3 water = vec3(0.25, 0.75, 1.00);
	vec3 gold = vec3(1.00, 0.72, 0.22);
	float bright = (0.5 + 0.9 * sqrt(lvl)) * (0.55 + 0.45 * local) * (fine_style > 0.5 ? 0.7 : 1.0);
	// Dashes merge into the stem at the collar (no bright knot where every root starts) and stay
	// discreet as rings around very thick roots.
	bright *= smoothstep(0.1, 0.7, path) * mix(1.0, 0.45, smoothstep(0.04, 0.2, rad));
	return mix(water, gold, mineral) * dash * lit * bright;
}
void fragment() {
	if (length(wpos.xz) > clip_radius || -wpos.y > clip_depth) {
		discard;
	}
	if (!FRONT_FACING) {
		ALBEDO = cap_color.rgb;
		ROUGHNESS = 1.0;
		SPECULAR = 0.0;
	} else {
		vec3 a = texture(albedo_tex, UV).rgb * COLOR.rgb * root_tint;
		ALBEDO = mix(a, mix(a, vec3(0.80, 0.70, 0.52), 0.55), fine_style);
		NORMAL_MAP = texture(normal_tex, UV).xyz;
		NORMAL_MAP_DEPTH = 0.7;
		ROUGHNESS = 0.88;
		SPECULAR = 0.12;
		EMISSION = flux(UV2.x, floor(UV2.y), fract(UV2.y));
	}
}
"""

## Motes: instance origin = a fine-root point (target); INSTANCE_CUSTOM.x = random 0..1. Each mote starts
## up to ~0.5 m away in the soil and drifts into its target over its cycle, then restarts elsewhere.
const MOTE_GLSL: String = """
shader_type spatial;
render_mode unshaded, blend_add, depth_draw_never, cull_disabled, shadows_disabled;
uniform float mote_size = 0.05;
uniform float mote_reach = 0.5;
varying float glow;
%s
void vertex() {
	float rnd = INSTANCE_CUSTOM.x;
	vec3 target = MODEL_MATRIX[3].xyz;
	float t = fract(flux_phase * 0.22 + rnd * 7.31);
	vec3 off = normalize(vec3(fhash(vec2(rnd * 91.0, 1.0)) - 0.5, fhash(vec2(rnd * 57.0, 2.0)) - 0.5,
		fhash(vec2(rnd * 33.0, 3.0)) - 0.5) + 1e-4);
	vec3 p = target + off * mote_reach * (1.0 - t) * (0.5 + 0.5 * fhash(vec2(rnd * 13.0, 4.0)));
	p.y = min(p.y, -0.02);
	float local = local_uptake(-p.y);
	float lvl = flux_level * flux_on;
	float alive = step(fhash(vec2(rnd * 71.0, floor(flux_phase * 0.22 + rnd * 7.31))), lvl * (0.3 + 0.7 * local) * 1.4);
	glow = alive * sin(3.14159 * t) * (0.6 + 1.4 * sqrt(lvl));
	// Camera-facing quad around p.
	vec3 right = INV_VIEW_MATRIX[0].xyz;
	vec3 up = INV_VIEW_MATRIX[1].xyz;
	vec3 wp = p + (right * VERTEX.x + up * VERTEX.y) * mote_size * (alive > 0.5 ? 1.0 : 0.0);
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(wp, 1.0);
	UV = VERTEX.xy + 0.5;
}
void fragment() {
	float r = length(UV - 0.5) * 2.0;
	float a = (1.0 - smoothstep(0.2, 1.0, r)) * glow;
	ALBEDO = vec3(0.30, 0.62, 0.80) * a;
	ALPHA = 1.0;
}
"""

var root_mat: ShaderMaterial
var fine_root_mat: ShaderMaterial
var enabled: bool = true
var level: float = 0.0        # smoothed flux level 0..1
var phase: float = 0.0
var spacing: float = 0.3
var _motes: MultiMeshInstance3D
var _mote_mat: ShaderMaterial
var _mote_src: Mesh = null
var _mote_refresh: float = 10.0
var _profile_age: float = 10.0
var _up_n: int = 0
var _up_bottom: PackedFloat32Array = PackedFloat32Array()
var _up_rel: PackedFloat32Array = PackedFloat32Array()


static func make_root_material(tex: Array[ImageTexture]) -> ShaderMaterial:
	var sh: Shader = Shader.new()
	sh.code = ROOT_GLSL % UPTAKE_GLSL
	var m: ShaderMaterial = ShaderMaterial.new()
	m.shader = sh
	m.set_shader_parameter("albedo_tex", tex[0])
	m.set_shader_parameter("normal_tex", tex[1])
	return m


func _init(bark_tex: Array[ImageTexture] = []) -> void:
	if bark_tex.size() >= 2:
		root_mat = make_root_material(bark_tex)
		fine_root_mat = make_root_material(bark_tex)
		fine_root_mat.set_shader_parameter("fine_style", 1.0)
	_up_bottom.resize(MAX_LAYERS)
	_up_rel.resize(MAX_LAYERS)


func _ready() -> void:
	_motes = MultiMeshInstance3D.new()
	var mm: MultiMesh = MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_custom_data = true
	var q: QuadMesh = QuadMesh.new()
	q.size = Vector2(1.0, 1.0)
	mm.mesh = q
	mm.instance_count = MOTES
	mm.visible_instance_count = 0
	_motes.multimesh = mm
	var sh: Shader = Shader.new()
	sh.code = MOTE_GLSL % UPTAKE_GLSL
	_mote_mat = ShaderMaterial.new()
	_mote_mat.shader = sh
	_motes.material_override = _mote_mat
	_motes.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(_motes)


func set_enabled(on: bool) -> void:
	enabled = on
	_push("flux_on", 1.0 if on else 0.0)
	_motes.visible = on


## Fine-root mesh to take the mote targets from (call after each fine-root rebuild; sampled lazily).
func set_fine_roots(mesh: Mesh) -> void:
	_mote_src = mesh


## Per frame. `playing`: the dash phase only moves while the simulation runs. `view_dist`: camera
## distance (m), sets the dash spacing so dashes stay legible from a seedling close-up to a 200-year oak.
func update(sim: TreeSim, delta: float, playing: bool, view_dist: float) -> void:
	var target: float = clampf(sim.get_water_uptake_mm() / STRONG_UPTAKE_MM, 0.0, 1.0)
	level = lerpf(level, target, 1.0 - exp(-2.5 * delta))
	if level < 0.002 and target == 0.0:
		level = 0.0
	if playing:
		phase += delta * (0.25 + 1.1 * sqrt(level))
	# Dash spacing: powers of two of ~1.2 % of the view distance (changes rarely, no sliding).
	spacing = pow(2.0, roundf(log(clampf(view_dist * 0.02, 0.06, 2.4)) / log(2.0)))
	_push("flux_level", level)
	_push("flux_phase", phase)
	_push("flux_spacing", spacing)
	_mote_mat.set_shader_parameter("mote_size", clampf(view_dist * 0.004, 0.02, 0.25))
	_mote_mat.set_shader_parameter("mote_reach", clampf(view_dist * 0.006, 0.12, 0.35))
	_profile_age += delta
	if _profile_age >= 0.1:
		_profile_age = 0.0
		_update_profile(sim)
	_mote_refresh += delta
	if _mote_src != null and _mote_refresh > 0.5:
		_refresh_motes()


## Jumps the smoothed level to the current uptake (start-up, screenshots, tests).
func snap(sim: TreeSim) -> void:
	level = clampf(sim.get_water_uptake_mm() / STRONG_UPTAKE_MM, 0.0, 1.0)
	_push("flux_level", level)
	_update_profile(sim)
	if _mote_src != null:
		_refresh_motes()


## Uptake per unit root length of each soil layer, relative to the best layer (dry layers fade out).
func _update_profile(sim: TreeSim) -> void:
	var p: PackedFloat32Array = sim.get_uptake_profile()
	var n: int = mini(p.size() / 3, MAX_LAYERS)
	var best: float = 0.0
	var per: PackedFloat32Array = PackedFloat32Array()
	per.resize(n)
	for i: int in n:
		var dens: float = p[i * 3 + 2]
		per[i] = p[i * 3 + 1] / dens if dens > 0.02 else 0.0
		best = maxf(best, per[i])
	for i: int in MAX_LAYERS:
		_up_bottom[i] = p[i * 3] if i < n else 99.0
		_up_rel[i] = (per[i] / best) if (i < n and best > 0.0) else 0.0
	if n > 0:
		_up_bottom[n - 1] = 99.0
	_up_n = n
	for m: ShaderMaterial in [root_mat, fine_root_mat, _mote_mat]:
		if m != null:
			m.set_shader_parameter("up_n", _up_n)
			m.set_shader_parameter("up_bottom", _up_bottom)
			m.set_shader_parameter("up_rel", _up_rel)


## Mote targets: deterministic sample of fine-root vertices.
func _refresh_motes() -> void:
	_mote_refresh = 0.0
	var src: Mesh = _mote_src
	_mote_src = null
	var mm: MultiMesh = _motes.multimesh
	if src == null or src.get_surface_count() == 0:
		mm.visible_instance_count = 0
		return
	var verts: PackedVector3Array = src.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
	if verts.is_empty():
		mm.visible_instance_count = 0
		return
	var rng: RandomNumberGenerator = RandomNumberGenerator.new()
	rng.seed = 7
	for i: int in MOTES:
		var v: Vector3 = verts[rng.randi() % verts.size()]
		mm.set_instance_transform(i, Transform3D(Basis.IDENTITY, v))
		mm.set_instance_custom_data(i, Color(rng.randf(), 0.0, 0.0, 0.0))
	mm.visible_instance_count = MOTES
	mm.custom_aabb = src.get_aabb().grow(1.5)
	_motes.custom_aabb = mm.custom_aabb


func _push(param: String, value: Variant) -> void:
	for m: ShaderMaterial in [root_mat, fine_root_mat, _mote_mat]:
		if m != null:
			m.set_shader_parameter(param, value)
