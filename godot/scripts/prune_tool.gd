class_name PruneTool
extends Node3D
## Pruning tool (« Taille », key T): hover highlights the branch under the mouse and tints what a cut would
## remove; the cut point snaps to the collar near a fork; a tooltip gives the cut type, diameter, share of
## foliage removed, predicted closure, decay / infection risk and season advice. Left click (without dragging)
## cuts; Shift = flush cut; Ctrl+Z undoes the last cut. Dragging still orbits the camera.
## Also draws the wounds (cut faces, woundwood, decay, pollard knobs), the falling cut part, the optional
## "what if" ghost of the unpruned control tree, and the side panel (journal, comparison, time jumps).
## All biology lives in the core (crates/dendro-core/src/pruning.rs).

const PICK_TOL_PX: float = 7.0
const DRAG_PX: float = 5.0
const KIND_COLORS: Array[Color] = [
	Color(0.45, 0.85, 0.40),  # collar
	Color(0.95, 0.60, 0.20),  # flush
	Color(0.95, 0.75, 0.25),  # stub
	Color(0.55, 0.85, 0.45),  # reduction
	Color(0.95, 0.80, 0.35),  # heading
	Color(0.92, 0.32, 0.22),  # topping
	Color(0.85, 0.55, 0.85),  # pollard
	Color(0.92, 0.32, 0.22),  # coppice
	Color(0.6, 0.6, 0.6),     # natural
]
const KIND_HINTS: Array[String] = [
	"coupe juste à l'extérieur du bourrelet : bonne compartimentation",
	"coupe rase : bourrelet détruit, plaie plus large, carie plus probable",
	"un chicot reste : il meurt, retarde la fermeture et invite la carie",
	"réduction sur une branche d'au moins 1/3 du diamètre",
	"rabattage entre deux ramifications : rejets sous la coupe",
	"étêtage : grosse plaie qui ne se referme pas, rejets nombreux",
	"têtard : coupes répétées au même endroit, une « tête » se forme",
	"recépage : la souche rejette (ou drageonne)",
	"",
]

var host: Node3D
var sim: TreeSim
var cam: OrbitCamera
var hud: Hud
var active: bool = false
var flush_mode: bool = false
var ghost_on: bool = false
var ghost_extent: Vector2 = Vector2.ZERO   # (horizontal reach m, height m) of the visible ghost, else zero

var wound_mi: MeshInstance3D
var preview_mi: MeshInstance3D
var marker_mi: MeshInstance3D
var ghost_mi: MeshInstance3D
var _wound_ver: int = -1
var _ghost_ver: int = -1
var _ghost_age: float = 10.0

# Hover state.
var _mouse: Vector2 = Vector2(-1, -1)
var _mouse_moved: bool = false
var _forced_hover: bool = false
var hover: Dictionary = {}       # prune_pick result + "preview" (prune_preview) when hit
var _hover_key: String = ""
var _press_pos: Vector2 = Vector2(-1, -1)

# UI.
var _layer: CanvasLayer
var _tip: PanelContainer
var _tip_text: RichTextLabel
var panel: PanelContainer
var _journal: RichTextLabel
var _scroll: ScrollContainer
var _scroll_body: VBoxContainer
var _compare: RichTextLabel
var _flush_btn: Button
var _ghost_btn: Button
var undo_button: Button
var _toast: Label
var _toast_t: float = 0.0
var _panel_age: float = 1.0
var _journal_n: int = -1


func setup(main: Node3D) -> void:
	host = main
	sim = main.get("sim") as TreeSim
	cam = main.get("cam") as OrbitCamera
	hud = main.get("hud") as Hud
	wound_mi = MeshInstance3D.new()
	# Display colours (sRGB), double-sided, swaying with the wood (wind_sway.gd).
	WindSway.setup()
	wound_mi.material_override = WindSway.make_vertex_color_shader(0.92, true, true)
	add_child(wound_mi)
	preview_mi = MeshInstance3D.new()
	preview_mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	preview_mi.material_override = _overlay_mat(Color(1.0, 0.52, 0.12, 0.55), 2)
	add_child(preview_mi)
	marker_mi = MeshInstance3D.new()
	marker_mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var tm: TorusMesh = TorusMesh.new()
	tm.inner_radius = 0.82
	tm.outer_radius = 1.0
	tm.rings = 24
	tm.ring_segments = 8
	marker_mi.mesh = tm
	marker_mi.material_override = _overlay_mat(Color(0.45, 0.85, 0.40, 0.95), 3)
	marker_mi.visible = false
	add_child(marker_mi)
	ghost_mi = MeshInstance3D.new()
	ghost_mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var gm: StandardMaterial3D = _overlay_mat(Color(0.78, 0.90, 1.0, 0.22), 1)
	gm.no_depth_test = false
	ghost_mi.material_override = gm
	ghost_mi.visible = false
	add_child(ghost_mi)
	_build_ui()
	hud.prune_toggled.connect(set_active)


## Size of the control tree's silhouette for the camera framing (x: horizontal reach, y: height), zero while the
## ghost is hidden or not computed yet.
func _measure_ghost() -> void:
	ghost_extent = Vector2.ZERO
	if ghost_on and ghost_mi.mesh != null and ghost_mi.mesh.get_surface_count() > 0:
		var bb: AABB = ghost_mi.mesh.get_aabb()
		ghost_extent = Vector2(maxf(maxf(absf(bb.position.x), absf(bb.end.x)), maxf(absf(bb.position.z), absf(bb.end.z))), bb.end.y)


