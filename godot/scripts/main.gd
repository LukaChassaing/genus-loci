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
var flower_mi: MeshInstance3D
var flower_mat: ShaderMaterial
var leaf_mat: ShaderMaterial
var _tex_cache: Dictionary = {}   # species look key -> {leaf, bark}
var _oak_bark: Array[ImageTexture] = []
var _oak_leaf: ImageTexture
var root_mi: MeshInstance3D
var fine_root_mi: MeshInstance3D
var fine_root_mat: ShaderMaterial
var soil: SoilBlock
var backdrop: Backdrop
var guides: Guides
var leaf_fall: GPUParticles3D
var leaf_fall_mat: StandardMaterial3D
var bark_mat: StandardMaterial3D
var root_mat: ShaderMaterial
var flux_panel: FluxPanel        # per-kind flux toggles, legend and live values (shown with Flux)
var prune_tool: PruneTool         # pruning tool « Taille » (T), wounds, control ghost
var flux: RootFlux               # root material + water / nutrient flux (dashes, motes)
var _last_water_q: int = -1
# Tree stats cached once per frame (the core getters walk the tree: crown radius is not free).
var _stats_dirty: bool = true
var tree_h: float = 0.0
var tree_crown_r: float = 0.0
var tree_root_depth: float = 0.0
var _litter_smooth: float = 0.0
var _fresh_smooth: float = 0.5
var _vigor_smooth: float = 1.0

var _wood_ver: int = -1
var _leaf_ver: int = -1
var _flower_ver: int = -1
var _bark_fissures: float = 0.85
var _root_ver: int = -1
var _rr: int = 0
var proc_ms_sum: float = 0.0   # own _process() cost (sim + rebuild + HUD), for the bench
var proc_ms_max: float = 0.0
var rebuild_counts: Array[int] = [0, 0, 0, 0]  # wood, leaf, root, flower (stats / self-test)
var envelope_ms: float = 0.0   # time spent turning the root envelope into the soil goal (bench)
var _envelope_pending: bool = false
var _envelope_age: float = 10.0

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
	var seed_env: String = OS.get_environment("DENDRO_SEED")
	if seed_env != "":
		seed_value = int(seed_env)
	var soil_env: String = OS.get_environment("DENDRO_SOIL")
	if soil_env != "":
		sim.set_soil(int(soil_env))
	var species_env: String = OS.get_environment("DENDRO_SPECIES")
	if species_env != "":
		sim.set_species(int(species_env))
	# Arboretum choice from the title screen (the DENDRO_* variables above take precedence).
	var cfg: Dictionary = Game.take_pending()
	var start_days: float = 0.0
	if not cfg.is_empty():
		if seed_env == "" and int(cfg["seed"]) >= 0:
			seed_value = int(cfg["seed"])
		if soil_env == "":
			sim.set_soil(int(cfg["soil"]))
		if species_env == "":
			sim.set_species(int(cfg["species"]))
		if int(cfg["years"]) > 0:
			start_days = float(cfg["years"]) * 365.0 + 100.0  # start in spring of that year
	sim.reset(seed_value)
	if start_days > 0.0:
		_advance_days_direct(start_days)
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
	hud.set_soil_names(sim.get_soil_names())
	hud.select_soil(sim.get_soil())
	hud.soil_selected.connect(_on_soil_selected)
	hud.set_species_names(sim.get_species_common_names(), sim.get_species_names())
	hud.select_species(sim.get_species())
	hud.species_selected.connect(_on_species_selected)
	_apply_species_look()
	hud.soil_opacity_changed.connect(func(v: float) -> void: soil.set_opacity(v))
	hud.flux_toggled.connect(func(on: bool) -> void: flux.set_enabled(on))
	flux_panel = FluxPanel.new(flux.view)
	hud.add_child(flux_panel)
	hud.flux_toggled.connect(func(on: bool) -> void: flux_panel.visible = on)
	hud.guides_toggled.connect(func(on: bool) -> void: guides.set_shown(on))
	prune_tool = PruneTool.new()
	add_child(prune_tool)
	prune_tool.setup(self)
	guides.soil = soil
	guides.cam = cam
	cam.wedge_dir = soil.wedge_dir
	cam.soil = soil
	hud.set_seed_text(seed_value)
	_rebuild_meshes(start_days > 0.0)
	_settle_view()
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


## Changing the soil restarts the same seed on the new site, so the two trees can be compared.
func _on_soil_selected(index: int) -> void:
	sim.set_soil(index)
	_apply_seed(seed_value)


## Changing the species restarts the same seed with the other tree (same site).
func _on_species_selected(index: int) -> void:
	sim.set_species(index)
	_apply_seed(seed_value)


## Bark, leaf and info-panel look of the species now simulated (textures are generated on first use and
## cached; the oak ones are the startup default).
func _apply_species_look() -> void:
	var fis: float = sim.get_bark_fissures()
	var lent: float = sim.get_bark_lenticels()
	var lobes: int = sim.get_leaf_lobes()
	var serr: float = sim.get_leaf_serration()
	var arr: Vector4 = sim.get_leaf_arrangement()  # leaves per shoot, tip bunching, petiole ratio, droop
	var key: String = "%d|%.2f|%.2f|%.2f|%.3f|%.3f|%s" % [lobes, serr, fis, lent, sim.get_leaf_length_m(),
		sim.get_leaf_width_m(), arr]
	if not _tex_cache.has(key):
		var t0: int = Time.get_ticks_msec()
		var oak_leaf_look: bool = lobes > 0 and arr.is_equal_approx(Vector4(7.0, 0.75, 0.06, 0.15))
		if oak_leaf_look and absf(fis - 0.85) < 0.01 and lent < 0.01:
			_tex_cache[key] = {"leaf": _oak_leaf, "bark": _oak_bark}  # the startup oak textures
		else:
			_tex_cache[key] = {
			"leaf": ProcTextures.make_leaf_cluster_texture(lobes, serr, sim.get_leaf_length_m(), sim.get_leaf_width_m(),
				arr.x, arr.y, arr.z, arr.w),
			"bark": ProcTextures.make_bark_textures(fis, lent),
			}
		print("[dendro] species textures generated in %d ms" % (Time.get_ticks_msec() - t0))
	var entry: Dictionary = _tex_cache[key]
	leaf_mat.set_shader_parameter("leaf_tex", entry["leaf"])
	var bt: Array[ImageTexture] = entry["bark"]
	bark_mat.albedo_texture = bt[0]
	bark_mat.normal_texture = bt[1]
	_bark_fissures = fis
	ProcTextures.set_bark_look(bark_mat, sim.get_bark_gloss(), fis)
	hud.select_species(sim.get_species())
	hud.set_bark_color(sim.get_bark_color())


func _apply_seed(new_seed: int) -> void:
	seed_value = new_seed
	sim.reset(seed_value)
	if prune_tool != null:
		prune_tool.on_reset()
	_apply_species_look()
	hud.set_seed_text(seed_value)
	hud.force_ring_refresh()
	hud.reset_drought_button()
	accumulator = 0.0
	_last_water_q = -1
	soil.reset()
	cam.reset_view()
	_stats_dirty = true
	_litter_smooth = 0.0
	_last_water_q = -1
	_wood_ver = -1
	_leaf_ver = -1
	_flower_ver = -1
	_root_ver = -1
	mesh_dirty = true


