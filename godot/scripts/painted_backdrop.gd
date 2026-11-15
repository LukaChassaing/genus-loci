class_name PaintedBackdrop
extends Control
## Full-screen "oil sketch" backdrop. Uses godot/art/<art_name>.(png|jpg|webp) when present (cover-fit),
## otherwise paints a stormy sky procedurally ONCE into a SubViewport texture: anisotropic fbm "brush
## strokes", a limited palette (slate, teal, grey, ochre), a warm break of light near a dark horizon,
## and the silhouette of a real simulated old oak (bare, winter) rendered from the TreeSim wood mesh.
## The simulation of the tree runs in a worker thread (about 2 s) and is cached for the session.

const SHADER: String = """
shader_type canvas_item;
uniform sampler2D tree_tex : filter_linear, repeat_disable;
uniform vec4 tree_rect = vec4(0.07, 0.22, 0.54, 0.60);
uniform float tree_on = 0.0;
uniform vec2 light_pos = vec2(0.10, 0.52);
uniform float aspect = 1.7778;
uniform float seed = 0.0;
uniform float horizon = 0.72;

float hash(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
float vnoise(vec2 p) {
	vec2 i = floor(p), f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1, 0)), f.x), mix(hash(i + vec2(0, 1)), hash(i + vec2(1, 1)), f.x), f.y);
}
float fbm(vec2 p) {
	float a = 0.5, s = 0.0;
	for (int i = 0; i < 5; i++) { s += a * vnoise(p); p = p * 2.03 + vec2(7.3, 1.9); a *= 0.5; }
	return s / 0.96875;
}
vec2 rot(vec2 p, float a) { float c = cos(a), s = sin(a); return vec2(c * p.x - s * p.y, s * p.x + c * p.y); }
float stroke(vec2 p, float ang, float len, float wid) {
	vec2 q = rot(p, ang);
	return fbm(vec2(q.x / len, q.y / wid));
}
void fragment() {
	vec2 uv = UV;
	vec2 p = vec2(uv.x * aspect, uv.y) + seed;
	vec2 w = vec2(fbm(p * 2.2), fbm(p * 2.2 + 9.0)) - 0.5;
	vec2 pw = p + w * 0.06;

	// --- sky: soft blue to warm cream at the horizon, morning sun upper left
	float t = smoothstep(0.0, horizon, uv.y);
	vec3 col = mix(vec3(0.07, 0.21, 0.43), vec3(0.24, 0.47, 0.68), smoothstep(0.0, 0.55, t));
	col = mix(col, vec3(0.97, 0.69, 0.43), smoothstep(0.52, 1.0, t) * 0.9);
	vec2 gd = (uv - light_pos) * vec2(aspect, 1.0);
	float glow = exp(-dot(gd, gd) * 5.0);
	col += vec3(1.0, 0.72, 0.36) * glow * 0.50;
	col += (stroke(pw, -0.03, 1.5, 0.045) - 0.5) * 0.13;      // broad brush streaks in the blue
	col += (stroke(pw + 6.0, -0.35, 0.35, 0.03) - 0.5) * 0.07; // short diagonal dabs
	col = mix(col, floor(col * 11.0 + 0.5 + (stroke(pw + 1.0, -0.2, 0.3, 0.05) - 0.5) * 0.8) / 11.0, 0.35);

	// --- big soft cumulus with flat-ish bases, lit from the sun side
	vec2 cp = vec2(pw.x * 0.80, pw.y * 2.0);
	float band = smoothstep(0.04, 0.26, uv.y) * smoothstep(horizon + 0.02, horizon - 0.30, uv.y);
	float d = fbm(cp + vec2(3.0, 1.0)) + (band - 0.5) * 0.42 - 0.07 * smoothstep(0.0, 0.3, uv.y);
	vec2 ld = normalize((light_pos - uv) * vec2(aspect, 1.0));
	float d2 = fbm(cp + vec2(ld.x, ld.y * 2.0) * 0.11 + vec2(3.0, 1.0)) + (band - 0.5) * 0.42 - 0.07 * smoothstep(0.0, 0.3, uv.y);
	float c = smoothstep(0.560, 0.585, d);
	float lit = clamp((d - d2) * 8.0 + 0.5, 0.0, 1.0);
	float core = clamp((d - 0.57) * 3.0, 0.0, 1.0);
	vec3 shade = vec3(0.47, 0.47, 0.72);
	vec3 bright = vec3(1.0, 0.90, 0.72);
	vec3 ccol = mix(shade, bright, clamp(lit * 0.75 + core * 0.35 + 0.18, 0.0, 1.0));
	float edge = smoothstep(0.560, 0.590, d) * (1.0 - smoothstep(0.590, 0.68, d));
	ccol += vec3(1.0, 0.68, 0.30) * edge * lit * 0.75;
	ccol *= mix(vec3(1.0), vec3(1.0, 0.80, 0.68), smoothstep(0.35, 0.95, t) * 0.7);
	ccol *= 0.78 + 0.22 * smoothstep(0.55, 0.80, d);
	// painterly: a few flat tonal steps with ragged edges
	float rag = stroke(pw + 5.0, -0.05, 0.25, 0.035) - 0.5;
	ccol = mix(ccol, floor(ccol * 7.0 + 0.5 + rag * 0.9) / 7.0, 0.35);
	ccol += (stroke(pw + 2.0, -0.04, 0.5, 0.02) - 0.5) * 0.05;
	col = mix(col, ccol, c);

	// --- distant hills, soft blue-green
	float h1 = horizon - 0.035 + (fbm(vec2(p.x * 0.8, 3.0)) - 0.5) * 0.09;
	vec3 farc = mix(vec3(0.66, 0.63, 0.70), vec3(0.40, 0.56, 0.46), clamp((uv.y - h1) * 9.0, 0.0, 1.0));
	farc += (stroke(p + 4.0, -0.03, 0.6, 0.03) - 0.5) * 0.07;
	col = mix(col, farc, smoothstep(h1 - 0.002, h1 + 0.002, uv.y));

	// --- meadow: fresh greens, sunlit patches, brush strokes, scattered blossoms
	float h2 = horizon + 0.012 + (fbm(vec2(p.x * 0.6 + 5.0, 1.0)) - 0.5) * 0.06;
	float dm = smoothstep(h2, 1.0, uv.y);
	vec3 mead = mix(vec3(0.50, 0.64, 0.20), vec3(0.08, 0.30, 0.10), pow(dm, 0.7));
	mead += (stroke(p + 8.0, 0.0, 0.30, 0.016) - 0.5) * 0.20;
	float mp = stroke(p + 2.0, -0.02, 0.9, 0.09);
	mead = mix(mead, vec3(0.86, 0.72, 0.30), smoothstep(0.55, 0.72, mp) * 0.50);
	mead = mix(mead, vec3(0.05, 0.17, 0.10), smoothstep(0.50, 0.30, mp) * 0.45 * dm);
	// tree shadow, cast away from the sun (to the right)
	float bx = tree_rect.x + tree_rect.z * 0.5 + 0.14;
	float by = tree_rect.y + tree_rect.w * 0.965;
	vec2 sh = vec2((uv.x - bx) * aspect / 0.52, (uv.y - by) / 0.034);
	mead = mix(mead, mead * vec3(0.30, 0.46, 0.42), (1.0 - smoothstep(0.55, 1.0, length(sh))) * tree_on);
	// blossoms
	vec2 g = vec2(uv.x * aspect * 58.0, uv.y * 58.0);
	vec2 id = floor(g);
	vec2 f = fract(g) - 0.5;
	float hh = hash(id);
	vec2 jit = (vec2(hash(id + 1.0), hash(id + 2.0)) - 0.5) * 0.45;
	float fl = step(0.955, hh) * (1.0 - smoothstep(0.09, 0.16, length(f - jit))) * smoothstep(h2 + 0.012, h2 + 0.08, uv.y);
	float hc = hash(id + 5.0);
	vec3 fcol = hc < 0.38 ? vec3(1.0, 0.92, 0.78) : (hc < 0.70 ? vec3(0.97, 0.66, 0.76) : vec3(0.99, 0.86, 0.36));
	mead = mix(mead, fcol, fl);
	mead = mix(mead, vec3(0.95, 0.66, 0.38), (1.0 - smoothstep(h2, h2 + 0.14, uv.y)) * 0.28);
	col = mix(col, mead, smoothstep(h2 - 0.003, h2 + 0.003, uv.y));

	// --- the tree (rendered once from the real simulation)
	vec2 tuv = (uv - tree_rect.xy) / tree_rect.zw;
	if (tree_on > 0.0 && tuv.x > 0.0 && tuv.x < 1.0 && tuv.y > 0.0 && tuv.y < 1.0) {
		vec4 tc = texture(tree_tex, tuv);
		col = mix(col, tc.rgb, tc.a * tree_on);
	}

	// gentle vignette, canvas grain
	col *= 1.0 - 0.34 * smoothstep(0.45, 1.1, length((uv - 0.5) * vec2(1.1, 1.0)));
	vec2 cuv = FRAGCOORD.xy;
	float weave = (sin(cuv.x * 1.7) * sin(cuv.y * 1.7)) * 0.5 + 0.5;
	col += (hash(floor(cuv * 0.5)) - 0.5) * 0.018 + (weave - 0.5) * 0.008;
	COLOR = vec4(col, 1.0);
}
"""

