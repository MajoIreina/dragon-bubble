class_name PetMotion
extends RefCounted

# Screen coordinates in physical pixels. No dependency on a particular avatar.
const GRAVITY := 1800.0
const AIR_DRAG := 0.25
const GROUND_FRICTION := 1250.0
const FLOOR_BOUNCE := 0.32
const WALL_BOUNCE := 0.48
const MAX_THROW_SPEED := 2600.0
const SAMPLE_WINDOW := 0.12
var position := Vector2.ZERO
var velocity := Vector2.ZERO
var active := false
var samples: Array[Dictionary] = []

func begin_drag(at: Vector2, time: float) -> void:
	stop(at)
	samples.clear()
	record_drag(at, time)

func record_drag(at: Vector2, time: float) -> void:
	position = at
	if not samples.is_empty() and time <= float(samples.back().time):
		return
	samples.append({"position": at, "time": time})
	while samples.size() > 1 and time - float(samples.front().time) > SAMPLE_WINDOW:
		samples.pop_front()

func release_velocity(time: float) -> Vector2:
	if samples.size() < 2 or time - float(samples.back().time) > SAMPLE_WINDOW:
		return Vector2.ZERO
	var first: Dictionary = samples.front()
	var last: Dictionary = samples.back()
	var duration := float(last.time) - float(first.time)
	if duration < 0.008:
		return Vector2.ZERO
	return ((Vector2(last.position) - Vector2(first.position)) / duration).limit_length(MAX_THROW_SPEED)

func launch(at: Vector2, speed: Vector2) -> void:
	position = at
	velocity = speed.limit_length(MAX_THROW_SPEED)
	active = true
	samples.clear()

func stop(at: Vector2) -> void:
	position = at
	velocity = Vector2.ZERO
	active = false

# bounds describes the permitted WINDOW ORIGIN, already adjusted for the feet.
# Substeps keep collision and friction stable across different render rates.
func advance(delta: float, bounds: Rect2, desktop: RefCounted = null) -> Dictionary:
	var result := {"landed": false, "settled": false, "impact": 0.0}
	if not active or delta <= 0:
		return result
	var time_left := minf(delta, 0.1)
	while time_left > 0.000001:
		if desktop != null:
			bounds = desktop.bounds_for(position)
		var step := minf(time_left, 1.0 / 120.0)
		time_left -= step
		var on_floor := position.y >= bounds.end.y - 0.01 and is_zero_approx(velocity.y)
		if on_floor:
			position.y = bounds.end.y
			velocity.y = 0
			velocity.x = move_toward(velocity.x, 0, GROUND_FRICTION * step)
		else:
			velocity.y += GRAVITY * step
			velocity.x *= exp(-AIR_DRAG * step)
		position += velocity * step
		if desktop != null:
			bounds = desktop.bounds_for(position)
		if position.x < bounds.position.x:
			position.x = bounds.position.x
			if velocity.x < 0:
				velocity.x *= -WALL_BOUNCE
		elif position.x > bounds.end.x:
			position.x = bounds.end.x
			if velocity.x > 0:
				velocity.x *= -WALL_BOUNCE
		if position.y < bounds.position.y:
			position.y = bounds.position.y
			if velocity.y < 0:
				velocity.y *= -WALL_BOUNCE
		if position.y >= bounds.end.y:
			position.y = bounds.end.y
			if velocity.y > 0:
				result.landed = true
				result.impact = maxf(result.impact, velocity.y)
				velocity.y = -velocity.y * FLOOR_BOUNCE if velocity.y > 180 else 0.0
		if position.y >= bounds.end.y - 0.01 and absf(velocity.y) < 1 and absf(velocity.x) < 8:
			velocity = Vector2.ZERO
			active = false
			result.settled = true
			break
	return result
