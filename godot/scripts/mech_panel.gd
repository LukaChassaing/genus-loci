class_name MechPanel
extends PanelContainer
## « Mécanique » panel (key M): static mechanics of the tree under a steady gust (TreeSim.get_mechanics, see
## dendro_core::mechanics): wind slider 0-50 m/s with km/h and Beaufort, lowest safety factor, weak point, expected
## failure, critical speed of each failure mode, masses and moments. While it is open the wood can be drawn coloured
## by safety factor (red <= 1, green >= 4) instead of bark. A dockable side panel (Hud.layout_side_panels).
## The mechanics is a reading of the tree: nothing breaks.

signal tint_changed(on: bool)

const MAX_WIND: float = 50.0
## Upper limits (m/s) of Beaufort forces 0..11 (WMO; force 12 from 32.7 m/s). Beaufort is defined on the
## 10-minute mean wind; the slider speed is a gust: the label is only an order of magnitude.
const BEAUFORT: Array[float] = [0.5, 1.5, 3.3, 5.5, 7.9, 10.7, 13.8, 17.1, 20.7, 24.4, 28.4, 32.6]
## Cost: a recomputation (model + critical-speed search, ~40-80 ms on a 150-year oak) and the tinted wood mesh
## (~100 ms) run on the main thread. While time runs the panel keeps the values of the last pause (no hitch every
## second in time-lapse); it recomputes on pause, on opening, on a slider move and after a cut, undo or load.
const ROLE_LABELS: Array[String] = ["tronc", "charpentière", "rameau"]

var sim: TreeSim
var host: Node                 # main scene (reads its `playing` flag)
var wind: float = 25.0
var tint_on: bool = true
var values: Dictionary = {}
var slider: HSlider
var tint_check: CheckBox
var _wind_l: Label
var _rows: Dictionary = {}
var _foot: Label
var _age: float = 10.0
var _key: Array = []
var _dirty: bool = true
var _stale_note: bool = false  # time runs and the values are those of the last pause
var mesh_dirty: bool = true   # the tinted wood mesh must be rebuilt (read and cleared by main)
var _head: Button
var _body: VBoxContainer       # everything below the slider (hidden when folded)
var _summary: Label            # one-line summary shown when folded
var user_collapsed: bool = false
var auto_collapsed: bool = false
var docked_out: bool = false   # no room left in the column even folded: parked off screen by the dock
var full_height: float = 0.0   # height when expanded, measured the last time it was (for the dock's budget)


func _init(s: TreeSim) -> void:
	sim = s


