# Chemcore Developer Log - 2026-05-04

Author: Jiajun Zhang

Time range: 2026-05-04 00:00 to 2026-05-04 23:59, Asia/Shanghai

Compared with commit: `7dba596 feat: add bracket symbols and valence labels`

Included commits:

- `95867fb Remove obsolete Python and C++ code`
- `5376949 feat: improve CDXML rendering fidelity`
- `0255c7c refactor: move render bounds into engine`
- `4fa0f23 refactor: split viewer editor modules`
- `66cd959 refactor: split chemcore engine modules`
- `3d365bf refactor: consolidate thin engine modules`

## Summary

Today was not only a module-splitting day. The work had four main threads: first, obsolete Python and C++ paths that had been superseded by the Rust engine were removed; second, ChemDraw comparison fixtures drove another pass on CDXML, ACS templates, arrows, wedge bonds, label clipping, text scripts, and imported-object focus; third, geometry that can be computed by the engine, especially render bounds, moved out of the frontend; and only then did the viewer and Rust engine receive the larger module split and the follow-up consolidation pass.

The direction stayed consistent: ChemDraw/CDXML semantics and geometry parameters should live in the engine and JSON data model whenever possible. The viewer should handle interaction, file flow, and presentation. If a user draws an object and later scales it through a selection box or switches drawing templates, the final rendering should not depend on temporary frontend inference.

## Legacy Implementation Cleanup

The first step removed the old Python CDXML conversion layer and the old C++ glyph kernel project, both of which are no longer the maintained primary path.

The cleanup included:

- Removing the old Python CDXML parsing, layout, and conversion implementation under `src/chemcore`.
- Removing the old C++ glyph kernel, C API, demos, and tests under `cpp/chemcore_glyph_kernel`.
- Removing scripts tied to those old paths, including glyph previews, structure comparison, arrow measurement, and glyph wasm build scripts.
- Updating README and architecture documentation to describe the current Rust engine plus Wasm viewer structure.
- Keeping the Rust glyph/render/CDXML path as the single maintained implementation.

This reduced historical maintenance burden and made the boundary clearer: CDXML import/export, text layout, rendering geometry, and editing semantics are owned by `chemcore-engine`.

## CDXML and ChemDraw Rendering Fidelity

The second thread continued aligning Chemcore output with ChemDraw's Default and ACS Document 1996 templates. The `compare/` directory gained and updated ChemDraw CDXML/SVG files plus Chemcore JSON/SVG outputs so Default and ACS output can be compared directly.

One important rule was made explicit: Default and ACS are not one template scaled by bond length. They are separate drawing templates with their own fixed parameters. When switching templates, existing bonds and future bonds must switch to the corresponding template parameters instead of deriving them from the current bond length.

The key parameters documented and stored include:

- Normal stroke width, bold width, hash spacing, and bond spacing.
- Solid and hashed wedge wide-end width.
- Label clipping/retreat distance, with different values for Default and ACS.
- Bond-level numeric fields for bond order, wedge bonds, hash bonds, and double-bond spacing, so JSON stores the actual drawing parameters instead of applying a template only at render time.

This round also fixed several concrete rendering issues:

- Solid and hashed wedge wide ends now use template widths and are not affected by bond length.
- Legacy JSON or imported documents without wedge width receive the correct default from the active document template.
- Switching to the ACS template reflows endpoint label geometry instead of only scaling the old label bbox.
- Default and ACS use different label clip margins, bringing bond-to-label retreat closer to ChemDraw.
- Equal center double bonds keep normal line width through contact and join geometry, avoiding local thickness changes at intersections.
- Small brackets, shapes, and text boxes no longer hit unreasonable fixed minimums during import/rendering.
- Imported shape objects can be focused normally by the Select tool.

## Arrows, Shapes, and Format Fields

Arrow handling was rechecked against ChemDraw's size fields instead of only approximating the three visual size levels.

The added semantics include:

- `length` maps to CDXML `HeadSize / 100`.
- `centerLength` maps to `ArrowheadCenterSize / 100`.
- `width` maps to `ArrowheadWidth / 100`, with different meanings for solid, hollow, and open arrows.
- `curve` maps to `AngularSize`, where positive and negative values represent opposite bend directions.
- `noGo` maps to ChemDraw cross/hash no-go marks.
- Hollow and open arrows use their own three-size templates instead of reusing the solid arrow template.

CDXML import, JSON storage, SVG rendering, and CDXML export now preserve those numeric fields. This matters when users draw an arrow and later scale it through a selection box, or when an existing arrow is imported from CDXML: the geometry should not be overwritten by a fixed frontend size bucket.

Shape fields were also completed:

- Rectangles and rounded rectangles use local `bbox`.
- Rounded corners use `cornerRadius`.
- Circles and ellipses store `center`, `majorAxisEnd`, and `minorAxisEnd`.
- `shaded`, `shadow`, and `shadowSize` from ChemDraw graphics are stored in JSON.

These changes were written into both English and Chinese format documentation.

## Text Faces, Scripts, and Label Layout

CDXML text face bit combinations were expanded. The key rule is that import should respect source CDXML formatting for bold, italic, subscript, superscript, and combinations of those flags instead of falling through to normal text.

