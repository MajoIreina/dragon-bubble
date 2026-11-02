"""Photograph-based editable Shu plush reconstruction in Blender 5.x.

Run make_plush_textures.py first, then Blender --background --disable-autoexec
--python tools/build_shu_plush.py. Coordinates in this script: +Y up, +Z face.
The Blender coordinate conversion is (x,-z,y), preserving handedness.
"""
from pathlib import Path
import sys, math, random, json
import bpy
from mathutils import Vector, Matrix, Quaternion
from mathutils.bvhtree import BVHTree

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
from shu_geometry import profile, profile_at_z, face, flank, curve, rounded_outline, linear_rgb
from shu_dorsal import dorsal_geometry
from shu_bracelet import fit_bracelet
from shu_horn_cord import fit_horn_cord, script as cord_script, native as cord_native
from shu_bracelet import _project

OUT=ROOT/'source_models/shu-plush'
OUT.mkdir(parents=True,exist_ok=True)
bpy.ops.wm.read_factory_settings(use_empty=True)
scene=bpy.context.scene
scene.render.engine='CYCLES'
scene.cycles.samples=40
scene.cycles.use_denoising=True
scene.render.resolution_x=1100
scene.render.resolution_y=1100
scene.render.resolution_percentage=100
scene.render.image_settings.file_format='PNG'
scene.render.film_transparent=False
scene.view_settings.view_transform='AgX'
scene.render.fps=24
scene.world=bpy.data.worlds.new('Warm studio')
scene.world.use_nodes=True
scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.65,.70,.68,1)
scene.world.node_tree.nodes['Background'].inputs[1].default_value=.35
pet=bpy.data.collections.new('ShuPlush'); scene.collection.children.link(pet)
studio=bpy.data.collections.new('Studio'); scene.collection.children.link(studio)
origins={}
def B(p): return (p[0],-p[2],p[1])
def attach(obj,collection=pet):
    for col in list(obj.users_collection): col.objects.unlink(obj)
    collection.objects.link(obj)
def group(name,at=(0,0,0),parent=None):
    obj=bpy.data.objects.new(name,None); pet.objects.link(obj)
    obj.parent=parent
    base=origins.get(parent,(0,0,0))
    obj.location=B(tuple(at[k]-base[k] for k in range(3)))
    origins[obj]=at
    obj.empty_display_size=.12
    return obj
root=group('ShuRoot')
body=group('PlushBody',parent=root)
def mesh(name,verts,faces,mat,parent=body,uv=None):
    origin=origins.get(parent,(0,0,0))
    data=bpy.data.meshes.new(name)
    data.from_pydata([B(tuple(v[k]-origin[k] for k in range(3))) for v in verts],[],faces)
    data.update()
    obj=bpy.data.objects.new(name,data); pet.objects.link(obj); obj.parent=parent
    if mat: data.materials.append(mat)
    for p in data.polygons: p.use_smooth=True
    layer=data.uv_layers.new(name='ClothUV')
    for p in data.polygons:
        for li in p.loop_indices:
            vi=data.loops[li].vertex_index
            if uv: layer.data[li].uv=uv[vi]
            else:
                v=verts[vi]
                layer.data[li].uv=(v[1]*4,v[2]*4)
    return obj
def cloth(name,color,texture=None,rough=.9):
    m=bpy.data.materials.new(name); m.use_nodes=True
    p=m.node_tree.nodes.get('Principled BSDF')
    p.inputs['Base Color'].default_value=(*linear_rgb(color),1)
    p.inputs['Roughness'].default_value=rough
    p.inputs['Specular IOR Level'].default_value=.25
    p.inputs['Sheen Weight'].default_value=.22
    if texture:
        tex=m.node_tree.nodes.new('ShaderNodeTexImage')
        tex.image=bpy.data.images.load(str(OUT/'textures'/f'{texture}-cloth.png'),check_existing=True)
        tex.image.pack(); m.node_tree.links.new(tex.outputs['Color'],p.inputs['Base Color'])
    tex=m.node_tree.nodes.new('ShaderNodeTexImage')
    tex.image=bpy.data.images.load(str(OUT/'textures/short-pile-normal.png'),check_existing=True)
    tex.image.colorspace_settings.name='Non-Color'; tex.image.pack()
    norm=m.node_tree.nodes.new('ShaderNodeNormalMap'); norm.inputs['Strength'].default_value=.82
    m.node_tree.links.new(tex.outputs['Color'],norm.inputs['Color'])
    m.node_tree.links.new(norm.outputs['Normal'],p.inputs['Normal'])
    return m
