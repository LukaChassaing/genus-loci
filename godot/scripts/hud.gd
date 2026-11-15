class_name Hud
extends CanvasLayer
## Minimal HUD: info panel (top-left), controls bar (bottom), ring view (right).
## All user-visible text is in French.

signal play_toggled(playing: bool)
signal speed_changed(days_per_second: float)
signal drought_toggled(active: bool)
signal new_seed_requested
signal soil_selected(index: int)
signal rings_toggled(shown: bool)
signal xray_toggled(on: bool)
signal guides_toggled(on: bool)

const MONTHS: Array[String] = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet",
	"août", "septembre", "octobre", "novembre", "décembre"]
const MONTH_DAYS: Array[int] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
const PHASES: Array[String] = ["Dormance hivernale", "Débourrement", "Expansion des feuilles",
	"Été — feuillage mature", "Coloration automnale", "Chute des feuilles"]
const MAX_SPEED_DAYS: float = 3650.0 # 10 years / s

var _age: Label
var _date: Label
var _phase: Label
var _height: Label
var _diam: Label
var _temp: Label
var _root_depth: Label
var _root_spread: Label
var _legend: PanelContainer
var _water: ProgressBar
var _play: Button
var _speed_slider: HSlider
var _speed_label: Label
var _seed_label: Label
var _ring_panel: PanelContainer
var _ring_view: RingView
var _last_ring_year: int = -1
var _info_panel: PanelContainer
var _bottom_panel: PanelContainer
## Controls exposed so the self-test can drive the very same signal paths as a user click.
var drought_button: Button
var seed_button: Button
var rings_button: Button
var xray_button: Button
var guides_button: Button
var play_button: Button
var speed_slider: HSlider


func _ready() -> void:
	var root: Control = Control.new()
	root.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var th: Theme = Theme.new()
	th.default_font_size = 18
	root.theme = th
	add_child(root)
	_build_info_panel(root)
	_build_legend(root)
	_build_ring_panel(root)
	_build_bottom_bar(root)


func _no_focus(c: Control) -> Control:
	c.focus_mode = Control.FOCUS_NONE # keeps Space for play/pause
	return c


func _build_info_panel(root: Control) -> void:
	var p: PanelContainer = PanelContainer.new()
	_info_panel = p
	p.position = Vector2(14, 14)
	p.mouse_filter = Control.MOUSE_FILTER_IGNORE
	root.add_child(p)
	var m: MarginContainer = MarginContainer.new()
	for side: String in ["left", "right", "top", "bottom"]:
		m.add_theme_constant_override("margin_" + side, 10)
	p.add_child(m)
	var v: VBoxContainer = VBoxContainer.new()
	v.add_theme_constant_override("separation", 3)
	m.add_child(v)
	var name_lbl: RichTextLabel = RichTextLabel.new()
	name_lbl.bbcode_enabled = true
	name_lbl.fit_content = true
	name_lbl.scroll_active = false
	name_lbl.custom_minimum_size = Vector2(330, 0)
	name_lbl.add_theme_font_size_override("normal_font_size", 20)
	name_lbl.add_theme_font_size_override("italics_font_size", 20)
	name_lbl.text = "[b]Chêne pédonculé[/b] — [i]Quercus robur[/i]"
	name_lbl.mouse_filter = Control.MOUSE_FILTER_IGNORE
	v.add_child(name_lbl)
	_age = _row(v)
	_date = _row(v)
	_phase = _row(v)
	_height = _row(v)
	_diam = _row(v)
	_temp = _row(v)
	_root_depth = _row(v)
	_root_spread = _row(v)
	var wl: Label = Label.new()
	wl.text = "Eau du sol"
	v.add_child(wl)
	_water = ProgressBar.new()
	_water.min_value = 0.0
	_water.max_value = 1.0
	_water.step = 0.01
	_water.custom_minimum_size = Vector2(0, 18)
	_water.show_percentage = false
	v.add_child(_water)
	var hint: Label = Label.new()
	hint.text = "F cadrer · R racines · X transparence · G repères"
	hint.add_theme_font_size_override("font_size", 13)
	hint.modulate = Color(1, 1, 1, 0.7)
	v.add_child(hint)


func _row(parent: Control) -> Label:
	var l: Label = Label.new()
	parent.add_child(l)
	return l


