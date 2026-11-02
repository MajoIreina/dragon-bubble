extends SceneTree

var checks := 0
var failures := 0

func _initialize() -> void:
	call_deferred("run")

func check(condition: bool, description: String) -> void:
	checks += 1
	if not condition:
		failures += 1
		push_error(description)

func run() -> void:
	for entry in [["zh-TW", "zh_CN"], ["ja_JP", "ja"], ["en-GB", "en"], ["EN_us", "en"], ["fr", "zh_CN"], [null, "zh_CN"], [42, "zh_CN"]]:
		check(PetI18n.normalize(entry[0]) == entry[1], "Normalize locale " + str(entry[0]))
	check(ProjectSettings.get_setting("application/config/name") == "My Dragon Bubble", "Application has the new product name")
	check(OS.get_user_data_dir().ends_with("LPPDesktopPet"), "Existing user data directory is retained")
	var placeholders := RegEx.new()
	placeholders.compile("%(?:%|[dsf])")
	for language in ["en", "ja"]:
		PetI18n.apply(language)
		var catalog := load("res://i18n/" + language + ".po") as Translation
		check(catalog != null and catalog.get_message_count() >= 275, "Complete catalog loads: " + language)
		if catalog == null:
			continue
		for source in catalog.get_message_list():
			var translated := PetI18n.text(source)
			var source_tokens: Array[String] = []
			var translated_tokens: Array[String] = []
			for match_result in placeholders.search_all(source):
				source_tokens.append(match_result.get_string())
			for match_result in placeholders.search_all(translated):
				translated_tokens.append(match_result.get_string())
			check(not translated.is_empty() and translated == str(catalog.get_message(source)) and source_tokens == translated_tokens, language + " translation and placeholders: " + source)
		var path := "C:/Models/自定义の龍/skin.json"
		var diagnostic := "导入失败：" + path + ": 软体 Belly 需要模型内部名称或相对节点路径"
		var localized := PetI18n.message(diagnostic)
		check(localized.contains(path) and localized.contains("Belly") and not localized.contains("导入失败") and not localized.contains("需要模型"), "Nested diagnostic translates and preserves path and node name: " + language)
		check(PetI18n.message("schema_version 必须为 1").contains("schema_version"), "Diagnostic preserves schema field names")
		check(PetI18n.skin_name({"id": "custom.test", "name": "设置"}) == "设置", "User appearance names stay verbatim")
		check(PetI18n.skin_name({"id": "builtin.shu", "name": "黍泡泡"}) != "黍泡泡", "Built-in appearance name translates")
		var prefs := PetPreferences.new()
		prefs.path = "user://localization-test-" + str(OS.get_process_id()) + ".json"
		prefs.locale = language
		prefs.skin_id = "custom.test"
		prefs.zoom = 0.8
		prefs.always_on_top = false
		prefs.position = Vector2i(-550, 240)
		check(prefs.save() == OK, "Locale can be saved: " + language)
		var restored := PetPreferences.new()
		restored.path = prefs.path
		restored.load_saved()
		check(restored.locale == language and restored.skin_id == prefs.skin_id and is_equal_approx(restored.zoom, 0.8) and not restored.always_on_top and restored.position == prefs.position, "Language and existing preferences round-trip: " + language)
		var legacy := FileAccess.open(prefs.path, FileAccess.WRITE)
		legacy.store_string('{"skin_id":"custom.legacy","zoom":0.8}')
		legacy.close()
		restored = PetPreferences.new()
		restored.path = prefs.path
		restored.load_saved()
		check(restored.locale == "zh_CN" and restored.skin_id == "custom.legacy", "Older configuration defaults to Chinese without losing appearance")
		DirAccess.remove_absolute(prefs.path)
	PetI18n.apply("zh_CN")
	check(PetI18n.text("设置") == "设置" and PetI18n.message("导入失败：模型不存在") == "导入失败：模型不存在", "Chinese falls back to original source")
	print("LOCALIZATION TESTS: ", checks, " checks, ", failures, " failures")
	quit(1 if failures else 0)
