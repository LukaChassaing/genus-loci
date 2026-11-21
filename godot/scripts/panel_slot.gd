class_name PanelSlot
extends PanelContainer
## Emplacement unique des panneaux de niveau 2 (divulgation progressive à deux niveaux) : un seul panneau ouvert
## à la fois, sur le côté droit, avec un en-tête (titre + croix) et un défilement vertical quand il dépasse.
## Chaque panneau est enregistré une fois (`register`) ; `open(id)` ferme l'ancien. Indépendant du contenu
## (cernes, flux, sol, mécanique, taille... ou d'autres dans Seed Master).

signal changed(id: String)

const WIDTH: float = 340.0

var current: String = ""
var _entries: Dictionary = {}    # id -> {"control": Control, "title": String}
var _title: Label
var _close: Button
var _scroll: ScrollContainer
var _holder: VBoxContainer


func _init() -> void:
	theme_type_variation = "SidePanel"
	mouse_filter = Control.MOUSE_FILTER_STOP
	visible = false
	var col: VBoxContainer = VBoxContainer.new()
	col.add_theme_constant_override("separation", 4)
	add_child(col)
	var head: HBoxContainer = HBoxContainer.new()
	col.add_child(head)
	_title = UiStyle.label("", "HeaderLabel")
	_title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_title.clip_text = true
	head.add_child(_title)
	_close = Button.new()
	_close.text = "✕"
	_close.theme_type_variation = "FlatButton"
	_close.focus_mode = Control.FOCUS_NONE
	_close.tooltip_text = "Fermer le panneau"
	_close.pressed.connect(close)
	head.add_child(_close)
	col.add_child(UiStyle.rule(UiStyle.LINE_DIM))
	_scroll = ScrollContainer.new()
	_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	col.add_child(_scroll)
	# Marge à droite : la barre de défilement (grande échelle d'interface) ne recouvre pas le texte.
	var pad: MarginContainer = MarginContainer.new()
	pad.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	pad.add_theme_constant_override("margin_right", 10)
	_scroll.add_child(pad)
	_holder = VBoxContainer.new()
	_holder.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	pad.add_child(_holder)


## Enregistre un panneau (reparenté dans l'emplacement, caché tant qu'il n'est pas ouvert). Un PanelContainer perd
## son propre fond : c'est l'emplacement qui porte le cadre.
func register(id: String, c: Control, title: String) -> void:
	_entries[id] = {"control": c, "title": title}
	if c.get_parent() != null:
		c.get_parent().remove_child(c)
	if c is PanelContainer:
		c.add_theme_stylebox_override("panel", StyleBoxEmpty.new())
	c.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	c.visible = false
	_holder.add_child(c)


func has_panel(id: String) -> bool:
	return _entries.has(id)


func panel(id: String) -> Control:
	return (_entries[id] as Dictionary)["control"] if _entries.has(id) else null


func is_open(id: String) -> bool:
	return current == id and visible


## Ouvre `id` (ferme celui qui était ouvert) ; "" ou un id inconnu ferme tout.
func open(id: String) -> void:
	if not _entries.has(id):
		id = ""
	if id == current and visible == (id != ""):
		return
	for k: String in _entries:
		((_entries[k] as Dictionary)["control"] as Control).visible = k == id
	current = id
	visible = id != ""
	_title.text = String((_entries[id] as Dictionary)["title"]).to_upper() if id != "" else ""
	_scroll.scroll_vertical = 0
	changed.emit(id)


## Hauteur sans défilement (en-tête + contenu complet) : l'emplacement prend cette hauteur, bornée par la place.
func natural_height() -> float:
	return get_combined_minimum_size().y + _holder.get_combined_minimum_size().y


func close() -> void:
	open("")


func toggle(id: String) -> void:
	open("" if current == id else id)
