#!/usr/bin/env python3

from __future__ import annotations

import argparse
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage
from fontTools.ttLib import TTFont


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FONT = Path(r"C:\Windows\Fonts\arial.ttf")

FONT_SIZE = 130
GRID_COLS = 5
CELL_W = 250
CELL_H = 220
MARGIN_X = 24
MARGIN_Y = 86
GAP_X = 18
GAP_Y = 18

BG = (251, 250, 247, 255)
CARD_BG = (255, 255, 255, 255)
CARD_STROKE = (229, 224, 214, 255)
TITLE = (17, 17, 17, 255)
SUB = (85, 85, 85, 255)
GLYPH = (17, 17, 17, 255)
MIDPOINT = (37, 99, 235, 255)
INSET_POINT = (34, 197, 94, 255)
NATURAL_FILL = (255, 255, 255, 255)
NATURAL_STROKE = (214, 209, 196, 255)


ANCHOR_MAP = {
    "A": [("midpoint", "c0", 1, 2), ("point", "c0", 0), ("point", "c0", 3)],
    "B": [("point", "c0", 1), ("point", "c0", 0)],
    "C": [],
    "D": [("point", "c0", 1), ("point", "c0", 0)],
    "E": [("point", "c0", 1), ("point", "c0", 2), ("point", "c0", 0), ("point", "c0", 11)],
    "F": [("point", "c0", 1), ("point", "c0", 2), ("point", "c0", 0)],
    "G": [],
    "H": [("point", "c0", 1), ("point", "c0", 6), ("point", "c0", 0), ("point", "c0", 7)],
    "I": [("midpoint", "c0", 1, 2), ("midpoint", "c0", 0, 3)],
    "J": [("midpoint", "c0", 9, 10)],
    "K": [("point", "c0", 1), ("point", "c0", 5), ("point", "c0", 7), ("point", "c0", 0)],
    "L": [("point", "c0", 1), ("point", "c0", 0), ("point", "c0", 5)],
    "M": [("point", "c0", 1), ("point", "c0", 9), ("point", "c0", 0), ("point", "c0", 10)],
    "N": [("point", "c0", 1), ("point", "c0", 5), ("point", "c0", 0), ("point", "c0", 6)],
    "O": [],
    "P": [("point", "c0", 1), ("point", "c0", 0)],
    "Q": [("midpoint", "c0", 2, 3)],
    "R": [("point", "c0", 1), ("point", "c0", 0), ("point", "c0", 14)],
    "S": [],
    "T": [("midpoint", "c0", 2, 3), ("midpoint", "c0", 4, 5), ("midpoint", "c0", 0, 7)],
    "U": [("midpoint", "c0", 11, 12), ("midpoint", "c0", 0, 1)],
    "V": [("point", "c0", 1), ("point", "c0", 9), ("midpoint", "c0", 0, 10)],
    "W": [("point", "c0", 1), ("point", "c0", 16), ("point", "c0", 0), ("point", "c0", 17)],
    "X": [("point", "c0", 2), ("point", "c0", 10), ("point", "c0", 0), ("point", "c0", 12)],
    "Y": [("point", "c0", 2), ("point", "c0", 10), ("midpoint", "c0", 0, 12)],
    "Z": [("point", "c0", 6), ("point", "c0", 7), ("point", "c0", 0), ("point", "c0", 12)],
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Render A-Z glyph corner and cap-segment anchor atlas.")
    parser.add_argument("--font", default=str(DEFAULT_FONT), help="Path to the TTF font.")
    parser.add_argument(
        "--output",
        default=str(ROOT / "tmp" / "glyph-anchor-corner-atlas-AZ.png"),
        help="Output PNG path.",
    )
    parser.add_argument(
        "--show-inset-green",
        action="store_true",
        help="Also draw inset green anchor points, shifted inward by 0.2 * glyph height.",
    )
    parser.add_argument(
        "--merge-green-circles",
        action="store_true",
        help="Merge filled circles centered at inset green points into the natural white region.",
    )
    parser.add_argument(
        "--natural-outset-ratio",
        type=float,
        default=0.20,
        help="Natural dilation radius as a fraction of glyph height.",
    )
    parser.add_argument(
        "--green-inset-ratio",
        type=float,
        default=0.20,
        help="Inset distance for green points as a fraction of glyph height.",
    )
    parser.add_argument(
        "--circle-radius-ratio",
        type=float,
        default=0.40,
        help="Merged circle radius as a fraction of glyph height.",
    )
    return parser.parse_args()


def load_contours(ttfont: TTFont, ch: str) -> list[list[tuple[int, int, bool]]]:
    glyph_name = ttfont.getBestCmap()[ord(ch)]
    glyph = ttfont["glyf"][glyph_name]
    coords, end_pts, flags = glyph.getCoordinates(ttfont["glyf"])
    contours: list[list[tuple[int, int, bool]]] = []
    start = 0
    for end in end_pts:
        contour = []
        for (x, y), flag in zip(coords[start : end + 1], flags[start : end + 1]):
            contour.append((int(x), int(y), bool(flag & 1)))
        contours.append(contour)
        start = end + 1
    return contours


def disk_kernel(radius: int) -> np.ndarray:
    if radius <= 0:
        return np.ones((1, 1), dtype=bool)
    yy, xx = np.ogrid[-radius : radius + 1, -radius : radius + 1]
    return (xx * xx + yy * yy) <= radius * radius


def render_glyph_with_natural_outline(
    font: ImageFont.FreeTypeFont,
    ch: str,
    width: int,
    height: int,
    ox: int,
    oy: int,
    natural_outset_ratio: float,
    circle_radius_ratio: float,
    merged_circle_centers: list[tuple[float, float]] | None = None,
) -> Image.Image:
    mask_image = Image.new("L", (width, height), 0)
    ImageDraw.Draw(mask_image).text((ox, oy), ch, font=font, fill=255, anchor="ls")
    mask = np.array(mask_image) >= 128
    ys, xs = np.nonzero(mask)
    glyph_height = int(ys.max() - ys.min() + 1)
    dilation_radius = max(1, int(round(glyph_height * natural_outset_ratio)))
    dilated = ndimage.binary_dilation(mask, structure=disk_kernel(dilation_radius))
    merged = dilated.copy()

    if merged_circle_centers:
        circle_radius = glyph_height * circle_radius_ratio
        yy, xx = np.ogrid[:height, :width]
        for cx, cy in merged_circle_centers:
            circle_mask = (xx - cx) ** 2 + (yy - cy) ** 2 <= circle_radius * circle_radius
            merged = np.logical_or(merged, circle_mask)

    outline = np.logical_and(merged, np.logical_not(mask))

    rgba = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    rgba.paste(Image.new("RGBA", (width, height), NATURAL_FILL), (0, 0), Image.fromarray((merged * 255).astype(np.uint8)))
    rgba.paste(Image.new("RGBA", (width, height), NATURAL_STROKE), (0, 0), Image.fromarray((outline * 255).astype(np.uint8)))
    rgba.paste(Image.new("RGBA", (width, height), GLYPH), (0, 0), Image.fromarray((mask * 255).astype(np.uint8)))
    return rgba


def collect_anchor_positions(
    contour_points: dict[str, dict[int, tuple[float, float]]], ch: str
) -> list[tuple[float, float]]:
    anchors: list[tuple[float, float]] = []
    for spec in ANCHOR_MAP[ch]:
        mode = spec[0]
        contour_key = spec[1]
        if contour_key not in contour_points:
            continue
        if mode == "point":
            point_index = spec[2]
            if point_index not in contour_points[contour_key]:
                continue
            anchors.append(contour_points[contour_key][point_index])
            continue
        a, b = spec[2], spec[3]
        if a not in contour_points[contour_key] or b not in contour_points[contour_key]:
            continue
        p1 = contour_points[contour_key][a]
        p2 = contour_points[contour_key][b]
        anchors.append(((p1[0] + p2[0]) * 0.5, (p1[1] + p2[1]) * 0.5))
    return anchors


def inset_anchor(
    anchor: tuple[float, float],
    glyph_center: tuple[float, float],
    glyph_w: float,
    glyph_h: float,
    green_inset_ratio: float,
) -> tuple[float, float]:
    offset = glyph_h * green_inset_ratio
    center_band = glyph_w * 0.12
    ax, ay = anchor
    cx, cy = glyph_center

    if ax < cx - center_band:
        dx = offset
    elif ax > cx + center_band:
        dx = -offset
    else:
        dx = 0.0

    dy = offset if ay < cy else -offset
    return ax + dx, ay + dy


def render_letter(
    draw: ImageDraw.ImageDraw,
    ttfont: TTFont,
    font: ImageFont.FreeTypeFont,
    label_font: ImageFont.FreeTypeFont,
    small_font: ImageFont.FreeTypeFont,
    show_inset_green: bool,
    merge_green_circles: bool,
    natural_outset_ratio: float,
    green_inset_ratio: float,
    circle_radius_ratio: float,
    ch: str,
    x0: int,
    y0: int,
) -> None:
    draw.rounded_rectangle((x0, y0, x0 + CELL_W, y0 + CELL_H), radius=12, fill=CARD_BG, outline=CARD_STROKE, width=1)
    draw.text((x0 + 12, y0 + 10), ch, font=label_font, fill=TITLE)
    draw.text((x0 + 42, y0 + 14), f"{len(ANCHOR_MAP[ch])} anchors", font=small_font, fill=SUB)

    left, top, right, bottom = font.getbbox(ch, anchor="ls")
    glyph_w = right - left
    glyph_h = bottom - top
    target_w = CELL_W - 38
    target_h = CELL_H - 52
    ox = x0 + (CELL_W - glyph_w) // 2 - left
    oy = y0 + 34 + (target_h - glyph_h) // 2 - top

    contours = load_contours(ttfont, ch)
    scale = FONT_SIZE / ttfont["head"].unitsPerEm

    contour_points: dict[str, dict[int, tuple[float, float]]] = {}
    for contour_index, contour in enumerate(contours):
        contour_key = f"c{contour_index}"
        contour_points[contour_key] = {}
        for point_index, (x, y, on_curve) in enumerate(contour):
            if not on_curve:
                continue
            px = ox + x * scale
            py = oy - y * scale
            contour_points[contour_key][point_index] = (px, py)

    anchors = collect_anchor_positions(contour_points, ch)
    glyph_center = (ox + (left + right) * 0.5, oy + (top + bottom) * 0.5)
    inset_anchors: list[tuple[float, float]] = []
    if show_inset_green or merge_green_circles:
        inset_anchors = [
            inset_anchor((px, py), glyph_center, glyph_w, glyph_h, green_inset_ratio)
            for px, py in anchors
        ]

    local_circle_centers = None
    if merge_green_circles:
        local_circle_centers = [(gx - x0, gy - y0) for gx, gy in inset_anchors]

    glyph_layer = render_glyph_with_natural_outline(
        font,
        ch,
        CELL_W,
        CELL_H,
        ox - x0,
        oy - y0,
        natural_outset_ratio,
        circle_radius_ratio,
        merged_circle_centers=local_circle_centers,
    )
    image = draw._image
    image.alpha_composite(glyph_layer, (x0, y0))

    for px, py in anchors:
        draw.ellipse((px - 4.8, py - 4.8, px + 4.8, py + 4.8), fill=MIDPOINT, outline=(255, 255, 255, 255), width=1)
    if show_inset_green:
        for gx, gy in inset_anchors:
            draw.ellipse((gx - 4.8, gy - 4.8, gx + 4.8, gy + 4.8), fill=INSET_POINT, outline=(255, 255, 255, 255), width=1)


def main() -> int:
    args = parse_args()
    letters = list("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
    rows = (len(letters) + GRID_COLS - 1) // GRID_COLS
    width = MARGIN_X * 2 + GRID_COLS * CELL_W + (GRID_COLS - 1) * GAP_X
    height = MARGIN_Y + rows * CELL_H + (rows - 1) * GAP_Y + 44

    image = Image.new("RGBA", (width, height), BG)
    draw = ImageDraw.Draw(image, "RGBA")
    title_font = ImageFont.truetype(args.font, 30)
    sub_font = ImageFont.truetype(args.font, 16)
    label_font = ImageFont.truetype(args.font, 28)
    small_font = ImageFont.truetype(args.font, 14)
    glyph_font = ImageFont.truetype(args.font, FONT_SIZE)
    ttfont = TTFont(args.font)

    title = "A-Z glyph anchor candidates"
    subtitle = "black = glyph, blue = chosen anchor"
    if args.show_inset_green:
        title = "A-Z glyph anchor candidates with inward inset"
        subtitle = f"black = glyph, blue = original anchor, green = inward {args.green_inset_ratio:.2f} * glyph height"
    if args.merge_green_circles:
        title = "A-Z anchors with merged green-center circle union"
        subtitle = (
            f"white = natural {args.natural_outset_ratio:.2f}h + circle {args.circle_radius_ratio:.2f}h union, "
            f"blue = original anchor, green = inset center"
        )
    draw.text((MARGIN_X, 24), title, font=title_font, fill=TITLE)
    draw.text((MARGIN_X, 58), subtitle, font=sub_font, fill=SUB)

    for index, ch in enumerate(letters):
        col = index % GRID_COLS
        row = index // GRID_COLS
        cell_x = MARGIN_X + col * (CELL_W + GAP_X)
        cell_y = MARGIN_Y + row * (CELL_H + GAP_Y)
        render_letter(
            draw,
            ttfont,
            glyph_font,
            label_font,
            small_font,
            args.show_inset_green,
            args.merge_green_circles,
            args.natural_outset_ratio,
            args.green_inset_ratio,
            args.circle_radius_ratio,
            ch,
            cell_x,
            cell_y,
        )

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    image.save(output_path)
    print(output_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
