"""Planted idle, relaxed sleep and stepping turn; transform animation only."""
import math

import bpy
from mathutils import Matrix, Vector


def bake_rest_actions(collection):
    scene = bpy.context.scene
    animated = [obj for obj in collection.all_objects
                if obj.animation_data and obj.animation_data.nla_tracks]
    for obj in animated:
        obj.animation_data.action = None
        for track in obj.animation_data.nla_tracks:
            track.mute = True
    scene.frame_set(1)
    bpy.context.view_layer.update()
    rests = {obj: obj.matrix_basis.copy() for obj in animated}
    worlds = {obj: obj.matrix_world.copy() for obj in animated}
    body, tail, eyes, lids = [collection.objects[name] for name in
                              ["PlushBody", "TailWag", "BlinkBothEyes", "ClosedEyelids"]]
    feet = [collection.objects[name] for name in
            ["FrontFoot.L", "RearFoot.L", "FrontFoot.R", "RearFoot.R"]]
    originals = {obj: {track.name: track.strips[0].action for track in obj.animation_data.nla_tracks}
                 for obj in animated}
    report = {}
    for name, frames in [("Idle", 144), ("Sleep", 132), ("Turn", 19)]:
        for obj in animated:
            for track in list(obj.animation_data.nla_tracks):
                if track.name == name:
                    obj.animation_data.nla_tracks.remove(track)
        for obj in animated:
            if obj not in [body, tail, eyes, lids] + feet:
                # Hanging ribbon keeps its proven safe angular envelope.
                action = originals[obj]["Sleep" if name == "Sleep" else "Idle"].copy()
                action.name = name + "." + obj.name
                _track(obj, name, action, frames, retime=True)
        samples = frames * 2
        for i in range(samples + 1):
            t = i / samples
            phase = t * math.tau
            for obj in animated:
                obj.matrix_basis = rests[obj]
            if name == "Idle":
                vertical = 1 + 0.012 * math.sin(phase * 2)
                height = 0.004 * math.sin(phase * 2)
                pitch = 0.006 * math.sin(phase)
                # A slow glance, followed by a quiet pause and return.
                glance = math.sin(phase) ** 3 * 0.026
                yaw = glance
                # Two fast blinks at different points in the long idle loop.
                blink = max(_blink(t, 0.34, 0.024), _blink(t, 0.77, 0.025), _blink(t, 0.82, 0.018))
                tail.rotation_euler.z += 0.065 * math.sin(phase - 0.45) + 0.018 * math.sin(phase * 3)
            elif name == "Sleep":
                breath = math.sin(phase)
                vertical = 0.925 + 0.014 * breath
                height = -0.025 + 0.004 * breath
                pitch = 0.008 + 0.006 * math.sin(phase - 0.3)
                yaw = 0.005 * math.sin(phase)
                blink = 1.0
                tail.rotation_euler.z += 0.018 * math.sin(phase - 0.8)
                tail.rotation_euler.x += 0.008 * math.sin(phase)
            else:
                step = math.sin(phase * 2)
                vertical = 1 - 0.018 * math.sin(math.pi * t) ** 2
                height = 0.006 * (1 - math.cos(phase * 2))
                pitch = 0.007 * step
                yaw = 0.0
                blink = _blink(t, 0.47, 0.075)
                tail.rotation_euler.z += 0.10 * math.sin(phase)
            body.matrix_basis = rests[body] @ Matrix.Translation((0, 0, height)) @ Matrix.Rotation(yaw, 4, "Z") @ Matrix.Rotation(pitch, 4, "X") @ Matrix.Diagonal((1 / math.sqrt(vertical), 1 / math.sqrt(vertical), vertical, 1))
            eyes.scale.z *= 0.04 if name == "Sleep" else 1.0 - 0.96 * blink
            closed = 1.0 if name == "Sleep" else max(0.001, (blink - 0.65) / 0.35)
            lids.scale = Vector((closed, closed, closed))
            bpy.context.view_layer.update()
            for j, foot in enumerate(feet):
                world = worlds[foot].copy()
                if name == "Turn":
                    p = (2 * t + (0.5 if j in (1, 2) else 0)) % 1.0
                    # Overlapping support survives the glTF exporter's 24 Hz
                    # sampling, so interpolation never lifts all four feet.
                    arch = math.sin(math.pi * (p - 0.64) / 0.36) ** 2 if p >= 0.64 else 0.0
                    world.translation.z += 0.095 * arch
                foot.matrix_basis = foot.parent.matrix_world.inverted() @ world
            for obj in [body, tail, eyes, lids] + feet:
                for prop in ("location", "rotation_euler", "scale"):
                    obj.keyframe_insert(data_path=prop, frame=1 + t * frames, group=obj.name)
        for obj in [body, tail, eyes, lids] + feet:
            action = obj.animation_data.action
            action.name = name + "." + obj.name
            for layer in action.layers:
                for strip in layer.strips:
                    for bag in strip.channelbags:
                        for curve in bag.fcurves:
                            for key in curve.keyframe_points:
                                key.interpolation = "LINEAR"
            obj.animation_data.action = None
            _track(obj, name, action, frames)
        report[name] = {"seconds": frames / 24, "planted_feet": True}
    for obj in animated:
        obj.animation_data.action = None
        obj.matrix_basis = rests[obj]
    scene.frame_set(1)
    bpy.context.view_layer.update()
    return report


def _blink(t, center, half_width):
    u = abs(t - center) / half_width
    return 0.5 + 0.5 * math.cos(math.pi * u) if u < 1 else 0.0


def _track(obj, name, action, frames, retime=False):
    track = obj.animation_data.nla_tracks.new()
    track.name = name
    strip = track.strips.new(name, 1, action)
    strip.action_frame_start = 1
    if not retime:
        strip.action_frame_end = frames + 1
    strip.frame_end = frames + 1
    track.mute = True
