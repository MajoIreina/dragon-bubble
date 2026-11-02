extends RefCounted

const SoftBody = preload("res://scripts/avatars/soft_body.gd")

## A supported cross-section lattice changes the body silhouette, including
## attached details. Feet are explicit supports; native animation nodes remain.
static func install(actor: PetAvatar, model: Node3D, definitions: Array) -> Array:
	var bodies: Array = []
	for options: Dictionary in definitions:
		var target := _target(model, options.node)
		var reference := _target(target, options.profile_mesh) if target != null else null
		if not reference is MeshInstance3D or reference.mesh == null:
			push_warning("软体需要有效的轮廓参考网格：" + str(options.node))
			continue
		var anchors: Array[Node3D] = []
		var usable := true
		for path: String in options.get("anchors", []):
			var anchor := _target(target, path)
			if anchor == null:
				push_warning("软体承托节点不存在：" + path)
				usable = false
			else:
				anchors.append(anchor)
		var candidates: Array[MeshInstance3D] = []
		if target is MeshInstance3D:
			candidates.append(target)
		for child in target.find_children("*", "MeshInstance3D", true, false):
			candidates.append(child)
		var meshes: Array[MeshInstance3D] = []
		for instance in candidates:
			var anchored := false
			for anchor in anchors:
				if anchor == instance or anchor.is_ancestor_of(instance):
					anchored = true
			if anchored or instance.mesh == null:
				continue
			var relative := target.global_transform.affine_inverse() * instance.global_transform
			var axis := Vector3(options.axis[0], options.axis[1], options.axis[2]).normalized()
			var extent := -INF
			for corner in 8:
				extent = maxf(extent, (relative * instance.mesh.get_aabb().get_endpoint(corner)).dot(axis))
			if extent <= float(options.get("start", 0.0)):
				continue
			if instance.skin != null or instance.mesh.get_blend_shape_count() > 0:
				push_warning("软体部件已绑定骨骼或形态键：" + str(instance.name))
				usable = false
				break
			for surface in instance.mesh.get_surface_count():
				var arrays := instance.mesh.surface_get_arrays(surface)
				if arrays[Mesh.ARRAY_BONES] != null and not arrays[Mesh.ARRAY_BONES].is_empty():
					push_warning("软体部件已有蒙皮权重：" + str(instance.name))
					usable = false
			meshes.append(instance)
		if not usable or meshes.is_empty():
			push_warning("软体仅支持未蒙皮、无形态键且承托节点完整的独立部件：" + str(options.node))
			continue
		var skeleton := Skeleton3D.new()
		skeleton.name = "SoftBodyLattice"
		target.add_child(skeleton)
		var body := SoftBody.new()
		skeleton.add_child(body)
		body.setup(actor, options, reference)
		for i in body.sections:
			skeleton.add_bone("section_%d" % i)
			var rest := Transform3D(Basis.IDENTITY, body.axis * body.station(i))
			skeleton.set_bone_rest(i, rest)
			skeleton.set_bone_pose(i, rest)
		for instance in meshes:
			_bind_mesh(instance, skeleton, body)
		body.finish_setup()
		bodies.append(body)
	return bodies

static func _target(model: Node3D, name: String) -> Node3D:
	if str(model.name) == name:
		return model
	if name.contains("/"):
		return model.get_node_or_null(NodePath(name)) as Node3D
	for child in model.find_children("*", "Node3D", true, false):
		if str(child.name) == name:
			return child as Node3D
	return null

static func _bind_mesh(instance: MeshInstance3D, skeleton: Skeleton3D, body) -> void:
	var relative := skeleton.global_transform.affine_inverse() * instance.global_transform
	var skin := Skin.new()
	for i in body.sections:
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
			var along: float = body.coordinate(position)
			var bone := int(floorf(along))
			bones[i * 4] = bone
			bones[i * 4 + 1] = mini(bone + 1, body.sections - 1)
			weights[i * 4] = 1.0 - (along - bone)
			weights[i * 4 + 1] = along - bone
			body.include_vertex(position, bone)
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
	# Bake into the skeleton frame. Original transform nodes stay available to
	# native clips; their relative motion is incorporated in each skin's binds.
	var proxy := MeshInstance3D.new()
	proxy.name = str(instance.name) + "_Soft"
	proxy.mesh = result
	proxy.skin = skin
	proxy.cast_shadow = instance.cast_shadow
	proxy.layers = instance.layers
	proxy.custom_aabb = result.get_aabb().grow(body.length * 0.6)
	skeleton.add_child(proxy)
	proxy.skeleton = proxy.get_path_to(skeleton)
	instance.mesh = null
	body.bindings.append({"mesh": instance, "proxy": proxy, "skin": skin, "inverse_rest": relative.affine_inverse(), "last_relative": Transform3D(), "revision": -1})
