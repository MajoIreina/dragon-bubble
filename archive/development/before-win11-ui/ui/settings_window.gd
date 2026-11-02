extends Window

signal skin_selected(id: String)
signal import_requested(path: String)
signal zoom_changed(value: float)
signal roaming_changed(enabled: bool)
signal top_changed(enabled: bool)
signal reset_requested
signal quit_requested
signal screen_requested(index: int)
signal adaptation_changed
signal performance_changed(options: Dictionary)
var screen_chooser := OptionButton.new()

const ThemeMaker = preload("res://scripts/ui/settings_theme.gd")
const Toggle = preload("res://scripts/ui/settings_toggle.gd")
const ScaleControl = preload("res://scripts/ui/scale_control.gd")
const ModelAvatar = preload("res://scripts/avatars/model_avatar.gd")
const SpriteAvatar = preload("res://scripts/avatars/sprite_avatar.gd")
const PerformancePanel = preload("res://scripts/ui/performance_panel.gd")
const SECTIONS := [
	["外观与模型", "选择你的桌面伙伴，或带来新的模样。", "skin"],
	["行为与互动", "调整小伙伴的活动习惯，了解互动方式。", "behavior"],
	["桌面与显示", "让桌宠以合适的大小，待在合适的位置。", "display"],
	["动作与适配", "识别角色能力，让每个动作都有合适的表现。", "behavior"],
	["性能与画质", "调整清晰度、帧率与画面细节。", "display"]
]

var chooser := OptionButton.new()
var status := Label.new()
var file_dialog := FileDialog.new()
var state_label := Label.new()
var catalog: SkinCatalog
var prefs: PetPreferences
var pages: Array[ScrollContainer] = []
var navigation: Array[Button] = []
var current_page := 0
var page_title := Label.new()
var page_description := Label.new()
var preview_host := Control.new()
var preview: PetAvatar
var preview_id := ""
var preview_name := Label.new()
var preview_type := Label.new()
var pack_count := Label.new()
var status_panel := PanelContainer.new()
var root_control := Control.new()
var scale_control: VBoxContainer
var roaming_toggle: Button
var top_toggle: Button
var import_button: Button
var adaptation_host := VBoxContainer.new()
var action_preview_host := Control.new()
var action_preview: PetAvatar
var converter_dialog := FileDialog.new()
var converter_path := LineEdit.new()
var adaptation_summary := Label.new()
var performance_panel: VBoxContainer

func setup(source: SkinCatalog, preferences: PetPreferences) -> void:
	catalog = source
	prefs = preferences
	title = "LPP · 桌宠设置"
	size = Vector2i(960, 720)
	min_size = Vector2i(820, 620)
	var available := DisplayServer.screen_get_usable_rect(DisplayServer.get_primary_screen()).size
	if available.x > 0 and available.y > 0:
		size = Vector2i(clampi(available.x - 80, 820, 960), clampi(available.y - 80, 620, 720))
	transparent = false
	transparent_bg = false
	borderless = false
	transient = true
	visible = false
	close_requested.connect(func():
		if scale_control:
			scale_control.commit()
		hide())
	root_control.theme = ThemeMaker.create()
	add_child(root_control)
	root_control.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var background := ColorRect.new()
	background.color = ThemeMaker.BACKGROUND
	background.mouse_filter = Control.MOUSE_FILTER_IGNORE
	root_control.add_child(background)
	background.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var layout := HBoxContainer.new()
	layout.add_theme_constant_override("separation", 0)
	root_control.add_child(layout)
	layout.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_build_sidebar(layout)
	_build_content(layout)
	_build_dialog()
	visibility_changed.connect(_sync_preview_activity)
	refresh()
	_select_page(0)

