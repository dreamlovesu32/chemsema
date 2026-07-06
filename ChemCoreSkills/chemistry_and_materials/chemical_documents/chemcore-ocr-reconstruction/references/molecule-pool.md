# Molecule Pool

Use a molecule pool to prevent overfitting a few focused crops.

## Pool Extraction

For each source document:

1. Run `targets`.
2. Select molecule targets in deterministic order.
3. Capture each molecule with consistent scale/width and expansion.
4. Save detail JSON with `--include-resource`.
5. Save a manifest tying crop id, source file, selector, detail path, and image
   path.

For multi-molecule source objects, split the target as cleanly as the ChemCore
importer permits. If importer molecule splitting is wrong, fix the engine or
import semantics instead of teaching OCR to treat a giant grouped target as one
chemical molecule.

## Gate Strategy

Useful pools:

- `dev_first100`
- `easy_100`
- `labels_100`
- `stereo_100`
- `rings_100`
- `random_500`
- `holdout_100`

Early work can use first-N order. After rule stabilization, add random and
stratified pools with a seed.

## Compare Folder

Maintain one current comparison root with one image per molecule. Do not keep
timestamped evolution history unless explicitly requested.

Each molecule directory should contain:

- source crop
- candidate render
- side-by-side comparison
- ground-truth detail JSON
- candidate JSON
- commands emitted by OCR
- `structure-metrics.json`
- primitive/debug overlays only as diagnostics

## Regression Ledger

Record a repair count only when a molecule had passed and a later unrelated
change breaks it. Do not count the same active development round, gate upgrades,
or agreed changes to the acceptance definition.
