extends SceneTree

# Rasterize the same SVG used by Settings and the tray at native Windows sizes.
func _initialize() -> void:
	var svg := FileAccess.get_file_as_string("res://assets/tray.svg")
	var sizes := [16, 24, 32, 48, 64, 128, 256]
	var directory := PackedByteArray()
	directory.resize(6 + 16 * sizes.size())
	directory.encode_u16(2, 1)
	directory.encode_u16(4, sizes.size())
	for i in sizes.size():
		var image := Image.new()
		if image.load_svg_from_string(svg, sizes[i] / 64.0) != OK:
			push_error("Could not render the application icon")
			quit(1)
			return
		var png := image.save_png_to_buffer()
		var entry := 6 + 16 * i
		directory[entry] = sizes[i] % 256
		directory[entry + 1] = sizes[i] % 256
		directory.encode_u16(entry + 4, 1)
		directory.encode_u16(entry + 6, 32)
		directory.encode_u32(entry + 8, png.size())
		directory.encode_u32(entry + 12, directory.size())
		directory.append_array(png)
	var target := "res://assets/tray.ico"
	if not FileAccess.file_exists(target) or FileAccess.get_file_as_bytes(target) != directory:
		var output := FileAccess.open(target, FileAccess.WRITE)
		if not output:
			push_error("Could not write the Windows application icon")
			quit(1)
			return
		output.store_buffer(directory)
		output.close()
	print("Windows icon generated from assets/tray.svg: 16–256 px")
	quit()
