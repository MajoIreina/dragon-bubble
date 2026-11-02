"""Combine actual Godot renders into a labeled turnaround sheet (Pillow)."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

root = Path(__file__).resolve().parents[1] / 'outputs'
sheet = Image.new('RGB', (1500, 650), '#edf2ed')
draw = ImageDraw.Draw(sheet)
font = ImageFont.truetype('C:/Windows/Fonts/msyh.ttc', 26)
small = ImageFont.truetype('C:/Windows/Fonts/msyh.ttc', 18)
draw.text((48, 24), '黍泡泡', font=font, fill='#30483b')
draw.text((48, 66), '18 张实物照片重建 · Godot 实际渲染', font=small, fill='#6f8174')
views=[('front', '正面'), ('three-quarter', '半侧面'), ('side', '侧面')]
images=[Image.open(root/f'shu-{view}.png').convert('RGBA') for view,_ in views]
boxes=[im.getbbox() for im in images]
crop=(min(b[0] for b in boxes)-24,min(b[1] for b in boxes)-24,
      max(b[2] for b in boxes)+24,max(b[3] for b in boxes)+24)
for i, ((view, label), model) in enumerate(zip(views,images)):
    x = 18+i*494
    draw.rounded_rectangle((x, 110, x+476, 625), radius=22, fill='#f8faf5')
    model=model.crop(crop)
    model.thumbnail((455,435),Image.Resampling.LANCZOS)
    sheet.paste(model,(x+(476-model.width)//2,125+(435-model.height)//2),model)
    draw.text((x+212, 584), label, font=small, fill='#536d5b')
sheet.save(root/'shu-turnaround.png')
print(root/'shu-turnaround.png')