func _build_environment() -> void:
	# No sky and no ground planes: the background is a 2-D gradient canvas (Backdrop), see BG_CANVAS.
	backdrop = Backdrop.new()
	add_child(backdrop)
	env = Environment.new()
	env.background_mode = Environment.BG_CANVAS
	env.background_canvas_max_layer = -5
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color(0.64, 0.64, 0.64)  # neutral-ish; tinted by season in _update_view_state
	env.ambient_light_energy = 0.68
	env.reflected_light_source = Environment.REFLECTION_SOURCE_DISABLED
	env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	env.ssao_enabled = false
	env.sdfgi_enabled = false
	env.volumetric_fog_enabled = false
	env.fog_enabled = false
	var we: WorldEnvironment = WorldEnvironment.new()
	we.environment = env
	add_child(we)

	sun = DirectionalLight3D.new()
	sun.shadow_enabled = true
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_2_SPLITS
	sun.directional_shadow_max_distance = 120.0
	sun.shadow_normal_bias = 1.5
	sun.light_energy = 1.2
	sun.rotation_degrees = Vector3(-50.0, 62.0, 0.0)
	add_child(sun)


func _mat(c: Color, rough: float = 0.9) -> StandardMaterial3D:
	var m: StandardMaterial3D = StandardMaterial3D.new()
	m.albedo_color = c
	m.roughness = rough
	return m


func _build_tree_nodes() -> void:
	# All textures are generated once, here (a few hundred ms at startup).
	var t0: int = Time.get_ticks_msec()
	var bark_tex: Array[ImageTexture] = ProcTextures.make_bark_textures()   # oak bark: also used for roots
	var leaf_tex: ImageTexture = ProcTextures.make_leaf_cluster_texture()  # oak leaves
	_oak_bark = bark_tex
	_oak_leaf = leaf_tex
	print("[dendro] procedural textures generated in %d ms" % (Time.get_ticks_msec() - t0))
	wood_mi = MeshInstance3D.new()
	bark_mat = ProcTextures.make_bark_material(bark_tex)
	wood_mi.material_override = bark_mat
	add_child(wood_mi)
	leaf_mi = MeshInstance3D.new()
	leaf_mat = ProcTextures.make_leaf_material(leaf_tex)
	leaf_mi.material_override = leaf_mat
	add_child(leaf_mi)
	flower_mi = MeshInstance3D.new()
	flower_mat = ProcTextures.make_flower_material(ProcTextures.make_flower_texture(), ProcTextures.make_fruit_texture())
	flower_mi.material_override = flower_mat
	flower_mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(flower_mi)
	flux = RootFlux.new(bark_tex)
	add_child(flux)
	root_mi = MeshInstance3D.new()
	root_mi.name = "Roots"
	root_mat = flux.root_mat
	root_mi.material_override = root_mat
	add_child(root_mi)
	fine_root_mi = MeshInstance3D.new()
	fine_root_mi.name = "FineRoots"
	fine_root_mat = flux.fine_root_mat
	fine_root_mi.material_override = fine_root_mat
	fine_root_mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(fine_root_mi)
	_build_leaf_fall()


## Diorama plug (soil cylinder with a wedge cut away) and the scale references.
func _build_ground() -> void:
	soil = SoilBlock.new()
	add_child(soil)
	guides = Guides.new()
	add_child(guides)


## A few falling leaves during leaf fall (GPU particles, low count).
func _build_leaf_fall() -> void:
	leaf_fall = GPUParticles3D.new()
	leaf_fall.amount = 70
	leaf_fall.lifetime = 8.0
	leaf_fall.emitting = false
	leaf_fall.local_coords = false
	leaf_fall.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	leaf_fall.visibility_aabb = AABB(Vector3(-60.0, -10.0, -60.0), Vector3(120.0, 90.0, 120.0))
	var pm: ParticleProcessMaterial = ParticleProcessMaterial.new()
	pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
	pm.emission_box_extents = Vector3(2.0, 0.5, 2.0)
	pm.direction = Vector3(0.0, -1.0, 0.0)
	pm.spread = 60.0
	pm.initial_velocity_min = 0.0
	pm.initial_velocity_max = 0.4
	pm.gravity = Vector3(0.0, -1.6, 0.0)
	pm.damping_min = 0.8
	pm.damping_max = 1.4
	pm.turbulence_enabled = true
	pm.turbulence_noise_strength = 1.3
	pm.turbulence_noise_scale = 1.6
	pm.scale_min = 0.8
	pm.scale_max = 1.4
	leaf_fall.process_material = pm
	var q: QuadMesh = QuadMesh.new()
	q.size = Vector2(0.16, 0.11)
	leaf_fall_mat = StandardMaterial3D.new()
	leaf_fall_mat.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
	leaf_fall_mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	leaf_fall_mat.roughness = 1.0
	q.material = leaf_fall_mat
	leaf_fall.draw_pass_1 = q
	add_child(leaf_fall)


## Rebuilds the meshes whose own version changed (all of them when `force`). Outside `force` at most
## ONE mesh is rebuilt per call (round-robin, so none starves) and the four rebuilds land on different
## frames instead of one big hitch.
func _rebuild_meshes(force: bool = false) -> void:
	var vers: Array[int] = [sim.get_wood_version(), sim.get_leaf_version(), sim.get_root_version(), sim.get_flower_version()]
	var have: Array[int] = [_wood_ver, _leaf_ver, _root_ver, _flower_ver]
	for step: int in 4:
		var k: int = (_rr + step) % 4
		if not force and vers[k] == have[k]:
			continue
		match k:
			0:
				wood_mi.mesh = sim.build_wood_mesh()
				_wood_ver = vers[0]
				flux.mark_geometry_dirty()
			1:
				leaf_mi.mesh = sim.build_leaf_mesh()
				_leaf_ver = vers[1]
			2:
				root_mi.mesh = sim.build_root_mesh()
				fine_root_mi.mesh = sim.build_fine_root_mesh()
				_root_ver = vers[2]
				_envelope_pending = true  # applied in _process, at most 4 times per second
				flux.set_fine_roots(fine_root_mi.mesh)
				flux.mark_geometry_dirty()
			_:
				flower_mi.mesh = sim.build_flower_mesh()
				_flower_ver = vers[3]
		rebuild_counts[k] += 1
		if not force:
			_rr = (k + 1) % 4
			break
	mesh_dirty = false
	since_rebuild = 0.0


func _versions_changed() -> bool:
	return sim.get_wood_version() != _wood_ver or sim.get_leaf_version() != _leaf_ver \
		or sim.get_root_version() != _root_ver


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey:
		var k: InputEventKey = event
		if not k.pressed or k.echo:
			return
		match k.keycode:
			KEY_ESCAPE:
				_open_pause_menu(true)
			KEY_SPACE:
				hud.set_playing(not hud.is_playing())
			KEY_X:
				hud.set_soil_opacity(SoilBlock.LOW_OPACITY if soil.opacity > 0.5 else 1.0)
			KEY_G:
				hud.guides_button.button_pressed = not hud.guides_button.button_pressed
			KEY_F:
				cam.reframe()
			KEY_R:
				cam.toggle_root_view()
			KEY_E:
				cam.toggle_ensemble()
			_:
				return
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
	_refresh_tree_stats()
	_envelope_age += delta
	if _envelope_pending and _envelope_age >= 0.25:
		_update_soil_envelope()
	soil.fit(delta)
	_update_view_state(delta)
	_frame_camera(delta)
	flux.update(sim, delta, playing, cam.distance)
	_place_flux_panel()
	hud.update_from(sim)
	guides.update_from(tree_h)
	hud.set_legend_visible(soil.xray or cam.root_view)
	var pm: float = float(Time.get_ticks_usec() - t_proc) / 1000.0
	proc_ms_sum += pm
	proc_ms_max = maxf(proc_ms_max, pm)


