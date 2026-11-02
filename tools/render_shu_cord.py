"""Blender close views of the right cord, end eyes, clamp and ribbon."""
from pathlib import Path
import bpy
from mathutils import Vector

ROOT=Path(__file__).resolve().parents[1]
bpy.ops.wm.open_mainfile(filepath=str(ROOT/'source_models/shu-plush/shu-plush.blend'),use_scripts=False)
scene=bpy.context.scene
scene.render.resolution_x=1200
scene.render.resolution_y=1200
scene.cycles.samples=80
camera=scene.camera
for name,at,target,scale in [
    ('detail',(4.8,-4.8,2.9),(.70,-.22,.68),1.65),
    ('top',(.65,-.20,5.0),(.65,-.20,.68),1.45),
    ('side',(5,-.35,1.5),(.72,-.26,.66),1.65),
]:
    camera.location=at
    camera.rotation_euler=(Vector(target)-camera.location).to_track_quat('-Z','Y').to_euler()
    camera.data.ortho_scale=scale
    scene.render.filepath=str(ROOT/'outputs'/f'shu-cord-{name}.png')
    bpy.ops.render.render(write_still=True)
    print('RIGHT CORD RENDERED',name,flush=True)
