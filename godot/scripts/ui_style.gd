class_name UiStyle
extends RefCounted
## Apparence commune de tous les écrans : coque sombre vert forêt, texte ivoire à empattements, or réservé à la
## sélection (bouton enfoncé, choix actif) et à l'identité (titre), rouge-orangé réservé à l'urgent (déclin, tempête),
## toujours accompagné d'une forme (triangle) : jamais la couleur seule (WCAG 2.2, SC 1.4.1).
## Un seul `Theme` (UiStyle.theme()), construit en code, avec des variantes de type (set_type_variation) pour les
## barres, panneaux latéraux, cartes, titres de carte, valeurs, textes atténués et boutons compacts.
## Les contrastes sont mesurés par le selftest (UiStyle.contrast_pairs) : texte >= 4,5:1 (SC 1.4.3), éléments
## graphiques >= 3:1 (SC 1.4.11), pire cas = panneau translucide devant un ciel clair (SKY_WORST).
## Taille de texte : 12 px au minimum à la base 1280×720 (FONT_MIN).

# --- palette de la coque (vert forêt, ivoire, or) ------------------------------------------------------------
const PANEL: Color = Color(0.15, 0.21, 0.11)          # vert forêt sombre
const PANEL_ALPHA: float = 0.94                         # assez opaque pour que le texte tienne devant un ciel clair
const PANEL_A: Color = Color(0.15, 0.21, 0.11, 0.94)
const CARD: Color = Color(0.11, 0.16, 0.08, 0.92)      # carte de valeur (un cran plus sombre que le panneau)
const INK: Color = Color(0.13, 0.10, 0.04)             # texte sombre sur fond or
const TEXT: Color = Color(0.97, 0.94, 0.82)            # ivoire
const TEXT_DIM: Color = Color(0.86, 0.83, 0.70)        # parchemin atténué (était 0.80/0.76/0.60 : 4,37:1, échec)
const TEXT_OFF: Color = Color(0.66, 0.67, 0.56)        # désactivé (exempté par le SC 1.4.3, gardé lisible)
const LINE: Color = Color(0.80, 0.66, 0.36)            # filet or (était 0.70/0.56/0.28 : 2,57:1, échec)
const LINE_HI: Color = Color(0.95, 0.82, 0.48)         # filet survol
const LINE_DIM: Color = Color(0.55, 0.60, 0.45, 0.85)  # séparateurs discrets (décoratifs, non porteurs de sens)
const ACCENT: Color = Color(0.95, 0.80, 0.45)          # or : sélection, choix actif, titre
const ACCENT_BG: Color = Color(0.90, 0.72, 0.36, 0.97) # fond or des boutons enfoncés (texte INK)
const BTN_BG: Color = Color(0.08, 0.12, 0.05, 0.55)
const BTN_HOVER: Color = Color(0.30, 0.38, 0.21, 0.97)
const BTN_OFF: Color = Color(0.22, 0.25, 0.18, 0.40)
const FOCUS_RING: Color = Color(0.99, 0.95, 0.78, 0.95) # focus = contour seul, jamais un remplissage
const VEIL: Color = Color(0.09, 0.12, 0.06, 0.72)
const TITLE: Color = Color(0.99, 0.95, 0.80)
const TITLE_SHADOW: Color = Color(0.10, 0.08, 0.20, 0.55)
const SLIDER_TRACK: Color = Color(0.05, 0.07, 0.03, 0.85)
const SLIDER_FILL: Color = Color(0.86, 0.68, 0.32)
const PLANT_BG: Color = Color(0.93, 0.76, 0.38, 0.98)
# --- états (texte : >= 4,5:1 sur le panneau) ------------------------------------------------------------------
const URGENT: Color = Color(1.0, 0.56, 0.42)           # urgent : déclin, mort, tempête, chablis (+ triangle)
const URGENT_BG: Color = Color(0.42, 0.13, 0.07, 0.96) # fond des bandeaux urgents (texte URGENT ou TEXT)
const WARN: Color = Color(1.0, 0.74, 0.40)             # avertissement : année sèche, risque
const GOOD: Color = Color(0.55, 0.85, 0.50)            # état vigoureux, sécurité élevée
const WATER: Color = Color(0.45, 0.72, 0.88)           # eau (jauge, courbe)
const SPARK: Color = Color(0.86, 0.83, 0.70)           # tracé des sparklines
const SWATCH_EDGE: Color = Color(0.86, 0.83, 0.70)     # liseré clair des pastilles sombres (SC 1.4.11)
## Pire fond derrière un panneau translucide : ciel clair d'été (mesure des contrastes).
const SKY_WORST: Color = Color(0.80, 0.86, 0.92)

