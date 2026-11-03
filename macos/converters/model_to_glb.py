"""Data-only model conversion invoked by LPP with Blender autoexec disabled."""
import argparse
import sys
from pathlib import Path
import bpy

parser = argparse.ArgumentParser()
parser.add_argument('--input', required=True)
parser.add_argument('--output', required=True)
args = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
source = Path(args.input)
if bpy.context.preferences.filepaths.use_scripts_auto_execute:
    raise RuntimeError('Embedded scripts must be disabled')
if source.suffix.lower() != '.blend':
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete(use_global=False)
if source.suffix.lower() == '.blend':
    bpy.ops.wm.open_mainfile(filepath=str(source), use_scripts=False)
elif source.suffix.lower() == '.stl':
    bpy.ops.wm.stl_import(filepath=str(source))
elif source.suffix.lower() == '.ply':
    bpy.ops.wm.ply_import(filepath=str(source))
else:
    raise ValueError('Unsupported source format')

bpy.context.scene.frame_set(bpy.context.scene.frame_start)
if bpy.context.object and bpy.context.object.mode != 'OBJECT':
    bpy.ops.object.mode_set(mode='OBJECT')
bpy.ops.object.select_all(action='DESELECT')
meshes = [o for o in bpy.context.scene.objects if o.type == 'MESH' and not o.hide_render]
if not meshes:
    raise ValueError('Source has no visible mesh')
selected = set(meshes)
for obj in meshes:
    for modifier in obj.modifiers:
        if modifier.type == 'ARMATURE' and modifier.object:
            selected.add(modifier.object)
for obj in list(selected):
    parent = obj.parent
    while parent:
        selected.add(parent)
        parent = parent.parent
for obj in selected:
    obj.hide_set(False)
    obj.hide_viewport = False
    obj.select_set(True)
bpy.context.view_layer.objects.active = meshes[0]
bpy.ops.export_scene.gltf(
    filepath=args.output, export_format='GLB', use_selection=True,
    export_materials='EXPORT', export_skins=True, export_animations=True,
    export_cameras=False, export_lights=False, export_extras=False,
)
