extends RefCounted

# Fluent light palette shared by settings, flyouts and quick controls.
const INK := Color("1a1a1a")
const MUTED := Color("606060")
const ACCENT := Color("0067c0")
const LINE := Color("e5e5e5")
const BACKGROUND := Color("f3f3f3")
const SURFACE := Color("fbfbfb")
const HOVER := Color("f5f5f5")
const SELECTED := Color("e9e9e9")

static func panel(color: Color, radius := 8, padding := 16, border := Color.TRANSPARENT) -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = color
	style.set_corner_radius_all(radius)
	style.corner_detail = 12
	style.anti_aliasing = true
	style.set_content_margin_all(padding)
	style.border_color = border
	style.set_border_width_all(1 if border.a > 0 else 0)
	return style

static func font(weight := 400) -> SystemFont:
	var result := SystemFont.new()
	result.font_names = PackedStringArray(["Segoe UI Variable Text", "Segoe UI", "Microsoft YaHei UI", "sans-serif"])
	if TranslationServer.get_locale().begins_with("ja"):
		result.font_names = PackedStringArray(["Segoe UI Variable Text", "Segoe UI", "Yu Gothic UI", "Yu Gothic", "Meiryo", "Microsoft YaHei UI", "sans-serif"])
	result.font_weight = weight
	return result

static func texture(body: String, width := 20, height := 20) -> ImageTexture:
	var image := Image.new()
	image.load_svg_from_string('<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d">%s</svg>' % [width, height, width, height, body])
	return ImageTexture.create_from_image(image)

static func icon(kind: String, color := INK) -> ImageTexture:
	var path: String = {
		"skin": '<path d="m12 3 9 5-9 5-9-5 9-5Zm-9 9 9 5 9-5M3 16l9 5 9-5"/>',
		"behavior": '<path d="M6 12c-5-4 0-10 4-5l2 2 2-2c4-5 9 1 4 5l-6 7-6-7Z"/>',
		"display": '<rect x="2.5" y="3.5" width="19" height="13" rx="2"/><path d="M8 21h8M12 17v4"/>',
		"motion": '<path d="m14 3-4 5 4 4-3 8M5 13l5-5m4 4 5 1M4 20l5-4"/><circle cx="17" cy="4" r="2"/>',
		"performance": '<path d="M4 18a9 9 0 1 1 16 0M7 18h10M12 13l5-6"/><circle cx="12" cy="13" r="1"/>',
		"pin": '<path d="m8 3 8 0-1 6 4 4v2H5v-2l4-4-1-6ZM12 15v7"/>',
		"import": '<path d="M12 16V3m-5 5 5-5 5 5M3 16v4a1 1 0 0 0 1 1h16a1 1 0 0 0 1-1v-4"/>',
		"folder": '<path d="M3 6a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/>',
		"exit": '<path d="M9 4H5a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h4m6-13 5 5-5 5M8 12h12"/>',
		"arrow": '<path d="m7 10 5 5 5-5"/>',
		"next": '<path d="m9 6 6 6-6 6"/>',
		"reset": '<path d="M3 4v6h6M3 10a9 9 0 1 1 1 9"/>',
		"search": '<circle cx="10" cy="10" r="6.5"/><path d="m15 15 6 6"/>',
		"check": '<path d="m5 12 4 4L19 6"/>',
		"settings": '<path d="m9 3 6 0 1 3 3 1 2 5-2 5-3 1-1 3H9l-1-3-3-1-2-5 2-5 3-1 1-3Z"/><circle cx="12" cy="12" r="3"/>'
	}.get(kind, "")
	return texture('<g transform="scale(.833333)" fill="none" stroke="#%s" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">%s</g>' % [color.to_html(false), path])

static func circle(color: Color, diameter: int, ring := false) -> ImageTexture:
	return texture('<circle cx="%f" cy="%f" r="%f" fill="white" stroke="%s" stroke-width="1"/><circle cx="%f" cy="%f" r="%f" fill="#%s"/>' % [diameter / 2.0, diameter / 2.0, diameter / 2.0 - 1, "#d1d1d1" if ring else "#" + color.to_html(false), diameter / 2.0, diameter / 2.0, diameter * 0.25, color.to_html(false)], diameter, diameter)

static func animate_button(button: Button) -> void:
	if button.has_meta("fluent_motion") or (button.toggle_mode and not button is OptionButton):
		return
	var normal := button.get_theme_stylebox("normal") as StyleBoxFlat
	var hover := button.get_theme_stylebox("hover") as StyleBoxFlat
	var pressed := button.get_theme_stylebox("pressed") as StyleBoxFlat
	if not normal or not hover or not pressed:
		return
	button.set_meta("fluent_motion", true)
	var surface := normal.duplicate() as StyleBoxFlat
	var colors := [normal.bg_color, hover.bg_color, pressed.bg_color]
	for state in ["normal", "hover", "pressed"]:
		button.add_theme_stylebox_override(state, surface)
	var motion := {"tween": null}
	var transition := func(color: Color):
		if motion.tween:
			motion.tween.kill()
		motion.tween = button.create_tween()
		motion.tween.set_trans(Tween.TRANS_CUBIC).set_ease(Tween.EASE_OUT)
		motion.tween.tween_property(surface, "bg_color", color, 0.10)
	button.mouse_entered.connect(func(): transition.call(colors[1]))
	button.mouse_exited.connect(func(): transition.call(colors[2] if button.button_pressed else colors[0]))
	button.button_down.connect(func(): transition.call(colors[2]))
	button.button_up.connect(func(): transition.call(colors[1] if button.is_hovered() else colors[0]))
	if button is OptionButton:
		button.toggled.connect(func(enabled: bool): transition.call(colors[2] if enabled else colors[1] if button.is_hovered() else colors[0]))