# --- palette des planches scolaires ---------------------------------------------------------------------------
const PAPER: Color = Color(0.925, 0.885, 0.760)
const PAPER_DARK: Color = Color(0.86, 0.80, 0.64)
const PAPER_INK: Color = Color(0.08, 0.07, 0.06)
const PLATE_BLUE: Color = Color(0.10, 0.25, 0.58)
const PLATE_RED: Color = Color(0.71, 0.12, 0.10)
const WALL: Color = Color(0.20, 0.25, 0.15)

const SERIF_NAMES: PackedStringArray = ["Noto Serif", "DejaVu Serif", "Liberation Serif", "Times New Roman", "serif"]
## Plus petite taille de texte autorisée (px à la base 1280×720) ; le selftest la vérifie sur tous les Labels et Buttons.
const FONT_MIN: int = 12
## Taille de base du thème en jeu.
const FONT_BASE: int = 15

static var _fonts: Dictionary = {}
static var _grabber: ImageTexture
static var _theme: Theme


## Police à empattements. `weight` 400 / 700, italique facultatif, interlettrage (petites capitales).
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


static func panel_box(alpha: float = PANEL_ALPHA) -> StyleBoxFlat:
	return box(Color(PANEL.r, PANEL.g, PANEL.b, alpha), Color(LINE.r, LINE.g, LINE.b, 0.85), 1, 10.0, 8.0)


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


## Case à cocher 16 px : contour ivoire (3:1 au moins sur le panneau) ; cochée, fond or et coche sombre.
static func _check_tex(on: bool) -> ImageTexture:
	var n: int = 16
	var img: Image = Image.create(n, n, false, Image.FORMAT_RGBA8)
	img.fill(Color(0, 0, 0, 0))
	for y: int in range(1, n - 1):
		for x: int in range(1, n - 1):
			var edge: bool = x == 1 or y == 1 or x == n - 2 or y == n - 2
			if on:
				img.set_pixel(x, y, ACCENT if not edge else LINE_HI)
			elif edge:
				img.set_pixel(x, y, TEXT_DIM)
			else:
				img.set_pixel(x, y, Color(0.05, 0.07, 0.03, 0.7))
	if on:
		# Coche : deux segments épais.
		for t: int in 5:
			for w: int in 2:
				img.set_pixel(4 + t, 7 + t + w - 1, INK)
		for t: int in 6:
			for w: int in 2:
				img.set_pixel(8 + t - 1, 11 - t * 1 + w - 1, INK)
	return ImageTexture.create_from_image(img)


## Le focus est un contour dessiné par-dessus (sans remplissage) : il ne ressemble jamais à « sélectionné » (fond or).
static func focus_box(mx: float, my: float) -> StyleBoxFlat:
	return box(Color(0, 0, 0, 0), FOCUS_RING, 2, mx, my)


## Applique l'apparence bouton (tous les états) à un Control (surcharges) ou à un type du Theme.
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


## Le thème unique de l'application (mis en cache). Variantes de type :
##   PanelContainer : BarPanel (barres haute et basse), SidePanel (panneau latéral), Card (carte de valeur),
##                    LegendPanel (bandeau de légende), UrgentPanel (bandeau urgent), MenuPanel (menu ≡).
##   Label : CardTitle (petites capitales atténuées), CardValue (valeur), DimLabel, SmallLabel (12 px),
##           UrgentLabel, HeaderLabel (titre de panneau).
##   Button : BarButton (compact, barres), FlatButton (sans cadre, en-têtes et croix de fermeture).
static func theme() -> Theme:
	if _theme == null:
		_theme = _build_theme()
	return _theme


