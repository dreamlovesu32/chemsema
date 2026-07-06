#!/usr/bin/env python3
"""Run chemcore-cli from an installed build or a checkout."""

from __future__ import annotations

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


def executable_from_env() -> Path | None:
    value = os.environ.get("CHEMCORE_CLI")
    if not value:
        return None
    path = Path(value)
    return path if path.is_file() else None


def executable_from_path() -> Path | None:
    found = shutil.which("chemcore-cli")
    return Path(found) if found else None


def executable_from_repo(repo: Path | None) -> Path | None:
    if not repo:
        return None
    for rel in ("target/release/chemcore-cli.exe", "target/debug/chemcore-cli.exe"):
        path = repo / rel
        if path.is_file():
            return path
    return None


def main(argv: list[str]) -> int:
    script_root = Path(__file__).resolve().parent
    repo = find_repo_root(script_root)
    exe = executable_from_env() or executable_from_path() or executable_from_repo(repo)
    if exe:
        return subprocess.run([str(exe), *argv]).returncode
    if repo:
        return subprocess.run(["cargo", "run", "-p", "chemcore-cli", "--", *argv], cwd=repo).returncode
    print("chemcore-cli was not found. Build it or set CHEMCORE_CLI.", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
