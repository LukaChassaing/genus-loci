class_name Backdrop
extends CanvasLayer
## Background drawn on a canvas layer behind the 3D scene (Environment.BG_CANVAS): a mid-tone sky above
## the world horizon and a dark cutaway ground below it, with a faint vignette and paper grain. Cool in
## winter, warm in summer; also tints the ambient light and the sun.

# Sky: mid-tone, never near-white, so pale things (vapour, O2 dashes, blossom) stay readable against it.
const COOL_TOP: Color = Color(0.30, 0.38, 0.50)
const COOL_HORIZON: Color = Color(0.66, 0.66, 0.66)
const WARM_TOP: Color = Color(0.17, 0.34, 0.55)
const WARM_HORIZON: Color = Color(0.84, 0.69, 0.50)
# Below the horizon: a dark "cutaway" ground, so roots and luminous flux dashes read as light on dark
# when the soil is made transparent.
const COOL_EARTH: Color = Color(0.17, 0.17, 0.19)
const WARM_EARTH: Color = Color(0.23, 0.18, 0.14)
const DEEP: Color = Color(0.085, 0.075, 0.08)

const SHADER: String = """
shader_type canvas_item;
uniform vec4 top : source_color = vec4(0.17, 0.34, 0.55, 1.0);
uniform vec4 horizon_col : source_color = vec4(0.84, 0.69, 0.50, 1.0);
uniform vec4 earth : source_color = vec4(0.17, 0.13, 0.10, 1.0);
uniform vec4 deep : source_color = vec4(0.045, 0.04, 0.05, 1.0);
uniform float horizon = 0.45;  // screen-space v of the world horizon
uniform float vignette = 0.30;
float hash(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}
void fragment() {
	float d = UV.y - horizon;  // > 0 below the horizon
	// Sky: glow concentrated near the horizon, deepening quickly upward.
	float up = clamp(-d / 0.75, 0.0, 1.0);
	vec3 sky = mix(horizon_col.rgb, top.rgb, pow(up, 0.55));
	// Ground: hazy at the horizon line, then dark, then near-black with depth.
	float down = clamp(d / 0.8, 0.0, 1.0);
	vec3 ground = mix(earth.rgb, deep.rgb, pow(down, 0.7));
	ground = mix(mix(horizon_col.rgb, earth.rgb, 0.72), ground, smoothstep(0.0, 0.10, d));
	vec3 c = mix(sky, ground, smoothstep(-0.012, 0.012, d));
	vec2 q = UV - 0.5;
	c *= 1.0 - vignette * smoothstep(0.25, 0.85, length(q * vec2(1.3, 1.0)));
	c += (hash(floor(FRAGCOORD.xy)) - 0.5) * 0.012;  // paper grain
	COLOR = vec4(c, 1.0);
}
"""

var _mat: ShaderMaterial
## 0 = coldest winter, 1 = high summer (eased from the day of year).
var warmth: float = 1.0


func _ready() -> void:
	layer = -10
	var rect: ColorRect = ColorRect.new()
	rect.set_anchors_preset(Control.PRESET_FULL_RECT)
	rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var sh: Shader = Shader.new()
	sh.code = SHADER
	_mat = ShaderMaterial.new()
	_mat.shader = sh
	rect.material = _mat
	add_child(rect)
	set_season(172)


static func warmth_for_day(day_of_year: int) -> float:
	return 0.5 + 0.5 * sin(TAU * (float(day_of_year) - 80.0) / 365.0)


func set_season(day_of_year: int) -> void:
	warmth = warmth_for_day(day_of_year)
	var w: float = smoothstep(0.0, 1.0, warmth)
	_mat.set_shader_parameter("top", COOL_TOP.lerp(WARM_TOP, w))
	_mat.set_shader_parameter("horizon_col", COOL_HORIZON.lerp(WARM_HORIZON, w))
	_mat.set_shader_parameter("earth", COOL_EARTH.lerp(WARM_EARTH, w))
	_mat.set_shader_parameter("deep", DEEP)


func _process(_delta: float) -> void:
	# Follow the camera so the sky/ground split sits on the true world horizon.
	var cam: Camera3D = get_viewport().get_camera_3d()
	if cam == null:
		return
	var fwd: Vector3 = -cam.global_transform.basis.z
	var elev: float = asin(clampf(fwd.y, -1.0, 1.0))
	var half: float = deg_to_rad(cam.fov) * 0.5
	var v: float = 0.5 + 0.5 * tan(clampf(elev, -1.4, 1.4)) / tan(half)
	_mat.set_shader_parameter("horizon", clampf(v, -0.5, 1.5))
