class_name ProcTextures
extends RefCounted
## Procedurally generated textures and materials (no files on disk). Everything is generated once at startup.

const LEAF_SIZE: int = 256
const BARK_SIZE: int = 256


## Leaf cluster: ~11 overlapping lobed oak leaves (obovate, widest above the middle, sinuate rounded
## lobes, short petiole) on a twig. Near-white/grey albedo (vertex colour drives hue); alpha = silhouette.
## Card UV convention: v = 0 at the distal end, so the twig base is at the bottom of the image.
## `lobes` > 0 gives the oak look; lobes == 0 gives unlobed ovate-acuminate leaves whose margin is
## toothed by `serration` (wild cherry), hanging a little. `length_m` / `width_m` set the aspect ratio.
static func make_leaf_cluster_texture(lobes: int = 10, serration: float = 0.0, length_m: float = 0.10,
		width_m: float = 0.06) -> ImageTexture:
	var n: int = LEAF_SIZE
	var cfg: Dictionary = {"lobed": lobes > 0, "serr": serration, "half_w": 0.34, "bend": 0.0, "veins": 5.0}
	if lobes <= 0:
		cfg["half_w"] = clampf(0.52 * width_m / maxf(length_m, 0.01), 0.18, 0.40)
		cfg["bend"] = 0.22
		cfg["veins"] = 8.0
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
	var lobed: bool = lobes > 0
	var n_side: int = 8 if lobed else 6
	for k: int in n_side:
		var y: float = lerpf(0.90, 0.52, float(k) / float(n_side - 1))
		var side: float = 1.0 if k % 2 == 0 else -1.0
		var a_deg: float = rng.randf_range(32.0, 78.0) if lobed else rng.randf_range(48.0, 92.0)
		var ang: float = side * deg_to_rad(a_deg)
		var tx: float = 0.5 + 0.03 * sin((0.95 - y) * 7.0)
		var ln: float = rng.randf_range(0.30, 0.38) if lobed else rng.randf_range(0.36, 0.44)
		leaves.append([tx, y, ang, ln, rng.randf_range(0.70, 0.95)])
	var tip_angles: Array = [-0.5, 0.0, 0.5] if lobed else [-0.45, 0.45]
	for a: float in tip_angles:
		var ln2: float = rng.randf_range(0.38, 0.46) if lobed else rng.randf_range(0.44, 0.52)
		leaves.append([0.5, 0.5, a + rng.randf_range(-0.1, 0.1), ln2, rng.randf_range(0.82, 1.0)])
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
		_paint_leaf(data, n, lf[0], lf[1], lf[2], lf[3], lf[4], cfg)
	var img: Image = Image.create_from_data(n, n, false, Image.FORMAT_RGBA8, data)
	img.generate_mipmaps()
	return ImageTexture.create_from_image(img)


