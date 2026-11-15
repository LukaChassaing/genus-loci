class_name ProcTextures
extends RefCounted
## Procedurally generated textures and materials (no files on disk). Everything is generated once at startup.

const LEAF_SIZE: int = 256
const BARK_SIZE: int = 256


## Leaf cluster: ~11 overlapping lobed oak leaves (obovate, widest above the middle, sinuate rounded
## lobes, short petiole) on a twig. Near-white/grey albedo (vertex colour drives hue); alpha = silhouette.
## Card UV convention: v = 0 at the distal end, so the twig base is at the bottom of the image.
static func make_leaf_cluster_texture() -> ImageTexture:
	var n: int = LEAF_SIZE
	var data: PackedByteArray = PackedByteArray()
	data.resize(n * n * 4)
	# Background RGB = mid grey so mip-mapping does not bleed black into the silhouette edge.
	for i: int in n * n:
		data[i * 4] = 150
		data[i * 4 + 1] = 150
		data[i * 4 + 2] = 150
		data[i * 4 + 3] = 0
	var rng: RandomNumberGenerator = RandomNumberGenerator.new()
	rng.seed = 20240611
	# Leaves: [origin x, origin y, angle from "up" (rad), length, shade]
	var leaves: Array[Array] = []
	for k: int in 8:
		var y: float = lerpf(0.90, 0.52, float(k) / 7.0)
		var side: float = 1.0 if k % 2 == 0 else -1.0
		var ang: float = side * deg_to_rad(rng.randf_range(32.0, 78.0))
		var tx: float = 0.5 + 0.03 * sin((0.95 - y) * 7.0)
		leaves.append([tx, y, ang, rng.randf_range(0.30, 0.38), rng.randf_range(0.70, 0.95)])
	for a: float in [-0.5, 0.0, 0.5]:
		leaves.append([0.5, 0.5, a + rng.randf_range(-0.1, 0.1), rng.randf_range(0.38, 0.46), rng.randf_range(0.82, 1.0)])
	leaves.shuffle()
	# Twig (dark, thin) drawn first.
	for yi: int in range(int(0.45 * n), n):
		var y2: float = float(yi) / float(n)
		var tx2: float = 0.5 + 0.03 * sin((0.95 - y2) * 7.0)
		for xi: int in range(int((tx2 - 0.012) * n), int((tx2 + 0.012) * n) + 1):
			var o: int = (yi * n + xi) * 4
			data[o] = 96
			data[o + 1] = 86
			data[o + 2] = 74
			data[o + 3] = 255
	for lf: Array in leaves:
		_paint_leaf(data, n, lf[0], lf[1], lf[2], lf[3], lf[4])
	var img: Image = Image.create_from_data(n, n, false, Image.FORMAT_RGBA8, data)
	img.generate_mipmaps()
	return ImageTexture.create_from_image(img)


