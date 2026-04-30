# Chemcore Developer Log - 2026-04-30

Author: Jiajun Zhang

Time range: 2026-04-30 00:00 to 2026-04-30 23:59, Asia/Shanghai

Compared with commit: `65504fe chore: commit all outstanding changes`

## Summary

Today’s work moved the editor from bond-first structure editing into a broader chemistry drawing surface. The largest deliverable was the new arrow tool, including straight arrows, curved arrows, endpoint editing, hover handles, ChemDraw-style arrowhead variants, no-go cross/hash decorations, and smooth circular-arc rendering.

The other major thread was editor infrastructure: document units are now explicitly points (`pt`), command history has a semantic command layer, clipboard operations are wired into the engine and viewer, and the frontend controls now expose more of the engine’s editing surface through generated wasm bindings.

## Document Units

The file format now records document units explicitly:

- `format.unit = "pt"` was added to the format documentation.
- Engine unit helpers now expose point-based conversions.
- Viewer unit helpers now map CSS pixels to points instead of treating the world unit as abstract centimeters.
- Several hit radii, focus radii, bond defaults, and paste offsets were adjusted to use the new point-based scale consistently.

This keeps stored geometry closer to ChemDraw/CDXML coordinates and removes the previous ambiguity around “abstract document units.”

## Command History

A new command layer was added under the engine:

- `EditorCommand` records semantic operations such as adding bonds, adding arrows, applying arrow styles, cutting, pasting, inserting templates, moving selections, rotating selections, applying text edits, and replacing endpoint labels.
- `HistoryEntry` stores the command plus exact before/after document snapshots.
- Drag commands refresh their final `after` state during the gesture so redo lands at the final pointer-up state.
- A new `docs/editor-command-history.md` document describes the command surface and the difference between committed document changes and transient interaction state.

This gives undo/redo a clearer path toward semantic history while preserving exact document restoration today.

## Clipboard

The engine gained internal clipboard support:

- copy selected nodes, labels, and selected bonds;
- cut as one undoable command;
- paste with fresh node and bond ids;
- offset pasted content by a fixed point-based distance;
- refresh attached labels and selection bounds after paste;
- expose copy/cut/paste through wasm and viewer toolbar/keyboard paths.

The clipboard intentionally stores editor data, not browser clipboard text.

## Arrow Tool

The arrow tool was the largest feature set added today.

### Tool Entry and Drawing

The arrow tool now appears below the text tool in the left toolbar. Drawing behavior is:

- press at any point;
- drag to any point;
- release to create an arrow;
- without `Alt`, direction snaps to 15-degree increments;
- with `Alt`, direction is free;
- the pointer release point is the true arrow endpoint, including the arrowhead length.

New arrows are not auto-selected after creation, so users can continue drawing without an extra deselection step.

### Toolbar Controls

The arrow toolbar now includes:

- solid arrow;
- curved arrow;
- mirrored curved arrow;
- hollow arrow;
- open hollow arrow;
- arrowhead size: large, medium, small, with small as the default;
- endpoint style group: line, full head, full tail, head-left half, head-right half, tail-left half, tail-right half;
- no-go group: cross arrow and double-slash arrow;
- bold arrow.

Controls are separated into groups with dividers: arrow type, curve amount, head size, endpoint style, no-go decoration, and bold.

### Arrow Styles and Payload

Arrow objects store a structured `arrowHead` payload:

- `kind`: `solid`, `curved`, `curved-mirror`, `hollow`, or `open`;
- `curve`: signed curve angle in degrees;
- `head` and `tail`: `none`, `full`, `half-left`, or `half-right`;
- `length`, `centerLength`, and `width`;
- `bold`;
- `noGo`: `none`, `cross`, or `hash`.

The CDXML converter was adjusted so imported arrows only enable head/tail endpoints when the source CDXML endpoint setting is actually present and not false-like.

### Straight and Curved Rendering

Rendering now supports:

