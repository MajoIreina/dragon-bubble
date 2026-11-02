class_name SkinCatalog
extends RefCounted

const USER_ROOT := "user://skins"
const MAX_ASSET_BYTES := 256 * 1024 * 1024
var skins: Array[Dictionary] = []
var errors: PackedStringArray = []

func scan() -> void:
	skins.clear()
	errors.clear()
	DirAccess.make_dir_recursive_absolute(USER_ROOT)
	_scan_root("res://skins")
	_scan_root(USER_ROOT)

func _scan_root(root: String) -> void:
	var dir := DirAccess.open(root)
	if dir == null:
		return
	var names := dir.get_directories()
	names.sort()
	for folder in names:
		var manifest := root.path_join(folder).path_join("skin.json")
		var result := read_manifest(manifest)
		if not result.ok:
			errors.append(folder + ": " + result.error)
			continue
		if not find_skin(result.skin.id).is_empty():
			errors.append(folder + ": 外观 ID 重复")
			continue
		skins.append(result.skin)

func find_skin(id: String) -> Dictionary:
	for skin in skins:
		if skin.id == id:
			return skin
	return {}

func read_manifest(path: String) -> Dictionary:
	if not FileAccess.file_exists(path):
		return _fail("缺少 skin.json")
	if FileAccess.get_file_as_bytes(path).size() > 65536:
		return _fail("清单超过 64 KB")
	var data = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not data is Dictionary:
		return _fail("清单必须为 JSON 对象")
	if data.get("schema_version") != 1:
		return _fail("仅支持 schema_version: 1")
	for field in ["id", "name", "type"]:
		if not data.get(field) is String or data[field].strip_edges().is_empty():
			return _fail(field + " 必须是非空字符串")
	var regex := RegEx.new()
	regex.compile("^[a-z0-9][a-z0-9._-]{0,79}$")
	if regex.search(data.id) == null:
		return _fail("id 仅允许小写字母、数字、点、横线、下划线，最长 80 字符")
	if data.type not in ["procedural", "sprite", "model3d"]:
		return _fail("type 必须为 sprite 或 model3d")
	if data.type == "procedural" and not path.begins_with("res://skins/"):
		return _fail("procedural 仅供内置演示使用")
	if not data.get("animations", {}) is Dictionary:
		return _fail("animations 必须为对象")
	for key in data.get("animations", {}):
		if not data.animations[key] is String:
			return _fail("动画映射值必须是字符串")
	if data.type == "sprite" or data.type == "model3d":
		if not data.get("asset") is String or not safe_relative(data.asset):
			return _fail("asset 必须为包内相对路径，不允许 .. 或绝对路径")
		var extension: String = data.asset.get_extension().to_lower()
		if (data.type == "sprite" and extension not in AssetIO.IMAGES + ["gif"]) or (data.type == "model3d" and extension not in AssetIO.MODELS):
			return _fail("不支持的外观资源格式")
		var asset_path: String = path.get_base_dir().path_join(data.asset)
		if not (asset_path.begins_with("res://") and ResourceLoader.exists(asset_path)):
			var asset_file := FileAccess.open(asset_path, FileAccess.READ)
			if asset_file == null:
				return _fail("找不到资源: " + data.asset)
			if asset_file.get_length() > MAX_ASSET_BYTES:
				return _fail("资源超过 256 MB")
	var options_error := validate_options(data)
	if not options_error.is_empty():
		return _fail(options_error)
	if data.has("rotation_degrees"):
		if not data.rotation_degrees is Array or data.rotation_degrees.size() != 3:
			return _fail("rotation_degrees 必须为三个数")
		for value in data.rotation_degrees:
			if not (value is float or value is int) or not is_finite(float(value)):
				return _fail("rotation_degrees 必须为有限数值")
	data["root"] = path.get_base_dir()
	return {"ok": true, "skin": data, "error": ""}

func import_manifest(path: String) -> Dictionary:
	var result := read_manifest(path)
	if not result.ok:
		return result
	return _install(result.skin)

func import_file(path: String) -> Dictionary:
	if path.get_extension().to_lower() == "json":
		return import_manifest(path)
	var extension := path.get_extension().to_lower()
	if extension not in AssetIO.IMAGES + AssetIO.MODELS + ["gif"]:
		return _fail("不支持此文件格式。Live2D / Spine 专用工程暂需导出为精灵图或 GIF。")
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null or file.get_length() > MAX_ASSET_BYTES:
		return _fail("无法读取文件或文件超过 256 MB")
	var id := "custom." + str(Time.get_unix_time_from_system()).replace(".", "") + "." + str(Time.get_ticks_usec())
	return _install({"schema_version": 1, "id": id, "name": path.get_file().get_basename(),
		"type": "model3d" if extension in AssetIO.MODELS else "sprite", "asset": path.get_file(), "root": path.get_base_dir(), "source_format": extension})

func _install(skin: Dictionary) -> Dictionary:
	if not find_skin(skin.id).is_empty():
		return _fail("外观 ID 已存在，请修改 ID 后导入")
	var target := USER_ROOT.path_join(skin.id)
	if DirAccess.dir_exists_absolute(target):
		return _fail("目标文件夹已存在，请在外观目录检查同名包")
	var installed := AssetImporter.install(skin, target)
	if not installed.error.is_empty():
		return _fail(installed.error)
	var validated := read_manifest(target.path_join("skin.json"))
	if not validated.ok:
		AssetImporter._remove_created(target)
		return validated
	scan()
	return {"ok": true, "skin": find_skin(skin.id), "error": ""}

