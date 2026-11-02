"""Compose matching Godot side renders for reviewing the supported body shape."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

root = Path(__file__).resolve().parents[1] / "outputs"
images = [Image.open(root / f"shu-soft-side-{state}.png").convert("RGBA")
          for state in ("before", "after")]
boxes = [image.getbbox() for image in images]
crop = (min(b[0] for b in boxes) - 24, min(b[1] for b in boxes) - 24,
        max(b[2] for b in boxes) + 24, max(b[3] for b in boxes) + 24)
sheet = Image.new("RGB", (1500, 680), "#edf2ed")
draw = ImageDraw.Draw(sheet)
font = ImageFont.truetype("C:/Windows/Fonts/msyh.ttc", 28)
small = ImageFont.truetype("C:/Windows/Fonts/msyh.ttc", 20)
draw.text((40, 26), "黍泡泡 · 身体软体效果", font=font, fill="#30483b")
draw.text((40, 73), "Godot 实际侧视渲染 · 相同镜头、姿态和比例", font=small, fill="#627768")
for i, (image, title, subtitle) in enumerate(zip(
        images, ["关闭软体", "开启软体"],
        ["原始后腹与尾根向上收尖", "后半身下沉压缩 · 下腹承托 · 尾根随身体沉低"])):
    x = 24 + i * 744
    draw.rounded_rectangle((x, 125, x + 732, 653), radius=20, fill="#f8faf5")
    model = image.crop(crop)
    model.thumbnail((700, 400), Image.Resampling.LANCZOS)
    sheet.paste(model, (x + (732 - model.width) // 2, 147 + (400 - model.height) // 2), model)
    draw.text((x + 26, 566), title, font=font, fill="#30483b")
    draw.text((x + 26, 611), subtitle, font=small, fill="#627768")
sheet.save(root / "shu-soft-comparison.png")
print(root / "shu-soft-comparison.png")
