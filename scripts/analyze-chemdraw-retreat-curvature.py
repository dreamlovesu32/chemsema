#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path

import numpy as np


ROOT = Path(__file__).resolve().parents[1]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--measurements",
        default=str(ROOT / "tmp" / "chemdraw-label-retreat-directional" / "measurements.json"),
    )
    parser.add_argument("--margin", type=float, default=2.5)
    parser.add_argument("--half-window", type=int, default=5)
    parser.add_argument("--max-rms", type=float, default=0.015)
    parser.add_argument("--target-glyph")
    parser.add_argument("--target-angle", type=int)
    return parser.parse_args()


def angular_distance(left: float, right: float) -> float:
    return abs((left - right + 180.0) % 360.0 - 180.0)


def fit_circle(points: np.ndarray) -> tuple[float, float, float, float]:
    # x^2 + y^2 = 2*cx*x + 2*cy*y + k
    matrix = np.column_stack((2.0 * points[:, 0], 2.0 * points[:, 1], np.ones(len(points))))
    target = np.square(points[:, 0]) + np.square(points[:, 1])
    cx, cy, constant = np.linalg.lstsq(matrix, target, rcond=None)[0]
    radius = math.sqrt(max(0.0, constant + cx * cx + cy * cy))
    residuals = np.hypot(points[:, 0] - cx, points[:, 1] - cy) - radius
    return float(cx), float(cy), radius, float(np.sqrt(np.mean(np.square(residuals))))


def quantiles(values: list[float]) -> dict[str, float] | None:
    if not values:
        return None
    array = np.asarray(values, dtype=float)
    return {
        "count": len(values),
        "p05": float(np.quantile(array, 0.05)),
        "p25": float(np.quantile(array, 0.25)),
        "p50": float(np.quantile(array, 0.50)),
        "p75": float(np.quantile(array, 0.75)),
        "p95": float(np.quantile(array, 0.95)),
    }


def main() -> None:
    args = parse_args()
    payload = json.loads(Path(args.measurements).read_text(encoding="utf-8"))
    rows = [
        row
        for row in payload["measurements"]
        if row["font"] == "Arial"
        and row["size"] == 10
        and row["face"] == 0
        and row["lineWidth"] == 0.05
        and row["marginWidth"] == args.margin
        and len(row["glyph"]) == 1
    ]
    by_glyph: dict[str, dict[int, float]] = {}
    for row in rows:
        by_glyph.setdefault(row["glyph"], {})[int(row["angleDeg"])] = float(row["retreat"])

    fits = []
    for glyph, retreat_by_angle in by_glyph.items():
        for center_angle in range(360):
            if min(angular_distance(center_angle, axis) for axis in (0, 90, 180, 270)) <= 12:
                continue
            angles = [
                (center_angle + offset) % 360
                for offset in range(-args.half_window, args.half_window + 1)
            ]
            retreats = [retreat_by_angle.get(angle, 0.0) for angle in angles]
            if min(retreats) <= 0.2:
                continue
            points = np.asarray([
                (
                    retreat * math.cos(math.radians(angle)),
                    retreat * math.sin(math.radians(angle)),
                )
                for angle, retreat in zip(angles, retreats)
            ])
            cx, cy, radius, rms = fit_circle(points)
            if rms <= args.max_rms and 0.1 <= radius <= 12.0:
                fits.append({
                    "glyph": glyph,
                    "angle": center_angle,
                    "center": [cx, cy],
                    "radius": radius,
                    "radiusOverMargin": radius / args.margin,
                    "rms": rms,
                })

    near_natural = [fit for fit in fits if 0.7 <= fit["radiusOverMargin"] <= 1.3]
    near_reinforced = [fit for fit in fits if 1.7 <= fit["radiusOverMargin"] <= 2.3]
    result = {
        "margin": args.margin,
        "windowDegrees": args.half_window * 2 + 1,
        "maxRmsPt": args.max_rms,
        "allRadiusOverMargin": quantiles([fit["radiusOverMargin"] for fit in fits]),
        "naturalCount": len(near_natural),
        "reinforcedCount": len(near_reinforced),
        "bestNatural": sorted(near_natural, key=lambda fit: fit["rms"])[:30],
        "bestReinforced": sorted(near_reinforced, key=lambda fit: fit["rms"])[:30],
    }
    if args.target_glyph is not None and args.target_angle is not None:
        target = next(
            (
                fit
                for fit in fits
                if fit["glyph"] == args.target_glyph and fit["angle"] == args.target_angle % 360
            ),
            None,
        )
        result["targetFit"] = target
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
