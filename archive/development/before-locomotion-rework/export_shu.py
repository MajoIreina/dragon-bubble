"""Export the editable plush reconstruction without executing embedded scripts.

blender --background --disable-autoexec --python tools/export_shu.py
Only the ShuPlush collection enters the portable, animated runtime GLB.
"""
from pathlib import Path
import json
import shutil
import struct
import bpy

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "source_models" / "shu-plush" / "shu-plush.blend"
OUTPUT = ROOT / "skins" / "shu" / "shu.glb"
TEMP_OUTPUT = OUTPUT.with_name("shu-export.glb")
CLIPS = {"Idle", "Walk", "Sleep", "Happy", "PickedUp", "Land"}

if bpy.context.preferences.filepaths.use_scripts_auto_execute:
    raise RuntimeError("Run Blender with --disable-autoexec")
bpy.ops.wm.open_mainfile(filepath=str(SOURCE), use_scripts=False)
bpy.context.scene.frame_set(1)
bpy.ops.object.select_all(action="DESELECT")
collection = bpy.data.collections.get("ShuPlush")
if collection is None:
    raise RuntimeError("Missing ShuPlush character collection")
objects = [obj for obj in collection.all_objects if obj.type not in {"CAMERA", "LIGHT"}]
meshes = [obj for obj in objects if obj.type == "MESH"]
if not meshes:
    raise RuntimeError("No mesh in plush reconstruction")
tracks = {track.name for obj in objects if obj.animation_data
          for track in obj.animation_data.nla_tracks}
if not CLIPS.issubset(tracks):
    raise RuntimeError(f"Missing character NLA tracks: {sorted(CLIPS - tracks)}")
for obj in objects:
    obj.hide_set(False)
    obj.hide_viewport = False
    obj.select_set(True)
    if obj.animation_data:
        for track in obj.animation_data.nla_tracks:
            track.mute = False
bpy.context.view_layer.objects.active = meshes[0]
OUTPUT.parent.mkdir(parents=True, exist_ok=True)
# Retain the pre-reconstruction runtime asset once, including on later exports.
backup = SOURCE.parent / "pre-plush-shu.glb"
if OUTPUT.exists() and not backup.exists():
    shutil.copy2(OUTPUT, backup)
bpy.ops.export_scene.gltf(
    filepath=str(TEMP_OUTPUT), export_format="GLB", use_selection=True,
    export_yup=True, export_materials="EXPORT", export_skins=True,
    export_animations=True, export_animation_mode="NLA_TRACKS",
    export_optimize_animation_keep_anim_object=True,
    export_force_sampling=True, export_cameras=False, export_lights=False,
    export_extras=False,
)
# NLA names must survive exporter grouping before replacing the working skin.
with TEMP_OUTPUT.open("rb") as file:
    magic, version, size = struct.unpack("<4sII", file.read(12))
    chunk_size, chunk_kind = struct.unpack("<I4s", file.read(8))
    if magic != b"glTF" or version != 2 or chunk_kind != b"JSON":
        raise RuntimeError("Blender did not produce a valid GLB 2 file")
    document = json.loads(file.read(chunk_size))
exported_clips = {animation.get("name") for animation in document.get("animations", [])}
if not CLIPS.issubset(exported_clips):
    raise RuntimeError(f"Exported GLB is missing actions: {sorted(CLIPS - exported_clips)}")
if document.get("cameras") or document.get("extensions", {}).get("KHR_lights_punctual"):
    raise RuntimeError("Studio camera or lights leaked into character export")
if size != TEMP_OUTPUT.stat().st_size:
    raise RuntimeError("GLB length does not match its header")
TEMP_OUTPUT.replace(OUTPUT)
print(f"SHU EXPORTED: {OUTPUT} ({OUTPUT.stat().st_size:,} bytes)")