static func validate_options(data: Dictionary) -> String:
	var physics_error := _validate_physics(data.get("physics", {}))
	if not physics_error.is_empty():
		return physics_error
	if data.get("profile", "auto") not in ActionAdapter.PROFILES:
		return "profile 必须为 auto、blob、character 或 flat"
	if data.has("flip_with_direction") and not data.flip_with_direction is bool:
		return "flip_with_direction 必须为布尔值"
	var grid = data.get("grid", [1, 1])
	if not grid is Array or grid.size() != 2:
		return "grid 必须为 [列数, 行数]"
	for value in grid:
		if not _integer(value) or value < 1 or value > 256:
			return "精灵网格必须为 1–256 的整数"
	if int(grid[0]) * int(grid[1]) > 256:
		return "精灵网格最多 256 帧"
	if data.has("frames"):
		if not data.frames is Array or data.frames.is_empty() or data.frames.size() > 256:
			return "frames 需要 1–256 个相对图片路径"
		for frame in data.frames:
			if not frame is String or not safe_relative(frame) or frame.get_extension().to_lower() not in AssetIO.IMAGES:
				return "序列帧必须使用包内相对图片路径"
	var clips = data.get("clips", {})
	if not clips is Dictionary or clips.size() > 128:
		return "clips 必须为动作定义对象（最多 128 个）"
	for name in clips:
		var clip = clips[name]
		if not clip is Dictionary or not clip.get("frames") is Array or clip.frames.is_empty() or clip.frames.size() > 4096:
			return "每个 2D 动作需要 frames 整数索引列表"
		for frame in clip.frames:
			if not _integer(frame) or frame < 0 or frame > 255:
				return "2D 帧索引必须为 0–255 的整数"
		var fps = clip.get("fps", 10.0)
		if not (fps is float or fps is int) or not is_finite(float(fps)) or fps <= 0 or fps > 120:
			return "2D 动作 fps 需要在 0–120 之间"
		if clip.has("loop") and not clip.loop is bool:
			return "loop 必须为布尔值"
		if clip.has("durations"):
			if not clip.durations is Array or clip.durations.size() != clip.frames.size():
				return "durations 必须与动作帧数一致"
			for duration in clip.durations:
				if not (duration is float or duration is int) or not is_finite(float(duration)) or duration <= 0 or duration > 60:
					return "帧时长必须为 0–60 秒之间的有限数"
	return ""

static func _validate_physics(value: Variant) -> String:
	if not value is Dictionary or not value.get("soft_bodies", []) is Array or value.get("soft_bodies", []).size() > 8:
		return "physics 需要对象，soft_bodies 最多 8 个软体部件"
	var targets: Array[String] = []
	for body in value.get("soft_bodies", []):
		if not body is Dictionary:
			return "软体部件需要对象"
		for key in ["node", "profile_mesh"]:
			if not body.get(key) is String or body[key].length() > 256 or not safe_relative(body[key]):
				return "软体 %s 需要模型内部名称或相对节点路径" % key
		if body.node in targets:
			return "软体 node 必须为不重复的模型内部路径"
		targets.append(body.node)
		var anchors = body.get("anchors", [])
		if not anchors is Array or anchors.size() > 32:
			return "软体 anchors 最多 32 个承托节点"
		for anchor in anchors:
			if not anchor is String or anchor.length() > 256 or not safe_relative(anchor):
				return "软体 anchors 需要模型内部名称或相对节点路径"
		var axis = body.get("axis")
		if not axis is Array or axis.size() != 3:
			return "软体 axis 需要三个有限数值"
		for number in axis:
			if not (number is float or number is int) or not is_finite(float(number)) or absf(number) > 1000:
				return "软体 axis 需要三个有限数值"
		var direction := Vector3(axis[0], axis[1], axis[2])
		if direction.length() < 0.0001 or absf(direction.normalized().y) > 0.01:
			return "软体 axis 需要非零的水平向量（模型局部 Y 向上）"
		if not _integer(body.get("sections", 28)) or body.get("sections", 28) < 4 or body.get("sections", 28) > 48:
			return "软体 sections 需要 4–48 的整数"
		var ranges := {"start": [-10000.0, 10000.0], "length": [0.001, 10000.0], "support_height": [-10000.0, 10000.0], "floor_height": [-10000.0, 10000.0], "gravity": [0.0, 9.8], "compression": [0.0, 0.5], "stiffness": [1.0, 200.0], "damping": [0.5, 40.0]}
		for key in ranges:
			if not body.has(key):
				if key in ["length", "support_height", "floor_height"]:
					return "软体缺少 %s" % key
				continue
			var number = body[key]
			var limits: Array = ranges[key]
			if not (number is float or number is int) or not is_finite(float(number)) or number < limits[0] or number > limits[1]:
				return "软体 %s 超出允许范围" % key
		if body.floor_height > body.support_height:
			return "软体 floor_height 不能高于 support_height"
	return ""

static func _integer(value: Variant) -> bool:
	return (value is float or value is int) and is_finite(float(value)) and float(value) == floorf(float(value))

static func safe_relative(path: String) -> bool:
	if path.is_empty() or path.contains(":") or path.begins_with("/") or path.contains("\\"):
		return false
	for part in path.split("/"):
		if part in ["", ".", ".."]:
			return false
	return true

func _fail(message: String) -> Dictionary:
	return {"ok": false, "skin": {}, "error": message}
