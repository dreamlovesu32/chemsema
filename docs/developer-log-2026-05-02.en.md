# Chemcore Developer Log - 2026-05-02

Author: Jiajun Zhang

Time range: 2026-05-02 00:00 to 2026-05-02 23:59, Asia/Shanghai

Compared with commit: `0836b15 feat: add shape tools and engine workflow rules`

## Summary

This round moved CDXML from an external conversion/frontend compatibility path into first-class Rust engine input and output. Import now parses CDXML directly inside `chemcore-engine` and produces native molecule fragments, text objects, lines, arrows, and shapes. Export adds an engine-side CDXML writer that serializes the current `ChemcoreDocument` into a ChemDraw-readable CDXML document.

The second major thread was ChemDraw drawing-style convergence. Bond length, line width, bold width, hash spacing, double-bond spacing, and the ACS Document 1996 preset are now part of engine options and render formulas. After importing CDXML, newly drawn bonds inherit the source file’s format. If the source matches ACS, the viewer preset control moves to ACS and can return to Default.

The label system also became more chemical: implicit hydrogens, element labels, terminal and bridge abbreviations, composite abbreviations, `N3`, `CF3`, and `t-Bu/tBu` whole-label behavior now live in the Rust engine. The viewer consumes engine state and render primitives instead of defining chemical behavior on its own.

## Core Boundary

This work continues to enforce the project-level engine boundary:

- Added `crates/chemcore-engine/src/cdxml.rs`; CDXML parsing and exporting now live in Rust.
- Added `crates/chemcore-engine/src/abbreviation.rs`; abbreviation recognition, aliases, composite parsing, and expansion metadata now live in Rust.
- Added `quick-xml` to `chemcore-engine` for CDXML XML parsing.
- `lib.rs` exports the `cdxml` and `abbreviation` modules for tests, wasm, and engine use.
- The viewer only handles file open/save, toolbar state synchronization, and SVG/DOM presentation.

This keeps CDXML handling, label recognition, and follow-on drawing behavior out of the frontend conversion layer.

## CDXML Import

The new `parse_cdxml_document()` path covers common ChemDraw document structure:

- Reads CDXML root defaults: `BondLength`, `LineWidth`, `BoldWidth`, `HashSpacing`, and `BondSpacing`.
- Parses color tables and font tables, including ChemDraw legacy palette ids.
- Converts display fragments into `molecule_fragment2d` resources.
- Converts `n` elements into engine `Node` values with element, placeholder/nickname state, charge, hydrogen count, and source metadata.
- Converts `b` elements into engine `Bond` values with order, double placement, stereo, line styles, line weights, bond spacing, hash spacing, and bold width.
- Converts CDXML `arrow` and `graphic Line` nodes into `line` objects.
- Converts rectangle/oval graphics into `shape` objects with fill, stroke, dash, shadow, and shaded style data.
- Converts free text boxes into `text` objects with text, bbox, alignment, font size, runs, and colors.

Structure labels are no longer rendered by copying CDXML text boxes directly. Imported labels are routed through the internal attached-label layout engine so `NH`, `O`, `CF3`, `t-Bu`, and related node labels use the same clipping and anchoring path as native labels.

## Editing Normalization for CDXML

Raw `parse_cdxml_document()` preserves the source display-fragment object split for tests and future round-trip analysis. `Engine::load_cdxml_document()` performs an additional editing normalization step:

- Multiple CDXML molecule fragments are merged into one editable fragment.
- The merge rewrites each fragment’s nodes, bonds, label boxes, and glyph polygons into one local coordinate frame.
- Parser behavior remains unchanged; only documents loaded into the editor are merged.

This fixes imported bonds that could not be focused. The old hit-test and editing paths only used `document.editable_fragment()`, which returned the first molecule object. In multi-fragment CDXML files, later bonds were invisible to focus/hit-test logic. After merging, all imported molecular bonds are part of the editable graph.

## CDXML Export

