class_name Hud
extends CanvasLayer
## Interface en jeu, direction hybride de docs/ui-recherche.md (décisions du 2026-11-21) :
## * squelette D1 : l'arbre occupe l'écran, une barre fine en haut (identité, panneaux, calques, Expert, aide, ≡)
##   et une en bas (lecture et vitesse, actions Sécheresse / Taille / Tempête, jauge d'eau du sol) ;
## * niveau 2 à la demande : un seul panneau latéral ouvert à la fois (PanelSlot : Cernes, Flux, Sol, Mécanique,
##   Taille) ;
## * calques D3 (Eau, Flux), un seul à la fois, avec leur légende forme + couleur au-dessus de la barre du bas ;
## * mode Expert : colonne permanente de cartes D2 (valeur, sparkline sur l'historique de la partie, bullet graph).
## Tout le texte visible est en français. Le Hud ne pilote pas la simulation : il émet des signaux (main.gd).

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
signal overlay_changed(id: String)
signal storm_requested
signal expert_toggled(on: bool)
signal root_view_requested
signal save_requested
signal load_requested
signal pause_requested

const MONTHS: Array[String] = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet",
	"août", "septembre", "octobre", "novembre", "décembre"]
const MONTH_DAYS: Array[int] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
const PHASES: Array[String] = ["Dormance hivernale", "Débourrement", "Expansion des feuilles",
	"Été — feuillage mature", "Coloration automnale", "Chute des feuilles"]
## Les mêmes phases pour un persistant (l'« automne » est la chute de la plus vieille cohorte d'aiguilles).
const PHASES_EVERGREEN: Array[String] = ["Repos hivernal — feuillage persistant", "Débourrement", "Allongement des pousses",
	"Été — feuillage mature", "Automne — les plus vieilles aiguilles jaunissent", "Chute des plus vieilles aiguilles"]
## Forme courte de la saison pour la barre du haut (la forme longue est dans l'info-bulle).
const SEASONS_SHORT: Array[String] = ["Hiver", "Débourrement", "Feuillaison", "Été", "Automne", "Chute des feuilles"]
const SEASONS_SHORT_EVERGREEN: Array[String] = ["Hiver", "Débourrement", "Pousses", "Été", "Automne", "Chute d'aiguilles"]
const MAX_SPEED_DAYS: float = 3650.0 # 10 ans / s
const SOIL_LABELS: Array[String] = ["Limon profond", "Mince sur calcaire", "Argile engorgée", "Sable sec"]

const EDGE: float = 8.0         # marge des barres au bord de l'écran
const GAP: float = 8.0          # écart entre barres, colonne, panneau et zone 3D
const EXPERT_W: float = 236.0   # largeur de la colonne Expert
## Panneaux de niveau 2 : identifiant, titre, touche (texte), info-bulle.
const PANELS: Array = [
	["cernes", "Cernes", "C", "Coupe du tronc à 1,30 m : cernes annuels et courbe de leurs largeurs"],
	["flux", "Flux", "L", "Bilan des flux du jour (eau, sels, sucres, gaz, mycorhizes) ; active le calque Flux"],
	["sol", "Sol", "S", "Horizons, eau de chaque couche, nappe, roche ; opacité du sol"],
	["meca", "Mécanique", "M", "Tenue au vent et au poids propre, vitesses critiques, tempêtes"],
]
## Calques : identifiant, nom, touche, info-bulle.
const OVERLAYS: Array = [
	["", "Aucun", "0", "Pas de calque : l'arbre seul"],
	["eau", "Eau du sol", "1", "Le sol coloré selon son eau (sec hachuré, humide plein, saturé à pois) et l'eau puisée par les racines"],
	["flux", "Flux", "2", "Tous les flux sur leur trajet réel : sève brute, sels, sève élaborée, gaz, mycorhizes"],
]
## Aide (F1) : groupes [titre, [[touche, action], ...]].
const SHORTCUTS: Array = [
	["Temps", [["Espace", "Lecture / pause"], ["Échap", "Menu pause (sauvegarder, options…)"], ["F5 / F9", "Sauvegarde / chargement rapides"]]],
	["Vue", [["Glisser", "Tourner autour de l'arbre"], ["Clic droit", "Déplacer la vue"], ["Molette", "Zoom"],
		["Double-clic", "Recentrer sur le tronc"], ["F", "Recadrer l'arbre"], ["R", "Vue des racines"],
		["E", "Vue d'ensemble"], ["X", "Sol transparent / opaque"], ["G", "Repères d'échelle"]]],
	["Panneaux (un seul à la fois)", [["C", "Cernes"], ["L", "Flux"], ["S", "Sol"], ["M", "Mécanique"], ["T", "Taille (outil)"]]],
	["Calques et affichage", [["1", "Calque Eau du sol"], ["2", "Calque Flux"], ["0", "Aucun calque"], ["I", "Mode Expert"],
		["F1 · ? · H", "Cette aide"]]],
	["Taille (outil actif)", [["Clic", "Couper la branche survolée"], ["Maj + clic", "Coupe rase"], ["Ctrl+Z", "Annuler la coupe"]]],
]

# --- état -------------------------------------------------------------------------------------------------------
var overlay: String = "flux"     # calque actif : "", "eau", "flux"
var expert: bool = false
var history: GameHistory = GameHistory.new()
var _last_ring_year: int = -1
var _state_key: int = -1
var _state_level: int = 0        # 0 bien, 1 déclin, 2 urgent
var _alert_text: String = ""
var _toast_t: float = 0.0
var _hist_year: int = -1
var _expert_age: float = 10.0
var _species_common: PackedStringArray = PackedStringArray()
var _species_sci: PackedStringArray = PackedStringArray()

# --- nœuds ------------------------------------------------------------------------------------------------------
var _root: Control
var _info_panel: PanelContainer   # barre du haut (nom gardé : prune_tool et les tests le lisent)
var _bottom_panel: PanelContainer # barre du bas
var _name_lbl: Label
var _sci_lbl: Label
var _ident: Label
var _alert: HBoxContainer
var _alert_lbl: Label
var _toast: PanelContainer
var _toast_lbl: Label
var slot: PanelSlot
var rings_panel: RingsPanel
var soil_panel: SoilPanel
var overlay_legend: OverlayLegend
var _legend: OverlayLegend        # horizons du sol (sol transparent ou vue des racines)
var _legends: VBoxContainer
var _expert_col: ScrollContainer
var _menu: PanelContainer         # menu ≡
var _overlay_menu: PanelContainer # choix du calque
var help: HelpOverlay
var hover: OrganHover
var _water_gauge: BulletGraph
var _water_pct: Label
var _speed_label: Label
var _seed_label: Label
var _play: Button
var _speed_slider: HSlider
# Cartes du mode Expert
var _card_h: ValueCard
var _card_d: ValueCard
var _card_r: ValueCard
var _card_w: ValueCard
var _card_t: ValueCard
var _card_root: ValueCard
var _wind_spark: Sparkline
var _state: Label   # « État » : vigueur, réserves, mort du tronc (dendro_core::vitality)
var _wind: Label    # vent du jour (moyen et rafale à 10 m)
var _fallen_lbl: Label
var _root_spread: Label

