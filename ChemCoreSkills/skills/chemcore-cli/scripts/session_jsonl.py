#!/usr/bin/env python3
"""Drive a chemcore-cli JSONL session from a request file."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path


def find_repo_root(start: Path) -> Path | None:
    current = start.resolve()
    for candidate in [current, *current.parents]:
        if (candidate / "Cargo.toml").exists() and (candidate / "package.json").exists():
            return candidate
    return None


def find_cli() -> tuple[list[str], Path | None]:
    env_path = os.environ.get("CHEMCORE_CLI")
    if env_path and Path(env_path).is_file():
        return [env_path], None
    path_cli = shutil.which("chemcore-cli")
    if path_cli:
        return [path_cli], None
    repo = find_repo_root(Path(__file__).resolve().parent) or find_repo_root(Path.cwd())
    if repo:
        for rel in ("target/release/chemcore-cli.exe", "target/debug/chemcore-cli.exe"):
            exe = repo / rel
            if exe.is_file():
                return [str(exe)], None
        return ["cargo", "run", "-p", "chemcore-cli", "--"], repo
    raise FileNotFoundError("chemcore-cli was not found. Build it or set CHEMCORE_CLI.")


def request_lines(path: Path) -> list[str]:
    lines = [line.strip() for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]
    for line in lines:
        json.loads(line)
    if not any(json.loads(line).get("op") == "exit" for line in lines):
        lines.append(json.dumps({"id": "__auto_exit__", "op": "exit"}, separators=(",", ":")))
    return lines


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("input", help="Input document path for chemcore-cli session.")
    parser.add_argument("requests", help="JSONL requests to send.")
    parser.add_argument("--out", required=True, help="Transcript JSONL path.")
    args = parser.parse_args()

    command, cwd = find_cli()
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
