from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from collections import defaultdict
from pathlib import Path
from statistics import mean


ROOT = Path(__file__).resolve().parents[1]
PYTHON = Path(os.environ.get("CHEMCORE_PYTHON", sys.executable))


def run(args: list[str]) -> None:
    result = subprocess.run(
        args,
        cwd=str(ROOT),
        text=True,
        capture_output=True,
    )
    if result.stdout:
        print(result.stdout, end="")
    if result.stderr:
        print(result.stderr, end="")
    if result.returncode != 0:
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(args)}")


def ensure_label_boxes(sweep_dir: Path) -> Path:
    output = sweep_dir.with_name(f"{sweep_dir.name}-label-boxes.json")
    if output.exists():
        return output
    pattern = sweep_dir.relative_to(ROOT).as_posix() + "/*/same-shell-compare/*.payload.json"
    run(
        [
            str(PYTHON),
            str(ROOT / "scripts" / "summarize-ppt-label-boxes.py"),
            "--pattern",
            pattern,
            "--output",
            output.relative_to(ROOT).as_posix(),
        ]
    )
    return output


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Summarize PPT policy sweeps at global and label-family level."
    )
    parser.add_argument(
        "--sweeps-root",
        default="tmp/ppt-policy-sweeps",
        help="Root containing one subdirectory per evaluated policy.",
    )
    parser.add_argument(
        "--baseline-label-boxes",
        default="tmp/ppt-generalization-label-boxes.json",
        help="Baseline label-box JSON.",
    )
    parser.add_argument(
        "--render-primitives",
        default="tmp/ppt-generalization-render-text-primitives.json",
        help="Rendered text primitive summary JSON used to recover lineCount.",
    )
    parser.add_argument(
        "--output",
        default="tmp/ppt-policy-sweeps-summary.json",
        help="Output summary JSON.",
    )
    args = parser.parse_args()

    sweeps_root = ROOT / args.sweeps_root
    baseline_rows = json.loads((ROOT / args.baseline_label_boxes).read_text(encoding="utf-8"))["rows"]
    primitive_rows = json.loads((ROOT / args.render_primitives).read_text(encoding="utf-8"))["rows"]

    line_count_by_key = {}
    for row in primitive_rows:
        line_count_by_key[(row["sampleStem"], row["objectId"], row["nodeId"])] = row.get("lineCount", 1)

    baseline_by_key = {}
    for row in baseline_rows:
        key = (row["sampleStem"], row["objectId"], row["nodeId"])
        baseline_by_key[key] = row

    family_defs = [
        ("above_single_black", lambda r, lc: r.get("layout") == "attached-group-above" and lc == 1 and (r.get("fill") or "").lower() == "#000000" and r.get("anchor") == "start"),
        ("above_multi_black", lambda r, lc: r.get("layout") == "attached-group-above" and lc > 1 and (r.get("fill") or "").lower() == "#000000" and r.get("anchor") == "start"),
        ("lateral_multi_black", lambda r, lc: r.get("layout") == "attached-group" and lc > 1 and (r.get("fill") or "").lower() == "#000000" and r.get("anchor") == "start"),
    ]

    reports = []
    for sweep_dir in sorted(p for p in sweeps_root.iterdir() if p.is_dir()):
        label_boxes_path = ensure_label_boxes(sweep_dir)
        label_rows = json.loads(label_boxes_path.read_text(encoding="utf-8"))["rows"]
        sweep_by_key = {(r["sampleStem"], r["objectId"], r["nodeId"]): r for r in label_rows}
        summary = json.loads((sweep_dir / "summary.json").read_text(encoding="utf-8"))

        matched = []
        for key, base_row in baseline_by_key.items():
            sweep_row = sweep_by_key.get(key)
            if not sweep_row:
                continue
            lc = line_count_by_key.get(key, 1)
            matched.append((key, base_row, sweep_row, lc))

        overall = {
            "avgLabelIouDelta": mean(s["iou"] - b["iou"] for _, b, s, _ in matched),
            "sumLabelResidualDelta": sum(b["residualCount"] - s["residualCount"] for _, b, s, _ in matched),
            "countMatched": len(matched),
        }

        families = {}
        for fam_name, pred in family_defs:
            items = [(b, s) for _, b, s, lc in matched if pred(b, lc)]
            if not items:
                continue
            families[fam_name] = {
                "count": len(items),
                "avgIouDelta": mean(s["iou"] - b["iou"] for b, s in items),
                "sumResidualDelta": sum(b["residualCount"] - s["residualCount"] for b, s in items),
                "avgBaseIou": mean(b["iou"] for b, _ in items),
                "avgSweepIou": mean(s["iou"] for _, s in items),
            }

        reports.append(
            {
                "sweep": sweep_dir.name,
                "policy": summary["policy"],
                "global": {
                    "avgBestIou": summary["avgBestIou"],
                    "avgDx": summary["avgDx"],
                    "avgDy": summary["avgDy"],
                    "count": summary["count"],
                },
                "overallLabels": overall,
                "families": families,
            }
        )

    output = {
        "reports": reports,
        "sortedByGlobal": sorted(reports, key=lambda r: r["global"]["avgBestIou"], reverse=True),
        "sortedByLabelDelta": sorted(reports, key=lambda r: r["overallLabels"]["avgLabelIouDelta"], reverse=True),
        "sortedByLateralMulti": sorted(
            reports,
            key=lambda r: r["families"].get("lateral_multi_black", {}).get("avgIouDelta", float("-inf")),
            reverse=True,
        ),
        "sortedByAboveMulti": sorted(
            reports,
            key=lambda r: r["families"].get("above_multi_black", {}).get("avgIouDelta", float("-inf")),
            reverse=True,
        ),
        "sortedByAboveSingle": sorted(
            reports,
            key=lambda r: r["families"].get("above_single_black", {}).get("avgIouDelta", float("-inf")),
            reverse=True,
        ),
    }
    (ROOT / args.output).write_text(json.dumps(output, ensure_ascii=False, indent=2), encoding="utf-8")
    print(ROOT / args.output)
    for report in output["sortedByGlobal"]:
        print(
            f"{report['global']['avgBestIou']:.9f} | labelΔ={report['overallLabels']['avgLabelIouDelta']:+.6f} | "
            f"{report['policy']}"
        )


if __name__ == "__main__":
    main()