func _refresh_tree_stats() -> void:
	if not _stats_dirty:
		return
	_stats_dirty = false
	tree_h = sim.get_height_m()
	tree_crown_r = sim.get_crown_radius_m()
	tree_root_depth = sim.get_root_depth_m()


func _frame_camera(_delta: float) -> void:
	var view: Vector2 = get_viewport().get_visible_rect().size
	cam.set_free_rect(hud.free_rect(view), view)
	cam.fit_to(tree_h, tree_crown_r, soil.radius, soil.depth, soil.wedge_width, soil.keel_depth)


## Snaps plug, season look and camera to the current tree (start-up, screenshots, tests).
func _settle_view() -> void:
	_stats_dirty = true
	_refresh_tree_stats()
	_update_soil_envelope()
	soil.snap()
	flux.snap(sim)
	_update_view_state(100.0)
	_frame_camera(0.0)
	cam.snap()


## The soil body follows the root system (TreeSim.get_root_envelope); called on each root rebuild.
func _update_soil_envelope() -> void:
	_envelope_pending = false
	_envelope_age = 0.0
	var t0: int = Time.get_ticks_usec()
	soil.set_envelope(sim.get_root_envelope(SoilBlock.ENV_SECTORS, SoilBlock.ENV_BINS), sim.get_bedrock_depth_m())
	envelope_ms += float(Time.get_ticks_usec() - t0) / 1000.0


## Everything that follows the simulation state but is not geometry: backdrop, light, bark age, root
## clipping, soil horizons / moisture, seasonal ground, falling leaves.
func _update_view_state(delta: float) -> void:
	var doy: int = sim.get_day_of_year()
	backdrop.set_season(doy)
	var warm: float = backdrop.warmth
	env.ambient_light_color = Color(0.60, 0.64, 0.70).lerp(Color(0.68, 0.65, 0.60), warm)
	sun.light_color = Color(0.95, 0.97, 1.0).lerp(Color(1.0, 0.95, 0.86), warm)
	sun.light_energy = lerpf(1.0, 1.25, warm)
	var season: float = sin(TAU * (float(doy) - 80.0) / 365.0)
	sun.rotation_degrees.x = -(40.0 + 13.0 * season)

	# Roots are clipped at the plug (radius / depth uniforms).
	for m: ShaderMaterial in [root_mat, fine_root_mat]:
		m.set_shader_parameter("clip_radius", soil.radius)
		m.set_shader_parameter("clip_depth", soil.depth)
	flux.set_clip(soil.radius, soil.depth)

	# Bark: darker and rougher with trunk diameter, so an old trunk reads massive rather than pale.
	var age_t: float = smoothstep(8.0, 90.0, sim.get_trunk_diameter_cm())
	var rough_k: float = clampf(_bark_fissures / 0.85, 0.0, 1.0)  # smooth bark barely darkens / roughens
	bark_mat.albedo_color = Color(1.0, 1.0, 1.0).lerp(Color(0.68, 0.64, 0.60), age_t * rough_k)
	bark_mat.normal_scale = lerpf(0.35, 1.0, rough_k) * (1.0 + 1.0 * age_t * rough_k)

	# Blossom / fruit visibility (per-card hash thresholds in the shader).
	var bloom: float = sim.get_bloom_progress()
	var fl_keep: float = clampf(sim.get_flower_density() * 1.6, 0.0, 1.0) * (1.0 - 0.7 * smoothstep(0.6, 1.0, bloom))
	flower_mat.set_shader_parameter("flower_keep", fl_keep)
	flower_mat.set_shader_parameter("fruit_keep", clampf(sim.get_fruit_density() * 1.5, 0.0, 1.0))

	_update_soil_layers()

	# Seasonal ground.
	var phase: int = sim.get_phase()
	var water: float = sim.get_soil_water()
	var temp: float = sim.get_temperature_c()
	var k: float = 1.0 - exp(-3.0 * delta)
	var vigor_t: float = smoothstep(0.12, 0.42, warm)
	var fresh_t: float = 1.0 if (phase == 1 or phase == 2) else (0.3 if phase == 3 else 0.1)
	var crown_r: float = tree_crown_r
	var litter_t: float = _litter_for_day(doy) * smoothstep(0.4, 2.5, crown_r)
	_vigor_smooth = lerpf(_vigor_smooth, vigor_t, k)
	_fresh_smooth = lerpf(_fresh_smooth, fresh_t, k)
	_litter_smooth = lerpf(_litter_smooth, litter_t, k)
	var dryness: float = smoothstep(0.25, 0.04, water) * 0.85
	var frost: float = smoothstep(1.5, -3.0, temp)
	soil.set_ground(_vigor_smooth, _fresh_smooth, dryness, _litter_smooth, frost, crown_r)

	# Falling leaves.
	var h: float = tree_h
	leaf_fall.emitting = phase == 5 and crown_r > 1.5
	leaf_fall.position = Vector3(0.0, 0.72 * h, 0.0)
	leaf_fall.lifetime = clampf(0.72 * h / 1.0, 3.0, 25.0)
	(leaf_fall.process_material as ParticleProcessMaterial).emission_box_extents = Vector3(crown_r * 0.85, 0.15 * h + 0.2, crown_r * 0.85)
	leaf_fall_mat.albedo_color = sim.get_leaf_color()


## Ground litter 0..1 over the year: builds up during leaf fall (Oct-Nov), fades through winter, almost
## gone by summer.
static func _litter_for_day(doy: int) -> float:
	var d: float = float(doy)
	if d < 110.0:
		return lerpf(1.0, 0.12, smoothstep(0.0, 110.0, d))
	if d < 220.0:
		return lerpf(0.12, 0.0, smoothstep(110.0, 220.0, d))
	return smoothstep(265.0, 325.0, d)


## SOIL HORIZONS. Placeholder: fixed horizon depths, per-layer water derived from the single
## `get_soil_water()` value (surface dries first, depth stays moister), plus a drying front.
## TO USE REAL DATA (core `soil_layers`): replace the body with
##   soil.set_layers(bottom_m, kind, water)   # PackedFloat32Array, PackedInt32Array, PackedFloat32Array
## (one entry per layer, top to bottom; bottom depth in m; kind = SoilBlock.KIND_*; water 0..1), and call
##   soil.set_dry_front(depth_m)              # optional: 0 = none
func _update_soil_layers() -> void:
	var w: float = sim.get_soil_water()
	var q: int = int(w * 200.0) + sim.get_day_of_year() / 5 * 1000
	if q == _last_water_q:
		return
	_last_water_q = q
	var bottoms: PackedFloat32Array = sim.get_horizon_bottoms()
	if bottoms.size() > 8:
		bottoms.resize(8)
	if bottoms.size() > 0:
		bottoms[bottoms.size() - 1] = 99.0  # the last layer runs to the bottom of the plug
	var kinds: PackedInt32Array = sim.get_horizon_kinds()
	kinds.resize(bottoms.size())
	var waters: PackedFloat32Array = sim.get_horizon_water()
	waters.resize(bottoms.size())
	soil.set_layers(bottoms, kinds, waters)
	soil.set_dry_front(sim.get_dry_front_m())


