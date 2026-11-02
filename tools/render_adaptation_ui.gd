extends SceneTree

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	var app = load("res://scenes/main.tscn").instantiate()
	root.add_child(app)
	app._show_settings()
	app.settings._select_page(3)
	await create_timer(0.2).timeout
	await RenderingServer.frame_post_draw
	app.settings.get_texture().get_image().save_png("res://outputs/adaptation-settings.png")
	app.settings.pages[3].scroll_vertical = 350
	await process_frame
	await RenderingServer.frame_post_draw
	app.settings.get_texture().get_image().save_png("res://outputs/adaptation-settings-options.png")
	app.settings.hide()
	quit()
