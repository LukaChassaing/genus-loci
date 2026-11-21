class_name FluxPanel
extends PanelContainer
## Panneau « Flux » (touche L) : la légende des flux en forme + couleur (LegendGlyph, mêmes formes que dans la
## scène), une case par sorte de flux pour l'afficher ou la masquer, et la valeur du jour en unités réelles
## (TreeSim.get_flux, lue par TreeFluxView.values). Ouvert dans l'emplacement unique des panneaux (Hud) ; l'ouvrir
## active le calque Flux. Repliable en une ligne de légende (en-tête), comme avant.

## [sorte, nom, info-bulle, nom court de légende]
const ROWS: Array = [
	["water", "Sève brute (xylème)", "Eau puisée par les racines, tirée jusqu'aux feuilles", "sève brute"],
	["minerals", "Sels minéraux", "Azote, phosphore, potassium portés par la sève brute", "sels minéraux"],
	["sugar", "Sève élaborée (phloème)", "Sucres des feuilles vers le tronc, les racines, les réserves ; au printemps, des réserves vers les bourgeons", "sève élaborée"],
	["o2", "Dioxygène O₂", "Rejeté par la photosynthèse ; absorbé par la respiration (bois, racines, feuilles)", "O₂"],
	["co2", "Dioxyde de carbone CO₂", "Absorbé par les feuilles éclairées ; rejeté par la respiration", "CO₂"],
	["vapour", "Vapeur d'eau", "Transpiration par les stomates des feuilles", "vapeur d'eau"],
	["myco", "Mycorhizes", "Champignons associés aux racines : sucres donnés, phosphore, azote et eau reçus", "mycorhizes"],
]

var view: TreeFluxView
var _checks: Dictionary = {}
var _vals: Dictionary = {}
var _foot: Label
var _age: float = 10.0
var _body: VBoxContainer
var _strip: HBoxContainer      # légende repliée : une forme colorée par sorte de flux
var _head: Button
var user_collapsed: bool = false
var auto_collapsed: bool = false
var docked_out: bool = false   # (compatibilité de l'ancienne colonne ; toujours faux)
var full_height: float = 0.0


func _init(v: TreeFluxView) -> void:
	view = v


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_STOP
	var col: VBoxContainer = VBoxContainer.new()
	col.add_theme_constant_override("separation", 3)
	add_child(col)
	_head = Button.new()
	_head.theme_type_variation = "FlatButton"
	_head.alignment = HORIZONTAL_ALIGNMENT_LEFT
	_head.focus_mode = Control.FOCUS_NONE
	_head.add_theme_font_size_override("font_size", 12)
	_head.tooltip_text = "Replier / déplier le détail des flux"
	_head.pressed.connect(func() -> void: set_user_collapsed(not user_collapsed))
	col.add_child(_head)
	_strip = HBoxContainer.new()
	_strip.add_theme_constant_override("separation", 2)
	for r: Array in ROWS:
		var g: LegendGlyph = LegendGlyph.new(TreeFluxView.SHAPES[r[0]], TreeFluxView.COLORS[r[0]])
		g.tooltip_text = r[1]
		g.mouse_filter = Control.MOUSE_FILTER_PASS
		_strip.add_child(g)
	col.add_child(_strip)
	_body = VBoxContainer.new()
	_body.add_theme_constant_override("separation", 2)
	col.add_child(_body)
	var grid: GridContainer = GridContainer.new()
	grid.columns = 3
	grid.add_theme_constant_override("h_separation", 4)
	grid.add_theme_constant_override("v_separation", 2)
	_body.add_child(grid)
	for r: Array in ROWS:
		var kind: String = r[0]
		var cb: CheckBox = CheckBox.new()
		cb.focus_mode = Control.FOCUS_NONE
		cb.button_pressed = view.kind_on.get(kind, true)
		cb.tooltip_text = "Afficher / masquer : " + r[2]
		cb.toggled.connect(func(on: bool) -> void: view.set_kind(kind, on))
		_checks[kind] = cb
		var lab: HBoxContainer = HBoxContainer.new()
		lab.add_theme_constant_override("separation", 4)
		lab.add_child(LegendGlyph.new(TreeFluxView.SHAPES[kind], TreeFluxView.COLORS[kind]))
		var name_l: Label = UiStyle.label(r[1], "")
		name_l.add_theme_font_size_override("font_size", 13)
		name_l.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		name_l.clip_text = true
		lab.add_child(name_l)
		lab.tooltip_text = r[2]
		lab.mouse_filter = Control.MOUSE_FILTER_PASS
		lab.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		var val: Label = UiStyle.label("", "SmallLabel")
		val.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		val.custom_minimum_size = Vector2(100, 0)
		_vals[kind] = val
		grid.add_child(cb)
		grid.add_child(lab)
		grid.add_child(val)
	_foot = UiStyle.label("", "SmallLabel")
	_foot.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_foot.custom_minimum_size = Vector2(290, 0)
	_body.add_child(_foot)
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
	_strip.visible = c
	_head.text = ("▸ " if c else "▾ ") + ("DÉPLIER LE DÉTAIL" if c else "AUJOURD'HUI, EN UNITÉS RÉELLES")


