"""Inspect the corrected feet, folded tail and front-right horn ornament."""
from pathlib import Path
import sys
import bpy
from mathutils import Vector

ROOT=Path(__file__).resolve().parents[1]
bpy.ops.wm.open_mainfile(filepath=str(ROOT/'source_models/shu-plush/shu-plush.blend'),use_scripts=False)
scene=bpy.context.scene
draft='--draft' in sys.argv
scene.cycles.samples=16 if draft else 64
scene.render.resolution_x=800 if draft else 1200
scene.render.resolution_y=800 if draft else 1200
scene.render.resolution_percentage=100
camera=scene.camera
views=[
    ('whole',(4.2,-6.5,2.6),(0,.80,.18),4.45),
    ('side',(7,.85,1.1),(0,1.10,.18),4.45),
    ('feet',(2.8,-2.5,-.25),(.25,.12,-.81),1.65),
    ('soles',(0,.22,-6),(0,.20,-.65),2.25),
    ('tail',(4.8,3.8,1.9),(0,2.67,.20),1.75),
    ('tail-edge',(.65,7.0,1.2),(0,2.67,.20),1.50),
    ('cord-front',(.77,-6,1.1),(.77,-.25,.55),1.85),
    ('cord-side',(5,-.9,1.6),(.76,-.25,.50),1.75),
]
selected=next((a.split('=',1)[1].split(',') for a in sys.argv if a.startswith('--views=')),None)
for name,at,target,scale in views:
    if selected and name not in selected:continue
    floor=bpy.data.objects.get('StudioFloor');floor.hide_render=name=='soles'
    camera.location=at
    camera.rotation_euler=(Vector(target)-camera.location).to_track_quat('-Z','Y').to_euler()
    camera.data.ortho_scale=scale
    scene.render.filepath=str(ROOT/'outputs'/f'shu-corrected-{name}.png')
    bpy.ops.render.render(write_still=True)
    print('CORRECTION RENDERED',name,flush=True)
