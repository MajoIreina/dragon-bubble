class_name AssetIO
extends RefCounted

const IMAGES := ["png", "jpg", "jpeg", "webp", "bmp", "tga", "svg"]
const MODELS := ["glb", "gltf", "vrm", "fbx", "obj", "blend", "stl", "ply"]
const CONVERTED := ["blend", "stl", "ply"]
const MAX_BYTES := 256 * 1024 * 1024
const MAX_IMAGE_SIZE := 8192

static func load_image(path: String) -> Image:
	if path.begins_with("res://"):
		var texture := load(path) as Texture2D
		return texture.get_image() if texture else null
	return Image.load_from_file(path)

static func gltf_json(path: String) -> Dictionary:
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null or file.get_length() > MAX_BYTES:
		return {"error": "无法读取资源或文件超过 256 MB"}
	var data
	if path.get_extension().to_lower() == "gltf":
		data = JSON.parse_string(file.get_as_text())
	else:
		if file.get_length() < 20 or file.get_32() != 0x46546c67 or file.get_32() != 2:
			return {"error": "需要 glTF 2.0 二进制文件"}
		var declared := file.get_32()
		var length := file.get_32()
		if declared != file.get_length() or file.get_32() != 0x4e4f534a or length > file.get_length() - 20:
			return {"error": "GLB 头部或 JSON 块无效"}
		data = JSON.parse_string(file.get_buffer(length).get_string_from_utf8())
	if not data is Dictionary:
		return {"error": "glTF JSON 无效"}
	return {"error": "", "data": data}

static func validate_gltf(path: String, allow_dependencies := false) -> String:
	var result := gltf_json(path)
	if not result.error.is_empty():
		return result.error
	var total := FileAccess.get_file_as_bytes(path).size()
	for section in ["buffers", "images"]:
		var entries = result.data.get(section, [])
		if not entries is Array:
			return "glTF 资源列表无效"
		for entry in entries:
			if not entry is Dictionary:
				return "glTF 资源条目无效"
			if not entry.has("uri"):
				continue
			if not entry.uri is String:
				return "glTF URI 必须是字符串"
			if entry.uri.begins_with("data:"):
				continue
			var uri: String = entry.uri.uri_decode()
			if not allow_dependencies or not SkinCatalog.safe_relative(uri):
				return "纹理与缓冲区需位于模型目录内，不支持绝对路径、上级目录或网络 URI"
			var dependency := FileAccess.open(path.get_base_dir().path_join(uri), FileAccess.READ)
			if dependency == null:
				return "缺少模型依赖：" + uri
			total += dependency.get_length()
			if total > MAX_BYTES:
				return "模型与依赖合计超过 256 MB"
	return ""

static func model_scene(path: String, allow_dependencies := false) -> Dictionary:
	var extension := path.get_extension().to_lower()
	if extension == "obj":
		return ObjReader.read(path)
	if extension in ["glb", "gltf", "vrm"]:
		var error := validate_gltf(path, allow_dependencies)
		if not error.is_empty():
			return {"error": error}
	var doc: GLTFDocument = FBXDocument.new() if extension == "fbx" else GLTFDocument.new()
	var state: GLTFState = FBXState.new() if extension == "fbx" else GLTFState.new()
	var code := doc.append_from_file(path, state)
	if code != OK:
		return {"error": "模型解码失败：" + error_string(code)}
	var scene := doc.generate_scene(state)
	if scene == null:
		return {"error": "未生成有效模型场景"}
	return {"error": "", "scene": scene}

static func write_glb(scene: Node, path: String) -> String:
	var doc := GLTFDocument.new()
	var state := GLTFState.new()
	var code := doc.append_from_scene(scene, state)
	if code == OK:
		code = doc.write_to_filesystem(state, path)
	return "" if code == OK else "模型标准化失败：" + error_string(code)

