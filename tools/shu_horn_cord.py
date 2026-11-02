"""Right horn cord, clamp and ribbon attachment, fitted to the source meshes."""
import json
import math
from mathutils import Vector, Quaternion
from shu_bracelet import fit_base_loop, _resample, _signed, _project


def script(point): return (point.x, point.z, -point.y)
def native(point): return Vector((point[0], -point[2], point[1]))


def cubic(a,b,c,d,t):
    return a*(1-t)**3+b*(3*(1-t)**2*t)+c*(3*(1-t)*t*t)+d*t**3


def fit_horn_cord(horn, head, main_path, report_path):
    dense, colliders = fit_base_loop(horn, head, main_path, .0085)
    loop, length = _resample(dense,720)
    desired = native((.84,.49,.43))
    middle = min(range(len(loop)),key=lambda i:(loop[i]-desired).length)
    half = round(.225*.5/length*len(loop))
    a_index,b_index=(middle-half)%len(loop),(middle+half)%len(loop)
    a,b=loop[a_index],loop[b_index]
    tangent=(b-a).normalized()
    gravity=native((0,-1,0))
    down=(gravity-tangent*gravity.dot(tangent)).normalized()
    outward=down.cross(tangent).normalized()
    midpoint=(a+b)*.5
    if outward.dot(_signed(colliders[1][0],midpoint)[1])<0:
        tangent=-tangent
        outward=-outward
    width,height,depth=.244,.030,.022
    center=midpoint+down*.019+outward*.020
    # Move the complete small metal clamp out of both shells. The corners,
    # edges and face centers are included, rather than only its midpoint.
    for _ in range(16):
        displacement=0
        for x in (-.5,-.25,0,.25,.5):
            for y in (-.5,0,.5):
                for z in (-.5,0,.5):
                    # Include the lower teeth, rolled eyes and pile envelope.
                    point=center+tangent*(x*width)+down*(y*.050+.005)+outward*(z*.030)
                    for tree,margin in colliders:
                        distance,normal=_signed(tree,point)
                        required=margin-.003
                        displacement=max(displacement,(required-distance)/max(.25,outward.dot(normal)))
        if displacement<=0:break
        center+=outward*(displacement+.0001)
    # Each cord end passes through a rolled metal eye at the corresponding end.
    eye_centers=[center+tangent*(side*.112)-down*.008+outward*.003 for side in (-1,1)]
    eye_a=min(eye_centers,key=lambda p:(p-a).length)
    eye_b=max(eye_centers,key=lambda p:(p-a).length)
    # The long arc wraps the entire L-shaped base; the clamp closes its front.
    arc=[];i=b_index
    while True:
        arc.append(loop[i]);i=(i+1)%len(loop)
        if i==(a_index+1)%len(loop):break
    start=eye_b+outward*.010
    end=eye_a+outward*.010
    start_curve=[cubic(start,eye_b-outward*.022,arc[0]-outward*.010,arc[0],i/28) for i in range(29)]
    end_curve=[cubic(arc[-1],arc[-1]-outward*.010,eye_a-outward*.022,end,i/28) for i in range(29)]
    path=[_project(p,colliders) for p in start_curve[:-1]+arc+end_curve[1:]]
    # An open curve has no closing segment: the silver clamp bridges the ends.
    arc_lengths=[0.0]
    for first,second in zip(path,path[1:]):arc_lengths.append(arc_lengths[-1]+(second-first).length)
    import bisect
    evenly=[]
    for i in range(901):
        at=arc_lengths[-1]*i/900
        index=min(len(path)-2,max(0,bisect.bisect_right(arc_lengths,at)-1))
        amount=(at-arc_lengths[index])/max(1e-9,arc_lengths[index+1]-arc_lengths[index])
        evenly.append(_project(path[index].lerp(path[index+1],amount),colliders))
    gaps=[min(_signed(tree,p)[0]-.0068 for p in evenly) for tree,_ in colliders]
    if min(gaps)<.008:raise RuntimeError('Right cord intersects a shell')
    hinge=center+down*.008+outward*.002
    result={'path':evenly,'center':center,'hinge':hinge,'tangent':tangent,
            'down':down,'outward':outward,'eyes':eye_centers,
            'width':width,'height':height,'depth':depth,'colliders':colliders,
            'report_path':report_path}
    report={'minimum_horn_gap':gaps[0],'minimum_body_gap':gaps[1],
            'rope_diameter':.0136,'clamp_center':script(center),
            'eye_centers':[script(p) for p in eye_centers],
            'cord_endpoints':[script(evenly[0]),script(evenly[-1])],
            'endpoint_to_eye_distances':[(evenly[0]-eye_b).length,(evenly[-1]-eye_a).length]}
    report_path.write_text(json.dumps(report,indent=2),encoding='utf-8')
    print('RIGHT HORN CORD FIT',json.dumps(report),flush=True)
    return result


def fit_swept_ribbon(fit):
    """Keep the whole ribbon clear throughout its animated +/-0.075 rad swing.

    Fitting only the rest pose lets the lower end sweep inside the cheek.
    Solve the row's standoff against both head and forked horn for all widths
    and swing angles, including the fabric's thickness and a sampling margin.
    """
    hinge,tangent,down,outward=(fit[key] for key in ('hinge','tangent','down','outward'))
    colliders=[(fit['colliders'][0][0],.021),(fit['colliders'][1][0],.034)]
    rotations=[Quaternion(tangent,-.082+.164*i/16) for i in range(17)]
    def base(t,s):
        point=hinge+tangent*(s*.118)+down*(.98*t)+outward*(.018*math.sin(t*math.pi)+.012*t)
        return _project(point,colliders)
    offsets=[];rows=96
    for row in range(rows+1):
        t=row/rows;offset=0
        points=[base(t,-1+2*i/12) for i in range(13)]
        for iteration in range(20):
            correction=0
            for rotation in rotations:
                direction=rotation@outward
                for point in points:
                    swung=hinge+rotation@(point+outward*offset-hinge)
                    for tree,margin in colliders:
                        distance,normal=_signed(tree,swung)
                        correction=max(correction,(margin-distance)/max(.22,direction.dot(normal)))
            if correction<.00001:break
            offset+=correction+.0002
        else:raise RuntimeError('Ribbon swing envelope did not converge')
        offsets.append(offset)
    def surface(t,s):
        position=max(0,min(rows,t*rows));index=min(rows-1,int(position));fraction=position-index
        offset=offsets[index]*(1-fraction)+offsets[index+1]*fraction
        return base(t,s)+outward*offset
    report_path=fit['report_path']
    report=json.loads(report_path.read_text(encoding='utf-8'))
    report['ribbon_swing_envelope_radians']=[-.082,.082]
    report['ribbon_maximum_additional_standoff']=max(offsets)
    report_path.write_text(json.dumps(report,indent=2),encoding='utf-8')
    print('RIBBON SWEPT FIT',max(offsets),flush=True)
    return surface
