class_name ObjReader
extends RefCounted

## Runtime Wavefront geometry + MTL, including negative indices and texture UVs.
static func read(path: String) -> Dictionary:
	var positions: Array[Vector3] = []
	var normals: Array[Vector3] = []
	var uvs: Array[Vector2] = []
	var faces: Dictionary = {}
	var materials: Dictionary = {}
	var current := "default"
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null or file.get_length() > AssetIO.MAX_BYTES:
		return {"error": "OBJ 无法读取或超过 256 MB"}
	var regex := RegEx.new()
	regex.compile("\\s+")
	var triangles := 0
	while not file.eof_reached():
		var parts := regex.sub(file.get_line().split("#")[0].strip_edges(), " ", true).split(" ", false)
		if parts.is_empty():
			continue
		match parts[0]:
			"v", "vn":
				if parts.size() < 4:
					return {"error": "OBJ 顶点格式无效"}
				var v := Vector3(float(parts[1]), float(parts[2]), float(parts[3]))
				if not v.is_finite():
					return {"error": "OBJ 包含无效坐标"}
				if parts[0] == "v":
					positions.append(v)
				else:
					normals.append(v)
			"vt":
				if parts.size() >= 3:
					uvs.append(Vector2(float(parts[1]), 1.0 - float(parts[2])))
			"usemtl":
				current = " ".join(parts.slice(1))
			"mtllib":
				var joined := " ".join(parts.slice(1)).trim_prefix("\"").trim_suffix("\"")
				var libraries := PackedStringArray([joined]) if FileAccess.file_exists(path.get_base_dir().path_join(joined)) else parts.slice(1)
				for relative in libraries:
					if not SkinCatalog.safe_relative(relative):
						return {"error": "MTL 必须位于模型目录内"}
					var result := _materials(path.get_base_dir().path_join(relative))
					if not result.error.is_empty():
						return result
					materials.merge(result.materials)
			"f":
				if parts.size() < 4 or parts.size() > 513:
					return {"error": "OBJ 面顶点数量无效（3–512）"}
				var polygon: Array = []
				for item in parts.slice(1):
					var indices := item.split("/", true)
					var vertex := _index(indices[0], positions.size())
					var uv := _index(indices[1], uvs.size()) if indices.size() > 1 and not indices[1].is_empty() else -1
					var normal := _index(indices[2], normals.size()) if indices.size() > 2 and not indices[2].is_empty() else -1
					if vertex < 0 or uv == -2 or normal == -2:
						return {"error": "OBJ 索引越界"}
					polygon.append([vertex, uv, normal])
				var projected := PackedVector2Array()
				var normal := Vector3.ZERO
				for i in polygon.size():
					normal += positions[polygon[i][0]].cross(positions[polygon[(i + 1) % polygon.size()][0]])
				var axis := normal.abs().max_axis_index()
				for point in polygon:
					var p: Vector3 = positions[point[0]]
					projected.append(Vector2(p.y, p.z) if axis == 0 else Vector2(p.x, p.z) if axis == 1 else Vector2(p.x, p.y))
				var indices := Geometry2D.triangulate_polygon(projected)
				if indices.is_empty():
					return {"error": "OBJ 含无法三角化的面"}
				if not faces.has(current):
					faces[current] = []
				# triangulate_polygon may normalize winding; preserve the source winding.
				var source_clockwise := Geometry2D.is_polygon_clockwise(projected)
				for i in range(0, indices.size(), 3):
					var a := indices[i]
					var b := indices[i + 1]
					var c := indices[i + 2]
					var tri := PackedVector2Array([projected[a], projected[b], projected[c]])
					if Geometry2D.is_polygon_clockwise(tri) != source_clockwise:
						var swap := b
						b = c
						c = swap
					faces[current].append([polygon[a], polygon[c], polygon[b]])
					triangles += 1
		if positions.size() > 1000000 or triangles > 1000000:
			return {"error": "OBJ 超过一百万顶点或三角面，请先简化"}
	if triangles == 0:
		return {"error": "OBJ 没有可显示的面"}
	var mesh := ArrayMesh.new()
	for material in faces:
		var surface := SurfaceTool.new()
		surface.begin(Mesh.PRIMITIVE_TRIANGLES)
		for triangle in faces[material]:
			# Godot uses clockwise front faces; normals face outward.
			var flat: Vector3 = (positions[triangle[2][0]] - positions[triangle[0][0]]).cross(positions[triangle[1][0]] - positions[triangle[0][0]]).normalized()
			for point in triangle:
				surface.set_normal(normals[point[2]].normalized() if point[2] >= 0 else flat)
				surface.set_uv(uvs[point[1]] if point[1] >= 0 else Vector2.ZERO)
				surface.add_vertex(positions[point[0]])
		surface.set_material(materials.get(material, StandardMaterial3D.new()))
		surface.index()
		surface.commit(mesh)
	var scene := Node3D.new()
	scene.name = "ImportedOBJ"
	var instance := MeshInstance3D.new()
	instance.mesh = mesh
	scene.add_child(instance)
	return {"error": "", "scene": scene}

static func _index(text: String, size: int) -> int:
	if not text.is_valid_int():
		return -2
	var index := int(text)
	index = index - 1 if index > 0 else size + index if index < 0 else -2
	return index if index >= 0 and index < size else -2

static func _materials(path: String) -> Dictionary:
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null or file.get_length() > 1024 * 1024:
		return {"error": "MTL 缺失或超过 1 MB"}
	var result := {}
	var material: StandardMaterial3D
	while not file.eof_reached():
		var line := file.get_line().split("#")[0].strip_edges().replace("\t", " ")
		var parts := line.split(" ", false)
		if parts.is_empty():
			continue
		if parts[0] == "newmtl":
			material = StandardMaterial3D.new()
			material.resource_name = " ".join(parts.slice(1))
			result[material.resource_name] = material
		elif material:
			if parts[0] == "Kd" and parts.size() >= 4:
				material.albedo_color = Color(float(parts[1]), float(parts[2]), float(parts[3]))
			elif parts[0] in ["d", "Tr"] and parts.size() >= 2:
				material.albedo_color.a = clampf(1.0 - float(parts[1]) if parts[0] == "Tr" else float(parts[1]), 0.0, 1.0)
				if material.albedo_color.a < 1:
					material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
			elif parts[0] == "map_Kd":
				var relative := " ".join(parts.slice(1)).trim_prefix("\"").trim_suffix("\"")
				if not SkinCatalog.safe_relative(relative):
					return {"error": "OBJ 贴图必须位于 MTL 目录内；暂不支持 map_Kd 命令选项"}
				var image := AssetIO.load_image(path.get_base_dir().path_join(relative))
				if image == null or image.is_empty() or image.get_width() > 8192 or image.get_height() > 8192:
					return {"error": "无法读取 OBJ 贴图或贴图尺寸超过 8192"}
				material.albedo_texture = ImageTexture.create_from_image(image)
	return {"error": "", "materials": result}
