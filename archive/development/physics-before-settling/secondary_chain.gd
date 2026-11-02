extends SkeletonModifier3D

## Fixed-step, length-constrained particles drive a generated GPU skin.
## Simulation lives in render-world space plus desktop translation, so motion
## stays continuous across native clips and reacts to dragging / landing.
const STEP := 1.0 / 120.0
var actor: PetAvatar
var axis := Vector3.FORWARD
var length := 1.0
var segments := 6
var gravity := 5.0
var stiffness := 22.0
var damping := 5.0
var max_angle := deg_to_rad(75.0)
var points := PackedVector3Array()
var previous := PackedVector3Array()
var accumulator := 0.0
var generation := -1
var floor_height := -1.0
var last_origin := Vector3.ZERO
var bindings: Array[Dictionary] = []
var probes: Dictionary = {}
var probe_extents: Dictionary = {}
var mesh_frames: Dictionary = {}

func add_probe(mesh: MeshInstance3D, vertex: Vector3, position: Vector3, bone: int, blend: float) -> void:
	# A small support hull sampled across every surface, shared across meshes.
	# Includes body, seams, fins and fur without per-frame CPU mesh deformation.
	for dimension in 3:
		for sign_value in [-1, 1]:
			var key := bone * 6 + dimension * 2 + (1 if sign_value > 0 else 0)
			var value: float = position[dimension] * sign_value
			if not probe_extents.has(key) or value > probe_extents[key]:
				probe_extents[key] = value
				probes[key] = {"mesh": mesh, "vertex": vertex, "bone": bone, "blend": blend}

func setup(avatar: PetAvatar, options: Dictionary, span: float, floor_y: float) -> void:
	actor = avatar
	axis = Vector3(options.axis[0], options.axis[1], options.axis[2]).normalized()
	segments = int(options.get("segments", 6))
	length = span
	gravity = float(options.get("gravity", 5.0))
	stiffness = float(options.get("stiffness", 22.0))
	damping = float(options.get("damping", 5.0))
	max_angle = deg_to_rad(float(options.get("max_angle", 75.0)))
	floor_height = floor_y

func reset_simulation() -> void:
	var skeleton := get_skeleton()
	var frame := skeleton.global_transform
	points.resize(segments + 1)
	for i in points.size():
		points[i] = frame * (axis * length * i / segments) + actor.secondary_offset
	previous = points.duplicate()
	last_origin = points[0]
	accumulator = 0.0
	generation = actor.secondary_generation

func _process_modification() -> void:
	if not is_instance_valid(actor):
		return
	advance(get_process_delta_time())

func advance(delta: float) -> void:
	var skeleton := get_skeleton()
	if skeleton == null:
		return
	var frame := skeleton.global_transform
	mesh_frames.clear()
	for binding in bindings:
		var relative: Transform3D = frame.affine_inverse() * binding.mesh.global_transform
		mesh_frames[binding.mesh] = relative
		binding.proxy.visible = binding.mesh.is_visible_in_tree()
		for i in segments:
			binding.skin.set_bind_pose(i, skeleton.get_bone_global_rest(i).affine_inverse() * relative * binding.inverse_rest)
	var origin := frame.origin + actor.secondary_offset
	if points.is_empty() or generation != actor.secondary_generation or delta > 0.25 or origin.distance_to(last_origin) > 6.0:
		reset_simulation()
	last_origin = origin
	var direction := (frame.basis * axis).normalized()
	var step_length := (frame.basis * (axis * length / segments)).length()
	accumulator += clampf(delta, 0.0, 0.1)
	while accumulator >= STEP:
		# The first segment remains sewn to the body. Softness grows toward the tip.
		points[0] = origin
		points[1] = origin + direction * step_length
		previous[0] = points[0]
		previous[1] = points[1]
		for i in range(2, points.size()):
			var velocity := (points[i] - previous[i]) * exp(-damping * STEP)
			previous[i] = points[i]
			var target := points[i - 1] + direction * step_length
			var support := 1.0 + 3.0 * pow(1.0 - float(i - 1) / (segments - 1), 2)
			var acceleration := Vector3.DOWN * gravity + (target - points[i]) * stiffness * support
			points[i] += velocity + acceleration * STEP * STEP
		# Forward projection preserves every segment length and limits foldback.
		for i in range(2, points.size()):
			var candidate := (points[i] - points[i - 1]).normalized()
			if candidate.is_zero_approx():
				candidate = direction
			var angle := direction.angle_to(candidate)
			if angle > max_angle:
				candidate = direction.slerp(candidate, max_angle / angle).normalized()
			points[i] = points[i - 1] + candidate * step_length
		if actor.secondary_grounded:
			_constrain_floor(frame, direction, step_length)
		accumulator -= STEP
	# Set local poses from world directions; no accumulation on animated poses.
	skeleton.set_bone_pose_rotation(0, Quaternion.IDENTITY)
	var parent_basis := frame.basis.orthonormalized()
	for i in range(1, segments):
		var desired := (points[i + 1] - points[i]).normalized()
		var local_direction := parent_basis.inverse() * desired
		var rotation := Quaternion(axis, local_direction.normalized())
		skeleton.set_bone_pose_rotation(i, rotation)
		parent_basis = parent_basis * Basis(rotation)

func _bone_frames(frame: Transform3D) -> Array[Transform3D]:
	var frames: Array[Transform3D] = []
	var basis := frame.basis.orthonormalized()
	var scale_basis := Basis.from_scale(frame.basis.get_scale())
	frames.append(Transform3D(frame.basis, points[0]))
	for i in range(1, segments):
		var desired := (points[i + 1] - points[i]).normalized()
		var rotation := Quaternion(axis, (basis.inverse() * desired).normalized())
		basis = basis * Basis(rotation)
		frames.append(Transform3D(basis * scale_basis, points[i]))
	return frames

func lowest_support(frame: Transform3D) -> float:
	var frames := _bone_frames(frame)
	var lowest := INF
	for probe: Dictionary in probes.values():
		var position: Vector3 = mesh_frames[probe.mesh] * probe.vertex
		var first: int = probe.bone
		var second := mini(first + 1, segments - 1)
		var a := frames[first] * (position - axis * length * first / segments)
		var b := frames[second] * (position - axis * length * second / segments)
		lowest = minf(lowest, a.lerp(b, probe.blend).y)
	return lowest

func _constrain_floor(frame: Transform3D, direction: Vector3, step_length: float) -> void:
	var floor_y := floor_height + actor.secondary_offset.y - 0.005
	if lowest_support(frame) >= floor_y:
		return
	# Relax only the sagging curve when its support hull touches the desktop.
	# The attachment and all segment lengths remain fixed. In the air the whole
	# rear half can hang freely; on the desktop the belly / feet support it.
	var curved := points.duplicate()
	var low := 0.0
	var high := 1.0
	for iteration in 7:
		var amount := (low + high) * 0.5
		for i in range(2, points.size()):
			var curved_direction := (curved[i] - curved[i - 1]).normalized()
			points[i] = points[i - 1] + direction.slerp(curved_direction, amount).normalized() * step_length
		if lowest_support(frame) < floor_y:
			high = amount
		else:
			low = amount
	for i in range(2, points.size()):
		var curved_direction := (curved[i] - curved[i - 1]).normalized()
		points[i] = points[i - 1] + direction.slerp(curved_direction, low).normalized() * step_length