## Compatibilité : les anciens appels `make_theme(taille, ...)` reçoivent le thème unique.
static func make_theme(_font_size: int = FONT_BASE, _pad_x: float = 12.0, _pad_y: float = 6.0) -> Theme:
	return theme()


static func _build_theme() -> Theme:
	var th: Theme = Theme.new()
	th.default_font = font()
	th.default_font_size = FONT_BASE
	th.set_color("font_color", "Label", TEXT)
	th.set_color("font_shadow_color", "Label", Color(0, 0, 0, 0.0))
	th.set_stylebox("panel", "PanelContainer", panel_box())
	th.set_stylebox("panel", "Panel", panel_box())
	for t: String in ["Button", "OptionButton", "MenuButton"]:
		_button_styles(th, t, 10.0, 4.0)
	th.set_constant("h_separation", "OptionButton", 4)
	th.set_color("font_color", "CheckBox", TEXT)
	th.set_color("font_hover_color", "CheckBox", TEXT)
	th.set_color("font_pressed_color", "CheckBox", TEXT)
	th.set_color("font_hover_pressed_color", "CheckBox", TEXT)
	th.set_color("font_color", "CheckButton", TEXT)
	th.set_color("font_pressed_color", "CheckButton", TEXT)
	th.set_color("font_hover_color", "CheckButton", TEXT)
	th.set_color("font_hover_pressed_color", "CheckButton", TEXT)
	th.set_color("font_color", "OptionButton", TEXT)
	# Cases à cocher : sans cadre de bouton, case dessinée (contour ivoire ; cochée = fond or + coche sombre).
	for t: String in ["CheckBox", "CheckButton"]:
		var empty: StyleBoxFlat = box(Color(0, 0, 0, 0), Color(0, 0, 0, 0), 0, 4.0, 2.0)
		for st: String in ["normal", "pressed", "disabled", "hover_pressed"]:
			th.set_stylebox(st, t, empty)
		th.set_stylebox("hover", t, box(Color(BTN_HOVER.r, BTN_HOVER.g, BTN_HOVER.b, 0.6), Color(0, 0, 0, 0), 0, 4.0, 2.0))
		th.set_stylebox("hover_pressed", t, box(Color(BTN_HOVER.r, BTN_HOVER.g, BTN_HOVER.b, 0.6), Color(0, 0, 0, 0), 0, 4.0, 2.0))
		th.set_stylebox("focus", t, focus_box(4.0, 2.0))
		th.set_constant("h_separation", t, 6)
	th.set_icon("checked", "CheckBox", _check_tex(true))
	th.set_icon("unchecked", "CheckBox", _check_tex(false))
	th.set_icon("checked_disabled", "CheckBox", _check_tex(true))
	th.set_icon("unchecked_disabled", "CheckBox", _check_tex(false))
	# Liste déroulante des OptionButton
	th.set_stylebox("panel", "PopupMenu", box(Color(0.11, 0.16, 0.08, 0.99), LINE, 1, 4.0, 4.0))
	th.set_stylebox("hover", "PopupMenu", box(ACCENT_BG, Color(0, 0, 0, 0), 0, 6.0, 3.0))
	th.set_color("font_color", "PopupMenu", TEXT)
	th.set_color("font_hover_color", "PopupMenu", INK)
	th.set_color("font_disabled_color", "PopupMenu", TEXT_OFF)
	th.set_color("font_separator_color", "PopupMenu", ACCENT)
	th.set_constant("v_separation", "PopupMenu", 6)
	th.set_font_size("font_size", "PopupMenu", FONT_BASE)
	th.set_font_size("font_separator_size", "PopupMenu", 13)
	# Info-bulles
	th.set_stylebox("panel", "TooltipPanel", box(Color(0.09, 0.13, 0.06, 0.98), LINE, 1, 9.0, 6.0))
	th.set_color("font_color", "TooltipLabel", TEXT)
	th.set_font_size("font_size", "TooltipLabel", 14)
	# Curseurs : rail fin, remplissage or, bouton rond
	var track: StyleBoxFlat = box(SLIDER_TRACK, Color(0, 0, 0, 0), 0, 0.0, 0.0)
	track.content_margin_top = 2.0
	track.content_margin_bottom = 2.0
	th.set_stylebox("slider", "HSlider", track)
	th.set_stylebox("grabber_area", "HSlider", box(SLIDER_FILL.darkened(0.12), Color(0, 0, 0, 0), 0, 0.0, 0.0))
	th.set_stylebox("grabber_area_highlight", "HSlider", box(SLIDER_FILL, Color(0, 0, 0, 0), 0, 0.0, 0.0))
	th.set_icon("grabber", "HSlider", _grabber_tex())
	th.set_icon("grabber_highlight", "HSlider", _grabber_tex())
	th.set_icon("grabber_disabled", "HSlider", _grabber_tex())
	# Barre de progression
	th.set_stylebox("background", "ProgressBar", box(SLIDER_TRACK, Color(LINE.r, LINE.g, LINE.b, 0.7), 1, 0.0, 0.0))
	th.set_stylebox("fill", "ProgressBar", box(WATER, Color(0, 0, 0, 0), 0, 0.0, 0.0))
	# Champ de saisie
	th.set_stylebox("normal", "LineEdit", box(Color(0.05, 0.08, 0.03, 0.55), LINE, 1, 8.0, 5.0))
	th.set_stylebox("focus", "LineEdit", box(Color(0.05, 0.08, 0.03, 0.70), LINE_HI, 1, 8.0, 5.0))
	th.set_color("font_color", "LineEdit", TEXT)
	th.set_color("caret_color", "LineEdit", ACCENT)
	th.set_color("font_placeholder_color", "LineEdit", TEXT_OFF)
	th.set_color("selection_color", "LineEdit", Color(0.90, 0.72, 0.36, 0.40))
	# Texte riche
	th.set_font("normal_font", "RichTextLabel", font())
	th.set_font("bold_font", "RichTextLabel", font(700))
	th.set_font("italics_font", "RichTextLabel", font(400, true))
	th.set_font("bold_italics_font", "RichTextLabel", font(700, true))
	for k: String in ["normal_font_size", "bold_font_size", "italics_font_size", "bold_italics_font_size"]:
		th.set_font_size(k, "RichTextLabel", 14)
	th.set_color("default_color", "RichTextLabel", TEXT)
	th.set_stylebox("normal", "RichTextLabel", StyleBoxEmpty.new())
	th.set_stylebox("separator", "HSeparator", box(LINE_DIM, LINE_DIM, 0, 0.0, 0.0))
	th.set_constant("separation", "HSeparator", 6)
	th.set_stylebox("panel", "ScrollContainer", StyleBoxEmpty.new())

	# --- variantes : panneaux ---
	th.set_type_variation("BarPanel", "PanelContainer")
	th.set_stylebox("panel", "BarPanel", box(PANEL_A, Color(LINE.r, LINE.g, LINE.b, 0.85), 1, 8.0, 4.0))
	th.set_type_variation("SidePanel", "PanelContainer")
	th.set_stylebox("panel", "SidePanel", box(PANEL_A, Color(LINE.r, LINE.g, LINE.b, 0.85), 1, 10.0, 8.0))
	th.set_type_variation("Card", "PanelContainer")
	th.set_stylebox("panel", "Card", box(CARD, Color(LINE_DIM.r, LINE_DIM.g, LINE_DIM.b, 0.55), 1, 8.0, 5.0))
	th.set_type_variation("LegendPanel", "PanelContainer")
	th.set_stylebox("panel", "LegendPanel", box(PANEL_A, Color(LINE.r, LINE.g, LINE.b, 0.6), 1, 8.0, 3.0))
	th.set_type_variation("UrgentPanel", "PanelContainer")
	th.set_stylebox("panel", "UrgentPanel", box(URGENT_BG, URGENT, 2, 10.0, 5.0))
	th.set_type_variation("MenuPanel", "PanelContainer")
	th.set_stylebox("panel", "MenuPanel", box(Color(0.11, 0.16, 0.08, 0.98), LINE, 1, 12.0, 10.0))
	# --- variantes : textes ---
	_label_variation(th, "CardTitle", TEXT_DIM, 12, font(400, false, 2))
	_label_variation(th, "CardValue", TEXT, 17, font(700))
	_label_variation(th, "DimLabel", TEXT_DIM, 14, null)
	_label_variation(th, "SmallLabel", TEXT_DIM, FONT_MIN, null)
	_label_variation(th, "UrgentLabel", URGENT, 14, font(700))
	_label_variation(th, "HeaderLabel", TEXT, 14, font(400, false, 3))
	# --- variantes : boutons ---
	th.set_type_variation("BarButton", "Button")
	_button_styles(th, "BarButton", 9.0, 3.0)
	th.set_font_size("font_size", "BarButton", 15)
	th.set_type_variation("FlatButton", "Button")
	th.set_stylebox("normal", "FlatButton", StyleBoxEmpty.new())
	th.set_stylebox("pressed", "FlatButton", StyleBoxEmpty.new())
	th.set_stylebox("focus", "FlatButton", StyleBoxEmpty.new())
	th.set_stylebox("hover", "FlatButton", box(BTN_HOVER, Color(0, 0, 0, 0), 0, 4.0, 2.0))
	th.set_stylebox("hover_pressed", "FlatButton", box(BTN_HOVER, Color(0, 0, 0, 0), 0, 4.0, 2.0))
	th.set_color("font_color", "FlatButton", TEXT_DIM)
	th.set_color("font_hover_color", "FlatButton", TEXT)
	th.set_color("font_pressed_color", "FlatButton", TEXT)
	th.set_color("font_hover_pressed_color", "FlatButton", TEXT)
	th.set_font_size("font_size", "FlatButton", 14)
	return th


