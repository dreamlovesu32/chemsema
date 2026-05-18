from __future__ import annotations

import argparse
import importlib.util
import json
from collections import defaultdict
from pathlib import Path
from statistics import mean


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load module from {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def derive_family_fields(row: dict) -> dict:
    text = row.get("text", "")
    out = dict(row)
    out["lineCount"] = text.count("\n") + 1
    out["charCount"] = len(text.replace("\n", ""))
    out["isBlack"] = (row.get("fill") or "").lower() == "#000000"
    return out


def summarize_groups(rows: list[dict], key_fn, *, min_count: int = 3) -> list[dict]:
    groups: dict[object, list[dict]] = defaultdict(list)
    for row in rows:
        groups[key_fn(row)].append(row)

    out = []
    for key, items in groups.items():
        if len(items) < min_count:
            continue
        out.append(
            {
                "key": key,
                "count": len(items),
                "avgBaseIou": mean(item["baseIou"] for item in items),
                "avgBestIou": mean(item["bestIou"] for item in items),
                "avgDeltaIou": mean(item["bestIou"] - item["baseIou"] for item in items),
                "avgBestDx": mean(item["bestLocalDx"] for item in items),
                "avgBestDy": mean(item["bestLocalDy"] for item in items),
                "examples": sorted({item["text"] for item in items})[:12],
                "sampleExamples": [
                    f"{item['sampleStem']}:{item['text']}@({item['bestLocalDx']},{item['bestLocalDy']})"
                    for item in items[:6]
                ],
            }
        )
    out.sort(key=lambda item: (-item["avgDeltaIou"], -item["count"], str(item["key"])))
    return out


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Search local label dx/dy improvements inside PPT same-shell wordcopy comparisons."
    )
    parser.add_argument(
        "--input",
        default="tmp/ppt-generalization-label-boxes.json",
        help="Input JSON from summarize-ppt-label-boxes.py",
    )
    parser.add_argument(
        "--output",
        default="tmp/ppt-generalization-label-local-shifts.json",
        help="Output JSON path.",
    )
    parser.add_argument("--search", type=int, default=3, help="Search local dx/dy in [-search, search].")
    parser.add_argument("--pad", type=int, default=0)
    parser.add_argument("--threshold", type=int, default=740)
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parent.parent
    iou_mod = load_module(repo_root / "scripts" / "compare-full-label-iou.py", "compare_full_label_iou")

    data = json.loads(Path(args.input).read_text(encoding="utf-8"))
    objects = {obj["sampleStem"]: obj for obj in data["objects"]}
    rows = []
    image_cache: dict[tuple[str, str], tuple[object, object]] = {}

    for raw_row in data["rows"]:
        row = derive_family_fields(raw_row)
        obj = objects.get(row["sampleStem"])
        if obj is None:
            continue
        sample_dir = repo_root / "tmp" / row["sampleStem"].split("/", 1)[0] / "same-shell-compare"
        stem = row["sampleStem"].split("/", 1)[1]
        ours_png = sample_dir / f"{stem}.chemcore.wordcopy.png"
        ref_png = sample_dir / f"{stem}.chemdraw.wordcopy.png"
        cache_key = (str(ours_png), str(ref_png))
        if cache_key not in image_cache:
            image_cache[cache_key] = (iou_mod.load_rgba(ours_png), iou_mod.load_rgba(ref_png))
        ours, ref = image_cache[cache_key]

        box = row["pixelBox"]
        base_dx = int(obj["dx"])
        base_dy = int(obj["dy"])
        base = iou_mod.mask_box_counts(ours, ref, box, base_dx, base_dy, args.threshold)
        if base is None:
            continue
        best_iou = base["iou"]
        best_local_dx = 0
        best_local_dy = 0
        for local_dy in range(-args.search, args.search + 1):
            for local_dx in range(-args.search, args.search + 1):
                stats = iou_mod.mask_box_counts(
                    ours,
                    ref,
                    box,
                    base_dx + local_dx,
                    base_dy + local_dy,
                    args.threshold,
                )
                if stats is None:
                    continue
                if (
                    stats["iou"] > best_iou
                    or (
                        stats["iou"] == best_iou
                        and abs(local_dx) + abs(local_dy) < abs(best_local_dx) + abs(best_local_dy)
                    )
                ):
                    best_iou = stats["iou"]
                    best_local_dx = local_dx
                    best_local_dy = local_dy

        row["baseIou"] = base["iou"]
        row["bestIou"] = best_iou
        row["bestLocalDx"] = best_local_dx
        row["bestLocalDy"] = best_local_dy
        row["deltaIou"] = best_iou - base["iou"]
        rows.append(row)

    output = {
        "source": args.input,
        "search": args.search,
        "countRows": len(rows),
        "layoutLineColorFamilies": summarize_groups(
            rows,
            lambda r: (
                r["layout"],
                r["lineCount"],
                "black" if r["isBlack"] else "nonblack",
            ),
        ),
        "layoutLineFamilies": summarize_groups(
            rows,
            lambda r: (r["layout"], r["lineCount"]),
        ),
        "explicitFamilies": summarize_groups(
            rows,
            lambda r: (
                "attached_above_multiline_black"
                if r["layout"] == "attached-group-above" and r["lineCount"] >= 2 and r["isBlack"]
                else "attached_above_single_black"
                if r["layout"] == "attached-group-above" and r["lineCount"] == 1 and r["isBlack"]
                else "attached_multiline_black"
                if r["layout"] == "attached-group" and r["lineCount"] >= 2 and r["isBlack"]
                else "attached_long_black"
                if r["layout"] == "attached-group" and r["lineCount"] == 1 and r["charCount"] >= 4 and r["isBlack"]
                else "attached_compact_black_2char"
                if r["layout"] == "attached-group" and r["lineCount"] == 1 and r["charCount"] == 2 and r["isBlack"]
                else "attached_compact_black_1char"
                if r["layout"] == "attached-group" and r["lineCount"] == 1 and r["charCount"] == 1 and r["isBlack"]
                else "attached_nonblack"
                if r["layout"] == "attached-group" and not r["isBlack"]
                else "other"
            ),
        ),
        "topRows": sorted(rows, key=lambda r: (-r["deltaIou"], r["bestLocalDy"], r["bestLocalDx"]))[:40],
        "rows": rows,
    }

    Path(args.output).write_text(json.dumps(output, indent=2, ensure_ascii=False), encoding="utf-8")
    print(args.output)


if __name__ == "__main__":
    main()
