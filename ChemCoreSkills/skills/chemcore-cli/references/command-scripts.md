# Command Scripts

Use `new` for blank documents and `run` for edits against an existing document.

```powershell
chemcore-cli new commands.json --out output.ccjs --results results.json --document-json output-document.json --pretty
chemcore-cli run input.cdxml commands.json --out edited.cdxml --results results.json --inspect-after summary,objects,molecules --pretty
```

The command input is a JSON object, array, or
`chemcore.command-transaction.v1` envelope. Use `-` to read JSON from stdin.
The same command JSON is accepted by JSONL `session` operation `execute`.

## Reports

Results include:

- top-level `ok`, `commandCount`, `executedCount`, `failedCount`, `failedIndex`,
  and `failedIndices`
- per-command `ok`, `executed`, `commandType`, `document`, `changeSummary`, and
  `error`
- document hash and revision transitions

Use `--inspect-after <include>` only when a structural snapshot is needed.

## Transactions

Use a transaction envelope when an edit must prove it stayed in scope:

```json
{
  "schema": "chemcore.command-transaction.v1",
  "preconditions": {
    "expectedDocumentHash": "64-char-sha256",
    "requiredSelectors": ["object:obj_editor_molecule", "node:n_1"]
  },
  "scope": {
    "editableTargets": ["object:obj_editor_molecule"],
    "includeReferencedResources": true,
    "allowCreate": false,
    "allowDelete": false,
    "forbidChangesOutsideScope": true
  },
  "options": { "atomic": true, "dryRun": true },
  "commands": [
    { "type": "replace-node-label", "node_id": "n_1", "label": "OMe" }
  ],
  "postconditions": [
    { "type": "document-valid" },
    { "type": "no-unexpected-changes" }
  ]
}
```

- `dryRun:true` reports execution, diff, allowed selectors, and unexpected
  changes without mutating or saving the document.
- `editableTargets` is the only modification scope. Seeing nearby objects in a
  bundle/capture does not grant permission to edit them.
- Session `execute` accepts the transaction fields directly or under
  `transaction`.

## Add Bond

`add-bond` creates a ChemCore bond. For double bonds, omit placement unless the
source explicitly requires a fixed side.

```json
{
  "type": "add-bond",
  "begin": {"x": 100, "y": 120},
  "end": {"x": 145, "y": 120},
  "order": 2,
  "doublePlacement": "center"
}
```

## Planning Queries

`plan-bond` is a readonly query for GUI-like drawing agents. It accepts an
existing begin point plus cursor, angle, bondLength, order, and variant. It
returns an executable `add-bond` command, global snap angles, and keypad slots.

`plan-template` is a readonly query for template vertices and edges. It accepts
template, x, y, anchor, bondId, cursor, angle, bondLength, and side. It returns
vertices, edges, and an insert command.

Drawing agents should use these planning queries to avoid duplicating ChemCore
GUI placement logic.

## Template Inserts

Use `insert-template` when creating rings and standard fragments. Prefer the
extended anchored form when attaching a ring to a focused atom or bond, because
it reuses engine placement and overlap avoidance.

## Selection State

Use selection commands before GUI-style selection edits:

```json
[
  {
    "type": "select-targets",
    "targets": {
      "objects": ["text_1", "text_2"],
      "nodes": [],
      "bonds": [],
      "labelNodes": []
    }
  },
  { "type": "apply-selection-arrange", "command": "align-left" }
]
```

- `select-targets` sets single-select or multi-select from explicit target ids.
- `select-all` selects visible/editable document content.
- `clear-selection` clears the current selection.
- Selection commands change in-memory state, not document revision.

After selecting, commands such as `apply-selection-arrange`,
`scale-selection`, `center-selection-on-page`, `apply-selection-color`,
`group-selection`, `ungroup-selection`, `link-selection`, `unlink-selection`,
style commands, `apply-object-settings-to-selection`, `delete-selection`, and
`cut-selection` can omit id arrays and use the current selection.

## Target Editing

Prefer explicit target commands when the edit should be stateless:

```json
{ "type": "move-targets", "targets": { "bonds": ["bond_1"] }, "delta": { "dx": 10, "dy": 0 } }
```

Use `rotate-targets` with `center` and `degrees`. Use `scale-targets` with
`scaleX`, `scaleY`, and optional `pivot`; unequal factors perform scripted
stretch. Use `delete-targets` for explicit deletion.

Use `apply-object-settings-to-selection` for bond length, line width, bold
width, bond spacing, margin width, and hash spacing. It accepts explicit
`bond_ids`/`object_ids`, or the current selection after `select-targets`.
