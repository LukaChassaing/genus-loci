class_name OverlayLegend
extends PanelContainer
## Bandeau de légende d'un calque (overlay D3) : un titre, puis des paires forme + couleur + nom (LegendGlyph).
## Posé au-dessus de la barre du bas ; passe à la ligne quand la zone 3D est étroite (grande échelle d'interface,
## mode Expert et panneau ouverts). Sans lien avec la simulation (réutilisable).

const SEP: int = 8

var _row: HFlowContainer
var legend_title: String = ""
var item_count: int = 0


func _init() -> void:
	theme_type_variation = "LegendPanel"
	mouse_filter = Control.MOUSE_FILTER_STOP
	_row = HFlowContainer.new()
	_row.add_theme_constant_override("h_separation", SEP)
	_row.add_theme_constant_override("v_separation", 2)
	add_child(_row)


## Largeur sur une seule ligne (le conteneur à retour ne donne que la largeur de son plus large élément).
func line_width() -> float:
	var w: float = 0.0
	for c: Node in _row.get_children():
		w += (c as Control).get_combined_minimum_size().x + SEP
	var sb: StyleBox = get_theme_stylebox("panel")
	return w - SEP + (sb.get_minimum_size().x if sb != null else 0.0)


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
