## Run with Godot (a graphics display is required):
## godot --path . --script tools/render_shu.gd
extends SceneTree

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	root.title = "LPP model render"
	root.size = Vector2i(100, 100)
	root.position = Vector2i(-2000, -2000)
	DirAccess.make_dir_recursive_absolute("res://outputs")
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
	if avatar.player != null:
		avatar.player.seek(0.0, true)
		avatar.player.pause()
	avatar.viewport.size = Vector2i(1024, 1024)
	for pair in [["front", 0.0], ["desktop", 0.32], ["three-quarter", -0.72], ["side", -PI/2], ["other-side", PI/2], ["back", PI]]:
		avatar.pivot.rotation.y = pair[1]
		await process_frame
		await RenderingServer.frame_post_draw
		avatar.viewport.get_texture().get_image().save_png("res://outputs/shu-" + pair[0] + ".png")
	avatar.pivot.rotation.y = 0.0
	for pair in [["sleep", "sleep"], ["happy", "petted"], ["dragged", "dragged"]]:
		avatar.set_state(pair[1])
		if avatar.player != null:
			avatar.player.play(avatar.mapping[pair[1]], 0.0)
			avatar.player.advance(0.3)
			avatar.player.pause()
		avatar._process(0.15)
		avatar.pivot.rotation.y = 0.0
		await process_frame
		await RenderingServer.frame_post_draw
		avatar.viewport.get_texture().get_image().save_png("res://outputs/shu-" + pair[0] + ".png")
	print("SHU RENDERS OK")
	quit()
