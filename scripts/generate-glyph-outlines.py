#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
from pathlib import Path

from fontTools.pens.recordingPen import RecordingPen
from fontTools.ttLib import TTFont


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FONT = Path(r"C:\Windows\Fonts\arial.ttf")
DEFAULT_GLYPH_PROFILES = ROOT / "shared" / "glyph_profiles.json"
DEFAULT_OUTPUT = ROOT / "shared" / "glyph_outlines.json"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--font", default=str(DEFAULT_FONT))
    parser.add_argument("--glyph-profiles", default=str(DEFAULT_GLYPH_PROFILES))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    return parser.parse_args()


def point_em(point: tuple[float, float], units_per_em: int) -> list[float]:
    x, y = point
    return [round(x / units_per_em, 8), round(-y / units_per_em, 8)]


def command_payload(op: str, points: tuple[tuple[float, float], ...], units_per_em: int) -> dict:
    return {
        "op": op,
        "points": [point_em(point, units_per_em) for point in points if point is not None],
    }


def glyph_outline(ttfont: TTFont, ch: str) -> dict:
    cmap = ttfont.getBestCmap()
    glyph_name = cmap.get(ord(ch))
    if not glyph_name:
        raise RuntimeError(f"missing glyph for {ch!r}")
    glyph_set = ttfont.getGlyphSet()
    glyph = glyph_set[glyph_name]
    units_per_em = ttfont["head"].unitsPerEm
    bounds = ttfont["glyf"][glyph_name]

    pen = RecordingPen()
    glyph.draw(pen)
    commands = []
    for op, points in pen.value:
        if op == "moveTo":
            commands.append(command_payload("M", points, units_per_em))
        elif op == "lineTo":
            commands.append(command_payload("L", points, units_per_em))
        elif op == "qCurveTo":
            commands.append(command_payload("Q", points, units_per_em))
        elif op == "curveTo":
            commands.append(command_payload("C", points, units_per_em))
        elif op in {"closePath", "endPath"}:
            commands.append({"op": "Z", "points": []})

    return {
        "advanceEm": round(glyph.width / units_per_em, 8),
        "boundsEm": [
            round(bounds.xMin / units_per_em, 8),
            round(-bounds.yMax / units_per_em, 8),
            round(bounds.xMax / units_per_em, 8),
            round(-bounds.yMin / units_per_em, 8),
        ],
        "commands": commands,
    }


def main() -> None:
    args = parse_args()
    glyph_profiles = json.loads(Path(args.glyph_profiles).read_text(encoding="utf-8"))
    chars = [
        ch
        for ch, profile in glyph_profiles["specials"].items()
        if profile.get("visible", True)
    ]
    ttfont = TTFont(str(args.font))
    glyphs = {}
    for ch in chars:
        try:
            glyphs[ch] = glyph_outline(ttfont, ch)
        except Exception as error:  # noqa: BLE001
            print(f"skip {ch!r}: {error}")

    payload = {
        "version": 1,
        "sourceFont": str(args.font),
        "unitsPerEm": ttfont["head"].unitsPerEm,
        "glyphs": glyphs,
    }
    Path(args.output).write_text(
        json.dumps(payload, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
