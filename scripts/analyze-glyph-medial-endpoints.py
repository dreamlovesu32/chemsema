#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage
from skimage.morphology import convex_hull_image, skeletonize


FONT_SIZE_PX = 240
FONT_SIZE_PT = 10.0
PX_PER_PT = FONT_SIZE_PX / FONT_SIZE_PT
PADDING = 160


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--font", default=r"C:\Windows\Fonts\arial.ttf")
    parser.add_argument("--glyphs", default="AXWMIHEFBCOaij0+(")
    return parser.parse_args()


def render_mask(font: ImageFont.FreeTypeFont, character: str):
    left, top, right, bottom = font.getbbox(character, anchor="ls")
    width = max(PADDING * 2 + right - left + 4, 512)
    height = max(PADDING * 2 + bottom - top + 4, 512)
    origin = (PADDING - left, PADDING - top)
    image = Image.new("L", (width, height), 0)
    ImageDraw.Draw(image).text(origin, character, font=font, fill=255, anchor="ls")
    mask = np.asarray(image) >= 128
    anchor = (
        origin[0] + float(font.getlength(character)) * 0.5,
        origin[1] - FONT_SIZE_PX * 0.39,
    )
    return mask, anchor


def endpoint_records(mask: np.ndarray, anchor: tuple[float, float]):
    skeleton = skeletonize(mask)
    neighbors = ndimage.convolve(skeleton.astype(np.uint8), np.ones((3, 3), dtype=np.uint8)) - skeleton
    endpoints = skeleton & (neighbors == 1)
    ink_depth = ndimage.distance_transform_edt(mask)
    hull_depth = ndimage.distance_transform_edt(convex_hull_image(mask))
    ys, xs = np.nonzero(endpoints)
    records = []
    for y, x in zip(ys, xs):
        local_radius = float(ink_depth[y, x])
        outer_depth = float(hull_depth[y, x])
        records.append({
            "pointPt": [
                round((float(x) - anchor[0]) / PX_PER_PT, 4),
                round((float(y) - anchor[1]) / PX_PER_PT, 4),
            ],
            "localRadiusPt": round(local_radius / PX_PER_PT, 4),
            "hullDepthPt": round(outer_depth / PX_PER_PT, 4),
            "exposureRatio": round(outer_depth / max(local_radius, 1e-9), 4),
        })
    return sorted(records, key=lambda record: record["exposureRatio"])


def main() -> None:
    args = parse_args()
    font = ImageFont.truetype(str(Path(args.font)), FONT_SIZE_PX)
    result = {}
    for character in args.glyphs:
        mask, anchor = render_mask(font, character)
        result[character] = endpoint_records(mask, anchor)
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
