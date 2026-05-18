from __future__ import annotations

import argparse
import importlib.util
import json
import subprocess
import sys
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


def ensure_role_report(repo_root: Path, payload: Path, role_report: Path) -> None:
    if role_report.exists():
        return
    cmd = [
        "cargo",
        "run",
        "-q",
        "-p",
        "chemcore-engine",
        "--example",
        "render_role_report",
        "--",
        str(payload),
    ]
    result = subprocess.run(
        cmd,
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=True,
    )
    role_report.write_text(result.stdout, encoding="utf-8")


def summarize_rows(rows: list[dict]) -> dict:
    def family_table(input_rows: list[dict]) -> list[dict]:
        families = defaultdict(list)
        for row in input_rows:
            key = f"{row['align']}|{','.join(row['scripts'])}|lines={row['lines']}"
            families[key].append(row)

        family_summary = []
        for key, items in families.items():
            severity_values = [
                abs(item["deltaDims"][0])
                + abs(item["deltaDims"][1])
                + abs(item["deltaTopLeft"][0])
                + abs(item["deltaTopLeft"][1])
                for item in items
            ]
            family_summary.append(
                {
                    "key": key,
                    "count": len(items),
                    "avgDw": mean(item["deltaDims"][0] for item in items),
                    "avgDh": mean(item["deltaDims"][1] for item in items),
                    "avgDx": mean(item["deltaTopLeft"][0] for item in items),
                    "avgDy": mean(item["deltaTopLeft"][1] for item in items),
                    "avgL1": mean(severity_values),
                    "maxL1": max(severity_values),
                    "samples": sorted({item["sample"] for item in items}),
                    "exampleTexts": [item["textPreview"] for item in items[:5]],
                }
            )
        family_summary.sort(key=lambda item: (-item["count"], -item["avgL1"], item["key"]))
        return family_summary

    def is_textlike(row: dict) -> bool:
        text = row["textPreview"].replace(" / ", " ").strip()
        alnum_count = sum(ch.isalnum() for ch in text)
        has_space = " " in text
        return row["lines"] > 1 or has_space or alnum_count >= 4

    def font_table(input_rows: list[dict]) -> list[dict]:
        buckets = defaultdict(list)
        for row in input_rows:
            key = ",".join(row["fontFamilies"])
            buckets[key].append(row)
        table = []
        for key, items in buckets.items():
            table.append(
                {
                    "key": key,
                    "count": len(items),
                    "avgDw": mean(item["deltaDims"][0] for item in items),
                    "avgDh": mean(item["deltaDims"][1] for item in items),
                    "avgDx": mean(item["deltaTopLeft"][0] for item in items),
                    "avgDy": mean(item["deltaTopLeft"][1] for item in items),
                    "exampleTexts": [item["textPreview"] for item in items[:5]],
                }
            )
        table.sort(key=lambda item: (-item["count"], item["key"]))
        return table

    families = defaultdict(list)
    for row in rows:
        key = f"{row['align']}|{','.join(row['scripts'])}|lines={row['lines']}"
        families[key].append(row)

    outliers = sorted(
        rows,
        key=lambda row: (
            abs(row["deltaDims"][0])
            + abs(row["deltaDims"][1])
            + abs(row["deltaTopLeft"][0])
            + abs(row["deltaTopLeft"][1])
        ),
        reverse=True,
    )

    sample_summary = []
    sample_groups = defaultdict(list)
    for row in rows:
        sample_groups[row["sampleStem"]].append(row)
    for key, items in sorted(sample_groups.items()):
        sample_summary.append(
            {
                "sampleStem": key,
                "count": len(items),
                "avgDw": mean(item["deltaDims"][0] for item in items),
                "avgDh": mean(item["deltaDims"][1] for item in items),
                "avgDx": mean(item["deltaTopLeft"][0] for item in items),
                "avgDy": mean(item["deltaTopLeft"][1] for item in items),
            }
        )

    return {
        "familySummary": family_table(rows),
        "textLikeFamilySummary": family_table([row for row in rows if is_textlike(row)]),
        "fontFamilySummary": font_table(rows),
        "sampleSummary": sample_summary,
        "topOutliers": outliers[:25],
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Batch-compare same-shell PPT external text boxes against ChemDraw references."
    )
    parser.add_argument(
        "--pattern",
        default="tmp/ppt-sample-*/same-shell-compare/*.payload.json",
        help="Glob for payload JSON files.",
    )
    parser.add_argument(
        "--output",
        default="tmp/ppt-generalization-text-boxes.json",
        help="Output JSON path.",
    )
    parser.add_argument(
        "--pad-px",
        type=int,
        default=3,
        help="Padding passed to compare-full-text-boxes.",
    )
    parser.add_argument(
        "--threshold",
        type=int,
        default=740,
        help="Mask threshold passed to compare-full-text-boxes.",
    )
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parent.parent
    compare_mod = load_module(
        repo_root / "scripts" / "compare-full-text-boxes.py",
        "compare_full_text_boxes",
    )

    payload_paths = sorted(repo_root.glob(args.pattern))
    rows: list[dict] = []
    objects = []
    skipped = []

    for payload_path in payload_paths:
        stem = payload_path.with_suffix("")
        base_no_payload = payload_path.name.removesuffix(".payload.json")
        compare_dir = payload_path.parent
        role_report = compare_dir / f"{base_no_payload}.role-report.json"
        ours_png = compare_dir / f"{base_no_payload}.chemcore.wordcopy.png"
        ref_png = compare_dir / f"{base_no_payload}.chemdraw.wordcopy.png"
        bestshift = compare_dir / f"{base_no_payload}.bestshift.json"

        if not ours_png.exists() or not ref_png.exists() or not bestshift.exists():
            skipped.append(
                {
                    "payload": str(payload_path),
                    "reason": "missing compare asset",
                }
            )
            continue

        ensure_role_report(repo_root, payload_path, role_report)

        document = compare_mod.load_document(payload_path)
        role_data = compare_mod.load_json_any_encoding(role_report)
        best_data = json.loads(bestshift.read_text(encoding="utf-8"))
        ours_mask, width, height = compare_mod.load_mask(ours_png, args.threshold)
        ref_mask, ref_width, ref_height = compare_mod.load_mask(ref_png, args.threshold)
        if (width, height) != (ref_width, ref_height):
            skipped.append(
                {
                    "payload": str(payload_path),
                    "reason": "wordcopy size mismatch",
                }
            )
            continue

        visible = role_data["visibleBoundsNoKnockout"]
        dx = int(best_data["dx"])
        dy = int(best_data["dy"])

        sample_name = compare_dir.parent.name
        sample_stem = f"{sample_name}/{base_no_payload}"

        object_count = 0
        for obj in document.get("objects", []):
            if obj.get("type") != "text":
                continue
            payload = obj.get("payload", {})
            local_box = payload.get("box")
            if not local_box:
                continue
            world_box = compare_mod.transform_box(local_box, obj.get("transform", {}))
            pixel_box = compare_mod.project_box(world_box, visible, width, height, args.pad_px)
            ours_box = [
                max(0, pixel_box[0] + dx),
                max(0, pixel_box[1] + dy),
                min(width - 1, pixel_box[2] + dx),
                min(height - 1, pixel_box[3] + dy),
            ]
            ref_box = pixel_box
            ours_bbox = compare_mod.mask_bbox(ours_mask, ours_box)
            ref_bbox = compare_mod.mask_bbox(ref_mask, ref_box)
            if ours_bbox is None or ref_bbox is None:
                continue
            ours_dims = compare_mod.dims(ours_bbox)
            ref_dims = compare_mod.dims(ref_bbox)
            runs = payload.get("runs", [])
            font_families = sorted({run.get("fontFamily", "") for run in runs if run.get("fontFamily")})
            row = {
                "sample": sample_name,
                "sampleStem": sample_stem,
                "objectId": obj.get("id"),
                "textPreview": compare_mod.text_preview(payload.get("text", "")),
                "align": payload.get("align"),
                "lines": payload.get("text", "").count("\n") + 1,
                "scripts": compare_mod.scripts_summary(runs),
                "fontFamilies": font_families,
                "baselineOffset": payload.get("baselineOffset"),
                "fontSize": payload.get("fontSize"),
                "deltaDims": [ours_dims[0] - ref_dims[0], ours_dims[1] - ref_dims[1]],
                "deltaTopLeft": [ours_bbox[0] - ref_bbox[0], ours_bbox[1] - ref_bbox[1]],
            }
            rows.append(row)
            object_count += 1

        objects.append(
            {
                "sampleStem": sample_stem,
                "payload": str(payload_path),
                "roleReport": str(role_report),
                "dx": dx,
                "dy": dy,
                "textObjectCount": object_count,
            }
        )

    summary = summarize_rows(rows)
    output = {
        "pattern": args.pattern,
        "countObjects": len(objects),
        "countRows": len(rows),
        "objects": objects,
        "skipped": skipped,
        "rows": rows,
        **summary,
    }
    output_path = repo_root / args.output
    output_path.write_text(json.dumps(output, indent=2, ensure_ascii=False), encoding="utf-8")
    try:
        print(json.dumps(output, indent=2, ensure_ascii=False))
    except UnicodeEncodeError:
        print(json.dumps(output, indent=2, ensure_ascii=True))


if __name__ == "__main__":
    main()