static func _label_variation(th: Theme, name: String, col: Color, size: int, f: Font) -> void:
	th.set_type_variation(name, "Label")
	th.set_color("font_color", name, col)
	th.set_font_size("font_size", name, size)
	if f != null:
		th.set_font("font", name, f)


## Label d'une variante du thème.
static func label(text: String, variation: String = "") -> Label:
	var l: Label = Label.new()
	l.text = text
	l.theme_type_variation = variation
	return l


## Bouton de menu / de formulaire : bordé, discret, texte légèrement espacé.
static func button(text: String, size: int = 18, align: HorizontalAlignment = HORIZONTAL_ALIGNMENT_LEFT) -> Button:
	var b: Button = Button.new()
	b.text = text
	b.alignment = align
	b.add_theme_font_override("font", font(400, false, 1))
	b.add_theme_font_size_override("font_size", maxi(size, FONT_MIN))
	apply_button(func(n: String, sb: StyleBox) -> void: b.add_theme_stylebox_override(n, sb),
		func(n: String, c: Color) -> void: b.add_theme_color_override(n, c), 18.0, 9.0)
	return b


## Titre en petites capitales (majuscules espacées). Couleur atténuée par défaut : l'or est réservé à la sélection.
static func caps_label(text: String, size: int = 13, color: Color = TEXT_DIM) -> Label:
	var l: Label = Label.new()
	l.text = text.to_upper()
	l.add_theme_font_override("font", font(400, false, 3))
	l.add_theme_font_size_override("font_size", maxi(size, FONT_MIN))
	l.add_theme_color_override("font_color", color)
	return l


