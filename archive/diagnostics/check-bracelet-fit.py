"""Read-only Blender audit of the jade bracelet against the body and left horn.

blender --background --disable-autoexec --python work/check-bracelet-fit.py -- \
    --output work/bracelet-baseline.json

Distances use world units. A negative sphere clearance means intersection.
Only structural shells enter BVHs; open microfiber triangles are excluded.
No source scene, runtime asset, or product code is written by this script.
"""
from pathlib import Path
import argparse
import hashlib
import json
import math
import sys
from datetime import datetime, timezone

import bpy
from mathutils import Vector
from mathutils.bvhtree import BVHTree

ROOT = Path(__file__).resolve().parents[1]
args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
parser = argparse.ArgumentParser()
parser.add_argument("--source", type=Path, default=ROOT / "source_models/shu-plush/shu-plush.blend")
parser.add_argument("--output", type=Path, default=ROOT / "work/bracelet-check.json")
options = parser.parse_args(args)
source = options.source.resolve()
output = options.output.resolve()
if output.suffix.lower() != ".json" or output == source:
    raise RuntimeError("Audit output must be a separate JSON file")
before_hash = hashlib.sha256(source.read_bytes()).hexdigest()
if bpy.context.preferences.filepaths.use_scripts_auto_execute:
    raise RuntimeError("Run Blender with --disable-autoexec")
bpy.ops.wm.open_mainfile(filepath=str(source), use_scripts=False)
depsgraph = bpy.context.evaluated_depsgraph_get()


def structural(name):
    return ".MicroPile" not in name


def surface_bvh(objects):
    vertices, polygons = [], []
    for obj in objects:
        evaluated = obj.evaluated_get(depsgraph)
        mesh = evaluated.to_mesh()
        base = len(vertices)
        vertices.extend(evaluated.matrix_world @ vertex.co for vertex in mesh.vertices)
        polygons.extend(tuple(base + index for index in face.vertices) for face in mesh.polygons)
        evaluated.to_mesh_clear()
    return BVHTree.FromPolygons(vertices, polygons, all_triangles=False)


def signed_clearance(tree, center, radius):
    nearest, normal, face_index, distance = tree.find_nearest(center)
    if nearest is None:
        raise RuntimeError("Empty collision surface")
    normal.normalize()
    dot = (center - nearest).dot(normal)
    signed = math.copysign(distance, dot) if dot else 0.0
    return {
        "center_signed_distance": signed,
        "center_normal_dot": dot,
        "sphere_clearance": signed - radius,
        "nearest_point": list(nearest),
        "nearest_normal": list(normal),
        "nearest_face": face_index,
    }


body_objects = [obj for obj in bpy.context.scene.objects
                if obj.type == "MESH" and obj.name.startswith("BodyShell") and structural(obj.name)]
horn_objects = [obj for obj in bpy.context.scene.objects
                if obj.type == "MESH" and obj.name == "ForkedHorn.L"]
beads = sorted([obj for obj in bpy.context.scene.objects
                if obj.type == "MESH" and obj.name.startswith("JadeBead.")], key=lambda obj: obj.name)
if not body_objects or len(horn_objects) != 1 or not beads:
    raise RuntimeError("Expected BodyShell, one ForkedHorn.L and JadeBead.* meshes")
body_tree = surface_bvh(body_objects)
horn_tree = surface_bvh(horn_objects)
measurements = []
for obj in beads:
    evaluated = obj.evaluated_get(depsgraph)
    mesh = evaluated.to_mesh()
    center = evaluated.matrix_world @ Vector((0, 0, 0))
    radius = max((evaluated.matrix_world @ vertex.co - center).length for vertex in mesh.vertices)
    evaluated.to_mesh_clear()
    measurements.append({
        "name": obj.name,
        "center": list(center),
        "maximum_world_radius": radius,
        "body": signed_clearance(body_tree, center, radius),
        "horn": signed_clearance(horn_tree, center, radius),
    })

neighbours = []
for index, first in enumerate(measurements):
    second = measurements[(index + 1) % len(measurements)]
    distance = (Vector(first["center"]) - Vector(second["center"])).length
    gap = distance - first["maximum_world_radius"] - second["maximum_world_radius"]
    neighbours.append({"first": first["name"], "second": second["name"],
                       "center_distance": distance, "sphere_clearance": gap})
after_hash = hashlib.sha256(source.read_bytes()).hexdigest()
if after_hash != before_hash:
    raise RuntimeError("Source file changed during audit; discard this measurement")
report = {
    "source": str(source),
    "source_sha256": before_hash,
    "source_preserved": True,
    "blender_version": bpy.app.version_string,
    "measured_at_utc": datetime.now(timezone.utc).isoformat(),
    "coordinates": "Blender world: +Z up; distances are world units",
    "method": "Nearest structural-face normal dot gives center distance sign; subtract maximum world bead radius. Open microfibers excluded. Neighbour gaps use cyclic bead-name order.",
    "body_meshes": [obj.name for obj in body_objects],
    "horn_meshes": [obj.name for obj in horn_objects],
    "bead_count": len(measurements),
    "minimum_body_clearance": min(item["body"]["sphere_clearance"] for item in measurements),
    "minimum_horn_clearance": min(item["horn"]["sphere_clearance"] for item in measurements),
    "minimum_neighbour_clearance": min(item["sphere_clearance"] for item in neighbours),
    "beads": measurements,
    "neighbours": neighbours,
}
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
for item in measurements:
    print(f"{item['name']}: radius={item['maximum_world_radius']:.6f} "
          f"body_gap={item['body']['sphere_clearance']:+.6f} "
          f"horn_gap={item['horn']['sphere_clearance']:+.6f}")
print(f"MIN BODY {report['minimum_body_clearance']:+.6f}; "
      f"MIN HORN {report['minimum_horn_clearance']:+.6f}; "
      f"MIN ADJACENT {report['minimum_neighbour_clearance']:+.6f}")
print(f"BRACELET AUDIT SAVED: {output}")
