# Command Scripts

Use `new` for blank documents and `run` for edits against an existing document.

```powershell
chemcore-cli new commands.json --out output.ccjs --results results.json --document-json output-document.json --pretty
chemcore-cli run input.cdxml commands.json --out edited.cdxml --results results.json --inspect-after summary,objects,molecules --pretty
```

The command input is a JSON object or array. Use `-` to read JSON from stdin.

## Reports

Results include:

- top-level `ok`, `commandCount`, `executedCount`, `failedCount`, `failedIndex`,
  and `failedIndices`
- per-command `ok`, `executed`, `commandType`, `document`, `changeSummary`, and
  `error`
- document hash and revision transitions

Use `--inspect-after <include>` only when a structural snapshot is needed.

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

OCR must not use these planning queries as a replacement for measuring the
source raster. Drawing agents should use them to avoid duplicating ChemCore GUI
placement logic.

## Template Inserts

Use `insert-template` when creating rings and standard fragments. Prefer the
extended anchored form when attaching a ring to a focused atom or bond, because
it reuses engine placement and overlap avoidance.
