#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_ATLAS = ROOT / "tmp" / "glyph-anchor-green-circle-union-AZ.png"
DEFAULT_FIT_JSON = ROOT / "tmp" / "chemdraw-n-clip-fit-raster.json"
DEFAULT_OUT = ROOT / "tmp" / "glyph-anchor-green-circle-union-N-with-chemdraw-fit.png"
DEFAULT_SCREENSHOT = ROOT / "tmp" / "chemdraw_window_latest.jpg"

GRID_COLS = 5
CELL_W = 250
CELL_H = 220
MARGIN_X = 24
MARGIN_Y = 86
GAP_X = 18
GAP_Y = 18


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--atlas", default=str(DEFAULT_ATLAS))
    parser.add_argument("--fit-json", default=str(DEFAULT_FIT_JSON))
    parser.add_argument("--screenshot", default=str(DEFAULT_SCREENSHOT))
    parser.add_argument("--output", default=str(DEFAULT_OUT))
    return parser.parse_args()


def n_cell_rect() -> tuple[int, int, int, int]:
    index = ord("N") - ord("A")
    col = index % GRID_COLS
    row = index // GRID_COLS
    x0 = MARGIN_X + col * (CELL_W + GAP_X)
    y0 = MARGIN_Y + row * (CELL_H + GAP_Y)
    return x0, y0, x0 + CELL_W, y0 + CELL_H


def union_bbox(mask: np.ndarray) -> list[int]:
    ys, xs = np.nonzero(mask)
    return [int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1]


def extract_atlas_n_bbox(cell: Image.Image) -> list[int]:
    a = np.array(cell.convert("RGBA"))
    mask = (
        (a[:, :, 0] < 40)
        & (a[:, :, 1] < 40)
        & (a[:, :, 2] < 40)
        & (a[:, :, 3] > 200)
    )
    labels, _ = ndimage.label(mask)
    slices = ndimage.find_objects(labels)
    keep = np.zeros_like(mask, dtype=bool)
    for label, slc in enumerate(slices, start=1):
        if slc is None:
            continue
        y0, y1 = slc[0].start, slc[0].stop
        x0, x1 = slc[1].start, slc[1].stop
        area = int((labels[slc] == label).sum())
        if area < 400:
            continue
        if y1 < 60:
            continue
        keep |= labels == label
    return union_bbox(keep)


def extract_screenshot_n_mask(screenshot: Image.Image, bbox: list[int]) -> Image.Image:
    gray = np.array(screenshot.convert("L"))
    x0, y0, x1, y1 = bbox
    crop = gray[y0:y1, x0:x1]
    mask = (crop < 100).astype(np.uint8) * 255
    return Image.fromarray(mask, mode="L")


def main() -> int:
    args = parse_args()
    atlas = Image.open(args.atlas).convert("RGBA")
    screenshot = Image.open(args.screenshot).convert("RGBA")
    fit = json.loads(Path(args.fit_json).read_text(encoding="utf-8"))

    cell_x0, cell_y0, cell_x1, cell_y1 = n_cell_rect()
    cell = atlas.crop((cell_x0, cell_y0, cell_x1, cell_y1)).convert("RGBA")
    draw = ImageDraw.Draw(cell, "RGBA")

    atlas_glyph_bbox = extract_atlas_n_bbox(cell)
    atlas_center = (
        (atlas_glyph_bbox[0] + atlas_glyph_bbox[2]) / 2,
        (atlas_glyph_bbox[1] + atlas_glyph_bbox[3]) / 2,
    )

    fit_center = (float(fit["center"]["x"]), float(fit["center"]["y"]))
    fit_bbox = fit["center_component"]["bbox"]
    fit_mask = extract_screenshot_n_mask(screenshot, fit_bbox)
    fit_w = float(fit_bbox[2] - fit_bbox[0])
    fit_h = float(fit_bbox[3] - fit_bbox[1])
    atlas_w = float(atlas_glyph_bbox[2] - atlas_glyph_bbox[0])
    atlas_h = float(atlas_glyph_bbox[3] - atlas_glyph_bbox[1])
    scale = ((atlas_w / fit_w) * (atlas_h / fit_h)) ** 0.5

    scaled_w = max(1, int(round(fit_w * scale)))
    scaled_h = max(1, int(round(fit_h * scale)))
    scaled_fit_mask = fit_mask.resize((scaled_w, scaled_h), Image.Resampling.NEAREST)
    paste_x = int(round(atlas_center[0] - scaled_w / 2))
    paste_y = int(round(atlas_center[1] - scaled_h / 2))

    fit_curve = [(float(item["x"]), float(item["y"])) for item in fit["fit"]]
    transformed = [
        (
            atlas_center[0] + (x - fit_center[0]) * scale,
            atlas_center[1] + (y - fit_center[1]) * scale,
        )
        for x, y in fit_curve
    ]
    contour_layer = Image.new("RGBA", cell.size, (0, 0, 0, 0))
    contour_draw = ImageDraw.Draw(contour_layer, "RGBA")
    contour_draw.line(transformed + [transformed[0]], fill=(220, 38, 38, 255), width=2)
    for x, y in transformed[::30]:
        contour_draw.ellipse((x - 1.8, y - 1.8, x + 1.8, y + 1.8), fill=(220, 38, 38, 220))
    cell.alpha_composite(contour_layer)

    chemdraw_only = Image.new("RGBA", cell.size, (255, 255, 255, 255))
    black_fill = Image.new("RGBA", scaled_fit_mask.size, (17, 17, 17, 255))
    chemdraw_only.paste(black_fill, (paste_x, paste_y), scaled_fit_mask)
    chemdraw_only.alpha_composite(contour_layer)

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    cell.save(output_path)
    chemdraw_only.save(output_path.with_name(output_path.stem + "-chemdraw-only.png"))
    print(output_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
