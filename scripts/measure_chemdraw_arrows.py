#!/usr/bin/env python3
"""Measure ChemDraw arrow references and emit SVG diagnostics.

Preferred inputs are ChemDraw CDXML or exported SVG/PDF-derived SVG. Raster input
is supported for comparison, but it is not authoritative because antialiasing and
export scale change the measured contour.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any
from xml.etree import ElementTree as ET

import cv2
import numpy as np


ARROW_ATTRS = {
    "ArrowType",
    "HeadSize",
    "ArrowHeadType",
    "ArrowheadType",
    "HeadCenterSize",
    "ArrowheadCenterSize",
    "HeadWidth",
    "ArrowheadWidth",
    "ArrowShaftSpacing",
    "ArrowEquilibriumRatio",
    "ArrowHeadHead",
    "ArrowheadHead",
    "ArrowHeadTail",
    "ArrowheadTail",
    "Dipole",
    "NoGo",
    "BoundingBox",
    "Head3D",
    "Tail3D",
    "Center3D",
    "MajorAxisEnd3D",
    "MinorAxisEnd3D",
    "Head",
    "Tail",
    "LineType",
    "GraphicType",
    "FillType",
}


def strip_namespace(tag: str) -> str:
    return tag.rsplit("}", 1)[-1] if "}" in tag else tag


def parse_float_list(value: str) -> list[float]:
    return [float(item) for item in re.split(r"[\s,]+", value.strip()) if item]


def parse_xml(path: Path) -> dict[str, Any]:
    root = ET.parse(path).getroot()
    arrows: list[dict[str, Any]] = []
    svg_shapes: list[dict[str, Any]] = []
    root_tag = strip_namespace(root.tag).lower()
    root_attrs = dict(root.attrib)

    for element in root.iter():
        tag = strip_namespace(element.tag)
        attrs = dict(element.attrib)
        if tag.lower() in {"arrow", "graphic", "curve"} or any(key in attrs for key in ARROW_ATTRS):
            arrow_attrs = {key: attrs[key] for key in sorted(attrs) if key in ARROW_ATTRS}
            if arrow_attrs or tag.lower() == "arrow":
                arrows.append({"tag": tag, "attrs": arrow_attrs})

        if tag.lower() in {"path", "polygon", "polyline", "line"}:
            record = {"tag": tag, "attrs": attrs}
            if tag.lower() in {"polygon", "polyline"} and "points" in attrs:
                record["points"] = point_pairs(parse_float_list(attrs["points"]))
            if tag.lower() == "line":
                record["points"] = [
                    [float(attrs.get("x1", 0)), float(attrs.get("y1", 0))],
                    [float(attrs.get("x2", 0)), float(attrs.get("y2", 0))],
                ]
            svg_shapes.append(record)

    return {
        "kind": "xml",
        "source": str(path),
        "root": {"tag": root_tag, "attrs": root_attrs},
        "arrows": arrows,
        "svgShapes": svg_shapes,
    }


def point_pairs(values: list[float]) -> list[list[float]]:
    return [[values[index], values[index + 1]] for index in range(0, len(values) - 1, 2)]


def escape_attr(value: Any) -> str:
    return (
        str(value)
        .replace("&", "&amp;")
        .replace('"', "&quot;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
    )


def contour_to_path(contour: np.ndarray, simplify_px: float) -> tuple[str, list[list[float]]]:
    epsilon = max(0.0, simplify_px)
    if epsilon > 0:
        contour = cv2.approxPolyDP(contour, epsilon, True)
    points = contour.reshape(-1, 2).astype(float)
    if len(points) == 0:
        return "", []
    commands = [f"M {points[0][0]:.3f} {points[0][1]:.3f}"]
    commands.extend(f"L {point[0]:.3f} {point[1]:.3f}" for point in points[1:])
    commands.append("Z")
    return " ".join(commands), [[float(x), float(y)] for x, y in points]


def raster_components(path: Path, threshold: int, simplify_px: float) -> dict[str, Any]:
    image = cv2.imread(str(path), cv2.IMREAD_COLOR)
    if image is None:
        raise ValueError(f"Unable to read image: {path}")
    gray = cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)
    mask = (gray <= threshold).astype(np.uint8) * 255
    contours, _ = cv2.findContours(mask, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)
    components: list[dict[str, Any]] = []
    for index, contour in enumerate(sorted(contours, key=cv2.contourArea, reverse=True), start=1):
        area = float(cv2.contourArea(contour))
        if area < 2:
            continue
        x, y, width, height = cv2.boundingRect(contour)
        path_data, points = contour_to_path(contour, simplify_px)
        moments = cv2.moments(contour)
        centroid = None
        if abs(moments["m00"]) > 1e-9:
            centroid = [moments["m10"] / moments["m00"], moments["m01"] / moments["m00"]]
        components.append(
            {
                "id": f"component_{index}",
                "areaPx": area,
                "bboxPx": [x, y, width, height],
                "centroidPx": centroid,
                "path": path_data,
                "points": points,
            }
        )
    return {
        "kind": "raster",
        "source": str(path),
        "imageSizePx": [int(image.shape[1]), int(image.shape[0])],
        "threshold": threshold,
        "simplifyPx": simplify_px,
        "components": components,
    }


def render_raster_svg(result: dict[str, Any]) -> str:
    width, height = result["imageSizePx"]
    body: list[str] = [
        f'<rect x="0" y="0" width="{width}" height="{height}" fill="#fff"/>',
        '<g fill="none" stroke="#ddd" stroke-width="1">',
    ]
    grid = 85
    for x in range(0, width + 1, grid):
        body.append(f'<line x1="{x}" y1="0" x2="{x}" y2="{height}"/>')
    for y in range(0, height + 1, grid):
        body.append(f'<line x1="0" y1="{y}" x2="{width}" y2="{y}"/>')
    body.append("</g>")
    body.append('<g fill="rgba(0,0,0,0.18)" stroke="#000" stroke-width="1">')
    for component in result["components"]:
        if component["path"]:
            body.append(f'<path d="{component["path"]}" data-id="{component["id"]}"/>')
    body.append("</g>")
    body.append('<g fill="#e00000" font-family="Arial, sans-serif" font-size="12">')
    for component in result["components"]:
        cx, cy = component.get("centroidPx") or [component["bboxPx"][0], component["bboxPx"][1]]
        body.append(f'<circle cx="{cx:.3f}" cy="{cy:.3f}" r="3"/>')
        body.append(f'<text x="{cx + 5:.3f}" y="{cy - 5:.3f}">{component["id"]}</text>')
    body.append("</g>")
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}">\n  ' + "\n  ".join(body) + "\n</svg>\n"
    )


def render_xml_svg(result: dict[str, Any]) -> str:
    paths: list[str] = []
    points: list[list[float]] = []
    root_attrs = result.get("root", {}).get("attrs", {})
    for shape in result["svgShapes"]:
        attrs = shape["attrs"]
        tag = shape["tag"].lower()
        transform = attrs.get("transform")
        transform_attr = f' transform="{escape_attr(transform)}"' if transform else ""
        if tag == "path" and attrs.get("d"):
            fill = attrs.get("fill", "none")
            stroke = attrs.get("stroke", "#000")
            stroke_width = attrs.get("stroke-width", "1")
            paths.append(
                f'<path d="{escape_attr(attrs["d"])}" fill="{escape_attr(fill)}" '
                f'stroke="{escape_attr(stroke)}" stroke-width="{escape_attr(stroke_width)}"{transform_attr}/>'
            )
        elif tag in {"polygon", "polyline"} and attrs.get("points"):
            tag_name = tag
            fill = attrs.get("fill", "none" if tag == "polyline" else "rgba(0,0,0,0.12)")
            paths.append(
                f'<{tag_name} points="{escape_attr(attrs["points"])}" fill="{escape_attr(fill)}" '
                f'stroke="{escape_attr(attrs.get("stroke", "#000"))}" '
                f'stroke-width="{escape_attr(attrs.get("stroke-width", "1"))}"{transform_attr}/>'
            )
            points.extend(shape.get("points", []))
        elif tag == "line":
            p = shape.get("points", [[0, 0], [1, 0]])
            points.extend(p)
            paths.append(
                f'<line x1="{p[0][0]}" y1="{p[0][1]}" x2="{p[1][0]}" y2="{p[1][1]}" '
                f'stroke="{escape_attr(attrs.get("stroke", "#000"))}" '
                f'stroke-width="{escape_attr(attrs.get("stroke-width", "1"))}"{transform_attr}/>'
            )
    if root_attrs.get("viewBox"):
        view_box = root_attrs["viewBox"]
    elif not points:
        view_box = [0, 0, 800, 300]
    else:
        xs = [p[0] for p in points]
        ys = [p[1] for p in points]
        pad = 20
        view_box = [min(xs) - pad, min(ys) - pad, max(xs) - min(xs) + pad * 2, max(ys) - min(ys) + pad * 2]
    if isinstance(view_box, list):
        view_box = " ".join(str(value) for value in view_box)
    width = root_attrs.get("width", "1000")
    height = root_attrs.get("height", "420")
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{escape_attr(width)}" '
        f'height="{escape_attr(height)}" viewBox="{escape_attr(view_box)}">\n  '
        '<rect x="-100000" y="-100000" width="200000" height="200000" fill="#fff"/>\n  '
        + "\n  ".join(paths)
        + "\n</svg>\n"
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path)
    parser.add_argument("--json", type=Path, default=Path("tmp/arrow-measure.json"))
    parser.add_argument("--svg", type=Path, default=Path("tmp/arrow-measure.svg"))
    parser.add_argument("--threshold", type=int, default=80)
    parser.add_argument("--simplify-px", type=float, default=0.35)
    args = parser.parse_args()

    suffix = args.input.suffix.lower()
    if suffix in {".png", ".jpg", ".jpeg", ".bmp", ".tif", ".tiff"}:
        result = raster_components(args.input, args.threshold, args.simplify_px)
        svg = render_raster_svg(result)
    elif suffix in {".svg", ".cdxml", ".xml"}:
        result = parse_xml(args.input)
        svg = render_xml_svg(result)
    else:
        raise ValueError(f"Unsupported input type: {args.input.suffix}")

    args.json.parent.mkdir(parents=True, exist_ok=True)
    args.svg.parent.mkdir(parents=True, exist_ok=True)
    args.json.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    args.svg.write_text(svg, encoding="utf-8")
    print(args.json)
    print(args.svg)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"measure_chemdraw_arrows.py: {error}", file=sys.stderr)
        raise SystemExit(1)