cream=cloth('Ivory short pile • 米白短绒','F2F0E4','cream')
gold=cloth('Harvest yellow plush • 麦黄','D6BE55','gold')
blue=cloth('Indigo padded tail • 靛蓝','39336D','blue')
sage=cloth('Celadon eye embroidery','C2CFAB','sage')
gray=cloth('Gray satin stitch','9A9B9A','gray')
red=cloth('Orange red woven ribbon','DE542A','red')
thread=cloth('Ivory sewing thread','DDD9C7','thread')
ink=cloth('Charcoal embroidery','20251F',rough=.96)
gold_seam=cloth('Golden seam thread','AD9542')
blue_seam=cloth('Indigo seam thread','27234E')
gray_seam=cloth('Checker satin edging','74777A')
white=cloth('Horn vertex gradient','FFFFFF')
attr=white.node_tree.nodes.new('ShaderNodeVertexColor');attr.layer_name='Color'
white.node_tree.links.new(attr.outputs['Color'],white.node_tree.nodes['Principled BSDF'].inputs['Base Color'])
silver=cloth('Brushed silver clasp','B8BDBA',rough=.29)
silver.node_tree.nodes['Principled BSDF'].inputs['Metallic'].default_value=.88
jade=cloth('Milky translucent jade beads','ECEDE1',rough=.18)
jp=jade.node_tree.nodes['Principled BSDF'];jp.inputs['Transmission Weight'].default_value=.28;jp.inputs['IOR'].default_value=1.42
jp.inputs['Sheen Weight'].default_value=0
jp.inputs['Specular IOR Level'].default_value=.45
jp.inputs['Coat Weight'].default_value=.25
jp.inputs['Coat Roughness'].default_value=.12
for link in list(jp.inputs['Normal'].links):jade.node_tree.links.remove(link)

def sphere(name,at,scale,mat,parent=body,segments=40,rings=24):
    bpy.ops.mesh.primitive_uv_sphere_add(segments=segments,ring_count=rings,location=B(at))
    obj=bpy.context.object; obj.name=name;attach(obj)
    obj.scale=(scale[0],scale[2],scale[1])
    bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)
    origin=origins.get(parent,(0,0,0));obj.parent=parent
    obj.location=B(tuple(at[k]-origin[k] for k in range(3)))
    obj.data.materials.append(mat)
    for p in obj.data.polygons:p.use_smooth=True
    for uv in obj.data.uv_layers.active.data:uv.uv*=4
    return obj
def tube(name,path,radii,mat,parent=body,sides=8,flatten=1):
    verts=[];faces=[];uv=[]
    for i,p in enumerate(path):
        tangent=Vector(path[min(i+1,len(path)-1)])-Vector(path[max(0,i-1)])
        tangent.normalize();aux=Vector((0,0,1))
        if abs(tangent.dot(aux))>.94:aux=Vector((0,1,0))
        a=tangent.cross(aux).normalized();b=tangent.cross(a).normalized()
        r=radii[i] if isinstance(radii,list) else radii
        for j in range(sides):
            t=j*math.tau/sides;v=Vector(p)+r*(math.cos(t)*a+math.sin(t)*b*flatten)
            verts.append(tuple(v));uv.append((j/sides*2,i/len(path)*6))
        if i:
            for j in range(sides):
                a0=(i-1)*sides+j;b0=(i-1)*sides+(j+1)%sides
                faces.append((a0,b0,b0+sides,a0+sides))
    faces.append(tuple(reversed(range(sides))))
    faces.append(tuple((len(path)-1)*sides+j for j in range(sides)))
    return mesh(name,verts,faces,mat,parent,uv)
def seam(name,path,mat=thread,radius=.0028,parent=body,stitches=True):
    tube(name,path,radius,mat,parent,6)
    if not stitches:return
    # Short slightly raised transverse thread loops, grouped in one mesh.
    v=[];f=[]
    for i in range(2,len(path)-2,3):
        p=Vector(path[i]); tangent=(Vector(path[i+1])-Vector(path[i-1])).normalized()
        n=Vector((p.x,p.y,0)).normalized()
        if n.length<.1:n=Vector((1,0,0))
        across=tangent.cross(n).normalized()*.006
        a=p-across;b=p+across;c=p+n*.002
        j=len(v);v.extend([tuple(a),tuple(b),tuple(c+tangent*.0015)])
        f.append((j,j+1,j+2))
    if v:mesh(name+'.stitches',v,f,mat,parent)

# Continuous longitudinal body with a gently flattened belly, not a cone.
v=[];f=[];uv=[];rows=160;cols=112
for i in range(rows+1):
    z,r,cy=profile(i*4/rows)
    for j in range(cols+1):
        t=j*math.tau/cols;s=math.sin(t)
        # Belly compression is subtle; preserves the soft teardrop front.
        y=cy+.92*r*s
        if y<-.68:y=-.68+(y+.68)*.78
        v.append((r*math.cos(t),y,z));uv.append((j/cols*3,i/rows*4))
for i in range(rows):
    for j in range(cols):
        a=i*(cols+1)+j;b=a+cols+1;f.append((a,b,b+1,a+1))
