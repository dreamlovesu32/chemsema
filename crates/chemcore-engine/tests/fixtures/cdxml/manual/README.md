# Optional Local CDXML Fixtures

This directory is reserved for local CDXML regression fixtures used during
import, rendering, toolbar, arrow, shape, orbital, and large-selection checks.

CDXML/CDX files can contain unpublished reaction content, so they are ignored by
git. Tests that use these files must treat them as optional: run the regression
when the local fixture exists, and skip cleanly in public source checkouts.

- `desktop/`: local hand-authored or manually exported files used for
  desktop-facing drawing behavior.
