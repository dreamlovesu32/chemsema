from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path

from PIL import Image


def load_rgba(path: Path) -> Image.Image:
    return Image.open(path).convert("RGBA")


def mask_bbox(img: Image.Image, box, threshold: int = 740):
    x0, y0, x1, y1 = box
    x0 = max(0, x0)
    y0 = max(0, y0)
    x1 = min(img.width - 1, x1)
    y1 = min(img.height - 1, y1)
    if x1 < x0 or y1 < y0:
        return None
    px = img.load()
    xs = []
    ys = []
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            r, g, b, a = px[x, y]
            if a > 0 and (r + g + b) < threshold:
                xs.append(x - x0)
                ys.append(y - y0)
    if not xs:
        return None
    return [min(xs), min(ys), max(xs), max(ys)]


def dims(box):
    return [box[2] - box[0] + 1, box[3] - box[1] + 1]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("label_json")
    ap.add_argument("ours_png")
    ap.add_argument("reference_png")
    ap.add_argument("output_json")
    ap.add_argument("--dx", type=int, default=0)
    ap.add_argument("--dy", type=int, default=0)
    ap.add_argument("--pad", type=int, default=0)
    args = ap.parse_args()

    labels = json.loads(Path(args.label_json).read_text(encoding="utf-8"))["labels"]
    ours = load_rgba(Path(args.ours_png))
    ref = load_rgba(Path(args.reference_png))

    rows = []
    for row in labels:
        x0, y0, x1, y1 = row["pixelBox"]
        pad = args.pad
        box = [x0 - pad, y0 - pad, x1 + pad, y1 + pad]
        obox = [box[0] + args.dx, box[1] + args.dy, box[2] + args.dx, box[3] + args.dy]
        rbox = box
        ours_bbox = mask_bbox(ours, obox)
        ref_bbox = mask_bbox(ref, rbox)
        if ours_bbox is None or ref_bbox is None:
            continue
        ours_dims = dims(ours_bbox)
        ref_dims = dims(ref_bbox)
        out = {
            "nodeId": row["nodeId"],
            "text": row.get("text", ""),
            "fill": row.get("fill"),
            "layout": row.get("layout"),
            "anchor": row.get("anchor"),
            "attachment": row.get("attachment"),
            "residualCount": row.get("residualCount", 0),
            "pixelBox": row["pixelBox"],
            "pad": pad,
            "oursLocalBbox": ours_bbox,
            "refLocalBbox": ref_bbox,
            "oursDims": ours_dims,
            "refDims": ref_dims,
            "deltaDims": [ours_dims[0] - ref_dims[0], ours_dims[1] - ref_dims[1]],
            "deltaTopLeft": [ours_bbox[0] - ref_bbox[0], ours_bbox[1] - ref_bbox[1]],
        }
        rows.append(out)

    grouped = defaultdict(lambda: {"count": 0, "sumDw": 0, "sumDh": 0, "sumDx": 0, "sumDy": 0, "sumResidual": 0})
    for row in rows:
        key = f"{row['text']}|{row.get('fill')}"
        g = grouped[key]
        g["count"] += 1
        g["sumDw"] += row["deltaDims"][0]
        g["sumDh"] += row["deltaDims"][1]
        g["sumDx"] += row["deltaTopLeft"][0]
        g["sumDy"] += row["deltaTopLeft"][1]
        g["sumResidual"] += row["residualCount"]

    summary = []
    for key, g in grouped.items():
        count = g["count"]
        summary.append(
            {
                "key": key,
                "count": count,
                "avgDw": g["sumDw"] / count,
                "avgDh": g["sumDh"] / count,
                "avgDx": g["sumDx"] / count,
                "avgDy": g["sumDy"] / count,
                "avgResidual": g["sumResidual"] / count,
                "sumResidual": g["sumResidual"],
            }
        )
    summary.sort(key=lambda item: item["sumResidual"], reverse=True)

    payload = {
        "oursPng": str(Path(args.ours_png).resolve()),
        "referencePng": str(Path(args.reference_png).resolve()),
        "dx": args.dx,
        "dy": args.dy,
        "pad": args.pad,
        "rows": rows,
        "summaryByTextFill": summary,
    }
    Path(args.output_json).write_text(json.dumps(payload, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps(payload, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