func _build_sidebar(parent: Control) -> void:
	var panel := PanelContainer.new()
	panel.custom_minimum_size.x = 212
	panel.add_theme_stylebox_override("panel", ThemeMaker.panel(Color.WHITE, 0, 22))
	parent.add_child(panel)
	var column := _column(panel, 8)
	var brand := HBoxContainer.new()
	brand.add_theme_constant_override("separation", 12)
	column.add_child(brand)
	var monogram := PanelContainer.new()
	monogram.custom_minimum_size = Vector2(40, 40)
	monogram.add_theme_stylebox_override("panel", ThemeMaker.panel(ThemeMaker.ACCENT, 12, 0))
	brand.add_child(monogram)
	var mark := _label("L", 24, Color.WHITE)
	mark.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	monogram.add_child(mark)
	var brand_text := _column(brand, 0)
	brand_text.add_child(_label("LPP", 21))
	brand_text.add_child(_label("你的桌面小伙伴", 11, ThemeMaker.MUTED))
	_space(column, 34)
	column.add_child(_label("偏好设置", 12, ThemeMaker.MUTED))
	_space(column, 8)
	for index in SECTIONS.size():
		var button := Button.new()
		button.text = SECTIONS[index][0]
		button.icon = ThemeMaker.icon(SECTIONS[index][2], ThemeMaker.MUTED)
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.custom_minimum_size.y = 48
		button.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
		button.pressed.connect(_select_page.bind(index))
		column.add_child(button)
		navigation.append(button)
	var stretch := Control.new()
	stretch.size_flags_vertical = Control.SIZE_EXPAND_FILL
	column.add_child(stretch)
	var note := _label("轻点回应，右键快捷菜单。\n陪伴就在桌面一角。", 12, ThemeMaker.MUTED)
	note.add_theme_constant_override("line_spacing", 6)
	column.add_child(note)
	_space(column, 16)
	var exit_button := _button(column, "退出桌宠", func(): quit_requested.emit(), "exit")
	exit_button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	exit_button.add_theme_stylebox_override("normal", ThemeMaker.panel(Color.TRANSPARENT, 9, 12))

func _build_content(parent: Control) -> void:
	var margin := MarginContainer.new()
	margin.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	for edge in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + edge, 30)
	parent.add_child(margin)
	var column := _column(margin, 18)
	var top := HBoxContainer.new()
	column.add_child(top)
	var crumb := _label("设 置", 12, ThemeMaker.MUTED)
	crumb.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	top.add_child(crumb)
	top.add_child(_label("●  自动保存", 12, ThemeMaker.ACCENT))
	var heading := _column(column, 8)
	page_title.add_theme_font_size_override("font_size", 27)
	heading.add_child(page_title)
	page_description.add_theme_color_override("font_color", ThemeMaker.MUTED)
	page_description.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	heading.add_child(page_description)
	var body := Control.new()
	body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	column.add_child(body)
	for i in SECTIONS.size():
		var scroll := ScrollContainer.new()
		scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
		body.add_child(scroll)
		scroll.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		pages.append(scroll)
		var page := _column(scroll, 16)
		page.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		match i:
			0: _build_appearance(page)
			1: _build_behavior(page)
			2: _build_display(page)
			3: _build_adaptation(page)
			4:
				performance_panel = PerformancePanel.new()
				page.add_child(performance_panel)
				performance_panel.setup(prefs)
				performance_panel.changed.connect(func(options: Dictionary): performance_changed.emit(options))
	column.add_child(_label("修改即时生效，无需另外保存。", 12, ThemeMaker.MUTED))

func _build_appearance(page: VBoxContainer) -> void:
	var hero := _card(page)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 22)
	hero.add_child(row)
	var well := PanelContainer.new()
	well.custom_minimum_size = Vector2(164, 164)
	well.add_theme_stylebox_override("panel", ThemeMaker.panel(Color("f0f5ef"), 14, 0))
	row.add_child(well)
	preview_host.custom_minimum_size = Vector2(164, 164)
	preview_host.clip_contents = true
	preview_host.mouse_filter = Control.MOUSE_FILTER_IGNORE
	well.add_child(preview_host)
	var details := _column(row, 10)
	details.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	details.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	details.add_child(_label("当前伙伴", 12, ThemeMaker.MUTED))
	preview_name.add_theme_font_size_override("font_size", 21)
	preview_name.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	details.add_child(preview_name)
	preview_type.add_theme_color_override("font_color", ThemeMaker.ACCENT)
	preview_type.add_theme_font_size_override("font_size", 12)
	details.add_child(preview_type)
	details.add_child(_wrapped("换一个模样，继续陪你。", 13, ThemeMaker.MUTED))
	var selection := _card(page)
	var heading := HBoxContainer.new()
	selection.add_child(heading)
	var title_label := _label("选择外观", 16)
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	heading.add_child(title_label)
	pack_count.add_theme_color_override("font_color", ThemeMaker.MUTED)
	pack_count.add_theme_font_size_override("font_size", 12)
	heading.add_child(pack_count)
	chooser.custom_minimum_size.y = 46
	chooser.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	chooser.fit_to_longest_item = false
	chooser.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	selection.add_child(chooser)
	chooser.item_selected.connect(func(index: int): skin_selected.emit(chooser.get_item_metadata(index)))
	var actions := HBoxContainer.new()
	actions.add_theme_constant_override("separation", 10)
	selection.add_child(actions)
	import_button = _button(actions, "导入新外观", func(): file_dialog.popup_centered_ratio(0.7), "import")
	_primary(import_button)
	_button(actions, "外观文件夹", func(): OS.shell_open(ProjectSettings.globalize_path(SkinCatalog.USER_ROOT)), "folder")
	selection.add_child(_wrapped("直接选择 3D 模型、图片、GIF 或 skin.json。支持 GLB / glTF / FBX / OBJ / VRM；Blender、STL、PLY 文件可自动转换。", 12, ThemeMaker.MUTED))
	status_panel.add_theme_stylebox_override("panel", ThemeMaker.panel(Color("edf4ee"), 10, 14))
	page.add_child(status_panel)
	status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	status.add_theme_font_size_override("font_size", 12)
	status_panel.add_child(status)

