class_name OrbitCamera
extends Node3D
## Free 360-degree orbit camera around the diorama. Left-drag orbits (pitch may go below the horizon: the
## plug floats), wheel zooms, right/middle-drag pans in screen space, double-click re-centres on the
## trunk / crown / root zone. Framing (tree + plug inside the part of the window not covered by HUD
## panels) follows the tree smoothly until the user zooms or pans; F re-frames, R toggles a root view
## that looks through the wedge.

const FOV_DEG: float = 40.0
const MIN_PITCH: float = -0.62     # ~ -35 deg: look slightly from below
const MAX_PITCH: float = 1.45
const DEFAULT_YAW: float = 0.25
const DEFAULT_PITCH: float = 0.07  # slightly low: the keel under the ground reads from the default view
const ENSEMBLE_PITCH: float = 0.28
const ROOT_PITCH: float = 0.16
const MIN_DIST: float = 0.15       # a leaf fills the screen
const MAX_DIST: float = 1500.0     # the whole 80 m block and then some
const FILL: float = 0.93           # fraction of the free rect the framing box may occupy

var yaw: float = DEFAULT_YAW
var pitch: float = DEFAULT_PITCH
var distance: float = 6.0
var target: Vector3 = Vector3(0.0, 0.5, 0.0)
var shift_px: Vector2 = Vector2.ZERO
var manual: bool = false           # the user zoomed/panned: auto-follow is off until reframe()
var root_view: bool = false
## Ensemble preset: frames the whole block (crown, full rim, keel) instead of the tree.
var ensemble: bool = false
## Set by main: lets the camera keep out of the solid soil (never start or end up inside the ground).
var soil: SoilBlock
## Azimuth of the removed wedge (the root view looks along it).
var wedge_dir: float = 0.45

var goal_yaw: float = DEFAULT_YAW
var goal_pitch: float = DEFAULT_PITCH
var goal_target: Vector3 = Vector3(0.0, 0.5, 0.0)
var goal_distance: float = 6.0
var _goal_shift: Vector2 = Vector2.ZERO
var _free_rect: Rect2 = Rect2(0, 0, 1280, 720)
var _view_size: Vector2 = Vector2(1280, 720)
var _cam: Camera3D
# Tree shape used by double-click picking.
var _h: float = 1.0
var _crown_r: float = 1.0
var _plug_r: float = 2.5
var _plug_d: float = 2.0
var _wedge_w: float = 1.92
var _keel_d: float = 0.0           # keel thickness under the axis (m)
var _fit_yaw: float = DEFAULT_YAW  # angles the framing is solved at (preset angles, not the orbiting ones)
var _fit_pitch: float = DEFAULT_PITCH
var _fit_key: Array = []
var _fit_sol: Dictionary = {}
## Debug/preset override: look at `override_point` from `override_dist` (no HUD offset).
var use_override: bool = false
var override_point: Vector3 = Vector3.ZERO
var override_dist: float = 3.0


func _ready() -> void:
	_cam = Camera3D.new()
	_cam.fov = FOV_DEG
	_cam.near = 0.05
	_cam.far = 1000.0
	add_child(_cam)
	_apply()


func get_camera() -> Camera3D:
	return _cam


## Region of the (canvas-unit) viewport not covered by HUD panels; the tree is framed inside it.
func set_free_rect(free: Rect2, view_size: Vector2) -> void:
	_free_rect = free
	_view_size = view_size


## Smooth step used to blend "young tree" framing (crop the plug) into "mature tree" framing (whole plug).
static func _smooth(a: float, b: float, x: float) -> float:
	var t: float = clampf((x - a) / (b - a), 0.0, 1.0)
	return t * t * (3.0 - 2.0 * t)


