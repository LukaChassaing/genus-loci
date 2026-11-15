class_name SoilBlock
extends Node3D
## Museum-diorama ground plug: a cylinder of soil (grass / litter top) with a wedge removed, floating in
## space. The two radial cut faces meet on the trunk axis and show the soil horizons; the roots live in
## the wedge as real 3-D geometry (see ProcTextures.make_root_material, clipped at `radius`).
##
## Size rules: radius = max(1.25 x crown radius, 2.5 m), depth = root depth + margin (min 2 m); both ease
## toward their goals and never shrink until reset() (new seed).
##
## Horizons: described by up to MAX_LAYERS layers (bottom depth in m below the surface, kind, water
## fraction). `set_layers()` is THE entry point for real soil data; `default_layers()` is the placeholder.
## Conventions: azimuth a -> direction (sin a, 0, cos a); +z is azimuth 0.

const MIN_RADIUS: float = 2.5
const MIN_DEPTH: float = 2.0
const RADIUS_PER_CROWN: float = 1.25
const WEDGE_DEFAULT_DEG: float = 110.0
const MAX_LAYERS: int = 8
const ARC_SEGMENTS: int = 96

## Layer kinds (palette index in the shader and in the HUD legend).
const KIND_LITTER: int = 0
const KIND_A: int = 1
const KIND_B: int = 2
const KIND_C: int = 3
const KIND_ROCK: int = 4
const KIND_NAMES: Array[String] = ["Litière", "Terre arable (horizon A)", "Sous-sol (horizon B)",
	"Roche mère altérée (horizon C)", "Roche"]
const KIND_COLORS: Array[Color] = [Color(0.20, 0.13, 0.08), Color(0.30, 0.20, 0.12),
	Color(0.50, 0.36, 0.21), Color(0.66, 0.60, 0.46), Color(0.55, 0.56, 0.58)]

var radius: float = MIN_RADIUS
var depth: float = MIN_DEPTH
var wedge_dir: float = 0.45                       # azimuth of the wedge centre (faces the default camera)
var wedge_width: float = deg_to_rad(WEDGE_DEFAULT_DEG)
var xray: bool = false
var layer_count: int = 0
var layer_bottom: PackedFloat32Array = PackedFloat32Array()

var _goal_radius: float = MIN_RADIUS
var _goal_depth: float = MIN_DEPTH
var _soil_mi: MeshInstance3D
var _top_mi: MeshInstance3D
var _soil_mat: ShaderMaterial
var _soil_xray_mat: ShaderMaterial
var _top_mat: ShaderMaterial
var _top_xray_mat: ShaderMaterial

# ---------------------------------------------------------------------------------------------
# Shaders
# ---------------------------------------------------------------------------------------------

const NOISE_GLSL: String = """
float hash13(vec3 p) {
	p = fract(p * 0.1031);
	p += dot(p, p.zyx + 31.32);
	return fract((p.x + p.y) * p.z);
}
vec3 hash33(vec3 p) {
	p = fract(p * vec3(0.1031, 0.1030, 0.0973));
	p += dot(p, p.yxz + 33.33);
	return fract((p.xxy + p.yxx) * p.zyx);
}
float vnoise3(vec3 p) {
	vec3 i = floor(p);
	vec3 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	return mix(mix(mix(hash13(i), hash13(i + vec3(1, 0, 0)), f.x), mix(hash13(i + vec3(0, 1, 0)), hash13(i + vec3(1, 1, 0)), f.x), f.y),
		mix(mix(hash13(i + vec3(0, 0, 1)), hash13(i + vec3(1, 0, 1)), f.x), mix(hash13(i + vec3(0, 1, 1)), hash13(i + vec3(1, 1, 1)), f.x), f.y), f.z);
}
"""

