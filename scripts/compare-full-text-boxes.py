from __future__ import annotations

import argparse
import json
import math
from collections import defaultdict
from pathlib import Path

from PIL import Image


def load_document(path: Path) -> dict:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if "chemcoreDocumentJson" in payload:
        return json.loads(payload["chemcoreDocumentJson"])
    return payload


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


def transform_point(point: tuple[float, float], transform: dict) -> tuple[float, float]:
    x, y = point
    sx, sy = transform.get("scale", [1.0, 1.0])
    x *= float(sx)
    y *= float(sy)
    rotate_deg = float(transform.get("rotate", 0.0))
    if rotate_deg:
        theta = math.radians(rotate_deg)
        cos_t = math.cos(theta)
        sin_t = math.sin(theta)
        x, y = (x * cos_t - y * sin_t, x * sin_t + y * cos_t)
    tx, ty = transform.get("translate", [0.0, 0.0])
    return x + float(tx), y + float(ty)


def transform_box(box: list[float], transform: dict) -> list[float]:
    x1, y1, x2, y2 = box
    corners = [
        transform_point((x1, y1), transform),
        transform_point((x2, y1), transform),
        transform_point((x2, y2), transform),
        transform_point((x1, y2), transform),
    ]
    xs = [p[0] for p in corners]
    ys = [p[1] for p in corners]
    return [min(xs), min(ys), max(xs), max(ys)]


def project_box(
    box: list[float],
    visible: list[float],
    width: int,
    height: int,
    pad_px: int,
) -> list[int]:
    min_x, min_y, max_x, max_y = visible
    source_width = max_x - min_x
    source_height = max_y - min_y
    scale = min(width / source_width, height / source_height)
    offset_x = (width - source_width * scale) / 2.0
    offset_y = (height - source_height * scale) / 2.0
    x1, y1, x2, y2 = box
    px1 = int((x1 - min_x) * scale + offset_x - pad_px)
    py1 = int((y1 - min_y) * scale + offset_y - pad_px)
    px2 = int((x2 - min_x) * scale + offset_x + pad_px)
    py2 = int((y2 - min_y) * scale + offset_y + pad_px)
    return [
        max(0, px1),
        max(0, py1),
        min(width - 1, px2),
        min(height - 1, py2),
    ]


def mask_bbox(img: list[list[bool]], box: list[int]) -> list[int] | None:
    x0, y0, x1, y1 = box
    xs: list[int] = []
    ys: list[int] = []
    for y in range(y0, y1 + 1):
        row = img[y]
        for x in range(x0, x1 + 1):
            if row[x]:
                xs.append(x - x0)
                ys.append(y - y0)
    if not xs:
        return None
    return [min(xs), min(ys), max(xs), max(ys)]


def dims(box: list[int]) -> list[int]:
    return [box[2] - box[0] + 1, box[3] - box[1] + 1]


def scripts_summary(runs: list[dict]) -> list[str]:
    return sorted({run.get("script", "normal") for run in runs})


def text_preview(text: str, limit: int = 80) -> str:
    compact = text.replace("\n", " ⏎ ")
    return compact if len(compact) <= limit else compact[: limit - 1] + "…"


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Compare same-shell full-doc top-level text object boxes against ChemDraw reference."
    )
    parser.add_argument("payload_json")
    parser.add_argument("role_report_json")
    parser.add_argument("ours_png")
    parser.add_argument("reference_png")
    parser.add_argument("output_json")
    parser.add_argument("--dx", type=int, default=0)
    parser.add_argument("--dy", type=int, default=0)
    parser.add_argument("--threshold", type=int, default=740)
    parser.add_argument("--pad-px", type=int, default=3)
    args = parser.parse_args()

    document = load_document(Path(args.payload_json))
    role_report = json.loads(Path(args.role_report_json).read_text(encoding="utf-16"))
    ours, width, height = load_mask(Path(args.ours_png), args.threshold)
    reference, ref_width, ref_height = load_mask(Path(args.reference_png), args.threshold)
    if (width, height) != (ref_width, ref_height):
        raise SystemExit("PNG sizes must match")

    visible = role_report["visibleBoundsNoKnockout"]

    rows = []
    for obj in document.get("objects", []):
        if obj.get("type") != "text":
            continue
        payload = obj.get("payload", {})
        local_box = payload.get("box")
        if not local_box:
            continue
        world_box = transform_box(local_box, obj.get("transform", {}))
        pixel_box = project_box(world_box, visible, width, height, args.pad_px)
        obox = [
            max(0, pixel_box[0] + args.dx),
            max(0, pixel_box[1] + args.dy),
            min(width - 1, pixel_box[2] + args.dx),
            min(height - 1, pixel_box[3] + args.dy),
        ]
        rbox = pixel_box
        ours_bbox = mask_bbox(ours, obox)
        ref_bbox = mask_bbox(reference, rbox)
        if ours_bbox is None or ref_bbox is None:
            continue
        ours_dims = dims(ours_bbox)
        ref_dims = dims(ref_bbox)
        runs = payload.get("runs", [])
        text = payload.get("text", "")
        row = {
            "objectId": obj.get("id"),
            "textPreview": text_preview(text),
            "align": payload.get("align"),
            "lines": text.count("\n") + 1,
            "scripts": scripts_summary(runs),
            "baselineOffset": payload.get("baselineOffset"),
            "fontSize": payload.get("fontSize"),
            "worldBox": world_box,
            "pixelBox": pixel_box,
            "oursLocalBbox": ours_bbox,
            "refLocalBbox": ref_bbox,
            "oursDims": ours_dims,
            "refDims": ref_dims,
            "deltaDims": [ours_dims[0] - ref_dims[0], ours_dims[1] - ref_dims[1]],
            "deltaTopLeft": [ours_bbox[0] - ref_bbox[0], ours_bbox[1] - ref_bbox[1]],
        }
        rows.append(row)

    summary_groups = defaultdict(
        lambda: {"count": 0, "sumDw": 0, "sumDh": 0, "sumDx": 0, "sumDy": 0}
    )
    for row in rows:
        key = f"{row['align']}|{','.join(row['scripts'])}|lines={row['lines']}"
        g = summary_groups[key]
        g["count"] += 1
        g["sumDw"] += row["deltaDims"][0]
        g["sumDh"] += row["deltaDims"][1]
        g["sumDx"] += row["deltaTopLeft"][0]
        g["sumDy"] += row["deltaTopLeft"][1]

    summary = []
    for key, g in summary_groups.items():
        count = g["count"]
        summary.append(
            {
                "key": key,
                "count": count,
                "avgDw": g["sumDw"] / count,
                "avgDh": g["sumDh"] / count,
                "avgDx": g["sumDx"] / count,
                "avgDy": g["sumDy"] / count,
            }
        )
    summary.sort(key=lambda item: item["count"], reverse=True)

    payload = {
        "payload": str(Path(args.payload_json).resolve()),
        "ours_png": str(Path(args.ours_png).resolve()),
        "reference_png": str(Path(args.reference_png).resolve()),
        "dx": args.dx,
        "dy": args.dy,
        "pad": args.pad_px,
        "rows": rows,
        "summary": summary,
    }
    Path(args.output_json).write_text(
        json.dumps(payload, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )
    print(json.dumps(payload, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