## Contrôles exposés : le selftest et les accroches DENDRO_* passent par les mêmes signaux qu'un clic.
var play_button: Button
var speed_slider: HSlider
var drought_button: Button
var prune_button: Button
var storm_button: Button
var rings_button: Button
var flux_panel_button: Button
var soil_panel_button: Button
var mech_button: Button
var overlay_button: Button
var water_button: Button        # calque Eau
var flux_button: Button         # calque Flux
var none_button: Button         # aucun calque
var expert_button: Button
var help_button: Button
var menu_button: Button
var seed_button: Button
var guides_button: Button
var expert_menu_check: CheckBox
var soil_slider: HSlider
var soil_option: OptionButton
var species_option: OptionButton
## Anciennes colonnes latérales (compatibilité) : non utilisées, l'emplacement unique les remplace.
var side_panels: Array[Control] = []
var _panel_buttons: Dictionary = {}   # id -> Button


func _ready() -> void:
	_root = Control.new()
	_root.set_anchors_preset(Control.PRESET_FULL_RECT)
	_root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_root.theme = UiStyle.theme()
	add_child(_root)
	_build_expert(_root)
	_build_legends(_root)
	slot = PanelSlot.new()
	_root.add_child(slot)
	slot.changed.connect(_on_slot_changed)
	rings_panel = RingsPanel.new()
	slot.register("cernes", rings_panel, "Cernes de croissance")
	soil_panel = SoilPanel.new()
	soil_panel.root_view_requested.connect(func() -> void: root_view_requested.emit())
	slot.register("sol", soil_panel, "Sol")
	soil_slider = soil_panel.soil_slider
	soil_slider.value_changed.connect(func(v: float) -> void: soil_opacity_changed.emit(v))
	_build_top_bar(_root)
	_build_bottom_bar(_root)
	_build_menus(_root)
	_build_toast(_root)
	hover = OrganHover.new()
	hover.blocked = _hover_blocked
	_root.add_child(hover)
	help = HelpOverlay.new()
	help.set_shortcuts(SHORTCUTS)
	help.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
	help.grow_horizontal = Control.GROW_DIRECTION_BOTH
	help.grow_vertical = Control.GROW_DIRECTION_BOTH
	_root.add_child(help)
	_sync_overlay_buttons()
	_apply_overlay_legend()
	set_expert(_initial_expert(), false)
	Game.settings_changed.connect(func() -> void:
		var want: bool = bool(Game.settings.get("expert", false))
		if want != expert:
			set_expert(want, false))


## Mode Expert au lancement : réglage mémorisé (options) ; en automatisation, seulement DENDRO_EXPERT=1
## (le réglage réel de l'utilisateur ne doit pas changer les tests ni les captures).
static func _initial_expert() -> bool:
	if Game.is_automation():
		return OS.get_environment("DENDRO_EXPERT") == "1"
	return bool(Game.settings.get("expert", false))


func _no_focus(c: Control) -> Control:
	c.focus_mode = Control.FOCUS_NONE # Espace reste à lecture / pause
	return c


func _bar_button(text: String, tip: String, toggle: bool = true) -> Button:
	var b: Button = _no_focus(Button.new()) as Button
	b.text = text
	b.toggle_mode = toggle
	b.theme_type_variation = "BarButton"
	b.tooltip_text = tip
	return b


# ------------------------------------------------------------------------------------------------ barre haute

func _build_top_bar(root: Control) -> void:
	var p: PanelContainer = PanelContainer.new()
	p.theme_type_variation = "BarPanel"
	p.set_anchors_preset(Control.PRESET_TOP_WIDE)
	p.offset_left = EDGE
	p.offset_right = -EDGE
	p.offset_top = EDGE
	p.mouse_filter = Control.MOUSE_FILTER_STOP
	_info_panel = p
	root.add_child(p)
	var h: HBoxContainer = HBoxContainer.new()
	h.add_theme_constant_override("separation", 6)
	p.add_child(h)
	_name_lbl = UiStyle.label("Chêne pédonculé", "")
	_name_lbl.add_theme_font_override("font", UiStyle.font(700))
	_name_lbl.add_theme_font_size_override("font_size", 16)
	h.add_child(_name_lbl)
	_sci_lbl = UiStyle.label("Quercus robur", "DimLabel")
	_sci_lbl.add_theme_font_override("font", UiStyle.font(400, true))
	h.add_child(_sci_lbl)
	_ident = UiStyle.label("", "")
	_ident.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_ident.clip_text = true
	_ident.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	_ident.mouse_filter = Control.MOUSE_FILTER_PASS
	h.add_child(_ident)
	# Signal urgent : triangle dessiné + texte, couleur dédiée.
	_alert = HBoxContainer.new()
	_alert.add_theme_constant_override("separation", 4)
	_alert.mouse_filter = Control.MOUSE_FILTER_PASS
	_alert.visible = false
	_alert.add_child(LegendGlyph.new("tri", UiStyle.URGENT))
	_alert_lbl = UiStyle.label("", "UrgentLabel")
	_alert.add_child(_alert_lbl)
	h.add_child(_alert)
	h.add_child(VSeparator.new())
	for e: Array in PANELS:
		var id: String = e[0]
		var b: Button = _bar_button(e[1], "%s (%s) — %s" % [e[1], e[2], e[3]])
		b.toggled.connect(func(on: bool) -> void: _on_panel_button(id, on))
		_panel_buttons[id] = b
		h.add_child(b)
	rings_button = _panel_buttons["cernes"]
	flux_panel_button = _panel_buttons["flux"]
	soil_panel_button = _panel_buttons["sol"]
	mech_button = _panel_buttons["meca"]
	h.add_child(VSeparator.new())
	overlay_button = _bar_button("Calque", "Calques (1 : Eau du sol, 2 : Flux, 0 : aucun) : un thème à la fois, peint sur l'arbre et le sol")
	overlay_button.toggled.connect(func(on: bool) -> void: _show_dropdown(_overlay_menu, overlay_button, on))
	h.add_child(overlay_button)
	expert_button = _bar_button("Expert", "Mode Expert (I) : colonne permanente des valeurs, avec leur évolution depuis le début de la partie")
	expert_button.toggled.connect(func(on: bool) -> void: set_expert(on))
	h.add_child(expert_button)
	help_button = _bar_button("?", "Aide : raccourcis clavier et souris (F1, ?, H)", false)
	help_button.pressed.connect(toggle_help)
	h.add_child(help_button)
	menu_button = _bar_button("≡", "Menu : espèce, terrain, nouvelle graine, repères, sauvegarde, pause")
	menu_button.toggled.connect(func(on: bool) -> void: _show_dropdown(_menu, menu_button, on))
	h.add_child(menu_button)


