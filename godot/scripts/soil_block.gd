class_name SoilBlock
extends Node3D
## Museum-diorama ground: an "iceberg" of soil floating in space. The grassy top is the visible ground;
## below it an irregular, faceted body of broken earth whose envelope follows the actual root system
## (wide and shallow where the laterals run, a deeper keel where the taproot and sinkers go), with a
## margin. A wedge is cut away: its two radial cut faces meet on the trunk axis and show the soil
## horizons; the roots live in it as real 3-D geometry (see RootMaterial, clipped at radius / depth).
##
## Shape: set_envelope() takes the core's polar root-occupancy grid (TreeSim.get_root_envelope) and turns
## it into a goal profile: rim radius per azimuth sector and the depth of the underside at
## N_KNOTS radial knots (fractions of the rim radius). The shown profile eases toward the goal; the mesh
## is rebuilt only when it moved noticeably (REBUILD_*). On a site with bedrock the body is cut flat at
## a rock slab under the bedrock surface.
##
## Opacity: 1 = opaque earth (opaque material, no discard); below 1 the same shader runs with an ordered
## 4x4 screen-door dither (no blending, no sorting: depth-correct with the roots inside, old-GPU friendly).
## Silhouette, horizon lines and the turf edge keep more coverage so the outline stays readable.
##
## Horizons: described by up to MAX_LAYERS layers (bottom depth in m below the surface, kind, water
## fraction), see set_layers(). Conventions: azimuth a -> direction (sin a, 0, cos a); +z is azimuth 0.
## The core's sectors use atan2(z, x) = PI/2 - a.

const MIN_RADIUS: float = 0.6
const MIN_DEPTH: float = 0.35
const WEDGE_DEFAULT_DEG: float = 110.0
const MAX_LAYERS: int = 8
## Core envelope grid (TreeSim.get_root_envelope).
const ENV_SECTORS: int = 32
const ENV_BINS: int = 20
## Shown profile: knots along the radius (t = r / rim radius, 0..1).
const N_KNOTS: int = 12
const ARC_COLUMNS: int = 84          # columns over the full circle (the wedge removes ~30 %)
const REBUILD_REL: float = 0.025     # rebuild when the profile moved by > 2.5 % ...
const REBUILD_ABS: float = 0.03      # ... and > 3 cm somewhere
const REBUILD_MIN_INTERVAL: float = 0.25
const ROCK_SLAB: float = 0.22        # rock shown under the bedrock surface (m)
const DEPTH_MARGIN: float = 0.42     # earth under the deepest root of a cell (fine-root tufts reach ~0.4 m)
const LOW_OPACITY: float = 0.2       # key X / DENDRO_XRAY

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

## Largest rim radius and deepest point of the shown body (m): camera framing, guides, root clipping.
var radius: float = MIN_RADIUS
var depth: float = MIN_DEPTH
var wedge_dir: float = 0.45                       # azimuth of the wedge centre (faces the default camera)
var wedge_width: float = deg_to_rad(WEDGE_DEFAULT_DEG)
## True when the soil is see-through (opacity < 1): legend, guides on hidden faces.
var xray: bool = false
var opacity: float = 1.0
var layer_count: int = 0
var layer_bottom: PackedFloat32Array = PackedFloat32Array()
var bedrock_m: float = -1.0
var rebuild_count: int = 0
var build_ms: float = 0.0             # time spent in _build_meshes (bench)

# Profiles (per core sector s): rim radius, and depth at knot j (index s * (N_KNOTS + 1) + j).
var _goal_rim: PackedFloat32Array = PackedFloat32Array()
var _goal_keel: PackedFloat32Array = PackedFloat32Array()
var _rim: PackedFloat32Array = PackedFloat32Array()
var _keel: PackedFloat32Array = PackedFloat32Array()
var _built_rim: PackedFloat32Array = PackedFloat32Array()
var _built_keel: PackedFloat32Array = PackedFloat32Array()
var _since_build: float = 10.0
var _easing: bool = false
var _noise: FastNoiseLite
var _soil_mi: MeshInstance3D
var _top_mi: MeshInstance3D
var _soil_mat: ShaderMaterial
var _soil_dither_mat: ShaderMaterial
var _top_mat: ShaderMaterial
var _top_dither_mat: ShaderMaterial

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

