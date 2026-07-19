---
name: chemsema-office
description: Diagnose ChemSema Office/OLE payloads, Windows clipboard handoff, Word and PowerPoint paste results, EMF previews, chemsema-office.exe, and chemsema-cli copy workflows. Use for payload JSON inspection, object-count paste checks, editable-paste verification, and desktop copy/paste behavior verification.
---

# ChemSema Office

## Workflow

Start from the source ChemSema document and reproduce the Office/OLE payload
outside the GUI:

```powershell
chemsema-cli targets input.cdxml --out targets.json --pretty
chemsema-cli copy input.cdxml --target all --payload payload.json --no-copy --pretty
chemsema-cli copy input.cdxml --target molecule:0 --payload molecule-payload.json --no-copy --pretty
```

Then compare three layers:

1. ChemSema structure in the payload.
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
- Keep Office-helper failures separate from ChemSema document failures.
- Close or restart Office/desktop apps only when the task needs live clipboard
  testing.
