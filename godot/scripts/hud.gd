class_name Hud
extends CanvasLayer
## Minimal HUD: info panel (top-left), controls bar (bottom), ring view (right).
## All user-visible text is in French.

signal play_toggled(playing: bool)
signal speed_changed(days_per_second: float)
signal drought_toggled(active: bool)
signal new_seed_requested
signal soil_selected(index: int)
signal species_selected(index: int)
signal rings_toggled(shown: bool)
signal soil_opacity_changed(opacity: float)
signal flux_toggled(on: bool)
signal guides_toggled(on: bool)
signal prune_toggled(on: bool)
signal mech_toggled(on: bool)

const MONTHS: Array[String] = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet",
	"août", "septembre", "octobre", "novembre", "décembre"]
const MONTH_DAYS: Array[int] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
const PHASES: Array[String] = ["Dormance hivernale", "Débourrement", "Expansion des feuilles",
	"Été — feuillage mature", "Coloration automnale", "Chute des feuilles"]
## The same phases for an evergreen (the « autumn » is the cast of the oldest needle cohort).
const PHASES_EVERGREEN: Array[String] = ["Repos hivernal — feuillage persistant", "Débourrement", "Allongement des pousses",
	"Été — feuillage mature", "Automne — les plus vieilles aiguilles jaunissent", "Chute des plus vieilles aiguilles"]
const MAX_SPEED_DAYS: float = 3650.0 # 10 years / s

var _age: Label
var _date: Label
var _phase: Label
var _name_lbl: RichTextLabel
var _height: Label
var _diam: Label
var _state: Label   # « État » : vigour, reserves, death of the trunk (dendro_core::vitality)
var _state_key: int = -1
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
var soil_slider: HSlider     # soil opacity 0..1 ("Sol")
var flux_button: Button      # water / nutrient flux on the roots ("Flux")
var guides_button: Button
var prune_button: Button     # pruning tool (« Taille », key T)
var mech_button: Button      # mechanics panel (« Mécanique », key M)
## Extra right-hand panels (pruning journal...) that the 3-D view must avoid, see free_rect.
var side_panels: Array[Control] = []
var play_button: Button
var speed_slider: HSlider


func _ready() -> void:
	var root: Control = Control.new()
	root.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	root.theme = UiStyle.make_theme(18)  # serif, dark translucent panels, thin light borders
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
	_name_lbl = name_lbl
	v.add_child(name_lbl)
	_age = _row(v)
	_date = _row(v)
	_phase = _row(v)
	_height = _row(v)
	_diam = _row(v)
	_state = _row(v)
	_state.clip_text = true  # its changing width (reserves %) must not resize the panel (camera free area)
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
	hint.text = "F cadrer · R racines · X sol transparent · G repères · T taille"
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


