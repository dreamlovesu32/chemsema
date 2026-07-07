---
name: chemcore-office
description: Debug and operate ChemCore Office integration, Windows clipboard, OLE editable objects, Word and PowerPoint paste behavior, EMF previews, ChemCore clipboard payloads, chemcore-office.exe, and chemcore-cli copy workflows. Use when Office paste loses objects, pasted ChemCore content is not editable, payload JSON needs inspection, or desktop copy/paste behavior must be verified.
---

# ChemCore Office

## Workflow

Start from the source ChemCore document and reproduce the Office payload outside
the GUI:

```powershell
chemcore-cli targets input.cdxml --out targets.json --pretty
chemcore-cli copy input.cdxml --target all --payload payload.json --no-copy --pretty
chemcore-cli copy input.cdxml --target molecule:0 --payload molecule-payload.json --no-copy --pretty
```

Then compare three layers:

1. ChemCore structure in the payload.
2. Preview/render resources such as SVG or EMF.
3. Office paste result in Word or PowerPoint.

## Read References As Needed

- For copy/paste diagnosis, read `references/copy-paste-debug.md`.
- For payload fields, OLE formats, and expected artifacts, read
  `references/office-payload.md`.

## Guardrails

- If Word misses objects after full selection, first prove whether the payload
  already missed them. Do not start by blaming Word.
- Use `--target all` and explicit multi-target selections to isolate selection
  logic from payload writing.
- Keep Office-helper failures separate from ChemCore document failures.
- Close or restart Office/desktop apps only when the task needs live clipboard
  testing.
