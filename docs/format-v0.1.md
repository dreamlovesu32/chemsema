# chemcore Format v0.1

## Scope

This document defines the first persisted document format for `chemcore`.

Version `0.1` is intentionally narrow. It is a document/object format for
rendering and future editing, not a complete chemistry interchange standard.

Its immediate purpose is:

- to represent a single chemistry page
- to support read-only rendering
- to receive converted data from CDXML extraction
- to act as the base for future runtime and editing logic

## Format Overview

The file is a JSON document with five top-level sections:

- `format`
- `document`
- `styles`
- `objects`
- `resources`

At a high level:

- `document` defines global metadata and page settings
- `styles` stores reusable rendering styles
- `objects` stores the scene graph nodes
- `resources` stores reusable chemistry payloads such as `molecule_fragment2d`

## Top-Level Structure

```json
{
  "format": {
    "name": "chemcore",
    "version": "0.1"
  },
  "document": {},
  "styles": {},
  "objects": [],
  "resources": {}
}
```

## Coordinate System

Version `0.1` uses a single world coordinate system:

- origin: top-left
- x increases to the right
- y increases downward
- units: abstract document units

No backend-specific pixel assumptions belong in the file.

## Object Identity

Every object must have a globally unique `id` within the document.

Rules:

- object ids are strings
- style ids are strings
- resource ids are strings
- references are by id, never by array position

## Containment Rules

Version `0.1` uses a strict tree for object ownership.

Rules:

- Every object must belong to exactly one container
- A container is either the top-level `objects` array or one `group.children` list
- An object may have at most one direct parent group
- An object must not appear both at top level and inside a group
- An object must not appear in more than one `group.children` list

This keeps ownership, traversal, selection, and editing behavior deterministic.

## Object Model

Each scene object shares a common envelope:

```json
{
  "id": "obj_001",
  "type": "molecule",
  "name": "optional human label",
  "visible": true,
  "locked": false,
  "zIndex": 10,
  "transform": {
    "translate": [0, 0],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_default",
  "meta": {},
  "payload": {}
}
```

### Common Fields

- `id`: unique object id
- `type`: one of the supported object types
- `name`: optional label for debugging or UI
- `visible`: whether the object participates in rendering
- `locked`: whether the object is editable
- `zIndex`: global stacking key
- `transform`: local transform
- `styleRef`: optional style id
- `meta`: non-render-critical metadata
- `payload`: type-specific data

### Supported Object Types in v0.1

- `molecule`
- `text`
- `line`
- `bracket`
- `shape`
- `group`

Other graphical primitives can be added later.

## Object Type Baseline

Version `0.1` should start from a small but stable set of first-class object
types:

- `molecule`: chemistry-bearing 2D structure
- `text`: positioned rich text
- `line`: straight/curved stroke objects, including arrows
- `bracket`: bracket-like graphical objects
- `shape`: simple filled or stroked regions
- `group`: logical grouping and shared transform

This split is intentional.

- `molecule` owns chemistry semantics
- `text`, `line`, `bracket`, and `shape` are document graphics
- `group` owns containment and transform only

Important: labels that belong to a `molecule` are not generic `text` objects.
Examples include `CN`, `Ph`, `N3`, `t-Bu`, `HN`, or stacked hetero labels such as
`H` over `N`. These are structure labels with:

- an attachment anchor inside the label
- orientation relative to the attached bond
- chemistry-aware ordering rules
- optional inline sub/superscript formatting
- optional multiline run data such as `lineRuns`, when a structure label is
  displayed as stacked lines but still needs per-token styling like the
  subscript `2` in `SO2`
- normalized display runs should preserve chemistry-relevant inline formatting
  such as subscript and superscript, but should not directly inherit
  source-format text styling like CDXML `face` weight/style flags
- raw source runs may still be preserved for import fidelity, but they belong
  under `meta.import.<source>`, not beside normalized display fields

They should live inside molecule resources or molecule-specific payloads, not be
modeled as standalone document text boxes.

