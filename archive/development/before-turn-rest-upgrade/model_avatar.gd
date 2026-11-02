extends PetAvatar

const RigMotion = preload("res://scripts/avatars/procedural_rig.gd")
const ModelPhysics = preload("res://scripts/avatars/model_physics.gd")

var viewport: SubViewport
var pivot: Node3D
var player: AnimationPlayer
var rest_nodes: Dictionary = {}
var rest_bones: Dictionary = {}
var rest_shapes: Dictionary = {}
var expressions: Array[Dictionary] = []
var soft_bodies: Array = []

func configure(skin: Dictionary) -> String:
	var physics_model: Node3D
	viewport = SubViewport.new()
	pivot = Node3D.new()
	viewport.size = Vector2i(512, 512)
	viewport.msaa_3d = Viewport.MSAA_4X
	viewport.transparent_bg = true
	viewport.own_world_3d = true
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	add_child(viewport)
	var world := Node3D.new()
	viewport.add_child(world)
	world.add_child(pivot)
	var camera := Camera3D.new()
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	# A wider orthographic view leaves room for fins, tails and animated poses.
	camera.size = MODEL_VIEW_SIZE
	camera.position = Vector3(0, 0.05, 6)
	world.add_child(camera)
	camera.current = true
	var light := DirectionalLight3D.new()
	light.rotation_degrees = Vector3(-30, -30, 0)
	light.light_energy = 1.1
	world.add_child(light)
	var environment := WorldEnvironment.new()
	environment.environment = Environment.new()
	environment.environment.background_mode = Environment.BG_CANVAS
	environment.environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.environment.ambient_light_color = Color("e8edff")
	environment.environment.ambient_light_energy = 0.65
	world.add_child(environment)
	var texture := Sprite2D.new()
	texture.texture = viewport.get_texture()
	texture.scale = Vector2.ONE * (MODEL_DISPLAY_SIZE / viewport.size.x)
	texture.position.y = -46
	visual.add_child(texture)
	if skin.type == "procedural":
		_make_demo(Color(skin.get("color", "f3ede1")))
	else:
		var path: String = skin.root.path_join(skin.asset)
		var model: Node
		if path.begins_with("res://"):
			var scene := load(path) as PackedScene
			if scene == null:
				return "内置 GLB 场景读取失败"
			model = scene.instantiate()
		else:
			var result := AssetIO.model_scene(path)
			if not result.error.is_empty():
				return result.error
			model = result.scene
		if model == null or not model is Node3D:
			return "GLB 未生成有效的 3D 场景"
		# Imported cameras and lights must not override the desktop renderer.
		for child in model.find_children("*", "Node3D", true, false):
			if not is_instance_valid(child):
				continue
			if child is Camera3D or child is Light3D:
				child.get_parent().remove_child(child)
				child.free()
		var normalized := Node3D.new()
		pivot.add_child(normalized)
		var orientation := Node3D.new()
		normalized.add_child(orientation)
		orientation.add_child(model)
		var rotation: Array = skin.get("rotation_degrees", [0, 0, 0])
		orientation.rotation_degrees = Vector3(rotation[0], rotation[1], rotation[2])
		var bounds := _bounds(model, pivot)
		var longest := maxf(bounds.size.x, maxf(bounds.size.y, bounds.size.z))
		if longest < 0.00001 or not is_finite(longest):
			return "GLB 缺少有效网格尺寸"
		var normalization := 2.35 / longest
		normalized.scale = Vector3.ONE * normalization
		normalized.position = -bounds.get_center() * normalization
		physics_model = model
		player = _animation_player(model)
		var bones := PackedStringArray()
		var nodes: Array[Node] = [model]
		nodes.append_array(model.find_children("*", "Node3D", true, false))
		for node in nodes:
			rest_nodes[node] = node.transform
			if node is Skeleton3D:
				var poses: Array[Transform3D] = []
				for i in node.get_bone_count():
					bones.append(node.get_bone_name(i))
					poses.append(node.get_bone_pose(i))
				rest_bones[node] = poses
			if node is MeshInstance3D and node.mesh:
				var shapes: Array[float] = []
				for i in node.get_blend_shape_count():
					shapes.append(node.get_blend_shape_value(i))
					var shape := "".join(ActionAdapter.words(node.mesh.get_blend_shape_name(i)))
					var role := "blink" if shape.contains("blink") or shape.contains("eyeclose") or shape.contains("eyesclosed") or shape.contains("まばたき") or shape.contains("闭眼") else "smile" if shape.contains("smile") or shape.contains("happy") or shape.contains("笑") else ""
					if not role.is_empty():
						expressions.append({"mesh": node, "index": i, "role": role, "base": shapes[i]})
				rest_shapes[node] = shapes
		rest_nodes[model] = model.transform
		if player:
			_prepare_animations()
			available_clips = player.get_animation_list()
			if "RESET" in available_clips:
				available_clips.remove_at(available_clips.find("RESET"))
		explicit_mapping = skin.get("animations", {})
		mapping = ActionAdapter.resolve(available_clips, explicit_mapping)
		action_profile = ActionAdapter.profile_for("model3d", bounds.size, bones, skin.get("profile", "auto"))
		var rig_parts := 0
		for skeleton: Skeleton3D in rest_bones:
			var motion := RigMotion.new()
			motion.setup(self, skeleton)
			rig_parts += motion.parts.size()
			skeleton.add_child(motion)
		capability = {"kind": "3D 模型", "bones": bones.size(), "rig_parts": rig_parts, "expressions": expressions.size(), "profile": action_profile}
		_play_state()
	if skin.type == "procedural":
		action_profile = ActionAdapter.profile_for("model3d", Vector3.ONE, PackedStringArray(), skin.get("profile", "auto"))
		capability = {"kind": "3D 模型", "bones": 0, "profile": action_profile}
	var fitted_bounds := _bounds(pivot, world)
	ground_offset = 160.0 + (camera.position.y - fitted_bounds.position.y) * (MODEL_DISPLAY_SIZE / camera.size)
	# Fit and record native poses before adding the deformation lattice. Its
	# culling envelope must never change the feet's desktop contact height.
	if physics_model != null:
		soft_bodies = ModelPhysics.install(self, physics_model, skin.get("physics", {}).get("soft_bodies", []))
	capability["soft_bodies"] = soft_bodies.size()
	return ""

