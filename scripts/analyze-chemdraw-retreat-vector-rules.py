#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import math
from collections import defaultdict
from pathlib import Path

import numpy as np
from fontTools.pens.basePen import BasePen
from fontTools.ttLib import TTFont
from shapely import affinity
from shapely.geometry import Point, Polygon, box
from shapely.ops import unary_union


ROOT = Path(__file__).resolve().parents[1]
FONT_PATH = Path(r"C:\Windows\Fonts\arial.ttf")
FONT_SIZE_PT = 10.0
BASELINE_TO_NODE_EM = 0.39
BOND_LENGTH_PT = 32.0

LEGACY_ANCHOR_MAP = {
    "A": [("midpoint", 0, 1, 2), ("point", 0, 0), ("point", 0, 3)],
    "B": [("point", 0, 1), ("point", 0, 0)],
    "C": [], "D": [("point", 0, 1), ("point", 0, 0)],
    "E": [("point", 0, 1), ("point", 0, 2), ("point", 0, 0), ("point", 0, 11)],
    "F": [("point", 0, 1), ("point", 0, 2), ("point", 0, 0)],
    "G": [], "H": [("point", 0, 1), ("point", 0, 6), ("point", 0, 0), ("point", 0, 7)],
    "I": [("midpoint", 0, 1, 2), ("midpoint", 0, 0, 3)],
    "J": [("midpoint", 0, 9, 10)],
    "K": [("point", 0, 1), ("point", 0, 5), ("point", 0, 7), ("point", 0, 0)],
    "L": [("point", 0, 1), ("point", 0, 0), ("point", 0, 5)],
    "M": [("point", 0, 1), ("point", 0, 9), ("point", 0, 0), ("point", 0, 10)],
    "N": [("point", 0, 1), ("point", 0, 5), ("point", 0, 0), ("point", 0, 6)],
    "O": [], "P": [("point", 0, 1), ("point", 0, 0)],
    "Q": [("midpoint", 0, 2, 3)],
    "R": [("point", 0, 1), ("point", 0, 0), ("point", 0, 14)],
    "S": [], "T": [("midpoint", 0, 2, 3), ("midpoint", 0, 4, 5), ("midpoint", 0, 0, 7)],
    "U": [("midpoint", 0, 11, 12), ("midpoint", 0, 0, 1)],
    "V": [("point", 0, 1), ("point", 0, 9), ("midpoint", 0, 0, 10)],
    "W": [("point", 0, 1), ("point", 0, 16), ("point", 0, 0), ("point", 0, 17)],
    "X": [("point", 0, 2), ("point", 0, 10), ("point", 0, 0), ("point", 0, 12)],
    "Y": [("point", 0, 2), ("point", 0, 10), ("midpoint", 0, 0, 12)],
    "Z": [("point", 0, 6), ("point", 0, 7), ("point", 0, 0), ("point", 0, 12)],
}


class FlattenPen(BasePen):
    def __init__(self, glyph_set, steps: int = 16):
        super().__init__(glyph_set)
        self.steps = steps
        self.contours: list[list[tuple[float, float]]] = []
        self.current: list[tuple[float, float]] = []

    def _moveTo(self, point):
        if self.current:
            self.contours.append(self.current)
        self.current = [point]

    def _lineTo(self, point):
        self.current.append(point)

    def _curveToOne(self, control1, control2, point):
        start = self.current[-1]
        for index in range(1, self.steps + 1):
            t = index / self.steps
            mt = 1.0 - t
            self.current.append((
                mt**3 * start[0] + 3 * mt**2 * t * control1[0] + 3 * mt * t**2 * control2[0] + t**3 * point[0],
                mt**3 * start[1] + 3 * mt**2 * t * control1[1] + 3 * mt * t**2 * control2[1] + t**3 * point[1],
            ))

    def _qCurveToOne(self, control, point):
        start = self.current[-1]
        for index in range(1, self.steps + 1):
            t = index / self.steps
            mt = 1.0 - t
            self.current.append((
                mt * mt * start[0] + 2 * mt * t * control[0] + t * t * point[0],
                mt * mt * start[1] + 2 * mt * t * control[1] + t * t * point[1],
            ))

    def _closePath(self):
        if self.current:
            self.contours.append(self.current)
            self.current = []

    def _endPath(self):
        self._closePath()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--measurements",
        default=str(ROOT / "tmp" / "chemdraw-label-retreat-comprehensive" / "measurements.json"),
    )
    parser.add_argument(
        "--output",
        default=str(ROOT / "tmp" / "chemdraw-label-retreat-comprehensive" / "vector-rule-analysis.json"),
    )
    parser.add_argument("--font-name", default="Arial")
    parser.add_argument("--font-path", default=str(FONT_PATH))
    parser.add_argument("--font-number", type=int, default=-1)
    parser.add_argument("--font-size", type=float, default=FONT_SIZE_PT)
    parser.add_argument("--face", type=int, default=0)
    parser.add_argument("--line-width", type=float, default=0.05)
    parser.add_argument("--focused", action="store_true")
    parser.add_argument("--rule-only", action="store_true")
    parser.add_argument("--max-angular-deflection", type=float)
    return parser.parse_args()


