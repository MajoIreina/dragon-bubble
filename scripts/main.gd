extends Node

const SpriteAvatar = preload("res://scripts/avatars/sprite_avatar.gd")
const ModelAvatar = preload("res://scripts/avatars/model_avatar.gd")
const SettingsWindow = preload("res://scripts/ui/settings_window.gd")
const QuickMenu = preload("res://scripts/ui/quick_menu.gd")
const Motion = preload("res://scripts/core/pet_motion.gd")
const DesktopLayout = preload("res://scripts/core/desktop_layout.gd")
const TrayIcon = preload("res://scripts/ui/tray_icon.gd")
var desktop := DesktopLayout.new()
var screen_signature := ""
var screen_poll := 0.0
var catalog := SkinCatalog.new()
var prefs := PetPreferences.new()
var brain := PetBrain.new()
var avatar: PetAvatar
var settings: Window
var quick_menu: PopupMenu
var motion := Motion.new()
var dragging := false
var pressed := false
var press_cursor := Vector2i.ZERO
var press_window := Vector2i.ZERO
var walk_remainder := 0.0
var initializing := true
var import_worker: Thread
var tray: Node

func _ready() -> void:
	get_window().transparent_bg = true
	get_window().unfocusable = true
	get_window().gui_embed_subwindows = false
	get_window().close_requested.connect(_quit)
	prefs.load_saved()
	PetI18n.apply(prefs.locale)
	get_window().title = PetI18n.APP_NAME
	PetRenderOptions.apply_global(prefs.performance)
	_refresh_screens()
	catalog.scan()
	settings = SettingsWindow.new()
	add_child(settings)
	settings.setup(catalog, prefs)
	settings.skin_selected.connect(_switch_skin)
	settings.import_requested.connect(_import_skin)
	settings.adaptation_changed.connect(func(): _switch_skin(prefs.skin_id))
	settings.performance_changed.connect(_set_performance)
	settings.zoom_changed.connect(_set_zoom)
	settings.roaming_changed.connect(_set_roaming)
	settings.top_changed.connect(_set_on_top)
	settings.locale_changed.connect(_set_locale)
	settings.reset_requested.connect(_reset_position)
	settings.quit_requested.connect(_quit)
	settings.screen_requested.connect(_move_to_screen)
	quick_menu = QuickMenu.new()
	add_child(quick_menu)
	quick_menu.setup(catalog, prefs)
	quick_menu.settings_requested.connect(_show_settings)
	quick_menu.performance_requested.connect(func():
		_show_settings()
		settings._select_page(4))
	quick_menu.roaming_requested.connect(_set_roaming)
	quick_menu.top_requested.connect(_set_on_top)
	quick_menu.skin_requested.connect(_switch_skin)
	quick_menu.zoom_requested.connect(_set_zoom)
	quick_menu.reset_requested.connect(_reset_position)
	quick_menu.quit_requested.connect(_quit)
	quick_menu.screen_requested.connect(_move_to_screen)
	tray = TrayIcon.new()
	add_child(tray)
	tray.setup()
	tray.settings_requested.connect(_show_settings)
	tray.context_requested.connect(_open_quick_menu)
	screen_signature = ""
	_refresh_screens()
	brain.state_changed.connect(_on_state)
	brain.set_roaming(prefs.roaming)
	get_window().always_on_top = prefs.always_on_top
	if not _switch_skin(prefs.skin_id):
		if not _switch_skin(PetPreferences.DEFAULT_SKIN):
			_switch_skin("builtin.mochi")
	_set_zoom(prefs.zoom, false)
	if prefs.position.x == -99999:
		_reset_position()
	else:
		get_window().position = prefs.position
		_clamp_position()
	# First launch presents discoverable controls; subsequent launches stay quiet.
	if not FileAccess.file_exists("user://welcome_seen"):
		_show_settings.call_deferred()
		var marker := FileAccess.open("user://welcome_seen", FileAccess.WRITE)
		if marker:
			marker.store_string("1")
	initializing = false
	motion.stop(Vector2(get_window().position))
	_save()
	# A bounded run for smoke tests; it is never enabled by normal launch.
	if "--smoke-test" in OS.get_cmdline_user_args():
		_smoke_test.call_deferred()

