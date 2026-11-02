"""Compare the previous dorsal strip and the sketch revision in Godot."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

root = Path(__file__).resolve().parents[1]
before = root / "source_models/shu-plush/revisions/before-dorsal-sketch-fix/shu-soft-side-after.png"
after = root / "outputs/shu-soft-side-after.png"
images = [Image.open(path).convert("RGBA") for path in (before, after)]
boxes = [image.getbbox() for image in images]
crop = (min(b[0] for b in boxes) - 24, min(b[1] for b in boxes) - 24,
        max(b[2] for b in boxes) + 24, max(b[3] for b in boxes) + 24)
sheet = Image.new("RGB", (1500, 680), "#edf2ed")
draw = ImageDraw.Draw(sheet)
font = ImageFont.truetype("C:/Windows/Fonts/msyh.ttc", 28)
small = ImageFont.truetype("C:/Windows/Fonts/msyh.ttc", 20)
draw.text((40, 26), "黍泡泡 · 背鳍手稿调整", font=font, fill="#30483b")
draw.text((40, 73), "Godot 实际侧视渲染 · 两侧均开启身体软体", font=small, fill="#627768")
for i, (image, title, subtitle) in enumerate(zip(
        images, ["调整前", "按手稿调整后"],
        ["背鳍较短，后峰随身体下沉后不够明显", "连续三峰 · 前缘长斜坡 · 后缘陡降 · 圆滑峰谷"])):
    x = 24 + i * 744
    draw.rounded_rectangle((x, 125, x + 732, 653), radius=20, fill="#f8faf5")
    model = image.crop(crop)
    model.thumbnail((700, 400), Image.Resampling.LANCZOS)
    sheet.paste(model, (x + (732 - model.width) // 2, 147 + (400 - model.height) // 2), model)
    draw.text((x + 26, 566), title, font=font, fill="#30483b")
    draw.text((x + 26, 611), subtitle, font=small, fill="#627768")
path = root / "outputs/shu-dorsal-comparison.png"
sheet.save(path)
print(path)