def glyph_geometry(font: TTFont, character: str):
    cmap = font.getBestCmap()
    glyph_name = cmap.get(ord(character))
    if glyph_name is None:
        return None
    glyph_set = font.getGlyphSet()
    pen = FlattenPen(glyph_set)
    glyph_set[glyph_name].draw(pen)
    pen._endPath()
    units_per_em = font["head"].unitsPerEm
    advance_width = font["hmtx"].metrics[glyph_name][0]
    scale = FONT_SIZE_PT / units_per_em
    geometry = None
    for contour in pen.contours:
        if len(contour) < 3:
            continue
        points = [
            (
                (x - advance_width * 0.5) * scale,
                -y * scale + FONT_SIZE_PT * BASELINE_TO_NODE_EM,
            )
            for x, y in contour
        ]
        polygon = Polygon(points)
        if not polygon.is_valid:
            polygon = polygon.buffer(0)
        geometry = polygon if geometry is None else geometry.symmetric_difference(polygon)
    return geometry


def glyph_advance_pt(font: TTFont, character: str) -> float:
    glyph_name = font.getBestCmap().get(ord(character))
    if glyph_name is None:
        return 0.0
    return font["hmtx"].metrics[glyph_name][0] * FONT_SIZE_PT / font["head"].unitsPerEm


def label_geometry_parts(font: TTFont, text: str, anchor_index: int | None = None):
    advances = [glyph_advance_pt(font, character) for character in text]
    resolved_anchor = (
        max(0, min(len(text) - 1, anchor_index))
        if anchor_index is not None and text
        else None
    )
    anchor_center = (
        sum(advances[:resolved_anchor]) + advances[resolved_anchor] * 0.5
        if resolved_anchor is not None
        else sum(advances) * 0.5
    )
    cursor = -anchor_center
    parts = []
    for character, advance in zip(text, advances):
        geometry = glyph_geometry(font, character)
        if geometry is not None and not geometry.is_empty:
            offset = cursor + advance * 0.5
            parts.append((character, geometry, offset))
        cursor += advance
    return parts


def label_geometry(font: TTFont, text: str, anchor_index: int | None = None):
    parts = [
        affinity.translate(geometry, xoff=offset)
        for _, geometry, offset in label_geometry_parts(font, text, anchor_index)
    ]
    return unary_union(parts) if parts else None


def retreat_for_geometry(geometry, angle_deg: float, line_width: float) -> float:
    if geometry is None or geometry.is_empty:
        return 0.0
    rotated = affinity.rotate(geometry, -angle_deg, origin=(0.0, 0.0), use_radians=False)
    strip = box(0.0, -line_width * 0.5, BOND_LENGTH_PT, line_width * 0.5)
    intersection = rotated.intersection(strip)
    return 0.0 if intersection.is_empty else max(0.0, min(BOND_LENGTH_PT, intersection.bounds[2]))


