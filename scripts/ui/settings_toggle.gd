extends Button

const ThemeMaker = preload("res://scripts/ui/settings_theme.gd")
var progress := 0.0
var transition: Tween

func _init() -> void:
	toggle_mode = true
	custom_minimum_size = Vector2(80, 32)
	alignment = HORIZONTAL_ALIGNMENT_LEFT
	mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
	for state in ["normal", "hover", "pressed", "hover_pressed", "focus", "disabled"]:
		add_theme_stylebox_override(state, StyleBoxEmpty.new())

func _ready() -> void:
	progress = 1.0 if button_pressed else 0.0
	text = "开" if button_pressed else "关"
	toggled.connect(func(enabled: bool):
		text = "开" if enabled else "关"
		if transition:
			transition.kill()
		transition = create_tween()
		transition.tween_method(_set_progress, progress, 1.0 if enabled else 0.0, 0.15))
	mouse_entered.connect(queue_redraw)
	mouse_exited.connect(queue_redraw)
	focus_entered.connect(queue_redraw)
	focus_exited.connect(queue_redraw)

func sync(enabled: bool) -> void:
	text = "开" if enabled else "关"
	if button_pressed == enabled:
		return
	set_pressed_no_signal(enabled)
	if transition:
		transition.kill()
	_set_progress(1.0 if enabled else 0.0)

func _set_progress(value: float) -> void:
	progress = value
	queue_redraw()

func _draw() -> void:
	var track := Rect2(Vector2(size.x - 42, (size.y - 20) / 2), Vector2(40, 20))
	var color := Color("fafafa").lerp(ThemeMaker.ACCENT, progress)
	if is_hovered():
		color = color.darkened(0.04)
	var outline := Color("8a8a8a").lerp(ThemeMaker.ACCENT, progress)
	var thumb := ThemeMaker.MUTED.lerp(Color.WHITE, progress)
	if disabled:
		color = Color("f0f0f0")
		outline = Color("c5c5c5")
		thumb = Color("b0b0b0")
	draw_style_box(ThemeMaker.panel(color, 10, 0, outline), track)
	draw_circle(Vector2(track.position.x + 10 + progress * 20, size.y / 2), 6, thumb, true, -1, true)
	if has_focus():
		draw_style_box(ThemeMaker.panel(Color.TRANSPARENT, 4, 0, ThemeMaker.INK), Rect2(Vector2.ZERO, size).grow(2))
