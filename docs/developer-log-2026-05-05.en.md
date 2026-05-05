# Chemcore Developer Log - 2026-05-05

Author: Jiajun Zhang

Time range: 2026-05-05 00:00 to 2026-05-05 23:59, Asia/Shanghai

Baseline commit: `56cdd4a feat: adopt ccjs and ccjz document extensions`

Workspace: `D:\chemcore`

### Summary

Today's main thread was moving the project from a WSL handoff into a Windows-native development environment and confirming that `chemcore-engine`, the WASM viewer, npm scripts, Git, VS Code, and browser save flow can work directly on Windows. The architecture direction remains unchanged: Rust `chemcore-engine` is authoritative for the document model, editing semantics, import/export, hit testing, and render primitives; `viewer/` is only the browser adapter.

The viewer save failure was also fixed today. The direct cause was that `viewer/document_flow.js` used `CHEMCORE_TEXT_EXTENSION` without importing it, so clicking Save as threw a `ReferenceError`. The save order was also hardened so the browser save picker opens first inside the user click gesture, and content generation/compression happens afterward. This avoids Chromium File System Access API failures caused by losing transient user activation during async compression.

The temporary root-level `HANDOFF-2026-05-05.md` has been folded into this log, including migration context, recent commits, handoff notes, known risks, and future direction. That temporary handoff file has been deleted and is no longer kept as a separate handoff document.

### Migration State Received Today

The migration target is `D:\chemcore`, corresponding to `/mnt/d/chemcore` in WSL. The original WSL source directory was `/home/jiajun/chemcore`. The migration was a full copy, including `.git`, `target/`, `node_modules/`, `tmp/`, generated `viewer/engine` artifacts, and the worktree state at the time.

Both worktrees were aligned at:

```text
56cdd4a feat: adopt ccjs and ccjz document extensions
```

The two recent development commits that needed to be carried forward were:

```text
0e4e0e1 feat: improve cdxml rendering fidelity
56cdd4a feat: adopt ccjs and ccjz document extensions
```

`0e4e0e1` continued improving CDXML import, internal representation, save, re-import, and rendering fidelity:

- Added a unified CDXML color-table import/export path.
- Matched ChemDraw color semantics: `color="0"` is foreground, `bgcolor="1"` is background, and `<colortable>` entries start at id `2`.
- Routed line, shape, text, text-run, fragment-label, page-background, and object-style colors through the unified table.
- Expanded internal arrow semantics: solid/hollow/open, single/double-headed, half arrows, curved arrows, bold arrows, and no-go cross/hash markers.
- Expanded `render_objects/arrows.rs` toward ChemDraw-level arrow rendering behavior.
- Stopped treating molecule labels as ordinary text objects; structural labels can preserve `lineRuns`.
- Added text-symbol and glyph-profile paths, including `shared/text_symbols.json`, `viewer/text_symbol_palette.js`, glyph-profile generation, and regression scripts.
- Expanded Rust tests, especially CDXML, text, arrow, glyph, and render-stability coverage in `crates/chemcore-engine/tests/render_document.rs`.

`56cdd4a` moved native file entry points to `.ccjs` / `.ccjz`:

- `.ccjz` is the default product save format: gzip-compressed chemcore JSON.
- `.ccjs` is the readable debug format: plain-text chemcore JSON.
- Viewer file flow supports opening `.ccjz`, `.ccjs`, and `.cdxml`, plus saving `.ccjz`, `.ccjs`, and exporting `.cdxml` / `.svg`.
- The example file moved from `examples/document-v0.1.json` to `examples/document-v0.1.ccjs`.
- README, format docs, project rules, and viewer rendering reports were updated to use `.ccjs/.ccjz` terminology.

### Windows-Native Environment Rebuild

The Windows toolchain was rebuilt and normalized today. The guiding rule was to install tools onto D drive where practical; if existing system tools were not new enough or not suitable, the Windows-native path was updated rather than forcing compatibility.

