# Templates And Style

## Templates

Use templates for standard rings and chemical motifs. A drawing agent should not
hand-compute benzene, cyclopentane, or fused-ring coordinates when ChemCore can
return template vertices.

For polygon-like tools, clicking a focused bond should reuse that bond when the
engine supports it, placing the polygon on the side that avoids overlap.

## Bond Style

Use ChemCore defaults unless the user asks for a specific style:

- order: single, double, triple as needed
- variant: plain, wedge, hashed wedge, wavy, dashed
- double placement: omit for automatic; set only when the intended structure
  requires center/left/right

## Label Style

Use template style defaults for ordinary drawing. When matching an existing
document, copy visible font family, size, bold, italic, and color where ChemCore
stores them.

For chemical labels, call `label-query` instead of deciding reversal manually.