const VARIANT1_RECT: Vector4 = Vector4(0.28, 0.30, 0.42, 0.50)

static var tree_mesh: ArrayMesh = null
static var leaf_mesh: ArrayMesh = null
static var _sim: TreeSim = null
static var _days_left: float = 0.0

## Which composition: 0 = title (tree on the left, light behind it), 1 = setup (lower, wider light).
var variant: int = 0
var art_name: String = "title_backdrop"
var with_tree: bool = true
## True once the picture will not change any more (image loaded, or painting + tree finished).
var settled: bool = false

var _painted: SubViewport
var _paint_mat: ShaderMaterial
var _tree_vp: SubViewport
var _tex_rect: TextureRect
var _wait: int = 0
var _state: int = 0   # 0 painting without tree, 1 waiting for the tree, 2 tree viewport rendering, 3 repaint, 4 done


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_tex_rect = TextureRect.new()
	_tex_rect.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_tex_rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	_tex_rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED  # crop, never stretch
	_tex_rect.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS
	_tex_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_tex_rect)
	var img: Texture2D = Game.load_art(art_name)
	if img != null:
		_tex_rect.texture = img
		settled = true
		_state = 4
		set_process(false)
		return
	_build_painting()
	if with_tree and DisplayServer.get_name() != "headless":
		_start_tree()
	else:
		settled = true
		_state = 4
		set_process(false)