static func serif_label(text: String, size: int, color: Color = TEXT, weight: int = 400, italic: bool = false) -> Label:
	var l: Label = Label.new()
	l.text = text
	l.add_theme_font_override("font", font(weight, italic))
	l.add_theme_font_size_override("font_size", maxi(size, FONT_MIN))
	l.add_theme_color_override("font_color", color)
	return l


## Filet horizontal fin.
static func rule(color: Color = LINE) -> ColorRect:
	var r: ColorRect = ColorRect.new()
	r.color = color
	r.custom_minimum_size = Vector2(0, 1)
	r.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return r


## Prend le focus à la frame suivante, si le contrôle est encore dans l'arbre.
static func focus_later(c: Control) -> void:
	(func() -> void:
		if is_instance_valid(c) and c.is_inside_tree() and c.is_visible_in_tree():
			c.grab_focus()).call_deferred()


## Nombre à la française : virgule décimale.
static func num(v: float, digits: int = 1) -> String:
	return (("%." + str(digits) + "f") % v).replace(".", ",")


# --------------------------------------------------------------------------------------- contrastes WCAG 2.2

static func _lin(c: float) -> float:
	return c / 12.92 if c <= 0.04045 else pow((c + 0.055) / 1.055, 2.4)


## Luminance relative (WCAG 2.2) d'une couleur sRGB opaque.
static func luminance(c: Color) -> float:
	return 0.2126 * _lin(c.r) + 0.7152 * _lin(c.g) + 0.0722 * _lin(c.b)