Viewer note: a renderer may apply small bounded optical adjustments at display
time, for example to separate attached-group labels from nearby atom labels.
These adjustments are viewer behavior only. They must not rewrite the stored
document geometry.

Brackets are kept separate from `molecule` in `v0.1`. They often appear around
chemistry, but they are still document objects first. Chemical meaning, if
needed later, can be added through metadata rather than by collapsing brackets
into the molecule model.

## Transform

All objects may carry a local transform:

```json
"transform": {
  "translate": [120, 40],
  "rotate": 0,
  "scale": [1, 1]
}
```

Rules:

- `translate` is required
- `rotate` defaults to `0`
- `scale` defaults to `[1, 1]`

For `v0.1`, transforms are applied in local-to-world order:

1. scale
2. rotate
3. translate

## Styles

Styles are stored separately and referenced by `styleRef`.

Example:

```json
"styles": {
  "style_text_default": {
    "kind": "text",
    "fontFamily": "Helvetica",
    "fontSize": 12,
    "fontWeight": 400,
    "fill": "#111111",
    "stroke": null
  },
  "style_line_default": {
    "kind": "stroke",
    "stroke": "#222222",
    "strokeWidth": 1.5,
    "lineCap": "round",
    "lineJoin": "round"
  }
}
```

Version `0.1` does not enforce a hard style taxonomy beyond `kind`, but the
renderer should expect styles to describe either:

- text appearance
- stroke/fill appearance
- molecule appearance

## Resources

`resources` hold reusable content blobs that do not naturally belong inline in
every object.

Version `0.1` defines one resource type explicitly:

- `molecule_fragment2d`

Example:

```json
"resources": {
  "mol_a": {
    "type": "molecule_fragment2d",
    "encoding": "chemcore.molecule.fragment2d",
    "data": {}
  }
}
```

This keeps molecule objects small and makes repeated references possible.

## Molecule Object

The molecule object represents a placed chemistry-bearing structure on the page.

Example:

```json
{
  "id": "obj_mol_1",
  "type": "molecule",
  "visible": true,
  "locked": false,
  "zIndex": 10,
  "transform": {
    "translate": [96, 72],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_molecule_default",
  "meta": {
    "source": "cdxml",
    "collapsed": false
  },
  "payload": {
    "resourceRef": "mol_a",
    "bbox": [0, 0, 88, 42],
    "role": "substrate"
  }
}
```

### Molecule Payload Fields

- `resourceRef`: required, points to a `molecule_fragment2d` resource
- `bbox`: optional local bounding box
- `role`: optional semantic hint such as `substrate`, `product`, `ligand`

Version `0.1` does not encode full reaction semantics in the object model.
`role` is only a hint.

## Text Object

The text object represents positioned rich text content.

Example:

```json
{
  "id": "obj_text_1",
  "type": "text",
  "visible": true,
  "locked": false,
  "zIndex": 20,
  "transform": {
    "translate": [220, 88],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_text_default",
  "meta": {},
  "payload": {
    "text": "PhB(OH)2 (1.0 equiv)",
    "box": [0, 0, 120, 18],
    "align": "left",
    "valign": "middle"
  }
}
```

### Text Payload Fields

- `text`: required plain text content
- `box`: optional local text box
- `align`: `left | center | right`
- `valign`: `top | middle | bottom`
- `runs`: optional rich text runs for inline formatting

### Rich Text Support

Version `0.1` text should be able to represent:

- font family
- font size
- font weight / italic
- superscript
- subscript
- symbols and special characters

Recommended inline model:

```json
"runs": [
  {
    "text": "SO",
    "fontFamily": "Arial",
    "fontSize": 10,
    "fill": "#000000",
    "fontWeight": 700,
    "fontStyle": "normal",
    "script": "normal"
  },
  {
    "text": "4",
    "fontFamily": "Arial",
    "fontSize": 10,
    "fill": "#000000",
    "fontWeight": 700,
    "fontStyle": "normal",
    "script": "subscript"
  }
]
```

