class_name UiSelftest
extends RefCounted
## Self-test of the shell screens (title, arboretum setup, plates, options, pause, settings round-trip, HUD fit).
## Called from main.gd's self-test with its `_check(name, ok, detail)`; builds nodes only, never reads pixels
## (the headless renderer is a dummy).

static func run(host: Node, check: Callable) -> void:
	var tree: SceneTree = host.get_tree()

	# Title screen: the menu entries, the "à venir" ones disabled.
	var title: TitleScreen = TitleScreen.new()
	title.embedded = true
	host.add_child(title)
	await tree.process_frame
	var disabled: int = 0
	for b: Button in title.menu_buttons:
		if b.disabled:
			disabled += 1
	var off: int = 0
	for e: Array in TitleScreen.MENU_ENTRIES:
		off += 0 if bool(e[2]) else 1
	check.call("title: one button per menu entry", title.menu_buttons.size() == TitleScreen.MENU_ENTRIES.size(), str(title.menu_buttons.size()))
	check.call("title: entries à venir disabled", disabled == off, "%d of %d" % [disabled, off])
	var charger: bool = false
	for b: Button in title.menu_buttons:
		charger = charger or (b.text == "Charger" and not b.disabled)
	check.call("title: Charger entry enabled", charger)
	check.call("title: first entry is Arboretum and enabled", title.menu_buttons[0].text == "Arboretum" and not title.menu_buttons[0].disabled)
	title.show_setup()
	await tree.process_frame
	check.call("title: setup page opens", title.page == "setup" and title.setup != null)
	title.show_options()
	await tree.process_frame
	check.call("title: options page opens", title.page == "options" and title.options != null)
	title.show_load()
	await tree.process_frame
	check.call("title: load page opens", title.page == "load" and title.saves != null and title.saves.mode == "load")
	title.show_menu()
	check.call("title: back to menu", title.page == "menu")
	title.queue_free()

	# Arboretum setup: every registry species (grouped menu), 4 soils, 4 start ages, seed parsing, launch signal.
	var reg: TreeSim = TreeSim.new()
	var n_sp: int = reg.get_species_ids().size()
	var cherry_i: int = reg.find_species("prunus_avium")
	var setup: ArboretumSetup = ArboretumSetup.new()
	host.add_child(setup)
	await tree.process_frame
	var n_items: int = 0
	var n_sep: int = 0
	for k: int in setup.species_menu.item_count:
		if setup.species_menu.is_item_separator(k):
			n_sep += 1
		else:
			n_items += 1
	check.call("setup: lists every registry species", n_items == n_sp and n_sp >= 4, "%d items, %d species" % [n_items, n_sp])
	check.call("setup: grouped by category (headers)", n_sep >= 2 and n_sep <= 3, "%d headers" % n_sep)
	check.call("setup: default species is the oak", setup.species == reg.find_species("quercus_robur"))
	var ci: int = setup.species_menu.get_item_index(cherry_i)
	check.call("setup: species named from the core", setup.species_menu.get_item_text(ci) == "Merisier", setup.species_menu.get_item_text(ci))
	check.call("setup: lists 4 soils", setup.soil_seg.buttons.size() == 4, str(setup.soil_seg.buttons.size()))
	check.call("setup: 4 start ages", setup.age_seg.buttons.size() == 4)
	check.call("setup: empty seed = random (-1)", setup.chosen_seed() == -1)
	var got: Array = []
	setup.launched.connect(func(sp: int, so: int, sd: int, yr: int) -> void: got.append([sp, so, sd, yr]))
	setup.species_menu.select(ci)
	setup.species_menu.item_selected.emit(ci)
	setup.soil_seg.buttons[2].pressed.emit()
	setup.age_seg.buttons[3].pressed.emit()
	setup.seed_edit.text = "4242"
	setup.plant_button.pressed.emit()
	check.call("setup: launch carries the choice", got.size() == 1 and got[0] == [cherry_i, 2, 4242, 100], str(got))
	setup.queue_free()

	# Plates: the two hand-drawn plates build and lay out; other species get a generic card, no plate.
	for p: int in PlateView.PLATE_IDS.size():
		var i: int = reg.find_species(PlateView.PLATE_IDS[p])
		var pv: PlateView = PlateView.new()
		host.add_child(pv)
		pv.size = Vector2(1280, 720)
		pv.set_species(i)
		await tree.process_frame
		await tree.process_frame
		check.call("plate %d builds and is laid out" % p, pv.species == p and pv._s > 0.3 and PlateView.species_data(i)["latin"] != "", "scale %.2f" % pv._s)
		pv.queue_free()
	var spruce_i: int = reg.find_species("picea_abies")
	check.call("plate: a species without plate gets a generic card", spruce_i < 0 or (not PlateView.has_plate(spruce_i)
		and PlateView.species_data(spruce_i)["latin"] == "Picea abies"))
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
	check.call("pause: overlay opens with 7 entries", pm != null and pm.buttons.size() == 7, str(pm.buttons.size() if pm != null else -1))
	if pm != null:
		pm._on_entry("save")
		await tree.process_frame
		var psb: SaveBrowser = pm.get_node_or_null("Control/PanelContainer/SaveBrowser") as SaveBrowser
		if psb == null:
			psb = pm.find_child("SaveBrowser", true, false) as SaveBrowser
		check.call("pause: Sauvegarder opens the save page with a default name", psb != null and psb.mode == "save"
			and psb.name_edit.text.contains(" an"), psb.name_edit.text if psb != null else "none")
		pm._show_menu_again()
		await tree.process_frame
	if pm != null:
		pm.close()
		await tree.process_frame
	check.call("pause: Reprendre closes it", host.get_node_or_null("PauseMenu") == null or (host.get_node("PauseMenu") as Node).is_queued_for_deletion())

	await _save_load_checks(tree, check)

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

	# Side panels: rings + pruning + flux open together dock in one column without overlapping or covering the bar.
	if pt != null and fp != null:
		var view: Vector2 = host.get_viewport().get_visible_rect().size
		hud_node.rings_button.button_pressed = true
		hud_node.flux_button.button_pressed = true
		pt.set_active(true)
		await tree.process_frame
		await tree.process_frame
		var fr: Rect2 = hud_node.free_rect(view)
		var rects: Array[Rect2] = hud_node.side_panel_rects()
		var bar_y: float = hud_node._bottom_panel.global_position.y
		var overlap: bool = false
		var out_of_view: bool = false
		for i: int in rects.size():
			out_of_view = out_of_view or rects[i].position.x < 0.0 or rects[i].end.x > view.x or rects[i].position.y < 0.0 or rects[i].end.y > bar_y
			for j: int in range(i + 1, rects.size()):
				overlap = overlap or rects[i].grow(-0.5).intersects(rects[j].grow(-0.5))
		check.call("side panels: rings + taille + flux all docked (3 rects)", rects.size() == 3, str(rects))
		check.call("side panels: no overlap between panels", not overlap, str(rects))
		check.call("side panels: inside the view and above the control bar", not out_of_view, "%s bar_y=%.0f" % [str(rects), bar_y])
		var left_most: float = view.x
		for r: Rect2 in rects:
			left_most = minf(left_most, r.position.x)
		check.call("side panels: free area stops before the column", fr.end.x <= left_most, "%.0f <= %.0f" % [fr.end.x, left_most])
		check.call("flux panel folds when there is no room", fp.is_collapsed() or rects[2].end.y <= bar_y)
		fp.set_user_collapsed(true)
		await tree.process_frame
		check.call("flux panel: click header folds it to a strip", fp.is_collapsed() and not fp._body.visible and fp._strip.visible)
		fp.set_user_collapsed(false)
		# The mechanics panel joins the column: still no overlap, nothing over the bar (it folds or leaves).
		var mp: MechPanel = host.get("mech_panel") as MechPanel
		if mp != null:
			hud_node.mech_button.button_pressed = true
			await tree.process_frame
			await tree.process_frame
			hud_node.free_rect(view)
			var rects4: Array[Rect2] = hud_node.side_panel_rects()
			var bad: bool = false
			for i: int in rects4.size():
				bad = bad or rects4[i].end.y > bar_y + 0.5 or rects4[i].position.x < 0.0
				for j: int in range(i + 1, rects4.size()):
					bad = bad or rects4[i].grow(-0.5).intersects(rects4[j].grow(-0.5))
			check.call("side panels: + mechanics, still docked without overlap above the bar", not bad and (rects4.size() == 4 or mp.docked_out),
				"%s bar_y=%.0f" % [str(rects4), bar_y])
			mp.set_user_collapsed(true)
			await tree.process_frame
			check.call("mechanics panel: header folds it to slider + summary", mp.is_collapsed() and not mp._body.visible and mp._summary.visible)
			mp.set_user_collapsed(false)
			hud_node.mech_button.button_pressed = false

		# Ghost framing: with the control tree shown, both trees are inside the free area; hidden, framing returns.
		var cam_node: OrbitCamera = host.get("cam") as OrbitCamera
		pt.run_op("top", 0.45)
		pt.set_ghost(false)
		for i: int in 3:
			await tree.process_frame  # the topped tree's stats refresh
		host.call("_frame_camera", 0.0)
		var dist_live: float = cam_node.goal_distance
		pt.set_ghost(true)
		await pt.prepare_shot()
		host.call("_frame_camera", 0.0)
		cam_node.snap()
		var gr: Rect2 = hud_node.free_rect(view)
		gr = gr.grow(maxf(4.0, 0.02 * gr.size.x))
		var gcam: Camera3D = cam_node.get_camera()
		var ge: Vector2 = pt.ghost_extent
		var ghost_in: bool = ge.y > 0.0
		var gdetail: String = "extent %s free %s" % [str(ge), str(gr)]
		for p: Vector3 in [Vector3(0.0, ge.y, 0.0), Vector3(ge.x, 0.7 * ge.y, 0.0), Vector3(-ge.x, 0.7 * ge.y, 0.0), Vector3(0.0, 0.7 * ge.y, ge.x), Vector3(0.0, 0.7 * ge.y, -ge.x)]:
			ghost_in = ghost_in and gr.has_point(gcam.unproject_position(p))
		check.call("ghost: control tree measured", ge.y > 0.0, gdetail)
		check.call("ghost: shown -> whole ghost inside the free area", ghost_in, gdetail)
		var dist_ghost: float = cam_node.goal_distance
		pt.set_ghost(false)
		host.call("_frame_camera", 0.0)
		check.call("ghost: hidden -> framing returns to the live tree", cam_node.goal_distance < dist_ghost and absf(cam_node.goal_distance - dist_live) < 0.05 * dist_live + 0.05,
			"live %.1f ghost %.1f now %.1f" % [dist_live, dist_ghost, cam_node.goal_distance])
		pt.undo()
		pt.set_active(false)
		hud_node.rings_button.button_pressed = false
		await tree.process_frame


