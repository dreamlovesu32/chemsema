# Third-Party Notices

This project is intended to keep its core document model, viewer, and depiction
engine under direct control of the `chemsema` codebase.

The current policy is:

- prefer original implementations for core architecture and rendering
- allow reference-driven reimplementation of chemical drawing rules
- allow selective reuse of permissively licensed code when it is clearly
  attributed and legally compatible

At the time of writing, the viewer and converter code in this repository are
implemented directly in `chemsema`. No third-party source files have been
vendored into the repository yet.

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

- no vendored third-party source files
- no bundled third-party JavaScript runtime dependencies
- references are currently used for design study only
