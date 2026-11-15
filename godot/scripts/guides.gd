class_name Guides
extends Node3D
## Scale references ("Repères"): a 1.75 m human standee on the plug (real 3-D, so trees occlude it), a
## vertical height scale (ticks every 1 / 5 / 10 m depending on tree size) and metre labels along the
## soil cut faces. Ticks and text are drawn in a 2-D overlay projected from 3-D points, which stays crisp.

const HUMAN_HEIGHT: float = 1.75
const INK: Color = Color(0.13, 0.16, 0.20, 0.90)
const HALO: Color = Color(1.0, 1.0, 1.0, 0.60)
const FONT_SIZE: int = 15

var soil: SoilBlock
var cam: OrbitCamera
var tree_height: float = 1.0
var shown: bool = false

var _standee: MeshInstance3D
var _layer: CanvasLayer
var _overlay: Overlay


class Overlay extends Control:
	var owner_guides: Guides

	func _draw() -> void:
		if owner_guides != null:
			owner_guides.draw_overlay(self)


func _ready() -> void:
	_standee = MeshInstance3D.new()
	_standee.mesh = _build_human()
	_standee.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var m: StandardMaterial3D = StandardMaterial3D.new()
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	m.albedo_color = Color(0.17, 0.20, 0.26, 0.88)
	m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	m.cull_mode = BaseMaterial3D.CULL_DISABLED
	m.billboard_mode = BaseMaterial3D.BILLBOARD_FIXED_Y
	_standee.material_override = m
	add_child(_standee)
	_layer = CanvasLayer.new()
	_layer.layer = 0
	add_child(_layer)
	_overlay = Overlay.new()
	_overlay.owner_guides = self
	_overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
	_overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_layer.add_child(_overlay)
	set_shown(false)


func set_shown(on: bool) -> void:
	shown = on
	_standee.visible = on
	_overlay.visible = on


## Called every frame.
func update_from(height_m: float) -> void:
	tree_height = height_m
	if not shown:
		return
	var a: float = soil.face_azimuth(0) + 0.5
	_standee.position = Vector3(sin(a), 0.0, cos(a)) * soil.rim_radius(a) * 0.62
	_overlay.queue_redraw()


func ruler_base() -> Vector3:
	var a: float = soil.face_azimuth(1) - 0.35
	return Vector3(sin(a), 0.0, cos(a)) * soil.rim_radius(a) * 0.95


## Tick spacing (m) for the height scale: [minor, major (labelled)].
static func scale_steps(h: float) -> Vector2:
	if h < 8.0:
		return Vector2(1.0, 1.0)
	if h < 40.0:
		return Vector2(1.0, 5.0)
	return Vector2(5.0, 10.0)


func _proj(p: Vector3) -> Vector2:
	return cam.get_camera().unproject_position(p)


func _text(c: Control, pos: Vector2, s: String, align_right: bool = false) -> void:
	var font: Font = c.get_theme_default_font()
	var w: float = font.get_string_size(s, HORIZONTAL_ALIGNMENT_LEFT, -1, FONT_SIZE).x
	var at: Vector2 = pos - Vector2(w if align_right else 0.0, 0.0)
	c.draw_string_outline(font, at, s, HORIZONTAL_ALIGNMENT_LEFT, -1, FONT_SIZE, 4, HALO)
	c.draw_string(font, at, s, HORIZONTAL_ALIGNMENT_LEFT, -1, FONT_SIZE, INK)


func draw_overlay(c: Control) -> void:
	if cam == null or soil == null or cam.get_camera() == null:
		return
	var camera: Camera3D = cam.get_camera()
	# --- Height scale. ---
	var steps: Vector2 = scale_steps(tree_height)
	var top_m: float = ceilf(maxf(tree_height, 1.0) / steps.x) * steps.x
	var base: Vector3 = ruler_base()
	if not camera.is_position_behind(base) and not camera.is_position_behind(base + Vector3(0, top_m, 0)):
		c.draw_line(_proj(base), _proj(base + Vector3(0.0, top_m, 0.0)), INK, 1.5, true)
		var m: float = 0.0
		while m <= top_m + 0.001:
			var is_major: bool = is_zero_approx(fposmod(m, steps.y)) or steps.y == steps.x
			var p: Vector2 = _proj(base + Vector3(0.0, m, 0.0))
			c.draw_line(p, p + Vector2(12.0 if is_major else 6.0, 0.0), INK, 1.5, true)
			if is_major and m > 0.0:
				_text(c, p + Vector2(16.0, 5.0), "%d m" % roundi(m))
			m += steps.x
	# --- Depth ticks on the cut faces (only on faces turned toward the camera, or in X-ray). ---
	var step_d: float = 1.0 if soil.depth <= 6.0 else 2.0
	for which: int in 2:
		# On the cut face, above its deep part (the soil body tapers toward the rim).
		var rim: Vector3 = soil.face_ruler(which)
		var mid: Vector3 = rim * 0.5
		if not soil.xray and soil.face_normal(which).dot(camera.global_position - mid) <= 0.0:
			continue
		var side: float = 1.0 if which == 0 else -1.0
		var d: float = step_d
		while d <= soil.face_ruler_depth(which) - 0.15:
			var pt: Vector3 = rim + Vector3(0.0, -d, 0.0)
			if not camera.is_position_behind(pt):
				var p: Vector2 = _proj(pt)
				c.draw_line(p, p + Vector2(10.0 * side, 0.0), INK, 1.5, true)
				_text(c, p + Vector2(14.0 * side, 5.0), "%d m" % roundi(d), which == 1)
			d += step_d


## Flat stylised 1.75 m figure (head, torso, arms, legs) in the XY plane, feet on y = 0.
static func _build_human() -> ArrayMesh:
	var polys: Array[PackedVector2Array] = []
	var head: PackedVector2Array = PackedVector2Array()
	for i: int in 16:
		var t: float = TAU * i / 16.0
		head.append(Vector2(cos(t) * 0.10, 1.65 + sin(t) * 0.10))
	polys.append(head)
	polys.append(PackedVector2Array([Vector2(-0.05, 1.54), Vector2(-0.20, 1.48), Vector2(-0.18, 1.02),
		Vector2(-0.16, 0.88), Vector2(0.16, 0.88), Vector2(0.18, 1.02), Vector2(0.20, 1.48), Vector2(0.05, 1.54)]))
	for s: float in [-1.0, 1.0]:
		polys.append(PackedVector2Array([Vector2(s * 0.015, 0.92), Vector2(s * 0.165, 0.92),
			Vector2(s * 0.145, 0.06), Vector2(s * 0.17, 0.0), Vector2(s * 0.03, 0.0), Vector2(s * 0.03, 0.06)]))
		polys.append(PackedVector2Array([Vector2(s * 0.215, 1.46), Vector2(s * 0.285, 1.42),
			Vector2(s * 0.32, 0.94), Vector2(s * 0.25, 0.92)]))
	var v: PackedVector3Array = PackedVector3Array()
	var idx: PackedInt32Array = PackedInt32Array()
	for poly: PackedVector2Array in polys:
		var tri: PackedInt32Array = Geometry2D.triangulate_polygon(poly)
		var o: int = v.size()
		for p: Vector2 in poly:
			v.append(Vector3(p.x, p.y, 0.0))
		for t: int in tri:
			idx.append(o + t)
	var arrays: Array = []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = v
	arrays[Mesh.ARRAY_INDEX] = idx
	var mesh: ArrayMesh = ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh
