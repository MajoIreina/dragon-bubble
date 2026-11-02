extends SceneTree
func _initialize():
	call_deferred("run")
func run():
	root.position = Vector2i(-2000, -2000)
	root.size = Vector2i(64, 64)
	if not FileAccess.file_exists("res://converters/model_to_glb.py"):
		push_error("Packed converter is missing")
		quit(1)
		return
	for name in ["chibi.blend", "chibi.fbx", "image.gif"]:
		var worker := Thread.new()
		worker.start(func():
			var catalog := SkinCatalog.new()
			catalog.scan()
			return catalog.import_file("L:/LPP/work/adapter-fixtures/" + name))
		while worker.is_alive():
			await process_frame
		var result: Dictionary = worker.wait_to_finish()
		if not result.ok:
			push_error("Packed import " + name + ": " + result.error)
			quit(1)
			return
		var skin: Dictionary = result.skin
		var avatar: PetAvatar = load("res://scripts/avatars/model_avatar.gd" if skin.type == "model3d" else "res://scripts/avatars/sprite_avatar.gd").new()
		root.add_child(avatar)
		var error := avatar.configure(skin)
		if not error.is_empty():
			push_error(error)
			quit(1)
			return
		for state in ActionAdapter.STATES:
			avatar.set_state(state)
			avatar._process(0.15)
		print("PACKED IMPORT OK: ", name, " / ", avatar.mapping)
		avatar.queue_free()
		await process_frame
		AssetImporter._remove_created(SkinCatalog.USER_ROOT.path_join(skin.id))
	print("PACKED ADAPTERS OK")
	quit()
