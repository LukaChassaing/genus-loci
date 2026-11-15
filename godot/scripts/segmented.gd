class_name Segmented
extends GridContainer
## Row / grid of mutually exclusive toggle buttons (one always selected), in the shell button style.

signal picked(index: int)

var buttons: Array[Button] = []
var selected: int = 0
var _group: ButtonGroup = ButtonGroup.new()


func setup(labels: Array, sel: int, cols: int, font_size: int = 16) -> Segmented:
	columns = cols
	add_theme_constant_override("h_separation", 6)
	add_theme_constant_override("v_separation", 6)
	for i: int in labels.size():
		var b: Button = UiStyle.button(str(labels[i]), font_size, HORIZONTAL_ALIGNMENT_CENTER)
		b.toggle_mode = true
		b.button_group = _group
		b.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		b.custom_minimum_size = Vector2(0, 36)
		UiStyle.apply_button(func(n: String, sb: StyleBox) -> void: b.add_theme_stylebox_override(n, sb),
			func(n: String, c: Color) -> void: b.add_theme_color_override(n, c), 8.0, 6.0)
		b.pressed.connect(_on_pressed.bind(i))
		add_child(b)
		buttons.append(b)
	select(sel)
	return self


func select(i: int) -> void:
	if i >= 0 and i < buttons.size():
		selected = i
		for k: int in buttons.size():
			buttons[k].set_pressed_no_signal(k == i)


func _on_pressed(i: int) -> void:
	selected = i
	picked.emit(i)
