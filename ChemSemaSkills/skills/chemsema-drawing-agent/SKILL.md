---
name: chemsema-drawing-agent
description: Plan ChemSema-native drawing command scripts using engine queries instead of hand geometry. Use when an agent must create or edit molecules with ChemSema CLI, match GUI bond drawing, attach labels, insert rings/templates, choose label source text and visible reversal, or use plan-bond, plan-template, insert-template, label-query, new, or run.
---

# ChemSema Drawing Agent

## Workflow

Use the engine as the drawing oracle. The agent should describe the intended
chemical edit and ask ChemSema for landing geometry, label behavior, and
template placement before emitting commands.

1. Use `label-query` for source text, visible text, anchors, and
   `defaultChemical`.
2. Use `plan-bond` to simulate GUI bond drawing from a known atom or endpoint.
3. Use `plan-template` to simulate GUI ring/template insertion.
4. Emit the returned `add-bond` or `insert-template` commands.
5. Run the command script with `new` or `run`.
6. Inspect the result using `targets`, `detail`, and `capture`.

## Read References As Needed

- For engine planning calls, read `references/planning-queries.md`.
- For document creation and edit loops, read `references/drawing-workflows.md`.
- For template, bond, and style decisions, read `references/templates-style.md`.

Use `scripts/write_commands.py` to assemble small command arrays when the task
needs deterministic JSON writing.