## Bottom bar, two rows (fits 1280 px with room to spare):
##   1. time and tools: play, speed, drought | Taille, Repères, Cernes, Flux, Sol opacity
##   2. the site: species, soil type, seed (these restart the simulation) | Mécanique (the milieu's loads)
func _build_bottom_bar(root: Control) -> void:
	var p: PanelContainer = PanelContainer.new()
	p.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
	p.grow_vertical = Control.GROW_DIRECTION_BEGIN
	p.offset_left = 14
	p.offset_right = -14
	p.offset_bottom = -14
	_bottom_panel = p
	root.add_child(p)
	var bar_theme: Theme = UiStyle.make_theme(15, 8.0, 3.0)
	p.theme = bar_theme
	var rows: VBoxContainer = VBoxContainer.new()
	rows.add_theme_constant_override("separation", 5)
	p.add_child(rows)
	var h: HBoxContainer = HBoxContainer.new()   # row 1: time and tools
	h.add_theme_constant_override("separation", 4)
	rows.add_child(h)
	var h2: HBoxContainer = HBoxContainer.new()  # row 2: the site
	h2.add_theme_constant_override("separation", 4)
	rows.add_child(h2)

	_play = _no_focus(Button.new()) as Button
	_play.toggle_mode = true
	_play.button_pressed = true
	_play.custom_minimum_size = Vector2(86, 0)
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
	_speed_slider.custom_minimum_size = Vector2(150, 0)
	_speed_slider.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	_speed_slider.value_changed.connect(_on_speed_changed)
	speed_slider = _speed_slider
	h.add_child(_speed_slider)
	_speed_label = Label.new()
	_speed_label.custom_minimum_size = Vector2(84, 0)
	h.add_child(_speed_label)

	var drought: Button = _no_focus(Button.new()) as Button
	drought.text = "Sécheresse"
	drought.toggle_mode = true
	drought.tooltip_text = "Sécheresse forcée : ni pluie, évaporation accrue"
	drought_button = drought
	drought.toggled.connect(func(on: bool) -> void: drought_toggled.emit(on))
	h.add_child(drought)

	var spacer: Control = Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	h.add_child(spacer)

	prune_button = _no_focus(Button.new()) as Button
	prune_button.text = "✂ Taille"
	prune_button.toggle_mode = true
	prune_button.tooltip_text = "Outil de taille (T) : survoler une branche, clic pour couper"
	prune_button.toggled.connect(func(on: bool) -> void: prune_toggled.emit(on))
	h.add_child(prune_button)

	var guides: Button = _no_focus(Button.new()) as Button
	guides.text = "Repères"
	guides.toggle_mode = true
	guides.tooltip_text = "Échelles de hauteur et de profondeur (G)"
	guides_button = guides
	guides.toggled.connect(func(on: bool) -> void: guides_toggled.emit(on))
	h.add_child(guides)

	var rings: Button = _no_focus(Button.new()) as Button
	rings.text = "Cernes"
	rings.toggle_mode = true
	rings.tooltip_text = "Coupe du tronc à 1,3 m : cernes annuels"
	rings_button = rings
	rings.toggled.connect(func(on: bool) -> void:
		_ring_panel.visible = on
		rings_toggled.emit(on))
	h.add_child(rings)

	flux_button = _no_focus(Button.new()) as Button
	flux_button.text = "Flux"
	flux_button.toggle_mode = true
	flux_button.button_pressed = true
	flux_button.tooltip_text = "Eau et sels minéraux absorbés par les racines"
	flux_button.toggled.connect(func(on: bool) -> void: flux_toggled.emit(on))
	h.add_child(flux_button)

	var soil_lbl: Label = Label.new()
	soil_lbl.text = "Sol"
	h.add_child(soil_lbl)
	soil_slider = _no_focus(HSlider.new()) as HSlider
	soil_slider.min_value = 0.0
	soil_slider.max_value = 1.0
	soil_slider.step = 0.01
	soil_slider.value = 1.0
	soil_slider.tooltip_text = "Opacité du sol (X : opaque / transparent)"
	soil_slider.custom_minimum_size = Vector2(90, 0)
	soil_slider.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	soil_slider.value_changed.connect(func(v: float) -> void: soil_opacity_changed.emit(v))
	h.add_child(soil_slider)

	# Row 2: the site (each choice restarts the same seed).
	var sp_lbl: Label = Label.new()
	sp_lbl.text = "Espèce"
	sp_lbl.modulate = Color(1, 1, 1, 0.8)
	h2.add_child(sp_lbl)
	species_option = _no_focus(OptionButton.new()) as OptionButton
	species_option.tooltip_text = "Espèce (relance la même graine)"
	species_option.clip_text = true
	species_option.fit_to_longest_item = false
	species_option.custom_minimum_size = Vector2(190, 0)
	# Items carry the species index as their id (category headers are separators).
	species_option.item_selected.connect(func(i: int) -> void: species_selected.emit(species_option.get_item_id(i)))
	h2.add_child(species_option)

	var so_lbl: Label = Label.new()
	so_lbl.text = "Terrain"
	so_lbl.modulate = Color(1, 1, 1, 0.8)
	h2.add_child(so_lbl)
	soil_option = _no_focus(OptionButton.new()) as OptionButton
	soil_option.tooltip_text = "Type de sol (relance la même graine)"
	soil_option.clip_text = true
	soil_option.fit_to_longest_item = false
	soil_option.custom_minimum_size = Vector2(200, 0)
	soil_option.item_selected.connect(func(i: int) -> void: soil_selected.emit(i))
	h2.add_child(soil_option)

	var seed_btn: Button = _no_focus(Button.new()) as Button
	seed_btn.text = "Nouvelle graine"
	seed_btn.tooltip_text = "Un autre individu (autre graine, même site)"
	seed_button = seed_btn
	seed_btn.pressed.connect(func() -> void: new_seed_requested.emit())
	h2.add_child(seed_btn)
	_seed_label = Label.new()
	_seed_label.modulate = Color(1, 1, 1, 0.8)
	h2.add_child(_seed_label)
	# The milieu put to the test (M1): on the site row, right-aligned (row 1 has no room left at 1280 px x 1.3).
	var spacer2: Control = Control.new()
	spacer2.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	h2.add_child(spacer2)
	mech_button = _no_focus(Button.new()) as Button
	mech_button.text = "Mécanique"
	mech_button.toggle_mode = true
	mech_button.tooltip_text = "Tenue au vent et au poids propre, vitesses critiques (M)"
	mech_button.toggled.connect(func(on: bool) -> void: mech_toggled.emit(on))
	h2.add_child(mech_button)
	_on_speed_changed(_speed_slider.value)


