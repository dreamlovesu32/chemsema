#!/usr/bin/env python
from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage
from scipy.interpolate import splprep, splev


@dataclass
class Component:
    label: int
    area: int
    x0: int
    y0: int
    x1: int
    y1: int

    @property
    def cx(self) -> float:
        return (self.x0 + self.x1) / 2

    @property
    def cy(self) -> float:
        return (self.y0 + self.y1) / 2

    def contains(self, x: float, y: float) -> bool:
        return self.x0 <= x <= self.x1 and self.y0 <= y <= self.y1


def load_components(mask: np.ndarray) -> tuple[np.ndarray, list[Component]]:
    labels, _ = ndimage.label(mask)
    slices = ndimage.find_objects(labels)
    components: list[Component] = []
    for label, slc in enumerate(slices, start=1):
        if slc is None:
            continue
        area = int((labels[slc] == label).sum())
        y0, y1 = slc[0].start, slc[0].stop
        x0, x1 = slc[1].start, slc[1].stop
        components.append(Component(label, area, x0, y0, x1, y1))
    return labels, components


def pick_star_components(components: list[Component], width: int, height: int) -> list[Component]:
    x_lo, x_hi = width * 0.25, width * 0.75
    y_lo, y_hi = height * 0.20, height * 0.85
    return [
        comp
        for comp in components
        if comp.area >= 500 and x_lo <= comp.cx <= x_hi and y_lo <= comp.cy <= y_hi
    ]


def sample_inner_cap_midpoints(
    labels: np.ndarray,
    star_components: list[Component],
) -> tuple[tuple[float, float], Component, list[dict[str, float]]]:
    x0 = min(comp.x0 for comp in star_components)
    y0 = min(comp.y0 for comp in star_components)
    x1 = max(comp.x1 for comp in star_components)
    y1 = max(comp.y1 for comp in star_components)

    cx0 = (x0 + x1) / 2
    cy0 = (y0 + y1) / 2

    center_component = min(
        (comp for comp in star_components if comp.contains(cx0, cy0)),
        key=lambda comp: comp.area,
    )

    points: list[dict[str, float]] = []
    for comp in star_components:
        if comp.label == center_component.label:
            continue
        ys, xs = np.where(labels[comp.y0 : comp.y1, comp.x0 : comp.x1] == comp.label)
        xs = xs + comp.x0
        ys = ys + comp.y0
        distances = np.hypot(xs - cx0, ys - cy0)
        d_min = float(distances.min())
        # Take the closest black pixels on the inner flat cap and average them.
        keep = distances <= d_min + 1.5
        px = float(xs[keep].mean())
        py = float(ys[keep].mean())
        angle = float(np.arctan2(py - cy0, px - cx0))
        points.append(
            {
                "label": comp.label,
                "x": px,
                "y": py,
                "distance_from_initial_center": d_min,
                "angle": angle,
            }
        )

    points.sort(key=lambda item: item["angle"])

    opposite_midpoints = []
    half = len(points) // 2
    for index in range(half):
        first = points[index]
        second = points[index + half]
        opposite_midpoints.append(((first["x"] + second["x"]) / 2, (first["y"] + second["y"]) / 2))

    cx = float(np.mean([mid[0] for mid in opposite_midpoints]))
    cy = float(np.mean([mid[1] for mid in opposite_midpoints]))

    for point in points:
        point["radius"] = float(np.hypot(point["x"] - cx, point["y"] - cy))
        point["angle_from_refined_center"] = float(np.arctan2(point["y"] - cy, point["x"] - cx))

    return (cx, cy), center_component, points


def periodic_fit(points: list[dict[str, float]], smoothness: float) -> tuple[np.ndarray, np.ndarray]:
    xy = np.array([[point["x"], point["y"]] for point in points], dtype=float)
    closed = np.vstack([xy, xy[0]])
    tck, _ = splprep([closed[:, 0], closed[:, 1]], s=smoothness, per=True)
    u = np.linspace(0.0, 1.0, 720)
    x_fit, y_fit = splev(u, tck)
    return np.asarray(x_fit), np.asarray(y_fit)


def render_overlay(
    image: Image.Image,
    star_components: list[Component],
    center: tuple[float, float],
    points: list[dict[str, float]],
    x_fit: np.ndarray,
    y_fit: np.ndarray,
    output_path: Path,
) -> None:
    margin = 36
    x0 = min(comp.x0 for comp in star_components) - margin
    y0 = min(comp.y0 for comp in star_components) - margin
    x1 = max(comp.x1 for comp in star_components) + margin
    y1 = max(comp.y1 for comp in star_components) + margin

    crop = image.crop((x0, y0, x1, y1)).convert("RGB")
    draw = ImageDraw.Draw(crop, "RGBA")

    fit_path = [(float(x - x0), float(y - y0)) for x, y in zip(x_fit, y_fit)]
    draw.polygon(fit_path, fill=(80, 170, 255, 48))
    draw.line(fit_path + [fit_path[0]], fill=(40, 120, 255, 220), width=3)

    cx, cy = center
    draw.ellipse((cx - x0 - 4, cy - y0 - 4, cx - x0 + 4, cy - y0 + 4), fill=(255, 140, 0, 255))
    for point in points:
        px = point["x"] - x0
        py = point["y"] - y0
        draw.ellipse((px - 3, py - 3, px + 3, py + 3), fill=(220, 0, 0, 255))

    crop.save(output_path)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True, help="Raster screenshot path")
    parser.add_argument("--png-output", required=True, help="Overlay output PNG path")
    parser.add_argument("--json-output", required=True, help="Point/fit output JSON path")
    parser.add_argument("--threshold", type=int, default=100)
    parser.add_argument("--smoothness", type=float, default=0.2)
    args = parser.parse_args()

    image = Image.open(args.input).convert("L")
    pixels = np.array(image)
    mask = pixels < args.threshold
    labels, components = load_components(mask)
    star_components = pick_star_components(components, image.width, image.height)
    if len(star_components) != 25:
        raise RuntimeError(f"Expected 25 central components (24 bonds + N), found {len(star_components)}")

    center, center_component, points = sample_inner_cap_midpoints(labels, star_components)
    x_fit, y_fit = periodic_fit(points, smoothness=args.smoothness)

    render_overlay(
        image=Image.open(args.input),
        star_components=star_components,
        center=center,
        points=points,
        x_fit=x_fit,
        y_fit=y_fit,
        output_path=Path(args.png_output),
    )

    radii = [point["radius"] for point in points]
    payload = {
        "input": args.input,
        "threshold": args.threshold,
        "smoothness": args.smoothness,
        "center_component": {
            "label": center_component.label,
            "area": center_component.area,
            "bbox": [center_component.x0, center_component.y0, center_component.x1, center_component.y1],
        },
        "center": {"x": center[0], "y": center[1]},
        "radius_summary": {
            "min": float(np.min(radii)),
            "max": float(np.max(radii)),
            "mean": float(np.mean(radii)),
        },
        "points": points,
        "fit": [{"x": float(x), "y": float(y)} for x, y in zip(x_fit, y_fit)],
    }
    Path(args.json_output).write_text(json.dumps(payload, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
