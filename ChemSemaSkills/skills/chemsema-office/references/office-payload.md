# Office Payload

ChemSema Office paste depends on a payload that can contain:

- `windows-office-ole` clipboard output
- `chemsema-payload-json` debug payload output
- ChemSema internal document JSON or selected fragment JSON
- CDXML/CDX-compatible data when available
- SVG preview or vector data
- EMF preview for Office rendering
- Unicode text fallback
- Object Descriptor / Embed Source metadata for OLE
- ChemSema custom clipboard format data

## Expected Properties

The payload should preserve:

- all selected scene objects
- molecule resources and node/bond topology
- text and labels with style where supported
- previews that match the selected bounds
- enough metadata for later editable roundtrip

## Debug Rules

- `--payload <path> --no-copy` is the safest first diagnostic.
- Payload generation should be deterministic for the same input and target.
- The editable object path should take priority over `CF_METAFILEPICT` in Word.
- When investigating a missed object, compare `targets`, `detail`, payload
  object counts, and preview crops in that order.
