from __future__ import annotations

import argparse
import json
import subprocess
from collections import defaultdict
from pathlib import Path
from statistics import mean


ROOT = Path(__file__).resolve().parents[1]


def run_dump_text_primitives(payload: Path) -> list[dict]:
    cmd = [
        "cargo",
        "run",
        "-q",
        "-p",
        "chemcore-engine",
        "--example",
        "dump_text_primitives",
        "--",
        str(payload),
    ]
    result = subprocess.run(
        cmd,
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(result.stdout)


def primitive_text(primitive: dict) -> str:
    runs = primitive.get("runs") or []
    if runs:
        return "".join((run.get("text") or "") for run in runs)
    return primitive.get("text") or ""


def primitive_scripts(primitive: dict) -> list[str]:
    out = []
    for run in primitive.get("runs") or []:
        script = run.get("script") or "normal"
        if script not in out:
            out.append(script)
    return out


def family_table(rows: list[dict], key_fn, *, min_count: int = 1) -> list[dict]:
    groups: dict[object, list[dict]] = defaultdict(list)
    for row in rows:
        groups[key_fn(row)].append(row)

    out = []
    for key, items in groups.items():
        if len(items) < min_count:
            continue
        y_steps = [step for item in items for step in item["ySteps"]]
        baseline_values = [
            value for item in items for value in item["baselineOffsets"]
        ]
        out.append(
            {
                "key": key,
                "count": len(items),
                "avgPrimitiveCount": mean(item["primitiveCount"] for item in items),
                "avgLineCount": mean(item["lineCount"] for item in items),
                "avgResidual": mean(item["residualCount"] for item in items),
                "avgIoU": mean(item["iou"] for item in items),
                "shareAnchorStart": mean(
                    1.0 if item["allAnchors"] == ["start"] else 0.0 for item in items
                ),
                "shareNullLineHeight": mean(
                    1.0 if not item["lineHeights"] else 0.0 for item in items
                ),
                "sharePreserveLinesFalse": mean(
                    1.0 if item["preserveLines"] == [False] else 0.0 for item in items
                ),
                "shareSplitPrimitivePerLine": mean(
                    1.0 if item["splitPrimitivePerLine"] else 0.0 for item in items
                ),
                "avgBaselineOffset": mean(baseline_values) if baseline_values else None,
                "avgYStep": mean(y_steps) if y_steps else None,
                "examples": [
                    f"{item['sampleStem']}:{item['nodeId']}:{item['text'].replace(chr(10), '|')}"
                    for item in items[:6]
                ],
            }
        )
    out.sort(key=lambda item: (-item["avgResidual"], -item["count"], str(item["key"])))
    return out


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Summarize render_document Text primitive structure for PPT same-shell label families."
    )
    parser.add_argument(
        "--label-boxes",
        default="tmp/ppt-generalization-label-boxes.json",
        help="Input JSON from summarize-ppt-label-boxes.py",
    )
    parser.add_argument(
        "--output",
        default="tmp/ppt-generalization-render-text-primitives.json",
        help="Output JSON path.",
    )
    args = parser.parse_args()

    data = json.loads((ROOT / args.label_boxes).read_text(encoding="utf-8"))
    rows_in = data["rows"]
    objects_by_stem = {obj["sampleStem"]: obj for obj in data["objects"]}

    dump_cache: dict[str, list[dict]] = {}
    indexed_cache: dict[str, dict[str, list[dict]]] = {}
    rows_out = []

    for row in rows_in:
        sample_stem = row["sampleStem"]
        obj_meta = objects_by_stem.get(sample_stem)
        if obj_meta is None:
            continue
        payload = Path(obj_meta["payload"])
        if sample_stem not in dump_cache:
            dump_cache[sample_stem] = run_dump_text_primitives(payload)
            by_node: dict[str, list[dict]] = defaultdict(list)
            for primitive in dump_cache[sample_stem]:
                node_id = primitive.get("nodeId")
                if node_id:
                    by_node[node_id].append(primitive)
            indexed_cache[sample_stem] = by_node

        primitives = indexed_cache[sample_stem].get(row["nodeId"], [])
        primitives = sorted(primitives, key=lambda item: (item.get("y", 0.0), item.get("x", 0.0)))
        line_count = row["text"].count("\n") + 1
        baseline_offsets = sorted(
            {round(float(item.get("baselineOffset") or 0.0), 6) for item in primitives}
        )
        line_heights = sorted(
            {
                round(float(item.get("lineHeight")), 6)
                for item in primitives
                if item.get("lineHeight") is not None
            }
        )
        preserve_lines = sorted({bool(item.get("preserveLines")) for item in primitives})
        all_anchors = sorted({item.get("textAnchor") for item in primitives if item.get("textAnchor")})
        y_values = [float(item.get("y") or 0.0) for item in primitives]
        y_steps = [round(y_values[i + 1] - y_values[i], 6) for i in range(len(y_values) - 1)]

        out_row = dict(row)
        out_row.update(
            {
                "lineCount": line_count,
                "primitiveCount": len(primitives),
                "primitiveTexts": [primitive_text(item) for item in primitives],
                "primitiveScripts": [primitive_scripts(item) for item in primitives],
                "primitiveXs": [round(float(item.get("x") or 0.0), 6) for item in primitives],
                "primitiveYs": [round(v, 6) for v in y_values],
                "ySteps": y_steps,
                "baselineOffsets": baseline_offsets,
                "lineHeights": line_heights,
                "preserveLines": preserve_lines,
                "allAnchors": all_anchors,
                "splitPrimitivePerLine": (
                    len(primitives) == line_count
                    and len(primitives) > 0
                    and all_anchors == ["start"]
                    and not line_heights
                ),
                "passesLegacyAttachedGroupGate": (
                    row.get("layout") == "attached-group"
                    and row.get("anchor") == "start"
                ),
            }
        )
        rows_out.append(out_row)

    output = {
        "countRows": len(rows_out),
        "rows": rows_out,
        "layoutLineColorFamilies": family_table(
            rows_out,
            lambda item: (
                item.get("layout"),
                item["lineCount"],
                (item.get("fill") or "").lower() == "#000000",
            ),
            min_count=3,
        ),
        "layoutCharColorFamilies": family_table(
            rows_out,
            lambda item: (
                item.get("layout"),
                len(item["text"].replace("\n", "")),
                (item.get("fill") or "").lower() == "#000000",
            ),
            min_count=3,
        ),
    }

    output_path = ROOT / args.output
    output_path.write_text(json.dumps(output, ensure_ascii=False, indent=2), encoding="utf-8")
    print(output_path)


if __name__ == "__main__":
    main()
