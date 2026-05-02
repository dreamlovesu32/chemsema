#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT_DIR"

cargo test
npm run build:engine-wasm
node --check viewer/app.js

if [[ -n "$(git status --porcelain -- viewer/engine)" ]]; then
  git status --short -- viewer/engine
  git diff -- viewer/engine
  exit 1
fi
