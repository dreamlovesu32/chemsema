# Changelog

All notable public changes to ChemCore are tracked here.

## 1.0.0-beta.4

Large-document interaction, CDXML fidelity, and agent-friendly CLI beta release.

- Added the `chemcore-cli` crate for headless document inspection, conversion, export, blank-document generation, and JSON command execution.
- Added direct engine commands for agent-style editing: text creation, text run replacement, node label run replacement, target deletion, target movement, target rotation, arrow geometry editing, shape geometry editing, document load/export/convert/inspect, and document style application.
- Added structured CLI execution reports that record whether each command executed, whether it changed the document, created/updated/deleted target ids, command errors, per-command after snapshots, and final document state.
- Added `--document-json` and `--inspect-after` CLI options so scripts and agents can inspect internal ChemCore JSON, molecule state, and object state without opening the GUI.
- Improved `.json`/`.ccjs` handling in the desktop file format service so internal ChemCore JSON is easier to exchange.
- Improved CDXML import/export fidelity for labels, arrows, symbols, bold line widths, radical valence, grouped arrows, stacked labels, attached-label group layout, numeric glyph anchors, cached fragments inside labels, and parenthesized sulfonyl labels.
- Tightened glyph clipping and label geometry, including refreshed glyph clip polygon coverage, more conservative imported-label anchors, and updated synthetic SVG snapshots.
- Reworked large-document interaction performance by making editor rendering updates more local, caching structure move preview inputs, avoiding unnecessary full refreshes, optimizing object creation latency, and adding deferred object creation synchronization before patching.
- Rebuilt the selection and drag-preview pipeline for large structures, including local drawing previews, frame-local structure drag previews, backend target primitives for drag previews, frontend partial-bond previews, and more stable live selection previews.
- Fixed hover, focus, and overlay lifecycle problems after drawing, pointer commits, object creation, selection drags, selected object drags, bracket handle edits, arrow handle edits, and multi-molecule drag operations.
- Improved grouped-object and selection semantics, including grouped child editing, grouped molecule hover hit testing, preventing region selection from dragging parent groups, and fixing incremental render targets for multi-molecule drags.
- Refined arrow, bracket, shape, and object control handles, including curved-arrow style/geometry previews, bracket handle resize refresh, consistent object control handle styling, and hidden diagnostic markers during selection drag.
- Added browser file drag-and-drop support, opening dropped files in the current viewer, shared viewer display scale handling, and fast desktop/viewer development scripts.
- Added regression and diagnostic coverage for viewer interactions, large drag previews, large object operations, glyph clip manifest coverage, SVG pixel comparisons, and PowerPoint/CDX render comparison workflows.
- Added English and Chinese CLI command guides, public interaction-feedback rules, early project history notes, and README architecture updates describing the shared Rust engine, human-friendly UI, and agent-friendly CLI.

## 1.0.0-beta.3

Installer hotfix beta release.

- Fixed the Windows NSIS installer Office/OLE registration hook so it finds `chemcore-office.exe` in the installed application directory instead of assuming the old `resources` subdirectory layout.
- Kept compatibility with both root-level and `resources`-level Office server layouts so older packaging experiments do not break registration.
- Hardened post-install registration: the installer now tries machine-wide COM/OLE registration first, then falls back to current-user registration if the machine step cannot run or returns a failure code.
- Hardened uninstall cleanup by attempting both machine-wide and current-user Office/OLE unregistration.
- Rebuilt and manually verified the Windows x64 installer after a clean-install trace cleanup.

## 1.0.0-beta.2

Second public beta release.

- Added bracket-to-count text links for repeating units, including Link/Unlink context-menu actions, `Ctrl+L` / `Ctrl+Shift+L` shortcuts, CDXML import pairing, and repeat-unit refresh after edits.
- Improved bracket text editing: empty labels created by bracket drawing are discarded on the next tool action, non-empty labels commit before switching tools, bracket labels remain editable with the text tool, and bracket-label placement/font defaults are aligned with ChemDraw bracket fixtures.
- Fixed repeat-unit chemistry summaries so linked numeric bracket counts contribute to formula and mass when the repeat unit is well-defined; unlinking detaches the count semantics without breaking bracket selection.
- Expanded selection behavior around grouped objects and brackets: double-clicking a molecule includes enclosing brackets and linked counts, grouped scene text remains editable, and switching away from Select clears the current selection state.
- Fixed stale hover/focus state after drawing, changing curved-arrow geometry, and moving between selected objects; selected labels, bonds, and atoms no longer keep internal hover highlights inside selection boxes.
- Added desktop/browser editing polish: the window top edge can start dragging even while modal prompts are open, browser context menu and common browser shortcuts are intercepted during editing, and context-menu glyph mojibake is removed.
- Fixed chemistry summaries for indeterminate generic labels: selected molecules containing `R`, `R'`, `R''`, or connected `Ar` no longer show formula or molecular-weight values that would imply a fully known composition.
- Treated connected `Ar` labels as generic aryl abbreviations instead of argon during structure-label editing, while keeping explicit element replacement available through the element workflow.
- Rebuilt the browser WASM engine and Windows desktop executable so the web and desktop surfaces use the same corrected engine behavior.
- Added regression coverage and public fixtures for bracket CDXML imports, repeat-unit links, grouped editing, selected-object hover suppression, generic-label chemistry summaries, and complete abbreviation expansion summaries.

## 1.0.0-beta.1

Initial public beta release.

- Published the shared Rust chemistry editor engine, browser viewer, Windows desktop shell, and Office/OLE integration foundations.
- Added CDXML/CDX import and export paths, SVG export, EMF preview generation, and Word-oriented clipboard/OLE payload support.
- Included public synthetic CDXML regression fixtures plus maintainer-authored published-figure benchmark files.
- Added GitHub Actions CI, GitHub Pages demo deployment, issue templates, roadmap, and rendering comparison documentation.
- Documented the current beta status: source builds are available now, and Windows installer packaging is still under test.
