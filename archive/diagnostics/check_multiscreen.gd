extends SceneTree

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	root.size = Vector2i(100, 100)
	root.position = Vector2i(-2000, -2000)
	for i in DisplayServer.get_screen_count():
		print("DISPLAY ", i + 1, " work=", DisplayServer.screen_get_usable_rect(i), " dpi=", DisplayServer.screen_get_dpi(i))
	var catalog := SkinCatalog.new()
	catalog.scan()
	var settings = load("res://scripts/ui/settings_window.gd").new()
	root.add_child(settings)
	settings.setup(catalog, PetPreferences.new())
	var labels: Array[String] = ["显示器 1（主屏） · 2560 × 1440", "显示器 2 · 1920 × 1080"]
	settings.set_screens(labels)
	settings.sync_screen(1)
	assert(settings.screen_chooser.selected == 1)
	var result := [-1]
	settings.screen_requested.connect(func(index: int): result[0] = index)
	settings.screen_chooser.item_selected.emit(0)
	assert(result[0] == 0)
	var menu = load("res://scripts/ui/quick_menu.gd").new()
	root.add_child(menu)
	menu.setup(catalog, PetPreferences.new())
	menu.set_screens(labels)
	menu.screen_requested.connect(func(index: int): result[0] = index)
	menu.screens_menu.id_pressed.emit(1)
	assert(result[0] == 1)
	assert(menu.screens_menu.item_count == 2)
	settings._select_page(2)
	settings.popup_centered()
	await process_frame
	await RenderingServer.frame_post_draw
	settings.get_texture().get_image().save_png("res://work/multiscreen-settings.png")
	print("MULTISCREEN UI OK")
	quit()
