# Document Inspection

Use selectors instead of coordinate guessing.

## Whole-Document Inspection

Use `inspect` before target-specific work when you need document-level object,
molecule, resource, or style summaries:

```powershell
chemcore-cli inspect input.cdxml --include summary,objects,molecules,resources,styles --out inspect.json --pretty
```

Keep `inspect` output file-backed for large documents.

## Target Discovery

```powershell
chemcore-cli targets input.cdxml --out targets.json --pretty
```

Valid selectors include:

```text
all
object:<scene-object-id>
molecule:<zero-based-index>
node:<node-id>
bond:<bond-id>
bounds:<minX>,<minY>,<maxX>,<maxY>
selection:<selector;selector>
```

Use `molecule:<index>` for a whole molecule, `object:<id>` for a document
object, and `bounds:` only for crop-style queries.

## Context Before Detail

Use `context` when deciding what is nearby:

```powershell
chemcore-cli context input.cdxml --target molecule:1 --expand-rel 0.2 --out context.json --capture-out context.png --scale 6 --pretty
```

`selectionBox.contents` reports objects, molecules, nodes, and bonds inside a
multi-target query. `selectionBoxRelation` distinguishes `inside` from
`partial`.

## Detail

After discovering an id:

```powershell
chemcore-cli detail input.cdxml --target molecule:0 --include-resource --out detail.json --pretty
chemcore-cli detail input.cdxml --target node:n_1 --out node.json --pretty
chemcore-cli detail input.cdxml --target bond:b_1 --out bond.json --pretty
```

For molecule comparisons, use `raw.fragment` and resource payloads rather than
rendered pixels.

## Capture

Use capture for deterministic visual checks:

```powershell
chemcore-cli capture input.cdxml --target molecule:0 --out molecule.png --scale 6 --expand-rel 0.15 --pretty
chemcore-cli capture input.cdxml --target object:obj_a --target object:obj_b --out selection.png --width 1800 --pretty
```

PNG is best for visual inspection. SVG is best when inspecting vector output.
The manifest confirms `output.verified`, byte size, bounds, scale, and render
targets.

## Bundle

Use `bundle` when one selector should become a complete object-grounded work
unit:

```powershell
chemcore-cli bundle input.cdxml --target molecule:0 --out-dir molecule-0-bundle --context-radius 40 --capture-format png --subset-format ccjs --pretty
```

The directory contains `manifest.json`, `target.json`, `context.json`,
`editable-subset.<format>`, `capture.png` or `capture.svg`, and
`identity-map.json`, plus `provenance.json`. The manifest separates `editableScope` from `visualScope`;
objects visible in the capture are not editable unless they are target
selectors. Context entries retain `selectionBoxRelation` and `isTarget`.
The identity map keeps stable selector pairs, while provenance records source
hashes, source bounds, subset translation, and privacy-preserving source file
metadata.

## Diff

Use `diff` for structured before/after comparison:

```powershell
chemcore-cli diff before.ccjs after.ccjs --out diff.json --pretty
```

The report compares document/page data, objects, resources, styles, molecule
nodes, molecule bonds, and field-level changes by ID rather than raw JSON text.

## Multi-Target Selection

Repeated `--target`, `--targets <a;b>`, and `selection:<a;b>` use a union crop
box. This matches the GUI selection box and is useful when reconstructing a
multi-object clipboard payload.
