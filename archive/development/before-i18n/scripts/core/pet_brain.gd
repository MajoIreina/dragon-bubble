class_name PetBrain
extends RefCounted

signal state_changed(state: String)
const WALK_SPEED := 42.0
const RUN_SPEED := 112.0
var state := "idle"
var direction := 1
var roaming := true
var remaining := 4.0
var rng := RandomNumberGenerator.new()

func _init() -> void:
	rng.randomize()

func tick(delta: float) -> void:
	if state in ["dragged", "airborne"]:
		return
	remaining -= delta
	if remaining > 0:
		return
	if state in ["petted", "fall"]:
		change("idle", rng.randf_range(3, 6))
	elif state in ["walk", "run"]:
		change("idle", rng.randf_range(3, 7))
	elif state == "sleep":
		change("idle", rng.randf_range(4, 8))
	elif roaming and rng.randf() < 0.65:
		direction = -1 if rng.randf() < 0.5 else 1
		if rng.randf() < 0.25:
			change("run", rng.randf_range(1.2, 2.8))
		else:
			change("walk", rng.randf_range(2, 5))
	else:
		change("sleep", rng.randf_range(6, 12))

func change(next: String, seconds := 2.0) -> void:
	remaining = seconds
	if state != next:
		state = next
		state_changed.emit(state)

func set_roaming(enabled: bool) -> void:
	roaming = enabled
	if not enabled and state in ["walk", "run"]:
		change("idle", 5)

func movement_speed() -> float:
	return WALK_SPEED if state == "walk" else RUN_SPEED if state == "run" else 0.0