func _overlay_mat(c: Color, prio: int) -> StandardMaterial3D:
	var m: StandardMaterial3D = StandardMaterial3D.new()
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	m.albedo_color = c
	m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	m.cull_mode = BaseMaterial3D.CULL_DISABLED
	m.no_depth_test = true
	m.depth_draw_mode = BaseMaterial3D.DEPTH_DRAW_DISABLED
	m.render_priority = prio
	return m


# ------------------------------------------------------------------------------------------------ UI

func _build_ui() -> void:
	_layer = CanvasLayer.new()
	_layer.layer = 3
	add_child(_layer)
	var root: Control = Control.new()
	root.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	root.theme = UiStyle.make_theme(15, 8.0, 3.0)
	_layer.add_child(root)

	_toast = Label.new()
	_toast.add_theme_font_size_override("font_size", 17)
	_toast.add_theme_color_override("font_color", UiStyle.TEXT)
	_toast.add_theme_color_override("font_outline_color", Color(0.05, 0.07, 0.03, 0.9))
	_toast.add_theme_constant_override("outline_size", 4)
	_toast.set_anchors_preset(Control.PRESET_CENTER_TOP)
	_toast.position = Vector2(0, 18)
	_toast.visible = false
	root.add_child(_toast)

	# Panneau latéral : options de l'outil, sauts dans le temps, comparaison avec le témoin, journal. Il vit dans
	# l'emplacement unique des panneaux du Hud (un seul panneau ouvert à la fois).
	panel = PanelContainer.new()
	panel.custom_minimum_size = Vector2(290, 0)
	root.add_child(panel)
	hud.register_panel("taille", panel, "Taille")
	var v: VBoxContainer = VBoxContainer.new()
	v.add_theme_constant_override("separation", 5)
	panel.add_child(v)
	var hint: Label = UiStyle.serif_label("Survoler une branche, clic : couper. Maj : coupe rase.", 13, UiStyle.TEXT_DIM)
	v.add_child(hint)
	var r1: HFlowContainer = HFlowContainer.new()  # passe à la ligne dans le panneau de 340 px
	r1.add_theme_constant_override("h_separation", 4)
	r1.add_theme_constant_override("v_separation", 4)
	v.add_child(r1)
	_flush_btn = _btn(r1, "Coupe rase", "Couper au ras du tronc (déconseillé) au lieu du col")
	_flush_btn.toggle_mode = true
	_flush_btn.toggled.connect(func(on: bool) -> void:
		flush_mode = on
		_hover_key = "")
	undo_button = _btn(r1, "Annuler", "Annuler la dernière coupe (Ctrl+Z) : re-simule depuis sa date")
	undo_button.pressed.connect(undo)
	_ghost_btn = _btn(r1, "Témoin", "Silhouette de l'arbre jamais taillé (même graine)")
	_ghost_btn.toggle_mode = true
	_ghost_btn.toggled.connect(set_ghost)
	var r2: HFlowContainer = HFlowContainer.new()  # passe à la ligne dans le panneau de 340 px
	r2.add_theme_constant_override("h_separation", 4)
	r2.add_theme_constant_override("v_separation", 4)
	v.add_child(r2)
	var jl: Label = UiStyle.serif_label("Avancer :", 14, UiStyle.TEXT_DIM)
	r2.add_child(jl)
	for y: int in [1, 10, 50]:
		var b: Button = _btn(r2, "+%d an%s" % [y, "s" if y > 1 else ""], "Simuler %d an%s d'un coup" % [y, "s" if y > 1 else ""])
		b.pressed.connect(func() -> void: jump_years(y))
	var r3: HFlowContainer = HFlowContainer.new()  # passe à la ligne dans le panneau de 340 px
	r3.add_theme_constant_override("h_separation", 4)
	r3.add_theme_constant_override("v_separation", 4)
	v.add_child(r3)
	var ops: Array = [["Élaguer 3 m", "raise", 3.0, "Supprimer les branches basses du tronc jusqu'à 3 m (coupes au col)"],
		["Éclaircir", "thin", 0.25, "Retirer environ un quart des rameaux secondaires"],
		["Réduire", "reduce", 0.25, "Raccourcir la couronne d'un quart"],
		["Têtard", "pollard", 0.0, "Recéper les rejets des têtes (ou étêter à 2,5 m)"]]
	for o: Array in ops:
		var b: Button = _btn(r3, o[0], o[3])
		var op: String = o[1]
		var arg: float = o[2]
		b.pressed.connect(func() -> void: run_op(op, arg))
	v.add_child(UiStyle.rule())
	# Comparison + journal scroll when the dock is short of height (Hud.layout_side_panels -> dock_fit).
	_scroll = ScrollContainer.new()
	_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	v.add_child(_scroll)
	var body: VBoxContainer = VBoxContainer.new()
	body.add_theme_constant_override("separation", 5)
	body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_scroll.add_child(body)
	_scroll_body = body
	panel.set_meta("dock_fit", dock_fit)
	_compare = RichTextLabel.new()
	_compare.bbcode_enabled = true
	_compare.fit_content = true
	_compare.scroll_active = false
	_compare.custom_minimum_size = Vector2(290, 0)
	_compare.add_theme_font_size_override("normal_font_size", 13)
	_compare.add_theme_font_size_override("bold_font_size", 13)
	body.add_child(_compare)
	body.add_child(UiStyle.rule())
	_journal = RichTextLabel.new()
	_journal.bbcode_enabled = true
	_journal.fit_content = true
	_journal.scroll_active = false
	_journal.custom_minimum_size = Vector2(290, 0)
	_journal.add_theme_font_size_override("normal_font_size", 13)
	_journal.add_theme_font_size_override("bold_font_size", 13)
	body.add_child(_journal)

	# Tooltip last: drawn above the panel.
	_tip = PanelContainer.new()
	_tip.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_tip.visible = false
	root.add_child(_tip)
	_tip_text = RichTextLabel.new()
	_tip_text.bbcode_enabled = true
	_tip_text.fit_content = true
	_tip_text.scroll_active = false
	_tip_text.autowrap_mode = TextServer.AUTOWRAP_OFF
	_tip_text.custom_minimum_size = Vector2(290, 0)
	_tip_text.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_tip_text.add_theme_font_size_override("normal_font_size", 14)
	_tip_text.add_theme_font_size_override("bold_font_size", 15)
	_tip.add_child(_tip_text)


