extends SceneTree

const Renderer = preload("res://scripts/avatars/model_avatar.gd")
var failures := 0
var checks := 0

func _initialize() -> void:
	call_deferred("run")

func check(condition: bool, message: String) -> void:
	checks += 1
	if not condition:
		failures += 1
		push_error(message)

func validate(options: Dictionary) -> String:
	return SkinCatalog.validate_options({"physics": {"soft_bodies": [options]}})

func skin_point(binding: Dictionary, index: int, surface: int = 0) -> Vector3:
	var arrays: Array = binding.proxy.mesh.surface_get_arrays(surface)
	var point: Vector3 = arrays[Mesh.ARRAY_VERTEX][index]
	var skeleton := binding.proxy.get_parent() as Skeleton3D
	var result := Vector3.ZERO
	for j in 4:
		var slot := index * 4 + j
		var bone: int = arrays[Mesh.ARRAY_BONES][slot]
		result += (skeleton.get_bone_global_pose(bone) * binding.skin.get_bind_pose(bone) * point) * arrays[Mesh.ARRAY_WEIGHTS][slot]
	return result

func support_minimum(body) -> float:
	var lowest := INF
	for binding in body.bindings:
		for surface in binding.proxy.mesh.get_surface_count():
			for point: Vector3 in binding.proxy.mesh.surface_get_arrays(surface)[Mesh.ARRAY_VERTEX]:
				lowest = minf(lowest, body.deform_point(point).y)
	return lowest

