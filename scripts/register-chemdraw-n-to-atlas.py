#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage
from skimage import measure


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_ATLAS_CELL = ROOT / "tmp" / "glyph-anchor-green-circle-union-N-cell.png"
DEFAULT_SCREENSHOT = ROOT / "tmp" / "chemdraw_window_latest.jpg"
DEFAULT_FIT_JSON = ROOT / "tmp" / "chemdraw-n-clip-fit-raster.json"
DEFAULT_OUTPUT = ROOT / "tmp" / "chemdraw-atlas-n-registration.png"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--atlas-cell", default=str(DEFAULT_ATLAS_CELL))
    parser.add_argument("--screenshot", default=str(DEFAULT_SCREENSHOT))
    parser.add_argument("--fit-json", default=str(DEFAULT_FIT_JSON))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    return parser.parse_args()


def union_bbox(mask: np.ndarray) -> list[int]:
    ys, xs = np.nonzero(mask)
    return [int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1]


def extract_atlas_n_mask(cell: Image.Image) -> tuple[np.ndarray, list[int]]:
    a = np.array(cell.convert("RGBA"))
    mask = (
        (a[:, :, 0] < 40)
        & (a[:, :, 1] < 40)
        & (a[:, :, 2] < 40)
        & (a[:, :, 3] > 200)
    )
    labels, _ = ndimage.label(mask)
    slices = ndimage.find_objects(labels)
    keep = np.zeros_like(mask, dtype=bool)
    for label, slc in enumerate(slices, start=1):
        if slc is None:
            continue
        y0, y1 = slc[0].start, slc[0].stop
        x0, x1 = slc[1].start, slc[1].stop
        area = int((labels[slc] == label).sum())
        if area < 400 or y1 < 60:
            continue
        keep |= labels == label
    return keep, union_bbox(keep)


def extract_atlas_union_mask(cell: Image.Image) -> np.ndarray:
    a = np.array(cell.convert("RGBA"))
    mask = (
        (a[:, :, 3] > 0)
        & (a[:, :, 0] < 245)
        & (a[:, :, 1] < 245)
        & (a[:, :, 2] < 245)
    )
    mask[:50, :] = False
    labels, _ = ndimage.label(mask)
    slices = ndimage.find_objects(labels)
    best_label = None
    best_area = -1
    for label, slc in enumerate(slices, start=1):
        if slc is None:
            continue
        area = int((labels[slc] == label).sum())
        if area > best_area:
            best_area = area
            best_label = label
    if best_label is None:
        raise RuntimeError("Failed to extract atlas union mask")
    return labels == best_label


def extract_chemdraw_n_mask(screenshot: Image.Image, bbox: list[int]) -> np.ndarray:
    gray = np.array(screenshot.convert("L"))
    x0, y0, x1, y1 = bbox
    crop = gray[y0:y1, x0:x1]
    return crop < 100


def place_mask(mask: np.ndarray, scale: float, tx: float, ty: float, out_shape: tuple[int, int]) -> np.ndarray:
    src = Image.fromarray((mask.astype(np.uint8) * 255), mode="L")
    scaled_w = max(1, int(round(src.width * scale)))
    scaled_h = max(1, int(round(src.height * scale)))
    scaled = src.resize((scaled_w, scaled_h), Image.Resampling.NEAREST)
    dst = Image.new("L", (out_shape[1], out_shape[0]), 0)
    dst.paste(scaled, (int(round(tx)), int(round(ty))))
    return np.array(dst) > 0


def iou(a: np.ndarray, b: np.ndarray) -> float:
    inter = np.logical_and(a, b).sum()
    union = np.logical_or(a, b).sum()
    return float(inter / union) if union else 0.0


