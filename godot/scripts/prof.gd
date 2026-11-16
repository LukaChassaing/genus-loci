class_name Prof
extends RefCounted
## Frame-cost profiler for the bench (DENDRO_PROFILE=1 with DENDRO_BENCH): GDScript sections add their
## time here, the binding keeps its own (TreeSim.prof_take), main.gd prints both per frame at the end.

static var on: bool = OS.get_environment("DENDRO_PROFILE") == "1"
static var acc: Dictionary = {}  # section -> Vector3(total ms, calls, longest call ms)


## Adds the time since `t0` (Time.get_ticks_usec()) under `key`.
static func add(key: String, t0: int) -> void:
	if not on:
		return
	var ms: float = float(Time.get_ticks_usec() - t0) / 1000.0
	var v: Vector3 = acc.get(key, Vector3.ZERO)
	acc[key] = Vector3(v.x + ms, v.y + 1.0, maxf(v.z, ms))


static func take() -> Dictionary:
	var d: Dictionary = acc
	acc = {}
	return d