## Soil-horizon legend (under the info panel), shown in root view / X-ray.
func _build_legend(root: Control) -> void:
	_legend = PanelContainer.new()
	_legend.position = Vector2(14, 14)  # re-placed under the info panel in _process
	_legend.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_legend.visible = false
	root.add_child(_legend)
	var m: MarginContainer = MarginContainer.new()
	for side: String in ["left", "right", "top", "bottom"]:
		m.add_theme_constant_override("margin_" + side, 8)
	_legend.add_child(m)
	var v: VBoxContainer = VBoxContainer.new()
	v.add_theme_constant_override("separation", 3)
	m.add_child(v)
	var t: Label = Label.new()
	t.text = "Horizons du sol"
	t.add_theme_font_size_override("font_size", 16)
	v.add_child(t)
	for i: int in SoilBlock.KIND_NAMES.size():
		var row: HBoxContainer = HBoxContainer.new()
		row.add_theme_constant_override("separation", 8)
		var sw: ColorRect = ColorRect.new()
		sw.color = SoilBlock.KIND_COLORS[i]
		sw.custom_minimum_size = Vector2(18, 14)
		sw.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		row.add_child(sw)
		var l: Label = Label.new()
		l.text = SoilBlock.KIND_NAMES[i]
		l.add_theme_font_size_override("font_size", 15)
		row.add_child(l)
		v.add_child(row)


func set_legend_visible(on: bool) -> void:
	_legend.visible = on


func is_legend_visible() -> bool:
	return _legend.visible


func _process(_delta: float) -> void:
	_legend.position = Vector2(14.0, _info_panel.position.y + _info_panel.size.y + 8.0)


func _build_ring_panel(root: Control) -> void:
	_ring_panel = PanelContainer.new()
	_ring_panel.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	_ring_panel.grow_horizontal = Control.GROW_DIRECTION_BEGIN
	_ring_panel.offset_top = 14
	_ring_panel.offset_right = -14
	_ring_panel.visible = false
	root.add_child(_ring_panel)
	var v: VBoxContainer = VBoxContainer.new()
	_ring_panel.add_child(v)
	var t: Label = Label.new()
	t.text = "Cernes de croissance"
	t.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	v.add_child(t)
	_ring_view = RingView.new()
	v.add_child(_ring_view)


func _build_bottom_bar(root: Control) -> void:
	var p: PanelContainer = PanelContainer.new()
	p.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
	p.grow_vertical = Control.GROW_DIRECTION_BEGIN
	p.offset_left = 14
	p.offset_right = -14
	p.offset_bottom = -14
	_bottom_panel = p
	root.add_child(p)
	var h: HBoxContainer = HBoxContainer.new()
	h.add_theme_constant_override("separation", 6)
	p.add_child(h)

	_play = _no_focus(Button.new()) as Button
	_play.toggle_mode = true
	_play.button_pressed = true
	_play.custom_minimum_size = Vector2(100, 0)
	_play.toggled.connect(_on_play_toggled)
	play_button = _play
	h.add_child(_play)
	_on_play_toggled(true)

	var sl: Label = Label.new()
	sl.text = "Vitesse"
	h.add_child(sl)
	_speed_slider = _no_focus(HSlider.new()) as HSlider
	_speed_slider.min_value = 0.0
	_speed_slider.max_value = 1.0
	_speed_slider.step = 0.001
	_speed_slider.value = log(30.0) / log(MAX_SPEED_DAYS)
	_speed_slider.custom_minimum_size = Vector2(70, 0)
	_speed_slider.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	_speed_slider.value_changed.connect(_on_speed_changed)
	speed_slider = _speed_slider
	h.add_child(_speed_slider)
	_speed_label = Label.new()
	_speed_label.custom_minimum_size = Vector2(88, 0)
	h.add_child(_speed_label)

	var drought: Button = _no_focus(Button.new()) as Button
	drought.text = "Sécheresse"
	drought.toggle_mode = true
	drought_button = drought
	drought.toggled.connect(func(on: bool) -> void: drought_toggled.emit(on))
	h.add_child(drought)

	var seed_btn: Button = _no_focus(Button.new()) as Button
	seed_btn.text = "Nouvelle graine"
	seed_button = seed_btn
	seed_btn.pressed.connect(func() -> void: new_seed_requested.emit())
	h.add_child(seed_btn)
	_seed_label = Label.new()
	h.add_child(_seed_label)

	soil_option = _no_focus(OptionButton.new()) as OptionButton
	soil_option.tooltip_text = "Type de sol (relance la même graine)"
	soil_option.item_selected.connect(func(i: int) -> void: soil_selected.emit(i))
	h.add_child(soil_option)

	var spacer: Control = Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	h.add_child(spacer)

	var xray: Button = _no_focus(Button.new()) as Button
	xray.toggle_mode = true
	xray.text = "Sous-sol : coupe"
	xray_button = xray
	xray.toggled.connect(func(on: bool) -> void:
		xray.text = "Sous-sol : transparence" if on else "Sous-sol : coupe"
		xray_toggled.emit(on))
	h.add_child(xray)

	var guides: Button = _no_focus(Button.new()) as Button
	guides.text = "Repères"
	guides.toggle_mode = true
	guides_button = guides
	guides.toggled.connect(func(on: bool) -> void: guides_toggled.emit(on))
	h.add_child(guides)

	var rings: Button = _no_focus(Button.new()) as Button
	rings.text = "Cernes"
	rings.toggle_mode = true
	rings_button = rings
	rings.toggled.connect(func(on: bool) -> void:
		_ring_panel.visible = on
		rings_toggled.emit(on))
	h.add_child(rings)
	_on_speed_changed(_speed_slider.value)


