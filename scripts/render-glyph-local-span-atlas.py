#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage


ROOT = Path(__file__).resolve().parents[1]
PROFILE_PATH = ROOT / "shared" / "glyph_profiles.json"
DEFAULT_FONT = Path(r"C:\Windows\Fonts\arial.ttf")

FONT_SIZE_PX = 148
GLYPH_MARGIN = 28
CELL_W = 220
CELL_H = 220
GRID_COLS = 6
GAP_X = 18
GAP_Y = 18
MARGIN_X = 28
MARGIN_Y = 92

NATURAL_OUTSET_RATIO = 0.20
PETAL_RADIUS_RATIO = 0.30

FAMILY_POINTS = {
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

LETTER_OVERRIDES = {
    "M": "petal-nehkxz",
    "W": "petal-nehkxz",
}

BG = (251, 250, 247, 255)
CARD_BG = (255, 255, 255, 255)
CARD_STROKE = (229, 224, 214, 255)
TITLE = (17, 17, 17, 255)
SUB = (85, 85, 85, 255)
BLUE_FILL = (96, 165, 250, 92)
BLUE_STROKE = (77, 163, 255, 255)
ORANGE_FILL = (245, 158, 11, 46)
ORANGE_STROKE = (217, 119, 6, 255)
ORANGE_DOT = (180, 83, 9, 255)
BLACK = (17, 17, 17, 255)
ROW_GUIDE = (214, 226, 255, 255)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Render A-Z glyph atlas using local-span anchor centers and mask-based natural dilation."
    )
    parser.add_argument("--font", default=str(DEFAULT_FONT), help="Path to the TTF font.")
    parser.add_argument(
        "--output",
        default=str(ROOT / "tmp" / "glyph-local-span-atlas-AZ.png"),
        help="Output PNG path.",
    )
    parser.add_argument(
        "--json-out",
        default=str(ROOT / "tmp" / "glyph-local-span-atlas-AZ.json"),
        help="Where to write derived center data.",
    )
    return parser.parse_args()


def load_shapes() -> dict[str, str]:
    data = json.loads(PROFILE_PATH.read_text(encoding="utf-8"))
    return {ch: info["shape"] for ch, info in data["specials"].items()}


def disk_kernel(radius: int) -> np.ndarray:
    if radius <= 0:
        return np.ones((1, 1), dtype=bool)
    yy, xx = np.ogrid[-radius : radius + 1, -radius : radius + 1]
    return (xx * xx + yy * yy) <= radius * radius


def glyph_mask(font: ImageFont.FreeTypeFont, ch: str) -> tuple[np.ndarray, dict[str, int]]:
    left, top, right, bottom = font.getbbox(ch, anchor="ls")
    width = right - left
    height = bottom - top
    canvas = Image.new("L", (width + GLYPH_MARGIN * 2, height + GLYPH_MARGIN * 2), 0)
    draw = ImageDraw.Draw(canvas)
    draw.text((GLYPH_MARGIN - left, GLYPH_MARGIN - top), ch, font=font, fill=255, anchor="ls")
    arr = np.array(canvas)
    mask = arr >= 128

    ys, xs = np.nonzero(mask)
    bbox = {
        "x1": int(xs.min()),
        "y1": int(ys.min()),
        "x2": int(xs.max()),
        "y2": int(ys.max()),
    }
    return mask, bbox


def row_span(mask: np.ndarray, bbox: dict[str, int], y_norm: float) -> tuple[int, int, int]:
    top = bbox["y1"]
    bottom = bbox["y2"]
    target_y = top + int(round((bottom - top) * y_norm))

    for distance in range(0, max(mask.shape) + 1):
        for candidate_y in (target_y - distance, target_y + distance):
            if candidate_y < top or candidate_y > bottom:
                continue
            xs = np.flatnonzero(mask[candidate_y])
            if xs.size:
                return int(xs[0]), int(xs[-1]), int(candidate_y)
    raise ValueError("no occupied span found for row")


def derive_centers(mask: np.ndarray, bbox: dict[str, int], points: list[tuple[float, float]]) -> list[dict[str, float]]:
    centers = []
    for tx, ty in points:
        left, right, row_y = row_span(mask, bbox, ty)
        cx = left + (right - left) * tx
        centers.append(
            {
                "tx": tx,
                "ty": ty,
                "row_y": float(row_y),
                "row_left": float(left),
                "row_right": float(right),
                "cx": float(cx),
                "cy": float(row_y),
            }
        )
    return centers


def dilate_mask(mask: np.ndarray, bbox: dict[str, int]) -> tuple[np.ndarray, int]:
    height = bbox["y2"] - bbox["y1"] + 1
    radius = max(1, int(round(height * NATURAL_OUTSET_RATIO)))
    return ndimage.binary_dilation(mask, structure=disk_kernel(radius)), radius


def build_letter_record(font: ImageFont.FreeTypeFont, ch: str, shape_map: dict[str, str]) -> dict:
    mask, bbox = glyph_mask(font, ch)
    dilated_mask, dilation_radius = dilate_mask(mask, bbox)
    family = LETTER_OVERRIDES.get(ch, shape_map[ch])
    points = FAMILY_POINTS.get(family, [])
    centers = derive_centers(mask, bbox, points) if points else []
    glyph_height = bbox["y2"] - bbox["y1"] + 1
    return {
        "letter": ch,
        "family": family,
        "mask": mask,
        "dilated_mask": dilated_mask,
        "bbox": bbox,
        "glyph_height": glyph_height,
        "natural_outset_px": dilation_radius,
        "petal_radius_px": glyph_height * PETAL_RADIUS_RATIO,
        "centers": centers,
    }


