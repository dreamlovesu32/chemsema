# Label Query

Use `label-query` whenever source text, visible text, label reversal, chemical
validity, generated hydrogens, anchors, or plain-text preservation matters.

## Forward Query

Use `--text` when ChemCore receives source text:

```powershell
chemcore-cli label-query --text CF3 --connection-angle 0 --pretty
chemcore-cli label-query --text NH2 --connection-angle 180 --connection-count 1 --pretty
```

The response reports accepted status, displayed text, source runs, recognition
metadata, semantic anchor atom, generated hydrogens, and whether default display
differs from source text.

## Reverse Query

Use `--visible-text` for imported drawings:

```powershell
chemcore-cli label-query --visible-text F3C --connection-angle 0 --pretty
chemcore-cli label-query --visible-text H2N --connection-angle 180 --pretty
```

ChemCore proposes source labels that would render back to the visible text. If
no chemical candidate validates and renders back to the visible string, prefer
the recommended `defaultChemical:false` candidate to preserve the source drawing
instead of forcing chemical rewriting.

## Plain Text Preservation

Use `--no-default-chemical` or command fields equivalent to
`defaultChemical:false` when a source document intentionally displays a string
that ChemCore's chemical label engine would otherwise rewrite. This is not a
label special case; it is preserving author intent.

Examples:

- A visible `CF3` on the side where default rendering would show `F3C`.
- A visible `NH2` where valence context would normally hide or alter hydrogens.
- A printed abbreviation that is not meant to be a chemical group.

## Geometry Boundary

Generated hydrogens are layout glyphs, not topology anchors, unless the visible
label is standalone `H`. Recover atom sites from bond endpoints, centerline
extension/intersection, endpoint-pair stroke support, or template centerlines.
Then attach labels through ChemCore label commands.