## Dock protocol: make the panel at most `max_h` tall by shortening the scrolling area (never below ~3 lines).
func dock_fit(max_h: float) -> void:
	var nat: float = _scroll_body.get_combined_minimum_size().y
	_scroll.custom_minimum_size.y = nat
	var over: float = panel.get_combined_minimum_size().y - max_h
	if over > 0.0:
		_scroll.custom_minimum_size.y = maxf(nat - over, 30.0)


func _btn(parent: Control, text: String, tip: String) -> Button:
	var b: Button = Button.new()
	b.text = text
	b.tooltip_text = tip
	b.focus_mode = Control.FOCUS_NONE
	parent.add_child(b)
	return b


func set_active(on: bool) -> void:
	active = on
	# Le panneau suit l'emplacement unique du Hud (il ferme le panneau ouvert, ou se ferme).
	if on and hud.current_panel() != "taille":
		hud.open_panel("taille")
	elif not on and hud.current_panel() == "taille":
		hud.open_panel("")
	if hud.prune_button.button_pressed != on:
		hud.prune_button.set_pressed_no_signal(on)
	if not on:
		_clear_hover()
	_panel_age = 1.0


func set_ghost(on: bool) -> void:
	ghost_on = on
	ghost_mi.visible = on
	if not on:
		ghost_extent = Vector2.ZERO
	else:
		_measure_ghost()
	if _ghost_btn.button_pressed != on:
		_ghost_btn.set_pressed_no_signal(on)
	_ghost_ver = -1
	_ghost_age = 10.0


## A new seed / species / soil: the old cuts are gone.
func on_reset() -> void:
	_clear_hover()
	_wound_ver = -1
	_journal_n = -1
	ghost_mi.mesh = null
	ghost_extent = Vector2.ZERO
	_ghost_ver = -1


# --------------------------------------------------------------------------------------------- input

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey:
		var k: InputEventKey = event
		if not k.pressed or k.echo:
			return
		if k.keycode == KEY_T:
			set_active(not active)
			get_viewport().set_input_as_handled()
		elif k.keycode == KEY_Z and (k.ctrl_pressed or k.meta_pressed) and active:
			undo()
			get_viewport().set_input_as_handled()
		return
	if not active:
		return
	if event is InputEventMouseMotion:
		var m: InputEventMouseMotion = event
		_mouse = m.position
		_mouse_moved = true
		_forced_hover = false
		if m.button_mask != 0:
			_clear_hover()  # orbit / pan: no highlight while dragging
	elif event is InputEventMouseButton:
		var b: InputEventMouseButton = event
		if b.button_index != MOUSE_BUTTON_LEFT:
			return
		if b.pressed:
			_press_pos = b.position
			if b.double_click and hover.get("hit", false):
				get_viewport().set_input_as_handled()  # no camera recentre on a double cut
		else:
			if _press_pos.x >= 0.0 and b.position.distance_to(_press_pos) <= DRAG_PX and hover.get("hit", false):
				cut_hovered(b.shift_pressed or flush_mode)
				get_viewport().set_input_as_handled()
			_press_pos = Vector2(-1, -1)
			_mouse = b.position
			_mouse_moved = true


## Picks under the screen point `p` and refreshes the highlight and tooltip.
func hover_at(p: Vector2, flush: bool = false) -> void:
	var c: Camera3D = cam.get_camera()
	var o: Vector3 = c.project_ray_origin(p)
	var d: Vector3 = c.project_ray_normal(p)
	var view_h: float = get_viewport().get_visible_rect().size.y
	var tol: float = PICK_TOL_PX * deg_to_rad(OrbitCamera.FOV_DEG) / maxf(view_h, 1.0)
	var h: Dictionary = sim.prune_pick(o, d, tol)
	_mouse = p
	if not h.get("hit", false):
		_clear_hover()
		return
	var fl: bool = flush or flush_mode or Input.is_key_pressed(KEY_SHIFT)
	var t: float = float(h["t"])
	var key: String = "%d|%.2f|%s" % [int(h["node_id"]), t, str(fl)]
	if key != _hover_key:
		_hover_key = key
		var pv: Dictionary = sim.prune_preview(int(h["node_id"]), t, fl)
		if not pv.get("ok", false):
			_clear_hover()
			return
		pv["phase"] = sim.get_phase()
		pv["doy"] = sim.get_day_of_year()
		h["preview"] = pv
		h["flush"] = fl
		hover = h
		preview_mi.mesh = sim.build_prune_preview_mesh(int(h["node_id"]), t, fl)
		preview_mi.visible = true
		_place_marker(pv)
		_tip_text.text = tooltip_text(pv)
	_tip.visible = true
	_place_tip()


func _clear_hover() -> void:
	hover = {}
	_hover_key = ""
	if preview_mi != null:
		preview_mi.visible = false
		marker_mi.visible = false
		_tip.visible = false


