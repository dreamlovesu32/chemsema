# Editor Command History

This document defines the command layer used by the editor engine. The current
implementation records each committed editing action as a command plus document
snapshots:

```text
HistoryEntry {
  command: EditorCommand,
  before: ChemSemaDocument,
  after: ChemSemaDocument
}
```

The command object is the stable semantic record. The `before` and `after`
documents keep undo/redo behavior exact while the command surface is still
evolving. Later, individual commands can replace full snapshots with smaller
patches or inverse operations without changing the editor event model.

## Rule

Only committed document changes are commands.

Pointer hover, focus halos, preview bonds, lasso outlines, active drag state,
and text cursor movement are interaction state outside history.

## Current Commands

### `add-bond`

Creates a bond from one anchor to another. Either anchor may refer to an
existing node or to a world position that will create/reuse a node.

Recorded data:

- `begin`: anchor node id and world point
- `end`: anchor node id and world point
- `order`: bond order
- `variant`: active bond variant

Typical sources:

- click on blank canvas with the bond tool
- click or drag from an endpoint
- drag from a focus point

### `cycle-bond-style`

Changes the style/order/stereo state of an existing bond center.

Recorded data:

- `bond_id`
- `variant`: active bond variant

Typical sources:

- clicking a bond with single/double/triple/dashed/bold/wedge tools

### `delete-selection`

Deletes the current selection or, when nothing is selected, deletes the current
focused item using command-key semantics.

Selection delete semantics:

- selected bonds are removed
- endpoints of selected bonds are removed only when their original degree is
  fully covered by the selected bonds
- selected atoms are removed together with their incident bonds
- neighboring atoms are kept
- selected labels are converted back to carbon atoms

Typical sources:

- `Delete`
- `Backspace`

### `delete-focused-at-point`

Deletes the focused item at a pointer location.

Recorded data:

- `x`, `y`: world point
- `source`: `delete-tool` or `command-key`

The delete tool and command-key deletion intentionally remain separate because
their endpoint semantics differ.

### `cut-selection`

Copies the current selection into the internal editor clipboard, then deletes
the selection as one undoable command.

Typical source:

- `Ctrl/Cmd+X`

### `paste-clipboard`

Pastes the internal editor clipboard into the editable molecule.

Typical sources:

- paste toolbar button
- `Ctrl/Cmd+V`

### `insert-template`

Commits a structure template.

Recorded data:

- `template`: template id, such as `ring-6` or `benzene`
- `x`, `y`: commit point

Typical sources:

- click or drag with the template tool

For `benzene`, the six ring edges always resolve to one alternating set of three double bonds. A bond-anchored insertion reuses the shared edge instead of creating a duplicate. If the shared edge is already double, it counts as one of the three and its side placement moves into the inserted ring. If the shared edge is the single edge of an existing alternating six-membered ring, that ring is relaid out so the shared edge becomes the one double bond reused by both fused rings.

### `apply-selection-arrange`

Applies a selection layout command.

Recorded data:

- `command`: toolbar command id

Current command ids:

- `align-left`
- `align-right`
- `align-top`
- `align-bottom`
- `align-h-center`
- `align-v-center`
- `distribute-h`
- `distribute-v`
- `flip-h`
- `flip-v`

### `apply-selection-color`

Applies a color to the current selection.

Recorded data:

- `color`: normalized hex color string

Current behavior:

- selected text objects update their text fill style and rich text run fills
- selected molecule labels update label and run fills
- selected molecule nodes or bonds update the molecule style color
- selected line, bracket, symbol, and shape objects update their stroke and/or fill style color

### `move-targets`, `rotate-targets`, `scale-targets`, `delete-targets`

Apply command-script edits to explicit target sets without relying on the GUI
selection state.

Recorded data:

- `targets`: nodes, bonds, scene objects, and label nodes
- `delta` for `move-targets`
- `center` and `degrees` for `rotate-targets`
- `scaleX`, `scaleY`, and optional `pivot` for `scale-targets`

### `move-selection`

Moves the current selected molecule parts.

The command is opened on the first document-changing drag update and its
`after` snapshot is refreshed until the final mouse-up position.

### `rotate-selection`

Rotates the current selected molecule parts.

The command is opened on the first document-changing rotate update and its
`after` snapshot is refreshed until the final mouse-up angle.

### `apply-text-edit`

Applies an active text edit session.

Recorded target:

- `text-object`: optional object id
- `endpoint-label`: node id

### `replace-hovered-endpoint-label`

Replaces the hovered endpoint with a typed atom or abbreviation label.

Recorded data:

- `label`

### `legacy-mutation`

Fallback command used if a document change still calls the low-level snapshot
API outside a command context. This should be treated as a migration warning:
new editing features should use semantic commands.

## Transient Actions

The following actions are transient UI/runtime actions:

- `copy-selection`: changes only the internal clipboard
- `select-targets`, `select-all`, and `clear-selection`: change only current
  in-memory selection unless followed by a document-changing command
- `set-tool`
- `set-template`
- hover/focus updates
- preview generation
- viewport zoom and pan
- open/load document, which resets history

## Implementation Notes

All committed mutations should run inside `Engine::with_command`. Existing
low-level mutation helpers may still call `push_undo_snapshot`; the command
context assigns the current semantic command to that snapshot.

If one user command creates multiple internal snapshots, the command layer
coalesces them into one `HistoryEntry` using the first `before` document and
the final `after` document.

For drag commands, intermediate updates may mutate the document after the first
snapshot. The command layer refreshes the latest matching history entry's
`after` document so redo returns to the final pointer-up state.
