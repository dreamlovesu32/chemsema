# Chemcore Developer Log - 2026-04-28

Author: Jiajun Zhang

Time range: 2026-04-28 00:00 to 2026-04-28 23:59, Asia/Shanghai

## Summary

The main thread of today’s work was not another round of viewer-side patching. It was another push toward making cm-based geometry the single source of truth. The biggest changes fall into four groups:

- text-edit geometry, anchors, and the visible editing layer were moved further away from browser-driven logic and closer to the engine;
- the overweight text-edit, delete, and select logic in `engine.rs` started to split into dedicated modules;
- selection and deletion were fully connected as interaction chains, and molecule-internal multi-selection rules were tightened toward expected chemistry-editor behavior;
- bond selection boxes stopped tracking only the main center line and started using the full rendered bond outline, so double, triple, bold, and wedge bonds are all enclosed correctly.

The real gain from this round is that several problems that used to live in the “almost right” zone are now converging on shared engine geometry and shared rendering results instead of browser measurements and frontend compensation.

## Continuing the Move of Text Editing Into the Engine

The previous round had already started moving text editing away from `contenteditable` semantics. Today continued that by tightening the part of editing-state geometry that still had the highest drift risk.

### Separating Visible Editing From Input Transport

The editor still follows the same structure:

- the `textarea` is only used for keyboard and IME input;
- visible text, black caret, blue selection, and the black editor box are all self-rendered;
- the viewer no longer infers text geometry from actual DOM layout results.

The point is simple: the browser may handle input, but it should not define text geometry.

### Text Box Anchor Adjustment

When a text box enters edit mode, the anchor model is now closer to “top-left of the I-beam caret” rather than “somewhere around glyph center plus compensation.”

This looks like a small interaction detail, but it removes a deeper inconsistency between:

- the clicked point,
- the edit-box origin,
- and the final committed text position.

### Historical DOM Measurement Fields Keep Leaving the Boundary

Legacy fields such as `measuredSize` continue to be pushed out of the commit path. The viewer no longer treats DOM-measured dimensions as engine geometry input. Text and label geometry increasingly depend only on engine layout and explicit cm-side boundary values.

## Rust Engine Split

This round also addressed a structural problem: `crates/chemcore-engine/src/engine.rs` had become too large and too mixed.

The main modules split out today were:

- `crates/chemcore-engine/src/engine/text_edit.rs`
- `crates/chemcore-engine/src/engine/delete.rs`
- `crates/chemcore-engine/src/engine/select.rs`

That gives two practical benefits:

- text editing, deletion, and selection no longer fight for space inside one oversized file;
- future interaction fixes are easier to localize and do not require scanning the main `engine.rs` file every time.

## Tightening Selection and Deletion Behavior

Selection and deletion both moved from “working” toward “internally consistent.”

### Delete Mode

Delete mode now works as a dedicated tool instead of only as a top-bar command:

- switching to the delete tool enters deletion cursor mode;
- clicking a bond, endpoint, label, or text box follows the correct chemistry-specific delete semantics for that target;
- pressing `Delete` in ordinary modes still follows the “delete the current focused target” rule.

### Select Mode

Select mode now supports:

- point selection,
- rectangle selection,
- lasso selection,
- and additive multi-selection with `Shift`.

More importantly, molecule-internal display behavior now follows a single rule set:

- if exactly one object inside a molecule is selected, only that object’s own box and center dot are shown;
- if multiple objects inside the same molecule are selected, only the minimal outer bounding box is shown, while the internal objects keep center dots only;
- different molecules compute their own minimal boxes independently.

Labels, atoms, and bonds are all part of this same logic now rather than each using a separate rendering rule.

## Bond Selection Boxes Now Use Real Visual Bounds

The last visible issue today was that bond selection boxes were still computed from the main bond center line plus a fixed width. That made selected double and triple bonds look obviously undersized.

This was fixed by reusing rendering results instead of adding more selection-side special cases:

- `render.rs` now provides `fragment_bond_visual_bounds(...)`;
- it reuses existing bond rendering logic to generate the actual `DocumentBond` primitives for one bond;
- then computes a true axis-aligned bounding box from the rendered line / polygon / polyline output;
- `select.rs` consumes that result instead of guessing the outer shape of double, triple, or bold bonds itself.

After that change:

- double-bond selection boxes cover both lines;
- triple-bond selection boxes cover all three lines;
- bold and wedge-style bonds expand naturally as well;
- and future bond-rendering constant changes will automatically flow into selection geometry without requiring a second offset system to be manually kept in sync.

## Frontend Responsibilities Keep Shrinking

On the viewer side this round was less about adding new logic and more about removing geometry responsibility:

- text-edit control flow in `viewer/app.js` was reduced further;
- stale geometry scaffolding in `viewer/text_editor_render.js` was removed further;
- wasm exports and frontend bindings were updated together so the viewer runs against the new engine interfaces and the regenerated package.

That pushes the viewer closer to its intended role:

- input events,
- world-coordinate conversion,
- and primitive rendering,

instead of maintaining a second hidden text or label geometry system.

## Verification

The main verification steps for today were:

```bash
cargo test -p chemcore-engine --test bond_tool --test text_tool
npm run build:engine-wasm
```

This covered:

- text-editor regression behavior,
- label and attached-label interaction behavior,
- delete and select paths,
- single, double, and triple bond selection-box regressions,
- and consistency between the regenerated wasm package and the viewer bindings.

## Risks and Next Steps

Even after today’s progress, two areas still deserve continued attention:

- text editing no longer depends primarily on DOM geometry, but caret, selection, and IME behavior can still be pushed further toward engine-driven results;
- bond selection boxes now use true rendered bounds, and the same “rendering is truth” approach should be reused later for more complex multi-segment objects, arrows, or shape selection instead of returning to hand-written approximations.

The conclusion from today is straightforward:  
geometry truth kept moving out of browser compensation and back into the engine and renderer, which is necessary for long-term maintainability and for future multi-platform consistency.
