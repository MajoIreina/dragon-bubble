extends SceneTree
func _initialize():
	var doc := FBXDocument.new()
	var state := FBXState.new()
	print(doc.append_from_file("res://work/adapter-fixtures/chibi.fbx", state))
	var scene := doc.generate_scene(state)
	print("anims ", state.get_animations().size())
	print(GLTFDocument.new().write_to_filesystem(state, "res://work/adapter-fixtures/fbx-generated.glb"))
	scene.free()
	var output := AssetIO.model_scene("res://work/adapter-fixtures/fbx-generated.glb", true)
	if output.error.is_empty():
		for player in output.scene.find_children("*", "AnimationPlayer", true, false):
			print("CONVERTED CLIPS ", player.get_animation_list())
		output.scene.free()
	quit()