func _build_behavior(page: VBoxContainer) -> void:
	var activities := _card(page)
	activities.add_child(_label("日常活动", 16))
	_toggle_row(activities, "自由散步", "开启后，桌宠会随机走路、小跑、停留和休息。", prefs.roaming,
		func(value: bool): roaming_changed.emit(value))
	var live := PanelContainer.new()
	live.add_theme_stylebox_override("panel", ThemeMaker.panel(Color("f0f5ef"), 9, 12))
	activities.add_child(live)
	state_label.add_theme_color_override("font_color", ThemeMaker.ACCENT)
	live.add_child(state_label)
	var interaction := _card(page)
	interaction.add_child(_label("和它互动", 16))
	_guide_row(interaction, "轻点", "摸摸它，看看小小的回应。", "左键单击")
	_separator(interaction)
	_guide_row(interaction, "提起与抛出", "松手后沿拖动方向滑行，再落到屏幕底部。", "拖动后松手")
	_separator(interaction)
	_guide_row(interaction, "快捷菜单", "快速切换外观、大小、散步和置顶。", "右键单击")
	page.add_child(_wrapped("关闭自由散步后，点击和拖动互动仍然可用。", 12, ThemeMaker.MUTED))

func _build_display(page: VBoxContainer) -> void:
	var screen_card := _card(page)
	screen_card.add_child(_label("所在显示器", 16))
	screen_card.add_child(screen_chooser)
	screen_chooser.item_selected.connect(func(index: int): screen_requested.emit(index))
	screen_card.add_child(_wrapped("选择显示器可立即移动桌宠。也可直接拖过屏幕边缘；相邻工作区之间支持惯性跨屏和散步。断开显示器后会自动找回。", 12, ThemeMaker.MUTED))
	var scale_card := _card(page)
	scale_control = ScaleControl.new()
	scale_control.sync(prefs.zoom)
	scale_control.zoom_changed.connect(func(value: float): zoom_changed.emit(value))
	scale_card.add_child(scale_control)
	scale_card.add_child(_wrapped("拖动滑块实时调节；也可输入百分比后按 Enter 应用。", 12, ThemeMaker.MUTED))
	var window_card := _card(page)
	window_card.add_child(_label("窗口与位置", 16))
	_toggle_row(window_card, "始终置顶", "让桌宠显示在其他普通窗口上方。", prefs.always_on_top,
		func(value: bool): top_changed.emit(value))
	_separator(window_card)
	var location := HBoxContainer.new()
	location.add_theme_constant_override("separation", 16)
	window_card.add_child(location)
	var info := _column(location, 6)
	info.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	info.add_child(_label("找回桌宠", 14))
	info.add_child(_wrapped("将它移回主屏幕右下角。", 12, ThemeMaker.MUTED))
	var button := _button(location, "重置位置", func(): reset_requested.emit(), "reset")
	button.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	page.add_child(_wrapped("位置会自动保存，下次启动时继续待在原来的地方。", 12, ThemeMaker.MUTED))