Main tools verified or configured:

```text
Git:                 D:\Git
Git version:         2.54.0.windows.1
Git Bash:            D:\Git\bin\bash.exe
Git Bash version:    GNU bash 5.3.9

Rust cargo home:     D:\Rust\cargo
Rust rustup home:    D:\Rust\rustup
Rust toolchain:      stable-x86_64-pc-windows-msvc
rustc:               1.95.0
cargo:               1.95.0
Rust targets:        x86_64-pc-windows-msvc, wasm32-unknown-unknown
wasm-pack:           0.14.0

Node installed path: D:\nodejs-24.15.0
Node installed ver.: 24.15.0
npm installed ver.:  11.12.1

Playwright browsers: D:\ms-playwright
VS Build Tools:      C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools
VS Build version:    17.14.36717.8 / 17.14.21
```

One operational detail remains important: the already-running Codex/PowerShell process may still resolve the old `D:\nodejs` and WSL `bash.exe`, because process PATH is fixed at launch time. The user PATH has been updated to put these entries first:

```text
D:\nodejs-24.15.0
D:\Rust\cargo\bin
D:\Git\cmd
D:\Git\bin
D:\Git\usr\bin
```

New PowerShell windows, VS Code terminals, or reloaded development sessions should pick up the newer Node, Rust, and Git Bash. npm `script-shell` is set to:

```text
D:\Git\bin\bash.exe
```

So scripts such as `npm run build:engine-wasm` and `npm run verify`, which call `bash scripts/*.sh`, should use Git Bash instead of WSL bash.

### Git, VS Code, And Windows Filesystem Handling

Git was configured as follows:

```powershell
git config --global core.autocrlf false
git config --global core.eol lf
git config core.filemode false
```

These settings avoid CRLF churn, avoid NTFS/WSL file-mode noise, and make `D:\chemcore` the main Windows-native development directory instead of editing the old WSL copy through `\\wsl$`.

VS Code's built-in Git support is enough for tracking status; no extension is required. `%APPDATA%\Code\User\settings.json` was updated with:

```json
{
  "git.path": "D:\\Git\\cmd\\git.exe",
  "git.enabled": true
}
```

GitLens is optional, not required. If VS Code still does not show Git status, verify that it opened local `D:\chemcore` rather than a WSL remote window, then reload VS Code.

The Windows copy contained WSL/Windows metadata filenames using a private-use colon glyph, such as `*Zone.Identifier` and `*mshield`. Those untracked files under `compare/` were removed, and `.gitignore` now ignores:

```text
*Zone.Identifier
*mshield
```

### Dependency Install And Validation Results

`npm install` was run. npm 11 wrote root package metadata into `package-lock.json`, so the lockfile gained a small metadata-only diff. The install showed cleanup warnings for old `.bin` temporary symlinks, but they did not block installation or validation.

Validation completed:

```text
cargo test
npm test
npm run build:engine-wasm
node --check viewer/app.js
node --check viewer/document_flow.js
```

`cargo test` passed with:

- library unit tests: 44 passed
- `bond_tool`: 141 passed
- `render_document`: 98 passed, 2 ignored
- `text_tool`: 36 passed
- doc-tests: 0

`npm test` passed and runs:

```text
cargo test && node --check viewer/app.js
```

`npm run build:engine-wasm` passed. After the Windows-native rebuild, `viewer/engine/chemcore_engine_bg.wasm` changed. Rebuilding twice produced the same SHA-256:

```text
BC3A87EF5A2310D622F6E58D44DB351C002348A2205D527FBDDF8A52036EA62C
```

So the difference is stable, not random. `npm run verify` passed its tests, WASM build, and JS syntax checks, then failed only at the generated-artifact sync check because `viewer/engine/chemcore_engine_bg.wasm` differs from the repository's previous generated artifact. That WASM diff should be reviewed as a Windows-native rebuild artifact before committing.

