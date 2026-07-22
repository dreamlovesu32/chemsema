#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import math
from collections import defaultdict
from pathlib import Path

import numpy as np
from fontTools.ttLib import TTFont
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage
from skimage.morphology import convex_hull_image


ROOT = Path(__file__).resolve().parents[1]
FONT_PATH = Path(r"C:\Windows\Fonts\arial.ttf")
FONT_SIZE_PX = 240
FONT_SIZE_PT = 10.0
PX_PER_PT = FONT_SIZE_PX / FONT_SIZE_PT
PADDING = 160


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--measurements",
        default=str(ROOT / "tmp" / "chemdraw-label-retreat-comprehensive" / "measurements.json"),
    )
    parser.add_argument(
        "--output",
        default=str(ROOT / "tmp" / "chemdraw-label-retreat-comprehensive" / "rule-analysis.json"),
    )
    return parser.parse_args()


def render_mask(
    font: ImageFont.FreeTypeFont,
    character: str,
) -> tuple[np.ndarray, tuple[int, int, int, int], tuple[float, float]]:
    left, top, right, bottom = font.getbbox(character, anchor="ls")
    width = max(PADDING * 2 + right - left + 4, 512)
    height = max(PADDING * 2 + bottom - top + 4, 512)
    origin = (PADDING - left, PADDING - top)
    image = Image.new("L", (width, height), 0)
    ImageDraw.Draw(image).text(origin, character, font=font, fill=255, anchor="ls")
    mask = np.asarray(image) >= 128
    ys, xs = np.nonzero(mask)
    if not len(xs):
        raise ValueError(f"No glyph pixels for {character!r}")
    bbox = (int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1)
    # ChemDraw centers a single-character label by advance width, while its
    # atom/node y is tied to the text baseline rather than each glyph ink box.
    anchor = (
        origin[0] + float(font.getlength(character)) * 0.5,
        origin[1] - FONT_SIZE_PX * 0.39,
    )
    return mask, bbox, anchor


def expanded_mask(
    distance: np.ndarray,
    margin_width: float,
    margin_scale: float,
    base_outset_pt: float = 0.0,
) -> np.ndarray:
    return distance <= (base_outset_pt + margin_width * margin_scale) * PX_PER_PT + 1e-9


def strip_exit(
    mask: np.ndarray,
    anchor: tuple[float, float],
    angle_deg: float,
    line_width_pt: float,
    node_circle_pt: float,
) -> float:
    radians = math.radians(angle_deg)
    cosine = math.cos(radians)
    sine = math.sin(radians)
    max_distance_px = math.hypot(mask.shape[0], mask.shape[1])
    distances_px = np.arange(0.0, max_distance_px, 0.5)
    half_width_px = line_width_pt * PX_PER_PT * 0.5
    touched = np.zeros(distances_px.shape, dtype=bool)
    for side in [-1.0, 0.0, 1.0]:
        offset = side * half_width_px
        xs = np.rint(anchor[0] + distances_px * cosine - offset * sine).astype(int)
        ys = np.rint(anchor[1] + distances_px * sine + offset * cosine).astype(int)
        valid = (xs >= 0) & (ys >= 0) & (xs < mask.shape[1]) & (ys < mask.shape[0])
        touched[valid] |= mask[ys[valid], xs[valid]]
    glyph_exit = float(distances_px[touched].max(initial=0.0) / PX_PER_PT)
    return max(glyph_exit, node_circle_pt)


def axial_contact_projection(
    bbox: tuple[int, int, int, int],
    anchor: tuple[float, float],
    margin_width: float,
    angle_deg: float,
    half_sector_deg: float,
) -> float:
    left, top, right, bottom = bbox
    min_x = (left - anchor[0]) / PX_PER_PT
    max_x = (right - 1 - anchor[0]) / PX_PER_PT
    min_y = (top - anchor[1]) / PX_PER_PT
    max_y = (bottom - 1 - anchor[1]) / PX_PER_PT
    radians = math.radians(angle_deg)
    direction = (math.cos(radians), math.sin(radians))
    contacts = [
        (0.0, (max_x + margin_width, 0.0)),
        (90.0, (0.0, max_y + margin_width)),
        (180.0, (min_x - margin_width, 0.0)),
        (270.0, (0.0, min_y - margin_width)),
    ]
    candidates = []
    for axis_deg, point in contacts:
        difference = abs((angle_deg - axis_deg + 180.0) % 360.0 - 180.0)
        if difference < half_sector_deg:
            candidates.append(point[0] * direction[0] + point[1] * direction[1])
    return max([0.0, *candidates])


