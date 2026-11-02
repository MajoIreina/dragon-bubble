extends Node

signal context_requested(cursor: Vector2i)
signal settings_requested
var indicator: StatusIndicator

func setup() -> void:
	if not DisplayServer.has_feature(DisplayServer.FEATURE_STATUS_INDICATOR):
		return
	indicator = StatusIndicator.new()
	indicator.name = "LPPTray"
	indicator.icon = preload("res://assets/tray.svg")
	indicator.tooltip = PetI18n.window_title("左键打开设置，右键快速设置")
	indicator.pressed.connect(_pressed)
	add_child(indicator)
	indicator.visible = true

func retranslate() -> void:
	if is_instance_valid(indicator):
		indicator.tooltip = PetI18n.window_title("左键打开设置，右键快速设置")

func _pressed(button: int, cursor: Vector2i) -> void:
	if button == MOUSE_BUTTON_RIGHT:
		context_requested.emit(cursor)
	elif button == MOUSE_BUTTON_LEFT:
		settings_requested.emit()

func _exit_tree() -> void:
	if is_instance_valid(indicator):
		indicator.visible = false
