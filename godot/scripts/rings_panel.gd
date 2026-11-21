class_name RingsPanel
extends VBoxContainer
## Panneau « Cernes » : la coupe du tronc à 1,30 m (RingView) et, dessous, la série des largeurs de cernes en
## sparkline (une courbe de croissance radiale, le « squelette » du dendrochronologue), avec les années sèches
## marquées d'un triangle (forme + couleur).

var ring_view: RingView
var spark: Sparkline
var _marks: Control
var _caption: Label
var _widths: PackedFloat32Array = PackedFloat32Array()
var _stress: PackedFloat32Array = PackedFloat32Array()
var _forced: PackedByteArray = PackedByteArray()


func _init() -> void:
	add_theme_constant_override("separation", 4)
	ring_view = RingView.new()
	add_child(ring_view)
	add_child(UiStyle.label("LARGEUR DES CERNES (MM), DE LA MOELLE À L'ÉCORCE", "CardTitle"))
	var box: Control = Control.new()
	box.custom_minimum_size = Vector2(0, 46)
	add_child(box)
	spark = Sparkline.new()
	spark.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	spark.offset_bottom = -8
	spark.y_min = 0.0
	box.add_child(spark)
	_marks = Control.new()
	_marks.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_marks.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_marks.draw.connect(_draw_marks)
	box.add_child(_marks)
	_caption = UiStyle.label("", "SmallLabel")
	_caption.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	add_child(_caption)


func set_data(widths: PackedFloat32Array, stress: PackedFloat32Array, early: PackedFloat32Array, forced: PackedByteArray) -> void:
	ring_view.set_data(widths, stress, early, forced)
	_widths = widths
	_stress = stress
	_forced = forced
	spark.set_data(widths)
	_marks.queue_redraw()
	var n: int = widths.size()
	if n == 0:
		_caption.text = "Aucun cerne complet (moins d'un an)."
		return
	var lo: float = INF
	var hi: float = 0.0
	var dry: int = 0
	for i: int in n:
		lo = minf(lo, widths[i])
		hi = maxf(hi, widths[i])
		if (i < forced.size() and forced[i] != 0) or (i < stress.size() and stress[i] > 0.25):
			dry += 1
	_caption.text = "%d cernes · de %s à %s mm · dernier %s mm · %d année%s sèche%s (triangles)" % [n, UiStyle.num(lo, 2),
		UiStyle.num(hi, 2), UiStyle.num(widths[n - 1], 2), dry, "s" if dry > 1 else "", "s" if dry > 1 else ""]


## Années sèches (stress > 0,25 ou sécheresse forcée) : petits triangles sous la courbe.
func _draw_marks() -> void:
	var n: int = _widths.size()
	if n < 2:
		return
	var w: float = _marks.size.x - 5.0
	var y: float = _marks.size.y - 7.0
	for i: int in n:
		var forced: bool = i < _forced.size() and _forced[i] != 0
		var st: float = _stress[i] if i < _stress.size() else 0.0
		if forced or st > 0.25:
			var x: float = 2.5 + w * float(i) / float(n - 1)
			var c: Color = UiStyle.URGENT if forced else UiStyle.WARN
			_marks.draw_colored_polygon(PackedVector2Array([Vector2(x, y), Vector2(x + 3.5, y + 6.0), Vector2(x - 3.5, y + 6.0)]), c)
