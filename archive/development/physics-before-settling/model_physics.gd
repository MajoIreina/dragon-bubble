extends RefCounted

const Chain = preload("res://scripts/avatars/secondary_chain.gd")

## Only explicitly declared, unskinned subtrees are weighted. Source resources
## stay intact, and existing rigs / blend shapes are never guessed or replaced.
static func install(actor: PetAvatar, model: Node3D, definitions: Array, floor_y: float) -> Array:
	var chains: Array = []
	for options: Dictionary in definitions:
		var target := _target(model, options.node)
		if target == null:
			push_warning("模型物理部件不存在：" + str(options.node))
			continue
		var meshes: Array[MeshInstance3D] = []
		if target is MeshInstance3D:
			meshes.append(target)
		for child in target.find_children("*", "MeshInstance3D", true, false):
			meshes.append(child)
		var usable := not meshes.is_empty()
		var axis := Vector3(options.axis[0], options.axis[1], options.axis[2]).normalized()
		var start: Array = options.get("origin", [0, 0, 0])
		var origin := Vector3(start[0], start[1], start[2])
		var span := 0.0
		var affected: Array[MeshInstance3D] = []
		for mesh in meshes:
			var relative := target.global_transform.affine_inverse() * mesh.global_transform
			var extent := 0.0
			if mesh.mesh != null:
				for corner in 8:
					extent = maxf(extent, (relative * mesh.mesh.get_aabb().get_endpoint(corner) - origin).dot(axis))
			if extent < 0.0001:
				continue
			if mesh.mesh == null or mesh.skin != null or mesh.mesh.get_blend_shape_count() > 0:
				usable = false
				break
			affected.append(mesh)
			for surface in mesh.mesh.get_surface_count():
				var arrays := mesh.mesh.surface_get_arrays(surface)
				if arrays[Mesh.ARRAY_BONES] != null and not arrays[Mesh.ARRAY_BONES].is_empty():
					usable = false
				for vertex: Vector3 in arrays[Mesh.ARRAY_VERTEX]:
					span = maxf(span, (relative * vertex - origin).dot(axis))
		if not usable or span < 0.0001 or affected.is_empty():
			push_warning("模型物理需要沿 axis 延伸的独立静态网格：" + str(options.node))
			continue
		var skeleton := Skeleton3D.new()
		skeleton.name = "SecondarySkeleton"
		target.add_child(skeleton)
		skeleton.position = origin
		var count := int(options.get("segments", 6))
		for i in count:
			skeleton.add_bone("secondary_%d" % i)
			if i > 0:
				skeleton.set_bone_parent(i, i - 1)
			var rest := Transform3D(Basis.IDENTITY, axis * (0.0 if i == 0 else span / count))
			skeleton.set_bone_rest(i, rest)
			skeleton.set_bone_pose_position(i, rest.origin)
		var chain := Chain.new()
		chain.setup(actor, options, span, floor_y)
		skeleton.add_child(chain)
		for mesh in affected:
			_bind_mesh(mesh, skeleton, chain, axis, span, count)
		chains.append(chain)
	return chains

static func _target(model: Node3D, name: String) -> Node3D:
	if str(model.name) == name:
		return model
	# Exact paths or exact names; no wildcard matching of unrelated parts.
	if name.contains("/"):
		return model.get_node_or_null(NodePath(name)) as Node3D
	for child in model.find_children("*", "Node3D", true, false):
		if str(child.name) == name:
			return child as Node3D
	return null

static func _bind_mesh(instance: MeshInstance3D, skeleton: Skeleton3D, chain, axis: Vector3, span: float, count: int) -> void:
	var relative := skeleton.global_transform.affine_inverse() * instance.global_transform
	var skin := Skin.new()
	for i in count:
		skin.add_bind(i, skeleton.get_bone_global_rest(i).affine_inverse())
	var normal_basis := relative.basis.inverse().transposed()
	var result := ArrayMesh.new()
	for surface in instance.mesh.get_surface_count():
		var arrays := instance.mesh.surface_get_arrays(surface).duplicate(true)
		var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var normals: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL] if arrays[Mesh.ARRAY_NORMAL] != null else PackedVector3Array()
		var tangents: PackedFloat32Array = arrays[Mesh.ARRAY_TANGENT] if arrays[Mesh.ARRAY_TANGENT] != null else PackedFloat32Array()
		var bones := PackedInt32Array()
		var weights := PackedFloat32Array()
		bones.resize(vertices.size() * 4)
		weights.resize(vertices.size() * 4)
		for i in vertices.size():
			var position := relative * vertices[i]
			var along := clampf(position.dot(axis) / span * count, 0, count - 1)
			var bone := int(floorf(along))
			bones[i * 4] = bone
			bones[i * 4 + 1] = mini(bone + 1, count - 1)
			weights[i * 4] = 1.0 - (along - bone)
			weights[i * 4 + 1] = along - bone
			if i % maxi(1, vertices.size() / 1024) == 0:
				chain.add_probe(instance, vertices[i], position, bone, along - bone)
			vertices[i] = position
			if i < normals.size():
				normals[i] = (normal_basis * normals[i]).normalized()
			if i * 4 < tangents.size():
				var tangent := (relative.basis * Vector3(tangents[i * 4], tangents[i * 4 + 1], tangents[i * 4 + 2])).normalized()
				tangents[i * 4] = tangent.x
				tangents[i * 4 + 1] = tangent.y
				tangents[i * 4 + 2] = tangent.z
		arrays[Mesh.ARRAY_VERTEX] = vertices
		arrays[Mesh.ARRAY_NORMAL] = normals if not normals.is_empty() else null
		arrays[Mesh.ARRAY_TANGENT] = tangents if not tangents.is_empty() else null
		arrays[Mesh.ARRAY_BONES] = bones
		arrays[Mesh.ARRAY_WEIGHTS] = weights
		result.add_surface_from_arrays(instance.mesh.surface_get_primitive_type(surface), arrays)
		result.surface_set_material(surface, instance.get_active_material(surface))
	# Render in the skeleton frame. Original nodes remain as native animation
	# targets; their current transforms enter each proxy's bind matrices.
	var proxy := MeshInstance3D.new()
	proxy.name = str(instance.name) + "_Flexible"
	skeleton.add_child(proxy)
	proxy.mesh = result
	proxy.skin = skin
	proxy.skeleton = proxy.get_path_to(skeleton)
	proxy.cast_shadow = instance.cast_shadow
	proxy.layers = instance.layers
	proxy.custom_aabb = AABB(Vector3.ONE * -span * 2.0, Vector3.ONE * span * 4.0)
	instance.mesh = null
	chain.bindings.append({"mesh": instance, "proxy": proxy, "skin": skin, "inverse_rest": relative.affine_inverse()})
