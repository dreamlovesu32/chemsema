# ChemCore

> Maintainer opening goes here.

## Technical Overview

ChemCore is a cross-platform chemistry document editor built around a shared
Rust core. The core owns the document model, editing behavior, hit testing,
chemical label logic, CDXML/CDX-oriented import and export, and render primitive
generation. Browser, desktop, and Office/OLE hosts consume that shared behavior
rather than reimplementing chemistry rules in each surface.

## Repository Layout

```text
chemcore/
  crates/chemcore-engine/          Rust document, editing, rendering, CDXML, and WASM core
  crates/chemcore-desktop-service/ Native desktop engine sessions and file helpers
  apps/chemcore-desktop/           Tauri Windows desktop application
  apps/chemcore-office/            Windows Office/OLE integration server
  viewer/                          Browser editor host and generated WASM package
  docs/                            Architecture, format, rendering, and behavior notes
  examples/                        Example ChemCore native documents
  scripts/                         Build, verification, and regression helpers
  shared/                          Shared JSON data consumed by Rust and viewer code
```

## Prerequisites

- Rust stable with the MSVC toolchain on Windows
- Node.js and npm
- Python 3 for local static serving and some optional analysis scripts
- `wasm-pack` is installed automatically by `npm run build:engine-wasm` when needed
- Windows is required for the desktop shell and Office/OLE integration paths

## Quick Start

```bash
npm install
cargo test
npm run build:engine-wasm
```

Run the browser editor from the repository root:

```bash
python -m http.server 8765 --bind 127.0.0.1 --directory .
```

Then open:

```text
http://127.0.0.1:8765/viewer/
```

Run the Windows desktop shell:

```bash
npm run desktop:dev
```

Build release binaries:

```bash
npm run desktop:build-fast
cargo build -p chemcore-office --release
```

Register the Office/OLE integration for the current user:

```bash
npm run office:register-dev
```

Unregister it:

```bash
npm run office:unregister-dev
```

## Verification

The main verification command is:

```bash
npm run verify
```

It runs Rust tests, rebuilds the browser engine WASM, checks viewer JavaScript
syntax, and verifies that generated `viewer/engine` files are synchronized.

Useful focused commands:

```bash
npm test
cargo test -p chemcore-engine
cargo test -p chemcore-office
npm run build:engine-wasm
node --check viewer/app.js
```

Some scripts compare output against locally installed desktop applications or
Office. Those flows are optional and may require Windows-specific software,
local documents, or `CHEMCORE_PYTHON` to point at a Python environment with the
needed analysis packages.

## Design Documents

- [docs/architecture.md](./docs/architecture.md)
- [docs/format-v0.1.md](./docs/format-v0.1.md)
- [docs/project-rules.zh-CN.md](./docs/project-rules.zh-CN.md)
- [docs/implicit-hydrogen-rules.zh-CN.md](./docs/implicit-hydrogen-rules.zh-CN.md)
- [docs/abbreviation-recognition-rules.zh-CN.md](./docs/abbreviation-recognition-rules.zh-CN.md)
- [docs/bond-rendering-rules.zh-CN.md](./docs/bond-rendering-rules.zh-CN.md)
- [docs/editor-command-history.md](./docs/editor-command-history.md)
- [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)

## License

ChemCore is licensed under the Apache License, Version 2.0. See
[LICENSE](./LICENSE) and [NOTICE](./NOTICE).
