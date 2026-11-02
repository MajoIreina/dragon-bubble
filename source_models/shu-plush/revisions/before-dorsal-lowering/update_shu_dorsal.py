"""Replace only the dorsal strip, ridge seam and micro-pile in the saved model.

blender --background --disable-autoexec --python tools/update_shu_dorsal.py
The full build uses the same geometry; unrelated meshes and clips stay intact.
"""
from pathlib import Path
import ast
import hashlib
import json
import math
import random
import sys
from array import array
from collections import Counter

import bpy
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from shu_dorsal import dorsal_geometry

SOURCE = ROOT / "source_models/shu-plush/shu-plush.blend"
NAMES = {"ContinuousDorsalFin", "ContinuousDorsalFin.MicroPile", "DorsalRidgeSeam"}
bpy.ops.wm.open_mainfile(filepath=str(SOURCE), use_scripts=False)
pet = bpy.data.collections["ShuPlush"]
body = bpy.data.objects["PlushBody"]
origins = {body: (0, 0, 0)}
gold = bpy.data.materials["Harvest yellow plush • 麦黄"]
thread = bpy.data.materials["Golden seam thread"]


def fingerprint(obj):
    digest = hashlib.sha256()
    digest.update(repr((obj.name, obj.parent.name if obj.parent else None,
                        tuple(tuple(row) for row in obj.matrix_local))).encode())
    if obj.type == "MESH":
        for collection, field, width, kind in [(obj.data.vertices, "co", 3, "f"),
                                               (obj.data.loops, "vertex_index", 1, "i")]:
            values = array(kind, [0]) * (len(collection) * width)
            collection.foreach_get(field, values)
            digest.update(values.tobytes())
        digest.update(repr([m.name for m in obj.data.materials]).encode())
    if obj.animation_data:
        digest.update(repr([(t.name, t.mute, [(s.name, s.action.name, s.frame_start, s.frame_end)
                                             for s in t.strips])
                            for t in obj.animation_data.nla_tracks]).encode())
    return digest.hexdigest()


unrelated = {obj.name: fingerprint(obj) for obj in pet.objects if obj.name not in NAMES}
actions = {action.name: action.as_pointer() for action in bpy.data.actions}
for name in NAMES:
    if name not in bpy.data.objects:
        raise RuntimeError(f"Expected existing dorsal component: {name}")
for name in NAMES:
    obj = bpy.data.objects[name]
    data = obj.data
    bpy.data.objects.remove(obj, do_unlink=True)
    if data.users == 0:
        bpy.data.meshes.remove(data)

# Reuse the build's mesh, tube and micro-pile implementations without running
# its factory reset, ornament fitting, animation generation or render stage.
build = ast.parse((ROOT / "tools/build_shu_plush.py").read_text(encoding="utf-8"))
functions = [node for node in build.body if isinstance(node, ast.FunctionDef)
             and node.name in {"B", "mesh", "tube", "seam", "fur"}]
exec(compile(ast.Module(body=functions, type_ignores=[]), "build_shu_helpers", "exec"))
vertices, faces, uv, ridge = dorsal_geometry()
dorsal = mesh("ContinuousDorsalFin", vertices, faces, gold, uv=uv)
seam("DorsalRidgeSeam", ridge, thread, .0022, stitches=False)
fur(dorsal, 2500, .005, gold)

edge_counts = Counter()
for face in faces:
    for a, b in zip(face, face[1:] + face[:1]):
        edge_counts[tuple(sorted((a, b)))] += 1
if any(count != 2 for count in edge_counts.values()):
    raise RuntimeError("Dorsal strip must be a closed manifold mesh")
if unrelated != {obj.name: fingerprint(obj) for obj in pet.objects if obj.name not in NAMES}:
    raise RuntimeError("An unrelated model component changed")
if actions != {action.name: action.as_pointer() for action in bpy.data.actions}:
    raise RuntimeError("Native animations changed")

bpy.context.scene["ModelVersion"] = "2026-11-02 photograph reconstruction v6 • swept continuous dorsal sketch"
bpy.ops.wm.save_as_mainfile(filepath=str(SOURCE))
report = {"modified_objects": sorted(NAMES), "unrelated_objects_preserved": len(unrelated),
          "actions_preserved": len(actions), "closed_manifold": True,
          "vertices": len(vertices), "faces": len(faces)}
(ROOT / "work/shu-dorsal-validation.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
print("DORSAL UPDATED", json.dumps(report), flush=True)
