"""Measure actual ornament surfaces in every clip, not only fitted centerlines."""
from pathlib import Path
import json
import sys
import bpy
import bmesh
from mathutils import Vector

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
from shu_bracelet import _world_tree,_signed
bpy.ops.wm.open_mainfile(filepath=str(ROOT/'source_models/shu-plush/shu-plush.blend'),use_scripts=False)
scene=bpy.context.scene
collection=bpy.data.collections['ShuPlush']
objects=list(collection.all_objects)
head=bpy.data.objects['BodyShell • 横向水滴']
horn=bpy.data.objects['ForkedHorn.R']
ornaments=[o for o in objects if o.type=='MESH' and (
    o.name.startswith(('BraidedHornCord.','SilverRectangularRibbonClamp','ClampTooth.','ClaspEyelet.','RibbonSelvedge.'))
    or o.name=='LongOrangeRedFabricRibbon')]
points={}
for obj in ornaments:
    samples=[v.co.copy() for v in obj.data.vertices]
    # Face centroids and edge midpoints catch chords crossing a curved shell.
    samples.extend(sum((obj.data.vertices[i].co for i in p.vertices),Vector())/len(p.vertices) for p in obj.data.polygons)
    samples.extend((obj.data.vertices[e.vertices[0]].co+obj.data.vertices[e.vertices[1]].co)*.5 for e in obj.data.edges)
    points[obj]=samples
animated=[o for o in objects if o.animation_data and o.animation_data.nla_tracks]
report={'clips':{},'geometry':{}}
failures=[]
for obj in objects:
    if obj.type!='MESH' or obj.name.endswith('.MicroPile'):continue
    if not (obj.name.startswith('StuffedFoot.') or obj.name in ('TailPanel.Outer','TailPanel.InwardFold')):continue
    bm=bmesh.new();bm.from_mesh(obj.data)
    nonmanifold=sum(not edge.is_manifold for edge in bm.edges)
    report['geometry'][obj.name]={'nonmanifold_edges':nonmanifold,'dimensions':list(obj.dimensions)}
    bm.free()
    if nonmanifold:failures.append(f'{obj.name}: {nonmanifold} nonmanifold edges')
for clip in ['Idle','Walk','Run','Turn','Sleep','Happy','PickedUp','Land']:
    for obj in animated:
        for track in obj.animation_data.nla_tracks:track.mute=track.name!=clip
    duration=max(track.strips[0].frame_end for track in animated[0].animation_data.nla_tracks if track.name==clip)
    minimum={'head':(1e9,None,None),'horn':(1e9,None,None)}
    for index in range(33):
        frame=1+(duration-1)*index/32
        scene.frame_set(int(frame),subframe=frame%1)
        bpy.context.view_layer.update()
        trees={'head':_world_tree(head),'horn':_world_tree(horn)}
        for obj,samples in points.items():
            for sample in samples:
                point=obj.matrix_world@sample
                for label,tree in trees.items():
                    distance=_signed(tree,point)[0]
                    if distance<minimum[label][0]:minimum[label]=(distance,obj.name,frame)
    report['clips'][clip]={label:{'minimum_shell_gap':value[0],'object':value[1],'frame':value[2]} for label,value in minimum.items()}
    print('ORNAMENT CLEARANCE',clip,json.dumps(report['clips'][clip]),flush=True)
    for label,(distance,name,frame) in minimum.items():
        required=.0185 if label=='head' else .0085
        if distance<required:failures.append(f'{clip}: {name} {label} gap {distance:.6f} at {frame}')
report['failures']=failures
(ROOT/'work/shu-correction-validation.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
if failures:raise RuntimeError('; '.join(failures))
print('SHU CORRECTION GEOMETRY AND ANIMATED CLEARANCE PASSED',flush=True)
