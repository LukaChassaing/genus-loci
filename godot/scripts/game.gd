extends Node
## Autoload "Game": settings (user://settings.cfg), the arboretum choice handed to the simulation scene,
## scene routing and optional painted backdrops (godot/art/*.png|jpg|webp, procedural fallback otherwise).

const SETTINGS_PATH: String = "user://settings.cfg"
const MAIN_SCENE: String = "res://main.tscn"
const TITLE_SCENE: String = "res://title.tscn"
const RESOLUTIONS: Array[Vector2i] = [Vector2i(1280, 720), Vector2i(1600, 900), Vector2i(1920, 1080), Vector2i(2560, 1440)]
const UI_SCALES: Array[float] = [0.85, 1.0, 1.15, 1.3]
const AUTOMATION_VARS: Array[String] = ["DENDRO_SHOT", "DENDRO_SELFTEST", "DENDRO_BENCH", "DENDRO_YEARS", "DENDRO_SAVE", "DENDRO_LOAD"]
const SHELL_SCREENS: Array[String] = ["title", "setup", "plate0", "plate1", "options", "load", "load_confirm"]

var settings: Dictionary = default_settings()
## Choice made on the setup screen, consumed once by main.gd. Empty = nothing chosen (env vars / defaults).
var pending: Dictionary = {}
var _art_cache: Dictionary = {}


static func default_settings() -> Dictionary:
	return {"fullscreen": false, "resolution": 0, "msaa": 4, "fxaa": true, "vsync": true, "ui_scale": 1.0}


func _ready() -> void:
	load_settings()
	if not is_automation():
		apply_window()


## True when a debug / test hook wants the simulation scene straight away (title screen skipped) and the
## user's saved window settings must not alter the run.
static func is_automation() -> bool:
	for v: String in AUTOMATION_VARS:
		if OS.get_environment(v) != "":
			return true
	return false


## The DENDRO_SCREEN value if it names a shell screen (title, setup, plate0, plate1, options), else "".
static func shell_screen() -> String:
	var s: String = OS.get_environment("DENDRO_SCREEN")
	return s if s in SHELL_SCREENS else ""


# ------------------------------------------- settings ------------------------------------------

func load_settings(path: String = SETTINGS_PATH) -> void:
	settings = default_settings()
	var cf: ConfigFile = ConfigFile.new()
	if cf.load(path) != OK:
		return
	for k: String in settings.keys():
		if cf.has_section_key("display", k):
			var v: Variant = cf.get_value("display", k)
			if typeof(v) == typeof(settings[k]) or (k == "ui_scale" and (typeof(v) == TYPE_FLOAT or typeof(v) == TYPE_INT)):
				settings[k] = v
	settings["resolution"] = clampi(int(settings["resolution"]), 0, RESOLUTIONS.size() - 1)
	settings["ui_scale"] = clampf(float(settings["ui_scale"]), 0.7, 1.5)
	if not int(settings["msaa"]) in [0, 2, 4]:
		settings["msaa"] = 4


func save_settings(path: String = SETTINGS_PATH) -> int:
	var cf: ConfigFile = ConfigFile.new()
	for k: String in settings.keys():
		cf.set_value("display", k, settings[k])
	return cf.save(path)


func msaa_mode() -> Viewport.MSAA:
	if is_automation():
		return Viewport.MSAA_4X  # hooks always run on the shipping defaults
	match int(settings["msaa"]):
		0:
			return Viewport.MSAA_DISABLED
		2:
			return Viewport.MSAA_2X
	return Viewport.MSAA_4X


func fxaa_mode() -> Viewport.ScreenSpaceAA:
	if is_automation():
		return Viewport.SCREEN_SPACE_AA_FXAA
	return Viewport.SCREEN_SPACE_AA_FXAA if bool(settings["fxaa"]) else Viewport.SCREEN_SPACE_AA_DISABLED


## Window mode, size, vsync, UI scale and anti-aliasing of the root viewport.
func apply_window() -> void:
	var win: Window = get_window()
	if DisplayServer.get_name() == "headless":
		return
	win.content_scale_factor = float(settings["ui_scale"])
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_ENABLED if bool(settings["vsync"]) else DisplayServer.VSYNC_DISABLED)
	if bool(settings["fullscreen"]):
		win.mode = Window.MODE_FULLSCREEN
	else:
		if win.mode == Window.MODE_FULLSCREEN:
			win.mode = Window.MODE_WINDOWED
		var res: Vector2i = RESOLUTIONS[int(settings["resolution"])]
		var screen: Vector2i = DisplayServer.screen_get_size()
		if res.x <= screen.x and res.y <= screen.y:
			win.size = res
			win.move_to_center()
	apply_aa(win)


func apply_aa(vp: Viewport) -> void:
	vp.msaa_3d = msaa_mode()
	vp.screen_space_aa = fxaa_mode()


# ------------------------------------------- routing -------------------------------------------

## Launches the simulation with an arboretum choice: species, soil, seed (-1 = random), start age in years.
func start_simulation(species: int, soil: int, seed_value: int, years: int) -> void:
	pending = {"species": species, "soil": soil, "seed": seed_value, "years": years}
	get_tree().paused = false
	get_tree().change_scene_to_file(MAIN_SCENE)


## Opens a saved tree (absolute path of a .ron in user://saves/): the simulation scene regrows it by replay.
func load_save(path: String) -> void:
	pending = {"load": path}
	get_tree().paused = false
	get_tree().change_scene_to_file(MAIN_SCENE)


## Boots the simulation scene with no arboretum choice (env vars / defaults), used by the debug hooks.
func start_simulation_direct() -> void:
	get_tree().change_scene_to_file(MAIN_SCENE)


func take_pending()-> Dictionary:
	var p: Dictionary = pending
	pending = {}
	return p


func go_title() -> void:
	get_tree().paused = false
	get_tree().change_scene_to_file(TITLE_SCENE)


func quit_game() -> void:
	get_tree().quit()


# --------------------------------------------- art ---------------------------------------------

## Optional painted image `godot/art/<name>.(png|jpg|jpeg|webp)`, loaded at runtime without import.
## Returns null when absent (callers then use their procedural version).
func load_art(art_name: String) -> Texture2D:
	if _art_cache.has(art_name):
		return _art_cache[art_name]
	var tex: Texture2D = null
	for ext: String in ["png", "jpg", "jpeg", "webp"]:
		var path: String = ProjectSettings.globalize_path("res://art/%s.%s" % [art_name, ext])
		if FileAccess.file_exists(path):
			var img: Image = Image.load_from_file(path)
			if img != null and not img.is_empty():
				img.generate_mipmaps()
				tex = ImageTexture.create_from_image(img)
				break
	_art_cache[art_name] = tex
	return tex
