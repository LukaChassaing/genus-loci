class_name RingView
extends Control
## Trunk cross-section drawn with concentric annual rings (ring-porous oak).
## Width-aware: rings narrower than a few pixels are merged into tone bands instead of aliasing into
## moiré; every 10th ring is emphasised. Mouse wheel zooms, left-drag pans, double-click resets.
## Redraws only on data change, hover change, zoom/pan.

const EARLY_SAP: Color = Color(0.90, 0.78, 0.55)
const EARLY_HEART: Color = Color(0.74, 0.55, 0.35)
const LATE_SAP: Color = Color(0.56, 0.38, 0.20)
const LATE_HEART: Color = Color(0.42, 0.25, 0.12)
const STRESS_TINT: Color = Color(0.58, 0.20, 0.10)
const BARK: Color = Color(0.22, 0.15, 0.10)
const SAPWOOD_RINGS: int = 20
const MIN_BAND_PX: float = 3.0
const TEXT_H: float = 46.0
const MAX_ZOOM: float = 60.0

## Earlywood / latewood contrast 0..1 (1 = ring-porous oak): scales the tone difference inside a ring
## and the earlywood vessels (drawn only for strongly ring-porous wood).
var contrast: float = 1.0
var _bark_col: Color = BARK
var _widths: PackedFloat32Array = PackedFloat32Array()
var _stress: PackedFloat32Array = PackedFloat32Array()
var _early_frac: PackedFloat32Array = PackedFloat32Array()
var _forced: PackedByteArray = PackedByteArray()
var _radii: PackedFloat32Array = PackedFloat32Array() # cumulative outer radius (mm) per ring
var _early_col: Array[Color] = []
var _late_col: Array[Color] = []
var _avg_col: Array[Color] = []
var _hover: int = -1
var _zoom: float = 1.0
var _pan: Vector2 = Vector2.ZERO
var _dragging: bool = false
var _drag_moved: bool = false


func _ready() -> void:
	custom_minimum_size = Vector2(318, 360)
	mouse_filter = Control.MOUSE_FILTER_STOP
	clip_contents = true


## Redraw only happens when new data is pushed (once per completed year).
func set_data(widths: PackedFloat32Array, stress: PackedFloat32Array,
		early_frac: PackedFloat32Array = PackedFloat32Array(), forced: PackedByteArray = PackedByteArray()) -> void:
	var t0: int = Time.get_ticks_usec()
	_widths = widths
	_stress = stress
	_early_frac = early_frac
	_forced = forced
	_radii = PackedFloat32Array()
	_early_col.clear()
	_late_col.clear()
	_avg_col.clear()
	var acc: float = 0.0
	var n: int = _widths.size()
	for i: int in n:
		acc += _widths[i]
		_radii.append(acc)
		var heart: float = 1.0 if i < n - SAPWOOD_RINGS else 0.0
		var st: float = _stress[i] if i < _stress.size() else 0.0
		var ec: Color = EARLY_SAP.lerp(EARLY_HEART, heart).lerp(STRESS_TINT, st * 0.5)
		var lc: Color = LATE_SAP.lerp(LATE_HEART, heart).lerp(STRESS_TINT.darkened(0.3), st * 0.45)
		lc = ec.lerp(lc, 0.35 + 0.65 * contrast)  # diffuse-porous wood: latewood only slightly darker
		_early_col.append(ec)
		_late_col.append(lc)
		_avg_col.append(ec.lerp(lc, 1.0 - _ef(i)))
	_hover = -1
	queue_redraw()
	Prof.add("gd.ring_set_data", t0)


func reset_view() -> void:
	_zoom = 1.0
	_pan = Vector2.ZERO
	_hover = -1
	queue_redraw()


## Outer bark ring colour of diffuse-porous species (the oak keeps BARK).
func set_bark_color(c: Color) -> void:
	_bark_col = c.darkened(0.45)
	queue_redraw()


func get_zoom() -> float:
	return _zoom


func get_hover() -> int:
	return _hover


func _ef(i: int) -> float:
	return clampf(_early_frac[i], 0.05, 0.95) if i < _early_frac.size() else 0.55


