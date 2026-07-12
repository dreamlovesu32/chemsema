# ChemCore Agent Bundle v1

Protocol id: `chemcore.agent.bundle.v1`.

`chemcore-cli bundle` writes a deterministic directory of artifacts for one
object-grounded agent work unit. It combines target detail, nearby context,
visual capture, editable subset export, and identity mapping without requiring
the caller to stitch separate CLI outputs together.

## Command

```powershell
chemcore-cli bundle input.cdxml --target object:obj_mol_001 --out-dir output/bundle --context-radius 40 --capture-format png --subset-format ccjs --pretty
```

Accepted target forms are `object:<id>`, `molecule:<index>`, `node:<id>`,
`bond:<id>`, repeated `--target`, `--targets <selector;selector>`, and
`selection:<selector;selector>`. `all` and `bounds:` are rejected because a
bundle must have a bounded editable target.

## Artifacts

The output directory contains:

- `manifest.json`
- `target.json`
- `context.json`
- `editable-subset.<format>`
- `capture.png` or `capture.svg`
- `identity-map.json`

`manifest.json` includes `schema`, `source`, `target`, `editableScope`,
`visualScope`, `artifacts`, `artifactVerification`, and `integrity`.

## Scope Rule

`editableScope` is the only document content the agent may modify. It contains
the target objects and required editable dependencies exported through the
existing target-only export path.

`visualScope` is the capture and context region. It can include visible
non-target neighbors such as arrows, text, or adjacent molecules. Context
entries keep `selectionBoxRelation` and `isTarget` so callers can tell whether
an object was selected or merely visible.

Seeing an object in `capture.png` or `context.json` does not grant edit scope.

## Verification

Every written artifact is verified as a regular file with nonzero bytes.
`artifactVerification[]` records relative path, format, byte count, and SHA-256.
`integrity` reports whether referenced resources and styles are resolved,
whether capture was verified, and whether the editable subset was valid.

## Session

JSONL session supports `op:"bundle"` with `target`, `outDir`,
`contextRadius`, `captureFormat`, `scale`/`width`/`height`, `subsetFormat`, and
`pretty`.
