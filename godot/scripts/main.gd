extends Node3D
## Dendrologia main scene: tree meshes, soil cross-section, light, camera, HUD, simulation stepping.

const REBUILD_INTERVAL: float = 0.1       # max ~10 mesh rebuilds per second
const FRAME_BUDGET_MS: float = 8.0        # max time spent simulating per frame
const MAX_BACKLOG_SECONDS: float = 1.0    # drop simulated time beyond this (UI never freezes)

var sim: TreeSim = TreeSim.new()
var hud: Hud
var cam: OrbitCamera
var sun: DirectionalLight3D
var env: Environment

var wood_mi: MeshInstance3D
var leaf_mi: MeshInstance3D
var root_mi: MeshInstance3D
var soil: SoilBlock

var _wood_ver: int = -1
var _leaf_ver: int = -1
var _root_ver: int = -1
var _rr: int = 0
var proc_ms_sum: float = 0.0   # own _process() cost (sim + rebuild + HUD), for the bench
var proc_ms_max: float = 0.0
var rebuild_counts: Array[int] = [0, 0, 0]  # wood, leaf, root (stats / self-test)

var playing: bool = true
var speed_dps: float = 30.0
var accumulator: float = 0.0
var mesh_dirty: bool = true
var since_rebuild: float = 1.0
var seed_value: int = 1
var frame_count: int = 0
var _last_phase: int = -1


func _ready() -> void:
	seed_value = randi() % 1000000
	sim.reset(seed_value)
	_apply_window_overrides()
	_build_environment()
	_build_ground()
	_build_tree_nodes()
	cam = OrbitCamera.new()
	add_child(cam)
	hud = Hud.new()
	add_child(hud)
	hud.play_toggled.connect(func(on: bool) -> void: playing = on)
	hud.speed_changed.connect(func(d: float) -> void: speed_dps = d)
	hud.drought_toggled.connect(func(on: bool) -> void: sim.set_drought(on))
	hud.new_seed_requested.connect(_new_seed)
	hud.set_seed_text(seed_value)
	_rebuild_meshes()
	_frame_camera(0.0)
	cam.snap()
	hud.update_from(sim)
	print("[dendro] TreeSim loaded: age=%.3f y, height=%.2f m, seed=%d" % [sim.get_age_years(), sim.get_height_m(), seed_value])
	var shot: String = OS.get_environment("DENDRO_SHOT")
	if OS.get_environment("DENDRO_SELFTEST") == "1":
		_run_selftest()
	elif OS.get_environment("DENDRO_BENCH") != "":
		_run_bench(float(OS.get_environment("DENDRO_BENCH")), shot)
	elif shot != "":
		_screenshot_and_quit(shot)


func _new_seed() -> void:
	_apply_seed(randi() % 1000000)


func _apply_seed(new_seed: int) -> void:
	seed_value = new_seed
	sim.reset(seed_value)
	hud.set_seed_text(seed_value)
	hud.force_ring_refresh()
	hud.reset_drought_button()
	accumulator = 0.0
	soil.reset()
	cam.reset_view()
	_wood_ver = -1
	_leaf_ver = -1
	_root_ver = -1
	mesh_dirty = true


func _build_environment() -> void:
	var sky_mat: ProceduralSkyMaterial = ProceduralSkyMaterial.new()
	sky_mat.sky_top_color = Color(0.30, 0.52, 0.85)
	sky_mat.sky_horizon_color = Color(0.70, 0.80, 0.90)
	sky_mat.ground_horizon_color = Color(0.70, 0.80, 0.90)
	sky_mat.ground_bottom_color = Color(0.35, 0.40, 0.45)
	var sky: Sky = Sky.new()
	sky.sky_material = sky_mat
	env = Environment.new()
	env.background_mode = Environment.BG_SKY
	env.sky = sky
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color(0.62, 0.64, 0.64)  # neutral: a blue sky ambient tints shaded bark blue
	env.ambient_light_energy = 0.75
	env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	env.ssao_enabled = false
	env.sdfgi_enabled = false
	env.volumetric_fog_enabled = false
	# Light distance haze so the far ground plane melts into the horizon (no hard edge against the sky).
	env.fog_enabled = true
	env.fog_light_color = Color(0.70, 0.80, 0.90)
	env.fog_density = 0.0010
	env.fog_sky_affect = 0.0
	var we: WorldEnvironment = WorldEnvironment.new()
	we.environment = env
	add_child(we)

	sun = DirectionalLight3D.new()
	sun.shadow_enabled = true
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_2_SPLITS
	sun.directional_shadow_max_distance = 120.0
	sun.shadow_normal_bias = 1.5
	sun.light_energy = 1.2
	sun.rotation_degrees = Vector3(-50.0, 35.0, 0.0)
	add_child(sun)


