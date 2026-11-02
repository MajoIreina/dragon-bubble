"""Read-only geometry/hierarchy/animation audit of the right horn attachment.

Run only after the new source has been saved:
blender --background --disable-autoexec --python work/check-horn-cord.py -- \
    --output work/horn-cord-check.json

The blend is opened with scripts disabled and is never saved. Animation tracks
are sampled in memory; only the separate JSON audit report is written.
"""
from pathlib import Path
import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
import sys

import bpy
from mathutils import Quaternion, Vector
from mathutils.bvhtree import BVHTree

ROOT = Path(__file__).resolve().parents[1]
argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
parser = argparse.ArgumentParser()
parser.add_argument("--source", type=Path, default=ROOT / "source_models/shu-plush/shu-plush.blend")
parser.add_argument("--output", type=Path, default=ROOT / "work/horn-cord-check.json")
parser.add_argument("--tolerance", type=float, default=1e-5)
options = parser.parse_args(argv)
source, output = options.source.resolve(), options.output.resolve()
if output.suffix.lower() != ".json" or output == source:
    raise RuntimeError("Audit output must be a separate JSON report")
if bpy.context.preferences.filepaths.use_scripts_auto_execute:
    raise RuntimeError("Run Blender with --disable-autoexec")
before_hash = hashlib.sha256(source.read_bytes()).hexdigest()
bpy.ops.wm.open_mainfile(filepath=str(source), use_scripts=False)
scene = bpy.context.scene
objects = list(scene.objects)
depsgraph = bpy.context.evaluated_depsgraph_get()


def ancestors(obj):
    names = []
    parent = obj.parent
    while parent:
        names.append(parent.name)
        parent = parent.parent
    return names


def evaluated_vertices(obj):
    evaluated = obj.evaluated_get(depsgraph)
    mesh = evaluated.to_mesh()
    vertices = [evaluated.matrix_world @ vertex.co for vertex in mesh.vertices]
    evaluated.to_mesh_clear()
    return vertices


def surface_bvh(mesh_objects):
    vertices, polygons = [], []
    for obj in mesh_objects:
        evaluated = obj.evaluated_get(depsgraph)
        mesh = evaluated.to_mesh()
        offset = len(vertices)
        vertices.extend(evaluated.matrix_world @ vertex.co for vertex in mesh.vertices)
        polygons.extend(tuple(offset + index for index in face.vertices) for face in mesh.polygons)
        evaluated.to_mesh_clear()
    return BVHTree.FromPolygons(vertices, polygons, all_triangles=False)


def clearance(vertices, tree):
    values, worst = [], None
    for index, position in enumerate(vertices):
        point, normal, face, distance = tree.find_nearest(position)
        if point is None:
            raise RuntimeError("Empty structural collision mesh")
        normal.normalize()
        dot = (position - point).dot(normal)
        signed = math.copysign(distance, dot) if dot else 0.0
        values.append(signed)
        if worst is None or signed < worst["signed_clearance"]:
            worst = {"vertex_index": index, "world_position": list(position),
                     "signed_clearance": signed, "nearest_point": list(point),
                     "nearest_normal": list(normal), "normal_dot": dot, "face": face}
    return {"vertex_count": len(values), "minimum": min(values), "maximum": max(values),
            "negative_count": sum(value < 0 for value in values),
            "penetrating_count_beyond_tolerance": sum(value < -options.tolerance for value in values),
            "worst_vertex": worst}


def relative_matrix(obj, anchor):
    return anchor.evaluated_get(depsgraph).matrix_world.inverted() @ obj.evaluated_get(depsgraph).matrix_world


def matrix_difference(first, second):
    return max(abs(first[row][col] - second[row][col]) for row in range(4) for col in range(4))


def quaternion_angle(first, second):
    difference = first.rotation_difference(second).normalized()
    vector_length = math.sqrt(difference.x ** 2 + difference.y ** 2 + difference.z ** 2)
    return 2 * math.atan2(vector_length, abs(difference.w))


