"""Build a tiny, self-contained example GLB using Python's standard library."""
import json
import math
from pathlib import Path
import struct

root = Path(__file__).resolve().parents[1]
destination = root / 'work' / 'glb-fixture'
destination.mkdir(parents=True, exist_ok=True)
positions, normals, triangles = [], [], []
rows, columns = 20, 32
for row in range(rows + 1):
    theta = math.pi * row / rows
    for column in range(columns + 1):
        phi = 2 * math.pi * column / columns
        x, y, z = math.sin(theta) * math.cos(phi), math.cos(theta), math.sin(theta) * math.sin(phi)
        positions.extend((x, y, z))
        normals.extend((x, y, z))
for row in range(rows):
    for column in range(columns):
        a = row * (columns + 1) + column
        b = a + columns + 1
        triangles.extend((a, a + 1, b, a + 1, b + 1, b))

binary = bytearray()
views = []


def append(values, format_char):
    while len(binary) % 4:
        binary.append(0)
    offset = len(binary)
    binary.extend(struct.pack('<' + format_char * len(values), *values))
    views.append({'buffer': 0, 'byteOffset': offset, 'byteLength': len(binary) - offset})
    return len(views) - 1


pos_view = append(positions, 'f')
normal_view = append(normals, 'f')
index_view = append(triangles, 'H')
time_view = append([0, 1, 2], 'f')
scale_view = append([1, 1, 1, 1.04, .94, 1.04, 1, 1, 1], 'f')
gltf = {
    'asset': {'version': '2.0', 'generator': 'LPP example fixture'},
    'scene': 0, 'scenes': [{'nodes': [0]}],
    'nodes': [{'name': 'ExampleOrb', 'mesh': 0}],
    'materials': [{'pbrMetallicRoughness': {
        'baseColorFactor': [.62, .70, .94, 1], 'metallicFactor': 0, 'roughnessFactor': .75}}],
    'meshes': [{'primitives': [{'attributes': {'POSITION': 0, 'NORMAL': 1}, 'indices': 2, 'material': 0}]}],
    'buffers': [{'byteLength': len(binary)}], 'bufferViews': views,
    'accessors': [
        {'bufferView': pos_view, 'componentType': 5126, 'count': len(positions)//3,
         'type': 'VEC3', 'min': [-1, -1, -1], 'max': [1, 1, 1]},
        {'bufferView': normal_view, 'componentType': 5126, 'count': len(normals)//3, 'type': 'VEC3'},
        {'bufferView': index_view, 'componentType': 5123, 'count': len(triangles), 'type': 'SCALAR'},
        {'bufferView': time_view, 'componentType': 5126, 'count': 3,
         'type': 'SCALAR', 'min': [0], 'max': [2]},
        {'bufferView': scale_view, 'componentType': 5126, 'count': 3, 'type': 'VEC3'}],
    'animations': [{'name': 'Breathe',
        'samplers': [{'input': 3, 'output': 4, 'interpolation': 'LINEAR'}],
        'channels': [{'sampler': 0, 'target': {'node': 0, 'path': 'scale'}}]}]
}
json_bytes = json.dumps(gltf, separators=(',', ':')).encode()
json_bytes += b' ' * ((-len(json_bytes)) % 4)
binary.extend(b'\0' * ((-len(binary)) % 4))
total = 12 + 8 + len(json_bytes) + 8 + len(binary)
payload = struct.pack('<III', 0x46546C67, 2, total)
payload += struct.pack('<II', len(json_bytes), 0x4E4F534A) + json_bytes
payload += struct.pack('<II', len(binary), 0x004E4942) + binary
(destination / 'pet.glb').write_bytes(payload)
(destination / 'skin.json').write_text(json.dumps({
    'schema_version': 1, 'id': 'example.glb', 'name': '蓝莓球 · GLB 动画演示',
    'type': 'model3d', 'asset': 'pet.glb', 'rotation_degrees': [0, 0, 0],
    'animations': {'idle': 'Breathe', 'sleep': 'Breathe'}
}, ensure_ascii=False, indent=2), encoding='utf-8')
print(destination / 'pet.glb')
