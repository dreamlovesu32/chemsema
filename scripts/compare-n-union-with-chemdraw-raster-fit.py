#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FONT = Path(r"C:\Windows\Fonts\arial.ttf")
DEFAULT_FIT_JSON = ROOT / "tmp" / "chemdraw-n-clip-fit-raster.json"
DEFAULT_SCREENSHOT = ROOT / "tmp" / "chemdraw_window_latest.jpg"
DEFAULT_OUT = ROOT / "tmp" / "n-union-vs-chemdraw-raster-fit.png"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fit-json", default=str(DEFAULT_FIT_JSON))
    parser.add_argument("--font", default=str(DEFAULT_FONT))
    parser.add_argument("--screenshot", default=str(DEFAULT_SCREENSHOT))
    parser.add_argument("--output", default=str(DEFAULT_OUT))
    parser.add_argument("--contours-only", action="store_true")
    return parser.parse_args()


def disk_kernel(radius: int) -> np.ndarray:
    if radius <= 0:
        return np.ones((1, 1), dtype=bool)
    yy, xx = np.ogrid[-radius : radius + 1, -radius : radius + 1]
    return (xx * xx + yy * yy) <= radius * radius


def binary_dilation(mask: np.ndarray, radius: int) -> np.ndarray:
    return ndimage.binary_dilation(mask, structure=disk_kernel(radius))


def extract_contour(mask: np.ndarray) -> list[tuple[float, float]]:
    padded = np.pad(mask.astype(np.uint8), 1)
    contours = []
    for y in range(1, padded.shape[0] - 1):
        for x in range(1, padded.shape[1] - 1):
            if padded[y, x] and not np.all(padded[y - 1 : y + 2, x - 1 : x + 2]):
                contours.append((x - 1, y - 1))
    if not contours:
        return []

    pts = np.array(contours, dtype=float)
    cx, cy = pts[:, 0].mean(), pts[:, 1].mean()
    ang = np.arctan2(pts[:, 1] - cy, pts[:, 0] - cx)
    order = np.argsort(ang)
    return [(float(pts[i, 0]), float(pts[i, 1])) for i in order]


def match_font_size(font_path: Path, target_w: int, target_h: int) -> tuple[ImageFont.FreeTypeFont, int, tuple[int, int, int, int]]:
    best = None
    for size in range(8, 80):
        font = ImageFont.truetype(str(font_path), size)
        bbox = font.getbbox("N", anchor="ls")
        w = bbox[2] - bbox[0]
        h = bbox[3] - bbox[1]
        err = abs(w - target_w) + abs(h - target_h)
        if best is None or err < best[0]:
            best = (err, size, font, bbox)
    assert best is not None
    return best[2], best[1], best[3]


def n_anchor_points(text_x: float, baseline_y: float, bbox: tuple[int, int, int, int]) -> tuple[list[tuple[float, float]], tuple[float, float], float]:
    left, top, right, bottom = bbox
    glyph_w = right - left
    glyph_h = bottom - top
    center = (text_x + (left + right) * 0.5, baseline_y + (top + bottom) * 0.5)
    anchors = [
        (text_x + left, baseline_y + top),
        (text_x + right, baseline_y + top),
        (text_x + left, baseline_y + bottom),
        (text_x + right, baseline_y + bottom),
    ]
    return anchors, center, float(glyph_h)


def inset_anchor(anchor: tuple[float, float], center: tuple[float, float], glyph_w: float, glyph_h: float) -> tuple[float, float]:
    offset = glyph_h * 0.20
    center_band = glyph_w * 0.12
    ax, ay = anchor
    cx, cy = center
    if ax < cx - center_band:
        dx = offset
    elif ax > cx + center_band:
        dx = -offset
    else:
        dx = 0.0
    dy = offset if ay < cy else -offset
    return (ax + dx, ay + dy)


