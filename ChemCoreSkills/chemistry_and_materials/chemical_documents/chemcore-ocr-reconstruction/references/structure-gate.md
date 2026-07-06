# Structure Gate

The structure gate decides whether candidate ChemCore JSON is equivalent enough
to the ground truth molecule fragment.

## Must Match

- molecule object count for the selected crop
- graph connectivity
- node element, source label, and visible label semantics
- bond order
- bond variant: plain, wedge, hashed wedge, wavy, aromatic, etc.
- stereochemical direction when represented
- double-bond placement: left, right, center, or equivalent automatic result
- label attachment site
- standalone text vs atom/group label boundary
- font style when the source contains chemical label text with bold/italic
  styling and ChemCore stores that style

## May Differ

- node ids
- bond ids
- atom ordering
- bond ordering
- command order
- tiny coordinate differences within configured tolerance
- non-structural debug objects

## Must Not Hide

- large coordinate drift
- wrong bond length caused by measuring to the wrong glyph point
- wrong label side or anchor atom
- missing or extra atom sites
- missing ring edges
- wrong wedge direction
- a visual-only pass when internal JSON is wrong

## Recommended Metrics

Record these in `structure-metrics.json`:

- `structure_status`
- `structure_isomorphic`
- `differences[]`
- `failure_taxonomy[]`
- `node_mapping`
- `bond_mapping`
- `coordinate_rms`
- `max_node_distance`
- `max_bond_endpoint_distance`
- `label_style_mismatches`
- measured scale: text height, stroke width, expected bond length, tolerances

The rendered comparison directory is for human review, but the gate decision
must come from structure metrics.
