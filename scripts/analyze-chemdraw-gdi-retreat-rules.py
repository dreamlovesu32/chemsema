#!/usr/bin/env python3

from __future__ import annotations

import argparse
import ctypes
import json
import math
from collections import defaultdict
from ctypes import wintypes
from pathlib import Path

import numpy as np
from scipy import ndimage


ROOT = Path(__file__).resolve().parents[1]
FONT_SIZE_PT = 10.0
BOND_LENGTH_PT = 32.0

GGO_GRAY8_BITMAP = 6
GDI_ERROR = 0xFFFFFFFF
DEFAULT_CHARSET = 1
ANTIALIASED_QUALITY = 4
FW_NORMAL = 400


class FIXED(ctypes.Structure):
    _fields_ = [("fract", wintypes.WORD), ("value", ctypes.c_short)]


class MAT2(ctypes.Structure):
    _fields_ = [("eM11", FIXED), ("eM12", FIXED), ("eM21", FIXED), ("eM22", FIXED)]


class POINT(ctypes.Structure):
    _fields_ = [("x", wintypes.LONG), ("y", wintypes.LONG)]


class GLYPHMETRICS(ctypes.Structure):
    _fields_ = [
        ("gmBlackBoxX", wintypes.UINT),
        ("gmBlackBoxY", wintypes.UINT),
        ("gmptGlyphOrigin", POINT),
        ("gmCellIncX", ctypes.c_short),
        ("gmCellIncY", ctypes.c_short),
    ]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--measurements",
        default=str(ROOT / "tmp" / "chemdraw-label-retreat-comprehensive" / "measurements.json"),
    )
    parser.add_argument(
        "--output",
        default=str(ROOT / "tmp" / "chemdraw-label-retreat-comprehensive" / "gdi-rule-analysis.json"),
    )
    return parser.parse_args()


def gdi_gray_glyph(character: str, font_height_px: int):
    gdi32 = ctypes.WinDLL("gdi32", use_last_error=True)
    gdi32.CreateCompatibleDC.argtypes = [wintypes.HDC]
    gdi32.CreateCompatibleDC.restype = wintypes.HDC
    gdi32.CreateFontW.argtypes = [
        ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int,
        wintypes.DWORD, wintypes.DWORD, wintypes.DWORD, wintypes.DWORD,
        wintypes.DWORD, wintypes.DWORD, wintypes.DWORD, wintypes.DWORD,
        wintypes.LPCWSTR,
    ]
    gdi32.CreateFontW.restype = wintypes.HFONT
    gdi32.SelectObject.argtypes = [wintypes.HDC, wintypes.HGDIOBJ]
    gdi32.SelectObject.restype = wintypes.HGDIOBJ
    gdi32.GetGlyphOutlineW.argtypes = [
        wintypes.HDC, wintypes.UINT, wintypes.UINT, ctypes.POINTER(GLYPHMETRICS),
        wintypes.DWORD, wintypes.LPVOID, ctypes.POINTER(MAT2),
    ]
    gdi32.GetGlyphOutlineW.restype = wintypes.DWORD
    gdi32.DeleteObject.argtypes = [wintypes.HGDIOBJ]
    gdi32.DeleteDC.argtypes = [wintypes.HDC]

    dc = gdi32.CreateCompatibleDC(None)
    if not dc:
        raise ctypes.WinError(ctypes.get_last_error())
    font = gdi32.CreateFontW(
        -font_height_px, 0, 0, 0, FW_NORMAL, 0, 0, 0,
        DEFAULT_CHARSET, 0, 0, ANTIALIASED_QUALITY, 0, "Arial",
    )
    if not font:
        gdi32.DeleteDC(dc)
        raise ctypes.WinError(ctypes.get_last_error())
    previous = gdi32.SelectObject(dc, font)
    metrics = GLYPHMETRICS()
    identity = MAT2(FIXED(0, 1), FIXED(0, 0), FIXED(0, 0), FIXED(0, 1))
    size = gdi32.GetGlyphOutlineW(
        dc, ord(character), GGO_GRAY8_BITMAP, ctypes.byref(metrics), 0, None, ctypes.byref(identity),
    )
    if size == GDI_ERROR:
        gdi32.SelectObject(dc, previous)
        gdi32.DeleteObject(font)
        gdi32.DeleteDC(dc)
        raise ctypes.WinError(ctypes.get_last_error())
    buffer = (ctypes.c_ubyte * size)()
    result = gdi32.GetGlyphOutlineW(
        dc, ord(character), GGO_GRAY8_BITMAP, ctypes.byref(metrics), size, buffer, ctypes.byref(identity),
    )
    gdi32.SelectObject(dc, previous)
    gdi32.DeleteObject(font)
    gdi32.DeleteDC(dc)
    if result == GDI_ERROR:
        raise ctypes.WinError(ctypes.get_last_error())
    width = int(metrics.gmBlackBoxX)
    height = int(metrics.gmBlackBoxY)
    pitch = (width + 3) & ~3
    gray = np.frombuffer(buffer, dtype=np.uint8).reshape((height, pitch))[:, :width].copy()
    return gray, metrics


