#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import math
import re
import xml.etree.ElementTree as ET
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy.interpolate import CubicSpline


SVG_NS = {"svg": "http://www.w3.org/2000/svg"}
DEFAULT_SVG = Path(r"C:\Users\Dream\OneDrive\Desktop\untitled.svg")
ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUT_PNG = ROOT / "tmp" / "chemdraw-n-clip-fit.png"
DEFAULT_OUT_JSON = ROOT / "tmp" / "chemdraw-n-clip-fit.json"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Fit ChemDraw N label clipping contour from radial bond SVG samples.")
    parser.add_argument("--svg", default=str(DEFAULT_SVG), help="Input SVG path.")
    parser.add_argument("--png-out", default=str(DEFAULT_OUT_PNG), help="Output PNG path.")
    parser.add_argument("--json-out", default=str(DEFAULT_OUT_JSON), help="Output JSON path.")
    return parser.parse_args()


def parse_svg(svg_path: Path) -> tuple[list[tuple[float, float]], dict[str, float]]:
    root = ET.fromstring(svg_path.read_text(encoding="utf-8"))

    n_text = None
    for text in root.findall(".//svg:text", SVG_NS):
        content = "".join(text.itertext()).strip()
        if content != "N":
            continue
        tspan = text.find("svg:tspan", SVG_NS)
        if tspan is None:
            continue
        x = float(tspan.attrib["x"])
        y = float(tspan.attrib["y"])
        font_size = float(tspan.attrib["font-size"])
        if x > 200 and y > 150:
            n_text = {"x": x, "y": y, "font_size": font_size}
            break

    if n_text is None:
        raise ValueError("could not find target N text in SVG")

    samples: list[tuple[float, float]] = []
    for path in root.findall(".//svg:path", SVG_NS):
        d = path.attrib.get("d", "")
        nums = [float(value) for value in re.findall(r"-?\d+(?:\.\d+)?", d)]
        if not nums:
            continue
        xs = nums[0::2]
        ys = nums[1::2]
        if min(xs) <= 220 or max(xs) >= 300 or min(ys) <= 160 or max(ys) >= 240:
            continue
        # ChemDraw bond polygons store the inner cap as the first 3 points.
        # The middle point of that cap is the clipped bond centerline start.
        samples.append((xs[1], ys[1]))

    if len(samples) != 24:
        raise ValueError(f"expected 24 radial bond samples, got {len(samples)}")

    return samples, n_text


def estimate_center(samples: list[tuple[float, float]]) -> tuple[float, float]:
    half = len(samples) // 2
    mids = [
        ((samples[i][0] + samples[i + half][0]) * 0.5, (samples[i][1] + samples[i + half][1]) * 0.5)
        for i in range(half)
    ]
    return (
        sum(point[0] for point in mids) / len(mids),
        sum(point[1] for point in mids) / len(mids),
    )


def fit_periodic_curve(samples: list[tuple[float, float]], center: tuple[float, float]) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    cx, cy = center
    radii = []
    angles = []
    for point in samples:
        dx = point[0] - cx
        dy = point[1] - cy
        angle = math.atan2(dy, dx)
        angles.append(angle)
        radii.append(math.hypot(dx, dy))

    # Samples are every 15 degrees, starting near +Y and moving CCW.
    # Sort by angle in [0, 2pi) before fitting.
    paired = sorted(
        ((angle % (2 * math.pi), radius, point) for angle, radius, point in zip(angles, radii, samples)),
        key=lambda item: item[0],
    )
    sorted_angles = np.array([item[0] for item in paired], dtype=float)
    sorted_radii = np.array([item[1] for item in paired], dtype=float)

    ext_angles = np.concatenate([sorted_angles, [sorted_angles[0] + 2 * math.pi]])
    ext_radii = np.concatenate([sorted_radii, [sorted_radii[0]]])
    spline = CubicSpline(ext_angles, ext_radii, bc_type="periodic")

    dense_angles = np.linspace(0.0, 2 * math.pi, 720, endpoint=False)
    dense_radii = spline(dense_angles)
    dense_x = cx + np.cos(dense_angles) * dense_radii
    dense_y = cy + np.sin(dense_angles) * dense_radii
    return dense_x, dense_y, dense_radii


def draw_fit(
    samples: list[tuple[float, float]],
    center: tuple[float, float],
    n_text: dict[str, float],
    fit_x: np.ndarray,
    fit_y: np.ndarray,
    png_path: Path,
) -> None:
    all_x = [point[0] for point in samples] + fit_x.tolist() + [center[0]]
    all_y = [point[1] for point in samples] + fit_y.tolist() + [center[1]]
    pad = 12
    min_x, max_x = min(all_x) - pad, max(all_x) + pad
    min_y, max_y = min(all_y) - pad, max(all_y) + pad

    scale = 8
    width = int(round((max_x - min_x) * scale))
    height = int(round((max_y - min_y) * scale))
    image = Image.new("RGBA", (width, height), (255, 255, 255, 255))
    draw = ImageDraw.Draw(image, "RGBA")

    def map_pt(x: float, y: float) -> tuple[float, float]:
        return ((x - min_x) * scale, (y - min_y) * scale)

    polygon = [map_pt(x, y) for x, y in zip(fit_x, fit_y)]
    draw.polygon(polygon, fill=(214, 236, 255, 140), outline=(55, 125, 255, 255))

    cx, cy = map_pt(*center)
    draw.ellipse((cx - 4, cy - 4, cx + 4, cy + 4), fill=(255, 128, 0, 255))

    for sx, sy in samples:
        px, py = map_pt(sx, sy)
        draw.ellipse((px - 3.2, py - 3.2, px + 3.2, py + 3.2), fill=(220, 38, 38, 255))

    font_size = max(12, int(round(n_text["font_size"] * scale)))
    font = ImageFont.truetype(r"C:\Windows\Fonts\arial.ttf", font_size)
    tx, ty = map_pt(n_text["x"], n_text["y"])
    draw.text((tx, ty - font_size), "N", font=font, fill=(0, 0, 0, 255))

    png_path.parent.mkdir(parents=True, exist_ok=True)
    image.save(png_path)


def write_json(
    samples: list[tuple[float, float]],
    center: tuple[float, float],
    dense_radii: np.ndarray,
    json_path: Path,
) -> None:
    json_path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "center": {"x": round(center[0], 6), "y": round(center[1], 6)},
        "sample_count": len(samples),
        "samples": [
            {"x": round(x, 6), "y": round(y, 6)} for x, y in samples
        ],
        "radius_summary": {
            "min": round(float(dense_radii.min()), 6),
            "max": round(float(dense_radii.max()), 6),
            "mean": round(float(dense_radii.mean()), 6),
        },
    }
    json_path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")


def main() -> int:
    args = parse_args()
    samples, n_text = parse_svg(Path(args.svg))
    center = estimate_center(samples)
    fit_x, fit_y, dense_radii = fit_periodic_curve(samples, center)
    draw_fit(samples, center, n_text, fit_x, fit_y, Path(args.png_out))
    write_json(samples, center, dense_radii, Path(args.json_out))
    print(Path(args.png_out))
    print(Path(args.json_out))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
