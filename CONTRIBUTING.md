# Contributing

Thanks for taking an interest in ChemCore. The project is still young, and the
most valuable contributions are precise bug reports, small reproducible fixtures,
and focused patches that keep behavior in the shared Rust engine.

## Development Setup

```bash
npm install
cargo test
npm run build:engine-wasm
```

The browser editor can be served from the repository root:

```bash
python -m http.server 8765 --bind 127.0.0.1 --directory .
```

Open `http://127.0.0.1:8765/viewer/`.

## Project Boundaries

- Chemistry semantics, hit testing, selection behavior, implicit hydrogens,
  label recognition, document mutation, and render primitives belong in
  `crates/chemcore-engine`.
- The browser viewer should handle UI, file flow, coordinate conversion, and
  SVG/DOM presentation without reimplementing chemistry behavior.
- The Windows desktop app should route system capabilities through the Tauri
  boundary and shared desktop service.
- Office/OLE integration belongs in `apps/chemcore-office`; it should not fork
  chemistry or rendering rules away from the engine.

## Before Opening a Pull Request

Run the narrowest useful tests while iterating, then run the broader checks
before submitting:

```bash
npm test
cargo test -p chemcore-engine
cargo test -p chemcore-office
npm run build:engine-wasm
```

For changes that affect generated WebAssembly bindings, include the updated
files under `viewer/engine`.

## Fixtures

Small, focused fixtures are welcome. Keep them minimal, deterministic, and
license-compatible. Do not add private documents, unpublished data, generated
debug output, screenshots from ignored `tmp/` directories, or files with unclear
redistribution rights.

## Optional Local Tooling

Some comparison scripts need Windows, Office, ChemDraw-compatible local
installations, or Python packages such as Pillow, NumPy, and SciPy. Use the
`CHEMCORE_PYTHON` environment variable to point scripts at a Python environment
with those optional packages installed.
