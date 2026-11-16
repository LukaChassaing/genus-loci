class_name SaveBrowser
extends VBoxContainer
## List of saved trees, in the shell style (title column « Charger », pause menu « Charger » / « Sauvegarder »).
## Load mode: pick a save, « Charger » or « Supprimer » (confirmed). Save mode: a name field (sensible
## default) and « Sauvegarder », confirmed before overwriting; picking a row reuses its name.
## Emits the requests; the host does the I/O (SaveFlow) and calls `refresh()` / `set_status()`.

signal load_requested(path: String)
signal save_requested(save_name: String, path: String)
signal closed

var mode: String = "load"
## Folder listed (absolute); the self-test points it at a scratch folder.
var dir: String = ""
var default_name: String = ""
var entries: Array = []          # dictionaries from TreeSim.list_saves
var rows: Array[Button] = []
var selected: int = -1
var name_edit: LineEdit
var action_button: Button
var delete_button: Button
var back_button: Button
var status: Label
var _list: VBoxContainer
var _scroll: ScrollContainer
var _actions: HBoxContainer
var _confirm: HBoxContainer
var _confirm_label: Label


func _init(p_mode: String = "load", p_dir: String = "", p_default_name: String = "") -> void:
	mode = p_mode
	dir = p_dir if p_dir != "" else SaveFlow.saves_dir()
	default_name = p_default_name


func _ready() -> void:
	add_theme_constant_override("separation", 8)
	size_flags_vertical = Control.SIZE_EXPAND_FILL
	add_child(UiStyle.caps_label("Sauvegarder l'arbre" if mode == "save" else "Charger un arbre", 13))
	add_child(UiStyle.rule())
	if mode == "save":
		add_child(UiStyle.serif_label("Nom", 15, UiStyle.TEXT_DIM, 400, true))
		name_edit = LineEdit.new()
		name_edit.text = default_name
		name_edit.max_length = 60
		name_edit.select_all_on_focus = true
		name_edit.custom_minimum_size = Vector2(0, 38)
		name_edit.add_theme_font_override("font", UiStyle.font())
		name_edit.add_theme_font_size_override("font_size", 17)
		name_edit.text_submitted.connect(func(_t: String) -> void: _on_action())
		name_edit.text_changed.connect(func(_t: String) -> void:
			_picked_path = ""  # a typed name targets its own file
			_hide_confirm())
		add_child(name_edit)
		add_child(UiStyle.serif_label("Sauvegardes existantes", 15, UiStyle.TEXT_DIM, 400, true))
	_scroll = ScrollContainer.new()
	_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_scroll.custom_minimum_size = Vector2(0, 220 if mode == "save" else 300)
	add_child(_scroll)
	_list = VBoxContainer.new()
	_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_list.add_theme_constant_override("separation", 6)
	_scroll.add_child(_list)
	status = UiStyle.serif_label("", 15, UiStyle.ACCENT, 400, true)
	status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	status.custom_minimum_size = Vector2(200, 0)
	status.visible = false
	add_child(status)
	_confirm = HBoxContainer.new()
	_confirm.add_theme_constant_override("separation", 6)
	_confirm.visible = false
	_confirm_label = UiStyle.serif_label("", 16, UiStyle.TEXT)
	_confirm_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_confirm_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_confirm_label.custom_minimum_size = Vector2(120, 0)
	_confirm.add_child(_confirm_label)
	var yes: Button = UiStyle.button("Oui", 16, HORIZONTAL_ALIGNMENT_CENTER)
	yes.name = "ConfirmYes"
	yes.custom_minimum_size = Vector2(64, 38)
	yes.pressed.connect(_on_confirm_yes)
	_confirm.add_child(yes)
	var no: Button = UiStyle.button("Non", 16, HORIZONTAL_ALIGNMENT_CENTER)
	no.custom_minimum_size = Vector2(64, 38)
	no.pressed.connect(_hide_confirm)
	_confirm.add_child(no)
	add_child(_confirm)
	_actions = HBoxContainer.new()
	_actions.add_theme_constant_override("separation", 6)
	action_button = _primary("Sauvegarder" if mode == "save" else "Charger")
	action_button.pressed.connect(_on_action)
	_actions.add_child(action_button)
	delete_button = UiStyle.button("Supprimer", 16, HORIZONTAL_ALIGNMENT_CENTER)
	delete_button.custom_minimum_size = Vector2(0, 42)
	delete_button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	delete_button.pressed.connect(_on_delete)
	if mode == "load":
		_actions.add_child(delete_button)
	add_child(_actions)
	back_button = UiStyle.button("Retour", 17, HORIZONTAL_ALIGNMENT_CENTER)
	back_button.custom_minimum_size = Vector2(0, 40)
	back_button.pressed.connect(func() -> void: closed.emit())
	add_child(back_button)
	refresh()
	if mode == "save":
		UiStyle.focus_later(name_edit)
	elif not rows.is_empty():
		UiStyle.focus_later(rows[0])
	else:
		UiStyle.focus_later(back_button)


