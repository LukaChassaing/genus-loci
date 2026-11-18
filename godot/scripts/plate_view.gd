class_name PlateView
extends Control
## "Planche" of a species, drawn entirely in code like an antique herbarium print: aged foxed paper, an
## engraved moulded frame with a baroque cartouche, specimens in watercolour wash with sepia engraved
## contours and hatching, italic serif captions. Leaf shape uses the TreeSim leaf getters.
## One design space (PLATE_W x PLATE_H) is scaled to fit; text is drawn at the scaled size (stays sharp).

signal closed

const PLATE_W: float = 1500.0
const PLATE_H: float = 880.0
const STRIP_H: float = 46.0
const SEPIA: Color = Color(0.22, 0.15, 0.09)
const SEPIA_MID: Color = Color(0.37, 0.27, 0.17)
const PAPER: Color = Color(0.915, 0.865, 0.715)
const LIGHT: Vector2 = Vector2(-0.62, -0.78)   # direction toward the light (upper left)
const LBL: int = 15

var species: int = 0
var _ts: TreeSim = TreeSim.new()
var _s: float = 1.0
var _o: Vector2 = Vector2.ZERO
var _back: Button
var _seg: Segmented
var _title_lbl: Label
var _rng: RandomNumberGenerator = RandomNumberGenerator.new()


## Hand-drawn plates exist for these registry ids only (index = plate number, the `species` of the drawing code).
const PLATE_IDS: Array[String] = ["quercus_robur", "prunus_avium"]
static var _reg: TreeSim


static func _registry() -> TreeSim:
	if _reg == null:
		_reg = TreeSim.new()
	return _reg


## Plate number (0 oak, 1 cherry) of a TreeSim species index, -1 when the species has no hand-drawn plate.
static func plate_of(tree_index: int) -> int:
	var ids: PackedStringArray = _registry().get_species_ids()
	if tree_index < 0 or tree_index >= ids.size():
		return -1
	return PLATE_IDS.find(ids[tree_index])


static func has_plate(tree_index: int) -> bool:
	return plate_of(tree_index) >= 0


## Static species facts (kept modest and verifiable). `i` = TreeSim species index. Species without a
## hand-written card get their names and category from the core only (no unsourced facts).
static func species_data(i: int) -> Dictionary:
	var plate: int = plate_of(i)
	if plate < 0:
		var reg: TreeSim = _registry()
		var names: PackedStringArray = reg.get_species_common_names()
		var latin: PackedStringArray = reg.get_species_names()
		var ids: PackedStringArray = reg.get_species_ids()
		var cats: PackedStringArray = reg.get_species_categories()
		var k: int = clampi(i, 0, names.size() - 1)
		return {
			"id": ids[k], "common": names[k], "title": names[k], "latin": latin[k], "auth": "", "family": cats[k],
			"ordinal": "", "height": "—", "longevity": "—", "flowering": "—", "fruit": "—", "soil": "—",
			"caption": "",
			"blurb": "%s (%s). La planche illustrée de cette espèce n'est pas encore dessinée ; ses traits et leurs sources sont dans docs/species/%s.md." % [names[k], cats[k].to_lower(), ids[k]],
		}
	if plate == 1:
		return {
			"id": "prunus_avium", "common": "Merisier", "title": "Le Merisier", "latin": "Prunus avium",
			"auth": "L.", "family": "famille des Rosacées", "ordinal": "II",
			"height": "15 à 25 m, parfois 30 m",
			"longevity": "60 à 100 ans environ",
			"flowering": "mars-avril, avant ou avec les feuilles",
			"fruit": "la merise, mûre en juin-juillet",
			"soil": "profond, frais et bien drainé ; craint l'engorgement",
			"caption": "Pourquoi le merisier fleurit-il avant ses feuilles ? Ses boutons floraux sont formés dès l'été précédent : ils s'ouvrent aux premiers jours doux, quand les feuilles ne cachent pas encore les fleurs aux insectes pollinisateurs.",
			"blurb": "Arbre de lisière à croissance rapide, plus vite fleuri mais de vie plus brève que le chêne. Ses fleurs blanches s'ouvrent au printemps, ses fruits rouges, les merises, mûrissent en juin.",
		}
	return {
		"id": "quercus_robur", "common": "Chêne pédonculé", "title": "Le Chêne pédonculé", "latin": "Quercus robur",
		"auth": "L.", "family": "famille des Fagacées", "ordinal": "I",
		"height": "25 à 35 m, parfois 40 m",
		"longevity": "500 à 1 000 ans, parfois davantage",
		"flowering": "avril-mai, en même temps que les feuilles",
		"fruit": "le gland, mûr en septembre-octobre",
		"soil": "profond et frais ; tolère l'engorgement temporaire",
		"caption": "Pourquoi le chêne a-t-il de si gros vaisseaux au printemps ? Il les forme avant le débourrement pour conduire l'eau très vite ; ils ne servent guère qu'une saison, et leur anneau dessine chaque cerne.",
		"blurb": "Grand arbre des plaines et des vallées d'Europe, lent à croître et capable de vivre plusieurs siècles. Ses glands, portés par un long pédoncule, lui ont donné son nom.",
	}


func _ready() -> void:
	clip_contents = true
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	theme = UiStyle.make_theme(16)
	var strip: PanelContainer = PanelContainer.new()
	strip.set_anchors_and_offsets_preset(Control.PRESET_TOP_WIDE)
	strip.custom_minimum_size = Vector2(0, STRIP_H)
	strip.add_theme_stylebox_override("panel", UiStyle.box(Color(UiStyle.PANEL.r, UiStyle.PANEL.g, UiStyle.PANEL.b, 0.96), UiStyle.LINE, 1, 14.0, 5.0))
	add_child(strip)
	var h: HBoxContainer = HBoxContainer.new()
	h.add_theme_constant_override("separation", 14)
	strip.add_child(h)
	_back = UiStyle.button("‹  Retour", 15, HORIZONTAL_ALIGNMENT_CENTER)
	_back.custom_minimum_size = Vector2(110, 32)
	_back.pressed.connect(func() -> void: closed.emit())
	h.add_child(_back)
	h.add_child(UiStyle.caps_label("Planches botaniques", 12))
	var sp: Control = Control.new()
	sp.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	h.add_child(sp)
	_seg = Segmented.new().setup(["Chêne pédonculé", "Merisier"], species, 2, 14)
	_seg.custom_minimum_size = Vector2(300, 0)
	_seg.picked.connect(_show_plate)
	h.add_child(_seg)
	resized.connect(queue_redraw)
	_apply_species()


## Shows the plate of TreeSim species `i` (the oak's when that species has no plate yet).
func set_species(i: int) -> void:
	_show_plate(maxi(plate_of(i), 0))


## Shows plate number `p` (0 oak, 1 cherry).
func _show_plate(p: int) -> void:
	species = clampi(p, 0, PLATE_IDS.size() - 1)
	if _seg != null:
		_seg.select(species)
	_apply_species()
	queue_redraw()


func _apply_species() -> void:
	_ts.set_species(_ts.find_species(PLATE_IDS[species]))
	_ts.reset(1)


func _unhandled_input(event: InputEvent) -> void:
	if visible and event is InputEventKey and event.pressed and not event.echo:
		var k: int = (event as InputEventKey).keycode
		if k == KEY_ESCAPE:
			closed.emit()
			get_viewport().set_input_as_handled()
		elif k == KEY_LEFT or k == KEY_RIGHT:
			_show_plate(1 - species)
			get_viewport().set_input_as_handled()


# ---------------------------------------------- helpers ----------------------------------------

static var _pfonts: Dictionary = {}


## Plate serif: the narrowest Times/Garamond-like face available, for a printed-book feel.
static func pf(weight: int = 400, italic: bool = false, tracking: int = 0) -> Font:
	var key: String = "%d|%s|%d" % [weight, str(italic), tracking]
	if _pfonts.has(key):
		return _pfonts[key]
	var sf: SystemFont = SystemFont.new()
	sf.font_names = PackedStringArray(["EB Garamond", "Cormorant Garamond", "Liberation Serif", "Times New Roman", "Noto Serif", "serif"])
	sf.font_weight = weight
	sf.font_italic = italic
	sf.antialiasing = TextServer.FONT_ANTIALIASING_GRAY
	var f: Font = sf
	if tracking != 0:
		var fv: FontVariation = FontVariation.new()
		fv.base_font = sf
		fv.spacing_glyph = tracking
		f = fv
	_pfonts[key] = f
	return f


static func _h(x: float) -> float:
	return fposmod(sin(x * 12.9898 + 78.233) * 43758.5453, 1.0)


func _shapes() -> void:
	draw_set_transform(_o, 0.0, Vector2(_s, _s))


func _text_mode() -> void:
	draw_set_transform_matrix(Transform2D.IDENTITY)


func _txt(s: String, p: Vector2, size: int, col: Color, f: Font) -> void:
	_text_mode()
	var fs: int = maxi(8, roundi(float(size) * _s))
	draw_string(f, _o + p * _s, s, HORIZONTAL_ALIGNMENT_LEFT, -1.0, fs, col)
	_shapes()