func _switch_skin(id: String) -> bool:
	var skin := SkinOverrides.effective(catalog.find_skin(id))
	if skin.is_empty():
		settings.message("找不到外观包：" + id)
		return false
	var next: PetAvatar = SpriteAvatar.new() if skin.type == "sprite" else ModelAvatar.new()
	add_child(next)
	var error := next.configure(skin)
	if not error.is_empty():
		next.queue_free()
		settings.message("外观加载失败，保留当前外观：" + error)
		settings.refresh()
		settings.message("外观加载失败：" + error)
		return false
	var previous_ground := avatar.ground_offset if avatar != null else 248.0
	if avatar != null:
		avatar.queue_free()
	avatar = next
	avatar.apply_performance(prefs.performance)
	avatar.scale = Vector2.ONE * prefs.zoom
	avatar.direction = brain.direction
	avatar.set_state(brain.state)
	if not initializing:
		get_window().position.y += roundi((previous_ground - avatar.ground_offset) * prefs.zoom)
		motion.position = Vector2(get_window().position)
	_set_zoom(prefs.zoom, false)
	prefs.skin_id = id
	settings.refresh()
	_save()
	return true

func _import_skin(path: String) -> void:
	if import_worker != null:
		return
	settings.set_importing(true)
	import_worker = Thread.new()
	var error := import_worker.start(func():
		var importer := SkinCatalog.new()
		importer.scan()
		return importer.import_file(path))
	if error != OK:
		import_worker = null
		settings.set_importing(false)
		settings.message("无法启动导入任务：" + error_string(error))

func _finish_import() -> void:
	var result: Dictionary = import_worker.wait_to_finish()
	import_worker = null
	catalog.scan()
	settings.set_importing(false)
	settings.refresh()
	if result.ok:
		_switch_skin(result.skin.id)
	else:
		settings.message("导入失败：" + result.error)

func _input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_RIGHT and event.pressed:
			if not pressed:
				_open_quick_menu(DisplayServer.mouse_get_position())
		elif event.button_index == MOUSE_BUTTON_LEFT:
			if event.pressed:
				pressed = true
				press_cursor = DisplayServer.mouse_get_position()
				press_window = get_window().position
				motion.begin_drag(Vector2(press_window), _now())
			else:
				_release()
	elif event is InputEventMouseMotion and pressed:
		_track_drag()

func _process(delta: float) -> void:
	if import_worker != null and not import_worker.is_alive():
		_finish_import()
	screen_poll += delta
	if screen_poll >= 1.0:
		screen_poll = 0
		_refresh_screens()
	# Global cursor coordinates keep dragging stable when the small pet window moves.
	if pressed:
		_track_drag()
		# Recover if the release happened outside this window.
		if not (DisplayServer.mouse_get_button_state() & MOUSE_BUTTON_MASK_LEFT):
			_release()
	var locomotion_blocked: bool = motion.active or pressed or settings.visible or quick_menu.is_open()
	if avatar:
		avatar.locomotion_paused = locomotion_blocked
		avatar.locomotion_rate = 1.0
	if not motion.active and not pressed and not quick_menu.is_open() and not (avatar and avatar.turning):
		brain.tick(delta)
	if brain.movement_speed() > 0 and not locomotion_blocked and not avatar.turning:
		# Match stride length at every pet size, including 20% and 300%.
		var bounds := desktop.bounds_for(Vector2(get_window().position))
		var distance := bounds.end.x - get_window().position.x if brain.direction > 0 else get_window().position.x - bounds.position.x
		# Ease down over the last fraction of a stride before planting the turn.
		var braking := clampf(distance / maxf(brain.movement_speed() * prefs.zoom * 0.18, 2.0), 0.22, 1.0)
		braking *= avatar.resume_rate()
		avatar.locomotion_rate = braking
		walk_remainder += delta * brain.movement_speed() * prefs.zoom * braking * brain.direction
		var step := int(walk_remainder)
		walk_remainder -= step
		get_window().position.x += step
		var before := get_window().position
		_clamp_position()
		if before != get_window().position or distance <= 1.0:
			avatar.direction = brain.direction
			avatar.begin_turn(-brain.direction)
			brain.direction = avatar.direction
			walk_remainder = 0.0
	if avatar:
		avatar.direction = brain.direction
		avatar.secondary_grounded = not motion.active and not dragging
		avatar.update_desktop_motion(Vector2(get_window().position), prefs.zoom)
	if settings.visible:
		settings.sync_screen(desktop.current(Vector2(get_window().position)))
	_update_state_label()

