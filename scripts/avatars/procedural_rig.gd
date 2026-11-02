extends SkeletonModifier3D

## Conservative additive motion for recognized rig parts. Native state clips
## take precedence. Unknown bones stay untouched; no guessed skin weights.
var actor: PetAvatar
var parts: Array[Dictionary] = []

func setup(avatar: PetAvatar, skeleton: Skeleton3D) -> void:
	actor = avatar
	for i in skeleton.get_bone_count():
		var tokens := ActionAdapter.words(skeleton.get_bone_name(i))
		var name := "".join(tokens)
		var role := ""
		for candidate in [
			["calf", ["lowerleg", "leftleg", "rightleg", "calf", "shin", "小腿", "ひざ"]],
			["leg", ["upperleg", "upleg", "thigh", "大腿", "太もも"]],
			["forearm", ["lowerarm", "forearm", "前腕", "ひじ"]],
			["arm", ["upperarm", "leftarm", "rightarm", "上腕"]],
			["head", ["head", "頭"]], ["spine", ["spine", "chest", "脊柱", "上半身"]],
			["tail", ["tail", "尾", "しっぽ"]]]:
			for hint: String in candidate[1]:
				if name.contains(hint) and not name.ends_with("end") and not name.ends_with("tip"):
					role = candidate[0]
					break
			if not role.is_empty():
				break
		if not role.is_empty():
			var side := -1.0 if name.contains("left") or "l" in tokens or name.contains("左") else 1.0
			parts.append({"bone": i, "role": role, "side": side})

func _process_modification() -> void:
	if not is_instance_valid(actor) or actor.native_action:
		return
	var skeleton := get_skeleton()
	if skeleton == null:
		return
	var time := actor.elapsed
	var blend := minf(time * 6.0, 1.0)
	var running := actor.state == "run"
	var locomotion := actor.state in ["walk", "run"]
	var stride := sin(time * (10.75 if running else 5.25))
	for part in parts:
		var angle := 0.0
		var axis := Vector3.RIGHT
		match part.role:
			"leg":
				if locomotion:
					angle = stride * part.side * (0.43 if running else 0.25)
				elif actor.state == "sleep":
					angle = 0.08
			"calf":
				if locomotion:
					angle = maxf(0, -stride * part.side) * (0.52 if running else 0.25)
				elif actor.state == "sleep":
					angle = 0.12
			"arm", "forearm":
				if locomotion:
					angle = -stride * part.side * (0.28 if running else 0.14)
				elif actor.state == "petted":
					angle = sin(time * 10) * part.side * 0.14
				elif actor.state in ["dragged", "airborne"]:
					angle = -0.1
			"head":
				angle = sin(time * 8) * exp(-time) * 0.12 if actor.state == "petted" else 0.14 if actor.state == "sleep" else sin(time * 1.7) * 0.012
			"spine":
				angle = 0.07 if actor.state == "sleep" else -0.045 + stride * 0.012 if running else sin(time * 2.2) * 0.008
			"tail":
				axis = Vector3.UP
				angle = sin(time * (10.75 if running else 7 if actor.state == "petted" else 3)) * (0.23 if running else 0.16 if actor.state == "walk" else 0.07)
		if actor.state == "fall" and part.role in ["calf", "spine"]:
			angle += sin(time * 12) * exp(-time * 5) * 0.14
		if absf(angle) < 0.00001:
			continue
		# Convert world axes through each bone's rest basis, accommodating common
		# Blender / FBX / VRM bone-axis conventions and the user's orientation.
		var basis := (skeleton.global_transform.basis * skeleton.get_bone_global_rest(part.bone).basis).orthonormalized()
		var local_axis := (basis.inverse() * axis).normalized()
		var pose := skeleton.get_bone_pose_rotation(part.bone)
		skeleton.set_bone_pose_rotation(part.bone, pose * Quaternion(local_axis, angle * blend))
