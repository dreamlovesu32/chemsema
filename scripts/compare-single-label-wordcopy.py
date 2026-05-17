from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image


def load_mask(path: Path, threshold: int = 740):
    img = Image.open(path).convert("RGBA")
    w, h = img.size
    px = img.load()
    mask = [[False] * w for _ in range(h)]
    for y in range(h):
        row = mask[y]
        for x in range(w):
            r, g, b, a = px[x, y]
            row[x] = a > 0 and (r + g + b) < threshold
    return img, mask


def bbox_from_mask(mask):
    h = len(mask)
    w = len(mask[0]) if h else 0
    xs = []
    ys = []
    for y in range(h):
        for x in range(w):
            if mask[y][x]:
                xs.append(x)
                ys.append(y)
    if not xs:
        return None
    return min(xs), min(ys), max(xs), max(ys)


def crop_image(img: Image.Image, box):
    return img.crop(box)


def crop_mask(mask, box):
    x0, y0, x1, y1 = box
    return [row[x0:x1] for row in mask[y0:y1]]


def align_score(a_mask, b_mask, dx, dy):
    h = min(len(a_mask), len(b_mask))
    w = min(len(a_mask[0]), len(b_mask[0]))
    inter = only_a = only_b = 0
    for y in range(h):
        ay = y + dy
        if ay < 0 or ay >= len(a_mask):
            continue
        for x in range(w):
            ax = x + dx
            if ax < 0 or ax >= len(a_mask[0]):
                continue
            av = a_mask[ay][ax]
            bv = b_mask[y][x]
            if av and bv:
                inter += 1
            elif av:
                only_a += 1
            elif bv:
                only_b += 1
    union = inter + only_a + only_b
    iou = inter / union if union else 1.0
    return {"iou": iou, "intersection": inter, "onlyA": only_a, "onlyB": only_b}


def best_shift(a_mask, b_mask, span):
    best = None
    for dy in range(-span, span + 1):
        for dx in range(-span, span + 1):
            score = align_score(a_mask, b_mask, dx, dy)
            key = (score["iou"], -score["onlyA"] - score["onlyB"])
            if best is None or key > best["key"]:
                best = {"dx": dx, "dy": dy, "score": score, "key": key}
    return best


def overlay(a_mask, b_mask, dx, dy):
    h = max(len(a_mask), len(b_mask))
    w = max(len(a_mask[0]), len(b_mask[0]))
    img = Image.new("RGBA", (w, h), (255, 255, 255, 255))
    out = img.load()
    for y in range(h):
        for x in range(w):
            ax = x + dx
            ay = y + dy
            av = 0 <= ay < len(a_mask) and 0 <= ax < len(a_mask[0]) and a_mask[ay][ax]
            bv = y < len(b_mask) and x < len(b_mask[0]) and b_mask[y][x]
            if av and bv:
                out[x, y] = (0, 0, 0, 255)
            elif av:
                out[x, y] = (255, 0, 0, 255)
            elif bv:
                out[x, y] = (0, 0, 255, 255)
    return img


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("label_json")
    ap.add_argument("node_id")
    ap.add_argument("reference_png")
    ap.add_argument("wordcopy_png")
    ap.add_argument("out_dir")
    ap.add_argument("--pad", type=int, default=12)
    ap.add_argument("--shift-span", type=int, default=8)
    args = ap.parse_args()

    labels = json.loads(Path(args.label_json).read_text(encoding="utf-8"))["labels"]
    row = next(item for item in labels if item["nodeId"] == args.node_id)
    x0, y0, x1, y1 = row["pixelBox"]
    pad = args.pad
    ref_img, ref_mask_all = load_mask(Path(args.reference_png))
    ref_box = (
        max(0, x0 - pad),
        max(0, y0 - pad),
        min(ref_img.size[0], x1 + pad + 1),
        min(ref_img.size[1], y1 + pad + 1),
    )
    ref_crop = crop_image(ref_img, ref_box)
    ref_mask = crop_mask(ref_mask_all, ref_box)

    ours_img, ours_mask_all = load_mask(Path(args.wordcopy_png))
    ours_bbox = bbox_from_mask(ours_mask_all)
    if ours_bbox is None:
        raise SystemExit("No ink in wordcopy")
    ox0, oy0, ox1, oy1 = ours_bbox
    ours_box = (ox0, oy0, ox1 + 1, oy1 + 1)
    ours_crop = crop_image(ours_img, ours_box)
    ours_mask = crop_mask(ours_mask_all, ours_box)

    best = best_shift(ours_mask, ref_mask, args.shift_span)
    overlay_img = overlay(ours_mask, ref_mask, best["dx"], best["dy"])

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    ref_crop.save(out_dir / "chemdraw-crop.png")
    ours_crop.save(out_dir / "ours-crop.png")
    overlay_img.save(out_dir / "overlay.png")
    payload = {
        "nodeId": args.node_id,
        "text": row["text"],
        "fill": row.get("fill"),
        "pixelBox": row["pixelBox"],
        "bestShift": {"dx": best["dx"], "dy": best["dy"]},
        "score": best["score"],
        "oursInkBox": ours_bbox,
        "referenceCropBox": ref_box,
    }
    (out_dir / "metrics.json").write_text(
        json.dumps(payload, indent=2, ensure_ascii=False), encoding="utf-8"
    )
    print(json.dumps(payload, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