## Aligne les cases sur la vue (accroches, tests) sans émettre.
func sync_checks() -> void:
	for k: String in _checks:
		(_checks[k] as CheckBox).set_pressed_no_signal(view.kind_on[k])


func toggle_for(kind: String) -> CheckBox:
	return _checks.get(kind)


static func num(v: float) -> String:
	var a: float = absf(v)
	if a == 0.0:
		return "0"
	if a < 0.1:
		return "%.2f" % v
	if a < 10.0:
		return "%.1f" % v
	if a < 10000.0:
		return "%.0f" % v
	return "%.1f k" % (v / 1000.0)


func _process(delta: float) -> void:
	var t0: int = Time.get_ticks_usec()
	_process_panel(delta)
	Prof.add("gd.flux_panel", t0)


func _process_panel(delta: float) -> void:
	_age += delta
	if _age < 0.25 or not is_visible_in_tree():
		return
	_age = 0.0
	var f: Dictionary = view.values
	if f.is_empty():
		return
	var g: Callable = func(k: String) -> float: return float(f.get(k, 0.0))
	_vals["water"].text = "%s L/jour" % num(g.call("xylem_l"))
	_vals["minerals"].text = "N %s · P %s g/j" % [num(g.call("n_uptake_g")), num(g.call("p_uptake_g"))]
	var up: float = g.call("phloem_up_gc")
	var dn: float = g.call("phloem_down_gc")
	_vals["sugar"].text = ("↑ %s g C/j" % num(up)) if up > dn else ("↓ %s g C/j" % num(dn))
	_vals["o2"].text = "+%s / −%s g/j" % [num(g.call("o2_out_g")), num(g.call("o2_in_g"))]
	_vals["co2"].text = "−%s / +%s g/j" % [num(g.call("co2_in_g")), num(g.call("co2_out_g"))]
	_vals["vapour"].text = "%s L/jour" % num(g.call("transpiration_l"))
	var myco: int = int(g.call("mycorrhiza"))
	if myco == 0:
		_vals["myco"].text = "aucune"
	else:
		_vals["myco"].text = "%s g C/j →" % num(g.call("alloc_myco_gc"))
	var kind_txt: String = ["", "endomycorhizes à arbuscules", "ectomycorhizes"][myco]
	var lines: PackedStringArray = PackedStringArray()
	lines.append("Photosynthèse %s g C/j · respiration %s g C/j" % [num(g.call("gpp_gc")),
		num(g.call("resp_leaf_gc") + g.call("resp_wood_gc") + g.call("resp_root_gc") + g.call("resp_growth_gc"))])
	if myco > 0:
		lines.append("%s : P %s g/j, N %s g/j reçus" % [kind_txt[0].to_upper() + kind_txt.substr(1),
			num(g.call("myco_p_g")), num(g.call("myco_n_g"))])
	var hyp: float = g.call("root_hypoxia")
	if hyp > 0.05:
		lines.append("Sol asphyxiant : %d %% des racines manquent d'O₂ (fermentation)" % roundi(hyp * 100.0))
	if g.call("water_stress") > 0.2:
		lines.append("Sécheresse : stomates fermés à %d %%" % roundi(g.call("water_stress") * 100.0))
	if up > dn and up > 0.0:
		lines.append("Débourrement : les réserves remontent vers les bourgeons")
	_foot.text = "\n".join(lines)
