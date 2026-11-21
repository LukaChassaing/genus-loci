class_name BulletGraph
extends Control
## Bullet graph (Few) compact : bandes qualitatives en fond (du plus sombre = mauvais au plus clair = bon),
## barre de la valeur, trait vertical du seuil. Les bandes et le seuil sont des formes (position, trait),
## pas seulement des couleurs. Indépendant de la simulation : réutilisable (Seed Master).

var value: float = 0.0
var min_value: float = 0.0
var max_value: float = 1.0
## Limites hautes des bandes qualitatives (unités de la valeur), en ordre croissant ; la dernière va jusqu'à max.
var bands: PackedFloat32Array = PackedFloat32Array()
## Seuil (NAN = aucun) : trait vertical plein, dépasse la barre.
var threshold: float = NAN
var bar_color: Color = UiStyle.TEXT
## Valeur sous le seuil : barre en couleur urgente (la forme reste la position par rapport au trait).
var low_is_bad: bool = true


func _init() -> void:
	custom_minimum_size = Vector2(80, 12)
	mouse_filter = Control.MOUSE_FILTER_IGNORE


func set_value(v: float) -> void:
	if not is_equal_approx(v, value):
		value = v
		queue_redraw()


func _x(v: float) -> float:
	return size.x * clampf((v - min_value) / maxf(max_value - min_value, 1e-6), 0.0, 1.0)


func _draw() -> void:
	var h: float = size.y
	# Bandes : du plus sombre (mauvais) au plus clair (bon) si low_is_bad.
	var prev: float = min_value
	var nb: int = bands.size() + 1
	for i: int in nb:
		var hi: float = bands[i] if i < bands.size() else max_value
		var t: float = float(i) / float(maxi(nb - 1, 1))
		if not low_is_bad:
			t = 1.0 - t
		var c: Color = Color(0.06, 0.09, 0.04, 0.9).lerp(Color(0.30, 0.36, 0.24, 0.9), t)
		draw_rect(Rect2(_x(prev), 0.0, _x(hi) - _x(prev), h), c)
		prev = hi
	draw_rect(Rect2(0.0, 0.0, size.x, h), UiStyle.LINE_DIM, false, 1.0)
	# Barre de la valeur (tiers central).
	var bad: bool = not is_nan(threshold) and ((value < threshold) if low_is_bad else (value > threshold))
	var col: Color = UiStyle.WARN if bad else bar_color
	draw_rect(Rect2(0.0, h * 0.30, _x(value), h * 0.40), col)
	if not is_nan(threshold):
		var xt: float = _x(threshold)
		draw_line(Vector2(xt, 0.0), Vector2(xt, h), UiStyle.TEXT, 2.0)
