#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <out-dir> [--clean]" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT="$1"
CLEAN="${2:-}"
OUT_PARENT="$(dirname "$OUT")"
OUT_NAME="$(basename "$OUT")"
mkdir -p "$OUT_PARENT"
OUT_ABS="$(cd "$OUT_PARENT" && pwd -P)/$OUT_NAME"

if [[ "$CLEAN" == "--clean" && -d "$OUT" ]]; then
  rm -rf "$OUT"
fi
mkdir -p "$OUT"

while IFS= read -r -d '' skill_file; do
  skill_dir="$(dirname "$skill_file")"
  name="$(basename "$skill_dir")"
  rm -rf "$OUT/$name"
  cp -R "$skill_dir" "$OUT/$name"
  find "$OUT/$name" \
    \( -name '__pycache__' -o -name '.pytest_cache' -o -name '.mypy_cache' -o -name '.ruff_cache' \) \
    -type d -prune -exec rm -rf {} +
  find "$OUT/$name" \
    \( -name '*.pyc' -o -name '*.pyo' -o -name '*.log' -o -name '*.tmp' -o -name '*.bak' -o -name '*.orig' -o -name '*.rej' -o -name '*~' \) \
    -type f -delete
  echo "flattened $name -> $OUT/$name"
done < <(find "$ROOT" -path "$OUT_ABS" -prune -o -name SKILL.md -print0)