## Screen-door transparency shared by the body and the top (same pattern: overlapping faces don't
## compound into a darker moire). `cover` 0..1 = fraction of pixels kept.
const DITHER_GLSL: String = """
uniform float opacity = 1.0;
float bayer4(vec2 p) {
	ivec2 i = ivec2(mod(p, 4.0));
	int k = i.x + i.y * 4;
	float m[16] = float[16](0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
	return (m[k] + 0.5) / 16.0;
}
void screen_door(float cover, vec2 frag) {
#ifdef DITHER
	if (cover < bayer4(frag)) {
		discard;
	}
#endif
}
"""

## Horizon description. Data comes from SoilBlock.set_layers().
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
varying float face_kind;   // 0 = cut face, 1 = broken underside / rim

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
	face_kind = UV.y;
}
"""

const SOIL_BODY_GLSL: String = """
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
	// Faceted broken earth: flat per-triangle normals (no duplicated vertices needed).
	vec3 fn = normalize(cross(dFdy(VERTEX), dFdx(VERTEX)));
	if (dot(fn, VIEW) < 0.0) fn = -fn;
	NORMAL = fn;
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
	// Broken underside: weathered, a little darker and crumbly.
	col *= mix(1.0, 0.80 + 0.25 * vnoise3(wpos * 6.0), face_kind);
	// Metre ticks on the cut faces.
	float k = abs(fract(d) - 0.5);
	float line = (1.0 - smoothstep(0.0, fwidth(d) * 1.5 + 0.003, 0.5 - k)) * step(0.3, d) * (1.0 - face_kind);
	col = mix(col, col * 0.62, line * 0.40);
	// See-through: horizon boundaries and the silhouette keep more coverage (the "iceberg" outline).
	float ring = 0.0;
	for (int i = 0; i < 7; i++) {
		if (i >= n_layers - 1) break;
		ring = max(ring, 1.0 - smoothstep(0.0, 0.012 + fwidth(dd) * 1.6, abs(dd - layer_bottom[i])));
	}
	float fres = pow(1.0 - abs(dot(fn, VIEW)), 3.0);
	float see = 1.0 - opacity;
	col = mix(col, mix(col, vec3(0.70, 0.66, 0.60), 0.35), see);
	screen_door(opacity + see * (0.85 * fres + 0.45 * ring * (1.0 - face_kind)), FRAGCOORD.xy);
	ALBEDO = col;
	ROUGHNESS = 1.0;
	SPECULAR = 0.0;
}
"""

## Top: grass / litter, driven by the season (set_ground()). UV.x = r / rim radius.
const TOP_GLSL: String = """
uniform float vigor = 1.0;      // 0 = dormant dull grass, 1 = fresh green
uniform float fresh = 0.5;      // 0 = deep summer green, 1 = bright spring green
uniform float dryness = 0.0;
uniform float litter = 0.0;
uniform float frost = 0.0;
uniform float crown_r = 3.0;
varying vec3 wpos;
varying float rim_t;

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
	rim_t = UV.x;
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
	// Turf edge (in rim-relative units: the rim is irregular).
	float edge = smoothstep(0.965, 1.0, rim_t);
	col *= 1.0 - 0.28 * edge;
	// See-through: the turf edge keeps a thin readable outline.
	screen_door(opacity + (1.0 - opacity) * 0.9 * smoothstep(0.975, 0.995, rim_t), FRAGCOORD.xy);
	ALBEDO = col;
	ROUGHNESS = 1.0;
	SPECULAR = 0.0;
}
"""

# ---------------------------------------------------------------------------------------------


func _ready() -> void:
	var body: String = NOISE_GLSL + DITHER_GLSL + LAYERS_GLSL + SOIL_BODY_GLSL
	_soil_mat = _make_mat("shader_type spatial;\nrender_mode cull_disabled;\n" + body)
	_soil_dither_mat = _make_mat("shader_type spatial;\nrender_mode cull_disabled;\n#define DITHER\n" + body)
	var top: String = NOISE_GLSL + DITHER_GLSL + TOP_GLSL
	_top_mat = _make_mat("shader_type spatial;\nrender_mode cull_disabled;\n" + top)
	_top_dither_mat = _make_mat("shader_type spatial;\nrender_mode cull_disabled;\n#define DITHER\n" + top)
	_noise = FastNoiseLite.new()
	_noise.noise_type = FastNoiseLite.TYPE_VALUE_CUBIC
	_noise.seed = 1871
	_noise.frequency = 1.0
	_noise.fractal_octaves = 2
	_soil_mi = MeshInstance3D.new()
	_top_mi = MeshInstance3D.new()
	add_child(_soil_mi)
	add_child(_top_mi)
	reset()
	set_layers(PackedFloat32Array([0.05, 0.35, 1.2, 99.0]), PackedInt32Array([0, 1, 2, 3]),
		PackedFloat32Array([0.4, 0.4, 0.4, 0.4]))
	set_opacity(1.0)


func _make_mat(code: String) -> ShaderMaterial:
	var sh: Shader = Shader.new()
	sh.code = code
	var m: ShaderMaterial = ShaderMaterial.new()
	m.shader = sh
	return m


## Back to a seedling clod (new seed / new site).
func reset() -> void:
	var n: int = ENV_SECTORS
	_goal_rim = PackedFloat32Array()
	_goal_rim.resize(n)
	_goal_rim.fill(MIN_RADIUS)
	_goal_keel = PackedFloat32Array()
	_goal_keel.resize(n * (N_KNOTS + 1))
	for s: int in n:
		for j: int in N_KNOTS + 1:
			_goal_keel[s * (N_KNOTS + 1) + j] = lerpf(MIN_DEPTH, _lip(MIN_RADIUS), float(j) / N_KNOTS)
	snap()


## REAL DATA ENTRY POINT for the shape: `env` = TreeSim.get_root_envelope(ENV_SECTORS, ENV_BINS)
## ([r_max, then sector-major max root depths]); `bedrock` = depth of rock (m) or < 0 for none.
## Sets the goal profile; the shown body eases toward it in fit().
func set_envelope(env: PackedFloat32Array, bedrock: float) -> void:
	bedrock_m = bedrock
	var ns: int = ENV_SECTORS
	var nb: int = ENV_BINS
	if env.size() != 1 + ns * nb:
		return
	var r_max: float = maxf(env[0], 0.05)
	var bw: float = r_max / nb
	# 1. Raw spread and tapered depth per sector (depth non-increasing outward: a keel, not a comb).
	var spread: PackedFloat32Array = PackedFloat32Array()
	spread.resize(ns)
	var dep: PackedFloat32Array = PackedFloat32Array()
	dep.resize(ns * nb)
	for s: int in ns:
		var last: int = -1
		var run: float = 0.0
		for b: int in range(nb - 1, -1, -1):
			var v: float = env[1 + s * nb + b]
			if v > 0.0 and last < 0:
				last = b
			run = maxf(run, v)
			dep[s * nb + b] = run
		spread[s] = (float(last) + 0.8) * bw if last >= 0 else 0.0
	# Under the stem the taproot and sinkers sit in one sector or another: give the keel a round bottom
	# there (the deepest root near the axis, easing out over ~1-2 m).
	var core_r: float = maxf(1.0, 0.12 * r_max)
	var axis_d: float = 0.0
	for s: int in ns:
		for b: int in nb:
			if (float(b) + 0.5) * bw < 0.5:
				axis_d = maxf(axis_d, dep[s * nb + b])
	for b: int in nb:
		var x: float = (float(b) + 0.5) * bw / core_r
		if x >= 1.0:
			break
		var shape: float = cos(x * PI * 0.5)
		for s: int in ns:
			dep[s * nb + b] = maxf(dep[s * nb + b], axis_d * shape)
	# 2. Across azimuth: a soft max-filter (no needle for a lone pioneer lateral, no gaps between
	#    laterals), then a blur.
	spread = _circ_blur(_circ_soft_max(spread, 3, [0.9, 0.78, 0.62]), 2)
	var dep2: PackedFloat32Array = PackedFloat32Array()
	dep2.resize(ns * nb)
	for b: int in nb:
		var col: PackedFloat32Array = PackedFloat32Array()
		col.resize(ns)
		for s: int in ns:
			col[s] = dep[s * nb + b]
		col = _circ_blur(_circ_soft_max(col, 2, [0.85, 0.65]), 1)
		for s: int in ns:
			dep2[s * nb + b] = col[s]
	# 3. Goal profile on rim-relative knots, with a margin.
	var deepest: float = 0.0
	for s: int in ns:
		var rim: float = maxf(spread[s] * 1.06 + 0.35, MIN_RADIUS)
		_goal_rim[s] = rim
		var lip: float = _lip(rim)
		for j: int in N_KNOTS + 1:
			var t: float = float(j) / N_KNOTS
			var x: float = clampf(t * rim / bw - 0.5, 0.0, nb - 1.0)
			var b0: int = int(x)
			var b1: int = mini(b0 + 1, nb - 1)
			var dr: float = lerpf(dep2[s * nb + b0], dep2[s * nb + b1], x - b0)
			# Knots are coarser than the bins: take the deepest bin within one knot step either side, so the
			# underside interpolated between knots never cuts above a root.
			var half: float = rim / N_KNOTS / bw
			for b: int in range(maxi(int(ceil(x - half)), 0), mini(int(floor(x + half)), nb - 1) + 1):
				dr = maxf(dr, dep2[s * nb + b])
			var dv: float = maxf(dr + DEPTH_MARGIN + 0.10 * dr, lerpf(MIN_DEPTH, lip, t)) if dr > 0.0 else lerpf(MIN_DEPTH, lip, t)
			# Taper into the rim lip over the outer quarter.
			dv = lerpf(dv, lip, smoothstep(0.8, 1.0, t) * 0.85)
			dv = maxf(dv, maxf(lip, dr + 0.2 if dr > 0.0 else 0.0))
			if bedrock > 0.0:
				# Shallow soil on rock: flat rock bottom, rounded only at the rim.
				dv = lerpf(bedrock + ROCK_SLAB, minf(lip, bedrock + ROCK_SLAB), smoothstep(0.86, 1.0, t))
			_goal_keel[s * (N_KNOTS + 1) + j] = dv
			deepest = maxf(deepest, dv)
	# The axis point is shared by every sector: use the deepest.
	var d0: float = 0.0
	for s: int in ns:
		d0 = maxf(d0, _goal_keel[s * (N_KNOTS + 1)])
	for s: int in ns:
		_goal_keel[s * (N_KNOTS + 1)] = d0
	_easing = true


## Height (m) of the broken outer wall of the slab at the rim.
static func _lip(rim: float) -> float:
	return clampf(0.10 + 0.025 * rim, 0.12, 0.55)


static func _circ_soft_max(v: PackedFloat32Array, reach: int, w: Array) -> PackedFloat32Array:
	var n: int = v.size()
	var out: PackedFloat32Array = v.duplicate()
	for i: int in n:
		var m: float = v[i]
		for k: int in reach:
			var f: float = w[mini(k, w.size() - 1)]
			m = maxf(m, f * maxf(v[(i + k + 1) % n], v[(i - k - 1 + n * 4) % n]))
		out[i] = m
	return out


static func _circ_blur(v: PackedFloat32Array, passes: int) -> PackedFloat32Array:
	var n: int = v.size()
	var cur: PackedFloat32Array = v
	for p: int in passes:
		var out: PackedFloat32Array = PackedFloat32Array()
		out.resize(n)
		for i: int in n:
			out[i] = 0.25 * cur[(i - 1 + n) % n] + 0.5 * cur[i] + 0.25 * cur[(i + 1) % n]
		cur = out
	return cur


## Eases the shown body toward the goal; rebuilds the mesh when it moved noticeably (throttled).
func fit(delta: float) -> void:
	_since_build += delta
	if not _easing:
		# The last easing step may have landed inside the rebuild throttle: build the settled shape.
		if _since_build >= REBUILD_MIN_INTERVAL and (_built_rim != _rim or _built_keel != _keel):
			_build_meshes()
		return
	var k: float = 1.0 - exp(-3.0 * delta)
	var left: float = 0.0
	for i: int in _rim.size():
		_rim[i] = lerpf(_rim[i], _goal_rim[i], k)
		left = maxf(left, absf(_goal_rim[i] - _rim[i]))
	for i: int in _keel.size():
		_keel[i] = lerpf(_keel[i], _goal_keel[i], k)
		left = maxf(left, absf(_goal_keel[i] - _keel[i]))
	if left < 0.002:
		_rim = _goal_rim.duplicate()
		_keel = _goal_keel.duplicate()
		_easing = false
	if _since_build >= REBUILD_MIN_INTERVAL and _moved():
		_build_meshes()


func _moved() -> bool:
	if _built_rim.size() != _rim.size() or _built_keel.size() != _keel.size():
		return true
	for i: int in _rim.size():
		var d: float = absf(_rim[i] - _built_rim[i])
		if d > REBUILD_ABS and d > REBUILD_REL * _built_rim[i]:
			return true
	for i: int in _keel.size():
		var d: float = absf(_keel[i] - _built_keel[i])
		if d > REBUILD_ABS and d > REBUILD_REL * _built_keel[i]:
			return true
	return false


## Jumps to the goal profile (start-up, screenshots, tests).
func snap() -> void:
	_rim = _goal_rim.duplicate()
	_keel = _goal_keel.duplicate()
	_easing = false
	if _soil_mi != null:
		_build_meshes()


## Goal profile reached and built.
func settled() -> bool:
	return not _easing and _built_rim == _rim and _built_keel == _keel


## 1 = opaque earth, down to ~0.03 (only the outline and horizon lines remain).
func set_opacity(v: float) -> void:
	opacity = clampf(v, 0.0, 1.0)
	xray = opacity < 0.995
	if _soil_mi == null:
		return
	_soil_mi.material_override = _soil_dither_mat if xray else _soil_mat
	_top_mi.material_override = _top_dither_mat if xray else _top_mat
	for m: ShaderMaterial in [_soil_dither_mat, _top_dither_mat]:
		m.set_shader_parameter("opacity", maxf(opacity, 0.03))
	# A see-through body must not cast a shadow (it would darken the roots inside it).
	var cast: int = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if opacity >= 0.95 else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_soil_mi.cast_shadow = cast as GeometryInstance3D.ShadowCastingSetting
	_top_mi.cast_shadow = _soil_mi.cast_shadow


## Kept for callers of the old binary X-ray toggle.
func set_xray(on: bool) -> void:
	set_opacity(LOW_OPACITY if on else 1.0)


## Wedge centre azimuth (rad) and opening (deg). Rebuilds the mesh.
func set_wedge(dir_rad: float, width_deg: float) -> void:
	wedge_dir = dir_rad
	wedge_width = deg_to_rad(clampf(width_deg, 30.0, 200.0))
	if _soil_mi != null:
		_build_meshes()


## REAL SOIL DATA ENTRY POINT. Arrays have one entry per layer, top to bottom (<= MAX_LAYERS):
##   bottom_m[i]  depth of the layer's bottom below the surface, metres (the last one may be huge)
##   kind[i]      KIND_LITTER / KIND_A / KIND_B / KIND_C / KIND_ROCK (palette + legend)
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
	for m: ShaderMaterial in [_soil_mat, _soil_dither_mat]:
		m.set_shader_parameter("n_layers", n)
		m.set_shader_parameter("layer_bottom", b)
		m.set_shader_parameter("layer_kind", kf)
		m.set_shader_parameter("layer_water", w)


## Depth (m) down to which the soil is visibly dry from the top (0 = no drying front).
func set_dry_front(depth_m: float) -> void:
	for m: ShaderMaterial in [_soil_mat, _soil_dither_mat]:
		m.set_shader_parameter("dry_front", depth_m)


## Seasonal look of the top: vigor 0..1 (green vs dormant), fresh 0..1 (spring vs summer green), dryness,
## litter and frost 0..1 each.
func set_ground(vigor: float, fresh: float, dryness: float, litter: float, frost: float, crown_r: float) -> void:
	for m: ShaderMaterial in [_top_mat, _top_dither_mat]:
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


## Shown rim radius (m) at azimuth `a` (no rim noise).
func rim_radius(a: float) -> float:
	return _sample(_rim, a, -1)


## Shown depth (m) of the underside at distance `r` from the axis, azimuth `a` (no noise).
func depth_at(r: float, a: float) -> float:
	var rim: float = rim_radius(a)
	var t: float = clampf(r / maxf(rim, 0.01), 0.0, 1.0) * N_KNOTS
	var j0: int = mini(int(t), N_KNOTS - 1)
	return lerpf(_sample(_keel, a, j0), _sample(_keel, a, j0 + 1), t - j0)


## How far (m) a point lies outside the shown body (0 = inside): below the underside or beyond the rim.
func outside(p: Vector3) -> float:
	var a: float = atan2(p.x, p.z)
	var r: float = Vector2(p.x, p.z).length()
	return maxf(r - rim_radius(a), -p.y - depth_at(r, a))


## Where the depth scale goes on cut face `which`: the point of the top edge (y = 0) above the deepest
## part of the face (away from the axis so the trunk doesn't hide it); and that depth.
func face_ruler(which: int) -> Vector3:
	var a: float = face_azimuth(which)
	var r: float = rim_radius(a) * 0.35
	return Vector3(sin(a), 0.0, cos(a)) * r


func face_ruler_depth(which: int) -> float:
	var a: float = face_azimuth(which)
	return depth_at(rim_radius(a) * 0.35, a)


## Linear interpolation of a per-sector profile at soil azimuth `a`; `j` < 0 = the rim array.
func _sample(arr: PackedFloat32Array, a: float, j: int) -> float:
	var n: int = ENV_SECTORS
	if arr.is_empty():
		return MIN_RADIUS
	var x: float = fposmod(PI * 0.5 - a, TAU) / TAU * n - 0.5
	var i0: int = int(floor(x))
	var f: float = x - i0
	i0 = posmod(i0, n)
	var i1: int = (i0 + 1) % n
	if j < 0:
		return lerpf(arr[i0], arr[i1], f)
	var st: int = N_KNOTS + 1
	return lerpf(arr[i0 * st + j], arr[i1 * st + j], f)


## Raw noise of the broken underside per (column, knot), a function of (azimuth, knot) only, so the cut
## faces (which reuse the edge columns) stay sealed. Cached: it only changes with the wedge.
var _noise_key: Vector3 = Vector3(-1.0, -1.0, -1.0)
var _noise_r: PackedFloat32Array = PackedFloat32Array()
var _noise_y: PackedFloat32Array = PackedFloat32Array()


func _noise_table(a0: float, a1: float, cols: int) -> void:
	var key: Vector3 = Vector3(a0, a1, cols)
	if key == _noise_key:
		return
	_noise_key = key
	var st: int = N_KNOTS + 1
	_noise_r.resize((cols + 1) * st)
	_noise_y.resize((cols + 1) * st)
	for c: int in cols + 1:
		var a: float = lerpf(a0, a1, float(c) / cols)
		for j: int in st:
			var p: Vector3 = Vector3(sin(a) * 2.2, float(j) / N_KNOTS * 3.1, cos(a) * 2.2)
			_noise_r[c * st + j] = _noise.get_noise_3dv(p * 1.7)
			_noise_y[c * st + j] = _noise.get_noise_3dv(p * 2.3 + Vector3(11.0, 5.0, 3.0))


## Displacement (dr, dy) in metres of knot j of column c (rim radius `rim`, depth `dv` there).
func _jag(c: int, j: int, rim: float, dv: float) -> Vector2:
	if j == 0:
		return Vector2.ZERO
	var t: float = float(j) / N_KNOTS
	var k: int = c * (N_KNOTS + 1) + j
	var dr: float = _noise_r[k] * (0.035 * rim + 0.04) * t
	var dy: float = _noise_y[k] * (0.06 + 0.13 * dv) * smoothstep(0.0, 0.25, t)
	if bedrock_m > 0.0 and t < 0.86:
		dy = 0.0  # the rock bottom is flat
	return Vector2(dr, dy)


## Builds the body (underside, rim wall, cut faces) and the top over the remaining arc.
func _build_meshes() -> void:
	var t0: int = Time.get_ticks_usec()
	_build_meshes_now()
	build_ms += float(Time.get_ticks_usec() - t0) / 1000.0


func _build_meshes_now() -> void:
	_built_rim = _rim.duplicate()
	_built_keel = _keel.duplicate()
	_since_build = 0.0
	rebuild_count += 1
	var a0: float = wedge_dir + wedge_width * 0.5
	var a1: float = wedge_dir - wedge_width * 0.5 + TAU
	var cols: int = maxi(int(ceil((a1 - a0) / TAU * ARC_COLUMNS)), 6)
	var st: int = N_KNOTS + 1
	_noise_table(a0, a1, cols)
	# Per column: rim point (top) and underside points (knot j = 0 axis .. N_KNOTS rim lip).
	var top_r: PackedFloat32Array = PackedFloat32Array()
	var under: PackedVector2Array = PackedVector2Array()  # (r, y) per column x knot
	var max_r: float = 0.0
	var max_d: float = 0.0
	# The cut faces bound the roots drawn in the open wedge: near the faces the profile takes the
	# largest rim / depth found across the wedge, so no root hangs below the visible section.
	# Wedge profile: deepest underside across the wedge at WEDGE_R absolute radii out to its widest rim.
	var w_rim: float = 0.0
	var w_az: Array[float] = []
	for k: int in 9:
		var aw: float = lerpf(a1 - TAU, a0, float(k) / 8.0)
		w_az.append(aw)
		w_rim = maxf(w_rim, rim_radius(aw))
	var w_dep: PackedFloat32Array = PackedFloat32Array()
	w_dep.resize(st)
	for j: int in st:
		var r: float = float(j) / N_KNOTS * w_rim
		var dw: float = 0.0
		for aw: float in w_az:
			if r <= rim_radius(aw):
				dw = maxf(dw, depth_at(r, aw))
		w_dep[j] = dw
	var ns: int = ENV_SECTORS
	for c: int in cols + 1:
		var a: float = lerpf(a0, a1, float(c) / cols)
		var wk: float = exp(-float(mini(c, cols - c)) / 2.0)
		# Sector interpolation for this column, once.
		var x: float = fposmod(PI * 0.5 - a, TAU) / TAU * ns - 0.5
		var i0: int = int(floor(x))
		var f: float = x - i0
		i0 = posmod(i0, ns)
		var i1: int = (i0 + 1) % ns
		var rim0: float = lerpf(_rim[i0], _rim[i1], f)
		var rim: float = lerpf(rim0, maxf(w_rim, rim0), wk) if wk > 0.01 else rim0
		var lip_dv: float = lerpf(_keel[i0 * st + N_KNOTS], _keel[i1 * st + N_KNOTS], f)
		var rim_j: Vector2 = _jag(c, N_KNOTS, rim, lip_dv)
		var r_edge: float = rim + rim_j.x
		top_r.append(r_edge)
		max_r = maxf(max_r, r_edge)
		for j: int in st:
			var t: float = float(j) / N_KNOTS
			var dv: float
			if wk > 0.01:
				dv = depth_at(t * rim, a)
				var wr: float = clampf(t * rim / maxf(w_rim, 0.01), 0.0, 1.0) * N_KNOTS
				var w0: int = mini(int(wr), N_KNOTS - 1)
				var dw: float = lerpf(w_dep[w0], w_dep[w0 + 1], wr - w0) if t * rim <= w_rim else 0.0
				dv = maxf(dv, dw * wk + dv * (1.0 - wk))
			else:
				dv = lerpf(_keel[i0 * st + j], _keel[i1 * st + j], f)
			var jg: Vector2 = _jag(c, j, rim, dv)
			var r: float = r_edge if j == N_KNOTS else t * rim + jg.x
			var y: float = -maxf(dv + jg.y, 0.06)
			under.append(Vector2(r, y))
			max_d = maxf(max_d, -y)
	radius = max_r
	depth = max_d
	var v: PackedVector3Array = PackedVector3Array()
	var uv: PackedVector2Array = PackedVector2Array()
	var nrm: PackedVector3Array = PackedVector3Array()  # approximate (the shader shades flat facets)
	var idx: PackedInt32Array = PackedInt32Array()
	# Underside + rim wall: per column, rows = [top rim, lip, knots N_KNOTS-1 .. 0]. UV.y = 1 (broken).
	var rows: int = st + 1
	for c: int in cols + 1:
		var a: float = lerpf(a0, a1, float(c) / cols)
		var dir: Vector3 = Vector3(sin(a), 0.0, cos(a))
		v.append(dir * top_r[c])
		uv.append(Vector2(1.0, 1.0))
		nrm.append(dir)
		for j: int in range(N_KNOTS, -1, -1):
			var q: Vector2 = under[c * st + j]
			v.append(dir * q.x + Vector3(0.0, q.y, 0.0))
			uv.append(Vector2(float(j) / N_KNOTS, 1.0))
			nrm.append((dir * (float(j) / N_KNOTS) + Vector3.DOWN).normalized())
	for c: int in cols:
		for k: int in rows - 1:
			var p: int = c * rows + k
			var n: int = p + rows
			idx.append_array(PackedInt32Array([p, n, p + 1, p + 1, n, n + 1]))
	# Two cut faces: vertical strips from the top edge (y = 0) down to the underside, reusing the edge
	# columns' (r, y). UV.y = 0 (cut).
	for which: int in 2:
		var c: int = 0 if which == 0 else cols
		var a: float = lerpf(a0, a1, float(c) / cols)
		var dir: Vector3 = Vector3(sin(a), 0.0, cos(a))
		var fnrm: Vector3 = face_normal(which)
		var o: int = v.size()
		for j: int in st:
			var q: Vector2 = under[c * st + j]
			v.append(dir * q.x)
			v.append(dir * q.x + Vector3(0.0, q.y, 0.0))
			uv.append(Vector2(0.0, 0.0))
			uv.append(Vector2(0.0, 0.0))
			nrm.append(fnrm)
			nrm.append(fnrm)
		for j: int in N_KNOTS:
			var p: int = o + j * 2
			idx.append_array(PackedInt32Array([p, p + 2, p + 1, p + 1, p + 2, p + 3]))
	var arrays: Array = []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = v
	arrays[Mesh.ARRAY_NORMAL] = nrm
	arrays[Mesh.ARRAY_TEX_UV] = uv
	arrays[Mesh.ARRAY_INDEX] = idx
	var mesh: ArrayMesh = ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	_soil_mi.mesh = mesh
	# Top: fan from the axis over the remaining arc, out to the jagged rim. UV.x = r / rim.
	var tv: PackedVector3Array = PackedVector3Array([Vector3.ZERO])
	var tn: PackedVector3Array = PackedVector3Array([Vector3.UP])
	var tuv: PackedVector2Array = PackedVector2Array([Vector2.ZERO])
	var ti: PackedInt32Array = PackedInt32Array()
	for c: int in cols + 1:
		var a: float = lerpf(a0, a1, float(c) / cols)
		var dir: Vector3 = Vector3(sin(a), 0.0, cos(a))
		# Two rings: 92 % and the rim, so the turf-edge band interpolates over a thin strip only.
		tv.append(dir * top_r[c] * 0.92)
		tv.append(dir * top_r[c])
		tn.append(Vector3.UP)
		tn.append(Vector3.UP)
		tuv.append(Vector2(0.92, 0.0))
		tuv.append(Vector2(1.0, 0.0))
	for c: int in cols:
		var p: int = 1 + c * 2
		ti.append_array(PackedInt32Array([0, p + 2, p]))
		ti.append_array(PackedInt32Array([p, p + 2, p + 1, p + 1, p + 2, p + 3]))
	var tarr: Array = []
	tarr.resize(Mesh.ARRAY_MAX)
	tarr[Mesh.ARRAY_VERTEX] = tv
	tarr[Mesh.ARRAY_NORMAL] = tn
	tarr[Mesh.ARRAY_TEX_UV] = tuv
	tarr[Mesh.ARRAY_INDEX] = ti
	var tmesh: ArrayMesh = ArrayMesh.new()
	tmesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, tarr)
	_top_mi.mesh = tmesh
