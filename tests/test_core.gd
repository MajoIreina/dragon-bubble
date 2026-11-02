extends SceneTree

var failures := 0
var checks := 0
const ROOT := "user://test-fixtures"

func _initialize() -> void:
	call_deferred("run")

func check(condition: bool, message: String) -> void:
	checks += 1
	if not condition:
		failures += 1
		push_error(message)

func manifest(data: Dictionary) -> String:
	var path := ROOT.path_join("skin.json")
	var file := FileAccess.open(path, FileAccess.WRITE)
	file.store_string(JSON.stringify(data))
	file.close()
	return path

func run() -> void:
	DirAccess.make_dir_recursive_absolute(ROOT)
	var catalog := SkinCatalog.new()
	catalog.scan()
	check(catalog.skins.size() >= 5, "Shu and all four demonstration skins must load")
	check(not catalog.find_skin(PetPreferences.DEFAULT_SKIN).is_empty(), "Default Shu pack must exist")
	check(catalog.errors.is_empty(), "Built-in manifests must be valid")
	for path in ["../outside.png", "/absolute.png", "C:/secret.png", "foo/../../x.png", "foo\\x.png", "a//b.png"]:
		check(not SkinCatalog.safe_relative(path), "Must reject path: " + path)
	check(SkinCatalog.safe_relative("textures/pet.png"), "Must allow nested relative assets")
	var data := {"schema_version": 1, "id": "test.pet", "name": "Test", "type": "sprite", "asset": "../pet.png"}
	check(not catalog.read_manifest(manifest(data)).ok, "Reject manifest path traversal")
	data.asset = "missing-" + str(Time.get_ticks_usec()) + ".png"
	check(not catalog.read_manifest(manifest(data)).ok, "Reject missing asset")
	data.asset = "pet.png"
	DirAccess.copy_absolute("res://skins/sprite-demo/pet.png", ROOT.path_join("pet.png"))
	check(catalog.read_manifest(manifest(data)).ok, "Accept a valid external PNG pack")
	data.type = "procedural"
	check(not catalog.read_manifest(manifest(data)).ok, "External packs cannot invoke procedural implementation")
	data.type = "sprite"
	data.schema_version = 2
	check(not catalog.read_manifest(manifest(data)).ok, "Reject unsupported schema")
	data.schema_version = 1
	data.animations = {"idle": 42}
	check(not catalog.read_manifest(manifest(data)).ok, "Reject invalid animation mapping")
	data.animations = {}
	data.id = "../escape"
	check(not catalog.read_manifest(manifest(data)).ok, "Reject invalid IDs")
	data.id = "builtin.mochi"
	check(not catalog.import_manifest(manifest(data)).ok, "Imports cannot overwrite an existing skin")
	var brain := PetBrain.new()
	brain.change("dragged")
	brain.tick(100)
	check(brain.state == "dragged", "Drag must not time out")
	brain.change("petted", 1)
	brain.tick(2)
	check(brain.state == "idle", "Pet reaction must return to idle")
	brain.change("walk", 3)
	brain.set_roaming(false)
	check(brain.state == "idle", "Turning roaming off stops walking immediately")
	brain.change("run", 3)
	brain.set_roaming(false)
	check(brain.state == "idle", "Turning roaming off stops running immediately")
	for i in 30:
		brain.tick(100)
		check(brain.state not in ["walk", "run"], "Paused roaming must never walk or run")
	var preferences := PetPreferences.new()
	preferences.path = ROOT.path_join("preferences.json")
	preferences.position = Vector2i(-125, 777)
	preferences.skin_id = "example.glb"
	preferences.zoom = 1.4
	preferences.roaming = false
	check(preferences.save() == OK, "Preferences save")
	var restored := PetPreferences.new()
	restored.path = preferences.path
	restored.load_saved()
	check(restored.position == preferences.position, "Position including negative monitor coordinates round-trips")
	check(restored.skin_id == "example.glb" and is_equal_approx(restored.zoom, 1.4) and not restored.roaming, "Preferences round-trip")
	var broken := FileAccess.open(preferences.path, FileAccess.WRITE)
	broken.store_string("[]")
	broken.close()
	var fallback := PetPreferences.new()
	fallback.path = preferences.path
	fallback.load_saved()
	check(fallback.skin_id == "builtin.shu", "Corrupt preferences fall back to Shu")
	var legacy := FileAccess.open(preferences.path, FileAccess.WRITE)
	legacy.store_string(JSON.stringify({"skin_id": "example.sprite", "zoom": 1.72, "roaming": false, "position": [-20, 777]}))
	legacy.close()
	var migrated := PetPreferences.new()
	migrated.path = preferences.path
	migrated.load_saved()
	check(migrated.skin_id == "builtin.shu", "Old demo preference migrates to Shu")
	check(is_equal_approx(migrated.zoom, 1.72) and not migrated.roaming and migrated.position == Vector2i(-20, 777), "Appearance migration preserves other preferences")
	migrated.skin_id = "builtin.mint"
	check(migrated.save() == OK, "Save appearance revision")
	var chosen := PetPreferences.new()
	chosen.path = preferences.path
	chosen.load_saved()
	check(chosen.skin_id == "builtin.mint", "After migration, choosing a demo persists")
	legacy = FileAccess.open(preferences.path, FileAccess.WRITE)
	legacy.store_string(JSON.stringify({"skin_id": "custom.dragon"}))
	legacy.close()
	chosen.load_saved()
	check(chosen.skin_id == "custom.dragon", "Do not replace user-imported appearances")
	print("CORE TESTS: ", checks, " checks, ", failures, " failures")
	quit(1 if failures else 0)
