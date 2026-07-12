# Object-Grounded Agent Model

ChemCore separates semantic understanding from document operation.

Semantic understanding is about chemical meaning: molecule identity, reaction
roles, properties, provenance, dictionaries, ontology, and future CML
interchange.

Document operation is about editable drawing state: object identity, molecule
resources, styles, layout, transforms, grouping, visual rendering, commands,
and export.

The object-grounded agent layer belongs to document operation. It gives an
agent one shared selector across:

- `targets`
- `detail`
- `context`
- `capture`
- `bundle`
- `run` or session `execute`
- `diff`
- target-only `convert`/`export`

The agent does not need to process the whole document every time. It can
resolve a selector, inspect only the necessary raw data, view a local visual
crop, export only the editable target subset, and verify the before/after
document changes.

## Editable Scope And Visual Scope

Editable scope and visual scope are intentionally different.

The editable scope is the target-only document subset. It contains the selected
object or objects and required dependencies such as molecule resources, styles,
group ancestors, and links preserved by the existing export path.

The visual scope is the context/capture region. It may include non-target
objects that are visible inside the selection box or expanded neighborhood.
Those objects remain read-only unless they are explicitly part of the target
selectors. `context.json` preserves `selectionBoxRelation` and `isTarget` so
callers can distinguish target content from visible context.

## CML Boundary

CML can become a future semantic layer for chemical meaning, properties,
provenance, dictionaries, ontology, and reaction semantics. This phase does not
implement a CML parser or exporter.

CCJS remains ChemCore's editable document/runtime model. CDXML remains the
compatibility bridge for ChemDraw documents. CML should not replace document
layout fields such as object transforms, style references, z-order, grouping,
visual bounds, or capture geometry.