## Steps the simulation in chunks, bounded by a per-frame time budget.
func _simulate() -> void:
	var t0: int = Time.get_ticks_usec()
	while accumulator > 0.0:
		var chunk: float = minf(accumulator, 30.0)
		accumulator -= chunk
		_stats_dirty = true
		if sim.advance_days(chunk):
			mesh_dirty = true
		if float(Time.get_ticks_usec() - t0) / 1000.0 > FRAME_BUDGET_MS:
			break


# ---------------------------------------------------------------------------------------------
# Debug / verification hooks (env vars, never used in normal play)
#   DENDRO_SHOT=path.png  fast-forward and save a screenshot, then quit
#   DENDRO_SPECIES=1      species index (0 oak, 1 wild cherry)
#   DENDRO_YEARS=60       years to fast-forward (default 40)
#   DENDRO_DOY=300        day of year to stop at (default 200 = mid-July)
#   DENDRO_CAM=leaves|flowers|trunk|low|roots|wide|default   camera preset (wide = whole block; roots = root view through the wedge;
#                         flowers = blossoms/fruit, also the fallback of leaves when the crown is bare)
#   DENDRO_SEED=n         fixed random seed (weather and phenology vary a lot between seeds)
#   DENDRO_YAW / DENDRO_PITCH   camera angles in degrees (override the preset)
#   DENDRO_XRAY=1         see-through soil (opacity SoilBlock.LOW_OPACITY);  DENDRO_GUIDES=1  scale references
#   DENDRO_SOIL_OPACITY=0..1  soil opacity;  DENDRO_FLUX=0|1  flux dashes / motes on the roots
#   DENDRO_FLUX_T=0.25    extra flux phase (dashes travelled) for motion stills
#   DENDRO_FLUX_KINDS=all|none|water,minerals,sugar,o2,co2,vapour,myco   flux kinds shown (panel toggles)
#   DENDRO_DROUGHT_YEARS=n  forced drought during the last n years before the shot
#   DENDRO_STATIC=1       with DENDRO_BENCH: no time-lapse, the camera orbits (tree at DENDRO_YEARS)
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
	vp.msaa_3d = Game.msaa_mode()  # user setting; the shipping 4x under automation hooks
	var m: String = OS.get_environment("DENDRO_MSAA")
	if m != "":
		vp.msaa_3d = {"0": Viewport.MSAA_DISABLED, "2": Viewport.MSAA_2X, "4": Viewport.MSAA_4X}.get(m, Viewport.MSAA_4X)
	# MSAA 4x (edges, alpha-to-coverage leaves) + FXAA (sub-pixel twigs that MSAA cannot resolve).
	vp.screen_space_aa = Viewport.SCREEN_SPACE_AA_DISABLED if OS.get_environment("DENDRO_FXAA") == "0" else Game.fxaa_mode()
	var sz: String = OS.get_environment("DENDRO_SIZE")
	if sz != "" and DisplayServer.get_name() != "headless":
		var p: PackedStringArray = sz.split("x")
		if p.size() == 2:
			get_window().size = Vector2i(int(p[0]), int(p[1]))


## Pause overlay (Esc). `freeze` pauses the scene tree; the DENDRO_SCREEN=pause screenshot shows it unfrozen.
func _open_pause_menu(freeze: bool) -> void:
	if get_tree().paused or get_node_or_null("PauseMenu") != null:
		return
	var pm: PauseMenu = PauseMenu.new()
	pm.name = "PauseMenu"
	pm.species = sim.get_species()
	add_child(pm)
	if freeze:
		get_tree().paused = true


func _advance_days_direct(days: float) -> void:
	while days > 0.0:
		var c: float = minf(days, 30.0)
		_stats_dirty = true
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
	var dy_env: String = OS.get_environment("DENDRO_DROUGHT_YEARS")
	var dry_years: float = minf(float(dy_env), years) if dy_env != "" else 0.0
	_advance_days_direct((years - dry_years) * 365.0)
	if dry_years > 0.0:
		sim.set_drought(true)
		_advance_days_direct(dry_years * 365.0)
	_advance_days_direct(doy - 1.0)
	_apply_view_env()
	_rebuild_meshes(true)
	for i: int in 4:
		await get_tree().process_frame
	_apply_camera_preset()
	_settle_view()
	cam.snap()
	await prune_tool.prepare_shot()  # DENDRO_GHOST / DENDRO_PRUNE_HOVER
	hud.update_from(sim)
	print("[dendro] view: h=%.1f crown_r=%.1f root_depth=%.1f spread=%.1f plug r=%.1f d=%.1f cam dist=%.1f target=%s" % [
		sim.get_height_m(), sim.get_crown_radius_m(), sim.get_root_depth_m(), sim.get_root_spread_m(),
		soil.radius, soil.depth, cam.distance, str(cam.target)])
	print("[dendro] framing extent px=%s free=%s cam=%s" % [str(cam.projected_extent()), str(hud.free_rect(get_viewport().get_visible_rect().size)), str(cam.get_camera().global_position)])
	for mi: MeshInstance3D in [root_mi, fine_root_mi]:
		if mi.mesh != null and mi.mesh.get_surface_count() > 0:
			var out_n: int = 0
			var worst: float = 0.0
			var vs: PackedVector3Array = mi.mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
			for p: Vector3 in vs:
				var o: float = soil.outside(p)
				if o > 0.02:
					out_n += 1
				worst = maxf(worst, o)
			print("[dendro] %s: %d / %d vertices outside the soil body (worst %.2f m)" % [mi.name, out_n, vs.size(), worst])
	if flower_mi.mesh != null and flower_mi.mesh.get_surface_count() > 0:
		print("[dendro] flowers: verts=%d density=%.2f bloom=%.2f fruit=%.2f keep=%.2f" % [
			(flower_mi.mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX] as PackedVector3Array).size(), sim.get_flower_density(),
			sim.get_bloom_progress(), sim.get_fruit_density(), flower_mat.get_shader_parameter("flower_keep") as float])
	var rz: String = OS.get_environment("DENDRO_RINGZOOM")
	if rz != "":
		var rv: RingView = hud.get_ring_view()
		rv._zoom_at(Vector2(rv.size.x * 0.5 + 40.0, (rv.size.y - RingView.TEXT_H) * 0.5 - 10.0), float(rz))
	if OS.get_environment("DENDRO_SCREEN") == "pause":
		_open_pause_menu(false)
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


## Flux panel: top-right corner of the free view area (usually sky; bottom-right with the ring panel) (left of the ring panel when it is shown).
func _place_flux_panel() -> void:
	if not flux_panel.visible:
		return
	var fr: Rect2 = hud.free_rect(get_viewport().get_visible_rect().size)
	flux_panel.size = flux_panel.get_combined_minimum_size()
	# With the ring panel open the top of the free area is the crown: go to the bottom corner instead.
	var y: float = fr.end.y - flux_panel.size.y if hud.is_rings_visible() else fr.position.y
	flux_panel.position = Vector2(fr.end.x - flux_panel.size.x, y)


