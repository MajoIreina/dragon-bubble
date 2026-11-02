"""Deterministic, seamless short-pile cloth textures; no external image API."""
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

OUT = Path(__file__).resolve().parents[1] / 'source_models/shu-plush/textures'
OUT.mkdir(parents=True, exist_ok=True)
rng = np.random.default_rng(12419)
n = 512
h = Image.new('L', (n, n), 128)
d = ImageDraw.Draw(h)
for _ in range(38000):
    x, y = rng.integers(0, n, 2)
    dx, dy = rng.normal(0, 1.8), rng.uniform(1.5, 5.5)
    value = int(rng.integers(80, 190))
    for ox in (-n, 0, n):
        for oy in (-n, 0, n):
            d.line((x+ox, y+oy, x+ox+dx, y+oy+dy), fill=value)
height = np.asarray(h.filter(ImageFilter.GaussianBlur(.45)), dtype=float)/255
dx = (np.roll(height,-1,1)-np.roll(height,1,1))*5.0
dy = (np.roll(height,-1,0)-np.roll(height,1,0))*5.0
normal = np.stack((-dx, dy, np.ones_like(dx)), axis=2)
normal /= np.linalg.norm(normal, axis=2, keepdims=True)
Image.fromarray(np.uint8(np.clip(normal*.5+.5,0,1)*255)).save(OUT/'short-pile-normal.png')
colors = {'cream':'F2F0E4','gold':'D6BE55','blue':'39336D','sage':'C2CFAB',
          'gray':'9A9B9A','red':'DE542A','thread':'DDD9C7'}
for name, color in colors.items():
    base=np.array([int(color[i:i+2],16) for i in (0,2,4)])
    cloth=base[None,None,:]*(.96+(height-.5)[...,None]*.38)
    Image.fromarray(np.uint8(np.clip(cloth,0,255))).save(OUT/f'{name}-cloth.png')
print(OUT)