body_meshes = [obj for obj in objects if obj.type == "MESH" and obj.name.startswith("BodyShell") and ".MicroPile" not in obj.name]
horn_meshes = [obj for obj in objects if obj.type == "MESH" and obj.name == "ForkedHorn.R"]
cord_meshes = sorted([obj for obj in objects if obj.type == "MESH" and obj.name.startswith("BraidedHornCord.")], key=lambda obj: obj.name)
clamps = [obj for obj in objects if obj.name.startswith("SilverRectangularRibbonClamp")]
eyelets = sorted([obj for obj in objects if obj.name.startswith("ClaspEyelet.")], key=lambda obj: obj.name)
teeth = sorted([obj for obj in objects if obj.name.startswith("ClampTooth.")], key=lambda obj: obj.name)
ribbons = [obj for obj in objects if obj.name.startswith("LongOrangeRedFabricRibbon")]
attachment = bpy.data.objects.get("HornCordAttachment")
pendant = bpy.data.objects.get("PendantSway")
required = {"body": bool(body_meshes), "right_horn": len(horn_meshes) == 1,
            "braided_cord": bool(cord_meshes), "attachment_group": attachment is not None,
            "silver_clamp": bool(clamps), "clasp_eyelets": bool(eyelets),
            "thirteen_clamp_teeth": len(teeth) == 13,
            "ribbon": bool(ribbons), "pendant_sway": pendant is not None}
if not all(required.values()):
    raise RuntimeError(f"New attachment source is incomplete: {required}")
horn = horn_meshes[0]
body_tree, horn_tree = surface_bvh(body_meshes), surface_bvh(horn_meshes)
cord_results = []
for cord in cord_meshes:
    vertices = evaluated_vertices(cord)
    cord_results.append({"name": cord.name, "ancestors": ancestors(cord),
                         "body": clearance(vertices, body_tree),
                         "horn": clearance(vertices, horn_tree)})

static_parts = [attachment] + cord_meshes + clamps + eyelets + teeth
rest_relative = {obj.name: relative_matrix(obj, horn).copy() for obj in static_parts}
rest_sway = relative_matrix(pendant, horn).copy()
animation_objects = [obj for obj in objects if obj.animation_data]
saved_nla = {obj: {track.name: track.mute for track in obj.animation_data.nla_tracks} for obj in animation_objects}
saved_animation = {obj: (obj.animation_data.action, obj.animation_data.action_slot, obj.animation_data.use_nla)
                   for obj in animation_objects}
strip_metadata = {}
clip_results = {}
for clip in ("Sleep", "Walk"):
    spans = []
    strip_metadata[clip] = {}
    for obj in animation_objects:
        if saved_animation[obj][0] is not None:
            raise RuntimeError(f"Unexpected active action on {obj.name}; NLA-only audit required")
        clip_strips = []
        for track in obj.animation_data.nla_tracks:
            track.mute = True
            if track.name == clip:
                spans.extend((strip.frame_start, strip.frame_end) for strip in track.strips)
                clip_strips.extend(track.strips)
        if len(clip_strips) > 1:
            raise RuntimeError(f"Expected one {clip} strip on {obj.name}")
        # The saved preview mutes all NLA strips and their cached influence is
        # zero. Evaluate the actual action/slot referenced by each clip strip in
        # isolation, equivalent to the exporter's per-clip action sampling.
        obj.animation_data.use_nla = False
        obj.animation_data.action = None
        if clip_strips:
            strip = clip_strips[0]
            if abs(strip.scale - 1) > 1e-6 or abs(strip.repeat - 1) > 1e-6 or abs(strip.frame_start - strip.action_frame_start) > 1e-6:
                raise RuntimeError("Audit expects unretimed one-to-one NLA strips")
            strip_metadata[clip][obj.name] = {
                "action": strip.action.name, "slot": strip.action_slot.identifier,
                "saved_influence": strip.influence, "saved_track_muted": saved_nla[obj][clip],
            }
            obj.animation_data.action = strip.action
            obj.animation_data.action_slot = strip.action_slot
    if not spans:
        raise RuntimeError(f"Missing {clip} NLA tracks")
    first_frame, last_frame = min(span[0] for span in spans), max(span[1] for span in spans)
    samples = []
    for fraction in (0.0, 0.2, 0.5, 0.8):
        frame = first_frame + (last_frame - first_frame) * fraction
        scene.frame_set(math.floor(frame), subframe=frame % 1)
        depsgraph.update()
        relative = {obj.name: relative_matrix(obj, horn).copy() for obj in static_parts}
        sway_matrix = relative_matrix(pendant, horn).copy()
        samples.append({"fraction": fraction, "frame": frame,
                        "static_part_relative_matrix_max_delta": {
                            name: matrix_difference(matrix, rest_relative[name]) for name, matrix in relative.items()},
                        "pendant_relative_quaternion": list(sway_matrix.to_quaternion()),
                        "pendant_relative_rotation_from_static_rest": quaternion_angle(rest_sway.to_quaternion(), sway_matrix.to_quaternion())})
    quaternions = [Quaternion(sample["pendant_relative_quaternion"]).normalized() for sample in samples]
    # atan2 avoids acos rounding noise that could make an unchanged pose seem to rotate.
    sway_span = max(quaternion_angle(a, b) for a in quaternions for b in quaternions)
    static_max = max(delta for sample in samples for delta in sample["static_part_relative_matrix_max_delta"].values())
    clip_results[clip] = {"frame_range": [first_frame, last_frame], "samples": samples,
                          "maximum_static_relative_matrix_delta": static_max,
                          "static_parts_stay_at_horn": static_max <= options.tolerance,
                          "pendant_rotation_span_radians": sway_span,
                          "pendant_rotates": sway_span > 1e-4}

