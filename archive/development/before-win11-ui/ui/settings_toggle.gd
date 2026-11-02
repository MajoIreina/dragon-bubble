extends Button

const ThemeMaker = preload("res://scripts/ui/settings_theme.gd")
var progress := 0.0
var transition: Tween

func _init() -> void:
	toggle_mode = true
	custom_minimum_size = Vector2(46, 32)
	mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
	for state in ["normal", "hover", "pressed", "hover_pressed", "focus"]:
		add_theme_stylebox_override(state, StyleBoxEmpty.new())

func _ready() -> void:
	progress = 1.0 if button_pressed else 0.0
	toggled.connect(func(enabled: bool):
		if transition:
			transition.kill()
		transition = create_tween()
		transition.tween_method(_set_progress, progress, 1.0 if enabled else 0.0, 0.15))
	mouse_entered.connect(queue_redraw)
	mouse_exited.connect(queue_redraw)
	focus_entered.connect(queue_redraw)
	focus_exited.connect(queue_redraw)

func _set_progress(value: float) -> void:
	progress = value
	queue_redraw()

func _draw() -> void:
	var track := Rect2(Vector2(2, (size.y - 24) / 2), Vector2(42, 24))
	var color := Color("d5dfd8").lerp(ThemeMaker.ACCENT, progress)
	if is_hovered():
		color = color.darkened(0.05)
	draw_style_box(ThemeMaker.panel(color, 12, 0), track)
	draw_circle(Vector2(track.position.x + 12 + progress * 18, size.y / 2), 9, Color.WHITE, true, -1, true)
	if has_focus():
		draw_style_box(ThemeMaker.panel(Color.TRANSPARENT, 15, 0, ThemeMaker.ACCENT), track.grow(3))