func set_state(value: String) -> void:
	super.set_state(value)
	_update_facing(0.0)
	_play_state()

func _play_state() -> void:
	native_action = mapping.has(state)
	if player == null:
		return
	player.stop()
	_restore_pose()
	var clip: String = mapping.get(state, "" if explicit_mapping.has(state) else mapping.get("idle", ""))
	if player.has_animation(clip) and not clip.is_empty():
		var animation := player.get_animation(clip)
		animation.loop_mode = Animation.LOOP_LINEAR if state in ["idle", "walk", "run", "sleep", "dragged", "airborne"] or not native_action else Animation.LOOP_NONE
		player.speed_scale = PetBrain.RUN_SPEED / PetBrain.WALK_SPEED if state == "run" and clip == mapping.get("walk", "") else 1.0
		player.play(clip, 0.12)

func _restore_pose() -> void:
	for node: Node3D in rest_nodes:
		node.transform = rest_nodes[node]
	for skeleton: Skeleton3D in rest_bones:
		for i in rest_bones[skeleton].size():
			skeleton.set_bone_pose(i, rest_bones[skeleton][i])
	for mesh: MeshInstance3D in rest_shapes:
		for i in rest_shapes[mesh].size():
			mesh.set_blend_shape_value(i, rest_shapes[mesh][i])