## Points that must stay inside the free rect at the preset angles.
## Tree view: trunk, crown extremities and the soil mass under the crown: a ring of the rim cropped to ~1.3
## crown radii (an old tree's 40 m wide lens may run off-screen) at the ground and at the keel's flank, and the
## keel apex, so the underside is perceived. Young trees crop the plug further (the sapling would otherwise be
## tiny: ring radius / depth blend into the full plug by height). Ensemble: the whole rim and keel.
func framing_points(height_m: float, crown_r: float, plug_r: float, plug_d: float, wedge_w: float, keel_d: float = 0.0) -> Array[Vector3]:
	var pts: Array[Vector3] = []
	var f: float = _smooth(2.0, 7.0, height_m)
	var crown_w: float = maxf(crown_r, 0.45 * height_m) * 1.1 + 0.15
	var top: float = maxf(height_m, 0.8) * 1.06 + 0.2
	if root_view:
		# Root zone near the trunk, seen through the wedge: the axis column and the two cut faces up to
		# ~50 % of the radius (the plug is much wider than deep; the whole disc would be tiny).
		for which: float in [-1.0, 1.0]:
			var fa: float = wedge_dir + which * wedge_w * 0.5
			var fr: float = plug_r * 0.5  # the soil body follows the root spread: frame its inner half
			pts.append(Vector3(sin(fa) * fr, 2.0, cos(fa) * fr))
			pts.append(Vector3(sin(fa) * fr, -plug_d - 0.12 * keel_d, cos(fa) * fr))
		pts.append(Vector3(0.0, 2.2, 0.0))
		pts.append(Vector3(0.0, -plug_d - 0.12 * keel_d, 0.0))
		return pts
	var rc: float = plug_r
	var depth_c: float = plug_d
	if not ensemble:
		rc = minf(plug_r, maxf(crown_w * lerpf(1.0, 1.3, f) + 0.5 * f, 0.45 * plug_r * f))
		depth_c = lerpf(minf(plug_d, 0.30 * height_m + 0.3), plug_d, f)
	var apex: float = plug_d + keel_d
	var flank: float = depth_c + keel_d * SoilBlock.keel_shape(rc / maxf(plug_r, 0.01))
	if not ensemble:
		flank = lerpf(depth_c, flank, f)
		apex = lerpf(depth_c, apex, f)
	pts.append(Vector3(0.0, top, 0.0))
	var cr: float = maxf(crown_r, 0.2 * height_m)
	var yc: float = 0.7 * height_m
	# Crown shell: widest ring plus the shoulders (seen from below / above they are the silhouette).
	for a: float in [0.0, PI * 0.25, PI * 0.5, PI * 0.75, PI, PI * 1.25, PI * 1.5, PI * 1.75]:
		pts.append(Vector3(sin(a) * cr, yc, cos(a) * cr))
		pts.append(Vector3(sin(a) * cr * 0.8, 0.96 * height_m, cos(a) * cr * 0.8))
		pts.append(Vector3(sin(a) * cr * 0.8, 0.45 * height_m, cos(a) * cr * 0.8))
	var a0: float = wedge_dir + wedge_w * 0.5
	var a1: float = wedge_dir - wedge_w * 0.5 + TAU
	for i: int in 13:
		var a: float = lerpf(a0, a1, float(i) / 12.0)
		pts.append(Vector3(sin(a) * rc, 0.0, cos(a) * rc))
		pts.append(Vector3(sin(a) * rc, -flank, cos(a) * rc))
	pts.append(Vector3(0.0, -apex, 0.0))
	return pts


## Debug: screen rectangle (px) spanned by the framing points through the live camera.
func projected_extent() -> Rect2:
	var pts: Array[Vector3] = framing_points(_h, _crown_r, _plug_r, _plug_d, _wedge_w, _keel_d)
	var r: Rect2 = Rect2(_cam.unproject_position(pts[0]), Vector2.ZERO)
	for p: Vector3 in pts:
		r = r.expand(_cam.unproject_position(p))
	return r


## Camera basis (x right, y up, z back toward the camera) for given angles.
static func basis_for(yw: float, pt: float) -> Basis:
	var cp: float = cos(pt)
	var dir: Vector3 = Vector3(sin(yw) * cp, sin(pt), cos(yw) * cp)
	var right: Vector3 = (-dir).cross(Vector3.UP).normalized()
	var up: Vector3 = right.cross(-dir).normalized()
	return Basis(right, up, dir)


