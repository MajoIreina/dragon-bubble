"""Create a GIF from the actual Godot settings animation capture."""
from pathlib import Path

from PIL import Image


root = Path(__file__).resolve().parents[1]
frames = []
for path in sorted((root / "work/win11-motion").glob("*.png")):
    with Image.open(path) as image:
        frames.append(image.convert("RGB").resize((800, 608), Image.Resampling.LANCZOS))
if len(frames) != 80:
    raise RuntimeError(f"Expected 80 captured frames, found {len(frames)}")
frames[0].save(
    root / "outputs/win11-ui-preview.gif",
    save_all=True,
    append_images=frames[1:],
    duration=40,
    loop=0,
    optimize=True,
)
print("Created outputs/win11-ui-preview.gif")
