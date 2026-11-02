extends SkeletonModifier3D

## Reduced soft body: independently compressible sections settle onto supports.
## A fixed-step spring adds inertia; a GPU lattice carries seams, fur and fins.
const STEP := 1.0 / 120.0
var actor: PetAvatar
var axis := Vector3.FORWARD
var start := 0.0
var length := 2.3
var sections := 28
var support_height := -0.82
var floor_height := -1.01
var compression := 0.18
var gravity := 9.8
var stiffness := 42.0
var damping := 10.0
var lower := PackedFloat32Array()
var upper := PackedFloat32Array()
var mesh_lower := PackedFloat32Array()
var deformations: Array[Transform3D] = []
var bindings: Array[Dictionary] = []
var load_amount := 1.0
var load_velocity := 0.0
var sway := Vector3.ZERO
var sway_velocity := Vector3.ZERO
var accumulator := 0.0
var generation := -1
var last_offset := Vector3.ZERO
var last_velocity := Vector3.ZERO
var pose_revision := 0
var applied_load := -INF
var applied_sway := Vector3(INF, INF, INF)
var applied_grounded := false
var applied_flex := 0.0

func station(index: int) -> float:
	return start + length * index / (sections - 1)

func coordinate(point: Vector3) -> float:
	return clampf((point.dot(axis) - start) / length * (sections - 1), 0.0, sections - 1.0)

func setup(avatar: PetAvatar, options: Dictionary, reference: MeshInstance3D) -> void:
	actor = avatar
	axis = Vector3(options.axis[0], options.axis[1], options.axis[2]).normalized()
	start = float(options.get("start", 0.0))
	length = float(options.length)
	sections = int(options.get("sections", 28))
	support_height = float(options.support_height)
	floor_height = float(options.floor_height)
	compression = float(options.get("compression", 0.18))
	gravity = float(options.get("gravity", 9.8))
	stiffness = float(options.get("stiffness", 42.0))
	damping = float(options.get("damping", 10.0))
	lower.resize(sections)
	upper.resize(sections)
	mesh_lower.resize(sections)
	mesh_lower.fill(INF)
	# Sample actual body rings rather than the bounding box. Narrow rear rings
	# must fall farther than the large front section to form a supported belly.
	var slices: Dictionary = {}
	var relative := get_skeleton().global_transform.affine_inverse() * reference.global_transform
	for surface in reference.mesh.get_surface_count():
		for vertex: Vector3 in reference.mesh.surface_get_arrays(surface)[Mesh.ARRAY_VERTEX]:
			var point := relative * vertex
			var key := roundi(point.dot(axis) * 10000.0)
			if not slices.has(key):
				slices[key] = Vector2(point.y, point.y)
			else:
				slices[key] = Vector2(minf(slices[key].x, point.y), maxf(slices[key].y, point.y))
	var keys := slices.keys()
	keys.sort()
	for i in sections:
		var at := station(i) * 10000.0
		var index := 0
		while index < keys.size() - 1 and float(keys[index + 1]) < at:
			index += 1
		var next := mini(index + 1, keys.size() - 1)
		var blend := clampf((at - float(keys[index])) / maxf(1.0, float(keys[next] - keys[index])), 0.0, 1.0)
		var extent: Vector2 = slices[keys[index]].lerp(slices[keys[next]], blend)
		lower[i] = extent.x
		upper[i] = extent.y

func include_vertex(point: Vector3, index: int) -> void:
	# Conservative hull of all cloth, including its fur and tail, at both
	# neighboring stations so their interpolation cannot penetrate the support.
	mesh_lower[index] = minf(mesh_lower[index], point.y)
	var next := mini(index + 1, sections - 1)
	mesh_lower[next] = minf(mesh_lower[next], point.y)

func finish_setup() -> void:
	for i in sections:
		if not is_finite(mesh_lower[i]):
			mesh_lower[i] = lower[i]
	reset_simulation()
	_apply()

func reset_simulation() -> void:
	load_amount = gravity / 9.8
	load_velocity = 0.0
	sway = Vector3.ZERO
	sway_velocity = Vector3.ZERO
	last_offset = actor.secondary_offset
	last_velocity = Vector3.ZERO
	generation = actor.secondary_generation
	accumulator = 0.0