## Solves distance and target height so `pts` fill FILL of the free rect at the given angles.
## Returns {distance, target_y}.
func solve_framing(pts: Array[Vector3], yw: float, pt: float) -> Dictionary:
	var b: Basis = basis_for(yw, pt)
	var tan_half: float = tan(deg_to_rad(FOV_DEG) * 0.5)
	var fh: float = maxf(_free_rect.size.y / _view_size.y, 0.2) * FILL     # half-height fraction x2
	var fw: float = maxf(_free_rect.size.x / _view_size.y, 0.2) * FILL
	var ty: float = 0.0
	var ymin: float = INF
	var ymax: float = -INF
	for p: Vector3 in pts:
		ymin = minf(ymin, p.y)
		ymax = maxf(ymax, p.y)
	ty = (ymin + ymax) * 0.5
	var rad: float = 1.0
	for p: Vector3 in pts:
		rad = maxf(rad, (p - Vector3(0.0, ty, 0.0)).length())
	var d: float = rad * 3.5 / tan_half * 0.5 + 2.0   # starts well outside: every point in front of the camera
	for it: int in 16:
		var t: Vector3 = Vector3(0.0, ty, 0.0)
		var sy_min: float = INF
		var sy_max: float = -INF
		var sx_max: float = 0.0
		var need: float = 0.0
		for p: Vector3 in pts:
			var rel: Vector3 = p - t
			var x: float = rel.dot(b.x)
			var y: float = rel.dot(b.y)
			var z: float = rel.dot(b.z)
			var depth: float = maxf(d - z, 0.2)
			var ny: float = y / (depth * tan_half)   # 1 = half view height
			sy_min = minf(sy_min, ny)
			sy_max = maxf(sy_max, ny)
			sx_max = maxf(sx_max, absf(x) / (depth * tan_half))
			# Distance at which this point alone would touch the (centred) limits.
			need = maxf(need, maxf(absf(y) / (tan_half * fh), absf(x) / (tan_half * fw)) + z)
		# Re-centre vertically (screen y mid -> 0), then update the distance.
		var mid: float = (sy_min + sy_max) * 0.5
		ty += mid * d * tan_half / maxf(b.y.y, 0.3) * 0.9
		d = lerpf(d, maxf(need, 1.0), 0.8)
	if not is_finite(d) or not is_finite(ty):
		d = rad * 3.0 / tan_half  # degenerate input: a safe wide view rather than a camera inside the tree
		ty = (ymin + ymax) * 0.5
	return {"distance": d, "target_y": ty}


## Called every frame with the tree and plug size; sets the framing goals (unless the user took over).
func fit_to(height_m: float, crown_r: float, plug_r: float, plug_d: float, wedge_w: float = 1.92, keel_d: float = 0.0) -> void:
	_keel_d = keel_d
	_h = height_m
	_crown_r = crown_r
	_plug_r = plug_r
	_plug_d = plug_d
	_wedge_w = wedge_w
	_goal_shift = _free_rect.get_center() - _view_size * 0.5
	if use_override:
		goal_distance = override_dist
		goal_target = override_point
		_goal_shift = Vector2.ZERO
		return
	if manual:
		return
	if root_view:
		_fit_pitch = root_pitch()  # follows the soil body's flatness
		goal_pitch = _fit_pitch
	# The solve is only redone when its inputs change (it is cheap, but runs in script every frame).
	var key: Array = [snappedf(height_m, 0.01), snappedf(crown_r, 0.01), snappedf(plug_r, 0.01), snappedf(plug_d, 0.01),
		wedge_w, root_view, wedge_dir, _free_rect, _view_size, snappedf(keel_d, 0.01), ensemble, _fit_yaw, _fit_pitch]
	if key != _fit_key:
		_fit_key = key
		var pts: Array[Vector3] = framing_points(height_m, crown_r, plug_r, plug_d, wedge_w, keel_d)
		_fit_sol = solve_framing(pts, _fit_yaw, _fit_pitch)
	goal_distance = clampf(_fit_sol["distance"], 1.0, MAX_DIST)
	goal_target = Vector3(0.0, _fit_sol["target_y"], 0.0)


func snap() -> void:
	target = goal_target
	distance = goal_distance
	shift_px = _goal_shift
	yaw = goal_yaw
	pitch = goal_pitch
	_apply()


## Restores automatic framing and the default view (new seed).
func reset_view() -> void:
	manual = false
	root_view = false
	use_override = false
	ensemble = false
	_set_preset_angles(DEFAULT_YAW, DEFAULT_PITCH)


## F: re-frame the tree (or the root zone in root view) and return to the preset angles.
func reframe() -> void:
	manual = false
	use_override = false
	if root_view:
		_set_preset_angles(wedge_dir, root_pitch())
	elif ensemble:
		_set_preset_angles(DEFAULT_YAW, ENSEMBLE_PITCH)
	else:
		_set_preset_angles(DEFAULT_YAW, DEFAULT_PITCH)


