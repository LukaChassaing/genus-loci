class_name UiSelftest
extends RefCounted
## Self-test of the shell screens (title, arboretum setup, plates, options, pause, settings round-trip, HUD fit).
## Called from main.gd's self-test with its `_check(name, ok, detail)`; builds nodes only, never reads pixels
## (the headless renderer is a dummy).

static func run(host: Node, check: Callable) -> void:
	var tree: SceneTree = host.get_tree()

	# Title screen: 6 entries, 2 disabled ("à venir").
	var title: TitleScreen = TitleScreen.new()
	title.embedded = true
	host.add_child(title)
	await tree.process_frame
	var disabled: int = 0
	for b: Button in title.menu_buttons:
		if b.disabled:
			disabled += 1
	check.call("title: 6 menu entries", title.menu_buttons.size() == 6, str(title.menu_buttons.size()))
	check.call("title: 2 entries disabled (à venir)", disabled == 2, str(disabled))
	check.call("title: first entry is Arboretum and enabled", title.menu_buttons[0].text == "Arboretum" and not title.menu_buttons[0].disabled)
	title.show_setup()
	await tree.process_frame
	check.call("title: setup page opens", title.page == "setup" and title.setup != null)
	title.show_options()
	await tree.process_frame
	check.call("title: options page opens", title.page == "options" and title.options != null)
	title.show_menu()
	check.call("title: back to menu", title.page == "menu")
	title.queue_free()

	# Arboretum setup: 2 species, 4 soils, 4 start ages, seed parsing, launch signal.
	var setup: ArboretumSetup = ArboretumSetup.new()
	host.add_child(setup)
	await tree.process_frame
	check.call("setup: lists 2 species", setup.species_seg.buttons.size() == 2, str(setup.species_seg.buttons.size()))
	check.call("setup: species named from the core", setup.species_seg.buttons[1].text == "Merisier", setup.species_seg.buttons[1].text)
	check.call("setup: lists 4 soils", setup.soil_seg.buttons.size() == 4, str(setup.soil_seg.buttons.size()))
	check.call("setup: 4 start ages", setup.age_seg.buttons.size() == 4)
	check.call("setup: empty seed = random (-1)", setup.chosen_seed() == -1)
	var got: Array = []
	setup.launched.connect(func(sp: int, so: int, sd: int, yr: int) -> void: got.append([sp, so, sd, yr]))
	setup.species_seg.buttons[1].pressed.emit()
	setup.soil_seg.buttons[2].pressed.emit()
	setup.age_seg.buttons[3].pressed.emit()
	setup.seed_edit.text = "4242"
	setup.plant_button.pressed.emit()
	check.call("setup: launch carries the choice", got.size() == 1 and got[0] == [1, 2, 4242, 100], str(got))
	setup.queue_free()

	# Plates: both species build and lay out.
	for i: int in 2:
		var pv: PlateView = PlateView.new()
		host.add_child(pv)
		pv.size = Vector2(1280, 720)
		pv.set_species(i)
		await tree.process_frame
		await tree.process_frame
		check.call("plate %d builds and is laid out" % i, pv.species == i and pv._s > 0.3 and PlateView.species_data(i)["latin"] != "", "scale %.2f" % pv._s)
		pv.queue_free()
	check.call("plate leaf data: oak lobed, cherry serrate", _leaf_ok())

	# Settings round-trip (temporary file, the real settings are untouched).
	var saved: Dictionary = Game.settings.duplicate()
	var tmp: String = "user://settings_selftest.cfg"
	Game.settings = {"fullscreen": true, "resolution": 2, "msaa": 2, "fxaa": false, "vsync": false, "ui_scale": 1.15}
	var err: int = Game.save_settings(tmp)
	Game.settings = Game.default_settings()
	Game.load_settings(tmp)
	check.call("settings: round-trip", err == OK and Game.settings["fullscreen"] == true and Game.settings["resolution"] == 2 \
		and Game.settings["msaa"] == 2 and Game.settings["fxaa"] == false and Game.settings["vsync"] == false \
		and absf(float(Game.settings["ui_scale"]) - 1.15) < 0.001, str(Game.settings))
	check.call("settings: msaa mode follows setting only without hooks", Game.msaa_mode() == Viewport.MSAA_4X)  # hooks active here
	var op: OptionsPanel = OptionsPanel.new()
	op.apply_live = false
	op.save_path = tmp
	host.add_child(op)
	await tree.process_frame
	op.apply_choice("msaa", 0)
	op.apply_choice("vsync", 1)
	Game.settings = Game.default_settings()
	Game.load_settings(tmp)
	check.call("options panel: change is saved", Game.settings["msaa"] == 0 and Game.settings["vsync"] == true, str(Game.settings))
	op.queue_free()
	DirAccess.remove_absolute(ProjectSettings.globalize_path(tmp))
	Game.settings = saved
	check.call("art: missing image gives null (procedural fallback)", Game.load_art("no_such_backdrop") == null)

	# Pause overlay (not frozen here: the self-test keeps running).
	host.call("_open_pause_menu", false)
	await tree.process_frame
	var pm: PauseMenu = host.get_node_or_null("PauseMenu") as PauseMenu
	check.call("pause: overlay opens with 5 entries", pm != null and pm.buttons.size() == 5, str(pm.buttons.size() if pm != null else -1))
	if pm != null:
		pm.close()
		await tree.process_frame
	check.call("pause: Reprendre closes it", host.get_node_or_null("PauseMenu") == null or (host.get_node("PauseMenu") as Node).is_queued_for_deletion())

	# Pruning tool (« Taille »): hover, cut, undo, control tree, panel.
	var pt: PruneTool = host.get("prune_tool") as PruneTool
	if pt != null:
		await pt.selftest(check)

	# HUD bottom bar still fits at 1280 px.
	var hud_node: Hud = host.get("hud") as Hud
	var need: float = hud_node._bottom_panel.get_combined_minimum_size().x
	check.call("hud: bottom bar fits at 1280 px", need <= 1280.0 - 28.0, "needs %.0f px" % need)
	# Flux panel: fits beside the ring panel at 1280 x 720, readable number format.
	var fp: FluxPanel = host.get("flux_panel") as FluxPanel
	if fp != null:
		var fsz: Vector2 = fp.get_combined_minimum_size()
		check.call("flux panel: compact (< 400 x 300 px)", fsz.x < 400.0 and fsz.y < 300.0, str(fsz))
	check.call("flux panel: number format", FluxPanel.num(2705.4) == "2705" and FluxPanel.num(3.27) == "3.3"
		and FluxPanel.num(0.042) == "0.04" and FluxPanel.num(31400.0) == "31.4 k")


static func _leaf_ok() -> bool:
	var ts: TreeSim = TreeSim.new()
	ts.set_species(0)
	ts.reset(1)
	var oak: bool = ts.get_leaf_lobes() > 0
	ts.set_species(1)
	ts.reset(1)
	var cherry: bool = ts.get_leaf_lobes() == 0 and ts.get_leaf_serration() > 0.0
	return oak and cherry