## Soil opacity 0..1 through the slider (same signal path as a drag).
func set_soil_opacity(v: float) -> void:
	soil_slider.value = clampf(v, 0.0, 1.0)


func set_rings_visible(on: bool) -> void:
	rings_button.button_pressed = on  # fires the same toggled handler as a click


func is_rings_visible() -> bool:
	return _ring_panel.visible


func get_ring_view() -> RingView:
	return _ring_view


## Part of the viewport (canvas units) not covered by HUD panels: between the info panel and the
## ring panel (when shown), above the control bar. The 3D camera frames the tree inside it.
## free_rect for the per-frame camera framing: the side-column layout (several ms of Control minimum-size
## queries) is redone at once when the view size, the interface scale or a panel's visibility changes, and
## otherwise at most every FREE_RECT_REFRESH_MS (panel contents grow or shrink slowly: text updates).
const FREE_RECT_REFRESH_MS: int = 200
var _free_cache: Rect2
var _free_key: Array = []
var _free_time: int = -1000000


func free_rect_cached(view: Vector2) -> Rect2:
	var key: Array = [view, get_window().content_scale_factor, _ring_panel.visible, _info_panel.size,
		_bottom_panel.global_position.y]
	for sp: Control in side_panels:
		key.append(is_instance_valid(sp) and sp.visible)
	var now: int = Time.get_ticks_msec()
	if key != _free_key or now - _free_time >= FREE_RECT_REFRESH_MS:
		_free_key = key
		_free_time = now
		_free_cache = free_rect(view)
	return _free_cache


func free_rect(view: Vector2) -> Rect2:
	var left: float = _info_panel.position.x + _info_panel.size.x + 12.0
	var top: float = 14.0
	var bottom: float = _bottom_panel.global_position.y - 10.0
	var right: float = layout_side_panels(view, top, bottom)
	return Rect2(left, top, maxf(right - left, 200.0), maxf(bottom - top, 200.0))


const SIDE_GAP: float = 8.0
const SIDE_EDGE: float = 14.0
const RING_MAX: float = 360.0   # ring disc area (px) at natural size and when the dock is squeezed
const RING_MIN: float = 60.0
const RING_EXTRA: float = 60.0  # text strip under the disc


