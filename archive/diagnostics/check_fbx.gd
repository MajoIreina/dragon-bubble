extends SceneTree
func _initialize():
	for file in ["res://work/adapter-fixtures/chibi.fbx", "res://work/adapter-fixtures/rig.glb"]:
		var result := AssetIO.model_scene(file, true)
		print(file, " ", result.error)
		if result.error.is_empty():
			for player in result.scene.find_children("*", "AnimationPlayer", true, false):
				print("CLIPS ", player.get_animation_list())
			result.scene.free()
	var error := AssetIO.fbx_to_glb("res://work/adapter-fixtures/chibi.fbx", "res://work/adapter-fixtures/fbx-out.glb")
	print("CONVERTED ", error)
	var output := AssetIO.model_scene("res://work/adapter-fixtures/fbx-out.glb", true)
	if output.error.is_empty():
		for player in output.scene.find_children("*", "AnimationPlayer", true, false):
			print("CONVERTED CLIPS ", player.get_animation_list())
		output.scene.free()
	quit()