func _mat(c: Color, rough: float = 0.9) -> StandardMaterial3D:
	var m: StandardMaterial3D = StandardMaterial3D.new()
	m.albedo_color = c
	m.roughness = rough
	return m


func _build_tree_nodes() -> void:
	# All textures are generated once, here (a few hundred ms at startup).
	var t0: int = Time.get_ticks_msec()
	var bark_tex: Array[ImageTexture] = ProcTextures.make_bark_textures()
	var leaf_tex: ImageTexture = ProcTextures.make_leaf_cluster_texture()
	print("[dendro] procedural textures generated in %d ms" % (Time.get_ticks_msec() - t0))
	wood_mi = MeshInstance3D.new()
	wood_mi.material_override = ProcTextures.make_bark_material(bark_tex)
	add_child(wood_mi)
	leaf_mi = MeshInstance3D.new()
	leaf_mi.material_override = ProcTextures.make_leaf_material(leaf_tex)
	add_child(leaf_mi)
	root_mi = MeshInstance3D.new()
	root_mi.material_override = ProcTextures.make_root_material(bark_tex)
	root_mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(root_mi)


## Soil block occupying z < 0 only (grass top, cut face at z = 0 with horizon bands + depth ticks).
func _build_ground() -> void:
	soil = SoilBlock.new()
	add_child(soil)


## Rebuilds the meshes whose own version changed (all of them when `force`). Outside `force` at most
## ONE mesh is rebuilt per call (round-robin, so none starves) and the three rebuilds land on different
## frames instead of one big hitch.
func _rebuild_meshes(force: bool = false) -> void:
	var vers: Array[int] = [sim.get_wood_version(), sim.get_leaf_version(), sim.get_root_version()]
	var have: Array[int] = [_wood_ver, _leaf_ver, _root_ver]
	for step: int in 3:
		var k: int = (_rr + step) % 3
		if not force and vers[k] == have[k]:
			continue
		match k:
			0:
				wood_mi.mesh = sim.build_wood_mesh()
				_wood_ver = vers[0]
			1:
				leaf_mi.mesh = sim.build_leaf_mesh()
				_leaf_ver = vers[1]
			_:
				root_mi.mesh = sim.build_root_mesh()
				_root_ver = vers[2]
		rebuild_counts[k] += 1
		if not force:
			_rr = (k + 1) % 3
			break
	mesh_dirty = false
	since_rebuild = 0.0


func _versions_changed() -> bool:
	return sim.get_wood_version() != _wood_ver or sim.get_leaf_version() != _leaf_ver \
		or sim.get_root_version() != _root_ver


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey:
		var k: InputEventKey = event
		if k.pressed and not k.echo and k.keycode == KEY_SPACE:
			hud.set_playing(not hud.is_playing())
			get_viewport().set_input_as_handled()


func _process(delta: float) -> void:
	var t_proc: int = Time.get_ticks_usec()
	frame_count += 1
	since_rebuild += delta
	if playing:
		accumulator = minf(accumulator + delta * speed_dps, speed_dps * MAX_BACKLOG_SECONDS)
		_simulate()
	if since_rebuild >= REBUILD_INTERVAL / 3.0 and _versions_changed():
		_rebuild_meshes()
	soil.fit(sim.get_root_spread_m(), sim.get_root_depth_m(), delta)
	_frame_camera(delta)
	hud.update_from(sim)
	_update_sun()
	var pm: float = float(Time.get_ticks_usec() - t_proc) / 1000.0
	proc_ms_sum += pm
	proc_ms_max = maxf(proc_ms_max, pm)


func _frame_camera(_delta: float) -> void:
	var view: Vector2 = get_viewport().get_visible_rect().size
	cam.set_free_rect(hud.free_rect(view), view)
	cam.fit_to(sim.get_height_m(), sim.get_crown_radius_m(), sim.get_root_depth_m(), sim.get_root_spread_m())


