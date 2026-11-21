class_name SoilPanel
extends VBoxContainer
## Panneau « Sol » : type de terrain, eau du sol (bullet graph avec le seuil de stress), nappe, roche mère, front
## de dessèchement, horizons (nom, profondeur, eau de chaque couche) et la transparence du sol (touche X), avec
## l'accès à la vue des racines (R). Rafraîchi au plus quatre fois par seconde, seulement quand il est ouvert.

signal root_view_requested

## Seuil d'eau du sol en dessous duquel le sol est dit « sec » (même seuil que le calque Eau et la pelouse
## jaunie de main.gd, `smoothstep(0.25, 0.04, water)`). Convention d'affichage, pas une valeur publiée.
const DRY: float = 0.25

var soil_slider: HSlider
var _type: Label
var _water: BulletGraph
var _water_l: Label
var _facts: Label
var _grid: GridContainer
var _age: float = 10.0
var _n_rows: int = -1


func _init() -> void:
	add_theme_constant_override("separation", 4)
	_type = UiStyle.label("", "CardValue")
	add_child(_type)
	var wr: HBoxContainer = HBoxContainer.new()
	wr.add_theme_constant_override("separation", 6)
	add_child(wr)
	wr.add_child(UiStyle.label("Eau du sol", "DimLabel"))
	_water = BulletGraph.new()
	_water.bands = PackedFloat32Array([DRY, 0.6])
	_water.threshold = DRY
	_water.bar_color = UiStyle.WATER
	_water.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_water.custom_minimum_size = Vector2(90, 12)
	_water.tooltip_text = "Eau disponible dans la zone racinaire (0 = point de flétrissement, 1 = capacité au champ). Trait : seuil de sécheresse."
	_water.mouse_filter = Control.MOUSE_FILTER_PASS
	wr.add_child(_water)
	_water_l = UiStyle.label("", "DimLabel")
	_water_l.custom_minimum_size = Vector2(44, 0)
	_water_l.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	wr.add_child(_water_l)
	_facts = UiStyle.label("", "SmallLabel")
	_facts.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	add_child(_facts)
	add_child(UiStyle.label("HORIZONS (PROFONDEUR · EAU)", "CardTitle"))
	_grid = GridContainer.new()
	_grid.columns = 4
	_grid.add_theme_constant_override("h_separation", 6)
	_grid.add_theme_constant_override("v_separation", 2)
	add_child(_grid)
	add_child(UiStyle.rule(UiStyle.LINE_DIM))
	var orow: HBoxContainer = HBoxContainer.new()
	orow.add_theme_constant_override("separation", 6)
	add_child(orow)
	orow.add_child(UiStyle.label("Opacité du sol (X)", "DimLabel"))
	soil_slider = HSlider.new()
	soil_slider.focus_mode = Control.FOCUS_NONE
	soil_slider.min_value = 0.0
	soil_slider.max_value = 1.0
	soil_slider.step = 0.01
	soil_slider.value = 1.0
	soil_slider.tooltip_text = "Opacité du sol : à gauche, le sol devient transparent et laisse voir les racines (X : opaque / transparent)"
	soil_slider.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	soil_slider.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	orow.add_child(soil_slider)
	var rv: Button = Button.new()
	rv.text = "Vue des racines (R)"
	rv.focus_mode = Control.FOCUS_NONE
	rv.tooltip_text = "Regarder les racines par l'entaille du sol (R)"
	rv.pressed.connect(func() -> void: root_view_requested.emit())
	add_child(rv)


static func _m(v: float) -> String:
	return UiStyle.num(v, 2 if v < 1.0 else 1) + " m"


func refresh(sim: TreeSim, soil_name: String, delta: float) -> void:
	_age += delta
	if _age < 0.25 or not is_visible_in_tree():
		return
	_age = 0.0
	_type.text = soil_name
	var w: float = sim.get_soil_water()
	_water.set_value(w)
	_water_l.text = "%d %%" % roundi(w * 100.0)
	var lines: PackedStringArray = PackedStringArray()
	var table: float = sim.get_water_table_depth_m()
	lines.append("Nappe : " + (_m(table) if table >= 0.0 else "aucune à portée"))
	var rock: float = sim.get_bedrock_depth_m()
	lines.append("Roche mère : " + (_m(rock) if rock >= 0.0 else "aucune (sol profond)"))
	var front: float = sim.get_dry_front_m()
	lines.append("Front de dessèchement : " + (_m(front) if front > 0.01 else "aucun"))
	lines.append("Racines : %s de profondeur, %s d'étalement" % [_m(sim.get_root_depth_m()), _m(sim.get_root_spread_m())])
	_facts.text = "\n".join(lines)
	var bottoms: PackedFloat32Array = sim.get_horizon_bottoms()
	var kinds: PackedInt32Array = sim.get_horizon_kinds()
	var waters: PackedFloat32Array = sim.get_horizon_water()
	var n: int = mini(mini(bottoms.size(), kinds.size()), mini(waters.size(), 8))
	if n != _n_rows:
		_n_rows = n
		for c: Node in _grid.get_children():
			_grid.remove_child(c)
			c.queue_free()
		for i: int in n:
			_grid.add_child(LegendGlyph.new("swatch", Color.BLACK))
			var name_l: Label = UiStyle.label("", "SmallLabel")
			name_l.add_theme_color_override("font_color", UiStyle.TEXT)
			name_l.size_flags_horizontal = Control.SIZE_EXPAND_FILL
			name_l.clip_text = true
			_grid.add_child(name_l)
			_grid.add_child(UiStyle.label("", "SmallLabel"))
			var b: BulletGraph = BulletGraph.new()
			b.bands = PackedFloat32Array([DRY, 0.6])
			b.threshold = DRY
			b.bar_color = UiStyle.WATER
			b.custom_minimum_size = Vector2(56, 10)
			_grid.add_child(b)
	var top: float = 0.0
	for i: int in n:
		var k: int = clampi(kinds[i], 0, SoilBlock.KIND_NAMES.size() - 1)
		var g: LegendGlyph = _grid.get_child(i * 4) as LegendGlyph
		g.color = SoilBlock.KIND_COLORS[k]
		g.queue_redraw()
		var nm: Label = _grid.get_child(i * 4 + 1) as Label
		nm.text = SoilBlock.KIND_NAMES[k]
		nm.tooltip_text = SoilBlock.KIND_NAMES[k]
		var last: bool = i == n - 1
		(_grid.get_child(i * 4 + 2) as Label).text = ("%s–%s m" % [UiStyle.num(top, 1), UiStyle.num(bottoms[i], 1)]) if not last else ("> %s m" % UiStyle.num(top, 1))
		(_grid.get_child(i * 4 + 3) as BulletGraph).set_value(waters[i])
		top = bottoms[i]
