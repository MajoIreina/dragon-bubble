## Actual Godot frames; both directions use the same camera, size and floor.
extends SceneTree

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	root.title = "LPP gait render"
	root.size = Vector2i(64, 64)
	root.position = Vector2i(-2000, -2000)
	DirAccess.make_dir_recursive_absolute("res://work/locomotion-frames")
	var catalog := SkinCatalog.new()
	catalog.scan()
	var avatar = load("res://scripts/avatars/model_avatar.gd").new()
	root.add_child(avatar)
	var error: String = avatar.configure(catalog.find_skin("builtin.shu"))
	if not error.is_empty():
		push_error(error)
		quit(1)
		return
	avatar.set_process(false)
	avatar.viewport.size = Vector2i(640, 640)
	for body in avatar.soft_bodies:
		body.active = false
	for state in ["walk", "run"]:
		avatar.set_state(state)
		var clip: String = avatar.mapping[state]
		var duration: float = avatar.player.get_animation(clip).length
		var frames := roundi(duration * 24.0)
		for direction: int in [-1, 1]:
			avatar.direction = direction
			avatar._process(0.0)
			for i in frames:
				avatar.player.stop()
				avatar._restore_pose()
				avatar.player.play(clip, 0.0)
				avatar.player.seek(i / 24.0, true)
				avatar.player.advance(0.0)
				avatar.player.pause()
				for body in avatar.soft_bodies:
					body.advance(1.0 / 24.0)
				await process_frame
				await RenderingServer.frame_post_draw
				var path := "res://work/locomotion-frames/%s-%s-%02d.png" % [state, "left" if direction < 0 else "right", i]
				avatar.viewport.get_texture().get_image().save_png(path)
		print("RENDERED ", state, " frames=", frames, " duration=", duration)
	avatar.free()
	print("LOCOMOTION RENDERS OK")
	quit()
