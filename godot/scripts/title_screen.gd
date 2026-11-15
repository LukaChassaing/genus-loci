class_name TitleScreen
extends Control
## Shell scene (run/main_scene): painted title screen with the menu column on the right, and the pages
## reached from it (arboretum setup, plates, options). Routes straight to the simulation when a debug /
## test hook is set (DENDRO_SHOT, DENDRO_SELFTEST, DENDRO_BENCH, DENDRO_YEARS).
##
## Debug hook: DENDRO_SCREEN=title|setup|plate0|plate1|options (+ DENDRO_SHOT=<absolute png>, DENDRO_SIZE=WxH).
## DENDRO_SCREEN=pause is handled by the simulation scene (it needs a running tree).

const MENU_ENTRIES: Array = [
	["Arboretum", "arboretum", true],
	["Planches", "planches", true],
	["Laboratoire", "laboratoire", false],
	["Commandes", "commandes", false],
	["Options", "options", true],
	["Quitter", "quitter", true],
]

var backdrop: PaintedBackdrop
var backdrop_setup: PaintedBackdrop
var _shade: TextureRect
var column: Control
var page: String = ""
var menu_buttons: Array[Button] = []
var setup: ArboretumSetup
var options: OptionsPanel
var plates: PlateView
var _title_block: Control
var _info_block: Control
var _col_content: Control
## Built inside another scene (self-test): never routes away.
var embedded: bool = false


func _ready() -> void:
	var scr: String = OS.get_environment("DENDRO_SCREEN")
	if not embedded and (scr == "pause" or (Game.shell_screen() == "" and Game.is_automation())):
		Game.start_simulation_direct.call_deferred()
		return
	theme = UiStyle.make_theme(18)
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var sz: String = OS.get_environment("DENDRO_SIZE")
	if sz != "" and DisplayServer.get_name() != "headless":
		var p: PackedStringArray = sz.split("x")
		if p.size() == 2:
			get_window().size = Vector2i(int(p[0]), int(p[1]))
	var bg: ColorRect = ColorRect.new()
	bg.color = UiStyle.PANEL
	bg.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(bg)
	backdrop = PaintedBackdrop.new()
	add_child(backdrop)
	backdrop_setup = PaintedBackdrop.new()
	backdrop_setup.variant = 1
	backdrop_setup.art_name = "setup_backdrop"
	backdrop_setup.visible = false
	add_child(backdrop_setup)
	_build_title_block()
	_build_column()
	show_menu()
	if scr != "":
		_run_screen_hook(scr)


# ------------------------------------------ layout ---------------------------------------------

func _build_title_block() -> void:
	_title_block = Control.new()
	_title_block.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_title_block.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_title_block)
	var box: VBoxContainer = VBoxContainer.new()
	box.position = Vector2(60, 40)
	box.add_theme_constant_override("separation", 2)
	_title_block.add_child(box)
	var t: Label = UiStyle.serif_label("Genus Loci", 78, UiStyle.TITLE)
	t.add_theme_font_override("font", UiStyle.font(400, false, 2))
	t.add_theme_color_override("font_shadow_color", UiStyle.TITLE_SHADOW)
	t.add_theme_constant_override("shadow_offset_x", 2)
	t.add_theme_constant_override("shadow_offset_y", 3)
	box.add_child(t)
	var sub: Label = UiStyle.caps_label("Dendrologia", 24, Color(0.98, 0.82, 0.46))
	sub.add_theme_font_override("font", UiStyle.font(400, false, 9))
	sub.add_theme_color_override("font_shadow_color", UiStyle.TITLE_SHADOW)
	sub.add_theme_constant_override("shadow_offset_x", 1)
	sub.add_theme_constant_override("shadow_offset_y", 2)
	var subm: MarginContainer = MarginContainer.new()
	subm.add_theme_constant_override("margin_left", 6)
	subm.add_child(sub)
	box.add_child(subm)
	var proto: Label = UiStyle.serif_label("prototype", 15, Color(0.95, 0.90, 0.75), 400, true)
	proto.add_theme_color_override("font_shadow_color", UiStyle.TITLE_SHADOW)
	proto.add_theme_constant_override("shadow_offset_y", 1)
	var protom: MarginContainer = MarginContainer.new()
	protom.add_theme_constant_override("margin_left", 6)
	protom.add_theme_constant_override("margin_top", 8)
	protom.add_child(proto)
	box.add_child(protom)


