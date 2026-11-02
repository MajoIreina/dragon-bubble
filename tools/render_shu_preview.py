"""Render the current saved Shu model without changing the Blender source."""
from pathlib import Path
import hashlib
import bpy
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
source = ROOT / 'source_models/shu-plush/shu-plush.blend'
digest = hashlib.sha256(source.read_bytes()).hexdigest()
bpy.ops.wm.open_mainfile(filepath=str(source), use_scripts=False)
scene = bpy.context.scene
scene.render.engine = 'CYCLES'
scene.cycles.samples = 128
scene.cycles.use_denoising = True
scene.render.resolution_x = 1600
scene.render.resolution_y = 1400
scene.render.resolution_percentage = 100
scene.render.image_settings.file_format = 'PNG'
scene.render.film_transparent = False
camera = scene.camera
camera.data.type = 'ORTHO'
camera.data.ortho_scale = 4.25
camera.location = (4.2, -6.5, 2.6)
target = Vector((0, .72, .20))
camera.rotation_euler = (target-camera.location).to_track_quat('-Z', 'Y').to_euler()
scene.render.filepath = str(ROOT / 'outputs/shu-current-blender-render.png')
bpy.ops.render.render(write_still=True)
if hashlib.sha256(source.read_bytes()).hexdigest() != digest:
    raise RuntimeError('Source file changed during render')
print('CURRENT SHU RENDER:', scene.render.filepath, flush=True)
print('SOURCE PRESERVED:', digest, flush=True)
