class_name RingView
extends Control
## Trunk cross-section drawn with concentric annual rings.

var _widths: PackedFloat32Array = PackedFloat32Array()
var _stress: PackedFloat32Array = PackedFloat32Array()
var _radii: PackedFloat32Array = PackedFloat32Array() # cumulative outer radius (mm) per ring
var _hover: int = -1

const EARLY: Color = Color(0.86, 0.70, 0.46)
const LATE: Color = Color(0.45, 0.28, 0.13)
const STRESS_TINT: Color = Color(0.62, 0.22, 0.12)
const BARK: Color = Color(0.25, 0.17, 0.11)


func _ready() -> void:
	custom_minimum_size = Vector2(360, 380)
	mouse_filter = Control.MOUSE_FILTER_STOP


## Redraw only happens when new data is pushed (once per completed year).
func set_data(widths: PackedFloat32Array, stress: PackedFloat32Array) -> void:
	_widths = widths
	_stress = stress
	_radii = PackedFloat32Array()
	var acc: float = 0.0
	for w: float in _widths:
		acc += w
		_radii.append(acc)
	_hover = -1
	queue_redraw()


func _geometry() -> Dictionary:
	var c: Vector2 = Vector2(size.x * 0.5, size.y * 0.5 - 14.0)
	var avail: float = minf(size.x, size.y - 28.0) * 0.5 - 6.0
	var total: float = _radii[_radii.size() - 1] if _radii.size() > 0 else 1.0
	var bark_mm: float = maxf(total * 0.06, 1.5)
	var scale_px: float = avail / (total + bark_mm)
	return {"c": c, "scale": scale_px, "bark": bark_mm, "total": total}


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion and _radii.size() > 0:
		var g: Dictionary = _geometry()
		var d: float = (event.position - (g["c"] as Vector2)).length() / (g["scale"] as float)
		var idx: int = -1
		for i: int in _radii.size():
			if d <= _radii[i]:
				idx = i
				break
		if idx != _hover:
			_hover = idx
			queue_redraw()


func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT and _hover != -1:
		_hover = -1
		queue_redraw()


func _fmt(v: float) -> String:
	return ("%.1f" % v).replace(".", ",")


func _draw() -> void:
	var font: Font = ThemeDB.fallback_font
	var c: Vector2 = Vector2(size.x * 0.5, size.y * 0.5 - 14.0)
	if _radii.size() == 0:
		draw_circle(c, 6.0, LATE)
		draw_string(font, Vector2(0, size.y - 8), "Aucun cerne complet (moins d'un an)",
			HORIZONTAL_ALIGNMENT_CENTER, size.x, 16, Color(0.9, 0.9, 0.9))
		return
	var g: Dictionary = _geometry()
	var s: float = g["scale"]
	var total: float = g["total"]
	# Bark outline, then rings from the outside in.
	draw_circle(c, (total + (g["bark"] as float)) * s, BARK)
	draw_circle(c, total * s + 0.5, LATE)
	for i: int in range(_radii.size() - 1, -1, -1):
		var r: float = _radii[i] * s
		var st: float = _stress[i] if i < _stress.size() else 0.0
		var early: Color = EARLY.lerp(STRESS_TINT, st * 0.55)
		var late: Color = LATE.lerp(Color(0.30, 0.10, 0.06), st * 0.5)
		draw_circle(c, r, early)
		var lw: float = maxf(1.2, _widths[i] * s * 0.22)
		draw_arc(c, maxf(r - lw * 0.5, 0.5), 0.0, TAU, 72, late, lw, true)
	draw_circle(c, maxf(2.5, 0.5 * s), Color(0.30, 0.18, 0.09)) # pith
	if _hover >= 0:
		var hr: float = _radii[_hover] * s
		draw_arc(c, hr, 0.0, TAU, 72, Color(1, 1, 1, 0.9), 2.0, true)
		var txt: String = "Cerne n° %d — %s mm" % [_hover + 1, _fmt(_widths[_hover])]
		if _hover < _stress.size() and _stress[_hover] > 0.15:
			txt += " (stress hydrique)"
		draw_string(font, Vector2(0, size.y - 8), txt, HORIZONTAL_ALIGNMENT_CENTER, size.x, 16, Color(1, 1, 1))
	else:
		draw_string(font, Vector2(0, size.y - 8), "%d cernes — survolez pour les détails" % _radii.size(),
			HORIZONTAL_ALIGNMENT_CENTER, size.x, 14, Color(0.8, 0.8, 0.8))
