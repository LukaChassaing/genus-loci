class_name OverlayLegend
extends PanelContainer
## Bandeau de légende d'un calque (overlay D3) : un titre, puis des paires forme + couleur + nom (LegendGlyph).
## Une seule ligne, posée au-dessus de la barre du bas. Sans lien avec la simulation (réutilisable).

var _row: HBoxContainer
var legend_title: String = ""
var item_count: int = 0


func _init() -> void:
	theme_type_variation = "LegendPanel"
	mouse_filter = Control.MOUSE_FILTER_STOP
	_row = HBoxContainer.new()
	_row.add_theme_constant_override("separation", 6)
	add_child(_row)


## `items` : tableaux [forme, couleur, nom, info-bulle facultative].
func set_items(title: String, items: Array) -> void:
	legend_title = title
	for c: Node in _row.get_children():
		_row.remove_child(c)
		c.queue_free()
	var t: Label = UiStyle.label(title.to_upper(), "CardTitle")
	_row.add_child(t)
	item_count = items.size()
	for it: Array in items:
		var cell: HBoxContainer = HBoxContainer.new()
		cell.add_theme_constant_override("separation", 3)
		cell.mouse_filter = Control.MOUSE_FILTER_PASS
		if it.size() > 3:
			cell.tooltip_text = String(it[3])
		var g: LegendGlyph = LegendGlyph.new(String(it[0]), it[1] as Color)
		cell.add_child(g)
		var l: Label = UiStyle.label(String(it[2]), "SmallLabel")
		l.add_theme_color_override("font_color", UiStyle.TEXT)
		cell.add_child(l)
		_row.add_child(cell)
