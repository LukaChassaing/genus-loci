class_name UiStyle
extends RefCounted
## Shared look of every screen: dark painted shell (near-black panels, parchment serif text, one ochre
## accent, thin borders) and the cream school-plate palette used by the species plates.
## Everything is built in code: fonts, StyleBoxes, a Theme and a button factory.

# --- shell palette: golden-hour family (deep forest-olive panels, ivory serif text, antique gold) ----------
const PANEL: Color = Color(0.20, 0.27, 0.15)        # deep forest green-olive (~20 % lightness)
const PANEL_A: Color = Color(0.20, 0.27, 0.15, 0.88)
const INK: Color = Color(0.13, 0.10, 0.04)             # dark text on gold fills
const TEXT: Color = Color(0.97, 0.94, 0.82)            # ivory
const TEXT_DIM: Color = Color(0.80, 0.76, 0.60)        # muted parchment
const TEXT_OFF: Color = Color(0.56, 0.58, 0.46)        # disabled: desaturated mid-tone
const LINE: Color = Color(0.70, 0.56, 0.28)            # antique gold, thin borders
const LINE_HI: Color = Color(0.93, 0.78, 0.42)         # bright gold (hover, focus ring)
const ACCENT: Color = Color(0.95, 0.80, 0.45)          # gold for small caps / accents on dark
const ACCENT_BG: Color = Color(0.90, 0.72, 0.36, 0.97)  # selected: warm gold fill (dark text)
const BTN_BG: Color = Color(0.11, 0.16, 0.07, 0.45)    # a notch darker than the panel
const BTN_HOVER: Color = Color(0.33, 0.41, 0.23, 0.96)  # lighter panel tone
const BTN_OFF: Color = Color(0.22, 0.25, 0.18, 0.40)
const FOCUS_RING: Color = Color(0.99, 0.95, 0.78, 0.95) # focus = outline only, never a fill
const VEIL: Color = Color(0.09, 0.12, 0.06, 0.72)      # dark green veil behind text over the painting
const TITLE: Color = Color(0.99, 0.95, 0.80)           # ivory title over the sky
const TITLE_SHADOW: Color = Color(0.10, 0.08, 0.20, 0.55)
const SLIDER_TRACK: Color = Color(0.07, 0.10, 0.05, 0.75)
const SLIDER_FILL: Color = Color(0.86, 0.68, 0.32)
const PLANT_BG: Color = Color(0.93, 0.76, 0.38, 0.98)  # primary action

# --- school plate palette --------------------------------------------------------------------
const PAPER: Color = Color(0.925, 0.885, 0.760)
const PAPER_DARK: Color = Color(0.86, 0.80, 0.64)
const PAPER_INK: Color = Color(0.08, 0.07, 0.06)
const PLATE_BLUE: Color = Color(0.10, 0.25, 0.58)
const PLATE_RED: Color = Color(0.71, 0.12, 0.10)
const WALL: Color = Color(0.20, 0.25, 0.15)

const SERIF_NAMES: PackedStringArray = ["Noto Serif", "DejaVu Serif", "Liberation Serif", "Times New Roman", "serif"]

static var _fonts: Dictionary = {}
static var _grabber: ImageTexture


## Serif font. `weight` 400 / 700, optional italic and extra letter spacing (small-caps tracking).
static func font(weight: int = 400, italic: bool = false, tracking: int = 0) -> Font:
	var key: String = "%d|%s|%d" % [weight, str(italic), tracking]
	if _fonts.has(key):
		return _fonts[key]
	var sf: SystemFont = SystemFont.new()
	sf.font_names = SERIF_NAMES
	sf.font_weight = weight
	sf.font_italic = italic
	sf.antialiasing = TextServer.FONT_ANTIALIASING_GRAY
	sf.subpixel_positioning = TextServer.SUBPIXEL_POSITIONING_AUTO
	var f: Font = sf
	if tracking != 0:
		var fv: FontVariation = FontVariation.new()
		fv.base_font = sf
		fv.spacing_glyph = tracking
		f = fv
	_fonts[key] = f
	return f


static func box(bg: Color, border: Color, bw: int = 1, mx: float = 12.0, my: float = 6.0) -> StyleBoxFlat:
	var s: StyleBoxFlat = StyleBoxFlat.new()
	s.bg_color = bg
	s.border_color = border
	s.set_border_width_all(bw)
	s.content_margin_left = mx
	s.content_margin_right = mx
	s.content_margin_top = my
	s.content_margin_bottom = my
	s.anti_aliasing = false
	return s