def alpha_bounds(alpha: np.ndarray) -> tuple[int, int, int, int]:
    ys, xs = np.nonzero(alpha)
    return int(xs.min()), int(ys.min()), int(xs.max()), int(ys.max())


def composite_letter(entry: dict) -> Image.Image:
    mask = entry["mask"]
    dilated = entry["dilated_mask"]
    bbox = entry["bbox"]

    natural_alpha = np.where(dilated, 255, 0).astype(np.uint8)
    glyph_alpha = np.where(mask, 255, 0).astype(np.uint8)

    image = Image.new("RGBA", (mask.shape[1], mask.shape[0]), (0, 0, 0, 0))
    image.paste(Image.new("RGBA", image.size, BLUE_FILL), (0, 0), Image.fromarray(natural_alpha))
    image.paste(Image.new("RGBA", image.size, BLACK), (0, 0), Image.fromarray(glyph_alpha))

    draw = ImageDraw.Draw(image, "RGBA")
    radius = entry["petal_radius_px"]
    for center in entry["centers"]:
        cx = center["cx"]
        cy = center["cy"]
        draw.ellipse((cx - radius, cy - radius, cx + radius, cy + radius), fill=ORANGE_FILL, outline=ORANGE_STROKE, width=1)
        draw.ellipse((cx - 2.5, cy - 2.5, cx + 2.5, cy + 2.5), fill=ORANGE_DOT)
        draw.line((bbox["x1"], cy, bbox["x2"], cy), fill=ROW_GUIDE, width=1)

    image.paste(Image.new("RGBA", image.size, BLACK), (0, 0), Image.fromarray(glyph_alpha))
    return image


def render_atlas(entries: list[dict], output_path: Path) -> None:
    rows = (len(entries) + GRID_COLS - 1) // GRID_COLS
    width = MARGIN_X * 2 + GRID_COLS * CELL_W + (GRID_COLS - 1) * GAP_X
    height = MARGIN_Y + rows * CELL_H + (rows - 1) * GAP_Y + 44

    canvas = Image.new("RGBA", (width, height), BG)
    draw = ImageDraw.Draw(canvas, "RGBA")
    title_font = ImageFont.truetype(str(DEFAULT_FONT), 30)
    sub_font = ImageFont.truetype(str(DEFAULT_FONT), 16)
    label_font = ImageFont.truetype(str(DEFAULT_FONT), 22)
    small_font = ImageFont.truetype(str(DEFAULT_FONT), 14)

    draw.text((MARGIN_X, 22), "A-Z local-span anchors and natural dilation", font=title_font, fill=TITLE)
    draw.text(
        (MARGIN_X, 58),
        "black = glyph, blue = true mask dilation (0.2 * glyph height), orange = circles (0.3 * glyph height), guide = sampled rows",
        font=sub_font,
        fill=SUB,
    )

    for index, entry in enumerate(entries):
        col = index % GRID_COLS
        row = index // GRID_COLS
        cell_x = MARGIN_X + col * (CELL_W + GAP_X)
        cell_y = MARGIN_Y + row * (CELL_H + GAP_Y)
        draw.rounded_rectangle((cell_x, cell_y, cell_x + CELL_W, cell_y + CELL_H), radius=12, fill=CARD_BG, outline=CARD_STROKE, width=1)
        draw.text((cell_x + 12, cell_y + 10), entry["letter"], font=label_font, fill=TITLE)
        draw.text((cell_x + 42, cell_y + 14), entry["family"], font=small_font, fill=SUB)

        letter_image = composite_letter(entry)
        alpha = np.array(letter_image.getchannel("A"))
        x1, y1, x2, y2 = alpha_bounds(alpha)
        crop = letter_image.crop((x1, y1, x2 + 1, y2 + 1))

        target_w = CELL_W - 30
        target_h = CELL_H - 52
        scale = min(target_w / crop.width, target_h / crop.height)
        scaled = crop.resize((max(1, int(round(crop.width * scale))), max(1, int(round(crop.height * scale)))), Image.Resampling.LANCZOS)
        paste_x = cell_x + (CELL_W - scaled.width) // 2
        paste_y = cell_y + 34 + (target_h - scaled.height) // 2
        canvas.alpha_composite(scaled, (paste_x, paste_y))

    output_path.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(output_path)


def write_json(entries: list[dict], output_path: Path) -> None:
    serializable = []
    for entry in entries:
        serializable.append(
            {
                "letter": entry["letter"],
                "family": entry["family"],
                "bbox": entry["bbox"],
                "glyph_height": entry["glyph_height"],
                "natural_outset_px": entry["natural_outset_px"],
                "petal_radius_px": round(entry["petal_radius_px"], 3),
                "centers": [
                    {
                        "tx": center["tx"],
                        "ty": center["ty"],
                        "row_y": round(center["row_y"], 3),
                        "row_left": round(center["row_left"], 3),
                        "row_right": round(center["row_right"], 3),
                        "cx": round(center["cx"], 3),
                        "cy": round(center["cy"], 3),
                    }
                    for center in entry["centers"]
                ],
            }
        )
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(serializable, ensure_ascii=False, indent=2), encoding="utf-8")


def main() -> int:
    args = parse_args()
    shape_map = load_shapes()
    font = ImageFont.truetype(args.font, FONT_SIZE_PX)

    entries = [build_letter_record(font, ch, shape_map) for ch in "ABCDEFGHIJKLMNOPQRSTUVWXYZ"]
    render_atlas(entries, Path(args.output))
    write_json(entries, Path(args.json_out))
    print(Path(args.output))
    print(Path(args.json_out))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