func _geometry() -> Dictionary:
	var area_h: float = size.y - TEXT_H
	var c: Vector2 = Vector2(size.x * 0.5, area_h * 0.5) + _pan
	var avail: float = minf(size.x, area_h) * 0.5 - 6.0
	var total: float = _radii[_radii.size() - 1] if _radii.size() > 0 else 1.0
	var bark_mm: float = maxf(total * 0.05, 1.5)
	var scale_px: float = avail / (total + bark_mm) * _zoom
	return {"c": c, "scale": scale_px, "bark": bark_mm, "total": total}


func _ring_at(pos: Vector2) -> int:
	if _radii.size() == 0:
		return -1
	var g: Dictionary = _geometry()
	var d: float = (pos - (g["c"] as Vector2)).length() / (g["scale"] as float)
	# Binary search for the first ring whose outer radius >= d.
	var lo: int = 0
	var hi: int = _radii.size()
	while lo < hi:
		var mid: int = (lo + hi) >> 1
		if _radii[mid] >= d:
			hi = mid
		else:
			lo = mid + 1
	return lo if lo < _radii.size() else -1


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		var m: InputEventMouseMotion = event
		if _dragging:
			_pan += m.relative
			_drag_moved = true
			_clamp_pan()
			queue_redraw()
			accept_event()
			return
		var idx: int = _ring_at(m.position)
		if idx != _hover:
			_hover = idx
			queue_redraw()
	elif event is InputEventMouseButton:
		var b: InputEventMouseButton = event
		if b.button_index == MOUSE_BUTTON_WHEEL_UP and b.pressed:
			_zoom_at(b.position, 1.25)
			accept_event()
		elif b.button_index == MOUSE_BUTTON_WHEEL_DOWN and b.pressed:
			_zoom_at(b.position, 0.8)
			accept_event()
		elif b.button_index == MOUSE_BUTTON_LEFT:
			_dragging = b.pressed
			if b.pressed and b.double_click:
				reset_view()
			accept_event()


func _zoom_at(pos: Vector2, factor: float) -> void:
	var old: float = _zoom
	_zoom = clampf(_zoom * factor, 1.0, MAX_ZOOM)
	var f: float = _zoom / old
	if f != 1.0:
		var area_c: Vector2 = Vector2(size.x * 0.5, (size.y - TEXT_H) * 0.5)
		# Keep the point under the cursor fixed.
		var c_old: Vector2 = area_c + _pan
		var c_new: Vector2 = pos + (c_old - pos) * f
		_pan = c_new - area_c
		_clamp_pan()
		_hover = _ring_at(pos)
		queue_redraw()


func _clamp_pan() -> void:
	if _zoom <= 1.001:
		_pan = Vector2.ZERO
		return
	var lim: float = minf(size.x, size.y - TEXT_H) * 0.5 * _zoom
	_pan = _pan.clamp(Vector2(-lim, -lim), Vector2(lim, lim))


func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT:
		_dragging = false
		if _hover != -1:
			_hover = -1
			queue_redraw()
	elif what == NOTIFICATION_RESIZED:
		queue_redraw()


func _fmt(v: float, d: int = 1) -> String:
	return String.num(v, d).replace(".", ",")


func _arc_segments(r: float) -> int:
	return clampi(int(r * 0.9), 32, 360)


func _draw() -> void:
	var t0: int = Time.get_ticks_usec()
	_draw_rings()
	Prof.add("gd.ring_draw", t0)


