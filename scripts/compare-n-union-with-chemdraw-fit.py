#!/usr/bin/env python3

from __future__ import annotations

import argparse
import importlib.util
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont

from chemcore_script_env import tmp_input_path, windows_font_path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FONT = windows_font_path("arial.ttf")
DEFAULT_FIT_SCRIPT = ROOT / "scripts" / "fit-chemdraw-n-clip-from-svg.py"
DEFAULT_FIT_SVG = tmp_input_path("chemdraw-n-clip-source.svg")
DEFAULT_OUT = ROOT / "tmp" / "n-union-vs-chemdraw-fit.png"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Overlay current N union shape against ChemDraw-fitted clipping contour.")
    parser.add_argument("--svg", default=str(DEFAULT_FIT_SVG), help="ChemDraw sampling SVG.")
    parser.add_argument("--font", default=str(DEFAULT_FONT), help="TTF font path.")
    parser.add_argument("--output", default=str(DEFAULT_OUT), help="Output PNG path.")
    return parser.parse_args()


def load_fit_module(path: Path):
    spec = importlib.util.spec_from_file_location("fit_chemdraw_n_clip", path)
    module = importlib.util.module_from_spec(spec)
    assert spec and spec.loader
    spec.loader.exec_module(module)
    return module


def disk_kernel(radius: int) -> np.ndarray:
    if radius <= 0:
        return np.ones((1, 1), dtype=bool)
    yy, xx = np.ogrid[-radius : radius + 1, -radius : radius + 1]
    return (xx * xx + yy * yy) <= radius * radius


def binary_dilation(mask: np.ndarray, radius: int) -> np.ndarray:
    from scipy import ndimage

    return ndimage.binary_dilation(mask, structure=disk_kernel(radius))


def n_anchor_points(font_path: Path, text_x: float, baseline_y: float, font_size: float) -> tuple[list[tuple[float, float]], tuple[float, float], float, tuple[float, float, float, float]]:
    from fontTools.ttLib import TTFont

    tt = TTFont(str(font_path))
    glyph_name = tt.getBestCmap()[ord("N")]
    glyph = tt["glyf"][glyph_name]
    coords, end_pts, flags = glyph.getCoordinates(tt["glyf"])
    contour = [(int(x), int(y), bool(flag & 1)) for (x, y), flag in zip(coords[: end_pts[0] + 1], flags[: end_pts[0] + 1])]
    on_curve = {idx: (x, y) for idx, (x, y, on) in enumerate(contour) if on}

    scale = font_size / tt["head"].unitsPerEm
    anchor_indices = [1, 5, 0, 6]
    anchors = [(text_x + on_curve[idx][0] * scale, baseline_y - on_curve[idx][1] * scale) for idx in anchor_indices]

    pil_font = ImageFont.truetype(str(font_path), int(round(font_size)))
    left, top, right, bottom = pil_font.getbbox("N", anchor="ls")
    bbox = (text_x + left, baseline_y + top, text_x + right, baseline_y + bottom)
    center = ((bbox[0] + bbox[2]) * 0.5, (bbox[1] + bbox[3]) * 0.5)
    glyph_height = bottom - top
    return anchors, center, float(glyph_height), bbox


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


