extends RefCounted

const INK := Color("243b36")
const MUTED := Color("73817c")
const ACCENT := Color("287c62")
const LINE := Color("e3eae6")
const BACKGROUND := Color("f5f7f5")

static func panel(color: Color, radius := 14, padding := 20, border := Color.TRANSPARENT) -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = color
	style.set_corner_radius_all(radius)
	style.set_content_margin_all(padding)
	style.border_color = border
	style.set_border_width_all(1 if border.a > 0 else 0)
	return style

static func icon(kind: String, color := ACCENT) -> ImageTexture:
	var path: String = {
		"skin": '<path d="m12 3 9 5-9 5-9-5 9-5Zm-9 9 9 5 9-5M3 16l9 5 9-5"/>',
		"behavior": '<rect x="6" y="2" width="12" height="20" rx="6"/><path d="M12 2v7"/>',
		"display": '<rect x="2" y="3" width="20" height="14" rx="3"/><path d="M8 22h8M12 17v5"/>',
		"import": '<path d="M12 16V3m-5 5 5-5 5 5M3 16v4a1 1 0 0 0 1 1h16a1 1 0 0 0 1-1v-4"/>',
		"folder": '<path d="M3 6a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/>',
		"exit": '<path d="M9 4H5a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h4m6-13 5 5-5 5M8 12h12"/>',
		"arrow": '<path d="m7 10 5 5 5-5"/>',
		"next": '<path d="m9 6 6 6-6 6"/>',
		"reset": '<path d="M3 4v6h6M3 10a9 9 0 1 1 1 9"/>'
	}.get(kind, "")
	var svg := '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g fill="none" stroke="#%s" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">%s</g></svg>' % [color.to_html(false), path]
	var image := Image.new()
	image.load_svg_from_string(svg)
	return ImageTexture.create_from_image(image)

static func circle(color: Color, diameter: int, ring := false) -> ImageTexture:
	var fill := "white" if ring else "#" + color.to_html(false)
	var svg := '<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d"><circle cx="%f" cy="%f" r="%f" fill="%s" stroke="#%s" stroke-width="2"/></svg>' % [diameter, diameter, diameter / 2.0, diameter / 2.0, diameter / 2.0 - 2, fill, color.to_html(false)]
	var image := Image.new()
	image.load_svg_from_string(svg)
	return ImageTexture.create_from_image(image)

static func create() -> Theme:
	var theme := Theme.new()
	var font := SystemFont.new()
	font.font_names = PackedStringArray(["Microsoft YaHei UI", "Segoe UI", "sans-serif"])
	theme.default_font = font
	theme.default_font_size = 14
	theme.set_color("font_color", "Label", INK)
	for type in ["Button", "OptionButton"]:
		theme.set_stylebox("normal", type, panel(Color.WHITE, 9, 12, LINE))
		theme.set_stylebox("hover", type, panel(Color("f0f6f2"), 9, 12, Color("bbd4c6")))
		theme.set_stylebox("pressed", type, panel(Color("e3f0e7"), 9, 12, Color("bbd4c6")))
		theme.set_stylebox("focus", type, panel(Color.TRANSPARENT, 9, 0, ACCENT))
		for state in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
			theme.set_color(state, type, INK)
		theme.set_constant("h_separation", type, 9)
	theme.set_icon("arrow", "OptionButton", icon("arrow", MUTED))
	theme.set_constant("arrow_margin", "OptionButton", 12)
	theme.set_stylebox("normal", "LineEdit", panel(Color("f8faf8"), 8, 10, LINE))
	theme.set_stylebox("focus", "LineEdit", panel(Color.TRANSPARENT, 8, 0, ACCENT))
	theme.set_color("font_color", "LineEdit", INK)
	theme.set_color("caret_color", "LineEdit", ACCENT)
	theme.set_color("selection_color", "LineEdit", Color("cfe5d7"))
	theme.set_color("font_selected_color", "LineEdit", INK)
	for state in ["up_icon_modulate", "down_icon_modulate", "up_hover_icon_modulate", "down_hover_icon_modulate"]:
		theme.set_color(state, "SpinBox", ACCENT)
	theme.set_stylebox("panel", "PopupMenu", panel(Color.WHITE, 10, 8, LINE))
	theme.set_stylebox("hover", "PopupMenu", panel(Color("e8f2ec"), 6, 6))
	theme.set_color("font_color", "PopupMenu", INK)
	theme.set_color("font_hover_color", "PopupMenu", ACCENT)
	theme.set_constant("v_separation", "PopupMenu", 16)
	theme.set_icon("radio_checked", "PopupMenu", circle(ACCENT, 16))
	theme.set_icon("radio_unchecked", "PopupMenu", circle(LINE, 16, true))
	theme.set_icon("checked", "PopupMenu", circle(ACCENT, 16))
	theme.set_icon("unchecked", "PopupMenu", circle(LINE, 16, true))
	theme.set_icon("submenu", "PopupMenu", icon("next", MUTED))
	theme.set_color("font_separator_color", "PopupMenu", MUTED)
	for state in ["slider", "grabber_area", "grabber_area_highlight"]:
		var track := panel(Color("e0e8e2") if state == "slider" else ACCENT, 3, 0)
		track.content_margin_top = 3
		track.content_margin_bottom = 3
		theme.set_stylebox(state, "HSlider", track)
	theme.set_icon("grabber", "HSlider", circle(ACCENT, 20, true))
	theme.set_icon("grabber_highlight", "HSlider", circle(ACCENT, 22, true))
	theme.set_constant("grabber_offset", "HSlider", 0)
	theme.set_constant("center_grabber", "HSlider", 1)
	theme.set_stylebox("scroll", "VScrollBar", panel(Color.TRANSPARENT, 3, 0))
	theme.set_stylebox("grabber", "VScrollBar", panel(Color("cedbd2"), 3, 0))
	theme.set_stylebox("grabber_highlight", "VScrollBar", panel(Color("aabfb1"), 3, 0))
	theme.set_stylebox("grabber_pressed", "VScrollBar", panel(ACCENT, 3, 0))
	return theme