# ------------------------------------------------------------------------------------------------- barre basse

func _build_bottom_bar(root: Control) -> void:
	var p: PanelContainer = PanelContainer.new()
	p.theme_type_variation = "BarPanel"
	p.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
	p.grow_vertical = Control.GROW_DIRECTION_BEGIN
	p.offset_left = EDGE
	p.offset_right = -EDGE
	p.offset_bottom = -EDGE
	p.mouse_filter = Control.MOUSE_FILTER_STOP
	_bottom_panel = p
	root.add_child(p)
	var h: HBoxContainer = HBoxContainer.new()
	h.add_theme_constant_override("separation", 6)
	p.add_child(h)

	_play = _bar_button("Pause", "Lecture / pause (Espace)")
	_play.button_pressed = true
	_play.custom_minimum_size = Vector2(86, 0)
	_play.toggled.connect(_on_play_toggled)
	play_button = _play
	h.add_child(_play)
	_on_play_toggled(true)
	h.add_child(UiStyle.label("Vitesse", "DimLabel"))
	_speed_slider = _no_focus(HSlider.new()) as HSlider
	_speed_slider.min_value = 0.0
	_speed_slider.max_value = 1.0
	_speed_slider.step = 0.001
	_speed_slider.value = log(30.0) / log(MAX_SPEED_DAYS)
	_speed_slider.custom_minimum_size = Vector2(150, 0)
	_speed_slider.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	_speed_slider.tooltip_text = "Vitesse de la simulation : de 1 jour à 10 ans par seconde"
	_speed_slider.value_changed.connect(_on_speed_changed)
	speed_slider = _speed_slider
	h.add_child(_speed_slider)
	_speed_label = UiStyle.label("", "")
	_speed_label.custom_minimum_size = Vector2(84, 0)
	h.add_child(_speed_label)

	var sp1: Control = Control.new()
	sp1.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	h.add_child(sp1)
	drought_button = _bar_button("Sécheresse", "Sécheresse forcée : ni pluie, évaporation accrue (ouvre le calque Eau du sol)")
	drought_button.toggled.connect(func(on: bool) -> void:
		if on:
			set_overlay("eau")
		drought_toggled.emit(on))
	h.add_child(drought_button)
	prune_button = _bar_button("✂ Taille", "Outil de taille (T) : survoler une branche, clic pour couper ; ouvre le panneau Taille")
	prune_button.toggled.connect(func(on: bool) -> void:
		if on and slot.has_panel("taille"):
			slot.open("taille")
		elif not on and slot.current == "taille":
			slot.close()
		prune_toggled.emit(on))
	h.add_child(prune_button)
	storm_button = _bar_button("Tempête", "Tempête : une rafale frappe l'arbre maintenant, ce qui cède casse (vitesse réglable dans Mécanique ; ouvre ce panneau)", false)
	storm_button.pressed.connect(func() -> void: storm_requested.emit())
	h.add_child(storm_button)
	var sp2: Control = Control.new()
	sp2.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	h.add_child(sp2)

	var wl: Button = _bar_button("Eau du sol", "Eau disponible dans la zone racinaire ; trait : seuil de sécheresse. Clic : calque Eau du sol (1)", false)
	wl.theme_type_variation = "FlatButton"
	wl.pressed.connect(func() -> void: set_overlay("" if overlay == "eau" else "eau"))
	h.add_child(wl)
	_water_gauge = BulletGraph.new()
	_water_gauge.bands = PackedFloat32Array([SoilPanel.DRY, 0.6])
	_water_gauge.threshold = SoilPanel.DRY
	_water_gauge.bar_color = UiStyle.WATER
	_water_gauge.custom_minimum_size = Vector2(120, 14)
	_water_gauge.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	h.add_child(_water_gauge)
	_water_pct = UiStyle.label("", "")
	_water_pct.custom_minimum_size = Vector2(44, 0)
	_water_pct.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	h.add_child(_water_pct)
	_on_speed_changed(_speed_slider.value)


# -------------------------------------------------------------------------------------- menus déroulants

func _dropdown_panel() -> PanelContainer:
	var d: PanelContainer = PanelContainer.new()
	d.theme_type_variation = "MenuPanel"
	d.mouse_filter = Control.MOUSE_FILTER_STOP
	d.visible = false
	return d


