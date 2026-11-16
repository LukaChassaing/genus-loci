class_name SaveFlow
extends Node
## Save / load glue of the simulation scene: quick save / load (F5 / F9), the autosave when leaving to the
## menu (one rolling slot), the loading card while a save is regrown by replay, and the verification hooks.
## Files live in user://saves/ (RON recipes written by the Rust core, see dendro_core::save).
##
## Debug / verification hooks (env vars):
##   DENDRO_SAVE=/abs/path.ron   grow DENDRO_YEARS (default 40, DENDRO_DROUGHT_YEARS, DENDRO_PRUNE apply),
##                               save there, print the state hash, quit
##   DENDRO_LOAD=/abs/path.ron   load that save (progress card), print hash / saved hash / time, quit with
##                               code 0 if they match (2 if not); with DENDRO_SHOT: screenshot of the loaded
##                               tree, or of the loading card with DENDRO_SCREEN=loading

const SAVES_DIR: String = "user://saves"
const AUTOSAVE_FILE: String = "autosave"
const QUICK_FILE: String = "rapide"

var host: Node              # main.gd
var sim: TreeSim
var overlay: LoadOverlay
var loading: bool = false
var _was_playing: bool = true
var _load_path: String = ""
var _toast: Label
var _toast_layer: CanvasLayer
var _toast_t: float = 0.0


static func saves_dir() -> String:
	return ProjectSettings.globalize_path(SAVES_DIR)


## Absolute path of the save named `save_name` in `dir` (file name made safe; "arbre" when empty).
static func path_for(dir: String, save_name: String) -> String:
	var f: String = save_name.strip_edges().validate_filename().replace(" ", "_")
	if f == "" or f.begins_with("."):
		f = "arbre"
	return dir.path_join(f.left(64) + ".ron")


## Local "16/11/2026 14:05" from Unix seconds.
static func format_date(unix: int) -> String:
	var bias: int = int(Time.get_time_zone_from_system().get("bias", 0))
	var d: Dictionary = Time.get_datetime_dict_from_unix_time(unix + bias * 60)
	return "%02d/%02d/%04d %02d:%02d" % [d["day"], d["month"], d["year"], d["hour"], d["minute"]]


## "aujourd'hui 14:05", "hier 09:12" or "16/11/2026" (save lists).
static func format_date_short(unix: int) -> String:
	var bias: int = int(Time.get_time_zone_from_system().get("bias", 0)) * 60
	var day: int = int(floor(float(unix + bias) / 86400.0))
	var today: int = int(floor(float(now_unix() + bias) / 86400.0))
	var d: Dictionary = Time.get_datetime_dict_from_unix_time(unix + bias)
	if day == today:
		return "aujourd'hui %02d:%02d" % [d["hour"], d["minute"]]
	if day == today - 1:
		return "hier %02d:%02d" % [d["hour"], d["minute"]]
	return "%02d/%02d/%04d" % [d["day"], d["month"], d["year"]]


## "Chêne pédonculé, 45 ans".
static func default_name(ts: TreeSim) -> String:
	var names: PackedStringArray = ts.get_species_common_names()
	var sp: int = ts.get_species()
	var common: String = names[sp] if sp >= 0 and sp < names.size() else "Arbre"
	var age: float = ts.get_age_years()
	if age < 1.0:
		return "%s, semis" % common
	return "%s, %d an%s" % [common, int(age), "s" if int(age) >= 2 else ""]


## Save to load at scene start: the « Charger » choice handed over by Game, else DENDRO_LOAD.
static func requested_load(cfg: Dictionary) -> String:
	if cfg.has("load"):
		return str(cfg["load"])
	return OS.get_environment("DENDRO_LOAD")


static func now_unix() -> int:
	return int(Time.get_unix_time_from_system())