func _build_painting() -> void:
	_painted = SubViewport.new()
	_painted.size = Vector2i(1280, 720)
	_painted.render_target_update_mode = SubViewport.UPDATE_ONCE
	_painted.disable_3d = true
	_painted.transparent_bg = false
	add_child(_painted)
	var rect: ColorRect = ColorRect.new()
	rect.size = Vector2(1280, 720)
	var sh: Shader = Shader.new()
	sh.code = SHADER
	_paint_mat = ShaderMaterial.new()
	_paint_mat.shader = sh
	if variant == 1:
		_paint_mat.set_shader_parameter("light_pos", Vector2(0.12, 0.50))
		_paint_mat.set_shader_parameter("horizon", 0.74)
		_paint_mat.set_shader_parameter("seed", 3.1)
		_paint_mat.set_shader_parameter("tree_rect", VARIANT1_RECT)
	else:
		_paint_mat.set_shader_parameter("seed", 1.3)
	rect.material = _paint_mat
	_painted.add_child(rect)
	_tex_rect.texture = _painted.get_texture()


func _start_tree() -> void:
	if tree_mesh == null and _sim == null:
		_sim = TreeSim.new()
		_sim.set_species(0)
		_sim.reset(7)
		_days_left = 150.0 * 365.0 + 145.0  # late May: the crown in fresh leaf
	_state = 1