def support_retreat_for_geometry(geometry, angle_deg: float) -> float:
    if geometry is None or geometry.is_empty:
        return 0.0
    rotated = affinity.rotate(geometry, -angle_deg, origin=(0.0, 0.0), use_radians=False)
    return max(0.0, min(BOND_LENGTH_PT, rotated.bounds[2]))


def component_line_support_retreat(geometry, angle_deg: float, line_width: float) -> float:
    if geometry is None or geometry.is_empty:
        return 0.0
    rotated = affinity.rotate(geometry, -angle_deg, origin=(0.0, 0.0), use_radians=False)
    strip = box(-BOND_LENGTH_PT, -line_width * 0.5, BOND_LENGTH_PT, line_width * 0.5)
    components = list(rotated.geoms) if hasattr(rotated, "geoms") else [rotated]
    candidates = [
        component.bounds[2]
        for component in components
        if not component.is_empty and component.intersects(strip)
    ]
    return 0.0 if not candidates else max(0.0, min(BOND_LENGTH_PT, max(candidates)))


def component_convex_hull(geometry):
    if geometry.geom_type == "Polygon":
        return geometry.convex_hull
    if hasattr(geometry, "geoms"):
        return unary_union([component.convex_hull for component in geometry.geoms])
    return geometry.convex_hull


def glyph_bbox_shape(geometry, exponent: float):
    min_x, min_y, max_x, max_y = geometry.bounds
    center_x = (min_x + max_x) * 0.5
    center_y = (min_y + max_y) * 0.5
    radius_x = (max_x - min_x) * 0.5
    radius_y = (max_y - min_y) * 0.5
    points = []
    for index in range(256):
        angle = math.tau * index / 256
        cosine = math.cos(angle)
        sine = math.sin(angle)
        points.append((
            center_x + radius_x * math.copysign(abs(cosine) ** (2.0 / exponent), cosine),
            center_y + radius_y * math.copysign(abs(sine) ** (2.0 / exponent), sine),
        ))
    return Polygon(points)


def glyph_bbox_safety_shape(geometry, exponent: float):
    return geometry.union(glyph_bbox_shape(geometry, exponent))


def sharp_vertices(geometry, minimum_turn_deg: float, convex_only: bool) -> list[tuple[float, float]]:
    polygons = list(geometry.geoms) if geometry.geom_type == "MultiPolygon" else [geometry]
    vertices = []
    for polygon in polygons:
        if polygon.geom_type != "Polygon":
            continue
        ring = polygon.exterior
        coordinates = list(ring.coords)[:-1]
        for index, current in enumerate(coordinates):
            previous = coordinates[index - 1]
            following = coordinates[(index + 1) % len(coordinates)]
            incoming = (previous[0] - current[0], previous[1] - current[1])
            outgoing = (following[0] - current[0], following[1] - current[1])
            incoming_length = math.hypot(*incoming)
            outgoing_length = math.hypot(*outgoing)
            if incoming_length <= 1e-9 or outgoing_length <= 1e-9:
                continue
            cosine = max(-1.0, min(1.0, (
                incoming[0] * outgoing[0] + incoming[1] * outgoing[1]
            ) / (incoming_length * outgoing_length)))
            interior_angle = math.degrees(math.acos(cosine))
            if 180.0 - interior_angle < minimum_turn_deg:
                continue
            cross = incoming[0] * outgoing[1] - incoming[1] * outgoing[0]
            is_convex = cross < 0 if ring.is_ccw else cross > 0
            if not convex_only or is_convex:
                vertices.append(current)
    return vertices