shell=mesh('BodyShell • 横向水滴',v,f,cream,uv=uv)

def stuffed_foot(name,at,parent):
    # Circular sewn sole, rounded lower rim and a short stuffed sidewall.
    sections=[]
    for control in [
        ((-.190,0),(-.190,.09),(-.188,.176),(-.146,.211)),
        ((-.146,.211),(-.126,.228),(-.099,.238),(-.071,.239)),
        ((-.071,.239),(-.013,.251),(.070,.237),(.135,.209)),
        ((.135,.209),(.190,.182),(.211,.09),(.211,0)),
    ]:
        sections.extend(curve(*control,14)[:-1])
    sections.append((.211,0))
    verts=[];faces=[];uv=[];sides=80
    for i,(height,radius) in enumerate(sections):
        for j in range(sides):
            angle=math.tau*j/sides
            r=radius*(1+.008*math.cos(3*angle))
            verts.append((at[0]+r*math.cos(angle),at[1]+height,at[2]+r*math.sin(angle)))
            uv.append((j/sides*3,i/(len(sections)-1)*3))
            if i:
                a=(i-1)*sides+j;b=(i-1)*sides+(j+1)%sides
                faces.append((a,b,b+sides,a+sides))
    obj=mesh(name,verts,faces,gold,parent,uv)
    bpy.ops.object.select_all(action='DESELECT')
    bpy.context.view_layer.objects.active=obj;obj.select_set(True)
    bpy.ops.object.mode_set(mode='EDIT');bpy.ops.mesh.select_all(action='SELECT')
    bpy.ops.mesh.remove_doubles(threshold=.00001);bpy.ops.mesh.normals_make_consistent(inside=False)
    bpy.ops.object.mode_set(mode='OBJECT');obj.select_set(False)
    return obj

feet=[]
for side,suffix in [(-1,'L'),(1,'R')]:
    for rear in (False,True):
        at=(side*(.47 if rear else .54),-.835,-.69 if rear else .28)
        pivot=group(('RearFoot.' if rear else 'FrontFoot.')+suffix,at,body);feet.append(pivot)
        foot=stuffed_foot('StuffedFoot.'+('Rear' if rear else 'Front')+suffix,at,pivot)
        # Sole joins the upper foot around the lower perimeter.
        path=[(at[0]+.2405*(1+.008*math.cos(3*t))*math.cos(t),at[1]-.071,at[2]+.2405*(1+.008*math.cos(3*t))*math.sin(t)) for t in [math.tau*k/160 for k in range(161)]]
        seam('FootSoleSeam.'+str(len(feet)),path,gold_seam,.0024,pivot)

horns=[]
for side,suffix in [(-1,'L'),(1,'R')]:
    # Thick forward spur is fused to the long upward/outward tip by voxel remesh.
    main=curve((side*.56,.48,.13),(side*.66,.84,.08),(side*.91,1.19,.03),(side*1.10,1.43,.0),48)
    radii=[(.225+.035*math.sin(math.pi*i/48))*(1-i/48)**.70+.010 for i in range(49)]
    a=tube('ForkedHorn.'+suffix,main,radii,white,sides=32)
    spur=curve((side*.59,.61,.18),(side*.69,.63,.36),(side*.74,.72,.54),(side*.77,.81,.55),32)
    b=tube('HornSpurTemp',spur,[.198*(1-i/32)**.68+.008 for i in range(33)],white,sides=28)
    bpy.ops.object.select_all(action='DESELECT');a.select_set(True);b.select_set(True);bpy.context.view_layer.objects.active=a
    bpy.ops.object.join()
    rem=a.modifiers.new('Unified sewn horn silhouette','REMESH');rem.mode='VOXEL';rem.voxel_size=.019;rem.use_smooth_shade=True
    bpy.ops.object.modifier_apply(modifier=rem.name)
    sm=a.modifiers.new('Soft stuffing transitions','SMOOTH');sm.factor=.7;sm.iterations=6
    bpy.ops.object.modifier_apply(modifier=sm.name)
    for p in a.data.polygons:p.use_smooth=True
    colors=a.data.color_attributes.new(name='Color',type='FLOAT_COLOR',domain='POINT')
    low=linear_rgb('D6BE55');high=linear_rgb('37316A')
    for vi,vert in enumerate(a.data.vertices):
        height=vert.co.z
        k=min(1,max(0,(height-.93)/.32));k=k*k*(3-2*k)
        colors.data[vi].color=(*(low[n]*(1-k)+high[n]*k for n in range(3)),1)
    # Remeshing drops UVs: cylindrical cloth coordinates in Blender axes.
    if not a.data.uv_layers:a.data.uv_layers.new(name='ClothUV')
    for loop in a.data.loops:
        co=a.data.vertices[loop.vertex_index].co
        a.data.uv_layers.active.data[loop.index].uv=(co.x*5,co.z*5)
    horns.append(a)
    # The actual seam follows the front face of the horn and forward spur.
    seam_path=[]
    for i in range(5,47):
        p=list(main[i]);p[2]+=radii[i]*.97;seam_path.append(tuple(p))
    # Match the seam to the gradient rather than placing a yellow stripe over indigo.
    for part,material in [(seam_path[:25],gold_seam),(seam_path[24:],blue_seam)]:
        seam('HornLongitudinalSeam.'+suffix+str(len(part)),part,material,.0022,stitches=False)
    seam('HornSpurSeam.'+suffix,[tuple(Vector(p)+Vector((0,-.04,.08))) for p in spur[2:28]],gold_seam,.0023,stitches=False)