func _update_state_label() -> void:
	settings.state_label.text = tr("当前状态：") + tr({"idle": "发呆", "walk": "走路", "run": "跑步", "sleep": "打盹",
		"petted": "被摸摸", "dragged": "被提起", "fall": "回弹", "airborne": "惯性 / 下落"}.get(brain.state, brain.state))

func _physics_process(delta: float) -> void:
	if initializing or pressed or quick_menu.is_open():
		return
	var bounds := _motion_bounds()
	if not motion.active and get_window().position.y < bounds.end.y - 1:
		motion.launch(Vector2(get_window().position), Vector2.ZERO)
		brain.change("airborne")
	if not motion.active:
		return
	var result := motion.advance(delta, bounds, desktop)
	get_window().position = Vector2i(motion.position.round())
	if absf(motion.velocity.x) > 20:
		brain.direction = 1 if motion.velocity.x > 0 else -1
	if result.landed and avatar:
		avatar.impact(result.impact)
	if result.settled:
		brain.change("fall", 0.8)
		_save()

func _track_drag() -> void:
	var offset := DisplayServer.mouse_get_position() - press_cursor
	if offset.length() > 4:
		dragging = true
		brain.change("dragged")
	if dragging:
		get_window().position = press_window + offset
	motion.record_drag(Vector2(get_window().position), _now())

func _now() -> float:
	return Time.get_ticks_usec() / 1000000.0

func _release() -> void:
	if not pressed:
		return
	_track_drag()
	pressed = false
	_clamp_position()
	if dragging:
		var speed := motion.release_velocity(_now())
		motion.launch(Vector2(get_window().position), speed)
		brain.change("airborne")
	else:
		brain.change("petted", 1.5)
	dragging = false
	_save()

func _on_state(state: String) -> void:
	walk_remainder = 0.0
	if avatar:
		avatar.direction = brain.direction
		avatar.set_state(state)

func _set_zoom(value: float, persist := true) -> void:
	var old_zoom := prefs.zoom
	prefs.zoom = PetPreferences.normalize_zoom(value)
	get_window().size = Vector2i(Vector2.ONE * 320 * prefs.zoom)
	if avatar:
		avatar.scale = Vector2.ONE * prefs.zoom
		if not initializing:
			get_window().position.y += roundi(avatar.ground_offset * (old_zoom - prefs.zoom))
	# Windows clips rendering outside this polygon. Keep an ample shared region
	# around all supported normalized avatars, including squash and bounce.
	var bottom := minf(306.0, avatar.ground_offset + 2.0) if avatar else 306.0
	var polygon := PackedVector2Array([Vector2(8, 4), Vector2(312, 4),
		Vector2(312, bottom), Vector2(8, bottom)])
	for i in polygon.size():
		polygon[i] *= prefs.zoom
	get_window().mouse_passthrough_polygon = polygon
	_update_extent()
	_clamp_position()
	motion.position = Vector2(get_window().position)
	settings.sync_preferences()
	if quick_menu:
		quick_menu.sync_preferences()
	if persist:
		_save()

func _usable_area() -> Rect2i:
	return Rect2i(desktop.areas[desktop.current(Vector2(get_window().position))])

func _clamp_position() -> void:
	get_window().position = Vector2i(desktop.clamp_origin(Vector2(get_window().position)).round())

func _motion_bounds() -> Rect2:
	return desktop.bounds_for(Vector2(get_window().position))

func _update_extent() -> void:
	desktop.extent = Vector2(get_window().size.x, avatar.ground_offset * prefs.zoom if avatar else get_window().size.y)

