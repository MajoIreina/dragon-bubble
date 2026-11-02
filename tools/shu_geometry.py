"""Editable Shu surfaces based on the supplied front / side / detail photographs.

All proportions use a shared coordinate system: +Z face, -Z tail, +Y up.
The body uses a continuous Bezier profile; features are placed on that surface.
"""
import math

PI = math.pi


def add(a,b): return tuple(x+y for x,y in zip(a,b))
def sub(a,b): return tuple(x-y for x,y in zip(a,b))
def mul(a,s): return tuple(x*s for x in a)
def mix(a,b,t): return add(mul(a,1-t),mul(b,t))
def cross(a,b): return (a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0])
def unit(a): return mul(a,1/max(1e-10,math.sqrt(sum(x*x for x in a))))


def cubic(a,b,c,d,t):
    return tuple((1-t)**3*a[k]+3*(1-t)**2*t*b[k]+3*(1-t)*t*t*c[k]+t**3*d[k] for k in range(len(a)))


def curve(a,b,c,d,steps=24):
    return [cubic(a,b,c,d,i/steps) for i in range(steps+1)]


# Each point is (Z, cross-section radius, vertical center). Shared tangents
# keep the broad head, low belly and rising tail root visually continuous.
PROFILE = [
    ((.86,0,0),(.86,.53,0),(.63,.98,-.025),(.10,.98,-.025)),
    ((.10,.98,-.025),(-.33,.98,-.025),(-.68,.92,-.08),(-1.05,.68,-.035)),
    ((-1.05,.68,-.035),(-1.48,.38,.01),(-1.88,.075,.07),(-2.16,.065,.09)),
    ((-2.16,.065,.09),(-2.22,.063,.095),(-2.26,.03,.10),(-2.26,0,.10)),
]
Y_RATIO = .92


def profile(t):
    i=min(3,int(t)); local=min(1,max(0,t-i))
    return cubic(*PROFILE[i],local)


def profile_at_z(z):
    lo,hi=0.,4.
    for _ in range(35):
        mid=(lo+hi)/2
        if profile(mid)[0]>z: lo=mid
        else: hi=mid
    return profile((lo+hi)/2)


def face(x,y,offset=.008):
    lo,hi=.000001,1.
    for _ in range(32):
        t=(lo+hi)/2; z,r,cy=profile(t)
        rho=math.sqrt(x*x+((y-cy)/Y_RATIO)**2)
        if r<rho: lo=t
        else: hi=t
    z=profile((lo+hi)/2)[0]
    return (x,y,z+offset)


def flank(side,y,z,offset=.008):
    _,r,cy=profile_at_z(z)
    x=r*math.sqrt(max(.005,1-((y-cy)/(Y_RATIO*r))**2))
    return (side*(x+offset),y,z)


def rounded_outline(corners,rounding=.07):
    points=[]
    for i,p in enumerate(corners):
        before=mix(p,corners[(i-1)%len(corners)],rounding)
        after=mix(p,corners[(i+1)%len(corners)],rounding)
        next_before=mix(corners[(i+1)%len(corners)],p,rounding)
        for j in range(7):
            t=j/7
            points.append(add(add(mul(before,(1-t)**2),mul(p,2*(1-t)*t)),mul(after,t*t)))
        for j in range(9): points.append(mix(after,next_before,j/9))
    return points


