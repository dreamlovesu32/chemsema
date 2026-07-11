#!/usr/bin/env python3
"""Run chemcore-cli from an installed or skill-bundled runtime."""

from __future__ import annotations

import subprocess
import sys

from chemcore_runtime import ChemCoreRuntimeNotFound, find_cli


def main(argv: list[str]) -> int:
    try:
        command, cwd, _source = find_cli()
    except ChemCoreRuntimeNotFound as exc:
        print(str(exc), file=sys.stderr)
        return 1
    return subprocess.run([*command, *argv], cwd=cwd).returncode


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