static func _leaf_ok() -> bool:
	var ts: TreeSim = TreeSim.new()
	ts.set_species(ts.find_species("quercus_robur"))
	ts.reset(1)
	var oak: bool = ts.get_leaf_lobes() > 0
	ts.set_species(ts.find_species("prunus_avium"))
	ts.reset(1)
	var cherry: bool = ts.get_leaf_lobes() == 0 and ts.get_leaf_serration() > 0.0
	return oak and cherry


## Save -> load by replay -> same state hash; listing, overwrite name, deletion, clean failures. Uses a
## scratch folder (the player's saves are untouched).
static func _save_load_checks(tree: SceneTree, check: Callable) -> void:
	var dir: String = ProjectSettings.globalize_path("user://saves_selftest")
	DirAccess.make_dir_recursive_absolute(dir)
	for f: String in DirAccess.get_files_at(dir):
		DirAccess.remove_absolute(dir.path_join(f))
	var a: TreeSim = TreeSim.new()
	var cherry_i: int = a.find_species("prunus_avium")
	a.set_species(cherry_i)
	a.set_soil(2)
	a.reset(4321)
	a.advance_days(365.0 * 7.0 + 140.0)
	a.set_drought(true)
	a.advance_days(45.0)
	a.set_drought(false)
	a.advance_days(300.0)
	var cuts: int = a.prune_op("raise", 1.0) + a.prune_op("thin", 0.3) + a.prune_op("top", a.get_height_m() * 0.8)
	a.advance_days(200.0)
	var path: String = SaveFlow.path_for(dir, "Merisier d'essai")
	var err: String = a.save_to(path, "Merisier d'essai", SaveFlow.now_unix())
	check.call("save: written", err == "" and FileAccess.file_exists(path), err)
	var text: String = FileAccess.get_file_as_string(path)
	check.call("save: versioned RON recipe", text.contains("version: 2") and text.contains("Prunus avium") and text.contains("Drought(true)"),
		"%d bytes, %d cuts" % [text.length(), cuts])
	var b: TreeSim = TreeSim.new()
	var t0: int = Time.get_ticks_msec()
	var e2: String = b.begin_load(path)
	var polls: int = 0
	var last: float = 0.0
	var monotonic: bool = true
	while b.is_loading() and polls < 6000:
		var p: float = b.load_poll()
		monotonic = monotonic and p >= last
		last = p
		polls += 1
		await tree.process_frame
	var info: Dictionary = b.get_load_info()
	check.call("load: replay in the background with progress", e2 == "" and not b.is_loading() and monotonic and polls >= 1,
		"%d polls, %d ms" % [polls, Time.get_ticks_msec() - t0])
	check.call("load: same state hash as the saved tree", b.get_state_hash() == a.get_state_hash() and bool(info.get("match", false)),
		"%s vs %s" % [b.get_state_hash(), a.get_state_hash()])
	check.call("load: species, soil, seed, day, drought restored", b.get_species() == cherry_i and b.get_soil() == 2 and b.get_seed() == 4321
		and b.get_days_elapsed() == a.get_days_elapsed() and b.get_drought() == a.get_drought() and b.get_prune_cut_count() == a.get_prune_cut_count())
	check.call("load: the history has cuts, undo works on the loaded tree", cuts > 0 and b.prune_undo(), "%d cuts" % cuts)
	var list: Array = TreeSim.list_saves(dir)
	check.call("saves: listed with metadata", list.size() == 1 and list[0]["name"] == "Merisier d'essai" and list[0]["common"] == "Merisier"
		and float(list[0]["age"]) > 7.0, str(list))
	var sb: SaveBrowser = SaveBrowser.new("save", dir, "Merisier, 8 ans")
	tree.root.add_child(sb)
	await tree.process_frame
	check.call("save page: one row, default name", sb.rows.size() == 1 and sb.name_edit.text == "Merisier, 8 ans")
	sb._select(0)
	check.call("save page: picking a row reuses its name (overwrite target)", sb.target_path() == path, sb.target_path())
	var asked: Array = []
	sb.save_requested.connect(func(n: String, p: String) -> void: asked.append([n, p]))
	sb._on_action()
	check.call("save page: overwrite asks first", asked.is_empty() and sb._confirm.visible)
	sb._on_confirm_yes()
	check.call("save page: confirmed overwrite saves", asked.size() == 1 and asked[0][1] == path)
	sb.queue_free()
	var bad: String = dir.path_join("futur.ron")
	var fw: FileAccess = FileAccess.open(bad, FileAccess.WRITE)
	fw.store_string(text.replace("version: 2", "version: 99"))
	fw.close()
	var c: TreeSim = TreeSim.new()
	var e3: String = c.begin_load(bad)
	check.call("load: unknown version refused cleanly", e3.contains("v99") and not c.is_loading(), e3)
	check.call("load: missing file refused cleanly", c.begin_load(dir.path_join("absent.ron")) != "")
	var lb: SaveBrowser = SaveBrowser.new("load", dir)
	tree.root.add_child(lb)
	await tree.process_frame
	var n_bad: int = 0
	for e: Dictionary in lb.entries:
		n_bad += 0 if bool(e["ok"]) else 1
	check.call("load page: lists both, unreadable one flagged", lb.rows.size() == 2 and n_bad == 1, str(lb.entries.size()))
	for i: int in lb.entries.size():
		if not bool(lb.entries[i]["ok"]):
			lb._select(i)
	check.call("load page: an unreadable save cannot be loaded", lb.action_button.disabled and SaveBrowser.describe(lb.entries[lb.selected]).begins_with("illisible"))
	lb._on_delete()
	lb._on_confirm_yes()
	check.call("load page: delete after confirmation", TreeSim.list_saves(dir).size() == 1 and not FileAccess.file_exists(bad))
	lb.queue_free()
	TreeSim.delete_save(path)
	DirAccess.remove_absolute(dir)