def search_registration(atlas_mask: np.ndarray, chemdraw_mask: np.ndarray, atlas_bbox: list[int]) -> dict[str, float]:
    atlas_h, atlas_w = atlas_mask.shape
    target_w = atlas_bbox[2] - atlas_bbox[0]
    target_h = atlas_bbox[3] - atlas_bbox[1]
    base_scale = ((target_w / chemdraw_mask.shape[1]) * (target_h / chemdraw_mask.shape[0])) ** 0.5
    cx = (atlas_bbox[0] + atlas_bbox[2]) / 2
    cy = (atlas_bbox[1] + atlas_bbox[3]) / 2
    best = None

    def sweep(scale_values, dx_values, dy_values, seed=None):
        nonlocal best
        for scale in scale_values:
            sw = chemdraw_mask.shape[1] * scale
            sh = chemdraw_mask.shape[0] * scale
            if seed is None:
                tx_base = cx - sw / 2
                ty_base = cy - sh / 2
            else:
                tx_base = seed["tx"]
                ty_base = seed["ty"]
            for dx in dx_values:
                tx = tx_base + dx
                for dy in dy_values:
                    ty = ty_base + dy
                    placed = place_mask(chemdraw_mask, scale, tx, ty, atlas_mask.shape)
                    score = iou(atlas_mask, placed)
                    if best is None or score > best["iou"]:
                        best = {"scale": float(scale), "tx": float(tx), "ty": float(ty), "iou": float(score)}

    sweep(
        np.linspace(base_scale * 0.85, base_scale * 1.15, 25),
        np.linspace(-12, 12, 25),
        np.linspace(-12, 12, 25),
    )
    seed = dict(best)
    sweep(
        np.linspace(seed["scale"] * 0.96, seed["scale"] * 1.04, 21),
        np.linspace(-2.5, 2.5, 21),
        np.linspace(-2.5, 2.5, 21),
        seed=seed,
    )
    assert best is not None
    return best


def transform_points(points: list[tuple[float, float]], bbox: list[int], scale: float, tx: float, ty: float) -> list[tuple[float, float]]:
    x0, y0, _, _ = bbox
    return [((x - x0) * scale + tx, (y - y0) * scale + ty) for x, y in points]


def contour_points(mask: np.ndarray) -> list[tuple[float, float]]:
    contours = measure.find_contours(mask.astype(float), 0.5)
    if not contours:
        return []
    contour = max(contours, key=lambda c: c.shape[0])
    # skimage returns row, col; convert to x, y
    return [(float(col), float(row)) for row, col in contour]


def main() -> int:
    args = parse_args()
    atlas_cell = Image.open(args.atlas_cell).convert("RGBA")
    atlas_mask, atlas_bbox = extract_atlas_n_mask(atlas_cell)
    atlas_union = extract_atlas_union_mask(atlas_cell)
    screenshot = Image.open(args.screenshot).convert("RGBA")
    fit = json.loads(Path(args.fit_json).read_text(encoding="utf-8"))
    cd_bbox = fit["center_component"]["bbox"]
    chemdraw_mask = extract_chemdraw_n_mask(screenshot, cd_bbox)
    fit_curve = [(float(item["x"]), float(item["y"])) for item in fit["fit"]]

    reg = search_registration(atlas_mask, chemdraw_mask, atlas_bbox)
    placed = place_mask(chemdraw_mask, reg["scale"], reg["tx"], reg["ty"], atlas_mask.shape)
    transformed_curve = transform_points(fit_curve, cd_bbox, reg["scale"], reg["tx"], reg["ty"])

    overlay = Image.new("RGBA", atlas_cell.size, (255, 255, 255, 255))
    atlas_fill = Image.new("RGBA", atlas_cell.size, (37, 99, 235, 170))
    overlay.paste(atlas_fill, (0, 0), Image.fromarray((atlas_mask.astype(np.uint8) * 255), mode="L"))
    chem_fill = Image.new("RGBA", atlas_cell.size, (0, 0, 0, 160))
    overlay.paste(chem_fill, (0, 0), Image.fromarray((placed.astype(np.uint8) * 255), mode="L"))
    draw = ImageDraw.Draw(overlay, "RGBA")
    atlas_union_contour = contour_points(atlas_union)
    if atlas_union_contour:
        draw.line(atlas_union_contour + [atlas_union_contour[0]], fill=(37, 99, 235, 255), width=2)
    draw.line(transformed_curve + [transformed_curve[0]], fill=(220, 38, 38, 255), width=2)

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    overlay.save(output)
    output.with_suffix(".json").write_text(json.dumps({"registration": reg, "atlas_bbox": atlas_bbox, "chemdraw_bbox": cd_bbox}, indent=2), encoding="utf-8")
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