# Continuous three-peak strip with swept slopes, fitted for the settled body.
v,f,uv,ridge=dorsal_geometry()
dorsal=mesh('ContinuousDorsalFin',v,f,gold,uv=uv)
seam('DorsalRidgeSeam',ridge,gold_seam,.0022,stitches=False)

# Face embroidery is fitted to the curved front, including the upper eyelid.
eyes=group('BlinkBothEyes',(0,-.35,.82),body)
closed_eyes=group('ClosedEyelids',(0,-.35,.82),body)
closed_eyes.scale=(.001,.001,.001)
def eye_outline(cx,scale=1):
    paths=[((-.21,.16),(-.08,.183),(.08,.183),(.21,.16)),
           ((.21,.16),(.21,-.14),(.15,-.28),(0,-.28)),
           ((0,-.28),(-.15,-.28),(-.21,-.14),(-.21,.16))]
    return [(cx+x*scale,-.32+y*scale) for path in paths for x,y in curve(*path,20)[:-1]]
def patch(name,outline,mat,parent=body,offset=.012):
    center=tuple(sum(p[k] for p in outline)/len(outline) for k in range(2));n=len(outline)
    v=[face(*center,offset)];uv=[(center[0]*8,center[1]*8)];f=[];rings=10
    for j in range(1,rings+1):
        for x,y in outline:
            xx=center[0]+(x-center[0])*j/rings;yy=center[1]+(y-center[1])*j/rings
            v.append(face(xx,yy,offset));uv.append((xx*8,yy*8))
    for k in range(n):f.append((0,1+k,1+(k+1)%n))
    for j in range(rings-1):
        for k in range(n):
            a=1+j*n+k;b=1+j*n+(k+1)%n;f.append((a,a+n,b+n,b))
    return mesh(name,v,[tuple(reversed(face_indices)) for face_indices in f],mat,parent,uv)
for side in (-1,1):
    cx=side*.425;outline=eye_outline(cx)
    tube('EmbroideredEyeBorder.'+str(side),[face(x,y,.019) for x,y in outline+[outline[0]]],.009,ink,eyes,8)
    patch('CeladonEyeFill.'+str(side),eye_outline(cx,.97),sage,eyes,.018)
    path=[face(cx+x,-.16+.014*math.cos(x/.24*math.pi/2),.026) for x in [-.24+i*.48/40 for i in range(41)]]
    tube('HeavyUpperLid.'+str(side),path,.019,ink,eyes,10)
    closed_path=[face(cx+x,-.35-.021*math.cos(x/.20*math.pi/2),.033) for x in [-.20+i*.40/40 for i in range(41)]]
    tube('ClosedEmbroideredLid.'+str(side),closed_path,.011,ink,closed_eyes,8)
    spot=face(side*.285,.008,.014)
    sphere('GrayForeheadSpot.'+str(side),spot,(.075,.057,.011),gray,segments=32,rings=16)
    path=[face(side*.285+.075*math.cos(t),.008+.057*math.sin(t),.025) for t in [i*math.tau/70 for i in range(71)]]
    tube('SpotEmbroideredEdge.'+str(side),path,.0028,gray_seam)
mouth=curve((-.135,-.59),(-.085,-.635),(-.045,-.635),(0,-.59),20)[:-1]+curve((0,-.59),(.045,-.635),(.085,-.635),(.135,-.59),20)
tube('TinyWEmbroideredMouth',[face(x,y,.023) for x,y in mouth],.012,ink,sides=10)

# Five surface-conforming satin squares in an X arrangement on each flank.
for side in (-1,1):
    for row in range(3):
        for col in range(3):
            if (row+col)%2:continue
            y0=-.23+row*.23;z0=-.94+col*.23;edge=.214;steps=14
            v=[];f=[];uv=[]
            for j in range(steps+1):
                for k in range(steps+1):
                    y=y0+edge*j/steps;z=z0+edge*k/steps
                    v.append(flank(side,y,z,.009));uv.append((j/steps*2,k/steps*2))
                    if j and k:
                        a=(j-1)*(steps+1)+k-1;b=a+steps+1
                        quad=(a,b,b+1,a+1);f.append(quad if side>0 else tuple(reversed(quad)))
            name=f'Checker.{side}.{row}.{col}';mesh(name,v,f,gray,uv=uv)
            p=[]
            for a,b in [((0,0),(edge,0)),((edge,0),(edge,edge)),((edge,edge),(0,edge)),((0,edge),(0,0))]:
                for j in range(22):
                    t=j/22;y=y0+a[0]*(1-t)+b[0]*t;z=z0+a[1]*(1-t)+b[1]*t
                    p.append(flank(side,y,z,.014))
            p.append(p[0]);seam(name+'.SatinBorder',p,gray_seam,.0035,stitches=False)