func _apply_view_env() -> void:
	prune_tool.apply_env_script()  # DENDRO_PRUNE / DENDRO_TAILLE / DENDRO_GHOST
	if OS.get_environment("DENDRO_XRAY") == "1":
		hud.set_soil_opacity(SoilBlock.LOW_OPACITY)
	var so: String = OS.get_environment("DENDRO_SOIL_OPACITY")
	if so != "":
		hud.set_soil_opacity(float(so) / (100.0 if float(so) > 1.0 else 1.0))  # 0..1, or a percentage
	var fx: String = OS.get_environment("DENDRO_FLUX")
	if fx != "":
		hud.flux_button.button_pressed = fx != "0"
	var ft: String = OS.get_environment("DENDRO_FLUX_T")
	if ft != "":
		flux.phase += float(ft)
		for k: String in flux.view.phases:
			flux.view.phases[k] += float(ft)
	var fk: String = OS.get_environment("DENDRO_FLUX_KINDS")
	if fk != "":
		flux.view.set_kinds_from_string(fk)
		flux_panel.sync_checks()
	if OS.get_environment("DENDRO_GUIDES") == "1":
		hud.guides_button.button_pressed = true


## Close-up on an outer cluster of blossoms (or of fruit once the bloom is over), seen from outside the crown.
func _aim_at_blossoms() -> void:
	if flower_mi.mesh == null or flower_mi.mesh.get_surface_count() == 0:
		return
	var arr: Array = flower_mi.mesh.surface_get_arrays(0)
	var verts: PackedVector3Array = arr[Mesh.ARRAY_VERTEX]
	var cols: PackedColorArray = arr[Mesh.ARRAY_COLOR]
	var want_fruit: bool = sim.get_fruit_density() > 0.1 and sim.get_flower_density() < 0.2
	var best: Vector3 = Vector3.ZERO
	var best_r: float = -1.0
	var h: float = sim.get_height_m()
	for i: int in range(0, verts.size() - 3, 4):
		if (cols[i].a < 0.75) != want_fruit:
			continue
		var c: Vector3 = (verts[i] + verts[i + 2]) * 0.5
		if c.y < 0.35 * h or c.y > 0.7 * h:
			continue
		var r: float = Vector2(c.x, c.z).length()
		if r > best_r:
			best_r = r
			best = c
	if best_r < 0.0:
		return
	cam.use_override = true
	cam.override_point = best
	var d: String = OS.get_environment("DENDRO_CAMDIST")
	cam.override_dist = float(d) if d != "" else 1.2
	cam.set_angles(atan2(best.x, best.z), 0.1)


func _apply_camera_preset() -> void:
	var preset: String = OS.get_environment("DENDRO_CAM")
	if preset == "flowers" or (preset == "leaves" and leaf_mi.mesh.get_aabb().size.length() < 0.01):
		_aim_at_blossoms()  # blossoms / fruit (also a leafless cherry in bloom)
	elif preset == "leaves" and leaf_mi.mesh != null:
		var bb: AABB = leaf_mi.mesh.get_aabb()
		cam.use_override = true
		cam.override_point = Vector3(bb.position.x + bb.size.x * 0.5, bb.position.y + bb.size.y * 0.62,
			bb.end.z - bb.size.z * 0.3)
		var d: String = OS.get_environment("DENDRO_CAMDIST")
		cam.override_dist = float(d) if d != "" else 1.6
		cam.set_angles(0.0, 0.1)
	elif preset == "trunk":
		cam.use_override = true
		cam.override_point = Vector3(0.0, 1.3, 0.0)
		cam.override_dist = 1.4
		cam.set_angles(0.0, 0.05)
	elif preset == "low":
		cam.frame_from(0.5, -0.3)
	elif preset == "wide":
		cam.toggle_ensemble()
	elif preset == "roots":
		cam.toggle_root_view()
	var yw: String = OS.get_environment("DENDRO_YAW")
	var pt: String = OS.get_environment("DENDRO_PITCH")
	if yw != "" or pt != "":
		cam.set_angles(deg_to_rad(float(yw)) if yw != "" else cam.goal_yaw,
			deg_to_rad(float(pt)) if pt != "" else cam.goal_pitch)


## Fast-forward at max speed (10 years/s) through the normal _process path and report frame times.
func _run_bench(seconds: float, shot: String) -> void:
	hud.set_rings_visible(true)
	var static_bench: bool = OS.get_environment("DENDRO_STATIC") == "1"
	if static_bench:
		var by: String = OS.get_environment("DENDRO_YEARS")
		_advance_days_direct((float(by) if by != "" else 200.0) * 365.0 + 199.0)
		_apply_view_env()
		_rebuild_meshes(true)
		hud.set_playing(false)
		_settle_view()
	else:
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
	envelope_ms = 0.0
	soil.build_ms = 0.0
	var soil_builds0: int = soil.rebuild_count
	proc_ms_sum = 0.0
	proc_ms_max = 0.0
	while float(Time.get_ticks_usec() - t_start) / 1.0e6 < seconds:
		await get_tree().process_frame
		if static_bench:
			cam.set_angles(cam.yaw + 0.02, 0.1 + 0.4 * sin(float(n) * 0.01))
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
	print("[dendro][bench] soil body: %d rebuilds, %.1f ms building, %.1f ms envelope processing (total over the run)" % [
		soil.rebuild_count - soil_builds0, soil.build_ms, envelope_ms])
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


func _press_key(code: Key) -> void:
	for pressed: bool in [true, false]:
		var ev: InputEventKey = InputEventKey.new()
		ev.keycode = code
		ev.physical_keycode = code
		ev.pressed = pressed
		Input.parse_input_event(ev)


func _run_days_ui(days: float) -> void:
	accumulator = days
	while accumulator > 0.0:
		_simulate()


func _run_until_rings(count: int, max_days: int = 4000) -> void:
	var d: int = 0
	while sim.get_ring_widths().size() < count and d < max_days:
		_run_days_ui(5.0)
		d += 5


## The shown soil body (snapped) contains every cell of the core's root envelope: rim beyond the farthest
## root of each sector, underside below the deepest root at each distance.
func _check_soil_encloses_roots(label: String) -> void:
	var ns: int = SoilBlock.ENV_SECTORS
	var nb: int = SoilBlock.ENV_BINS
	var env: PackedFloat32Array = sim.get_root_envelope(ns, nb)
	var bw: float = env[0] / nb
	var worst: String = ""
	for s: int in ns:
		var a: float = PI * 0.5 - (float(s) + 0.5) / ns * TAU  # core sector centre -> soil azimuth
		for b: int in nb:
			var d: float = env[1 + s * nb + b]
			if d <= 0.0:
				continue
			var r: float = (float(b) + 0.5) * bw
			if soil.rim_radius(a) < r - 0.5 * bw:
				worst = "rim %.2f < root at %.2f (sector %d)" % [soil.rim_radius(a), r, s]
			elif soil.depth_at(r, a) < d - 0.03:
				worst = "underside %.2f above root at %.2f m (r %.2f, sector %d)" % [soil.depth_at(r, a), d, r, s]
	_check("soil body encloses the roots (%s)" % label, worst == "", worst + " r=%.1f d=%.1f spread=%.1f root depth=%.1f" % [
		soil.radius, soil.depth, sim.get_root_spread_m(), sim.get_root_depth_m()])


