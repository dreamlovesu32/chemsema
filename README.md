# chemcore

`chemcore` is a cross-platform chemistry document core.

The project goal is not "a web demo first, then a desktop rewrite later". The
goal is to define a stable document core from day one:

- a platform-independent document model
- a stable file format
- a runtime scene model suitable for editing and rendering
- import paths from legacy tools such as CDXML
- renderer backends for web and desktop hosts

The first implementation focus is still narrow:

- define the boundary of the format
- make it readable and writable
- make it renderable
- keep CDXML parsing as an import path

## Native Glyph Kernel

The project now also contains a first C++ text-geometry kernel under
[`cpp/chemcore_glyph_kernel`](./cpp/chemcore_glyph_kernel).

This kernel is intended to become the shared authority for:

- per-glyph chemical label geometry
- script scaling and baseline shifts
- scalable label background padding
- host-independent rect / ellipse knockout shapes

Minimal build commands:

```bash
cmake -S . -B build
cmake --build build
ctest --test-dir build
./build/cpp/chemcore_glyph_kernel/chemcore_glyph_demo
```

The visual preview can be regenerated with:

```bash
./build/cpp/chemcore_glyph_kernel/chemcore_glyph_svg_demo
python3 scripts/glyph_kernel_reference.py render
```

## Current Scope

Current code under [`src/chemcore/cdxml`](./src/chemcore/cdxml) provides the
first import-side foundation:

- CDXML extraction entrypoint: `extract_cdxml`
- molecule extraction from CDXML geometry
- text / table / arrow extraction
- SDF matching to enrich molecules with `smiles` and `molblock2d`
- stereo post-processing for 2D structures

This is intentionally only the parsing side. It is not yet a chemcore
renderer, editor, or final file serializer.

## Design Documents

The current design baseline lives in:

- [docs/architecture.md](./docs/architecture.md)
- [docs/format-v0.1.md](./docs/format-v0.1.md)
- [docs/project-rules.zh-CN.md](./docs/project-rules.zh-CN.md)
- [docs/viewer-rendering-report.zh-CN.md](./docs/viewer-rendering-report.zh-CN.md)
- [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)
- [README.zh-CN.md](./README.zh-CN.md)
- [docs/architecture.zh-CN.md](./docs/architecture.zh-CN.md)
- [docs/format-v0.1.zh-CN.md](./docs/format-v0.1.zh-CN.md)
- [examples/document-v0.1.json](./examples/document-v0.1.json)

These documents are the working contract for the next step:

- convert imported data into the `chemcore` document model
- render that model in a first backend
- validate round-trip behavior before editing features are added

## Workspace Layout

```text
chemcore/
  README.md
  README.zh-CN.md
  docs/
    architecture.md
    architecture.zh-CN.md
    format-v0.1.md
    format-v0.1.zh-CN.md
  examples/
    document-v0.1.json
  src/
    chemcore/
      __init__.py
      cdxml/
        __init__.py
        extract_cdxml.py
        cdxml_layout.py
        cdxml_molecule.py
        cdxml_sdf_match.py
        cdxml_shared.py
        cdxml_stereo.py
```

## Conda Environment

Environment name: `chemcore`

Recommended creation command:

```bash
conda create -y -n chemcore python=3.11 rdkit -c conda-forge
```

Activate it with:

```bash
conda activate chemcore
```

## Minimal Usage

From `/home/jiajun/chemcore`:

```bash
PYTHONPATH=src python -c "from chemcore import extract_cdxml; print(extract_cdxml('/path/to/base_without_cdxml_suffix'))"
```

`extract_cdxml()` expects the base path without the `.cdxml` suffix, and the
matching `.sdf` file beside it, mirroring the current importer pipeline.

## Near-Term Plan

1. map extracted CDXML objects into the `chemcore` v0.1 document model
2. build a first read-only renderer backend against that model
3. verify object identity, coordinates, style references, and z-order
4. only then start minimal editing operations
