class_name AssetImporter
extends RefCounted

## Transactional import. Source files are read-only; only a newly created target
## is cleaned on failure. Models become self-contained GLB before installation.
static func install(skin: Dictionary, target: String) -> Dictionary:
	if DirAccess.make_dir_recursive_absolute(target) != OK:
		return {"error": "无法创建外观目录"}
	var data := skin.duplicate(true)
	data.erase("root")
	var source: String = skin.root.path_join(skin.asset)
	var error := ""
	if skin.type == "model3d":
		data.asset = "model.glb"
		if source.get_extension().to_lower() in AssetIO.CONVERTED:
			error = AssetIO.convert_with_blender(source, target.path_join(data.asset))
			if error.is_empty():
				source = target.path_join(data.asset)
		if error.is_empty():
			var extension := source.get_extension().to_lower()
			if extension == "fbx":
				error = AssetIO.fbx_to_glb(source, target.path_join(data.asset))
			elif extension in ["glb", "gltf", "vrm"] and source != target.path_join(data.asset):
				error = AssetIO.repack_gltf(source, target.path_join(data.asset))
			if extension in ["glb", "gltf", "vrm", "fbx"]:
				source = target.path_join(data.asset)
		if error.is_empty():
			var result := AssetIO.model_scene(source, true)
			error = result.error
			if error.is_empty():
				var scene: Node = result.scene
				if scene.find_children("*", "MeshInstance3D", true, false).is_empty() and not scene is MeshInstance3D:
					error = "文件不包含可显示的网格"
				elif source.get_extension().to_lower() == "obj":
					error = AssetIO.write_glb(scene, target.path_join(data.asset))
				scene.free()
	else:
		var relatives: Array = skin.get("frames", [skin.asset]).duplicate()
		if skin.asset not in relatives:
			relatives.append(skin.asset)
		var total := 0
		for relative: String in relatives:
			var from: String = skin.root.path_join(relative)
			var file := FileAccess.open(from, FileAccess.READ)
			if file == null:
				error = "找不到图片：" + relative
				break
			total += file.get_length()
			if total > AssetIO.MAX_BYTES:
				error = "图片文件合计超过 256 MB"
				break
			var destination := target.path_join(relative)
			DirAccess.make_dir_recursive_absolute(destination.get_base_dir())
			if DirAccess.copy_absolute(from, destination) != OK:
				error = "图片复制失败"
				break
		if error.is_empty():
			# Image decode and frame validation do not require a scene tree.
			var avatar = load("res://scripts/avatars/sprite_avatar.gd").new()
			avatar.add_child(avatar.visual)
			var candidate := data.duplicate(true)
			candidate.root = target
			error = avatar.configure(candidate)
			avatar.free()
	if error.is_empty():
		var file := FileAccess.open(target.path_join("skin.json"), FileAccess.WRITE)
		if file:
			file.store_string(JSON.stringify(data, "\t"))
			file.close()
		else:
			error = "清单写入失败"
	if not error.is_empty():
		_remove_created(target)
	return {"error": error}

static func _remove_created(path: String) -> void:
	# Caller only supplies a new, private target under user://skins.
	if not path.begins_with(SkinCatalog.USER_ROOT + "/"):
		return
	var dir := DirAccess.open(path)
	if dir == null:
		return
	for file in dir.get_files():
		DirAccess.remove_absolute(path.path_join(file))
	for folder in dir.get_directories():
		_remove_created(path.path_join(folder))
	DirAccess.remove_absolute(path)