func _primary(text: String) -> Button:
	var b: Button = UiStyle.button(text, 18, HORIZONTAL_ALIGNMENT_CENTER)
	b.custom_minimum_size = Vector2(0, 42)
	b.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	b.add_theme_stylebox_override("normal", UiStyle.box(UiStyle.PLANT_BG, UiStyle.LINE_HI, 2, 18.0, 9.0))
	b.add_theme_stylebox_override("hover", UiStyle.box(UiStyle.PLANT_BG.lightened(0.12), UiStyle.FOCUS_RING, 2, 18.0, 9.0))
	b.add_theme_stylebox_override("disabled", UiStyle.box(UiStyle.BTN_OFF, UiStyle.LINE, 1, 18.0, 9.0))
	for c: String in ["font_color", "font_hover_color", "font_focus_color", "font_pressed_color"]:
		b.add_theme_color_override(c, UiStyle.INK)
	b.add_theme_color_override("font_disabled_color", UiStyle.TEXT_OFF)
	return b


## Re-read the folder and rebuild the rows (keeps the selection by path when possible).
func refresh() -> void:
	var keep: String = entries[selected]["path"] if selected >= 0 and selected < entries.size() else ""
	entries = TreeSim.list_saves(dir)
	for r: Button in rows:
		r.queue_free()
	rows.clear()
	for c: Node in _list.get_children():
		c.queue_free()
	selected = -1
	if entries.is_empty():
		var none: Label = UiStyle.serif_label("Aucune sauvegarde pour l'instant.", 16, UiStyle.TEXT_DIM, 400, true)
		_list.add_child(none)
	for i: int in entries.size():
		var e: Dictionary = entries[i]
		var b: Button = _row(e)
		b.pressed.connect(_select.bind(i))
		_list.add_child(b)
		rows.append(b)
		if e["path"] == keep:
			selected = i
	if selected < 0 and mode == "load" and not entries.is_empty():
		selected = 0
	_update_rows()
	_hide_confirm()


func _row(e: Dictionary) -> Button:
	var b: Button = UiStyle.button("", 16)
	b.toggle_mode = true
	b.custom_minimum_size = Vector2(0, 58)
	b.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	b.clip_contents = true
	var v: VBoxContainer = VBoxContainer.new()
	v.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	v.offset_left = 14
	v.offset_right = -10
	v.offset_top = 6
	v.offset_bottom = -6
	v.alignment = BoxContainer.ALIGNMENT_CENTER
	v.add_theme_constant_override("separation", 0)
	v.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var title: Label = UiStyle.serif_label(_title_of(e), 17, UiStyle.TEXT)
	title.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	title.mouse_filter = Control.MOUSE_FILTER_IGNORE
	title.name = "Title"
	var sub: Label = UiStyle.serif_label(SaveBrowser.describe(e), 14, UiStyle.TEXT_DIM, 400, true)
	sub.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	sub.mouse_filter = Control.MOUSE_FILTER_IGNORE
	sub.name = "Sub"
	v.add_child(title)
	v.add_child(sub)
	b.add_child(v)
	b.tooltip_text = str(e["path"]) if bool(e["ok"]) else "Illisible : %s" % e["error"]
	return b


static func _title_of(e: Dictionary) -> String:
	var n: String = str(e["name"])
	if n.begins_with("Sauvegarde"):
		return n
	if str(e["file"]) == SaveFlow.AUTOSAVE_FILE:
		return "%s (automatique)" % n
	if str(e["file"]) == SaveFlow.QUICK_FILE:
		return "%s (rapide)" % n
	return n


