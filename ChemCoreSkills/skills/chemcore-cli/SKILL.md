---
name: chemcore-cli
description: Use ChemCore CLI for chemical document inspection, editing, capture, conversion, label queries, Office payload debugging, and JSONL sessions. Trigger this skill when working with ChemCore documents or formats such as CCJS, CCJZ, CDXML, CDX, SDF, SVG, PNG, Office clipboard payloads, ChemCore command scripts, selectors, molecule targets, label-query, plan-bond, plan-template, capture, context, detail, targets, new, run, convert, export, or session workflows.
---

# ChemCore CLI

## Core Workflow

Use the CLI as the machine contract for ChemCore. Prefer runtime discovery over
memory:

```powershell
chemcore-cli version --pretty
chemcore-cli doctor --pretty
chemcore-cli capabilities --pretty
chemcore-cli schema all --out chemcore-schema.json --pretty
chemcore-cli guide --kind detailed --out chemcore-guide.json --pretty
```

For large JSON, always pass `--out <path>` and read the file. `--pretty` only
changes whitespace.

Use this order for document work:

1. `about`, `examples`, `guide`, `schema`, and `capabilities` to discover the
   installed runtime.
2. `inspect` for whole-document summaries.
3. `targets` to discover stable selectors.
4. `context` to inspect neighborhoods and selection boxes.
5. `detail` to expand one object, molecule, node, or bond.
6. `capture` for deterministic SVG/PNG crops.
7. `new` or `run` with command scripts for edits.
8. `convert` or `export` for whole-document output, or target-only editable
   subset output with `--target`/`--targets`.
9. `copy` for Windows Office/OLE clipboard payloads.
10. `session` for repeated work on one document.

## Read References As Needed

- For locating the executable and handling repo vs installed builds, read
  `references/runtime-discovery.md`.
- For selectors, targets, detail, context, and capture, read
  `references/document-inspection.md`.
- For `new`, `run`, `session execute`, command JSON, selection state, target
  editing, arrange/group/link/style commands, `plan-bond`, and `plan-template`,
  read `references/command-scripts.md`.
- For `convert`, `export`, editable formats, and raster/vector output, read
  `references/formats-conversion.md`.
- For chemical text, visible text, reverse labels, anchors, and
  `defaultChemical:false`, read `references/label-query.md`.
- For long iterative operations, read `references/session-jsonl.md`.

## Helpers

Use bundled helpers instead of retyping discovery code:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\find_chemcore_cli.ps1 -Json
python scripts\run_chemcore_cli.py version --pretty
python scripts\session_jsonl.py input.cdxml requests.jsonl --out transcript.jsonl
python scripts\check_cli_skill_sync.py --suite-root ..\..\..\.. --json
```

## Guardrails

- Do not parse console text when a JSON output path exists.
- Do not invent selectors; discover them from `targets` or `context`.
- Do not manually calculate label reversal, generated hydrogens, or chemical
  anchors when `label-query` can answer.
- Do not manually assemble temporary CCJS just to export part of a document;
  discover selectors with `targets` and use `convert`/`export --target` for
  editable subset output.
- Preserve original document semantics when the visible drawing intentionally
  disagrees with default chemical rewriting; use `defaultChemical:false` when
  appropriate.
