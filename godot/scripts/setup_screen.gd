class_name ArboretumSetup
extends VBoxContainer
## Arboretum setup form: species, soil, seed, starting age. Emits `launched` with the choice.

signal launched(species: int, soil: int, seed_value: int, years: int)
signal back_requested
signal species_changed(index: int)

const SOIL_LABELS: Array[String] = ["Limon profond", "Mince sur calcaire", "Argile engorgée", "Sable sec"]
const AGES: Array[int] = [0, 10, 40, 100]
const AGE_LABELS: Array[String] = ["Graine", "10 ans", "40 ans", "100 ans"]

var species: int = 0
var species_seg: Segmented
var soil_seg: Segmented
var age_seg: Segmented
var seed_edit: LineEdit
var plant_button: Button
var species_names: PackedStringArray = PackedStringArray()
var species_sci: PackedStringArray = PackedStringArray()
var soil_names: PackedStringArray = PackedStringArray()


func _ready() -> void:
	add_theme_constant_override("separation", 6)
	alignment = BoxContainer.ALIGNMENT_CENTER
	var ts: TreeSim = TreeSim.new()
	species_names = ts.get_species_common_names()
	species_sci = ts.get_species_names()
	soil_names = ts.get_soil_names()
	var soils: Array = []
	for i: int in soil_names.size():
		soils.append(SOIL_LABELS[i] if i < SOIL_LABELS.size() else soil_names[i])

	add_child(UiStyle.caps_label("Arboretum", 13))
	add_child(UiStyle.rule())
	species_seg = _section("Espèce", Segmented.new().setup(Array(species_names), 0, 2, 17))
	species_seg.picked.connect(func(i: int) -> void:
		species = i
		species_changed.emit(i))
	soil_seg = _section("Sol", Segmented.new().setup(soils, 0, 2, 15))
	# seed row
	var seed_row: HBoxContainer = HBoxContainer.new()
	seed_row.add_theme_constant_override("separation", 6)
	seed_edit = LineEdit.new()
	seed_edit.placeholder_text = "aléatoire"
	seed_edit.max_length = 9
	seed_edit.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	seed_edit.custom_minimum_size = Vector2(0, 36)
	seed_edit.add_theme_font_override("font", UiStyle.font())
	seed_edit.add_theme_font_size_override("font_size", 16)
	seed_edit.text_changed.connect(func(t: String) -> void:
		var clean: String = ""
		for ch: String in t:
			if ch.is_valid_int():
				clean += ch
		if clean != t:
			seed_edit.text = clean
			seed_edit.caret_column = clean.length())
	seed_row.add_child(seed_edit)
	var dice: Button = UiStyle.button("Tirer", 15, HORIZONTAL_ALIGNMENT_CENTER)
	dice.custom_minimum_size = Vector2(88, 36)
	dice.pressed.connect(func() -> void: seed_edit.text = str(randi() % 1000000))
	seed_row.add_child(dice)
	_add_titled("Graine (nombre)", seed_row)
	age_seg = _section("Âge de départ", Segmented.new().setup(AGE_LABELS, 0, 4, 15))

	var sp: Control = Control.new()
	sp.custom_minimum_size = Vector2(0, 14)
	add_child(sp)
	plant_button = UiStyle.button("Planter", 20, HORIZONTAL_ALIGNMENT_CENTER)
	plant_button.custom_minimum_size = Vector2(0, 46)
	plant_button.add_theme_stylebox_override("normal", UiStyle.box(UiStyle.PLANT_BG, UiStyle.LINE_HI, 2, 18.0, 9.0))
	plant_button.add_theme_stylebox_override("hover", UiStyle.box(UiStyle.PLANT_BG.lightened(0.12), UiStyle.FOCUS_RING, 2, 18.0, 9.0))
	plant_button.add_theme_color_override("font_color", UiStyle.INK)
	plant_button.add_theme_color_override("font_hover_color", UiStyle.INK)
	plant_button.add_theme_color_override("font_focus_color", UiStyle.INK)
	plant_button.pressed.connect(_launch)
	add_child(plant_button)
	var back: Button = UiStyle.button("Retour", 17, HORIZONTAL_ALIGNMENT_CENTER)
	back.custom_minimum_size = Vector2(0, 38)
	back.pressed.connect(func() -> void: back_requested.emit())
	add_child(back)


func _section(title: String, seg: Segmented) -> Segmented:
	_add_titled(title, seg)
	return seg


func _add_titled(title: String, c: Control) -> void:
	var m: MarginContainer = MarginContainer.new()
	m.add_theme_constant_override("margin_top", 10)
	m.add_child(UiStyle.caps_label(title, 12))
	add_child(m)
	add_child(c)


func chosen_seed() -> int:
	var t: String = seed_edit.text.strip_edges()
	return int(t) if t.is_valid_int() else -1


func _launch() -> void:
	launched.emit(species, soil_seg.selected, chosen_seed(), AGES[age_seg.selected])


func focus_first() -> void:
	UiStyle.focus_later(species_seg.buttons[species])