# Jade beads follow the actual complete horn base and the sloping head shell.
bpy.context.view_layer.update()
left_main=curve((-.56,.48,.13),(-.66,.84,.08),(-.91,1.19,.03),(-1.10,1.43,.0),48)
bead_centers,bead_radius,bracelet_path=fit_bracelet(horns[0],shell,left_main,ROOT/'work/shu-bracelet-fit.json')
bracelet=group('JadeBracelet',parent=body)
for i,p in enumerate(bead_centers):
    sphere(f'JadeBead.{i+1:02}',p,(bead_radius,)*3,jade,bracelet,segments=32,rings=20)
tube('BraceletThread',bracelet_path,.002,thread,bracelet,sides=6)
# A fitted white braid wraps the right base and terminates at the clamp eyes.
right_main=curve((.56,.48,.13),(.66,.84,.08),(.91,1.19,.03),(1.10,1.43,.0),48)
cord_fit=fit_horn_cord(horns[1],shell,right_main,ROOT/'work/shu-right-cord-fit.json')
cord_attachment=group('HornCordAttachment',parent=body)
cord_path=cord_fit['path'];cord_t=cord_fit['tangent'];cord_v=cord_fit['down'];cord_n=cord_fit['outward']
frame=[];previous=None;distance=0
for i,p in enumerate(cord_path):
    tangent=(cord_path[min(i+1,len(cord_path)-1)]-cord_path[max(0,i-1)]).normalized()
    if previous is None:
        previous=cord_n-tangent*cord_n.dot(tangent)
    perpendicular=(previous-tangent*previous.dot(tangent)).normalized()
    previous=perpendicular
    if i:distance+=(p-cord_path[i-1]).length
    frame.append((perpendicular,tangent.cross(perpendicular).normalized(),distance))
for braid in range(3):
    path=[]
    for p,(normal,binormal,distance) in zip(cord_path,frame):
        phase=distance*math.tau/.025+braid*math.tau/3
        point=p+.0034*(normal*math.cos(phase)+binormal*math.sin(phase))
        path.append(cord_script(point))
    tube('BraidedHornCord.'+str(braid),path,.0034,thread,cord_attachment,sides=8)
pendant_origin=cord_script(cord_fit['hinge']);pendant=group('PendantSway',pendant_origin,body)
from shu_horn_cord import fit_swept_ribbon
ribbon_surface=fit_swept_ribbon(cord_fit)
def ribbon_point(t,s):
    return cord_script(ribbon_surface(t,s))
v=[];f=[];uv=[]
for j in range(49):
    for k in range(9):
        v.append(ribbon_point(j/48,k/4-1));uv.append((k/8,j/48*5))
        if j and k:
            a=(j-1)*9+k-1;f.append((a,a+9,a+10,a+1))
ribbon=mesh('LongOrangeRedFabricRibbon',v,f,red,pendant,uv)
sol=ribbon.modifiers.new('Fabric thickness','SOLIDIFY');sol.thickness=.005
bpy.context.view_layer.objects.active=ribbon;bpy.ops.object.modifier_apply(modifier=sol.name)
for side in (-1,1):seam('RibbonSelvedge.'+str(side),[ribbon_point(i/100,side*.96) for i in range(101)],gold_seam,.0018,pendant,False)
def cuboid(name,at,scale,mat,parent,bevel=.01,basis=None):
    bpy.ops.mesh.primitive_cube_add(size=1,location=B(at));obj=bpy.context.object;obj.name=name;attach(obj)
    obj.scale=(scale[0],scale[2],scale[1]);bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)
    origin=origins[parent];obj.parent=parent;obj.location=B(tuple(at[k]-origin[k] for k in range(3)));obj.data.materials.append(mat)
    mod=obj.modifiers.new('Soft metal corners','BEVEL');mod.width=bevel;mod.segments=3;bpy.context.view_layer.objects.active=obj;bpy.ops.object.modifier_apply(modifier=mod.name)
    for p in obj.data.polygons:p.use_smooth=True
    if basis is not None:obj.rotation_euler=basis.to_euler()
    return obj
