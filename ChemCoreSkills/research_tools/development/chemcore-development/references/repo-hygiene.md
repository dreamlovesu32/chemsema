# Repo Hygiene

## Dirty Worktree

Before editing:

```powershell
git status --short
```

Treat unknown dirty files as user work unless proven otherwise. Do not revert
unrelated changes. If a user asks to commit, inspect the diff first.

## Commit Scope

Good ChemCore commits usually contain:

- code change
- docs/protocol updates when contracts changed
- focused tests or updated fixtures
- generated runtime files only when repo policy expects them

Avoid mixing unrelated OCR, desktop, Office, and packaging changes unless the
user explicitly asks for a bulk sync.

## Push

After a successful commit:

```powershell
git push
```

If the remote rejects, inspect branch and authentication state; do not force
push unless explicitly requested.
