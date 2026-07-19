# Formats And Conversion

Use `convert` for editable document conversion and rendered exports. Use
`export` as the export-oriented alias when the intent is image/vector output.

## Commands

```powershell
chemsema-cli convert input.cdxml output.ccjs
chemsema-cli convert input.ccjs output.cdxml
chemsema-cli convert input.cdxml output.svg
chemsema-cli convert input.cdxml output.png --scale 6
chemsema-cli export input.cdxml output.png --width 1800
chemsema-cli convert input.ccjs molecule-1.cdxml --target molecule:1
chemsema-cli export input.ccjs selected.ccjs --targets "object:obj_a;object:obj_b"
```

Use `--format <format>` when the output extension is ambiguous:

```powershell
chemsema-cli convert input.cdxml output --format svg
chemsema-cli export input.cdxml output --format png --width 1800
```

## Runtime Formats

Read the current format contract from:

```powershell
chemsema-cli capabilities --out capabilities.json --pretty
```

As of protocol v1, common editable inputs include `ccjs`, `ccjz`, `cdxml`,
`cdx`, and `sdf`. Document outputs include `json`, `ccjs`, `ccjz`, `cdxml`,
`cdx`, `sdf`, `svg`, and `png`. Capture output includes `svg` and `png`.

## Guardrails

- Use `capture` when the target is a visual bounds crop.
- Use `convert` or `export` when the target is the whole input document or an
  editable target subset.
- For editable subset export, use `--target <selector>` for one object,
  molecule, node, or bond. Use repeated `--target` or `--targets
  "object:a;object:b"` for multi-object/multi-molecule selection. Discover
  selectors with `targets` first.
- For PNG, specify `--scale`, `--width`, or `--height` when deterministic pixel
  dimensions matter.
- For structural comparisons, prefer `ccjs`/JSON over rendered SVG/PNG.
