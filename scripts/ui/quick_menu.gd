extends PopupMenu

signal settings_requested
signal performance_requested
signal roaming_requested(value: bool)
signal top_requested(value: bool)
signal skin_requested(id: String)
signal zoom_requested(value: float)
signal reset_requested
signal quit_requested
signal screen_requested(index: int)
var screens_menu := PopupMenu.new()

const ThemeMaker = preload("res://scripts/ui/settings_theme.gd")
const ScaleControl = preload("res://scripts/ui/scale_control.gd")
enum Action { SETTINGS = 1, ROAMING, ON_TOP, RESET, QUIT, SIZE, PERFORMANCE }
var skins_menu := PopupMenu.new()
var size_popup := PopupPanel.new()
var scale_control := ScaleControl.new()
var source: SkinCatalog
var prefs: PetPreferences
var ids: Array[String] = []
var anchor := Vector2i.ZERO
var available := Rect2i()
var flyout_motion: Tween

func setup(catalog: SkinCatalog, preferences: PetPreferences) -> void:
	source = catalog
	prefs = preferences
	theme = ThemeMaker.create()
	min_size.x = 252
	# PopupMenu is already transient; do not combine that with always_on_top.
	skins_menu.name = "Skins"
	skins_menu.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
	skins_menu.min_size.x = 252
	add_child(skins_menu)
	screens_menu.name = "Screens"
	screens_menu.min_size.x = 252
	add_child(screens_menu)
	screens_menu.id_pressed.connect(func(index: int): screen_requested.emit(index))
	for submenu in [skins_menu, screens_menu]:
		submenu.about_to_popup.connect(_slide_submenu.bind(submenu))
	size_popup.title = PetI18n.window_title("大小调节")
	size_popup.theme = theme
	size_popup.transparent = false
	size_popup.transparent_bg = false
	size_popup.add_theme_stylebox_override("panel", ThemeMaker.panel(ThemeMaker.SURFACE, 10, 20, ThemeMaker.LINE))
	# Sibling popup survives the context menu closing after item selection.
	get_parent().add_child(size_popup)
	size_popup.add_child(scale_control)
	for control in scale_control.find_children("*", "Button", true, false):
		ThemeMaker.animate_button(control)
	size_popup.popup_hide.connect(scale_control.commit)
	scale_control.zoom_changed.connect(func(value: float): zoom_requested.emit(value))
	id_pressed.connect(_activate)
	skins_menu.id_pressed.connect(func(index: int): skin_requested.emit(ids[index]))

func open_at(cursor: Vector2i, work_area: Rect2i) -> void:
	anchor = cursor
	available = work_area
	clear()
	add_icon_item(ThemeMaker.icon("settings"), "打开设置", Action.SETTINGS)
	add_separator()
	add_check_item("自由散步", Action.ROAMING)
	set_item_checked(get_item_index(Action.ROAMING), prefs.roaming)
	add_check_item("桌宠始终置顶", Action.ON_TOP)
	set_item_checked(get_item_index(Action.ON_TOP), prefs.always_on_top)
	skins_menu.clear()
	ids.clear()
	for skin in source.skins:
		var index := ids.size()
		ids.append(skin.id)
		skins_menu.add_radio_check_item(PetI18n.skin_name(skin), index)
		skins_menu.set_item_checked(index, skin.id == prefs.skin_id)
	add_submenu_item("切换外观", "Skins", 20)
	add_submenu_item("移到显示器", "Screens", 21)
	add_item(tr("桌宠大小 · %d%%…") % roundi(prefs.zoom * 100), Action.SIZE)
	add_item("性能与画质…", Action.PERFORMANCE)
	add_separator()
	add_icon_item(ThemeMaker.icon("reset"), "找回桌宠", Action.RESET)
	add_icon_item(ThemeMaker.icon("exit"), "退出桌宠", Action.QUIT)
	reset_size()
	_popup_at(self, cursor, work_area)

func _popup_at(window: Window, cursor: Vector2i, work_area: Rect2i) -> void:
	# PopupMenu recomputes min_size when its translated theme changes.
	# Reapply the flyout width after populating items, before screen clamping.
	window.min_size.x = maxi(252, window.min_size.x)
	window.size.x = maxi(252, window.size.x)
	var target := Vector2i(
		clampi(cursor.x, work_area.position.x, maxi(work_area.position.x, work_area.end.x - window.size.x)),
		clampi(cursor.y, work_area.position.y, maxi(work_area.position.y, work_area.end.y - window.size.y)))
	window.popup(Rect2i(target, window.size))
	# Godot fits a hidden popup to its visible parent's screen during popup().
	# Place the native window on the requested screen after it has been created.
	window.position = target
	_slide_flyout(window, work_area)

func _slide_flyout(window: Window, work_area: Rect2i) -> void:
	if flyout_motion:
		flyout_motion.kill()
	var target := window.position
	window.position.y -= mini(6, target.y - work_area.position.y)
	flyout_motion = create_tween()
	flyout_motion.set_trans(Tween.TRANS_CUBIC).set_ease(Tween.EASE_OUT)
	flyout_motion.tween_property(window, "position", target, 0.14)

func _slide_submenu(window: Window) -> void:
	# about_to_popup is emitted before Godot places the submenu.
	_slide_submenu_ready.call_deferred(window)

func _slide_submenu_ready(window: Window) -> void:
	if window.visible:
		var screen := DisplayServer.get_screen_from_rect(Rect2i(window.position, window.size))
		var work_area := DisplayServer.screen_get_usable_rect(screen)
		window.min_size.x = maxi(252, window.min_size.x)
		window.size.x = maxi(252, window.size.x)
		window.position.x = clampi(window.position.x, work_area.position.x, maxi(work_area.position.x, work_area.end.x - window.size.x))
		_slide_flyout(window, work_area)

func _activate(id: int) -> void:
	match id:
		Action.SETTINGS: settings_requested.emit()
		Action.PERFORMANCE: performance_requested.emit()
		Action.ROAMING: roaming_requested.emit(not prefs.roaming)
		Action.ON_TOP: top_requested.emit(not prefs.always_on_top)
		Action.RESET: reset_requested.emit()
		Action.QUIT: quit_requested.emit()
		Action.SIZE: _open_scale.call_deferred()

func _open_scale() -> void:
	scale_control.sync(prefs.zoom)
	size_popup.size = Vector2i(340, 190)
	_popup_at(size_popup, anchor, available)

func sync_preferences() -> void:
	scale_control.sync(prefs.zoom)
	for entry in [[Action.ROAMING, prefs.roaming], [Action.ON_TOP, prefs.always_on_top]]:
		var index := get_item_index(entry[0])
		if index >= 0:
			set_item_checked(index, entry[1])

func retranslate() -> void:
	hide()
	size_popup.hide()
	theme = ThemeMaker.create()
	size_popup.theme = theme
	size_popup.title = PetI18n.window_title("大小调节")

func set_screens(labels: Array[String]) -> void:
	screens_menu.clear()
	for i in labels.size():
		screens_menu.add_item(labels[i], i)

func is_open() -> bool:
	return visible or size_popup.visible