func _place_marker(pv: Dictionary) -> void:
	var p: Vector3 = pv["point"]
	var d: Vector3 = (pv["dir"] as Vector3).normalized()
	var r: float = float(pv["radius_m"]) * 1.35 + 0.02
	var up: Vector3 = Vector3.UP if absf(d.dot(Vector3.UP)) < 0.95 else Vector3.RIGHT
	var x: Vector3 = up.cross(d).normalized()
	var z: Vector3 = x.cross(d).normalized()
	marker_mi.transform = Transform3D(Basis(x * r, d * r * 0.6, z * r), p)
	var kind: int = int(pv["kind"])
	(marker_mi.material_override as StandardMaterial3D).albedo_color = KIND_COLORS[clampi(kind, 0, 8)]
	marker_mi.visible = true


func _place_tip() -> void:
	var vs: Vector2 = get_viewport().get_visible_rect().size
	var sz: Vector2 = _tip.get_combined_minimum_size()
	var p: Vector2 = _mouse + Vector2(22, 18)
	if p.x + sz.x > vs.x - 8.0:
		p.x = _mouse.x - sz.x - 22.0
	if p.y + sz.y > vs.y - 8.0:
		p.y = vs.y - sz.y - 8.0
	_tip.position = p
	_tip.size = sz


static func _pct(x: float) -> String:
	return "%d %%" % roundi(x * 100.0)


static func _num(x: float, dec: int = 1) -> String:
	return (("%." + str(dec) + "f") % x).replace(".", ",")


static func _risk_word(x: float) -> String:
	if x < 0.1:
		return "faible"
	if x < 0.35:
		return "modéré"
	return "élevé"


## French tooltip for a cut preview.
static func tooltip_text(pv: Dictionary) -> String:
	var sim_doy: int = int(pv.get("doy", 100))
	var kind: int = int(pv["kind"])
	var col: Color = KIND_COLORS[clampi(kind, 0, 8)]
	var lines: Array[String] = []
	lines.append("[b][color=#%s]%s[/color][/b] — Ø %s cm" % [col.to_html(false), pv["label"], _num(float(pv["diameter_cm"]))])
	lines.append("[color=#cfc7a0][i]%s[/i][/color]" % KIND_HINTS[clampi(kind, 0, 8)])
	lines.append("Feuillage retiré : %s   ·   bois : %s" % [_pct(float(pv["foliage_share"])), _pct(float(pv["wood_share"]))])
	var cy: float = float(pv["closure_years"])
	var never: String = "très lente (selon les rejets)" if float(pv["sprouts"]) >= 0.5 else "ne se refermera pas"
	lines.append("Fermeture de la plaie : " + (never if cy >= 98.0 else "~%d an%s" % [maxi(1, roundi(cy)), "s" if roundi(cy) > 1 else ""]))
	lines.append("Risque de carie : %s (%s)" % [_risk_word(float(pv["decay_risk"])), _pct(float(pv["decay_risk"]))])
	var inf: float = float(pv["infection_risk"])
	if inf >= 0.05:
		lines.append("Infection de la plaie (plomb, chancre) : %s (%s)" % [_risk_word(inf), _pct(inf)])
	var s: int = int(pv["season"])
	var season: String = ["[color=#9fdc8a]bonne saison[/color]", "[color=#e8d27a]saison acceptable[/color]", "[color=#f08a6a]saison déconseillée[/color]"][clampi(s, 0, 2)]
	var why: String = ""
	if s == 2 and int(pv.get("phase", 0)) in [1, 2]:
		why = " — débourrement, réserves au plus bas"
	elif s == 2:
		why = " — plaies exposées au plomb et au chancre : tailler en été"
	elif s == 0 and inf < 0.05:
		var d: int = sim_doy
		why = " — repos végétatif ; en plein hiver la plaie se ferme un peu plus lentement" if (d >= 335 or d < 60) else " — repos végétatif, repousse vigoureuse"
	elif s == 0:
		why = " — l'été, les plaies se ferment vite"
	lines.append("Saison : " + season + why)
	if bool(pv["bleeding"]):
		lines.append("Écoulement de sève possible (fin d'hiver)")
	var sp: float = float(pv["sprouts"])
	if sp >= 0.5:
		lines.append("Rejets attendus : ~%d" % roundi(sp))
	return "\n".join(lines)


# ---------------------------------------------------------------------------------------------- acts

func cut_hovered(flush: bool) -> int:
	if not hover.get("hit", false):
		return -1
	var id: int = int(hover["node_id"])
	var t: float = float(hover["t"])
	var pv: Dictionary = hover["preview"]
	var falling: ArrayMesh = preview_mi.mesh as ArrayMesh
	var kind: int = sim.prune_cut(id, t, flush)
	if kind < 0:
		return kind
	_drop_piece(falling)
	_show_toast("%s — Ø %s cm, %s du feuillage" % [sim.prune_kind_label(kind), _num(float(pv["diameter_cm"])), _pct(float(pv["foliage_share"]))])
	_after_change(false)
	return kind


func run_op(op: String, arg: float) -> int:
	var n: int = 0
	if op == "pollard":
		n = sim.prune_op("pollard", 0.0)
		if n == 0:
			# First pollarding: clear the bole, then cut the stem at 2.5 m.
			n = sim.prune_op("raise", 2.4) + sim.prune_op("top", 2.5)
	else:
		n = sim.prune_op(op, arg)
	_show_toast("%d coupe%s" % [n, "s" if n > 1 else ""] if n > 0 else "Rien à couper")
	_after_change(false)
	return n


func undo() -> bool:
	var ok: bool = sim.prune_undo()
	_show_toast("Dernière coupe annulée" if ok else "Aucune coupe à annuler")
	_after_change(true)
	return ok


