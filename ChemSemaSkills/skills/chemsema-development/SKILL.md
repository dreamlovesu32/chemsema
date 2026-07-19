---
name: chemsema-development
description: Build, test, package, release, and diagnose ChemSema repositories. Use for Rust engine work, WASM rebuilds, desktop/Tauri packages, Office helper checks, CI failures, npm verification, cargo tests, CLI protocol changes, regression gates, repository hygiene, dirty worktree triage, and start-menu package replacement.
---

# ChemSema Development

## First Pass

Start with repository state and runtime discovery:

```powershell
git status --short
npm run verify
cargo test --workspace
cargo run -p chemsema-cli -- version --pretty
cargo run -p chemsema-cli -- doctor --pretty
```

Read references based on the task:

- `references/build-test-package.md` for normal verification and packaging.
- `references/desktop-office-verification.md` for desktop, WASM, Office helper,
  and start-menu package replacement.
- `references/ci-debug.md` for GitHub Actions failures.
- `references/repo-hygiene.md` for commits, dirty worktrees, and generated
  artifacts.

Use `scripts/chemsema_check.ps1` as a repeatable local check wrapper.

## Engineering Guardrails

- Fix root causes, not only failing snapshots.
- Keep CLI protocol changes documented in docs/protocol and runtime schema.
- Do not overwrite unrelated user changes.
- Before committing, inspect the diff and run the narrowest meaningful checks.
- For desktop work, test both engine behavior and UI interaction if the change
  crosses that boundary.
