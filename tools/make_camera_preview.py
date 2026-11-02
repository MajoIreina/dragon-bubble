"""Compare camera framing on matching, uncropped Godot render canvases."""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


root = Path(__file__).resolve().parents[1]
output = root / "outputs"
baseline = root / "work" / "before-camera-framing"
sheet = Image.new("RGB", (900, 1010), "#edf2ef")
draw = ImageDraw.Draw(sheet)
font = ImageFont.truetype("C:/Windows/Fonts/msyh.ttc", 26)
small = ImageFont.truetype("C:/Windows/Fonts/msyh.ttc", 18)
draw.text((30, 22), "桌宠显示范围调整", font=font, fill="#30483b")
draw.text((30, 64), "相同姿势、相同画布比例 · 完整画面，未单独裁剪放大", font=small, fill="#60786a")
for column, label in enumerate(["调整前", "调整后 · 可见范围 +11%"]):
    draw.text((38 + column * 440, 104), label, font=small, fill="#30483b")
for row, (view, label) in enumerate([("desktop", "桌面默认朝向"), ("side", "侧面 · 背鳍与尾巴")]):
    top = 146 + row * 424
    for column, directory in enumerate([baseline, output]):
        left = 30 + column * 440
        draw.rounded_rectangle((left, top, left + 400, top + 400), radius=16, fill="#f8faf7", outline="#cbd8d0", width=2)
        image = Image.open(directory / f"shu-{view}.png").convert("RGBA")
        image = image.resize((400, 400), Image.Resampling.LANCZOS)
        sheet.paste(image, (left, top), image)
        draw.text((left + 18, top + 16), label, font=small, fill="#60786a")
sheet.save(output / "camera-framing-comparison.png")
print(output / "camera-framing-comparison.png")