func _build_adaptation(page: VBoxContainer) -> void:
	var hero := _card(page)
	var row := HBoxContainer.new()
	hero.add_child(row)
	action_preview_host.custom_minimum_size = Vector2(120, 120)
	action_preview_host.clip_contents = true
	action_preview_host.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(action_preview_host)
	var info := _column(row, 8)
	info.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	info.add_child(_label("每种外观，独立适配", 16))
	adaptation_summary.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	adaptation_summary.add_theme_color_override("font_color", ThemeMaker.MUTED)
	adaptation_summary.add_theme_font_size_override("font_size", 12)
	info.add_child(adaptation_summary)
	page.add_child(adaptation_host)
	adaptation_host.add_theme_constant_override("separation", 16)
	var converter := _card(page)
	converter.add_child(_label("模型转换器", 16))
	converter.add_child(_wrapped("GLB、glTF、FBX、OBJ 和 VRM 无需外部软件。导入 .blend、STL、PLY 需要本机安装 Blender 4.2+，程序会自动查找，也可手动选择。", 12, ThemeMaker.MUTED))
	var tools_row := HBoxContainer.new()
	converter.add_child(tools_row)
	converter_path.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	converter_path.placeholder_text = "blender.exe 路径"
	converter_path.text = AssetIO.blender_path()
	tools_row.add_child(converter_path)
	_button(tools_row, "浏览", func(): converter_dialog.popup_centered_ratio(0.7))
	converter_dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE
	converter_dialog.access = FileDialog.ACCESS_FILESYSTEM
	converter_dialog.filters = PackedStringArray(["blender.exe ; Blender 程序"])
	converter_dialog.use_native_dialog = true
	add_child(converter_dialog)
	converter_dialog.file_selected.connect(func(path: String):
		converter_path.text = path
		_save_converter(path))
	converter_path.text_submitted.connect(_save_converter)
	converter_path.focus_exited.connect(func(): _save_converter(converter_path.text))
	page.add_child(_wrapped("平面图片使用整体动作；骨骼与形态键使用模型自带动画。Live2D / Spine 工程请先导出为 GIF、精灵图或序列帧。VRM 当前保留基础网格、材质和骨架，不包含专用表情、MToon 和弹簧骨物理。", 12, ThemeMaker.MUTED))

func _save_converter(path: String) -> void:
	path = path.strip_edges().trim_prefix("\"").trim_suffix("\"")
	if not path.is_empty() and (not FileAccess.file_exists(path) or path.get_file().to_lower() != "blender.exe"):
		message("请选择有效的 blender.exe 文件。")
		return
	var file := FileAccess.open("user://converter-path.txt", FileAccess.WRITE)
	if file:
		file.store_string(path)