## Steps the simulation in chunks, bounded by a per-frame time budget.
func _simulate() -> void:
	var t0: int = Time.get_ticks_usec()
	while accumulator > 0.0:
		var chunk: float = minf(accumulator, 30.0)
		accumulator -= chunk
		if sim.advance_days(chunk):
			mesh_dirty = true
		if float(Time.get_ticks_usec() - t0) / 1000.0 > FRAME_BUDGET_MS:
			break


func _update_sun() -> void:
	# Gentle seasonal sun: higher in summer (day ~172), lower in winter.
	var season: float = sin(TAU * (float(sim.get_day_of_year()) - 80.0) / 365.0)
	sun.rotation_degrees.x = -(48.0 + 14.0 * season)


# ---------------------------------------------------------------------------------------------
# Debug / verification hooks (env vars, never used in normal play)
#   DENDRO_SHOT=path.png  fast-forward and save a screenshot, then quit
#   DENDRO_YEARS=60       years to fast-forward (default 40)
#   DENDRO_DOY=300        day of year to stop at (default 200 = mid-July)
#   DENDRO_CAM=leaves|low|default   camera preset
#   DENDRO_CAMDIST=1.5    distance for the "leaves" preset
#   DENDRO_RINGS=0|1      ring panel visibility in the shot (default 1)
#   DENDRO_RINGZOOM=6     ring-view zoom factor in the shot
#   DENDRO_SIZE=1920x1080 window size
#   DENDRO_MSAA=0|2|4, DENDRO_FXAA=1   anti-aliasing overrides
#   DENDRO_BENCH=8        run 8 s at 10 years/s and print frame-time stats (with DENDRO_SHOT: also shoot)
#   DENDRO_SELFTEST=1     scripted UI self-test, prints PASS/FAIL, exits non-zero on failure
# ---------------------------------------------------------------------------------------------

func _apply_window_overrides() -> void:
	var vp: Viewport = get_viewport()
	vp.msaa_3d = Viewport.MSAA_4X  # shipping default (see project.godot too)
	var m: String = OS.get_environment("DENDRO_MSAA")
	if m != "":
		vp.msaa_3d = {"0": Viewport.MSAA_DISABLED, "2": Viewport.MSAA_2X, "4": Viewport.MSAA_4X}.get(m, Viewport.MSAA_4X)
	# MSAA 4x (edges, alpha-to-coverage leaves) + FXAA (sub-pixel twigs that MSAA cannot resolve).
	vp.screen_space_aa = Viewport.SCREEN_SPACE_AA_DISABLED if OS.get_environment("DENDRO_FXAA") == "0" else Viewport.SCREEN_SPACE_AA_FXAA
	var sz: String = OS.get_environment("DENDRO_SIZE")
	if sz != "" and DisplayServer.get_name() != "headless":
		var p: PackedStringArray = sz.split("x")
		if p.size() == 2:
			get_window().size = Vector2i(int(p[0]), int(p[1]))


func _advance_days_direct(days: float) -> void:
	while days > 0.0:
		var c: float = minf(days, 30.0)
		sim.advance_days(c)
		days -= c


func _screenshot_and_quit(path: String) -> void:
	playing = false
	hud.set_playing(false)
	hud.set_rings_visible(OS.get_environment("DENDRO_RINGS") != "0")
	var years_env: String = OS.get_environment("DENDRO_YEARS")
	var years: float = float(years_env) if years_env != "" else 40.0
	var doy_env: String = OS.get_environment("DENDRO_DOY")
	var doy: float = float(doy_env) if doy_env != "" else 200.0
	_advance_days_direct(years * 365.0 + doy - 1.0)
	_rebuild_meshes(true)
	for i: int in 4:
		await get_tree().process_frame
	_apply_camera_preset()
	_frame_camera(0.0)
	cam.snap()
	hud.update_from(sim)
	var rz: String = OS.get_environment("DENDRO_RINGZOOM")
	if rz != "":
		var rv: RingView = hud.get_ring_view()
		rv._zoom_at(Vector2(rv.size.x * 0.5 + 40.0, (rv.size.y - RingView.TEXT_H) * 0.5 - 10.0), float(rz))
	for i: int in 12:
		await get_tree().process_frame
	if DisplayServer.get_name() == "headless":
		print("[dendro] headless: screenshot skipped")
	else:
		var img: Image = get_viewport().get_texture().get_image()
		DirAccess.make_dir_recursive_absolute(path.get_base_dir())
		var err: int = img.save_png(path)
		print("[dendro] screenshot saved (%d, %dx%d) -> %s" % [err, img.get_width(), img.get_height(), path])
	get_tree().quit()