func jump_years(y: int) -> void:
	var t0: int = Time.get_ticks_msec()
	sim.advance_days(float(y) * 365.0)
	_after_change(false)
	_show_toast("+%d an%s (%d ms)" % [y, "s" if y > 1 else "", Time.get_ticks_msec() - t0])


## The tree changed outside the pruning tool (a storm): wounds, journal and panel follow.
func on_external_change() -> void:
	_after_change(false)


## The tree changed outside the normal stepping: make the host rebuild everything.
func _after_change(replaced: bool) -> void:
	_clear_hover()
	_wound_ver = -1
	_journal_n = -1
	_panel_age = 1.0
	if replaced:
		for v: String in ["_wood_ver", "_leaf_ver", "_flower_ver", "_root_ver"]:
			host.set(v, -1)
		hud.force_ring_refresh()
	host.set("_stats_dirty", true)
	host.set("mesh_dirty", true)
	host.call("on_mechanics_stale")  # (day, wood version) can repeat after an undo: recompute the « Mécanique » panel
	# Flux paths follow the new wood at once (no dashes left on removed branches).
	var fx: Node = host.get("flux") as Node
	if fx != null and fx.has_method("mark_geometry_dirty"):
		fx.mark_geometry_dirty()
		var fv: Object = fx.get("view")
		if fv != null and bool(fv.get("enabled")) and fv.has_method("rebuild_geometry"):
			fv.call("rebuild_geometry", sim)


func _drop_piece(m: ArrayMesh) -> void:
	if m == null or m.get_surface_count() == 0:
		return
	var mi: MeshInstance3D = MeshInstance3D.new()
	mi.mesh = m
	var mat: StandardMaterial3D = StandardMaterial3D.new()
	mat.albedo_color = Color(0.45, 0.36, 0.28, 1.0)
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mi.material_override = mat
	mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(mi)
	var tw: Tween = create_tween()
	tw.set_parallel(true)
	var drop: float = clampf(m.get_aabb().position.y + 0.5, 1.0, 12.0)
	tw.tween_property(mi, "position:y", -drop, 1.1).set_ease(Tween.EASE_IN).set_trans(Tween.TRANS_QUAD)
	tw.tween_property(mat, "albedo_color:a", 0.0, 1.1).set_ease(Tween.EASE_IN)
	tw.chain().tween_callback(mi.queue_free)


func _show_toast(t: String) -> void:
	_toast.text = t
	_toast.visible = true
	_toast.reset_size()
	_toast.position.x = -_toast.size.x * 0.5
	_toast_t = 2.5


# ------------------------------------------------------------------------------------------- per frame

func _process(delta: float) -> void:
	var t0: int = Time.get_ticks_usec()
	_process_tool(delta)
	Prof.add("gd.prune_tool", t0)


func _process_tool(delta: float) -> void:
	# Wounds follow the wood mesh: rebuilt in the frame the host rebuilt the wood (not on every version bump).
	var wv: int = int(host.get("_wood_ver"))
	if wv != _wound_ver and wv >= 0:
		_wound_ver = wv
		wound_mi.mesh = sim.build_wound_mesh()
	if _toast_t > 0.0:
		_toast_t -= delta
		_toast.modulate.a = clampf(_toast_t / 0.6, 0.0, 1.0)
		_toast.visible = _toast_t > 0.0
	if active and _mouse_moved and not _forced_hover:
		_mouse_moved = false
		if Input.get_mouse_button_mask() == 0 and _mouse.x >= 0.0 and not _over_ui(_mouse):
			hover_at(_mouse)
		elif _over_ui(_mouse):
			_clear_hover()
	if ghost_on or (active and panel.visible):
		var done: bool = sim.control_poll(ghost_on)
		_ghost_age += delta
		if ghost_on and _ghost_age >= 1.0:
			var gv: int = sim.get_control_version()
			if gv != _ghost_ver or (done and ghost_mi.mesh == null):
				_ghost_ver = gv
				_ghost_age = 0.0
				ghost_mi.mesh = sim.build_control_wood_mesh()
				_measure_ghost()
	if active:
		_panel_age += delta
		if _panel_age >= 0.25:
			_panel_age = 0.0
			_refresh_panel()


func _over_ui(p: Vector2) -> bool:
	return hud.covers(p)


func _row(name: String, a: float, b: float, dec: int, unit: String, has_b: bool) -> String:
	var bs: String = (_num(b, dec) + unit) if has_b else "—"
	var diff: String = ""
	if has_b and absf(b) > 1e-6:
		var r: float = a / b - 1.0
		var good: bool = r >= 0.0
		if name.begins_with("Carie"):
			good = r <= 0.0
		diff = "[color=%s]%s%d %%[/color]" % ["#9fdc8a" if good else "#f08a6a", "+" if r >= 0.0 else "", roundi(r * 100.0)]
	return "[cell]%s[/cell][cell]%s%s[/cell][cell]%s[/cell][cell]%s[/cell]" % [name, _num(a, dec), unit, bs, diff]


