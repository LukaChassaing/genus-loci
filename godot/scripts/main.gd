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
	cam.fit_to(sim.get_height_m(), sim.get_crown_radius_m())
	cam.snap()
	hud.update_from(sim)
	print("[dendro] TreeSim loaded: age=%.3f y, height=%.2f m, seed=%d" % [sim.get_age_years(), sim.get_height_m(), seed_value])
	var shot: String = OS.get_environment("DENDRO_SHOT")
	if shot != "":
		_screenshot_and_quit(shot)


func _new_seed() -> void:
	seed_value = randi() % 1000000
	sim.reset(seed_value)
	hud.set_seed_text(seed_value)
	hud.force_ring_refresh()
	accumulator = 0.0
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
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.ambient_light_energy = 0.7
	env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	env.ssao_enabled = false
	env.sdfgi_enabled = false
	env.volumetric_fog_enabled = false
	var we: WorldEnvironment = WorldEnvironment.new()
	we.environment = env
	add_child(we)

	sun = DirectionalLight3D.new()
	sun.shadow_enabled = true
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_2_SPLITS
	sun.directional_shadow_max_distance = 80.0
	sun.light_energy = 1.2
	sun.rotation_degrees = Vector3(-50.0, 35.0, 0.0)
	add_child(sun)


func _mat(c: Color, rough: float = 0.9) -> StandardMaterial3D:
	var m: StandardMaterial3D = StandardMaterial3D.new()
	m.albedo_color = c
	m.roughness = rough
	return m


func _build_tree_nodes() -> void:
	wood_mi = MeshInstance3D.new()
	wood_mi.material_override = _mat(Color(0.48, 0.33, 0.20))
	add_child(wood_mi)
	leaf_mi = MeshInstance3D.new()
	var lm: StandardMaterial3D = _mat(Color.WHITE, 0.8)
	lm.vertex_color_use_as_albedo = true
	lm.cull_mode = BaseMaterial3D.CULL_DISABLED
	leaf_mi.material_override = lm
	add_child(leaf_mi)
	root_mi = MeshInstance3D.new()
	root_mi.material_override = _mat(Color(0.62, 0.46, 0.30))
	add_child(root_mi)


## Soil block occupying z < 0 only: grass top, cut face (z = 0) with horizon colour bands.
func _build_ground() -> void:
	var width: float = 30.0
	var depth: float = 8.0
	var length: float = 15.0
	var body: MeshInstance3D = MeshInstance3D.new()
	var bm: BoxMesh = BoxMesh.new()
	bm.size = Vector3(width, depth, length)
	body.mesh = bm
	body.material_override = _mat(Color(0.40, 0.29, 0.19))
	body.position = Vector3(0.0, -depth * 0.5, -length * 0.5 - 0.01)
	add_child(body)

	var top: MeshInstance3D = MeshInstance3D.new()
	var pm: PlaneMesh = PlaneMesh.new()
	pm.size = Vector2(width, length)
	top.mesh = pm
	top.material_override = _mat(Color(0.30, 0.52, 0.20))
	top.position = Vector3(0.0, 0.002, -length * 0.5)
	add_child(top)

	# Soil horizons on the cut face: [top y, bottom y, colour]
	var bands: Array = [
		[0.0, -0.25, Color(0.22, 0.14, 0.09)],  # humus / topsoil
		[-0.25, -1.4, Color(0.38, 0.26, 0.15)],
		[-1.4, -3.5, Color(0.55, 0.40, 0.24)],
		[-3.5, -depth, Color(0.62, 0.55, 0.42)],
	]
	for b: Array in bands:
		var y0: float = b[0]
		var y1: float = b[1]
		var q: MeshInstance3D = MeshInstance3D.new()
		var qm: QuadMesh = QuadMesh.new()
		qm.size = Vector2(width, y0 - y1)
		q.mesh = qm
		q.material_override = _mat(b[2] as Color, 1.0)
		q.position = Vector3(0.0, (y0 + y1) * 0.5, 0.0)
		add_child(q)


func _rebuild_meshes() -> void:
	wood_mi.mesh = sim.build_wood_mesh()
	leaf_mi.mesh = sim.build_leaf_mesh()
	root_mi.mesh = sim.build_root_mesh()
	mesh_dirty = false
	since_rebuild = 0.0


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey:
		var k: InputEventKey = event
		if k.pressed and not k.echo and k.keycode == KEY_SPACE:
			hud.set_playing(not hud.is_playing())
			get_viewport().set_input_as_handled()


func _process(delta: float) -> void:
	frame_count += 1
	since_rebuild += delta
	if playing:
		accumulator = minf(accumulator + delta * speed_dps, speed_dps * MAX_BACKLOG_SECONDS)
		_simulate()
	if mesh_dirty and since_rebuild >= REBUILD_INTERVAL:
		_rebuild_meshes()
	cam.fit_to(sim.get_height_m(), sim.get_crown_radius_m())
	hud.update_from(sim)
	_update_sun()
	if frame_count == 200:
		print("[dendro] frame 200: age=%.3f y, height=%.2f m, rings=%d" % [sim.get_age_years(), sim.get_height_m(), sim.get_ring_widths().size()])


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


## Debug hook: DENDRO_SHOT=path fast-forwards DENDRO_YEARS (default 40) to mid-July, saves a screenshot, quits.
func _screenshot_and_quit(path: String) -> void:
	playing = false
	hud.set_playing(false)
	hud.set_rings_visible(true)
	var years_env: String = OS.get_environment("DENDRO_YEARS")
	var years: float = float(years_env) if years_env != "" else 40.0
	var days: float = years * 365.0 + 200.0
	while days > 0.0:
		var c: float = minf(days, 30.0)
		sim.advance_days(c)
		days -= c
	_rebuild_meshes()
	cam.fit_to(sim.get_height_m(), sim.get_crown_radius_m())
	cam.snap()
	hud.update_from(sim)
	for i: int in 10:
		await get_tree().process_frame
	if DisplayServer.get_name() == "headless":
		print("[dendro] headless: screenshot skipped")
	else:
		var img: Image = get_viewport().get_texture().get_image()
		DirAccess.make_dir_recursive_absolute(path.get_base_dir())
		var err: int = img.save_png(path)
		print("[dendro] screenshot saved (%d) -> %s" % [err, path])
	get_tree().quit()
