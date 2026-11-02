extends SceneTree

const Renderer = preload("res://scripts/avatars/model_avatar.gd")
var checks := 0
var failures := 0

func _initialize() -> void:
	call_deferred("run")

func check(condition: bool, message: String) -> void:
	checks += 1
	if not condition:
		failures += 1
		push_error(message)

func sample(avatar, clip: String, time: float) -> void:
	avatar.player.stop()
	avatar._restore_pose()
	avatar.player.play(clip, 0.0)
	avatar.player.seek(time, true)
	avatar.player.advance(0.0)
	avatar.player.pause()

func run() -> void:
	var catalog := SkinCatalog.new()
	catalog.scan()
	var avatar := Renderer.new()
	root.add_child(avatar)
	check(avatar.configure(catalog.find_skin("builtin.shu")).is_empty(), "Load enhanced plush actions")
	avatar.set_process(false)
	for body in avatar.soft_bodies:
		body.active = false
	var body: Node3D = avatar.pivot.find_children("PlushBody", "Node3D", true, false)[0]
	var eyes: Node3D = avatar.pivot.find_children("BlinkBothEyes", "Node3D", true, false)[0]
	var feet: Array[Node3D] = []
	avatar.player.stop()
	avatar._restore_pose()
	var floors: Array[float] = []
	for part in ["FrontFoot_L", "RearFoot_L", "FrontFoot_R", "RearFoot_R"]:
		feet.append(avatar.pivot.find_children(part, "Node3D", true, false)[0])
		floors.append(avatar._bounds(feet.back(), avatar.pivot).position.y)
	check(avatar.player.has_animation("Turn"), "Native turning uses a stepping clip")
	for state in ["walk", "run"]:
		for direction: int in [-1, 1]:
			avatar.direction = direction
			avatar.set_state(state)
			avatar._process(0.0)
			avatar.begin_turn(-direction)
			check(avatar.turning and avatar.player.current_animation == "Turn", "Start a planted " + state + " turn " + str(direction))
			check(is_equal_approx(avatar.pivot.rotation.y, direction * PI / 2), "Turn begins without a yaw pop")
			avatar.locomotion_paused = true
			avatar._process(0.3)
			check(is_zero_approx(avatar.turn_elapsed) and is_zero_approx(avatar.player.speed_scale), "Menus pause both turn timer and foot animation")
			avatar.locomotion_paused = false
			var previous: float = avatar.pivot.rotation.y
			var maximum_step := 0.0
			var front_seen := false
			var frames := ceili(avatar.turn_duration * 120)
			for i in frames:
				avatar._process(avatar.turn_duration / frames)
				var yaw: float = avatar.pivot.rotation.y
				maximum_step = maxf(maximum_step, absf(yaw - previous))
				front_seen = front_seen or absf(yaw) < 0.04
				previous = yaw
			avatar._process(0.0001)
			check(front_seen and maximum_step < 0.09, "Rotate continuously through the front over 180 degrees")
			check(not avatar.turning and absf(avatar.pivot.rotation.y + direction * PI / 2) < 0.00001, "Finish fully side-on in the reverse direction")
			check(avatar.player.current_animation == avatar.mapping[state] and avatar.resume_rate() < 0.3, "Resume the original gait with a gentle takeoff")
	avatar.set_state("walk")
	avatar.begin_turn(-avatar.direction)
	avatar.set_state("dragged")
	check(not avatar.turning, "Dragging immediately cancels an unfinished turn")
	for clip in ["Idle", "Sleep", "Turn"]:
		var duration: float = avatar.player.get_animation(clip).length
		var minimum_height := INF
		var maximum_height := -INF
		var minimum_eye := INF
		var maximum_eye := 0.0
		var minimum_supports := 4
		var lowest := INF
		var lift := 0.0
		for i in 144:
			sample(avatar, clip, duration * i / 144.0)
			minimum_height = minf(minimum_height, body.scale.y)
			maximum_height = maxf(maximum_height, body.scale.y)
			minimum_eye = minf(minimum_eye, eyes.scale.y)
			maximum_eye = maxf(maximum_eye, eyes.scale.y)
			var supports := 0
			for j in feet.size():
				var clearance: float = avatar._bounds(feet[j], avatar.pivot).position.y - floors[j]
				lowest = minf(lowest, clearance)
				lift = maxf(lift, clearance)
				if clearance < 0.003:
					supports += 1
			minimum_supports = mini(minimum_supports, supports)
		print("REST ", clip, " lowest=", lowest, " supports=", minimum_supports, " lift=", lift)
		check(lowest > -0.003 and minimum_supports >= 2, "Keep actual soles supported throughout " + clip)
		check(maximum_height - minimum_height > 0.015, "Visible body breathing or settling in " + clip)
		if clip == "Idle":
			check(minimum_eye < 0.15 and maximum_eye > 0.99, "Idle has visible blinks and open-eye pauses")
		elif clip == "Sleep":
			check(maximum_eye < 0.05 and maximum_height < 0.95, "Sleep maintains relaxed body and fully closed eyes")
		else:
			check(lift > 0.04, "Turning visibly lifts the stepping feet")
		sample(avatar, clip, 0.0)
		var first := body.transform
		sample(avatar, clip, duration - 0.00001)
		check(body.transform.is_equal_approx(first), "Rest/turn clip has no loop seam: " + clip)
	avatar.set_state("sleep")
	avatar.player.advance(1.0)
	var sleeping_body := body.transform
	avatar.set_state("idle")
	check(body.transform.is_equal_approx(sleeping_body), "Waking keeps the preceding body pose at the transition boundary")
	avatar.player.advance(0.1)
	check(eyes.scale.y > 0.04 and eyes.scale.y < 0.7, "Wake-up opens eyes gradually through the native blend")
	avatar.player.advance(0.5)
	check(eyes.scale.y > 0.99, "Wake-up completes with fully open eyes")
	avatar.set_state("sleep")
	avatar._process(0.6)
	check(avatar.effects.sleep_opacity > 0.95, "Sleep accents fade in")
	var particles: Array[Dictionary] = avatar.effects.sleep_particles()
	check(particles.size() == 3 and particles[0].position != particles[1].position, "Sleep letters rise in staggered phases")
	avatar.set_state("idle")
	avatar._process(0.6)
	check(is_zero_approx(avatar.effects.sleep_opacity), "Sleep accents fade away after waking")
	avatar.free()
	print("TURN AND REST TESTS: ", checks, " checks, ", failures, " failures")
	quit(1 if failures else 0)