## Grows a 150-year-old oak in small slices (about 2 s of work spread over the first frames; GDExtension
## objects cannot leave the main thread) and keeps its bare wood mesh for the session.
static func _grow_slice() -> void:
	var t0: int = Time.get_ticks_usec()
	while _days_left > 0.0 and float(Time.get_ticks_usec() - t0) < 9000.0:
		var c: float = minf(_days_left, 30.0)
		_sim.advance_days(c)
		_days_left -= c
	if _days_left <= 0.0:
		tree_mesh = _sim.build_wood_mesh()
		leaf_mesh = _sim.build_leaf_mesh()
		_sim = null


func _process(_delta: float) -> void:
	match _state:
		1:
			if tree_mesh == null and _sim != null:
				_grow_slice()
			if tree_mesh != null:
				_make_tree_viewport()
				_state = 2
				_wait = 3
		2:
			_wait -= 1
			if _wait <= 0:
				_paint_mat.set_shader_parameter("tree_tex", _tree_vp.get_texture())
				_paint_mat.set_shader_parameter("tree_on", 1.0)
				_painted.render_target_update_mode = SubViewport.UPDATE_ONCE
				_state = 3
				_wait = 3
		3:
			_wait -= 1
			if _wait <= 0:
				_state = 4
				settled = true
				set_process(false)


func _make_tree_viewport() -> void:
	_tree_vp = SubViewport.new()
	_tree_vp.size = Vector2i(1024, 640)
	_tree_vp.transparent_bg = true
	_tree_vp.own_world_3d = true
	_tree_vp.msaa_3d = Viewport.MSAA_4X
	_tree_vp.render_target_update_mode = SubViewport.UPDATE_ONCE
	add_child(_tree_vp)
	var bb: AABB = tree_mesh.get_aabb()
	if leaf_mesh != null:
		bb = bb.merge(leaf_mesh.get_aabb())
	var wood: MeshInstance3D = MeshInstance3D.new()
	wood.mesh = tree_mesh
	var bm: StandardMaterial3D = StandardMaterial3D.new()
	bm.albedo_color = Color(0.36, 0.27, 0.19)
	bm.roughness = 1.0
	wood.material_override = bm
	_tree_vp.add_child(wood)
	if leaf_mesh != null:
		var lf: MeshInstance3D = MeshInstance3D.new()
		lf.mesh = leaf_mesh
		var lm: Material = ProcTextures.make_leaf_material(ProcTextures.make_leaf_cluster_texture())
		if lm is StandardMaterial3D:
			(lm as StandardMaterial3D).albedo_color = Color(1.50, 1.70, 0.74)  # fresh spring green
		lf.material_override = lm
		_tree_vp.add_child(lf)
	var sun: DirectionalLight3D = DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-24.0, 68.0, 0.0)   # from the upper left, like the painted sun
	sun.light_energy = 2.3
	sun.light_color = Color(1.0, 0.82, 0.56)
	_tree_vp.add_child(sun)
	var env: Environment = Environment.new()
	env.background_mode = Environment.BG_CLEAR_COLOR
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color(0.36, 0.46, 0.60)
	env.ambient_light_energy = 1.0
	var we: WorldEnvironment = WorldEnvironment.new()
	we.environment = env
	_tree_vp.add_child(we)
	var cam: Camera3D = Camera3D.new()
	cam.projection = Camera3D.PROJECTION_ORTHOGONAL
	cam.size = bb.size.y * 1.04
	cam.position = Vector3(bb.position.x + bb.size.x * 0.5, bb.position.y + bb.size.y * 0.5, 120.0)
	cam.far = 400.0
	_tree_vp.add_child(cam)
	cam.current = true
	var rect: Vector4 = VARIANT1_RECT if variant == 1 else Vector4(0.07, 0.22, 0.54, 0.60)
	_paint_mat.set_shader_parameter("tree_rect", rect)