static func _paint_leaf(data: PackedByteArray, n: int, ox: float, oy: float, ang: float, length: float, shade: float) -> void:
	var dir: Vector2 = Vector2(sin(ang), -cos(ang))  # image y grows downward, "up" = -y
	var perp: Vector2 = Vector2(-dir.y, dir.x)
	var half_w: float = 0.34
	# Bounding box from the rotated rectangle corners.
	var xmin: float = 2.0
	var xmax: float = -1.0
	var ymin: float = 2.0
	var ymax: float = -1.0
	for t: float in [-0.15, 1.0]:
		for s: float in [-half_w, half_w]:
			var p: Vector2 = Vector2(ox, oy) + (dir * t + perp * s) * length
			xmin = minf(xmin, p.x)
			xmax = maxf(xmax, p.x)
			ymin = minf(ymin, p.y)
			ymax = maxf(ymax, p.y)
	var x0: int = maxi(0, int(xmin * n) - 1)
	var x1: int = mini(n - 1, int(xmax * n) + 1)
	var y0: int = maxi(0, int(ymin * n) - 1)
	var y1: int = mini(n - 1, int(ymax * n) + 1)
	var px_l: float = float(n) * length  # pixels per leaf-length unit
	for yi: int in range(y0, y1 + 1):
		for xi: int in range(x0, x1 + 1):
			var p: Vector2 = Vector2((float(xi) + 0.5) / n - ox, (float(yi) + 0.5) / n - oy)
			var t: float = p.dot(dir) / length
			if t < -0.15 or t > 1.0:
				continue
			var s: float = p.dot(perp) / length
			var d: float  # signed distance inside the shape, leaf-length units
			var v: float = shade
			if t < 0.0:
				d = 0.016 - absf(s)  # petiole
				v = shade * 0.7
			else:
				var tb: float = t
				var env: float = sin(PI * pow(tb, 1.35))
				var lobe: float = 0.52 + 0.48 * pow(absf(sin(PI * (4.6 * tb + 0.35))), 0.55)
				var hw: float = half_w * pow(env, 0.85) * lobe
				d = hw - absf(s)
				# Midrib + lateral veins + slight base-to-tip gradient + edge darkening.
				if absf(s) < 0.011:
					v *= 1.18
				else:
					var q: float = absf(fposmod((tb - 0.45 * absf(s)) * 5.0, 1.0) - 0.5)
					if q < 0.035:
						v *= 1.1
				v *= 0.88 + 0.12 * tb
				v *= 0.82 + 0.18 * clampf(d * px_l / 5.0, 0.0, 1.0)
			var a: float = clampf(d * px_l + 0.5, 0.0, 1.0)
			if a <= 0.0:
				continue
			var o: int = (yi * n + xi) * 4
			var g: int = clampi(int(v * 235.0), 0, 255)
			if a >= 1.0 or data[o + 3] < 255:
				# Overwrite where fully covered; blend edge over nothing/other leaf.
				var old_a: float = float(data[o + 3]) / 255.0
				var k: float = a
				data[o] = int(lerpf(float(data[o]), float(g), k))
				data[o + 1] = int(lerpf(float(data[o + 1]), float(g), k))
				data[o + 2] = int(lerpf(float(data[o + 2]), float(g), k))
				data[o + 3] = int(maxf(old_a, a) * 255.0)
			else:
				var k2: float = a
				data[o] = int(lerpf(float(data[o]), float(g), k2))
				data[o + 1] = int(lerpf(float(data[o + 1]), float(g), k2))
				data[o + 2] = int(lerpf(float(data[o + 2]), float(g), k2))


## Tileable bark: returns [albedo ImageTexture, normal ImageTexture]. u wraps around the circumference,
## v runs along the axis; features are stretched along v (vertical furrows).
static func make_bark_textures() -> Array[ImageTexture]:
	var n: int = BARK_SIZE
	var noise: FastNoiseLite = FastNoiseLite.new()
	noise.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	noise.seed = 77
	noise.fractal_type = FastNoiseLite.FRACTAL_FBM
	noise.fractal_octaves = 3
	noise.frequency = 1.0
	var ru: float = 4.5   # radius of the circle in noise space around the trunk (more = more furrows)
	var lv: float = 4.0   # noise-space length of one tile along the axis (more = shorter furrows)
	var cu: PackedFloat32Array = PackedFloat32Array()
	var su: PackedFloat32Array = PackedFloat32Array()
	for i: int in n:
		cu.append(cos(TAU * i / n) * ru)
		su.append(sin(TAU * i / n) * ru)
	var h: PackedFloat32Array = PackedFloat32Array()
	h.resize(n * n)
	for y: int in n:
		var t: float = float(y) / n
		var z: float = t * lv
		for x: int in n:
			# u wraps via the cylinder; v wraps via a cross-fade between the noise and its shifted copy.
			var f: float = lerpf(noise.get_noise_3d(cu[x], su[x], z), noise.get_noise_3d(cu[x], su[x], z - lv), t)
			var r: float = 1.0 - pow(1.0 - absf(f) * 2.2, 2.0) if absf(f) < 0.4545 else 1.0
			h[y * n + x] = clampf(r, 0.0, 1.0)
	var albedo: PackedByteArray = PackedByteArray()
	albedo.resize(n * n * 3)
	var nrm: PackedByteArray = PackedByteArray()
	nrm.resize(n * n * 3)
	for y: int in n:
		for x: int in n:
			var i: int = y * n + x
			var hv: float = h[i]
			var hx: float = h[y * n + (x + 1) % n] - h[y * n + (x + n - 1) % n]
			var hy: float = h[((y + 1) % n) * n + x] - h[((y + n - 1) % n) * n + x]
			var g: float = 0.50 + 0.50 * hv
			var tint: float = g * (0.97 + 0.03 * hy)
			albedo[i * 3] = clampi(int(tint * 255.0 * 1.02), 0, 255)
			albedo[i * 3 + 1] = clampi(int(tint * 255.0), 0, 255)
			albedo[i * 3 + 2] = clampi(int(tint * 255.0 * 0.97), 0, 255)
			var nv: Vector3 = Vector3(-hx * 2.2, hy * 2.2, 1.0).normalized()
			nrm[i * 3] = int((nv.x * 0.5 + 0.5) * 255.0)
			nrm[i * 3 + 1] = int((nv.y * 0.5 + 0.5) * 255.0)
			nrm[i * 3 + 2] = int((nv.z * 0.5 + 0.5) * 255.0)
	var ai: Image = Image.create_from_data(n, n, false, Image.FORMAT_RGB8, albedo)
	ai.generate_mipmaps()
	var ni: Image = Image.create_from_data(n, n, false, Image.FORMAT_RGB8, nrm)
	ni.generate_mipmaps()
	return [ImageTexture.create_from_image(ai), ImageTexture.create_from_image(ni)]