## `load_path` / `load_error`: the save begin_load was given at scene start and what it returned
## ("" = fine or nothing to load).
func setup(p_host: Node, load_path: String, load_error: String) -> void:
	host = p_host
	_load_path = load_path
	sim = host.get("sim") as TreeSim
	_toast_layer = CanvasLayer.new()
	_toast_layer.layer = 65
	_toast_layer.process_mode = Node.PROCESS_MODE_ALWAYS
	add_child(_toast_layer)
	_toast = UiStyle.serif_label("", 17, UiStyle.TEXT, 400, true)
	_toast.theme = UiStyle.make_theme(17)
	var sb: StyleBoxFlat = UiStyle.box(Color(UiStyle.PANEL.r, UiStyle.PANEL.g, UiStyle.PANEL.b, 0.92), UiStyle.LINE, 1, 16.0, 8.0)
	_toast.add_theme_stylebox_override("normal", sb)
	_toast.set_anchors_and_offsets_preset(Control.PRESET_CENTER_TOP)
	_toast.grow_horizontal = Control.GROW_DIRECTION_BOTH
	_toast.offset_top = 64
	_toast.visible = false
	_toast_layer.add_child(_toast)
	process_mode = Node.PROCESS_MODE_ALWAYS
	if load_error != "":
		toast("Chargement impossible : %s" % load_error, 6.0)
		print("[dendro][save] load failed: %s" % load_error)
	elif sim.is_loading():
		_start_overlay()


func _start_overlay() -> void:
	var info: Dictionary = TreeSim.read_save_info(_load_path) if _load_path != "" else {}
	var latin: String = sim.get_species_names()[sim.get_species()]
	var common: String = sim.get_species_common_names()[sim.get_species()]
	overlay = LoadOverlay.new(common, latin, str(info.get("name", "")), float(info.get("day", 0)) / 365.0)
	add_child(overlay)
	loading = true
	_was_playing = host.get("hud").is_playing()
	host.get("hud").set_playing(false)


func _process(delta: float) -> void:
	if _toast_t > 0.0:
		_toast_t -= delta
		_toast.modulate.a = clampf(_toast_t / 0.6, 0.0, 1.0)
		if _toast_t <= 0.0:
			_toast.visible = false
	if not loading:
		return
	var p: float = sim.load_poll()
	overlay.set_progress(p)
	if sim.is_loading():
		return
	loading = false
	host.call("on_tree_loaded")
	host.get("hud").set_playing(_was_playing)
	overlay.queue_free()
	overlay = null
	var err: String = sim.get_load_error()
	var info: Dictionary = sim.get_load_info()
	if err != "":
		toast("Chargement impossible : %s" % err, 6.0)
	elif not info.is_empty():
		print("[dendro][save] loaded '%s' in %d ms: day %d, hash %s, saved %s, match %s" % [info["name"], info["millis"], info["day"],
			info["hash"], info["saved_hash"], info["match"]])
		if bool(info["match"]):
			toast("« %s » chargé." % info["name"])
		else:
			toast("« %s » chargé (le moteur a changé depuis : l'arbre a repoussé un peu autrement)." % info["name"], 6.0)
	if OS.get_environment("DENDRO_LOAD") != "":
		_finish_load_hook(info)


## While the card is up, keys must not reach the scene (Esc would pause the hidden placeholder tree).
func _input(event: InputEvent) -> void:
	if loading and event is InputEventKey:
		get_viewport().set_input_as_handled()


func toast(text: String, seconds: float = 2.5) -> void:
	if _toast == null:
		return
	_toast.text = text
	_toast.visible = true
	_toast.modulate.a = 1.0
	_toast_t = seconds


# ------------------------------------------- actions -------------------------------------------

## Save the live tree. Returns "" or an error message.
func save_as(save_name: String, path: String) -> String:
	if loading:
		return "chargement en cours"
	var err: String = sim.save_to(path, save_name, now_unix())
	if err == "":
		print("[dendro][save] saved '%s' -> %s" % [save_name, path])
	return err