func run() -> void:
	var catalog := SkinCatalog.new()
	catalog.scan()
	var skin := catalog.find_skin("builtin.shu")
	var options: Dictionary = skin.physics.soft_bodies[0]
	check(validate(options).is_empty(), "Accept Shu supported soft-body declaration")
	check(SkinCatalog.validate_options({}).is_empty(), "Physics remains optional for other skins")
	for pair in [["axis", [0, 0, 0]], ["axis", [0, 1, 0]], ["axis", [0, NAN, 1]], ["node", "../other"], ["profile_mesh", "/outside"], ["anchors", ["../foot"]], ["sections", 3], ["sections", 49], ["sections", 5.5], ["length", 0], ["stiffness", INF], ["compression", 0.9], ["floor_height", 5.0]]:
		var invalid := options.duplicate(true)
		invalid[pair[0]] = pair[1]
		check(not validate(invalid).is_empty(), "Reject invalid soft-body setting " + str(pair))
	var avatar := Renderer.new()
	root.add_child(avatar)
	check(avatar.configure(skin).is_empty(), "Configure actual Shu with soft-body physics")
	avatar.set_process(false)
	avatar.player.stop()
	avatar._restore_pose()
	check(avatar.soft_bodies.size() == 1, "Install one whole-body lattice")
	if avatar.soft_bodies.is_empty():
		quit(1)
		return
	var body = avatar.soft_bodies[0]
	body.active = false
	body.advance(1.0 / 120.0)
	var skeleton := body.get_skeleton() as Skeleton3D
	check(skeleton.get_bone_count() == 28 and not avatar.rest_bones.has(skeleton), "Soft sections stay independent of native animation and procedural rigs")
	var independent := true
	for i in skeleton.get_bone_count():
		independent = independent and skeleton.get_bone_parent(i) == -1
	check(independent, "Sections compress independently without a rigid rear chain")
	for point in [Vector3(0, 0.8, 0.5), Vector3(0.5, -0.6, 0.1), Vector3(0, 0, 0)]:
		check(body.deform_point(point).is_equal_approx(point), "Keep front body and face anchored")
	for z in [-1.25, -1.6, -1.95]:
		var i: int = roundi(body.coordinate(Vector3(0, 0, z)))
		var bottom: Vector3 = body.deform_point(Vector3(0, body.lower[i], body.station(i) * -1.0))
		check(bottom.y > -0.9 and bottom.y < -0.76, "Rear abdomen settles to a nearly level support at " + str(z))
	var rear := roundi(body.coordinate(Vector3(0, 0, -1.6)))
	var top := Vector3(0, body.upper[rear], -body.station(rear))
	check(body.deform_point(top).y < top.y - 0.5, "Rear white body sinks substantially along with the tail")
	check(body.deform_point(top).y - body.deform_point(Vector3(0, body.lower[rear], top.z)).y < (body.upper[rear] - body.lower[rear]) * 0.92, "Rear cross-section compresses as its filling settles")
	for name in ["FrontFoot_L", "FrontFoot_R", "RearFoot_L", "RearFoot_R"]:
		var foot := avatar.pivot.find_children(name, "Node3D", true, false)[0]
		var supported := true
		for instance in foot.find_children("*", "MeshInstance3D", true, false):
			supported = supported and instance.mesh != null and instance.skin == null
		check(supported, "Keep support foot and its fur together: " + name)
	var detail_names: Array[String] = []
	var gpu_agrees := true
	var tail_binding: Dictionary = {}
	for binding in body.bindings:
		detail_names.append(str(binding.mesh.name))
		var point: Vector3 = binding.proxy.mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX][0]
		gpu_agrees = gpu_agrees and skin_point(binding, 0).distance_to(body.deform_point(point)) < 0.0001
		if binding.mesh.name == "TailPanel_Outer":
			tail_binding = binding
	check(gpu_agrees, "GPU skin matrices reproduce the soft-body deformation including affine shear")
	for name in ["BodyShell • 横向水滴", "BodyShell • 横向水滴_MicroPile", "ContinuousDorsalFin", "DorsalRidgeSeam", "TailPanel_Outer", "TailPanel_Outer_MicroPile"]:
		check(name in detail_names, "Body surface detail shares the continuous lattice: " + name)
	check(support_minimum(body) >= body.floor_height - 0.0001, "Resting body, tail and fur remain above the floor")
	body.sway = Vector3(0.04, -0.12, 0.02)
	body.load_amount = 1.2
	body._apply()
	check(support_minimum(body) >= body.floor_height - 0.0001, "Floor support also constrains dynamic overshoot and downward sway")
	body.reset_simulation()
	body._apply()
	if not tail_binding.is_empty():
		var before := skin_point(tail_binding, 10)
		avatar.player.play("Walk", 0.0)
		avatar.player.seek(0.25, true)
		avatar.player.advance(0.0)
		avatar.player.pause()
		body.advance(1.0 / 60.0)
		check(skin_point(tail_binding, 10).distance_to(before) > 0.001, "Native tail wag remains visible through the soft skin")
	avatar.player.stop()
	avatar._restore_pose()
	avatar.secondary_grounded = false
	body.reset_simulation()
	avatar.secondary_offset.x += 0.1
	body.advance(1.0 / 60.0)
	check(body.sway.x < 0, "Rear filling lags behind desktop acceleration")
	for i in 480:
		body.advance(1.0 / 120.0)
	check(body.sway.length() < 0.0001 and absf(body.load_amount - 1.0) < 0.0001, "Damping returns the body to a stable settled shape")
	for fps: int in [30, 60, 120]:
		body.reset_simulation()
		var step := 1.0 / fps
		for i in fps:
			var time := float(i + 1) * step
			avatar.secondary_offset.y = -0.5 * avatar.desktop_gravity() * time * time
			body.advance(step)
		check(body.load_amount < 0.06 and body.sway.is_finite(), "Free fall unloads the filling at " + str(fps) + " FPS")
	avatar.reset_secondary_motion()
	body.advance(1.0 / 60.0)
	check(body.sway.is_zero_approx() and is_equal_approx(body.load_amount, 1.0), "Screen relocation resets accumulated inertia")
	avatar.update_desktop_motion(Vector2(100, 100), 1.0)
	avatar.update_desktop_motion(Vector2(110, 90), 1.0)
	check(avatar.secondary_offset.x > 0 and avatar.secondary_offset.y > 0, "Desktop coordinates map to model motion")
	var generation := avatar.secondary_generation
	avatar.update_desktop_motion(Vector2(110, 90), 3.0)
	check(avatar.secondary_generation > generation and avatar.secondary_offset.is_zero_approx(), "Zoom changes reset motion without a spurious impulse")
	body.advance(1.0)
	check(body.sway.is_zero_approx(), "A stalled frame resets the simulation safely")
	var pristine := (load("res://skins/shu/shu.glb") as PackedScene).instantiate()
	root.add_child(pristine)
	var source: MeshInstance3D = pristine.find_children("BodyShell • 横向水滴", "MeshInstance3D", true, false)[0]
	check(source.mesh != null and source.skin == null and source.mesh.surface_get_arrays(0)[Mesh.ARRAY_BONES] == null, "Installing soft physics never mutates cached GLB mesh resources")
	pristine.free()
	var original := Renderer.new()
	root.add_child(original)
	var unmodified := skin.duplicate(true)
	unmodified.erase("physics")
	check(original.configure(unmodified).is_empty(), "Keep non-physical GLB loading available")
	check(is_equal_approx(original.ground_offset, avatar.ground_offset), "Generated culling bounds do not move the desktop contact height")
	original.free()
	avatar.free()
	print("SOFT BODY TESTS: ", checks, " checks, ", failures, " failures")
	quit(1 if failures else 0)