def pillow(g,name,corners,thickness,mat,parent=0,plane='yz',at=(0,0,0)):
    """Soft-edged solid fin. Corners are in Y/Z or X/Y, thickness across it."""
    outline=rounded_outline(corners)
    center=tuple(sum(p[k] for p in outline)/len(outline) for k in range(2))
    v=[]; f=[]; normals=[]; rings=16; n=len(outline)
    for side in (-1,1):
        offset=len(v)
        for j in range(rings+1):
            r=max(.00001,j/rings)
            for i,p in enumerate(outline):
                a,b=mix(center,p,r)
                h=side*thickness*math.sqrt(max(0,1-r*r))
                pos=(h,a,b) if plane=='yz' else (a,b,h)
                v.append(add(at,pos))
                d=sub(p,center); tangent=sub(outline[(i+1)%n],outline[(i-1)%n])
                dh=-side*thickness*r/math.sqrt(max(.00001,1-r*r))
                radial=(dh,d[0],d[1]) if plane=='yz' else (d[0],d[1],dh)
                around=(0,tangent[0],tangent[1]) if plane=='yz' else (tangent[0],tangent[1],0)
                normal=cross(radial,around)
                axis=0 if plane=='yz' else 2
                if normal[axis]*side<0: normal=mul(normal,-1)
                normals.append(unit(normal))
        for j in range(rings):
            for i in range(n):
                a=offset+j*n+i; b=offset+j*n+(i+1)%n
                for tri in [(a,b,a+n),(b,b+n,a+n)]:
                    aa,bb,cc=tri
                    normal=cross(sub(v[bb],v[aa]),sub(v[cc],v[aa]))
                    axis=0 if plane=='yz' else 2
                    f.append(tri if normal[axis]*side>=0 else (aa,cc,bb))
    return g.mesh(name,v,f,mat,parent,normals=normals)


def face_patch(g,name,outline,mat,parent,offset=.01):
    center=tuple(sum(p[k] for p in outline)/len(outline) for k in range(2))
    origin=g.world_positions[parent]
    v=[sub(face(*center,offset),origin)]; f=[]; n=len(outline); rings=10
    for j in range(1,rings+1):
        for p in outline: v.append(sub(face(*mix(center,p,j/rings),offset),origin))
    for i in range(n): f.append((0,1+i,1+(i+1)%n))
    for j in range(rings-1):
        for i in range(n):
            a=1+j*n+i; b=1+j*n+(i+1)%n
            f.extend([(a,b,a+n),(b,b+n,a+n)])
    f=[(a,c,b) if cross(sub(v[b],v[a]),sub(v[c],v[a]))[2]<0 else (a,b,c) for a,b,c in f]
    return g.mesh(name,v,f,mat,parent)


def eye_outline(cx,width=1,height=1):
    # Almost straight upper lid; rounded, slightly tapered U-shaped lower lid.
    paths=[((-.18,.17),(-.07,.183),(.07,.183),(.18,.17)),
           ((.18,.17),(.18,-.12),(.15,-.24),(0,-.24)),
           ((0,-.24),(-.15,-.24),(-.18,-.12),(-.18,.17))]
    return [(cx+x*width,-.34+y*height) for path in paths for x,y in curve(*path,24)[:-1]]


