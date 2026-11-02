extends SceneTree

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	root.title = "LPP soft body render"
	root.size = Vector2i(100, 100)
	root.position = Vector2i(-2000, -2000)
	var catalog := SkinCatalog.new()
	catalog.scan()
	for enabled in [false, true]:
		var skin: Dictionary = catalog.find_skin("builtin.shu").duplicate(true)
		if not enabled:
			skin.erase("physics")
		var avatar = load("res://scripts/avatars/model_avatar.gd").new()
		root.add_child(avatar)
		var error: String = avatar.configure(skin)
		if not error.is_empty():
			push_error(error)
			quit(1)
			return
		avatar.set_process(false)
		avatar.player.stop()
		avatar._restore_pose()
		avatar.viewport.size = Vector2i(1024, 1024)
		for body in avatar.soft_bodies:
			body.active = false
			body.advance(1.0 / 120.0)
		for view in [{"name": "side", "angle": -PI / 2.0}, {"name": "oblique", "angle": -0.85}, {"name": "front", "angle": 0.0}]:
			avatar.pivot.rotation.y = view.angle
			await process_frame
			await RenderingServer.frame_post_draw
			avatar.viewport.get_texture().get_image().save_png("res://outputs/shu-soft-" + view.name + ("-after" if enabled else "-before") + ".png")
		if enabled:
			if avatar.soft_bodies.is_empty():
				print(avatar.pivot.get_tree_string_pretty())
				quit(1)
				return
			var body = avatar.soft_bodies[0]
			for z in [0.0, -0.5, -1.0, -1.5, -2.0, -2.25]:
				var i: int = roundi(body.coordinate(Vector3(0, 0, z)))
				print("SECTION ", z, " lower ", body.lower[i], " -> ", body.deform_point(Vector3(0, body.lower[i], z)), " upper ", body.upper[i], " -> ", body.deform_point(Vector3(0, body.upper[i], z)))
		avatar.free()
	print("SOFT BODY RENDER OK")
	quit()
