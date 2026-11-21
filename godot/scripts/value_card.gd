class_name ValueCard
extends PanelContainer
## Carte de valeur (direction D2) : titre en petites capitales, valeur en gras, sparkline sur l'historique de la
## partie, bullet graph facultatif et lignes de détail. Info-bulle riche (`_make_custom_tooltip`) : valeur avec son
## unité, tendance, définition courte. Les références de sources ne sont pas encore exposées par la liaison :
## la ligne « source » s'affichera quand elles le seront (voir `source`).
## Ne lit pas la simulation : le propriétaire pousse les valeurs (réutilisable).

var title: String = ""
var value_label: Label
var spark: Sparkline
var bullet: BulletGraph
var _lines: VBoxContainer
var _col: VBoxContainer
## Info-bulle : définition courte, tendance (texte calculé par le propriétaire), source (vide = non exposée).
var definition: String = ""
var trend: String = ""
var source: String = ""


func _init(card_title: String, with_bullet: bool = false) -> void:
	title = card_title
	theme_type_variation = "Card"
	mouse_filter = Control.MOUSE_FILTER_STOP
	_col = VBoxContainer.new()
	_col.add_theme_constant_override("separation", 1)
	_col.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_col)
	var t: Label = UiStyle.label(card_title.to_upper(), "CardTitle")
	t.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_col.add_child(t)
	var row: HBoxContainer = HBoxContainer.new()
	row.add_theme_constant_override("separation", 6)
	row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_col.add_child(row)
	value_label = UiStyle.label("—", "CardValue")
	value_label.custom_minimum_size = Vector2(78, 0)
	value_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(value_label)
	spark = Sparkline.new()
	spark.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	spark.custom_minimum_size = Vector2(70, 20)
	row.add_child(spark)
	if with_bullet:
		bullet = BulletGraph.new()
		bullet.custom_minimum_size = Vector2(0, 10)
		_col.add_child(bullet)
	_lines = VBoxContainer.new()
	_lines.add_theme_constant_override("separation", 0)
	_lines.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_col.add_child(_lines)
	tooltip_text = card_title  # non vide : déclenche _make_custom_tooltip


## Ligne de détail sous la valeur (texte atténué, 12-13 px).
func add_line(variation: String = "SmallLabel") -> Label:
	var l: Label = UiStyle.label("", variation)
	l.mouse_filter = Control.MOUSE_FILTER_IGNORE
	l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_lines.add_child(l)
	return l


## Sparkline supplémentaire sous les lignes (deuxième grandeur de la carte).
func add_spark() -> Sparkline:
	var s: Sparkline = Sparkline.new()
	s.custom_minimum_size = Vector2(0, 16)
	_lines.add_child(s)
	return s


func set_value(text: String) -> void:
	if value_label.text != text:
		value_label.text = text


func set_series(d: PackedFloat32Array) -> void:
	spark.set_data(d)


func _make_custom_tooltip(_for_text: String) -> Object:
	var v: VBoxContainer = VBoxContainer.new()
	v.add_theme_constant_override("separation", 2)
	var t: Label = Label.new()
	t.text = "%s : %s" % [title, value_label.text]
	t.add_theme_font_override("font", UiStyle.font(700))
	t.add_theme_font_size_override("font_size", 14)
	t.add_theme_color_override("font_color", UiStyle.TEXT)
	v.add_child(t)
	for line: String in [trend, definition, ("Source : " + source) if source != "" else ""]:
		if line == "":
			continue
		var l: Label = Label.new()
		l.text = line
		l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		l.custom_minimum_size = Vector2(280, 0)
		l.add_theme_font_size_override("font_size", 13)
		l.add_theme_color_override("font_color", UiStyle.TEXT_DIM)
		v.add_child(l)
	return v
