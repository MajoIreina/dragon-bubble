extends SceneTree

var failures := 0
var checks := 0

func _initialize() -> void:
	call_deferred("run")

func check(condition: bool, message: String) -> void:
	checks += 1
	if not condition:
		failures += 1
		push_error(message)

func run() -> void:
	var prefs_path := ProjectSettings.globalize_path(PetPreferences.PATH)
	var old_preferences := FileAccess.get_file_as_bytes(prefs_path) if FileAccess.file_exists(prefs_path) else PackedByteArray()
	var catalog := SkinCatalog.new()
	catalog.scan()
	var fixture := catalog.import_file(ProjectSettings.globalize_path("res://work/adapter-fixtures/sheet.json"))
	check(fixture.ok, "Install UI animation fixture")
	if not fixture.ok:
		quit(1)
		return
	var app = load("res://scenes/main.tscn").instantiate()
	root.add_child(app)
	await process_frame
	var original: String = app.prefs.skin_id
	app._switch_skin(fixture.skin.id)
	app._show_settings()
	app.settings._select_page(3)
	await create_timer(0.2).timeout
	check(app.settings.pages.size() == 5, "Settings expose adaptation and performance categories")
	check(app.settings.preview.mapping.get("walk") == "Walk", "Settings show automatic 2D mapping")
	var run_choice: OptionButton = app.settings.adaptation_host.find_child("Action_run", true, false)
	check(run_choice != null and app.settings.preview.mapping.get("run") == "Walk", "Settings expose running with compatibility fallback")
	for child in run_choice.get_parent().get_children():
		if child is Button and not child is OptionButton:
			child.pressed.emit()
	check(app.settings.action_preview.state == "run" and app.settings.action_preview.native_action, "Running preview button plays its mapped action")
	var walk_choice: OptionButton = app.settings.adaptation_host.find_child("Action_walk", true, false)
	for i in walk_choice.item_count:
		if walk_choice.get_item_metadata(i) == "Happy":
			walk_choice.item_selected.emit(i)
			break
	await process_frame
	check(app.avatar.mapping.get("walk") == "Happy" and not app.avatar.mapping.has("sleep"), "Selecting an action dropdown edits only its own state")
	check(app.avatar.mapping.get("run") == "Walk", "Editing walking leaves the running mapping independent")
	app.settings._save_action("walk", "__auto__")
	app.settings._save_action("sleep", "Happy")
	await process_frame
	check(app.avatar.mapping.get("sleep") == "Happy" and app.settings.preview.mapping.get("sleep") == "Happy", "Manual mapping updates live pet and preview")
	check(SkinOverrides.effective(fixture.skin).animations.sleep == "Happy", "Mapping persists separately from source manifest")
	check(not catalog.read_manifest("res://work/adapter-fixtures/sheet.json").skin.get("animations", {}).has("sleep"), "Source manifest remains unchanged")
	app.settings._save_action("walk", "")
	await process_frame
	app.avatar.set_state("walk")
	check(not app.avatar.native_action and not app.avatar.mapping.has("walk"), "Generic override suppresses native clip")
	app.settings._save_adaptation("profile", "character")
	await process_frame
	check(app.avatar.action_profile == "character", "Per-skin profile applies immediately")
	app.settings._save_adaptation("flip_with_direction", false)
	app.avatar.direction = -1
	app.avatar._process(0.1)
	check(not app.avatar.sprite.flip_h, "Per-skin direction flip override applies")
	app.settings._save_action("walk", "__auto__")
	await process_frame
	check(app.avatar.mapping.get("walk") == "Walk", "Automatic mapping can be restored")
	app.settings.action_preview.set_state("petted")
	await create_timer(0.2).timeout
	await RenderingServer.frame_post_draw
	app.settings.get_texture().get_image().save_png("res://outputs/adaptation-settings.png")
	var old_avatar: PetAvatar = app.avatar
	app._import_skin(ProjectSettings.globalize_path("res://work/adapter-fixtures/bad.gif"))
	while app.import_worker != null:
		await process_frame
	check(app.avatar == old_avatar and app.prefs.skin_id == fixture.skin.id, "Failed asynchronous import keeps active appearance")
	app._import_skin(ProjectSettings.globalize_path("res://work/adapter-fixtures/image.gif"))
	while app.import_worker != null:
		await process_frame
	var gif_id: String = app.prefs.skin_id
	check(gif_id != fixture.skin.id and app.avatar.capability.frames == 4, "Successful asynchronous GIF import switches live appearance")
	app._switch_skin(original)
	app.settings.hide()
	SkinOverrides.reset(fixture.skin.id)
	AssetImporter._remove_created(SkinCatalog.USER_ROOT.path_join(fixture.skin.id))
	AssetImporter._remove_created(SkinCatalog.USER_ROOT.path_join(gif_id))
	app.set_process(false)
	app.set_physics_process(false)
	app.queue_free()
	await process_frame
	if not old_preferences.is_empty():
		var file := FileAccess.open(prefs_path, FileAccess.WRITE)
		file.store_buffer(old_preferences)
	print("ADAPTATION UI TESTS: ", checks, " checks, ", failures, " failures")
	quit(1 if failures else 0)