## "Chêne pédonculé · 45 ans · 16/11/2026 14:05", or the reason it cannot be read.
static func describe(e: Dictionary) -> String:
	if not bool(e["ok"]):
		return "illisible : %s" % e["error"]
	var age: float = float(e["age"])
	var age_txt: String = "%d an%s" % [int(age), "s" if int(age) >= 2 else ""] if age >= 1.0 else "%d jours" % int(age * 365.0)
	var parts: Array[String] = [str(e["common"]), age_txt]
	if int(e["cuts"]) > 0:
		parts.append("%d coupe%s" % [int(e["cuts"]), "s" if int(e["cuts"]) > 1 else ""])
	parts.append(SaveFlow.format_date_short(int(e["written"])))
	return " · ".join(parts)


func _select(i: int) -> void:
	selected = i
	_hide_confirm()
	if mode == "save" and i >= 0 and i < entries.size() and bool(entries[i]["ok"]):
		name_edit.text = str(entries[i]["name"])
		_picked_path = str(entries[i]["path"])
	_update_rows()


func _update_rows() -> void:
	for i: int in rows.size():
		var on: bool = i == selected
		rows[i].set_pressed_no_signal(on)
		# Dark ink on the gold selected fill, ivory / parchment otherwise.
		(rows[i].find_child("Title", true, false) as Label).add_theme_color_override("font_color", UiStyle.INK if on else UiStyle.TEXT)
		(rows[i].find_child("Sub", true, false) as Label).add_theme_color_override("font_color",
			Color(UiStyle.INK, 0.8) if on else UiStyle.TEXT_DIM)
	var ok_sel: bool = selected >= 0 and selected < entries.size()
	if mode == "load":
		action_button.disabled = not (ok_sel and bool(entries[selected]["ok"]))
		delete_button.disabled = not ok_sel


func set_status(text: String, error: bool = false) -> void:
	status.text = text
	status.add_theme_color_override("font_color", Color(0.98, 0.62, 0.48) if error else UiStyle.ACCENT)
	status.visible = text != ""


# ------------------------------------------- actions -------------------------------------------

var _pending: String = ""   # "overwrite" | "delete"
var _picked_path: String = ""  # save mode: file of the picked row (overwritten as is)
var _pending_path: String = ""


func target_path() -> String:
	if _picked_path != "":
		return _picked_path
	return SaveFlow.path_for(dir, name_edit.text if name_edit != null else "")


func _on_action() -> void:
	if mode == "load":
		if selected >= 0 and bool(entries[selected]["ok"]):
			load_requested.emit(str(entries[selected]["path"]))
		return
	var n: String = name_edit.text.strip_edges()
	if n == "":
		n = default_name
		name_edit.text = n
	var path: String = target_path()
	if FileAccess.file_exists(path) and _pending != "overwrite":
		_ask("overwrite", path, "Remplacer « %s » ?" % n)
		return
	_hide_confirm()
	save_requested.emit(n, path)


func _on_delete() -> void:
	if selected < 0 or selected >= entries.size():
		return
	_ask("delete", str(entries[selected]["path"]), "Supprimer « %s » ? C'est définitif." % entries[selected]["name"])


func _ask(what: String, path: String, text: String) -> void:
	_pending = what
	_pending_path = path
	_confirm_label.text = text
	_confirm.visible = true
	_actions.visible = false
	UiStyle.focus_later(_confirm.get_node("ConfirmYes") as Button)


func _hide_confirm() -> void:
	_pending = ""
	_pending_path = ""
	if _confirm != null:
		_confirm.visible = false
		_actions.visible = true


func _on_confirm_yes() -> void:
	match _pending:
		"overwrite":
			var n: String = name_edit.text.strip_edges()
			var p: String = _pending_path
			_hide_confirm()
			save_requested.emit(n if n != "" else default_name, p)
		"delete":
			var p: String = _pending_path
			_hide_confirm()
			if TreeSim.delete_save(p):
				set_status("Sauvegarde supprimée.")
			else:
				set_status("Impossible de supprimer ce fichier.", true)
			refresh()
		_:
			_hide_confirm()