clamp_basis=Matrix((cord_t,-cord_n,-cord_v)).transposed()
clamp=cuboid('SilverRectangularRibbonClamp',cord_script(cord_fit['center']),(.244,.030,.022),silver,cord_attachment,.003,clamp_basis)
for side,eye_center in enumerate(cord_fit['eyes']):
    ring=[cord_script(eye_center+cord_t*(.0115*math.cos(t))+cord_v*(.0115*math.sin(t))) for t in [math.tau*i/64 for i in range(65)]]
    tube('ClaspEyelet.'+str(side),ring,.0022,silver,cord_attachment,sides=10)
for i in range(13):
    center=cord_fit['center']+cord_t*((i/12-.5)*.226)+cord_v*.018+cord_n*.006
    cuboid('ClampTooth.'+str(i),cord_script(center),(.010,.012,.013),silver,cord_attachment,.0018,clamp_basis)

# Thin padded cloth panels, with a forked outline and an inward fold.
tail=group('TailWag',(0,.10,-2.12),body)
def padded_panel(name,corners,at,thickness,crease):
    outline=rounded_outline(corners,.19);n=len(outline)
    center=tuple(sum(p[k] for p in outline)/n for k in range(2));rings=32;v=[];f=[];uv=[]
    start_fold,end_fold=map(Vector,crease)
    fold_axis=end_fold-start_fold
    def surface(side,y,z,t):
        yz=Vector((y,z));along=max(0,min(1,(yz-start_fold).dot(fold_axis)/fold_axis.length_squared))
        distance=(yz-(start_fold+fold_axis*along)).length
        envelope=max(0,1-t*t)**.55
        # Sculpt the depression and adjacent soft roll into the stuffed cloth.
        valley=.0345*math.exp(-(distance/.090)**2)*math.sin(math.pi*along)**.55
        roll=.0115*math.exp(-((distance-.115)/.072)**2)*math.sin(math.pi*along)
        h=max(.009,thickness-valley+roll)*envelope
        sweep=.017*math.sin(math.pi*t)*math.sin((z+.8)*3)
        return (at[0]+side*h+sweep,at[1]+y*.78,at[2]+z*.90)
    for side in (-1,1):
        start=len(v)
        for j in range(rings+1):
            t=max(.0001,j/rings)
            for p in outline:
                y=center[0]+(p[0]-center[0])*t;z=center[1]+(p[1]-center[1])*t
                v.append(surface(side,y,z,t));uv.append((y*5,z*5))
        for j in range(rings):
            for i in range(n):
                a=start+j*n+i;b=start+j*n+(i+1)%n;quad=(a,b,b+n,a+n)
                f.append(quad if side<0 else tuple(reversed(quad)))
    # Orient each padded side outward; shared edge rings remain watertight in silhouette.
    fixed=[]
    for indices in f:
        a,b,c=[Vector(v[k]) for k in indices[:3]]
        normal=(b-a).cross(c-a)
        side=-1 if sum(v[k][0] for k in indices)/len(indices)<at[0] else 1
        fixed.append(indices if normal.x*side>=0 else tuple(reversed(indices)))
    obj=mesh(name,v,fixed,blue,tail,uv)
    bpy.ops.object.select_all(action='DESELECT')
    bpy.context.view_layer.objects.active=obj;obj.select_set(True)
    bpy.ops.object.mode_set(mode='EDIT');bpy.ops.mesh.select_all(action='SELECT')
    bpy.ops.mesh.remove_doubles(threshold=.00015);bpy.ops.mesh.normals_make_consistent(inside=False)
    bpy.ops.object.mode_set(mode='OBJECT');obj.select_set(False)
    path=[(at[0],at[1]+p[0]*.78,at[2]+p[1]*.90) for p in outline];path.append(path[0])
    seam(name+'.SewnEdge',path,blue_seam,.0022,tail,False)
    return obj
panels=[]
panels.append(padded_panel('TailPanel.Outer',
    [(.035,.04),(.04,-.32),(.38,-.67),(.61,-.91),(.58,-1.035),(-.42,-.94),(-.51,-.73),(-.48,-.28),(-.15,-.035)],
    (-.020,.10,-2.12),.076,((-.015,-.15),(-.39,-.78))))
panels.append(padded_panel('TailPanel.InwardFold',
    [(.025,.065),(.51,-.17),(.73,-.37),(.69,-.49),(.34,-.64),(-.16,-.64),(-.245,-.43),(-.075,-.13)],
    (.0375,.10,-2.12),.071,((.14,-.14),(-.15,-.52))))

# Body panel seams are kept on the flanks and belly; the face is uninterrupted.
for side in (-1,1):
    path=[]
    for i in range(180):
        z=.04-i*2.12/179;_,r,cy=profile_at_z(z)
        path.append((side*r*.69,cy+r*.92*.724,z))
    seam('BodyPanelSeam.'+str(side),path,thread,.0021,stitches=False)
