extends SceneTree
func _initialize():
	var doc := FBXDocument.new()
	var state := FBXState.new()
	doc.append_from_file("res://work/adapter-fixtures/chibi.fbx", state)
	for a in state.get_animations():
		for p in a.get_property_list():
			print(p.name, " ", str(a.get(p.name)).left(600))
	quit()
