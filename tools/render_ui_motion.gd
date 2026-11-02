extends SceneTree

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	var app = load("res://scenes/main.tscn").instantiate()
	root.add_child(app)
	await process_frame
	app.set_process(false)
	app.set_physics_process(false)
	app._switch_skin("builtin.shu")
	app._show_settings()
	app._set_on_top(true)
	var ui = app.settings
	ui._select_page(0)
	await create_timer(0.3).timeout
	DirAccess.make_dir_recursive_absolute("res://work/win11-motion")
	for frame in 80:
		match frame:
			12: ui._select_page(2)
			26: ui.top_toggle.button_pressed = false
			38: ui.top_toggle.button_pressed = true
			50: ui._select_page(4)
			64: ui._select_page(0)
			70: ui.import_button.mouse_entered.emit()
			76: ui.import_button.mouse_exited.emit()
		# Run with --fixed-fps 24, so PNG encoding cannot alter motion timing.
		await RenderingServer.frame_post_draw
		ui.get_texture().get_image().save_png("res://work/win11-motion/%03d.png" % frame)
	ui.hide()
	app.queue_free()
	await process_frame
	print("WIN11 MOTION: captured 80 frames")
	quit()
