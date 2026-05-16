from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
from PIL import Image


def parse_region(text: str) -> tuple[int, int, int, int]:
    parts = [int(float(part.strip())) for part in text.split(",")]
    if len(parts) != 4:
        raise ValueError(f"invalid region: {text}")
    return tuple(parts)  # type: ignore[return-value]


def load_mask(path: Path, threshold: int) -> np.ndarray:
    image = Image.open(path).convert("RGBA")
    rgba = np.asarray(image)
    rgb = rgba[..., :3].astype(np.int16)
    alpha = rgba[..., 3].astype(np.int16)
    return (alpha > 0) & (rgb.sum(axis=2) < threshold)


def region_view(mask: np.ndarray, region: tuple[int, int, int, int] | None) -> np.ndarray:
    if region is None:
        return mask
    left, top, right, bottom = region
    return mask[top:bottom, left:right]


def iou_at_shift(
    ours: np.ndarray,
    reference: np.ndarray,
    dx: int,
    dy: int,
    region: tuple[int, int, int, int] | None = None,
) -> dict:
    ours = region_view(ours, region)
    reference = region_view(reference, region)

    height = min(ours.shape[0], reference.shape[0])
    width = min(ours.shape[1], reference.shape[1])
    ours = ours[:height, :width]
    reference = reference[:height, :width]

    ours_y0 = max(0, dy)
    ref_y0 = max(0, -dy)
    ours_x0 = max(0, dx)
    ref_x0 = max(0, -dx)
    h = height - abs(dy)
    w = width - abs(dx)
    if h <= 0 or w <= 0:
        return {
            "iou": 0.0,
            "intersection": 0,
            "only_ours": 0,
            "only_reference": 0,
            "shared_width": width,
            "shared_height": height,
        }

    ours_crop = ours[ours_y0 : ours_y0 + h, ours_x0 : ours_x0 + w]
    ref_crop = reference[ref_y0 : ref_y0 + h, ref_x0 : ref_x0 + w]
    intersection = int(np.count_nonzero(ours_crop & ref_crop))
    only_ours = int(np.count_nonzero(ours_crop & ~ref_crop))
    only_reference = int(np.count_nonzero(ref_crop & ~ours_crop))
    union = intersection + only_ours + only_reference
    iou = 1.0 if union == 0 else intersection / union
    return {
        "iou": iou,
        "intersection": intersection,
        "only_ours": only_ours,
        "only_reference": only_reference,
        "shared_width": width,
        "shared_height": height,
    }


def best_shift(
    ours: np.ndarray,
    reference: np.ndarray,
    limit: int,
    region: tuple[int, int, int, int] | None = None,
) -> dict:
    best_key = (-1.0, -1, -10**9)
    best_data: dict | None = None
    for dy in range(-limit, limit + 1):
        for dx in range(-limit, limit + 1):
            stats = iou_at_shift(ours, reference, dx, dy, region)
            key = (stats["iou"], stats["intersection"], -(abs(dx) + abs(dy)))
            if key > best_key:
                best_key = key
                best_data = {"dx": dx, "dy": dy, **stats}
    assert best_data is not None
    return best_data


def main() -> None:
    parser = argparse.ArgumentParser(description="Compare regional IoU under global/local shifts.")
    parser.add_argument("ours_png")
    parser.add_argument("reference_png")
    parser.add_argument("--threshold", type=int, default=740)
    parser.add_argument("--limit", type=int, default=12)
    parser.add_argument(
        "--region",
        action="append",
        default=[],
        help="name=left,top,right,bottom; may repeat",
    )
    parser.add_argument("--output")
    args = parser.parse_args()

    ours = load_mask(Path(args.ours_png), args.threshold)
    reference = load_mask(Path(args.reference_png), args.threshold)
    report: dict[str, object] = {
        "ours_png": args.ours_png,
        "reference_png": args.reference_png,
        "threshold": args.threshold,
        "limit": args.limit,
        "global_best": best_shift(ours, reference, args.limit),
        "regions": {},
    }

    global_dx = int(report["global_best"]["dx"])  # type: ignore[index]
    global_dy = int(report["global_best"]["dy"])  # type: ignore[index]
    regions: dict[str, object] = {}
    for item in args.region:
        name, raw_region = item.split("=", 1)
        region = parse_region(raw_region)
        regions[name] = {
            "region": list(region),
            "global_shift": {
                "dx": global_dx,
                "dy": global_dy,
                **iou_at_shift(ours, reference, global_dx, global_dy, region),
            },
            "local_best": best_shift(ours, reference, args.limit, region),
        }
    report["regions"] = regions

    text = json.dumps(report, indent=2)
    if args.output:
        Path(args.output).write_text(text, encoding="utf8")
    else:
        print(text)


if __name__ == "__main__":
    main()
