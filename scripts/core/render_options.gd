class_name PetRenderOptions
extends RefCounted

const RESOLUTIONS := [256, 384, 512, 768, 1024, 1536]
const FRAME_RATES := [24, 30, 60, 90, 120, 144, 0]
const ANTIALIASING := ["off", "fxaa", "msaa2", "msaa4", "msaa8"]
const DEFAULTS := {"resolution": 512, "fps": 60, "quality": 1, "aa": "msaa4", "ao": false, "vsync": true}
const PRESETS := {
	"power": {"resolution": 256, "fps": 30, "quality": 0, "aa": "fxaa", "ao": false, "vsync": true},
	"balanced": DEFAULTS,
	"high": {"resolution": 1024, "fps": 60, "quality": 2, "aa": "msaa4", "ao": true, "vsync": true}
}

static func normalize(value: Variant) -> Dictionary:
	var result := DEFAULTS.duplicate()
	if not value is Dictionary:
		return result
	for key in ["resolution", "fps", "quality"]:
		var allowed: Array = RESOLUTIONS if key == "resolution" else FRAME_RATES if key == "fps" else [0, 1, 2]
		if value.get(key) is int or value.get(key) is float:
			for candidate in allowed:
				if float(value[key]) == float(candidate):
					result[key] = candidate
	if value.get("aa") in ANTIALIASING:
		result.aa = value.aa
	for key in ["ao", "vsync"]:
		if value.get(key) is bool:
			result[key] = value[key]
	return result

static func apply_global(value: Dictionary) -> void:
	var options := normalize(value)
	Engine.max_fps = options.fps
	if DisplayServer.get_name() != "headless":
		DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_ENABLED if options.vsync else DisplayServer.VSYNC_DISABLED)

static func supports_ao() -> bool:
	return RenderingServer.get_current_rendering_method() == "forward_plus"
