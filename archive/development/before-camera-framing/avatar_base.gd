class_name PetAvatar
extends Node2D

const DESKTOP_WORLD_PER_PIXEL := 3.5 / 300.0
var state := "idle"
var direction := 1
var elapsed := 0.0
var visual := Node2D.new()
var ground_offset := 248.0
var impact_time := 10.0
var impact_strength := 0.0
var action_profile := "blob"
var native_action := false
var available_clips := PackedStringArray()
var mapping: Dictionary = {}
var explicit_mapping: Dictionary = {}
var capability: Dictionary = {}
var secondary_offset := Vector3.ZERO
var secondary_generation := 0
var secondary_grounded := true
var _desktop_position := Vector2.ZERO
var _desktop_zoom := 0.0

func update_desktop_motion(origin: Vector2, zoom: float) -> void:
	if _desktop_zoom > 0.0 and is_equal_approx(zoom, _desktop_zoom):
		var movement := (origin - _desktop_position) / zoom
		if movement.length() < 640.0:
			secondary_offset += Vector3(movement.x, -movement.y, 0) * DESKTOP_WORLD_PER_PIXEL
		else:
			reset_secondary_motion()
	elif _desktop_zoom > 0.0:
		reset_secondary_motion()
	_desktop_position = origin
	_desktop_zoom = zoom

func desktop_gravity() -> float:
	return PetMotion.GRAVITY * DESKTOP_WORLD_PER_PIXEL / (_desktop_zoom if _desktop_zoom > 0.0 else 1.0)

func reset_secondary_motion() -> void:
	secondary_offset = Vector3.ZERO
	secondary_generation += 1
	_desktop_zoom = 0.0

func _ready() -> void:
	add_child(visual)
	visual.position = Vector2(160, 206)

func configure(_skin: Dictionary) -> String:
	return ""

func set_state(value: String) -> void:
	state = value
	elapsed = 0

func impact(speed: float) -> void:
	impact_time = 0.0
	impact_strength = clampf(speed / 6000.0, 0.04, 0.2)

func _process(delta: float) -> void:
	elapsed += delta
	impact_time += delta
	var stretch := 1.0 + sin(elapsed * 2.2) * 0.012
	var bounce := 0.0
	if state == "walk":
		stretch = 1.0 + sin(elapsed * 13) * 0.045
		bounce = -absf(sin(elapsed * 6.5)) * 9
	elif state == "petted" or state == "fall":
		stretch = 1.0 - cos(elapsed * 17) * exp(-elapsed * 4) * 0.18
	elif state == "dragged":
		stretch = 1.08
	elif state == "airborne":
		stretch = 1.035
	elif state == "sleep":
		stretch = 0.87 + sin(elapsed * 1.5) * 0.018
	# Rigid characters use gentler deformation; native clips supply their own motion.
	var amplitude := 0.32 if action_profile == "character" else 0.7 if action_profile == "flat" else 1.0
	if native_action:
		amplitude *= 0.12
	stretch = 1.0 + (stretch - 1.0) * amplitude
	bounce *= amplitude
	stretch -= cos(impact_time * 20) * exp(-impact_time * 8) * impact_strength * (0.5 if action_profile == "character" else 1.0)
	visual.scale = Vector2(1.0 / stretch, stretch)
	# Deform around the feet so landing and sleeping don't sink through the floor.
	visual.position.y = ground_offset - (ground_offset - 206) * stretch + bounce