func _refresh_panel() -> void:
	var m: Dictionary = sim.get_prune_metrics()
	var c: Dictionary = sim.get_control_metrics()
	var has_c: bool = not c.is_empty() and sim.get_prune_cut_count() > 0
	var computing: bool = sim.get_prune_cut_count() > 0 and (c.is_empty() or int(c["day"]) != int(m["day"]))
	var lines: Array[String] = []
	lines.append("[b]Comparaison avec le témoin non taillé[/b]%s" % (" [color=#cfc7a0](calcul…)[/color]" if computing else ""))
	var cv: Callable = func(k: String) -> float: return float(c.get(k, 0.0))
	var rows: Array[String] = ["[cell][color=#cfc7a0] [/color][/cell][cell][color=#cfc7a0]taillé  [/color][/cell][cell][color=#cfc7a0]témoin  [/color][/cell][cell][/cell]"]
	rows.append(_row("Hauteur", m["height_m"], cv.call("height_m"), 1, " m", has_c))
	rows.append(_row("Rayon couronne", m["crown_radius_m"], cv.call("crown_radius_m"), 1, " m", has_c))
	rows.append(_row("Diamètre 1,3 m", m["dbh_cm"], cv.call("dbh_cm"), 1, " cm", has_c))
	rows.append(_row("Feuillage", m["leaf_area_m2"], cv.call("leaf_area_m2"), 0, " m²", has_c))
	rows.append(_row("Réserves", float(m["reserve"]) * 100.0, cv.call("reserve") * 100.0, 0, " %", has_c))
	rows.append(_row("Vigueur", float(m["vigour"]), cv.call("vigour"), 2, "", has_c))
	rows.append(_row("Carie", float(m["decay_volume_m3"]) * 1000.0, cv.call("decay_volume_m3") * 1000.0, 0, " dm³", has_c))
	rows.append(_row("Espérance de vie", m["life_expectancy"], cv.call("life_expectancy"), 0, " ans", has_c))
	lines.append("[table=4]" + "".join(rows) + "[/table]")
	lines.append("Fût sans branche : %s m · sans nœud ouvert : %s m" % [_num(m["clear_bole_m"]), _num(m["knot_free_bole_m"])])
	if int(m["pollard_heads"]) > 0:
		lines.append("Têtes sur tronc coupé : %d · recépages de la tête : %d" % [int(m["pollard_heads"]), int(m["pollard_max_recuts"])])
	if int(m["sprouts"]) > 0:
		lines.append("Rejets vivants : %d" % int(m["sprouts"]))
	if float(m["alive"]) < 0.5:
		lines.append("[color=#f08a6a][b]L'arbre est mort.[/b][/color]")
	_compare.text = "\n".join(lines)
	var j: Array = sim.get_prune_journal()
	var jl: Array[String] = ["[b]Journal des coupes[/b] (%d)" % j.size()]
	if j.is_empty():
		jl.append("[color=#cfc7a0]Aucune coupe pour l'instant.[/color]")
	var first: int = maxi(0, j.size() - 7)
	for i: int in range(j.size() - 1, first - 1, -1):
		var e: Dictionary = j[i]
		var st: String = e["state_label"]
		var stc: String = {"ouverte": "#f0c070", "en cicatrisation": "#d8d890", "refermée": "#9fdc8a", "cariée": "#f08a6a", "chicot": "#e0a060", "disparue": "#a0a0a0"}.get(st, "#ffffff")
		var extra: String = ""
		if st == "en cicatrisation":
			extra = " %s" % _pct(float(e["closure"]))
		if bool(e["pathogen"]):
			extra += " · infectée"
		jl.append("an %d, %s — %s Ø %s cm : [color=%s]%s%s[/color]" % [int(e["year"]), Hud.date_text(int(e["doy"])), e["label"],
			_num(float(e["diameter_cm"]), 0), stc, st, extra])
	if j.size() > 7:
		jl.append("[color=#cfc7a0]… %d plus anciennes[/color]" % (j.size() - 7))
	_journal.text = "\n".join(jl)
	undo_button.disabled = sim.get_prune_cut_count() == 0


# ---------------------------------------------------------------------------------- verification hooks
#   DENDRO_PRUNE="year:doy:op:arg;..."  scripted cuts (doy 1-based like DENDRO_DOY); op = raise (m), top (m),
#                 coppice, pollard (recut the heads), thin (fraction), reduce (fraction), limb (radius m: collar
#                 cut of the stem limb closest to that radius), flushlimb, stublimb, headlimb (t 0.5 of it)
#   DENDRO_STORM="year:doy:speed;..."  scripted storms (gust m/s at 10 m), merged in date order with DENDRO_PRUNE
#                 (also accepted there as op `storm`, e.g. DENDRO_PRUNE="40:300:storm:42")
#   DENDRO_TAILLE=1  tool active (panel shown);  DENDRO_GHOST=1  control ghost shown
#   DENDRO_PRUNE_HOVER=auto[:radius_m[:frac]]|x,y   hover (highlight + tooltip) at a screen point, or on the stem
#                 limb closest to that radius (default 3 cm), `frac` along it