func _draw_rings() -> void:
	var font: Font = UiStyle.font()
	var area_h: float = size.y - TEXT_H
	var c0: Vector2 = Vector2(size.x * 0.5, area_h * 0.5) + _pan
	if _radii.size() == 0:
		draw_circle(c0, 6.0, LATE_HEART)
		draw_string(font, Vector2(0, size.y - 14), "Aucun cerne complet (moins d'un an)",
			HORIZONTAL_ALIGNMENT_CENTER, size.x, 15, UiStyle.TEXT)
		return
	var g: Dictionary = _geometry()
	var c: Vector2 = g["c"]
	var s: float = g["scale"]
	var total: float = g["total"]
	var n: int = _radii.size()
	# Bark, then rings from the outside in.
	draw_circle(c, (total + (g["bark"] as float)) * s, BARK if contrast > 0.9 else _bark_col)
	var i: int = n - 1
	var vessels: Array[int] = []
	while i >= 0:
		var wpx: float = _widths[i] * s
		if wpx >= MIN_BAND_PX:
			var late_w: float = _widths[i] * (1.0 - _ef(i)) * s
			draw_circle(c, _radii[i] * s, _late_col[i])
			draw_circle(c, _radii[i] * s - late_w, _early_col[i])
			if wpx >= 9.0 and contrast > 0.6:
				vessels.append(i)
			i -= 1
		else:
			# Merge thin rings into one tone band (shade by local ring density).
			var r_out: float = _radii[i] * s
			var acc_px: float = 0.0
			var wsum: float = 0.0
			var col_sum: Color = Color(0, 0, 0, 0)
			var j: int = i
			var cnt: int = 0
			while j >= 0 and acc_px < MIN_BAND_PX and _widths[j] * s < MIN_BAND_PX:
				col_sum += _avg_col[j] * _widths[j]
				wsum += _widths[j]
				acc_px += _widths[j] * s
				cnt += 1
				j -= 1
			if wsum <= 0.0:
				break
			var col: Color = col_sum / wsum
			var density: float = clampf(acc_px / maxf(float(cnt), 1.0) / 1.5, 0.0, 1.0)
			col = col.darkened(0.12 * (1.0 - density))
			draw_circle(c, r_out, col)
			i = j
	# Radial rays (pale) and a couple of cracks (dark), subtle.
	var r_in: float = maxf(0.6 * s, 3.0)
	var r_max: float = total * s
	for k: int in 7:
		var a: float = float(k) * TAU / 7.0 + 0.37 * sin(float(k) * 2.3)
		var d: Vector2 = Vector2(cos(a), sin(a))
		draw_line(c + d * r_in, c + d * r_max, Color(1.0, 0.95, 0.8, 0.10), 1.0, true)
	for a2: float in [1.1, 3.9]:
		var d2: Vector2 = Vector2(cos(a2), sin(a2))
		draw_line(c + d2 * r_in, c + d2 * r_max * 0.62, Color(0.15, 0.07, 0.03, 0.40), 1.2, true)
	# Large earlywood vessels (ring-porous oak) when zoomed in far enough.
	for vi: int in vessels:
		_draw_vessels(c, s, vi)
	# Emphasised decade rings.
	var k10: int = 9
	while k10 < n:
		var rr: float = _radii[k10] * s
		draw_arc(c, rr, 0.0, TAU, _arc_segments(rr), Color(0.12, 0.06, 0.02, 0.60), 1.6, true)
		k10 += 10
	# Pith.
	draw_circle(c, maxf(2.5, 0.5 * s), Color(0.30, 0.18, 0.09))
	_draw_decade_labels(c, s, font, n)
	# Hover highlight and info text.
	draw_rect(Rect2(0.0, size.y - TEXT_H, size.x, TEXT_H), Color(UiStyle.PANEL.r * 0.6, UiStyle.PANEL.g * 0.6, UiStyle.PANEL.b * 0.6, 0.9))
	if _hover >= 0:
		var hr: float = _radii[_hover] * s
		draw_arc(c, hr, 0.0, TAU, _arc_segments(hr), Color(1, 1, 1, 0.95), 2.0, true)
		var hin: float = (_radii[_hover - 1] if _hover > 0 else 0.0) * s
		if hin > 1.0:
			draw_arc(c, hin, 0.0, TAU, _arc_segments(hin), Color(1, 1, 1, 0.55), 1.2, true)
		_draw_info(font, _hover)
	else:
		var hint: String = "%d cernes — survolez pour les détails" % n
		draw_string(font, Vector2(0, size.y - 26), hint, HORIZONTAL_ALIGNMENT_CENTER, size.x, 15, UiStyle.TEXT_DIM)
		draw_string(font, Vector2(0, size.y - 8), "Molette : zoom · glisser : déplacer · double-clic : recentrer"
			if _zoom > 1.01 else "Molette : zoom", HORIZONTAL_ALIGNMENT_CENTER, size.x, 13, UiStyle.TEXT_OFF.lightened(0.15))