func _build_menus(root: Control) -> void:
	# Calques (exclusivité gérée par set_overlay / _sync_overlay_buttons, pas par un ButtonGroup : un groupe
	# relâcherait les autres boutons avec signal, au milieu du changement)
	_overlay_menu = _dropdown_panel()
	root.add_child(_overlay_menu)
	var ov: VBoxContainer = VBoxContainer.new()
	ov.add_theme_constant_override("separation", 4)
	_overlay_menu.add_child(ov)
	ov.add_child(UiStyle.label("CALQUE (UN SEUL À LA FOIS)", "CardTitle"))
	for e: Array in OVERLAYS:
		var id: String = e[0]
		var b: Button = _no_focus(Button.new()) as Button
		b.text = "%s  (%s)" % [e[1], e[2]]
		b.toggle_mode = true
		b.alignment = HORIZONTAL_ALIGNMENT_LEFT
		b.tooltip_text = e[3]
		b.toggled.connect(func(on: bool) -> void:
			if on:
				set_overlay(id)
			elif overlay == id:
				set_overlay(""))
		ov.add_child(b)
		match id:
			"":
				none_button = b
			"eau":
				water_button = b
			"flux":
				flux_button = b
	# Menu ≡
	_menu = _dropdown_panel()
	root.add_child(_menu)
	var m: VBoxContainer = VBoxContainer.new()
	m.add_theme_constant_override("separation", 5)
	m.custom_minimum_size = Vector2(300, 0)
	_menu.add_child(m)
	m.add_child(UiStyle.label("ARBRE ET SITE (RELANCENT LA SIMULATION)", "CardTitle"))
	var g: GridContainer = GridContainer.new()
	g.columns = 2
	g.add_theme_constant_override("h_separation", 8)
	g.add_theme_constant_override("v_separation", 4)
	m.add_child(g)
	g.add_child(UiStyle.label("Espèce", "DimLabel"))
	species_option = _no_focus(OptionButton.new()) as OptionButton
	species_option.tooltip_text = "Espèce (relance la même graine sur le même site)"
	species_option.clip_text = true
	species_option.fit_to_longest_item = false
	species_option.custom_minimum_size = Vector2(210, 0)
	species_option.item_selected.connect(func(i: int) -> void: species_selected.emit(species_option.get_item_id(i)))
	g.add_child(species_option)
	g.add_child(UiStyle.label("Terrain", "DimLabel"))
	soil_option = _no_focus(OptionButton.new()) as OptionButton
	soil_option.tooltip_text = "Type de sol (relance la même graine)"
	soil_option.clip_text = true
	soil_option.fit_to_longest_item = false
	soil_option.custom_minimum_size = Vector2(210, 0)
	soil_option.item_selected.connect(func(i: int) -> void: soil_selected.emit(i))
	g.add_child(soil_option)
	var seed_row: HBoxContainer = HBoxContainer.new()
	seed_row.add_theme_constant_override("separation", 8)
	m.add_child(seed_row)
	seed_button = _no_focus(Button.new()) as Button
	seed_button.text = "Nouvelle graine"
	seed_button.tooltip_text = "Un autre individu (autre graine, même espèce, même site)"
	seed_button.pressed.connect(func() -> void:
		_close_dropdowns()
		new_seed_requested.emit())
	seed_row.add_child(seed_button)
	_seed_label = UiStyle.label("", "DimLabel")
	_seed_label.tooltip_text = "Numéro de la graine : la même graine redonne le même arbre"
	_seed_label.mouse_filter = Control.MOUSE_FILTER_PASS
	seed_row.add_child(_seed_label)
	m.add_child(UiStyle.rule(UiStyle.LINE_DIM))
	m.add_child(UiStyle.label("AFFICHAGE", "CardTitle"))
	guides_button = _no_focus(CheckBox.new()) as CheckBox
	guides_button.text = "Repères d'échelle (G)"
	guides_button.tooltip_text = "Échelles de hauteur et de profondeur"
	guides_button.toggled.connect(func(on: bool) -> void: guides_toggled.emit(on))
	m.add_child(guides_button)
	expert_menu_check = _no_focus(CheckBox.new()) as CheckBox
	expert_menu_check.text = "Mode Expert (I)"
	expert_menu_check.tooltip_text = "Colonne permanente des valeurs (réglage mémorisé, aussi dans Options)"
	expert_menu_check.toggled.connect(func(on: bool) -> void: set_expert(on))
	m.add_child(expert_menu_check)
	var hb: Button = _no_focus(Button.new()) as Button
	hb.text = "Aide des raccourcis (F1)"
	hb.alignment = HORIZONTAL_ALIGNMENT_LEFT
	hb.pressed.connect(func() -> void:
		_close_dropdowns()
		toggle_help())
	m.add_child(hb)
	m.add_child(UiStyle.rule(UiStyle.LINE_DIM))
	m.add_child(UiStyle.label("PARTIE", "CardTitle"))
	for e: Array in [["Sauvegarde rapide (F5)", save_requested], ["Chargement rapide (F9)", load_requested],
			["Menu pause, options… (Échap)", pause_requested]]:
		var b: Button = _no_focus(Button.new()) as Button
		b.text = e[0]
		b.alignment = HORIZONTAL_ALIGNMENT_LEFT
		var sig: Signal = e[1]
		b.pressed.connect(func() -> void:
			_close_dropdowns()
			sig.emit())
		m.add_child(b)


func _show_dropdown(d: PanelContainer, anchor: Button, on: bool) -> void:
	if on:
		for other: Array in [[_menu, menu_button], [_overlay_menu, overlay_button]]:
			if other[0] != d:
				(other[0] as Control).visible = false
				(other[1] as Button).set_pressed_no_signal(false)
	d.visible = on
	if on:
		d.size = d.get_combined_minimum_size()
		var r: Rect2 = anchor.get_global_rect()
		var view: Vector2 = _root.get_viewport_rect().size
		d.position = Vector2(clampf(r.end.x - d.size.x, EDGE, view.x - d.size.x - EDGE), r.end.y + 6.0)


func _close_dropdowns() -> void:
	_show_dropdown(_menu, menu_button, false)
	_show_dropdown(_overlay_menu, overlay_button, false)
	menu_button.set_pressed_no_signal(false)
	overlay_button.set_pressed_no_signal(false)


func is_menu_open() -> bool:
	return _menu.visible


func open_menu(on: bool) -> void:
	menu_button.button_pressed = on


# ----------------------------------------------------------------------------------------- colonne Expert

func _build_expert(root: Control) -> void:
	_expert_col = ScrollContainer.new()
	_expert_col.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_expert_col.mouse_filter = Control.MOUSE_FILTER_PASS
	_expert_col.visible = false
	root.add_child(_expert_col)
	var v: VBoxContainer = VBoxContainer.new()
	v.add_theme_constant_override("separation", 5)
	v.custom_minimum_size = Vector2(EXPERT_W - 4.0, 0)
	_expert_col.add_child(v)
	_card_h = ValueCard.new("Hauteur")
	_card_h.definition = "Hauteur totale de l'arbre, du collet au bourgeon le plus haut. Courbe : depuis le début de la partie."
	_card_d = ValueCard.new("Diamètre à 1,30 m (DBH)")
	_card_d.definition = "Diamètre du tronc à hauteur de poitrine (1,30 m), la mesure de base du forestier. Courbe : depuis le début de la partie."
	_card_r = ValueCard.new("Réserves et état", true)
	_card_r.definition = "Réserves carbonées (amidon) en part du plein, et classe de vigueur. Trait du graphique : 60 % (en dessous, l'arbre est dit « en déclin » ; sous 25 %, « dépérissant »). Seuils d'affichage, pas des catégories publiées."
	_card_r.bullet.bands = PackedFloat32Array([0.25, 0.6])
	_card_r.bullet.threshold = 0.6
	_card_r.spark.y_min = 0.0
	_card_r.spark.y_max = 1.0
	_card_w = ValueCard.new("Eau du sol", true)
	_card_w.definition = "Eau disponible dans la zone racinaire, de 0 (point de flétrissement) à 100 % (capacité au champ). Trait : seuil de sécheresse (25 %)."
	_card_w.bullet.bands = PackedFloat32Array([SoilPanel.DRY, 0.6])
	_card_w.bullet.threshold = SoilPanel.DRY
	_card_w.bullet.bar_color = UiStyle.WATER
	_card_w.spark.color = UiStyle.WATER
	_card_w.spark.y_min = 0.0
	_card_w.spark.y_max = 1.0
	_card_w.spark.threshold = SoilPanel.DRY
	_card_t = ValueCard.new("Température et vent")
	_card_t.definition = "Température de l'air du jour et vent du jour à 10 m en terrain dégagé (moyen et rafale ; climat calé sur Rennes). Courbes : température, puis rafales."
	_card_root = ValueCard.new("Racines")
	_card_root.definition = "Profondeur atteinte par les racines et étalement horizontal (rayon). Courbe : profondeur depuis le début de la partie."
	for c: ValueCard in [_card_h, _card_d, _card_r, _card_w, _card_t, _card_root]:
		v.add_child(c)
	_state = _card_r.add_line("SmallLabel")
	_fallen_lbl = _card_r.add_line("UrgentLabel")
	_fallen_lbl.add_theme_font_size_override("font_size", 13)
	_fallen_lbl.text = "Arbre déraciné (chablis) : le tronc est mort, à terre"
	_fallen_lbl.visible = false
	_wind = _card_t.add_line("SmallLabel")
	_wind_spark = _card_t.add_spark()
	_root_spread = _card_root.add_line("SmallLabel")


