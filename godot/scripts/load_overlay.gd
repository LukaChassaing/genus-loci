class_name LoadOverlay
extends CanvasLayer
## Full-screen card shown while a saved tree is regrown by replay: species, save name, progress bar and
## the simulated year. Runs while the tree is paused (PROCESS_MODE_ALWAYS); the host feeds `set_progress`.

var bar: ProgressBar
var year_label: Label
var _target_years: float = 0.0


func _init(common: String, latin: String, save_name: String, target_years: float) -> void:
	layer = 70
	process_mode = Node.PROCESS_MODE_ALWAYS
	name = "LoadOverlay"
	_target_years = target_years
	var root: Control = Control.new()
	root.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	root.theme = UiStyle.make_theme(18)
	root.mouse_filter = Control.MOUSE_FILTER_STOP
	add_child(root)
	var bg: ColorRect = ColorRect.new()
	bg.color = Color(UiStyle.PANEL.r * 0.55, UiStyle.PANEL.g * 0.55, UiStyle.PANEL.b * 0.55, 1.0)
	bg.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	root.add_child(bg)
	var panel: PanelContainer = PanelContainer.new()
	panel.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
	panel.grow_horizontal = Control.GROW_DIRECTION_BOTH
	panel.grow_vertical = Control.GROW_DIRECTION_BOTH
	panel.add_theme_stylebox_override("panel", UiStyle.box(Color(UiStyle.PANEL.r, UiStyle.PANEL.g, UiStyle.PANEL.b, 0.96), UiStyle.LINE, 1, 40.0, 30.0))
	root.add_child(panel)
	var v: VBoxContainer = VBoxContainer.new()
	v.custom_minimum_size = Vector2(460, 0)
	v.add_theme_constant_override("separation", 8)
	panel.add_child(v)
	v.add_child(UiStyle.caps_label("Chargement", 13))
	v.add_child(UiStyle.rule())
	var cap: Label = UiStyle.caps_label(common, 15, Color(0.98, 0.82, 0.46))
	v.add_child(cap)
	var sci: Label = UiStyle.serif_label(latin, 28, UiStyle.TITLE, 400, true)
	v.add_child(sci)
	var nm: Label = UiStyle.serif_label("« %s »" % save_name, 17, UiStyle.TEXT)
	nm.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	v.add_child(nm)
	var sp: Control = Control.new()
	sp.custom_minimum_size = Vector2(0, 8)
	v.add_child(sp)
	bar = ProgressBar.new()
	bar.min_value = 0.0
	bar.max_value = 1.0
	bar.step = 0.0
	bar.show_percentage = false
	bar.custom_minimum_size = Vector2(0, 14)
	bar.add_theme_stylebox_override("fill", UiStyle.box(UiStyle.SLIDER_FILL, Color(0, 0, 0, 0), 0, 0.0, 0.0))
	v.add_child(bar)
	year_label = UiStyle.serif_label("", 15, UiStyle.TEXT_DIM, 400, true)
	v.add_child(year_label)
	var hint: Label = UiStyle.serif_label("L'arbre repousse depuis la graine : même graine, même climat, mêmes tailles.", 14, UiStyle.TEXT_DIM, 400, true)
	hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	hint.custom_minimum_size = Vector2(460, 0)
	v.add_child(hint)
	set_progress(0.0)


func set_progress(p: float) -> void:
	bar.value = clampf(p, 0.0, 1.0)
	var y: int = int(floor(p * _target_years))
	year_label.text = "Rejeu de l'histoire : année %d sur %d · %d %%" % [y, int(ceil(_target_years)), int(p * 100.0)]
