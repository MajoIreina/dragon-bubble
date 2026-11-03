"""Reject private/local assets in the Git index and loose models in release ZIPs."""
import argparse
from pathlib import Path, PurePosixPath
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--zip', type=Path, help='also verify a release ZIP')
args = parser.parse_args()
allowed_binary = {'assets/tray.ico'}
model_extensions = {'.glb', '.gltf', '.fbx', '.obj', '.vrm', '.stl', '.ply', '.blend'}
binary_extensions = model_extensions | {'.png', '.jpg', '.jpeg', '.gif', '.webp', '.bmp', '.tga', '.ico', '.exe', '.dll', '.zip', '.7z', '.pck', '.bundle'}
private_roots = {'archives', 'archive', 'source_models', 'skins', 'work', 'outputs', 'target', 'macos', '.godot'}
tracked = subprocess.check_output(['git', 'ls-files', '-z'], cwd=ROOT).decode('utf-8').split('\0')
errors = []
for name in filter(None, tracked):
    path = PurePosixPath(name)
    if (path.parts[0] in private_roots or '.blend' in path.suffix.lower()
            or path.suffix.lower() in binary_extensions and name not in allowed_binary):
        errors.append(name)
if args.zip:
    resources = {line.strip() for line in (ROOT / 'tools/public-resources.txt').read_text().splitlines()
                 if line.strip() and not line.startswith('#')}
    expected = resources | {'My Dragon Bubble.exe', 'LICENSE-RENDERING.txt'}
    with zipfile.ZipFile(args.zip) as package:
        files = [item.filename for item in package.infolist() if not item.is_dir()]
        if len(files) != len(set(files)) or set(files) != expected:
            errors.append('ZIP file list differs from the explicit release resource list')
        if package.testzip():
            errors.append('ZIP integrity check failed')
if errors:
    raise SystemExit('Private/unexpected files must not be published:\n' + '\n'.join(errors))
print('Public source/release file check passed.')
