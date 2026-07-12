# Object-Grounded Edit

This workflow shows a bounded agent edit on the public `figure1.cdxml`
fixture. It treats one molecule object as the editable work unit, inspects the
local context, changes one labeled node from `Me` to `OMe`, and verifies the
result with a dry-run transaction, structured diff, before/after captures, and
editable exports.

Run from the repository root:

```powershell
examples/agent/07-object-grounded-edit/run.ps1
```

Generated outputs include:

- `bundle/manifest.json`, `bundle/identity-map.json`, and `bundle/provenance.json`
- `before.png` and `after.png`
- `dry-run-report.json` and `execute-report.json`
- `diff.json`
- `target-subset.ccjs` and `target-subset.cdxml`
- `output.ccjs` and `output.cdxml`
- `acceptance.json`
