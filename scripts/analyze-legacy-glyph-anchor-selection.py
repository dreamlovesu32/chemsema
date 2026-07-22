#!/usr/bin/env python3

from __future__ import annotations

import json
import math
import runpy
from pathlib import Path

from fontTools.ttLib import TTFont
from shapely.geometry import Point

FONT_PATH = Path(r"C:\Windows\Fonts\arial.ttf")


def vector_angle(left, right):
    left_length = math.hypot(*left)
    right_length = math.hypot(*right)
    if left_length <= 1e-9 or right_length <= 1e-9:
        return 0.0
    cosine = max(-1.0, min(1.0, (
        left[0] * right[0] + left[1] * right[1]
    ) / (left_length * right_length)))
    return math.degrees(math.acos(cosine))


def main() -> None:
    # The source file has hyphens, so keep this analyzer self-contained when
    # invoked directly by loading the anchor table from its Python source.
    source = (Path(__file__).with_name("analyze-chemdraw-retreat-vector-rules.py"))
    namespace = runpy.run_path(str(source))
    anchor_map = namespace["LEGACY_ANCHOR_MAP"]

    font = TTFont(str(FONT_PATH))
    report = {}
    reconstruction = {}
    for character in "ABCDEFGHIJKLMNOPQRSTUVWXYZ":
        glyph_name = font.getBestCmap()[ord(character)]
        glyph = font["glyf"][glyph_name]
        coordinates, end_points, flags = glyph.getCoordinates(font["glyf"])
        contours = []
        start = 0
        for end in end_points:
            contours.append(list(range(start, end + 1)))
            start = end + 1

        selected_points = {
            (spec[1], spec[2])
            for spec in anchor_map.get(character, [])
            if spec[0] == "point"
        }
        midpoint_points = {
            (spec[1], index)
            for spec in anchor_map.get(character, [])
            if spec[0] == "midpoint"
            for index in spec[2:4]
        }
        points = []
        for contour_index, contour in enumerate(contours):
            for local_index, global_index in enumerate(contour):
                if not flags[global_index] & 1:
                    continue
                previous_index = contour[local_index - 1]
                following_index = contour[(local_index + 1) % len(contour)]
                current = coordinates[global_index]
                previous = coordinates[previous_index]
                following = coordinates[following_index]
                incoming = (current[0] - previous[0], current[1] - previous[1])
                outgoing = (following[0] - current[0], following[1] - current[1])
                cross = incoming[0] * outgoing[1] - incoming[1] * outgoing[0]
                points.append({
                    "contour": contour_index,
                    "index": local_index,
                    "x": int(current[0]),
                    "y": int(current[1]),
                    "turnDeg": round(vector_angle(incoming, outgoing), 3),
                    "cross": int(cross),
                    "selected": (contour_index, local_index) in selected_points,
                    "midpointMember": (contour_index, local_index) in midpoint_points,
                    "previousOnCurve": bool(flags[previous_index] & 1),
                    "nextOnCurve": bool(flags[following_index] & 1),
                })
        report[character] = points

        outline = namespace["glyph_geometry"](font, character)
        hull_boundary = outline.convex_hull.boundary
        units_per_em = font["head"].unitsPerEm
        advance_width = font["hmtx"].metrics[glyph_name][0]
        scale = namespace["FONT_SIZE_PT"] / units_per_em
        baseline = namespace["FONT_SIZE_PT"] * namespace["BASELINE_TO_NODE_EM"]
        mapped = lambda point: ((point[0] - advance_width * 0.5) * scale, -point[1] * scale + baseline)
        contour = contours[0]
        glyph_height_units = max(coordinates[index][1] for index in contour) - min(coordinates[index][1] for index in contour)
        automatic_candidates = []
        for local_index, global_index in enumerate(contour):
            previous_index = contour[local_index - 1]
            following_index = contour[(local_index + 1) % len(contour)]
            if not (flags[global_index] & 1 and flags[previous_index] & 1 and flags[following_index] & 1):
                continue
            current = coordinates[global_index]
            previous = coordinates[previous_index]
            following = coordinates[following_index]
            incoming = (current[0] - previous[0], current[1] - previous[1])
            outgoing = (following[0] - current[0], following[1] - current[1])
            cross = incoming[0] * outgoing[1] - incoming[1] * outgoing[0]
            mapped_point = mapped(current)
            if cross < 0 and hull_boundary.distance(Point(mapped_point)) <= 1e-5:
                automatic_candidates.append((local_index, current, mapped_point))

        candidate_by_index = {entry[0]: entry for entry in automatic_candidates}
        consumed = set()
        automatic_anchors = []
        for local_index, current, mapped_point in automatic_candidates:
            if local_index in consumed:
                continue
            next_index = (local_index + 1) % len(contour)
            neighbor = candidate_by_index.get(next_index)
            if neighbor is not None:
                distance = math.hypot(current[0] - neighbor[1][0], current[1] - neighbor[1][1])
                if distance <= glyph_height_units * 0.16:
                    automatic_anchors.append((
                        (mapped_point[0] + neighbor[2][0]) * 0.5,
                        (mapped_point[1] + neighbor[2][1]) * 0.5,
                    ))
                    consumed.update([local_index, next_index])
                    continue
            automatic_anchors.append(mapped_point)
            consumed.add(local_index)

        expected = namespace["legacy_anchor_points"](font, character)
        remaining = list(automatic_anchors)
        distances = []
        for expected_point in expected:
            if not remaining:
                distances.append(None)
                continue
            nearest_index = min(
                range(len(remaining)),
                key=lambda index: math.dist(expected_point, remaining[index]),
            )
            distances.append(math.dist(expected_point, remaining.pop(nearest_index)))
        reconstruction[character] = {
            "expectedCount": len(expected),
            "automaticCount": len(automatic_anchors),
            "matchedDistancesPt": distances,
            "extraAutomaticCount": len(remaining),
            "candidateIndices": [entry[0] for entry in automatic_candidates],
        }

    selected = [point for points in report.values() for point in points if point["selected"]]
    midpoint = [point for points in report.values() for point in points if point["midpointMember"]]
    unselected = [point for points in report.values() for point in points if not point["selected"] and not point["midpointMember"]]
    summary = {
        "selectedCount": len(selected),
        "selectedTurns": sorted({point["turnDeg"] for point in selected}),
        "midpointMemberCount": len(midpoint),
        "midpointTurns": sorted({point["turnDeg"] for point in midpoint}),
        "unselectedSharp": sorted(
            [point for point in unselected if point["turnDeg"] >= 20],
            key=lambda point: point["turnDeg"],
            reverse=True,
        ),
        "exactReconstructionGlyphs": [
            character
            for character, result in reconstruction.items()
            if result["expectedCount"] == result["automaticCount"]
            and all(distance is not None and distance <= 1e-6 for distance in result["matchedDistancesPt"])
        ],
        "reconstructionMismatches": {
            character: result
            for character, result in reconstruction.items()
            if result["expectedCount"] != result["automaticCount"]
            or any(distance is None or distance > 1e-6 for distance in result["matchedDistancesPt"])
        },
    }
    output = Path("tmp/chemdraw-label-retreat-comprehensive/legacy-anchor-selection.json")
    output.write_text(json.dumps({"summary": summary, "reconstruction": reconstruction, "glyphs": report}, indent=2), encoding="utf-8")
    print(json.dumps({
        "output": str(output.resolve()),
        "exactReconstructionGlyphs": summary["exactReconstructionGlyphs"],
        "reconstructionMismatches": summary["reconstructionMismatches"],
    }, indent=2))


if __name__ == "__main__":
    main()