def legacy_anchor_points(font: TTFont, character: str) -> list[tuple[float, float]]:
    specs = LEGACY_ANCHOR_MAP.get(character, [])
    if not specs:
        return []
    glyph_name = font.getBestCmap().get(ord(character))
    if glyph_name is None:
        return []
    glyph = font["glyf"][glyph_name]
    coordinates, end_points, flags = glyph.getCoordinates(font["glyf"])
    contours = []
    start = 0
    for end in end_points:
        contours.append({
            index: coordinates[start + index]
            for index in range(end - start + 1)
            if flags[start + index] & 1
        })
        start = end + 1
    units_per_em = font["head"].unitsPerEm
    advance_width = font["hmtx"].metrics[glyph_name][0]
    scale = FONT_SIZE_PT / units_per_em
    mapped = lambda point: ((point[0] - advance_width * 0.5) * scale, -point[1] * scale + FONT_SIZE_PT * BASELINE_TO_NODE_EM)
    anchors = []
    for spec in specs:
        mode, contour_index = spec[:2]
        points = contours[contour_index]
        if mode == "point" and spec[2] in points:
            anchors.append(mapped(points[spec[2]]))
        elif mode == "midpoint" and spec[2] in points and spec[3] in points:
            left = mapped(points[spec[2]])
            right = mapped(points[spec[3]])
            anchors.append(((left[0] + right[0]) * 0.5, (left[1] + right[1]) * 0.5))
    return anchors


def inset_anchors(anchors, geometry, inset_mode: str, margin: float | None = None):
    min_x, min_y, max_x, max_y = geometry.bounds
    center = ((min_x + max_x) * 0.5, (min_y + max_y) * 0.5)
    height = max_y - min_y
    offset = (
        float(inset_mode.removeprefix("margin")) * (margin or 0.0)
        if inset_mode.startswith("margin")
        else height * 0.22
    )
    center_band = (max_x - min_x) * 0.12
    inset = []
    for x, y in anchors:
        if inset_mode == "axis":
            dx = offset if x < center[0] - center_band else (-offset if x > center[0] + center_band else 0.0)
            dy = offset if y < center[1] else -offset
            inset.append((x + dx, y + dy))
        else:
            dx = center[0] - x
            dy = center[1] - y
            length = math.hypot(dx, dy)
            inset.append((x, y) if length <= 1e-9 else (x + dx / length * offset, y + dy / length * offset))
    return inset


def vertex_reinforced_geometry(
    geometry,
    margin: float,
    minimum_turn_deg: float,
    convex_only: bool,
    inset_mode: str,
):
    expanded = geometry if margin == 0 else geometry.buffer(margin, quad_segs=16, join_style=1)
    if margin == 0:
        return expanded
    anchors = inset_anchors(sharp_vertices(geometry, minimum_turn_deg, convex_only), geometry, inset_mode)
    circles = [Point(anchor).buffer(2.0 * margin, quad_segs=16) for anchor in anchors]
    return unary_union([expanded, *circles]) if circles else expanded


def legacy_reinforced_geometry(font, character, geometry, margin: float, radius_mode: str):
    expanded = geometry if margin == 0 else geometry.buffer(margin, quad_segs=16, join_style=1)
    anchors = inset_anchors(legacy_anchor_points(font, character), geometry, "axis")
    height = geometry.bounds[3] - geometry.bounds[1]
    if radius_mode == "2margin":
        radius = 2.0 * margin
    elif radius_mode.startswith("times"):
        radius = float(radius_mode.removeprefix("times")) * margin
    elif radius_mode.startswith("fixed"):
        radius = float(radius_mode.removeprefix("fixed")) * height
    else:
        ratio = float(radius_mode.removeprefix("plus"))
        radius = ratio * height + margin
    circles = [Point(anchor).buffer(radius, quad_segs=16) for anchor in anchors if radius > 0]
    return unary_union([expanded, *circles]) if circles else expanded


