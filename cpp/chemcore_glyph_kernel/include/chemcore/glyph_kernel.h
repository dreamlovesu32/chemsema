#pragma once

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum ChemcoreShapeKind {
  CHEMCORE_SHAPE_RECT = 0,
  CHEMCORE_SHAPE_ELLIPSE = 1,
  CHEMCORE_SHAPE_RECT_CUT_TOP_RIGHT = 2,
  CHEMCORE_SHAPE_RECT_CUT_BOTTOM_RIGHT = 3,
  CHEMCORE_SHAPE_RECT_CUT_TOP_LEFT = 4,
  CHEMCORE_SHAPE_RECT_CUT_BOTTOM_LEFT = 5
} ChemcoreShapeKind;

typedef enum ChemcoreScriptKind {
  CHEMCORE_SCRIPT_NORMAL = 0,
  CHEMCORE_SCRIPT_SUBSCRIPT = 1,
  CHEMCORE_SCRIPT_SUPERSCRIPT = 2
} ChemcoreScriptKind;

typedef enum ChemcoreAnchorKind {
  CHEMCORE_ANCHOR_GLYPH_STANDARD_CENTER = 0
} ChemcoreAnchorKind;

typedef enum ChemcoreLabelAlign {
  CHEMCORE_LABEL_ALIGN_RIGHT = 0,
  CHEMCORE_LABEL_ALIGN_LEFT = 1,
  CHEMCORE_LABEL_ALIGN_ABOVE = 2,
  CHEMCORE_LABEL_ALIGN_BELOW = 3
} ChemcoreLabelAlign;

#define CHEMCORE_DEFAULT_ANCHOR_GLYPH_INDEX ((size_t)-1)

typedef struct ChemcoreGlyphInput {
  uint32_t codepoint;
  int script_kind;
} ChemcoreGlyphInput;

typedef struct ChemcoreLayoutConfig {
  float font_size_px;
  float tracking_em;
  float subscript_scale;
  float superscript_scale;
  float subscript_shift_down_em;
  float superscript_shift_up_em;
} ChemcoreLayoutConfig;

typedef struct ChemcoreGlyphPlacement {
  uint32_t codepoint;
  int script_kind;
  int visible;
  float font_size_px;
  float origin_x_px;
  float baseline_y_px;
  float advance_px;
  float ink_x1_px;
  float ink_y1_px;
  float ink_x2_px;
  float ink_y2_px;
  float background_x1_px;
  float background_y1_px;
  float background_x2_px;
  float background_y2_px;
  int shape_kind;
  float shape_cx_px;
  float shape_cy_px;
  float shape_rx_px;
  float shape_ry_px;
} ChemcoreGlyphPlacement;

typedef struct ChemcoreLabelAnchor {
  int valid;
  int anchor_kind;
  size_t glyph_index;
  float x_px;
  float y_px;
} ChemcoreLabelAnchor;

ChemcoreLayoutConfig chemcore_default_layout_config(void);

size_t chemcore_layout_glyph_run(
  const ChemcoreGlyphInput* glyphs,
  size_t glyph_count,
  const ChemcoreLayoutConfig* config,
  float start_x_px,
  float baseline_y_px,
  ChemcoreGlyphPlacement* out_placements,
  size_t out_capacity
);

size_t chemcore_layout_glyph_run_aligned(
  const ChemcoreGlyphInput* glyphs,
  size_t glyph_count,
  const ChemcoreLayoutConfig* config,
  float anchor_origin_x_px,
  float anchor_baseline_y_px,
  size_t anchor_glyph_index,
  int align,
  ChemcoreGlyphPlacement* out_placements,
  size_t out_capacity
);

ChemcoreLabelAnchor chemcore_locate_glyph_run(
  const ChemcoreGlyphInput* glyphs,
  size_t glyph_count,
  const ChemcoreLayoutConfig* config,
  float start_x_px,
  float baseline_y_px
);

ChemcoreLabelAnchor chemcore_locate_glyph_run_at(
  const ChemcoreGlyphInput* glyphs,
  size_t glyph_count,
  const ChemcoreLayoutConfig* config,
  float start_x_px,
  float baseline_y_px,
  size_t anchor_glyph_index
);

ChemcoreLabelAnchor chemcore_locate_glyph_run_aligned(
  const ChemcoreGlyphInput* glyphs,
  size_t glyph_count,
  const ChemcoreLayoutConfig* config,
  float anchor_origin_x_px,
  float anchor_baseline_y_px,
  size_t anchor_glyph_index,
  int align
);

#ifdef __cplusplus
}
#endif
