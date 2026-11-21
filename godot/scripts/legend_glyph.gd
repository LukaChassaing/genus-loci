class_name LegendGlyph
extends Control
## Pastille de légende qui combine forme ET couleur (WCAG 2.2, SC 1.4.1 : jamais la couleur seule).
## Le vocabulaire des formes est le même que dans la scène (flux_view.gd, soil_block.gd) :
##   dash   tirets longs        (sève brute, eau)
##   dot    points              (sels minéraux)
##   rail   double trait        (sève élaborée)
##   short  tirets courts fins  (mycorhizes)
##   disc   disque plein        (O₂)
##   ring   anneau creux        (CO₂)
##   puff   nuage diffus        (vapeur d'eau)
##   hatch  carré hachuré       (sol sec)
##   solid  carré plein         (sol humide)
##   grid   carré à pois        (sol saturé)
##   swatch carré liseré        (horizon, couleur seule + nom écrit à côté)
##   tri    triangle d'alerte   (urgent)

var shape: String = "swatch"
var color: Color = Color.WHITE


func _init(glyph_shape: String = "swatch", glyph_color: Color = Color.WHITE) -> void:
	shape = glyph_shape
	color = glyph_color
	custom_minimum_size = Vector2(26, 14)
	size_flags_vertical = Control.SIZE_SHRINK_CENTER
	mouse_filter = Control.MOUSE_FILTER_IGNORE


func _draw() -> void:
	draw_glyph(self, shape, color, Rect2(Vector2.ZERO, size))


## Dessine la forme dans `r` sur le CanvasItem `ci` (aussi utilisé par d'autres contrôles).
static func draw_glyph(ci: CanvasItem, shp: String, col: Color, r: Rect2) -> void:
	var cy: float = r.position.y + r.size.y * 0.5
	var x0: float = r.position.x + 1.0
	var x1: float = r.end.x - 1.0
	var halo: Color = Color(0.0, 0.0, 0.0, 0.55)
	match shp:
		"dash":
			for seg: Vector2 in [Vector2(x0, x0 + 9.0), Vector2(x0 + 14.0, x1)]:
				ci.draw_line(Vector2(seg.x, cy), Vector2(seg.y, cy), halo, 5.0)
				ci.draw_line(Vector2(seg.x, cy), Vector2(seg.y, cy), col, 3.0)
		"dot":
			for k: int in 3:
				var p: Vector2 = Vector2(x0 + 3.0 + 9.0 * float(k), cy)
				ci.draw_circle(p, 3.2, halo)
				ci.draw_circle(p, 2.2, col)
		"rail":
			for dy: float in [-2.5, 2.5]:
				for seg: Vector2 in [Vector2(x0, x0 + 10.0), Vector2(x0 + 14.0, x1)]:
					ci.draw_line(Vector2(seg.x, cy + dy), Vector2(seg.y, cy + dy), col, 1.6)
		"short":
			var x: float = x0
			while x < x1 - 2.0:
				ci.draw_line(Vector2(x, cy), Vector2(minf(x + 3.0, x1), cy), col, 1.6)
				x += 6.0
		"disc":
			var c: Vector2 = Vector2((x0 + x1) * 0.5, cy)
			ci.draw_circle(c, 5.5, halo)
			ci.draw_circle(c, 4.5, col)
		"ring":
			var c2: Vector2 = Vector2((x0 + x1) * 0.5, cy)
			ci.draw_arc(c2, 4.6, 0.0, TAU, 20, halo, 3.4, true)
			ci.draw_arc(c2, 4.6, 0.0, TAU, 20, col, 1.8, true)
		"puff":
			var c3: Vector2 = Vector2((x0 + x1) * 0.5, cy)
			for k: int in 3:
				ci.draw_circle(c3 + Vector2(-4.0 + 4.0 * float(k), -1.0 + float(k % 2) * 2.0), 4.0, Color(col.r, col.g, col.b, 0.45))
			ci.draw_arc(c3, 6.5, 0.0, TAU, 20, Color(col.r, col.g, col.b, 0.8), 1.0, true)
		"hatch", "solid", "grid", "swatch":
			var sq: Rect2 = Rect2(Vector2((x0 + x1) * 0.5 - 7.0, cy - 6.0), Vector2(14.0, 12.0))
			if shp == "hatch":
				ci.draw_rect(sq, Color(col.r, col.g, col.b, 0.35))
				var k2: float = -12.0
				while k2 < 14.0:
					var a: Vector2 = Vector2(sq.position.x + maxf(k2, 0.0), sq.position.y + maxf(-k2, 0.0))
					var b: Vector2 = Vector2(sq.position.x + minf(k2 + 12.0, 14.0), sq.position.y + minf(12.0, 14.0 - k2))
					ci.draw_line(a, b, col, 1.6)
					k2 += 4.5
			elif shp == "grid":
				ci.draw_rect(sq, Color(col.r, col.g, col.b, 0.55))
				for gy: int in 3:
					for gx: int in 3:
						ci.draw_circle(sq.position + Vector2(2.5 + 4.5 * gx, 2.0 + 4.0 * gy), 1.3, UiStyle.TEXT)
			else:
				ci.draw_rect(sq, col)
			ci.draw_rect(sq, UiStyle.SWATCH_EDGE, false, 1.0)
		"tri":
			draw_alert_triangle(ci, Rect2(Vector2((x0 + x1) * 0.5 - 7.0, cy - 7.0), Vector2(14.0, 14.0)), col)


## Triangle d'alerte (forme de l'urgent) avec un point d'exclamation dessiné (pas de glyphe : la police peut ne pas l'avoir).
static func draw_alert_triangle(ci: CanvasItem, r: Rect2, col: Color) -> void:
	var pts: PackedVector2Array = PackedVector2Array([Vector2(r.position.x + r.size.x * 0.5, r.position.y),
		Vector2(r.end.x, r.end.y), Vector2(r.position.x, r.end.y)])
	ci.draw_colored_polygon(pts, col)
	var cx: float = r.position.x + r.size.x * 0.5
	ci.draw_line(Vector2(cx, r.position.y + r.size.y * 0.32), Vector2(cx, r.position.y + r.size.y * 0.68), UiStyle.INK, 2.0)
	ci.draw_circle(Vector2(cx, r.position.y + r.size.y * 0.83), 1.2, UiStyle.INK)
