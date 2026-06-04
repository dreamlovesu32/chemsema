#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import re
import xml.etree.ElementTree as ET
from collections import defaultdict
from pathlib import Path


SVG_NS = {"svg": "http://www.w3.org/2000/svg"}

CURRENT_CODE_CENTERS = {
    "petal-nehkxz": [[0.31, 0.30], [0.69, 0.30], [0.69, 0.70], [0.31, 0.70]],
    "petal-a": [[0.50, 0.33], [0.31, 0.70], [0.69, 0.70]],
    "petal-v": [[0.31, 0.30], [0.69, 0.30], [0.50, 0.67]],
    "petal-i": [[0.50, 0.30], [0.50, 0.70]],
    "petal-j": [[0.50, 0.30]],
    "petal-l": [[0.31, 0.30], [0.69, 0.70], [0.31, 0.70]],
    "petal-f": [[0.31, 0.30], [0.69, 0.30], [0.31, 0.70]],
    "petal-r": [[0.31, 0.30], [0.69, 0.70], [0.31, 0.70]],
    "petal-t": [[0.31, 0.30], [0.50, 0.30], [0.69, 0.30]],
    "petal-u": [[0.31, 0.30], [0.69, 0.30]],
    "petal-y": [[0.31, 0.30], [0.69, 0.30], [0.50, 0.70]],
    "petal-bdp": [[0.31, 0.30], [0.31, 0.70]],
    "petal-q": [[0.72, 0.72]],
}

COMPARE_TOLERANCE = 0.02


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Extract normalized glyph expansion rules from an oracle SVG."
    )
    parser.add_argument(
        "--svg",
        default="tmp/glyph-expansion-scheme-current-centered.svg",
        help="Path to the oracle SVG.",
    )
    parser.add_argument(
        "--json-out",
        default="tmp/glyph-expansion-oracle-rules.json",
        help="Where to write the extracted rules JSON.",
    )
    return parser.parse_args()


def parse_path_bounds(path_d: str) -> dict[str, float]:
    numbers = [float(value) for value in re.findall(r"-?\d+(?:\.\d+)?", path_d)]
    xs = numbers[0::2]
    ys = numbers[1::2]
    return {
        "x1": min(xs),
        "y1": min(ys),
        "x2": max(xs),
        "y2": max(ys),
    }


def normalize_point(value: float) -> float:
    return round(value + 0.0, 4)


def extract_rows(svg_path: Path) -> list[dict]:
    root = ET.parse(svg_path).getroot()
    rows: list[dict] = []
    for group in root.findall("svg:g", SVG_NS):
        texts = group.findall("svg:text", SVG_NS)
        if len(texts) < 2:
            continue
        if texts[0].get("class") != "cellLabel" or texts[1].get("class") != "small":
            continue

        letter = "".join(texts[0].itertext()).strip()
        shape = "".join(texts[1].itertext()).strip()
        base_path = None
        circles = []

        for child in group:
            tag = child.tag.rsplit("}", 1)[-1]
            if tag == "path" and child.get("fill") == "#d6d1c4":
                base_path = child.get("d")
            elif tag == "circle" and child.get("fill") == "#f59e0b":
                circles.append(
                    {
                        "cx": float(child.get("cx")),
                        "cy": float(child.get("cy")),
                        "r": float(child.get("r")),
                    }
                )

        if base_path is None:
            raise ValueError(f"missing base polygon for {letter}")

        box = parse_path_bounds(base_path)
        width = box["x2"] - box["x1"]
        height = box["y2"] - box["y1"]
        normalized_centers = [
            [
                normalize_point((circle["cx"] - box["x1"]) / width),
                normalize_point((circle["cy"] - box["y1"]) / height),
            ]
            for circle in circles
        ]
        normalized_radius = normalize_point(circles[0]["r"] / height) if circles else None
        rows.append(
            {
                "letter": letter,
                "shape": shape,
                "box": box,
                "width": width,
                "height": height,
                "circle_count": len(circles),
                "normalized_centers": normalized_centers,
                "normalized_radius": normalized_radius,
            }
        )
    rows.sort(key=lambda row: row["letter"])
    return rows


def build_shape_rules(rows: list[dict]) -> dict[str, dict]:
    grouped: dict[str, list[dict]] = defaultdict(list)
    for row in rows:
        grouped[row["shape"]].append(row)

    shape_rules: dict[str, dict] = {}
    for shape, members in sorted(grouped.items()):
        center_patterns = {tuple(tuple(point) for point in row["normalized_centers"]) for row in members}
        radius_patterns = {row["normalized_radius"] for row in members}
        width_patterns = {normalize_point(row["width"]) for row in members}
        height_patterns = {normalize_point(row["height"]) for row in members}
        if len(center_patterns) != 1:
            raise ValueError(f"{shape} has inconsistent center patterns: {center_patterns}")
        if len(radius_patterns) > 1:
            raise ValueError(f"{shape} has inconsistent radius patterns: {radius_patterns}")
        shape_rules[shape] = {
            "letters": [row["letter"] for row in members],
            "normalized_centers": [list(point) for point in next(iter(center_patterns))],
            "normalized_radius": next(iter(radius_patterns)),
            "widths": sorted(width_patterns),
            "heights": sorted(height_patterns),
        }
    return shape_rules


def compare_with_current_code(shape_rules: dict[str, dict]) -> dict[str, dict]:
    def canonicalize(points: list[list[float]]) -> list[list[float]]:
        return sorted([[float(x), float(y)] for x, y in points], key=lambda point: (point[0], point[1]))

    def points_match(lhs: list[list[float]], rhs: list[list[float]]) -> bool:
        if len(lhs) != len(rhs):
            return False
        lhs_sorted = canonicalize(lhs)
        rhs_sorted = canonicalize(rhs)
        return all(
            abs(left[0] - right[0]) <= COMPARE_TOLERANCE
            and abs(left[1] - right[1]) <= COMPARE_TOLERANCE
            for left, right in zip(lhs_sorted, rhs_sorted)
        )

    comparisons: dict[str, dict] = {}
    for shape, oracle in sorted(shape_rules.items()):
        current = CURRENT_CODE_CENTERS.get(shape)
        if current is None:
            continue
        current_rounded = [[normalize_point(x), normalize_point(y)] for x, y in current]
        comparisons[shape] = {
            "matches_current_code": points_match(current_rounded, oracle["normalized_centers"]),
            "oracle_centers": oracle["normalized_centers"],
            "current_code_centers": current_rounded,
        }
    return comparisons


def main() -> int:
    args = parse_args()
    svg_path = Path(args.svg)
    json_path = Path(args.json_out)

    rows = extract_rows(svg_path)
    shape_rules = build_shape_rules(rows)
    comparisons = compare_with_current_code(shape_rules)

    output = {
        "source_svg": str(svg_path),
        "derived_global_rules": {
            "center_formula": "center = (box.x1 + width * nx, box.y1 + height * ny)",
            "radius_formula": "radius = height * normalized_radius",
        },
        "rows": rows,
        "shape_rules": shape_rules,
        "compare_with_current_code": comparisons,
    }

    json_path.parent.mkdir(parents=True, exist_ok=True)
    json_path.write_text(json.dumps(output, ensure_ascii=False, indent=2), encoding="utf-8")

    print(f"wrote {json_path}")
    for shape, info in comparisons.items():
        status = "match" if info["matches_current_code"] else "DIFF"
        print(f"{status:>5} {shape}: oracle={info['oracle_centers']} current={info['current_code_centers']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
