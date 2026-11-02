extends SceneTree
func _initialize() -> void:
	call_deferred("run")
func run() -> void:
	var app = load("res://scenes/main.tscn").instantiate()
	root.add_child(app)
	await process_frame
	var original := root.position
	for i in app.desktop.areas.size():
		app._move_to_screen(i)
		await create_timer(0.06).timeout
		assert(app.desktop.current(Vector2(root.position)) == i)
		var area: Rect2 = app.desktop.areas[i]
		assert(area.has_point(Vector2(root.position) + Vector2(root.size.x * 0.5, app.desktop.extent.y - 1)))
		app._show_settings()
		await process_frame
		assert(app.settings.current_screen == i)
		assert(area.has_point(Vector2(app.settings.position) + Vector2(2, 2)))
		app.settings.hide()
		print("REAL SCREEN ", i + 1, " OK pet=", root.position)
	root.position = original
	app.motion.stop(Vector2(original))
	app._save()
	print("REAL MULTISCREEN INTEGRATION OK")
	quit()
