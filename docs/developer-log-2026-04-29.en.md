# Chemcore Developer Log - 2026-04-29

Author: Jiajun Zhang

Time range: 2026-04-29 00:00 to 2026-04-29 23:59, Asia/Shanghai

## Summary

Today’s main thread was moving the editor beyond basic bond drawing toward object manipulation and structure templates that behave more like a chemistry drawing application. Work landed in six areas:

- selection-mode drag semantics, including dragging inside the selection bounds, temporary dragging of unselected objects, 15-degree snapping for terminal endpoints, and selection preservation after drag;
- alignment, distribution, and mirroring commands in the selection toolbar, with the rule that content inside the same molecule is treated as one object;
- ring templates from three-membered through eight-membered rings, plus benzene, with endpoint clicks, bond clicks, blank-canvas clicks, and drag-directed placement;
- multi-bond junction rendering reworked around one geometric rule, especially for three-way, four-way, and wedge-plus-single-bond junctions;
- frontend open/save for the current `.chemcore.json` document format;
- canvas interaction improvements, including `Ctrl/Command + wheel` canvas zoom, an 800% maximum zoom, and a rotation handle above the selection box.

The most important structural change was reconnecting the debug diagrams with the actual frontend rendering path. Multi-bond junctions no longer depend on frontend guesses or an extra center patch. The backend primitives are now the source that the frontend renders directly.

## Selection Dragging

The first area was selection-tool dragging.

### Dragging Inside the Selection Box

In selection mode, moving the cursor into the selected bounds switches to a hand cursor. Pressing and dragging moves the selected content freely. The behavior now distinguishes several cases:

- if a selection box already exists, pressing inside it moves the current selection;
- if no selection box exists, pressing directly on a focused object can still start a drag;
- if only part of a molecule is selected, only the selected atoms, labels, or bond endpoints move, while the bonds continue to connect to the moved nodes;
- if the drag target is a single terminal atom, the gesture keeps endpoint-rotation semantics;
- terminal atom dragging snaps to 15-degree increments unless `Alt` is held;
- ordinary multi-object movement is not angle-constrained.

### Temporary Selection and Preserved Selection

The rule for selection state after a drag was refined:

- if an endpoint or bond was already selected before dragging, it remains selected after the drag;
- if an endpoint or bond was only temporarily selected to start that drag, the selection is cleared on mouse up;
- if the user only clicks without dragging, normal click selection still applies.

This avoids leaving an object selected just because it was used as a temporary drag target.

### Stale Focus Cleanup

A visual bug was also fixed where the original endpoint focus could remain visible until mouse up while dragging an endpoint. Starting a drag now clears hover/focus overlays so the old and new positions do not both appear focused.

## Selection Toolbar Arrangement Commands

The selection toolbar gained the full arrangement set:

- align left;
- align right;
- align top;
- align bottom;
- horizontal center;
- vertical center;
- distribute horizontally;
- distribute vertically;
- mirror left/right;
- mirror up/down.

The important part is the object-boundary model:

- separate molecule objects are arranged as whole objects;
- nodes, bonds, and labels inside one molecule count as one object;
- align-left uses each object’s leftmost outer edge;
- distribution equalizes gaps between adjacent object edges, not center-to-center distances;
- mirroring keeps the overall selection center fixed.

The toolbar icons were adjusted as part of this pass. Vertical center, vertical distribution, and horizontal distribution no longer use misleading older icons. The left-side template icon now also follows the currently selected template from the top toolbar.

## Ring and Template Tools

The template tool expanded from one default six-membered ring into a small structure set:

- three-membered ring;
- four-membered ring;
- five-membered ring;
- six-membered ring;
- seven-membered ring;
- eight-membered ring;
- benzene.

### Endpoint Clicks

Clicking an endpoint creates the ring directly attached to that endpoint. It no longer creates an extra connecting bond first.  
For endpoint clicks, the ring center is placed on the extension of the existing bond at that endpoint. Without dragging, the generated ring is the regular default orientation.

### Bond Clicks

