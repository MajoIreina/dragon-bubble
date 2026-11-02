class_name SkinOverrides
extends RefCounted

const PATH := "user://appearance-overrides.json"

static func read_all() -> Dictionary:
	var value = JSON.parse_string(FileAccess.get_file_as_string(PATH)) if FileAccess.file_exists(PATH) else {}
	return value if value is Dictionary else {}

static func effective(skin: Dictionary) -> Dictionary:
	if skin.is_empty():
		return {}
	var result := skin.duplicate(true)
	var entry = read_all().get(skin.id, {})
	if entry is Dictionary:
		for key in ["profile", "rotation_degrees", "animations", "flip_with_direction"]:
			if entry.has(key):
				result[key] = entry[key]
		if not SkinCatalog.validate_options(result).is_empty() or not result.get("animations", {}) is Dictionary:
			return skin.duplicate(true)
		for value in result.get("animations", {}).values():
			if not value is String:
				return skin.duplicate(true)
		var rotation = result.get("rotation_degrees", [0, 0, 0])
		if not rotation is Array or rotation.size() != 3:
			return skin.duplicate(true)
		for value in rotation:
			if not (value is float or value is int) or not is_finite(float(value)):
				return skin.duplicate(true)
	return result

static func save(id: String, options: Dictionary) -> Error:
	var all := read_all()
	all[id] = options
	var file := FileAccess.open(PATH + ".tmp", FileAccess.WRITE)
	if file == null:
		return FileAccess.get_open_error()
	file.store_string(JSON.stringify(all, "\t"))
	file.close()
	return DirAccess.rename_absolute(PATH + ".tmp", PATH)

static func reset(id: String) -> Error:
	var all := read_all()
	all.erase(id)
	var file := FileAccess.open(PATH, FileAccess.WRITE)
	if file == null:
		return FileAccess.get_open_error()
	file.store_string(JSON.stringify(all, "\t"))
	return OK
