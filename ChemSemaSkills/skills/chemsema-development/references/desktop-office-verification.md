# Desktop And Office Verification

## Desktop/WASM

When the task asks to update the desktop app:

1. Rebuild WASM with the current engine.
2. Build the desktop package.
3. Replace the package used by the Start Menu shortcut if requested.
4. Launch the app and verify the affected workflow.
5. Check browser/desktop console logs when UI behavior is involved.

If the desktop app is open and locks files, close it before packaging.

## Interaction Checks

For hover/focus/tool behavior:

- test default style and ACS style if style affects hit testing
- test atom hover, bond hover, endpoint hover, and focused clicks
- test keyboard modifiers and active tool changes
- clear hover after creating an object, but preserve normal hover behavior
  before and during the interaction

## Office

For copy/paste or OLE changes:

```powershell
chemsema-cli copy input.cdxml --target all --payload payload.json --no-copy --pretty
chemsema-cli copy input.cdxml --target all --payload live-payload.json --pretty
```

Then paste into Word or PowerPoint and verify:

- all selected objects are visible
- object is editable as ChemSema when expected
- preview matches capture
- payload includes the same selected object count as `targets`
