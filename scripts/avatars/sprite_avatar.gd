extends PetAvatar

var sprite := Sprite2D.new()
var frames: Array[Texture2D] = []
var clips: Dictionary = {}
var current_clip := ""
var frame_clock := 0.0
var flip_with_direction := true

func configure(skin: Dictionary) -> String:
	visual.add_child(sprite)
	var images: Array[Image] = []
	var path: String = skin.root.path_join(skin.asset)
	if path.get_extension().to_lower() == "gif":
		var decoded := GifReader.new().decode(path)
		if not decoded.error.is_empty():
			return decoded.error
		images.assign(decoded.images)
		clips["idle"] = {"frames": range(images.size()), "durations": decoded.durations}
	elif skin.has("frames"):
		for relative: String in skin.frames:
			var image := AssetIO.load_image(skin.root.path_join(relative))
			if not _valid_image(image):
				return "序列帧无法读取或尺寸超过 8192：" + relative
			images.append(image)
	else:
		var image := AssetIO.load_image(path)
		if not _valid_image(image):
			return "图片无法读取、内容为空或尺寸超过 8192 × 8192"
		var grid: Array = skin.get("grid", [1, 1])
		if image.get_width() % int(grid[0]) != 0 or image.get_height() % int(grid[1]) != 0:
			return "精灵图尺寸必须能被网格列数和行数整除"
		var cell := Vector2i(image.get_width() / int(grid[0]), image.get_height() / int(grid[1]))
		for y in int(grid[1]):
			for x in int(grid[0]):
				images.append(image.get_region(Rect2i(Vector2i(x, y) * cell, cell)))
	if images.is_empty() or images.size() > 256:
		return "需要 1–256 张图片帧"
	var dimensions := Vector2i.ZERO
	var used := Rect2i()
	var bytes := 0
	for image in images:
		image.convert(Image.FORMAT_RGBA8)
		bytes += image.get_width() * image.get_height() * 4
		dimensions = dimensions.max(image.get_size())
		var rect := image.get_used_rect()
		if rect.has_area():
			used = used.merge(rect) if used.has_area() else rect
	if bytes > 128 * 1024 * 1024 or not used.has_area():
		return "序列帧为空或解码后超过 128 MB"
	# A shared alpha crop and canvas keep frame changes anchored to the same feet.
	for image in images:
		var canvas := Image.create(dimensions.x, dimensions.y, false, Image.FORMAT_RGBA8)
		canvas.fill(Color.TRANSPARENT)
		canvas.blit_rect(image, Rect2i(Vector2i.ZERO, image.get_size()), Vector2i.ZERO)
		frames.append(ImageTexture.create_from_image(canvas.get_region(used)))
	clips.merge(skin.get("clips", {}), true)
	if clips.is_empty() and images.size() > 1:
		clips["idle"] = {"frames": range(images.size()), "fps": 10.0}
	for clip in clips:
		for frame in clips[clip].frames:
			if int(frame) < 0 or int(frame) >= frames.size():
				return "动作 %s 的帧索引超出图片范围" % clip
	available_clips = PackedStringArray(clips.keys())
	explicit_mapping = skin.get("animations", {})
	mapping = ActionAdapter.resolve(available_clips, explicit_mapping)
	action_profile = ActionAdapter.profile_for("sprite", Vector3.ZERO, PackedStringArray(), skin.get("profile", "auto"))
	flip_with_direction = skin.get("flip_with_direction", true)
	var ratio := 232.0 / maxf(used.size.x, used.size.y)
	sprite.texture = frames[0]
	sprite.scale = Vector2.ONE * ratio
	sprite.position.y = -60
	ground_offset = 146.0 + used.size.y * 0.5 * ratio
	capability = {"kind": "2D 动画" if frames.size() > 1 else "静态图片", "frames": frames.size(), "bones": 0, "profile": action_profile}
	_select_clip()
	return ""

func _valid_image(image: Image) -> bool:
	return image != null and not image.is_empty() and image.get_width() <= 8192 and image.get_height() <= 8192

func set_state(value: String) -> void:
	super.set_state(value)
	_select_clip()

func _select_clip() -> void:
	native_action = mapping.has(state)
	current_clip = mapping.get(state, "" if explicit_mapping.has(state) else mapping.get("idle", ""))
	frame_clock = 0
	if not frames.is_empty():
		sprite.texture = frames[0]
	_update_frame()

func _update_frame() -> void:
	if not clips.has(current_clip):
		return
	var clip: Dictionary = clips[current_clip]
	var indices: Array = clip.frames
	var durations: Array = clip.get("durations", [])
	var total := 0.0
	for i in indices.size():
		total += float(durations[i]) if not durations.is_empty() else 1.0 / float(clip.get("fps", 10.0))
	var loop: bool = clip.get("loop", state in ["idle", "walk", "run", "sleep", "dragged", "airborne"] or not native_action)
	var time := fmod(frame_clock, total) if loop else minf(frame_clock, total - 0.000001)
	for i in indices.size():
		var duration := float(durations[i]) if not durations.is_empty() else 1.0 / float(clip.get("fps", 10.0))
		if time < duration:
			sprite.texture = frames[int(indices[i])]
			break
		time -= duration

func _process(delta: float) -> void:
	super._process(delta)
	if not locomotion_paused or state not in ["walk", "run"]:
		frame_clock += delta * locomotion_rate * (PetBrain.RUN_SPEED / PetBrain.WALK_SPEED if state == "run" and current_clip == mapping.get("walk", "") else 1.0)
	_update_frame()
	var facing := turn_origin_direction if turning and turn_progress() < 0.5 else direction
	sprite.flip_h = flip_with_direction and facing < 0
	sprite.skew = sin(turn_progress() * TAU) * 0.06 if turning else 0.0