func _prepare_animations() -> void:
	# Each avatar owns its clips, so previews cannot alter another avatar's loop
	# mode or root tracks. Lock horizontal root displacement to keep pets in-window.
	for library_name in player.get_animation_library_list():
		var original := player.get_animation_library(library_name)
		var library := AnimationLibrary.new()
		for name in original.get_animation_list():
			var animation := original.get_animation(name).duplicate(true) as Animation
			for track in animation.get_track_count():
				if animation.track_get_type(track) != Animation.TYPE_POSITION_3D or animation.track_get_key_count(track) == 0:
					continue
				var path := animation.track_get_path(track)
				var target := player.get_node(player.root_node).get_node_or_null(NodePath(path.get_concatenated_names()))
				var root_track := target != null and (target in rest_nodes) and (target == player.get_node(player.root_node) or target.get_parent() == player.get_node(player.root_node))
				if path.get_subname_count() > 0:
					var bone := "".join(ActionAdapter.words(path.get_subname(0)))
					root_track = bone in ["root", "hips", "pelvis", "mixamorighips", "center", "センター", "全ての親"]
				if root_track:
					var first: Vector3 = animation.track_get_key_value(track, 0)
					for key in animation.track_get_key_count(track):
						var value: Vector3 = animation.track_get_key_value(track, key)
						value.x = first.x
						value.z = first.z
						animation.track_set_key_value(track, key, value)
			library.add_animation(name, animation)
		player.remove_animation_library(library_name)
		player.add_animation_library(library_name, library)

func _process(delta: float) -> void:
	super._process(delta)
	_update_facing(delta)
	if not native_action:
		for expression in expressions:
			var amount := 0.0
			if expression.role == "blink":
				amount = 1.0 if state == "sleep" else maxf(0, 1.0 - absf(fmod(elapsed, 4.3) - 3.7) / 0.11)
			elif state == "petted":
				amount = minf(elapsed * 5, 0.7)
			expression.mesh.set_blend_shape_value(expression.index, lerpf(expression.base, 1.0, amount))

func _update_facing(delta: float) -> void:
	if state in ["walk", "run", "airborne"]:
		# Movement must face exactly along the screen, including edge reversals.
		pivot.rotation.y = -PI / 2.0 if direction < 0 else PI / 2.0
	else:
		pivot.rotation.y = lerp_angle(pivot.rotation.y, -0.32 if direction < 0 else 0.32, 1.0 - exp(-delta * 8.0))

func _make_demo(color: Color) -> void:
	_sphere(Vector3.ZERO, Vector3(1.15, 0.95, 0.8), color)
	for side in [-1, 1]:
		_sphere(Vector3(side * 0.6, -0.73, 0.25), Vector3(0.28, 0.18, 0.35), color.darkened(0.08))
		_sphere(Vector3(side * 0.32, 0.15, 0.76), Vector3(0.095, 0.13, 0.06), Color("304743"))
		_sphere(Vector3(side * 0.3, 0.19, 0.81), Vector3.ONE * 0.025, Color.WHITE)
		_sphere(Vector3(side * 0.55, -0.07, 0.68), Vector3(0.14, 0.06, 0.04), Color("eeb5a8"))
	_sphere(Vector3(0, -0.1, 0.79), Vector3(0.08, 0.03, 0.035), Color("6a5952"))

func _sphere(at: Vector3, dimensions: Vector3, color: Color) -> void:
	var mesh := MeshInstance3D.new()
	var sphere := SphereMesh.new()
	sphere.radius = 1
	sphere.height = 2
	mesh.mesh = sphere
	mesh.position = at
	mesh.scale = dimensions
	var material := StandardMaterial3D.new()
	material.albedo_color = color
	material.roughness = 0.72
	mesh.material_override = material
	pivot.add_child(mesh)

func _bounds(node: Node, space: Node3D) -> AABB:
	var result := AABB()
	var found := false
	var meshes := node.find_children("*", "MeshInstance3D", true, false)
	if node is MeshInstance3D:
		meshes.append(node)
	for mesh in meshes:
		if mesh.mesh == null or not mesh.is_visible_in_tree():
			continue
		var box: AABB = (space.global_transform.affine_inverse() * mesh.global_transform) * mesh.get_aabb()
		result = result.merge(box) if found else box
		found = true
	return result

func _animation_player(node: Node) -> AnimationPlayer:
	for child in node.find_children("*", "AnimationPlayer", true, false):
		return child as AnimationPlayer
	return null

func _validate_glb(path: String) -> String:
	return AssetIO.validate_gltf(path)
