extends SceneTree

const Model = preload("res://scripts/avatars/model_avatar.gd")
var checks := 0
var failures := 0

func _initialize() -> void:
	call_deferred("run")

func check(condition: bool, message: String) -> void:
	checks += 1
	if not condition:
		failures += 1
		push_error(message)

func run() -> void:
	for invalid in [null, [], "high", 3]:
		check(PetRenderOptions.normalize(invalid) == PetRenderOptions.DEFAULTS, "Invalid performance section uses safe defaults")
	check(PetRenderOptions.normalize({"resolution": -2, "fps": 8000, "quality": "high", "aa": "TAA", "ao": "yes"}) == PetRenderOptions.DEFAULTS, "Reject unsupported performance values")
	var prefs := PetPreferences.new()
	prefs.path = "user://test-performance-prefs.json"
	prefs.performance = PetRenderOptions.PRESETS.high.duplicate()
	prefs.performance.fps = 144
	check(prefs.save() == OK, "Save performance settings")
	var restored := PetPreferences.new()
	restored.path = prefs.path
	restored.load_saved()
	check(restored.performance == prefs.performance, "All performance settings round-trip")
	DirAccess.remove_absolute(prefs.path)
	var catalog := SkinCatalog.new()
	catalog.scan()
	var avatar := Model.new()
	root.add_child(avatar)
	check(avatar.configure(catalog.find_skin("builtin.shu")).is_empty(), "Configure renderer")
	avatar.set_process(false)
	var ground: float = avatar.ground_offset
	var camera_size: float = avatar.viewport.get_camera_3d().size
	var sprite: Sprite2D = avatar.visual.get_child(0)
	for resolution in PetRenderOptions.RESOLUTIONS:
		avatar.apply_performance({"resolution": resolution})
		check(avatar.viewport.size == Vector2i.ONE * resolution, "Apply resolution " + str(resolution))
		check(absf(sprite.scale.x * resolution - 300.0) < 0.001 and is_equal_approx(avatar.ground_offset, ground) and is_equal_approx(avatar.viewport.get_camera_3d().size, camera_size), "Resolution preserves displayed size, camera and floor")
	for aa in PetRenderOptions.ANTIALIASING:
		avatar.apply_performance({"aa": aa})
		check(avatar.viewport.msaa_3d == {"msaa2": Viewport.MSAA_2X, "msaa4": Viewport.MSAA_4X, "msaa8": Viewport.MSAA_8X}.get(aa, Viewport.MSAA_DISABLED), "Apply MSAA independently: " + aa)
		check((avatar.viewport.screen_space_aa == Viewport.SCREEN_SPACE_AA_FXAA) == (aa == "fxaa"), "FXAA matches selection without accidental double AA")
	var fur: MeshInstance3D
	for mesh in avatar.pivot.find_children("*", "MeshInstance3D", true, false):
		if str(mesh.name).contains("MicroPile") and mesh.mesh:
			fur = mesh
			break
	check(fur != null, "Locate actual fur geometry")
	avatar.apply_performance(PetRenderOptions.PRESETS.power)
	check(not fur.visible and not avatar.key_light.shadow_enabled, "Power preset suppresses expensive fur and shadows")
	avatar.apply_performance(PetRenderOptions.PRESETS.high)
	check(fur.visible and avatar.key_light.shadow_enabled, "High preset restores fur and enables real shadows")
	check(avatar.render_environment.ssao_enabled == PetRenderOptions.supports_ao(), "SSAO follows renderer capability")
	for fps in PetRenderOptions.FRAME_RATES:
		PetRenderOptions.apply_global({"fps": fps})
		check(Engine.max_fps == fps, "Apply frame rate limit " + str(fps))
	PetRenderOptions.apply_global(PetRenderOptions.DEFAULTS)
	avatar.free()
	print("PERFORMANCE TESTS: ", checks, " checks, ", failures, " failures")
	quit(1 if failures else 0)