static func switch_texture(enabled: bool, disabled := false) -> ImageTexture:
	var fill := "#c5c5c5" if disabled else "#0067c0" if enabled else "#fafafa"
	var border := fill if enabled else "#8a8a8a"
	var thumb := "white" if enabled else "#606060"
	return texture('<rect x=".5" y=".5" width="39" height="19" rx="9.5" fill="%s" stroke="%s"/><circle cx="%d" cy="10" r="6" fill="%s"/>' % [fill, border, 30 if enabled else 10, thumb], 40, 20)

static func create() -> Theme:
	var theme := Theme.new()
	theme.default_font = font()
	theme.default_font_size = 14
	theme.set_color("font_color", "Label", INK)
	for type in ["Button", "OptionButton"]:
		theme.set_stylebox("normal", type, panel(Color.WHITE, 6, 10, Color("d1d1d1")))
		theme.set_stylebox("hover", type, panel(HOVER, 6, 10, Color("d1d1d1")))
		theme.set_stylebox("pressed", type, panel(Color("ededed"), 6, 10, LINE))
		theme.set_stylebox("disabled", type, panel(Color("f7f7f7"), 6, 10, LINE))
		theme.set_stylebox("focus", type, panel(Color.TRANSPARENT, 6, 0, INK))
		for state in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
			theme.set_color(state, type, INK)
		theme.set_color("font_disabled_color", type, Color("9b9b9b"))
		theme.set_constant("h_separation", type, 12)
	theme.set_icon("arrow", "OptionButton", icon("arrow", MUTED))
	theme.set_constant("arrow_margin", "OptionButton", 10)
	var input := panel(Color.WHITE, 6, 9, Color("c8c8c8"))
	input.border_width_bottom = 2
	theme.set_stylebox("normal", "LineEdit", input)
	var focused := input.duplicate()
	focused.border_color = ACCENT
	theme.set_stylebox("focus", "LineEdit", focused)
	theme.set_color("font_color", "LineEdit", INK)
	theme.set_color("font_placeholder_color", "LineEdit", MUTED)
	theme.set_color("caret_color", "LineEdit", ACCENT)
	theme.set_color("selection_color", "LineEdit", ACCENT)
	theme.set_color("font_selected_color", "LineEdit", Color.WHITE)
	for state in ["up_icon_modulate", "down_icon_modulate", "up_hover_icon_modulate", "down_hover_icon_modulate"]:
		theme.set_color(state, "SpinBox", MUTED)
	theme.set_stylebox("panel", "PopupMenu", panel(Color("fafafa"), 10, 8, Color("d1d1d1")))
	theme.set_stylebox("hover", "PopupMenu", panel(Color("ebebeb"), 4, 6))
	theme.set_color("font_color", "PopupMenu", INK)
	theme.set_color("font_hover_color", "PopupMenu", INK)
	theme.set_constant("v_separation", "PopupMenu", 14)
	theme.set_constant("h_separation", "PopupMenu", 10)
	theme.set_icon("radio_checked", "PopupMenu", circle(ACCENT, 16))
	theme.set_icon("radio_unchecked", "PopupMenu", texture("", 16, 16))
	theme.set_icon("checked", "PopupMenu", icon("check"))
	theme.set_icon("unchecked", "PopupMenu", texture(""))
	theme.set_icon("submenu", "PopupMenu", icon("next", MUTED))
	theme.set_color("font_separator_color", "PopupMenu", MUTED)
	theme.set_stylebox("separator", "PopupMenu", panel(LINE, 0, 0))
	for type in ["CheckButton", "CheckBox"]:
		for enabled in [false, true]:
			var key := "checked" if enabled else "unchecked"
			theme.set_icon(key, type, switch_texture(enabled))
			theme.set_icon(key + "_disabled", type, switch_texture(enabled, true))
		for state in ["normal", "hover", "pressed", "hover_pressed", "disabled"]:
			theme.set_stylebox(state, type, panel(Color.TRANSPARENT, 4, 6))
		theme.set_stylebox("focus", type, panel(Color.TRANSPARENT, 4, 0, INK))
		theme.set_color("font_color", type, INK)
		theme.set_color("font_hover_color", type, INK)
		theme.set_color("font_pressed_color", type, INK)
	for state in ["slider", "grabber_area", "grabber_area_highlight"]:
		var track := panel(Color("8a8a8a") if state == "slider" else ACCENT, 2, 0)
		track.content_margin_top = 2
		track.content_margin_bottom = 2
		theme.set_stylebox(state, "HSlider", track)
	theme.set_icon("grabber", "HSlider", circle(ACCENT, 20, true))
	theme.set_icon("grabber_highlight", "HSlider", circle(ACCENT, 22, true))
	theme.set_constant("grabber_offset", "HSlider", 0)
	theme.set_constant("center_grabber", "HSlider", 1)
	theme.set_stylebox("scroll", "VScrollBar", panel(Color.TRANSPARENT, 2, 0))
	theme.set_stylebox("grabber", "VScrollBar", panel(Color("b5b5b5"), 2, 0))
	theme.set_stylebox("grabber_highlight", "VScrollBar", panel(Color("858585"), 2, 0))
	theme.set_stylebox("grabber_pressed", "VScrollBar", panel(MUTED, 2, 0))
	theme.set_stylebox("panel", "TooltipPanel", panel(SURFACE, 4, 10, LINE))
	theme.set_color("font_color", "TooltipLabel", INK)
	return theme