static func panel_box(alpha: float = 0.88) -> StyleBoxFlat:
	var s: StyleBoxFlat = box(Color(PANEL.r, PANEL.g, PANEL.b, alpha), Color(LINE.r, LINE.g, LINE.b, 0.80), 1, 10.0, 8.0)
	return s


static func _grabber_tex() -> ImageTexture:
	if _grabber == null:
		var n: int = 16
		var img: Image = Image.create(n, n, false, Image.FORMAT_RGBA8)
		for y: int in n:
			for x: int in n:
				var d: float = Vector2(x + 0.5 - n * 0.5, y + 0.5 - n * 0.5).length()
				var a: float = clampf(n * 0.5 - 1.0 - d, 0.0, 1.0)
				var c: Color = Color(0.96, 0.82, 0.46) if d < n * 0.5 - 3.0 else Color(0.45, 0.32, 0.10)
				img.set_pixel(x, y, Color(c.r, c.g, c.b, a))
		_grabber = ImageTexture.create_from_image(img)
	return _grabber


## Focus is a drawn-over outline (no fill), so it never looks like "selected" (gold fill).
static func focus_box(mx: float, my: float) -> StyleBoxFlat:
	var f: StyleBoxFlat = box(Color(0, 0, 0, 0), FOCUS_RING, 2, mx, my)
	return f


## Applies the button look (all states) to any Control that takes theme overrides or to a Theme type.
static func apply_button(set_box: Callable, set_col: Callable, mx: float, my: float) -> void:
	set_box.call("normal", box(BTN_BG, LINE, 1, mx, my))
	set_box.call("hover", box(BTN_HOVER, LINE_HI, 1, mx, my))
	set_box.call("focus", focus_box(mx, my))
	set_box.call("pressed", box(ACCENT_BG, LINE_HI, 1, mx, my))
	set_box.call("hover_pressed", box(ACCENT_BG.lightened(0.10), LINE_HI, 1, mx, my))
	set_box.call("disabled", box(BTN_OFF, Color(LINE.r, LINE.g, LINE.b, 0.30), 1, mx, my))
	set_col.call("font_color", TEXT)
	set_col.call("font_hover_color", TEXT)
	set_col.call("font_focus_color", TEXT)
	set_col.call("font_pressed_color", INK)
	set_col.call("font_hover_pressed_color", INK)
	set_col.call("font_disabled_color", TEXT_OFF)


static func _button_styles(th: Theme, type: String, mx: float, my: float) -> void:
	apply_button(func(n: String, b: StyleBox) -> void: th.set_stylebox(n, type, b),
		func(n: String, c: Color) -> void: th.set_color(n, type, c), mx, my)
	th.set_constant("outline_size", type, 0)