## Horizon description shared by the cut and X-ray shaders. Data comes from SoilBlock.set_layers().
const LAYERS_GLSL: String = """
uniform int n_layers = 4;
uniform float layer_bottom[8];
uniform float layer_kind[8];
uniform float layer_water[8];
uniform float dry_front = 0.0;
uniform vec4 c_litter : source_color = vec4(0.20, 0.13, 0.08, 1.0);
uniform vec4 c_a : source_color = vec4(0.30, 0.20, 0.12, 1.0);
uniform vec4 c_b : source_color = vec4(0.50, 0.36, 0.21, 1.0);
uniform vec4 c_c : source_color = vec4(0.66, 0.60, 0.46, 1.0);
varying vec3 wpos;

vec3 kind_col(float k) {
	if (k < 0.5) return c_litter.rgb;
	if (k < 1.5) return c_a.rgb;
	if (k < 2.5) return c_b.rgb;
	if (k < 3.5) return c_c.rgb;
	return vec3(0.55, 0.56, 0.58); // rock
}
// Wavy horizon boundaries, continuous across faces (a function of the world position).
float soil_wave(vec3 p, float d) {
	float amp = 0.010 + 0.05 * clamp(d / 3.0, 0.0, 1.0);
	float n = vnoise3(vec3(p.x * 0.9, 0.0, p.z * 0.9)) * 0.65 + vnoise3(vec3(p.x * 3.1, 3.7, p.z * 3.1)) * 0.35;
	return (n - 0.5) * 2.0 * amp;
}
void soil_layers(float dd, out vec3 col, out float wet) {
	col = kind_col(layer_kind[0]);
	wet = layer_water[0];
	for (int i = 1; i < 8; i++) {
		if (i >= n_layers) break;
		float b = layer_bottom[i - 1];
		float s = 0.012 + 0.03 * b;
		float t = smoothstep(b - s, b + s, dd);
		col = mix(col, kind_col(layer_kind[i]), t);
		wet = mix(wet, layer_water[i], t);
	}
}
void vertex() {
	wpos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}
"""

const SOIL_CUT_GLSL: String = """
vec3 stones(vec3 col, vec3 sp, float dens, float rad) {
	vec3 ci = floor(sp);
	vec3 h = hash33(ci);
	float present = step(1.0 - dens, hash13(ci + 11.7));
	vec3 c = ci + 0.25 + 0.5 * h;
	float r = rad * (0.35 + 0.65 * h.z) * 0.5;
	float m = 1.0 - smoothstep(r * 0.8, r, length(sp - c));
	vec3 sc = mix(vec3(0.58, 0.54, 0.47), vec3(0.34, 0.32, 0.29), h.x) * (0.85 + 0.3 * h.y);
	return mix(col, sc, m * present);
}
void fragment() {
	float d = -wpos.y;
	float dd = d + soil_wave(wpos, d);
	vec3 col;
	float wet;
	soil_layers(dd, col, wet);
	// Grain, stretched along the strata.
	float g = vnoise3(wpos * vec3(8.0, 20.0, 8.0)) * 0.55 + vnoise3(wpos * vec3(26.0, 58.0, 26.0)) * 0.45;
	col *= 0.78 + 0.44 * g;
	// Humus: darker near the surface.
	col *= mix(0.68, 1.0, smoothstep(0.03, 0.55, dd));
	// Stones and gravel flecks, more frequent deeper.
	float dens = 0.03 + 0.42 * smoothstep(0.3, 1.5, dd);
	col = stones(col, wpos * 4.0, dens, 0.55);
	col = stones(col, wpos * 13.0 + 17.0, dens * 1.4, 0.40);
	// Moisture: wet layers darker and more saturated; a drying front paler from the top.
	float w = clamp(wet, 0.0, 1.0);
	float lum = dot(col, vec3(0.30, 0.59, 0.11));
	col = mix(col * 1.10, col * 0.62, w);
	col = mix(vec3(lum), col, 0.78 + 0.5 * w);
	float front = 1.0 - smoothstep(dry_front - 0.15, dry_front + 0.04, dd);
	col = mix(col, mix(col, vec3(0.80, 0.72, 0.57), 0.5), front * step(0.01, dry_front));
	// Metre ticks.
	float k = abs(fract(d) - 0.5);
	float line = (1.0 - smoothstep(0.0, fwidth(d) * 1.5 + 0.003, 0.5 - k)) * step(0.3, d);
	col = mix(col, col * 0.62, line * 0.40);
	ALBEDO = col;
	ROUGHNESS = 1.0;
	SPECULAR = 0.0;
}
"""

