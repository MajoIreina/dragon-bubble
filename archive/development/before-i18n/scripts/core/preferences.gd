class_name PetPreferences
extends RefCounted

const PATH := "user://preferences.json"
const MIN_ZOOM := 0.2
const MAX_ZOOM := 3.0
var path := PATH
const DEFAULT_SKIN := "builtin.shu"
const APPEARANCE_REVISION := 1
var skin_id := DEFAULT_SKIN
var zoom := 1.0
var roaming := true
var always_on_top := true
var position := Vector2i(-99999, -99999)
var performance := PetRenderOptions.DEFAULTS.duplicate()

func load_saved() -> void:
	if not FileAccess.file_exists(path):
		return
	var data = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not data is Dictionary:
		return
	if data.get("skin_id") is String:
		skin_id = data.skin_id
	# Replace the old demonstration appearance once, preserving custom packs.
	# Saving the revision allows users to select any demonstration again later.
	if data.get("appearance_revision", 0) != APPEARANCE_REVISION and skin_id in ["builtin.mochi", "builtin.mint", "example.sprite", "example.glb"]:
		skin_id = DEFAULT_SKIN
	if data.get("zoom") is float or data.get("zoom") is int:
		zoom = normalize_zoom(float(data.zoom))
	if data.get("roaming") is bool:
		roaming = data.roaming
	if data.get("always_on_top") is bool:
		always_on_top = data.always_on_top
	performance = PetRenderOptions.normalize(data.get("performance", {}))
	var p = data.get("position", [])
	if p is Array and p.size() == 2:
		if (p[0] is float or p[0] is int) and (p[1] is float or p[1] is int):
			position = Vector2i(int(p[0]), int(p[1]))

func save() -> Error:
	var file := FileAccess.open(path + ".tmp", FileAccess.WRITE)
	if file == null:
		return FileAccess.get_open_error()
	file.store_string(JSON.stringify({"skin_id": skin_id, "appearance_revision": APPEARANCE_REVISION, "zoom": zoom,
		"roaming": roaming, "always_on_top": always_on_top,
		"position": [position.x, position.y], "performance": PetRenderOptions.normalize(performance)}, "\t"))
	file.close()
	return DirAccess.rename_absolute(path + ".tmp", path)

static func normalize_zoom(value: float) -> float:
	if not is_finite(value):
		return 1.0
	return snappedf(clampf(value, MIN_ZOOM, MAX_ZOOM), 0.01)