### Local Server And Viewer State

A local static file server was started from `D:\chemcore`:

```text
URL:      http://127.0.0.1:8766/viewer/
Process: 8632
Runtime: C:\Python314\python.exe
```

Port `8766` was used because `8765` already had a listener. `Invoke-WebRequest http://127.0.0.1:8766/viewer/` returned `200 OK`.

### Save Fix

The user reported that the viewer could not save. The investigated path was:

- The top save button in `viewer/index.html` uses `data-command="save"`.
- `viewer/editor_bindings.js` maps that command to `saveCurrentDocumentAs`.
- `viewer/document_flow.js` used `CHEMCORE_TEXT_EXTENSION` in the `.ccjs` accept list without importing it.

Direct failure:

```text
ReferenceError: CHEMCORE_TEXT_EXTENSION is not defined
```

Fix:

- Imported `CHEMCORE_TEXT_EXTENSION` in `viewer/document_flow.js`.
- Reordered `saveCurrentDocumentNative`, `saveCurrentDocumentCdxml`, and `saveCurrentDocumentSvg` so browsers with `showSaveFilePicker` open the save picker first, then generate/export/compress content.
- Updated the `app.js` query in `viewer/index.html` to `20260505-savefix` to reduce stale-browser-cache risk.

Verification:

```text
node --check viewer/document_flow.js
node --check viewer/app.js
```

Playwright was also used with a fake injected `showSaveFilePicker`; clicking the Save button successfully reached the write path:

```json
{
  "suggestedName": "Untitled.ccjz",
  "write": {
    "constructorName": "Uint8Array",
    "byteLength": 524
  },
  "closed": true
}
```

### Worktree Changes Today

Before this log was written, today's main modified files were:

```text
.gitignore
package-lock.json
viewer/document_flow.js
viewer/engine/chemcore_engine_bg.wasm
viewer/index.html
docs/developer-log-2026-05-05.md
```

`HANDOFF-2026-05-05.md` has been absorbed into this log and deleted from the repository root.

Meaning of each change:

- `.gitignore`: ignore private-use colon metadata files created by Windows/WSL copying.
- `package-lock.json`: npm 11 root package metadata.
- `viewer/document_flow.js`: fix missing save constant import and open the save picker before content generation.
- `viewer/index.html`: update `app.js` cache-busting query.
- `viewer/engine/chemcore_engine_bg.wasm`: stable Windows-native WASM rebuild artifact.
- `docs/developer-log-2026-05-05.md`: this bilingual developer log.

### Follow-Up Notes

Windows development should happen directly in `D:\chemcore`. The old WSL copy can remain as a reference backup, but dual-track uncommitted development should be avoided. Commit or explicitly back up before switching environments.

If Windows shows massive line-ending diffs, stop and confirm `core.autocrlf=false` and `core.eol=lf` before continuing. If massive file-mode diffs appear, confirm local repo `core.filemode=false`.

If `npm run verify` fails only because `viewer/engine` generated artifacts differ, first confirm that Rust tests, WASM build, and JS syntax checks passed, then decide whether to commit the generated artifact.

`.ccjz` depends on browser `CompressionStream` / `DecompressionStream`. Modern Chromium supports them; future Tauri/WebView2 packaging should verify support or provide native compression in the Tauri backend.

CDXML/SVG fidelity should continue using fixtures under `tmp/` and `compare/` for import, save, re-import, and SVG comparison. Important checks include object counts, object types, colors, text runs, structural labels, arrow parameters, relative positions, line widths, font sizes, and SVG primitives.

For a future desktop shell, Tauri remains the recommended path. Phase 1 should only provide shell capabilities: window, menu, shortcuts, open/save dialogs, file associations, recent files, drag-and-drop open, and installer packaging. Chemistry editing logic should stay out of the shell. Phase 2 can consider direct Rust engine calls from the Tauri backend, native compression, thumbnails, and OS integration.
