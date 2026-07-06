# Failure Taxonomy

Use these buckets in structure metrics and pool summaries.

## Primitive And Geometry

- `primitive_missing_segment`
- `primitive_extra_segment`
- `junction_intersection_missed`
- `endpoint_pair_mismatch`
- `coordinate_drift`
- `bond_length_mismeasured`
- `ring_regularization_needed`
- `ring_regularization_wrong`

## Topology

- `missing_node`
- `extra_node`
- `duplicate_node`
- `missing_bond`
- `extra_bond`
- `wrong_attachment`
- `fragment_split_wrong`
- `fragment_merge_wrong`

## Bond Semantics

- `bond_order_mismatch`
- `double_placement_mismatch`
- `wedge_variant_mismatch`
- `wedge_direction_mismatch`
- `hashed_wedge_mismatch`
- `wavy_bond_mismatch`
- `aromatic_or_ring_crossing_mismatch`

## Labels

- `text_recognition_error`
- `label_source_mismatch`
- `visible_text_mismatch`
- `label_style_mismatch`
- `label_anchor_mismatch`
- `label_plain_text_flag_mismatch`
- `implicit_hydrogen_mismatch`
- `charge_or_valence_mismatch`

## System

- `truth_import_split_bug`
- `chemcore_render_retreat_bug`
- `unsupported_object_family`
- `gate_bug`
- `ocr_debug_artifact_leaked`

Prefer fixing the highest structural cause first. A visible render defect can be
filed as render-only only after the internal structure is correct.