const SOIL_XRAY_GLSL: String = """
void fragment() {
	float d = -wpos.y;
	float dd = d + soil_wave(wpos, d);
	vec3 col;
	float wet;
	soil_layers(dd, col, wet);
	float fres = pow(1.0 - abs(dot(normalize(NORMAL), normalize(VIEW))), 2.2);
	float ring = 0.0;
	for (int i = 0; i < 7; i++) {
		if (i >= n_layers - 1) break;
		ring = max(ring, 1.0 - smoothstep(0.0, 0.010 + fwidth(dd) * 1.6, abs(dd - layer_bottom[i])));
	}
	vec3 tint = mix(col * 0.95, vec3(0.62, 0.70, 0.78), 0.42);
	ALBEDO = mix(tint, vec3(0.30, 0.20, 0.10), ring);
	ALPHA = clamp(0.10 + 0.50 * fres + 0.75 * ring, 0.0, 0.95);
}
"""

## Plug top: grass / litter, driven by the season (set_ground()).
const TOP_GLSL: String = """
uniform float vigor = 1.0;      // 0 = dormant dull grass, 1 = fresh green
uniform float fresh = 0.5;      // 0 = deep summer green, 1 = bright spring green
uniform float dryness = 0.0;
uniform float litter = 0.0;
uniform float frost = 0.0;
uniform float crown_r = 3.0;
uniform float plug_r = 3.0;
uniform float xray_alpha = 1.0;
varying vec3 wpos;

float hash13(vec3 p) {
	p = fract(p * 0.1031);
	p += dot(p, p.zyx + 31.32);
	return fract((p.x + p.y) * p.z);
}
vec3 hash33(vec3 p) {
	p = fract(p * vec3(0.1031, 0.1030, 0.0973));
	p += dot(p, p.yxz + 33.33);
	return fract((p.xxy + p.yxx) * p.zyx);
}
float vn(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	float a = hash13(vec3(i, 1.0));
	float b = hash13(vec3(i + vec2(1, 0), 1.0));
	float c = hash13(vec3(i + vec2(0, 1), 1.0));
	float d = hash13(vec3(i + vec2(1, 1), 1.0));
	return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}
void vertex() {
	wpos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}
void fragment() {
	vec2 p = wpos.xz;
	float r = length(p);
	float patch = vn(p * 0.7) * 0.6 + vn(p * 2.3) * 0.4;
	float blades = vn(p * vec2(38.0, 31.0)) * 0.6 + vn(p * 90.0) * 0.4;
	vec3 deep = vec3(0.10, 0.22, 0.06);
	vec3 bright = vec3(0.20, 0.37, 0.10);
	vec3 grass = mix(deep, bright, clamp(fresh + (patch - 0.5) * 0.6, 0.0, 1.0));
	vec3 dormant = vec3(0.27, 0.25, 0.14);
	grass = mix(dormant, grass, vigor);
	vec3 straw = vec3(0.46, 0.38, 0.19);
	grass = mix(grass, straw * (0.85 + 0.3 * patch), dryness);
	vec3 col = grass * (0.80 + 0.40 * blades);
	// Leaf litter under (and a bit beyond) the crown: scattered leaves + a general warm tint.
	float cover = litter * smoothstep(crown_r * 1.15 + 0.5, crown_r * 0.25, r);
	col = mix(col, col * vec3(1.15, 0.78, 0.50), cover * 0.55);
	for (int layer = 0; layer < 2; layer++) {
		float sc = layer == 0 ? 1.5 : 3.4;
		vec2 sp = p * sc + float(layer) * 17.0;
		vec3 ci = vec3(floor(sp), 3.0 + float(layer));
		vec3 h = hash33(ci);
		vec2 c = ci.xy + 0.25 + 0.5 * h.xy;
		vec2 dv = sp - c;
		float ang = h.z * 6.2831;
		vec2 q = vec2(dv.x * cos(ang) + dv.y * sin(ang), -dv.x * sin(ang) + dv.y * cos(ang));
		float leaf = 1.0 - smoothstep(0.80, 1.0, length(q / vec2(0.36, 0.20)));
		float present = step(1.0 - cover * 0.9, hash13(ci + 5.1));
		vec3 lc = mix(vec3(0.58, 0.38, 0.12), vec3(0.30, 0.17, 0.06), h.x);
		lc = mix(lc, vec3(0.48, 0.20, 0.07), step(0.8, h.y));
		col = mix(col, lc * (0.8 + 0.4 * h.y), leaf * present);
	}
	// Frost: pale, slightly sparkling.
	col = mix(col, vec3(0.86, 0.90, 0.94) * (0.9 + 0.1 * blades), frost * 0.72);
	// Turf edge.
	col *= 1.0 - 0.28 * smoothstep(plug_r - 0.10, plug_r, r);
	ALBEDO = col;
	ROUGHNESS = 1.0;
	SPECULAR = 0.0;
	//ALPHA_LINE
}
"""