func _ctr(s: String, p: Vector2, size: int, col: Color, f: Font) -> void:
	_txt(s, Vector2(p.x - _text_w(s, size, f) * 0.5, p.y), size, col, f)


func _text_w(s: String, size: int, f: Font) -> float:
	return f.get_string_size(s, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x


func _close(pts: PackedVector2Array) -> PackedVector2Array:
	var c: PackedVector2Array = pts.duplicate()
	c.append(pts[0])
	return c


func _ellipse(c: Vector2, rx: float, ry: float, rot: float = 0.0, n: int = 36) -> PackedVector2Array:
	var pts: PackedVector2Array = PackedVector2Array()
	for k: int in n:
		var a: float = TAU * float(k) / float(n)
		pts.append(c + Vector2(cos(a) * rx, sin(a) * ry).rotated(rot))
	return pts


func _blob(c: Vector2, r: float, seed_i: int, amp: float, n: int = 120, jag: float = 0.0) -> PackedVector2Array:
	var rg: RandomNumberGenerator = RandomNumberGenerator.new()
	rg.seed = seed_i
	var ph: Array[float] = [rg.randf() * TAU, rg.randf() * TAU, rg.randf() * TAU]
	var pts: PackedVector2Array = PackedVector2Array()
	for k: int in n:
		var a: float = TAU * float(k) / float(n)
		var m: float = 1.0 + amp * (sin(3.0 * a + ph[0]) * 0.5 + sin(5.0 * a + ph[1]) * 0.3 + sin(9.0 * a + ph[2]) * 0.2)
		if jag > 0.0:
			m += jag * (rg.randf() - 0.5)
		pts.append(c + Vector2(cos(a), sin(a)) * r * m)
	return pts


func _centroid(pts: PackedVector2Array) -> Vector2:
	var c: Vector2 = Vector2.ZERO
	for p: Vector2 in pts:
		c += p
	return c / float(maxi(pts.size(), 1))


func _scaled(pts: PackedVector2Array, c: Vector2, k: float, shift: Vector2 = Vector2.ZERO) -> PackedVector2Array:
	var out: PackedVector2Array = PackedVector2Array()
	for p: Vector2 in pts:
		out.append(c + (p - c) * k + shift)
	return out


func _valid(pts: PackedVector2Array) -> bool:
	return pts.size() >= 3 and not Geometry2D.triangulate_polygon(pts).is_empty()


## Fine engraved line: width varies along the stroke and swells on the shadow side of `around`.
func _ink(pts: PackedVector2Array, w: float, col: Color = SEPIA, seed_i: int = 0, closed: bool = false, around: Vector2 = Vector2.INF) -> void:
	var n: int = pts.size()
	if n < 2:
		return
	var segs: int = n if closed else n - 1
	for i: int in segs:
		var a: Vector2 = pts[i]
		var b: Vector2 = pts[(i + 1) % n]
		var k: float = 0.78 + 0.28 * sin(float(i) * 0.83 + float(seed_i)) + 0.18 * (_h(float(i) + float(seed_i)) - 0.5)
		if around != Vector2.INF:
			var nrm: Vector2 = ((a + b) * 0.5 - around).normalized()
			k += 0.65 * maxf(0.0, nrm.dot(-LIGHT))
		draw_line(a, b, col, maxf(w * k, 0.4), true)


## Parallel hatching clipped to `regions` (polygons), at angle `ang` (radians) with spacing `gap`.
func _hatch(regions: Array, ang: float, gap: float, col: Color, w: float, seed_i: int) -> void:
	if regions.is_empty():
		return
	var bb: Rect2 = Rect2((regions[0] as PackedVector2Array)[0], Vector2.ZERO)
	for reg: PackedVector2Array in regions:
		for p: Vector2 in reg:
			bb = bb.expand(p)
	var dir: Vector2 = Vector2.from_angle(ang)
	var nrm: Vector2 = Vector2(-dir.y, dir.x)
	var cen: Vector2 = bb.get_center()
	var rad: float = bb.size.length() * 0.5 + 1.0
	var t: float = -rad
	var i: int = 0
	while t < rad:
		var a: Vector2 = cen + nrm * t - dir * rad
		var b: Vector2 = cen + nrm * t + dir * rad
		for reg: PackedVector2Array in regions:
			var res: Array[PackedVector2Array] = Geometry2D.intersect_polyline_with_polygon(PackedVector2Array([a, b]), reg)
			for seg: PackedVector2Array in res:
				if seg.size() >= 2:
					var s0: Vector2 = seg[0]
					var s1: Vector2 = seg[seg.size() - 1]
					var sh: float = 0.06 + 0.18 * _h(float(i) * 1.7 + float(seed_i))
					draw_line(s0.lerp(s1, sh * 0.5), s1.lerp(s0, sh), col, w * (0.65 + 0.55 * _h(float(i) + float(seed_i))), true)
		t += gap * (0.8 + 0.5 * _h(float(i) * 1.3 + float(seed_i)))
		i += 1


## Watercolour wash with an engraved finish: flat colour, uneven pooled pigment, paper showing through
## on the lit side, hatched shadow on the other, and a sepia contour of varying weight.
func _wash(pts_in: PackedVector2Array, base: Color, seed_i: int, ink_w: float = 1.3, hatch: float = 1.0, shade_k: float = -1.0) -> void:
	var pts: PackedVector2Array = pts_in
	if pts.size() < 3:
		return
	if not _valid(pts):
		pts = Geometry2D.convex_hull(pts)
		if not _valid(pts):
			return
	var c: Vector2 = _centroid(pts)
	var bb: Rect2 = Rect2(pts[0], Vector2.ZERO)
	for p: Vector2 in pts:
		bb = bb.expand(p)
	var m: float = minf(bb.size.x, bb.size.y)
	var big: float = maxf(bb.size.x, bb.size.y)
	draw_colored_polygon(pts, base)
	var rg: RandomNumberGenerator = RandomNumberGenerator.new()
	rg.seed = seed_i
	var pool: PackedVector2Array = _scaled(pts, c, 0.82, -LIGHT * m * 0.10)
	if _valid(pool):
		draw_colored_polygon(pool, Color(base.darkened(0.20), 0.30))
	var pool2: PackedVector2Array = _scaled(pts, c, 0.52, -LIGHT * m * 0.18)
	if _valid(pool2):
		draw_colored_polygon(pool2, Color(base.darkened(0.34), 0.22))
	var lit: PackedVector2Array = _scaled(pts, c, 0.46, LIGHT * m * 0.22)
	if _valid(lit):
		draw_colored_polygon(lit, Color(PAPER.lightened(0.12), 0.26))
	var tint: Color = base.lerp(Color(0.78, 0.60, 0.26), 0.35) if rg.randf() < 0.5 else base.lerp(Color(0.25, 0.38, 0.30), 0.30)
	var tb: PackedVector2Array = _scaled(pts, c, rg.randf_range(0.38, 0.6), Vector2(rg.randf_range(-0.2, 0.2), rg.randf_range(-0.2, 0.2)) * m)
	if _valid(tb):
		draw_colored_polygon(tb, Color(tint, 0.20))
	if hatch > 0.05 and big > 16.0:
		var k: float = shade_k if shade_k > 0.0 else m * 0.30
		var regs: Array = Geometry2D.clip_polygons(pts, _scaled(pts, c, 1.0, LIGHT * k))
		var gap: float = clampf(4.4 - hatch * 1.4, 2.2, 5.0)
		_hatch(regs, 0.95 + 0.08 * rg.randf(), gap, Color(SEPIA, 0.50 * minf(hatch, 1.2)), 0.8, seed_i)
		if hatch > 0.85:
			var regs2: Array = Geometry2D.clip_polygons(pts, _scaled(pts, c, 1.0, LIGHT * k * 0.45))
			_hatch(regs2, -0.55, gap * 1.15, Color(SEPIA, 0.34), 0.7, seed_i + 7)
	if ink_w > 0.0:
		_ink(pts, ink_w, SEPIA, seed_i, true, c)


## Open path as a tapered ribbon polygon (twigs, stalks, limbs).
func _ribbon(path: PackedVector2Array, w0: float, w1: float) -> PackedVector2Array:
	var left: PackedVector2Array = PackedVector2Array()
	var right: PackedVector2Array = PackedVector2Array()
	var n: int = path.size()
	for i: int in n:
		var d: Vector2 = (path[mini(i + 1, n - 1)] - path[maxi(i - 1, 0)]).normalized()
		var nr: Vector2 = Vector2(-d.y, d.x)
		var w: float = lerpf(w0, w1, float(i) / float(maxi(n - 1, 1))) * 0.5
		left.append(path[i] + nr * w)
		right.append(path[i] - nr * w)
	right.reverse()
	left.append_array(right)
	return left


func _bez(p0: Vector2, c1: Vector2, c2: Vector2, p1: Vector2, n: int = 28) -> PackedVector2Array:
	var pts: PackedVector2Array = PackedVector2Array()
	for i: int in n + 1:
		var t: float = float(i) / float(n)
		var u: float = 1.0 - t
		pts.append(p0 * u * u * u + c1 * 3.0 * u * u * t + c2 * 3.0 * u * t * t + p1 * t * t * t)
	return pts


## A thin stalk: dark sepia hairline with a coloured core.
func _stalk(path: PackedVector2Array, w: float, core: Color) -> void:
	_ink(path, w + 1.6, SEPIA, 3)
	draw_polyline(path, core, w * 0.7, true)


## Small italic label with a very thin leader line to `anchor`. side "r": text starts at tpos.x;
## side "l": text ends at tpos.x.
func _label(text: String, tpos: Vector2, anchor: Vector2, side: String = "r") -> void:
	var f: Font = pf(400, true)
	var lines: PackedStringArray = text.split("\n")
	var lh: float = float(LBL) * 1.12
	for i: int in lines.size():
		var lx: float = tpos.x - _text_w(lines[i], LBL, f) if side == "l" else tpos.x
		_txt(lines[i], Vector2(lx, tpos.y + float(i) * lh), LBL, SEPIA, f)
	var mid_y: float = tpos.y - float(LBL) * 0.30 + float(lines.size() - 1) * lh * 0.5
	var start: Vector2 = Vector2(tpos.x + 4.0, mid_y) if side == "l" else Vector2(tpos.x - 4.0, mid_y)
	draw_line(start, anchor, Color(SEPIA, 0.85), 0.8, true)
	draw_circle(anchor, 1.6, SEPIA)


## Centred element caption: italic name and two short lines.
func _caption(cx: float, y: float, name_s: String, lines: Array) -> void:
	_ctr(name_s, Vector2(cx, y), 25, SEPIA, pf(400, true))
	for i: int in lines.size():
		_ctr(lines[i], Vector2(cx, y + 23.0 + float(i) * 19.0), 16, SEPIA_MID, pf(400, true))


# ----------------------------------------------- main draw -------------------------------------

func _texts() -> Dictionary:
	if species == 1:
		return {
			"twig": ["Feuilles et fruits", ["feuilles dentées, deux glandes rouges sous le limbe ;", "merises groupées sur de longues queues"]],
			"flower": ["Les fleurs", ["ombelles de 2 à 6 fleurs blanches à cinq pétales,", "ouvertes avant ou avec les feuilles"]],
			"habit": ["Le port en hiver", ["houppier ovoïde, tronc élancé", "tout droit jusqu'à la cime"]],
			"trunk": ["Tronc et écorce", ["écorce lisse brun-rouge, lenticelles horizontales,", "se détachant en bandes"]],
		}
	return {
		"twig": ["Feuilles et glands", ["feuilles lobées à pétiole très court ;", "glands portés par un long pédoncule"]],
		"flower": ["Les fleurs", ["chatons mâles pendants, fleurs femelles minuscules,", "épanouis avec les premières feuilles"]],
		"habit": ["Le port en hiver", ["houppier large, charpentières tortueuses,", "tronc court et puissant"]],
		"trunk": ["Tronc et écorce", ["aubier clair, cœur de chêne sombre ;", "écorce épaisse aux crevasses profondes"]],
	}


func _draw() -> void:
	if size.x < 100.0 or size.y < 100.0:
		return
	var avail: Vector2 = Vector2(size.x, size.y - STRIP_H)
	_s = minf(avail.x / PLATE_W, avail.y / PLATE_H)
	_o = Vector2((size.x - PLATE_W * _s) * 0.5, STRIP_H + (avail.y - PLATE_H * _s) * 0.5)
	_text_mode()
	draw_rect(Rect2(Vector2.ZERO, size), UiStyle.WALL)
	for k: int in 14:
		draw_rect(Rect2(Vector2.ZERO, size).grow(-float(k) * size.x * 0.012), Color(0, 0, 0, 0.05), false, size.x * 0.012)
	_shapes()
	var d: Dictionary = species_data(_ts.find_species(PLATE_IDS[species]))
	_paper()
	_frame()
	var t: Dictionary = _texts()
	_draw_twig(Rect2(80, 122, 420, 230))
	_draw_flowers(Rect2(80, 395, 420, 210))
	_draw_habit(Vector2(750.0, 588.0))
	_draw_trunk_group()
	_draw_facts(d, Rect2(1030, 440, 400, 240))
	_caption(290.0, 378.0, t["twig"][0], t["twig"][1])
	_caption(290.0, 632.0, t["flower"][0], t["flower"][1])
	_caption(750.0, 632.0, t["habit"][0], t["habit"][1])
	_caption(1230.0, 378.0, t["trunk"][0], t["trunk"][1])
	_title(d)


func _paper() -> void:
	for k: int in 5:
		draw_rect(Rect2(20 + k * 2, 12 + k * 2, 1460, 856), Color(0, 0, 0, 0.10))
	var sheet: Rect2 = Rect2(24, 14, 1452, 852)
	draw_rect(sheet, PAPER)
	var r: RandomNumberGenerator = RandomNumberGenerator.new()
	r.seed = 11
	# uneven tone: big soft blotches, denser toward the edges
	for k: int in 70:
		var edge: float = pow(r.randf(), 0.6)
		var ang: float = r.randf() * TAU
		var p: Vector2 = Vector2(750, 440) + Vector2(cos(ang) * 740.0, sin(ang) * 430.0) * (1.0 - edge * 0.55)
		var rad: float = r.randf_range(50.0, 170.0)
		var tone: Color = Color(0.62, 0.46, 0.22) if r.randf() < 0.7 else Color(1.0, 0.97, 0.88)
		for j: int in 5:
			draw_circle(p, rad * (1.0 - 0.17 * float(j)), Color(tone, 0.012 + 0.0015 * float(5 - j)))
	for k: int in 14:
		var a: float = 0.07 * float(14 - k) / 14.0
		draw_rect(sheet.grow(-float(k) * 3.5), Color(0.50, 0.34, 0.14, a), false, 3.6)
	for c: Vector2 in [Vector2(24, 14), Vector2(1476, 14), Vector2(24, 866), Vector2(1476, 866)]:
		for j: int in 8:
			draw_circle(c, 150.0 - 16.0 * float(j), Color(0.52, 0.34, 0.12, 0.03))
	# foxing: small rusty spots with pale halos, clustered near the margins
	for k: int in 120:
		var p2: Vector2 = Vector2(r.randf_range(34, 1466), r.randf_range(22, 858))
		if r.randf() < 0.65:
			p2.x = 34.0 + r.randf() * 180.0 if r.randf() < 0.5 else 1466.0 - r.randf() * 180.0
		var rr: float = r.randf_range(0.8, 4.2) * (1.0 if r.randf() < 0.9 else 2.6)
		draw_circle(p2, rr * 2.0, Color(0.70, 0.52, 0.26, 0.04))
		draw_circle(p2, rr, Color(0.58, 0.34, 0.12, r.randf_range(0.07, 0.22)))
	# fibres
	for k: int in 260:
		var p3: Vector2 = Vector2(r.randf_range(30, 1470), r.randf_range(18, 862))
		var a2: float = r.randf() * TAU
		var l: float = r.randf_range(6.0, 24.0)
		var mid: Vector2 = p3 + Vector2.from_angle(a2 + 0.5) * l * 0.5
		var fc: Color = Color(0.45, 0.33, 0.18, 0.10) if r.randf() < 0.7 else Color(1.0, 0.98, 0.9, 0.18)
		draw_polyline(PackedVector2Array([p3, mid, p3 + Vector2.from_angle(a2) * l]), fc, 0.7, true)
	draw_line(Vector2(24, 440), Vector2(1476, 443), Color(0.45, 0.32, 0.16, 0.09), 2.5)
	draw_line(Vector2(24, 442), Vector2(1476, 445), Color(1.0, 0.98, 0.90, 0.12), 1.5)


func _rect_pts(r: Rect2) -> PackedVector2Array:
	var pts: PackedVector2Array = PackedVector2Array()
	var steps: int = 28
	for i: int in steps:
		pts.append(r.position + Vector2(r.size.x * float(i) / float(steps), 0))
	for i: int in steps:
		pts.append(Vector2(r.end.x, r.position.y + r.size.y * float(i) / float(steps)))
	for i: int in steps:
		pts.append(r.end - Vector2(r.size.x * float(i) / float(steps), 0))
	for i: int in steps:
		pts.append(Vector2(r.position.x, r.end.y - r.size.y * float(i) / float(steps)))
	return pts


const CART_GAP: float = 236.0


## Closed rectangle path with a gap in the top edge (where the cartouche sits), as an open polyline.
func _rect_open(r: Rect2) -> PackedVector2Array:
	var all: PackedVector2Array = _rect_pts(r)
	var out: PackedVector2Array = PackedVector2Array()
	var n: int = all.size()
	var start: int = 0
	for i: int in n:
		if all[i].y <= r.position.y + 0.1 and all[i].x >= 750.0 + CART_GAP:
			start = i
			break
	for i: int in n:
		var p: Vector2 = all[(start + i) % n]
		if p.y <= r.position.y + 0.1 and absf(p.x - 750.0) <= CART_GAP:
			if out.size() > 0:
				break
			continue
		out.append(p)
	return out


func _frame() -> void:
	var f: Rect2 = Rect2(46, 36, 1408, 808)
	_ink(_rect_open(f), 2.6, SEPIA, 1)
	_ink(_rect_open(f.grow(-6.0)), 0.9, SEPIA, 2)
	_ink(_rect_open(f.grow(-9.5)), 0.8, SEPIA_MID, 3)
	var o: Rect2 = f.grow(-9.5)
	var inn: Rect2 = f.grow(-15.0)
	var step: float = 4.2
	var x: float = o.position.x
	while x < o.end.x:
		if absf(x - 750.0) > CART_GAP:
			draw_line(Vector2(x, o.position.y), Vector2(x + 2.5, inn.position.y), Color(SEPIA, 0.55), 0.6, true)
		draw_line(Vector2(x, o.end.y), Vector2(x + 2.5, inn.end.y), Color(SEPIA, 0.55), 0.6, true)
		x += step
	var y: float = o.position.y
	while y < o.end.y:
		draw_line(Vector2(o.position.x, y), Vector2(inn.position.x, y + 2.5), Color(SEPIA, 0.55), 0.6, true)
		draw_line(Vector2(o.end.x, y), Vector2(inn.end.x, y + 2.5), Color(SEPIA, 0.55), 0.6, true)
		y += step
	_ink(_rect_open(inn), 1.6, SEPIA, 4)
	_ink(_rect_open(f.grow(-18.5)), 0.7, SEPIA_MID, 5)
	# notched ornaments at the lower corners, studs at the upper ones
	for c: Vector2 in [Vector2(f.position.x, f.end.y), Vector2(f.end.x, f.end.y)]:
		var sx: float = 1.0 if c.x < 700.0 else -1.0
		var q: Vector2 = c + Vector2(sx * 8.0, -8.0)
		var pts: PackedVector2Array = PackedVector2Array([
			q + Vector2(-sx * 6.0, 6.0), q + Vector2(sx * 22.0, 6.0), q + Vector2(sx * 22.0, -4.0), q + Vector2(sx * 12.0, -4.0),
			q + Vector2(sx * 12.0, -22.0), q + Vector2(sx * 2.0, -22.0), q + Vector2(sx * 2.0, -12.0), q + Vector2(-sx * 6.0, -12.0)])
		draw_colored_polygon(pts, PAPER)
		_ink(pts, 1.3, SEPIA, 9, true)
		draw_circle(q + Vector2(sx * 11.0, -6.0), 2.2, SEPIA)
		draw_arc(q + Vector2(sx * 11.0, -6.0), 5.0, 0.0, TAU, 18, Color(SEPIA, 0.7), 0.7, true)
	for c2: Vector2 in [Vector2(f.position.x, f.position.y), Vector2(f.end.x, f.position.y)]:
		var sgx: float = 1.0 if c2.x < 700.0 else -1.0
		var q2: Vector2 = c2 + Vector2(sgx * 12.0, 12.0)
		draw_circle(q2, 5.0, PAPER)
		draw_arc(q2, 5.0, 0.0, TAU, 18, SEPIA, 1.1, true)
		draw_circle(q2, 1.8, SEPIA)
	_cartouche(Vector2(750.0, 36.0))


func _spiral(c: Vector2, r0: float, turns: float, a0: float, dirn: float, n: int = 34) -> PackedVector2Array:
	var pts: PackedVector2Array = PackedVector2Array()
	for i: int in n + 1:
		var t: float = float(i) / float(n)
		var a: float = a0 + dirn * t * turns * TAU
		pts.append(c + Vector2.from_angle(a) * r0 * (1.0 - 0.88 * t))
	return pts


## Baroque cartouche centred on the top edge: a shield with scrolls and acanthus leaflets either side.
func _cartouche(c0: Vector2) -> void:
	var k: float = 1.2
	draw_set_transform(_o + c0 * _s * (1.0 - k), 0.0, Vector2(_s * k, _s * k))
	var c: Vector2 = c0
	for sx: float in [-1.0, 1.0]:
		var vol: Vector2 = c + Vector2(sx * 118.0, 24.0)
		var sp: PackedVector2Array = _spiral(vol, 24.0, 1.35, -0.4 if sx > 0.0 else PI + 0.4, 1.0 if sx > 0.0 else -1.0)
		_ink(sp, 1.7, SEPIA, 21)
		var tail: PackedVector2Array = _bez(c + Vector2(sx * 28.0, 52.0), c + Vector2(sx * 62.0, 62.0), c + Vector2(sx * 78.0, 40.0), vol + Vector2(sx * 4.0, 24.0))
		_ink(tail, 1.5, SEPIA, 22)
		var tail2: PackedVector2Array = _bez(c + Vector2(sx * 24.0, 44.0), c + Vector2(sx * 56.0, 52.0), c + Vector2(sx * 72.0, 32.0), vol + Vector2(sx * 1.0, 18.0))
		_ink(tail2, 0.8, SEPIA_MID, 23)
		for i: int in 5:
			var t: float = 0.12 + 0.17 * float(i)
			var p: Vector2 = tail[int(t * float(tail.size() - 1))]
			var lf: PackedVector2Array = _ellipse(p + Vector2(sx * 7.0, -9.0), 11.0 - float(i), 3.6, -sx * (0.9 - 0.15 * float(i)), 14)
			draw_colored_polygon(lf, PAPER)
			_ink(lf, 0.9, SEPIA, 30 + i, true)
		var fl: PackedVector2Array = _bez(vol + Vector2(sx * 24.0, -2.0), vol + Vector2(sx * 48.0, -16.0), vol + Vector2(sx * 60.0, 6.0), c + Vector2(sx * 188.0, -4.0))
		_ink(fl, 1.2, SEPIA, 40)
		var vs: PackedVector2Array = _spiral(c + Vector2(sx * 150.0, 8.0), 11.0, 1.1, 0.0 if sx > 0.0 else PI, 1.0 if sx > 0.0 else -1.0, 20)
		_ink(vs, 1.1, SEPIA, 41)
	var sh: PackedVector2Array = PackedVector2Array([
		c + Vector2(-30, 14), c + Vector2(-18, 8), c + Vector2(0, 12), c + Vector2(18, 8), c + Vector2(30, 14),
		c + Vector2(28, 40), c + Vector2(12, 58), c + Vector2(0, 64), c + Vector2(-12, 58), c + Vector2(-28, 40)])
	draw_colored_polygon(sh, PAPER.lightened(0.04))
	_ink(sh, 1.5, SEPIA, 50, true)
	var inner: PackedVector2Array = _scaled(sh, c + Vector2(0, 38), 0.82)
	_ink(inner, 0.7, SEPIA_MID, 51, true)
	_hatch([inner], 0.8, 3.6, Color(SEPIA, 0.35), 0.6, 52)
	for sx2: float in [-1.0, 1.0]:
		var lf2: PackedVector2Array = _ellipse(c + Vector2(sx2 * 12.0, 6.0), 14.0, 4.6, -sx2 * 0.55, 16)
		draw_colored_polygon(lf2, PAPER)
		_ink(lf2, 1.0, SEPIA, 60, true)
	draw_circle(c + Vector2(0, 8.0), 2.6, SEPIA)
	for i: int in 7:
		var a: float = PI * (0.18 + 0.64 * float(i) / 6.0)
		draw_line(c + Vector2(0, 66.0), c + Vector2(0, 66.0) + Vector2(cos(a), sin(a)) * 11.0, Color(SEPIA, 0.8), 0.8, true)
	_shapes()


func _title(d: Dictionary) -> void:
	_ctr(d["latin"], Vector2(750, 748), 58, SEPIA, pf(400, true, 1))
	_ctr("(%s)" % d["title"], Vector2(750, 773), 22, SEPIA_MID, pf(400, true))
	draw_line(Vector2(560, 696), Vector2(720, 696), Color(SEPIA, 0.8), 0.8, true)
	draw_line(Vector2(780, 696), Vector2(940, 696), Color(SEPIA, 0.8), 0.8, true)
	draw_colored_polygon(PackedVector2Array([Vector2(750, 690), Vector2(756, 696), Vector2(750, 702), Vector2(744, 696)]), SEPIA)
	_text_mode()
	var fs: int = maxi(8, roundi(16.0 * _s))
	draw_multiline_string(pf(400, true), _o + Vector2(190, 797) * _s, d["caption"], HORIZONTAL_ALIGNMENT_CENTER, 1120.0 * _s, fs, 2, SEPIA_MID)
	_shapes()


# ----------------------------------------------- leaf ------------------------------------------

## Half-width profile of the blade at parameter t (0 base .. 1 tip), as a fraction of the half-width.
func _leaf_hw(t: float, lobes_per_side: int, serr: float, flip: float) -> float:
	var b: float
	if lobes_per_side > 0:
		b = pow(sin(PI * pow(t, 0.70)), 0.85)  # obovate: widest beyond the middle
		if t > 0.12 and t < 0.97:
			var u: float = (t - 0.12) / 0.85
			var k: float = 0.5 - 0.5 * cos(TAU * (float(lobes_per_side) * u + 0.22 * flip))
			var m: float = lerpf(0.36, 1.0, pow(k, 0.7))
			b *= m
		b += 0.15 * exp(-pow((t - 0.035) / 0.035, 2.0))  # basal auricles
	else:
		b = pow(sin(PI * pow(t, 0.78)), 0.80)  # obovate-oblong
		b *= 1.0 - 0.45 * smoothstep(0.78, 1.0, t)  # acuminate tip
		if serr > 0.0 and t > 0.06 and t < 0.97:
			var saw: float = fposmod(t * 30.0, 1.0)
			b *= 1.0 + 0.12 * serr * (saw - 0.5)
			b += 0.085 * serr * saw * sin(PI * t)
	return maxf(b, 0.0)


func _leaf_outline(length: float, half_w: float, lobes_ps: int, serr: float) -> PackedVector2Array:
	var pts: PackedVector2Array = PackedVector2Array()
	var n: int = 360
	for i: int in n + 1:
		var t: float = float(i) / float(n)
		pts.append(Vector2(t * length, _leaf_hw(t, lobes_ps, serr, 0.0) * half_w))
	for i: int in range(n, -1, -1):
		var t: float = float(i) / float(n)
		pts.append(Vector2(t * length, -_leaf_hw(t, lobes_ps, serr, 1.0) * half_w))
	return pts


func _leaf_at(node: Vector2, rot: float, length: float, pet: float, base: Color, seed_i: int, red_glands: bool = false) -> Vector2:
	var ll: float = _ts.get_leaf_length_m()
	var lw: float = _ts.get_leaf_width_m()
	var ratio: float = (lw / ll) if ll > 0.001 else 0.55
	var lobes_ps: int = maxi(_ts.get_leaf_lobes() / 2, 0)
	var serr: float = _ts.get_leaf_serration()
	var dir: Vector2 = Vector2.from_angle(rot)
	var origin: Vector2 = node + dir * pet
	var xf: Transform2D = Transform2D(rot, origin)
	if pet > 1.0:
		_stalk(PackedVector2Array([node, origin]), 2.4, Color(0.58, 0.58, 0.28))
	var raw: PackedVector2Array = _leaf_outline(length, length * ratio * 0.5, lobes_ps, serr)
	var pts: PackedVector2Array = PackedVector2Array()
	for i: int in raw.size():
		if i > 0 and raw[i].distance_to(raw[i - 1]) < 0.05:
			continue
		pts.append(xf * raw[i])
	_wash(pts, base, seed_i, 1.2, 1.0)
	var hw: float = length * ratio * 0.5
	var vein: Color = Color(base.lightened(0.55), 0.85)
	draw_line(xf * Vector2(0, 0), xf * Vector2(length * 0.96, 0), vein, 2.0, true)
	draw_line(xf * Vector2(0, 0.6), xf * Vector2(length * 0.96, 0.6), Color(SEPIA, 0.55), 0.6, true)
	if lobes_ps > 0:
		for k: int in lobes_ps:
			var u: float = (float(k) + 0.5) / float(lobes_ps)
			var tc: float = 0.12 + 0.85 * u
			var tip_t: float = minf(tc + 0.02, 0.97)
			var from: Vector2 = Vector2((tc - 0.07) * length, 0)
			for sg: float in [1.0, -1.0]:
				var hh: float = _leaf_hw(tc, lobes_ps, serr, 0.0 if sg > 0.0 else 1.0) * hw
				draw_line(xf * from, xf * Vector2(tip_t * length, sg * hh * 0.86), vein, 1.1, true)
				draw_line(xf * from, xf * Vector2(tip_t * length, sg * hh * 0.86), Color(SEPIA, 0.35), 0.5, true)
	else:
		var tv: float = 0.10
		while tv < 0.86:
			var te: float = tv + 0.13
			var hw2: float = _leaf_hw(te, 0, serr, 0.0) * hw
			for sg2: float in [1.0, -1.0]:
				var mid: Vector2 = Vector2((tv + 0.06) * length, sg2 * hw2 * 0.30)
				var ve: PackedVector2Array = PackedVector2Array([xf * Vector2(tv * length, 0), xf * mid, xf * Vector2(te * length, sg2 * hw2 * 0.88)])
				draw_polyline(ve, vein, 1.0, true)
				draw_polyline(ve, Color(SEPIA, 0.32), 0.5, true)
			tv += 0.085
	if red_glands:
		for sg3: float in [-1.0, 1.0]:
			var gp: Vector2 = xf * Vector2(-1.0, sg3 * 4.0)
			_wash(_ellipse(gp, 3.4, 2.4, rot, 12), Color(0.74, 0.14, 0.12), seed_i + 5, 0.8, 0.0)
	return origin


func _twig_path() -> PackedVector2Array:
	return _bez(Vector2(96, 280), Vector2(190, 312), Vector2(300, 196), Vector2(470, 190))


func _bark_twig(path: PackedVector2Array, w0: float, w1: float, seed_i: int) -> void:
	_wash(_ribbon(path, w0, w1), Color(0.48, 0.36, 0.24), seed_i, 1.1, 0.7, 3.0)


func _draw_twig(_box: Rect2) -> void:
	var oak: bool = species == 0
	var path: PackedVector2Array = _twig_path()
	_bark_twig(path, 7.5, 3.4, 201)
	var gcol: Color = Color(0.38, 0.54, 0.20) if oak else Color(0.36, 0.55, 0.20)
	var pet: float = 4.0 if oak else 20.0
	var nodes: Array = [[0.17, -1.38, 140.0], [0.50, -2.0, 142.0], [0.82, -0.45, 128.0]]
	var bases: Array[Vector2] = []
	for i: int in nodes.size():
		var nd: Array = nodes[i]
		var node: Vector2 = path[int(float(nd[0]) * float(path.size() - 1))]
		bases.append(_leaf_at(node, nd[1], nd[2], pet, gcol.lerp(Color(0.62, 0.68, 0.30), 0.12 * float(i)), 210 + i * 11, not oak and i == 0))
	# terminal bud
	_wash(_ellipse(path[path.size() - 1] + Vector2(7, 0), 8.0, 3.2, 0.0, 14), Color(0.52, 0.40, 0.22), 230, 0.9, 0.0)
	if oak:
		var t0: Vector2 = path[int(0.60 * float(path.size() - 1))]
		var ped: PackedVector2Array = _bez(t0, t0 + Vector2(2, 14), t0 + Vector2(-4, 18), t0 + Vector2(2, 28), 12)
		_stalk(ped, 2.6, Color(0.56, 0.58, 0.26))
		var e: Vector2 = ped[ped.size() - 1]
		_acorn(e + Vector2(-20, 2), 16.0, 56.0, 0.22, 240)
		_acorn(e + Vector2(22, 1), 15.5, 53.0, -0.24, 241)
		_acorn(e + Vector2(0, 6), 17.0, 60.0, 0.0, 242)
		_label("cupule\nécailleuse", Vector2(360, 322), e + Vector2(30, 8), "r")
	else:
		var spur: Vector2 = path[int(0.46 * float(path.size() - 1))]
		_wash(_ellipse(spur, 5.0, 4.0, 0.0, 12), Color(0.44, 0.30, 0.20), 250, 0.9, 0.0)
		var ends: Array[Vector2] = [Vector2(-34, 50), Vector2(6, 60), Vector2(44, 46)]
		for j: int in ends.size():
			var en: Vector2 = ends[j]
			var sp: PackedVector2Array = _bez(spur, spur + Vector2(en.x * 0.1, en.y * 0.5), spur + Vector2(en.x * 0.9, en.y * 0.6), spur + en, 14)
			_stalk(sp, 2.2, Color(0.56, 0.52, 0.26))
			_cherry(spur + en + Vector2(0, 15), 15.0, 260 + j)
		_label("2 glandes\nrouges", Vector2(92, 322), bases[0] + Vector2(2, 4), "r")


func _acorn(top: Vector2, w: float, h: float, ang: float, seed_i: int) -> void:
	var xf: Transform2D = Transform2D(ang, top)
	var body: PackedVector2Array = PackedVector2Array()
	var cy: float = h * 0.40
	var ry: float = h * 0.62
	for k: int in 40:
		var a: float = TAU * float(k) / 40.0
		var sa: float = maxf(sin(a), 0.0)
		var x: float = w * cos(a) * (1.0 - 0.50 * pow(sa, 3.0))
		var y: float = cy + ry * sin(a) + 0.16 * ry * pow(sa, 5.0)
		body.append(xf * Vector2(x, y))
	_wash(body, Color(0.74, 0.56, 0.24), seed_i, 1.1, 1.1)
	for k: int in 4:
		var x2: float = lerpf(-w * 0.55, w * 0.55, float(k) / 3.0)
		draw_line(xf * Vector2(x2, h * 0.56), xf * Vector2(x2 * 0.18, h * 1.0), Color(0.45, 0.30, 0.12, 0.55), 0.7, true)
	var cup: PackedVector2Array = PackedVector2Array()
	for k: int in 21:
		var a2: float = PI * float(k) / 20.0
		cup.append(xf * Vector2(cos(a2) * w * 1.10, sin(a2) * h * 0.40 + 0.02 * h))
	cup.append(xf * Vector2(w * 1.04, -h * 0.02))
	cup.append(xf * Vector2(0, -h * 0.07))
	cup.append(xf * Vector2(-w * 1.04, -h * 0.02))
	_wash(cup, Color(0.50, 0.42, 0.26), seed_i + 1, 1.2, 1.2)
	for row: int in 3:
		var yy: float = h * (0.08 + 0.10 * float(row))
		var span: float = w * 0.95 * sqrt(maxf(1.0 - pow(yy / (h * 0.42), 2.0), 0.05))
		var nn: int = 5 - row
		for k: int in nn:
			var xx: float = lerpf(-span * 0.8, span * 0.8, (float(k) + (0.5 if row % 2 == 1 else 0.0)) / float(maxi(nn - 1, 1)))
			draw_arc(xf * Vector2(xx, yy), 2.6, 0.2, PI - 0.2, 6, Color(SEPIA, 0.85), 0.8, true)


func _cherry(c: Vector2, r: float, seed_i: int) -> void:
	var pts: PackedVector2Array = _ellipse(c, r, r * 1.04, 0.0, 30)
	_wash(pts, Color(0.70, 0.12, 0.14), seed_i, 1.2, 1.15)
	# a dimple where the stalk enters, a highlight left as bare paper
	draw_arc(c + Vector2(0, -r * 0.86), r * 0.2, 0.1, PI - 0.1, 8, Color(SEPIA, 0.85), 1.0, true)
	draw_colored_polygon(_ellipse(c + Vector2(-r * 0.38, -r * 0.34), r * 0.20, r * 0.12, -0.7, 12), Color(PAPER.lightened(0.15), 0.85))
	draw_line(c + Vector2(-r * 0.2, r * 0.45), c + Vector2(r * 0.55, -r * 0.1), Color(0.35, 0.03, 0.06, 0.35), 0.8, true)


# ----------------------------------------------- flowers ---------------------------------------

func _blossom(c: Vector2, r: float, rot: float, seed_i: int) -> void:
	for k: int in 5:
		var a: float = rot + TAU * float(k) / 5.0
		var pc: Vector2 = c + Vector2(cos(a), sin(a)) * r * 0.58
		_wash(_ellipse(pc, r * 0.54, r * 0.46, a, 20), Color(0.97, 0.90, 0.90), seed_i + k, 0.9, 0.5, r * 0.18)
		draw_colored_polygon(_ellipse(c + Vector2(cos(a), sin(a)) * r * 0.30, r * 0.20, r * 0.10, a, 10), Color(0.93, 0.62, 0.68, 0.30))
	draw_circle(c, r * 0.20, Color(0.86, 0.74, 0.26))
	for k: int in 14:
		var a2: float = TAU * float(k) / 14.0 + 0.2
		var e: Vector2 = c + Vector2(cos(a2), sin(a2)) * r * (0.46 + 0.08 * _h(float(k)))
		draw_line(c, e, Color(SEPIA, 0.7), 0.6, true)
		draw_circle(e, 1.3, Color(0.70, 0.42, 0.14))


func _draw_flowers(_box: Rect2) -> void:
	if species == 1:
		var path: PackedVector2Array = _bez(Vector2(104, 602), Vector2(210, 612), Vector2(330, 566), Vector2(480, 572))
		_bark_twig(path, 7.0, 3.4, 301)
		var spur: Vector2 = path[int(0.46 * float(path.size() - 1))]
		var stalks: Array[Vector2] = [Vector2(-74, -64), Vector2(-26, -96), Vector2(30, -98), Vector2(80, -62)]
		for i: int in stalks.size():
			var sp: PackedVector2Array = _bez(spur, spur + Vector2(stalks[i].x * 0.2, stalks[i].y * 0.6), spur + Vector2(stalks[i].x * 0.8, stalks[i].y * 0.8), spur + stalks[i], 14)
			_stalk(sp, 2.2, Color(0.56, 0.58, 0.28))
		for i: int in stalks.size():
			var c: Vector2 = spur + stalks[i]
			_wash(_ellipse(c + Vector2(0, 9), 6.0, 9.0, 0.0, 12), Color(0.58, 0.30, 0.22), 310 + i, 0.8, 0.3)
			_blossom(c, 28.0, 0.5 + float(i) * 1.3, 320 + i * 9)
		_leaf_at(path[int(0.12 * float(path.size() - 1))], -1.1, 62.0, 0.0, Color(0.50, 0.58, 0.24), 340)
		_leaf_at(path[int(0.80 * float(path.size() - 1))], -0.45, 66.0, 0.0, Color(0.52, 0.60, 0.26), 341)
	else:
		var path2: PackedVector2Array = _bez(Vector2(104, 510), Vector2(210, 530), Vector2(330, 490), Vector2(480, 498))
		_bark_twig(path2, 7.0, 3.4, 301)
		_leaf_at(path2[int(0.10 * float(path2.size() - 1))], -0.6, 78.0, 3.0, Color(0.50, 0.62, 0.24), 340)
		_leaf_at(path2[int(0.90 * float(path2.size() - 1))], -0.2, 72.0, 3.0, Color(0.52, 0.64, 0.26), 341)
		var cat_t: Array[float] = [0.26, 0.48, 0.70]
		for i: int in cat_t.size():
			var a: Vector2 = path2[int(cat_t[i] * float(path2.size() - 1))]
			var len_c: float = 86.0 - 8.0 * float(i)
			var spine: PackedVector2Array = _bez(a, a + Vector2(3, len_c * 0.3), a + Vector2(-5 + 6.0 * float(i), len_c * 0.7), a + Vector2(2.0 - 3.0 * float(i), len_c), 20)
			_ink(spine, 1.8, SEPIA, 400 + i)
			for k: int in 15:
				var t: float = float(k) / 14.0
				var bp: Vector2 = spine[int(t * 19.0)]
				for sg: float in [-1.0, 1.0]:
					var bead: PackedVector2Array = _ellipse(bp + Vector2(sg * 3.6, 0), 4.4, 3.2, sg * 0.4, 8)
					_wash(bead, Color(0.76 - 0.04 * float(k % 2), 0.74, 0.32), 420 + k * 3 + i, 0.6, 0.0)
		# a tiny female flower with red stigmas on a short stalk
		var fp: Vector2 = path2[int(0.96 * float(path2.size() - 1))]
		_stalk(PackedVector2Array([fp, fp + Vector2(2, -22)]), 2.0, Color(0.52, 0.58, 0.24))
		_wash(_ellipse(fp + Vector2(2, -26), 5.0, 5.4, 0.0, 12), Color(0.55, 0.62, 0.26), 440, 0.9, 0.3)
		for sx: float in [-5.0, 0.0, 5.0]:
			draw_line(fp + Vector2(2, -30), fp + Vector2(2 + sx * 1.3, -41), Color(0.74, 0.12, 0.10), 1.5, true)


# ----------------------------------------------- winter habit ----------------------------------

var _env_cx: float = 750.0
var _env_gy: float = 588.0
var _env_h: float = 360.0


func _crown_halfwidth(f: float) -> float:
	# ovoid (cherry) half-width as a fraction f of the height
	return 140.0 * pow(maxf(sin(PI * pow(clampf(f, 0.0, 1.0), 0.8)), 0.0), 0.9)


func _inside(q: Vector2) -> bool:
	if species == 0:
		var e: Vector2 = q - Vector2(_env_cx, _env_gy - _env_h * 0.625)
		return pow(e.x / 232.0, 2.0) + pow(e.y / (_env_h * 0.375), 2.0) <= 1.0
	var f: float = (_env_gy - q.y) / _env_h
	return f <= 1.0 and absf(q.x - _env_cx) <= _crown_halfwidth(f) * 1.08 + 3.0


func _limb(rg: RandomNumberGenerator, p: Vector2, ang: float, length: float, w: float, depth: int, oak: bool) -> void:
	var segs: int = maxi(3, int(length / 12.0))
	var pts: PackedVector2Array = PackedVector2Array([p])
	var a: float = ang
	var q: Vector2 = p
	var wob: float = 0.34 if oak else 0.12
	var ph: float = rg.randf() * TAU
	for i: int in segs:
		a += sin(float(i) * 0.8 + ph) * wob * 0.40 + (rg.randf() - 0.5) * wob * 0.25
		if not oak:
			a = lerpf(a, 0.0, 0.05)
		q += Vector2(sin(a), -cos(a)) * (length / float(segs))
		if not _inside(q):
			break
		pts.append(q)
	if pts.size() < 2:
		return
	var w1: float = w * 0.62
	if w >= 2.4:
		var rib: PackedVector2Array = _ribbon(pts, w, w1)
		if _valid(rib):
			draw_colored_polygon(rib, Color(0.36, 0.27, 0.19))
		_ink(rib, 0.8, SEPIA, depth, true)
	else:
		draw_polyline(pts, Color(SEPIA, 0.92), maxf(w, 0.75), true)
	if depth <= 0:
		return
	var nk: int = (2 if rg.randf() < 0.55 else 3) if oak else (2 if rg.randf() < 0.7 else 3)
	for k: int in nk:
		var idx: int = clampi(int(float(pts.size() - 1) * rg.randf_range(0.40, 1.0)), 1, pts.size() - 1)
		var side: float = 1.0 if k % 2 == 0 else -1.0
		var d: Vector2 = pts[idx] - pts[idx - 1]
		var la: float = atan2(d.x, -d.y)
		var da: float = side * rg.randf_range(0.35, 0.85 if oak else 0.65)
		var na: float = la + da
		if not oak:
			na = lerpf(na, 0.0, 0.30)
		_limb(rg, pts[idx], na, length * rg.randf_range(0.50, 0.70), w1 * 0.9, depth - 1, oak)


func _draw_habit(base: Vector2) -> void:
	var oak: bool = species == 0
	var cx: float = base.x
	var gy: float = base.y
	_env_cx = cx
	_env_gy = gy
	_env_h = 360.0 if oak else 455.0
	# ground: pale olive wash that fades out, with fine strokes
	for k: int in 4:
		var gw: float = 440.0 - 70.0 * float(k)
		draw_colored_polygon(_ellipse(Vector2(cx, gy + 5.0), gw * 0.5, 13.0 - 2.0 * float(k), 0.0, 36), Color(0.52, 0.56, 0.30, 0.20))
	var rg: RandomNumberGenerator = RandomNumberGenerator.new()
	rg.seed = 77 + species
	for k: int in 60:
		var gx: float = cx + rg.randf_range(-210.0, 210.0)
		var gyy: float = gy + rg.randf_range(0.0, 14.0)
		var gl: float = rg.randf_range(10.0, 34.0) * (1.0 - absf(gx - cx) / 300.0)
		draw_line(Vector2(gx, gyy), Vector2(gx + gl, gyy + rg.randf_range(-0.6, 0.6)), Color(SEPIA, rg.randf_range(0.30, 0.60)), 0.7, true)
	for k: int in 7:
		var tx: float = cx + rg.randf_range(-200.0, 200.0)
		for b: int in 3:
			draw_line(Vector2(tx, gy + 3.0), Vector2(tx + (float(b) - 1.0) * 4.0, gy - rg.randf_range(5.0, 10.0)), Color(0.32, 0.36, 0.14, 0.9), 0.8, true)
	if oak:
		_draw_trunk_poly(cx, gy, 100.0, 34.0, 24.0, true)
		var top: Vector2 = Vector2(cx, gy - 98.0)
		var angs: Array[float] = [-1.32, -0.98, -0.62, -0.26, 0.10, 0.45, 0.82, 1.15, 1.40]
		for i: int in angs.size():
			var o: Vector2 = top + Vector2(rg.randf_range(-9.0, 9.0), rg.randf_range(-4.0, 22.0))
			_limb(rg, o, angs[i], rg.randf_range(150.0, 200.0), 9.5 - 0.4 * absf(angs[i]) * 2.0, 6, true)
	else:
		# straight leader to the top, with tiers of ascending branches
		var lead: PackedVector2Array = PackedVector2Array()
		for i: int in 40:
			var t: float = float(i) / 39.0
			lead.append(Vector2(cx + sin(t * 5.0) * 2.0 * t, gy - _env_h * t))
		_draw_trunk_poly(cx, gy, 118.0, 17.0, 11.0, false)
		var rib: PackedVector2Array = _ribbon(lead.slice(8), 10.0, 1.6)
		draw_colored_polygon(rib, Color(0.34, 0.25, 0.18))
		_ink(rib, 0.8, SEPIA, 5, true)
		var tiers: Array[float] = [0.24, 0.33, 0.42, 0.51, 0.60, 0.69, 0.78, 0.87]
		for i: int in tiers.size():
			var f: float = tiers[i]
			var hw: float = _crown_halfwidth(f)
			var ang0: float = 1.18 - 0.60 * f
			var p0: Vector2 = Vector2(cx, gy - _env_h * f)
			for sg: float in [-1.0, 1.0]:
				var jit: float = rg.randf_range(-0.22, 0.20)
				var ang: float = sg * (ang0 + jit)
				_limb(rg, p0 + Vector2(0, rg.randf_range(-4.0, 4.0)), ang, hw / sin(absf(ang)) * rg.randf_range(0.78, 1.02), 4.6 - 2.2 * f, 4, false)
			if i % 2 == 0:
				_limb(rg, p0, rg.randf_range(-0.25, 0.25), 70.0 - 40.0 * f, 2.6, 3, false)
		_limb(rg, Vector2(cx, gy - _env_h * 0.985), 0.0, 26.0, 1.6, 2, false)
	# a person for scale
	var m_px: float = _env_h / (30.0 if oak else 21.0)
	var px: float = cx - (205.0 if oak else 165.0)
	var ph: float = 1.7 * m_px
	_wash(_ellipse(Vector2(px, gy - ph * 0.55), ph * 0.15, ph * 0.30, 0.0, 12), Color(0.66, 0.30, 0.24), 91, 0.8, 0.0)
	draw_circle(Vector2(px, gy - ph * 0.92), ph * 0.09, Color(0.88, 0.72, 0.56))
	draw_arc(Vector2(px, gy - ph * 0.92), ph * 0.09, 0.0, TAU, 10, SEPIA, 0.8, true)
	draw_line(Vector2(px - 1.8, gy - ph * 0.26), Vector2(px - 2.2, gy), SEPIA, 1.2, true)
	draw_line(Vector2(px + 1.8, gy - ph * 0.26), Vector2(px + 2.2, gy), SEPIA, 1.2, true)


## Trunk with a flared foot: wash + shadow hatching on the right side.
func _draw_trunk_poly(cx: float, gy: float, h: float, w_base: float, w_top: float, oak: bool) -> void:
	var left: PackedVector2Array = PackedVector2Array()
	var right: PackedVector2Array = PackedVector2Array()
	var n: int = 16
	for i: int in n + 1:
		var t: float = float(i) / float(n)
		var w: float = lerpf(w_base, w_top, pow(t, 0.6)) * 0.5
		var flare: float = (0.55 if oak else 0.35) * exp(-t * 14.0) * w_base
		var wob: float = sin(t * 9.0 + (1.0 if oak else 0.0)) * (1.6 if oak else 0.5)
		var y: float = gy - h * t
		left.append(Vector2(cx - w - flare + wob, y))
		right.append(Vector2(cx + w + flare + wob, y))
	right.reverse()
	left.append_array(right)
	_wash(left, Color(0.45, 0.36, 0.27), 500 + species, 1.4, 1.3, w_base * 0.45)
	if oak:
		for k: int in 5:   # bark fissures
			var x0: float = cx - w_base * 0.35 + float(k) * w_base * 0.17
			draw_line(Vector2(x0, gy - 4.0), Vector2(x0 + sin(float(k)) * 3.0, gy - h * 0.9), Color(SEPIA, 0.55), 0.8, true)


# ----------------------------------------------- trunk section + bark --------------------------

func _draw_trunk_group() -> void:
	var c: Vector2 = Vector2(1165.0, 235.0)
	var r: float = 88.0
	_draw_section(c, r)
	if species == 0:
		_draw_bark_oak(Rect2(1330, 132, 84, 203))
	else:
		_draw_bark_cherry(Rect2(1330, 132, 84, 203))
	var oak: bool = species == 0
	var lx: float = 1060.0
	_label("écorce", Vector2(lx, 172.0), c + Vector2.from_angle(-2.35) * r * 0.95, "l")
	_label("aubier", Vector2(lx, 240.0), c + Vector2.from_angle(PI + 0.12) * r * (0.70 if oak else 0.80), "l")
	_label("duramen", Vector2(lx, 304.0), c + Vector2.from_angle(2.4) * r * 0.34, "l")


func _draw_section(c: Vector2, r: float) -> void:
	var oak: bool = species == 0
	var bark_in: float = 0.80 if oak else 0.91
	var sap_in: float = 0.60 if oak else 0.70
	var cols_bark: Color = Color(0.34, 0.27, 0.22) if oak else Color(0.50, 0.28, 0.20)
	var col_sap: Color = Color(0.91, 0.80, 0.55) if oak else Color(0.93, 0.78, 0.62)
	var col_heart: Color = Color(0.68, 0.46, 0.24) if oak else Color(0.72, 0.44, 0.31)
	var seed_i: int = 100 + species
	var outer: PackedVector2Array = _blob(c, r, seed_i, 0.025, 120, 0.07 if oak else 0.0)
	_wash(outer, cols_bark, seed_i, 1.8, 1.2, r * 0.22)
	if oak:
		for k: int in 22:
			var a: float = TAU * float(k) / 22.0 + 0.1 * sin(float(k) * 2.3)
			draw_line(c + Vector2.from_angle(a) * r * (bark_in + 0.05), c + Vector2.from_angle(a) * r * 0.985, Color(0.06, 0.04, 0.03, 0.85), 1.6, true)
	var wood: PackedVector2Array = _blob(c, r * bark_in, seed_i + 1, 0.03, 120)
	_wash(wood, col_sap, seed_i + 1, 1.0, 0.0)
	var heart: PackedVector2Array = _blob(c + Vector2(3, -2), r * sap_in, seed_i + 2, 0.045, 120)
	_wash(heart, col_heart, seed_i + 2, 0.9, 0.5, r * 0.14)
	var rg: RandomNumberGenerator = RandomNumberGenerator.new()
	rg.seed = seed_i + 3
	var nrings: int = 34 if oak else 24
	var widths: Array[float] = []
	var total: float = 0.0
	for i: int in nrings:
		var w: float = (1.9 * exp(-float(i) / float(nrings) * 1.6) + 0.45) * (0.72 + 0.56 * rg.randf())
		widths.append(w)
		total += w
	var rr: float = 0.0
	var wood_r: float = r * bark_in
	for i: int in nrings:
		rr += widths[i] / total * wood_r
		if rr > wood_r * 0.985:
			break
		var ring: PackedVector2Array = _blob(c + Vector2(3, -2).lerp(Vector2.ZERO, rr / wood_r), rr, seed_i + 10 + i, 0.03, 80)
		_ink(ring, 0.55, Color(0.28, 0.16, 0.07, 0.6), seed_i + i, true)
		if oak and i > 0:
			var prev: float = rr - widths[i] / total * wood_r
			var np: int = int(clampf(prev * 0.55, 8.0, 70.0))
			for k: int in np:
				var a2: float = TAU * float(k) / float(np) + rg.randf() * 0.04
				draw_circle(c + Vector2.from_angle(a2) * (prev + 1.4), 0.7, Color(0.25, 0.14, 0.06, 0.8))
	for k: int in 24:
		var a3: float = TAU * float(k) / 24.0 + 0.07 * sin(float(k) * 1.7)
		draw_line(c + Vector2.from_angle(a3) * r * 0.08, c + Vector2.from_angle(a3) * wood_r * 0.97, Color(0.30, 0.18, 0.08, 0.16), 0.7, true)
	# a radial shake in the heartwood and the pith
	var sa: float = 0.55
	draw_polyline(PackedVector2Array([c + Vector2.from_angle(sa) * r * 0.10, c + Vector2.from_angle(sa + 0.03) * r * 0.32, c + Vector2.from_angle(sa - 0.02) * r * 0.50]), Color(0.18, 0.10, 0.05, 0.9), 1.2, true)
	draw_circle(c + Vector2(3, -2), 2.6, Color(0.22, 0.12, 0.06))
	# shadow hatching over the whole disc, lower right
	var regs: Array = Geometry2D.clip_polygons(outer, _scaled(outer, c, 1.0, LIGHT * r * 0.14))
	_hatch(regs, 0.95, 3.4, Color(SEPIA, 0.45), 0.7, seed_i + 40)


func _bark_patch(box: Rect2, seed_i: int, fill: Color) -> PackedVector2Array:
	var rg: RandomNumberGenerator = RandomNumberGenerator.new()
	rg.seed = seed_i
	var pts: PackedVector2Array = PackedVector2Array()
	var n: int = 12
	for i: int in n:
		pts.append(Vector2(box.position.x + box.size.x * float(i) / float(n), box.position.y + rg.randf_range(-3.0, 3.0)))
	for i: int in n:
		pts.append(Vector2(box.end.x + rg.randf_range(-2.0, 2.0), box.position.y + box.size.y * float(i) / float(n)))
	for i: int in n:
		pts.append(Vector2(box.end.x - box.size.x * float(i) / float(n), box.end.y + rg.randf_range(-3.0, 3.0)))
	for i: int in n:
		pts.append(Vector2(box.position.x + rg.randf_range(-2.0, 2.0), box.end.y - box.size.y * float(i) / float(n)))
	_wash(pts, fill, seed_i, 1.5, 0.9, box.size.x * 0.22)
	return pts


func _draw_bark_oak(box: Rect2) -> void:
	_bark_patch(box, 21, Color(0.60, 0.50, 0.38))
	var rg: RandomNumberGenerator = RandomNumberGenerator.new()
	rg.seed = 22
	var nf: int = 4
	var fis: Array[PackedVector2Array] = []
	for k: int in nf:
		var x0: float = box.position.x + box.size.x * (float(k) + 0.5 + rg.randf_range(-0.15, 0.15)) / float(nf)
		var pts: PackedVector2Array = PackedVector2Array()
		var ph: float = rg.randf() * TAU
		var y: float = box.position.y + 6.0
		while y < box.end.y - 4.0:
			pts.append(Vector2(x0 + sin(y * 0.05 + ph) * 5.0 + sin(y * 0.14 + ph * 2.0) * 2.0, y))
			y += 6.0
		fis.append(pts)
	for k: int in nf:
		var lite: PackedVector2Array = PackedVector2Array()
		var dark: PackedVector2Array = PackedVector2Array()
		for p: Vector2 in fis[k]:
			lite.append(p + Vector2(7.0, 0))
			dark.append(p + Vector2(-5.5, 0))
		draw_polyline(lite, Color(0.76, 0.70, 0.60, 0.9), 3.4, true)
		draw_polyline(dark, Color(0.30, 0.26, 0.22, 0.55), 2.4, true)
		_ink(fis[k], 3.6, Color(0.08, 0.06, 0.05), 60 + k)
	for k: int in 12:
		var kf: int = rg.randi() % (nf - 1)
		var iy: int = 2 + rg.randi() % (fis[kf].size() - 4)
		draw_line(fis[kf][iy] + Vector2(2, 0), fis[kf + 1][iy + 1] - Vector2(2, 0), Color(0.09, 0.07, 0.06, 0.9), 1.5, true)


func _draw_bark_cherry(box: Rect2) -> void:
	_bark_patch(box, 31, Color(0.54, 0.30, 0.21))
	var rg: RandomNumberGenerator = RandomNumberGenerator.new()
	rg.seed = 32
	for k: int in 4:
		var y: float = box.position.y + box.size.y * (0.12 + 0.24 * float(k)) + rg.randf_range(-6, 6)
		var pts: PackedVector2Array = PackedVector2Array()
		for i: int in 12:
			pts.append(Vector2(box.position.x + 3.0 + (box.size.x - 6.0) * float(i) / 11.0, y + sin(float(i) * 0.9 + float(k)) * 2.5))
		_ink(pts, 1.4, Color(0.18, 0.08, 0.05), 70 + k)
		var hi: PackedVector2Array = PackedVector2Array()
		for p: Vector2 in pts:
			hi.append(p + Vector2(0, 2.6))
		draw_polyline(hi, Color(0.80, 0.54, 0.40, 0.55), 1.4, true)
	for row: int in 11:
		var y2: float = box.position.y + 14.0 + float(row) * (box.size.y - 28.0) / 10.0 + rg.randf_range(-3, 3)
		var nn: int = 1 + rg.randi() % 3
		for k: int in nn:
			var x: float = box.position.x + 6.0 + rg.randf() * (box.size.x - 40.0)
			var l: float = rg.randf_range(12.0, 26.0)
			draw_line(Vector2(x, y2), Vector2(x + l, y2 + rg.randf_range(-0.8, 0.8)), Color(0.14, 0.06, 0.04), 4.2, true)
			draw_line(Vector2(x + 0.8, y2), Vector2(x + l - 0.8, y2), Color(0.90, 0.80, 0.64), 2.2, true)
	_label("lenticelles", Vector2(1262.0, 262.0), Vector2(box.position.x + 14.0, box.position.y + 108.0), "r")


# ----------------------------------------------- facts ("en bref") -----------------------------

func _draw_facts(d: Dictionary, box: Rect2) -> void:
	var cx: float = box.get_center().x
	_ctr("En bref", Vector2(cx, box.position.y + 24.0), 25, SEPIA, pf(400, true))
	draw_line(Vector2(box.position.x + 20.0, box.position.y + 36.0), Vector2(box.end.x - 20.0, box.position.y + 36.0), Color(SEPIA, 0.8), 0.9, true)
	var rows: Array = [["Hauteur", d["height"]], ["Longévité", d["longevity"]], ["Floraison", d["flowering"]], ["Fruit", d["fruit"]], ["Sol", d["soil"]]]
	var y: float = box.position.y + 66.0
	var f_l: Font = pf(400, true)
	var f_v: Font = pf(400, false)
	var lab_r: float = box.position.x + 100.0
	var val_x: float = box.position.x + 116.0
	var val_w: float = box.end.x - val_x - 8.0
	for r: Array in rows:
		var val: String = r[1]
		_txt(r[0], Vector2(lab_r - _text_w(r[0], 16, f_l), y), 16, SEPIA_MID, f_l)
		var lines: int = 1
		if _text_w(val, 16, f_v) > val_w:
			lines = 2
			_text_mode()
			draw_multiline_string(f_v, _o + Vector2(val_x, y) * _s, val, HORIZONTAL_ALIGNMENT_LEFT, val_w * _s, maxi(8, roundi(16.0 * _s)), 2, SEPIA)
			_shapes()
		else:
			_txt(val, Vector2(val_x, y), 16, SEPIA, f_v)
		y += 12.0 + 19.0 * float(lines)
		draw_line(Vector2(box.position.x + 20.0, y - 15.0), Vector2(box.end.x - 20.0, y - 15.0), Color(SEPIA, 0.22), 0.7, true)
		y += 9.0