func quick_save() -> void:
	if loading:
		return
	var err: String = save_as("Sauvegarde rapide", saves_dir().path_join(QUICK_FILE + ".ron"))
	toast("Sauvegarde rapide (F9 pour la recharger)." if err == "" else "Échec de la sauvegarde : %s" % err)


func quick_load() -> void:
	if loading:
		return
	var path: String = saves_dir().path_join(QUICK_FILE + ".ron")
	if not FileAccess.file_exists(path):
		toast("Aucune sauvegarde rapide (F5 pour en faire une).")
		return
	Game.load_save(path)


## Rolling autosave when leaving to the menu (never under the test hooks).
func autosave() -> void:
	if loading or Game.is_automation():
		return
	var err: String = save_as("Sauvegarde automatique", saves_dir().path_join(AUTOSAVE_FILE + ".ron"))
	if err != "":
		push_warning("autosave failed: %s" % err)


# -------------------------------------------- hooks --------------------------------------------

static func env_hook_active() -> bool:
	return OS.get_environment("DENDRO_SAVE") != "" or OS.get_environment("DENDRO_LOAD") != ""


## Runs the DENDRO_SAVE hook now; DENDRO_LOAD finishes in `_process` when the replay is done.
func run_env_hooks() -> void:
	var save_path: String = OS.get_environment("DENDRO_SAVE")
	if save_path != "":
		var years_env: String = OS.get_environment("DENDRO_YEARS")
		var years: float = float(years_env) if years_env != "" else 40.0
		var dy_env: String = OS.get_environment("DENDRO_DROUGHT_YEARS")
		var dry: float = minf(float(dy_env), years) if dy_env != "" else 0.0
		host.call("_advance_days_direct", (years - dry) * 365.0 + 120.0)
		if dry > 0.0:
			sim.set_drought(true)
			host.call("_advance_days_direct", dry * 365.0)
		(host.get("prune_tool") as PruneTool).apply_env_script()
		var t0: int = Time.get_ticks_msec()
		var err: String = save_as(OS.get_environment("DENDRO_SAVE_NAME") if OS.get_environment("DENDRO_SAVE_NAME") != "" else default_name(sim), save_path)
		print("[dendro][save] DENDRO_SAVE %s: %s in %d ms, day %d, cuts %d, hash %s" % [save_path, "ok" if err == "" else err,
			Time.get_ticks_msec() - t0, sim.get_days_elapsed(), sim.get_prune_cut_count(), sim.get_state_hash()])
		get_tree().quit(0 if err == "" else 1)
		return
	if not loading:  # DENDRO_LOAD failed before the replay started
		get_tree().quit(1)
		return
	if OS.get_environment("DENDRO_SCREEN") == "loading" and OS.get_environment("DENDRO_SHOT") != "":
		while sim.is_loading() and overlay != null and overlay.bar.value < 0.45:
			await get_tree().process_frame
		await _shot(OS.get_environment("DENDRO_SHOT"))
		sim.cancel_load()
		get_tree().quit()


func _finish_load_hook(info: Dictionary) -> void:
	if OS.get_environment("DENDRO_SCREEN") == "loading":
		return  # run_env_hooks takes the shot of the card and quits
	var shot: String = OS.get_environment("DENDRO_SHOT")
	if shot != "" and OS.get_environment("DENDRO_SCREEN") != "loading":
		host.call("_rebuild_meshes", true)
		for i: int in 4:
			await get_tree().process_frame
		host.call("_settle_view")
		(host.get("cam") as OrbitCamera).snap()
		await _shot(shot)
	get_tree().quit(0 if bool(info.get("match", false)) else 2)


func _shot(path: String) -> void:
	for i: int in 10:
		await get_tree().process_frame
	if DisplayServer.get_name() == "headless":
		print("[dendro] headless: screenshot skipped")
		return
	var img: Image = get_viewport().get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	var err: int = img.save_png(path)
	print("[dendro] screenshot saved (%d, %dx%d) -> %s" % [err, img.get_width(), img.get_height(), path])
