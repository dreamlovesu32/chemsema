# Legacy glyph clip manifest

The former `shared/glyph_clip_polygons.json` character table and its generator have been removed. The Rust engine has no legacy character-table input.

The active rule derives retreat geometry from real font outlines at runtime. See [glyph-kernel.md](./glyph-kernel.md). Do not add new character exceptions or restore this manifest as a renderer fallback.
