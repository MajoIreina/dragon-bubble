extends SceneTree
const QuickMenu = preload("res://scripts/ui/quick_menu.gd")
func _initialize() -> void:
	call_deferred("run")
func run() -> void:
	root.title = "LPP · 大小控件验证"
	root.borderless = false
	root.unfocusable = false
	root.transparent_bg = false
	root.transparent = false
	root.size = Vector2i(390, 250)
	root.position = Vector2i(450, 220)
	var catalog := SkinCatalog.new()
	catalog.scan()
	var menu := QuickMenu.new()
	root.add_child(menu)
	menu.setup(catalog, PetPreferences.new())
	menu.anchor = Vector2i(470, 250)
	menu.available = DisplayServer.screen_get_usable_rect()
	menu._open_scale()
	root.close_requested.connect(quit)