`script` is one of `normal | subscript | superscript`. Import-specific flags
such as CDXML `face`, `font`, or `color` should be decoded into these explicit
fields. Raw source values may be kept in `meta.import.cdxml` for debugging and
round-trip work.

## Molecule Fragment2D

`molecule_fragment2d` resources store nodes and bonds in local coordinates.
Fields should describe chemistry and rendering intent directly rather than
exposing source-format bit masks.

Example node label:

```json
{
  "id": "n1",
  "element": "N",
  "atomicNumber": 7,
  "position": [47.4, 29.96],
  "charge": 0,
  "numHydrogens": 0,
  "label": {
    "text": "N",
    "sourceText": "N",
    "position": [43.79, 33.86],
    "box": [43.79, 25.52, 51.01, 33.86],
    "layout": "default",
    "anchor": "start",
    "runs": [
      {
        "text": "N",
        "fontFamily": "Arial",
        "fontSize": 10,
        "fill": "#000000",
        "fontWeight": 400,
        "fontStyle": "normal",
        "script": "normal"
      }
    ]
  }
}
```

Example bonds:

```json
{
  "id": "b1",
  "begin": "n1",
  "end": "n2",
  "order": 1,
  "stereo": {
    "kind": "solid-wedge",
    "wideEnd": "end"
  }
}
```

```json
{
  "id": "b2",
  "begin": "n2",
  "end": "n3",
  "order": 2,
  "double": {
    "placement": "right"
  }
}
```

Molecule label fields:

- `text`: normalized display text
- `sourceText`: optional original label text before chemistry-oriented
  reordering
- `position`: local label point
- `box`: local label bounding box
- `layout`: label layout mode such as `default`, `attached-group`,
  `attached-group-above`, or `centered-atom`
- `anchor`: connection anchor inside the label, usually `start | center | end`
- `runs`: normalized display runs
- `lineRuns`: optional normalized runs per rendered line

Bond fields:

- `order`: numeric bond order
- `stereo.kind`: `solid-wedge | hashed-wedge`
- `stereo.wideEnd`: `begin | end`
- `double.placement`: `left | right | center`

## Line Object

The line object represents stroke-based linear geometry on the page.

It should cover:

- straight lines
- dashed lines
- polylines
- curved lines
- half arrows
- full arrows

Example:

```json
{
  "id": "obj_line_1",
  "type": "line",
  "visible": true,
  "locked": false,
  "zIndex": 15,
  "transform": {
    "translate": [0, 0],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_line_default",
  "meta": {},
  "payload": {
    "kind": "polyline",
    "points": [[260, 120], [380, 120]],
    "head": "end",
    "tail": "none",
    "arrowHead": {
      "kind": "solid",
      "head": "full",
      "tail": "none",
      "length": 18,
      "centerLength": 14,
      "width": 5
    }
  }
}
```

### Line Payload Fields

- `kind`: required geometry kind such as `line | polyline | curve`
- `points`: required control points in local coordinates
- `head`: `none | start | end | both`
- `tail`: `none | start | end | both`
- `arrowHead`: optional arrow decoration data; omitted or `null` means plain line
- `curve`: optional curve metadata for bezier or arc-like lines

Line appearance belongs primarily in styles, including:

- stroke color
- stroke width
- dash pattern
- line cap
- line join

Arrow semantics are therefore modeled as line-end decoration on the same `line`
object type, not as a separate top-level object class.

## Bracket Object

The bracket object represents standalone bracket graphics.

It should cover:

- parenthesis: `(`
- square bracket: `[]`
- curly brace: `{}`

Example:

```json
{
  "id": "obj_bracket_1",
  "type": "bracket",
  "visible": true,
  "locked": false,
  "zIndex": 12,
  "transform": {
    "translate": [0, 0],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_bracket_default",
  "meta": {
    "semanticRole": "annotation"
  },
  "payload": {
    "kind": "square",
    "side": "left",
    "box": [180, 60, 12, 80]
  }
}
```