for obj in animation_objects:
    action, slot, use_nla = saved_animation[obj]
    obj.animation_data.action = action
    if action is not None:
        obj.animation_data.action_slot = slot
    obj.animation_data.use_nla = use_nla
    for track in obj.animation_data.nla_tracks:
        track.mute = saved_nla[obj][track.name]

after_hash = hashlib.sha256(source.read_bytes()).hexdigest()
if after_hash != before_hash:
    raise RuntimeError("Source changed during audit; discard this measurement")
hierarchy = {
    "attachment_ancestors": ancestors(attachment),
    "clamps": {obj.name: ancestors(obj) for obj in clamps},
    "eyelets": {obj.name: ancestors(obj) for obj in eyelets},
    "teeth": {obj.name: ancestors(obj) for obj in teeth},
    "ribbons": {obj.name: ancestors(obj) for obj in ribbons},
    "clamp_and_eyelets_in_static_attachment": all("HornCordAttachment" in ancestors(obj) for obj in clamps + eyelets),
    "cord_in_static_attachment": all("HornCordAttachment" in ancestors(obj) for obj in cord_meshes),
    "teeth_in_static_attachment": all("HornCordAttachment" in ancestors(obj) for obj in teeth),
    "ribbon_in_pendant_sway": all("PendantSway" in ancestors(obj) for obj in ribbons),
}
report = {
    "source": str(source), "source_sha256": before_hash, "source_preserved": True,
    "measured_at_utc": datetime.now(timezone.utc).isoformat(),
    "blender_version": bpy.app.version_string,
    "method": "Signed nearest-face distance from every world-space braided-cord vertex to structural body/right-horn BVH; open microfibers excluded. Negative means inside. Sleep/Walk strip-linked action/slots sampled in isolation in memory relative to right horn, without the muted preview NLA mixer suppressing channels.",
    "tolerance": options.tolerance, "required_objects": required,
    "cords": cord_results, "hierarchy": hierarchy, "animations": clip_results,
    "source_nla_strips": strip_metadata,
    "minimum_body_clearance": min(item["body"]["minimum"] for item in cord_results),
    "minimum_horn_clearance": min(item["horn"]["minimum"] for item in cord_results),
    "body_penetrating_vertices": sum(item["body"]["penetrating_count_beyond_tolerance"] for item in cord_results),
    "horn_penetrating_vertices": sum(item["horn"]["penetrating_count_beyond_tolerance"] for item in cord_results),
}
report["passed"] = (report["body_penetrating_vertices"] == 0 and report["horn_penetrating_vertices"] == 0
                     and hierarchy["clamp_and_eyelets_in_static_attachment"]
                     and hierarchy["teeth_in_static_attachment"]
                     and hierarchy["cord_in_static_attachment"] and hierarchy["ribbon_in_pendant_sway"]
                     and all(result["static_parts_stay_at_horn"] and result["pendant_rotates"] for result in clip_results.values()))
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
for cord in cord_results:
    print(f"{cord['name']}: {cord['body']['vertex_count']} vertices, "
          f"body_min={cord['body']['minimum']:+.8f}, horn_min={cord['horn']['minimum']:+.8f}")
print("HIERARCHY", hierarchy)
for clip, result in clip_results.items():
    print(f"{clip}: static_delta={result['maximum_static_relative_matrix_delta']:.9f}, "
          f"ribbon_sway={result['pendant_rotation_span_radians']:.8f} rad")
print(f"HORN CORD AUDIT SAVED: {output}; passed={report['passed']}")