- filled solid arrowheads;
- half arrowheads that preserve a sharp tip and keep the shaft from drawing through the tip;
- hollow and open arrow outlines;
- curved solid arrows as SVG paths;
- cross and double-slash no-go marks, scaled from the current arrowhead size;
- bold arrow variants.

`RenderPrimitive::Path` was added so curved shafts can be rendered as paths instead of approximated by visible polylines.

### Curved Arrow Fix

A serious curved-arrow rendering bug was fixed. The circular arc sampled points were correct, but the cubic Bézier control-point coefficient was wrong:

- wrong: `4 / 3 * tan(delta / 2)`;
- correct: `4 / 3 * tan(delta / 4)`.

The incorrect coefficient pushed control points far away from the circle, making 270-degree arrows look like rounded rectangles or connected straight segments. The corrected path now follows a true circular arc. A regression test checks that the first control point stays near the circular tangent.

### Hover and Editing

Arrow hover behavior was added for both the select tool and the arrow tool:

- unselected arrows show hover handles near the endpoints, center, and arrowhead side points;
- no-head arrows show only start, center, and end handles;
- half arrows show only the relevant side handle;
- double-headed arrows show side handles at both ends;
- selected arrows suppress hover and use the normal selection box.

In select mode:

- clicking a hovered arrow selects it;
- dragging a hovered head/tail endpoint edits that endpoint;
- dragging the hovered center edits curve amount;
- endpoint dragging snaps to 15-degree increments unless `Alt` is held;
- curve dragging also snaps to 15-degree increments unless `Alt` is held;
- curve and rotation angle labels now display integer degrees.

The arrow tool can also use the hover endpoint/curve drag affordances without selecting arrows.

## Selection and Object Interaction

Selection behavior was extended to cover arrow objects and richer object manipulation:

- arrow objects can be selected with a selection box;
- selected arrow bounds include path primitives;
- selection movement, temporary object dragging, endpoint movement, and rotation continue to use command-backed history;
- selected objects suppress hover affordances that would conflict with selection-box operations;
- rotation labels round to integer degrees.

The selection overlay gained arrow-specific center halos and circular hover handles.

## Templates and Bond Editing

Template and bond editing continued to move toward chemistry-editor behavior:

- ring templates support several ring sizes and benzene;
- template insertion is command-backed;
- endpoint, bond, blank-canvas, and drag-directed template placement paths are covered by tests;
- connected label geometry is refreshed after structure mutations;
- deletion semantics distinguish selected deletion from focused delete-tool deletion;
- multi-bond and wedge rendering tests cover more junction cases.

## Text Editing

Text editing and label editing were tightened:

- text edit application is command-backed;
- endpoint labels refresh their geometry after bond and structure changes;
- regression scripts account for the point-based unit scale;
- text editor controller behavior was adjusted to keep edit sessions consistent with engine state.

## Viewer and Wasm

The viewer was updated to expose the new engine capabilities:

- arrow tool button and secondary toolbar;
- arrow option synchronization into wasm;
- selection and arrow pointer routing;
- copy, cut, and paste command paths;
- fixed zoom dropdown levels: `12%`, `25%`, `50%`, `75%`, `100%`, `150%`, `200%`, `400%`, `600%`, and `800%`;
- updated generated wasm JS, TypeScript declarations, and wasm binary.

The frontend render path now handles `path` primitives for curved arrows and selection bounds.

## Tests and Validation

The test suite was expanded around:

- arrow defaults;
- arrow style application to selected arrows;
- curved arrow path rendering;
- circular arc control-point regression;
- half-arrow side behavior on curves;
- arrow hover handles and editing gestures;
- command-backed move/rotate/edit operations;
- clipboard cut/copy/paste;
- template insertion;
- point-based units;
- text edit geometry.

Validation run during development:

- `cargo test -p chemcore-engine`
- `npm run build:engine-wasm`
- `node --check viewer/app.js`
- `git diff --check`