Specific improvements:

- Face values that combine multiple bits, such as ChemDraw's compact `96` and `97` combinations, are decoded.
- CDXML text run import preserves both source runs and display runs, so scripts, bold, and italic style continue into rendering.
- Formula-like node labels expand numeric digits into subscript display where the CDXML/source run information implies formula formatting, so labels such as `CF3`, `PF6`, and `CH3` keep their script semantics.
- Non-chemical text still follows source text formatting and is not forced through chemical recognition.

The principle is that Chemcore can own anchors, clipping, and label geometry, but source text formatting such as scripts and font faces should be preserved as faithfully as possible.

## SVG Output and Comparison Tools

To make backend comparison more reliable, a Rust-side SVG output path was added.

- Added `render_svg.rs`, which serializes engine render primitives to SVG.
- Added `crates/chemcore-engine/examples/cdxml_to_svg.rs`, which imports CDXML in the backend and writes SVG for comparison with ChemDraw SVG.
- CDXML shape, arrow, text, and molecule tests now rely more on backend-generated output rather than only visual inspection in the viewer.

This gives future issues such as incorrect arrow head size, unequal center double-bond width, or mismatched wedge outlines a direct backend comparison path through primitives or SVG.

## Render Bounds Moved Into the Engine

The frontend previously walked render primitives to estimate document bounds, selection bounds, and object bounds. That logic depends on primitive variants and render-role filtering, so keeping it in the viewer duplicated renderer knowledge.

Render bounds now live in the engine:

- Added `RenderBoundsScope` for `all`, `document`, and `selection`.
- `Engine::render_bounds()` renders primitives and filters them by scope.
- The `document` scope excludes knockout, hover, selection, and preview roles.
- Wasm exposes `renderBoundsJson(scope)`.
- The viewer consumes engine bounds through `engine_bridge.js`.

Fit-to-document behavior, selection extents, export view calculations, and object focus no longer need a second set of primitive bounds rules in frontend code.

## Viewer Module Organization

`viewer/app.js` had accumulated too many responsibilities. The stable pieces were split out:

- Engine JSON/render/bounds bridging.
- File open/save, CDXML detection, and download helpers.
- Viewer geometry helpers such as bounds, viewBox, and point distance.
- Direct SVG DOM rendering for render primitives.
- JSON/CDXML document loading, sample loading, document title, and metadata refresh.
- Toolbar, input controls, import/export buttons, and text editor bindings.
- Primary and secondary toolbar SVG button rendering.

After the split, `app.js` still owns application state, the core pointer/text-editing flow, and render orchestration. One missing helper reference surfaced during the split and was fixed by making the import explicit instead of relying on an implicit global name.

## Rust Engine Module Organization and Consolidation

Large Rust hub files were also modularized. The split covered the abbreviation, CDXML, editing, engine, select, text edit, render, and render object areas.

After the first mechanical split, a consolidation pass moved modules that were too thin back into their real context. For example, render bounds moved back into `engine.rs`, local arrow object geometry moved into the arrow object renderer, and free-text wrapping helpers moved into the text object renderer.

Small modules were kept only when they have clear ownership, such as CDXML XML tree handling, CDXML text run conversion, bond geometry, bond metrics, label refresh, and selection arrangement. Future cleanup should not split by line count; it should use whether files are changed together, whether a stable domain concept exists, and whether the split reduces cross-file jumping as the criteria.

## Tests and Verification

New or strengthened coverage includes:

- Default/ACS template parameters, wedge width, label clip margin, and ACS label geometry reflow.
- CDXML arrow geometry modifier import and export.
- Hollow/open arrow independent size templates and thin stroke rendering.
- CDXML shape style import/export.
- CDXML face bit combinations, formula-like label subscripts, and subscripts in example files.
- Small text bboxes, small bracket geometry, and Select-tool focus for shape objects.
- Center double-bond joins preserving normal line width.
- SVG export using engine render primitives.

Verification commands run today:

- `cargo fmt`
- `cargo test -p chemcore-engine`
- `npm run build:engine-wasm`
- `node --check viewer/app.js`
- `node --check viewer/*.js`
- `git diff --check`

Final `chemcore-engine` test results stayed green:

- unit tests: 39 passed
- `tests/bond_tool.rs`: 141 passed
- `tests/render_document.rs`: 81 passed, 2 ignored
- `tests/text_tool.rs`: 32 passed
- doctests: 0

## Code Volume

Statistics scope: `7dba596..3d365bf`, from the previous full developer-log commit through the end of today's committed work. The count uses Git-tracked text files; binary files are counted as files but not as lines. This section does not include the new developer log files added in this follow-up.

- Commits: 6
- Changed files: 120
- Git diff: `+25,477 / -25,497`
- File status: 50 added, 44 modified, 26 deleted
- Total tracked project files: 132 -> 156, net +24
- Text files: 127 -> 151, net +24
- Binary files: 5 -> 5
- Total project text lines: 69,494 -> 69,474, net -20

The near-zero net line change comes from two opposing changes: a large amount of obsolete Python/C++ implementation was removed, while CDXML/rendering fidelity work, comparison fixtures, tests, SVG output, and the Rust/viewer module structure were added.
