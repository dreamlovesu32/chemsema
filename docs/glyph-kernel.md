# Glyph Kernel

## Purpose

`chemcore` needs a host-independent text geometry kernel for chemical labels.

The browser should not be the authority for:

- per-glyph ink bounds
- glyph advance widths
- subscript / superscript scaling and baseline shifts
- background padding used for knockout and bond clipping

If hosts derive geometry independently, web and desktop renderers will drift.

## Kernel Model

The first native kernel lives under:

- [cpp/chemcore_glyph_kernel](../cpp/chemcore_glyph_kernel)

It defines geometry from built-in normalized glyph profiles:

- each glyph profile stores `advance_em`
- ink bounds are normalized to font size
- padding is normalized to font size
- shape kind is owned by the kernel: `rect` or `ellipse`
- the initial ASCII chemical-label profiles are generated from one reference font, then stored as host-independent constants

This means:

- host zoom does not change logical geometry
- larger font size automatically produces larger background padding
- web and desktop can consume the same placements and shapes

## Current API

The kernel exposes:

- a C++ API in [glyph_kernel.hpp](../cpp/chemcore_glyph_kernel/include/chemcore/glyph_kernel.hpp)
- a stable C ABI in [glyph_kernel.h](../cpp/chemcore_glyph_kernel/include/chemcore/glyph_kernel.h)

Current output per glyph includes:

- baseline position
- font size after script scaling
- advance width
- ink box
- background box
- final shape geometry

The kernel also computes a run-level anchor point for label placement:

- callers may pass an anchor glyph index returned by the frontend
- if no anchor glyph index is provided, the first visible glyph is used
- both uppercase and lowercase glyphs can be anchor glyphs
- anchor `x` uses that glyph's background horizontal center
- anchor `y` uses the standard uppercase glyph center line, not the selected glyph's actual height

Aligned layout is supported for labels where the attached atoms must move around the anchor glyph:

- `right` / `left`: preserve the frontend-provided glyph order; the anchor glyph is translated to the requested anchor origin
- left-side labels such as `O2S` must be passed in that order with the `S` glyph selected as the anchor
- `above`: non-anchor glyphs are placed above the anchor glyph; the first non-anchor glyph is x-aligned with the anchor glyph
- `below`: non-anchor glyphs are placed below the anchor glyph; the first non-anchor glyph is x-aligned with the anchor glyph

## Preview and Verification

The SVG demo can be generated with:

```bash
./build/cpp/chemcore_glyph_kernel/chemcore_glyph_svg_demo
python3 scripts/glyph_kernel_reference.py render
```

The reference checker re-measures the demo glyphs with the same reference font and fails if glyph ink escapes the kernel-produced background shape:

```bash
python3 scripts/glyph_kernel_reference.py check
```

When the local Python environment has Pillow and a usable reference font, this check is also registered in `ctest`.

## Scope of v1

The current kernel intentionally targets the label-geometry slice first:

- glyph profile registry
- run layout with normal / subscript / superscript
- scalable padding
- deterministic rect / ellipse output

It does not yet include:

- full font shaping
- Unicode-wide profile coverage
- molecule-wide collision routing
- Python bindings

## Web Viewer Binding

The web viewer consumes the same C ABI through Emscripten:

- [viewer/glyph_kernel_runtime.js](../viewer/glyph_kernel_runtime.js) wraps the wasm module.
- [viewer/chemcore_glyph_kernel.js](../viewer/chemcore_glyph_kernel.js) and `viewer/chemcore_glyph_kernel.wasm` are generated artifacts.
- [viewer/app.js](../viewer/app.js) renders molecule labels from kernel placements and uses kernel background shapes for label debug and wedge retreat.

Build the web binding with:

```bash
npm run build:glyph-wasm
```

The old browser measurement implementation has been removed. The viewer no longer calls `getExtentOfChar`, `getBBox`, canvas glyph scanning, or the previous JS optical-contact modules for chemical label geometry.
