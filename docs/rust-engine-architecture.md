# ChemCore Rust Engine

ChemCore editing capability is governed by the Rust core. Web, Windows, and iPad surfaces are responsible for UI, input events, file systems, and pixel rendering; the document model, hit testing, snapping, tool state, and command behavior belong in the same engine.

## Target Boundary

`crates/chemcore-engine` is responsible for:

- `.ccjs` / `.ccjz` native document model and serialization
- editing tool state
- processing normalized pointer/key events
- hit testing endpoints, bonds, labels, shapes, and other objects
- chemical drawing snapping rules such as bond length, angle, and empty-angle choices
- document mutation and undo/redo command model
- geometry display lists or renderable overlay output

Platform shells are responsible for:

- toolbar, menus, and file open/save
- concrete rendering through DOM/SVG/Canvas/Skia/CoreGraphics or equivalent backends
- collecting pointer/key/menu/file events
- calling the engine and rendering engine output

## Relationship Between WASM And Native

WASM is the browser/WebView runtime form of the same Rust `chemcore-engine`.

The long-term runtime boundary is:

- Browser: call the WASM core through `WasmEngineHost`.
- Windows desktop default hot editing path: call the WebView WASM core through `DesktopHybridEngineHost`.
- Windows desktop system capabilities: call the native desktop service through Tauri commands.
- `TauriEngineHost` / `?engine=tauri-native`: retained for diagnostics and future native editor-path validation; the current desktop default hot interaction path uses `DesktopHybridEngineHost`.

High-frequency editing behavior such as pointer move, hover, focus, hit testing, selection, drag preview, rotate/scale/move, and object settings should not synchronously cross Tauri IPC and fetch full JSON snapshots. Unless the native path has proven incremental updates, event coalescing, and large-file performance, these behaviors must stay in the in-process core runtime.

Whether the call shape is WASM or native, chemical drawing rules, hit testing, selection semantics, and document mutation must be implemented in the Rust engine. The viewer may show forms and buttons, but must not reimplement object settings, context menus, rotation/scaling, or chemical bond behavior.

## Current Implementation

The first Rust engine version has taken over the single-bond drawing path in the Web editor:

- blank document creation
- single-bond tool state
- endpoint hover hit testing
- adding a horizontal single bond on blank canvas click
- extending from an endpoint by 120 degrees
- fixed bond length and angle snapping while dragging
- WASM API output for current native document JSON and overlay state
- bond-center focus under the bond tool
- cycling a clicked bond center under the single-bond button through offset double, centered equal-length double, and opposite-side offset double
- selecting a single endpoint or bond
- deleting the current selection
- snapshot-based undo/redo skeleton

The old JS single-bond geometry, hit testing, snapping, and add-bond logic has been removed from `viewer/app.js`. The viewer now only converts pointer events into document coordinates and passes them to the Rust WASM engine.

## Build

Rust tests:

```bash
cargo test
```

Web engine WASM:

```bash
npm run build:engine-wasm
```

Auto-rebuild for high-frequency development:

```bash
npm run dev:engine
```

Full verification before commit or delivery:

```bash
npm run verify
```

The viewer uses `viewer/engine/chemcore_engine.js`, `viewer/engine/chemcore_engine.d.ts`, and `viewer/engine/chemcore_engine_bg.wasm`. These files are current runtime artifacts for the Web shell; routine development may leave them briefly out of sync, but they must be synchronized with Rust core changes before viewer validation or commit.
