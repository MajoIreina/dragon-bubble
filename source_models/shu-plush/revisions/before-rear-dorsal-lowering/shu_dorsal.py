"""Continuous padded dorsal outline, following the user's side-view sketch.

Coordinates are (Z, Y): +Z faces forward. The rear peaks are taller relative
to their local body surface so all three remain visible after body settling.
"""
import math

from shu_geometry import cubic, profile_at_z

# Lower the visible fin height relative to the local back, preserving its root.
HEIGHT_SCALE = .85


def body_top(z):
    _, radius, center = profile_at_z(z)
    return center + .92 * radius


# Long rising front edges, short steep rear edges, and rounded tips/valleys.
# Shared end tangents keep the strip smooth without symmetric bell-shaped peaks.
OUTLINE = [
    ((.42, body_top(.42) + .002), (.26, .90), (.01, 1.37), (-.18, 1.37)),
    ((-.18, 1.37), (-.27, 1.37), (-.33, .99), (-.43, .965)),
    ((-.43, .965), (-.49, .950), (-.90, 1.32), (-.98, 1.32)),
    ((-.98, 1.32), (-1.055, 1.32), (-1.045, .84), (-1.10, .80)),
    ((-1.10, .80), (-1.22, .713), (-1.54, 1.29), (-1.65, 1.29)),
    ((-1.65, 1.29), (-1.72, 1.29), (-1.68, .45), (-1.75, .35)),
    ((-1.75, .35), (-1.77, .321), (-1.80, body_top(-1.82) + .002),
     (-1.82, body_top(-1.82) + .002)),
]


def dorsal_geometry():
    outline = []
    for segment in OUTLINE:
        # Sample by approximate arc length so near-vertical edges are as smooth
        # as the long slopes, including their rounded caps.
        length = sum(math.dist(segment[i], segment[i + 1]) for i in range(3))
        steps = max(12, math.ceil(length / .012))
        outline.extend(cubic(*segment, i / steps) for i in range(steps))
    outline.append(OUTLINE[-1][-1])
    if any(outline[i][0] <= outline[i + 1][0] for i in range(len(outline) - 1)):
        raise ValueError("Dorsal outline must progress from head to tail")

    vertices, faces, uv, ridge = [], [], [], []
    columns = 24
    distance = 0.0
    for i, (z, top) in enumerate(outline):
        if i:
            distance += math.dist(outline[i - 1], outline[i])
        surface = body_top(z)
        bottom = surface - .035
        top = max(surface + (top - surface) * HEIGHT_SCALE, bottom + .018)
        # Both ends taper into the body. The sewn root stays embedded even
        # where the third peak follows the much narrower rear body.
        taper = min(1.0, (.42 - z) / .07, (z + 1.82) / .07)
        for j in range(columns):
            angle = j * math.tau / columns
            y = bottom + (top - bottom) * (.5 + .5 * math.cos(angle))
            x = .047 * math.sin(angle) * max(.025, taper)
            vertices.append((x, y, z))
            uv.append((j / columns * 2, distance * 2.5))
            if i:
                a = (i - 1) * columns + j
                b = (i - 1) * columns + (j + 1) % columns
                faces.append((a, b, b + columns, a + columns))
        ridge.append((0, top + .001, z))
    faces.extend([tuple(reversed(range(columns))),
                  tuple((len(outline) - 1) * columns + j for j in range(columns))])
    return vertices, faces, uv, ridge