static func _paint_leaf(data: PackedByteArray, n: int, ox: float, oy: float, ang: float, length: float, shade: float,
		cfg: Dictionary) -> void:
	var dir: Vector2 = Vector2(sin(ang), -cos(ang))  # image y grows downward, "up" = -y
	var perp: Vector2 = Vector2(-dir.y, dir.x)
	var half_w: float = cfg["half_w"]
	var lobed: bool = cfg["lobed"]
	var serr: float = cfg["serr"]
	var veins: float = cfg["veins"]
	# Droop: the blade sags to the side it points to (sag grows with t^2 along the blade).
	var bend: float = cfg["bend"]
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
	ymax += bend * length
	var x0: int = maxi(0, int(xmin * n) - 1)
	var x1: int = mini(n - 1, int(xmax * n) + 1)
	var y0: int = maxi(0, int(ymin * n) - 1)
	var y1: int = mini(n - 1, int(ymax * n) + 1)
	var px_l: float = float(n) * length  # pixels per leaf-length unit
	for yi: int in range(y0, y1 + 1):
		for xi: int in range(x0, x1 + 1):
			var p: Vector2 = Vector2((float(xi) + 0.5) / n - ox, (float(yi) + 0.5) / n - oy)
			if bend != 0.0:
				var t0: float = maxf(p.dot(dir) / length, 0.0)
				p.y -= bend * t0 * t0 * length  # sag: sample the undrooped leaf from higher up
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
				var hw: float
				if lobed:
					var env: float = sin(PI * pow(tb, 1.35))
					var lobe: float = 0.52 + 0.48 * pow(absf(sin(PI * (4.6 * tb + 0.35))), 0.55)
					hw = half_w * pow(env, 0.85) * lobe
				else:
					# Ovate, widest ~40 % from the base, drawn out into a pointed tip; toothed margin.
					var env2: float = sin(PI * pow(tb, 0.75))
					hw = half_w * pow(maxf(env2, 0.0), 0.8) * (1.0 - 0.35 * pow(tb, 4.0))
					var saw: float = absf(fposmod(tb * 22.0, 1.0) - 0.5) * 2.0
					hw *= 1.0 - serr * 0.06 * (1.0 - saw) - serr * 0.02
				d = hw - absf(s)
				# Midrib + lateral veins + slight base-to-tip gradient + edge darkening.
				if absf(s) < 0.011:
					v *= 1.18
				else:
					var q: float = absf(fposmod((tb - 0.45 * absf(s)) * veins, 1.0) - 0.5)
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
## `fissures` 0..1 (oak 0.85 = the original deep furrows; low = smooth periderm) and `lenticels` 0..1
## (horizontal bands of pale lenticels, wild cherry) pick the species look.
static func make_bark_textures(fissures: float = 0.85, lenticels: float = 0.0) -> Array[ImageTexture]:
	var n: int = BARK_SIZE
	var noise: FastNoiseLite = FastNoiseLite.new()
	noise.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	noise.seed = 77
	noise.fractal_type = FastNoiseLite.FRACTAL_FBM
	noise.fractal_octaves = 3
	noise.frequency = 1.0
	var fiss_amt: float = clampf(fissures / 0.85, 0.0, 1.0)
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
			if fiss_amt < 1.0:
				# Smooth periderm: only a faint, low-contrast undulation of the same noise.
				r = lerpf(0.93 + 0.07 * r, r, fiss_amt)
			h[y * n + x] = clampf(r, 0.0, 1.0)
	var len_mask: PackedFloat32Array = PackedFloat32Array()
	len_mask.resize(n * n)
	if lenticels > 0.01:
		var ln: FastNoiseLite = FastNoiseLite.new()
		ln.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
		ln.seed = 311
		ln.frequency = 1.0
		var lru: float = 1.6    # few features around the stem: wide dashes
		var llv: float = 14.0   # many bands along the axis (integer cycles over one tile)
		for y: int in n:
			var tz: float = float(y) / n
			var zz: float = tz * llv
			for x: int in n:
				var cx: float = cos(TAU * x / n) * lru
				var sx: float = sin(TAU * x / n) * lru
				var band: float = 0.5 + 0.5 * cos(TAU * tz * 16.0 + 2.2 * ln.get_noise_3d(cx * 0.8, sx * 0.8, 3.0))
				var dash: float = lerpf(ln.get_noise_3d(cx * 2.0, sx * 2.0, zz), ln.get_noise_3d(cx * 2.0, sx * 2.0, zz - llv), tz)
				var m: float = smoothstep(0.82, 0.97, band) * smoothstep(-0.05, 0.25, dash)
				len_mask[y * n + x] = m * lenticels
	var albedo: PackedByteArray = PackedByteArray()
	albedo.resize(n * n * 3)
	var nrm: PackedByteArray = PackedByteArray()
	nrm.resize(n * n * 3)
	for y: int in n:
		for x: int in n:
			var i: int = y * n + x
			var lm: float = len_mask[i]
			var hv: float = h[i] + 0.12 * lm if fiss_amt < 1.0 else h[i]
			var hx: float = h[y * n + (x + 1) % n] - h[y * n + (x + n - 1) % n]
			var hy: float = h[((y + 1) % n) * n + x] - h[((y + n - 1) % n) * n + x]
			var g: float = 0.50 + 0.50 * hv
			if fiss_amt < 1.0:
				# Smooth bark: calmer base value, lenticels pale and cool-grey against it.
				g = lerpf(0.60 + 0.08 * hv, g, fiss_amt)
				g = lerpf(g, 1.0, clampf(lm * 1.6, 0.0, 1.0))
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