func _draw_vessels(c: Vector2, s: float, i: int) -> void:
	var r_out: float = _radii[i] * s
	var early_w: float = _widths[i] * _ef(i) * s
	var r_start: float = (_radii[i - 1] if i > 0 else 0.0) * s
	var r_mid: float = r_start + early_w * 0.5
	# Skip rings whose annulus does not intersect the panel.
	var rect: Rect2 = Rect2(Vector2.ZERO, size)
	var dmin: float = c.distance_to(c.clamp(rect.position, rect.end))
	var dmax: float = maxf(maxf(c.distance_to(rect.position), c.distance_to(rect.end)),
		maxf(c.distance_to(Vector2(rect.end.x, 0.0)), c.distance_to(Vector2(0.0, rect.end.y))))
	if r_out < dmin or r_start > dmax:
		return
	var count: int = clampi(int(TAU * r_mid / 5.0), 8, 160)
	var pts: PackedVector2Array = PackedVector2Array()
	var rs: int = i * 7919 + 13
	var vcol: Color = Color(0.45, 0.28, 0.14, 0.75)
	for k: int in count:
		rs = (rs * 1103515245 + 12345) & 0x7fffffff
		var jitter: float = float(rs % 1000) / 1000.0
		rs = (rs * 1103515245 + 12345) & 0x7fffffff
		var jr: float = float(rs % 1000) / 1000.0
		var a: float = (float(k) + jitter * 0.8) * TAU / float(count)
		var rr: float = r_start + early_w * (0.25 + 0.5 * jr)
		var p: Vector2 = c + Vector2(cos(a), sin(a)) * rr
		pts.append(p)
		pts.append(p + Vector2(1.6, 0.0))
	draw_multiline(pts, vcol, 2.2)


func _draw_decade_labels(c: Vector2, s: float, font: Font, n: int) -> void:
	# Label every 10th (or 20th/40th...) ring on the right-hand horizontal ray when there is room.
	var ppr: float = (_radii[n - 1] / float(n)) * s
	var step: int = 10
	while step * ppr < 26.0 and step < 640:
		step *= 2
	var k: int = step - 1
	while k < n:
		var p: Vector2 = c + Vector2(_radii[k] * s + 3.0, -3.0)
		if p.x > 0.0 and p.x < size.x - 14.0 and p.y > 8.0 and p.y < size.y - TEXT_H:
			draw_string(font, p, str(k + 1), HORIZONTAL_ALIGNMENT_LEFT, -1, 12, Color(1, 1, 1, 0.85))
		k += step


func _draw_info(font: Font, i: int) -> void:
	var line1: String = "Cerne n° %d (année %d) — largeur %s mm" % [i + 1, i + 1, _fmt(_widths[i], 2)]
	var st: float = _stress[i] if i < _stress.size() else 0.0
	var forced: bool = i < _forced.size() and _forced[i] != 0
	var line2: String = "Bois initial %d %% · bois final %d %%" % [roundi(_ef(i) * 100.0), roundi((1.0 - _ef(i)) * 100.0)]
	var col2: Color = UiStyle.TEXT_DIM
	if forced:
		line2 = "Année de sécheresse provoquée — stress hydrique %s" % _fmt(st, 2)
		col2 = Color(1.0, 0.74, 0.45)
	elif st > 0.25:
		line2 = "Année sèche — stress hydrique %s" % _fmt(st, 2)
		col2 = Color(1.0, 0.74, 0.45)
	var heart: String = "aubier" if i >= _radii.size() - SAPWOOD_RINGS else "duramen"
	draw_string(font, Vector2(0, size.y - 26), line1 + " · " + heart, HORIZONTAL_ALIGNMENT_CENTER, size.x, 15, UiStyle.TEXT)
	draw_string(font, Vector2(0, size.y - 8), line2, HORIZONTAL_ALIGNMENT_CENTER, size.x, 14, col2)