Clicking a bond uses that bond as one edge of the ring. Direction and reuse rules moved closer to chemistry-editor behavior:

- prefer the less substituted side of the bond;
- if both sides are substituted, check whether existing bond angles match the target ring angle;
- if one side has a matching angle, reuse the relevant nodes;
- if both sides have matching angles, try to reuse three nodes;
- during ring generation, if a new vertex coincides with an existing endpoint or label center, reuse that existing node.

The core constraint is that generating bonds or rings must never create two carbons at the same point.

### Blank Canvas Clicks and Dragging

Clicking a polygon template on blank canvas follows ChemDraw-style behavior and creates a regular ring centered at the click point.  
Dragging is different: drag placement still uses endpoint-origin direction logic. The drag direction snaps in 15-degree increments, with the same `Alt` free-angle behavior as endpoint dragging.

## Default Double-Bond Direction

The default behavior when clicking a single bond to convert it to a double bond was tightened.

The confirmed rule is:

- only when one side of the bond has two connected bonds and the other endpoint has no connected bonds should the default be a centered, equal-length double bond;
- all other cases should not default to a centered double bond;
- a bond with one single bond connected at each end must not be misclassified as an equal-length double-bond case.

The root cause was that the “default center double” condition was too broad. It was narrowed to the true structural case of one crowded side and one empty side.

## Multi-Bond Junction Rendering

The largest debugging thread was multi-bond junction rendering, especially three-bond and four-bond junctions and junctions that mix a wedge bond with ordinary bonds.

### Reconfirmed Geometry Rule

The final general rule is:

1. first build each bond’s original outline polygon;
2. identify the two contour lines near the shared junction for each bond;
3. extend adjacent outer contour lines and compute their intersections;
4. the number of intersections should match the number of bonds;
5. each bond’s junction-side contour is built from its two adjacent intersection points plus the center point `C`;
6. no extra center triangle or center polygon patch is needed.

For the three-way junction, enlarged SVG diagrams were used to verify the positions of `C`, `P1`, `P2`, and `P3`. One important correction was that a previous implementation had effectively swapped the `C` and `P3` classification, which then caused the polygon connection order to keep failing.

### Removing the Center Patch

The extra black triangle in the frontend came from the backend emitting an extra center patch in addition to the three bond polygons.  
That patch could be gray in a debug diagram, but in normal frontend rendering it was filled black, creating a visible triangle that did not belong to any bond.

The center patch was removed. A three-way junction now emits only the three `document-bond` polygons. Frontend debug output confirmed:

- `document-bond primitives: 3`;
- DOM polygons match primitive data;
- no extra center patch is present.

### Wedge Bonds Use the Same Junction Rule

Junctions between a solid wedge and two ordinary single bonds previously produced black overflow.  
The corrected behavior uses the same geometry rule:

- the wedge bond is still a polygon;
- the bond endpoint anchor should lie at the center of the terminal edge;
- the junction center should not drift into the interior of the wedge;
- adjacent wedge and single-bond contour lines participate in the same extended-line intersection calculation;
- `CP1`, `CP2`, and `CP3` boundary segments are assigned by adjacent region ownership.

The fix is not a wedge-specific special case. The important part was removing unnecessary restrictions and letting wedge bonds return to the same “all bonds are outline polygons” model.

### Backend Debug Diagrams and Frontend Rendering Were Reconnected

Debugging exposed a more serious process issue: the backend diagram and frontend rendering had not always been generated from the same output.  
A frontend debug script now reads the viewer’s `renderListJson` primitives and DOM polygons directly, producing:

- primitive JSON;
- SVG;
- PNG;
- a DOM-versus-primitive consistency check.

This means future junction debugging can compare the actual frontend output to the actual primitive stream instead of relying on a separate hand-drawn or backend-only diagram.

## Frontend JSON Open and Save

File I/O was wired into the frontend toolbar.

### Current Storage Format

The current storage format is the chemcore document JSON. The project convention is:

- `.chemcore.json`

Plain `.json` files can also be opened.

### Save