func _process_modification() -> void:
	if is_instance_valid(actor):
		advance(get_process_delta_time())

func advance(delta: float) -> void:
	if not is_finite(delta) or delta <= 0.0:
		return
	if delta > 0.25 or generation != actor.secondary_generation:
		reset_simulation()
		_apply()
		return
	var velocity := (actor.secondary_offset - last_offset) / delta
	var acceleration := ((velocity - last_velocity) / delta).limit_length(120.0)
	last_offset = actor.secondary_offset
	last_velocity = velocity
	var local_acceleration := get_skeleton().global_basis.inverse() * acceleration
	# Window gravity is 1800 logical px/s². A freely falling body unloads;
	# accelerating upwards increases the effective weight of its filling.
	var desktop_gravity := actor.desktop_gravity()
	var target_load := gravity / 9.8 * clampf(1.0 + acceleration.y / desktop_gravity, 0.0, 1.12)
	accumulator += delta
	while accumulator >= STEP:
		load_velocity += ((target_load - load_amount) * stiffness - load_velocity * damping) * STEP
		load_amount = clampf(load_amount + load_velocity * STEP, 0.0, 1.2)
		sway_velocity += (-local_acceleration * 0.10 - sway * stiffness - sway_velocity * damping) * STEP
		sway = (sway + sway_velocity * STEP).limit_length(length * 0.06)
		accumulator -= STEP
	_apply()

func _apply() -> void:
	var skeleton := get_skeleton()
	var flex := actor.gait_flex()
	if absf(load_amount - applied_load) > 0.00001 or not sway.is_equal_approx(applied_sway) or actor.secondary_grounded != applied_grounded or absf(flex - applied_flex) > 0.0001:
		applied_flex = flex
		_build_deformations()
		pose_revision += 1
		applied_load = load_amount
		applied_sway = sway
		applied_grounded = actor.secondary_grounded
	var frame := skeleton.global_transform.affine_inverse()
	for binding in bindings:
		var relative: Transform3D = frame * binding.mesh.global_transform
		binding.proxy.visible = binding.mesh.is_visible_in_tree()
		if binding.revision == pose_revision and relative.is_equal_approx(binding.last_relative):
			continue
		binding.last_relative = relative
		binding.revision = pose_revision
		for i in sections:
			# Skin matrices retain the affine shear. Bone pose setters decompose
			# to rotation/scale and would lose the soft cross-section distortion.
			binding.skin.set_bind_pose(i, skeleton.get_bone_global_rest(i).affine_inverse() * deformations[i] * relative * binding.inverse_rest)

func _build_deformations() -> void:
	deformations.clear()
	var side := axis.cross(Vector3.UP).normalized()
	for i in sections:
		var t := float(i) / (sections - 1)
		var influence := smoothstep(0.0, 0.55, t)
		var soft := smoothstep(0.0, 1.0, t)
		var squash := 1.0 - compression * soft * load_amount
		var bottom := lerpf(lower[i], minf(lower[i], support_height), influence * load_amount)
		var shift := bottom - lower[i] * squash + sway.y * soft
		if actor.secondary_grounded:
			shift = maxf(shift, floor_height - mesh_lower[i] * squash)
		var spread := 1.0 + (1.0 - squash) * 0.35
		var lean := 0.12 * soft * load_amount
		var basis := Basis.IDENTITY
		for dimension in 3:
			basis[dimension] += side * side[dimension] * (spread - 1.0) + Vector3.UP * Vector3.UP[dimension] * (squash - 1.0) + axis * Vector3.UP[dimension] * lean
		var offset := Vector3.UP * shift - axis * lean * lower[i] + Vector3(sway.x, 0.0, sway.z) * soft
		# The plush spine arches while the hind feet gather under the belly and
		# unfolds for the extended flight. Front sections remain the stable head.
		var arch := sin(t * PI) * maxf(applied_flex, 0.0) * 0.12
		offset += Vector3.UP * arch - axis * soft * applied_flex * 0.11
		deformations.append(Transform3D(basis, offset))

func deform_point(point: Vector3) -> Vector3:
	var along := coordinate(point)
	var first := int(floorf(along))
	var next := mini(first + 1, sections - 1)
	return (deformations[first] * point).lerp(deformations[next] * point, along - first)