func _apply_camera_preset() -> void:
	var preset: String = OS.get_environment("DENDRO_CAM")
	if preset == "leaves" and leaf_mi.mesh != null:
		var bb: AABB = leaf_mi.mesh.get_aabb()
		cam.use_override = true
		cam.override_point = Vector3(bb.position.x + bb.size.x * 0.5, bb.position.y + bb.size.y * 0.62,
			bb.end.z - bb.size.z * 0.3)
		var d: String = OS.get_environment("DENDRO_CAMDIST")
		cam.override_dist = float(d) if d != "" else 1.6
		cam.yaw = 0.0
		cam.pitch = 0.1
	elif preset == "trunk":
		cam.use_override = true
		cam.override_point = Vector3(0.0, 1.3, 0.0)
		cam.override_dist = 1.4
		cam.yaw = 0.0
		cam.pitch = 0.05
	elif preset == "low":
		cam.pitch = -0.5  # clamped by the ground-level rule
		cam.yaw = 0.5


## Fast-forward at max speed (10 years/s) through the normal _process path and report frame times.
func _run_bench(seconds: float, shot: String) -> void:
	hud.set_rings_visible(true)
	hud.speed_slider.value = 1.0
	hud.set_playing(true)
	var vp_rid: RID = get_viewport().get_viewport_rid()
	RenderingServer.viewport_set_measure_render_time(vp_rid, true)
	for i: int in 30:
		await get_tree().process_frame
	var t_start: int = Time.get_ticks_usec()
	var last: int = t_start
	var n: int = 0
	var worst: float = 0.0
	var proc_sum: float = 0.0
	var gpu_sum: float = 0.0
	var cpu_sum: float = 0.0
	var counts0: Array[int] = rebuild_counts.duplicate()
	proc_ms_sum = 0.0
	proc_ms_max = 0.0
	while float(Time.get_ticks_usec() - t_start) / 1.0e6 < seconds:
		await get_tree().process_frame
		var now: int = Time.get_ticks_usec()
		var ft: float = float(now - last) / 1000.0
		last = now
		worst = maxf(worst, ft)
		proc_sum += Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0
		gpu_sum += RenderingServer.viewport_get_measured_render_time_gpu(vp_rid)
		cpu_sum += RenderingServer.viewport_get_measured_render_time_cpu(vp_rid)
		n += 1
	var wall: float = float(Time.get_ticks_usec() - t_start) / 1000.0
	print("[dendro][bench] msaa=%d frames=%d avg_frame=%.2f ms (%.1f FPS) worst=%.1f ms script_process avg=%.2f ms max=%.1f ms, avg_render_cpu=%.2f ms avg_render_gpu=%.2f ms" % [
		get_viewport().msaa_3d, n, wall / n, 1000.0 * n / wall, worst, proc_ms_sum / n, proc_ms_max, cpu_sum / n, gpu_sum / n])
	print("[dendro][bench] end age=%.1f y height=%.1f m rings=%d, mesh rebuilds (wood/leaf/root) = %d/%d/%d in %.1f s" % [
		sim.get_age_years(), sim.get_height_m(), sim.get_ring_widths().size(),
		rebuild_counts[0] - counts0[0], rebuild_counts[1] - counts0[1], rebuild_counts[2] - counts0[2], wall / 1000.0])
	if shot != "" and DisplayServer.get_name() != "headless":
		var img: Image = get_viewport().get_texture().get_image()
		DirAccess.make_dir_recursive_absolute(shot.get_base_dir())
		img.save_png(shot)
	get_tree().quit()


# ------------------------------------ self-test --------------------------------------------

var _failures: int = 0


func _check(name: String, ok: bool, detail: String = "") -> void:
	if ok:
		print("[selftest] PASS  %s %s" % [name, detail])
	else:
		_failures += 1
		print("[selftest] FAIL  %s %s" % [name, detail])


func _run_days_ui(days: float) -> void:
	accumulator = days
	while accumulator > 0.0:
		_simulate()


func _run_until_rings(count: int, max_days: int = 4000) -> void:
	var d: int = 0
	while sim.get_ring_widths().size() < count and d < max_days:
		_run_days_ui(5.0)
		d += 5


