#include "chemcore/glyph_kernel.h"
#include "chemcore/glyph_kernel.hpp"

#include <algorithm>
#include <vector>

using chemcore::glyph::GlyphInput;
using chemcore::glyph::GlyphPlacement;
using chemcore::glyph::LabelAnchor;
using chemcore::glyph::LabelAlign;
using chemcore::glyph::LayoutConfig;
using chemcore::glyph::LayoutGlyphRun;
using chemcore::glyph::LayoutGlyphRunAligned;
using chemcore::glyph::LocateGlyphRun;
using chemcore::glyph::ScriptKind;
using chemcore::glyph::ShapeKind;

namespace {

LayoutConfig ToCppConfig(const ChemcoreLayoutConfig* config) {
  if (!config) {
    return LayoutConfig {};
  }
  LayoutConfig out {};
  out.font_size_px = config->font_size_px;
  out.tracking_em = config->tracking_em;
  out.subscript_scale = config->subscript_scale;
  out.superscript_scale = config->superscript_scale;
  out.subscript_shift_down_em = config->subscript_shift_down_em;
  out.superscript_shift_up_em = config->superscript_shift_up_em;
  return out;
}

ScriptKind ToCppScriptKind(int value) {
  switch (value) {
    case CHEMCORE_SCRIPT_SUBSCRIPT:
      return ScriptKind::kSubscript;
    case CHEMCORE_SCRIPT_SUPERSCRIPT:
      return ScriptKind::kSuperscript;
    case CHEMCORE_SCRIPT_NORMAL:
    default:
      return ScriptKind::kNormal;
  }
}

LabelAlign ToCppLabelAlign(int value) {
  switch (value) {
    case CHEMCORE_LABEL_ALIGN_LEFT:
      return LabelAlign::kLeft;
    case CHEMCORE_LABEL_ALIGN_ABOVE:
      return LabelAlign::kAbove;
    case CHEMCORE_LABEL_ALIGN_BELOW:
      return LabelAlign::kBelow;
    case CHEMCORE_LABEL_ALIGN_RIGHT:
    default:
      return LabelAlign::kRight;
  }
}

ChemcoreGlyphPlacement ToCPlacement(const GlyphPlacement& placement) {
  ChemcoreGlyphPlacement out {};
  out.codepoint = placement.codepoint;
  out.script_kind = static_cast<int>(placement.script);
  out.visible = placement.visible ? 1 : 0;
  out.font_size_px = placement.font_size_px;
  out.origin_x_px = placement.origin_x_px;
  out.baseline_y_px = placement.baseline_y_px;
  out.advance_px = placement.advance_px;
  out.ink_x1_px = placement.ink_box_px.x1;
  out.ink_y1_px = placement.ink_box_px.y1;
  out.ink_x2_px = placement.ink_box_px.x2;
  out.ink_y2_px = placement.ink_box_px.y2;
  out.background_x1_px = placement.background_box_px.x1;
  out.background_y1_px = placement.background_box_px.y1;
  out.background_x2_px = placement.background_box_px.x2;
  out.background_y2_px = placement.background_box_px.y2;
  out.shape_kind = static_cast<int>(placement.shape_px.kind);
  out.shape_cx_px = placement.shape_px.cx_px;
  out.shape_cy_px = placement.shape_px.cy_px;
  out.shape_rx_px = placement.shape_px.rx_px;
  out.shape_ry_px = placement.shape_px.ry_px;
  return out;
}

ChemcoreLabelAnchor ToCAnchor(const LabelAnchor& anchor) {
  ChemcoreLabelAnchor out {};
  out.valid = anchor.valid ? 1 : 0;
  out.anchor_kind = static_cast<int>(anchor.kind);
  out.glyph_index = anchor.glyph_index;
  out.x_px = anchor.point_px.x;
  out.y_px = anchor.point_px.y;
  return out;
}

std::vector<GlyphInput> ToCppInputs(const ChemcoreGlyphInput* glyphs, size_t glyph_count) {
  std::vector<GlyphInput> inputs;
  inputs.reserve(glyph_count);
  for (size_t index = 0; index < glyph_count; index += 1) {
    GlyphInput input {};
    input.codepoint = glyphs[index].codepoint;
    input.script = ToCppScriptKind(glyphs[index].script_kind);
    inputs.push_back(input);
  }
  return inputs;
}

size_t CopyPlacements(
  const std::vector<GlyphPlacement>& placements,
  ChemcoreGlyphPlacement* out_placements,
  size_t out_capacity
) {
  if (!out_placements || out_capacity == 0) {
    return placements.size();
  }

  const size_t copy_count = std::min(out_capacity, placements.size());
  for (size_t index = 0; index < copy_count; index += 1) {
    out_placements[index] = ToCPlacement(placements[index]);
  }
  return placements.size();
}

}  // namespace

