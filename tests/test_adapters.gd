extends SceneTree

## Run with a graphics renderer: FBX morph re-export requires mesh GPU data.

const Model = preload("res://scripts/avatars/model_avatar.gd")
const Sprite = preload("res://scripts/avatars/sprite_avatar.gd")
const FIXTURES := "res://work/adapter-fixtures"
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
	root.size = Vector2i(64, 64)
	root.position = Vector2i(-2000, -2000)
	var names := PackedStringArray(["Armature|Idle", "mixamo.com|Walking", "Sleeping", "Happy", "PickedUp", "Falling", "Landing", "RESET"])
	var matched := ActionAdapter.resolve(names)
	for state in ActionAdapter.STATES:
		check(matched.has(state), "Resolve named action " + state)
	check(ActionAdapter.resolve(PackedStringArray(["Runway", "Landscape", "Sleepless", "falling"])).keys() == ["airborne"], "Do not confuse embedded words or falling with landing")
	check(ActionAdapter.resolve(PackedStringArray(["待机", "行走", "跑步", "睡眠", "摸摸", "提起", "下落", "落地"])).size() == 8, "Match Chinese action names")
	check(not ActionAdapter.resolve(names, {"walk": ""}).has("walk"), "Explicit generic mode suppresses automatic matching")
	check(ActionAdapter.resolve(names, {"idle": "Happy"}).idle == "Happy", "Explicit clip overrides name heuristic")
	check(ActionAdapter.profile_for("model3d", Vector3(3, 2, 4), PackedStringArray()) == "blob", "Wide dragon uses soft profile")
	check(ActionAdapter.profile_for("model3d", Vector3.ONE, PackedStringArray(["mixamorig:Hips", "Head"])) == "character", "Detect humanoid bones")
	check(ActionAdapter.profile_for("sprite", Vector3.ONE, PackedStringArray()) == "flat", "Images use planar profile")
	for bad in [{"grid": [0, 1]}, {"grid": [2.5, 1]}, {"clips": {"Idle": {"frames": [0], "fps": 0}}}, {"clips": {"Idle": {"frames": [0], "durations": []}}}, {"frames": ["../outside.png"]}]:
		check(not SkinCatalog.validate_options(bad).is_empty(), "Reject invalid frame metadata: " + str(bad))
	check(not AssetIO.validate_gltf(FIXTURES.path_join("unsafe.gltf"), true).is_empty(), "Reject escaping glTF dependency")
	var decoded := GifReader.new().decode(FIXTURES.path_join("image.gif"))
	check(decoded.error.is_empty(), "Decode real GIF: " + decoded.error)
	if decoded.error.is_empty():
		check(decoded.images.size() == 4, "GIF retains all frames")
		for i in decoded.images.size():
			decoded.images[i].save_png("res://work/adapter-fixtures/gif-decoded-%d.png" % i)
			var expected := Image.load_from_file(ProjectSettings.globalize_path(FIXTURES.path_join("gif-expected-%d.png" % i)))
			expected.convert(Image.FORMAT_RGBA8)
			check(expected.get_data() == decoded.images[i].get_data(), "GIF compositing matches Pillow oracle frame %d" % i)
		check(is_equal_approx(decoded.durations[2], 0.16), "GIF retains per-frame duration")
	var noise := GifReader.new().decode(FIXTURES.path_join("noise.gif"))
	check(noise.error.is_empty(), "Decode large GIF LZW dictionary")
	if noise.error.is_empty():
		var expected := Image.load_from_file(ProjectSettings.globalize_path(FIXTURES.path_join("noise-expected.png")))
		expected.convert(Image.FORMAT_RGBA8)
		check(expected.get_data() == noise.images[0].get_data(), "GIF interlace and code widths preserve pixels")
	check(not GifReader.new().decode(FIXTURES.path_join("bad.gif")).error.is_empty(), "Reject truncated GIF")
	var catalog := SkinCatalog.new()
	catalog.scan()
	for file in ["image.png", "image.jpg", "image.jpeg", "image.webp", "image.bmp", "image.tga", "image.svg", "image.gif", "sheet.json", "sequence.json", "external.gltf", "pet.vrm", "pet.obj", "actions.glb", "rig.glb", "chibi.fbx", "chibi.blend", "pet.stl", "pet.ply"]:
		var result := catalog.import_file(ProjectSettings.globalize_path(FIXTURES.path_join(file)))
		check(result.ok, "Import " + file + ": " + result.error)
		if not result.ok:
			continue
		var skin: Dictionary = result.skin
		var avatar: PetAvatar = Model.new() if skin.type == "model3d" else Sprite.new()
		root.add_child(avatar)
		var error := avatar.configure(skin)
		check(error.is_empty(), "Render installed " + file + ": " + error)
		check(avatar.ground_offset > 146 and avatar.ground_offset < 280, "Stable ground anchor for " + file)
		for state: String in ActionAdapter.STATES:
			avatar.set_state(state)
			avatar._process(0.15)
			check(avatar.state == state and avatar.visual.scale.is_finite() and avatar.visual.position.is_finite(), file + " handles " + state)
		if file == "sheet.json" or file == "sequence.json":
			check(avatar.available_clips.size() == 3 and avatar.mapping.walk == "Walk", "2D sheet / sequence matches clips")
			avatar.set_state("walk")
			var first: Texture2D = avatar.sprite.texture
			avatar._process(0.15)
			check(first != avatar.sprite.texture, "2D frame playback advances")
			avatar.set_state("petted")
			avatar._process(10)
			check(avatar.sprite.texture == avatar.frames[3], "2D one-shot holds last frame")
		if file == "actions.glb":
			check(avatar.mapping.size() == 8, "Imported GLB matches native actions with a running fallback")
			avatar.set_state("walk")
			var animation: Animation = avatar.player.get_animation(avatar.mapping.walk)
			for track in animation.get_track_count():
				if animation.track_get_type(track) == Animation.TYPE_POSITION_3D:
					var a: Vector3 = animation.track_get_key_value(track, 0)
					var b: Vector3 = animation.track_get_key_value(track, animation.track_get_key_count(track) - 1)
					check(is_equal_approx(a.x, b.x) and is_equal_approx(a.z, b.z), "Root translation cannot drift out of desktop viewport")
			avatar.player.advance(0.5)
			avatar.explicit_mapping["sleep"] = ""
			avatar.mapping.erase("sleep")
			avatar.set_state("sleep")
			check(not avatar.native_action and not avatar.player.is_playing(), "Generic override stops previous clip")
			var restored := true
			for node: Node3D in avatar.rest_nodes:
				restored = restored and node.transform.is_equal_approx(avatar.rest_nodes[node])
			check(restored, "State change restores base transform")
		if file in ["rig.glb", "chibi.fbx", "chibi.blend"]:
			check(avatar.capability.bones >= 2 and avatar.action_profile == "character", "Keep actual rig and detect character for " + file)
			check(avatar.mapping.has("walk"), "Keep native skeletal walk for " + file)
			check(avatar.expressions.size() >= 2, "Keep morph target names for " + file)
			var textured := false
			for mesh in avatar.pivot.find_children("*", "MeshInstance3D", true, false):
				if mesh.mesh:
					for surface in mesh.mesh.get_surface_count():
						var material: Material = mesh.get_active_material(surface)
						textured = textured or (material is BaseMaterial3D and material.albedo_texture != null)
			check(textured, "Keep embedded texture for " + file)
		if file == "rig.glb":
			check(avatar.capability.rig_parts >= 6, "Recognize head, arms, legs and tail")
			check(avatar.expressions.size() >= 2, "Recognize common blink and smile morphs")
			avatar.mapping.clear()
			avatar.explicit_mapping.clear()
			avatar.set_state("sleep")
			avatar._process(0.3)
			for expression in avatar.expressions:
				if expression.role == "blink":
					check(is_equal_approx(expression.mesh.get_blend_shape_value(expression.index), 1.0), "Generic sleep closes recognized eye morph")
			avatar.set_state("petted")
			avatar._process(0.3)
			for expression in avatar.expressions:
				if expression.role == "smile":
					check(expression.mesh.get_blend_shape_value(expression.index) > 0.5, "Generic interaction smiles using morph")
			avatar.set_state("walk")
			avatar.elapsed = 0.35
			var moved := false
			for modifier in avatar.pivot.find_children("*", "SkeletonModifier3D", true, false):
				if modifier.get_script() != Model.RigMotion:
					continue
				modifier._process_modification()
				var skeleton: Skeleton3D = modifier.get_skeleton()
				for part in modifier.parts:
					if part.role in ["leg", "arm", "tail"]:
						moved = moved or skeleton.get_bone_pose_rotation(part.bone).angle_to(avatar.rest_bones[skeleton][part.bone].basis.get_rotation_quaternion()) > 0.01
			check(moved, "Generic walk actually moves recognized skeletal limbs")
			avatar.set_state("idle")
			for expression in avatar.expressions:
				check(is_equal_approx(expression.mesh.get_blend_shape_value(expression.index), expression.base), "Leaving generic reaction restores morph baseline")
			var measured := {"frames": 0, "angle": 0.0}
			for modifier in avatar.pivot.find_children("*", "SkeletonModifier3D", true, false):
				if modifier.get_script() != Model.RigMotion:
					continue
				for part in modifier.parts:
					if part.role != "leg":
						continue
					modifier.modification_processed.connect(func():
						var skeleton: Skeleton3D = modifier.get_skeleton()
						var base: Quaternion = avatar.rest_bones[skeleton][part.bone].basis.get_rotation_quaternion()
						measured.frames += 1
						measured.angle = maxf(measured.angle, skeleton.get_bone_pose_rotation(part.bone).angle_to(base)))
			avatar.set_state("walk")
			await create_timer(0.8).timeout
			check(measured.frames > 5 and measured.angle > 0.05, "Skeleton modifier produces visible motion across rendered frames")
			check(measured.angle < 0.35, "Procedural bone rotations do not accumulate over frames")
			var walking_angle: float = measured.angle
			measured.frames = 0
			measured.angle = 0.0
			avatar.set_state("run")
			await create_timer(0.8).timeout
			check(measured.frames > 5 and measured.angle > walking_angle * 1.4, "Generic running produces a larger limb stride than walking")
			check(measured.angle < 0.5, "Running bone rotations remain bounded across rendered frames")
		if file == "pet.obj":
			var mesh: MeshInstance3D = avatar.pivot.find_children("*", "MeshInstance3D", true, false)[0]
			check(mesh.mesh.surface_get_material(0).albedo_texture != null, "OBJ MTL texture embeds in installed GLB")
		if file in ["pet.stl", "pet.ply"]:
			check(avatar.pivot.find_children("*", "MeshInstance3D", true, false).size() == 1, "Conversion excludes Blender's startup cube for " + file)
		avatar.queue_free()
		await process_frame
		AssetImporter._remove_created(SkinCatalog.USER_ROOT.path_join(skin.id))
		catalog.scan()
	for file in ["bad.gif", "bad.obj", "unsafe.gltf"]:
		var before := DirAccess.get_directories_at(SkinCatalog.USER_ROOT).size()
		var result := catalog.import_file(ProjectSettings.globalize_path(FIXTURES.path_join(file)))
		check(not result.ok, "Reject broken import " + file)
		check(before == DirAccess.get_directories_at(SkinCatalog.USER_ROOT).size(), "Failed import removes incomplete folder for " + file)
	# Run the same pipeline off the UI thread as normal imports.
	var worker := Thread.new()
	worker.start(func():
		var importer := SkinCatalog.new()
		importer.scan()
		return importer.import_file(ProjectSettings.globalize_path(FIXTURES.path_join("rig.glb"))))
	while worker.is_alive():
		await process_frame
	var threaded: Dictionary = worker.wait_to_finish()
	check(threaded.ok, "Background import completes: " + threaded.error)
	if threaded.ok:
		AssetImporter._remove_created(SkinCatalog.USER_ROOT.path_join(threaded.skin.id))
	print("ADAPTER TESTS: ", checks, " checks, ", failures, " failures")
	quit(1 if failures else 0)
