class_name ActionAdapter
extends RefCounted

const STATES := ["idle", "walk", "sleep", "petted", "dragged", "airborne", "fall"]
const LABELS := {"idle": "待机", "walk": "行走", "sleep": "睡眠", "petted": "摸摸回应", "dragged": "提起", "airborne": "空中 / 下落", "fall": "落地"}
const ALIASES := {
	"idle": ["idle", "stand", "standing", "breathe", "breathing", "rest", "待机", "站立", "呼吸", "休息"],
	"walk": ["walk", "walking", "run", "running", "locomotion", "散步", "走路", "行走", "跑步"],
	"sleep": ["sleep", "sleeping", "nap", "doze", "睡觉", "睡眠", "打盹"],
	"petted": ["happy", "pet", "petted", "wave", "greet", "joy", "cheer", "开心", "摸摸", "挥手", "互动"],
	"dragged": ["pickedup", "pickup", "grab", "held", "drag", "dragged", "提起", "抓起", "拖拽"],
	"airborne": ["airborne", "falling", "float", "fly", "jump", "下落", "空中", "跳跃", "飞行"],
	"fall": ["land", "landing", "impact", "落地", "着地"]
}
const PROFILES := ["auto", "blob", "character", "flat"]
const PROFILE_LABELS := ["自动识别", "龙泡泡 / 软团", "Q 版人物", "图片 / 平面角色"]

static func words(value: String) -> PackedStringArray:
	var regex := RegEx.new()
	regex.compile("([a-z])([A-Z])")
	value = regex.sub(value, "$1 $2", true).to_lower()
	regex.compile("([a-z])([0-9])")
	value = regex.sub(value, "$1 $2", true)
	regex.compile("[^a-z0-9\u4e00-\u9fff]+")
	return regex.sub(value, " ", true).split(" ", false)

static func resolve(clips: PackedStringArray, explicit: Dictionary = {}) -> Dictionary:
	var result := {}
	var sorted := Array(clips)
	sorted.sort()
	for state in STATES:
		if explicit.has(state):
			# An empty explicit value intentionally chooses the generic module.
			if explicit[state] in clips and explicit[state] != "RESET":
				result[state] = explicit[state]
			continue
		var best := 0
		for clip: String in sorted:
			if clip == "RESET":
				continue
			var tokens := words(clip)
			var compact := "".join(tokens)
			for alias: String in ALIASES[state]:
				var score := 0
				if compact == alias:
					score = 100
				elif alias in tokens:
					score = 80
				elif alias.unicode_at(0) > 127 and compact.contains(alias):
					score = 70
				elif compact.ends_with(alias) and (clip.contains(":") or clip.contains("/") or clip.contains("|")):
					score = 60
				if score > best:
					best = score
					result[state] = clip
	return result

static func profile_for(kind: String, size: Vector3, bones: PackedStringArray, requested := "auto") -> String:
	if requested in PROFILES and requested != "auto":
		return requested
	if kind == "sprite":
		return "flat"
	var hints := 0
	for bone in bones:
		var normalized := "".join(words(bone))
		for hint in ["hips", "pelvis", "spine", "head", "upperarm", "thigh", "小腿", "上臂", "脊柱"]:
			if normalized.contains(hint):
				hints += 1
				break
	return "character" if hints >= 2 or size.y > maxf(size.x, size.z) * 1.25 else "blob"

static func source_for(state: String, mapping: Dictionary, explicit: Dictionary) -> String:
	if mapping.has(state):
		return "手动指定" if explicit.has(state) else "自动识别"
	return "通用动作"
