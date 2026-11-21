class_name GameHistory
extends RefCounted
## Historique de la partie pour les sparklines : une série par grandeur, échantillonnée en temps SIMULÉ (jours),
## au plus CAPACITY points. Quand une série est pleine, elle est décimée d'un point sur deux et son pas double :
## la courbe couvre toujours toute la partie, à coût mémoire borné. Remise à zéro à chaque nouvelle graine,
## espèce, terrain ou chargement (le propriétaire appelle reset()). Aucune dépendance à la simulation.

const CAPACITY: int = 360
const FIRST_STEP: int = 5   # jours entre deux points au départ

var _series: Dictionary = {}   # nom -> {"days": PackedInt32Array, "vals": PackedFloat32Array, "step": int}


func reset() -> void:
	_series.clear()


## Ajoute `v` au jour `day` si le pas de la série est écoulé (ou si `force`). Un jour antérieur au dernier
## (simulation relancée) remet la série à zéro.
func sample(key: String, day: int, v: float, force: bool = false) -> void:
	if not _series.has(key):
		_series[key] = {"days": PackedInt32Array(), "vals": PackedFloat32Array(), "step": FIRST_STEP}
	var s: Dictionary = _series[key]
	var days: PackedInt32Array = s["days"]
	var vals: PackedFloat32Array = s["vals"]
	if days.size() > 0:
		var last: int = days[days.size() - 1]
		if day < last:
			days = PackedInt32Array()
			vals = PackedFloat32Array()
			s["step"] = FIRST_STEP
		elif day - last < int(s["step"]) and not force:
			return
		elif day == last:
			vals[vals.size() - 1] = v
			s["vals"] = vals
			return
	days.append(day)
	vals.append(v)
	if days.size() > CAPACITY:
		var nd: PackedInt32Array = PackedInt32Array()
		var nv: PackedFloat32Array = PackedFloat32Array()
		for i: int in range(0, days.size(), 2):
			nd.append(days[i])
			nv.append(vals[i])
		if (days.size() - 1) % 2 != 0:
			nd.append(days[days.size() - 1])
			nv.append(vals[vals.size() - 1])
		days = nd
		vals = nv
		s["step"] = int(s["step"]) * 2
	s["days"] = days
	s["vals"] = vals


func values(key: String) -> PackedFloat32Array:
	return (_series[key] as Dictionary)["vals"] if _series.has(key) else PackedFloat32Array()


func size(key: String) -> int:
	return values(key).size()


## Variation de la grandeur sur les `span` derniers jours simulés (NAN si l'historique est plus court).
func change(key: String, span: int = 365) -> float:
	if not _series.has(key):
		return NAN
	var s: Dictionary = _series[key]
	var days: PackedInt32Array = s["days"]
	var vals: PackedFloat32Array = s["vals"]
	var n: int = days.size()
	if n < 2:
		return NAN
	var target: int = days[n - 1] - span
	if days[0] > target:
		return NAN
	var i: int = n - 1
	while i > 0 and days[i] > target:
		i -= 1
	return vals[n - 1] - vals[i]


## Texte de tendance pour une info-bulle : « ↗ +0,4 m en un an », « → stable », ou "" si trop court.
static func trend_text(delta: float, unit: String, digits: int = 1, span_label: String = "en un an") -> String:
	if is_nan(delta):
		return ""
	var eps: float = 0.5 * pow(10.0, -digits)
	if absf(delta) < eps:
		return "→ stable %s" % span_label
	var arrow: String = "↗" if delta > 0.0 else "↘"
	return "%s %s%s %s %s" % [arrow, "+" if delta > 0.0 else "−", UiStyle.num(absf(delta), digits), unit, span_label]
