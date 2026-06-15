from __future__ import annotations

import os
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def python_executable() -> str:
    return os.environ.get("CHEMCORE_PYTHON", sys.executable)


def tmp_input_path(name: str) -> Path:
    return ROOT / "tmp" / name


def windows_font_path(name: str) -> Path:
    return Path(os.environ.get("CHEMCORE_WINDOWS_FONT", rf"C:\Windows\Fonts\{name}"))
