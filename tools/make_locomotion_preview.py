"""Compose fixed-camera Godot frames without recentering individual poses."""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
FRAMES = ROOT / "work/locomotion-frames"
OUTPUT = ROOT / "outputs"
FONT = "C:/Windows/Fonts/msyh.ttc"
TITLE = ImageFont.truetype(FONT, 25)
LABEL = ImageFont.truetype(FONT, 18)
BACKGROUND = (239, 243, 247)


def panel(state, direction, frame, size=(416, 268)):
    image = Image.open(FRAMES / f"{state}-{direction}-{frame:02d}.png").convert("RGBA")
    # One shared crop and scale preserve the floor, height and foot lift.
    return image.crop((60, 150, 580, 485)).resize(size, Image.Resampling.LANCZOS)


def canvas(state, frame):
    image = Image.new("RGB", (896, 370), BACKGROUND)
    draw = ImageDraw.Draw(image)
    title = "走路 · 四脚交替支撑" if state == "walk" else "猫科奔跑 · 蹬伸 / 展体 / 收腹"
    draw.text((28, 16), title, font=TITLE, fill=(34, 53, 66))
    for x, direction, label in [(22, "left", "向左"), (458, "right", "向右")]:
        draw.rounded_rectangle((x, 63, x + 416, 350), radius=16, fill="white")
        draw.text((x + 18, 75), label, font=LABEL, fill=(93, 110, 123))
        # Screen y=447 is the same neutral sole plane for every render.
        draw.line((x + 18, 100 + round((447 - 150) * 0.8),
                   x + 398, 100 + round((447 - 150) * 0.8)), fill=(211, 220, 225), width=2)
        image.paste(panel(state, direction, frame), (x, 100), panel(state, direction, frame))
    return image


for state, count in [("walk", 29), ("run", 16)]:
    frames = [canvas(state, i) for i in range(count)]
    # GIF uses 10 ms ticks; distribute rounding rather than changing gait speed.
    durations = [round((i + 1) * 100 / 24) * 10 - round(i * 100 / 24) * 10
                 for i in range(count)]
    frames[0].save(OUTPUT / f"shu-{state}-preview.gif", save_all=True,
                   append_images=frames[1:], duration=durations, loop=0,
                   disposal=2, optimize=True)

sheet = Image.new("RGB", (1280, 1030), BACKGROUND)
draw = ImageDraw.Draw(sheet)
draw.text((24, 15), "完整侧身 · 走路与跑步逐帧预览", font=TITLE, fill=(34, 53, 66))
for row, (state, direction) in enumerate([
        ("walk", "left"), ("walk", "right"), ("run", "left"), ("run", "right")]):
    samples = [0, 7, 14, 21] if state == "walk" else [1, 5, 9, 14]
    label = ("走路" if state == "walk" else "跑步") + (" · 向左" if direction == "left" else " · 向右")
    y = 65 + row * 240
    draw.text((24, y), label, font=LABEL, fill=(52, 72, 86))
    for col, sample in enumerate(samples):
        x = 22 + col * 314
        draw.rounded_rectangle((x, y + 28, x + 300, y + 229), radius=12, fill="white")
        frame = panel(state, direction, sample, (300, 193))
        sheet.paste(frame, (x, y + 36), frame)
sheet.save(OUTPUT / "locomotion-preview.png")
print("LOCOMOTION PREVIEWS OK")
