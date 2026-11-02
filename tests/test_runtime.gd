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

func sample_clip(avatar: Node, clip: String, time: float) -> void:
	# Remove cross-fades and previous-state transforms before evaluating actual keys.
	avatar.player.stop()
	avatar._restore_pose()
	avatar.player.play(clip, 0.0)
	avatar.player.seek(time, true)
	avatar.player.advance(0.0)
	avatar.player.pause()

func run() -> void:
	var catalog := SkinCatalog.new()
	catalog.scan()
	var shu := Renderer.new()
	root.add_child(shu)
	var shu_error := shu.configure(catalog.find_skin("builtin.shu"))
	check(shu_error.is_empty(), "Load built-in Shu GLB: " + shu_error)
	check(shu.ground_offset > 180 and shu.ground_offset < 306, "Shu feet stay inside the desktop window")
	var meshes := shu.pivot.find_children("*", "MeshInstance3D", true, false)
	check(not meshes.is_empty(), "Plush Shu contains renderable geometry")
	for part in ["PlushBody", "FrontFoot", "RearFoot", "ForkedHorn", "ContinuousDorsalFin", "TailWag", "PendantSway", "BlinkBothEyes", "ClosedEyelids"]:
		check(not shu.pivot.find_children(part + "*", "Node3D", true, false).is_empty(), "Preserve editable plush part " + part)
	for clip in ["Idle", "Walk", "Run", "Turn", "Sleep", "Happy", "PickedUp", "Land"]:
		check(shu.player != null and shu.player.has_animation(clip), "Import plush action " + clip)
	var bounds := shu._bounds(shu.pivot, shu.pivot.get_parent())
	check(bounds.size.is_finite() and bounds.size.y > 1.0, "Normalize plush model to finite visible bounds")
	var expected_clips := {"idle": "Idle", "walk": "Walk", "run": "Run", "sleep": "Sleep", "petted": "Happy", "dragged": "PickedUp", "airborne": "PickedUp", "fall": "Land"}
	for state in ActionAdapter.STATES:
		shu.set_state(state)
		shu._process(0.12)
		check(shu.state == state and shu.visual.scale.is_finite(), "Plush supports state " + state)
		check(shu.mapping.get(state, "") == expected_clips[state] and shu.native_action and shu.player != null and shu.player.current_animation == expected_clips[state], "Play dedicated plush action for " + state)
	var eye_parts := shu.pivot.find_children("BlinkBothEyes*", "Node3D", true, false)
	var body_parts := shu.pivot.find_children("PlushBody*", "Node3D", true, false)
	var front_feet := shu.pivot.find_children("FrontFoot*", "Node3D", true, false)
	if shu.player != null and not eye_parts.is_empty() and not body_parts.is_empty() and not front_feet.is_empty():
		var eyes := eye_parts[0] as Node3D
		var body := body_parts[0] as Node3D
		var foot := front_feet[0] as Node3D
		sample_clip(shu, "Sleep", 0.3)
		check(is_equal_approx(eyes.scale.y, 0.04), "Sleep keys close plush eyes on the imported Y axis")
		var closed_lids := shu.pivot.find_children("ClosedEyelids*", "Node3D", true, false)
		check(not closed_lids.is_empty() and is_equal_approx(closed_lids[0].scale.x, 1.0), "Sleep displays the embroidered closed eyelid strokes")
		check(body.scale.y < 0.95 and body.scale.y > 0.9, "Sleep keys compress the actual 3D body")
		sample_clip(shu, "Walk", 0.1)
		var first_foot_pose := foot.transform
		sample_clip(shu, "Walk", 0.35)
		check(not foot.transform.is_equal_approx(first_foot_pose), "Walk keys move an actual front foot over time")
		sample_clip(shu, "Idle", 0.1)
		check(is_equal_approx(eyes.scale.y, 1.0), "Idle restores open eyes after the Sleep clip")
		check(not closed_lids.is_empty() and closed_lids[0].scale.x < 0.01, "Idle hides the sleeping eyelid strokes")
	shu.set_state("dragged")
	shu._process(0.1)
	check(shu.visual.scale.y > 1.0, "Dragging retains responsive plush stretch")
	shu.set_state("sleep")
	shu._process(0.1)
	check(shu.visual.scale.y < 1.0, "Sleeping retains gentle plush squash")
	shu.set_state("idle")
	shu.impact(2000)
	shu._process(0.01)
	check(shu.visual.scale.y < 1.0, "Landing impact combines with native plush animation")
	shu.queue_free()
	# A new ID avoids touching any real user skin. Only these three fixture files
	# are removed at the end; never recurse through the user's skin directory.
	var id := "test.external." + str(Time.get_ticks_usec())
	var source := "user://test-fixtures/runtime"
	DirAccess.make_dir_recursive_absolute(source)
	DirAccess.copy_absolute("res://skins/glb-demo/pet.glb", source.path_join("pet.glb"))
	var data := {"schema_version": 1, "id": id, "name": "Runtime fixture", "type": "model3d",
		"asset": "pet.glb", "animations": {"idle": "Breathe"}}
	var file := FileAccess.open(source.path_join("skin.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(data))
	file.close()
	var result := catalog.import_manifest(source.path_join("skin.json"))
	check(result.ok, "Import external GLB manifest and copy model")
	if result.ok:
		var avatar := Renderer.new()
		root.add_child(avatar)
		var error := avatar.configure(result.skin)
		check(error.is_empty(), "Decode external GLB at runtime: " + error)
		check(avatar.player != null and avatar.player.has_animation("Breathe"), "Discover embedded animation")
		avatar.set_state("walk")
		check(avatar.player != null and avatar.player.current_animation == "Breathe", "Missing walk animation falls back to mapped idle")
		await create_timer(0.1).timeout
		avatar.queue_free()
		var installed := SkinCatalog.USER_ROOT.path_join(id)
		DirAccess.remove_absolute(installed.path_join("skin.json"))
		DirAccess.remove_absolute(installed.path_join(result.skin.asset))
		DirAccess.remove_absolute(installed)
	var invalid := FileAccess.open(source.path_join("broken.glb"), FileAccess.WRITE)
	invalid.store_string("invalid GLB")
	invalid.close()
	var renderer := Renderer.new()
	root.add_child(renderer)
	check(not renderer._validate_glb(source.path_join("broken.glb")).is_empty(), "Reject malformed GLB before decoding")
	renderer.queue_free()
	await process_frame
	print("RUNTIME TESTS: ", checks, " checks, ", failures, " failures")
	quit(1 if failures else 0)