## Applies DENDRO_PRUNE on the already advanced tree: rewinds to each event date, cuts, then returns to the
## present (the log replays deterministically). Called from main's _apply_view_env.
func apply_env_script() -> void:
	var spec: String = OS.get_environment("DENDRO_PRUNE")
	var storms: String = OS.get_environment("DENDRO_STORM")
	for part: String in storms.split(";", false):
		var f: PackedStringArray = part.split(":")
		if f.size() >= 3:
			spec += ";%s:%s:storm:%s" % [f[0], f[1], f[2]]
	if spec != "":
		var now: int = sim.get_days_elapsed()
		var events: Array = []
		for part: String in spec.split(";", false):
			var f: PackedStringArray = part.split(":")
			if f.size() < 3:
				continue
			var d: int = int(f[0]) * 365 + maxi(int(f[1]) - 1, 0)
			events.append([d, f[2], float(f[3]) if f.size() > 3 else 0.0])
		events.sort_custom(func(a: Array, b: Array) -> bool: return int(a[0]) < int(b[0]))
		for e: Array in events:
			if int(e[0]) > now:
				continue
			sim.prune_seek_day(int(e[0]))
			var n: int = _script_op(e[1], e[2])
			print("[dendro][prune] day %d (year %d): %s %.2f -> %d cut(s)" % [e[0], int(e[0]) / 365, e[1], e[2], n])
		sim.prune_seek_day(now)
		_after_change(true)
		var m: Dictionary = sim.get_prune_metrics()
		print("[dendro][prune] metrics: h=%.1f dbh=%.1f leaf=%.0f reserve=%.2f sprouts=%d decay=%.4f life=%.0f clear_bole=%.1f knot_free=%.1f heads=%d" % [
			m["height_m"], m["dbh_cm"], m["leaf_area_m2"], m["reserve"], int(m["sprouts"]), m["decay_volume_m3"],
			m["life_expectancy"], m["clear_bole_m"], m["knot_free_bole_m"], int(m["pollard_heads"])])
		for e: Dictionary in sim.get_prune_journal():
			print("[dendro][prune]   %s Ø %.1f cm year %d: %s closure %.2f decay %.2f" % [e["label"], e["diameter_cm"], e["year"], e["state_label"], e["closure"], e["decay"]])
	if OS.get_environment("DENDRO_TAILLE") == "1":
		set_active(true)
	if OS.get_environment("DENDRO_GHOST") == "1":
		set_ghost(true)
	# DENDRO_PRUNE_PLAY=1: after the scripted cuts, play at full speed (time-lapse bench of a pruned tree).
	if OS.get_environment("DENDRO_PRUNE_PLAY") == "1":
		(func() -> void:
			hud.speed_slider.value = 1.0
			hud.set_playing(true)).call_deferred()


func _script_op(op: String, arg: float) -> int:
	match op:
		"storm":
			var n: int = int(host.call("storm", arg))
			var hist: Array = sim.get_wind_history()
			for e: Dictionary in (hist.slice(hist.size() - n) if n > 0 else []):
				print("[dendro][storm]   %s at %.1f m, Ø %.1f cm (gust %.1f m/s)" % [e["label"], e["height_m"], e["diameter_cm"], e["gust_ms"]])
			return n
		"limb", "flushlimb", "stublimb", "headlimb":
			var hp: PackedFloat32Array = sim.prune_hint_point(arg, 0.0)
			if hp[3] < 0.0:
				return 0
			var t: float = 0.5 if op == "stublimb" or op == "headlimb" else 0.0
			return 1 if sim.prune_cut(int(hp[3]), t, op == "flushlimb") >= 0 else 0
		_:
			return sim.prune_op(op, arg)


## Screenshot preparation (after the camera settled): ghost computed, hover shown.
func prepare_shot() -> void:
	if ghost_on:
		var frames: int = 0
		while not sim.control_poll(true) and frames < 3000:
			await get_tree().process_frame
			frames += 1
		ghost_mi.mesh = sim.build_control_wood_mesh()
		_measure_ghost()
		_refresh_panel()
		print("[dendro][prune] control ready after %d frames" % frames)
	# DENDRO_PRUNE_CAM=wound[:k[:dist[:pitch[:yaw]]]]  close-up on the wound of journal entry k (default the first)
	var pc: String = OS.get_environment("DENDRO_PRUNE_CAM")
	if pc.begins_with("wound"):
		var f: PackedStringArray = pc.split(":")
		var k: int = int(f[1]) if f.size() > 1 and f[1] != "head" else 0
		if f.size() > 1 and f[1] == "head":  # the first stem cut (topping, pollard, coppice)
			var jj: Array = sim.get_prune_journal()
			for i: int in jj.size():
				if int(jj[i]["kind"]) in [5, 6, 7]:
					k = i
					break
		var pts: PackedVector3Array = sim.get_wound_positions()
		if k < pts.size() and not is_nan(pts[k].x):
			cam.use_override = true
			cam.override_point = pts[k]
			cam.override_dist = float(f[2]) if f.size() > 2 else 2.5
			var pk: Vector3 = pts[k]
			var yaw: float = atan2(pk.x, pk.z) if Vector2(pk.x, pk.z).length() > 0.05 else 0.6
			cam.set_angles(float(f[4]) if f.size() > 4 else yaw, float(f[3]) if f.size() > 3 else 0.15)
			host.call("_frame_camera", 0.0)
			cam.snap()
			for i: int in 3:
				await get_tree().process_frame
	var hv: String = OS.get_environment("DENDRO_PRUNE_HOVER")
	if hv == "":
		return
	set_active(true)
	var p: Vector2 = Vector2(-1, -1)
	if hv.begins_with("auto"):
		var hr: PackedStringArray = hv.split(":")
		var hp: PackedFloat32Array = sim.prune_hint_point(float(hr[1]) if hr.size() > 1 else 0.03, float(hr[2]) if hr.size() > 2 else 0.35)
		if hp[3] >= 0.0:
			p = cam.get_camera().unproject_position(Vector3(hp[0], hp[1], hp[2]))
	else:
		var xy: PackedStringArray = hv.split(",")
		if xy.size() == 2:
			p = Vector2(float(xy[0]), float(xy[1]))
	if p.x >= 0.0:
		_forced_hover = true
		hover_at(p)
		print("[dendro][prune] hover at %s: %s" % [str(p), str(hover.get("preview", {}).get("label", "none"))])
	_refresh_panel()


# ---------------------------------------------------------------------------------------------- self-test

