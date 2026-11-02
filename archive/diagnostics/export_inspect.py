import bpy,json
from pathlib import Path
report={}
for o in bpy.context.scene.objects:
    if o.type not in {'MESH','ARMATURE','CAMERA'}: continue
    ad=o.animation_data
    item={'parent':o.parent.name if o.parent else None,'rotation':list(o.rotation_euler),
          'scale':list(o.scale),'action':ad.action.name if ad and ad.action else None}
    if ad and ad.action:
        action=ad.action
        item['range']=list(action.frame_range); item['tracks']=[]
        for layer in action.layers:
            for strip in layer.strips:
                for bag in strip.channelbags:
                    for fc in bag.fcurves:
                        item['tracks'].append({'path':fc.data_path,'index':fc.array_index,
                            'keys':[[round(p.co.x,3),round(p.co.y,5)] for p in fc.keyframe_points]})
    if o.type=='ARMATURE': item['bones']=[b.name for b in o.data.bones]
    if o.type=='MESH': item['vertices']=len(o.data.vertices); item['shape_keys']=list(o.data.shape_keys.key_blocks.keys()) if o.data.shape_keys else []
    report[o.name]=item
report['materials']={m.name:[{'color':list(n.inputs['Base Color'].default_value),
     'roughness':n.inputs['Roughness'].default_value,'metallic':n.inputs['Metallic'].default_value}
     for n in m.node_tree.nodes if n.type=='BSDF_PRINCIPLED'] for m in bpy.data.materials}
Path('L:/LPP/work/blend-animation-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
