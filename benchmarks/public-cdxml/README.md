# Public CDXML/CDX Round-Trip Corpus

This benchmark uses public, license-clear ChemDraw CDXML/CDX files instead of
confidential research documents. Source files are downloaded into ignored
`tmp/` storage and are not vendored into the ChemSema repository.

The pinned manifest currently provides 413 files from five upstream projects:

| Source | License | CDXML | CDX | Main coverage |
| --- | --- | ---: | ---: | --- |
| RDKit | BSD-3-Clause | 94 | 126 | parser regressions, queries, templates, patent structures |
| Indigo | Apache-2.0 | 123 | 28 | molecules, reactions, rendering, malformed-input tests |
| cdxml-toolkit | MIT | 34 | 2 | complete linear, wrapped, and branched reaction schemes |
| SAMPL6 | MIT | 1 | 2 | published host/guest structures |
| SAMPL9 | MIT | 2 | 1 | published host/guest structures |

Two files are deliberate malformed-input tests. Four `.cdx` files contain
Base64 transport text rather than raw CDX bytes and are classified separately.
The remaining 407 files are positive round-trip cases. One deliberately broken
coordinate fixture is classified as safe sanitization, and two fixtures that
only discard unused shape styles are classified as lossless normalization.

## Reproduce

```bash
npm run benchmark:cdxml-public:fetch
cargo build -p chemsema-cli
npm run benchmark:cdxml-public
```

Set `CHEMSEMA_PUBLIC_CDXML_DIR` to choose another download directory. The
runner writes a detailed untracked report to
`tmp/public-cdxml-roundtrip/report.json`. By default, every positive case is
saved and reopened three times. Each generation is checked with molecule,
arrow-identity, and bracket-geometry semantic fingerprints as well as object,
resource, style, and object-type counts. Semantic drift and non-idempotence
always fail the run; pass `--strict-counts` to also fail on a classified count
drift.

The current ChemSema 1.0.0-beta.1 source baseline has no unexpected failures,
semantic drift, non-idempotence, or unclassified count drift. Of 413 files, 404
are exact through all three generations, one is expected safe sanitization, two
are expected lossless normalization, two are expected import rejection, and
four transport-encoded files are skipped. The semantic gates cover atomic
identity and charge, molecule connectivity, headless-arrow identity, and
bracket grouping and geometry; the count gates independently catch object and
resource growth.

The manifest pins every upstream commit and records its license URL. When the
corpus changes, update the manifest, rerun the benchmark, and commit a new
versioned summary rather than silently replacing an old baseline.

The license column records the license published for each upstream repository.
Because the downloader leaves every file in its original repository, this is
appropriate for a reproducible external benchmark. Before repackaging the
files as a standalone dataset, recheck per-file provenance and attribution,
especially for patent-derived RDKit fixtures.
