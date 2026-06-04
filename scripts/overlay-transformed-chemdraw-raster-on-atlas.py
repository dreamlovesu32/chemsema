#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_ATLAS_CELL = ROOT / "tmp" / "glyph-anchor-green-circle-union-N-cell.png"
DEFAULT_CHEMDRAW_ONLY = ROOT / "tmp" / "glyph-anchor-green-circle-union-N-with-chemdraw-fit-chemdraw-only.png"
DEFAULT_REG_JSON = ROOT / "tmp" / "chemdraw-atlas-n-registration.json"
DEFAULT_OUTPUT = ROOT / "tmp" / "overlay-transformed-chemdraw-raster-on-atlas.png"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--atlas-cell", default=str(DEFAULT_ATLAS_CELL))
    parser.add_argument("--chemdraw-only", default=str(DEFAULT_CHEMDRAW_ONLY))
    parser.add_argument("--registration", default=str(DEFAULT_REG_JSON))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    atlas = Image.open(args.atlas_cell).convert("RGBA")
    chemdraw = Image.open(args.chemdraw_only).convert("RGBA")
    reg = json.loads(Path(args.registration).read_text(encoding="utf-8"))["registration"]

    scale = float(reg["scale"])
    tx = float(reg["tx"])
    ty = float(reg["ty"])
    chemdraw_bbox = json.loads(Path(args.registration).read_text(encoding="utf-8"))["chemdraw_bbox"]

    x0, y0, x1, y1 = chemdraw_bbox
    crop = chemdraw.crop((x0, y0, x1, y1))
    rgba = crop.load()
    for y in range(crop.height):
        for x in range(crop.width):
            r, g, b, a = rgba[x, y]
            if r > 245 and g > 245 and b > 245:
                rgba[x, y] = (255, 255, 255, 0)
    scaled = crop.resize(
        (max(1, int(round(crop.width * scale))), max(1, int(round(crop.height * scale)))),
        Image.Resampling.NEAREST,
    )

    out = atlas.copy()
    out.alpha_composite(scaled, (int(round(tx)), int(round(ty))))

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    out.save(output_path)
    print(output_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
