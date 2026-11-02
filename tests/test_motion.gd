extends SceneTree

const Motion = preload("res://scripts/core/pet_motion.gd")
const BOUNDS := Rect2(0, 0, 1600, 800)
var checks := 0
var failures := 0

func check(value: bool, message: String) -> void:
	checks += 1
	if not value:
		failures += 1
		push_error(message)

func _initialize() -> void:
	var motion := Motion.new()
	motion.begin_drag(Vector2(100, 100), 1.0)
	motion.record_drag(Vector2(140, 80), 1.04)
	motion.record_drag(Vector2(180, 60), 1.08)
	var speed := motion.release_velocity(1.08)
	check(speed.x > 900 and speed.y < -400, "Release keeps both horizontal and upward drag velocity")
	motion.launch(Vector2(180, 200), speed)
	motion.advance(0.05, BOUNDS)
	check(motion.position.x > 180 and motion.position.y < 200, "A thrown pet continues along the drag direction")
	check(motion.velocity.y > speed.y, "Gravity changes upward motion toward falling")
	for i in 120:
		motion.advance(1.0 / 120.0, BOUNDS)
	check(motion.velocity.y >= 0 or not motion.active, "Pet eventually falls after upward throw")
	var impact_seen := false
	var bounced := false
	for i in 1200:
		var result := motion.advance(1.0 / 120.0, BOUNDS)
		if result.landed:
			impact_seen = true
			bounced = bounced or motion.velocity.y < 0
		if not motion.active:
			break
	check(impact_seen and bounced, "Floor contact causes a damped bounce")
	check(not motion.active and motion.position.y == BOUNDS.end.y, "Motion settles exactly on the floor")
	check(motion.velocity == Vector2.ZERO, "Ground friction stops residual sliding")
	motion.begin_drag(Vector2(10, 10), 2.0)
	motion.record_drag(Vector2(100, 10), 2.04)
	motion.record_drag(Vector2(100, 10), 2.25)
	motion.record_drag(Vector2(100, 10), 2.30)
	check(motion.release_velocity(2.3) == Vector2.ZERO, "Holding still before release removes stale throw velocity")
	check(motion.release_velocity(3.0) == Vector2.ZERO, "Old samples cannot create a delayed throw")
	motion.begin_drag(Vector2(200, 200), 3.0)
	check(motion.release_velocity(3.0) == Vector2.ZERO, "Single click has no invented release velocity")
	motion.record_drag(Vector2(200, 200), 3.0)
	check(motion.samples.size() == 1, "Duplicate timestamps cannot divide by zero")
	motion.launch(Vector2(800, 300), Vector2(100000, -100000))
	check(motion.velocity.length() <= Motion.MAX_THROW_SPEED + 0.1, "Extreme input cannot escape speed limit")
	motion.launch(Vector2(1598, 300), Vector2(1000, 0))
	motion.advance(0.05, BOUNDS)
	check(motion.position.x <= BOUNDS.end.x and motion.velocity.x < 0, "Right wall reflects velocity inward")
	motion.launch(Vector2(2, 300), Vector2(-1000, 0))
	motion.advance(0.05, BOUNDS)
	check(motion.position.x >= 0 and motion.velocity.x > 0, "Left wall reflects velocity inward")
	motion.launch(Vector2(300, 2), Vector2(0, -1000))
	motion.advance(0.05, BOUNDS)
	check(motion.position.y >= 0 and motion.velocity.y > 0, "Top collision cannot strand the pet off-screen")
	motion.launch(Vector2(300, 790), Vector2(0, 2500))
	motion.advance(1.0, BOUNDS)
	check(motion.position.y <= 800, "Long frame cannot tunnel through floor")
	motion.launch(Vector2(300, 800), Vector2(0, 1000))
	var floor_throw := motion.advance(1.0 / 120, BOUNDS)
	check(floor_throw.landed and motion.velocity.y < 0, "Downward throw at floor still produces impact and bounce")
	motion.begin_drag(Vector2(500, 400), 4)
	check(not motion.active and motion.velocity == Vector2.ZERO, "Grabbing during flight cancels old momentum")
	var fine := Motion.new()
	var coarse := Motion.new()
	fine.launch(Vector2(500, 50), Vector2(400, -100))
	coarse.launch(Vector2(500, 50), Vector2(400, -100))
	for i in 120:
		fine.advance(1.0 / 120, BOUNDS)
	for i in 30:
		coarse.advance(1.0 / 30, BOUNDS)
	check(fine.position.distance_to(coarse.position) < 0.1, "Substeps produce consistent trajectories at 30 and 120 fps")
	var negative := Rect2(-1920, -200, 1600, 900)
	motion.launch(Vector2(-1500, 0), Vector2(-800, 200))
	for i in 1200:
		motion.advance(1.0 / 120, negative)
	check(not motion.active and motion.position.y == negative.end.y, "Negative monitor coordinates settle correctly")
	check(motion.position.x >= negative.position.x and motion.position.x <= negative.end.x, "Negative-coordinate monitor bounds are respected")
	var brain := PetBrain.new()
	brain.change("airborne")
	brain.tick(1000)
	check(brain.state == "airborne", "Random behavior cannot interrupt flight")
	brain.set_roaming(false)
	check(brain.state == "airborne", "Pausing roaming does not cancel gravity")
	print("MOTION TESTS: ", checks, " checks, ", failures, " failures")
	quit(1 if failures else 0)
