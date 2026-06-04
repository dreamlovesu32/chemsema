#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
from pathlib import Path

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.ttLib import TTFont


ROOT = Path(__file__).resolve().parents[1]
PROFILE_PATH = ROOT / "shared" / "glyph_profiles.json"
DEFAULT_FONT = Path(r"C:\Windows\Fonts\arial.ttf")

FONT_SIZE_PX = 100.0
NATURAL_OUTSET_RATIO = 0.20
PETAL_RADIUS_HEIGHT_RATIO = 0.31

GRID_COLS = 6
CELL_W = 190
CELL_H = 180
MARGIN_X = 28
MARGIN_Y = 88
GAP_X = 16
GAP_Y = 18

FAMILY_CENTERS = {
    "petal-a": [(0.5, 0.3), (0.3, 0.7), (0.7, 0.7)],
    "petal-bdp": [(0.3, 0.3), (0.3, 0.7)],
    "petal-f": [(0.3, 0.3), (0.3, 0.7), (0.7, 0.3)],
    "petal-i": [(0.5, 0.3), (0.5, 0.7)],
    "petal-j": [(0.5, 0.3)],
    "petal-l": [(0.3, 0.3), (0.3, 0.7), (0.7, 0.7)],
    "petal-r": [(0.3, 0.3), (0.3, 0.7), (0.7, 0.7)],
    "petal-nehkxz": [(0.3, 0.3), (0.7, 0.3), (0.7, 0.7), (0.3, 0.7)],
    "petal-q": [(0.7, 0.7)],
    "petal-t": [(0.3, 0.3), (0.7, 0.3), (0.5, 0.7)],
    "petal-u": [(0.3, 0.3), (0.7, 0.3)],
    "petal-v": [(0.3, 0.3), (0.7, 0.3), (0.5, 0.7)],
    "petal-y": [(0.3, 0.3), (0.7, 0.3), (0.5, 0.7)],
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Render A-Z glyph natural expansion and petal circles atlas.")
    parser.add_argument("--font", default=str(DEFAULT_FONT), help="Path to TTF font.")
    parser.add_argument(
        "--output",
        default=str(ROOT / "tmp" / "glyph-expansion-atlas-AZ.svg"),
        help="Output SVG path.",
    )
    return parser.parse_args()


def load_profiles() -> dict:
    data = json.loads(PROFILE_PATH.read_text(encoding="utf-8"))
    return data["specials"]


def glyph_path_data(ttfont: TTFont, ch: str) -> str:
    glyph_set = ttfont.getGlyphSet()
    glyph_name = ttfont.getBestCmap()[ord(ch)]
    pen = SVGPathPen(glyph_set)
    glyph_set[glyph_name].draw(pen)
    return pen.getCommands()


def glyph_contours(ttfont: TTFont, ch: str) -> tuple[dict[str, int], list[list[tuple[int, int]]]]:
    glyph_name = ttfont.getBestCmap()[ord(ch)]
    glyph = ttfont["glyf"][glyph_name]
    coords, end_pts, _flags = glyph.getCoordinates(ttfont["glyf"])
    points = [(int(x), int(y)) for x, y in coords]
    contours = []
    start = 0
    for end in end_pts:
        contours.append(points[start : end + 1])
        start = end + 1
    bounds = {"xMin": glyph.xMin, "xMax": glyph.xMax, "yMin": glyph.yMin, "yMax": glyph.yMax}
    return bounds, contours


def mw_special_points(ttfont: TTFont, ch: str, box: dict[str, float], scale: float, baseline_y: float) -> list[tuple[float, float]]:
    bounds, contours = glyph_contours(ttfont, ch)
    points = contours[0]
    spans: list[tuple[int, int, int]] = []
    for index, p1 in enumerate(points):
        p2 = points[(index + 1) % len(points)]
        if p1[1] == p2[1] and p1[0] != p2[0]:
            x1, x2 = sorted((p1[0], p2[0]))
            spans.append((x1, p1[1], x2))

    top = sorted([span for span in spans if span[1] == bounds["yMax"]], key=lambda span: span[0])
    bottom = sorted([span for span in spans if span[1] == bounds["yMin"]], key=lambda span: span[0])
    selected = top[:1] + top[-1:] + bottom[:1] + bottom[-1:]
    result = []
    for x1, y, x2 in selected:
        mx = (x1 + x2) * 0.5
        svg_x = box["origin_x"] + mx * scale
        svg_y = baseline_y - y * scale
        result.append((svg_x, svg_y))
    return result


def letter_geometry(ttfont: TTFont, ch: str, profile: dict) -> dict:
    scale = FONT_SIZE_PX / ttfont["head"].unitsPerEm
    ink_left = profile["inkLeftEm"] * FONT_SIZE_PX
    ink_top = profile["inkTopEm"] * FONT_SIZE_PX
    ink_right = profile["inkRightEm"] * FONT_SIZE_PX
    ink_bottom = profile["inkBottomEm"] * FONT_SIZE_PX
    pad_x = profile["padXEm"] * FONT_SIZE_PX
    pad_y = profile["padYEm"] * FONT_SIZE_PX

    box_w = (ink_right - ink_left) + pad_x * 2
    box_h = (ink_bottom - ink_top) + pad_y * 2

    path = glyph_path_data(ttfont, ch)

    return {
        "ch": ch,
        "shape": profile["shape"],
        "box_w": box_w,
        "box_h": box_h,
        "scale": scale,
        "ink_left": ink_left,
        "ink_top": ink_top,
        "path": path,
    }


def render_svg(font_path: Path, output_path: Path) -> None:
    profiles = load_profiles()
    ttfont = TTFont(str(font_path))
    letters = [letter_geometry(ttfont, ch, profiles[ch]) for ch in "ABCDEFGHIJKLMNOPQRSTUVWXYZ"]

    rows = (len(letters) + GRID_COLS - 1) // GRID_COLS
    width = MARGIN_X * 2 + GRID_COLS * CELL_W + (GRID_COLS - 1) * GAP_X
    height = MARGIN_Y + rows * CELL_H + (rows - 1) * GAP_Y + 36

    parts = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<style>',
        'text{font-family:Arial,sans-serif}',
        '.title{font-size:28px;font-weight:700;fill:#111}',
        '.sub{font-size:15px;fill:#555}',
        '.label{font-size:16px;font-weight:700;fill:#111}',
        '.small{font-size:13px;fill:#444}',
        ".centerline{stroke:#ddd;stroke-width:1;stroke-dasharray:2 4}",
        '</style>',
        '<rect width="100%" height="100%" fill="#fbfaf7"/>',
        '<text class="title" x="28" y="36">A-Z glyph circles and natural expansion</text>',
        '<text class="sub" x="28" y="60">black = true Arial glyph outline; blue stroke = natural expansion (0.2 * glyph box height); orange = petal circles (0.31 * glyph box height)</text>',
    ]

    for index, entry in enumerate(letters):
        col = index % GRID_COLS
        row = index // GRID_COLS
        cell_x = MARGIN_X + col * (CELL_W + GAP_X)
        cell_y = MARGIN_Y + row * (CELL_H + GAP_Y)

        box_x = cell_x + (CELL_W - entry["box_w"]) * 0.5
        box_y = cell_y + 30
        baseline_y = box_y + entry["box_h"] - (profiles[entry["ch"]]["padYEm"] * FONT_SIZE_PX) - (profiles[entry["ch"]]["inkBottomEm"] * FONT_SIZE_PX)
        origin_x = box_x + profiles[entry["ch"]]["padXEm"] * FONT_SIZE_PX - entry["ink_left"]
        box = {"x": box_x, "y": box_y, "w": entry["box_w"], "h": entry["box_h"], "origin_x": origin_x}

        parts.append(f'<g>')
        parts.append(f'<rect x="{cell_x}" y="{cell_y}" width="{CELL_W}" height="{CELL_H}" rx="10" fill="#fff" stroke="#e5e0d6"/>')
        parts.append(f'<text class="label" x="{cell_x + 10}" y="{cell_y + 22}">{entry["ch"]}</text>')

        family_label = entry["shape"]
        if entry["ch"] in {"M", "W"}:
            family_label = "mw-special"
        parts.append(f'<text class="small" x="{cell_x + 34}" y="{cell_y + 22}">{family_label}</text>')

        center_x = box_x + entry["box_w"] * 0.5
        center_y = box_y + entry["box_h"] * 0.5
        parts.append(f'<line class="centerline" x1="{center_x}" y1="{box_y - 8}" x2="{center_x}" y2="{box_y + entry["box_h"] + 8}"/>')
        parts.append(f'<line class="centerline" x1="{box_x - 8}" y1="{center_y}" x2="{box_x + entry["box_w"] + 8}" y2="{center_y}"/>')
        parts.append(f'<rect x="{box_x:.2f}" y="{box_y:.2f}" width="{entry["box_w"]:.2f}" height="{entry["box_h"]:.2f}" fill="none" stroke="#4da3ff" stroke-width="1.1" stroke-dasharray="3 2"/>')

        natural_outset = entry["box_h"] * NATURAL_OUTSET_RATIO
        parts.append(
            f'<path d="{entry["path"]}" transform="translate({origin_x:.3f} {baseline_y:.3f}) scale({entry["scale"]:.6f} {-entry["scale"]:.6f})" '
            f'fill="none" stroke="#60a5fa" stroke-width="{natural_outset * 2:.3f}" stroke-linejoin="round" stroke-linecap="round" opacity="0.35"/>'
        )

        radius = entry["box_h"] * PETAL_RADIUS_HEIGHT_RATIO
        if entry["ch"] in {"M", "W"}:
            centers = mw_special_points(ttfont, entry["ch"], box, entry["scale"], baseline_y)
        else:
            centers = [
                (box_x + entry["box_w"] * nx, box_y + entry["box_h"] * ny)
                for nx, ny in FAMILY_CENTERS.get(entry["shape"], [])
            ]

        for cx, cy in centers:
            parts.append(
                f'<circle cx="{cx:.3f}" cy="{cy:.3f}" r="{radius:.3f}" fill="#f59e0b" opacity="0.18" stroke="#d97706" stroke-width="1"/>'
            )
            parts.append(f'<circle cx="{cx:.3f}" cy="{cy:.3f}" r="2.3" fill="#b45309"/>')

        parts.append(
            f'<path d="{entry["path"]}" transform="translate({origin_x:.3f} {baseline_y:.3f}) scale({entry["scale"]:.6f} {-entry["scale"]:.6f})" '
            'fill="#111" stroke="none"/>'
        )
        parts.append("</g>")

    parts.append("</svg>")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text("\n".join(parts), encoding="utf-8")


def main() -> int:
    args = parse_args()
    render_svg(Path(args.font), Path(args.output))
    print(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
