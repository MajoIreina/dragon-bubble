extends SceneTree
const ScaleControl = preload("res://scripts/ui/scale_control.gd")
const ThemeMaker = preload("res://scripts/ui/settings_theme.gd")
func _initialize() -> void:
	call_deferred("run")
func run() -> void:
	root.size = Vector2i(340, 210)
	root.position = Vector2i(60, 60)
	root.transparent = false
	root.transparent_bg = false
	var panel := PanelContainer.new()
	panel.theme = ThemeMaker.create()
	panel.add_theme_stylebox_override("panel", ThemeMaker.panel(Color.WHITE, 12, 20, ThemeMaker.LINE))
	root.add_child(panel)
	panel.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var scale := ScaleControl.new()
	panel.add_child(scale)
	await process_frame
	await process_frame
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png("res://work/scale-preview.png")
	quit()