func _ready() -> void:
	theme = UiStyle.make_theme(14, 6.0, 2.0)
	add_theme_stylebox_override("panel", UiStyle.panel_box(0.84))
	mouse_filter = Control.MOUSE_FILTER_STOP
	var m: MarginContainer = MarginContainer.new()
	for side: String in ["left", "right", "top", "bottom"]:
		m.add_theme_constant_override("margin_" + side, 8)
	add_child(m)
	var col: VBoxContainer = VBoxContainer.new()
	col.add_theme_constant_override("separation", 3)
	m.add_child(col)
	_head = Button.new()
	_head.flat = true
	_head.alignment = HORIZONTAL_ALIGNMENT_LEFT
	_head.focus_mode = Control.FOCUS_NONE
	_head.add_theme_font_size_override("font_size", 12)
	_head.add_theme_color_override("font_color", UiStyle.TEXT_DIM)
	_head.tooltip_text = "Replier / déplier le panneau de mécanique"
	_head.pressed.connect(func() -> void: set_user_collapsed(not user_collapsed))
	col.add_child(_head)
	_wind_l = Label.new()
	_wind_l.add_theme_font_size_override("font_size", 14)
	col.add_child(_wind_l)
	slider = HSlider.new()
	slider.focus_mode = Control.FOCUS_NONE
	slider.min_value = 0.0
	slider.max_value = MAX_WIND
	slider.step = 0.5
	slider.value = wind
	slider.custom_minimum_size = Vector2(300, 0)
	slider.tooltip_text = "Rafale à 10 m de haut, appliquée comme une charge constante"
	slider.value_changed.connect(func(v: float) -> void: set_wind(v))
	col.add_child(slider)
	_summary = Label.new()
	_summary.add_theme_font_size_override("font_size", 13)
	col.add_child(_summary)
	_body = VBoxContainer.new()
	_body.add_theme_constant_override("separation", 3)
	col.add_child(_body)
	var grid: GridContainer = GridContainer.new()
	grid.columns = 2
	grid.add_theme_constant_override("h_separation", 10)
	grid.add_theme_constant_override("v_separation", 1)
	_body.add_child(grid)
	for r: Array in [["safety", "Sécurité minimale"], ["weak", "Point faible"], ["mode", "Ruine attendue"],
			["c_stem", "Vent critique · tronc"], ["c_limb", "· charpentière"], ["c_uproot", "· déracinement"],
			["mass", "Masse aérienne"], ["moment", "Moment au collet"], ["drag", "Poussée du vent"]]:
		var k: Label = Label.new()
		k.text = r[1]
		k.add_theme_font_size_override("font_size", 13)
		k.add_theme_color_override("font_color", UiStyle.TEXT_DIM)
		var v: Label = Label.new()
		v.add_theme_font_size_override("font_size", 13)
		v.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		v.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		_rows[r[0]] = v
		grid.add_child(k)
		grid.add_child(v)
	var legend: HBoxContainer = HBoxContainer.new()
	legend.add_theme_constant_override("separation", 4)
	tint_check = CheckBox.new()
	tint_check.focus_mode = Control.FOCUS_NONE
	tint_check.text = "Colorer le bois"
	tint_check.button_pressed = tint_on
	tint_check.add_theme_font_size_override("font_size", 13)
	tint_check.tooltip_text = "Facteur de sécurité de chaque section : rouge ≤ 1 (rompt), vert ≥ 4"
	tint_check.toggled.connect(func(on: bool) -> void:
		tint_on = on
		mesh_dirty = true
		tint_changed.emit(on))
	legend.add_child(tint_check)
	for c: Array in [[1.0, "1"], [1.5, "1,5"], [2.0, "2"], [4.0, "4+"]]:
		var sw: ColorRect = ColorRect.new()
		sw.color = safety_color(c[0])
		sw.custom_minimum_size = Vector2(14, 8)
		sw.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		legend.add_child(sw)
		var l: Label = Label.new()
		l.text = c[1]
		l.add_theme_font_size_override("font_size", 12)
		legend.add_child(l)
	_body.add_child(legend)
	_foot = Label.new()
	_foot.add_theme_font_size_override("font_size", 11)
	_foot.add_theme_color_override("font_color", UiStyle.TEXT_DIM)
	_foot.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_foot.custom_minimum_size = Vector2(300, 0)
	_body.add_child(_foot)
	_update_wind_label()
	_apply_collapse()


func is_collapsed() -> bool:
	return user_collapsed or auto_collapsed


func set_user_collapsed(on: bool) -> void:
	user_collapsed = on
	_apply_collapse()


func set_auto_collapsed(on: bool) -> void:
	if on != auto_collapsed:
		auto_collapsed = on
		_apply_collapse()


func _apply_collapse() -> void:
	var c: bool = is_collapsed()
	_body.visible = not c
	_summary.visible = c
	_head.text = ("▸ " if c else "▾ ") + "MÉCANIQUE · VENT CONSTANT"


## Display colour of a safety factor (same stops as dendro_core::mechanics::safety_color).
static func safety_color(sf: float) -> Color:
	var stops: Array = [[1.0, Color(0.86, 0.12, 0.10)], [1.5, Color(0.95, 0.50, 0.10)], [2.0, Color(0.95, 0.85, 0.20)],
		[4.0, Color(0.25, 0.72, 0.30)]]
	if sf <= 1.0:
		return stops[0][1]
	for i: int in range(1, stops.size()):
		if sf <= stops[i][0]:
			var t: float = (sf - stops[i - 1][0]) / (stops[i][0] - stops[i - 1][0])
			return (stops[i - 1][1] as Color).lerp(stops[i][1], t)
	return stops[3][1]


static func beaufort(ms: float) -> int:
	for i: int in BEAUFORT.size():
		if ms < BEAUFORT[i]:
			return i
	return 12


static func num(v: float, digits: int = 1) -> String:
	return (("%." + str(digits) + "f") % v).replace(".", ",")


static func speed_text(ms: float) -> String:
	if ms < 0.0:
		return "> 80 m/s"
	return "%s m/s (%d km/h)" % [num(ms), roundi(ms * 3.6)]


