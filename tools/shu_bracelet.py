"""Fit the pearl bracelet to the actual forked horn and head, in Blender.

The loop follows the exposed base of the complete L-shaped horn. Sphere
clearance is solved against both meshes; a fixed ellipse can bury beads in
either one. Values are in the same units as the source model.
"""
import bisect
import json
import math
from mathutils import Vector
from mathutils.bvhtree import BVHTree


def _world_tree(obj):
    return BVHTree.FromPolygons(
        [obj.matrix_world @ vertex.co for vertex in obj.data.vertices],
        [tuple(poly.vertices) for poly in obj.data.polygons],
    )


def _signed(tree, point):
    hit, normal, _, distance = tree.find_nearest(point)
    if hit is None:
        raise RuntimeError('Bracelet collider has no surface')
    outside = (point - hit).dot(normal) >= 0
    direction = (point-hit).normalized() if outside and distance > 1e-9 else normal
    return distance if outside else -distance, direction


def _project(point, colliders):
    point = point.copy()
    for _ in range(12):
        moved = False
        for tree, margin in colliders:
            distance, normal = _signed(tree, point)
            if distance < margin:
                point += normal * (margin-distance+0.00005)
                moved = True
        if not moved:
            break
    return point


def _resample(points, count):
    closed = points + [points[0]]
    lengths = [0.0]
    for a, b in zip(closed, closed[1:]):
        lengths.append(lengths[-1] + (b-a).length)
    sampled = []
    for i in range(count):
        at = lengths[-1] * i / count
        index = min(len(points)-1, bisect.bisect_right(lengths, at)-1)
        fraction = (at-lengths[index]) / (lengths[index+1]-lengths[index])
        sampled.append(closed[index].lerp(closed[index+1], fraction))
    return sampled, lengths[-1]


def fit_base_loop(horn, head, main_path, max_radius=.047):
    """Fit a dense base loop, in Blender world coordinates, with pile clearance."""
    def native(p): return Vector((p[0], -p[2], p[1]))
    horn_tree, head_tree = _world_tree(horn), _world_tree(head)
    # Allow room for the horn's 0.008 and body's 0.018 maximum micro-pile.
    horn_gap, head_gap = .010, .021
    colliders = [(horn_tree, max_radius+horn_gap),
                 (head_tree, max_radius+head_gap)]
    centerline = [native(p) for p in main_path]
    axis = (centerline[18]-centerline[8]).normalized()
    front = native((0, 0, 1))
    across = (front-axis*front.dot(axis)).normalized()
    sideways = axis.cross(across).normalized()

    def center(t):
        index = min(len(centerline)-2, int(t*(len(centerline)-1)))
        return centerline[index].lerp(centerline[index+1], t*(len(centerline)-1)-index)

    def pearl_center(t, radial):
        origin = center(t)
        # Cast from outside towards the main axis to enclose the forward spur,
        # including any concave transition between the two stuffed branches.
        hit, _, _, _ = horn_tree.ray_cast(origin+radial*1.5, -radial, 2.0)
        if hit is None:
            raise RuntimeError('Bracelet section misses horn')
        low = max(0, (hit-origin).dot(radial))
        high = low+max_radius+horn_gap+.04
        while _signed(horn_tree, origin+radial*high)[0] < max_radius+horn_gap:
            high += .04
        for _ in range(18):
            middle = (low+high)/2
            if _signed(horn_tree, origin+radial*middle)[0] < max_radius+horn_gap:
                low = middle
            else:
                high = middle
        return origin+radial*high

    path = []
    for i in range(360):
        angle = math.tau*i/360
        radial = across*math.cos(angle)+sideways*math.sin(angle)
        low = .025
        candidate = pearl_center(low, radial)
        if _signed(head_tree, candidate)[0] < max_radius+head_gap:
            # Find the first exposed section, rather than moving the whole
            # bracelet upwards and leaving the outer side floating.
            for j in range(1, 33):
                high = .025+.70*j/32
                candidate = pearl_center(high, radial)
                if _signed(head_tree, candidate)[0] >= max_radius+head_gap:
                    break
                low = high
            else:
                raise RuntimeError('Bracelet cannot clear head at this angle')
            for _ in range(18):
                middle = (low+high)/2
                candidate = pearl_center(middle, radial)
                if _signed(head_tree, candidate)[0] < max_radius+head_gap:
                    low = middle
                else:
                    high = middle
            candidate = pearl_center(high, radial)
        path.append(_project(candidate, colliders))
    # Smooth section transitions while keeping every point outside both shells.
    for _ in range(5):
        path = [_project(path[i]*.6+(path[i-1]+path[(i+1)%len(path)])*.2,
                         colliders) for i in range(len(path))]
    return path, colliders


def fit_bracelet(horn, head, main_path, report_path):
    """Return evenly spaced pearls and a string in script +Y-up coordinates."""
    def script(p): return (p.x, p.z, -p.y)
    max_radius, horn_gap, head_gap = .047, .010, .021
    path, colliders = fit_base_loop(horn, head, main_path, max_radius)
    horn_tree, head_tree = (item[0] for item in colliders)
    _, length = _resample(path, 360)
    count = max(12, round(length/(2*max_radius+.0018)))
    while True:
        centers, _ = _resample(path, count)
        centers = [_project(p, colliders) for p in centers]
        minimum_pitch = min((a-b).length for a,b in zip(centers, centers[1:]+centers[:1]))
        radius = min(max_radius, minimum_pitch/2-.0008)
        if radius >= .045 or count <= 12:
            break
        count -= 1
    body_clearances = [_signed(head_tree, p)[0]-radius for p in centers]
    horn_clearances = [_signed(horn_tree, p)[0]-radius for p in centers]
    all_gaps = [(a-b).length-2*radius for i,a in enumerate(centers)
                for b in centers[i+1:]]
    # The string uses the fitted dense curve, never a chord through the horn.
    string_clearance = min(min(_signed(tree,p)[0]-.002 for tree,_ in colliders)
                           for p in path)
    report = {'beads': count, 'radius': radius, 'loop_length': length,
              'minimum_body_gap': min(body_clearances),
              'minimum_horn_gap': min(horn_clearances),
              'minimum_bead_gap': min(all_gaps),
              'minimum_string_gap': string_clearance,
              'centers_script_coordinates': [script(p) for p in centers]}
    if min(body_clearances) < head_gap-1e-5 or min(horn_clearances) < horn_gap-1e-5:
        raise RuntimeError('Bracelet fitting left a pearl inside a collider')
    if min(all_gaps) < .001 or string_clearance < .003:
        raise RuntimeError('Bracelet pearls or string intersect')
    report_path.write_text(json.dumps(report, indent=2), encoding='utf-8')
    print('BRACELET FIT',json.dumps({k:v for k,v in report.items() if k!='centers_script_coordinates'}),flush=True)
    return [script(p) for p in centers], radius, [script(p) for p in path+[path[0]]]