func _refresh_screens() -> void:
	var areas: Array[Rect2] = []
	var labels: Array[String] = []
	for i in DisplayServer.get_screen_count():
		var area := DisplayServer.screen_get_usable_rect(i)
		if area.size == Vector2i.ZERO:
			area = Rect2i(DisplayServer.screen_get_position(i), DisplayServer.screen_get_size(i))
		areas.append(Rect2(area))
		labels.append(tr("显示器 %d%s · %d × %d") % [i + 1, tr("（主屏）") if i == DisplayServer.get_primary_screen() else "", DisplayServer.screen_get_size(i).x, DisplayServer.screen_get_size(i).y])
	if areas.is_empty():
		areas.append(Rect2(0, 0, 1920, 1080))
		labels.append(tr("主屏幕"))
	var signature := str(areas) + str(labels)
	if signature == screen_signature:
		return
	screen_signature = signature
	var layout_changed := desktop.areas != areas
	desktop.areas = areas
	_update_extent()
	if settings != null:
		quick_menu.hide()
		quick_menu.size_popup.hide()
		settings.set_screens(labels)
		quick_menu.set_screens(labels)
		settings.sync_screen(desktop.current(Vector2(get_window().position)))
		if not layout_changed:
			return
		if not pressed:
			_clamp_position()
			motion.stop(Vector2(get_window().position))
			brain.change("idle", 2)
			_save()
		if settings.visible:
			_place_settings()

func _move_to_screen(index: int) -> void:
	if index < 0 or index >= desktop.areas.size():
		return
	var area := desktop.areas[index]
	var maximum := (area.end - desktop.extent).max(area.position)
	var target := area.end - Vector2(get_window().size.x + 40, desktop.extent.y)
	get_window().position = Vector2i(target.clamp(area.position, maximum).round())
	if avatar:
		avatar.reset_secondary_motion()
	motion.stop(Vector2(get_window().position))
	brain.change("idle", 4)
	settings.sync_screen(index)
	_save()

func _reset_position() -> void:
	_move_to_screen(DisplayServer.get_primary_screen())

func _show_settings() -> void:
	quick_menu.hide()
	settings.sync_preferences()
	_place_settings()
	settings.show()
	settings.grab_focus()

func _open_quick_menu(cursor: Vector2i) -> void:
	if not pressed:
		quick_menu.open_at(cursor, Rect2i(desktop.areas[desktop.screen_at(cursor)]))

func _set_performance(value: Dictionary) -> void:
	prefs.performance = PetRenderOptions.normalize(value)
	PetRenderOptions.apply_global(prefs.performance)
	if avatar:
		avatar.apply_performance(prefs.performance)
	settings.sync_performance()
	_save()

func _set_locale(value: String) -> void:
	prefs.locale = PetI18n.normalize(value)
	PetI18n.apply(prefs.locale)
	settings.retranslate()
	quick_menu.retranslate()
	tray.retranslate()
	_update_state_label()
	screen_signature = ""
	_refresh_screens()
	_save()

func _place_settings() -> void:
	var index := desktop.current(Vector2(get_window().position))
	var area := Rect2i(desktop.areas[index])
	settings.current_screen = index
	settings.min_size = SettingsWindow.MINIMUM_SIZE.min(area.size)
	settings.size = SettingsWindow.DEFAULT_SIZE.min(area.size)
	settings.position = area.position + (area.size - settings.size) / 2

func _set_roaming(value: bool) -> void:
	prefs.roaming = value
	brain.set_roaming(value)
	settings.sync_preferences()
	quick_menu.sync_preferences()
	_save()

func _set_on_top(value: bool) -> void:
	prefs.always_on_top = value
	get_window().always_on_top = value
	settings.sync_preferences()
	quick_menu.sync_preferences()
	_save()

func _save() -> void:
	if initializing:
		return
	prefs.position = get_window().position
	var error := prefs.save()
	if error != OK:
		push_warning("设置保存失败：" + error_string(error))

func _quit() -> void:
	if import_worker != null:
		settings.message("正在导入外观，完成后即可退出。")
		return
	_save()
	get_tree().quit()

func _smoke_test() -> void:
	var original_skin := prefs.skin_id
	for skin in catalog.skins:
		if not _switch_skin(skin.id):
			get_tree().quit(1)
			return
		brain.change("walk")
		await get_tree().create_timer(0.15).timeout
		brain.change("run")
		await get_tree().create_timer(0.15).timeout
		avatar.begin_turn(-brain.direction)
		brain.direction = avatar.direction
		await get_tree().create_timer(0.85).timeout
		brain.change("sleep", 2)
		await get_tree().create_timer(0.15).timeout
		brain.change("petted")
		await get_tree().create_timer(0.15).timeout
	print("SMOKE OK: switched ", catalog.skins.size(), " skins")
	_switch_skin(original_skin)
	get_tree().quit()
