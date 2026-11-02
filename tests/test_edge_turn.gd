## Run graphically: verifies actual window displacement and animation clocks.
extends SceneTree

var checks := 0
var failures := 0

func _initialize() -> void:
	call_deferred("run")

func check(condition: bool, message: String) -> void:
	checks += 1
	if not condition:
		failures += 1
		push_error(message)

func run() -> void:
	var app = load("res://scenes/main.tscn").instantiate()
	root.add_child(app)
	await process_frame
	app.settings.hide()
	root.always_on_top = false
	app.set_process(false)
	app.set_physics_process(false)
	app._switch_skin("builtin.shu")
	app.avatar.set_process(false)
	app.avatar.player.callback_mode_process = AnimationMixer.ANIMATION_CALLBACK_MODE_PROCESS_MANUAL
	for modifier in app.avatar.soft_bodies:
		modifier.active = false
	app.screen_poll = -1000.0
	for zoom in [0.2, 1.0, 3.0]:
		app._set_zoom(zoom, false)
		app.desktop.areas.assign([Rect2(100, 100, 1600, 1100)])
		for state in ["walk", "run"]:
			for direction: int in [-1, 1]:
				app.brain.direction = direction
				app.brain.change(state, 20.0)
				var bounds: Rect2 = app.desktop.bounds_for(Vector2(300, 300))
				root.position = Vector2i(roundi(bounds.end.x) - 1 if direction > 0 else roundi(bounds.position.x) + 1, floori(bounds.end.y))
				app.motion.stop(Vector2(root.position))
				var started := false
				var held := true
				var turn_at := Vector2i.ZERO
				for i in 240:
					var was_turning: bool = app.avatar.turning
					app._process(1.0 / 120.0)
					if app.avatar.turning and not was_turning:
						started = true
						turn_at = root.position
					if was_turning:
						held = held and root.position == turn_at
					app.avatar._process(1.0 / 120.0)
					app.avatar.player.advance(1.0 / 120.0)
				check(started and held, "Window plants at the edge during " + state + " turn at zoom " + str(zoom) + " direction " + str(direction))
				check(not app.avatar.turning and app.brain.direction == -direction and (root.position.x - turn_at.x) * -direction > 1, "Resume away from the edge after a single completed turn")
	# Adjacent screens with a level floor remain open, rather than turning at
	# the first monitor's edge. Use smaller scale to fit completely on either.
	app._set_zoom(1.0, false)
	app.desktop.areas.assign([Rect2(-1280, 0, 1280, 1000), Rect2(0, 0, 1920, 1000)])
	app.brain.direction = 1
	app.brain.change("run", 20.0)
	root.position = Vector2i(-325, floori(1000 - app.desktop.extent.y))
	app.motion.stop(Vector2(root.position))
	var turned := false
	for i in 480:
		app._process(1.0 / 120.0)
		app.avatar._process(1.0 / 120.0)
		turned = turned or app.avatar.turning
	check(not turned and root.position.x > 0, "Cross a connected monitor seam without a spurious edge turn")
	app.queue_free()
	await process_frame
	print("EDGE TURN TESTS: ", checks, " checks, ", failures, " failures")
	quit(1 if failures else 0)
