from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path

from PIL import Image


def load_rgba(path: Path) -> Image.Image:
    return Image.open(path).convert("RGBA")


def mask_box_counts(
    ours: Image.Image,
    ref: Image.Image,
    box,
    dx: int,
    dy: int,
    threshold: int = 740,
):
    x0, y0, x1, y1 = box
    x0 = max(0, x0)
    y0 = max(0, y0)
    x1 = min(ref.width - 1, x1)
    y1 = min(ref.height - 1, y1)
    if x1 < x0 or y1 < y0:
        return None

    opx = ours.load()
    rpx = ref.load()
    inter = ours_only = ref_only = 0
    for ry in range(y0, y1 + 1):
        oy = ry + dy
        if oy < 0 or oy >= ours.height:
            continue
        for rx in range(x0, x1 + 1):
            ox = rx + dx
            if ox < 0 or ox >= ours.width:
                continue
            rr, rg, rb, ra = rpx[rx, ry]
            or_, og, ob, oa = opx[ox, oy]
            rv = ra > 0 and (rr + rg + rb) < threshold
            ov = oa > 0 and (or_ + og + ob) < threshold
            if ov and rv:
                inter += 1
            elif ov:
                ours_only += 1
            elif rv:
                ref_only += 1
    union = inter + ours_only + ref_only
    iou = 1.0 if union == 0 else inter / union
    return {
        "intersection": inter,
        "oursOnly": ours_only,
        "refOnly": ref_only,
        "union": union,
        "iou": iou,
    }


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
        stats = mask_box_counts(
            ours,
            ref,
            [x0 - pad, y0 - pad, x1 + pad, y1 + pad],
            args.dx,
            args.dy,
        )
        if stats is None:
            continue
        rows.append(
            {
                "nodeId": row["nodeId"],
                "text": row.get("text", ""),
                "fill": row.get("fill"),
                "layout": row.get("layout"),
                "anchor": row.get("anchor"),
                "attachment": row.get("attachment"),
                "residualCount": row.get("residualCount", 0),
                "pixelBox": row["pixelBox"],
                "pad": pad,
                **stats,
            }
        )

    grouped = defaultdict(
        lambda: {
            "count": 0,
            "sumIoU": 0.0,
            "sumResidual": 0,
            "sumOursOnly": 0,
            "sumRefOnly": 0,
        }
    )
    for row in rows:
        key = f"{row['text']}|{row.get('fill')}"
        g = grouped[key]
        g["count"] += 1
        g["sumIoU"] += row["iou"]
        g["sumResidual"] += row["residualCount"]
        g["sumOursOnly"] += row["oursOnly"]
        g["sumRefOnly"] += row["refOnly"]

    summary = []
    for key, g in grouped.items():
        count = g["count"]
        summary.append(
            {
                "key": key,
                "count": count,
                "avgIoU": g["sumIoU"] / count,
                "avgResidual": g["sumResidual"] / count,
                "sumResidual": g["sumResidual"],
                "avgOursOnly": g["sumOursOnly"] / count,
                "avgRefOnly": g["sumRefOnly"] / count,
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
        "summary": summary,
    }
    Path(args.output_json).write_text(json.dumps(payload, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
