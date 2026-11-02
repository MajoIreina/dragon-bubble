"""Assemble the four correction review renders without altering their content."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ROOT=Path(__file__).resolve().parents[1]
tile=640;label=56;gap=20
canvas=Image.new('RGB',(tile*2+gap*3,(tile+label)*2+gap*3),(242,243,239))
draw=ImageDraw.Draw(canvas)
font=ImageFont.truetype(r'C:\Windows\Fonts\msyh.ttc',27)
for index,(name,title) in enumerate([
    ('side','整体侧面 · 脚与尾部比例'),
    ('feet','四脚 · 圆底、短柱侧壁与缝边'),
    ('tail','尾巴 · 双尖轮廓与内折布褶'),
    ('cord-front','正视图右侧 · 角挂饰间隙'),
]):
    x=gap+(index%2)*(tile+gap);y=gap+(index//2)*(tile+label+gap)
    photo=Image.open(ROOT/'outputs'/f'shu-corrected-{name}.png').convert('RGB').resize((tile,tile),Image.Resampling.LANCZOS)
    canvas.paste(photo,(x,y))
    draw.text((x+12,y+tile+10),title,font=font,fill=(39,45,41))
path=ROOT/'outputs/shu-corrections-preview.png'
canvas.save(path)
print(path)