## Goal angles of a preset; the framing is solved at these (not at the angles of a later free orbit).
func _set_preset_angles(yw: float, pt: float) -> void:
	goal_yaw = yw
	goal_pitch = pt
	_fit_yaw = yw
	_fit_pitch = pt


## Preset view from arbitrary angles (e.g. the "low" view from under the ground line): re-framed at them.
func frame_from(yw: float, pt: float) -> void:
	reframe()
	_set_preset_angles(yw, clampf(pt, MIN_PITCH, MAX_PITCH))


## E: whole block (crown, full rim, keel) in view.
func toggle_ensemble() -> void:
	ensemble = not ensemble
	root_view = false
	reframe()

## Root-view pitch: the soil body follows the root system, a flat lens under an old tree (much wider than
## deep) is looked at from higher up so its roots spread out instead of collapsing into a thin band.
func root_pitch() -> float:
	var aspect: float = _plug_d / maxf(_plug_r, 0.1)
	return lerpf(0.45, ROOT_PITCH, smoothstep(0.08, 0.35, aspect))


## R: toggle the root view (centred on the root zone, looking through the wedge).
func toggle_root_view() -> void:
	root_view = not root_view
	ensemble = false
	reframe()


func set_angles(new_yaw: float, new_pitch: float) -> void:
	yaw = new_yaw
	goal_yaw = new_yaw
	pitch = clampf(new_pitch, MIN_PITCH, MAX_PITCH)
	goal_pitch = pitch


func orbit(dx: float, dy: float) -> void:
	set_angles(yaw - dx * 0.006, pitch + dy * 0.006)  # no yaw limit


## Screen-space pan: moves the target along the camera's right/up vectors.
func pan(dx: float, dy: float) -> void:
	manual = true
	var basis_cam: Basis = _view_basis()
	var wpp: float = 2.0 * tan(deg_to_rad(FOV_DEG) * 0.5) * distance / _view_size.y
	goal_target += (-basis_cam.x * dx + basis_cam.y * dy) * wpp
	# Move the live target too so the picture follows the hand without lag.
	target += (-basis_cam.x * dx + basis_cam.y * dy) * wpp


func zoom(factor: float) -> void:
	manual = true
	goal_distance = clampf(goal_distance * factor, MIN_DIST, MAX_DIST)


func _process(delta: float) -> void:
	var k: float = 1.0 - exp(-4.0 * delta)
	var kr: float = 1.0 - exp(-7.0 * delta)
	target = target.lerp(goal_target, k)
	distance = lerpf(distance, goal_distance, k)
	shift_px = shift_px.lerp(_goal_shift, k)
	yaw = lerp_angle(yaw, goal_yaw, kr)
	pitch = lerpf(pitch, goal_pitch, kr)
	_apply()


func _view_basis() -> Basis:
	return basis_for(yaw, pitch)


func _apply() -> void:
	if _cam == null:
		return
	var b: Basis = _view_basis()
	var wpp: float = 2.0 * tan(deg_to_rad(FOV_DEG) * 0.5) * distance / _view_size.y
	var t: Vector3 = target + (-b.x * shift_px.x + b.y * shift_px.y) * wpp
	# Never inside the trunk or the solid soil: back off along the view ray until clear.
	var clear: float = clear_distance(t, b.z, distance)
	if clear > distance:
		distance = clear
		goal_distance = maxf(goal_distance, clear)
	_cam.global_position = t + b.z * distance
	_cam.look_at(t, Vector3.UP)
	# Depth range follows the zoom: centimetres for a leaf, the whole block when far away.
	_cam.near = clampf(distance * 0.01, 0.03, 2.0)
	_cam.far = maxf(1000.0, distance * 3.0 + _plug_r * 4.0)


## Trunk radius estimate (m) at the base (the real girth is not passed in: ~1.2 m at 33 m / 127 cm).
func trunk_radius() -> float:
	return 0.25 + 0.03 * _h


## True when `p` is inside the trunk (a generous cylinder) or in the solid soil.
func is_blocked(p: Vector3) -> bool:
	var hr: float = Vector2(p.x, p.z).length()
	if hr < trunk_radius() * 1.25 + 0.2 and p.y > -0.1 and p.y < _h:
		return true
	return soil != null and soil.solid_at(p, 0.1)