def canvas_for_glyph(gray: np.ndarray, metrics: GLYPHMETRICS, font_height_px: int):
    padding = max(16, font_height_px)
    height, width = gray.shape
    canvas = np.zeros((height + padding * 2, width + padding * 2), dtype=np.uint8)
    canvas[padding:padding + height, padding:padding + width] = gray
    baseline_x = padding - int(metrics.gmptGlyphOrigin.x)
    baseline_y = padding + int(metrics.gmptGlyphOrigin.y)
    anchor = (
        baseline_x + float(metrics.gmCellIncX) * 0.5,
        baseline_y - font_height_px * 0.39,
    )
    return canvas, anchor


def pixel_retreat(mask, anchor, angle_deg, line_width_pt, px_per_pt, pixel_model):
    ys, xs = np.nonzero(mask)
    if not len(xs):
        return 0.0
    dx = xs.astype(float) + 0.5 - anchor[0]
    dy = ys.astype(float) + 0.5 - anchor[1]
    radians = math.radians(angle_deg)
    cosine = math.cos(radians)
    sine = math.sin(radians)
    projection = dx * cosine + dy * sine
    perpendicular = -dx * sine + dy * cosine
    if pixel_model == "square":
        support = 0.5 * (abs(cosine) + abs(sine))
    else:
        support = 0.0
    touched = (
        (projection + support >= 0.0)
        & (projection - support <= BOND_LENGTH_PT * px_per_pt)
        & (np.abs(perpendicular) <= line_width_pt * px_per_pt * 0.5 + support)
    )
    if not np.any(touched):
        return 0.0
    return max(0.0, float(np.max(projection[touched] + support) / px_per_pt))


def pixel_retreat_grid(mask, anchor, angle_degs, line_width_pt, px_per_pt):
    ys, xs = np.nonzero(mask)
    if not len(xs):
        zeros = np.zeros(len(angle_degs), dtype=float)
        return {"center": zeros, "square": zeros}
    dx = (xs.astype(np.float32) + 0.5 - anchor[0])[:, None]
    dy = (ys.astype(np.float32) + 0.5 - anchor[1])[:, None]
    radians = np.radians(np.asarray(angle_degs, dtype=np.float32))[None, :]
    cosine = np.cos(radians)
    sine = np.sin(radians)
    projection = dx * cosine + dy * sine
    perpendicular = -dx * sine + dy * cosine
    results = {}
    for pixel_model in ["center", "square"]:
        support = 0.5 * (np.abs(cosine) + np.abs(sine)) if pixel_model == "square" else 0.0
        touched = (
            (projection + support >= 0.0)
            & (projection - support <= BOND_LENGTH_PT * px_per_pt)
            & (np.abs(perpendicular) <= line_width_pt * px_per_pt * 0.5 + support)
        )
        exits = np.max(np.where(touched, projection + support, 0.0), axis=0) / px_per_pt
        results[pixel_model] = np.maximum(0.0, exits)
    return results


