class_name HudSelftest
extends RefCounted
## Selftest de l'interface refondue (appelé par UiSelftest) : un seul panneau ouvert à la fois, mode Expert,
## calques et leurs légendes forme + couleur, contrastes WCAG mesurés, taille de police >= 12 px sur tous les
## Labels et Buttons, barres qui tiennent à 1280 px, étiquettes de survol, aide, menu ≡, historique, signal urgent.

static func _key(code: Key, unicode: int = 0) -> void:
	for pressed: bool in [true, false]:
		var ev: InputEventKey = InputEventKey.new()
		ev.keycode = code
		ev.physical_keycode = code
		ev.unicode = unicode
		ev.pressed = pressed
		Input.parse_input_event(ev)


## Panneaux réellement visibles (drapeau `visible` des contenus de l'emplacement), pas l'état des boutons.
static func _open_panels(hud: Hud) -> Array[String]:
	var out: Array[String] = []
	for id: String in ["cernes", "flux", "sol", "meca", "taille"]:
		var c: Control = hud.slot.panel(id)
		if c != null and c.is_visible_in_tree():
			out.append(id)
	return out


static func run(host: Node, check: Callable) -> void:
	var tree: SceneTree = host.get_tree()
	var hud: Hud = host.get("hud") as Hud
	var soil: SoilBlock = host.get("soil") as SoilBlock
	var flux: RootFlux = host.get("flux") as RootFlux
	var pt: PruneTool = host.get("prune_tool") as PruneTool
	var view: Vector2 = host.get_viewport().get_visible_rect().size

	# --- Contrastes WCAG 2.2 (texte >= 4,5:1, graphique >= 3:1, pire cas : panneau devant un ciel clair).
	var bad: PackedStringArray = PackedStringArray()
	for p: Array in UiStyle.contrast_pairs():
		var r: float = UiStyle.contrast(p[1] as Color, p[2] as Color)
		print("[contrast] %5.2f:1 (>= %.1f) %s" % [r, float(p[3]), String(p[0])])
		if r < float(p[3]):
			bad.append("%s %.2f" % [p[0], r])
	var panel: Color = UiStyle.over(UiStyle.PANEL_A, UiStyle.SKY_WORST)
	for sf: float in [0.5, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0]:
		var r2: float = UiStyle.contrast(MechPanel.safety_text_color(sf), panel)
		if r2 < 4.5:
			bad.append("sécurité %.2f en texte %.2f" % [sf, r2])
	check.call("contrastes WCAG : toutes les paires mesurées passent", bad.is_empty(), ", ".join(bad))

	# --- Barres fines : tiennent à 1280 px.
	var top_w: float = hud._info_panel.get_combined_minimum_size().x
	var bot_w: float = hud._bottom_panel.get_combined_minimum_size().x
	check.call("barre du haut tient à 1280 px", top_w <= 1280.0 - 2.0 * Hud.EDGE, "%.0f px" % top_w)
	check.call("barre du bas tient à 1280 px", bot_w <= 1280.0 - 2.0 * Hud.EDGE, "%.0f px" % bot_w)
	check.call("barres fines (au plus 48 px de haut)", hud._info_panel.size.y <= 48.0 and hud._bottom_panel.size.y <= 48.0,
		"%.0f / %.0f" % [hud._info_panel.size.y, hud._bottom_panel.size.y])

	# --- Un seul panneau à la fois, par tous les chemins (boutons, touches, outil de taille, accroche).
	hud.open_panel("")
	await tree.process_frame
	var seq_ok: bool = true
	var seq: String = ""
	for id: String in ["cernes", "flux", "sol", "meca"]:
		(hud._panel_buttons[id] as Button).button_pressed = true
		await tree.process_frame
		var open: Array[String] = _open_panels(hud)
		seq += "%s->%s " % [id, str(open)]
		seq_ok = seq_ok and open == [id] and hud.current_panel() == id
		for k: String in hud._panel_buttons:
			seq_ok = seq_ok and (hud._panel_buttons[k] as Button).button_pressed == (k == id)
	check.call("un seul panneau ouvert : boutons Cernes, Flux, Sol, Mécanique", seq_ok, seq)
	_key(KEY_C)
	await tree.process_frame
	var k1: Array[String] = _open_panels(hud)
	_key(KEY_S)
	await tree.process_frame
	var k2: Array[String] = _open_panels(hud)
	_key(KEY_L)
	await tree.process_frame
	var k3: Array[String] = _open_panels(hud)
	_key(KEY_L)
	await tree.process_frame
	var k4: Array[String] = _open_panels(hud)
	check.call("un seul panneau ouvert : touches C, S, L (L de nouveau ferme)", k1 == ["cernes"] and k2 == ["sol"] and k3 == ["flux"] and k4.is_empty(),
		"%s %s %s %s" % [k1, k2, k3, k4])
	if pt != null:
		hud.open_panel("meca")
		await tree.process_frame
		_key(KEY_T)
		await tree.process_frame
		var t1: Array[String] = _open_panels(hud)
		var t_active: bool = pt.active and hud.prune_button.button_pressed and not hud.mech_button.button_pressed
		_key(KEY_C)
		await tree.process_frame
		var t2: Array[String] = _open_panels(hud)
		check.call("un seul panneau : Taille (T) remplace Mécanique, Cernes (C) ferme la Taille et l'outil",
			t1 == ["taille"] and t_active and t2 == ["cernes"] and not pt.active and not hud.prune_button.button_pressed,
			"%s %s active %s" % [t1, t2, pt.active])
	hud.open_panel("")
	await tree.process_frame
	check.call("panneau fermé : zone 3D jusqu'au bord droit", hud.free_rect(view).end.x >= view.x - Hud.EDGE - 0.5 and hud.side_panel_rects().is_empty())
	hud.open_panel("sol")
	await tree.process_frame
	var fr: Rect2 = hud.free_rect(view)
	var rects: Array[Rect2] = hud.side_panel_rects()
	check.call("panneau ouvert : à droite, entre les barres, hors de la zone 3D", rects.size() == 1 and fr.end.x <= rects[0].position.x
		and rects[0].position.y >= hud._info_panel.get_global_rect().end.y and rects[0].end.y <= hud._bottom_panel.get_global_rect().position.y + 0.5
		and rects[0].size.x >= PanelSlot.WIDTH - 0.5 and rects[0].size.x <= PanelSlot.WIDTH + 24.0, "%s free %s" % [str(rects), str(fr)])
	hud.open_panel("")

	# --- Mode Expert : bouton, touche I, réglage des options ; la colonne pousse la zone 3D.
	var expert0: bool = hud.expert
	hud.set_expert(false)
	await tree.process_frame
	var left_off: float = hud.free_rect(view).position.x
	hud.expert_button.button_pressed = true
	await tree.process_frame
	var left_on: float = hud.free_rect(view).position.x
	check.call("Expert : le bouton montre la colonne de cartes, la zone 3D commence après", hud.is_expert_visible() and hud.expert_menu_check.button_pressed
		and left_on >= left_off + Hud.EXPERT_W, "%.0f -> %.0f" % [left_off, left_on])
	var cards: int = 0
	var sparks: int = 0
	for n: Node in hud._expert_col.find_children("*", "ValueCard", true, false):
		cards += 1
		sparks += 1 if (n as ValueCard).spark != null else 0
	check.call("Expert : six cartes avec sparkline (hauteur, DBH, réserves, eau, température et vent, racines)", cards == 6 and sparks == 6, "%d / %d" % [cards, sparks])
	var ccol: Rect2 = hud._expert_col.get_global_rect()
	check.call("Expert : colonne entre les barres", ccol.position.y >= hud._info_panel.get_global_rect().end.y
		and ccol.end.y <= hud._bottom_panel.get_global_rect().position.y + 0.5, str(ccol))
	_key(KEY_I)
	await tree.process_frame
	check.call("Expert : la touche I le referme", not hud.is_expert_visible() and not hud.expert_button.button_pressed)
	var saved: Dictionary = Game.settings.duplicate()
	var tmp: String = "user://settings_expert_selftest.cfg"
	var op: OptionsPanel = OptionsPanel.new()
	op.apply_live = false
	op.save_path = tmp
	host.add_child(op)
	await tree.process_frame
	op.apply_choice("expert", 1)
	await tree.process_frame
	var cf: ConfigFile = ConfigFile.new()
	cf.load(tmp)
	check.call("Expert : mémorisé dans les options (fichier) et suivi en jeu", hud.is_expert_visible() and bool(cf.get_value("display", "expert", false)))
	op.apply_choice("expert", 0)
	op.queue_free()
	DirAccess.remove_absolute(ProjectSettings.globalize_path(tmp))
	Game.settings = saved
	hud.set_expert(expert0, false)

	# --- Historique de la partie (sparklines).
	host.call("_apply_seed", 21)
	host.call("_advance_days_direct", 6.0 * 365.0)
	var hh: PackedFloat32Array = hud.history.values("height")
	check.call("historique : hauteur échantillonnée en temps simulé, croissante", hh.size() > 20 and hh[hh.size() - 1] > hh[0],
		"%d points" % hh.size())
	check.call("historique : réserves une fois par an", hud.history.size("reserve") >= 5, "%d" % hud.history.size("reserve"))
	host.call("_advance_days_direct", 300.0 * 365.0)
	check.call("historique : borné (décimation)", hud.history.size("temp") <= GameHistory.CAPACITY and hud.history.size("temp") > GameHistory.CAPACITY / 3,
		"%d points" % hud.history.size("temp"))
	host.call("_apply_seed", 21)
	check.call("historique : remis à zéro par une nouvelle graine", hud.history.size("height") == 0)

	# --- Calques : un seul à la fois, recolorent l'existant, légende forme + couleur.
	host.call("_advance_days_direct", 20.0 * 365.0 + 190.0)
	hud.set_overlay("eau")
	await tree.process_frame
	var wv: bool = true
	for m: ShaderMaterial in [soil._soil_mat, soil._soil_dither_mat, soil._soil_a2c_mat]:
		wv = wv and is_equal_approx(m.get_shader_parameter("water_view") as float, 1.0)
	check.call("calque Eau : sol peint (opaque, tramé et A2C), eau puisée seule", soil.water_view and wv and flux.enabled
		and flux.view.only == PackedStringArray(["water", "vapour"]) and not flux.view.is_shown("sugar") and flux.view.is_shown("water"))
	check.call("calque Eau : bouton et légende (5 entrées)", hud.water_button.button_pressed and not hud.flux_button.button_pressed
		and hud.overlay_legend.visible and hud.overlay_legend.item_count == 5 and hud.overlay_legend.legend_title.contains("Eau"))
	_key(KEY_2)
	await tree.process_frame
	check.call("calque Flux (touche 2) remplace Eau", hud.overlay == "flux" and not soil.water_view and flux.enabled and flux.view.only.is_empty()
		and hud.flux_button.button_pressed and not hud.water_button.button_pressed and hud.overlay_legend.item_count == 7)
	_key(KEY_0)
	await tree.process_frame
	check.call("aucun calque (touche 0) : ni flux ni sol peint, pas de légende", hud.overlay == "" and not flux.enabled and not soil.water_view
		and not hud.overlay_legend.visible)
	hud.drought_button.button_pressed = true
	await tree.process_frame
	check.call("Sécheresse ouvre le calque Eau", hud.overlay == "eau" and soil.water_view)
	hud.drought_button.button_pressed = false
	hud.open_panel("flux")
	hud.set_overlay("")
	hud.open_panel("")
	hud.open_panel("flux")
	await tree.process_frame
	check.call("le panneau Flux active le calque Flux", hud.overlay == "flux" and flux.enabled)
	hud.open_panel("")
	var shapes: Dictionary = {}
	for k: String in TreeFluxView.KINDS:
		shapes[TreeFluxView.SHAPES.get(k, "")] = true
	check.call("légende Flux : une forme distincte par sorte de flux", shapes.size() == TreeFluxView.KINDS.size() and not shapes.has(""), str(shapes.keys()))
	hud.set_overlay("flux")

	# --- Tempête : ouvre Mécanique (rafale faible : l'arbre sert encore après).
	var mpn: MechPanel = host.get("mech_panel") as MechPanel
	mpn.storm_slider.value = mpn.storm_slider.min_value
	hud.storm_button.pressed.emit()
	await tree.process_frame
	check.call("Tempête (barre du bas) ouvre le panneau Mécanique", hud.current_panel() == "meca" and _open_panels(hud) == ["meca"])
	hud.open_panel("")

	# --- Aide : F1, « ? » (Maj + , en AZERTY), Échap.
	_key(KEY_F1)
	await tree.process_frame
	var h1: bool = hud.help.visible
	_key(KEY_ESCAPE)
	await tree.process_frame
	var h2: bool = hud.help.visible
	_key(KEY_COMMA, 63)
	await tree.process_frame
	var h3: bool = hud.help.visible
	hud.toggle_help()
	check.call("aide : F1 ouvre, Échap ferme, « ? » ouvre", h1 and not h2 and h3 and host.get_node_or_null("PauseMenu") == null)
	var tips: int = 0
	var with_key: int = 0
	for b: Button in [hud.rings_button, hud.flux_panel_button, hud.soil_panel_button, hud.mech_button, hud.prune_button,
			hud.play_button, hud.expert_button, hud.help_button, hud.overlay_button]:
		tips += 1 if b.tooltip_text != "" else 0
		with_key += 1 if b.tooltip_text.contains("(") else 0
	check.call("info-bulles des boutons avec leur touche", tips == 9 and with_key == 9, "%d / %d" % [tips, with_key])

	# --- Menu ≡ : espèce, terrain, graine accessibles en jeu.
	hud.open_menu(true)
	await tree.process_frame
	check.call("menu ≡ : ouvert, avec espèce, terrain, nouvelle graine", hud.is_menu_open() and hud.species_option.is_visible_in_tree()
		and hud.soil_option.is_visible_in_tree() and hud.seed_button.is_visible_in_tree() and hud.species_option.item_count > 4)
	var mr: Rect2 = hud._menu.get_global_rect()
	check.call("menu ≡ : dans la vue", mr.position.x >= 0.0 and mr.end.x <= view.x and mr.end.y <= view.y, str(mr))
	hud.open_menu(false)
	await tree.process_frame
	check.call("menu ≡ : se referme", not hud.is_menu_open())

	# --- Étiquettes de survol (tronc, racines, ciel), sur un arbre neuf (la tempête a pu le blesser).
	host.call("_apply_seed", 21)
	host.call("_advance_days_direct", 20.0 * 365.0 + 190.0)
	host.call("_rebuild_meshes", true)
	host.call("_settle_view")
	var cam: Camera3D = (host.get("cam") as OrbitCamera).get_camera()
	var sim: TreeSim = host.get("sim") as TreeSim
	var p_trunk: Vector2 = cam.unproject_position(Vector3(0.0, 1.0, 0.0))
	var p_roots: Vector2 = cam.unproject_position(Vector3(0.0, -0.4 * sim.get_root_depth_m(), 0.0))
	var p_sky: Vector2 = cam.unproject_position(Vector3(0.0, sim.get_height_m() * 1.6, 0.0))
	var a: String = hud.hover.probe(p_trunk)
	var b2: String = hud.hover.probe(p_roots)
	var c2: String = hud.hover.probe(p_sky)
	check.call("survol : tronc, racines, rien dans le ciel", a == "tronc" and b2 == "racines" and c2 == "", "%s %s %s" % [a, b2, c2])
	hud.hover.force_point = p_trunk
	await tree.process_frame
	check.call("survol : étiquette du tronc (DBH et hauteur)", hud.hover.visible and hud.hover._label.text.contains("DBH"), hud.hover._label.text)
	hud.hover.force_point = Vector2(-1.0, -1.0)

	# --- Signal urgent : couleur dédiée ET triangle, hors mode Expert.
	hud._update_alert({"modelled": true, "death_age": -1, "reserve": 0.4, "vigour": 0.9}, sim)
	check.call("urgent : déclin signalé dans la barre du haut (triangle + texte)", hud.alert_text() == "En déclin" and hud._alert.get_child(0) is LegendGlyph
		and (hud._alert.get_child(0) as LegendGlyph).shape == "tri")
	hud._update_alert({"modelled": true, "death_age": -1, "reserve": 1.0, "vigour": 1.0}, sim)
	check.call("urgent : rien quand l'arbre est vigoureux", hud.alert_text() == "" or (hud.alert_text() == "Tronc cassé" and sim.get_last_stem_break_year() >= 0),
		"%s, cassure %d, déraciné %s" % [hud.alert_text(), sim.get_last_stem_break_year(), str(sim.get_uprooted())])
	hud.show_urgent("Essai")
	check.call("urgent : bandeau à triangle", hud._toast.visible and hud._toast.theme_type_variation == "UrgentPanel")

	# --- Taille de police >= 12 px sur tous les Labels et Buttons (HUD, panneaux, outil de taille, écrans).
	hud.toggle_help()
	hud.open_menu(true)
	await tree.process_frame
	var small: PackedStringArray = PackedStringArray()
	_font_floor(host, small)
	var title: TitleScreen = TitleScreen.new()
	title.embedded = true
	host.add_child(title)
	await tree.process_frame
	for page: String in ["menu", "setup", "options", "load"]:
		title.call("show_" + page)
		await tree.process_frame
		_font_floor(title, small)
	title.queue_free()
	host.call("_open_pause_menu", false)
	await tree.process_frame
	var pm: Node = host.get_node_or_null("PauseMenu")
	if pm != null:
		_font_floor(pm, small)
		pm.call("_on_entry", "save")
		await tree.process_frame
		_font_floor(pm, small)
		pm.call("close")
	hud.toggle_help()
	hud.open_menu(false)
	check.call("police >= 12 px sur tous les Labels et Buttons", small.is_empty(), ", ".join(small.slice(0, 12)))


static func _font_floor(root: Node, out: PackedStringArray) -> void:
	for n: Node in root.find_children("*", "Control", true, false):
		var sz: int = 99
		if n is Label or n is Button:
			sz = (n as Control).get_theme_font_size("font_size")
		elif n is RichTextLabel:
			sz = (n as Control).get_theme_font_size("normal_font_size")
		if sz < UiStyle.FONT_MIN:
			out.append("%s %d px (%s)" % [n.name, sz, (n.get("text") as String).left(24) if n.get("text") != null else ""])