Added `document_to_cdxml()` and `Engine::document_cdxml()`, exposed to wasm as `documentCdxml()`. The exporter intentionally does not reproduce every redundant or dirty ChemDraw field. It writes a clean core CDXML document from `ChemcoreDocument`:

- Standard `CDXML` root, DOCTYPE, page, color table, and font table.
- Molecule objects as `<fragment>`, nodes as `<n>`, and bonds as `<b>`.
- Plain carbon nodes stay compact; element nodes write `Element`.
- Placeholder/abbreviation labels write `NodeType="Nickname"` plus `<t><s>...</s></t>` label content.
- Double bonds include `Order`, `DoublePosition`, `BondSpacing`, `LineWidth`, `BoldWidth`, and `HashSpacing`.
- Wedges, hashed wedges, dashed lines, and bold double lines map back to CDXML display attributes.
- Free text writes as `<t>`; lines/arrows write as `graphic` or `arrow`; rectangles/ovals write as `graphic`.
- Colors are collected from document styles and label runs into the color table. Fallback runs inherit the label/text color.

The viewer now has a “Save CDXML” command using the browser save picker, and the open path accepts `.cdxml` plus common CDXML MIME types.

## ChemDraw / ACS Drawing Format

The Default and ACS Document 1996 drawing formats were recalibrated:

- ACS preset: bond length `14.4`, line width `0.6`, bold width `2.0`, hash spacing `2.5`, and graphic stroke width `0.6`.
- Newly drawn bonds, template bonds, downgraded replacement bonds, paste/template-generated bonds, and graphics inherit the current `EditorOptions`.
- CDXML import restores drawing options from root defaults and, where needed, inferred bond metrics.
- If an imported document matches ACS, `Engine::document_style_preset()` returns `acs-document-1996`.
- The viewer synchronizes the preset dropdown from the engine after loading JSON or CDXML so stale UI state cannot overwrite imported formatting.
- Switching to ACS can be reversed by switching back to Default; existing document geometry scales by the bond-length ratio.

Imported ACS documents therefore continue drawing in ACS style instead of silently falling back to the default style.

## Double Bonds and Bond Rendering

Double-bond spacing no longer uses a fixed visual ratio. It now follows ChemDraw `BondSpacing` and actual bond length:

```text
inner_gap = max(bond_length * BondSpacing / 100 - line_width, line_width * 0.5)
center_distance = inner_gap + (width_a + width_b) / 2
```

`width_a` and `width_b` depend on normal width, bold width, and double-line weights. Hashed wedge spacing also reads bond-level `HashSpacing`. Triple bonds and side double bonds continue to scale with actual bond length, so stretched terminal bonds no longer keep a static measured gap.

The render path now supports:

- bond-level `bold_width`, `hash_spacing`, and `bond_spacing`;
- bold bond contact and join logic using bond-level bold width;
- dash/hash knockout based on current line width and spacing;
- regression coverage for imported dashed double, bold double, side double, and ACS fixtures.

## Labels, Implicit Hydrogens, and Abbreviation Recognition

Endpoint labels are no longer plain text only:

- Simple element labels enter element recognition and refresh implicit hydrogen count from connectivity.
- Rules for `N`, `O`, `P`, `S`, halogens, `B`, `Si`, and related atoms are documented in `docs/implicit-hydrogen-rules.zh-CN.md`.
- Terminal abbreviations include `Me`, `Et`, `Pr`, `iPr`, `Bu`, `iBu`, `sBu`, `tBu`, `Ph`, `Bn`, `Ac`, `Boc`, `Cbz`, `Fmoc`, `TMS`, and others.
- Composite abbreviations include labels such as `CO2Et`, `COOEt`, `OAc`, and `SO2Me`.
- Bridge labels include `NH`, `CO`, `CO2/COO`, `OCO`, `SO/SO2`, `CH2`, and selected `NMe/NTs` forms.
- `N3` is recognized as azido.
- `CF3` uses normal abbreviation recognition; when connected on the right it displays as `F3C` while anchoring on `C`.
- `t-Bu` and `tBu` are aliases for the same legal label; related aliases such as `nBu` and `iPr` are handled by the same legal-label system.
- Recognized whole-label abbreviations and unknown invalid labels both behave as whole labels when connected on the left, anchoring at the rightmost glyph group.

