class_name SoilBlock
extends Node3D
## Soil cross-section block: occupies z < 0, the camera on the +z side sees the cut face (z = 0).
## Half-width and depth follow the root system (smoothly, never shrinking until reset()).
## The far ground is a large plane fading into fog so no underside/edge shows against the sky.

const MIN_HALF_WIDTH: float = 3.0
const MIN_DEPTH: float = 3.0

var half_width: float = MIN_HALF_WIDTH
var depth: float = MIN_DEPTH
var _goal_half_width: float = MIN_HALF_WIDTH
var _goal_depth: float = MIN_DEPTH

var _body: MeshInstance3D
var _face: MeshInstance3D
var _top: MeshInstance3D
var _far: MeshInstance3D
var _floor: MeshInstance3D
var _face_mat: ShaderMaterial

const FACE_SHADER: String = """
shader_type spatial;
render_mode cull_back;
uniform float depth = 3.0;
uniform vec4 c_litter : source_color = vec4(0.17, 0.11, 0.07, 1.0);
uniform vec4 c_top : source_color = vec4(0.27, 0.18, 0.11, 1.0);
uniform vec4 c_sub : source_color = vec4(0.45, 0.32, 0.19, 1.0);
uniform vec4 c_parent : source_color = vec4(0.62, 0.55, 0.42, 1.0);
varying vec3 wpos;

float hash(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}
float vnoise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1, 0)), f.x), mix(hash(i + vec2(0, 1)), hash(i + vec2(1, 1)), f.x), f.y);
}
void vertex() {
	wpos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}
void fragment() {
	float d = -wpos.y;
	// Horizon boundaries scale with the block depth (m below the surface).
	float b1 = clamp(0.03 * depth, 0.10, 0.30);
	float b2 = 0.14 * depth + 0.15;
	float b3 = 0.50 * depth;
	// Wavy boundaries.
	float w = (vnoise(vec2(wpos.x * 0.9, 1.0)) - 0.5) * 0.12 * (0.4 + 0.1 * depth);
	float dd = d + w;
	float s = 0.04;
	vec3 col = c_litter.rgb;
	col = mix(col, c_top.rgb, smoothstep(b1 - s, b1 + s, dd));
	col = mix(col, c_sub.rgb, smoothstep(b2 - s * 2.0, b2 + s * 2.0, dd));
	col = mix(col, c_parent.rgb, smoothstep(b3 - s * 3.0, b3 + s * 3.0, dd));
	// Speckle / stones / grain.
	float n = vnoise(wpos.xy * 14.0) * 0.6 + vnoise(wpos.xy * 41.0) * 0.4;
	col *= 0.82 + 0.36 * n;
	float stone = step(0.93, hash(floor(wpos.xy * 22.0))) * step(b2, dd);
	col = mix(col, col * 1.35, stone * 0.6);
	// Depth cue: faint horizontal tick line every metre (stronger every 5 m), label-free.
	float k = abs(fract(d) - 0.5);
	float line = 1.0 - smoothstep(0.0, fwidth(d) * 1.5 + 0.004, 0.5 - k);
	line *= step(0.3, d);
	col = mix(col, col * 0.62, line * 0.45);
	ALBEDO = col;
	ROUGHNESS = 1.0;
	SPECULAR = 0.0;
}
"""


func _ready() -> void:
	var bm: BoxMesh = BoxMesh.new()
	bm.size = Vector3.ONE
	_body = MeshInstance3D.new()
	_body.mesh = bm
	var side: StandardMaterial3D = StandardMaterial3D.new()
	side.albedo_color = Color(0.36, 0.27, 0.18)
	side.roughness = 1.0
	_body.material_override = side
	add_child(_body)

	_face = MeshInstance3D.new()
	_face.mesh = QuadMesh.new()
	var sh: Shader = Shader.new()
	sh.code = FACE_SHADER
	_face_mat = ShaderMaterial.new()
	_face_mat.shader = sh
	_face.material_override = _face_mat
	add_child(_face)

	# Ground: a big plane behind the cut (z < 0) reaching the horizon, faded by fog.
	_top = MeshInstance3D.new()
	var pm: PlaneMesh = PlaneMesh.new()
	pm.size = Vector2(1.0, 1.0)
	_top.mesh = pm
	var grass: StandardMaterial3D = StandardMaterial3D.new()
	grass.albedo_color = Color(0.30, 0.50, 0.20)
	grass.roughness = 1.0
	grass.metallic_specular = 0.0
	_top.material_override = grass
	add_child(_top)
	_far = MeshInstance3D.new()
	_far.mesh = pm
	_far.material_override = grass
	add_child(_far)
	# Floor of the "pit" in front of the cut: a plain at the block's bottom level, fading into the fog.
	_floor = MeshInstance3D.new()
	_floor.mesh = pm
	var floor_mat: StandardMaterial3D = StandardMaterial3D.new()
	floor_mat.albedo_color = Color(0.27, 0.19, 0.12)
	floor_mat.roughness = 1.0
	floor_mat.metallic_specular = 0.0
	_floor.material_override = floor_mat
	_floor.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(_floor)
	_layout()


func reset() -> void:
	half_width = MIN_HALF_WIDTH
	depth = MIN_DEPTH
	_goal_half_width = MIN_HALF_WIDTH
	_goal_depth = MIN_DEPTH
	_layout()


## Sets goals from the root system (never lowers them) and eases toward them.
func fit(spread_m: float, root_depth_m: float, delta: float) -> void:
	_goal_half_width = maxf(_goal_half_width, maxf(1.15 * spread_m + 0.8, MIN_HALF_WIDTH))
	_goal_depth = maxf(_goal_depth, maxf(root_depth_m * 1.2 + 1.0, MIN_DEPTH))
	var k: float = 1.0 - exp(-3.0 * delta)
	var hw: float = lerpf(half_width, _goal_half_width, k)
	var dp: float = lerpf(depth, _goal_depth, k)
	if absf(hw - half_width) > 0.0005 or absf(dp - depth) > 0.0005:
		half_width = maxf(half_width, hw)
		depth = maxf(depth, dp)
		_layout()


func snap() -> void:
	half_width = _goal_half_width
	depth = _goal_depth
	_layout()


func _layout() -> void:
	if _body == null:
		return
	var w: float = half_width * 2.0
	var len: float = maxf(half_width, 4.0)
	_body.scale = Vector3(w, depth, len)
	_body.position = Vector3(0.0, -depth * 0.5, -len * 0.5 - 0.01)
	_face.scale = Vector3(w, depth, 1.0)
	_face.position = Vector3(0.0, -depth * 0.5, 0.0)
	_face_mat.set_shader_parameter("depth", depth)
	# Ground plane: huge, starts at the cut plane and goes back to the horizon.
	_top.scale = Vector3(w, 1.0, len)
	_top.position = Vector3(0.0, 0.002, -len * 0.5 - 0.01)
	_floor.scale = Vector3(1200.0, 1.0, 1200.0)
	_floor.position = Vector3(0.0, -depth - 0.02, 0.0)
	_far.scale = Vector3(1200.0, 1.0, 600.0)
	_far.position = Vector3(0.0, 0.0, -len - 0.01 - 300.0)