extern "C" {

ChemcoreLayoutConfig chemcore_default_layout_config(void) {
  const LayoutConfig config {};
  ChemcoreLayoutConfig out {};
  out.font_size_px = config.font_size_px;
  out.tracking_em = config.tracking_em;
  out.subscript_scale = config.subscript_scale;
  out.superscript_scale = config.superscript_scale;
  out.subscript_shift_down_em = config.subscript_shift_down_em;
  out.superscript_shift_up_em = config.superscript_shift_up_em;
  return out;
}

size_t chemcore_layout_glyph_run(
  const ChemcoreGlyphInput* glyphs,
  size_t glyph_count,
  const ChemcoreLayoutConfig* config,
  float start_x_px,
  float baseline_y_px,
  ChemcoreGlyphPlacement* out_placements,
  size_t out_capacity
) {
  const std::vector<GlyphInput> inputs = ToCppInputs(glyphs, glyph_count);

  const std::vector<GlyphPlacement> placements = LayoutGlyphRun(
    inputs,
    ToCppConfig(config),
    start_x_px,
    baseline_y_px
  );
  return CopyPlacements(placements, out_placements, out_capacity);
}

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
) {
  const std::vector<GlyphInput> inputs = ToCppInputs(glyphs, glyph_count);

  const std::vector<GlyphPlacement> placements = LayoutGlyphRunAligned(
    inputs,
    ToCppConfig(config),
    anchor_origin_x_px,
    anchor_baseline_y_px,
    anchor_glyph_index,
    ToCppLabelAlign(align)
  );
  return CopyPlacements(placements, out_placements, out_capacity);
}

ChemcoreLabelAnchor chemcore_locate_glyph_run(
  const ChemcoreGlyphInput* glyphs,
  size_t glyph_count,
  const ChemcoreLayoutConfig* config,
  float start_x_px,
  float baseline_y_px
) {
  return chemcore_locate_glyph_run_at(
    glyphs,
    glyph_count,
    config,
    start_x_px,
    baseline_y_px,
    CHEMCORE_DEFAULT_ANCHOR_GLYPH_INDEX
  );
}

ChemcoreLabelAnchor chemcore_locate_glyph_run_at(
  const ChemcoreGlyphInput* glyphs,
  size_t glyph_count,
  const ChemcoreLayoutConfig* config,
  float start_x_px,
  float baseline_y_px,
  size_t anchor_glyph_index
) {
  const LayoutConfig cpp_config = ToCppConfig(config);
  const std::vector<GlyphInput> inputs = ToCppInputs(glyphs, glyph_count);
  const std::vector<GlyphPlacement> placements = LayoutGlyphRun(
    inputs,
    cpp_config,
    start_x_px,
    baseline_y_px
  );
  return ToCAnchor(LocateGlyphRun(inputs, placements, cpp_config, anchor_glyph_index));
}

ChemcoreLabelAnchor chemcore_locate_glyph_run_aligned(
  const ChemcoreGlyphInput* glyphs,
  size_t glyph_count,
  const ChemcoreLayoutConfig* config,
  float anchor_origin_x_px,
  float anchor_baseline_y_px,
  size_t anchor_glyph_index,
  int align
) {
  const LayoutConfig cpp_config = ToCppConfig(config);
  const std::vector<GlyphInput> inputs = ToCppInputs(glyphs, glyph_count);
  const std::vector<GlyphPlacement> placements = LayoutGlyphRunAligned(
    inputs,
    cpp_config,
    anchor_origin_x_px,
    anchor_baseline_y_px,
    anchor_glyph_index,
    ToCppLabelAlign(align)
  );
  return ToCAnchor(LocateGlyphRun(inputs, placements, cpp_config, anchor_glyph_index));
}

}  // extern "C"
