from pathlib import Path
import hashlib
import json
import struct

root = Path(__file__).resolve().parents[1]

def read(path):
    data = path.read_bytes()
    size = struct.unpack_from('<I', data, 12)[0]
    return json.loads(data[20:20 + size]), data[28 + size:]

def accessor(document, binary, index):
    item = document['accessors'][index]
    view = document['bufferViews'][item['bufferView']]
    width = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4, 'MAT4': 16}[item['type']]
    width *= {5120: 1, 5121: 1, 5122: 2, 5123: 2, 5125: 4, 5126: 4}[item['componentType']]
    stride = view.get('byteStride', width)
    start = view.get('byteOffset', 0) + item.get('byteOffset', 0)
    data = b''.join(binary[start + i*stride:start + i*stride + width] for i in range(item['count']))
    return hashlib.sha256(data).hexdigest(), item['count'], item['type'], item['componentType']

def summarize(path):
    doc, binary = read(path)
    meshes = {}
    for mesh in doc['meshes']:
        primitives = []
        for primitive in mesh['primitives']:
            primitives.append(([(k, accessor(doc, binary, v)) for k,v in sorted(primitive['attributes'].items())],
                               accessor(doc, binary, primitive['indices']),
                               doc['materials'][primitive['material']]['name']))
        meshes[mesh['name']] = primitives
    nodes = {i: node['name'] for i,node in enumerate(doc['nodes'])}
    animations = {}
    for animation in doc['animations']:
        channels = []
        for channel in animation['channels']:
            sampler = animation['samplers'][channel['sampler']]
            channels.append((nodes[channel['target']['node']], channel['target']['path'],
                             accessor(doc, binary, sampler['input']), accessor(doc, binary, sampler['output'])))
        animations[animation['name']] = sorted(channels)
    return meshes, animations

before, before_clips = summarize(root/'source_models/shu-plush/revisions/before-rear-dorsal-lowering/shu.glb')
after, after_clips = summarize(root/'skins/shu/shu.glb')
assert set(before) == set(after)
changed = sorted(name for name in before if before[name] != after[name])
assert changed == ['ContinuousDorsalFin', 'ContinuousDorsalFin.MicroPile', 'DorsalRidgeSeam'], changed
assert before_clips == after_clips
report = {'changed_meshes': changed, 'unchanged_meshes': len(before)-len(changed),
          'unchanged_clips': sorted(after_clips)}
(root/'work/shu-dorsal-glb-validation.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
print(json.dumps(report))