def automatic_hull_anchor_points(font: TTFont, character: str, geometry, mode: str):
    hull = geometry.convex_hull
    hull_vertices = list(hull.exterior.coords)[:-1]
    if mode == "geometry":
        return hull_vertices
    glyph_name = font.getBestCmap().get(ord(character))
    if glyph_name is None:
        return []
    glyph = font["glyf"][glyph_name]
    coordinates, end_points, flags = glyph.getCoordinates(font["glyf"])
    if not end_points:
        return []
    contours = []
    start = 0
    for end in end_points:
        contours.append(list(range(start, end + 1)))
        start = end + 1
    units_per_em = font["head"].unitsPerEm
    advance_width = font["hmtx"].metrics[glyph_name][0]
    scale = FONT_SIZE_PT / units_per_em
    mapped = lambda point: ((point[0] - advance_width * 0.5) * scale, -point[1] * scale + FONT_SIZE_PT * BASELINE_TO_NODE_EM)
    anchors = []
    for contour in contours:
        for local_index, global_index in enumerate(contour):
            previous_index = contour[local_index - 1]
            following_index = contour[(local_index + 1) % len(contour)]
            if not (
                flags[global_index] & 1
                and flags[previous_index] & 1
                and flags[following_index] & 1
            ):
                continue
            point = mapped(coordinates[global_index])
            if mode == "vertices":
                exposed = any(math.dist(point, hull_vertex) <= 1e-5 for hull_vertex in hull_vertices)
            else:
                exposed = hull.boundary.distance(Point(point)) <= 1e-5
            if exposed:
                anchors.append(point)
    return anchors


def automatic_hull_reinforced_geometry(
    font,
    character,
    geometry,
    margin: float,
    mode: str,
    inset_mode: str,
    radius_factor: float = 2.0,
    feature_margin_cap_em: float | None = None,
):
    expanded = geometry if margin == 0 else geometry.buffer(margin, quad_segs=16, join_style=1)
    feature_margin = (
        min(margin, feature_margin_cap_em * FONT_SIZE_PT)
        if feature_margin_cap_em is not None
        else margin
    )
    anchors = inset_anchors(
        automatic_hull_anchor_points(font, character, geometry, mode),
        geometry,
        inset_mode,
        feature_margin,
    )
    circles = [
        Point(anchor).buffer(radius_factor * feature_margin, quad_segs=16)
        for anchor in anchors
        if feature_margin > 0
    ]
    return unary_union([expanded, *circles]) if circles else expanded


def automatic_label_hull_reinforced_geometry(
    font,
    text: str,
    margin: float,
    scope: str,
    mode: str,
    inset_mode: str,
    radius_factor: float,
    anchor_index: int | None = None,
    feature_margin_cap_em: float | None = None,
):
    parts = []
    for character, geometry, offset in label_geometry_parts(font, text, anchor_index):
        reinforced = (
            automatic_hull_reinforced_geometry(
                font,
                character,
                geometry,
                margin,
                mode,
                inset_mode,
                radius_factor,
                feature_margin_cap_em,
            )
            if scope == "all" or (character.isascii() and character.isupper())
            else (geometry if margin == 0 else geometry.buffer(margin, quad_segs=16, join_style=1))
        )
        parts.append(affinity.translate(reinforced, xoff=offset))
    return unary_union(parts) if parts else None


def axial_contact_projection(geometry, margin: float, angle_deg: float, half_sector_deg: float):
    min_x, min_y, max_x, max_y = geometry.bounds
    radians = math.radians(angle_deg)
    direction = (math.cos(radians), math.sin(radians))
    contacts = [
        (0.0, (max_x + margin, 0.0)),
        (90.0, (0.0, max_y + margin)),
        (180.0, (min_x - margin, 0.0)),
        (270.0, (0.0, min_y - margin)),
    ]
    candidates = []
    for axis_deg, point in contacts:
        difference = abs((angle_deg - axis_deg + 180.0) % 360.0 - 180.0)
        if difference < half_sector_deg:
            candidates.append(point[0] * direction[0] + point[1] * direction[1])
    return max([0.0, *candidates])


def error_summary(errors: list[float]) -> dict:
    absolute = np.abs(np.asarray(errors, dtype=float))
    signed = np.asarray(errors, dtype=float)
    return {
        "count": len(errors),
        "maePt": float(absolute.mean()),
        "p50Pt": float(np.quantile(absolute, 0.50)),
        "p95Pt": float(np.quantile(absolute, 0.95)),
        "maxPt": float(absolute.max()),
        "biasPt": float(signed.mean()),
    }


