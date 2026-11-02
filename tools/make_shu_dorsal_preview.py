"""Compare the previous dorsal strip and the sketch revision in Godot."""
from pathlib import Path
import sys
from PIL import Image, ImageDraw, ImageFont

root = Path(__file__).resolve().parents[1]
lowering = "--lowering" in sys.argv
rear_lowering = "--rear-lowering" in sys.argv
revision = ("before-rear-dorsal-lowering" if rear_lowering else
            "before-dorsal-lowering" if lowering else "before-dorsal-sketch-fix")
before = root / f"source_models/shu-plush/revisions/{revision}/shu-soft-side-after.png"
after = root / "outputs/shu-soft-side-after.png"
images = [Image.open(path).convert("RGBA") for path in (before, after)]
boxes = [image.getbbox() for image in images]
crop = (min(b[0] for b in boxes) - 24, min(b[1] for b in boxes) - 24,
        max(b[2] for b in boxes) + 24, max(b[3] for b in boxes) + 24)
sheet = Image.new("RGB", (1500, 680), "#edf2ed")
draw = ImageDraw.Draw(sheet)
font = ImageFont.truetype("C:/Windows/Fonts/msyh.ttc", 28)
small = ImageFont.truetype("C:/Windows/Fonts/msyh.ttc", 20)
heading = ("黍泡泡 · 尾侧背鳍高度微调" if rear_lowering else
           "黍泡泡 · 背鳍高度微调" if lowering else "黍泡泡 · 背鳍手稿调整")
draw.text((40, 26), heading, font=font, fill="#30483b")
draw.text((40, 73), "Godot 实际侧视渲染 · 两侧均开启身体软体", font=small, fill="#627768")
titles = ["降低前", "三峰适度降低后"] if lowering else ["调整前", "按手稿调整后"]
subtitles = (["上一版连续三峰轮廓", "相对背部高度降低 15% · 保留三峰与圆滑衔接"] if lowering else
             ["背鳍较短，后峰随身体下沉后不够明显", "连续三峰 · 前缘长斜坡 · 后缘陡降 · 圆滑峰谷"])
if rear_lowering:
    titles = ["本轮调整前", "尾侧一峰再降低 15%"]
    subtitles = ["上一版三峰高度", "尾侧峰相对背部高度再降 15% · 峰谷平滑衔接"]
for i, (image, title, subtitle) in enumerate(zip(images, titles, subtitles)):
    x = 24 + i * 744
    draw.rounded_rectangle((x, 125, x + 732, 653), radius=20, fill="#f8faf5")
    model = image.crop(crop)
    model.thumbnail((700, 400), Image.Resampling.LANCZOS)
    sheet.paste(model, (x + (732 - model.width) // 2, 147 + (400 - model.height) // 2), model)
    draw.text((x + 26, 566), title, font=font, fill="#30483b")
    draw.text((x + 26, 611), subtitle, font=small, fill="#627768")
path = root / ("outputs/shu-rear-dorsal-lowering.png" if rear_lowering else
               "outputs/shu-dorsal-lowering.png" if lowering else "outputs/shu-dorsal-comparison.png")
sheet.save(path)
print(path)