def error_summary(errors: list[float]) -> dict:
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
    cmap = TTFont(str(FONT_PATH)).getBestCmap()
    targets = [
        entry
        for entry in source["measurements"]
        if entry["font"] == "Arial"
        and entry["face"] == 0
        and entry["size"] == FONT_SIZE_PT
        and entry["lineWidth"] == 0.05
        and len(entry["glyph"]) == 1
        and ord(entry["glyph"]) in cmap
    ]
    by_glyph: dict[str, list[dict]] = defaultdict(list)
    for entry in targets:
        by_glyph[entry["glyph"]].append(entry)

    shape_variants = []
    variants = []
    for metric in ["euclidean", "chessboard", "taxicab"]:
        for margin_scale in [0.9, 1.0, 1.05, 1.1, 1.15, 1.2, 1.25]:
            shape_variants.append((metric, margin_scale))
            for circle_base in [0.0, 0.25, 0.5, 0.75, 1.0]:
                variants.append((metric, margin_scale, circle_base))
    morphology_rules = ["fill-holes", "convex-hull"] + [
        f"close-{radius}" for radius in [0.5, 1.0, 1.5, 2.0, 2.5, 3.0]
    ]
    for rule in morphology_rules:
        for margin_scale in [1.0, 1.1]:
            shape_variants.append((rule, margin_scale))
            variants.append((rule, margin_scale, 0.0))
    base_outset_by_rule = {f"base-outset-{radius}": radius for radius in [0.1, 0.25, 0.5, 0.75, 1.0, 1.5]}
    for rule in base_outset_by_rule:
        for margin_scale in [0.9, 1.0, 1.1]:
            shape_variants.append((rule, margin_scale))
            variants.append((rule, margin_scale, 0.0))
    for half_sector in [9.0, 9.5, 10.0, 10.5]:
        rule = f"euclidean-axial-{half_sector}"
        shape_variants.append((rule, 1.0))
        variants.append((rule, 1.0, 0.0))
    for base_metric in ["chessboard", "taxicab"]:
        rule = f"{base_metric}-axial-9.5"
        shape_variants.append((rule, 1.0))
        variants.append((rule, 1.0, 0.0))
    errors: dict[tuple[str, float, float], list[float]] = {variant: [] for variant in variants}
    errors_by_glyph: dict[tuple[str, float, float], dict[str, list[float]]] = {
        variant: defaultdict(list) for variant in variants
    }
    errors_by_margin: dict[tuple[str, float, float], dict[float, list[float]]] = {
        variant: defaultdict(list) for variant in variants
    }
    observation_keys: list[dict] = []
    font = ImageFont.truetype(str(FONT_PATH), FONT_SIZE_PX)

    for character, entries in by_glyph.items():
        mask, bbox, anchor = render_mask(font, character)
        distances = {
            "euclidean": ndimage.distance_transform_edt(~mask),
            "chessboard": ndimage.distance_transform_cdt(~mask, metric="chessboard").astype(float),
            "taxicab": ndimage.distance_transform_cdt(~mask, metric="taxicab").astype(float),
        }
        distances["fill-holes"] = ndimage.distance_transform_edt(~ndimage.binary_fill_holes(mask))
        distances["convex-hull"] = ndimage.distance_transform_edt(~convex_hull_image(mask))
        distance_to_ink = distances["euclidean"]
        for radius_pt in [0.5, 1.0, 1.5, 2.0, 2.5, 3.0]:
            radius_px = radius_pt * PX_PER_PT
            dilated = distance_to_ink <= radius_px
            closed = ndimage.distance_transform_edt(dilated) > radius_px
            distances[f"close-{radius_pt}"] = ndimage.distance_transform_edt(~closed)
        for rule in base_outset_by_rule:
            distances[rule] = distance_to_ink
        for half_sector in [9.0, 9.5, 10.0, 10.5]:
            distances[f"euclidean-axial-{half_sector}"] = distance_to_ink
        distances["chessboard-axial-9.5"] = distances["chessboard"]
        distances["taxicab-axial-9.5"] = distances["taxicab"]
        mask_cache: dict[tuple[str, float, float], np.ndarray] = {}
        for entry in entries:
            observation_keys.append({
                "glyph": character,
                "marginWidth": entry["marginWidth"],
                "angleDeg": entry["angleDeg"],
                "retreat": entry["retreat"],
            })
            for metric, margin_scale in shape_variants:
                cache_key = (metric, margin_scale, entry["marginWidth"])
                expanded = mask_cache.get(cache_key)
                if expanded is None:
                    expanded = expanded_mask(
                        distances[metric],
                        entry["marginWidth"],
                        margin_scale,
                        base_outset_by_rule.get(metric, 0.0),
                    )
                    mask_cache[cache_key] = expanded
                glyph_exit = strip_exit(
                    expanded,
                    anchor,
                    entry["angleDeg"],
                    entry["lineWidth"],
                    0.0,
                )
                is_axial = "-axial-" in metric
                circle_bases = [0.0] if metric in morphology_rules or metric in base_outset_by_rule or is_axial else [0.0, 0.25, 0.5, 0.75, 1.0]
                for circle_base in circle_bases:
                    predicted = max(
                        glyph_exit,
                        circle_base + entry["marginWidth"] * margin_scale,
                    )
                    if is_axial:
                        predicted = max(
                            predicted,
                            axial_contact_projection(
                                bbox,
                                anchor,
                                entry["marginWidth"],
                                entry["angleDeg"],
                                float(metric.split("-")[-1]),
                            ),
                        )
                    error = predicted - entry["retreat"]
                    key = (metric, margin_scale, circle_base)
                    errors[key].append(error)
                    errors_by_glyph[key][character].append(error)
                    errors_by_margin[key][entry["marginWidth"]].append(error)

    ranked = sorted(
        (
            {
                "metric": metric,
                "marginScale": margin_scale,
                "nodeCircleBasePt": circle_base,
                **error_summary(values),
            }
            for (metric, margin_scale, circle_base), values in errors.items()
        ),
        key=lambda result: (result["maePt"], result["p95Pt"]),
    )
    winner = ranked[0]
    winner_key = (winner["metric"], winner["marginScale"], winner["nodeCircleBasePt"])
    by_glyph_ranked = sorted(
        (
            {"glyph": glyph, **error_summary(values)}
            for glyph, values in errors_by_glyph[winner_key].items()
        ),
        key=lambda result: result["maePt"],
        reverse=True,
    )
    worst_cases = sorted(
        (
            {**observation, "error": error, "predictedRetreat": observation["retreat"] + error}
            for observation, error in zip(observation_keys, errors[winner_key])
        ),
        key=lambda result: abs(result["error"]),
        reverse=True,
    )
    result = {
        "schema": "chemsema.chemdraw-retreat-rule-analysis.v1",
        "measurementCount": len(targets),
        "glyphCount": len(by_glyph),
        "font": str(FONT_PATH),
        "best": ranked[:12],
        "winnerByMarginWidth": {
            str(margin): error_summary(values)
            for margin, values in sorted(errors_by_margin[winner_key].items())
        },
        "winnerWorstGlyphs": by_glyph_ranked[:20],
        "winnerWorstCases": worst_cases[:100],
        "all": ranked,
    }
    Path(args.output).write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"output": str(Path(args.output).resolve()), **result, "all": None}, ensure_ascii=True, indent=2))


if __name__ == "__main__":
    main()