Recognition results are stored in `meta.labelRecognition`, and the format documentation now describes the `functionalGroupExpansion.v1` semantic layer. This expansion is extra semantic data, not a replacement for the main molecule graph.

## Text Editing and Label Layout

Endpoint label editing was further centralized in the engine:

- Text edit sessions can target normal text objects or endpoint labels.
- Preview/apply flows use the Rust label kernel for source runs, display runs, bbox, glyph polygons, and caret geometry.
- Endpoint label hover prefers the whole label box instead of a plain endpoint circle.
- During editing, the current label’s document text, knockout, and hover primitives are hidden to avoid overlap with the DOM editor.
- Reopening endpoint labels preserves stable anchor, bbox, and source text.
- Auto-generated implicit hydrogen characters are editable text, but they cannot become bond anchors; dragging from generated hydrogen routes back to the heavy atom.

The viewer `text_editor_controller` only handles DOM editor interaction and positioning. Geometry remains defined by the engine layout result.

## Selection, Hit Testing, and Interaction

Selection and focus behavior were updated to support native CDXML labels:

- `RenderPrimitive` now carries `node_id` so hover/text primitives can be associated with endpoint labels.
- The text tool can hover existing labels and open endpoint label editing.
- Select/delete/template paths refresh label geometry after structural changes.
- Multi-fragment CDXML is merged during engine load so hit testing covers all imported bonds.
- Bond-center hover and style cycling continue to reuse existing engine hit-test logic, now backed by the merged editable fragment.

## Documentation and Format

Documentation updates:

- README and Chinese README link to the implicit hydrogen and abbreviation recognition rules.
- `docs/project-rules.zh-CN.md` states that chemical label behavior belongs in the Rust engine.
- `docs/format-v0.1.md` and the Chinese version now describe `meta.labelRecognition`, `functionalGroupExpansion.v1`, and the rule that source-format bit masks do not become core fields.
- Added `docs/implicit-hydrogen-rules.zh-CN.md`.
- Added `docs/abbreviation-recognition-rules.zh-CN.md`.

## Viewer and Wasm

Viewer changes:

- Open supports JSON and CDXML.
- Save supports existing JSON plus new CDXML export.
- Toolbar includes a document style preset dropdown: Default / ACS Document 1996.
- After load, the viewer reads `documentStylePreset()` from the engine to keep UI and engine state aligned.
- Rendering supports primitives with `nodeId`, used to hide the endpoint label currently being edited.
- Wasm bindings now include `loadDocumentCdxml()`, `documentCdxml()`, `documentStylePreset()`, and the document style setter.
- Generated `viewer/engine` JS, TypeScript declarations, and wasm binary were rebuilt.

## Tests and Validation

Test coverage expanded around:

- CDXML assets/native molecule import.
- CDXML arrow, shape, free text, and table line/text import.
- ChemDraw legacy color palette.
- CDXML node labels routed through internal attached-label layout.
- Default and ACS double-bond spacing fixtures.
- Double-bond spacing scaling with stretched bond length.
- CDXML exporter round-trip.
- Multi-fragment CDXML becoming editable and hit-testable after engine load.
- CDXML load preserving ACS drawing options.
- ACS preset behavior for new bonds, bold bonds, graphic strokes, and return to Default.
- Abbreviation recognition, `CF3`, `t-Bu/tBu`, and invalid whole-label anchoring.
- Implicit hydrogens, generated hydrogen anchoring, endpoint label reopening, and preview geometry.

Validation run before this commit:

- `cargo test -p chemcore-engine`
- `./scripts/build-engine-wasm.sh`

