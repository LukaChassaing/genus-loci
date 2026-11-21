class_name HelpOverlay
extends PanelContainer
## Aide des raccourcis (touche F1, « ? » ou H) : la liste est fournie par le propriétaire (Hud.SHORTCUTS), en
## groupes. Se ferme par la croix, F1, « ? », H ou Échap.

signal closed

var _grid_box: VBoxContainer


func _init() -> void:
	theme_type_variation = "MenuPanel"
	mouse_filter = Control.MOUSE_FILTER_STOP
	visible = false
	var col: VBoxContainer = VBoxContainer.new()
	col.add_theme_constant_override("separation", 6)
	add_child(col)
	var head: HBoxContainer = HBoxContainer.new()
	col.add_child(head)
	var t: Label = UiStyle.label("RACCOURCIS CLAVIER ET SOURIS", "HeaderLabel")
	t.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	head.add_child(t)
	var x: Button = Button.new()
	x.text = "✕"
	x.theme_type_variation = "FlatButton"
	x.focus_mode = Control.FOCUS_NONE
	x.tooltip_text = "Fermer l'aide (F1)"
	x.pressed.connect(func() -> void:
		visible = false
		closed.emit())
	head.add_child(x)
	col.add_child(UiStyle.rule(UiStyle.LINE_DIM))
	_grid_box = VBoxContainer.new()
	_grid_box.add_theme_constant_override("separation", 8)
	col.add_child(_grid_box)


## `groups` : [[titre, [[touche, action], ...]], ...] ; deux colonnes de groupes.
func set_shortcuts(groups: Array) -> void:
	for c: Node in _grid_box.get_children():
		_grid_box.remove_child(c)
		c.queue_free()
	var cols: HBoxContainer = HBoxContainer.new()
	cols.add_theme_constant_override("separation", 28)
	_grid_box.add_child(cols)
	var left: VBoxContainer = VBoxContainer.new()
	var right: VBoxContainer = VBoxContainer.new()
	for v: VBoxContainer in [left, right]:
		v.add_theme_constant_override("separation", 8)
		cols.add_child(v)
	for gi: int in groups.size():
		var g: Array = groups[gi]
		var target: VBoxContainer = left if gi < (groups.size() + 1) / 2 else right
		target.add_child(UiStyle.label(String(g[0]).to_upper(), "CardTitle"))
		var grid: GridContainer = GridContainer.new()
		grid.columns = 2
		grid.add_theme_constant_override("h_separation", 12)
		grid.add_theme_constant_override("v_separation", 2)
		target.add_child(grid)
		for e: Array in g[1]:
			var k: Label = UiStyle.label(String(e[0]), "")
			k.add_theme_font_override("font", UiStyle.font(700))
			k.add_theme_font_size_override("font_size", 14)
			k.add_theme_color_override("font_color", UiStyle.TEXT)
			k.custom_minimum_size = Vector2(96, 0)
			grid.add_child(k)
			var d: Label = UiStyle.label(String(e[1]), "DimLabel")
			grid.add_child(d)
