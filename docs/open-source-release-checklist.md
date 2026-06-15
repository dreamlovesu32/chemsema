# Open Source Release Checklist

This checklist tracks the repository hygiene pass before making ChemCore public.

## Completed In This Pass

- Added Apache-2.0 licensing files and package metadata.
- Added contribution, security, and conduct documents.
- Left the README opening section for the maintainer and organized the rest of
  the technical setup.
- Added ignore rules for local environment files, private keys, certificates,
  installers, and generated packaging output.
- Replaced hardcoded local Python interpreter paths in optional analysis scripts
  with `CHEMCORE_PYTHON` / current Python.
- Replaced hardcoded desktop sample paths in optional fitting scripts with
  repository `tmp/` defaults.
- Changed the `inspect_label_center` example to read a tracked fixture by
  default while still accepting a custom CDXML path.

## Audit Commands

```bash
rg -n --hidden -g '!.git' -g '!target' -g '!node_modules' -g '!tmp' "SECRET_PATTERN" .
rg -n --hidden -g '!.git' -g '!target' -g '!node_modules' -g '!tmp' "LOCAL_PATH_PATTERN" .
git ls-files | sort
```

## Before Public Launch

- Replace the README opening placeholder with the maintainer's own project
  positioning.
- Decide whether historical developer logs should remain in the public history
  or move to a separate archive.
- Confirm that all tracked CDXML fixtures, images, and comparison assets are
  original, redistributable, or small compatibility fixtures that can be
  published.
- Configure the public repository's issue templates and private security
  advisory channel.
- Run `npm run verify` from a clean checkout.
