class_name PetBrain
extends RefCounted

signal state_changed(state: String)
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
	elif state == "walk":
		change("idle", rng.randf_range(3, 7))
	elif state == "sleep":
		change("idle", rng.randf_range(4, 8))
	elif roaming and rng.randf() < 0.65:
		direction = -1 if rng.randf() < 0.5 else 1
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
	if not enabled and state == "walk":
		change("idle", 5)
