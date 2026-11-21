class_name OptionsPanel
extends VBoxContainer
## Display options (window mode, resolution, MSAA, FXAA, VSync, UI scale) and the Expert mode (in-game value column). Changes apply immediately and are
## saved to user://settings.cfg. Used in the title column and in the pause overlay.

signal closed

var _res: Segmented
var _full: Segmented
var _msaa: Segmented
var _fxaa: Segmented
var _vsync: Segmented
var _scale: Segmented
var _expert: Segmented
var _back: Button
## Settings path override (tests); empty = the real user://settings.cfg.
var save_path: String = ""
var apply_live: bool = true


func _ready() -> void:
	add_theme_constant_override("separation", 6)
	alignment = BoxContainer.ALIGNMENT_CENTER
	var s: Dictionary = Game.settings
	add_child(UiStyle.caps_label("Options", 13))
	add_child(UiStyle.rule())
	_full = _row("Affichage", Segmented.new().setup(["Fenêtré", "Plein écran"], 1 if bool(s["fullscreen"]) else 0, 2), "fullscreen")
	var res_labels: Array = []
	for r: Vector2i in Game.RESOLUTIONS:
		res_labels.append("%d × %d" % [r.x, r.y])
	_res = _row("Résolution (fenêtré)", Segmented.new().setup(res_labels, int(s["resolution"]), 2), "resolution")
	_msaa = _row("Anticrénelage MSAA", Segmented.new().setup(["Aucun", "2×", "4×"], [0, 2, 4].find(int(s["msaa"])), 3), "msaa")
	_fxaa = _row("FXAA", Segmented.new().setup(["Non", "Oui"], 1 if bool(s["fxaa"]) else 0, 2), "fxaa")
	_vsync = _row("Synchronisation verticale", Segmented.new().setup(["Non", "Oui"], 1 if bool(s["vsync"]) else 0, 2), "vsync")
	var scale_labels: Array = []
	for u: float in Game.UI_SCALES:
		scale_labels.append("%d %%" % roundi(u * 100.0))
	_scale = _row("Échelle de l'interface", Segmented.new().setup(scale_labels, Game.UI_SCALES.find(float(s["ui_scale"])), 4), "ui_scale")
	_expert = _row("Mode Expert (colonne des valeurs en jeu)", Segmented.new().setup(["Non", "Oui"], 1 if bool(s.get("expert", false)) else 0, 2), "expert")
	var sp: Control = Control.new()
	sp.custom_minimum_size = Vector2(0, 8)
	add_child(sp)
	_back = UiStyle.button("Retour", 18)
	_back.custom_minimum_size = Vector2(0, 42)
	_back.pressed.connect(func() -> void: closed.emit())
	add_child(_back)
	_refresh_enabled()


func _row(label: String, seg: Segmented, key: String) -> Segmented:
	var l: Label = UiStyle.caps_label(label, 12)
	var m: MarginContainer = MarginContainer.new()
	m.add_theme_constant_override("margin_top", 8)
	m.add_child(l)
	add_child(m)
	add_child(seg)
	seg.picked.connect(_on_picked.bind(key))
	return seg


func _on_picked(i: int, key: String) -> void:
	apply_choice(key, i)


## Sets one option from its selector index, then applies and saves (also used by the self-test).
func apply_choice(key: String, i: int) -> void:
	match key:
		"fullscreen":
			Game.settings["fullscreen"] = i == 1
		"resolution":
			Game.settings["resolution"] = i
		"msaa":
			Game.settings["msaa"] = [0, 2, 4][i]
		"fxaa":
			Game.settings["fxaa"] = i == 1
		"vsync":
			Game.settings["vsync"] = i == 1
		"ui_scale":
			Game.settings["ui_scale"] = Game.UI_SCALES[i]
		"expert":
			Game.settings["expert"] = i == 1
	if apply_live:
		Game.apply_window()
	Game.save_settings(save_path if save_path != "" else Game.SETTINGS_PATH)
	Game.settings_changed.emit()
	_refresh_enabled()


func _refresh_enabled() -> void:
	var fs: bool = bool(Game.settings["fullscreen"])
	for b: Button in _res.buttons:
		b.disabled = fs
		b.focus_mode = Control.FOCUS_NONE if fs else Control.FOCUS_ALL


func focus_first() -> void:
	UiStyle.focus_later(_full.buttons[0])