## Species look of the bark material: glossy and smooth (cherry) vs dry and rough (oak, gloss 0 = the
## original values).
static func set_bark_look(m: StandardMaterial3D, gloss: float, fissures: float) -> void:
	m.roughness = lerpf(0.95, 0.42, clampf(gloss, 0.0, 1.0))
	m.metallic_specular = lerpf(0.15, 0.55, clampf(gloss, 0.0, 1.0))
	m.normal_scale = lerpf(0.35, 1.0, clampf(fissures / 0.85, 0.0, 1.0))


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


## Roots: real 3-D geometry, clipped at the plug radius / depth (discard). The inside of a clipped tube
## (back faces) is painted with `cap_color`, so cut root ends read as darker discs. `fine_style` (0..1)
## switches to a thinner/paler look for fine roots once the mesh carries per-vertex root class info.
static func make_root_material(tex: Array[ImageTexture]) -> ShaderMaterial:
	var sh: Shader = Shader.new()
	sh.code = """
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
void vertex() {
	wpos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
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
	}
}
"""
	var m: ShaderMaterial = ShaderMaterial.new()
	m.shader = sh
	m.set_shader_parameter("albedo_tex", tex[0])
	m.set_shader_parameter("normal_tex", tex[1])
	return m


# ---------------------------------------------------------------------------------------------
# Flowers and fruits (one material, two procedural textures)
# ---------------------------------------------------------------------------------------------

const BLOSSOM_SIZE: int = 256


static func _new_card_image(n: int, bg: int) -> PackedByteArray:
	var data: PackedByteArray = PackedByteArray()
	data.resize(n * n * 4)
	for i: int in n * n:
		data[i * 4] = bg
		data[i * 4 + 1] = bg
		data[i * 4 + 2] = bg
		data[i * 4 + 3] = 0
	return data


static func _put(data: PackedByteArray, n: int, xi: int, yi: int, r: float, g: float, b: float, a: float) -> void:
	if xi < 0 or yi < 0 or xi >= n or yi >= n or a <= 0.0:
		return
	var o: int = (yi * n + xi) * 4
	var old_a: float = float(data[o + 3]) / 255.0
	var k: float = clampf(a, 0.0, 1.0)
	data[o] = int(lerpf(float(data[o]), r * 255.0, k))
	data[o + 1] = int(lerpf(float(data[o + 1]), g * 255.0, k))
	data[o + 2] = int(lerpf(float(data[o + 2]), b * 255.0, k))
	data[o + 3] = int(maxf(old_a, k) * 255.0)


## A short stalk (quadratic curve) from `a` to `c` through control point `b`, UV units.
static func _stalk(data: PackedByteArray, n: int, a: Vector2, b: Vector2, c: Vector2, half_w: float, col: Color) -> void:
	var steps: int = 60
	for i: int in steps + 1:
		var t: float = float(i) / float(steps)
		var p: Vector2 = a.lerp(b, t).lerp(b.lerp(c, t), t)
		var rpx: float = half_w * n
		for dy: int in range(-ceili(rpx) - 1, ceili(rpx) + 2):
			for dx: int in range(-ceili(rpx) - 1, ceili(rpx) + 2):
				var px: int = int(p.x * n) + dx
				var py: int = int(p.y * n) + dy
				var dd: float = Vector2(float(dx), float(dy)).length()
				_put(data, n, px, py, col.r, col.g, col.b, clampf(rpx - dd + 0.5, 0.0, 1.0))