## Theme for the whole family: serif font, dark translucent panels with a thin light border, buttons,
## sliders, option lists, line edits. `pad` is the horizontal button padding (the bottom bar uses less).
static func make_theme(font_size: int = 18, pad_x: float = 12.0, pad_y: float = 6.0) -> Theme:
	var th: Theme = Theme.new()
	th.default_font = font()
	th.default_font_size = font_size
	th.set_color("font_color", "Label", TEXT)
	th.set_color("font_shadow_color", "Label", Color(0, 0, 0, 0.0))
	th.set_stylebox("panel", "PanelContainer", panel_box())
	th.set_stylebox("panel", "Panel", panel_box())
	for t: String in ["Button", "OptionButton", "MenuButton"]:
		_button_styles(th, t, pad_x, pad_y)
	th.set_constant("h_separation", "OptionButton", 4)
	th.set_color("font_color", "CheckBox", TEXT)
	th.set_color("font_color", "OptionButton", TEXT)
	# Popup menu of the option buttons
	th.set_stylebox("panel", "PopupMenu", box(Color(0.13, 0.17, 0.09, 0.98), LINE, 1, 4.0, 4.0))
	th.set_stylebox("hover", "PopupMenu", box(ACCENT_BG, Color(0, 0, 0, 0), 0, 6.0, 3.0))
	th.set_color("font_color", "PopupMenu", TEXT)
	th.set_color("font_hover_color", "PopupMenu", INK)
	th.set_color("font_disabled_color", "PopupMenu", TEXT_OFF)
	th.set_constant("v_separation", "PopupMenu", 6)
	# Sliders: hairline track, ochre fill, round grabber
	var track: StyleBoxFlat = box(SLIDER_TRACK, Color(0, 0, 0, 0), 0, 0.0, 0.0)
	track.content_margin_top = 2.0
	track.content_margin_bottom = 2.0
	th.set_stylebox("slider", "HSlider", track)
	th.set_stylebox("grabber_area", "HSlider", box(SLIDER_FILL.darkened(0.12), Color(0, 0, 0, 0), 0, 0.0, 0.0))
	th.set_stylebox("grabber_area_highlight", "HSlider", box(SLIDER_FILL, Color(0, 0, 0, 0), 0, 0.0, 0.0))
	th.set_icon("grabber", "HSlider", _grabber_tex())
	th.set_icon("grabber_highlight", "HSlider", _grabber_tex())
	th.set_icon("grabber_disabled", "HSlider", _grabber_tex())
	# Progress bar
	th.set_stylebox("background", "ProgressBar", box(SLIDER_TRACK, Color(LINE.r, LINE.g, LINE.b, 0.7), 1, 0.0, 0.0))
	th.set_stylebox("fill", "ProgressBar", box(Color(0.42, 0.68, 0.80), Color(0, 0, 0, 0), 0, 0.0, 0.0))
	# Line edit
	th.set_stylebox("normal", "LineEdit", box(Color(0.05, 0.08, 0.03, 0.55), LINE, 1, 8.0, 5.0))
	th.set_stylebox("focus", "LineEdit", box(Color(0.05, 0.08, 0.03, 0.70), LINE_HI, 1, 8.0, 5.0))
	th.set_color("font_color", "LineEdit", TEXT)
	th.set_color("caret_color", "LineEdit", ACCENT)
	th.set_color("font_placeholder_color", "LineEdit", TEXT_OFF)
	th.set_color("selection_color", "LineEdit", Color(0.90, 0.72, 0.36, 0.40))
	# Rich text
	th.set_font("normal_font", "RichTextLabel", font())
	th.set_font("bold_font", "RichTextLabel", font(700))
	th.set_font("italics_font", "RichTextLabel", font(400, true))
	th.set_font("bold_italics_font", "RichTextLabel", font(700, true))
	th.set_color("default_color", "RichTextLabel", TEXT)
	th.set_stylebox("normal", "RichTextLabel", StyleBoxEmpty.new())
	th.set_stylebox("separator", "HSeparator", box(LINE, LINE, 0, 0.0, 0.0))
	return th


## Menu / form button: bordered, discreet, left-aligned small tracked text.
static func button(text: String, size: int = 18, align: HorizontalAlignment = HORIZONTAL_ALIGNMENT_LEFT) -> Button:
	var b: Button = Button.new()
	b.text = text
	b.alignment = align
	b.add_theme_font_override("font", font(400, false, 1))
	b.add_theme_font_size_override("font_size", size)
	apply_button(func(n: String, sb: StyleBox) -> void: b.add_theme_stylebox_override(n, sb),
		func(n: String, c: Color) -> void: b.add_theme_color_override(n, c), 18.0, 9.0)
	return b


## Small-caps style heading label (uppercase, tracked, dim ochre).
static func caps_label(text: String, size: int = 13, color: Color = ACCENT) -> Label:
	var l: Label = Label.new()
	l.text = text.to_upper()
	l.add_theme_font_override("font", font(400, false, 3))
	l.add_theme_font_size_override("font_size", size)
	l.add_theme_color_override("font_color", color)
	return l


static func serif_label(text: String, size: int, color: Color = TEXT, weight: int = 400, italic: bool = false) -> Label:
	var l: Label = Label.new()
	l.text = text
	l.add_theme_font_override("font", font(weight, italic))
	l.add_theme_font_size_override("font_size", size)
	l.add_theme_color_override("font_color", color)
	return l


## Thin horizontal rule.
static func rule(color: Color = LINE) -> ColorRect:
	var r: ColorRect = ColorRect.new()
	r.color = color
	r.custom_minimum_size = Vector2(0, 1)
	r.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return r


## Grabs focus on the next idle frame, only if the control is still in the tree by then.
static func focus_later(c: Control) -> void:
	(func() -> void:
		if is_instance_valid(c) and c.is_inside_tree() and c.is_visible_in_tree():
			c.grab_focus()).call_deferred()
