import bpy
from mathutils import Vector
bpy.ops.wm.open_mainfile(filepath='L:/LPP/source_models/shu/龙泡泡.blend',use_scripts=False)
o=next(o for o in bpy.context.scene.objects if o.type=='MESH')
for i,m in enumerate(o.data.materials):
 ids={j for p in o.data.polygons if p.material_index==i for j in p.vertices}
 ps=[o.matrix_world@o.data.vertices[j].co for j in ids]
 if ps: print(i,tuple(round(x,4) for x in sum(ps,Vector())/len(ps)),len(ps))
