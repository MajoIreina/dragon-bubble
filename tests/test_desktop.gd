extends SceneTree
const Layout = preload("res://scripts/core/desktop_layout.gd")
const Motion = preload("res://scripts/core/pet_motion.gd")
var checks := 0
var failures := 0

func check(value: bool, message: String) -> void:
	checks += 1
	if not value:
		failures += 1
		push_error(message)

func _initialize() -> void:
	var layout := Layout.new()
	layout.extent = Vector2(200, 160)
	layout.areas = [Rect2(-1280, 0, 1280, 984), Rect2(0, 0, 1920, 1040)]
	check(layout.screen_at(Vector2(-600, 50)) == 0, "Left monitor with negative origin")
	check(layout.screen_at(Vector2(1500, 50)) == 1, "Mixed resolution right monitor")
	check(layout.bounds_for(Vector2(-300, 500)).end.x == 1720, "Connected right seam opens before window reaches edge")
	var motion := Motion.new()
	motion.launch(Vector2(-240, 400), Vector2(1300, -200))
	for i in 40:
		motion.advance(1.0 / 120, layout.bounds_for(motion.position), layout)
	check(motion.position.x > 0 and motion.velocity.x > 0, "Throw crosses seam without reflection")
	for i in 1200:
		motion.advance(1.0 / 120, layout.bounds_for(motion.position), layout)
	check(not motion.active and is_equal_approx(motion.position.y, 880), "Falls to target monitor work area floor")
	check(motion.position.x <= 1720, "Outer desktop edge still collides")
	# Reverse throw into a taller floor must meet a step, not jump upwards.
	check(layout.bounds_for(Vector2(20, 880)).position.x == 0, "Higher neighbouring floor blocks ground crossing")
	check(layout.bounds_for(Vector2(20, 500)).position.x == -1280, "Same neighbour permits airborne crossing")
	var at := Vector2(-250, 824)
	for i in 200:
		at.x += 2
		at = layout.clamp_origin(at)
	check(at.x > 0, "Walking can cross onto lower neighbouring floor")
	check(at.y < layout.bounds_for(at).end.y, "Walk into lower display triggers gravity")
	# Disconnected rectangles must not become a fictitious giant desktop.
	layout.areas = [Rect2(0, 0, 1000, 800), Rect2(1200, 200, 800, 800)]
	check(layout.bounds_for(Vector2(750, 100)).end.x == 800, "Gap between monitors stays closed")
	var recovered := layout.clamp_origin(Vector2(1050, 400))
	check(recovered.x == 1200, "Gap position recovers fully to nearest monitor")
	layout.areas = [Rect2(0, -900, 1600, 900), Rect2(0, 0, 1600, 1000)]
	motion.launch(Vector2(400, -200), Vector2(0, 500))
	for i in 1200:
		motion.advance(1.0 / 120, layout.bounds_for(motion.position), layout)
	check(not motion.active and is_equal_approx(motion.position.y, 840), "Gravity crosses vertically connected work areas")
	check(layout.bounds_for(Vector2(400, 10)).position.y == -900, "Vertical seam permits upward throw")
	layout.areas = [Rect2(0, 0, 1920, 1040)]
	recovered = layout.clamp_origin(Vector2(-1500, -500))
	check(recovered == Vector2.ZERO, "Removed display position recovers onto remaining screen")
	layout.extent = Vector2(2500, 1500)
	check(layout.clamp_origin(Vector2(300, 400)) == Vector2.ZERO, "Oversized pet recovers without inverted bounds")
	# Staggered screens connect only over their actual shared vertical interval.
	layout.extent = Vector2(200, 160)
	layout.areas = [Rect2(0, 0, 1920, 1040), Rect2(1920, -500, 1440, 1200)]
	check(layout.bounds_for(Vector2(1600, 800)).end.x == 1720, "Staggered non-overlapping edge stays solid")
	check(layout.bounds_for(Vector2(1600, 200)).end.x == 3160, "Staggered overlapping edge opens")
	print("DESKTOP TESTS: ", checks, " checks, ", failures, " failures")
	quit(1 if failures else 0)
