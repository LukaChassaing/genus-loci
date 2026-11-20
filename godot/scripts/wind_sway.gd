class_name WindSway
extends RefCounted
## Sway of the tree in the wind, drawn by the vertex shaders (wood, leaves, flowers, wounds, mechanics overlay): one
## displacement function of the model-space position, shared by every material so that leaves stay on their twigs.
## No mesh is rebuilt per frame; the CPU only sets four global shader uniforms (RenderingServer global parameters).
##
## Physics (dendro_core, TreeSim.get_sway): the first sway mode of the simulated tree (Rayleigh estimate, natural
## frequency f) and its static top displacement under a 10 m/s gust, scaled by (U/10)² (drag ∝ U² above the
## reconfiguration range; an upper bound below it). Display model, stated as such:
## * mean bend = static top displacement at the mean wind U of the day, along the wind, shape (y/H)² (the
##   static shape of a cantilever is close to it);
## * oscillation at the natural frequency f, amplitude = mean bend × 4 I (I ≈ 0.2, turbulence intensity near the
##   ground over open country: the load fluctuates by ±2 I of its mean, peaks ~2 standard deviations);
## * branch flutter: a faster, smaller oscillation (2.7 f, a branch mode above the stem mode: display choice) whose
##   phase follows the position, growing away from the stem axis.

const I_TURB: float = 0.2
const FLUTTER_RATIO: float = 2.7

## GLSL function inserted in every swaying shader's vertex(): `VERTEX = wind_sway(VERTEX);`
const GLSL: String = """
global uniform vec4 wind_a;  // x, y: wind direction (x, z); z: mean top bend (m); w: oscillation amplitude at the top (m)
global uniform vec4 wind_b;  // x: phase 2π f t; y: tree height (m); z: flutter amplitude per m from the axis; w: flutter phase
vec3 wind_sway(vec3 p) {
	float h = max(wind_b.y, 0.5);
	float s = clamp(p.y / h, 0.0, 1.3);
	float shape = s * s;
	vec2 d = wind_a.xy;
	float bend = shape * (wind_a.z + wind_a.w * sin(wind_b.x));
	float r = length(p.xz);
	float fl = wind_b.z * shape * r * sin(wind_b.w + 0.7 * p.x + 0.9 * p.z + 0.4 * p.y);
	vec3 off = vec3(d.x, 0.0, d.y) * (bend + fl) + vec3(0.0, 0.35 * fl, 0.0);
	// A bent axis keeps its length: the top sinks a little.
	off.y -= 0.5 * bend * bend / h * s;
	return p + off;
}
"""

static var _ready: bool = false
## Last values set (the rendering server does not read global parameters back in every mode).
static var last_a: Vector4 = Vector4.ZERO
static var last_b: Vector4 = Vector4.ZERO


## The global uniforms `wind_a`, `wind_b` are declared in project.godot ([shader_globals]); nothing to register at
## run time (the rendering server's global parameter list is an editor-only query).
static func setup() -> void:
	_ready = true


## Sets the uniforms: wind blowing towards `azimuth_deg` (TreeSim convention: from +x towards +z), mean wind `u_ms`
## at 10 m, sway parameters `sway` (TreeSim.get_sway), phases in radians.
static func apply(azimuth_deg: float, u_ms: float, sway: Dictionary, phase: float, flutter_phase: float) -> void:
	var a: float = deg_to_rad(azimuth_deg)
	var h: float = float(sway.get("height_m", 1.0))
	var per10: float = float(sway.get("top_per_10ms", 0.0))
	var bend: float = clampf(per10 * (u_ms / 10.0) * (u_ms / 10.0), 0.0, 0.45 * h)
	var amp: float = minf(bend * 4.0 * I_TURB, 0.25 * h)
	var flutter: float = clampf(0.0015 * u_ms, 0.0, 0.08)
	last_a = Vector4(cos(a), sin(a), bend, amp)
	last_b = Vector4(phase, h, flutter, flutter_phase)
	RenderingServer.global_shader_parameter_set(&"wind_a", last_a)
	RenderingServer.global_shader_parameter_set(&"wind_b", last_b)


## Still air (screens without a tree, tests).
static func calm() -> void:
	RenderingServer.global_shader_parameter_set(&"wind_a", Vector4(1.0, 0.0, 0.0, 0.0))
	RenderingServer.global_shader_parameter_set(&"wind_b", Vector4(0.0, 1.0, 0.0, 0.0))


## Bark shader with the sway: the same look as ProcTextures.make_bark_material (albedo texture × vertex colour ×
## tint, normal map, roughness, specular); its parameters follow a StandardMaterial3D (see `sync_bark`).
static func make_bark_shader() -> ShaderMaterial:
	var sh: Shader = Shader.new()
	sh.code = """
shader_type spatial;
render_mode diffuse_burley, specular_schlick_ggx;
uniform sampler2D albedo_tex : source_color, filter_linear_mipmap_anisotropic;
uniform sampler2D normal_tex : hint_normal, filter_linear_mipmap_anisotropic;
uniform vec4 tint : source_color = vec4(1.0);
uniform float roughness = 0.95;
uniform float specular = 0.15;
uniform float normal_scale = 1.0;
""" + GLSL + """
void vertex() {
	VERTEX = wind_sway(VERTEX);
}
void fragment() {
	ALBEDO = texture(albedo_tex, UV).rgb * COLOR.rgb * tint.rgb;
	ROUGHNESS = roughness;
	SPECULAR = specular;
	NORMAL_MAP = texture(normal_tex, UV).rgb;
	NORMAL_MAP_DEPTH = normal_scale;
}
"""
	var m: ShaderMaterial = ShaderMaterial.new()
	m.shader = sh
	return m


## Copies the look of `src` (the bark StandardMaterial3D the rest of the code tunes) into the bark shader.
static func sync_bark(dst: ShaderMaterial, src: StandardMaterial3D) -> void:
	dst.set_shader_parameter("albedo_tex", src.albedo_texture)
	dst.set_shader_parameter("normal_tex", src.normal_texture)
	dst.set_shader_parameter("tint", src.albedo_color)
	dst.set_shader_parameter("roughness", src.roughness)
	dst.set_shader_parameter("specular", src.metallic_specular)
	dst.set_shader_parameter("normal_scale", src.normal_scale)


## Vertex-coloured material with the sway (mechanics overlay, wounds, falling branches): `srgb` when the colours are
## display colours, `double_sided` for wound faces.
static func make_vertex_color_shader(roughness: float, srgb: bool, double_sided: bool) -> ShaderMaterial:
	var sh: Shader = Shader.new()
	sh.code = "shader_type spatial;\nrender_mode diffuse_burley%s;\n" % (", cull_disabled" if double_sided else "") + GLSL + """
uniform float roughness = %s;
void vertex() {
	VERTEX = wind_sway(VERTEX);
}
void fragment() {
	ALBEDO = %s;
	ROUGHNESS = roughness;
}
""" % [str(roughness), "pow(COLOR.rgb, vec3(2.2))" if srgb else "COLOR.rgb"]
	var m: ShaderMaterial = ShaderMaterial.new()
	m.shader = sh
	return m
