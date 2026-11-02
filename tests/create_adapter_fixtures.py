"""Generate small real format fixtures and GIF oracle frames (Pillow)."""
import base64
import json
import struct
from pathlib import Path
from PIL import Image, ImageDraw

root = Path(__file__).resolve().parents[1]
out = root / 'work' / 'adapter-fixtures'
out.mkdir(parents=True, exist_ok=True)
frames = []
for i, color in enumerate(['#e79346', '#66b585', '#a68ec4', '#639ec7']):
    image = Image.new('RGBA', (80, 80))
    draw = ImageDraw.Draw(image)
    draw.ellipse((14+i*2, 16, 60+i*2, 70-i*2), fill=color)
    draw.ellipse((30, 35, 34, 42), fill='#273b36')
    draw.ellipse((48, 35, 52, 42), fill='#273b36')
    frames.append(image)
    image.save(out / f'frame-{i}.png')
for ext in ['png', 'webp', 'bmp', 'tga']:
    frames[0].save(out / f'image.{ext}')
frames[0].convert('RGB').save(out / 'image.jpg')
frames[0].convert('RGB').save(out / 'image.jpeg')
(out / 'image.svg').write_text('<svg xmlns="http://www.w3.org/2000/svg" width="80" height="80"><ellipse cx="40" cy="40" rx="25" ry="30" fill="#77b392"/></svg>')
sheet = Image.new('RGBA', (320, 80))
for i, frame in enumerate(frames):
    sheet.paste(frame, (80*i, 0))
sheet.save(out / 'sheet.png')
frames[0].save(out / 'image.gif', save_all=True, append_images=frames[1:],
               duration=[80, 120, 160, 200], disposal=[1, 2, 3, 1], loop=0, optimize=True)
gif = Image.open(out / 'image.gif')
for i in range(gif.n_frames):
    gif.seek(i)
    gif.convert('RGBA').save(out / f'gif-expected-{i}.png')
# Exercise code-size transitions and larger dictionaries with a noise image.
noise = Image.effect_noise((100, 100), 80).convert('P')
noise.save(out / 'noise.gif')
Image.open(out / 'noise.gif').convert('RGBA').save(out / 'noise-expected.png')
(out / 'bad.gif').write_bytes(b'GIF89a\x50\x00\x50\x00\x80\x00\x00')
manifest = {'schema_version': 1, 'id': 'test.adapter.sheet', 'name': 'Sheet', 'type': 'sprite', 'asset': 'sheet.png',
            'grid': [4, 1], 'clips': {'Idle': {'frames': [0, 1], 'fps': 5}, 'Walk': {'frames': [1, 2, 3], 'fps': 10},
                                   'Happy': {'frames': [0, 3], 'fps': 4, 'loop': False}}}
(out / 'sheet.json').write_text(json.dumps(manifest))
sequence = {**manifest, 'id': 'test.adapter.sequence', 'asset': 'frame-0.png', 'frames': [f'frame-{i}.png' for i in range(4)]}
sequence.pop('grid')
(out / 'sequence.json').write_text(json.dumps(sequence))
(out / 'textured.mtl').write_text('newmtl cloth\nKd 0.7 0.9 0.8\nmap_Kd image.png\n')
(out / 'pet.obj').write_text('mtllib textured.mtl\nv -1 0 0\nv 1 0 0\nv 1 2 0\nv 0 1.5 0\nv -1 2 0\n'
                             'vt 0 0\nvt 1 0\nvt 1 1\nvt 0.5 0.75\nvt 0 1\nusemtl cloth\nf -5/1 -4/2 -3/3 -2/4 -1/5\n')
(out / 'bad.obj').write_text('v 0 0 0\nf 1 2 99\n')
payload = (root / 'skins/glb-demo/pet.glb').read_bytes()
length = struct.unpack_from('<I', payload, 12)[0]
gltf = json.loads(payload[20:20+length])
binary = payload[28+length:]
gltf['buffers'][0]['uri'] = 'mesh.bin'
(out / 'mesh.bin').write_bytes(binary)
gltf['images'] = [{'uri': 'color%20map.png'}]
gltf['textures'] = [{'source': 0}]
gltf['materials'][0]['pbrMetallicRoughness']['baseColorTexture'] = {'index': 0}
frames[0].save(out / 'color map.png')
(out / 'external.gltf').write_text(json.dumps(gltf))
gltf['buffers'][0]['uri'] = 'data:application/octet-stream;base64,' + base64.b64encode(binary).decode()
gltf['images'][0]['uri'] = '../outside.png'
(out / 'unsafe.gltf').write_text(json.dumps(gltf))
(out / 'pet.vrm').write_bytes(payload)
# Seven named clips, including root translation, for matching and motion locking.
gltf = json.loads(payload[20:20+length])
start = len(binary)
binary += struct.pack('<9f', 0, 0, 0, 12, 0, 7, 24, 0, 14)
gltf['bufferViews'].append({'buffer': 0, 'byteOffset': start, 'byteLength': 36})
gltf['accessors'].append({'bufferView': len(gltf['bufferViews'])-1, 'componentType': 5126, 'count': 3, 'type': 'VEC3'})
gltf['buffers'][0]['byteLength'] = len(binary)
names = ['Idle', 'mixamo.com|Walking', 'Sleeping', 'Happy', 'PickedUp', 'Falling', 'Landing']
gltf['animations'] = []
for name in names:
    gltf['animations'].append({'name': name, 'samplers': [{'input': 3, 'output': 4}, {'input': 3, 'output': len(gltf['accessors'])-1}],
                               'channels': [{'sampler': 0, 'target': {'node': 0, 'path': 'scale'}},
                                            {'sampler': 1, 'target': {'node': 0, 'path': 'translation'}}]})
data = json.dumps(gltf).encode()
data += b' ' * (-len(data) % 4)
binary += b'\0' * (-len(binary) % 4)
(out / 'actions.glb').write_bytes(struct.pack('<III', 0x46546c67, 2, 28+len(data)+len(binary)) +
                                 struct.pack('<II', len(data), 0x4e4f534a) + data +
                                 struct.pack('<II', len(binary), 0x004e4942) + binary)
print(out)
