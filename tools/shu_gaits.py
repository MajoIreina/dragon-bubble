"""Bake planted walking and a feline double-suspension gallop.

Only transform tracks change; foot soles stay level in stance even as their
parent body compresses, pitches and rises. Coordinates here are Blender axes.
"""
import math

import bpy
from mathutils import Matrix, Vector


GAITS = {
    "Walk": {"frames": 29, "speed": 42.0, "duty": 0.70, "lift": 0.16,
             "offsets": [0.0, 0.25, 0.5, 0.75]},
    "Run": {"frames": 16, "speed": 112.0, "duty": 0.25, "lift": 0.40,
            "offsets": [0.55, 0.05, 0.50, 0.0]},
}


def bake_gaits(collection, only=None):
    scene = bpy.context.scene
    objects = list(collection.all_objects)
    body = collection.objects["PlushBody"]
    feet = [collection.objects[name] for name in
            ("FrontFoot.L", "RearFoot.L", "FrontFoot.R", "RearFoot.R")]
    tail = collection.objects["TailWag"]
    animated = [obj for obj in objects if obj.animation_data and obj.animation_data.nla_tracks]
    for obj in animated:
        obj.animation_data.action = None
        for track in obj.animation_data.nla_tracks:
            track.mute = True
    scene.frame_set(1)
    bpy.context.view_layer.update()
    rests = {obj: obj.matrix_basis.copy() for obj in animated}
    world_rests = {obj: obj.matrix_world.copy() for obj in animated}
    corners = [obj.matrix_world @ Vector(corner)
               for obj in objects if obj.type == "MESH" for corner in obj.bound_box]
    longest = max(max(p[i] for p in corners) - min(p[i] for p in corners) for i in range(3))
    normalization = 2.35 / longest
    source_per_pixel = 3.9 / 300.0 / normalization
    fps = scene.render.fps / scene.render.fps_base
    originals = {obj: next(track.strips[0].action for track in obj.animation_data.nla_tracks
                            if track.name == "Walk") for obj in animated}
    report = {}
    for name, gait in GAITS.items():
        if only is not None and name not in only:
            continue
        frames, duty = gait["frames"], gait["duty"]
        duration = frames / fps
        travel = gait["speed"] * duration * duty * source_per_pixel
        report[name] = {"seconds": duration, "stance_fraction": duty,
                        "stance_travel": travel, "nominal_speed_px": gait["speed"]}
        for obj in animated:
            for track in list(obj.animation_data.nla_tracks):
                if track.name == name:
                    obj.animation_data.nla_tracks.remove(track)
        # Keep the original facial and hanging-ribbon tracks, with exactly the
        # same safe angular range. The running clip plays them more quickly.
        for obj in animated:
            if obj in [body, tail] + feet:
                continue
            action = originals[obj].copy()
            action.name = name + "." + obj.name
            _track(obj, name, action, frames, retime=True)
        for obj in [body, tail] + feet:
            obj.animation_data.action = None
        for index in range(73):
            t = index / 72.0
            phase = t * math.tau
            for obj in animated:
                obj.matrix_basis = rests[obj]
            if name == "Walk":
                height = 0.014 * (1.0 - math.cos(phase * 2))
                vertical = 1.0 - 0.006 * math.cos(phase * 2)
                pitch = 0.009 * math.sin(phase * 2)
                side = 0.009 * math.sin(phase)
            else:
                # Hind push-off, extended flight, fore landing, gathered flight.
                extension = math.cos(phase - math.tau * 0.35)
                suspension = (1.0 + math.cos(2 * phase - math.tau * 0.66)) * 0.5
                height = 0.035 + 0.15 * suspension
                vertical = 0.975 - 0.025 * extension
                pitch = 0.048 * math.sin(phase - 0.2)
                side = 0.008 * math.sin(phase)
            length_scale = 1.0 + 0.065 * extension if name == "Run" else 1 / math.sqrt(vertical)
            body.matrix_basis = rests[body] @ Matrix.Translation((side, 0, height)) @ Matrix.Rotation(pitch, 4, "X") @ Matrix.Diagonal((1 / math.sqrt(vertical), length_scale, vertical, 1))
            tail.rotation_euler.z += (0.10 if name == "Walk" else 0.16) * math.sin(phase - 0.6)
            if name == "Run":
                tail.rotation_euler.x += 0.055 * math.sin(phase - 0.4)
            bpy.context.view_layer.update()
            for foot, offset in zip(feet, gait["offsets"]):
                p = (t + offset) % 1.0
                if p < duty:
                    # Window travel cancels the linear backwards stance motion.
                    forward = travel * (0.5 - p / duty)
                    lift = roll = 0.0
                else:
                    swing = (p - duty) / (1.0 - duty)
                    forward = travel * (-0.5 + swing - math.sin(math.tau * swing) / math.tau)
                    arch = math.sin(math.pi * swing) ** 2
                    lift = gait["lift"] * arch * (1.10 if name == "Run" and foot in [feet[1], feet[3]] else 1.0)
                    roll = (0.12 if name == "Walk" else 0.30) * arch
                world = world_rests[foot].copy()
                world.translation += Vector((0, -forward, lift))
                world = world @ Matrix.Rotation(roll, 4, "X")
                foot.matrix_basis = foot.parent.matrix_world.inverted() @ world
            frame = 1 + t * frames
            for obj in [body, tail] + feet:
                for prop in ("location", "rotation_euler", "scale"):
                    obj.keyframe_insert(data_path=prop, frame=frame, group=obj.name)
        for obj in [body, tail] + feet:
            action = obj.animation_data.action
            action.name = name + "." + obj.name
            # Dense, linear samples avoid Bezier overshoot below the floor.
            for layer in action.layers:
                for strip in layer.strips:
                    for bag in strip.channelbags:
                        for curve in bag.fcurves:
                            for key in curve.keyframe_points:
                                key.interpolation = "LINEAR"
            obj.animation_data.action = None
            _track(obj, name, action, frames)
    for obj in animated:
        obj.animation_data.action = None
        obj.matrix_basis = rests[obj]
    scene.frame_set(1)
    bpy.context.view_layer.update()
    return report


def _track(obj, name, action, frames, retime=False):
    track = obj.animation_data.nla_tracks.new()
    track.name = name
    strip = track.strips.new(name, 1, action)
    strip.action_frame_start = 1
    if not retime:
        strip.action_frame_end = frames + 1
    else:
        strip.scale = frames / (strip.action_frame_end - strip.action_frame_start)
    strip.frame_end = frames + 1
    track.mute = True