func set_expert(on: bool, save: bool = true) -> void:
	expert = on
	_expert_col.visible = on
	if expert_button.button_pressed != on:
		expert_button.set_pressed_no_signal(on)
	if expert_menu_check.button_pressed != on:
		expert_menu_check.set_pressed_no_signal(on)
	_expert_age = 10.0
	if save and not Game.is_automation():
		Game.settings["expert"] = on
		Game.save_settings()
	expert_toggled.emit(on)


func is_expert_visible() -> bool:
	return _expert_col.visible


# ------------------------------------------------------------------------------------- légendes, bandeau

func _build_legends(root: Control) -> void:
	_legends = VBoxContainer.new()
	_legends.add_theme_constant_override("separation", 4)
	_legends.mouse_filter = Control.MOUSE_FILTER_IGNORE
	root.add_child(_legends)
	overlay_legend = OverlayLegend.new()
	_legends.add_child(overlay_legend)
	_legend = OverlayLegend.new()
	var items: Array = []
	for i: int in SoilBlock.KIND_NAMES.size():
		items.append(["swatch", SoilBlock.KIND_COLORS[i], SoilBlock.KIND_NAMES[i]])
	_legend.set_items("Horizons", items)
	_legend.visible = false
	_legends.add_child(_legend)


func _build_toast(root: Control) -> void:
	_toast = PanelContainer.new()
	_toast.theme_type_variation = "UrgentPanel"
	_toast.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_toast.visible = false
	var h: HBoxContainer = HBoxContainer.new()
	h.add_theme_constant_override("separation", 6)
	_toast.add_child(h)
	h.add_child(LegendGlyph.new("tri", UiStyle.URGENT))
	_toast_lbl = UiStyle.label("", "")
	_toast_lbl.add_theme_font_override("font", UiStyle.font(700))
	h.add_child(_toast_lbl)
	root.add_child(_toast)


## Bandeau urgent (tempête, mort du tronc...) : triangle + texte, quelques secondes sous la barre du haut.
func show_urgent(text: String, seconds: float = 5.0) -> void:
	_toast_lbl.text = text
	_toast.visible = true
	_toast_t = seconds
	_toast.size = _toast.get_combined_minimum_size()


## Opacité du sol 0..1 par le curseur du panneau Sol (même chemin qu'un glisser ; touche X).
func set_soil_opacity(v: float) -> void:
	soil_slider.value = clampf(v, 0.0, 1.0)


func set_legend_visible(on: bool) -> void:
	_legend.visible = on


func is_legend_visible() -> bool:
	return _legend.visible


# ----------------------------------------------------------------------------------------------- calques

## Calque actif : "" (aucun), "eau" ou "flux". Un seul à la fois.
func set_overlay(id: String) -> void:
	if id != "" and id != "eau" and id != "flux":
		id = ""
	if id == overlay:
		_sync_overlay_buttons()
		return
	overlay = id
	_sync_overlay_buttons()
	_apply_overlay_legend()
	overlay_changed.emit(id)
	flux_toggled.emit(id == "flux")


func _sync_overlay_buttons() -> void:
	if flux_button == null:
		return
	for pair: Array in [[none_button, ""], [water_button, "eau"], [flux_button, "flux"]]:
		var b: Button = pair[0]
		if b.button_pressed != (overlay == pair[1]):
			b.set_pressed_no_signal(overlay == pair[1])
	var name: String = "aucun"
	for e: Array in OVERLAYS:
		if e[0] == overlay:
			name = e[1]
	overlay_button.text = "Calque : %s ▾" % name


func _apply_overlay_legend() -> void:
	match overlay:
		"eau":
			overlay_legend.set_items("Calque Eau du sol", [
				["hatch", SoilBlock.WATER_DRY, "sec", "Sol sec : moins de 25 % de l'eau disponible (hachures)"],
				["solid", SoilBlock.WATER_MOIST, "humide", "Sol humide (aplat)"],
				["grid", SoilBlock.WATER_WET, "saturé", "Sol proche de la saturation : plus de 75 % (pois)"],
				["dash", TreeFluxView.COLORS["water"], "eau puisée (sève brute)", "Tirets longs : l'eau monte des racines aux feuilles"],
				["puff", TreeFluxView.COLORS["vapour"], "transpiration", "Nuages : vapeur d'eau rejetée par les feuilles"],
			])
			overlay_legend.visible = true
		"flux":
			var items: Array = []
			for r: Array in FluxPanel.ROWS:
				items.append([TreeFluxView.SHAPES[r[0]], TreeFluxView.COLORS[r[0]], r[3], r[2]])
			overlay_legend.set_items("Calque Flux", items)
			overlay_legend.visible = true
		_:
			overlay_legend.visible = false


func is_overlay(id: String) -> bool:
	return overlay == id


# ------------------------------------------------------------------------------------------------ panneaux

func _on_panel_button(id: String, on: bool) -> void:
	if on:
		open_panel(id)
	elif slot.current == id:
		open_panel("")


## Ouvre le panneau `id` dans l'emplacement unique ("" : ferme). Seule porte d'entrée : boutons, touches, accroches
## et tests passent par ici (ou par les boutons, qui y mènent).
func open_panel(id: String) -> void:
	if id != "" and not slot.has_panel(id):
		id = ""
	slot.open(id)