# ---------------------------------------------------------------------------------------------


func _ready() -> void:
	_soil_mat = _make_mat("shader_type spatial;\nrender_mode cull_disabled;\n" + NOISE_GLSL + LAYERS_GLSL + SOIL_CUT_GLSL)
	_soil_xray_mat = _make_mat("shader_type spatial;\nrender_mode cull_disabled, unshaded, depth_draw_never;\n" + NOISE_GLSL + LAYERS_GLSL + SOIL_XRAY_GLSL)
	_top_mat = _make_mat("shader_type spatial;\nrender_mode cull_disabled;\n" + TOP_GLSL)
	_top_xray_mat = _make_mat("shader_type spatial;\nrender_mode cull_disabled, depth_draw_never;\n" + TOP_GLSL.replace("//ALPHA_LINE", "ALPHA = xray_alpha;"))
	_top_xray_mat.set_shader_parameter("xray_alpha", 0.25)
	_soil_mi = MeshInstance3D.new()
	_top_mi = MeshInstance3D.new()
	add_child(_soil_mi)
	add_child(_top_mi)
	_build_meshes()
	set_layers(PackedFloat32Array([0.05, 0.35, 1.2, 99.0]), PackedInt32Array([0, 1, 2, 3]),
		PackedFloat32Array([0.4, 0.4, 0.4, 0.4]))
	set_xray(false)
	_layout()


func _make_mat(code: String) -> ShaderMaterial:
	var sh: Shader = Shader.new()
	sh.code = code
	var m: ShaderMaterial = ShaderMaterial.new()
	m.shader = sh
	return m


func reset() -> void:
	radius = MIN_RADIUS
	depth = MIN_DEPTH
	_goal_radius = MIN_RADIUS
	_goal_depth = MIN_DEPTH
	_layout()


## Sets goals from the tree (never lowers them) and eases toward them.
func fit(crown_radius_m: float, root_depth_m: float, delta: float) -> void:
	_goal_radius = maxf(_goal_radius, maxf(RADIUS_PER_CROWN * crown_radius_m, MIN_RADIUS))
	_goal_depth = maxf(_goal_depth, maxf(root_depth_m + 0.5 + 0.12 * root_depth_m, MIN_DEPTH))
	var k: float = 1.0 - exp(-3.0 * delta)
	var r: float = maxf(radius, lerpf(radius, _goal_radius, k))
	var d: float = maxf(depth, lerpf(depth, _goal_depth, k))
	if r - radius > 0.0005 or d - depth > 0.0005:
		radius = r
		depth = d
		_layout()