## Couleur `fg` (avec alpha) composée sur `bg` opaque.
static func over(fg: Color, bg: Color) -> Color:
	return Color(fg.r * fg.a + bg.r * (1.0 - fg.a), fg.g * fg.a + bg.g * (1.0 - fg.a), fg.b * fg.a + bg.b * (1.0 - fg.a), 1.0)


## Rapport de contraste WCAG (1..21) entre deux couleurs opaques.
static func contrast(a: Color, b: Color) -> float:
	var la: float = luminance(a)
	var lb: float = luminance(b)
	return (maxf(la, lb) + 0.05) / (minf(la, lb) + 0.05)


## Les paires mesurées : [nom, avant-plan, fond, seuil]. Le fond « panneau » est le pire cas (panneau translucide
## devant un ciel clair). Les pastilles sombres (horizons, sécurité) sont mesurées par leur liseré SWATCH_EDGE.
static func contrast_pairs() -> Array:
	var panel: Color = over(PANEL_A, SKY_WORST)
	var card: Color = over(CARD, panel)
	var btn: Color = over(BTN_BG, panel)
	var hov: Color = over(BTN_HOVER, panel)
	var acc: Color = over(ACCENT_BG, panel)
	var track: Color = over(SLIDER_TRACK, panel)
	var urg: Color = over(URGENT_BG, panel)
	return [
		["texte ivoire / panneau", TEXT, panel, 4.5],
		["texte atténué / panneau", TEXT_DIM, panel, 4.5],
		["texte atténué / carte", TEXT_DIM, card, 4.5],
		["or (sélection) / panneau", ACCENT, panel, 4.5],
		["texte / bouton", TEXT, btn, 4.5],
		["texte / bouton survolé", TEXT, hov, 4.5],
		["encre / bouton enfoncé (or)", INK, acc, 4.5],
		["urgent / panneau", URGENT, panel, 4.5],
		["urgent / bandeau urgent", URGENT, urg, 4.5],
		["ivoire / bandeau urgent", TEXT, urg, 4.5],
		["avertissement / panneau", WARN, panel, 4.5],
		["vigoureux / panneau", GOOD, panel, 4.5],
		["eau / panneau", WATER, panel, 3.0],
		["bordure de bouton / panneau", LINE, panel, 3.0],
		["bordure de bouton / bouton", LINE, btn, 3.0],
		["anneau de focus / panneau", FOCUS_RING, panel, 3.0],
		["remplissage du curseur / rail", SLIDER_FILL, track, 3.0],
		["jauge d'eau / rail", WATER, track, 3.0],
		["sparkline / carte", SPARK, card, 3.0],
		["liseré des pastilles / panneau", SWATCH_EDGE, panel, 3.0],
	]
