# Planning Queries

ChemCore exposes readonly planning commands through command scripts.

## plan-bond

Use when a GUI-like agent wants to draw a bond from an existing atom or point.
The query can accept begin, cursor or angle, optional bondLength, order, and
variant. The output includes a final executable `add-bond` command.

Use cases:

- simulate clicking a focused atom and dragging
- use global 15-degree snap behavior
- inspect keypad slot directions
- avoid manually calculating the endpoint for a standard bond

Do not use this as OCR measurement.

## plan-template

Use when inserting a ring or template. It can accept template name, center,
anchor atom, focused bond id, cursor, angle, bondLength, and side. The output
includes vertices, edges, and an insert command.

Use cases:

- benzene/cyclohexane ring attached to a focused atom
- ring fused or placed on a focused bond
- polygon tool behavior matching the GUI
- standard ring geometry without external trigonometry

## label-query

Use before adding text labels:

```powershell
chemcore-cli label-query --text CF3 --connection-angle 0 --pretty
chemcore-cli label-query --visible-text F3C --connection-angle 0 --pretty
```

Use the reported source text, display text, anchor atom, and chemical/default
behavior when emitting label commands.
