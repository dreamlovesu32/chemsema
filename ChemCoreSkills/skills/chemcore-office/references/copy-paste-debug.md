# Copy/Paste Debug

Use this ladder when Office paste loses objects or pastes a non-editable image.

## 1. Confirm Source Selection

```powershell
chemcore-cli targets input.cdxml --out targets.json --pretty
chemcore-cli capture input.cdxml --target all --out all.png --scale 4 --pretty
```

If the selection is not `all`, capture the exact multi-target selection:

```powershell
chemcore-cli capture input.cdxml --target object:obj_a --target object:obj_b --out selection.png --width 1800 --pretty
```

## 2. Generate Payload Without Touching Clipboard

```powershell
chemcore-cli copy input.cdxml --target all --payload payload.json --no-copy --pretty
```

Inspect the payload for document JSON, CDXML, SVG, custom ChemCore data, preview
data, object counts, molecule counts, and selector metadata.

## 3. Compare Payload To Source

Use source `detail` for objects that should be present:

```powershell
chemcore-cli detail input.cdxml --target object:obj_id --include-resource --out detail.json --pretty
```

If the payload is missing an object that appears in `targets`, the bug is in
selection or payload construction. If the payload includes it but Word does not
show it, the bug is in Office helper, preview generation, or Office paste.

## 4. Live Clipboard

Use live clipboard only after the no-copy payload looks correct:

```powershell
chemcore-cli copy input.cdxml --target all --payload payload-live.json --pretty
```

Paste into Word or PowerPoint, then check whether it is editable as a ChemCore
object and whether the preview includes all selected objects.

## 5. Failure Buckets

Classify the failure:

- `selection_missing_object`
- `payload_missing_resource`
- `payload_preview_missing_object`
- `office_helper_error`
- `clipboard_format_missing`
- `word_prefers_noneditable_preview`
- `paste_editable_but_preview_wrong`
- `paste_preview_ok_but_roundtrip_wrong`