func _refresh_adaptation(skin: Dictionary) -> void:
	for child in adaptation_host.get_children():
		adaptation_host.remove_child(child)
		child.queue_free()
	if preview == null:
		return
	var overview := _card(adaptation_host)
	var info: Dictionary = preview.capability
	var matched := preview.mapping.size()
	adaptation_summary.text = "%s · %d 个动画 · %d 根骨骼\n已匹配 %d / %d 个动作，其余由通用模组完成。\n在下方逐项预览或调整。" % [info.get("kind", "外观"), preview.available_clips.size(), info.get("bones", 0), matched, ActionAdapter.STATES.size()]
	overview.add_child(_wrapped("%s · %d 个动画 · %d 根骨骼\n已匹配 %d / %d 个动作，其余使用通用模组。" % [info.get("kind", "外观"), preview.available_clips.size(), info.get("bones", 0), matched, ActionAdapter.STATES.size()], 13, ThemeMaker.INK))
	if info.get("rig_parts", 0) > 0:
		overview.add_child(_wrapped("已识别 %d 个骨骼部位，缺少专用动作时可叠加轻量肢体动作。" % info.rig_parts, 12, ThemeMaker.MUTED))
	if info.get("soft_bodies", 0) > 0:
		overview.add_child(_wrapped("已启用 %d 处软体效果：身体自然下沉、腹部承托、移动惯性与弹性回稳。" % info.soft_bodies, 12, ThemeMaker.MUTED))
	if info.get("expressions", 0) > 0:
		overview.add_child(_wrapped("已识别 %d 个眨眼 / 微笑形态键，可在通用动作中使用。" % info.expressions, 12, ThemeMaker.MUTED))
	var profile_row := HBoxContainer.new()
	overview.add_child(profile_row)
	profile_row.add_child(_label("动作风格", 13))
	var profile := OptionButton.new()
	profile.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	for i in ActionAdapter.PROFILES.size():
		profile.add_item(ActionAdapter.PROFILE_LABELS[i])
	profile.select(ActionAdapter.PROFILES.find(skin.get("profile", "auto")))
	profile_row.add_child(profile)
	profile.item_selected.connect(func(index: int): _save_adaptation("profile", ActionAdapter.PROFILES[index]))
	overview.add_child(_wrapped("当前风格：" + ActionAdapter.PROFILE_LABELS[ActionAdapter.PROFILES.find(preview.action_profile)] + "。人物使用更轻的形变，软团使用更明显的弹性；自带动作播放时会减弱叠加形变。", 12, ThemeMaker.MUTED))
	var actions := _card(adaptation_host)
	adaptation_host.move_child(actions.get_parent(), 0)
	actions.add_theme_constant_override("separation", 10)
	actions.add_child(_label("动作匹配与预览", 16))
	var automatic := ActionAdapter.resolve(preview.available_clips)
	for state: String in ActionAdapter.STATES:
		var row := HBoxContainer.new()
		actions.add_child(row)
		var label := _label(ActionAdapter.LABELS[state], 13)
		label.custom_minimum_size.x = 92
		row.add_child(label)
		var choose := OptionButton.new()
		choose.name = "Action_" + state
		choose.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		choose.fit_to_longest_item = false
		choose.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
		choose.add_item("自动 · " + str(automatic.get(state, "通用动作")))
		choose.set_item_metadata(0, "__auto__")
		choose.add_item("通用动作")
		choose.set_item_metadata(1, "")
		for clip in preview.available_clips:
			choose.add_item(clip)
			choose.set_item_metadata(choose.item_count - 1, clip)
		var explicit: Dictionary = skin.get("animations", {})
		if explicit.has(state):
			for i in choose.item_count:
				if choose.get_item_metadata(i) == explicit[state]:
					choose.select(i)
		choose.tooltip_text = ActionAdapter.source_for(state, preview.mapping, explicit) + "：" + str(preview.mapping.get(state, "通用模组"))
		row.add_child(choose)
		choose.item_selected.connect(func(index: int): _save_action(state, choose.get_item_metadata(index)))
		_button(row, "预览", func():
			if action_preview:
				action_preview.set_state(state)
				if state == "fall":
					action_preview.impact(2400))
	if skin.type == "sprite":
		var flip := CheckButton.new()
		flip.text = "跟随行走方向翻转"
		flip.button_pressed = skin.get("flip_with_direction", true)
		actions.add_child(flip)
		flip.toggled.connect(func(value: bool): _save_adaptation("flip_with_direction", value))
	else:
		var row := HBoxContainer.new()
		actions.add_child(_label("站立朝向（度）", 13))
		actions.add_child(row)
		var rotation: Array = skin.get("rotation_degrees", [0, 0, 0])
		for axis in 3:
			row.add_child(_label(["X", "Y", "Z"][axis], 12))
			var input := SpinBox.new()
			input.min_value = -180
			input.max_value = 180
			input.step = 1
			input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
			input.value = rotation[axis]
			row.add_child(input)
			input.value_changed.connect(func(value: float):
				var next := rotation.duplicate()
				next[axis] = value
				_save_adaptation("rotation_degrees", next))
	_button(actions, "恢复此角色的默认适配", func():
		if SkinOverrides.reset(prefs.skin_id) == OK:
			_adaptation_saved()
		else:
			message("适配设置保存失败。"))

func _save_action(state: String, clip: String) -> void:
	var skin := SkinOverrides.effective(catalog.find_skin(prefs.skin_id))
	var mapping: Dictionary = skin.get("animations", {}).duplicate()
	if clip == "__auto__":
		mapping.erase(state)
	else:
		mapping[state] = clip
	_save_adaptation("animations", mapping)

func _save_adaptation(key: String, value: Variant) -> void:
	var entry: Dictionary = SkinOverrides.read_all().get(prefs.skin_id, {}).duplicate(true)
	entry[key] = value
	if SkinOverrides.save(prefs.skin_id, entry) == OK:
		_adaptation_saved()
	else:
		message("适配设置保存失败。")

func _adaptation_saved() -> void:
	preview_id = ""
	adaptation_changed.emit()

func set_importing(active: bool) -> void:
	import_button.disabled = active
	import_button.text = "正在导入…" if active else "导入新外观"
	if active:
		message("正在读取模型 / 图片并匹配动作。转换期间桌宠可以继续互动。")

