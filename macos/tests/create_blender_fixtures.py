"""Run with Blender --background --factory-startup --disable-autoexec."""
from pathlib import Path
import bpy

out = Path(__file__).resolve().parents[1] / 'work' / 'adapter-fixtures'
bpy.ops.object.select_all(action='SELECT')
bpy.ops.object.delete(use_global=False)
bpy.ops.mesh.primitive_uv_sphere_add(segments=12, ring_count=8, location=(0, 0, 1))
mesh = bpy.context.object
mesh.name = 'ChibiBody'
material = bpy.data.materials.new('TexturedCloth')
material.use_nodes = True
image = bpy.data.images.load(str(out / 'image.png'))
image.pack()
texture = material.node_tree.nodes.new('ShaderNodeTexImage')
texture.image = image
material.node_tree.links.new(texture.outputs['Color'], material.node_tree.nodes.get('Principled BSDF').inputs['Base Color'])
mesh.data.materials.append(material)
mesh.scale = (0.5, 0.4, 1)
bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
bpy.ops.object.armature_add(location=(0, 0, 0))
armature = bpy.context.object
bpy.ops.object.mode_set(mode='EDIT')
bone = armature.data.edit_bones[0]
bone.name = 'Hips'
bone.head = (0, 0, 0)
bone.tail = (0, 0, 1)
head = armature.data.edit_bones.new('Head')
head.head = (0, 0, 1)
head.tail = (0, 0, 2)
head.parent = bone
for side, x in [('Left', 0.2), ('Right', -0.2)]:
    leg = armature.data.edit_bones.new(side + 'UpLeg')
    leg.head = (x, 0, 0.7)
    leg.tail = (x, 0, 0.35)
    leg.parent = bone
    calf = armature.data.edit_bones.new(side + 'Leg')
    calf.head = leg.tail
    calf.tail = (x, 0, 0)
    calf.parent = leg
    arm = armature.data.edit_bones.new(side + 'Arm')
    arm.head = (x, 0, 1.5)
    arm.tail = (x*3, 0, 1.4)
    arm.parent = bone
tail = armature.data.edit_bones.new('Tail')
tail.head = (0, 0, 1)
tail.tail = (0, 0.8, 1)
tail.parent = bone
bpy.ops.object.mode_set(mode='OBJECT')
group = mesh.vertex_groups.new(name='Hips')
group.add(list(range(len(mesh.data.vertices))), 1.0, 'REPLACE')
modifier = mesh.modifiers.new('Rig', 'ARMATURE')
modifier.object = armature
mesh.parent = armature
mesh.shape_key_add(name='Basis')
blink = mesh.shape_key_add(name='eyeBlinkLeft')
smile = mesh.shape_key_add(name='mouthSmile')
blink.data[0].co.z += 0.05
smile.data[1].co.x += 0.05
bone = armature.pose.bones['Hips']
bone.rotation_mode = 'XYZ'
for frame, angle in [(1, 0), (10, 0.15), (20, 0)]:
    bone.rotation_euler.y = angle
    bone.keyframe_insert(data_path='rotation_euler', frame=frame)
armature.animation_data.action.name = 'Walk'
bpy.context.scene.frame_end = 20
bpy.context.scene.frame_set(1)
bpy.ops.wm.save_as_mainfile(filepath=str(out / 'chibi.blend'))
bpy.ops.export_scene.gltf(filepath=str(out / 'rig.glb'), export_format='GLB')
bpy.ops.export_scene.fbx(filepath=str(out / 'chibi.fbx'), use_selection=False, add_leaf_bones=False, bake_anim=True, path_mode='COPY', embed_textures=True)
bpy.ops.object.select_all(action='DESELECT')
mesh.select_set(True)
bpy.context.view_layer.objects.active = mesh
bpy.ops.wm.stl_export(filepath=str(out / 'pet.stl'), export_selected_objects=True)
bpy.ops.wm.ply_export(filepath=str(out / 'pet.ply'), export_selected_objects=True)
print('BLENDER FIXTURES OK')
