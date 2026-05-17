from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image


def load_mask(path: Path, threshold: int) -> tuple[list[list[bool]], int, int]:
    image = Image.open(path).convert("RGBA")
    width, height = image.size
    pixels = image.load()
    mask = [[False] * width for _ in range(height)]
    for y in range(height):
        row = mask[y]
        for x in range(width):
            r, g, b, a = pixels[x, y]
            row[x] = a > 0 and (r + g + b) < threshold
    return mask, width, height


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Partition residual pixels by disjoint layer masks."
    )
    parser.add_argument("ours_png")
    parser.add_argument("reference_png")
    parser.add_argument("text_png")
    parser.add_argument("knockout_png")
    parser.add_argument("nontext_png")
    parser.add_argument("output_json")
    parser.add_argument("--dx", type=int, default=0)
    parser.add_argument("--dy", type=int, default=0)
    parser.add_argument("--threshold", type=int, default=740)
    args = parser.parse_args()

    ours, width, height = load_mask(Path(args.ours_png), args.threshold)
    reference, rw, rh = load_mask(Path(args.reference_png), args.threshold)
    text, tw, th = load_mask(Path(args.text_png), args.threshold)
    knockout, kw, kh = load_mask(Path(args.knockout_png), args.threshold)
    nontext, nw, nh = load_mask(Path(args.nontext_png), args.threshold)
    if len({(width, height), (rw, rh), (tw, th), (kw, kh), (nw, nh)}) != 1:
        raise SystemExit("PNG sizes must match")

    dx = args.dx
    dy = args.dy
    residual = 0
    overlaps = {
        "text": 0,
        "knockout": 0,
        "nontext": 0,
        "halo_only": 0,
        "knockout_any": 0,
        "molecule_any": 0,
        "unexplained": 0,
    }
    for y in range(height):
        for x in range(width):
            xa = x + dx
            ya = y + dy
            ours_value = ours[ya][xa] if 0 <= xa < width and 0 <= ya < height else False
            ref_value = reference[y][x]
            if not (ours_value ^ ref_value):
                continue
            residual += 1

            text_px = text[y][x]
            knockout_px = knockout[y][x]
            nontext_px = nontext[y][x]
            molecule_any = text_px or knockout_px or nontext_px
            if text_px:
                overlaps["text"] += 1
            if knockout_px:
                overlaps["knockout"] += 1
                overlaps["knockout_any"] += 1
            if nontext_px:
                overlaps["nontext"] += 1
            if molecule_any:
                overlaps["molecule_any"] += 1
            halo_only = knockout_px and not text_px
            if halo_only:
                overlaps["halo_only"] += 1
            if not molecule_any:
                overlaps["unexplained"] += 1

    output = {
        "oursPng": str(Path(args.ours_png).resolve()),
        "referencePng": str(Path(args.reference_png).resolve()),
        "textPng": str(Path(args.text_png).resolve()),
        "knockoutPng": str(Path(args.knockout_png).resolve()),
        "nontextPng": str(Path(args.nontext_png).resolve()),
        "dx": dx,
        "dy": dy,
        "threshold": args.threshold,
        "residualPixelCount": residual,
        "overlaps": overlaps,
        "ratios": {k: (v / residual if residual else 0.0) for k, v in overlaps.items()},
    }
    Path(args.output_json).write_text(json.dumps(output, indent=2), encoding="utf-8")
    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