def main() -> None:
    global FONT_SIZE_PT
    args = parse_args()
    FONT_SIZE_PT = args.font_size
    source = json.loads(Path(args.measurements).read_text(encoding="utf-8"))
    font = TTFont(
        args.font_path,
        fontNumber=(args.font_number if args.font_number >= 0 else -1),
    )
    cmap = font.getBestCmap()
    targets = [
        entry
        for entry in source["measurements"]
        if entry["font"] == args.font_name
        and entry["face"] == args.face
        and entry["size"] == FONT_SIZE_PT
        and entry["lineWidth"] == args.line_width
        and (args.focused or len(entry["glyph"]) == 1)
        and all(ord(character) in cmap for character in entry["glyph"])
        and (
            args.max_angular_deflection is None
            or (
                0.0 <= entry["retreat"] <= entry["bondLength"]
                and abs(entry.get("angularDeflectionDeg", 0.0)) <= args.max_angular_deflection
            )
        )
    ]
    by_glyph: dict[tuple[str, int | None], list[dict]] = defaultdict(list)
    for entry in targets:
        by_glyph[(entry["glyph"], entry.get("anchorIndex"))].append(entry)

    variants = [
        ("ray-strip", "outline", "round", 1, margin_scale)
        for margin_scale in [1.0, 1.05, 1.1, 1.15]
    ] + [
        ("ray-strip", f"axial-{half_sector}", "round", 1, 1.0)
        for half_sector in [9.0, 9.5, 10.0, 10.5]
    ] + [
        ("ray-strip", "axial-9.5", f"mitre-{mitre_limit}", 2, 1.0)
        for mitre_limit in [1.0, 1.5, 2.0, 4.0, 10.0]
    ] + [
        ("ray-strip", "axial-9.5", "bevel", 3, 1.0)
    ] + [
        ("ray-strip", f"axial-auto-hull-{scope}-{mode}-{inset}-times{factor}-9.5", "round", 1, 1.0)
        for scope in ["upper", "all"]
        for mode in ["vertices", "boundary"]
        for inset in ["axis", "vector"]
        for factor in [1.4, 1.5, 1.6, 1.75, 2.0]
    ] + [
        (
            "ray-strip",
            f"axial-auto-hull-{scope}-vertices-margin{inset_factor}-times{radius_factor}-9.5",
            "round",
            1,
            1.0,
        )
        for scope in ["upper", "all"]
        for inset_factor in [0.5, 0.75, 1.0, 1.25]
        for radius_factor in [1.5, 1.75, 2.0]
    ] + [
        ("ray-strip", f"axial-legacy-times{factor}-9.5", "round", 1, 1.0)
        for factor in [1.1, 1.25, 1.4, 1.5, 1.6, 1.75, 2.0]
    ] + [
        ("ray-strip", f"axial-legacy-plus{ratio}-9.5", "round", 1, 1.0)
        for ratio in [0.05, 0.075, 0.1, 0.125, 0.15, 0.175, 0.2, 0.22]
    ] + [
        ("ray-strip", "legacy-2margin", "round", 1, 1.0)
    ] + [
        ("ray-strip", f"auto-hull-{mode}-{inset}", "round", 1, 1.0)
        for mode in ["vertices", "boundary"]
        for inset in ["axis", "vector"]
    ]
    if args.focused:
        variants = [
            ("ray-strip", "axial-9.5", "round", 1, 1.0),
            *[
                (
                    "ray-strip",
                    f"axial-auto-hull-{scope}-vertices-vector-times{factor}-9.5",
                    "round",
                    1,
                    1.0,
                )
                for scope in ["upper", "all"]
                for factor in [1.4, 1.5, 1.6, 1.75]
            ],
            *[
                (
                    "ray-strip",
                    f"axial-auto-hull-{scope}-vertices-margin{inset_factor}-times{radius_factor}-9.5",
                    "round",
                    1,
                    1.0,
                )
                for scope in ["upper", "all"]
                for inset_factor in [0.5, 0.75, 1.0, 1.25]
                for radius_factor in [1.5, 1.75, 2.0]
            ],
        ]
    if args.rule_only:
        variants = [
            ("ray-strip", "axial-9.5", "round", 1, 1.0),
            (
                "ray-strip",
                "axial-auto-hull-all-vertices-margin0.5-times1.5-9.5",
                "round",
                1,
                1.0,
            ),
            (
                "ray-strip",
                "axial-auto-hull-all-geometry-margin0.5-times1.5-9.5",
                "round",
                1,
                1.0,
            ),
            *[
                (
                    "ray-strip",
                    f"axial-auto-hull-all-geometry-margin0.5-times1.5cap{cap}-9.5",
                    "round",
                    1,
                    1.0,
                )
                for cap in [0.2, 0.25, 0.3]
            ],
        ]
    errors = {variant: [] for variant in variants}
    errors_by_glyph = {variant: defaultdict(list) for variant in variants}
    errors_by_margin = {variant: defaultdict(list) for variant in variants}
    errors_by_anchor = {variant: defaultdict(list) for variant in variants}
    observations = []
    for (character, anchor_index), entries in by_glyph.items():
        outline_geometry = label_geometry(font, character, anchor_index)
        geometries = {
            "outline": outline_geometry,
            "bbox-diamond": glyph_bbox_safety_shape(outline_geometry, 1.0),
            "bbox-ellipse": glyph_bbox_safety_shape(outline_geometry, 2.0),
            "bbox-superellipse-4": glyph_bbox_safety_shape(outline_geometry, 4.0),
            "component-convex-hull": component_convex_hull(outline_geometry),
        }
        expanded_cache = {}
        for entry in entries:
            observations.append(entry)
            for variant in variants:
                measurement, shape, _join_name, join_style, margin_scale = variant
                cache_key = (shape, join_style, margin_scale, entry["marginWidth"])
                expanded = expanded_cache.get(cache_key)
                if expanded is None:
                    distance = entry["marginWidth"] * margin_scale
                    if shape.startswith("axial-auto-hull-"):
                        _, _, _, scope, mode, inset, factor_text, _sector = shape.split("-")
                        factor_payload = factor_text.removeprefix("times")
                        if "cap" in factor_payload:
                            factor_value, cap_value = factor_payload.split("cap", 1)
                            feature_margin_cap_em = float(cap_value)
                        else:
                            factor_value = factor_payload
                            feature_margin_cap_em = None
                        expanded = automatic_label_hull_reinforced_geometry(
                                font,
                                character,
                                distance,
                                scope,
                                mode,
                                inset,
                                float(factor_value),
                                anchor_index,
                                feature_margin_cap_em,
                            )
                    elif shape.startswith("axial-legacy-"):
                        radius_mode = shape.split("-")[2]
                        expanded = (
                            legacy_reinforced_geometry(
                                font,
                                character,
                                outline_geometry,
                                distance,
                                radius_mode,
                            )
                            if character.isascii() and character.isupper()
                            else (
                                outline_geometry
                                if distance == 0
                                else outline_geometry.buffer(distance, quad_segs=16, join_style=1)
                            )
                        )
                    elif shape.startswith("axial-"):
                        base_geometry = outline_geometry
                        expanded = base_geometry if distance == 0 else base_geometry.buffer(
                            distance,
                            quad_segs=16,
                            join_style=join_style,
                            mitre_limit=(
                                float(_join_name.removeprefix("mitre-"))
                                if _join_name.startswith("mitre-")
                                else 4.0
                            ),
                        )
                    elif shape.startswith("auto-hull-"):
                        _, _, mode, inset = shape.split("-")
                        expanded = automatic_hull_reinforced_geometry(
                            font,
                            character,
                            outline_geometry,
                            distance,
                            mode,
                            inset,
                        )
                    elif shape.startswith("legacy-"):
                        expanded = legacy_reinforced_geometry(
                            font,
                            character,
                            outline_geometry,
                            distance,
                            shape.removeprefix("legacy-"),
                        )
                    elif shape.startswith("vertices-"):
                        _, scope, turn, inset = shape.split("-")
                        expanded = vertex_reinforced_geometry(
                            outline_geometry,
                            distance,
                            float(turn),
                            scope == "convex",
                            inset,
                        )
                    else:
                        base_geometry = geometries[shape]
                        expanded = base_geometry if distance == 0 else base_geometry.buffer(
                            distance,
                            quad_segs=16,
                            join_style=join_style,
                            mitre_limit=4.0,
                        )
                    expanded_cache[cache_key] = expanded
                predicted = retreat_for_geometry(expanded, entry["angleDeg"], entry["lineWidth"])
                if shape.startswith("axial-"):
                    predicted = max(
                        predicted,
                        axial_contact_projection(
                            outline_geometry,
                            entry["marginWidth"],
                            entry["angleDeg"],
                            float(shape.split("-")[-1]),
                        ),
                    )
                error = predicted - entry["retreat"]
                errors[variant].append(error)
                errors_by_glyph[variant][character].append(error)
                errors_by_margin[variant][entry["marginWidth"]].append(error)
                errors_by_anchor[variant][entry.get("anchorPosition", "none")].append(error)

    ranked = sorted(
        (
            {
                "measurement": measurement,
                "join": join_name,
                "shape": shape,
                "joinStyle": join_style,
                "marginScale": margin_scale,
                **error_summary(errors[(measurement, shape, join_name, join_style, margin_scale)]),
            }
            for measurement, shape, join_name, join_style, margin_scale in variants
        ),
        key=lambda result: (result["maePt"], result["p95Pt"]),
    )
    winner = ranked[0]
    winner_key = (
        winner["measurement"],
        winner["shape"],
        winner["join"],
        winner["joinStyle"],
        winner["marginScale"],
    )
    worst_glyphs = sorted(
        ({"glyph": glyph, **error_summary(values)} for glyph, values in errors_by_glyph[winner_key].items()),
        key=lambda result: result["maePt"],
        reverse=True,
    )
    worst_cases = sorted(
        (
            {
                "glyph": observation["glyph"],
                "marginWidth": observation["marginWidth"],
                "angleDeg": observation["angleDeg"],
                "retreat": observation["retreat"],
                "predictedRetreat": observation["retreat"] + error,
                "error": error,
            }
            for observation, error in zip(observations, errors[winner_key])
        ),
        key=lambda result: abs(result["error"]),
        reverse=True,
    )
    result = {
        "schema": "chemsema.chemdraw-retreat-vector-rule-analysis.v1",
        "measurementCount": len(targets),
        "glyphCount": len(by_glyph),
        "font": args.font_path,
        "fontName": args.font_name,
        "fontSizePt": FONT_SIZE_PT,
        "face": args.face,
        "lineWidth": args.line_width,
        "baselineToNodeEm": BASELINE_TO_NODE_EM,
        "ranked": ranked,
        "winnerWorstGlyphs": worst_glyphs[:20],
        "winnerWorstCases": worst_cases[:100],
        "candidateByGlyph": {
            shape: {
                glyph: error_summary(values)
                for glyph, values in errors_by_glyph[variant].items()
            }
            for variant in variants
            for shape in ["|".join((variant[1], variant[2]))]
            if variant[1].startswith("axial-auto-hull-")
            or variant[1].startswith("axial-legacy-")
            or (variant[1] == "axial-9.5" and variant[2] == "round")
        },
        "candidateByMargin": {
            shape: {
                str(margin): error_summary(values)
                for margin, values in errors_by_margin[variant].items()
            }
            for variant in variants
            for shape in ["|".join((variant[1], variant[2]))]
            if variant[1].startswith("axial-legacy-")
            or variant[1].startswith("axial-auto-hull-")
            or (variant[1] == "axial-9.5" and variant[2] == "round")
        },
        "candidateByAnchor": {
            shape: {
                anchor: error_summary(values)
                for anchor, values in errors_by_anchor[variant].items()
            }
            for variant in variants
            for shape in ["|".join((variant[1], variant[2]))]
            if variant[1].startswith("axial-auto-hull-")
            or (variant[1] == "axial-9.5" and variant[2] == "round")
        },
    }
    Path(args.output).write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"output": str(Path(args.output).resolve()), "best": ranked[:12]}, indent=2))


if __name__ == "__main__":
    main()
