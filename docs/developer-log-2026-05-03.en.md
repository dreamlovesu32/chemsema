# Chemcore Developer Log - 2026-05-03

Author: Jiajun Zhang

Time range: 2026-05-03 00:00 to 2026-05-03 23:59, Asia/Shanghai

Compared with commit: `7f406c9 docs: expand label recognition developer log`

## Summary

This round moved brackets, repeating units, charge/radical symbols, and formula-like label recognition into the Rust engine. The viewer gained bracket and symbol tools, Wasm exposes the matching tool options and double-click component selection, CDXML import/export now supports ChemDraw bracket and symbol graphics, and the renderer emits native primitives for parentheses, square brackets, braces, and eight charge/electron symbol kinds.

Chemically, symbols are no longer just standalone decorations. Positive and negative charge symbols, radical cations/anions, single-electron symbols, and lone-pair symbols can attach to endpoints or heavy atoms inside attached labels. That attachment refreshes node charge, radical count, effective implicit hydrogens, invalid state, and repeating unit expansion. Label recognition also moved beyond fixed abbreviation combinations with a valence-driven parser for formula-like terminal labels such as `CN`, `CO2Cl`, and `CH2COOCH2SO2NHCl`.

## Bracket and Symbol Tools

The editing engine added `Tool::Bracket` and `Tool::Symbol`, and `ToolState` now stores `bracket_kind` and `symbol_kind`.

- The Bracket tool supports dragging round, square, and curly bracket objects.
- The Symbol tool supports `circle-plus`, `plus`, `radical-cation`, `lone-pair`, `circle-minus`, `minus`, `radical-anion`, and `electron`.
- Clicking near an endpoint, attached label, or regular text chooses a reasonable insertion position automatically.
- Dragging a symbol from an endpoint or label glyph places the symbol on an endpoint/label orbit.
- The Bracket tool keeps bond-center hover while avoiding endpoint focus; the Symbol tool focuses endpoints and labels but not bond centers.
- The Select tool can click, box-select, move, arrange, and delete bracket/symbol objects.
- Double-clicking a molecular component selects the whole connected component and includes enclosing brackets in that selection.

The viewer added primary tool buttons and secondary toolbars. The bracket button exposes three bracket kinds, and the symbol button exposes eight electron/charge symbols. Tool state is synchronized into the Wasm engine through `setBracketOptions()` and `setSymbolOptions()`. After a bracket is created, the viewer opens the text editor at the lower right so the repeat count can be entered directly.

## Rendering and CDXML

The renderer added bracket/symbol object paths:

- Round brackets use approximated elliptical arcs, square brackets use lip-proportional strokes, and braces use cubic paths.
- Dagger and double dagger symbols use filled paths.
- Circled plus/minus symbols use an outer circle path plus internal signs; plain plus/minus symbols use filled rectangle combinations.
- Radical cation, radical anion, lone pair, and electron symbols combine dots and plus/minus signs.
- ACS and Default symbol metrics are handled separately so size and stroke width stay close to ChemDraw output.
- If an attached charge/radical state is invalid, the renderer draws a red circular invalid marker near the node.

The CDXML path was filled in as well:

- Imports `GraphicType="Bracket"` and pairs left/right bracket graphics into one `SceneObject { type: "bracket" }`.
- Imports `GraphicType="Symbol"` and maps ChemDraw `SymbolType` values to engine symbol kinds.
- Infers symbol style and metrics from the CDXML default line width, preserving anchor width/height, line width, and original bbox.
- Exports bracket/symbol objects back to CDXML graphics, including bracket pairs, symbol type, bbox, and z-order.
- CDXML root default export now prefers imported defaults, document styles, actual bonds, or symbol line width before falling back to engine defaults, avoiding parameter drift during round trips.

Added `crates/chemcore-engine/examples/cdxml_render_metrics.rs` and `scripts/compare-cdxml-symbol-pixels.mjs` for measuring CDXML render metrics and comparing ChemDraw SVG symbol pixels.

## Symbol Chemistry Semantics

Added `crates/chemcore-engine/src/symbols.rs` to centralize charge/radical symbol attachment and node semantic refresh.

- When a symbol center is within 10 pt of an endpoint or label anchor, it attaches to the nearest candidate atom.
- Symbol objects write `chemicalRole`, `chargeDelta`, `radicalDelta`, `attachedAtomId`, `attachmentSource`, and `attachmentDistance`.
- Node metadata writes `attachedElectronSymbols`, `radicalCount`, `effectiveNumHydrogens`, and `chargeSymbolInvalid`.
- Plain plus/minus symbols change formal charge; radical cation/anion symbols change both charge and radical count; electron symbols increase radical count; lone-pair symbols currently preserve display semantics only.
- After symbols attach, attached label geometry, implicit hydrogens, and repeating unit metadata are refreshed.
- Deleting symbols, moving/arranging symbols, or deleting connected bonds recomputes attachment and legality.
- Documents without symbols no longer receive unnecessary symbol bookkeeping metadata, preventing unrelated refreshes from accidentally treating normal labels as implicit hydrogen labels.