func _build_column() -> void:
	column = Control.new()
	column.anchor_left = 0.66
	column.anchor_right = 1.0
	column.anchor_top = 0.0
	column.anchor_bottom = 1.0
	column.offset_left = 0
	add_child(column)
	var bg: ColorRect = ColorRect.new()
	bg.color = Color(UiStyle.PANEL.r, UiStyle.PANEL.g, UiStyle.PANEL.b, 0.86)
	bg.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	bg.mouse_filter = Control.MOUSE_FILTER_STOP
	column.add_child(bg)
	var edge: ColorRect = ColorRect.new()
	edge.color = Color(UiStyle.LINE.r, UiStyle.LINE.g, UiStyle.LINE.b, 0.9)
	edge.set_anchors_and_offsets_preset(Control.PRESET_LEFT_WIDE)
	edge.offset_right = 1.0
	column.add_child(edge)
	_col_content = MarginContainer.new()
	_col_content.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_col_content.add_theme_constant_override("margin_left", 36)
	_col_content.add_theme_constant_override("margin_right", 36)
	_col_content.add_theme_constant_override("margin_top", 32)
	_col_content.add_theme_constant_override("margin_bottom", 26)
	column.add_child(_col_content)


func _clear_column() -> void:
	for c: Node in _col_content.get_children():
		_col_content.remove_child(c)
		c.queue_free()
	menu_buttons.clear()
	setup = null
	options = null
	if _info_block != null:
		_info_block.queue_free()
		_info_block = null
	if _shade != null:
		_shade.queue_free()
		_shade = null


# ------------------------------------------- pages ---------------------------------------------

func show_menu() -> void:
	_clear_column()
	_set_plates(false)
	backdrop_setup.visible = false
	page = "menu"
	_title_block.visible = true
	backdrop.visible = true
	column.visible = true
	var v: VBoxContainer = VBoxContainer.new()
	v.add_theme_constant_override("separation", 12)
	v.alignment = BoxContainer.ALIGNMENT_CENTER
	_col_content.add_child(v)
	var head: Label = UiStyle.caps_label("Menu principal", 13)
	v.add_child(head)
	v.add_child(UiStyle.rule())
	var spacer: Control = Control.new()
	spacer.custom_minimum_size = Vector2(0, 6)
	v.add_child(spacer)
	for e: Array in MENU_ENTRIES:
		var b: Button = UiStyle.button(e[0], 20)
		b.custom_minimum_size = Vector2(0, 46)
		b.name = "Menu_" + (e[1] as String)
		if not (e[2] as bool):
			b.disabled = true
			b.focus_mode = Control.FOCUS_NONE
			var tag: Label = UiStyle.serif_label("à venir", 14, UiStyle.TEXT_DIM, 400, true)
			tag.set_anchors_preset(Control.PRESET_CENTER_RIGHT)
			tag.grow_horizontal = Control.GROW_DIRECTION_BEGIN
			tag.grow_vertical = Control.GROW_DIRECTION_BOTH
			tag.offset_right = -18
			tag.mouse_filter = Control.MOUSE_FILTER_IGNORE
			b.add_child(tag)
		else:
			b.pressed.connect(_on_menu.bind(e[1] as String))
		v.add_child(b)
		menu_buttons.append(b)
	var foot: Label = UiStyle.serif_label("↑ ↓ naviguer · Entrée choisir · Échap retour", 13, UiStyle.TEXT_DIM, 400, true)
	foot.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	var fm: MarginContainer = MarginContainer.new()
	fm.add_theme_constant_override("margin_top", 28)
	fm.add_child(foot)
	v.add_child(fm)
	for b: Button in menu_buttons:
		if not b.disabled:
			UiStyle.focus_later(b)
			break


func _on_menu(which: String) -> void:
	match which:
		"arboretum":
			show_setup()
		"planches":
			show_plates(0)
		"options":
			show_options()
		"quitter":
			Game.quit_game()


