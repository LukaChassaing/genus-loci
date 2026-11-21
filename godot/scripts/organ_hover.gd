class_name OrganHover
extends PanelContainer
## Étiquette au survol de l'arbre (contiguïté spatiale, Mayer) : sur le tronc, son diamètre à 1,30 m et la hauteur ;
## sur la zone racinaire, la profondeur et l'étalement des racines. « Picking » géométrique simple, dans l'espace de
## l'écran, sans lancer de rayon sur les maillages (voir probe). Désactivée pendant la taille, au-dessus de
## l'interface et pendant un glisser de la caméra.

const TOL_PX: float = 7.0   # tolérance de survol autour du tronc (pixels)

var cam: Camera3D
var sim: TreeSim
## Rend vrai quand l'étiquette doit se taire (outil de taille actif, souris sur l'interface...).
var blocked: Callable = func() -> bool: return false
var _label: Label
var hit: String = ""          # "tronc", "racines" ou "" (tests)
## Point d'écran imposé (captures, tests) à la place de la souris ; x < 0 = la souris.
var force_point: Vector2 = Vector2(-1.0, -1.0)


func _init() -> void:
	theme_type_variation = "LegendPanel"
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	visible = false
	_label = UiStyle.label("", "SmallLabel")
	_label.add_theme_color_override("font_color", UiStyle.TEXT)
	_label.add_theme_font_size_override("font_size", 13)
	add_child(_label)


## Ce que désigne le point d'écran `p` : "tronc", "racines" ou "". Tout se fait dans l'espace de l'écran avec
## unproject_position (le même que les tests de cadrage) : le tronc est le segment projeté de l'axe (du collet au bas
## du houppier), épaissi de son diamètre projeté et d'une tolérance ; la zone racinaire est le rectangle projeté
## (étalement de part et d'autre de l'axe, face à la caméra, jusqu'à la profondeur des racines).
func probe(p: Vector2) -> String:
	if cam == null or sim == null or not cam.is_inside_tree():
		return ""
	var right: Vector3 = cam.global_transform.basis.x
	right = Vector3(right.x, 0.0, right.z)
	if right.length() < 1e-3:
		return ""
	right = right.normalized()
	var h: float = sim.get_height_m()
	var y_top: float = maxf(0.6 * h, 0.3)
	if cam.is_position_behind(Vector3.ZERO):
		return ""
	var a: Vector2 = cam.unproject_position(Vector3.ZERO)
	var b: Vector2 = cam.unproject_position(Vector3(0.0, y_top, 0.0))
	var r_trunk: float = 0.005 * sim.get_trunk_diameter_cm()
	var mid: Vector3 = Vector3(0.0, 0.5 * y_top, 0.0)
	var half_px: float = cam.unproject_position(mid + right * r_trunk * 1.15).distance_to(cam.unproject_position(mid))
	var q: Vector2 = Geometry2D.get_closest_point_to_segment(p, a, b)
	if q.distance_to(p) <= half_px + TOL_PX:
		return "tronc"
	var s: float = sim.get_root_spread_m()
	var d: float = sim.get_root_depth_m()
	if s > 0.0 and d > 0.0:
		var quad: PackedVector2Array = PackedVector2Array([cam.unproject_position(-right * s), cam.unproject_position(right * s),
			cam.unproject_position(right * s + Vector3.DOWN * d), cam.unproject_position(-right * s + Vector3.DOWN * d)])
		if Geometry2D.is_point_in_polygon(p, quad):
			return "racines"
	return ""


func _process(_delta: float) -> void:
	var t0: int = Time.get_ticks_usec()
	_update()
	Prof.add("gd.organ_hover", t0)


func _update() -> void:
	var forced: bool = force_point.x >= 0.0
	var mp: Vector2 = force_point if forced else get_viewport().get_mouse_position()
	var off: bool = not forced and (bool(blocked.call()) or Input.is_mouse_button_pressed(MOUSE_BUTTON_LEFT) \
		or Input.is_mouse_button_pressed(MOUSE_BUTTON_RIGHT) or Input.is_mouse_button_pressed(MOUSE_BUTTON_MIDDLE))
	hit = "" if off else probe(mp)
	if hit == "":
		visible = false
		return
	if hit == "tronc":
		_label.text = "Tronc · DBH %s cm · hauteur %s m" % [UiStyle.num(sim.get_trunk_diameter_cm(), 1), UiStyle.num(sim.get_height_m(), 1)]
	else:
		_label.text = "Racines · profondeur %s m · étalement %s m" % [UiStyle.num(sim.get_root_depth_m(), 1), UiStyle.num(sim.get_root_spread_m(), 1)]
	visible = true
	size = get_combined_minimum_size()
	var view: Vector2 = get_viewport().get_visible_rect().size
	position = Vector2(clampf(mp.x + 16.0, 0.0, view.x - size.x), clampf(mp.y + 18.0, 0.0, view.y - size.y))
