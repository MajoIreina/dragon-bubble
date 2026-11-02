extends VBoxContainer

signal changed(options: Dictionary)
const ThemeMaker = preload("res://scripts/ui/settings_theme.gd")
const Toggle = preload("res://scripts/ui/settings_toggle.gd")
var prefs: PetPreferences
var choices: Dictionary = {}
var ao := Toggle.new()
var vsync := Toggle.new()
var preset := OptionButton.new()
var live := Label.new()
var monitor_time := 0.0

func setup(preferences: PetPreferences) -> void:
	prefs = preferences
	add_theme_constant_override("separation", 12)
	var profiles := _card("一键方案")
	_label(profiles, "根据设备和使用场景选择，也可逐项调整。", 12, ThemeMaker.MUTED)
	profiles.add_child(preset)
	preset.name = "PerformancePreset"
	for item in [["power", "省电 · 30 FPS / 低画质"], ["balanced", "均衡 · 60 FPS / 标准画质"], ["high", "高画质 · 60 FPS / 精细阴影"], ["custom", "自定义"]]:
		preset.add_item(item[1])
		preset.set_item_metadata(preset.item_count - 1, item[0])
	preset.item_selected.connect(func(index: int):
		var key: String = preset.get_item_metadata(index)
		if PetRenderOptions.PRESETS.has(key):
			prefs.performance = PetRenderOptions.PRESETS[key].duplicate()
			changed.emit(prefs.performance)
			sync())
	var rendering := _card("渲染与帧率")
	_option(rendering, "内部渲染分辨率", "resolution", PetRenderOptions.RESOLUTIONS, ["256 × 256", "384 × 384", "512 × 512", "768 × 768", "1024 × 1024", "1536 × 1536"])
	_label(rendering, "提高分辨率可改善大尺寸桌宠的细节；不会改变桌宠大小或脚底位置。", 12, ThemeMaker.MUTED)
	_option(rendering, "帧率上限", "fps", PetRenderOptions.FRAME_RATES, ["24 FPS", "30 FPS", "60 FPS", "90 FPS", "120 FPS", "144 FPS", "不限帧率"])
	vsync.name = "PerformanceVsync"
	_switch(rendering, "垂直同步", vsync)
	vsync.toggled.connect(func(value: bool): _commit("vsync", value))
	_label(rendering, "垂直同步开启时，实际帧率还受显示器刷新率限制。", 12, ThemeMaker.MUTED)
	var detail := _card("画面细节")
	_option(detail, "画质", "quality", [0, 1, 2], ["低 · 简化短绒", "标准 · 完整短绒", "高 · 短绒与柔和阴影"])
	_option(detail, "抗锯齿", "aa", PetRenderOptions.ANTIALIASING, ["关闭", "FXAA · 较低开销", "MSAA 2×", "MSAA 4×", "MSAA 8×"])
	ao.name = "PerformanceAO"
	ao.disabled = not PetRenderOptions.supports_ao()
	_switch(detail, "环境光遮蔽（SSAO）", ao)
	ao.toggled.connect(func(value: bool): _commit("ao", value))
	_label(detail, "增强脚部、背鳍根部及褶皱附近的接触层次，会增加 GPU 开销。" if PetRenderOptions.supports_ao() else "当前使用兼容渲染，环境光遮蔽不可用。Windows 高级渲染支持此效果。", 12, ThemeMaker.MUTED)
	var status := _card("运行状态")
	live.add_theme_color_override("font_color", ThemeMaker.ACCENT)
	live.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	status.add_child(live)
	_label(status, "所有选项即时应用并自动保存；外观预览共用相同画质设置。", 12, ThemeMaker.MUTED)
	sync()

func _option(parent: Node, title: String, key: String, values: Array, labels: Array) -> void:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 16)
	parent.add_child(row)
	var label := _label(row, title, 14)
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var choose := OptionButton.new()
	choose.name = "Performance_" + key
	choose.custom_minimum_size = Vector2(200, 38)
	choose.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	choose.fit_to_longest_item = false
	choose.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	for i in values.size():
		choose.add_item(labels[i])
		choose.set_item_metadata(i, values[i])
	choose.item_selected.connect(func(index: int): _commit(key, choose.get_item_metadata(index)))
	row.add_child(choose)
	choices[key] = choose

func _commit(key: String, value: Variant) -> void:
	prefs.performance[key] = value
	prefs.performance = PetRenderOptions.normalize(prefs.performance)
	changed.emit(prefs.performance)
	sync()

func sync() -> void:
	for key in choices:
		var choice: OptionButton = choices[key]
		for i in choice.item_count:
			if choice.get_item_metadata(i) == prefs.performance[key]:
				choice.select(i)
	ao.sync(prefs.performance.ao)
	vsync.sync(prefs.performance.vsync)
	preset.select(3)
	for key in PetRenderOptions.PRESETS:
		if prefs.performance == PetRenderOptions.PRESETS[key]:
			for i in preset.item_count:
				if preset.get_item_metadata(i) == key:
					preset.select(i)
	_refresh_status()

func _process(delta: float) -> void:
	monitor_time += delta
	if monitor_time >= 0.5 and is_visible_in_tree() and prefs:
		monitor_time = 0.0
		_refresh_status()

func _refresh_status() -> void:
	live.text = tr("当前 %d FPS · 渲染 %d × %d\n%s") % [Engine.get_frames_per_second(), prefs.performance.resolution, prefs.performance.resolution, tr("高级渲染 · 支持环境光遮蔽") if PetRenderOptions.supports_ao() else tr("兼容渲染")]

func _switch(parent: Node, title: String, toggle: Button) -> void:
	var row := HBoxContainer.new()
	parent.add_child(row)
	var label := _label(row, title, 14)
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	toggle.accessibility_name = tr(title)
	toggle.set_meta("label_source", title)
	toggle.tooltip_text = title
	row.add_child(toggle)

func _card(title: String) -> VBoxContainer:
	var panel := PanelContainer.new()
	panel.add_theme_stylebox_override("panel", ThemeMaker.panel(ThemeMaker.SURFACE, 10, 20, ThemeMaker.LINE))
	add_child(panel)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 14)
	panel.add_child(column)
	_label(column, title, 16)
	return column

func _label(parent: Node, text: String, size: int, color := ThemeMaker.INK) -> Label:
	var label := Label.new()
	label.text = text
	label.add_theme_font_size_override("font_size", size)
	if size >= 16:
		label.add_theme_font_override("font", ThemeMaker.font(600))
	label.add_theme_color_override("font_color", color)
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	parent.add_child(label)
	return label