## Ajoute un panneau à l'emplacement (main.gd : flux, mécanique ; prune_tool.gd : taille).
func register_panel(id: String, c: Control, title: String) -> void:
	slot.register(id, c, title)


func current_panel() -> String:
	return slot.current


func _on_slot_changed(id: String) -> void:
	for k: String in _panel_buttons:
		var b: Button = _panel_buttons[k]
		if b.button_pressed != (k == id):
			b.set_pressed_no_signal(k == id)
	if prune_button.button_pressed != (id == "taille"):
		prune_button.set_pressed_no_signal(id == "taille")
		if id != "taille":
			prune_toggled.emit(false)
	if id == "flux" and overlay != "flux":
		set_overlay("flux")
	rings_toggled.emit(id == "cernes")
	mech_toggled.emit(id == "meca")
	_free_key = []


func set_rings_visible(on: bool) -> void:
	if on:
		open_panel("cernes")
	elif slot.current == "cernes":
		open_panel("")


func is_rings_visible() -> bool:
	return slot.is_open("cernes")


func get_ring_view() -> RingView:
	return rings_panel.ring_view


# ---------------------------------------------------------------------------------------------- clavier

func _unhandled_input(event: InputEvent) -> void:
	if not (event is InputEventKey):
		return
	var k: InputEventKey = event
	if not k.pressed or k.echo or k.ctrl_pressed or k.alt_pressed or k.meta_pressed:
		return
	var handled: bool = true
	if k.keycode == KEY_F1 or k.unicode == 63 or k.keycode == KEY_H:  # 63 = « ? » (Maj + , en AZERTY)
		toggle_help()
	elif k.keycode == KEY_ESCAPE and (help.visible or _menu.visible or _overlay_menu.visible):
		help.visible = false
		_close_dropdowns()
	elif k.keycode == KEY_C:
		slot.toggle("cernes")
	elif k.keycode == KEY_L:
		slot.toggle("flux")
	elif k.keycode == KEY_S:
		slot.toggle("sol")
	elif k.keycode == KEY_I:
		set_expert(not expert)
	elif k.physical_keycode == KEY_1 or k.keycode == KEY_KP_1:
		set_overlay("" if overlay == "eau" else "eau")
	elif k.physical_keycode == KEY_2 or k.keycode == KEY_KP_2:
		set_overlay("" if overlay == "flux" else "flux")
	elif k.physical_keycode == KEY_0 or k.keycode == KEY_KP_0:
		set_overlay("")
	else:
		handled = false
	if handled:
		get_viewport().set_input_as_handled()


## Clic hors d'un menu déroulant : il se ferme (sans consommer le clic).
func _input(event: InputEvent) -> void:
	if not (event is InputEventMouseButton) or not (event as InputEventMouseButton).pressed:
		return
	var p: Vector2 = (event as InputEventMouseButton).position
	for pair: Array in [[_menu, menu_button], [_overlay_menu, overlay_button]]:
		var d: Control = pair[0]
		if not d.visible:
			continue
		if species_option.get_popup().visible or soil_option.get_popup().visible:
			continue
		if not d.get_global_rect().has_point(p) and not (pair[1] as Button).get_global_rect().has_point(p):
			_show_dropdown(d, pair[1], false)
			(pair[1] as Button).set_pressed_no_signal(false)


func toggle_help() -> void:
	help.visible = not help.visible
	if help.visible:
		help.size = help.get_combined_minimum_size()
		help.position = (_root.get_viewport_rect().size - help.size) * 0.5


# --------------------------------------------------------------------------------- disposition, zone 3D libre

## Le point d'écran est-il sur l'interface (barres, panneau, colonne, menus) ? Sert à l'outil de taille et au survol.
func covers(p: Vector2) -> bool:
	for c: Control in [_info_panel, _bottom_panel, slot, _expert_col, _menu, _overlay_menu, help, overlay_legend, _legend, _toast]:
		if c != null and c.is_visible_in_tree() and c.get_global_rect().has_point(p):
			return true
	return false


func _hover_blocked() -> bool:
	if slot.current == "taille":
		return true
	var p: Vector2 = _root.get_viewport().get_mouse_position()
	return covers(p) or _root.get_viewport().gui_get_hovered_control() != null


## free_rect à chaque image pour le cadrage : recalculé dès que la taille de la vue, l'échelle, le panneau ouvert,
## le mode Expert ou les légendes changent, sinon au plus toutes les FREE_RECT_REFRESH_MS.
const FREE_RECT_REFRESH_MS: int = 200
var _free_cache: Rect2
var _free_key: Array = []
var _free_time: int = -1000000


func free_rect_cached(view: Vector2) -> Rect2:
	var key: Array = [view, get_window().content_scale_factor, slot.current, slot.visible, _expert_col.visible,
		overlay_legend.visible, _legend.visible, _info_panel.size, _bottom_panel.size]
	var now: int = Time.get_ticks_msec()
	if key != _free_key or now - _free_time >= FREE_RECT_REFRESH_MS:
		_free_key = key
		_free_time = now
		_free_cache = free_rect(view)
	return _free_cache


## Partie de la vue (unités du canevas) que l'interface ne couvre pas : sous la barre du haut, au-dessus des légendes
## et de la barre du bas, à droite de la colonne Expert, à gauche du panneau ouvert. La caméra y cadre l'arbre.
func free_rect(view: Vector2) -> Rect2:
	var top: float = _info_panel.position.y + _info_panel.size.y + GAP
	var bar_y: float = view.y - EDGE - _bottom_panel.get_combined_minimum_size().y
	var left: float = EDGE
	if _expert_col.visible:
		left = EDGE + EXPERT_W + GAP
	var right: float = layout_side_panels(view, top, bar_y - GAP)
	var bottom: float = _layout_legends(left, right, bar_y) - GAP
	return Rect2(left, top, maxf(right - left, 200.0), maxf(bottom - top, 200.0))


## Place l'emplacement du panneau (à droite, entre les barres) et la colonne Expert (à gauche). Renvoie la limite
## droite de la zone 3D. (Nom gardé : l'ancienne colonne de panneaux empilés.)
func layout_side_panels(view: Vector2, top: float, bottom: float) -> float:
	_expert_col.position = Vector2(EDGE, top)
	_expert_col.size = Vector2(EXPERT_W, maxf(bottom - top, 100.0))
	if not slot.visible:
		return view.x - EDGE
	var p: Control = slot.panel(slot.current)
	if p != null and p.has_meta("dock_fit"):
		(p.get_meta("dock_fit") as Callable).call(100000.0)
	var h: float = minf(slot.natural_height(), bottom - top)
	var w: float = maxf(PanelSlot.WIDTH, slot.get_combined_minimum_size().x)
	slot.size = Vector2(w, maxf(h, 120.0))
	slot.position = Vector2(view.x - EDGE - w, top)
	return slot.position.x - GAP


