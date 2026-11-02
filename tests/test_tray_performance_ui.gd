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
	app.settings._select_page(4)
	await create_timer(0.2).timeout
	check(app.settings.pages.size() == 5 and app.settings.current_page == 4, "Dedicated performance page opens")
	var panel = app.settings.performance_panel
	var resolution: OptionButton = panel.choices.resolution
	for i in resolution.item_count:
		if resolution.get_item_metadata(i) == 768:
			resolution.item_selected.emit(i)
	check(app.avatar.viewport.size == Vector2i(768, 768) and app.settings.preview.viewport.size == Vector2i(768, 768), "Changing resolution updates the pet and both preview renderers")
	var aa: OptionButton = panel.choices.aa
	for i in aa.item_count:
		if aa.get_item_metadata(i) == "msaa8":
			aa.item_selected.emit(i)
	check(app.avatar.viewport.msaa_3d == Viewport.MSAA_8X, "AA selector updates the real viewport")
	panel.vsync.toggled.emit(false)
	var fps: OptionButton = panel.choices.fps
	for i in fps.item_count:
		if fps.get_item_metadata(i) == 120:
			fps.item_selected.emit(i)
	check(Engine.max_fps == 120 and DisplayServer.window_get_vsync_mode() == DisplayServer.VSYNC_DISABLED, "FPS and VSync controls apply globally")
	panel.preset.item_selected.emit(0)
	check(app.prefs.performance == PetRenderOptions.PRESETS.power and panel.preset.selected == 0, "Power preset applies all its values")
	panel.preset.item_selected.emit(2)
	check(app.avatar.key_light.shadow_enabled and panel.ao.button_pressed, "High preset enables shadows and ambient occlusion")
	var saved := PetPreferences.new()
	saved.load_saved()
	check(saved.performance == app.prefs.performance, "UI changes are saved immediately")
	check(DisplayServer.has_feature(DisplayServer.FEATURE_STATUS_INDICATOR) and app.tray.indicator != null and app.tray.indicator.visible, "Native Windows tray indicator is registered")
	print("TRAY RECT ", app.tray.indicator.get_rect())
	var tray_rect: Rect2 = app.tray.indicator.get_rect()
	var cursor := Vector2i(tray_rect.get_center()) if tray_rect.has_area() else DisplayServer.mouse_get_position()
	app.settings.hide()
	app.tray._pressed(MOUSE_BUTTON_RIGHT, cursor)
	await process_frame
	check(app.quick_menu.visible and app.quick_menu.get_item_index(app.quick_menu.Action.PERFORMANCE) >= 0, "Tray right click opens quick settings, including performance")
	var work_area: Rect2 = app.desktop.areas[app.desktop.screen_at(cursor)]
	print("MENU RECT ", Rect2(Vector2(app.quick_menu.position), Vector2(app.quick_menu.size)), " WORK AREA ", work_area)
	check(work_area.encloses(Rect2(Vector2(app.quick_menu.position), Vector2(app.quick_menu.size))), "Tray menu stays inside the taskbar screen's work area")
	await RenderingServer.frame_post_draw
	app.quick_menu.get_texture().get_image().save_png("res://outputs/tray-quick-settings.png")
	var pet_position: Vector2i = root.position
	app.quick_menu.hide()
	app.quick_menu._open_scale()
	await process_frame
	check(work_area.encloses(Rect2(Vector2(app.quick_menu.size_popup.position), Vector2(app.quick_menu.size_popup.size))), "Tray size control stays inside the taskbar screen's work area")
	check(root.position == pet_position, "Opening tray controls does not move the pet between screens")
	app.quick_menu.size_popup.hide()
	app.tray._pressed(MOUSE_BUTTON_RIGHT, cursor)
	app.quick_menu._activate(app.quick_menu.Action.PERFORMANCE)
	await process_frame
	check(app.settings.visible and app.settings.current_page == 4 and not app.quick_menu.visible, "Tray performance entry opens the dedicated page")
	app.settings.hide()
	app.tray._pressed(MOUSE_BUTTON_LEFT, cursor)
	check(app.settings.visible, "Tray left click opens full settings")
	app.settings._select_page(4)
	# Let the first-frame shader work and FPS counter settle for the preview.
	await create_timer(1.2).timeout
	panel._refresh_status()
	await RenderingServer.frame_post_draw
	app.settings.get_texture().get_image().save_png("res://outputs/performance-settings.png")
	app.settings.pages[4].scroll_vertical = 580
	await process_frame
	await RenderingServer.frame_post_draw
	app.settings.get_texture().get_image().save_png("res://outputs/performance-detail-settings.png")
	# Verify pixels rather than just the SSAO property. Keep the same pose,
	# camera, lighting, resolution and AA in the before/after images.
	app.avatar.set_process(false)
	app.avatar.player.stop()
	app.avatar._restore_pose()
	for body in app.avatar.soft_bodies:
		body.active = false
	var without: Image
	for enabled in [false, true]:
		app.avatar.apply_performance({"resolution": 512, "ao": enabled, "quality": 1, "aa": "msaa4"})
		await process_frame
		await RenderingServer.frame_post_draw
		await process_frame
		await RenderingServer.frame_post_draw
		var frame: Image = app.avatar.viewport.get_texture().get_image()
		frame.save_png("res://outputs/shu-ao-" + ("on" if enabled else "off") + ".png")
		if not enabled:
			without = frame
		else:
			var difference := 0.0
			var pixels := 0
			for y in range(0, 512, 2):
				for x in range(0, 512, 2):
					var a := without.get_pixel(x, y)
					var b := frame.get_pixel(x, y)
					if a.a > 0.95 and b.a > 0.95:
						difference += absf(a.r - b.r) + absf(a.g - b.g) + absf(a.b - b.b)
						pixels += 1
			print("AO pixel difference=", difference / maxf(pixels, 1), " pixels=", pixels)
			check(PetRenderOptions.supports_ao() and difference / maxf(pixels, 1) > 0.001, "SSAO produces a measurable contact shading change in actual rendered pixels")
	if "--hold-ui" in OS.get_cmdline_user_args():
		app.tray.indicator.tooltip = "LPP 新功能验收 · 右键快速设置"
		await create_timer(45).timeout
	app.settings.hide()
	app.queue_free()
	await process_frame
	PetRenderOptions.apply_global(PetRenderOptions.DEFAULTS)
	print("TRAY/PERFORMANCE UI TESTS: ", checks, " checks, ", failures, " failures")
	quit(1 if failures else 0)