static func make_leaf_material(tex: ImageTexture) -> StandardMaterial3D:
	var m: StandardMaterial3D = StandardMaterial3D.new()
	m.albedo_texture = tex
	m.vertex_color_use_as_albedo = true
	m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR
	m.alpha_scissor_threshold = 0.5
	m.alpha_antialiasing_mode = BaseMaterial3D.ALPHA_ANTIALIASING_ALPHA_TO_COVERAGE
	m.cull_mode = BaseMaterial3D.CULL_DISABLED
	m.roughness = 0.9
	m.metallic_specular = 0.0
	m.backlight_enabled = true
	m.backlight = Color(0.42, 0.46, 0.26)
	m.texture_filter = BaseMaterial3D.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS_ANISOTROPIC
	return m


static func make_bark_material(tex: Array[ImageTexture]) -> StandardMaterial3D:
	var m: StandardMaterial3D = StandardMaterial3D.new()
	m.albedo_texture = tex[0]
	m.vertex_color_use_as_albedo = true
	m.normal_enabled = true
	m.normal_texture = tex[1]
	m.normal_scale = 1.0
	m.roughness = 0.95
	m.metallic_specular = 0.15
	m.texture_filter = BaseMaterial3D.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS_ANISOTROPIC
	return m


## Roots are drawn as a flattened relief in front of the soil cut face (profile illustration):
## z is squeezed to a thin sheet just in front of z = 0, so the whole root system reads on the cut.
static func make_root_material(tex: Array[ImageTexture]) -> ShaderMaterial:
	var sh: Shader = Shader.new()
	sh.code = """
shader_type spatial;
render_mode cull_back;
uniform sampler2D albedo_tex : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D normal_tex : hint_normal, filter_linear_mipmap_anisotropic, repeat_enable;
uniform float flatten = 0.012;
uniform float lift = 0.10;
void vertex() {
	VERTEX.z = lift + clamp(VERTEX.z, -12.0, 12.0) * flatten;
}
void fragment() {
	vec3 a = texture(albedo_tex, UV).rgb;
	ALBEDO = a * COLOR.rgb * 1.15;
	NORMAL_MAP = texture(normal_tex, UV).xyz;
	NORMAL_MAP_DEPTH = 0.8;
	ROUGHNESS = 0.95;
	SPECULAR = 0.1;
}
"""
	var m: ShaderMaterial = ShaderMaterial.new()
	m.shader = sh
	m.set_shader_parameter("albedo_tex", tex[0])
	m.set_shader_parameter("normal_tex", tex[1])
	return m
