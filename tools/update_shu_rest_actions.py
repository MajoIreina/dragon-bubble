"""Update only Idle, Sleep and Turn, verifying geometry and other actions."""
from array import array
import hashlib
import json
from pathlib import Path
import sys

import bpy

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from shu_rest_actions import bake_rest_actions

SOURCE = ROOT / "source_models/shu-plush/shu-plush.blend"
bpy.ops.wm.open_mainfile(filepath=str(SOURCE), use_scripts=False)
pet = bpy.data.collections["ShuPlush"]


def geometry_digest():
    digest = hashlib.sha256()
    for obj in sorted(pet.all_objects, key=lambda obj: obj.name):
        digest.update(repr((obj.name, obj.parent.name if obj.parent else None)).encode())
        if obj.type == "MESH":
            for values, field, width, kind in [(obj.data.vertices, "co", 3, "f"), (obj.data.loops, "vertex_index", 1, "i")]:
                data = array(kind, [0]) * (len(values) * width)
                values.foreach_get(field, data)
                digest.update(data.tobytes())
            digest.update(repr([m.name for m in obj.data.materials]).encode())
    return digest.hexdigest()


def other_actions():
    return {action.name: action.as_pointer() for action in bpy.data.actions
            if not action.name.startswith(("Idle.", "Sleep.", "Turn."))}


before = geometry_digest()
actions = other_actions()
report = bake_rest_actions(pet)
if geometry_digest() != before or other_actions() != actions:
    raise RuntimeError("Rest action update altered geometry or unrelated actions")
report["geometry_preserved"] = True
report["other_actions_preserved"] = len(actions)
bpy.ops.wm.save_as_mainfile(filepath=str(SOURCE))
(ROOT / "work/shu-rest-validation.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
print("SHU REST ACTIONS UPDATED", json.dumps(report), flush=True)