## Cluster of small five-petal blossoms on thin pedicels. Near-white albedo (vertex colour tints it).
static func make_flower_texture() -> ImageTexture:
	var n: int = BLOSSOM_SIZE
	var data: PackedByteArray = _new_card_image(n, 215)
	var rng: RandomNumberGenerator = RandomNumberGenerator.new()
	rng.seed = 5150
	# Blossoms: [centre, radius, rotation]
	var blossoms: Array[Array] = [
		[Vector2(0.50, 0.19), 0.19, 0.2], [Vector2(0.25, 0.33), 0.18, 1.1], [Vector2(0.76, 0.32), 0.18, 0.6],
		[Vector2(0.45, 0.46), 0.18, 0.0], [Vector2(0.68, 0.56), 0.17, 1.4], [Vector2(0.24, 0.62), 0.16, 0.8],
		[Vector2(0.80, 0.76), 0.15, 0.3], [Vector2(0.48, 0.70), 0.17, 1.0], [Vector2(0.30, 0.86), 0.12, 0.5],
		[Vector2(0.64, 0.88), 0.12, 0.9]]
	var base: Vector2 = Vector2(0.5, 1.0)
	for b: Array in blossoms:
		var c: Vector2 = b[0]
		_stalk(data, n, base, Vector2(c.x, lerpf(base.y, c.y, 0.55)), c, 0.008, Color(0.45, 0.55, 0.30))
	for b: Array in blossoms:
		var c: Vector2 = b[0]
		var rad: float = b[1]
		var rot: float = b[2]
		var x0: int = maxi(0, int((c.x - rad * 1.1) * n))
		var x1: int = mini(n - 1, int((c.x + rad * 1.1) * n))
		var y0: int = maxi(0, int((c.y - rad * 1.1) * n))
		var y1: int = mini(n - 1, int((c.y + rad * 1.1) * n))
		for yi: int in range(y0, y1 + 1):
			for xi: int in range(x0, x1 + 1):
				var q: Vector2 = Vector2((float(xi) + 0.5) / n - c.x, (float(yi) + 0.5) / n - c.y)
				var r: float = q.length() / rad
				if r > 1.1:
					continue
				var th: float = atan2(q.y, q.x) - rot
				# Five broad rounded petals with a small notch at the tip.
				var pc: float = absf(cos(2.5 * th))
				var edge: float = 0.62 + 0.38 * pow(pc, 0.55)
				var notch: float = 1.0 - 0.10 * pow(maxf(0.0, 1.0 - pc * 6.0), 2.0)
				var d: float = (edge * notch - r) * rad * n
				var a: float = clampf(d + 0.5, 0.0, 1.0)
				if a <= 0.0:
					continue
				# White petals, faintly pink-grey towards the base, with a yellow-green centre and stamens.
				var v: float = 0.96 - 0.10 * (1.0 - clampf(r / 0.7, 0.0, 1.0)) - 0.05 * pow(1.0 - pc, 3.0)
				var col: Color = Color(v, v * 0.985, v * 0.99)
				if r < 0.22:
					col = col.lerp(Color(0.80, 0.72, 0.32), smoothstep(0.22, 0.12, r))
				elif r < 0.45 and fposmod(th * 10.0 / TAU * 1.0, 1.0) < 0.18:
					col = col.lerp(Color(0.85, 0.75, 0.40), 0.55)
				_put(data, n, xi, yi, col.r, col.g, col.b, a)
	var img: Image = Image.create_from_data(n, n, false, Image.FORMAT_RGBA8, data)
	img.generate_mipmaps()
	return ImageTexture.create_from_image(img)


