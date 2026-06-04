from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
PYTHON = Path(os.environ.get("CHEMCORE_PYTHON", sys.executable))
POWERSHELL = "powershell"


def run(args: list[str], *, cwd: Path | None = None) -> None:
    result = subprocess.run(
        args,
        cwd=str(cwd or ROOT),
        text=True,
        capture_output=True,
    )
    if result.stdout:
        print(result.stdout, end="")
    if result.stderr:
        print(result.stderr, end="", file=sys.stderr)
    if result.returncode != 0:
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(args)}")


def parse_indices(spec: str | None, total: int) -> list[int]:
    if not spec:
        return list(range(total))
    out: list[int] = []
    for part in spec.split(","):
        part = part.strip()
        if not part:
            continue
        if "-" in part:
            start_s, end_s = part.split("-", 1)
            start = int(start_s)
            end = int(end_s)
            out.extend(range(start, end + 1))
        else:
            out.append(int(part))
    return [i for i in out if 0 <= i < total]


def safe_stem(entry: dict[str, Any]) -> str:
    slide = Path(entry["slide"]).stem
    bin_stem = Path(entry["binTarget"]).stem
    return f"{slide}.{bin_stem}"


def compare_entry(manifest_dir: Path, entry: dict[str, Any], out_dir: Path) -> dict[str, Any]:
    stem = safe_stem(entry)
    cdxml_path = manifest_dir / entry["cdxml"]
    preview_path = manifest_dir / entry["preview"]
    payload_path = out_dir / f"{stem}.payload.json"
    ours_docx = out_dir / f"{stem}.chemcore.docx"
    ref_docx = out_dir / f"{stem}.chemdraw-shell.docx"
    ours_png = out_dir / f"{stem}.chemcore.wordcopy.png"
    ref_png = out_dir / f"{stem}.chemdraw.wordcopy.png"
    shift_json = out_dir / f"{stem}.bestshift.json"

    run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "chemcore-engine",
            "--example",
            "cdxml_to_clipboard_payload",
            "--",
            str(cdxml_path),
            str(payload_path),
        ]
    )
    run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "chemcore-office",
            "--",
            "--write-word-docx-payload",
            str(payload_path),
            str(ours_docx),
        ]
    )
    run(
        [
            str(PYTHON),
            str(ROOT / "scripts" / "patch-docx-image1.py"),
            str(ours_docx),
            str(preview_path),
            str(ref_docx),
        ]
    )
    run(
        [
            POWERSHELL,
            "-ExecutionPolicy",
            "Bypass",
            "-NoProfile",
            "-File",
            str(ROOT / "scripts" / "word-copy-inline-shape.ps1"),
            "-InputDocx",
            str(ours_docx),
            "-OutputPng",
            str(ours_png),
        ]
    )
    run(
        [
            POWERSHELL,
            "-ExecutionPolicy",
            "Bypass",
            "-NoProfile",
            "-File",
            str(ROOT / "scripts" / "word-copy-inline-shape.ps1"),
            "-InputDocx",
            str(ref_docx),
            "-OutputPng",
            str(ref_png),
        ]
    )
    run(
        [
            str(PYTHON),
            str(ROOT / "scripts" / "png-best-shift.py"),
            "--output",
            str(shift_json),
            str(ours_png),
            str(ref_png),
        ]
    )
    shift = json.loads(shift_json.read_text(encoding="utf-8"))
    return {
        "stem": stem,
        "slide": entry["slide"],
        "binTarget": entry["binTarget"],
        "preview": entry["preview"],
        "cdxml": entry["cdxml"],
        "bestShift": shift,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Compare extracted ChemDraw OLE objects against their PPT preview in the same Word shell.")
    parser.add_argument("manifest_dir", help="Directory produced by extract-ppt-chemdraw.py")
    parser.add_argument("--out", required=True, help="Output directory for generated artifacts")
    parser.add_argument("--indices", help="Entry indices to process, e.g. 0,2-4")
    args = parser.parse_args()

    manifest_dir = Path(args.manifest_dir).resolve()
    out_dir = Path(args.out).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    manifest = json.loads((manifest_dir / "manifest.json").read_text(encoding="utf-8"))
    entries = manifest["entries"]
    indices = parse_indices(args.indices, len(entries))
    results: list[dict[str, Any]] = []
    failures: list[dict[str, Any]] = []

    for index in indices:
        entry = entries[index]
        if not entry.get("preview") or not str(entry["preview"]).lower().endswith(".emf"):
            failures.append({"index": index, "reason": "preview-not-emf", "entry": entry})
            continue
        try:
            result = compare_entry(manifest_dir, entry, out_dir)
            result["index"] = index
            results.append(result)
        except Exception as exc:  # noqa: BLE001
            failures.append(
                {
                    "index": index,
                    "reason": "compare-failed",
                    "error": repr(exc),
                    "entry": entry,
                }
            )

    report = {
        "manifestDir": str(manifest_dir),
        "outDir": str(out_dir),
        "indices": indices,
        "resultCount": len(results),
        "failureCount": len(failures),
        "results": results,
        "failures": failures,
    }
    (out_dir / "summary.json").write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"resultCount": len(results), "failureCount": len(failures), "summary": str(out_dir / "summary.json")}, ensure_ascii=False))


if __name__ == "__main__":
    main()
