extends SceneTree

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	root.title = "LPP model render"
	root.size = Vector2i(100, 100)
	root.position = Vector2i(-2000, -2000)
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
	avatar.player.pause()
	avatar.viewport.msaa_3d = Viewport.MSAA_4X
	avatar.viewport.size = Vector2i(1024, 1024)
	for pair in [["front", 0.0], ["desktop", 0.32], ["side", PI/2], ["back", PI]]:
		avatar.pivot.rotation.y = pair[1]
		await process_frame
		await RenderingServer.frame_post_draw
		avatar.viewport.get_texture().get_image().save_png("res://outputs/shu-" + pair[0] + ".png")
	print("SHU RENDERS OK")
	quit()
