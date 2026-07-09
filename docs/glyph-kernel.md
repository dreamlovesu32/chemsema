# Glyph Kernel

## Purpose

`chemcore` needs host-independent text geometry for chemical labels.

The Rust engine owns:

- per-glyph label geometry used by bond clipping
- glyph advance estimates
- subscript / superscript scaling and baseline shifts
- background padding used for knockout and label-aware bond retreat

If hosts derive these details independently, web and desktop renderers will drift.

## Current Model

The active glyph geometry implementation lives in Rust:

- [crates/chemcore-engine/src/glyph_kernel.rs](../crates/chemcore-engine/src/glyph_kernel.rs)

The Rust engine now consumes two shared manifests:

- [shared/glyph_profiles.json](../shared/glyph_profiles.json)
- [shared/glyph_clip_polygons.json](../shared/glyph_clip_polygons.json)
- [shared/text_symbols.json](../shared/text_symbols.json) lists the text-symbol
  catalog used by the viewer palette and by the profile generation script

`glyph_profiles.json` remains the source of normalized text layout metrics:

- normalized glyph advances
- normalized ink bounds
- conservative background padding for label layout metrics
- normal / subscript / superscript layout
- conservative Unicode-category fallbacks for characters missing from the shared
  profile manifest

`glyph_clip_polygons.json` is now the only runtime source of label clipping geometry.
The previous runtime `rect / ellipse / cut-corner / petal` polygon synthesis has
been removed from the Rust kernel.

The output is used by attached-label layout, label anchor geometry, label-aware bond clipping, and text edit preview geometry.

## Fixed Clipping Rules

The current clipping scheme is intentionally data-driven and deterministic:

1. Layout still starts from the normalized ink box in `glyph_profiles.json`.
2. The actual clipping polygon is loaded from `glyph_clip_polygons.json`.
3. ASCII uppercase letters use precomputed polygons built from:
   - canonical natural outline dilation: `1.0pt` at the `10pt` reference font size
   - inward anchor offset: `0.22 * glyph height`
   - canonical anchor circle radius: `2.0pt` at the `10pt` reference font size
4. Non-uppercase symbols use natural-outline dilation only:
   - canonical natural outline dilation: `1.0pt` at the `10pt` reference font size
5. Runtime label clipping remaps the outside dilation to the document source
   margin as an absolute pt value. For CDXML import, natural dilation equals
   `MarginWidth` and anchor circle radius equals `2 * MarginWidth`; neither value
   scales with the actual label font size.
6. Unknown visible characters missing from the clip manifest are manifest
   generation failures; runtime clipping does not synthesize replacement shapes.

The detailed uppercase anchor rules are documented in:

- [docs/glyph-clip-polygons.md](./glyph-clip-polygons.md)

## Manifest Generation

The clipping manifest is generated:

```bash
python scripts/generate-glyph-profiles.py
python scripts/generate-glyph-clip-polygons.py
```

The current clip manifest is generated from `Arial` outline geometry and locked to
the canonical point values above. Runtime renderers consume these precomputed
petal/corner rules and remap their outside dilation from the canonical source
margin to the document source margin.

## Consumer Chain

The same glyph polygons now flow through the whole stack:

- `chemcore-engine` Rust kernel builds glyph polygons:
  - [crates/chemcore-engine/src/glyph_kernel.rs](../crates/chemcore-engine/src/glyph_kernel.rs)
- label-aware bond clipping uses those polygons directly:
  - [crates/chemcore-engine/src/render/labels.rs](../crates/chemcore-engine/src/render/labels.rs)
- document knockouts use the same polygons:
  - [crates/chemcore-engine/src/render_objects.rs](../crates/chemcore-engine/src/render_objects.rs)
- Office / EMF preview replays the engine polygons through the same glyph
  clipping algorithm:
  - [apps/chemcore-office/src/windows_office/emf_preview/renderer.rs](../apps/chemcore-office/src/windows_office/emf_preview/renderer.rs)

This means kernel clipping, SVG/document knockouts, and EMF preview now share one
geometry source.

## Web Status

The web viewer consumes Rust engine state and render primitives through WASM:

- [crates/chemcore-engine/src/wasm.rs](../crates/chemcore-engine/src/wasm.rs)
- [viewer/app.js](../viewer/app.js)

The old C++ glyph kernel and standalone glyph WASM path have been removed. Current validation should go through the Rust engine tests and viewer engine WASM build.