## Small cluster of round, glossy cherries on long stalks hanging from the bottom-centre... the card's
## v = 0 edge is the distal end, so the stalks rise from the bottom of the image and the fruit hang at
## the top. Albedo is a grey shading (vertex colour gives green -> red).
static func make_fruit_texture() -> ImageTexture:
	var n: int = BLOSSOM_SIZE
	var data: PackedByteArray = _new_card_image(n, 190)
	var base: Vector2 = Vector2(0.5, 1.0)
	var cherries: Array[Array] = [[Vector2(0.30, 0.34), 0.17], [Vector2(0.68, 0.30), 0.18], [Vector2(0.50, 0.12), 0.15]]
	for c: Array in cherries:
		var p: Vector2 = c[0]
		_stalk(data, n, base, Vector2(lerpf(base.x, p.x, 0.3), lerpf(base.y, p.y, 0.5)), p - Vector2(0.0, c[1] * 0.8), 0.010, Color(0.50, 0.60, 0.30))
	for c: Array in cherries:
		var p: Vector2 = c[0]
		var rad: float = c[1]
		var x0: int = maxi(0, int((p.x - rad * 1.2) * n))
		var x1: int = mini(n - 1, int((p.x + rad * 1.2) * n))
		var y0: int = maxi(0, int((p.y - rad * 1.2) * n))
		var y1: int = mini(n - 1, int((p.y + rad * 1.2) * n))
		for yi: int in range(y0, y1 + 1):
			for xi: int in range(x0, x1 + 1):
				var q: Vector2 = Vector2((float(xi) + 0.5) / n - p.x, (float(yi) + 0.5) / n - p.y)
				var r: float = q.length() / rad
				if r > 1.05:
					continue
				var a: float = clampf((1.0 - r) * rad * n + 0.5, 0.0, 1.0)
				var qn: Vector2 = q / rad
				var z: float = sqrt(maxf(0.0, 1.0 - qn.length_squared()))
				var nrm: Vector3 = Vector3(qn.x, qn.y, z)
				var diff: float = clampf(nrm.dot(Vector3(-0.45, -0.6, 0.65).normalized()), 0.0, 1.0)
				var v: float = 0.50 + 0.50 * diff
				# Stem dimple at the top and a small specular highlight.
				v *= 1.0 - 0.45 * smoothstep(0.30, 0.0, (qn - Vector2(0.0, -0.78)).length())
				var spec: float = pow(clampf(nrm.dot(Vector3(-0.35, -0.5, 0.79).normalized()), 0.0, 1.0), 28.0)
				var col: Color = Color(v, v, v).lerp(Color(1.4, 1.4, 1.4), spec * 0.8)
				col = Color(minf(col.r, 1.0), minf(col.g, 1.0), minf(col.b, 1.0))
				_put(data, n, xi, yi, col.r, col.g, col.b, a)
	var img: Image = Image.create_from_data(n, n, false, Image.FORMAT_RGBA8, data)
	img.generate_mipmaps()
	return ImageTexture.create_from_image(img)


## One material for the flower mesh: cards with vertex alpha >= 0.75 are blossoms, the others fruit.
## Visibility is per card: a hash of the card's normal is compared with the current flower / fruit
## density (petal fall thins the blossoms out card by card instead of fading them).
static func make_flower_material(flower_tex: ImageTexture, fruit_tex: ImageTexture) -> ShaderMaterial:
	var sh: Shader = Shader.new()
	sh.code = """
shader_type spatial;
render_mode cull_disabled, alpha_to_coverage;
uniform sampler2D flower_tex : source_color, filter_linear_mipmap_anisotropic, repeat_disable;
uniform sampler2D fruit_tex : source_color, filter_linear_mipmap_anisotropic, repeat_disable;
uniform float flower_keep = 1.0;
uniform float fruit_keep = 1.0;
varying flat float card_hash;
varying flat float is_fruit;
void vertex() {
	card_hash = fract(sin(dot(NORMAL, vec3(12.9898, 78.233, 37.719))) * 43758.5453);
	is_fruit = COLOR.a < 0.75 ? 1.0 : 0.0;
}
void fragment() {
	float keep = mix(flower_keep, fruit_keep, is_fruit);
	if (card_hash > keep) {
		discard;
	}
	vec4 tf = texture(flower_tex, UV);
	vec4 tr = texture(fruit_tex, UV);
	vec4 t = mix(tf, tr, is_fruit);
	ALBEDO = t.rgb * COLOR.rgb;
	// Mip-mapping averages the cut-outs away at a distance: push the alpha back up so a crown in bloom stays a cloud.
	ALPHA = clamp((t.a - 0.15) * 1.7, 0.0, 1.0);
	ROUGHNESS = mix(0.8, 0.35, is_fruit);
	SPECULAR = mix(0.2, 0.6, is_fruit);
	BACKLIGHT = ALBEDO * 0.35 * (1.0 - is_fruit);
	EMISSION = ALBEDO * 0.22 * (1.0 - is_fruit);
}
"""
	var m: ShaderMaterial = ShaderMaterial.new()
	m.shader = sh
	m.set_shader_parameter("flower_tex", flower_tex)
	m.set_shader_parameter("fruit_tex", fruit_tex)
	return m