def render_current_union(
    font_path: Path,
    text_x: float,
    baseline_y: float,
    font_size: float,
    fit_points: list[tuple[float, float]],
) -> dict:
    pil_font = ImageFont.truetype(str(font_path), int(round(font_size)))
    left, top, right, bottom = pil_font.getbbox("N", anchor="ls")
    glyph_w = right - left
    glyph_h = bottom - top
    anchors, center, _, bbox = n_anchor_points(font_path, text_x, baseline_y, font_size)
    inset = [inset_anchor(point, center, glyph_w, glyph_h) for point in anchors]

    all_x = [p[0] for p in fit_points] + [bbox[0], bbox[2]]
    all_y = [p[1] for p in fit_points] + [bbox[1], bbox[3]]
    pad = glyph_h * 1.0
    min_x, max_x = min(all_x) - pad, max(all_x) + pad
    min_y, max_y = min(all_y) - pad, max(all_y) + pad
    width = int(np.ceil(max_x - min_x))
    height = int(np.ceil(max_y - min_y))

    ox = text_x - min_x
    oy = baseline_y - min_y

    mask_img = Image.new("L", (width, height), 0)
    ImageDraw.Draw(mask_img).text((ox, oy), "N", font=pil_font, fill=255, anchor="ls")
    mask = np.array(mask_img) >= 128

    natural = binary_dilation(mask, max(1, int(round(glyph_h * 0.20))))
    merged = natural.copy()
    yy, xx = np.ogrid[:height, :width]
    radius = glyph_h * 0.40
    for gx, gy in inset:
        cx = gx - min_x
        cy = gy - min_y
        merged |= (xx - cx) ** 2 + (yy - cy) ** 2 <= radius * radius

    return {
        "min_x": min_x,
        "min_y": min_y,
        "width": width,
        "height": height,
        "glyph_mask": mask,
        "union_mask": merged,
        "anchors": anchors,
        "inset_anchors": inset,
        "bbox": bbox,
        "glyph_h": glyph_h,
    }


def draw_overlay(
    fit_x: np.ndarray,
    fit_y: np.ndarray,
    fit_points: list[tuple[float, float]],
    text_meta: dict[str, float],
    current: dict,
    output_path: Path,
) -> None:
    scale = 12
    width = current["width"] * scale
    height = current["height"] * scale
    image = Image.new("RGBA", (width, height), (255, 255, 255, 255))
    draw = ImageDraw.Draw(image, "RGBA")

    union = current["union_mask"]
    union_img = Image.fromarray((union * 255).astype(np.uint8)).resize((width, height), Image.Resampling.NEAREST)
    image.paste((255, 244, 214, 255), (0, 0), union_img)

    def map_pt(x: float, y: float) -> tuple[float, float]:
        return ((x - current["min_x"]) * scale, (y - current["min_y"]) * scale)

    polygon = [map_pt(x, y) for x, y in zip(fit_x, fit_y)]
    draw.line(polygon + [polygon[0]], fill=(39, 126, 255, 255), width=2)

    pil_font = ImageFont.truetype(str(DEFAULT_FONT), int(round(text_meta["font_size"] * scale)))
    tx, ty = map_pt(text_meta["x"], text_meta["y"])
    draw.text((tx, ty - pil_font.size), "N", font=pil_font, fill=(0, 0, 0, 255))

    for ax, ay in current["anchors"]:
        px, py = map_pt(ax, ay)
        draw.ellipse((px - 4, py - 4, px + 4, py + 4), fill=(37, 99, 235, 255))
    for gx, gy in current["inset_anchors"]:
        px, py = map_pt(gx, gy)
        draw.ellipse((px - 4, py - 4, px + 4, py + 4), fill=(34, 197, 94, 255))
    for sx, sy in fit_points:
        px, py = map_pt(sx, sy)
        draw.ellipse((px - 2.5, py - 2.5, px + 2.5, py + 2.5), fill=(220, 38, 38, 255))

    output_path.parent.mkdir(parents=True, exist_ok=True)
    image.save(output_path)


def main() -> int:
    args = parse_args()
    fit_module = load_fit_module(DEFAULT_FIT_SCRIPT)
    samples, text_meta = fit_module.parse_svg(Path(args.svg))
    center = fit_module.estimate_center(samples)
    fit_x, fit_y, _ = fit_module.fit_periodic_curve(samples, center)
    current = render_current_union(Path(args.font), text_meta["x"], text_meta["y"], text_meta["font_size"], samples)
    draw_overlay(fit_x, fit_y, samples, text_meta, current, Path(args.output))
    print(Path(args.output))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
