"""Fixed framing for actual Godot turn, idle and sleep captures."""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
FRAMES = ROOT / "work/turn-rest-frames"
OUTPUT = ROOT / "outputs"
TITLE = ImageFont.truetype("C:/Windows/Fonts/msyh.ttc", 24)
LABEL = ImageFont.truetype("C:/Windows/Fonts/msyh.ttc", 17)
BG = (239, 243, 247)


def panel(scene, index):
    image = Image.open(FRAMES / f"{scene}-{index:03d}.png").convert("RGBA")
    return image.crop((50, 125, 590, 485)).resize((432, 288), Image.Resampling.LANCZOS)


def canvas(title, scenes, index):
    result = Image.new("RGB", (936, 405), BG)
    draw = ImageDraw.Draw(result)
    draw.text((24, 16), title, font=TITLE, fill=(34, 53, 66))
    for x, (scene, label) in zip((24, 480), scenes):
        draw.rounded_rectangle((x, 65, x + 432, 383), radius=16, fill="white")
        draw.text((x + 18, 77), label, font=LABEL, fill=(83, 106, 120))
        frame = panel(scene, index)
        result.paste(frame, (x, 95), frame)
    return result


for name, title, scenes, count, step in [
        ("shu-turn-preview", "边缘收步 · 原地转身 · 重新起步", [("turn-right", "走路 · 两个方向"), ("run-turn", "跑步 · 两个方向")], 120, 1),
        ("shu-rest-preview", "待机与睡眠 · 实际动画和特效", [("idle", "待机 · 呼吸 / 眨眼 / 顾盼"), ("sleep", "睡觉 · 闭眼 / 慢呼吸 / 气泡")], 144, 2)]:
    indices = list(range(0, count, step))
    frames = [canvas(title, scenes, i) for i in indices]
    durations = [round((i + step) * 100 / 24) * 10 - round(i * 100 / 24) * 10 for i in indices]
    frames[0].save(OUTPUT / f"{name}.gif", save_all=True, append_images=frames[1:],
                   duration=durations, loop=0, disposal=2, optimize=True)
    frames[len(frames) // 2].save(OUTPUT / f"{name}.png")

sheet = Image.new("RGB", (1248, 930), BG)
draw = ImageDraw.Draw(sheet)
draw.text((22, 14), "转身、待机与睡觉 · 逐帧检查", font=TITLE, fill=(34, 53, 66))
for row, (scene, label, samples) in enumerate([
        ("turn-right", "180° 原地转身", [12, 17, 22, 30]),
        ("idle", "待机：呼吸、眨眼和顾盼", [0, 36, 98, 110]),
        ("sleep", "睡眠：闭眼慢呼吸与气泡", [12, 48, 74, 116])]):
    y = 58 + row * 288
    draw.text((22, y), label, font=LABEL, fill=(52, 72, 86))
    for col, index in enumerate(samples):
        x = 18 + col * 308
        draw.rounded_rectangle((x, y + 30, x + 294, y + 262), radius=12, fill="white")
        frame = panel(scene, index).resize((294, 196), Image.Resampling.LANCZOS)
        sheet.paste(frame, (x, y + 62), frame)
sheet.save(OUTPUT / "turn-rest-preview.png")
print("TURN AND REST PREVIEWS OK")
