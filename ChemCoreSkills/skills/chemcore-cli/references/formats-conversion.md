# Formats And Conversion

Use `convert` for editable document conversion and rendered exports. Use
`export` as the export-oriented alias when the intent is image/vector output.

## Commands

```powershell
chemcore-cli convert input.cdxml output.ccjs
chemcore-cli convert input.ccjs output.cdxml
chemcore-cli convert input.cdxml output.svg
chemcore-cli convert input.cdxml output.png --scale 6
chemcore-cli export input.cdxml output.png --width 1800
```

Use `--format <format>` when the output extension is ambiguous:

```powershell
chemcore-cli convert input.cdxml output --format svg
chemcore-cli export input.cdxml output --format png --width 1800
```

## Runtime Formats

Read the current format contract from:

```powershell
chemcore-cli capabilities --out capabilities.json --pretty
```

As of protocol v1, common editable inputs include `ccjs`, `ccjz`, `cdxml`,
`cdx`, and `sdf`. Document outputs include `json`, `ccjs`, `ccjz`, `cdxml`,
`cdx`, `sdf`, `svg`, and `png`. Capture output includes `svg` and `png`.

## Guardrails

- Use `capture` when the target is a crop or selection.
- Use `convert` or `export` when the target is the whole input document.
- For PNG, specify `--scale`, `--width`, or `--height` when deterministic pixel
  dimensions matter.
- For structural comparisons, prefer `ccjs`/JSON over rendered SVG/PNG.
