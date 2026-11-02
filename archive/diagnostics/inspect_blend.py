import bpy, json
from pathlib import Path
from mathutils import Vector

def box(obj):
    return [list(obj.matrix_world @ Vector(p)) for p in obj.bound_box] if obj.type == 'MESH' else []

data = {
    'scenes': [s.name for s in bpy.data.scenes],
    'objects': [{'name':o.name,'type':o.type,'hidden':o.hide_render,'visible':o.visible_get(),
                 'location':list(o.location),'bounds':box(o),
                 'materials':[s.material.name if s.material else None for s in o.material_slots],
                 'modifiers':[(m.name,m.type) for m in o.modifiers]} for o in bpy.context.scene.objects],
    'materials': [{'name':m.name,'color':list(m.diffuse_color),
                   'nodes':[{'type':n.type,'name':n.name,'image':n.image.name if n.type=='TEX_IMAGE' and n.image else None}
                            for n in m.node_tree.nodes] if m.use_nodes else []} for m in bpy.data.materials],
    'images': [{'name':i.name,'path':i.filepath,'packed':bool(i.packed_file),'size':list(i.size)} for i in bpy.data.images],
    'actions':[a.name for a in bpy.data.actions],
}
Path('L:/LPP/work/blend-report.json').write_text(json.dumps(data,ensure_ascii=False,indent=2),encoding='utf-8')
print('INSPECT OK', len(data['objects']), 'objects')