## Docks the visible side panels (ring view, pruning panel, flux panel, mechanics panel: in that order) in one right-hand column,
## top to bottom, same width, no overlap. When the column would be taller than [top, bottom] it is compressed in
## steps: the flux panel folds into its legend strip and the mechanics panel to its slider and summary line, then
## the ring disc shrinks, then the pruning panel's scrolling text area; as a last resort they leave the column.
## Returns the x of the column's left edge minus a gap (the right limit of the free 3-D area).
func layout_side_panels(view: Vector2, top: float, bottom: float) -> float:
	var stack: Array[Control] = []
	if _ring_panel.visible:
		stack.append(_ring_panel)
	for sp: Control in side_panels:
		if is_instance_valid(sp) and sp.visible:
			stack.append(sp)
	if stack.is_empty():
		return view.x - SIDE_EDGE
	var avail: float = bottom - top - SIDE_GAP * float(stack.size() - 1)
	# Foldable panels (flux, then mechanics): fold first, leave the column last. Both expose is_collapsed,
	# set_auto_collapsed, full_height and docked_out.
	var foldable: Array[Control] = []
	var other: Array[Control] = []
	for sp: Control in stack:
		if sp is FluxPanel or sp is MechPanel:
			foldable.append(sp)
		else:
			other.append(sp)
	# 1. Everything at natural size; the foldable panels fold, in order, while the whole does not fit.
	_ring_view.custom_minimum_size = Vector2(RING_MAX, RING_MAX + RING_EXTRA)
	for sp: Control in other:
		_dock(sp)
		if sp.has_meta("dock_fit"):
			(sp.get_meta("dock_fit") as Callable).call(100000.0)
	var other_h: float = 0.0
	for sp: Control in other:
		other_h += sp.get_combined_minimum_size().y
	var fold_h: float = 0.0
	for f: Control in foldable:
		_dock(f)
		f.call("set_auto_collapsed", false)
		var fh: float = f.get_combined_minimum_size().y
		f.set("full_height", fh)
		fold_h += fh
	for f: Control in foldable:
		if other_h + fold_h > avail:
			var before: float = f.get_combined_minimum_size().y
			f.call("set_auto_collapsed", true)
			fold_h += f.get_combined_minimum_size().y - before
	other_h += fold_h
	# 2. Still too tall: shrink the ring disc, then the pruning panel's text area.
	var over: float = other_h - avail
	if over > 0.0 and _ring_panel.visible:
		var cut: float = minf(over, RING_MAX - RING_MIN)
		_ring_view.custom_minimum_size = Vector2(RING_MAX - cut, RING_MAX - cut + RING_EXTRA)
		over -= cut
	if over > 0.0:
		for sp: Control in other:
			if sp.has_meta("dock_fit"):
				var h: float = sp.get_combined_minimum_size().y
				(sp.get_meta("dock_fit") as Callable).call(h - over)
	# 3. Still too tall (large interface scale): foldable panels leave the column, the flux legend first.
	var rest: float = 0.0
	for sp: Control in other:
		rest += sp.get_combined_minimum_size().y
	var kept: float = 0.0
	for f: Control in foldable:
		kept += f.get_combined_minimum_size().y
	for f: Control in foldable:
		var out_of_room: bool = rest + kept > avail + 0.5
		f.set("docked_out", out_of_room)
		if out_of_room:
			kept -= f.get_combined_minimum_size().y
			stack.erase(f)
			f.position = Vector2(-10000.0, -10000.0)
	# 4. Stack: one width for the whole column.
	var w: float = 0.0
	for sp: Control in stack:
		w = maxf(w, sp.get_combined_minimum_size().x)
	var y: float = top
	for sp: Control in stack:
		var sz: Vector2 = Vector2(w, sp.get_combined_minimum_size().y)
		sp.size = sz
		sp.position = Vector2(view.x - SIDE_EDGE - w, y)
		y += sz.y + SIDE_GAP
	return view.x - SIDE_EDGE - w - 12.0


func _dock(c: Control) -> void:
	if not c.has_meta("docked"):
		c.set_meta("docked", true)
		c.set_anchors_preset(Control.PRESET_TOP_LEFT)
		c.grow_horizontal = Control.GROW_DIRECTION_END
		c.grow_vertical = Control.GROW_DIRECTION_END


## Screen rectangles (canvas units) of the visible docked side panels (tests).
func side_panel_rects() -> Array[Rect2]:
	var out: Array[Rect2] = []
	if _ring_panel.visible:
		out.append(Rect2(_ring_panel.position, _ring_panel.size))
	for sp: Control in side_panels:
		if is_instance_valid(sp) and sp.visible and not ((sp is FluxPanel or sp is MechPanel) and bool(sp.get("docked_out"))):
			out.append(Rect2(sp.global_position, sp.size))
	return out


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
	if sim.get_trunk_alive():
		_phase.text = "Saison : " + (PHASES_EVERGREEN if sim.get_evergreen() else PHASES)[clampi(sim.get_phase(), 0, 5)]
	else:
		_phase.text = "Saison : — (tronc mort)"
	_height.text = "Hauteur : %s m" % ("%.1f" % sim.get_height_m()).replace(".", ",")
	_diam.text = "Diamètre du tronc : %s cm" % ("%.1f" % sim.get_trunk_diameter_cm()).replace(".", ",")
	# Refreshed once a simulated year, or when the trunk dies or the stem falls (the vitality changes only at
	# the end of a season; `get_vitality` builds a dictionary, not worth doing every frame).
	var vkey: int = sim.get_year() * 4 + (2 if sim.get_trunk_alive() else 0) + (1 if sim.get_height_m() > 0.5 else 0)
	if vkey != _state_key:
		_state_key = vkey
		var st: String = vitality_text(sim.get_vitality())
		_state.text = st if st != "" else "État : —"
	_root_depth.text = "Profondeur des racines : %s m" % ("%.1f" % sim.get_root_depth_m()).replace(".", ",")
	_root_spread.text = "Étalement des racines : %s m" % ("%.1f" % sim.get_root_spread_m()).replace(".", ",")
	_temp.text = "Température : %s °C" % ("%.1f" % sim.get_temperature_c()).replace(".", ",")
	_water.value = sim.get_soil_water()
	if year != _last_ring_year:
		_last_ring_year = year
		_ring_view.contrast = sim.get_ring_contrast()
		_ring_view.set_data(sim.get_ring_widths(), sim.get_ring_stress(),
				sim.get_ring_earlywood_fraction(), sim.get_ring_forced_drought())