The top save button now saves the current document JSON:

- it prefers the browser File System Access API and opens a save-file dialog;
- when unsupported, it falls back to downloading JSON;
- before saving, it commits the active text-edit session so the saved document is not stale;
- the output is formatted chemcore document JSON.

### Open

The open button now shows a file picker:

- select a JSON file;
- validate the basic `document`, `objects`, and `resources` structure;
- load it into the editing engine;
- reset undo/redo state;
- continue editing the opened document.

## Zoom Interaction

The first version of `Ctrl/Command + mouse wheel` canvas zoom was added:

- browser page zoom is prevented;
- the chemcore canvas zooms instead;
- the mouse world position is used as the zoom anchor;
- maximum zoom was raised from `400%` to `800%`.

One newer user requirement still needs to be applied: the zoom control should become a fixed dropdown with `12%`, `25%`, `50%`, `75%`, `100%`, `150%`, `200%`, `400%`, `600%`, and `800%`; wheel zoom should snap to these same levels instead of using continuous increments.

## Selection Rotation Handle

A rotation interaction was added to the selection box.

### Interaction Behavior

In selection mode, a rotation handle appears above the selected bounds:

- dragging the handle rotates around the selection bounds center;
- without `Alt`, the angle snaps to 15-degree increments;
- with `Alt`, the angle is free;
- during rotation, the molecule’s selection boxes and selection dots are hidden;
- the current angle appears near the upper-right empty area;
- on mouse up, the selection bounds are recomputed and redrawn as an axis-aligned box.

The axis-aligned box behavior matters: the selection box itself does not rotate with the object. It is recalculated after the object geometry changes.

### Engine Implementation

The engine now has a selection-rotation drag channel:

- `beginSelectionRotate`;
- `updateSelectionRotate`;
- `finishSelectionRotate`.

Rotation stores the original positions of selected nodes and text objects, uses the selected bounds center as the pivot, and recomputes positions from the current angle.  
After molecular nodes rotate, attached-label geometry is refreshed and fragment bounds are updated.

### Verification

Regression coverage was added for:

- rotating a selected bond snaps to 15 degrees by default;
- `Alt` rotation keeps a free angle;
- selection boxes and selection dots are absent from the render list during rotation;
- frontend Playwright replay confirms angle display, hidden selection overlays during rotation, and redrawn bounds on mouse up.

## WASM and Viewer Synchronization

The `viewer/engine` WASM bindings were rebuilt several times so the frontend could call the new engine APIs.  
The synchronization points included:

- selection movement;
- selection arrangement;
- ring templates;
- JSON open/save;
- canvas zoom;
- selection rotation.

This exposed the same engineering rule again: changing the Rust engine alone does not change the frontend until the WASM package is rebuilt. Any future engine API or render-primitive change needs WASM rebuild and frontend debug verification in the same validation chain.

## Cleanup

Temporary generated image cache under `tmp/` was cleaned, including:

- `png`;
- `svg`;
- `jpg`;
- `jpeg`;
- `webp`.

Formal documentation images were left untouched.

## Verification

Verification run during the day included:

- `cargo test -p chemcore-engine select_tool_ -- --nocapture`;
- multiple targeted `render_document` tests;
- `npm run build:engine-wasm`;
- `node --check viewer/app.js`;
- `git diff --check`;
- Playwright replay for opening and saving JSON;
- Playwright replay for `Ctrl + wheel` zoom;
- Playwright replay for selection-box rotation.

## Follow-Up

Several items remain:

- change the zoom control from free text input to a fixed dropdown;
- make wheel zoom snap to fixed zoom levels;
- update the multi-bond region-map debug script for the no-center-patch output;
- broaden multi-bond junction tests, especially for four-way junctions, wedge bonds, bold bonds, and cross-molecule node reuse;
- consider retaining a file handle after save so a later “Save” can write to the same file instead of always behaving like Save As;
- text-object rotation currently moves the text anchor only. If text itself should rotate with the selection, `transform.rotate` needs to become part of the unified edit semantics.
