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
	var app = load("res://scenes/main.tscn").instantiate()
	root.add_child(app)
	await process_frame
	app.set_process(false)
	app.set_physics_process(false)
	app._switch_skin("builtin.shu")
	app._show_settings()
	var ui = app.settings
	ui._select_page(3)
	ui.pages[3].scroll_vertical = 60
	app.brain.change("sleep", 20)
	app.motion.launch(Vector2(root.position), Vector2(123, -456))
	var avatar: PetAvatar = app.avatar
	var preview: PetAvatar = ui.preview
	var action_preview: PetAvatar = ui.action_preview
	var settings_position: Vector2i = ui.position
	var pet_position: Vector2i = root.position
	var original_options: Dictionary = app.prefs.performance.duplicate()
	var original_zoom: float = app.prefs.zoom
	var original_top: bool = app.prefs.always_on_top
	ui.message("导入失败：C:/Models/自定义の龍/skin.json: 不支持的外观资源格式")
	ui._select_page(3)
	await process_frame
	await process_frame
	ui.pages[3].scroll_vertical = 60
	var original_scroll: int = ui.pages[3].scroll_vertical
	for language in ["en", "ja", "zh_CN", "en", "ja"]:
		var index: int = PetI18n.LOCALES.find(language)
		ui.language_chooser.select(index)
		ui.language_chooser.item_selected.emit(index)
		await process_frame
		await process_frame
		check(app.prefs.locale == language and PetI18n.normalize(TranslationServer.get_locale()) == language, "Language picker applies locale: " + language)
		check(ui.current_page == 3 and ui.pages[3].visible, "Language change retains the selected page: " + language)
		check(ui.pages[3].scroll_vertical == original_scroll, "Language change retains page scroll: " + language)
		check(app.avatar == avatar and ui.preview == preview and ui.action_preview == action_preview, "Language change keeps the live pet and both preview instances: " + language)
		check(app.brain.state == "sleep" and app.motion.active and app.motion.velocity == Vector2(123, -456), "Language change preserves current animation and physics: " + language)
		check(root.position == pet_position and ui.position == settings_position, "Language change preserves native window positions: " + language)
		check(app.prefs.performance == original_options and app.prefs.zoom == original_zoom and app.prefs.always_on_top == original_top, "Other preferences remain intact: " + language)
		check(ui.title == PetI18n.window_title("设置") and root.title == PetI18n.APP_NAME, "Native titles display the product name and translated settings: " + language)
		check(ui.status.text == PetI18n.message(ui.status_source) and ui.status.text.contains("C:/Models/自定义の龍/skin.json"), "Visible import error re-translates without altering the path: " + language)
		check(ui.status.text.ends_with(PetI18n.text("不支持的外观资源格式")), "Import diagnostic detail translates: " + language)
		check(ui.top_toggle.accessibility_name == PetI18n.text("桌宠始终置顶"), "Accessible control name follows language: " + language)
		check(ui.top_toggle.tooltip_text.begins_with(PetI18n.text("桌宠始终置顶")), "Control tooltip follows language: " + language)
		check(ui.file_dialog.title == PetI18n.text("选择模型、图片或外观包") and ui.file_dialog.filters[0].contains(PetI18n.text("模型、图片或外观包")), "File picker title and filters translate: " + language)
		if app.tray.indicator:
			check(app.tray.indicator.tooltip == PetI18n.window_title("左键打开设置，右键快速设置"), "Tray tooltip follows language: " + language)
		var saved := PetPreferences.new()
		saved.load_saved()
		check(saved.locale == language, "Language choice is persisted immediately: " + language)
	app.motion.stop(Vector2(root.position))
	ui.status_panel.visible = false
	for language in PetI18n.LOCALES:
		app._set_locale(language)
		ui.size = ui.DEFAULT_SIZE
		for i in ui.pages.size():
			ui._select_page(i)
			ui.pages[i].scroll_vertical = 0
			await create_timer(0.23).timeout
			var content: Control = ui.pages[i].get_child(0)
			check(content.size.x <= ui.pages[i].size.x + 1 and ui.pages[i].size.x <= ui.pages[i].get_parent().size.x + 1, "Default width fits page %d in %s" % [i, language])
			await RenderingServer.frame_post_draw
			ui.get_texture().get_image().save_png("res://outputs/language-%s-page%d.png" % [language, i])
		ui.size = ui.MINIMUM_SIZE
		for i in ui.pages.size():
			ui._select_page(i)
			ui.pages[i].scroll_vertical = 0
			await create_timer(0.23).timeout
			var content: Control = ui.pages[i].get_child(0)
			check(content.size.x <= ui.pages[i].size.x + 1 and ui.pages[i].size.x <= ui.pages[i].get_parent().size.x + 1, "Minimum width fits page %d in %s" % [i, language])
			check(ui.language_chooser.get_global_rect().end.y < ui.size.y and ui.footer_label.get_global_rect().end.x < ui.size.x, "Language picker and footer remain inside compact window: " + language)
			await RenderingServer.frame_post_draw
			ui.get_texture().get_image().save_png("res://outputs/language-%s-compact%d.png" % [language, i])
		app._open_quick_menu(DisplayServer.mouse_get_position())
		await process_frame
		await process_frame
		var menu: PopupMenu = app.quick_menu
		check(menu.size.x >= 252, "Localized quick menu retains its minimum width: " + language)
		check(menu.get_item_text(menu.get_item_index(menu.Action.SIZE)) == PetI18n.text("桌宠大小 · %d%%…") % roundi(app.prefs.zoom * 100), "Formatted quick-menu size label translates: " + language)
		check(menu.skins_menu.get_item_text(0) == PetI18n.skin_name(app.catalog.skins[0]), "Quick-menu built-in appearance translates: " + language)
		await RenderingServer.frame_post_draw
		menu.get_texture().get_image().save_png("res://outputs/language-%s-menu.png" % language)
		check(ui.screen_chooser.get_item_text(0).begins_with(PetI18n.text("显示器 %d%s · %d × %d").get_slice("%", 0)), "Display labels translate: " + language)
		menu.hide()
		ui.performance_panel._refresh_status()
		check(ui.performance_panel.live.text.contains(PetI18n.text("高级渲染 · 支持环境光遮蔽") if PetRenderOptions.supports_ao() else PetI18n.text("兼容渲染")), "Live renderer status translates: " + language)
		for entry in [["always on top", 2], ["最前面", 2], ["パフォーマンス", 4], ["import", 0]]:
			ui.search.text = entry[0]
			ui.search.text_changed.emit(entry[0])
			check(ui.current_page == entry[1], "Search finds translated keyword: " + entry[0])
		ui.search.text = ""
		ui.search.text_changed.emit("")
	# Restart through the production startup path to verify the saved locale.
	app._set_locale("en")
	ui.hide()
	app.queue_free()
	await process_frame
	app = load("res://scenes/main.tscn").instantiate()
	root.add_child(app)
	await process_frame
	check(app.prefs.locale == "en" and TranslationServer.get_locale().begins_with("en") and app.settings.title == "My Dragon Bubble · Settings", "Restart restores English throughout the interface")
	app.queue_free()
	await process_frame
	print("LOCALIZATION UI TESTS: ", checks, " checks, ", failures, " failures")
	quit(1 if failures else 0)
