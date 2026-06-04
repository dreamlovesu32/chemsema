#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import math
import xml.etree.ElementTree as ET
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy.interpolate import CubicSpline


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CDXML = Path(r"C:\Users\Dream\OneDrive\Desktop\untitled.cdxml")
DEFAULT_OUT_PNG = ROOT / "tmp" / "chemdraw-n-clip-fit-from-cdxml.png"
DEFAULT_OUT_JSON = ROOT / "tmp" / "chemdraw-n-clip-fit-from-cdxml.json"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Fit ChemDraw N clipping contour directly from CDXML bond endpoints.")
    parser.add_argument("--cdxml", default=str(DEFAULT_CDXML), help="Input CDXML path.")
    parser.add_argument("--png-out", default=str(DEFAULT_OUT_PNG), help="Output PNG path.")
    parser.add_argument("--json-out", default=str(DEFAULT_OUT_JSON), help="Output JSON path.")
    return parser.parse_args()


def parse_cdxml(path: Path) -> tuple[tuple[float, float], list[tuple[float, float]], dict[str, float]]:
    root = ET.fromstring(path.read_text(encoding="utf-8"))
    fragment = None
    center_node = None

    for frag in root.iter("fragment"):
        for node in frag.findall("n"):
            text = node.find("t")
            if text is not None:
                label = "".join(text.itertext()).strip()
                if label == "N":
                    fragment = frag
                    center_node = node
                    break
        if center_node is not None:
            break

    if fragment is None or center_node is None:
        raise ValueError("failed to locate central N fragment in CDXML")

    center_id = center_node.attrib["id"]
    cx, cy = [float(v) for v in center_node.attrib["p"].split()]
    t = center_node.find("t")
    n_text = {
        "x": float(t.attrib["p"].split()[0]),
        "y": float(t.attrib["p"].split()[1]),
        "font_size": float(t.find("s").attrib["size"]),
    }

    positions = {
        node.attrib["id"]: tuple(float(v) for v in node.attrib["p"].split())
        for node in fragment.findall("n")
    }
    samples = []
    for bond in fragment.findall("b"):
        begin_id = bond.attrib.get("B")
        end_id = bond.attrib.get("E")
        if begin_id == center_id and end_id in positions:
            samples.append(positions[end_id])
        elif end_id == center_id and begin_id in positions:
            samples.append(positions[begin_id])

    if len(samples) != 24:
        raise ValueError(f"expected 24 bond endpoint samples, got {len(samples)}")

    return (cx, cy), samples, n_text


def fit_periodic_curve(samples: list[tuple[float, float]], center: tuple[float, float]) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    cx, cy = center
    paired = []
    for x, y in samples:
        dx = x - cx
        dy = y - cy
        paired.append((math.atan2(dy, dx) % (2 * math.pi), math.hypot(dx, dy), (x, y)))
    paired.sort(key=lambda item: item[0])

    angles = np.array([item[0] for item in paired], dtype=float)
    radii = np.array([item[1] for item in paired], dtype=float)
    ext_angles = np.concatenate([angles, [angles[0] + 2 * math.pi]])
    ext_radii = np.concatenate([radii, [radii[0]]])
    spline = CubicSpline(ext_angles, ext_radii, bc_type="periodic")

    dense_angles = np.linspace(0.0, 2 * math.pi, 720, endpoint=False)
    dense_radii = spline(dense_angles)
    dense_x = cx + np.cos(dense_angles) * dense_radii
    dense_y = cy + np.sin(dense_angles) * dense_radii
    return dense_x, dense_y, dense_radii


def draw_fit(center: tuple[float, float], samples: list[tuple[float, float]], n_text: dict[str, float], fit_x: np.ndarray, fit_y: np.ndarray, out_path: Path) -> None:
    all_x = [p[0] for p in samples] + fit_x.tolist() + [center[0]]
    all_y = [p[1] for p in samples] + fit_y.tolist() + [center[1]]
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

    out_path.parent.mkdir(parents=True, exist_ok=True)
    image.save(out_path)


def write_json(center: tuple[float, float], samples: list[tuple[float, float]], dense_radii: np.ndarray, out_path: Path) -> None:
    payload = {
        "center": {"x": round(center[0], 6), "y": round(center[1], 6)},
        "samples": [{"x": round(x, 6), "y": round(y, 6)} for x, y in samples],
        "radius_summary": {
            "min": round(float(dense_radii.min()), 6),
            "max": round(float(dense_radii.max()), 6),
            "mean": round(float(dense_radii.mean()), 6),
        },
    }
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")


def main() -> int:
    args = parse_args()
    center, samples, text_meta = parse_cdxml(Path(args.cdxml))
    fit_x, fit_y, dense_radii = fit_periodic_curve(samples, center)
    draw_fit(center, samples, text_meta, fit_x, fit_y, Path(args.png_out))
    write_json(center, samples, dense_radii, Path(args.json_out))
    print(Path(args.png_out))
    print(Path(args.json_out))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
