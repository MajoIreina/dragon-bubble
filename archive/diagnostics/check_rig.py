import bpy,json
bpy.ops.wm.open_mainfile(filepath='L:/LPP/source_models/shu/龙泡泡.blend',use_scripts=False)
for o in bpy.context.scene.objects:
 if o.type=='MESH':
  print('GROUPS',[(g.name,g.index) for g in o.vertex_groups]); print('WEIGHTED VERTS',sum(bool(v.groups) for v in o.data.vertices))
  for m in o.modifiers: print('MOD',m.name,m.type,m.show_viewport,m.show_render, m.object.name if m.type=='ARMATURE' and m.object else '')
 if o.type=='ARMATURE': print('BONES',[(b.name,b.use_deform) for b in o.data.bones], 'HIDE',o.hide_viewport,o.hide_render)
