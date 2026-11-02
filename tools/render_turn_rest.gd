extends SceneTree

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	root.size = Vector2i(64, 64)
	root.position = Vector2i(-2000, -2000)
	DirAccess.make_dir_recursive_absolute("res://work/turn-rest-frames")
	var canvas := SubViewport.new()
	canvas.size = Vector2i(640, 640)
	canvas.transparent_bg = true
	canvas.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	root.add_child(canvas)
	var catalog := SkinCatalog.new()
	catalog.scan()
	var avatar = load("res://scripts/avatars/model_avatar.gd").new()
	canvas.add_child(avatar)
	if not avatar.configure(catalog.find_skin("builtin.shu")).is_empty():
		quit(1)
		return
	avatar.scale = Vector2.ONE * 2.0
	avatar.set_process(false)
	avatar.player.callback_mode_process = AnimationMixer.ANIMATION_CALLBACK_MODE_PROCESS_MANUAL
	for body in avatar.soft_bodies:
		body.active = false
	for scene in ["idle", "sleep", "turn-left", "turn-right", "run-turn"]:
		avatar.direction = -1 if scene == "turn-left" else 1
		avatar.set_state("run" if scene == "run-turn" else "walk" if scene.begins_with("turn") else scene)
		avatar.effects.clock = 0.0
		avatar.effects.sleep_opacity = 0.0
		avatar.pivot.rotation.y = avatar.direction * 0.32
		avatar._process(0.0)
		# Settle the preceding pose; then capture transitions with real blends.
		avatar.player.advance(0.5)
		var count := 144 if scene in ["idle", "sleep"] else 120
		for i in count:
			if scene not in ["idle", "sleep"] and i in [12, 72]:
				avatar.begin_turn(-avatar.direction)
			avatar._process(1.0 / 24.0)
			avatar.player.advance(1.0 / 24.0)
			for body in avatar.soft_bodies:
				body.advance(1.0 / 24.0)
			await process_frame
			await RenderingServer.frame_post_draw
			canvas.get_texture().get_image().save_png("res://work/turn-rest-frames/%s-%03d.png" % [scene, i])
		print("TURN/REST RENDERED ", scene, " anchor=", avatar.effect_anchor)
	avatar.free()
	print("TURN/REST RENDERS OK")
	quit()
