# ChemSema Agent Notes

These repository-level instructions are for coding agents working in this tree.
They summarize rules that are easy to miss when moving quickly. More detailed
contracts live in `docs/`.

## Release Notes

- Keep public release notes theme-based, not commit-by-commit. Inspect the
  commits since the previous release tag, but merge them into user-visible
  themes such as platform support, editor behavior, CLI/agent workflows,
  import/export fidelity, Office integration, documentation, and regression
  coverage.
- Preserve the existing changelog style: one short release summary under the
  version heading, followed by concise bullets grouped by topic. Avoid listing
  every commit, script, or test as its own bullet unless it is itself a public
  feature.
- Maintain both changelog files. Write English release notes in
  `CHANGELOG.md` and Chinese release notes in `CHANGELOG.zh-CN.md`; do not mix
  both languages into one file.
- Mention validation and rebuilt artifacts only when they matter to users or
  release consumers, such as shared WASM artifacts, installers, or platform
  packages.
- Before cutting or replacing a public version, regenerate and review the README
  editor screenshot and published CDXML comparison assets with the current
  engine. Commit the refreshed `docs/assets/readme/product-screenshot.png`,
  `docs/assets/readme/comparison/figure*.chemsema.svg`,
  `docs/assets/readme/comparison/figure*.chemsema.emf`, and
  `docs/assets/readme/comparison/published-cdxml-comparison.svg`.

## Engine And Viewer Boundary

- Rust `crates/chemsema-engine` is the authority for editing behavior,
  document mutation, hit testing, snapping, selection, deletion, command
  history, and render primitives.
- The viewer should collect browser events, handle toolbar/menu/file UI,
  convert coordinates, and render SVG/DOM output. Do not reimplement chemical
  geometry or editing rules in `viewer/app.js`.
- If the viewer needs geometry, target applicability, mixed values, or command
  behavior, add or use an engine API instead of duplicating logic in frontend
  code.
- WASM is the browser/desktop runtime form of the same Rust engine. After
  changing engine APIs, render primitives, or `crates/chemsema-engine/src/wasm.rs`,
  rebuild and commit synchronized `viewer/engine` artifacts when the change is
  meant to ship.

## Desktop, Harmony, And Office Boundaries

- Desktop and Office integration layers must not duplicate chemical logic. They
  should call the Rust engine and unified document service.
- Tauri/native services own system capabilities: files, clipboard, export,
  Office/OLE, windows, menus, recent files, and background tasks.
- High-frequency pointer behavior should stay in-process through the shared
  engine or hybrid host. Do not design pointer move, hover, focus, drag preview,
  hit testing, selection, rotation, scaling, or object settings as synchronous
  cross-process calls returning full document/render/state snapshots.
- Office/OLE remains bounded by the independent local server. COM/OLE storage,
  preview, clipboard objects, and desktop wake-up belong there; chemical
  parsing, mutation, import/export, and rendering semantics remain engine or
  desktop-service responsibilities.

## Document And Command Rules

- Native document coordinates are printing points (`pt`). CSS pixels belong at
  the viewer boundary only and must be explicitly converted before entering the
  engine.
- A document commit is a real content change that enters undo/redo history and
  advances the engine revision once. Hover, focus, lasso, selection visuals,
  menus, zoom, pan, tool switching, caret movement, and drag previews are
  transient interaction state.
- New editing features should use semantic `EditorCommand` names in kebab-case.
  Avoid fallback commands such as `mutation`, `pointer-up`, `toolbar-click`, or
  `legacy-mutation`.
- Runtime history is not persisted into `.ccjs`, `.ccjz`, `.cdxml`, EMF, or
  Office/OLE storage.

## Rendering And Label Rules

- Bond rendering behavior follows `docs/bond-rendering-rules.md`. Bond contact,
  label clipping, dash/hash knockout, preview geometry, and committed geometry
  should be defined by Rust render paths.
- Label recognition, implicit hydrogens, abbreviation expansion, valence-label
  parsing, charges, radicals, lone-pair semantics, glyph metrics, glyph
  polygons, and bond-drawing anchors are engine behavior, not viewer behavior.
- CDXML import is an input adapter. It may translate CDXML positions, boxes,
  runs, alignment, and script information into ChemSema's native model, but it
  must not become a second label-layout engine.
- CDXML root drawing defaults such as bond widths, bond spacing, `MarginWidth`,
  `ChainAngle`, label/caption font defaults, justification defaults, display
  flags, and print margins are document style parameters. Preserve known
  values on import and write current defaults on export.
- Attached molecule-label `BoundingBox`, `p`, and similar CDXML cached geometry
  are not active layout authority. Label anchors and bond retreat must be
  recomputed by the Rust engine from native label runs, glyph polygons, and the
  current `MarginWidth` profile.
- Do not reintroduce a per-bond `label_clip_margin`/`labelClipMargin` retreat
  path for atom labels. Bond-to-label retreat is glyph clipping: the rendered
  bond body must be clipped against glyph polygons generated from the current
  `MarginWidth` profile.
- `MarginWidth` natural glyph extension applies to every visible glyph edge,
  including internal bays and stroke ends that sit inside the glyph's overall
  bounding box. Do not approximate it by scaling only geometry outside the
  whole glyph bbox.
- Source-neutral measured label geometry from screenshots, pasted images, or
  other non-CDXML inputs must not be encoded as `meta.import.cdxml`.

## Interaction Feedback

- Endpoint hover visuals use the small visual handle radius; endpoint hit
  testing remains independent and larger.
- Non-bond object creation tools must not show chemical endpoint hover circles
  unless the command directly targets atom endpoints or attached labels. Symbol,
  text, and delete interactions are explicit exceptions because they edit those
  chemical targets directly.
- Completed, canceled, or abandoned pointer interactions must clear every
  transient layer they touched: engine interaction render list, editor overlay,
  canvas drag preview, document preview transforms, and masks.
- Tests for large-document or object-creation behavior should assert feedback
  cleanup without requiring full document render refreshes.

## Verification Habits

- Before committing, inspect the diff and run the narrowest meaningful checks
  for the touched behavior.
- For viewer-visible engine changes, run Rust tests for the behavior, rebuild
  WASM with `npm run build:engine-wasm`, and run a relevant browser/GUI
  regression.
- `npm run verify` is the broad pre-release gate. It runs Rust tests, rebuilds
  engine WASM, checks viewer syntax, and verifies generated engine artifacts.
