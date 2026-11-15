class_name Backdrop
extends CanvasLayer
## Quiet "naturalist plate" background: a soft vertical gradient with a faint vignette and paper grain,
## drawn on a canvas layer behind the 3D scene (Environment.BG_CANVAS). Cool and pale in winter, warm in
## summer; also tints the ambient light and the sun.

const COOL_TOP: Color = Color(0.46, 0.55, 0.68)
const COOL_BOTTOM: Color = Color(0.82, 0.80, 0.76)
const WARM_TOP: Color = Color(0.28, 0.50, 0.72)
const WARM_BOTTOM: Color = Color(0.96, 0.80, 0.58)

const SHADER: String = """
shader_type canvas_item;
uniform vec4 top : source_color = vec4(0.28, 0.50, 0.72, 1.0);
uniform vec4 bottom : source_color = vec4(0.96, 0.80, 0.58, 1.0);
uniform float vignette = 0.32;
float hash(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}
void fragment() {
	float t = smoothstep(0.0, 1.0, UV.y);
	vec3 c = mix(top.rgb, bottom.rgb, t);
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
	_mat.set_shader_parameter("bottom", COOL_BOTTOM.lerp(WARM_BOTTOM, w))
