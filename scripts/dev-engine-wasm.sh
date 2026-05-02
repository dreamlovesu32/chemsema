#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WATCH_PATHS=(
  "$ROOT_DIR/Cargo.toml"
  "$ROOT_DIR/Cargo.lock"
  "$ROOT_DIR/crates/chemcore-engine/Cargo.toml"
  "$ROOT_DIR/crates/chemcore-engine/src"
)

hash_inputs() {
  find "${WATCH_PATHS[@]}" \
    \( -type f -name '*.rs' -o -type f -name 'Cargo.toml' -o -type f -name 'Cargo.lock' \) \
    -print0 \
    | sort -z \
    | xargs -0 sha256sum \
    | sha256sum \
    | awk '{print $1}'
}

build_engine() {
  printf '\n[dev:engine] rebuilding viewer engine wasm...\n'
  npm run build:engine-wasm
  printf '[dev:engine] rebuild complete\n'
}

build_engine
last_hash="$(hash_inputs)"

printf '[dev:engine] watching Rust engine sources. Press Ctrl-C to stop.\n'
while true; do
  sleep 1
  next_hash="$(hash_inputs)"
  if [[ "$next_hash" == "$last_hash" ]]; then
    continue
  fi
  last_hash="$next_hash"
  build_engine
done