## Screen point for the hover check: a point of a first-order limb (see `prune_hint_point`) where the
## pick ray hits that limb itself; failing that (a dense crown hides every limb behind twigs), the first
## candidate whose hit segment has something to cut (non-empty preview mesh: the tip of a twig, t = 1,
## removes nothing). Candidates: limbs of radius closest to `radius_m` x 1, 1.25, 0.8, 1.5, 0.6, at
## 40, 60, 20 and 80 % of their first segment. Falls back to the first candidate.
func _visible_limb_point(radius_m: float) -> Vector2:
	var c: Camera3D = cam.get_camera()
	var view_h: float = get_viewport().get_visible_rect().size.y
	var tol: float = PICK_TOL_PX * deg_to_rad(OrbitCamera.FOV_DEG) / maxf(view_h, 1.0)
	var first: Vector2 = Vector2(-1, -1)
	var cuttable: Vector2 = Vector2(-1, -1)
	for k: float in [1.0, 1.25, 0.8, 1.5, 0.6]:
		for f: float in [0.4, 0.6, 0.2, 0.8]:
			var hp: PackedFloat32Array = sim.prune_hint_point(radius_m * k, f)
			if hp[3] < 0.0:
				continue
			var sp: Vector2 = c.unproject_position(Vector3(hp[0], hp[1], hp[2]))
			if first.x < 0.0:
				first = sp
			var h: Dictionary = sim.prune_pick(c.project_ray_origin(sp), c.project_ray_normal(sp), tol)
			if not h.get("hit", false):
				continue
			if int(h["node_id"]) == int(hp[3]):
				return sp
			if cuttable.x < 0.0:
				var m: ArrayMesh = sim.build_prune_preview_mesh(int(h["node_id"]), float(h["t"]), false)
				if m != null and m.get_surface_count() > 0:
					cuttable = sp
	return cuttable if cuttable.x >= 0.0 else first


func selftest(check: Callable) -> void:
	var tree: SceneTree = get_tree()
	host.call("_apply_seed", 77)
	host.call("_advance_days_direct", 25.0 * 365.0 + 20.0)
	host.call("_rebuild_meshes", true)
	host.call("_settle_view")
	await tree.process_frame
	# T toggles the tool through the real input path.
	var ev: InputEventKey = InputEventKey.new()
	ev.keycode = KEY_T
	ev.physical_keycode = KEY_T
	ev.pressed = true
	Input.parse_input_event(ev)
	await tree.process_frame
	check.call("prune: key T activates the tool", active and panel.visible and hud.prune_button.button_pressed)
	# Hover on a limb: highlight, marker and tooltip. The target is a point of a ~2 cm limb that the
	# pick ray actually reaches: a point hidden behind a twig would test the twig instead (seen with
	# seed 77: a 0.55 cm twig tip in front of the limb, whose cut removes nothing and has an empty
	# preview mesh). Only the aim changes; the checks below are the same.
	var sp: Vector2 = _visible_limb_point(0.02)
	_forced_hover = true
	hover_at(sp)
	check.call("prune: hover picks a branch and previews the cut", hover.get("hit", false) and preview_mi.visible
		and preview_mi.mesh != null and preview_mi.mesh.get_surface_count() > 0 and _tip.visible and _tip_text.text.contains("Fermeture"),
		"at %s" % str(sp))
	# A click (press + release without drag) cuts.
	var n0: int = sim.get_prune_cut_count()
	var had: bool = hover.get("hit", false)
	if had:
		cut_hovered(false)
	check.call("prune: click cuts and logs it", sim.get_prune_cut_count() == n0 + 1 and sim.get_prune_journal().size() >= 1)
	var fview: Object = (host.get("flux") as Node).get("view")
	check.call("prune: flux paths rebuilt at the cut", fview == null or not bool(fview.get("enabled"))
		or (not bool(fview.get("geometry_dirty")) and float(fview.get("_geo_age")) < 0.5))
	var wm: ArrayMesh = sim.build_wound_mesh()
	check.call("prune: wound mesh drawn", wm.get_surface_count() > 0)
	# Ops and undo.
	var c1: int = sim.get_prune_cut_count()
	run_op("raise", 2.0)
	var c2: int = sim.get_prune_cut_count()
	undo()
	check.call("prune: undo removes exactly the last cut", sim.get_prune_cut_count() == c2 - 1 and c2 >= c1, "%d -> %d -> %d" % [c1, c2, sim.get_prune_cut_count()])
	jump_years(1)
	check.call("prune: +1 an advances one year", absf(sim.get_age_years() - 26.0 - 20.0 / 365.0) < 0.01, "%.3f" % sim.get_age_years())
	# Control tree (never pruned) caught up in the background, one year behind the cuts.
	var frames: int = 0
	var t_frame: int = Time.get_ticks_usec()
	var polled: bool = sim.control_poll(false)
	var poll_ms: float = float(Time.get_ticks_usec() - t_frame) / 1000.0
	while not polled and frames < 900:
		await tree.process_frame
		frames += 1
		polled = sim.control_poll(false)
	var cm: Dictionary = sim.get_control_metrics()
	var lm: Dictionary = sim.get_prune_metrics()
	check.call("prune: control tree computed off the frame", polled and not cm.is_empty() and int(cm["day"]) == sim.get_days_elapsed()
		and poll_ms < 5.0 and absf(float(cm["leaf_area_m2"]) - float(lm["leaf_area_m2"])) > 0.01,
		"%d frames, first poll %.2f ms, leaf %.1f vs control %.1f" % [frames, poll_ms, lm["leaf_area_m2"], cm["leaf_area_m2"]])
	_refresh_panel()
	check.call("prune: journal lists the cut", _journal.text.contains("Journal") and _compare.text.contains("témoin"))
	# The side panel narrows the free view.
	var vs: Vector2 = get_viewport().get_visible_rect().size
	await tree.process_frame
	check.call("prune: panel keeps the tree view clear", hud.free_rect(vs).end.x <= panel.get_global_rect().position.x + 0.5)
	set_active(false)
	check.call("prune: tool off hides the panel", not panel.visible and not preview_mi.visible)
