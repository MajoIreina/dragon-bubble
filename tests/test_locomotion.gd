extends SceneTree

const Renderer = preload("res://scripts/avatars/model_avatar.gd")
var failures := 0
var checks := 0

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
	for body in avatar.soft_bodies:
		body.advance(1.0 / 120.0)

func run() -> void:
	var mapping := ActionAdapter.resolve(PackedStringArray(["Idle", "Walk", "Run"]))
	check(mapping.walk == "Walk" and mapping.run == "Run", "Walking and running select distinct native clips")
	check(ActionAdapter.resolve(PackedStringArray(["Walk"])).run == "Walk", "Older walking-only skins can run")
	check(ActionAdapter.resolve(PackedStringArray(["Running"])).walk == "Running", "Older running-only skins can walk")
	var overridden := ActionAdapter.resolve(PackedStringArray(["Walk", "Happy"]), {"walk": "Happy"})
	check(overridden.walk == "Happy" and overridden.run == "Walk", "A manual walk mapping does not alter running")
	check(not ActionAdapter.resolve(PackedStringArray(["Walk"]), {"run": ""}).has("run"), "Explicit generic running suppresses the native fallback")
	var brain := PetBrain.new()
	brain.rng.seed = 20261102
	var seen := {}
	for i in 100:
		brain.tick(100)
		seen[brain.state] = true
	check(seen.has("walk") and seen.has("run"), "Free roaming reaches both walking and running")
	brain.change("walk", 10)
	var walk_speed := brain.movement_speed()
	brain.change("run", 10)
	check(brain.movement_speed() > walk_speed * 2.0, "Running actually moves faster on the desktop")
	brain.set_roaming(false)
	check(brain.state == "idle" and is_zero_approx(brain.movement_speed()), "Turning roaming off stops a running pet immediately")
	for i in 100:
		brain.tick(100)
	check(brain.state not in ["walk", "run"], "Disabled roaming cannot start locomotion")
	brain.change("run", 0.1)
	brain.tick(0.2)
	check(brain.state == "idle", "Running ends in a stable idle state")
	var catalog := SkinCatalog.new()
	catalog.scan()
	var avatar := Renderer.new()
	root.add_child(avatar)
	check(avatar.configure(catalog.find_skin("builtin.shu")).is_empty(), "Load the updated plush model")
	avatar.set_process(false)
	for body in avatar.soft_bodies:
		body.active = false
	avatar.player.stop()
	avatar._restore_pose()
	var feet: Array[Node3D] = []
	var floors: Array[float] = []
	for name in ["FrontFoot_L", "RearFoot_L", "FrontFoot_R", "RearFoot_R"]:
		feet.append(avatar.pivot.find_children(name, "Node3D", true, false)[0])
		floors.append(avatar._bounds(feet.back(), avatar.pivot).position.y)
	var camera := avatar.viewport.get_camera_3d()
	var display := avatar.visual.get_child(0) as Sprite2D
	for state in ["walk", "run"]:
		avatar.set_state(state)
		var clip: String = avatar.mapping[state]
		var duration: float = avatar.player.get_animation(clip).length
		check(absf(duration - (29.0 if state == "walk" else 16.0) / 24.0) < 0.00001, "Start the exported " + state + " loop at zero without a frozen leading frame")
		check(avatar.native_action and clip == ("Walk" if state == "walk" else "Run"), "Use the dedicated " + state + " animation")
		check(avatar.player.get_animation(clip).loop_mode == Animation.LOOP_LINEAR, "Loop the complete " + state + " gait")
		for direction: int in [-1, 1, -1]:
			avatar.direction = direction
			avatar._process(1.0 / 60.0)
			var forward := avatar.pivot.basis * Vector3.BACK
			check(forward.distance_to(Vector3(direction, 0, 0)) < 0.00001, "Face fully sideways immediately for " + state + " direction " + str(direction))
		avatar.direction = 1
		avatar._process(0.0)
		var lowest := INF
		var maximum_lift := 0.0
		var minimum_supports := 4
		var maximum_supports := 0
		var flight_phases := 0
		var was_flying := false
		var mixed_front_hind_support := false
		for i in 72:
			sample(avatar, clip, duration * i / 72.0)
			var supports := 0
			var fore_support := false
			var hind_support := false
			for j in feet.size():
				var clearance: float = avatar._bounds(feet[j], avatar.pivot).position.y - floors[j]
				lowest = minf(lowest, clearance)
				maximum_lift = maxf(maximum_lift, clearance)
				if clearance < 0.003:
					supports += 1
					fore_support = fore_support or j in [0, 2]
					hind_support = hind_support or j in [1, 3]
			mixed_front_hind_support = mixed_front_hind_support or (fore_support and hind_support)
			if supports == 0 and not was_flying:
				flight_phases += 1
			was_flying = supports == 0
			minimum_supports = mini(minimum_supports, supports)
			maximum_supports = maxi(maximum_supports, supports)
		print("GAIT ", state, " lowest=", lowest, " lift=", maximum_lift, " supports=", minimum_supports, "..", maximum_supports)
		check(lowest > -0.003, "Soles stay above the floor throughout " + state)
		check(maximum_lift > (0.06 if state == "walk" else 0.12), "Feet visibly lift during " + state)
		check(minimum_supports >= 2 if state == "walk" else minimum_supports == 0 and maximum_supports >= 2, "Maintain walking supports / running flight phase for " + state)
		if state == "run":
			check(flight_phases == 2 and not mixed_front_hind_support, "Feline gallop alternates fore/hind support with two suspension phases")
			var body = avatar.soft_bodies[0]
			sample(avatar, clip, duration * 0.35)
			var extended: Vector3 = body.deform_point(Vector3(0, 0.3, -1.6))
			sample(avatar, clip, duration * 0.85)
			var gathered: Vector3 = body.deform_point(Vector3(0, 0.3, -1.6))
			check(gathered.z > extended.z + 0.12, "Rear body contracts forward during gathered suspension")
			check(gathered.y > extended.y + 0.04, "Plush spine arches visibly during gathering")
			check(body.deform_point(Vector3(0, 0.3, 0.0)).distance_to(Vector3(0, 0.3, 0.0)) < 0.00001, "Spinal flex keeps the front of the head anchored")
		for time in ([0.0, 0.1] if state == "walk" else [0.32, 0.35]):
			sample(avatar, clip, time)
			var first := camera.unproject_position(feet[0].global_position)
			sample(avatar, clip, time + 0.10)
			var last := camera.unproject_position(feet[0].global_position)
			var screen_motion := (last - first) * display.scale
			var expected_speed: float = PetBrain.WALK_SPEED if state == "walk" else PetBrain.RUN_SPEED
			print("STANCE ", state, " at ", time, " screen_motion=", screen_motion)
			check(absf(screen_motion.x + expected_speed * 0.10) < 0.25 and absf(screen_motion.y) < 0.1, "Planted sole cancels desktop travel without sliding for " + state + " at " + str(time))
		sample(avatar, clip, 0.0)
		var start := feet[0].global_transform
		sample(avatar, clip, duration - 0.00001)
		check(feet[0].global_transform.is_equal_approx(start), "Close the " + state + " loop without a foot pop")
	avatar.set_state("idle")
	avatar._process(1.0)
	check(absf(avatar.pivot.rotation.y - 0.32) < 0.01, "Return to a readable front view after stopping")
	avatar.free()
	print("LOCOMOTION TESTS: ", checks, " checks, ", failures, " failures")
	quit(1 if failures else 0)
