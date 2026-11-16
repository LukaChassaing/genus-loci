class_name PauseMenu
extends CanvasLayer
## In-game pause overlay (Esc): Reprendre, Sauvegarder, Charger, Planche de l'espèce, Options, Retour au menu
## (autosaves first), Quitter.
## Dims and blurs the frozen game view, or shows godot/art/pause_backdrop.* when that image exists.

signal resumed

const BLUR_SHADER: String = """
shader_type canvas_item;
uniform sampler2D screen_tex : hint_screen_texture, filter_linear_mipmap;
void fragment() {
	vec3 c = textureLod(screen_tex, SCREEN_UV, 2.6).rgb;
		c = mix(c, vec3(0.10, 0.14, 0.06), 0.62);
	float v = smoothstep(0.35, 1.0, length((SCREEN_UV - 0.5) * vec2(1.1, 1.0)));
	COLOR = vec4(c * (1.0 - 0.25 * v), 1.0);
}
"""

var species: int = 0
var root: Control
var buttons: Array[Button] = []
var _panel: PanelContainer
var _menu_box: VBoxContainer
var _sub: Control = null


func _ready() -> void:
	layer = 60
	process_mode = Node.PROCESS_MODE_ALWAYS
	root = Control.new()
	root.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	root.theme = UiStyle.make_theme(18)
	add_child(root)
	var art: Texture2D = Game.load_art("pause_backdrop")
	if art != null:
		var tr: TextureRect = TextureRect.new()
		tr.texture = art
		tr.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		tr.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		tr.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
		root.add_child(tr)
		var dim: ColorRect = ColorRect.new()
		dim.color = Color(0.08, 0.10, 0.05, 0.45)
		dim.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		root.add_child(dim)
	else:
		var blur: ColorRect = ColorRect.new()
		blur.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		var sh: Shader = Shader.new()
		sh.code = BLUR_SHADER
		var m: ShaderMaterial = ShaderMaterial.new()
		m.shader = sh
		blur.material = m
		root.add_child(blur)
	_panel = PanelContainer.new()
	_panel.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
	_panel.grow_horizontal = Control.GROW_DIRECTION_BOTH
	_panel.grow_vertical = Control.GROW_DIRECTION_BOTH
	_panel.add_theme_stylebox_override("panel", UiStyle.box(Color(UiStyle.PANEL.r, UiStyle.PANEL.g, UiStyle.PANEL.b, 0.94), UiStyle.LINE, 1, 34.0, 28.0))
	root.add_child(_panel)
	_show_menu()
	match OS.get_environment("DENDRO_SCREEN"):  # screenshot hooks
		"pause_save":
			_open_sub(_make_browser("save"))
		"pause_load":
			_open_sub(_make_browser("load"))


func _show_menu() -> void:
	if _sub != null:
		_sub.queue_free()
		_sub = null
	if _menu_box != null:
		_menu_box.queue_free()
	buttons.clear()
	_menu_box = VBoxContainer.new()
	_menu_box.custom_minimum_size = Vector2(340, 0)
	_menu_box.add_theme_constant_override("separation", 10)
	_panel.add_child(_menu_box)
	var title: Label = UiStyle.serif_label("Pause", 34, UiStyle.TEXT)
	title.add_theme_font_override("font", UiStyle.font(400, false, 2))
	_menu_box.add_child(title)
	_menu_box.add_child(UiStyle.rule())
	var sp: Control = Control.new()
	sp.custom_minimum_size = Vector2(0, 6)
	_menu_box.add_child(sp)
	var entries: Array = [["Reprendre", "resume"], ["Sauvegarder", "save"], ["Charger", "load"], ["Planche de l'espèce", "plate"], ["Options", "options"], ["Retour au menu", "title"], ["Quitter", "quit"]]
	for e: Array in entries:
		var b: Button = UiStyle.button(e[0], 19)
		b.custom_minimum_size = Vector2(0, 44)
		b.name = "Pause_" + (e[1] as String)
		b.pressed.connect(_on_entry.bind(e[1] as String))
		_menu_box.add_child(b)
		buttons.append(b)
	UiStyle.focus_later(buttons[0])


func _on_entry(which: String) -> void:
	match which:
		"resume":
			close()
		"plate":
			_open_sub(_make_plate())
		"options":
			var o: OptionsPanel = OptionsPanel.new()
			o.closed.connect(_show_menu_again)
			var wrap: VBoxContainer = VBoxContainer.new()
			wrap.custom_minimum_size = Vector2(400, 0)
			wrap.add_child(o)
			_open_sub(wrap)
			o.focus_first()
		"save", "load":
			_open_sub(_make_browser(which))
		"title":
			var sf: SaveFlow = get_parent().get("save_flow") as SaveFlow if get_parent() != null else null
			if sf != null:
				sf.autosave()  # single rolling slot
			Game.go_title()
		"quit":
			Game.quit_game()


## Save / load page (SaveBrowser) inside the pause panel; the I/O goes through the scene's SaveFlow.
func _make_browser(mode: String) -> Control:
	var sf: SaveFlow = get_parent().get("save_flow") as SaveFlow if get_parent() != null else null
	var dn: String = SaveFlow.default_name(sf.sim) if sf != null else "Arbre"
	var sb: SaveBrowser = SaveBrowser.new(mode, "", dn)
	sb.name = "SaveBrowser"
	sb.custom_minimum_size = Vector2(500, 0)
	sb.closed.connect(_show_menu_again)
	sb.load_requested.connect(func(path: String) -> void: Game.load_save(path))
	sb.save_requested.connect(func(save_name: String, path: String) -> void:
		var err: String = sf.save_as(save_name, path) if sf != null else "pas de simulation"
		sb.refresh()
		sb.set_status("Arbre sauvegardé : « %s »." % save_name if err == "" else "Échec de la sauvegarde : %s" % err, err != ""))
	return sb


func _make_plate() -> Control:
	var pv: PlateView = PlateView.new()
	pv.set_species(species)
	pv.closed.connect(_show_menu_again)
	root.add_child(pv)
	_panel.visible = false
	_sub = pv
	return pv


func _open_sub(c: Control) -> void:
	if _menu_box != null:
		_menu_box.queue_free()
		_menu_box = null
	if c.get_parent() == null:
		_panel.add_child(c)
	_sub = c
	_panel.visible = c is not PlateView


func _show_menu_again() -> void:
	_panel.visible = true
	_show_menu()


func close() -> void:
	get_tree().paused = false
	resumed.emit()
	queue_free()


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo and (event as InputEventKey).keycode == KEY_ESCAPE:
		if _sub != null:
			_show_menu_again()
		else:
			close()
		get_viewport().set_input_as_handled()