func set_screens(labels: Array[String]) -> void:
	screen_chooser.clear()
	for label in labels:
		screen_chooser.add_item(label)

func sync_screen(index: int) -> void:
	if index >= 0 and index < screen_chooser.item_count:
		screen_chooser.select(index)

func _build_dialog() -> void:
	file_dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE
	file_dialog.access = FileDialog.ACCESS_FILESYSTEM
	file_dialog.filters = PackedStringArray(["*.glb,*.gltf,*.fbx,*.obj,*.vrm,*.blend,*.stl,*.ply,*.png,*.jpg,*.jpeg,*.webp,*.bmp,*.tga,*.svg,*.gif,*.json ; 模型、图片或外观包", "*.glb,*.gltf,*.fbx,*.obj,*.vrm,*.blend,*.stl,*.ply ; 3D 模型", "*.png,*.jpg,*.jpeg,*.webp,*.bmp,*.tga,*.svg,*.gif ; 图片与动图", "skin.json ; LPP 外观清单"])
	file_dialog.title = "选择模型、图片或外观包"
	file_dialog.use_native_dialog = true
	add_child(file_dialog)
	file_dialog.file_selected.connect(func(path: String): import_requested.emit(path))

func _select_page(index: int) -> void:
	current_page = index
	page_title.text = SECTIONS[index][0]
	page_description.text = SECTIONS[index][1]
	for i in pages.size():
		pages[i].visible = i == index
		var selected := i == index
		var normal := ThemeMaker.panel(Color("e9f3ec") if selected else Color.TRANSPARENT, 10, 12)
		navigation[i].add_theme_stylebox_override("normal", normal)
		navigation[i].add_theme_color_override("font_color", ThemeMaker.ACCENT if selected else ThemeMaker.MUTED)
		navigation[i].icon = ThemeMaker.icon(SECTIONS[i][2], ThemeMaker.ACCENT if selected else ThemeMaker.MUTED)
	_sync_preview_activity()

func refresh() -> void:
	chooser.clear()
	for skin in catalog.skins:
		chooser.add_item(skin.name)
		var index := chooser.item_count - 1
		chooser.set_item_metadata(index, skin.id)
		if skin.id == prefs.skin_id:
			chooser.select(index)
	pack_count.text = "%d 个可用外观" % catalog.skins.size()
	var skin := catalog.find_skin(prefs.skin_id)
	if not skin.is_empty():
		preview_name.text = skin.name.split(" · ")[0]
		preview_type.text = "2D 图片 / 动画" if skin.type == "sprite" else "3D 模型外观"
		if skin.id != preview_id:
			_update_preview(SkinOverrides.effective(skin))
	status_panel.visible = not catalog.errors.is_empty()
	if not catalog.errors.is_empty():
		message("部分外观未加载：" + "; ".join(catalog.errors))

func message(text: String) -> void:
	status.text = text
	status_panel.visible = true
	status.add_theme_color_override("font_color", Color("965b35"))
	status_panel.add_theme_stylebox_override("panel", ThemeMaker.panel(Color("fff3e8"), 10, 14))
	_select_page(0)

func _update_preview(skin: Dictionary) -> void:
	if preview:
		preview.queue_free()
	preview = SpriteAvatar.new() if skin.type == "sprite" else ModelAvatar.new()
	preview_host.add_child(preview)
	var error := preview.configure(skin)
	if not error.is_empty():
		preview.queue_free()
		preview = null
		preview_id = ""
		return
	preview.scale = Vector2.ONE * 0.51
	preview.apply_performance(prefs.performance)
	if action_preview:
		action_preview.queue_free()
	action_preview = SpriteAvatar.new() if skin.type == "sprite" else ModelAvatar.new()
	action_preview_host.add_child(action_preview)
	var action_error := action_preview.configure(skin)
	if not action_error.is_empty():
		action_preview.queue_free()
		action_preview = null
	else:
		action_preview.apply_performance(prefs.performance)
		action_preview.scale = Vector2.ONE * (0.375 if skin.type == "sprite" else 0.55)
		if skin.type != "sprite":
			action_preview.position = Vector2(-28, -28)
	preview_id = skin.id
	_refresh_adaptation(skin)
	_sync_preview_activity()

