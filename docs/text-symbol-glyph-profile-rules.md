# Text Symbol Palette And Glyph Profile Rules

This document covers Unicode characters and special characters used in text editing. It does not cover chemical `SceneObject { type: "symbol" }` objects such as charge, radical, and lone-pair symbols.

## Goals

The text editor must be able to insert and render common symbols reliably. Characters not manually listed in `shared/glyph_profiles.json` still need correct width and height fallbacks. Issues with symbols such as `%` come from incomplete glyph-profile coverage and fallback strategy.

Current rules:

- Text symbols are stored internally as Unicode characters.
- The symbol palette UI reads character groups from the shared catalog.
- The Rust glyph kernel is the authority for caret advance, ink box, background box, glyph polygon, and label clipping.
- The viewer may display the symbol palette and insert characters into the active text editing session, but it cannot redefine glyph geometry.
- Characters not present in the manifest must receive Unicode-category fallbacks; they must not all collapse to a narrow punctuation profile.

## Shared Data

The text symbol palette uses:

```text
shared/text_symbols.json
```

This file only expresses UI groups and Unicode characters. It carries no chemical semantics. After a character enters a document, it remains ordinary text run content.

Glyph profiles use:

```text
shared/glyph_profiles.json
```

This file continues to store deterministic normalized profiles:

- `advanceEm`
- `inkLeftEm`
- `inkTopEm`
- `inkRightEm`
- `inkBottomEm`
- `padXEm`
- `padYEm`
- `shape`
- `visible`

When adding or adjusting text symbols, prefer running the generation script:

```bash
python scripts/generate-glyph-profiles.py
```

The generation script reads `shared/text_symbols.json`, measures character advance and ink bbox from locally available fonts, and only fills missing characters in the manifest. Existing manually calibrated profiles are preserved.

## Runtime Fallback

Even if a character is absent from `shared/glyph_profiles.json`, Rust and the viewer must produce a conservative profile by Unicode category:

- Whitespace characters: `visible` is false; they contribute advance only.
- CJK and fullwidth characters: handled as roughly 1em square profiles.
- Greek, extended Latin, Cyrillic, and similar letters: handled as letter profiles, not narrow punctuation.
- Mathematical symbols and arrows: handled as wide-symbol profiles.
- Unknown symbols: handled as medium-width conservative rectangles.

Fallback profile clipping shapes must be conservative. The auto-generation script can provide real ink bboxes; when corner clipping cannot be determined reliably, use rectangles.

## Clipping Strategy

The first stage guarantees that every character has at least usable bbox clipping:

1. Listed characters use profiles from `shared/glyph_profiles.json`.
2. Missing characters use Unicode-category fallback profiles.
3. `glyphPolygons` are emitted by the Rust glyph kernel.
4. Label clipping and knockout consume `glyphPolygons`; they fall back to label boxes only when no polygon exists.

Corner clipping is enabled only when the profile explicitly declares it. Chinese characters and complex symbols use conservative rectangles by default, so bond lines do not cut into strokes.

## UI Behavior

The bottom-right text symbol palette is an ordinary text input aid:

- Clicking the bottom-right button opens the palette toward the left.
- Clicking a character inserts it directly into the caret when an active text editor exists.
- If there is no active text editor, the tool switches to Text and keeps that character as pending insertion after the next text object is created.
- When not pinned, the palette closes automatically after a character click.
- When the top-right pin button is enabled, clicking characters does not close the palette.

This UI should not mix in chemical `symbol` objects such as charges, radicals, and lone-pair electrons.