func _run_selftest() -> void:
	print("[selftest] start (display driver: %s)" % DisplayServer.get_name())
	get_window().size = Vector2i(1600, 1000)  # the performance-target window (headless defaults to a square one)
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
	_check("new seed: soil body back to a seedling clod", soil.depth < 0.6 and soil.radius < 1.0, "r=%.2f d=%.2f" % [soil.radius, soil.depth])

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
	for i: int in 4:
		_rebuild_meshes()  # one stale mesh per call
	var c0: Array[int] = rebuild_counts.duplicate()
	for i: int in 4:
		_rebuild_meshes()
	_check("mesh rebuild skipped when versions unchanged", c0 == rebuild_counts, str(rebuild_counts))
	_run_days_ui(1.0)
	_check("versions reported", sim.get_wood_version() >= 0 and sim.get_leaf_version() >= 0 and sim.get_root_version() >= 0)

	# 7. Framing at default angles: crown + plug inside the free area, tree fills >= 55 % of its height.
	hud.rings_button.button_pressed = false  # default layout (the ring panel narrows the free area)
	await get_tree().process_frame
	var lens_r: Array[float] = []
	var species0: int = sim.get_species()
	var combos: Array[Vector2i] = []
	for sp: int in [0, 1]:
		for yrs: int in [1, 5, 20, 40, 100, 200, 300]:
			combos.append(Vector2i(sp, yrs))
	for combo: Vector2i in combos:
		var years: int = combo.y
		sim.set_species(combo.x)
		_apply_seed(7)
		_advance_days_direct(float(years) * 365.0 + 200.0)
		_rebuild_meshes(true)
		for i: int in 3:
			await get_tree().process_frame
		cam.reset_view()
		_settle_view()
		var vsize: Vector2 = get_viewport().get_visible_rect().size
		var free_raw: Rect2 = hud.free_rect(vsize)
		# 4 px, or 2 % of the width: a wide root lens is width-bound, and its near rim points shift by ~2 % with
		# the viewport offset the solver treats as centred.
		var free: Rect2 = free_raw.grow(maxf(4.0, 0.02 * free_raw.size.x))
		var camera: Camera3D = cam.get_camera()
		var h: float = sim.get_height_m()
		var cr: float = sim.get_crown_radius_m()
		var detail: String = "age %d h=%.1f cr=%.1f plug r=%.1f d=%.1f dist=%.1f view=%s free=%s" % [years, h, cr, soil.radius, soil.depth, cam.distance, str(vsize), str(free_raw)]
		var all_in: bool = true
		var ymin_s: float = INF
		var ymax_s: float = -INF
		var xmin_s: float = INF
		var xmax_s: float = -INF
		for p: Vector3 in cam.framing_points(h, cr, soil.radius, soil.depth, soil.wedge_width, soil.keel_depth):
			var sp2: Vector2 = camera.unproject_position(p)
			xmin_s = minf(xmin_s, sp2.x)
			xmax_s = maxf(xmax_s, sp2.x)
			ymin_s = minf(ymin_s, sp2.y)
			ymax_s = maxf(ymax_s, sp2.y)
			if not free.has_point(sp2):
				all_in = false
				detail += " OUT(%s->%s)" % [str(p), str(sp2)]
		_check("framing: crown+plug inside free area", all_in, detail + " ys=[%.0f..%.0f] free=[%.0f..%.0f]" % [ymin_s, ymax_s, free_raw.position.y, free_raw.end.y])
		var y_base: float = camera.unproject_position(Vector3.ZERO).y
		var y_top: float = camera.unproject_position(Vector3(0.0, h, 0.0)).y
		var frac: float = (y_base - y_top) / free_raw.size.y
		# The default view frames the TREE (crown + trunk + the soil mass under it), not the 80 m soil disc: the
		# rim of an old tree's lens may run off-screen, so the tree must still fill the free height.
		# (A seedling is smaller than the clod it grows in: only 28 % there.)
		_check("framing: tree fills 50-98 %% of the free height (sp %d)" % combo.x, frac >= (0.5 if h >= 3.0 else 0.28) and frac <= 0.98,
			"age %d: height %.0f %%" % [years, frac * 100.0])
		_check("framing: camera outside the trunk and the soil", not cam.is_blocked(camera.global_position),
			"cam %s" % str(camera.global_position))
		# Soil body follow rules (the camera/soil were just snapped to this tree).
		_check_soil_encloses_roots("age %d" % years)
		lens_r.append(soil.radius)
		_check("root clip uniform equals soil radius", is_equal_approx(root_mat.get_shader_parameter("clip_radius") as float, soil.radius),
			"%.3f vs %.3f" % [root_mat.get_shader_parameter("clip_radius") as float, soil.radius])
		_check("root clip depth equals plug depth", is_equal_approx(root_mat.get_shader_parameter("clip_depth") as float, soil.depth))
	sim.set_species(species0)
	_check("soil body scales with the roots (1 y clod -> 200 y lens)", lens_r.size() == 14 and lens_r[5] > 4.0 * lens_r[0] and lens_r[4] > lens_r[3],
		str(lens_r))
	# Eases toward a new envelope, rebuilding only when it moved noticeably (no per-frame rebuild).
	var builds0: int = soil.rebuild_count
	for i: int in 10:
		soil.fit(1.0 / 60.0)
	_check("soil mesh not rebuilt while the envelope is unchanged", soil.rebuild_count == builds0)
	_apply_seed(9)
	_advance_days_direct(30.0 * 365.0)
	var d_small: float = soil.depth
	_update_soil_envelope()
	var frames: int = 0
	while not soil.settled() and frames < 900:
		soil.fit(1.0 / 60.0)
		frames += 1
	_check("soil eases to the grown roots in a few seconds, with few rebuilds (final shape built)", soil.settled() and soil.depth > d_small
		and soil.rebuild_count - builds0 <= 16, "%d frames, %d rebuilds, d %.2f -> %.2f" % [frames, soil.rebuild_count - builds0, d_small, soil.depth])
	soil.reset()
	_check("soil reset on new seed", soil.radius < 1.0 and soil.depth < 0.6)
	_check("wedge opens 110 degrees", is_equal_approx(rad_to_deg(soil.wedge_width), 110.0))

	# 8. Camera: free 360-degree orbit (no yaw limit), pitch below the horizon, screen-space pan, zoom.
	_apply_seed(5)
	_advance_days_direct(40.0 * 365.0 + 200.0)
	_rebuild_meshes(true)
	_settle_view()
	var yaw0: float = cam.yaw
	var mm: InputEventMouseMotion = InputEventMouseMotion.new()
	mm.position = Vector2(700.0, 400.0)
	mm.relative = Vector2(-700.0, 0.0)
	mm.button_mask = MOUSE_BUTTON_MASK_LEFT
	Input.parse_input_event(mm)
	await get_tree().process_frame
	_check("orbit: yaw is unlimited (way past the old +-1.15 rad)", cam.yaw - yaw0 > 3.0 and cam.goal_yaw == cam.yaw, "%.2f -> %.2f" % [yaw0, cam.yaw])
	cam.orbit(0.0, -300.0)
	_check("orbit: pitch can go below the horizon", cam.pitch < -0.3 and cam.pitch >= OrbitCamera.MIN_PITCH - 0.001, "%.2f" % cam.pitch)
	cam.orbit(0.0, 3000.0)
	_check("orbit: pitch clamped from above", cam.pitch <= OrbitCamera.MAX_PITCH + 0.001)
	cam.reset_view()
	cam.snap()
	var cam_before: Vector3 = cam.get_camera().global_position
	cam.zoom(0.5)
	_check("zoom takes manual control", cam.manual and cam.goal_distance < cam.distance)
	var gd: float = cam.goal_distance
	_frame_camera(0.0)
	_check("manual: auto-framing no longer overrides the zoom", is_equal_approx(cam.goal_distance, gd))
	cam.reframe()
	_frame_camera(0.0)
	_check("F re-frames (manual off, distance restored)", not cam.manual and cam.goal_distance > gd)
	cam.snap()
	var right: Vector3 = cam.get_camera().global_transform.basis.x
	var tgt0: Vector3 = cam.goal_target
	cam.pan(100.0, 0.0)
	var moved: Vector3 = cam.goal_target - tgt0
	_check("pan works in screen space (along camera right)", cam.manual and moved.length() > 0.1 and absf(moved.normalized().dot(-right)) > 0.99,
		"dot=%.3f" % moved.normalized().dot(-right))
	cam.reframe()
	_frame_camera(0.0)
	cam.snap()
	var tr: Vector2 = cam.get_camera().unproject_position(Vector3(0.0, sim.get_height_m() * 0.3, 0.0))
	cam.recentre_at(tr)
	_check("double-click on the trunk re-centres on the axis", cam.manual and absf(cam.goal_target.x) < 0.01 and absf(cam.goal_target.z) < 0.01 and cam.goal_target.y > 0.5,
		str(cam.goal_target))
	cam.reframe()
	_frame_camera(0.0)
	cam.snap()
	_check("camera position sane after re-frame", cam.get_camera().global_position.distance_to(cam_before) < 1.0)
	# Never inside the trunk or the ground, even at maximum zoom, with a low pitch, or aimed at the trunk.
	var blocked: String = ""
	for pitch_t: float in [OrbitCamera.MIN_PITCH, 0.0, 0.5]:
		for dist_t: float in [OrbitCamera.MIN_DIST, 0.5, 2.0, 8.0]:
			cam.manual = true
			cam.goal_target = Vector3(0.0, 1.0, 0.0)
			cam.target = cam.goal_target
			cam.set_angles(0.9, pitch_t)
			cam.goal_distance = dist_t
			cam.distance = dist_t
			cam.snap()
			if cam.is_blocked(cam.get_camera().global_position):
				blocked += " (pitch %.2f dist %.2f)" % [pitch_t, dist_t]
	_check("camera never ends up inside the trunk or the soil", blocked == "", blocked)
	_check("zoom limits: leaf to whole block", OrbitCamera.MIN_DIST <= 0.2 and OrbitCamera.MAX_DIST >= 3.0 * soil.radius)
	# Ensemble preset (E / DENDRO_CAM=wide): crown, full rim and keel all inside the free area.
	cam.reset_view()
	cam.toggle_ensemble()
	_frame_camera(0.0)
	cam.snap()
	var ext: Rect2 = cam.projected_extent()
	var free_e: Rect2 = hud.free_rect(get_viewport().get_visible_rect().size).grow(6.0)
	_check("ensemble preset frames everything", cam.ensemble and free_e.encloses(ext), "%s in %s" % [str(ext), str(free_e)])
	cam.toggle_ensemble()
	_check("ensemble toggles off", not cam.ensemble)

	# 9. Keys and controls: X / "Sol" slider (soil opacity), "Flux", G (Repères), R (root view), F (re-frame)
	#    through the real input path.
	_check("soil opaque by default", is_equal_approx(hud.soil_slider.value, 1.0) and soil.opacity == 1.0 and not soil.xray)
	_press_key(KEY_X)
	await get_tree().process_frame
	await get_tree().process_frame
	_check("key X: soil see-through", soil.xray and is_equal_approx(soil.opacity, SoilBlock.LOW_OPACITY)
		and is_equal_approx(hud.soil_slider.value, SoilBlock.LOW_OPACITY), "%.2f" % soil.opacity)
	_check("legend shown when the soil is see-through", hud.is_legend_visible())
	_check("see-through soil casts no shadow", soil._soil_mi.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_OFF)
	_press_key(KEY_X)
	await get_tree().process_frame
	_check("key X again: opaque", not soil.xray and soil.opacity == 1.0 and soil._soil_mi.material_override == soil._soil_mat)
	hud.soil_slider.value = 0.5
	await get_tree().process_frame
	_check("Sol slider sets the opacity (dithered material)", is_equal_approx(soil.opacity, 0.5) and (soil._soil_mi.material_override == soil._soil_dither_mat or soil._soil_mi.material_override == soil._soil_a2c_mat)
		and is_equal_approx(soil._soil_dither_mat.get_shader_parameter("opacity") as float, 0.5))
	hud.soil_slider.value = 1.0
	await get_tree().process_frame
	_check("legend hidden again", not hud.is_legend_visible())
	# Flux follows the uptake: none in winter dormancy, strong in July; the toggle hides it.
	_apply_seed(5)
	_advance_days_direct(40.0 * 365.0 + 15.0)
	flux.snap(sim)
	_check("flux: nothing in January", flux.level == 0.0 and sim.get_water_uptake_mm() == 0.0)
	_advance_days_direct(185.0)
	flux.snap(sim)
	var july: float = flux.level
	_check("flux: strong in July", july > 0.3, "level %.2f, uptake %.2f mm/day" % [july, sim.get_water_uptake_mm()])
	var up: PackedFloat32Array = sim.get_uptake_profile()
	_check("uptake profile: one triplet per soil layer", up.size() % 3 == 0 and up.size() >= 24)
	_check("flux uniforms reach the root shader", is_equal_approx(root_mat.get_shader_parameter("flux_level") as float, july)
		and (root_mat.get_shader_parameter("up_n") as int) > 0)
	_rebuild_meshes(true)
	var rm: ArrayMesh = root_mi.mesh as ArrayMesh
	_check("root mesh carries the along-root coordinate (UV2)", rm != null and rm.get_surface_count() > 0
		and (rm.surface_get_format(0) & Mesh.ARRAY_FORMAT_TEX_UV2) != 0)
	var ph0: float = flux.phase
	hud.set_playing(false)
	for i: int in 3:
		await get_tree().process_frame
	_check("flux frozen while paused", is_equal_approx(flux.phase, ph0))
	hud.flux_button.button_pressed = false
	await get_tree().process_frame
	_check("Flux button hides the flux", not flux.enabled and (root_mat.get_shader_parameter("flux_on") as float) == 0.0)
	hud.flux_button.button_pressed = true
	await get_tree().process_frame
	_check("Flux button shows it again", flux.enabled and (fine_root_mat.get_shader_parameter("flux_on") as float) == 1.0)
	# Whole-tree flux (TreeFluxView): real-unit values, paths, panel toggles.
	var fv: Dictionary = sim.get_flux()
	_check("flux: July sap and sugar flows", float(fv["xylem_l"]) > 1.0 and float(fv["phloem_down_gc"]) > float(fv["phloem_up_gc"])
		and float(fv["co2_in_g"]) > 0.0, "xylem %.1f L/j" % float(fv["xylem_l"]))
	flux.view.rebuild_geometry(sim)
	var paths: Array = sim.build_flux_geometry()
	_check("flux: path ribbons and gas sites built", (paths[0] as ArrayMesh).get_surface_count() == 1
		and (paths[1] as ArrayMesh).get_surface_count() == 1 and (paths[3] as PackedVector3Array).size() > 0)
	_check("flux panel shown with Flux", flux_panel.visible)
	flux_panel.toggle_for("sugar").button_pressed = false
	await get_tree().process_frame
	_check("flux panel: sugar toggle hides phloem dashes", not flux.view.kind_on["sugar"])
	flux_panel.toggle_for("sugar").button_pressed = true
	hud.flux_button.button_pressed = false
	await get_tree().process_frame
	_check("flux panel hidden with Flux off", not flux_panel.visible)
	hud.flux_button.button_pressed = true
	_press_key(KEY_G)
	await get_tree().process_frame
	_check("key G: Repères on", hud.guides_button.button_pressed and guides.shown)
	hud.guides_button.button_pressed = false
	await get_tree().process_frame
	_check("Repères button off", not guides.shown)
	var steps_small: Vector2 = Guides.scale_steps(5.0)
	var steps_mid: Vector2 = Guides.scale_steps(20.0)
	var steps_big: Vector2 = Guides.scale_steps(45.0)
	_check("height scale adapts (1 / 5 / 10 m)", steps_small.y == 1.0 and steps_mid.y == 5.0 and steps_big.y == 10.0)
	_press_key(KEY_R)
	await get_tree().process_frame
	await get_tree().process_frame
	_check("key R: root view on, legend shown", cam.root_view and hud.is_legend_visible())
	_check("root view looks through the wedge", absf(wrapf(cam.goal_yaw - soil.wedge_dir, -PI, PI)) < 0.01)
	_press_key(KEY_R)
	await get_tree().process_frame
	await get_tree().process_frame
	_check("key R again: back to the tree view", not cam.root_view and not hud.is_legend_visible())
	cam.zoom(0.7)
	_press_key(KEY_F)
	await get_tree().process_frame
	_check("key F: re-frame", not cam.manual)

	# 10. Season look: backdrop warm in summer / cool in winter, litter in autumn, soil layer defaults.
	_check("backdrop: summer warm, winter cool", Backdrop.warmth_for_day(172) > 0.95 and Backdrop.warmth_for_day(355) < 0.15)
	_check("litter: autumn heavy, summer gone", _litter_for_day(310) > 0.8 and _litter_for_day(200) < 0.02)
	var hb: PackedFloat32Array = sim.get_horizon_bottoms()
	var increasing: bool = hb.size() >= 2
	for i: int in range(1, hb.size()):
		increasing = increasing and hb[i] > hb[i - 1]
	_check("soil layers: horizons come from the simulation, top to bottom",
		increasing and soil.layer_count == mini(hb.size(), 8) and sim.get_horizon_kinds().size() == hb.size(), "%d horizons" % hb.size())
	sim.set_soil(1)
	_apply_seed(seed_value)
	_check("soil choice: shallow soil has rock, default has none", sim.get_bedrock_depth_m() > 0.0, "%.2f m" % sim.get_bedrock_depth_m())
	_advance_days_direct(50.0 * 365.0 + 200.0)
	_rebuild_meshes(true)
	_settle_view()
	var rock: float = sim.get_bedrock_depth_m() + SoilBlock.ROCK_SLAB
	var flat: bool = true
	for f: float in [0.0, 0.2, 0.5, 0.75]:
		for a: float in [soil.wedge_dir + PI, soil.wedge_dir + PI * 0.6]:
			flat = flat and absf(soil.depth_at(f * soil.rim_radius(a), a) - rock) < 0.02
	_check("shallow soil: flat rock bottom under the bedrock surface", flat and soil.depth <= rock + 0.2, "rock %.2f m, depth %.2f" % [rock, soil.depth])
	_check_soil_encloses_roots("shallow soil")
	sim.set_soil(0)
	_apply_seed(seed_value)
	_check("soil choice: back to deep loam", sim.get_bedrock_depth_m() < 0.0 and sim.get_age_years() < 0.1)

	# 11. Species: selector populated, cherry blooms (flower mesh), oak never does.
	_check("species selector has 2 entries", hud.species_option.item_count == 2, str(hud.species_option.item_count))
	_check("species selector shows French common names", hud.species_option.get_item_text(1) == "Merisier", hud.species_option.get_item_text(1))
	hud.species_selected.emit(1)  # same path as the OptionButton
	_check("species switch: cherry simulated", sim.get_species() == 1 and sim.get_age_years() < 0.1 and sim.get_leaf_lobes() == 0)
	_check("cherry bark look: lenticels, no fissures", sim.get_bark_lenticels() > 0.5 and sim.get_bark_fissures() < 0.5 and bark_mat.roughness < 0.7,
		"rough %.2f" % bark_mat.roughness)
	_advance_days_direct(12.0 * 365.0)
	var bloom_verts: int = 0
	var bloom_doy: int = -1
	for d: int in 140:
		sim.advance_days(1.0)
		var vc: int = sim.build_flower_mesh().get_surface_count()
		if vc > 0 and sim.get_flower_density() > 0.3:
			var arr: Array = sim.build_flower_mesh().surface_get_arrays(0)
			bloom_verts = (arr[Mesh.ARRAY_VERTEX] as PackedVector3Array).size()
			bloom_doy = sim.get_day_of_year()
			break
	_check("cherry >= 10 y: flower mesh non-empty at bloom", bloom_verts > 0 and bloom_verts <= 60000, "%d verts, doy %d" % [bloom_verts, bloom_doy])
	_rebuild_meshes(true)
	_update_view_state(1.0)
	_check("flower mesh instance shows the blossoms", flower_mi.mesh != null and flower_mi.mesh.get_surface_count() > 0
		and (flower_mat.get_shader_parameter("flower_keep") as float) > 0.0)
	hud.species_selected.emit(0)
	_check("species switch back: oak simulated", sim.get_species() == 0 and sim.get_leaf_lobes() > 0)
	_advance_days_direct(12.0 * 365.0 + 102.0)
	var oak_fl: ArrayMesh = sim.build_flower_mesh()
	_check("oak: flower mesh empty", oak_fl.get_surface_count() == 0 or (oak_fl.surface_get_arrays(0)[Mesh.ARRAY_VERTEX] as PackedVector3Array).size() == 0)
	_check("oak bark look restored", bark_mat.roughness > 0.9 and bark_mat.albedo_texture == _oak_bark[0])

	await UiSelftest.run(self, _check)  # shell screens: title, setup, plates, options, pause, settings, HUD fit

	print("[selftest] %s (%d failure%s)" % ["ALL PASSED" if _failures == 0 else "FAILED", _failures, "" if _failures == 1 else "s"])
	get_tree().quit(0 if _failures == 0 else 1)
