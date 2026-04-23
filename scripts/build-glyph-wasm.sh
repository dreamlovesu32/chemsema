#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EMSDK_DIR="${EMSDK_DIR:-/home/jiajun/src/emsdk}"
EM_PYTHON="${EM_PYTHON:-/home/jiajun/ENTER/envs/chemcore/bin/python3}"

if [ -x "${EM_PYTHON}" ]; then
  export EM_PYTHON
  PATH="$(dirname "${EM_PYTHON}"):${PATH}"
  export PATH
fi

if ! command -v emcmake >/dev/null 2>&1; then
  if [ -f "${EMSDK_DIR}/emsdk_env.sh" ]; then
    export EMSDK_QUIET=1
    # shellcheck source=/dev/null
    source "${EMSDK_DIR}/emsdk_env.sh" >/dev/null
  fi
fi

if ! command -v emcmake >/dev/null 2>&1; then
  echo "emcmake not found. Install Emscripten or set EMSDK_DIR." >&2
  exit 1
fi

emcmake cmake -S "${ROOT_DIR}" -B "${ROOT_DIR}/build-wasm" -DCMAKE_BUILD_TYPE=Release
cmake --build "${ROOT_DIR}/build-wasm" --target chemcore_glyph_kernel_wasm
