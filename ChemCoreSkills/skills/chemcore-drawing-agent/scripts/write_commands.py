#!/usr/bin/env python3
"""Write a JSON command array from line-delimited JSON command snippets."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True)
    parser.add_argument("commands", nargs="*", help="JSON command objects. If empty, read stdin lines.")
    args = parser.parse_args()

    sources = args.commands or [line.strip() for line in sys.stdin if line.strip()]
    parsed = [json.loads(item) for item in sources]
    Path(args.out).write_text(json.dumps(parsed, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
