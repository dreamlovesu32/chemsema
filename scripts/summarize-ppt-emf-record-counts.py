from __future__ import annotations

import argparse
import json
import statistics as st
import subprocess
import tempfile
import zipfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
EMF_INSPECT = ROOT / "scripts" / "emf-inspect.mjs"
NODE = "node"

KEY_GDI = [
    "EMR_EXTTEXTOUTW",
    "EMR_EXTCREATEFONTINDIRECTW",
    "EMR_EXTCREATEPEN",
    "EMR_POLYLINE16",
    "EMR_POLYGON16",
    "EMR_MODIFYWORLDTRANSFORM",
]

KEY_PLUS = [
    "EmfPlusDrawString",
    "EmfPlusObject",
    "EmfPlusDrawLines",
    "EmfPlusFillPolygon",
    "EmfPlusSave",
    "EmfPlusRestore",
    "EmfPlusSetPageTransform",
]


def inspect_docx_image1(docx: Path, tmpdir: Path) -> dict[str, Any]:
    emf_path = tmpdir / f"{docx.stem}.emf"
    out_path = tmpdir / f"{docx.stem}.json"
    with zipfile.ZipFile(docx, "r") as zf:
        emf_path.write_bytes(zf.read("word/media/image1.emf"))
    result = subprocess.run(
        [NODE, str(EMF_INSPECT), "--out", str(out_path), str(emf_path)],
        cwd=str(ROOT),
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr or result.stdout)
    return json.loads(out_path.read_text(encoding="utf-8"))


def summarize_compare_dir(compare_dir: Path) -> list[dict[str, Any]]:
    summary = json.loads((compare_dir / "summary.json").read_text(encoding="utf-8"))
    rows: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory() as td:
        tmpdir = Path(td)
        for item in summary.get("results", []):
            stem = item["stem"]
            ours = inspect_docx_image1(compare_dir / f"{stem}.chemcore.docx", tmpdir)
            ref = inspect_docx_image1(compare_dir / f"{stem}.chemdraw-shell.docx", tmpdir)
            row: dict[str, Any] = {
                "sample": compare_dir.parent.name,
                "stem": stem,
                "bestIou": item["bestShift"]["best_iou"],
                "dx": item["bestShift"]["dx"],
                "dy": item["bestShift"]["dy"],
            }
            for key in KEY_GDI:
                row[f"{key}_ours"] = ours.get("typeCounts", {}).get(key, 0)
                row[f"{key}_ref"] = ref.get("typeCounts", {}).get(key, 0)
            for key in KEY_PLUS:
                row[f"{key}_ours"] = ours.get("emfPlusCounts", {}).get(key, 0)
                row[f"{key}_ref"] = ref.get("emfPlusCounts", {}).get(key, 0)
            rows.append(row)
    return rows


def aggregate(rows: list[dict[str, Any]]) -> dict[str, Any]:
    aggregate: dict[str, Any] = {}
    for key in KEY_GDI + KEY_PLUS:
        ours = [row[f"{key}_ours"] for row in rows]
        ref = [row[f"{key}_ref"] for row in rows]
        aggregate[key] = {
            "avgOurs": st.mean(ours),
            "avgRef": st.mean(ref),
            "avgDelta": st.mean(o - r for o, r in zip(ours, ref)),
            "identicalCount": sum(1 for o, r in zip(ours, ref) if o == r),
        }
    return aggregate


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Summarize ours/ref EMF record-count differences across same-shell PPT compare outputs."
    )
    parser.add_argument("compare_dirs", nargs="+", help="Directories containing same-shell comparison outputs")
    parser.add_argument("--output", required=True, help="Path to write JSON summary")
    args = parser.parse_args()

    rows: list[dict[str, Any]] = []
    for compare_dir_arg in args.compare_dirs:
        rows.extend(summarize_compare_dir(Path(compare_dir_arg)))

    report = {
        "count": len(rows),
        "aggregate": aggregate(rows),
        "rows": rows,
    }

    out_path = Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(out_path)
    print(json.dumps(report["aggregate"], indent=2))


if __name__ == "__main__":
    main()
