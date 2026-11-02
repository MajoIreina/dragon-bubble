class_name PetI18n
extends RefCounted

const APP_NAME := "My Dragon Bubble"
const LOCALES := ["zh_CN", "ja", "en"]
const LANGUAGE_NAMES := ["简体中文", "日本語", "English"]
static var message_keys: PackedStringArray = []
const FORMATTED_ERRORS := [
	["软体 %s 需要模型内部名称或相对节点路径", "^软体 (.+) 需要模型内部名称或相对节点路径$"],
	["软体缺少 %s", "^软体缺少 (.+)$"],
	["软体 %s 超出允许范围", "^软体 (.+) 超出允许范围$"],
	["动作 %s 的帧索引超出图片范围", "^动作 (.+) 的帧索引超出图片范围$"]
]

static func normalize(value: Variant) -> String:
	if not value is String:
		return "zh_CN"
	var language: String = value.replace("-", "_").get_slice("_", 0).to_lower()
	return language if language in ["en", "ja"] else "zh_CN"

static func apply(value: String) -> void:
	TranslationServer.set_locale(normalize(value))

static func text(source: String) -> String:
	return str(TranslationServer.translate(source))

static func window_title(source: String) -> String:
	return APP_NAME + " · " + text(source)

static func skin_name(skin: Dictionary) -> String:
	return text(skin.name) if skin.get("id", "") in ["builtin.shu", "builtin.mochi", "builtin.mint", "example.glb", "example.sprite"] else str(skin.get("name", ""))

# Import diagnostics keep their original source until displayed, so switching
# language can also translate a visible error while preserving paths and IDs.
static func message(source: String, depth := 0) -> String:
	if depth > 8 or normalize(TranslationServer.get_locale()) == "zh_CN":
		return source
	var translated := text(source)
	if translated != source:
		return translated
	for separator in ["\n", "; "]:
		if source.contains(separator):
			var parts := source.split(separator)
			for i in parts.size():
				parts[i] = message(parts[i], depth + 1)
			return separator.join(parts)
	for entry in FORMATTED_ERRORS:
		var expression := RegEx.new()
		expression.compile(entry[1])
		var match_result := expression.search(source)
		if match_result:
			return text(entry[0]) % match_result.get_string(1)
	if message_keys.is_empty():
		var translation := load("res://i18n/en.po") as Translation
		if translation:
			var keys := Array(translation.get_message_list())
			keys.sort_custom(func(a: String, b: String): return a.length() > b.length())
			message_keys = PackedStringArray(keys)
	for key in message_keys:
		if key.length() < 4 or key.contains("%"):
			continue
		if source.begins_with(key):
			return text(key) + message(source.substr(key.length()), depth + 1)
		if source.ends_with(key):
			return source.left(source.length() - key.length()) + text(key)
	var colon := source.find(": ")
	if colon >= 0:
		return source.left(colon + 2) + message(source.substr(colon + 2), depth + 1)
	return source