func set_rings_visible(on: bool) -> void:
	rings_button.button_pressed = on  # fires the same toggled handler as a click


func is_rings_visible() -> bool:
	return _ring_panel.visible


func get_ring_view() -> RingView:
	return _ring_view


## Part of the viewport (canvas units) not covered by HUD panels: between the info panel and the
## ring panel (when shown), above the control bar. The 3D camera frames the tree inside it.
func free_rect(view: Vector2) -> Rect2:
	var left: float = _info_panel.position.x + _info_panel.size.x + 12.0
	var right: float = view.x - 14.0
	if _ring_panel.visible:
		right = _ring_panel.global_position.x - 12.0
	var top: float = 14.0
	var bottom: float = _bottom_panel.global_position.y - 10.0
	return Rect2(left, top, maxf(right - left, 200.0), maxf(bottom - top, 200.0))


func reset_drought_button() -> void:
	if drought_button.button_pressed:
		drought_button.button_pressed = false


func set_playing(on: bool) -> void:
	_play.button_pressed = on


func is_playing() -> bool:
	return _play.button_pressed


func set_seed_text(seed_value: int) -> void:
	_seed_label.text = "n° %d" % seed_value


func _on_play_toggled(on: bool) -> void:
	_play.text = "Pause" if on else "Lecture"
	play_toggled.emit(on)


func _on_speed_changed(v: float) -> void:
	var dps: float = pow(MAX_SPEED_DAYS, v)
	_speed_label.text = _speed_text(dps)
	speed_changed.emit(dps)


static func _speed_text(dps: float) -> String:
	if dps < 28.0:
		var n: int = maxi(1, roundi(dps))
		return "%d jour/s" % n if n == 1 else "%d jours/s" % n
	if dps < 330.0:
		var mo: int = maxi(1, roundi(dps / 30.4))
		return "%d mois/s" % mo
	var yr: float = dps / 365.0
	if yr < 9.5:
		var s: String = ("%.1f" % yr).replace(".", ",")
		if s.ends_with(",0"):
			s = s.trim_suffix(",0")
		return s + (" an/s" if yr < 1.5 else " ans/s")
	return "%d ans/s" % roundi(yr)


static func date_text(day_of_year: int) -> String:
	var d: int = day_of_year
	for i: int in 12:
		if d <= MONTH_DAYS[i]:
			return "%d %s" % [d, MONTHS[i]]
		d -= MONTH_DAYS[i]
	return "31 décembre"


## Refreshes the labels; the ring view is redrawn only when the year count changes.
func update_from(sim: TreeSim) -> void:
	var age: float = sim.get_age_years()
	_age.text = "Âge : %s ans" % ("%.1f" % age).replace(".", ",")
	var year: int = sim.get_year()
	_date.text = "Date : %s — année %d" % [date_text(sim.get_day_of_year()), year]
	_phase.text = "Saison : " + PHASES[clampi(sim.get_phase(), 0, 5)]
	_height.text = "Hauteur : %s m" % ("%.1f" % sim.get_height_m()).replace(".", ",")
	_diam.text = "Diamètre du tronc : %s cm" % ("%.1f" % sim.get_trunk_diameter_cm()).replace(".", ",")
	_root_depth.text = "Profondeur des racines : %s m" % ("%.1f" % sim.get_root_depth_m()).replace(".", ",")
	_root_spread.text = "Étalement des racines : %s m" % ("%.1f" % sim.get_root_spread_m()).replace(".", ",")
	_temp.text = "Température : %s °C" % ("%.1f" % sim.get_temperature_c()).replace(".", ",")
	_water.value = sim.get_soil_water()
	if year != _last_ring_year:
		_last_ring_year = year
		_ring_view.set_data(sim.get_ring_widths(), sim.get_ring_stress(),
				sim.get_ring_earlywood_fraction(), sim.get_ring_forced_drought())


func force_ring_refresh() -> void:
	_last_ring_year = -1
	_ring_view.reset_view()


var soil_option: OptionButton

const SOIL_LABELS: Array[String] = ["Limon profond", "Mince sur calcaire", "Argile engorgée", "Sable sec"]


## Fills the soil selector; presets beyond the known French labels keep the core's own name.
func set_soil_names(names: PackedStringArray) -> void:
	soil_option.clear()
	for i: int in names.size():
		soil_option.add_item(SOIL_LABELS[i] if i < SOIL_LABELS.size() else names[i])


func select_soil(index: int) -> void:
	if index >= 0 and index < soil_option.item_count:
		soil_option.select(index)