def build_geometry(g):
    cream=g.material('Smooth warm rice mochi','F2F0E1',.62)
    gold=g.material('Soft harvest yellow','C8B443',.68)
    white=g.material('Horn gradient carrier','FFFFFF',.60)
    ink=g.material('Charcoal eye embroidery impression','26322D',.70)
    green=g.material('Pale celadon eyes','B9D3B8',.67)
    gray=g.material('Silver checker fill','969AA1',.7)
    red=g.material('Vermilion hanging strip','D94D31',.65)
    clasp=g.material('Small pale gold clasp','D3C68A',.4)
    jade=g.material('Pale jade bead necklace','DEE5BD',.35)
    blue=g.material('Indigo folded tail','3D4077',.65)
    pink=g.material('Happy tongue','DF9788',.65)
    body=g.group('MochiBody')

    # A single closed continuous body mesh, including the white tapered tail root.
    v=[]; normals=[]; faces=[]; rows=160; cols=112
    for row in range(rows+1):
        t=4*row/rows; z,r,cy=profile(t)
        p0=profile(max(0,t-.0001)); p1=profile(min(4,t+.0001))
        dz,dr,dcy=sub(p1,p0)
        for column in range(cols+1):
            phi=2*PI*column/cols; c,s=math.cos(phi),math.sin(phi)
            v.append((r*c,cy+Y_RATIO*r*s,z))
            tangent=(dr*c,dcy+Y_RATIO*dr*s,dz)
            around=(-s,Y_RATIO*c,0)
            normal=unit(cross(tangent,around))
            if row==0: normal=(0,0,1)
            if row==rows: normal=(0,0,-1)
            normals.append(normal)
    for row in range(rows):
        for column in range(cols):
            a=row*(cols+1)+column; b=a+cols+1
            faces.extend([(a,b,a+1),(a+1,b,b+1)])
    g.mesh('Continuous horizontal teardrop body',v,faces,cream,body,normals=normals)

    # Four low feet, with two front feet in front view and two in profile.
    for side in (-1,1):
        for rear in (False,True):
            g.sphere(('Rear' if rear else 'Front')+f' gold foot {side}',
                     (side*(.43 if rear else .51),-.88 if rear else -.90,-.68 if rear else .28),
                     (.20,.165 if rear else .145,.235),gold,body,28,44)

    horn_low=linear_rgb('D4BD46'); horn_high=linear_rgb('34366A')
    for side in (-1,1):
        path=curve((side*.50,.43,.24),(side*.61,.78,.25),(side*.83,1.17,.15),(side*1.06,1.47,.10),56)
        radii=[]; colors=[]
        for i in range(len(path)):
            t=i/(len(path)-1)
            radii.append((.18+.065*math.sin(PI*t))*(1-t)**.77+.002)
            k=max(0,min(1,(t-.35)/.43)); k=k*k*(3-2*k)
            colors.append(list(mix(horn_low,horn_high,k)))
        g.tube(f'Swept golden indigo horn {side}',path,radii,white,body,colors,40)
        # Rounded lateral spur at the gold root, visible in the front photograph.
        spur=curve((side*.55,.64,.22),(side*.75,.62,.22),(side*.94,.67,.21),(side*1.08,.78,.20),36)
        g.tube(f'Soft lateral horn spur {side}',spur,[.16*(1-i/36)**.7+.003 for i in range(37)],gold,body,sides=32,flatten=.72)

    # Three broad triangular dorsal plates run longitudinally along the spine.
    for i,(z,peak) in enumerate([(.00,1.18),(-.57,1.15),(-1.12,.91)]):
        front=z+.24; back=z-.22
        _,rf,cf=profile_at_z(front); _,rb,cb=profile_at_z(back)
        corners=[(cf+Y_RATIO*rf-.07,front),(peak,z-.10),(cb+Y_RATIO*rb-.07,back)]
        pillow(g,f'Longitudinal golden dorsal plate {i+1}',corners,.043,gold,body)

    eyes=g.group('BlinkBothEyes',(0,-.34,0),body)
    for side in (-1,1):
        cx=side*.34
        face_patch(g,f'Fine U-shaped eye border {side}',eye_outline(cx),ink,eyes,.009)
        face_patch(g,f'Celadon half-lidded eye {side}',eye_outline(cx,.91,.935),green,eyes,.016)
        lid=[sub(face(cx+x,-.17+.012*math.cos(x/.21*PI/2),.025),g.world_positions[eyes])
             for x in [-.21+i*.42/32 for i in range(33)]]
        g.tube(f'Calm heavy eyelid {side}',lid,[.018]*len(lid),ink,eyes,sides=12)
        g.sphere(f'Silver forehead spot {side}',face(side*.29,.025,.012),(.043,.034,.013),gray,body,20,32)
    mouth=curve((-.085,-.555),(-.055,-.58),(-.025,-.58),(0,-.553),18)[:-1]
    mouth+=curve((0,-.553),(.025,-.58),(.055,-.58),(.085,-.555),18)
    g.tube('Small calm w smile',[face(x,y,.021) for x,y in mouth],[.011]*len(mouth),ink,body,sides=12)
    tongue=g.group('HappyTongue',face(0,-.56,.021),body)
    g.doc['nodes'][tongue]['scale']=[.001,.001,.001]
    g.sphere('Small happy tongue',(0,-.035,.01),(.038,.061,.020),pink,tongue,20,28)

    # Necklace wraps the entire left horn root, instead of crossing the forehead.
    for i in range(19):
        t=2*PI*i/19
        p=(-.63+.235*math.cos(t),.70-.068*math.sin(t),.24+.205*math.sin(t))
        g.sphere(f'Round jade necklace bead {i+1:02}',p,(.037,.037,.037),jade,body,16,24)
    ring=[(.63+.235*math.cos(t),.70-.068*math.sin(t),.24+.205*math.sin(t))
          for t in [2*PI*i/80 for i in range(81)]]
    g.tube('Thin cord around right horn root',ring,[.010]*len(ring),jade,body,sides=10)
    root=(.91,.51,.49)
    cord=curve((.85,.65,.28),(.86,.58,.33),(.80,.50,root[2]),root,24)
    g.tube('Short fitted pendant cord',cord,[.01]*len(cord),jade,body,sides=10)
    tag=g.group('PendantSway',root,body)
    v=[]; f=[]; rows=32; columns=8
    for row in range(rows+1):
        t=row/rows; y=root[1]-1.01*t
        # A flat strip lies just outside a diagonal tangent plane of the cheek.
        # Its width is visible in both front and side views, as in the photos.
        for column in range(columns+1):
            s=2*column/columns-1
            width=.085*(.96 if row in (0,rows) else 1)
            p=(root[0]+.5*s*width+.015*t,y,root[2]-.866*s*width+.006*math.sin(PI*t))
            v.append(sub(p,root))
    for row in range(rows):
        for column in range(columns):
            a=row*(columns+1)+column; b=a+columns+1
            f.extend([(a,b,a+1),(a+1,b,b+1)])
    g.mesh('Flat diagonal red pendant',v,f,red,tag)
    clasp_node=g.sphere('Small gold pendant clasp',(0,-.005,.008),(.081,.016,.017),clasp,tag,16,24)
    g.doc['nodes'][clasp_node]['rotation']=[0,math.sin(PI/6),0,math.cos(PI/6)]

    # Five touching checker squares on each side, projected onto the real body.
    def tile(name,side,y0,z0,edge):
        v=[]; f=[]; colors=[]; steps=18
        for row in range(steps+1):
            for column in range(steps+1):
                v.append(flank(side,y0+edge*row/steps,z0+edge*column/steps,.009))
                edge_vertex=min(row,column,steps-row,steps-column)==0
                shade='B1B3B7' if edge_vertex else '969AA1'
                colors.append([int(shade[i:i+2],16)/255 for i in (0,2,4)]+[1])
        for row in range(steps):
            for column in range(steps):
                a=row*(steps+1)+column; b=a+steps+1
                tris=[(a,b,a+1),(a+1,b,b+1)]
                f.extend(tris if side>0 else [(aa,cc,bb) for aa,bb,cc in tris])
        return g.mesh(name,v,f,white,body,colors)
    for side in (-1,1):
        for row in range(3):
            for column in range(3):
                if (row+column)%2: continue
                y=-.23+row*.225; z=-.94+column*.225
                tile(f'Checker square with soft border {side} {row} {column}',side,y,z,.214)

    # Two upright overlapping fins are broad in profile and thin in front view.
    # The continuous white body meets their root directly; no sideways tube.
    tail=g.group('TailWag',(0,.10,-2.16),body)
    near=[(-.01,.04),(.59,-.59),(-.15,-.67),(-.32,-.28),(-.14,.03)]
    far=[(-.03,.10),(.42,-.32),(.24,-.58),(-.27,-.50),(-.31,-.13)]
    pillow(g,'Rear indigo folded tail plate',far,.043,blue,tail,at=(.11,.035,.13))
    pillow(g,'Front indigo folded tail plate',near,.045,blue,tail,at=(-.095,0,0))
    return body,eyes,tail,tag,tongue


def linear_rgb(h):
    values=[int(h[i:i+2],16)/255 for i in (0,2,4)]
    return tuple(c/12.92 if c<=.04045 else ((c+.055)/1.055)**2.4 for c in values)
