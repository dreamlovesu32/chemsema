# ChemSema Capture Manifest v1

Protocol id: `chemsema-cli-capture-manifest.v1`.

`chemsema-cli capture` and session `capture` return a JSON manifest describing
the visual output file.

## Stable Fields

- `ok`
- `input`
- `target`
- `warnings`
- `output`
- `bounds`
- `viewBox`
- `expansion`
- `render`

`output` includes:

- `path`
- `format`
- `defaulted`
- `verified`
- `bytes`
- `pixelSize`

`render` includes:

- `mode`
- `primitiveCount`
- `targets`

## Output Rules

PNG/SVG image data is written to `--out` or to the default temp path. Stdout is
reserved for the JSON manifest. If `--out` is omitted, capture writes a PNG into
the OS temp `chemsema-cli` directory and emits a `default_output_path` warning.

PNG resolution defaults to `--scale 10`. Use `--scale`, `--width`, or `--height`
for agent inspection crops that need higher resolution or bounded output size.
