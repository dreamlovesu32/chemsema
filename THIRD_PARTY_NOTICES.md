# Third-Party Notices

This project is intended to keep its core document model, viewer, and depiction
engine under direct control of the `chemsema` codebase.

The current policy is:

- prefer original implementations for core architecture and rendering
- allow reference-driven reimplementation of chemical drawing rules
- allow selective reuse of permissively licensed code when it is clearly
  attributed and legally compatible

The `chemsema-chemistry` SMILES grammar and lexer are a clean Rust
reimplementation informed by RDKit. They do not link to RDKit or require its
runtime, but the relevant files are treated as derived work and retain explicit
BSD attribution.

## Adapted Source

- RDKit
  - upstream: https://github.com/rdkit/rdkit
  - audited commit: `0062b670640352ab63d6256be608615e87e1af53`
  - source paths: `Code/GraphMol/SmilesParse/smiles.yy`,
    `Code/GraphMol/SmilesParse/smiles.ll`
  - ChemSema paths: `crates/chemsema-chemistry/src/smiles.rs`,
    `crates/chemsema-chemistry/src/lib.rs`
  - license: BSD 3-Clause; see `LICENSES/BSD-3-Clause-RDKit.txt`
  - changes: redesigned as a dependency-light Rust parser and writer over the
    ChemSema-owned typed chemical graph; no RDKit/Python runtime is included

- IUPAC InChI 1.07.5
  - upstream: https://github.com/IUPAC-InChI/InChI
  - release commit: `11a87982bb518f57ac013f0b258c283655e1ea1d`
  - source: unmodified `INCHI_BASE/src` and `INCHI_API/libinchi/src` under
    `third_party/inchi-1.07.5`
  - native boundary: `crates/chemsema-inchi`
  - browser artifacts: official InChI Web Demo builds
    `viewer/inchi/inchi-web-1075.js` and `inchi-web-1075.wasm`
  - browser artifact SHA-256:
    `1D1952024ECF9BF17844C7B6420F0FAA260D65F93891FAA95056090A11243060`
    and `4B93FEB7B8997E07EC6A13A44CD5CD4555F70C8AC18A1D840EB6341D53261849`
  - license: MIT; see `LICENSES/MIT-InChI.txt`, the vendored source license,
    and `viewer/inchi/LICENSE`
  - integration: compiled statically for native Rust targets and lazily loaded
    as the official WebAssembly module in the browser; no network service or
    Python runtime is used

## Reference Projects

The following open-source projects may be studied for depiction rules,
interaction patterns, and file-format handling:

- Ketcher
  - project: https://github.com/epam/ketcher
  - license: Apache License 2.0
- RDKit.js
  - project: https://github.com/rdkit/rdkit-js
  - license: BSD 3-Clause
- OpenChemLib
  - project: https://github.com/Actelion/openchemlib
  - repository license metadata indicates a permissive open-source license
- OpenChemLib JS
  - project: https://github.com/cheminfo/openchemlib-js
  - license: BSD 3-Clause

## Reuse Rules

If code is copied or adapted from a third-party project in the future:

1. Keep the original license header where required.
2. Add the source project, file path, commit, and license to this document.
3. Mark modified files clearly as changed by `chemsema`.
4. Preserve any required NOTICE or attribution text.
5. Do not imply endorsement by the upstream project or maintainers.

## Current Status

- RDKit-informed SMILES grammar/lexer reimplemented in Rust with BSD attribution
- official IUPAC InChI 1.07.5 is bundled for native and browser targets
- no Python runtime and no remote chemical conversion service
