# Label Semantics

Label recovery is not string-specific topology. Recover atom sites from geometry,
then use ChemCore's label engine to interpret text.

## OCR Text Role

The recognizer may provide candidate strings, font style, and confidence. It
must not decide topology, atom sites, or molecule boundaries.

Prioritize high-quality printed-text recognition for common chemistry fonts
such as Arial and Times New Roman, including bold and italic. If the recognizer
misses bold/italic or simple printed symbols, treat it as a recognizer quality
problem, not as acceptable noise.

## Source Text And Visible Text

Use `chemcore-cli label-query --visible-text` for OCR reverse mapping:

- visible `F3C` can map to source `CF3`
- visible `H2N` can map to source `NH2`
- visible `MeO` can map to source `OMe`
- visible strings that should not chemically rewrite should use
  `defaultChemical:false`

Do not hard-code these cases in OCR when the engine can answer.

## Plain Text Chemical-Looking Labels

Some source drawings intentionally show chemical-looking text without enabling
default chemical interpretation. Preserve this using `defaultChemical:false`.
Display reversal can still matter; the point is to preserve visible author
layout while avoiding generated chemistry semantics such as implicit hydrogens.

## Anchors

For terminal labels:

- If the bond reaches a label, measure the true atom site along the bond
  centerline and connection character center.
- If two bonds can be extended to intersect, use their extension intersection.
- If a ring or branch provides endpoint-pair support, use the endpoint from
  stroke geometry.
- Attach the label at that atom site through ChemCore.

Do not move the atom site to the glyph center, glyph side, or text bounding box.

## Special Bond Families

Recognize wavy, wedge, hashed wedge, and aromatic crossing bonds early when
their contour evidence is strong. Their rule should be based on the ChemCore
rendering pattern and stroke geometry, not label names or color.
