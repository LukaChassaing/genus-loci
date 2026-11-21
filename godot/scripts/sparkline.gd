class_name Sparkline
extends Control
## Sparkline (Tufte) : un « mot-graphique » de la taille du texte, sans axes. Trace la série, marque le dernier
## point (point plein) et, si `mark_extremes`, le minimum et le maximum (petits points creux). Une bande de
## référence facultative (`band_lo`..`band_hi`, unités des données) situe la plage normale ou un seuil.
## Ne dépend de rien d'autre que de ses données : réutilisable telle quelle (Seed Master).

var data: PackedFloat32Array = PackedFloat32Array()
var color: Color = UiStyle.SPARK
var line_width: float = 1.4
var mark_extremes: bool = true
## Bornes fixes de l'axe vertical (NAN = automatiques sur les données).
var y_min: float = NAN
var y_max: float = NAN
## Bande de référence (NAN = aucune) : dessinée en fond, discrète.
var band_lo: float = NAN
var band_hi: float = NAN
var band_color: Color = Color(UiStyle.LINE_DIM.r, UiStyle.LINE_DIM.g, UiStyle.LINE_DIM.b, 0.28)
## Seuil horizontal (NAN = aucun), tracé en pointillé.
var threshold: float = NAN
var threshold_color: Color = UiStyle.URGENT


func _init() -> void:
	custom_minimum_size = Vector2(60, 16)
	mouse_filter = Control.MOUSE_FILTER_IGNORE


func set_data(d: PackedFloat32Array) -> void:
	data = d
	queue_redraw()


func _range() -> Vector2:
	var lo: float = INF
	var hi: float = -INF
	for v: float in data:
		lo = minf(lo, v)
		hi = maxf(hi, v)
	if not is_nan(threshold):
		lo = minf(lo, threshold)
		hi = maxf(hi, threshold)
	if not is_nan(y_min):
		lo = y_min
	if not is_nan(y_max):
		hi = y_max
	if hi - lo < 1e-6:
		hi = lo + 1.0
		lo -= 0.0
	return Vector2(lo, hi)


func _draw() -> void:
	var n: int = data.size()
	var pad: float = 2.5
	var w: float = size.x - 2.0 * pad
	var h: float = size.y - 2.0 * pad
	if w <= 2.0 or h <= 2.0:
		return
	var r: Vector2 = _range()
	var to_y: Callable = func(v: float) -> float: return pad + h * (1.0 - (v - r.x) / (r.y - r.x))
	if not is_nan(band_lo) and not is_nan(band_hi):
		var y0: float = clampf(to_y.call(band_hi), pad, pad + h)
		var y1: float = clampf(to_y.call(band_lo), pad, pad + h)
		draw_rect(Rect2(pad, y0, w, maxf(y1 - y0, 1.0)), band_color)
	if not is_nan(threshold):
		var yt: float = to_y.call(threshold)
		draw_dashed_line(Vector2(pad, yt), Vector2(pad + w, yt), threshold_color, 1.0, 3.0)
	if n < 2:
		if n == 1:
			draw_circle(Vector2(pad + w, to_y.call(data[0])), 2.2, color)
		return
	# Au plus un point par pixel (pas de moiré sur une longue partie).
	var step: int = maxi(1, int(ceil(float(n) / maxf(w, 1.0))))
	var pts: PackedVector2Array = PackedVector2Array()
	var i: int = 0
	while i < n:
		pts.append(Vector2(pad + w * float(i) / float(n - 1), to_y.call(data[i])))
		i += step
	if (n - 1) % step != 0:
		pts.append(Vector2(pad + w, to_y.call(data[n - 1])))
	draw_polyline(pts, color, line_width, true)
	if mark_extremes and n > 3:
		var imin: int = 0
		var imax: int = 0
		for k: int in n:
			if data[k] < data[imin]:
				imin = k
			if data[k] > data[imax]:
				imax = k
		for k: int in [imin, imax]:
			var p: Vector2 = Vector2(pad + w * float(k) / float(n - 1), to_y.call(data[k]))
			draw_arc(p, 2.0, 0.0, TAU, 10, color, 1.0, true)
	draw_circle(pts[pts.size() - 1], 2.4, color)
