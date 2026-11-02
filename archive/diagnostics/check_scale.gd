extends SceneTree
const ScaleControl = preload("res://scripts/ui/scale_control.gd")
var failures := 0
var changes: Array[float] = []
func _initialize() -> void:
	call_deferred("run")
func verify(ok: bool, text: String) -> void:
	if not ok:
		failures += 1
		push_error(text)
func run() -> void:
	var control := ScaleControl.new()
	root.add_child(control)
	control.zoom_changed.connect(func(value: float): changes.append(value))
	control.slider.value = 275
	verify(is_equal_approx(changes.back(), 2.75) and control.percentage.value == 275, "Slider synchronizes percentage")
	control.percentage.get_line_edit().text = "137"
	control.commit()
	verify(is_equal_approx(changes.back(), 1.37) and control.slider.value == 137, "Typed percentage applies on submit")
	control.percentage.get_line_edit().text = "999"
	control.commit()
	verify(control.slider.value == 300 and control.percentage.value == 300, "Typed values respect new maximum")
	control.percentage.get_line_edit().text = "5"
	control.commit()
	verify(control.slider.value == 20 and control.percentage.value == 20, "Typed values respect new minimum")
	var count := changes.size()
	control.sync(2.34)
	verify(changes.size() == count and control.percentage.value == 234, "External sync does not cause feedback signals")
	var prefs := PetPreferences.new()
	prefs.path = "user://test-fixtures/scale-prefs.json"
	prefs.zoom = 2.75
	verify(prefs.save() == OK, "Save enlarged scale")
	var restored := PetPreferences.new()
	restored.path = prefs.path
	restored.load_saved()
	verify(is_equal_approx(restored.zoom, 2.75), "New scale survives restart without old clamp")
	control.queue_free()
	await process_frame
	print("SCALE INTEGRATION: 7 checks, ", failures, " failures")
	quit(1 if failures else 0)