def error_summary(errors):
    absolute = np.abs(np.asarray(errors, dtype=float))
    signed = np.asarray(errors, dtype=float)
    return {
        "count": len(errors),
        "maePt": float(absolute.mean()),
        "p50Pt": float(np.quantile(absolute, 0.50)),
        "p95Pt": float(np.quantile(absolute, 0.95)),
        "maxPt": float(absolute.max()),
        "biasPt": float(signed.mean()),
    }


def main() -> None:
    args = parse_args()
    source = json.loads(Path(args.measurements).read_text(encoding="utf-8"))
    targets = [
        entry for entry in source["measurements"]
        if entry["font"] == "Arial"
        and entry["face"] == 0
        and entry["size"] == FONT_SIZE_PT
        and entry["lineWidth"] == 0.05
        and len(entry["glyph"]) == 1
        and ord(entry["glyph"]) < 128
    ]
    by_glyph = defaultdict(list)
    for entry in targets:
        by_glyph[entry["glyph"]].append(entry)

    variants = [
        (font_height, threshold, pixel_model)
        for font_height in [13, 20, 40, 80, 160, 240]
        for threshold in [1, 32, 64]
        for pixel_model in ["center", "square"]
    ]
    errors = {variant: [] for variant in variants}
    errors_by_glyph = {variant: defaultdict(list) for variant in variants}
    observations = []
    for character, entries in by_glyph.items():
        glyphs = {}
        for font_height in sorted({variant[0] for variant in variants}):
            gray, metrics = gdi_gray_glyph(character, font_height)
            canvas, anchor = canvas_for_glyph(gray, metrics, font_height)
            glyphs[font_height] = (canvas, anchor)
        mask_cache = {}
        prediction_cache = {}
        margins = sorted({entry["marginWidth"] for entry in entries})
        angles = sorted({entry["angleDeg"] for entry in entries})
        for font_height in sorted({variant[0] for variant in variants}):
            canvas, anchor = glyphs[font_height]
            px_per_pt = font_height / FONT_SIZE_PT
            for threshold in sorted({variant[1] for variant in variants}):
                ink = canvas >= threshold
                distance = ndimage.distance_transform_edt(~ink)
                for margin in margins:
                    expanded = distance <= margin * px_per_pt + 1e-9
                    grid = pixel_retreat_grid(expanded, anchor, angles, entries[0]["lineWidth"], px_per_pt)
                    for pixel_model, exits in grid.items():
                        for angle, predicted in zip(angles, exits):
                            prediction_cache[(font_height, threshold, pixel_model, margin, angle)] = float(predicted)
        for entry in entries:
            observations.append(entry)
            for variant in variants:
                font_height, threshold, pixel_model = variant
                predicted = prediction_cache[(
                    font_height,
                    threshold,
                    pixel_model,
                    entry["marginWidth"],
                    entry["angleDeg"],
                )]
                error = predicted - entry["retreat"]
                errors[variant].append(error)
                errors_by_glyph[variant][character].append(error)

    ranked = sorted(({
        "fontHeightPx": font_height,
        "threshold64": threshold,
        "pixelModel": pixel_model,
        **error_summary(errors[(font_height, threshold, pixel_model)]),
    } for font_height, threshold, pixel_model in variants), key=lambda result: (result["maePt"], result["p95Pt"]))
    winner = ranked[0]
    winner_key = (winner["fontHeightPx"], winner["threshold64"], winner["pixelModel"])
    result = {
        "schema": "chemsema.chemdraw-gdi-retreat-rule-analysis.v1",
        "measurementCount": len(targets),
        "glyphCount": len(by_glyph),
        "ranked": ranked,
        "winnerWorstGlyphs": sorted((
            {"glyph": glyph, **error_summary(values)}
            for glyph, values in errors_by_glyph[winner_key].items()
        ), key=lambda item: item["maePt"], reverse=True)[:20],
    }
    output = Path(args.output)
    output.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"output": str(output.resolve()), "best": ranked[:15], "worst": result["winnerWorstGlyphs"]}, ensure_ascii=True, indent=2))


if __name__ == "__main__":
    main()
