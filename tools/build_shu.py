"""Build LPP's smooth Shu bubble interpretation as a portable animated GLB.

Legacy approximation only. Writes to source_models/legacy-shu, never the active skin.
Coordinates: +Y up, +Z face. All geometry and materials are embedded in the GLB.
Reference: Arknights Shu dragon bubble product photos, see docs/shu-model.md.
"""
import json
import math
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'source_models' / 'legacy-shu'
PI = math.pi


def add(a, b): return tuple(x+y for x, y in zip(a, b))
def sub(a, b): return tuple(x-y for x, y in zip(a, b))
def mul(a, s): return tuple(x*s for x in a)
def dot(a, b): return sum(x*y for x, y in zip(a, b))
def cross(a, b): return (a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0])
def unit(a): return mul(a, 1/max(1e-9, math.sqrt(dot(a, a))))
def mix(a, b, t): return tuple(x*(1-t)+y*t for x, y in zip(a, b))


def rgb(h):
    s = [int(h[i:i+2], 16)/255 for i in (0, 2, 4)]
    return [c/12.92 if c <= .04045 else ((c+.055)/1.055)**2.4 for c in s]


class GLB:
    def __init__(self):
        self.binary = bytearray()
        self.world_positions = [(0,0,0)]
        self.doc = {'asset': {'version': '2.0', 'generator': 'LPP Shu v3 product reference reconstruction'},
                    'scene': 0, 'scenes': [{'nodes': [0]}],
                    'nodes': [{'name': 'ShuBubble', 'translation': list(self.world_positions[0]), 'children': []}],
                    'materials': [], 'meshes': [], 'bufferViews': [], 'accessors': [], 'animations': []}

    def data(self, values, kind, component=5126):
        size = {'SCALAR': 1, 'VEC3': 3, 'VEC4': 4}[kind]
        flat = values if size == 1 else [c for row in values for c in row]
        self.binary.extend(b'\0' * (-len(self.binary) % 4))
        offset = len(self.binary)
        self.binary.extend(struct.pack('<'+('f' if component == 5126 else 'I')*len(flat), *flat))
        view = len(self.doc['bufferViews'])
        self.doc['bufferViews'].append({'buffer': 0, 'byteOffset': offset, 'byteLength': len(self.binary)-offset})
        result = {'bufferView': view, 'componentType': component, 'count': len(flat)//size, 'type': kind}
        if kind == 'VEC3':
            result.update(min=[min(row[i] for row in values) for i in range(3)],
                          max=[max(row[i] for row in values) for i in range(3)])
        if kind == 'SCALAR': result.update(min=[min(values)], max=[max(values)])
        self.doc['accessors'].append(result)
        return len(self.doc['accessors'])-1

    def material(self, name, color, rough=.5):
        self.doc['materials'].append({'name': name, 'doubleSided': True,
                                     'pbrMetallicRoughness': {'baseColorFactor': rgb(color)+[1],
                                                            'metallicFactor': 0, 'roughnessFactor': rough}})
        return len(self.doc['materials'])-1

    def group(self, name, at=(0, 0, 0), parent=0):
        node = len(self.doc['nodes'])
        position=add(self.world_positions[parent],at)
        self.world_positions.append(position)
        self.doc['nodes'].append({'name': name, 'translation': list(sub(position,self.world_positions[parent])), 'children': []})
        self.doc['nodes'][parent]['children'].append(node)
        return node

    def mesh(self, name, vertices, faces, mat, parent=0, colors=None, normals=None):
        if normals is None:
            normals = [(0., 0., 0.) for _ in vertices]
            for a, b, c in faces:
                normal = cross(sub(vertices[b], vertices[a]), sub(vertices[c], vertices[a]))
                for i in (a, b, c): normals[i] = add(normals[i], normal)
            normals = [unit(n) for n in normals]
        attrs = {'POSITION': self.data(vertices, 'VEC3'), 'NORMAL': self.data(normals, 'VEC3')}
        if colors: attrs['COLOR_0'] = self.data(colors, 'VEC4')
        primitive = {'attributes': attrs, 'indices': self.data([i for f in faces for i in f], 'SCALAR', 5125), 'material': mat}
        index = len(self.doc['meshes'])
        self.doc['meshes'].append({'name': name, 'primitives': [primitive]})
        node = self.group(name, parent=parent)
        self.doc['nodes'][node]['mesh'] = index
        return node

    def sphere(self, name, at, scale, mat, parent=0, rows=24, cols=40):
        v, n, f = [], [], []
        for j in range(rows+1):
            t = PI*j/rows
            for i in range(cols+1):
                p = 2*PI*i/cols
                q = (math.sin(t)*math.cos(p), math.cos(t), math.sin(t)*math.sin(p))
                v.append(add(at, tuple(q[k]*scale[k] for k in range(3))))
                n.append(unit(tuple(q[k]/scale[k] for k in range(3))))
        for j in range(rows):
            for i in range(cols):
                a = j*(cols+1)+i; b = a+cols+1
                f.extend([(a, a+1, b), (a+1, b+1, b)])
        return self.mesh(name, v, f, mat, parent, normals=n)

    def tube(self, name, path, radii, mat, parent=0, colors=None, sides=20, flatten=1):
        v, f, cs = [], [], []
        for j, point in enumerate(path):
            tangent = unit(sub(path[min(len(path)-1, j+1)], path[max(0, j-1)]))
            axis = unit(cross(tangent, (0, 0, 1) if abs(tangent[2]) < .9 else (0, 1, 0)))
            axis2 = unit(cross(tangent, axis))
            for i in range(sides):
                t = 2*PI*i/sides
                radial = add(mul(axis, math.cos(t)*radii[j]), mul(axis2, math.sin(t)*radii[j]*flatten))
                v.append(add(point, radial))
                if colors: cs.append(colors[j]+[1])
        for j in range(len(path)-1):
            for i in range(sides):
                a = j*sides+i; b = j*sides+(i+1)%sides
                f.extend([(a, b, a+sides), (b, b+sides, a+sides)])
        # Sealed ends; tip radii remain tiny to avoid degenerate normal artifacts.
        for j, reverse in [(0, True), (len(path)-1, False)]:
            c = len(v); v.append(path[j])
            if colors: cs.append(colors[j]+[1])
            for i in range(sides):
                a=j*sides+i; b=j*sides+(i+1)%sides
                f.append((c,b,a) if reverse else (c,a,b))
        return self.mesh(name, v, f, mat, parent, cs or None)

    def animation(self, name, duration, tracks):
        anim = {'name': name, 'samplers': [], 'channels': []}
        for node, path, keys in tracks:
            times, values = zip(*keys)
            inp = self.data(times, 'SCALAR')
            out = self.data(values, 'VEC4' if path == 'rotation' else 'VEC3')
            idx = len(anim['samplers'])
            anim['samplers'].append({'input': inp, 'output': out, 'interpolation': 'LINEAR'})
            anim['channels'].append({'sampler': idx, 'target': {'node': node, 'path': path}})
        self.doc['animations'].append(anim)

    def save(self, path):
        self.binary.extend(b'\0' * (-len(self.binary) % 4))
        self.doc['buffers'] = [{'byteLength': len(self.binary)}]
        js = json.dumps(self.doc, separators=(',', ':')).encode()
        js += b' ' * (-len(js) % 4)
        total = 12+8+len(js)+8+len(self.binary)
        path.write_bytes(struct.pack('<III', 0x46546c67, 2, total)+
                         struct.pack('<II', len(js), 0x4e4f534a)+js+
                         struct.pack('<II', len(self.binary), 0x004e4942)+self.binary)


from shu_geometry import build_geometry

g = GLB()
body, eyes, tail, tag, tongue = build_geometry(g)


def quat(axis,angle): return tuple(v*math.sin(angle/2) for v in axis)+(math.cos(angle/2),)


# Every clip explicitly resets each animated channel, so switching states cannot
# leave a closed eye or a tilted ribbon behind. The desktop supplies body squash.
for name,duration in [('Idle',4.8),('Walk',1.2),('Sleep',4.8),('Happy',1.5),('PickedUp',1.8),('Land',.8)]:
    samples=33
    ts=[duration*i/(samples-1) for i in range(samples)]
    tail_keys=[]; tag_keys=[]; body_keys=[]; eye_keys=[]; tongue_keys=[]
    for t in ts:
        phase=t/duration*2*PI
        wag=math.sin(phase)*( .22 if name in ('Walk','Happy') else .07)
        tail_keys.append((t,quat((0,1,0),wag)))
        tag_keys.append((t,quat((0,0,1),math.sin(phase)*(.08 if name=='Walk' else .025))))
        breath=math.sin(phase)*(.009 if name=='Sleep' else .005)
        body_keys.append((t,(1+breath,1-breath,1+breath)))
        # A quick double-key blink late in the idle cycle; sleeping stays closed.
        blink=max(.06,1-max(0,1-abs(t/duration-.75)/.045)*.94) if name=='Idle' else 1
        if name=='Sleep': blink=.07
        if name=='Happy': blink=.55+.45*abs(math.cos(phase))
        eye_keys.append((t,(1,blink,1)))
        tongue_keys.append((t,(1,1,1) if name=='Happy' else (.001,.001,.001)))
    g.animation(name,duration,[(tail,'rotation',tail_keys),(tag,'rotation',tag_keys),
                               (body,'scale',body_keys),(eyes,'scale',eye_keys),(tongue,'scale',tongue_keys)])

OUT.mkdir(parents=True,exist_ok=True)
g.save(OUT/'shu.glb')
(OUT/'skin.json').write_text(json.dumps({
    'schema_version':1,'id':'builtin.shu','name':'黍泡泡 · 软糯龙团',
    'type':'model3d','asset':'shu.glb','rotation_degrees':[0,0,0],
    'animations':{'idle':'Idle','walk':'Walk','sleep':'Sleep','petted':'Happy',
                  'dragged':'PickedUp','airborne':'PickedUp','fall':'Land'}
},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(f"Built {OUT/'shu.glb'} ({(OUT/'shu.glb').stat().st_size:,} bytes, {len(g.doc['meshes'])} meshes, 6 clips)")