func snap() -> void:
	radius = _goal_radius
	depth = _goal_depth
	_layout()


func set_xray(on: bool) -> void:
	xray = on
	if _soil_mi == null:
		return
	_soil_mi.material_override = _soil_xray_mat if on else _soil_mat
	_top_mi.material_override = _top_xray_mat if on else _top_mat
	# The translucent volume must not cast a shadow (it would darken the roots inside it).
	_soil_mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF if on else GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	_top_mi.cast_shadow = _soil_mi.cast_shadow


## Wedge centre azimuth (rad) and opening (deg). Rebuilds the mesh.
func set_wedge(dir_rad: float, width_deg: float) -> void:
	wedge_dir = dir_rad
	wedge_width = deg_to_rad(clampf(width_deg, 30.0, 200.0))
	if _soil_mi != null:
		_build_meshes()


## REAL SOIL DATA ENTRY POINT. Arrays have one entry per layer, top to bottom (<= MAX_LAYERS):
##   bottom_m[i]  depth of the layer's bottom below the surface, metres (the last one may be huge)
##   kind[i]      KIND_LITTER / KIND_A / KIND_B / KIND_C (palette + legend)
##   water[i]     volumetric water fraction normalised to 0..1 (0 = dry, 1 = saturated)
func set_layers(bottom_m: PackedFloat32Array, kind: PackedInt32Array, water: PackedFloat32Array) -> void:
	var n: int = mini(mini(bottom_m.size(), kind.size()), mini(water.size(), MAX_LAYERS))
	if n < 1:
		return
	var b: PackedFloat32Array = PackedFloat32Array()
	var kf: PackedFloat32Array = PackedFloat32Array()
	var w: PackedFloat32Array = PackedFloat32Array()
	for i: int in MAX_LAYERS:
		var j: int = mini(i, n - 1)
		b.append(bottom_m[j] if i < n else 99.0)
		kf.append(float(kind[j]))
		w.append(water[j])
	layer_count = n
	layer_bottom = b
	for m: ShaderMaterial in [_soil_mat, _soil_xray_mat]:
		m.set_shader_parameter("n_layers", n)
		m.set_shader_parameter("layer_bottom", b)
		m.set_shader_parameter("layer_kind", kf)
		m.set_shader_parameter("layer_water", w)


## Depth (m) down to which the soil is visibly dry from the top (0 = no drying front).
func set_dry_front(depth_m: float) -> void:
	for m: ShaderMaterial in [_soil_mat, _soil_xray_mat]:
		m.set_shader_parameter("dry_front", depth_m)


## Seasonal look of the top: vigor 0..1 (green vs dormant), fresh 0..1 (spring vs summer green), dryness,
## litter and frost 0..1 each.
func set_ground(vigor: float, fresh: float, dryness: float, litter: float, frost: float, crown_r: float) -> void:
	for m: ShaderMaterial in [_top_mat, _top_xray_mat]:
		m.set_shader_parameter("vigor", vigor)
		m.set_shader_parameter("fresh", fresh)
		m.set_shader_parameter("dryness", dryness)
		m.set_shader_parameter("litter", litter)
		m.set_shader_parameter("frost", frost)
		m.set_shader_parameter("crown_r", crown_r)


## Azimuth of the cut face `which` (0 = right-hand face seen from the wedge axis, 1 = left-hand one).
func face_azimuth(which: int) -> float:
	return wedge_dir + wedge_width * 0.5 if which == 0 else wedge_dir - wedge_width * 0.5