def render_our_union(
    font: ImageFont.FreeTypeFont,
    fit_center: tuple[float, float],
    n_bbox: list[int],
    fit_points: list[tuple[float, float]],
) -> dict:
    left, top, right, bottom = font.getbbox("N", anchor="ls")
    glyph_w = right - left
    glyph_h = bottom - top
    center_x, center_y = fit_center
    text_x = center_x - (left + right) * 0.5
    baseline_y = center_y - (top + bottom) * 0.5

    anchors, center, _, = n_anchor_points(text_x, baseline_y, (left, top, right, bottom))
    inset = [inset_anchor(point, center, glyph_w, glyph_h) for point in anchors]

    all_x = [p[0] for p in fit_points] + [n_bbox[0], n_bbox[2]]
    all_y = [p[1] for p in fit_points] + [n_bbox[1], n_bbox[3]]
    pad = glyph_h * 1.1
    min_x, max_x = min(all_x) - pad, max(all_x) + pad
    min_y, max_y = min(all_y) - pad, max(all_y) + pad
    width = int(np.ceil(max_x - min_x))
    height = int(np.ceil(max_y - min_y))

    ox = text_x - min_x
    oy = baseline_y - min_y

    glyph_img = Image.new("L", (width, height), 0)
    ImageDraw.Draw(glyph_img).text((ox, oy), "N", font=font, fill=255, anchor="ls")
    glyph_mask = np.array(glyph_img) >= 128

    union = binary_dilation(glyph_mask, max(1, int(round(glyph_h * 0.20))))
    yy, xx = np.ogrid[:height, :width]
    radius = glyph_h * 0.40
    for gx, gy in inset:
        cx = gx - min_x
        cy = gy - min_y
        union |= (xx - cx) ** 2 + (yy - cy) ** 2 <= radius * radius

    return {
        "min_x": min_x,
        "min_y": min_y,
        "width": width,
        "height": height,
        "glyph_mask": glyph_mask,
        "union_mask": union,
        "contour": extract_contour(union),
        "anchors": anchors,
        "inset_anchors": inset,
        "text_x": text_x,
        "baseline_y": baseline_y,
        "glyph_bbox": [text_x + left, baseline_y + top, text_x + right, baseline_y + bottom],
    }


def main() -> int:
    args = parse_args()
    data = json.loads(Path(args.fit_json).read_text(encoding="utf-8"))
    fit_points = [(float(item["x"]), float(item["y"])) for item in data["points"]]
    fit_curve = [(float(item["x"]), float(item["y"])) for item in data["fit"]]
    center = (float(data["center"]["x"]), float(data["center"]["y"]))
    n_bbox = data["center_component"]["bbox"]

    target_w = n_bbox[2] - n_bbox[0]
    target_h = n_bbox[3] - n_bbox[1]
    font, font_size, font_bbox = match_font_size(Path(args.font), target_w, target_h)
    current = render_our_union(font, center, n_bbox, fit_points)

    screenshot = Image.open(args.screenshot).convert("RGB")
    margin = 40
    x0 = int(min(min(x for x, _ in fit_curve), current["glyph_bbox"][0]) - margin)
    y0 = int(min(min(y for _, y in fit_curve), current["glyph_bbox"][1]) - margin)
    x1 = int(max(max(x for x, _ in fit_curve), current["glyph_bbox"][2]) + margin)
    y1 = int(max(max(y for _, y in fit_curve), current["glyph_bbox"][3]) + margin)

    if args.contours_only:
        crop = Image.new("RGBA", (x1 - x0, y1 - y0), (255, 255, 255, 255))
    else:
        crop = screenshot.crop((x0, y0, x1, y1)).convert("RGBA")
    draw = ImageDraw.Draw(crop, "RGBA")

    def map_pt(x: float, y: float) -> tuple[float, float]:
        return (x - x0, y - y0)

    # light overlay of our union area
    if not args.contours_only:
        union_img = Image.fromarray((current["union_mask"] * 255).astype(np.uint8))
        union_rgba = Image.new("RGBA", union_img.size, (60, 130, 246, 34))
        crop.paste(
            union_rgba,
            (int(round(current["min_x"] - x0)), int(round(current["min_y"] - y0))),
            union_img,
        )

    our_contour = [map_pt(current["min_x"] + x, current["min_y"] + y) for x, y in current["contour"]]
    if our_contour:
        draw.line(our_contour + [our_contour[0]], fill=(37, 99, 235, 255), width=2)

    fit_poly = [map_pt(x, y) for x, y in fit_curve]
    draw.line(fit_poly + [fit_poly[0]], fill=(220, 38, 38, 255), width=1)

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    crop.save(output_path)

    payload = {
        "fit_center": {"x": center[0], "y": center[1]},
        "matched_font_size": font_size,
        "matched_font_bbox": list(font_bbox),
        "n_bbox": n_bbox,
        "glyph_bbox": current["glyph_bbox"],
        "output": str(output_path),
    }
    Path(output_path.with_suffix(".json")).write_text(json.dumps(payload, indent=2), encoding="utf-8")
    print(output_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