## Repeating Units

Added `crates/chemcore-engine/src/repeating_units.rs` to recognize repeating units made from a bracket and numeric text at document level.

- Only clearly attachable bracket objects and lower-right numeric counts are recognized.
- Internal atoms and bonds are found through bracket bounds.
- The recognizer requires one crossing bond on each left/right boundary, with matching boundary bond orders.
- On success, bracket/text object metadata receives `repeatUnitId` and `repeatUnitRole`.
- The editable fragment metadata receives `repeatingUnits`, including atom ids, internal bond ids, boundary bonds, repeat count, and expansion.
- Expansion copies internal atoms/bonds while preserving node charge, effective hydrogens, radical count, and attached electron symbols.
- Missing numeric counts or incomplete boundaries do not generate expansion, avoiding fabricated chemistry for incomplete structures.

## Label Recognition and Text Editing

`abbreviation.rs` gained a valence-driven terminal label parser:

- Labels are tokenized into elements and terminal-group fragments, with numeric expansion such as `O2` and `H3`.
- The external connection first consumes one valence unit from the attachment atom.
- Later tokens build parent/child relationships from left to right according to valence and available connection capacity.
- Common multiple-bond patterns such as C-N, C-O/C-S, and S/P/As-O prefer more chemically reasonable bond orders.
- Charged B/N/O valence exceptions are supported, and `formalCharge` is recorded on components.
- Recognition metadata for valence parser results writes `source: "valence-parser"`, and components record `parentIndex`, `bondOrderToParent`, and optional `formalCharge`.
- `COOH`, `COCH3`, and `OCH3` normalize to `CO2H`, `COMe`, and `OMe`.

Text editing also gained a non-chemical endpoint label path:

- `TextEditSession.default_chemical` is inferred from source runs instead of assuming every endpoint label is chemical.
- Non-chemical runs skip abbreviation/element recognition and do not render a red error box.
- Non-chemical right-side labels preserve original text order and use whole-label anchoring.
- The text toolbar chemical button can now toggle between chemical and normal modes.
- Positive-charge implicit hydrogen calculation no longer subtracts `abs(charge)` blindly, so positively charged hetero atoms can gain hydrogens correctly.

## Viewer and Wasm

New Wasm bindings:

- `setBracketOptions(kind)`
- `setSymbolOptions(kind)`
- `selectComponentAtPoint(x, y, additive)`

Viewer updates:

- Added Bracket and Charge/Electron Symbol tools to the primary toolbar.
- Added secondary toolbar controls for bracket kind and symbol kind.
- The main symbol button displays the current symbol kind icon.
- Pointer routing covers bracket and symbol tools.
- After bracket drag completes, the count text editor opens automatically.
- Select tool double-click calls engine component selection.
- Bracket/symbol rendering consumes engine render primitives directly.
- Rebuilt the JS, d.ts, and wasm artifacts under `viewer/engine`.

## Documentation

Added and updated documentation:

- Added `docs/charge-radical-symbol-rules.zh-CN.md`, documenting attachment, valence, implicit hydrogen, invalid state, and expansion behavior for the eight symbol kinds.
- Added `docs/valence-label-recognition-rules.zh-CN.md`, documenting the formula-like label valence parser plan.
- Updated `docs/abbreviation-recognition-rules.zh-CN.md`, pointing the next formula-like parser stage to the dedicated design document.
- Updated `docs/project-rules.zh-CN.md`, adding valence label rules and charge/radical symbol attachment rules to the project baseline.

## Tests and Verification

New test coverage includes:

- Bracket drag creation, hover strategy, and CDXML bracket/symbol import.
- Symbol creation, selection, endpoint/label orbiting, ACS metrics, and Default/ACS symbol render sizes.
- Charge/radical symbol attachment to carbon and hetero atoms, covering charge, hydrogens, and invalid markers.
- Plain charge/single-electron symbols on four-connected carbon are invalid, while radical ions are allowed.
- Bracketed repeating unit recognition, count text matching, expansion, and no expansion when count is missing.
- Double-click component selection includes enclosing brackets.
- Non-chemical endpoint labels skip recognition, avoid red error boxes, and preserve non-chemical state when reopened.
- Valence parser coverage for formula-like labels, charged B/N/O exceptions, and named terminal groups.

Commands run before committing:

- `cargo fmt`
- `cargo test`: passed.
- `npm run build:engine-wasm`: passed and rebuilt `viewer/engine` artifacts.
- `node --check viewer/app.js`: passed.

Note: the test and wasm-build stages of `npm run verify` passed, but the script exits at the end when `viewer/engine` still has uncommitted generated diffs. Those generated outputs were intentionally part of this round, so the final verification used the split commands above.
