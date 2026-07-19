#!/usr/bin/env python3
"""Drive a chemsema-cli JSONL session from a request file."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

from chemsema_runtime import find_cli


def request_lines(path: Path) -> list[str]:
    lines = [line.strip() for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]
    for line in lines:
        json.loads(line)
    if not any(json.loads(line).get("op") == "exit" for line in lines):
        lines.append(json.dumps({"id": "__auto_exit__", "op": "exit"}, separators=(",", ":")))
    return lines


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("input", help="Input document path for chemsema-cli session.")
    parser.add_argument("requests", help="JSONL requests to send.")
    parser.add_argument("--out", required=True, help="Transcript JSONL path.")
    args = parser.parse_args()

    command, cwd, _source = find_cli()
    proc = subprocess.Popen(
        [*command, "session", args.input],
        cwd=cwd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    assert proc.stdin is not None
    assert proc.stdout is not None

    responses: list[str] = []
    for line in request_lines(Path(args.requests)):
        proc.stdin.write(line + "\n")
        proc.stdin.flush()
        response = proc.stdout.readline()
        if not response:
            break
        responses.append(response.rstrip("\n"))

    proc.stdin.close()
    stderr = proc.stderr.read() if proc.stderr else ""
    code = proc.wait()
    Path(args.out).write_text("\n".join(responses) + ("\n" if responses else ""), encoding="utf-8")
    if stderr:
        print(stderr, file=sys.stderr)
    return code


if __name__ == "__main__":
    raise SystemExit(main())