func _run_selftest() -> void:
	print("[selftest] start (display driver: %s)" % DisplayServer.get_name())
	for i: int in 3:
		await get_tree().process_frame

	# 1. Play / pause through the button and through the Space key.
	hud.play_button.button_pressed = false
	await get_tree().process_frame
	_check("pause button stops sim flag", not playing and not hud.is_playing())
	_check("pause button label", hud.play_button.text == "Lecture", hud.play_button.text)
	var age0: float = sim.get_age_years()
	for i: int in 5:
		await get_tree().process_frame
	_check("paused: age frozen", sim.get_age_years() == age0)
	var ev: InputEventKey = InputEventKey.new()
	ev.keycode = KEY_SPACE
	ev.physical_keycode = KEY_SPACE
	ev.pressed = true
	Input.parse_input_event(ev)
	await get_tree().process_frame
	_check("Space key resumes play", playing and hud.is_playing())
	hud.speed_slider.value = 0.6
	for i: int in 20:
		await get_tree().process_frame
	_check("playing: age advances", sim.get_age_years() > age0, "%.3f -> %.3f" % [age0, sim.get_age_years()])
	hud.play_button.button_pressed = false

	# 2. Speed extremes.
	hud.speed_slider.value = hud.speed_slider.min_value
	_check("speed min = 1 day/s", absf(speed_dps - 1.0) < 0.01, "%.3f" % speed_dps)
	hud.speed_slider.value = hud.speed_slider.max_value
	_check("speed max = 10 years/s", absf(speed_dps - 3650.0) < 1.0, "%.1f" % speed_dps)
	hud.speed_slider.value = 0.5

	# 3. New seed resets age, rings, soil block, drought button.
	_apply_seed(11)
	_run_days_ui(3.0 * 365.0 + 100.0)
	hud.drought_button.button_pressed = true
	var old_seed: int = seed_value
	var rings_before: int = sim.get_ring_widths().size()
	_check("grown before reseed", sim.get_age_years() > 3.0 and rings_before >= 3, "age %.1f rings %d" % [sim.get_age_years(), rings_before])
	hud.seed_button.pressed.emit()
	await get_tree().process_frame
	await get_tree().process_frame
	_check("new seed: age reset", sim.get_age_years() < 0.1, "%.3f" % sim.get_age_years())
	_check("new seed: rings cleared", sim.get_ring_widths().size() == 0)
	_check("new seed: seed changed", seed_value != old_seed)
	_check("new seed: drought released", not hud.drought_button.button_pressed)
	_check("new seed: soil block reset", is_equal_approx(soil.depth, SoilBlock.MIN_DEPTH))

	# 4. Drought produces a narrower ring (same seed with / without forced drought in year 10).
	_apply_seed(4242)
	_run_until_rings(10)
	var a_w: PackedFloat32Array = sim.get_ring_widths()
	var a_f: PackedByteArray = sim.get_ring_forced_drought()
	_apply_seed(4242)
	_run_until_rings(9)
	hud.drought_button.button_pressed = true
	_run_until_rings(10)
	hud.drought_button.button_pressed = false
	var b_w: PackedFloat32Array = sim.get_ring_widths()
	var b_f: PackedByteArray = sim.get_ring_forced_drought()
	var ok_sizes: bool = a_w.size() >= 10 and b_w.size() >= 10
	_check("rings produced", ok_sizes, "A=%d B=%d" % [a_w.size(), b_w.size()])
	if ok_sizes:
		_check("drought year: narrower ring", b_w[9] < a_w[9] * 0.95, "normal %.3f mm vs drought %.3f mm" % [a_w[9], b_w[9]])
		_check("drought year: flagged forced", b_f[9] == 1 and a_f[9] == 0)
		_check("earlier rings identical", is_equal_approx(a_w[8], b_w[8]))

	# 5. Ring panel toggle + ring view interaction.
	var view: Vector2 = get_viewport().get_visible_rect().size
	hud.rings_button.button_pressed = false
	await get_tree().process_frame
	var free_closed: Rect2 = hud.free_rect(view)
	hud.rings_button.button_pressed = true
	await get_tree().process_frame
	await get_tree().process_frame
	_check("rings button shows panel", hud.is_rings_visible())
	_check("rings panel shrinks free area", hud.free_rect(view).end.x < free_closed.end.x,
		"%.0f < %.0f" % [hud.free_rect(view).end.x, free_closed.end.x])
	var rv: RingView = hud.get_ring_view()
	var w: PackedFloat32Array = PackedFloat32Array()
	var st: PackedFloat32Array = PackedFloat32Array()
	var ef: PackedFloat32Array = PackedFloat32Array()
	for i: int in 150:
		w.append(0.5 + 1.5 * exp(-float(i) / 40.0))
		st.append(0.6 if i == 100 else 0.05)
		ef.append(0.5)
	rv.size = Vector2(360.0, 420.0)
	rv.set_data(w, st, ef, PackedByteArray())
	var g: Dictionary = rv._geometry()
	_check("ring view: centre hits an inner ring", rv._ring_at(g["c"]) == 0)
	_check("ring view: outside = none", rv._ring_at((g["c"] as Vector2) + Vector2(1000.0, 0.0)) == -1)
	var edge: float = rv._radii[149] * (g["scale"] as float)
	_check("ring view: outermost ring picked", rv._ring_at((g["c"] as Vector2) + Vector2(edge - 0.5, 0.0)) == 149)
	rv._zoom_at(g["c"], 1.25)
	_check("ring view: wheel zoom", rv.get_zoom() > 1.2, "%.2f" % rv.get_zoom())
	rv.reset_view()
	_check("ring view: reset", rv.get_zoom() == 1.0)
	await get_tree().process_frame
	hud.rings_button.button_pressed = false
	await get_tree().process_frame
	_check("rings button hides panel", not hud.is_rings_visible())

	# 6. Mesh rebuild gating: unchanged versions -> no rebuild.
	for i: int in 3:
		_rebuild_meshes()  # one stale mesh per call
	var c0: Array[int] = rebuild_counts.duplicate()
	for i: int in 3:
		_rebuild_meshes()
	_check("mesh rebuild skipped when versions unchanged", c0 == rebuild_counts, str(rebuild_counts))
	_run_days_ui(1.0)
	_check("versions reported", sim.get_wood_version() >= 0 and sim.get_leaf_version() >= 0 and sim.get_root_version() >= 0)

	# 7. Framing: crown and root zone stay inside the free area, at several ages.
	hud.rings_button.button_pressed = true
	for years: int in [3, 40, 120]:
		_apply_seed(7)
		_advance_days_direct(float(years) * 365.0 + 200.0)
		_rebuild_meshes(true)
		for i: int in 3:
			await get_tree().process_frame
		soil.fit(sim.get_root_spread_m(), sim.get_root_depth_m(), 100.0)
		_frame_camera(0.0)
		cam.snap()
		var vsize: Vector2 = get_viewport().get_visible_rect().size
		var free: Rect2 = hud.free_rect(vsize).grow(4.0)
		var camera: Camera3D = cam.get_camera()
		var h: float = sim.get_height_m()
		var cr: float = sim.get_crown_radius_m()
		var rd: float = sim.get_root_depth_m()
		var sp: float = sim.get_root_spread_m()
		var pts: Array[Vector3] = [Vector3(0, h, 0), Vector3(cr, h * 0.7, 0), Vector3(-cr, h * 0.7, 0),
			Vector3(0, -rd, 0), Vector3(sp, -rd * 0.5, 0), Vector3(-sp, -rd * 0.5, 0)]
		var all_in: bool = true
		var detail: String = "age %d h=%.1f cr=%.1f rd=%.1f sp=%.1f soil %.1fx%.1f free=%s" % [years, h, cr, rd, sp, soil.half_width * 2.0, soil.depth, str(free)]
		for p: Vector3 in pts:
			var sp2: Vector2 = camera.unproject_position(p)
			if not free.has_point(sp2):
				all_in = false
				detail += " OUT(%s->%s)" % [str(p), str(sp2)]
		_check("framing: crown+roots inside free area", all_in, detail)
		_check("soil block holds roots", soil.half_width >= 1.15 * sp and soil.depth >= rd + 0.5,
			"hw %.2f vs 1.15*spread %.2f, depth %.2f vs root %.2f" % [soil.half_width, 1.15 * sp, soil.depth, rd])
	# Soil never shrinks within one tree's life.
	_apply_seed(9)
	_advance_days_direct(60.0 * 365.0)
	soil.fit(sim.get_root_spread_m(), sim.get_root_depth_m(), 10.0)
	var d_big: float = soil.depth
	_advance_days_direct(1.0)
	soil.fit(0.0, 0.0, 1.0)
	_check("soil never shrinks", soil.depth >= d_big)

	print("[selftest] %s (%d failure%s)" % ["ALL PASSED" if _failures == 0 else "FAILED", _failures, "" if _failures == 1 else "s"])
	get_tree().quit(0 if _failures == 0 else 1)