## Horizontal normal of cut face `which` (points into the wedge, toward the viewer).
func face_normal(which: int) -> Vector3:
	var a: float = face_azimuth(which)
	return Vector3(-cos(a), 0.0, sin(a)) if which == 0 else Vector3(cos(a), 0.0, -sin(a))


func _layout() -> void:
	if _soil_mi == null:
		return
	_soil_mi.scale = Vector3(radius, depth, radius)
	_top_mi.scale = Vector3(radius, 1.0, radius)
	for m: ShaderMaterial in [_top_mat, _top_xray_mat]:
		m.set_shader_parameter("plug_r", radius)


## Unit plug (radius 1, depth 1, y in [-1, 0]) with the wedge removed. Scaled by _layout().
func _build_meshes() -> void:
	var a0: float = wedge_dir + wedge_width * 0.5
	var a1: float = wedge_dir - wedge_width * 0.5 + TAU
	var segs: int = maxi(int(ceil((a1 - a0) / TAU * ARC_SEGMENTS)), 4)
	var v: PackedVector3Array = PackedVector3Array()
	var nrm: PackedVector3Array = PackedVector3Array()
	var idx: PackedInt32Array = PackedInt32Array()
	var down: Vector3 = Vector3(0.0, -1.0, 0.0)
	# Outer wall.
	for i: int in segs + 1:
		var a: float = lerpf(a0, a1, float(i) / segs)
		var dir: Vector3 = Vector3(sin(a), 0.0, cos(a))
		v.append(dir)
		v.append(dir + down)
		nrm.append(dir)
		nrm.append(dir)
	for i: int in segs:
		var o: int = i * 2
		idx.append_array(PackedInt32Array([o, o + 2, o + 1, o + 1, o + 2, o + 3]))
	# Two radial cut faces.
	for which: int in 2:
		var a: float = face_azimuth(which)
		var dir: Vector3 = Vector3(sin(a), 0.0, cos(a))
		var nn: Vector3 = face_normal(which)
		var o: int = v.size()
		v.append(Vector3.ZERO)
		v.append(dir)
		v.append(dir + down)
		v.append(down)
		for k: int in 4:
			nrm.append(nn)
		idx.append_array(PackedInt32Array([o, o + 1, o + 2, o, o + 2, o + 3]))
	# Bottom.
	var ob: int = v.size()
	v.append(down)
	nrm.append(Vector3.DOWN)
	for i: int in segs + 1:
		var a: float = lerpf(a0, a1, float(i) / segs)
		v.append(Vector3(sin(a), -1.0, cos(a)))
		nrm.append(Vector3.DOWN)
	for i: int in segs:
		idx.append_array(PackedInt32Array([ob, ob + 1 + i, ob + 2 + i]))
	var arrays: Array = []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = v
	arrays[Mesh.ARRAY_NORMAL] = nrm
	arrays[Mesh.ARRAY_INDEX] = idx
	var mesh: ArrayMesh = ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	_soil_mi.mesh = mesh
	# Top: fan from the axis over the remaining arc.
	var tv: PackedVector3Array = PackedVector3Array([Vector3.ZERO])
	var tn: PackedVector3Array = PackedVector3Array([Vector3.UP])
	var ti: PackedInt32Array = PackedInt32Array()
	for i: int in segs + 1:
		var a: float = lerpf(a0, a1, float(i) / segs)
		tv.append(Vector3(sin(a), 0.0, cos(a)))
		tn.append(Vector3.UP)
	for i: int in segs:
		ti.append_array(PackedInt32Array([0, 2 + i, 1 + i]))
	var tarr: Array = []
	tarr.resize(Mesh.ARRAY_MAX)
	tarr[Mesh.ARRAY_VERTEX] = tv
	tarr[Mesh.ARRAY_NORMAL] = tn
	tarr[Mesh.ARRAY_INDEX] = ti
	var tmesh: ArrayMesh = ArrayMesh.new()
	tmesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, tarr)
	_top_mi.mesh = tmesh
