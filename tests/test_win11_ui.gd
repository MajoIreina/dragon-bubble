extends SceneTree

var checks := 0
var failures := 0

func _initialize() -> void:
	call_deferred("run")

func check(condition: bool, message: String) -> void:
	checks += 1
	if not condition:
		failures += 1
		push_error(message)

func run() -> void:
	var app = load("res://scenes/main.tscn").instantiate()
	root.add_child(app)
	await process_frame
	app.set_process(false)
	app.set_physics_process(false)
	app._switch_skin("builtin.shu")
	app._show_settings()
	var ui = app.settings
	check(ui.pages.size() == 5, "All five settings categories remain available")
	for query in ["置顶", "FPS", "导入", "互动", "动画"]:
		ui.search.text = query
		ui.search.text_changed.emit(query)
		var expected: int = {"置顶": 2, "FPS": 4, "导入": 0, "互动": 1, "动画": 3}[query]
		check(ui.current_page == expected and ui.pages[expected].visible, "Settings search finds " + query)
	ui.search.text = "no-setting-matches-this"
	ui.search.text_changed.emit(ui.search.text)
	check(ui.empty_search.visible and ui.no_results.visible, "Unmatched search shows a clear empty state")
	check(ui.preview.process_mode == Node.PROCESS_MODE_DISABLED and ui.action_preview.process_mode == Node.PROCESS_MODE_DISABLED, "Empty search suspends both 3D previews")
	ui.search.text = ""
	ui.search.text_changed.emit("")
	check(ui.navigation.all(func(button: Button): return button.visible), "Clearing search restores every category")
	ui._select_page(0)
	ui._select_page(2)
	check(ui.page_motion and ui.page_motion.is_running() and ui.pages[2].modulate.a < 1.0, "Category navigation starts a real page fade and slide")
	await create_timer(0.06).timeout
	check(ui.pages[2].modulate.a > 0.0 and ui.pages[2].position.x > 0.0, "Page motion produces an intermediate visual state")
	ui._select_page(4)
	ui._select_page(0)
	ui._select_page(2)
	await create_timer(0.25).timeout
	check(is_equal_approx(ui.pages[2].modulate.a, 1.0) and is_zero_approx(ui.pages[2].position.x) and is_equal_approx(ui.navigation_marks[2].position.y, 12.0), "Rapid category changes finish cleanly at the final page and indicator")
	check(ui.pages.filter(func(page: ScrollContainer): return page.visible).size() == 1, "Interrupted animations leave exactly one active page")
	check(ui.import_button.has_meta("fluent_motion"), "Shared button hover and press animations are installed")
	ui.pages[2].scroll_vertical = 0
	app._set_on_top(true)
	await process_frame
	check(ui.top_toggle.is_visible_in_tree() and ui.top_toggle.get_global_rect().intersects(ui.pages[2].get_global_rect()), "Always-on-top control is visible at the top of Display settings")
	var cursor := DisplayServer.mouse_get_position()
	app._open_quick_menu(cursor)
	ui.top_toggle.button_pressed = false
	check(not root.always_on_top and not DisplayServer.window_get_flag(DisplayServer.WINDOW_FLAG_ALWAYS_ON_TOP), "Settings toggle disables the actual native window's topmost flag")
	check(not app.quick_menu.is_item_checked(app.quick_menu.get_item_index(app.quick_menu.Action.ON_TOP)), "An open quick menu synchronizes the disabled topmost state")
	var saved := PetPreferences.new()
	saved.load_saved()
	check(not saved.always_on_top, "Disabling topmost persists immediately")
	app.quick_menu._activate(app.quick_menu.Action.ON_TOP)
	check(root.always_on_top and ui.top_toggle.button_pressed and ui.top_toggle.text == "开", "Quick menu re-enables topmost and synchronizes the settings switch")
	app.quick_menu.hide()
	ui.top_toggle.button_pressed = false
	app._switch_skin("builtin.mint")
	check(not root.always_on_top and not app.prefs.always_on_top, "Changing appearances preserves disabled topmost mode")
	app._switch_skin("builtin.shu")
	# Verify restoration through the same startup path used by the executable.
	ui.hide()
	app.queue_free()
	await process_frame
	app = load("res://scenes/main.tscn").instantiate()
	root.add_child(app)
	await process_frame
	app.set_process(false)
	app.set_physics_process(false)
	check(not app.prefs.always_on_top and not root.always_on_top, "Restarting restores the saved disabled topmost mode")
	app._show_settings()
	ui = app.settings
	app._set_on_top(true)
	var names := ["appearance", "behavior", "display", "adaptation", "performance"]
	for i in ui.pages.size():
		ui._select_page(i)
		ui.pages[i].scroll_vertical = 0
		await create_timer(0.22).timeout
		var page: Control = ui.pages[i].get_child(0)
		check(page.size.x <= ui.pages[i].get_parent().size.x + 1 and ui.pages[i].size.x <= ui.pages[i].get_parent().size.x + 1, "Default window fits " + names[i] + " without horizontal overflow")
		await RenderingServer.frame_post_draw
		ui.get_texture().get_image().save_png("res://outputs/win11-" + names[i] + ".png")
	ui.size = ui.MINIMUM_SIZE
	for i in ui.pages.size():
		ui._select_page(i)
		await process_frame
		await process_frame
		var page: Control = ui.pages[i].get_child(0)
		check(page.size.x <= ui.pages[i].get_parent().size.x + 1 and ui.pages[i].size.x <= ui.pages[i].get_parent().size.x + 1, "Minimum window fits " + names[i] + " without horizontal overflow")
	await RenderingServer.frame_post_draw
	await create_timer(0.22).timeout
	ui.get_texture().get_image().save_png("res://outputs/win11-compact.png")
	ui.size = ui.DEFAULT_SIZE
	ui._select_page(2)
	await create_timer(0.22).timeout
	print("TOP SWITCH ", ui.top_toggle.get_global_rect())
	print("WIN11 UI TESTS: ", checks, " checks, ", failures, " failures")
	if "--hold-ui" in OS.get_cmdline_user_args():
		ui.title = "LPP · Win11 界面验收"
		await create_timer(60).timeout
	ui.hide()
	app.queue_free()
	await process_frame
	quit(1 if failures else 0)
