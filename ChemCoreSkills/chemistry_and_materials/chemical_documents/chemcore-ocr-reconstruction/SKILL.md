---
name: chemcore-ocr-reconstruction
description: Reconstruct editable ChemCore molecule JSON and command streams from raster chemical drawings. Use for ChemCore OCR, PNG-to-CCJS, molecule-pool regression, structure gates, label chemistry, wedge/hash/wavy bond recognition, ring recovery, topology equivalence, compare image generation, and failure-taxonomy driven OCR development.
---

# ChemCore OCR Reconstruction

## Core Rule

Reconstruct a ChemCore document, not pixels. The primary gate is structural
agreement of the ChemCore molecule fragment: topology, labels/source text, bond
orders, bond variants, stereochemistry, double-bond placement, and label
attachment. Rendered comparison images are secondary diagnostics.

Before changing rules, read the relevant references:

- `references/reconstruction-principles.md` for the development contract.
- `references/structure-gate.md` for pass/fail semantics.
- `references/molecule-pool.md` for pool extraction and batch gates.
- `references/label-semantics.md` for labels, anchors, reversal, hydrogens, and
  plain text preservation.
- `references/failure-taxonomy.md` for error buckets and triage.

## Work Loop

1. Inspect the ground-truth JSON before looking at rendered images.
2. Run the molecule or pool gate and save `structure-metrics.json`.
3. Classify the first failure with the taxonomy.
4. Fix one structural cause, not one visible symptom.
5. Re-run the focused molecule.
6. Re-run the active group and the representative repair set.
7. Regenerate the current comparison directory.
8. Record regressions only when a previously passing molecule breaks after an
   unrelated fix.

Use the helper for summaries:

```powershell
python scripts\summarize_structure_metrics.py path\to\gate-output --json
```

## Guardrails

- Do not mark a molecule passed from visual similarity alone.
- Do not use raw color as a structure rule; chemical drawings often color
  arbitrary substructures.
- Do not special-case a label string to move topology. Recover atom sites from
  bond geometry, then attach labels through ChemCore.
- Do not use `plan-bond` for OCR bond length snapping. Measure the raster.
- Prefer structural rules over new thresholds. If a threshold is unavoidable,
  derive it from local scale, stroke width, text height, or measured bond
  length and record the effective value in metrics.
- Use ChemCore `label-query --visible-text` before inventing formula reversal
  rules.
