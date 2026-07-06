# Build, Test, Package

## Standard Checks

Run from the ChemCore repo root:

```powershell
git status --short
npm run verify
cargo test --workspace
cargo run -p chemcore-cli -- version --pretty
cargo run -p chemcore-cli -- capabilities --pretty
```

If a change only touches a crate, run that crate's focused tests first, then the
workspace or `npm run verify` before commit.

## WASM

Use the repository package scripts when available. After rebuilding WASM, start
the browser app and test the affected interaction in a real browser or the
in-app browser.

## CLI Protocol Changes

When adding or changing CLI behavior:

- update command implementation
- update `docs/protocol` when the machine contract changes
- update `docs/chemcore-cli-guide.md` and zh-CN counterpart when user-facing
  guidance changes
- update schema/capabilities output
- add tests for JSON shape and behavior

## Generated Artifacts

Only commit generated artifacts that belong in the repo. For local desktop
packages, installers, temp captures, and debug output, keep them outside tracked
paths unless the repository already tracks that artifact class.