func set_wind(v: float) -> void:
	wind = clampf(v, 0.0, MAX_WIND)
	if not is_equal_approx(slider.value, wind):
		slider.set_value_no_signal(wind)
	_update_wind_label()
	_dirty = true


func _update_wind_label() -> void:
	if _wind_l != null:
		_wind_l.text = "Rafale : %s m/s · %d km/h · ≈ Beaufort %d" % [num(wind), roundi(wind * 3.6), beaufort(wind)]


## Recomputes now (tests, screenshots).
func refresh() -> void:
	_dirty = true
	_process_panel(0.0, false)


func _process(delta: float) -> void:
	var t0: int = Time.get_ticks_usec()
	var playing: bool = host != null and bool(host.get("playing"))
	_process_panel(delta, playing)
	Prof.add("gd.mech_panel", t0)


func _process_panel(delta: float, playing: bool) -> void:
	_age += delta
	if not visible:
		return
	var key: Array = [sim.get_age_years(), sim.get_wood_version()]
	if key != _key and not playing:
		_dirty = true  # the tree changed: recompute once time stops
	if _stale_note != (playing and key != _key):
		_stale_note = playing and key != _key
		_show(values)
	if not _dirty:
		return
	_dirty = false
	_age = 0.0
	_key = key
	values = sim.get_mechanics(wind)
	mesh_dirty = true
	_show(values)


func _show(d: Dictionary) -> void:
	var g: Callable = func(k: String) -> float: return float(d.get(k, -1.0))
	var sf: float = g.call("min_safety")
	_rows["safety"].text = ("≥ 999" if sf >= 999.0 else num(sf, 2)) + ("  — rupture !" if sf < 1.0 else "")
	_rows["safety"].add_theme_color_override("font_color", safety_color(sf))
	if d.has("weak_role"):
		var role: String = ROLE_LABELS[clampi(int(d["weak_role"]), 0, 2)]
		_rows["weak"].text = "%s à %s m, Ø %s cm%s" % [role, num(g.call("weak_height_m")), num(g.call("weak_diameter_cm"), 0),
			" (carié)" if bool(d.get("weak_decayed", false)) else ""]
	else:
		_rows["weak"].text = "—"
	_rows["mode"].text = String(d.get("expected_label", ""))
	_rows["c_stem"].text = speed_text(g.call("critical_stem"))
	_rows["c_limb"].text = speed_text(g.call("critical_limb"))
	_rows["c_uproot"].text = speed_text(g.call("critical_uproot"))
	var first: String = String(d.get("first_label", ""))
	for k: String in ["c_stem", "c_limb", "c_uproot"]:
		(_rows[k] as Label).add_theme_color_override("font_color", UiStyle.TEXT)
	var first_key: String = ["c_stem", "c_limb", "c_uproot"][clampi(int(d.get("first_mode", 0)), 0, 2)]
	if first != "":
		(_rows[first_key] as Label).add_theme_color_override("font_color", UiStyle.ACCENT)
	var t: float = g.call("mass_kg") / 1000.0
	_rows["mass"].text = ("%s t" % num(t, 2)) if t < 10.0 else ("%s t" % num(t, 1))
	_rows["moment"].text = "%s / %s kN·m" % [num(g.call("base_moment_knm"), 0), num(g.call("critical_moment_knm"), 0)]
	_rows["drag"].text = "%s kN à %s m" % [num(g.call("drag_kn")), num(g.call("pressure_height_m"))]
	var first_v: float = g.call("first_speed")
	_summary.text = "Sécurité %s · 1re ruine : %s" % ["≥ 999" if sf >= 999.0 else num(sf, 2),
		(first + " à " + speed_text(first_v)) if first != "" else "aucune sous 80 m/s"]
	_summary.add_theme_color_override("font_color", safety_color(sf))
	var leaf: int = roundi(g.call("leaf_fraction") * 100.0)
	_foot.text = ("Valeurs de l'arbre à la dernière pause (recalculées à l'arrêt). " if _stale_note else "") + ("Feuillage %d %% · rafale à 10 m appliquée en charge statique (sans balancement) ; " % leaf
		+ "le moment au collet est comparé à l'ancrage de la motte. Premier mode : %s." % (first if first != "" else "aucun sous 80 m/s"))