static func repack_gltf(source: String, destination: String) -> String:
	# Pack JSON/buffers directly. Re-exporting a generated scene can lose morph
	# data and format extensions, especially on the headless dummy renderer.
	var error := validate_gltf(source, true)
	if not error.is_empty():
		return error
	var data: Dictionary = gltf_json(source).data.duplicate(true)
	var source_bin := PackedByteArray()
	if source.get_extension().to_lower() != "gltf":
		var file := FileAccess.open(source, FileAccess.READ)
		file.seek(12)
		var json_length := file.get_32()
		file.seek(20 + json_length)
		if file.get_position() + 8 <= file.get_length():
			var bin_length := file.get_32()
			if file.get_32() == 0x004e4942:
				source_bin = file.get_buffer(bin_length)
	var binary := PackedByteArray()
	var offsets: Array[int] = []
	var lengths: Array[int] = []
	for buffer in data.get("buffers", []):
		var payload := source_bin
		if buffer.has("uri"):
			var uri: String = buffer.uri
			payload = Marshalls.base64_to_raw(uri.get_slice(",", 1)) if uri.begins_with("data:") else FileAccess.get_file_as_bytes(source.get_base_dir().path_join(uri.uri_decode()))
		if payload.size() < int(buffer.get("byteLength", 0)):
			return "glTF 缓冲区被截断"
		while binary.size() % 4:
			binary.append(0)
		offsets.append(binary.size())
		lengths.append(payload.size())
		binary.append_array(payload)
	for view in data.get("bufferViews", []):
		var index := int(view.get("buffer", -1))
		if index < 0 or index >= offsets.size() or int(view.get("byteOffset", 0)) < 0 or int(view.get("byteOffset", 0)) + int(view.get("byteLength", 0)) > lengths[index]:
			return "glTF 缓冲视图越界"
		view.byteOffset = int(view.get("byteOffset", 0)) + offsets[index]
		view.buffer = 0
	for image in data.get("images", []):
		if not image.has("uri") or image.uri.begins_with("data:"):
			continue
		var uri: String = image.uri.uri_decode()
		var payload := FileAccess.get_file_as_bytes(source.get_base_dir().path_join(uri))
		while binary.size() % 4:
			binary.append(0)
		if not data.has("bufferViews"):
			data.bufferViews = []
		image.bufferView = data.bufferViews.size()
		image.mimeType = "image/jpeg" if uri.get_extension().to_lower() in ["jpg", "jpeg"] else "image/webp" if uri.get_extension().to_lower() == "webp" else "image/png"
		data.bufferViews.append({"buffer": 0, "byteOffset": binary.size(), "byteLength": payload.size()})
		binary.append_array(payload)
		image.erase("uri")
	if binary.size() > MAX_BYTES:
		return "标准化模型超过 256 MB"
	data.buffers = [{"byteLength": binary.size()}]
	var json := JSON.stringify(data).to_utf8_buffer()
	while json.size() % 4:
		json.append(32)
	while binary.size() % 4:
		binary.append(0)
	var file := FileAccess.open(destination, FileAccess.WRITE)
	if file == null:
		return "无法写入模型文件"
	for value in [0x46546c67, 2, 28 + json.size() + binary.size(), json.size(), 0x4e4f534a]:
		file.store_32(value)
	file.store_buffer(json)
	file.store_32(binary.size())
	file.store_32(0x004e4942)
	file.store_buffer(binary)
	return ""

static func fbx_to_glb(source: String, destination: String) -> String:
	var doc := FBXDocument.new()
	var state := FBXState.new()
	var code := doc.append_from_file(source, state)
	if code != OK:
		return "FBX 读取失败：" + error_string(code)
	# Export from the generated scene to carry AnimationPlayer bone/morph tracks.
	# Serializing the import state alone drops FBX animations.
	var scene := doc.generate_scene(state)
	if scene == null:
		return "FBX 未生成有效场景"
	if DisplayServer.get_name() == "headless":
		for mesh in scene.find_children("*", "MeshInstance3D", true, false):
			if mesh.mesh and mesh.mesh.get_blend_shape_count() > 0:
				scene.free()
				return "带形态键的 FBX 需要图形渲染器进行转换，请正常启动桌宠后导入。"
	var error := write_glb(scene, destination)
	scene.free()
	return error

static func blender_path() -> String:
	var configured := OS.get_environment("LPP_BLENDER_PATH")
	var config := "user://converter-path.txt"
	if FileAccess.file_exists(config):
		configured = FileAccess.get_file_as_string(config).strip_edges()
	if FileAccess.file_exists(configured):
		return configured
	var candidates: Array[String] = []
	for i in DirAccess.get_drive_count():
		var drive := DirAccess.get_drive_name(i)
		candidates.append(drive.path_join("SteamLibrary/steamapps/common/Blender/blender.exe"))
	for folder in ["C:/Program Files/Blender Foundation", "C:/Program Files (x86)/Steam/steamapps/common"]:
		var dir := DirAccess.open(folder)
		if dir:
			for sub in dir.get_directories():
				if sub.to_lower().begins_with("blender"):
					candidates.append(folder.path_join(sub).path_join("blender.exe"))
	for candidate in candidates:
		if FileAccess.file_exists(candidate):
			return candidate
	return ""

static func convert_with_blender(source: String, destination: String) -> String:
	var executable := blender_path()
	if executable.is_empty():
		return "此格式需要 Blender 4.2+。请在“动作与适配”中设置 blender.exe 路径，或导出为 GLB 后导入。"
	var script := "user://converter-model_to_glb.py"
	var file := FileAccess.open(script, FileAccess.WRITE)
	if file == null:
		return "无法写入模型转换脚本"
	file.store_string(FileAccess.get_file_as_string("res://converters/model_to_glb.py"))
	file.close()
	var output: Array = []
	var code := OS.execute(executable, PackedStringArray(["--background", "--factory-startup", "--disable-autoexec", "--python", ProjectSettings.globalize_path(script), "--", "--input", ProjectSettings.globalize_path(source), "--output", ProjectSettings.globalize_path(destination)]), output, true, false)
	if code != 0 or not FileAccess.file_exists(destination):
		var log := FileAccess.open("user://conversion-last.log", FileAccess.WRITE)
		if log:
			log.store_string("\n".join(output))
		return "Blender 转换失败，详情见运行数据目录中的 conversion-last.log"
	return ""