## Légendes au-dessus de la barre du bas, alignées sur la zone 3D. Renvoie le haut du bloc (ou bar_y sans légende).
func _layout_legends(left: float, right: float, bar_y: float) -> float:
	var any: bool = overlay_legend.visible or _legend.visible
	_legends.visible = any
	if not any:
		return bar_y
	# Largeur bornée par la zone 3D : les légendes passent à la ligne plutôt que de glisser sous le panneau
	# (la hauteur du retour à la ligne est connue après le tri du conteneur, à la mise à jour suivante).
	var line: float = 0.0
	for l: OverlayLegend in [overlay_legend, _legend]:
		if l.visible:
			line = maxf(line, l.line_width())
	var w: float = minf(line, maxf(right - left, 200.0))
	_legends.size = Vector2(w, 0.0)
	var sz: Vector2 = Vector2(w, _legends.get_combined_minimum_size().y)
	_legends.size = sz
	_legends.position = Vector2(left, bar_y - GAP - sz.y)
	return _legends.position.y


## Rectangles (canevas) des panneaux latéraux visibles (tests) : l'emplacement unique.
func side_panel_rects() -> Array[Rect2]:
	var out: Array[Rect2] = []
	if slot.visible:
		out.append(Rect2(slot.global_position, slot.size))
	return out


func _process(delta: float) -> void:
	if _toast.visible:
		_toast_t -= delta
		var view: Vector2 = _root.get_viewport_rect().size
		_toast.position = Vector2((view.x - _toast.size.x) * 0.5, _info_panel.position.y + _info_panel.size.y + 14.0)
		_toast.modulate.a = clampf(_toast_t, 0.0, 1.0)
		if _toast_t <= 0.0:
			_toast.visible = false


# ------------------------------------------------------------------------------------- lecture et vitesse

func reset_drought_button() -> void:
	if drought_button.button_pressed:
		drought_button.button_pressed = false


func set_playing(on: bool) -> void:
	_play.button_pressed = on


func is_playing() -> bool:
	return _play.button_pressed


func set_seed_text(seed_value: int) -> void:
	_seed_label.text = "graine n° %d" % seed_value


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


# ------------------------------------------------------------------------------------------- mise à jour

## Rafraîchit la barre du haut, la jauge, l'historique, les cartes (4 fois par seconde au plus) et les cernes
## (seulement quand le nombre d'années change).
func update_from(sim: TreeSim, delta: float = 0.0) -> void:
	var age: float = sim.get_age_years()
	var year: int = sim.get_year()
	var doy: int = sim.get_day_of_year()
	var alive: bool = sim.get_trunk_alive()
	var ph: int = clampi(sim.get_phase(), 0, 5)
	var ever: bool = sim.get_evergreen()
	var season: String = (SEASONS_SHORT_EVERGREEN if ever else SEASONS_SHORT)[ph] if alive else "tronc mort"
	_ident.text = "· %s ans · %s, an %d · %s" % [UiStyle.num(age, 1), date_text(doy), year, season]
	_ident.tooltip_text = "Âge %s ans · %s de l'année %d · %s" % [UiStyle.num(age, 1), date_text(doy), year,
		(PHASES_EVERGREEN if ever else PHASES)[ph] if alive else "tronc mort"]
	var water: float = sim.get_soil_water()
	_water_gauge.set_value(water)
	_water_pct.text = "%d %%" % roundi(water * 100.0)
	# État : une fois par année simulée, ou quand le tronc meurt ou tombe (`get_vitality` construit un dictionnaire).
	var vkey: int = year * 4 + (2 if alive else 0) + (1 if sim.get_height_m() > 0.5 else 0)
	if vkey != _state_key:
		_state_key = vkey
		var vd: Dictionary = sim.get_vitality()
		var st: String = vitality_text(vd)
		_state.text = st if st != "" else "État : —"
		_update_alert(vd, sim)
		if bool(vd.get("modelled", false)) and int(vd.get("death_age", -1)) < 0:
			var r: float = float(vd.get("reserve", 1.0))
			history.sample("reserve", sim.get_days_elapsed(), r, true)
			_card_r.bullet.set_value(r)
			_card_r.set_value("%d %%" % roundi(100.0 * r))
		elif bool(vd.get("modelled", false)):
			_card_r.bullet.set_value(0.0)
			_card_r.set_value("mort")
	var sb: int = sim.get_last_stem_break_year()
	if not _fallen_lbl.visible or _fallen_lbl.text.begins_with("Tronc"):
		_fallen_lbl.visible = sb >= 0
		if sb >= 0:
			_fallen_lbl.text = "Tronc cassé l'an %d : chandelle, rejets épicormiques" % sb
	var w: Vector3 = sim.get_wind_today()
	_wind.text = "Vent : %s m/s, rafale %s m/s (%d km/h)" % [UiStyle.num(w.x, 1), ("%.0f" % w.y), roundi(w.y * 3.6)]
	_record_history(sim, w)
	_expert_age += delta
	if _expert_col.visible and (_expert_age >= 0.25 or delta == 0.0):
		_expert_age = 0.0
		_refresh_cards(sim, w)
	soil_panel.refresh(sim, soil_option.get_item_text(soil_option.selected) if soil_option.selected >= 0 else "", delta if delta > 0.0 else 1.0)
	if year != _last_ring_year:
		_last_ring_year = year
		rings_panel.ring_view.contrast = sim.get_ring_contrast()
		rings_panel.set_data(sim.get_ring_widths(), sim.get_ring_stress(), sim.get_ring_earlywood_fraction(), sim.get_ring_forced_drought())


## Échantillonne l'historique de la partie (pas en jours simulés, voir GameHistory) : au plus une fois par image.
func _record_history(sim: TreeSim, w: Vector3) -> void:
	var day: int = sim.get_days_elapsed()
	history.sample("height", day, sim.get_height_m())
	history.sample("dbh", day, sim.get_trunk_diameter_cm())
	history.sample("water", day, sim.get_soil_water())
	history.sample("temp", day, sim.get_temperature_c())
	history.sample("gust", day, w.y)
	history.sample("root_depth", day, sim.get_root_depth_m())


## Échantillon pendant une avance directe (sans images : départ à 10, 40 ou 100 ans, captures) : les mêmes séries,
## et les réserves une fois par année simulée.
func sample_history(sim: TreeSim) -> void:
	_record_history(sim, sim.get_wind_today())
	var y: int = sim.get_year()
	if y != _hist_year:
		_hist_year = y
		var vd: Dictionary = sim.get_vitality()
		if bool(vd.get("modelled", false)) and int(vd.get("death_age", -1)) < 0:
			history.sample("reserve", sim.get_days_elapsed(), float(vd.get("reserve", 1.0)), true)


