# ChemCore Document Diff v1

Protocol id: `chemcore.document.diff.v1`.

`chemcore-cli diff` compares two editable documents by ChemCore object identity
and structured fields. It is not a textual JSON diff.

## Command

```powershell
chemcore-cli diff before.ccjs after.ccjs --out diff.json --pretty
```

## Output

The output object includes:

- `schema`
- `ok`
- `equal`
- `document`
- `page`
- `objects`
- `resources`
- `styles`
- `nodes`
- `bonds`
- `changes`
- `unexpectedChanges`
- `counts`

Entity sections report `created`, `updated`, and `deleted` selectors where
applicable. Field changes include:

```json
{
  "selector": "node:n_17",
  "path": "resources.*.data.nodes.n_17.label.sourceText",
  "before": "CF3",
  "after": "OMe"
}
```

Selectors and field changes are sorted deterministically.

## Intended Reuse

The same internal diff API is intended for later transaction work:

- atomic before/after reports
- dry-run previews
- allowed scope validation
- `no-unexpected-changes` postconditions

`unexpectedChanges` is present in v1 as an empty array so future scoped
transaction reports can use the same shape.