## « État » line from `TreeSim.get_vitality()`: vigour class and reserves while the trunk lives; once it is dead,
## age and cause of death, the standing dead stem (snag) and its expected collapse, stump sprouts. "" when the
## species has no carbon balance (genome without `maintenance_per_m`). The vigour classes are display
## conventions (thresholds of the vigour index and reserves), not published categories.
static func vitality_text(d: Dictionary) -> String:
	if not bool(d.get("modelled", false)):
		return ""
	var death: int = int(d.get("death_age", -1))
	if death < 0:
		var r: float = float(d.get("reserve", 1.0))
		var v: float = float(d.get("vigour", 1.0))
		var cls: String = "vigoureux"
		if v < 0.35 or r < 0.25:
			cls = "dépérissant"
		elif v < 0.7 or r < 0.6:
			cls = "en déclin"
		return "État : %s — réserves %d %%" % [cls, roundi(100.0 * r)]
	var cause: String = String(d.get("cause", ""))
	var txt: String = "État : tronc mort à %d ans" % death
	if cause != "":
		txt += "\nCause : " + cause
	if bool(d.get("standing_dead", false)):
		var left: int = int(d.get("collapse_in", -1))
		txt += "\nMort sur pied" + (", chute dans ~%d ans" % left if left > 0 else "")
	elif bool(d.get("collapsed", false)):
		txt += "\nTronc effondré"
	if bool(d.get("resprouted", false)):
		txt += "\nRejets de souche" + (" vivants" if bool(d.get("alive", false)) else " morts")
	return txt


func force_ring_refresh() -> void:
	_last_ring_year = -1
	_state_key = -1
	_ring_view.reset_view()


var soil_option: OptionButton
var species_option: OptionButton
var _species_common: PackedStringArray = PackedStringArray()
var _species_sci: PackedStringArray = PackedStringArray()

const SOIL_LABELS: Array[String] = ["Limon profond", "Mince sur calcaire", "Argile engorgée", "Sable sec"]


## Fills the soil selector; presets beyond the known French labels keep the core's own name.
func set_soil_names(names: PackedStringArray) -> void:
	soil_option.clear()
	for i: int in names.size():
		soil_option.add_item(SOIL_LABELS[i] if i < SOIL_LABELS.size() else names[i])


func select_soil(index: int) -> void:
	if index >= 0 and index < soil_option.item_count:
		soil_option.select(index)


## Fills the species selector with the French common names (scientific names go to the info panel), grouped
## under their category (« Feuillus », « Fruitiers », « Résineux »; the core lists species already grouped).
## Each item's id is the species index.
func set_species_names(common: PackedStringArray, scientific: PackedStringArray, categories: PackedStringArray = PackedStringArray()) -> void:
	_species_common = common
	_species_sci = scientific
	species_option.clear()
	var last: String = ""
	for i: int in common.size():
		var cat: String = categories[i] if i < categories.size() else ""
		if cat != "" and cat != last:
			species_option.add_separator(cat)
			last = cat
		species_option.add_item(common[i], i)


## Selects the entry and shows the species in the info panel (common name bold, scientific italic).
func select_species(index: int) -> void:
	if index < 0 or index >= _species_common.size():
		return
	species_option.select(species_option.get_item_index(index))
	var sci: String = _species_sci[index] if index < _species_sci.size() else ""
	_name_lbl.text = "[b]%s[/b] — [i]%s[/i]" % [_species_common[index], sci]


## Bark colour of the simulated species, used for the bark ring of the cross-section.
func set_bark_color(c: Color) -> void:
	_ring_view.set_bark_color(c)
