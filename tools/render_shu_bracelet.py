"""Render the corrected bracelet from the front, above and diagonally in Blender."""
from pathlib import Path
import bpy
from mathutils import Vector

ROOT=Path(__file__).resolve().parents[1]
bpy.ops.wm.open_mainfile(
    filepath=str(ROOT/'source_models/shu-plush/shu-plush.blend'),use_scripts=False)
scene=bpy.context.scene
camera=scene.camera
scene.render.resolution_x=1024
scene.render.resolution_y=1024
scene.cycles.samples=64
target=Vector((-.65,-.23,.74))
camera.data.ortho_scale=1.30
for name,at in [('front',(-.65,-5,.90)),('top',(-.65,-.23,5)),
                ('oblique',(-2.8,-3.8,3.1))]:
    camera.location=at
    camera.rotation_euler=(target-camera.location).to_track_quat('-Z','Y').to_euler()
    scene.render.filepath=str(ROOT/'outputs'/f'shu-bracelet-{name}-detail.png')
    bpy.ops.render.render(write_still=True)
    print('BRACELET DETAIL RENDERED',name,flush=True)
