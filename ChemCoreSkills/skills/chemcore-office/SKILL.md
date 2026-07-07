---
name: chemcore-office
description: Diagnose ChemCore Office/OLE payloads, Windows clipboard handoff, Word and PowerPoint paste results, EMF previews, chemcore-office.exe, and chemcore-cli copy workflows. Use for payload JSON inspection, object-count paste checks, editable-paste verification, and desktop copy/paste behavior verification.
---

# ChemCore Office

## Workflow

Start from the source ChemCore document and reproduce the Office/OLE payload
outside the GUI:

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

- For missing objects after full selection, first compare payload contents with
  source targets.
- Use `--target all` and explicit multi-target selections to isolate selection
  logic from payload writing.
- Keep Office-helper failures separate from ChemCore document failures.
- Close or restart Office/desktop apps only when the task needs live clipboard
  testing.
