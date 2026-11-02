class_name GifReader
extends RefCounted

## GIF87a/89a data decoder: LZW, local palettes, interlace, disposal 2/3.
var bytes: PackedByteArray
var offset := 0
var invalid := false

func byte() -> int:
	if offset >= bytes.size():
		invalid = true
		return 0
	var value := bytes[offset]
	offset += 1
	return value

func word() -> int:
	var low := byte()
	return low | (byte() << 8)

func palette(count: int) -> Array[Color]:
	var colors: Array[Color] = []
	for i in count:
		var r := byte()
		var g := byte()
		var b := byte()
		colors.append(Color8(r, g, b))
	return colors

func blocks() -> PackedByteArray:
	var result := PackedByteArray()
	var count := byte()
	while count > 0 and not invalid:
		if offset + count > bytes.size():
			invalid = true
			break
		result.append_array(bytes.slice(offset, offset + count))
		offset += count
		count = byte()
	return result

func decode(path: String) -> Dictionary:
	bytes = FileAccess.get_file_as_bytes(path)
	offset = 6
	invalid = false
	if bytes.size() < 13 or bytes.slice(0, 6).get_string_from_ascii() not in ["GIF87a", "GIF89a"]:
		return {"error": "GIF 头部无效"}
	var width := word()
	var height := word()
	if width < 1 or height < 1 or width > 4096 or height > 4096:
		return {"error": "GIF 尺寸需在 1–4096 内"}
	var packed := byte()
	var background := byte()
	byte()
	var global_palette: Array[Color] = palette(2 << (packed & 7)) if packed & 128 else []
	var canvas := Image.create(width, height, false, Image.FORMAT_RGBA8)
	canvas.fill(Color(0, 0, 0, 0))
	var images: Array[Image] = []
	var durations: Array[float] = []
	var delay := 0.1
	var disposal := 0
	var transparent := -1
	while not invalid and offset < bytes.size():
		var marker := byte()
		if marker == 0x3b:
			break
		if marker == 0x21:
			var label := byte()
			if label == 0xf9:
				if byte() != 4:
					return {"error": "GIF 控制块无效"}
				var control := byte()
				disposal = (control >> 2) & 7
				delay = maxf(word() / 100.0, 0.02)
				var index := byte()
				transparent = index if control & 1 else -1
				if byte() != 0:
					invalid = true
			else:
				blocks()
			continue
		if marker != 0x2c:
			return {"error": "GIF 图像块无效"}
		var left := word()
		var top := word()
		var w := word()
		var h := word()
		var flags := byte()
		if w < 1 or h < 1 or left + w > width or top + h > height:
			return {"error": "GIF 帧超出画布"}
		if images.size() >= 256 or (images.size() + 1) * width * height * 4 > 128 * 1024 * 1024:
			return {"error": "GIF 超过 256 帧或解码后超过 128 MB，请先压缩"}
		var colors := palette(2 << (flags & 7)) if flags & 128 else global_palette
		var code_size := byte()
		var compressed := blocks()
		var indices := _lzw(compressed, code_size, w * h)
		if invalid or indices.size() != w * h or colors.is_empty():
			return {"error": "GIF LZW 数据无效或被截断"}
		if images.is_empty() and transparent < 0 and background < global_palette.size():
			canvas.fill(global_palette[background])
		var before := canvas.duplicate() as Image if disposal == 3 else null
		var rows: Array[int] = []
		if flags & 64:
			for pass_index in 4:
				for y in range([0, 4, 2, 1][pass_index], h, [8, 8, 4, 2][pass_index]):
					rows.append(y)
		else:
			for y in h:
				rows.append(y)
		for row in h:
			for x in w:
				var index := indices[row * w + x]
				if index == transparent:
					continue
				if index >= colors.size():
					return {"error": "GIF 调色板索引越界"}
				canvas.set_pixel(left + x, top + rows[row], colors[index])
		images.append(canvas.duplicate())
		durations.append(delay)
		if disposal == 2:
			var clear := global_palette[background] if transparent < 0 and background < global_palette.size() else Color(0, 0, 0, 0)
			canvas.fill_rect(Rect2i(left, top, w, h), clear)
		elif disposal == 3 and before:
			canvas = before
		delay = 0.1
		disposal = 0
		transparent = -1
	if invalid or images.is_empty():
		return {"error": "GIF 文件不完整或没有图像帧"}
	return {"error": "", "images": images, "durations": durations}

func _lzw(data: PackedByteArray, minimum: int, expected: int) -> PackedByteArray:
	if minimum < 2 or minimum > 8:
		return PackedByteArray()
	var clear := 1 << minimum
	var end := clear + 1
	var table: Array[PackedByteArray] = []
	var bit := 0
	var size := minimum + 1
	var previous := PackedByteArray()
	var output := PackedByteArray()
	while bit + size <= data.size() * 8:
		var code := 0
		for i in size:
			code |= ((data[(bit + i) >> 3] >> ((bit + i) & 7)) & 1) << i
		bit += size
		if code == clear:
			table.clear()
			for i in clear:
				table.append(PackedByteArray([i]))
			table.append(PackedByteArray())
			table.append(PackedByteArray())
			size = minimum + 1
			previous = PackedByteArray()
			continue
		if code == end:
			return output
		var entry := PackedByteArray()
		if code < table.size():
			entry = table[code].duplicate()
		elif code == table.size() and not previous.is_empty():
			entry = previous.duplicate()
			entry.append(previous[0])
		if entry.is_empty() or output.size() + entry.size() > expected:
			return PackedByteArray()
		output.append_array(entry)
		if not previous.is_empty() and table.size() < 4096:
			var next := previous.duplicate()
			next.append(entry[0])
			table.append(next)
			if table.size() == 1 << size and size < 12:
				size += 1
		previous = entry
	return PackedByteArray()