belly_seam=[]
for i in range(241):
    t=i*math.tau/240;x=.61*math.cos(t);z=-.43+.96*math.sin(t)
    _,r,cy=profile_at_z(z)
    y=cy-.92*r*math.sqrt(max(.001,1-(x/max(.001,r))**2))
    if y<-.68:y=-.68+(y+.68)*.78
    belly_seam.append((x,y-.002,z))
seam('OvalBellyPanelSeam',belly_seam,thread,.002,stitches=False)

# Exportable short fibers. These are real triangles, not Blender-only particle hair.
# The normal map carries density at desktop scale; microfibers soften silhouettes.
def fur(obj,count,length,mat):
    rng=random.Random(obj.name);data=obj.data;data.calc_loop_triangles()
    tris=[];weights=[];total=0
    for tri in data.loop_triangles:
        a,b,c=[data.vertices[i].co for i in tri.vertices]
        area=(b-a).cross(c-a).length*.5
        if area<1e-10:continue
        total+=area;tris.append(tri);weights.append(total)
    import bisect
    verts=[];faces=[];colors=[]
    src=data.color_attributes.get('Color')
    for _ in range(count):
        tri=tris[bisect.bisect_left(weights,rng.random()*total)]
        r=math.sqrt(rng.random());s=rng.random();w=(1-r,r*(1-s),r*s)
        vs=[data.vertices[i] for i in tri.vertices]
        p=sum((vs[i].co*w[i] for i in range(3)),Vector())
        n=sum((vs[i].normal*w[i] for i in range(3)),Vector()).normalized()
        axis=n.cross(Vector((0,0,1)))
        if axis.length<.1:axis=n.cross(Vector((1,0,0)))
        axis.normalize();other=n.cross(axis);a=rng.random()*math.tau
        tangent=axis*math.cos(a)+other*math.sin(a)
        le=length*rng.uniform(.55,1.3);width=length*.07
        p+=n*.0005;j=len(verts)
        verts.extend([tuple(p-axis*width),tuple(p+axis*width),tuple(p+n*le*.55+tangent*le*.20),tuple(p+n*le+tangent*le*.42)])
        faces.extend([(j,j+1,j+2),(j+1,j+3,j+2)])
        if src:
            color=tuple(sum(src.data[tri.vertices[k]].color[q]*w[k] for k in range(3)) for q in range(4));colors.extend([color]*4)
    me=bpy.data.meshes.new(obj.name+'.MicroPile');me.from_pydata(verts,[],faces);me.materials.append(mat)
    ob=bpy.data.objects.new(obj.name+'.MicroPile',me);pet.objects.link(ob);ob.parent=obj
    if colors:
        attr=me.color_attributes.new(name='Color',type='FLOAT_COLOR',domain='POINT')
        for i,color in enumerate(colors):attr.data[i].color=color
    for p in me.polygons:p.use_smooth=True
    return ob
fur(shell,50000,.014,cream)
for obj in list(pet.objects):
    if obj.type!='MESH':continue
    if obj.name.startswith('StuffedFoot'):fur(obj,1400,.006,gold)
for obj in horns:fur(obj,3300,.006,white)
fur(dorsal,2500,.005,gold)
for obj in panels:fur(obj,3200,.006,blue)

# Six named NLA tracks share names across animated parts so glTF merges clips.
animated=[body,eyes,closed_eyes,tail,pendant]+feet
rests={o:(o.location.copy(),o.rotation_euler.copy(),o.scale.copy()) for o in animated}
clips={'Idle':4.8,'Walk':1.2,'Sleep':4.8,'Happy':1.5,'PickedUp':1.8,'Land':.8}
for name,duration in clips.items():
    frames=round(duration*24)
    for obj in animated:
        obj.animation_data_create();obj.animation_data.action=None
        loc,rot,sca=rests[obj]
        for i in range(33):
            t=i/32;phase=t*math.tau;obj.location=loc;obj.rotation_euler=rot;obj.scale=sca
            if obj==body:
                breath=.006*math.sin(phase)
                y=.93+.008*math.sin(phase) if name=='Sleep' else 1+breath
                if name=='Walk':y=1+.018*math.cos(phase*2);obj.location.z+=.028*abs(math.sin(phase))
                if name=='Happy':y=1+.03*math.sin(phase*2)
                if name=='PickedUp':y=1.04+.005*math.sin(phase);obj.rotation_euler.y=.028*math.sin(phase)
                if name=='Land':y=1-.12*math.sin(phase/2)*math.exp(-t*3)
                obj.scale=(1/math.sqrt(y),1/math.sqrt(y),y)
            elif obj==eyes:
                blink=max(0,1-abs(t-.77)/.032)
                y=.04 if name=='Sleep' else 1-.95*blink
                if name=='Happy':y=.68+.12*math.cos(phase)
                obj.scale=(1,1,y)
            elif obj==closed_eyes:
                blink=max(0,1-abs(t-.77)/.032)
                amount=1 if name=='Sleep' else max(.001,(blink-.65)/.35)
                obj.scale=(amount,amount,amount)
            elif obj==tail:obj.rotation_euler.z=(.16 if name in ('Walk','Happy') else .045)*math.sin(phase)
            elif obj==pendant:
                angle=(.075 if name in ('Walk','PickedUp') else .018)*math.sin(phase+.3)
                obj.rotation_euler=Quaternion(cord_t,angle).to_euler()
            else:
                index=feet.index(obj);wave=math.sin(phase+(index%2)*math.pi+(math.pi if index>=2 else 0))
                if name=='Walk':obj.location.z+=max(0,wave)*.04;obj.rotation_euler.x=wave*.18
                if name=='PickedUp':obj.rotation_euler.x=.12+.06*math.sin(phase)
            frame=1+i*frames/32
            for prop in ('location','rotation_euler','scale'):obj.keyframe_insert(data_path=prop,frame=frame,group=obj.name)
        action=obj.animation_data.action;action.name=name+'.'+obj.name
        track=obj.animation_data.nla_tracks.new();track.name=name
        strip=track.strips.new(name,1,action);strip.action_frame_start=1;strip.action_frame_end=frames+1
        obj.animation_data.action=None
