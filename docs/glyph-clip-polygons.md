# Glyph Clipping Rules

## Goals

This document fixes ChemCore glyph clipping rules so that the kernel, SVG, EMF, and Word/OLE use one shared geometry definition.

The only valid runtime source of clipping geometry is:

- [shared/glyph_clip_polygons.json](../shared/glyph_clip_polygons.json)

The generation script is:

- [scripts/generate-glyph-clip-polygons.py](../scripts/generate-glyph-clip-polygons.py)

Runtime consumers are:

- [crates/chemcore-engine/src/glyph_kernel.rs](../crates/chemcore-engine/src/glyph_kernel.rs)
- [crates/chemcore-engine/src/render/labels.rs](../crates/chemcore-engine/src/render/labels.rs)
- [crates/chemcore-engine/src/render_objects.rs](../crates/chemcore-engine/src/render_objects.rs)
- [apps/chemcore-office/src/windows_office/emf_preview/renderer.rs](../apps/chemcore-office/src/windows_office/emf_preview/renderer.rs)

## Geometry Source Boundary

Clipping geometry is defined by normalized polygons in `glyph_clip_polygons.json`. The following names are only historical metadata or test-identification markers:

- `ellipse`
- `rect-cut-*`
- `petal-*`
- `convex_hull`
- circular sampling concatenation

The `shape` field in `shared/glyph_profiles.json` is metadata. Clipping geometry is governed by `glyph_clip_polygons.json`.

## Clipping Rules

### 1. Layout Baseline

Layout metrics use values from `glyph_profiles.json`:

- `advanceEm`
- `inkLeftEm / inkTopEm / inkRightEm / inkBottomEm`
- script scaling and baseline shift

Ordinary text layout metrics are defined by `glyph_profiles.json`; `glyph_clip_polygons.json` only defines geometry for bond clipping and knockout.

### 2. Uppercase Letters

Clipping polygons for ASCII uppercase letters `A-Z` are generated offline by this fixed process:

1. Start from real `Arial` glyph outlines.
2. Apply the canonical natural dilation: `1.0pt` at the `10pt` reference font size.
3. Collect glyph anchor points.
4. Offset anchors inward uniformly by `0.22 * glyph height`.
5. Add circular reinforcement centered on the offset point, with radius `2.0pt` at the `10pt` reference font size.
6. Take the union of the natural dilation region and the circular reinforcement region.
7. Discretize the result into normalized polygons and write them to `shared/glyph_clip_polygons.json`.

At runtime, the inside of the normalized glyph contour follows the measured glyph
box, but the outside dilation is remapped from the manifest's canonical `1.0pt`
to the document source margin. For CDXML import, natural dilation is exactly the
source `MarginWidth` in absolute pt, and circular reinforcement radius is exactly
`2 * MarginWidth`. These values do not scale with the label font size.
The source margin remap is a polygon-level natural outset, so internal stroke
bays and stroke ends inside the overall glyph bounding box expand by the same
source `MarginWidth` as exterior edges.

### 3. Other Symbols

Visible characters other than ASCII uppercase letters use uniform natural dilation:

- canonical natural dilation: `1.0pt` at the `10pt` reference font size, remapped
  at runtime to the document source margin in absolute pt

### 4. Manifest Completeness

Every visible character listed in `glyph_profiles.json` must have a generated polygon in `glyph_clip_polygons.json`. Runtime label clipping does not synthesize replacement geometry for a missing visible glyph; missing coverage is a manifest-generation/test failure.

## Uppercase Anchor Table

Notation:

- `point(c0, i)`: use real on-curve vertex `i` from contour 0
- `midpoint(c0, i, j)`: use the midpoint of real on-curve vertices `i` and `j`
- `M/W` explicitly exclude the middle valley point group and use only the four outer contour points
- sampled points on circles or arcs are not vertices; only real glyph contour vertices participate

| Letter | Anchor rule |
| --- | --- |
| `A` | `midpoint(c0,1,2)`, `point(c0,0)`, `point(c0,3)` |
| `B` | `point(c0,1)`, `point(c0,0)` |
| `C` | no circular reinforcement |
| `D` | `point(c0,1)`, `point(c0,0)` |
| `E` | `point(c0,1)`, `point(c0,2)`, `point(c0,0)`, `point(c0,11)` |
| `F` | `point(c0,1)`, `point(c0,2)`, `point(c0,0)` |
| `G` | no circular reinforcement |
| `H` | `point(c0,1)`, `point(c0,6)`, `point(c0,0)`, `point(c0,7)` |
| `I` | `midpoint(c0,1,2)`, `midpoint(c0,0,3)` |
| `J` | `midpoint(c0,9,10)` |
| `K` | `point(c0,1)`, `point(c0,5)`, `point(c0,7)`, `point(c0,0)` |
| `L` | `point(c0,1)`, `point(c0,0)`, `point(c0,5)` |
| `M` | `point(c0,1)`, `point(c0,9)`, `point(c0,0)`, `point(c0,10)` |
| `N` | `point(c0,1)`, `point(c0,5)`, `point(c0,0)`, `point(c0,6)` |
| `O` | no circular reinforcement |
| `P` | `point(c0,1)`, `point(c0,0)` |
| `Q` | `midpoint(c0,2,3)` |
| `R` | `point(c0,1)`, `point(c0,0)`, `point(c0,14)` |
| `S` | no circular reinforcement |
| `T` | `midpoint(c0,2,3)`, `midpoint(c0,4,5)`, `midpoint(c0,0,7)` |
| `U` | `midpoint(c0,11,12)`, `midpoint(c0,0,1)` |
| `V` | `point(c0,1)`, `point(c0,9)`, `midpoint(c0,0,10)` |
| `W` | `point(c0,1)`, `point(c0,16)`, `point(c0,0)`, `point(c0,17)` |
| `X` | `point(c0,2)`, `point(c0,10)`, `point(c0,0)`, `point(c0,12)` |
| `Y` | `point(c0,2)`, `point(c0,10)`, `midpoint(c0,0,12)` |
| `Z` | `point(c0,6)`, `point(c0,7)`, `point(c0,0)`, `point(c0,12)` |

## Consumption Constraints

### 1. Runtime Clipping Geometry Source

Runtime code must not synthesize clipping polygons on the fly from tags such as:

- `petal-nehkxz`
- `petal-a`
- `ellipse`
- `rect-cut-*`

### 2. EMF Geometry Source

EMF/Office preview must directly consume glyph polygons computed by the engine.

- The kernel generates clipping polygons.
- EMF only replays these polygons.
- The EMF layer must not derive an independent glyph clipping model.

### 3. Ordinary Text Display Boundary

Clipping polygons and text metrics are separate:

- Text layout, advance, and baseline shift continue to come from `glyph_profiles.json`.
- Bond clipping and knockout geometry comes from `glyph_clip_polygons.json`.

Changes to clipping rules must not change ordinary text layout metrics.

## Regeneration

When updating glyph clipping rules, the order is fixed:

1. Modify [scripts/generate-glyph-clip-polygons.py](../scripts/generate-glyph-clip-polygons.py)
2. Regenerate [shared/glyph_clip_polygons.json](../shared/glyph_clip_polygons.json)
3. Run Rust tests
4. Validate SVG / EMF / Word copy-paste paths

Glyph clipping definitions accept only offline-generated normalized polygons.