## Smallest distance >= `dist` along `dir` from `t` at which the camera position is outside the trunk and the
## solid soil (marches outward; the view ray leaves any convex obstacle once).
func clear_distance(t: Vector3, dir: Vector3, dist: float) -> float:
	var d: float = dist
	for i: int in 64:
		if not is_blocked(t + dir * d):
			return d
		d += maxf(0.1, d * 0.06)
	return d


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		var m: InputEventMouseMotion = event
		if m.button_mask & MOUSE_BUTTON_MASK_LEFT:
			orbit(m.relative.x, m.relative.y)
		elif m.button_mask & (MOUSE_BUTTON_MASK_RIGHT | MOUSE_BUTTON_MASK_MIDDLE):
			pan(m.relative.x, m.relative.y)
	elif event is InputEventMouseButton:
		var b: InputEventMouseButton = event
		if not b.pressed:
			return
		if b.button_index == MOUSE_BUTTON_WHEEL_UP:
			zoom(0.9)
		elif b.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			zoom(1.1)
		elif b.button_index == MOUSE_BUTTON_LEFT and b.double_click:
			recentre_at(b.position)


## Double-click: re-centre on the trunk, the crown or the root zone under the cursor (analytic shapes).
func recentre_at(screen_pos: Vector2) -> void:
	var o: Vector3 = _cam.project_ray_origin(screen_pos)
	var d: Vector3 = _cam.project_ray_normal(screen_pos)
	var best_t: float = INF
	var best_c: Vector3 = Vector3.ZERO
	var best_dist: float = distance
	# Crown: sphere.
	var crown_c: Vector3 = Vector3(0.0, 0.68 * _h, 0.0)
	var t: float = _ray_sphere(o, d, crown_c, maxf(_crown_r, 0.3 * _h))
	if t >= 0.0 and t < best_t:
		best_t = t
		best_c = crown_c
		best_dist = distance
	# Root zone: the soil body below the surface (a flat lens: its mid-depth disc, or a sphere seen edge-on).
	var root_c: Vector3 = Vector3(0.0, -_plug_d * 0.5, 0.0)
	t = -1.0
	if absf(d.y) > 1e-3:
		var tp: float = (root_c.y - o.y) / d.y
		var hp: Vector3 = o + d * tp
		if tp > 0.0 and Vector2(hp.x, hp.z).length() < _plug_r * 0.7:
			t = tp
	if t < 0.0:
		t = _ray_sphere(o, d, root_c, _plug_d * 0.6)
	if t >= 0.0 and t < best_t:
		best_t = t
		best_c = root_c
		best_dist = maxf(_plug_r * 2.4, 2.0)
	# Trunk: closest approach of the ray to the axis segment.
	var trunk_top: float = maxf(0.55 * _h, 1.0)
	var axis_hit: Vector3 = _ray_axis(o, d, trunk_top)
	if axis_hit.x >= 0.0 and axis_hit.x < best_t + 0.5:
		best_t = axis_hit.x
		best_c = Vector3(0.0, axis_hit.y, 0.0)
		best_dist = clampf(maxf(_h * 0.7, 1.5), 1.5, distance)
	if best_t == INF:
		return
	manual = true
	goal_target = best_c
	goal_distance = clampf(best_dist, 0.8, MAX_DIST)


static func _ray_sphere(o: Vector3, d: Vector3, c: Vector3, r: float) -> float:
	var oc: Vector3 = o - c
	var b: float = oc.dot(d)
	var disc: float = b * b - (oc.dot(oc) - r * r)
	if disc < 0.0:
		return -1.0
	return -b - sqrt(disc)


## Returns (t along the ray, height on the axis, 0) when the ray passes within the trunk's radius of the
## trunk axis segment [0, top]; t = -1 otherwise.
func _ray_axis(o: Vector3, d: Vector3, top: float) -> Vector3:
	# Closest points between the ray (o + t d) and the axis (0, y, 0).
	var w: Vector3 = o
	var a: float = d.x * d.x + d.z * d.z
	if a < 1.0e-6:
		return Vector3(-1.0, 0.0, 0.0)
	var t: float = -(w.x * d.x + w.z * d.z) / a
	var p: Vector3 = o + d * t
	var radial: float = Vector2(p.x, p.z).length()
	var trunk_r: float = 0.25 + 0.03 * _h
	if t < 0.0 or radial > trunk_r or p.y < -0.2 or p.y > top:
		return Vector3(-1.0, 0.0, 0.0)
	return Vector3(t, clampf(p.y, 0.5, top), 0.0)
