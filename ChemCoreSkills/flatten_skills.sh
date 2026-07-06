#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <out-dir> [--clean]" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT="$1"
CLEAN="${2:-}"
OUT_ABS="$(python -c 'import os,sys; print(os.path.abspath(sys.argv[1]))' "$OUT")"

if [[ "$CLEAN" == "--clean" && -d "$OUT" ]]; then
  rm -rf "$OUT"
fi
mkdir -p "$OUT"

while IFS= read -r -d '' skill_file; do
  skill_dir="$(dirname "$skill_file")"
  name="$(basename "$skill_dir")"
  rm -rf "$OUT/$name"
  cp -R "$skill_dir" "$OUT/$name"
  echo "flattened $name -> $OUT/$name"
done < <(find "$ROOT" -path "$OUT_ABS" -prune -o -name SKILL.md -print0)