func show_setup() -> void:
	_clear_column()
	_set_plates(false)
	backdrop.visible = false
	backdrop_setup.visible = true
	page = "setup"
	_title_block.visible = true
	setup = ArboretumSetup.new()
	setup.launched.connect(func(sp: int, soil: int, seed_value: int, years: int) -> void:
		Game.start_simulation(sp, soil, seed_value, years))
	setup.back_requested.connect(show_menu)
	setup.species_changed.connect(_update_species_info)
	_col_content.add_child(setup)
	_update_species_info(setup.species)
	setup.focus_first()


## Short species note over the painting (lower left), like the narrative text of a dark RPG screen.
func _update_species_info(index: int) -> void:
	if _info_block != null:
		_info_block.queue_free()
	var d: Dictionary = PlateView.species_data(index)
	if _shade == null:
		var g: Gradient = Gradient.new()
		g.set_color(0, Color(0, 0, 0, 0.0))
		g.set_color(1, Color(0.06, 0.09, 0.04, 0.80))
		var gt: GradientTexture2D = GradientTexture2D.new()
		gt.gradient = g
		gt.fill_from = Vector2(0, 0)
		gt.fill_to = Vector2(0, 1)
		var tr: TextureRect = TextureRect.new()
		tr.texture = gt
		tr.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		tr.anchor_top = 1.0
		tr.anchor_bottom = 1.0
		tr.anchor_right = 0.66
		tr.offset_top = -260
		tr.mouse_filter = Control.MOUSE_FILTER_IGNORE
		add_child(tr)
		_shade = tr
	var box: VBoxContainer = VBoxContainer.new()
	box.anchor_top = 1.0
	box.anchor_bottom = 1.0
	box.anchor_left = 0.0
	box.anchor_right = 0.66
	box.offset_left = 60
	box.offset_right = -40
	box.offset_top = -190
	box.offset_bottom = -44
	box.alignment = BoxContainer.ALIGNMENT_END
	box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	box.add_theme_constant_override("separation", 4)
	var cap: Label = UiStyle.caps_label(d["common"], 15, Color(0.98, 0.82, 0.46))
	var sci: Label = UiStyle.serif_label(d["latin"], 26, UiStyle.TITLE, 400, true)
	var txt: Label = UiStyle.serif_label(d["blurb"], 16, Color(0.95, 0.92, 0.80))
	txt.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	txt.custom_minimum_size = Vector2(520, 0)
	box.add_child(cap)
	box.add_child(sci)
	box.add_child(txt)
	add_child(box)
	_info_block = box


func show_options() -> void:
	_clear_column()
	_set_plates(false)
	backdrop_setup.visible = false
	page = "options"
	_title_block.visible = true
	options = OptionsPanel.new()
	options.closed.connect(show_menu)
	_col_content.add_child(options)
	options.focus_first()


func show_plates(index: int) -> void:
	_clear_column()
	page = "plates"
	_title_block.visible = false
	if plates == null:
		plates = PlateView.new()
		plates.closed.connect(show_menu)
		add_child(plates)
	plates.set_species(index)
	_set_plates(true)


func _set_plates(on: bool) -> void:
	if plates != null:
		plates.visible = on
	backdrop.visible = not on and page != "setup"
	column.visible = not on


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo and (event as InputEventKey).keycode == KEY_ESCAPE:
		if page != "menu":
			show_menu()
			get_viewport().set_input_as_handled()


# ------------------------------------------ screenshots ----------------------------------------

func _run_screen_hook(scr: String) -> void:
	match scr:
		"setup":
			show_setup()
		"options":
			show_options()
		"plate0":
			show_plates(0)
		"plate1":
			show_plates(1)
	var shot: String = OS.get_environment("DENDRO_SHOT")
	if shot == "":
		return
	var tries: int = 0
	var active: PaintedBackdrop = backdrop_setup if page == "setup" else backdrop
	while tries < 1200 and not active.settled:
		await get_tree().process_frame
		tries += 1
	for i: int in 8:
		await get_tree().process_frame
	if DisplayServer.get_name() == "headless":
		print("[dendro] headless: screenshot skipped")
	else:
		var img: Image = get_viewport().get_texture().get_image()
		DirAccess.make_dir_recursive_absolute(shot.get_base_dir())
		var err: int = img.save_png(shot)
		print("[dendro] screen '%s' saved (%d, %dx%d) -> %s" % [scr, err, img.get_width(), img.get_height(), shot])
	get_tree().quit()
