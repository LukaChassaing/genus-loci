class_name OrbitCamera
extends Node3D
## Soil-profile orbit camera: left-drag rotates (yaw limited to the cut-face side), wheel zooms,
## right/middle-drag pans vertically. Framing keeps the whole crown AND the root zone inside the part
## of the window not covered by HUD panels (see set_free_rect), and never goes below the ground.

const FOV_DEG: float = 50.0
const MAX_YAW: float = 1.15
const MIN_CAM_HEIGHT: float = 0.3

var yaw: float = 0.25
var pitch: float = 0.12
var distance: float = 6.0
var center_y: float = 0.5
var shift_px: Vector2 = Vector2.ZERO
var pan_offset: float = 0.0
var zoom_mult: float = 1.0

var _goal_y: float = 0.5
var _goal_distance: float = 6.0
var _goal_shift: Vector2 = Vector2.ZERO
var _free_rect: Rect2 = Rect2(0, 0, 1280, 720)
var _view_size: Vector2 = Vector2(1280, 720)
var _cam: Camera3D
## Debug/preset override: look at `override_point` from `override_dist` (no HUD offset).
var use_override: bool = false
var override_point: Vector3 = Vector3.ZERO
var override_dist: float = 3.0


func _ready() -> void:
	_cam = Camera3D.new()
	_cam.fov = FOV_DEG
	_cam.near = 0.05
	_cam.far = 1500.0
	add_child(_cam)
	_apply()


func get_camera() -> Camera3D:
	return _cam


## Region of the (canvas-unit) viewport not covered by HUD panels; the tree is framed inside it.
func set_free_rect(free: Rect2, view_size: Vector2) -> void:
	_free_rect = free
	_view_size = view_size


## Called every frame with the tree size; sets the framing goals.
func fit_to(height_m: float, crown_radius_m: float, root_depth_m: float, root_spread_m: float) -> void:
	var top: float = maxf(height_m, 1.2) * 1.04 + 0.3
	var bottom: float = -maxf(root_depth_m * 1.15 + 0.3, 1.2)
	var half_w: float = maxf(maxf(crown_radius_m, root_spread_m * 1.1), 1.0) * 1.06
	var tan_half: float = tan(deg_to_rad(FOV_DEG) * 0.5)
	var fh: float = maxf(_free_rect.size.y / _view_size.y, 0.2)
	var fw: float = maxf(_free_rect.size.x / _view_size.y, 0.2)
	var d_v: float = (top - bottom) / (2.0 * tan_half * fh)
	var d_h: float = (2.0 * half_w) / (2.0 * tan_half * fw)
	var base: float = maxf(d_v, d_h) + crown_radius_m
	_goal_distance = clampf(base * zoom_mult, 1.0, 900.0)
	_goal_y = (top + bottom) * 0.5 + pan_offset
	_goal_shift = _free_rect.get_center() - _view_size * 0.5
	if use_override:
		_goal_distance = override_dist
		_goal_y = override_point.y
		_goal_shift = Vector2.ZERO


func snap() -> void:
	center_y = _goal_y
	distance = _goal_distance
	shift_px = _goal_shift
	_apply()


## Restores automatic framing (new seed).
func reset_view() -> void:
	zoom_mult = 1.0
	pan_offset = 0.0


func _process(delta: float) -> void:
	var k: float = 1.0 - exp(-4.0 * delta)
	center_y = lerpf(center_y, _goal_y, k)
	distance = lerpf(distance, _goal_distance, k)
	shift_px = shift_px.lerp(_goal_shift, k)
	_apply()


func _apply() -> void:
	if _cam == null:
		return
	var s: float = sin(pitch)
	# Never let the camera sink below the ground (would show the underside of the soil block).
	var base_y: float = center_y
	s = maxf(s, (MIN_CAM_HEIGHT - base_y) / maxf(distance, 0.1))
	s = clampf(s, -0.95, 0.99)
	var c: float = sqrt(1.0 - s * s)
	var dir: Vector3 = Vector3(sin(yaw) * c, s, cos(yaw) * c)
	var fwd: Vector3 = -dir
	var right: Vector3 = fwd.cross(Vector3.UP).normalized()
	var up: Vector3 = right.cross(fwd).normalized()
	var wpp: float = 2.0 * tan(deg_to_rad(FOV_DEG) * 0.5) * distance / _view_size.y
	var t: Vector3 = Vector3(override_point.x if use_override else 0.0, center_y, override_point.z if use_override else 0.0) + (-right * shift_px.x + up * shift_px.y) * wpp
	_cam.global_position = t + dir * distance
	_cam.look_at(t, Vector3.UP)


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		var m: InputEventMouseMotion = event
		if m.button_mask & MOUSE_BUTTON_MASK_LEFT:
			yaw = clampf(yaw - m.relative.x * 0.006, -MAX_YAW, MAX_YAW)
			pitch = clampf(pitch + m.relative.y * 0.006, deg_to_rad(-25.0), deg_to_rad(80.0))
		elif m.button_mask & (MOUSE_BUTTON_MASK_RIGHT | MOUSE_BUTTON_MASK_MIDDLE):
			pan_offset = clampf(pan_offset + m.relative.y * 0.004 * distance, -40.0, 40.0)
	elif event is InputEventMouseButton:
		var b: InputEventMouseButton = event
		if not b.pressed:
			return
		if b.button_index == MOUSE_BUTTON_WHEEL_UP:
			zoom(0.9)
		elif b.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			zoom(1.1)


func zoom(factor: float) -> void:
	zoom_mult = clampf(zoom_mult * factor, 0.03, 3.0)
