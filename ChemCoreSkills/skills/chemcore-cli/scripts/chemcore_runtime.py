#!/usr/bin/env python3
"""Locate a ChemCore CLI runtime for bundled skill helpers."""

from __future__ import annotations

import json
import os
import platform
import shutil
from pathlib import Path


class ChemCoreRuntimeNotFound(FileNotFoundError):
    """Raised when a helper cannot find chemcore-cli."""


def skill_root() -> Path:
    return Path(__file__).resolve().parents[1]


def platform_tag() -> str:
    system = platform.system().lower()
    machine = platform.machine().lower()
    if machine in {"amd64", "x86_64"}:
        arch = "x64"
    elif machine in {"arm64", "aarch64"}:
        arch = "arm64"
    else:
        arch = machine or "unknown"

    if system == "windows":
        os_name = "win"
    elif system == "darwin":
        os_name = "macos"
    elif system == "linux":
        os_name = "linux"
    else:
        os_name = system or "unknown"
    return f"{os_name}-{arch}"


def executable_name() -> str:
    return "chemcore-cli.exe" if platform.system().lower() == "windows" else "chemcore-cli"


def executable_from_env() -> Path | None:
    value = os.environ.get("CHEMCORE_CLI")
    if not value:
        return None
    path = Path(value)
    return path if path.is_file() else None


def executable_from_path() -> Path | None:
    found = shutil.which("chemcore-cli")
    return Path(found) if found else None


def bundled_manifest_path(root: Path) -> Path:
    return root / "assets" / "runtime-manifest.json"


def executable_from_manifest(root: Path, tag: str) -> Path | None:
    manifest_path = bundled_manifest_path(root)
    if not manifest_path.is_file():
        return None
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return None
    entry = manifest.get("platforms", {}).get(tag)
    if not isinstance(entry, dict):
        return None
    rel_path = entry.get("path")
    if not isinstance(rel_path, str) or not rel_path:
        return None
    candidate = root / "assets" / rel_path
    return candidate if candidate.is_file() else None


def executable_from_bundled_assets(root: Path | None = None) -> Path | None:
    root = root or skill_root()
    tag = platform_tag()
    from_manifest = executable_from_manifest(root, tag)
    if from_manifest:
        return from_manifest
    candidate = root / "assets" / "bin" / tag / executable_name()
    return candidate if candidate.is_file() else None


def find_cli() -> tuple[list[str], Path | None, str]:
    env_exe = executable_from_env()
    if env_exe:
        return [str(env_exe)], None, "CHEMCORE_CLI"

    path_exe = executable_from_path()
    if path_exe:
        return [str(path_exe)], None, "PATH"

    bundled_exe = executable_from_bundled_assets()
    if bundled_exe:
        return [str(bundled_exe)], None, f"bundled:{platform_tag()}"

    tag = platform_tag()
    raise ChemCoreRuntimeNotFound(
        "chemcore-cli was not found. Install the self-contained ChemCore CLI skill "
        f"with assets/bin/{tag}/{executable_name()}, install ChemCore CLI on PATH, "
        "or set CHEMCORE_CLI to an executable path. Source checkout builds are "
        "handled by the chemcore-development skill."
    )