for obj,(loc,rot,sca) in rests.items():obj.location=loc;obj.rotation_euler=rot;obj.scale=sca
scene.frame_start=1;scene.frame_end=116

# Studio is stored separately and excluded from runtime export.
def studio_obj(obj):attach(obj,studio);return obj
bpy.ops.mesh.primitive_plane_add(size=200,location=(0,0,-1.035));floor=studio_obj(bpy.context.object);floor.name='StudioFloor'
floor_mat=bpy.data.materials.new('Studio pale sage');floor_mat.diffuse_color=(.67,.73,.67,1);floor.data.materials.append(floor_mat)
for name,at,power,size in [('Key',(-3,-4,6),520,5),('Fill',(4,-1,3),240,4),('Rim',(1,5,5),420,3)]:
    data=bpy.data.lights.new(name,'AREA');data.energy=power;data.shape='DISK';data.size=size
    ob=bpy.data.objects.new(name,data);studio.objects.link(ob);ob.location=at;ob.rotation_euler=(Vector((0,.8,0))-ob.location).to_track_quat('-Z','Y').to_euler()
cam_data=bpy.data.cameras.new('ReviewCamera');cam=bpy.data.objects.new('ReviewCamera',cam_data);studio.objects.link(cam);scene.camera=cam
cam_data.type='ORTHO';cam_data.ortho_scale=4.7
def camera_at(at,target=(0,.18,-.82)):
    cam.location=B(at);cam.rotation_euler=(Vector(B(target))-cam.location).to_track_quat('-Z','Y').to_euler()
camera_at((4.1,2.5,5.8))
# Keep NLA strips disabled only for the static source default pose and previews.
for obj in animated:
    for track in obj.animation_data.nla_tracks:track.mute=True
scene.frame_set(1)
for obj,(loc,rot,sca) in rests.items():obj.location=loc;obj.rotation_euler=rot;obj.scale=sca
scene['ReferenceNotes']='18 user photographs IMG_20261101; bowl and brand tag omitted; actual geometry micro-pile plus packed normal textures.'
scene['CoordinateSystem']='Script +Y up,+Z face; Blender +Z up,-Y face; GLB +Y up,+Z face.'
scene['ModelVersion']='2026-11-02 photograph reconstruction v7 • lower swept dorsal peaks'
for screen in bpy.data.screens:
    for area in screen.areas:
        if area.type=='VIEW_3D':
            area.spaces.active.region_3d.view_perspective='CAMERA'
            area.spaces.active.shading.type='MATERIAL'
bpy.ops.object.select_all(action='DESELECT')
shell.select_set(True);bpy.context.view_layer.objects.active=shell
reference=ROOT/'work/references/contact-sheet.jpg'
if reference.exists():
    image=bpy.data.images.load(str(reference),check_existing=True)
    image.name='Reference • 18 photographs';image.pack()
# Export script unmutes NLA tracks for export, then restores their mute state.
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'shu-plush.blend'))
print('SHU PLUSH SOURCE SAVED',flush=True)
if '--skip-renders' in sys.argv:
    print('SHU PLUSH BUILD COMPLETE (render separately)',flush=True)
    sys.exit(0)
for name,at in [('front',(0,.40,7)),('three-quarter',(4.1,2.0,5.8)),('side',(7,1.2,-.82)),('back',(0,1.3,-7)),('details',(3.3,2.9,4.7))]:
    camera_at(at);scene.render.filepath=str(ROOT/'outputs'/f'shu-plush-blender-{name}.png')
    bpy.ops.render.render(write_still=True)
    print('RENDERED',name,flush=True)
print('SHU PLUSH BUILD COMPLETE',flush=True)
