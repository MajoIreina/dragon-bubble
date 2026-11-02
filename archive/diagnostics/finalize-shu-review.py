from pathlib import Path
import bpy
root=Path('L:/LPP')
source=root/'source_models/shu-plush/shu-plush.blend'
bpy.ops.wm.open_mainfile(filepath=str(source),use_scripts=False)
for screen in bpy.data.screens:
    for area in screen.areas:
        if area.type=='VIEW_3D':
            area.spaces.active.region_3d.view_perspective='CAMERA'
            area.spaces.active.shading.type='MATERIAL'
bpy.ops.object.select_all(action='DESELECT')
body=bpy.data.objects.get('BodyShell • 横向水滴')
body.select_set(True)
bpy.context.view_layer.objects.active=body
image=bpy.data.images.load(str(root/'work/references/contact-sheet.jpg'),check_existing=True)
image.name='Reference • 18 photographs'
image.pack()
bpy.ops.wm.save_as_mainfile(filepath=str(source))
