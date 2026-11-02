extends VBoxContainer

signal zoom_changed(value: float)
const ThemeMaker = preload("res://scripts/ui/settings_theme.gd")
var slider := HSlider.new()
var percentage := SpinBox.new()

func _init() -> void:
	add_theme_constant_override("separation", 12)
	var header := HBoxContainer.new()
	add_child(header)
	var title := Label.new()
	title.text = "桌宠大小"
	title.add_theme_font_size_override("font_size", 16)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title)
	percentage.name = "ScalePercent"
	percentage.min_value = PetPreferences.MIN_ZOOM * 100
	percentage.max_value = PetPreferences.MAX_ZOOM * 100
	percentage.step = 1
	percentage.value = 100
	percentage.suffix = "%"
	percentage.select_all_on_focus = true
	percentage.custom_minimum_size = Vector2(110, 40)
	percentage.tooltip_text = "输入百分比后按 Enter，或点击其他位置应用"
	header.add_child(percentage)
	percentage.value_changed.connect(_percentage_changed)
	slider.name = "ScaleSlider"
	slider.min_value = percentage.min_value
	slider.max_value = percentage.max_value
	slider.step = 1
	slider.value = 100
	slider.custom_minimum_size.y = 30
	add_child(slider)
	slider.value_changed.connect(_slider_changed)
	var footer := HBoxContainer.new()
	add_child(footer)
	var minimum := Label.new()
	minimum.text = "%d%%" % roundi(percentage.min_value)
	minimum.add_theme_color_override("font_color", ThemeMaker.MUTED)
	minimum.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	footer.add_child(minimum)
	var reset := Button.new()
	reset.text = "恢复 100%"
	reset.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
	reset.pressed.connect(func(): _slider_changed(100))
	footer.add_child(reset)
	var maximum := Label.new()
	maximum.text = "%d%%" % roundi(percentage.max_value)
	maximum.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	maximum.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	maximum.add_theme_color_override("font_color", ThemeMaker.MUTED)
	footer.add_child(maximum)

func sync(value: float) -> void:
	var percent := roundi(PetPreferences.normalize_zoom(value) * 100)
	slider.set_value_no_signal(percent)
	percentage.set_value_no_signal(percent)

func commit() -> void:
	percentage.apply()

func _slider_changed(value: float) -> void:
	sync(value / 100.0)
	zoom_changed.emit(slider.value / 100.0)

func _percentage_changed(value: float) -> void:
	slider.set_value_no_signal(value)
	zoom_changed.emit(value / 100.0)
