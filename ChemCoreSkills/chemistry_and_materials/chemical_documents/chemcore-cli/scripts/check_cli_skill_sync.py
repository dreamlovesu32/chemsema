#!/usr/bin/env python3
"""Check that ChemCore skill docs mention runtime CLI commands and formats."""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any


def find_repo_root(start: Path) -> Path | None:
    current = start.resolve()
    for candidate in [current, *current.parents]:
        if (candidate / "Cargo.toml").exists() and (candidate / "package.json").exists():
            return candidate
    return None


def find_suite_root(start: Path) -> Path:
    current = start.resolve()
    for candidate in [current, *current.parents]:
        if (candidate / "manifest.json").exists() and (candidate / "README_ZH.md").exists():
            return candidate
    return current


def cli_command() -> tuple[list[str], Path | None]:
    env_path = os.environ.get("CHEMCORE_CLI")
    if env_path and Path(env_path).is_file():
        return [env_path], None
    path_cli = shutil.which("chemcore-cli")
    if path_cli:
        return [path_cli], None
    repo = find_repo_root(Path.cwd()) or find_repo_root(Path(__file__).resolve())
    if repo:
        for rel in ("target/release/chemcore-cli.exe", "target/debug/chemcore-cli.exe"):
            exe = repo / rel
            if exe.is_file():
                return [str(exe)], None
        return ["cargo", "run", "-p", "chemcore-cli", "--"], repo
    raise FileNotFoundError("chemcore-cli was not found. Build it or set CHEMCORE_CLI.")


def runtime_capabilities() -> dict[str, Any]:
    command, cwd = cli_command()
    result = subprocess.run(
        [*command, "capabilities"],
        cwd=cwd,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    return json.loads(result.stdout)


def markdown_text(root: Path) -> str:
    parts: list[str] = []
    for path in sorted(root.rglob("*.md")):
        if any(part in {".git", ".generated"} for part in path.parts):
            continue
        parts.append(path.read_text(encoding="utf-8"))
    return "\n".join(parts)


def token_present(text: str, token: str) -> bool:
    return re.search(rf"(?<![A-Za-z0-9-]){re.escape(token)}(?![A-Za-z0-9-])", text) is not None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--suite-root",
        default=None,
        help="ChemCoreSkills root. Defaults to the nearest manifest parent or current tree.",
    )
    parser.add_argument("--json", action="store_true", help="Print JSON.")
    args = parser.parse_args()

    suite_root = Path(args.suite_root).resolve() if args.suite_root else find_suite_root(Path(__file__).resolve())
    caps = runtime_capabilities()
    text = markdown_text(suite_root)

    commands = [item["name"] for item in caps.get("commands", []) if isinstance(item, dict) and "name" in item]
    formats = sorted(
        {
            str(fmt)
            for values in caps.get("formats", {}).values()
            if isinstance(values, list)
            for fmt in values
        }
    )
    missing_commands = [name for name in commands if not token_present(text, name)]
    missing_formats = [name for name in formats if not token_present(text, name)]

    report = {
        "ok": not missing_commands and not missing_formats,
        "suiteRoot": str(suite_root),
        "commands": commands,
        "formats": formats,
        "missingCommands": missing_commands,
        "missingFormats": missing_formats,
    }

    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
    else:
        print(f"ok: {report['ok']}")
        print(f"suiteRoot: {report['suiteRoot']}")
        print(f"missingCommands: {', '.join(missing_commands) if missing_commands else '(none)'}")
        print(f"missingFormats: {', '.join(missing_formats) if missing_formats else '(none)'}")
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
