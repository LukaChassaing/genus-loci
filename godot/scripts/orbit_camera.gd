class_name OrbitCamera
extends Node3D
## Orbit camera: left-drag rotates, wheel zooms, right/middle-drag pans vertically.
## The target height follows ~40 % of the tree height; the distance adapts to tree size
## unless the user has zoomed manually.

var yaw: float = 0.35
var pitch: float = 0.12
var distance: float = 6.0
var target_y: float = 0.5
var pan_offset: float = 0.0

var _goal_y: float = 0.5
var _goal_distance: float = 6.0
var _manual_zoom: bool = false
var _cam: Camera3D


func _ready() -> void:
	_cam = Camera3D.new()
	_cam.fov = 50.0
	_cam.near = 0.05
	_cam.far = 400.0
	add_child(_cam)
	_apply()


## Called every frame with tree size; sets the follow goals.
func fit_to(height_m: float, crown_radius_m: float) -> void:
	_goal_y = height_m * 0.4
	if not _manual_zoom:
		_goal_distance = maxf(3.0, height_m * 1.6 + crown_radius_m * 1.2)


func snap() -> void:
	target_y = _goal_y
	distance = _goal_distance
	_apply()


func _process(delta: float) -> void:
	var k: float = 1.0 - exp(-4.0 * delta)
	target_y = lerpf(target_y, _goal_y, k)
	distance = lerpf(distance, _goal_distance, k)
	_apply()


func _apply() -> void:
	var t: Vector3 = Vector3(0.0, target_y + pan_offset, 0.0)
	var dir: Vector3 = Vector3(
		sin(yaw) * cos(pitch), sin(pitch), cos(yaw) * cos(pitch))
	_cam.global_position = t + dir * distance
	_cam.look_at(t, Vector3.UP)


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		var m: InputEventMouseMotion = event
		if m.button_mask & MOUSE_BUTTON_MASK_LEFT:
			yaw -= m.relative.x * 0.006
			pitch = clampf(pitch + m.relative.y * 0.006, deg_to_rad(-22.0), deg_to_rad(80.0))
		elif m.button_mask & (MOUSE_BUTTON_MASK_RIGHT | MOUSE_BUTTON_MASK_MIDDLE):
			pan_offset = clampf(pan_offset + m.relative.y * 0.004 * distance, -_goal_y - 2.0, 40.0)
	elif event is InputEventMouseButton:
		var b: InputEventMouseButton = event
		if not b.pressed:
			return
		if b.button_index == MOUSE_BUTTON_WHEEL_UP:
			_zoom(0.9)
		elif b.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			_zoom(1.1)


func _zoom(factor: float) -> void:
	_manual_zoom = true
	_goal_distance = clampf(_goal_distance * factor, 1.0, 150.0)