func _sync_preview_activity() -> void:
	if not is_instance_valid(preview):
		return
	for entry in [[preview, 0], [action_preview, 3]]:
		var actor: PetAvatar = entry[0]
		if not is_instance_valid(actor):
			continue
		var active: bool = visible and current_page == entry[1]
		actor.process_mode = Node.PROCESS_MODE_INHERIT if active else Node.PROCESS_MODE_DISABLED
		if actor.get("viewport") is SubViewport:
			actor.viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS if active else SubViewport.UPDATE_DISABLED

func sync_preferences() -> void:
	if scale_control:
		scale_control.sync(prefs.zoom)
	for entry in [[roaming_toggle, prefs.roaming], [top_toggle, prefs.always_on_top]]:
		var toggle: Button = entry[0]
		if toggle:
			toggle.set_pressed_no_signal(entry[1])
			if toggle.transition:
				toggle.transition.kill()
			toggle.progress = 1.0 if entry[1] else 0.0
			toggle.queue_redraw()

func sync_performance() -> void:
	if performance_panel:
		performance_panel.sync()
	for actor in [preview, action_preview]:
		if is_instance_valid(actor):
			actor.apply_performance(prefs.performance)

func _toggle_row(parent: Node, title_text: String, description: String, enabled: bool, action: Callable) -> void:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 24)
	parent.add_child(row)
	var info := _column(row, 6)
	info.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	info.add_child(_label(title_text, 14))
	info.add_child(_wrapped(description, 12, ThemeMaker.MUTED))
	var toggle := Toggle.new()
	if title_text == "自由散步":
		roaming_toggle = toggle
	else:
		top_toggle = toggle
	toggle.name = "RoamingToggle" if title_text == "自由散步" else "TopToggle"
	toggle.tooltip_text = title_text
	toggle.button_pressed = enabled
	toggle.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	toggle.toggled.connect(action)
	row.add_child(toggle)

func _guide_row(parent: Node, title_text: String, description: String, key: String) -> void:
	var row := HBoxContainer.new()
	parent.add_child(row)
	var info := _column(row, 5)
	info.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	info.add_child(_label(title_text, 14))
	info.add_child(_wrapped(description, 12, ThemeMaker.MUTED))
	var badge := PanelContainer.new()
	badge.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	badge.add_theme_stylebox_override("panel", ThemeMaker.panel(Color("f3f6f3"), 6, 8))
	row.add_child(badge)
	badge.add_child(_label(key, 12, ThemeMaker.MUTED))

func _card(parent: Node) -> VBoxContainer:
	var panel := PanelContainer.new()
	panel.add_theme_stylebox_override("panel", ThemeMaker.panel(Color.WHITE, 14, 22, ThemeMaker.LINE))
	parent.add_child(panel)
	return _column(panel, 16)

func _column(parent: Node, spacing: int) -> VBoxContainer:
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", spacing)
	parent.add_child(column)
	return column

func _label(text: String, font_size := 14, color := ThemeMaker.INK) -> Label:
	var label := Label.new()
	label.text = text
	label.add_theme_font_size_override("font_size", font_size)
	label.add_theme_color_override("font_color", color)
	return label

func _wrapped(text: String, font_size: int, color: Color) -> Label:
	var label := _label(text, font_size, color)
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	return label

func _space(parent: Node, height: float) -> void:
	var spacer := Control.new()
	spacer.custom_minimum_size.y = height
	parent.add_child(spacer)

func _separator(parent: Node) -> void:
	var line := ColorRect.new()
	line.color = ThemeMaker.LINE
	line.custom_minimum_size.y = 1
	parent.add_child(line)

func _button(parent: Node, text: String, action: Callable, icon_name := "") -> Button:
	var button := Button.new()
	button.text = text
	button.custom_minimum_size.y = 42
	button.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
	if not icon_name.is_empty():
		button.icon = ThemeMaker.icon(icon_name)
	button.pressed.connect(action)
	parent.add_child(button)
	return button

func _primary(button: Button) -> void:
	for state in ["normal", "hover", "pressed"]:
		var color := ThemeMaker.ACCENT
		if state == "hover":
			color = color.lightened(0.08)
		elif state == "pressed":
			color = color.darkened(0.08)
		button.add_theme_stylebox_override(state, ThemeMaker.panel(color, 9, 12))
	for state in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
		button.add_theme_color_override(state, Color.WHITE)
	button.icon = ThemeMaker.icon("import", Color.WHITE)