func _refresh_cards(sim: TreeSim, w: Vector3) -> void:
	_card_h.set_value("%s m" % UiStyle.num(sim.get_height_m(), 1))
	_card_h.set_series(history.values("height"))
	_card_h.trend = GameHistory.trend_text(history.change("height"), "m")
	_card_d.set_value("%s cm" % UiStyle.num(sim.get_trunk_diameter_cm(), 1))
	_card_d.set_series(history.values("dbh"))
	_card_d.trend = GameHistory.trend_text(history.change("dbh"), "cm")
	_card_r.set_series(history.values("reserve"))
	_card_r.trend = GameHistory.trend_text(100.0 * history.change("reserve", 730), "points", 0, "en deux ans")
	var water: float = sim.get_soil_water()
	_card_w.set_value("%d %%" % roundi(water * 100.0))
	_card_w.bullet.set_value(water)
	_card_w.set_series(history.values("water"))
	_card_w.trend = GameHistory.trend_text(100.0 * history.change("water", 30), "points", 0, "en un mois")
	_card_t.set_value("%s °C" % UiStyle.num(sim.get_temperature_c(), 1))
	_card_t.set_series(history.values("temp"))
	_wind_spark.set_data(history.values("gust"))
	_card_t.trend = "Vent du jour : moyen %s m/s, rafale %s m/s" % [UiStyle.num(w.x, 1), UiStyle.num(w.y, 0)]
	_card_root.set_value("%s m" % UiStyle.num(sim.get_root_depth_m(), 1))
	_card_root.set_series(history.values("root_depth"))
	_card_root.trend = GameHistory.trend_text(history.change("root_depth"), "m")
	_root_spread.text = "Étalement : %s m (rayon)" % UiStyle.num(sim.get_root_spread_m(), 1)


## Signal urgent dans la barre du haut : déclin, dépérissement, mort du tronc, chablis, tronc cassé.
func _update_alert(vd: Dictionary, sim: TreeSim) -> void:
	var txt: String = ""
	var level: int = 0
	if sim.get_uprooted().x >= 0.0:
		txt = "Déraciné"
		level = 2
	elif bool(vd.get("modelled", false)) and int(vd.get("death_age", -1)) >= 0:
		txt = "Tronc mort"
		level = 2
	elif bool(vd.get("modelled", false)):
		var r: float = float(vd.get("reserve", 1.0))
		var v: float = float(vd.get("vigour", 1.0))
		if v < 0.35 or r < 0.25:
			txt = "Dépérissant"
			level = 2
		elif v < 0.7 or r < 0.6:
			txt = "En déclin"
			level = 1
	if txt == "" and sim.get_last_stem_break_year() >= 0:
		txt = "Tronc cassé"
		level = 1
	_alert_text = txt
	_state_level = level
	_alert.visible = txt != ""
	_alert_lbl.text = txt
	_alert.tooltip_text = vitality_text(vd)


func alert_text() -> String:
	return _alert_text if _alert.visible else ""


## Ligne « État » depuis `TreeSim.get_vitality()` : classe de vigueur et réserves tant que le tronc vit ; une fois
## mort, âge et cause, chandelle et chute attendue, rejets de souche. "" quand l'espèce n'a pas de bilan carboné
## (génome sans `maintenance_per_m`). Les classes de vigueur sont des conventions d'affichage (seuils de l'indice
## de vigueur et des réserves), pas des catégories publiées. Un hôte de scolytes (épicéa) en année de crise
## (réserves épuisées) montre aussi son risque annuel d'attaque (`bark_beetle_probability`, `dendro_core::vitality`) ;
## une mort par scolytes se lit « Cause : scolytes … ».
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
		var txt_alive: String = "État : %s — réserves %d %%" % [cls, roundi(100.0 * r)]
		var bb: float = float(d.get("bark_beetle_probability", 0.0))
		if bb > 0.0:
			txt_alive += "\nRisque de scolytes : %s %% par an (arbre affaibli)" % ("%.1f" % (100.0 * bb)).replace(".", ",")
		return txt_alive
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


## L'arbre est à terre (déraciné) : une ligne explicite et le signal urgent.
func set_fallen(on: bool) -> void:
	_fallen_lbl.text = "Arbre déraciné (chablis) : le tronc est mort, à terre ; la souche peut rejeter selon l'essence"
	_fallen_lbl.visible = on
	_state_key = -1


## L'arbre a changé hors du temps normal (graine, coupe, annulation, chargement) : cernes et état sont relus.
func force_ring_refresh() -> void:
	_last_ring_year = -1
	_state_key = -1
	rings_panel.ring_view.reset_view()


## Nouvelle partie (graine, espèce, terrain, chargement) : l'historique des sparklines repart de zéro.
func reset_history() -> void:
	history.reset()
	_hist_year = -1
	_expert_age = 10.0


# ------------------------------------------------------------------------------------------ espèce, terrain

## Remplit le choix du terrain ; au-delà des étiquettes françaises connues, le nom du cœur.
func set_soil_names(names: PackedStringArray) -> void:
	soil_option.clear()
	for i: int in names.size():
		soil_option.add_item(SOIL_LABELS[i] if i < SOIL_LABELS.size() else names[i])


func select_soil(index: int) -> void:
	if index >= 0 and index < soil_option.item_count:
		soil_option.select(index)


## Remplit le choix de l'espèce avec les noms français, groupés par catégorie (« Feuillus », « Fruitiers »,
## « Résineux » ; le cœur liste déjà les espèces groupées). L'id de chaque entrée est l'indice de l'espèce.
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


## Sélectionne l'entrée et affiche l'espèce dans la barre du haut (nom commun en gras, nom latin en italique).
func select_species(index: int) -> void:
	if index < 0 or index >= _species_common.size():
		return
	species_option.select(species_option.get_item_index(index))
	_name_lbl.text = _species_common[index]
	_sci_lbl.text = _species_sci[index] if index < _species_sci.size() else ""


## Couleur d'écorce de l'espèce simulée, pour l'anneau d'écorce de la coupe.
func set_bark_color(c: Color) -> void:
	rings_panel.ring_view.set_bark_color(c)


## Thème unique : tous les Labels et Buttons sous le Hud (test de la taille minimale de police).
func all_text_controls() -> Array[Control]:
	var out: Array[Control] = []
	for n: Node in _root.find_children("*", "Control", true, false):
		if n is Label or n is Button:
			out.append(n as Control)
	return out
