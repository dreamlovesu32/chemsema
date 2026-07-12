# Command Transaction v1

Status: beta contract.

Schema id: `chemcore.command-transaction.v1`.

Command transactions are optional envelopes accepted anywhere command scripts are
accepted: `chemcore-cli new`, `chemcore-cli run`, and JSONL session `execute`.
Plain single-command objects and command arrays remain valid.

## Shape

```json
{
  "schema": "chemcore.command-transaction.v1",
  "preconditions": {
    "expectedDocumentHash": "64-char-sha256",
    "expectedRevision": 12,
    "requiredSelectors": ["object:obj_editor_molecule", "node:n_1"]
  },
  "scope": {
    "editableTargets": ["object:obj_editor_molecule"],
    "includeDescendants": true,
    "includeReferencedResources": true,
    "allowCreate": false,
    "allowDelete": false,
    "forbidChangesOutsideScope": true
  },
  "options": {
    "atomic": true,
    "dryRun": false
  },
  "commands": [
    { "type": "replace-node-label", "node_id": "n_1", "label": "OMe" }
  ],
  "postconditions": [
    { "type": "document-valid" },
    { "type": "no-unexpected-changes" },
    { "type": "selector-exists", "selector": "object:obj_editor_molecule" }
  ]
}
```

## Semantics

- Transactions execute on a cloned engine state first.
- `dryRun: true` reports execution and diff but does not replace the original
  state and does not save document outputs.
- Preconditions are checked before any command runs.
- Scope validation uses the structured document diff, not text diffing.
- `editableTargets` defines what may change. Visual context from bundle/capture
  does not grant edit permission.
- `includeReferencedResources` allows the molecule resource, nodes, and bonds
  referenced by an editable molecule object.
- `allowCreate: false` rejects created selectors.
- `allowDelete: false` rejects deleted selectors.
- `forbidChangesOutsideScope: true` rejects changes outside allowed selectors.

## Report

Reports include:

- `transaction`: schema, `atomic`, `dryRun`, and `applied`
- `preconditions`: check list and failures
- `execution`: command results
- `diff`: `chemcore.document.diff.v1`
- `scope`: allowed selectors and `unexpectedChanges`
- `postconditions`: check list and failures
- `document`: before/after hash and revision

`transaction.applied` is `false` for dry runs and for any failed transaction.