### Bracket Payload Fields

- `kind`: `round | square | curly`
- `side`: `left | right | pair`
- `box`: required local box used to fit the bracket geometry

Brackets are document graphics in `v0.1`. If a bracket later carries polymer,
repeat-unit, or grouping semantics, that meaning should be added through
explicit metadata or future chemistry-specific objects.

## Shape Object

The shape object represents simple filled or stroked regions.

It should cover:

- `circle`
- `ellipse`
- `rect`
- `roundRect`

Example:

```json
{
  "id": "obj_shape_1",
  "type": "shape",
  "visible": true,
  "locked": false,
  "zIndex": 8,
  "transform": {
    "translate": [0, 0],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_shape_default",
  "meta": {},
  "payload": {
    "kind": "roundRect",
    "box": [420, 80, 160, 64],
    "radius": 8
  }
}
```

### Shape Payload Fields

- `kind`: `circle | ellipse | rect | roundRect`
- `box`: required local bounding box
- `radius`: optional corner radius for `roundRect`

Shape appearance belongs primarily in styles, including:

- fill color
- stroke color
- stroke width
- dash pattern
- filled vs unfilled

## Group Object

The group object organizes children but does not itself carry visible geometry.

Example:

```json
{
  "id": "obj_group_1",
  "type": "group",
  "visible": true,
  "locked": false,
  "zIndex": 5,
  "transform": {
    "translate": [0, 0],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": null,
  "meta": {
    "kind": "reaction_block"
  },
  "payload": {
    "children": ["obj_mol_1", "obj_line_1", "obj_text_1"]
  }
}
```

### Group Payload Fields

- `children`: required ordered list of child object ids

Children inherit the group transform.

## Group Semantics

In `v0.1`, `group` is intentionally narrow.

- A `group` organizes ownership and shared transform
- A `group` does not create a separate stacking context
- A `group` does not decide front/back visibility for overlaps
- A `group` is not a layer
- A `group` does not need visible geometry of its own
- Top-level `objects` should contain only root objects with no parent group

This keeps grouping and overlap handling separate.

## Document Section

The `document` section stores global metadata and page settings.

Example:

```json
"document": {
  "id": "doc_001",
  "title": "example reaction page",
  "page": {
    "width": 1024,
    "height": 768,
    "background": "#ffffff"
  },
  "meta": {
    "createdBy": "chemcore",
    "sourceFormat": "cdxml"
  }
}
```

### Document Fields

- `id`: document id
- `title`: optional title
- `page.width`: required
- `page.height`: required
- `page.background`: optional
- `meta`: optional general metadata

## Example Rendering Order

Objects are painted by:

1. ascending `zIndex`
2. stable sibling order as tiebreaker

Later-painted objects appear in front of earlier-painted objects where they
overlap.

Groups do not replace child ordering; they only scope transforms and ownership.

## Overlap and Stacking

If two visible objects partially overlap, front/back display order is determined
only by stacking order, never by object type or overlap area.

Rules:

- Higher `zIndex` objects appear in front of lower `zIndex` objects
- If two objects have the same `zIndex`, later sibling order appears in front
- Rendering is defined as ordered painting; later paint covers earlier paint
- `group` membership does not change these rules

## Constraints for v0.1

Version `0.1` intentionally does not include:

- multiple pages
- embedded binary assets
- native reaction graph semantics
- query chemistry semantics
- editing history
- viewport state
- selection state
- collaborative metadata

Those belong in future versions once the base model is proven.

## File Extension

For now, the recommended extension is:

- `.chemcore.json`

This makes it obvious that:

- the payload is JSON
- the schema is still evolving

## Compatibility Promise

Version `0.1` is an unstable development format.

The current promise is:

- fields should be explicit
- ids should remain stable once generated
- migration should be possible by versioned transforms

Backward compatibility is not guaranteed yet.
