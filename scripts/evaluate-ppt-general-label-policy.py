from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PYTHON = Path(r"D:\anaconda3\python.exe")
POWERSHELL = "powershell"


def run(args: list[str], *, cwd: Path | None = None, env: dict[str, str] | None = None) -> None:
    merged_env = os.environ.copy()
    if env:
        merged_env.update(env)
    result = subprocess.run(
        args,
        cwd=str(cwd or ROOT),
        text=True,
        capture_output=True,
        env=merged_env,
    )
    if result.stdout:
        print(result.stdout, end="")
    if result.stderr:
        print(result.stderr, end="", file=sys.stderr)
    if result.returncode != 0:
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(args)}")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Evaluate a packaged attached-label family policy across PPT same-shell samples."
    )
    parser.add_argument(
        "--pattern",
        default="tmp/ppt-sample-*/same-shell-compare/*.payload.json",
        help="Glob for existing same-shell payloads.",
    )
    parser.add_argument(
        "--out-root",
        default="tmp/ppt-general-policy-v1",
        help="Output root mirroring sample/same-shell-compare structure.",
    )
    parser.add_argument(
        "--policy",
        default="ppt-box-v1",
        help="Value for CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_GENERAL_POLICY_EXPERIMENT.",
    )
    args = parser.parse_args()

    payload_paths = sorted(ROOT.glob(args.pattern))
    out_root = ROOT / args.out_root
    out_root.mkdir(parents=True, exist_ok=True)
    env = {
        "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_GENERAL_POLICY_EXPERIMENT": args.policy,
    }

    results = []
    failures = []
    for payload_path in payload_paths:
        compare_dir = payload_path.parent
        sample_dir = compare_dir.parent
        sample_name = sample_dir.name
        stem = payload_path.name.removesuffix(".payload.json")
        ref_docx = compare_dir / f"{stem}.chemdraw-shell.docx"
        ref_png = compare_dir / f"{stem}.chemdraw.wordcopy.png"
        if not ref_docx.exists() or not ref_png.exists():
            failures.append({"payload": str(payload_path), "reason": "missing reference asset"})
            continue

        dst_dir = out_root / sample_name / "same-shell-compare"
        dst_dir.mkdir(parents=True, exist_ok=True)
        dst_payload = dst_dir / payload_path.name
        shutil.copy2(payload_path, dst_payload)
        shutil.copy2(ref_docx, dst_dir / ref_docx.name)
        shutil.copy2(ref_png, dst_dir / ref_png.name)

        ours_docx = dst_dir / f"{stem}.chemcore.docx"
        ours_png = dst_dir / f"{stem}.chemcore.wordcopy.png"
        bestshift = dst_dir / f"{stem}.bestshift.json"

        try:
            run(
                [
                    "cargo",
                    "run",
                    "-q",
                    "-p",
                    "chemcore-office",
                    "--",
                    "--write-word-docx-payload",
                    str(dst_payload),
                    str(ours_docx),
                ],
                env=env,
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
                    str(PYTHON),
                    str(ROOT / "scripts" / "png-best-shift.py"),
                    "--output",
                    str(bestshift),
                    str(ours_png),
                    str(ref_png),
                ]
            )
            shift = json.loads(bestshift.read_text(encoding="utf-8"))
            results.append(
                {
                    "sample": sample_name,
                    "stem": stem,
                    "bestIou": shift["best_iou"],
                    "dx": shift["dx"],
                    "dy": shift["dy"],
                }
            )
        except Exception as exc:  # noqa: BLE001
            failures.append({"payload": str(payload_path), "reason": repr(exc)})

    avg_best_iou = sum(item["bestIou"] for item in results) / len(results) if results else None
    avg_dx = sum(item["dx"] for item in results) / len(results) if results else None
    avg_dy = sum(item["dy"] for item in results) / len(results) if results else None
    report = {
        "policy": args.policy,
        "count": len(results),
        "avgBestIou": avg_best_iou,
        "avgDx": avg_dx,
        "avgDy": avg_dy,
        "results": results,
        "failures": failures,
    }
    summary_path = out_root / "summary.json"
    summary_path.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(summary_path)
    print(json.dumps({"count": len(results), "avgBestIou": avg_best_iou, "avgDx": avg_dx, "avgDy": avg_dy, "failures": len(failures)}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
