# CI Debug

Use the GitHub CI tool or `gh` to inspect failing checks before guessing.

Workflow:

1. Identify the failing workflow and job.
2. Read the failed log section.
3. Reproduce locally with the same command if possible.
4. Patch the smallest root cause.
5. Run the focused check locally.
6. Commit and push.

Common ChemSema buckets:

- Rust formatting or clippy
- crate unit test failure
- CLI JSON protocol test failure
- npm/TypeScript build failure
- WASM generation mismatch
- desktop/Tauri packaging failure
- snapshot or fixture drift

If CI fails only on Windows path behavior, reproduce with PowerShell syntax and
avoid shell-specific assumptions.
