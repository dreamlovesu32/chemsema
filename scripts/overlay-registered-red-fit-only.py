#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_BASE = ROOT / "tmp" / "glyph-anchor-green-circle-union-N-cell-0p30-0p60.png"
DEFAULT_FIT = ROOT / "tmp" / "chemdraw-n-clip-fit-raster.json"
DEFAULT_REG = ROOT / "tmp" / "chemdraw-atlas-n-registration-0p30-0p60.json"
DEFAULT_OUT = ROOT / "tmp" / "glyph-anchor-green-circle-union-N-cell-0p30-0p60-with-red-fit.png"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default=str(DEFAULT_BASE))
    parser.add_argument("--fit-json", default=str(DEFAULT_FIT))
    parser.add_argument("--registration-json", default=str(DEFAULT_REG))
    parser.add_argument("--output", default=str(DEFAULT_OUT))
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    base = Image.open(args.base).convert("RGBA")
    fit = json.loads(Path(args.fit_json).read_text(encoding="utf-8"))
    reg = json.loads(Path(args.registration_json).read_text(encoding="utf-8"))

    scale = float(reg["registration"]["scale"])
    tx = float(reg["registration"]["tx"])
    ty = float(reg["registration"]["ty"])
    x0, y0, _, _ = reg["chemdraw_bbox"]

    fit_curve = [(float(item["x"]), float(item["y"])) for item in fit["fit"]]
    transformed = [((x - x0) * scale + tx, (y - y0) * scale + ty) for x, y in fit_curve]

    out = base.copy()
    draw = ImageDraw.Draw(out, "RGBA")
    draw.line(transformed + [transformed[0]], fill=(220, 38, 38, 255), width=2)

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    out.save(output_path)
    print(output_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
