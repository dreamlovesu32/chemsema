#include "chemcore/glyph_kernel.h"
#include "chemcore/glyph_kernel.hpp"

#include <cassert>
#include <cmath>
#include <algorithm>
#include <vector>

using chemcore::glyph::AnchorKind;
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

int main() {
  const LayoutConfig config {};

  {
    const std::vector<GlyphPlacement> placements = LayoutGlyphRun(
      std::vector<GlyphInput> {{static_cast<std::uint32_t>(U'O'), ScriptKind::kNormal}},
      config
    );
    assert(placements.size() == 1);
    assert(placements[0].shape_px.kind == ShapeKind::kEllipse);
    assert(placements[0].shape_px.rx_px > 0.0f);
    assert(placements[0].shape_px.ry_px > 0.0f);
  }

  {
    const std::vector<GlyphPlacement> placements = LayoutGlyphRun(
      std::vector<GlyphInput> {
        {static_cast<std::uint32_t>(U't'), ScriptKind::kNormal},
        {static_cast<std::uint32_t>(U'-'), ScriptKind::kNormal},
        {static_cast<std::uint32_t>(U'B'), ScriptKind::kNormal},
        {static_cast<std::uint32_t>(U'u'), ScriptKind::kNormal},
      },
      config
    );
    assert(placements.size() == 4);
    const float b_height = placements[2].background_box_px.y2 - placements[2].background_box_px.y1;
    const float u_height = placements[3].background_box_px.y2 - placements[3].background_box_px.y1;
    assert(u_height < b_height);
  }

  {
    const std::vector<GlyphPlacement> placements = LayoutGlyphRun(
      std::vector<GlyphInput> {
        {static_cast<std::uint32_t>(U'M'), ScriptKind::kNormal},
        {static_cast<std::uint32_t>(U'e'), ScriptKind::kNormal},
      },
      config
    );
    assert(placements.size() == 2);
    assert(placements[1].origin_x_px - placements[0].origin_x_px > config.font_size_px * 0.82f);
  }

  {
    const std::vector<GlyphPlacement> placements = LayoutGlyphRun(
      std::vector<GlyphInput> {
        {static_cast<std::uint32_t>(U'L'), ScriptKind::kNormal},
        {static_cast<std::uint32_t>(U'P'), ScriptKind::kNormal},
        {static_cast<std::uint32_t>(U'F'), ScriptKind::kNormal},
        {static_cast<std::uint32_t>(U'd'), ScriptKind::kNormal},
        {static_cast<std::uint32_t>(U'q'), ScriptKind::kNormal},
        {static_cast<std::uint32_t>(U'h'), ScriptKind::kNormal},
        {static_cast<std::uint32_t>(U'b'), ScriptKind::kNormal},
        {static_cast<std::uint32_t>(U'r'), ScriptKind::kNormal},
      },
      config
    );
    assert(placements.size() == 8);
    assert(placements[0].shape_px.kind == ShapeKind::kRectCutTopRight);
    assert(placements[1].shape_px.kind == ShapeKind::kRectCutBottomRight);
    assert(placements[2].shape_px.kind == ShapeKind::kRectCutBottomRight);
    assert(placements[3].shape_px.kind == ShapeKind::kRectCutTopLeft);
    assert(placements[4].shape_px.kind == ShapeKind::kRectCutBottomLeft);
    assert(placements[5].shape_px.kind == ShapeKind::kRectCutTopRight);
    assert(placements[6].shape_px.kind == ShapeKind::kRectCutTopRight);
    assert(placements[7].shape_px.kind == ShapeKind::kRect);
  }

  {
    assert(chemcore::glyph::LookupGlyphProfile(static_cast<std::uint32_t>(U'3')).ink_top_em <= -0.70f);
    const std::vector<GlyphPlacement> placements = LayoutGlyphRun(
      std::vector<GlyphInput> {
        {static_cast<std::uint32_t>(U'N'), ScriptKind::kNormal},
        {static_cast<std::uint32_t>(U'3'), ScriptKind::kSuperscript},
      },
      config
    );
    assert(placements.size() == 2);
    assert(placements[1].font_size_px < placements[0].font_size_px);
    assert(placements[1].baseline_y_px < placements[0].baseline_y_px);
    assert((placements[1].ink_box_px.y2 - placements[1].ink_box_px.y1) > placements[1].font_size_px * 0.68f);
    assert(placements[1].background_box_px.y1 < placements[1].ink_box_px.y1);
  }

  {
    assert(chemcore::glyph::LookupGlyphProfile(static_cast<std::uint32_t>(U'2')).ink_top_em <= -0.70f);
    const ChemcoreGlyphInput inputs[] = {
      {static_cast<std::uint32_t>(U'S'), CHEMCORE_SCRIPT_NORMAL},
      {static_cast<std::uint32_t>(U'O'), CHEMCORE_SCRIPT_NORMAL},
      {static_cast<std::uint32_t>(U'2'), CHEMCORE_SCRIPT_SUBSCRIPT},
    };
    const ChemcoreLayoutConfig c_config = chemcore_default_layout_config();
    ChemcoreGlyphPlacement outputs[3] {};
    const size_t required = chemcore_layout_glyph_run(inputs, 3, &c_config, 0.0f, 0.0f, outputs, 3);
    assert(required == 3);
    assert(outputs[1].shape_kind == CHEMCORE_SHAPE_ELLIPSE);
    assert(outputs[2].font_size_px < outputs[1].font_size_px);
    assert((outputs[2].ink_y2_px - outputs[2].ink_y1_px) > outputs[2].font_size_px * 0.68f);
    assert(outputs[2].background_y1_px < outputs[2].ink_y1_px);
  }

  {
    const std::vector<GlyphPlacement> placements = LayoutGlyphRun(
      std::vector<GlyphInput> {
        {static_cast<std::uint32_t>(U'2'), ScriptKind::kSuperscript},
        {static_cast<std::uint32_t>(U'-'), ScriptKind::kSuperscript},
      },
      config
    );
    assert(placements.size() == 2);
    const float digit_center = (placements[0].ink_box_px.y1 + placements[0].ink_box_px.y2) * 0.5f;
    const float minus_center = (placements[1].ink_box_px.y1 + placements[1].ink_box_px.y2) * 0.5f;
    assert(std::fabs(digit_center - minus_center) < 0.01f);
  }

  {
    const std::vector<GlyphPlacement> placements = LayoutGlyphRun(
      std::vector<GlyphInput> {
        {static_cast<std::uint32_t>(U'2'), ScriptKind::kSuperscript},
        {static_cast<std::uint32_t>(U'+'), ScriptKind::kSuperscript},
      },
      config
    );
    assert(placements.size() == 2);
    const float digit_center = (placements[0].ink_box_px.y1 + placements[0].ink_box_px.y2) * 0.5f;
    const float plus_center = (placements[1].ink_box_px.y1 + placements[1].ink_box_px.y2) * 0.5f;
    assert(std::fabs(digit_center - plus_center) < 0.01f);
  }

  {
    const std::vector<GlyphInput> inputs {
      {static_cast<std::uint32_t>(U'O'), ScriptKind::kNormal},
    };
    const std::vector<GlyphPlacement> placements = LayoutGlyphRun(inputs, config);
    const LabelAnchor anchor = LocateGlyphRun(inputs, placements, config);
    assert(anchor.valid);
    assert(anchor.kind == AnchorKind::kGlyphStandardCenter);
    assert(anchor.glyph_index == 0);
    assert(std::fabs(anchor.point_px.x - placements[0].shape_px.cx_px) < 0.01f);
    assert(std::fabs(anchor.point_px.y - placements[0].shape_px.cy_px) < 0.01f);
  }

  {
    const std::vector<GlyphInput> inputs {
      {static_cast<std::uint32_t>(U'H'), ScriptKind::kNormal},
      {static_cast<std::uint32_t>(U'N'), ScriptKind::kNormal},
    };
    const std::vector<GlyphPlacement> placements = LayoutGlyphRun(inputs, config, 10.0f, 20.0f);
    const LabelAnchor anchor = LocateGlyphRun(inputs, placements, config);
    const float h_center_x = (placements[0].background_box_px.x1 + placements[0].background_box_px.x2) * 0.5f;
    assert(anchor.valid);
    assert(anchor.kind == AnchorKind::kGlyphStandardCenter);
    assert(anchor.glyph_index == 0);
    assert(std::fabs(anchor.point_px.x - h_center_x) < 0.01f);
    assert(std::fabs(anchor.point_px.y - (20.0f - 0.365f * config.font_size_px)) < 0.01f);
  }

  {
    const std::vector<GlyphInput> inputs {
      {static_cast<std::uint32_t>(U'C'), ScriptKind::kNormal},
      {static_cast<std::uint32_t>(U'l'), ScriptKind::kNormal},
    };
    const std::vector<GlyphPlacement> placements = LayoutGlyphRun(inputs, config);
    const LabelAnchor anchor = LocateGlyphRun(inputs, placements, config);
    const float c_center_x = (placements[0].background_box_px.x1 + placements[0].background_box_px.x2) * 0.5f;
    assert(anchor.valid);
    assert(anchor.kind == AnchorKind::kGlyphStandardCenter);
    assert(anchor.glyph_index == 0);
    assert(std::fabs(anchor.point_px.x - c_center_x) < 0.01f);
  }

  {
    const std::vector<GlyphInput> inputs {
      {static_cast<std::uint32_t>(U'C'), ScriptKind::kNormal},
      {static_cast<std::uint32_t>(U'a'), ScriptKind::kNormal},
    };
    const std::vector<GlyphPlacement> placements = LayoutGlyphRun(inputs, config);
    const LabelAnchor anchor = LocateGlyphRun(inputs, placements, config, 1);
    const float a_center_x = (placements[1].background_box_px.x1 + placements[1].background_box_px.x2) * 0.5f;
    assert(anchor.valid);
    assert(anchor.kind == AnchorKind::kGlyphStandardCenter);
    assert(anchor.glyph_index == 1);
    assert(std::fabs(anchor.point_px.x - a_center_x) < 0.01f);
  }

  {
    const ChemcoreGlyphInput inputs[] = {
      {static_cast<std::uint32_t>(U'P'), CHEMCORE_SCRIPT_NORMAL},
      {static_cast<std::uint32_t>(U'h'), CHEMCORE_SCRIPT_NORMAL},
    };
    const ChemcoreLayoutConfig c_config = chemcore_default_layout_config();
    const ChemcoreLabelAnchor anchor = chemcore_locate_glyph_run_at(inputs, 2, &c_config, 0.0f, 0.0f, 1);
    assert(anchor.valid == 1);
    assert(anchor.anchor_kind == CHEMCORE_ANCHOR_GLYPH_STANDARD_CENTER);
    assert(anchor.glyph_index == 1);
  }

  {
    const std::vector<GlyphInput> inputs {
      {static_cast<std::uint32_t>(U'S'), ScriptKind::kNormal},
      {static_cast<std::uint32_t>(U'O'), ScriptKind::kNormal},
      {static_cast<std::uint32_t>(U'2'), ScriptKind::kSubscript},
    };

    const std::vector<GlyphPlacement> right = LayoutGlyphRunAligned(
      inputs,
      config,
      0.0f,
      0.0f,
      0,
      LabelAlign::kRight
    );
    assert(right[0].origin_x_px < right[1].origin_x_px);

    const std::vector<GlyphPlacement> left = LayoutGlyphRunAligned(
      inputs,
      config,
      0.0f,
      0.0f,
      0,
      LabelAlign::kLeft
    );
    assert(left[0].origin_x_px < left[1].origin_x_px);
    assert(left[1].origin_x_px < left[2].origin_x_px);
    const LabelAnchor left_anchor = LocateGlyphRun(inputs, left, config, 0);
    const float s_center_x = (left[0].background_box_px.x1 + left[0].background_box_px.x2) * 0.5f;
    assert(left_anchor.valid);
    assert(left_anchor.glyph_index == 0);
    assert(std::fabs(left_anchor.point_px.x - s_center_x) < 0.01f);

    const std::vector<GlyphPlacement> above = LayoutGlyphRunAligned(
      inputs,
      config,
      0.0f,
      0.0f,
      0,
      LabelAlign::kAbove
    );
    const float above_group_bottom = std::max(above[1].background_box_px.y2, above[2].background_box_px.y2);
    assert(above_group_bottom < above[0].background_box_px.y1);
    assert(std::fabs(above[1].origin_x_px - above[0].origin_x_px) < 0.01f);

    const std::vector<GlyphPlacement> below = LayoutGlyphRunAligned(
      inputs,
      config,
      0.0f,
      0.0f,
      0,
      LabelAlign::kBelow
    );
    const float below_group_top = std::min(below[1].background_box_px.y1, below[2].background_box_px.y1);
    assert(below_group_top > below[0].background_box_px.y2);
    assert(std::fabs(below[1].origin_x_px - below[0].origin_x_px) < 0.01f);
  }

  {
    const std::vector<GlyphInput> inputs {
      {static_cast<std::uint32_t>(U'O'), ScriptKind::kNormal},
      {static_cast<std::uint32_t>(U'2'), ScriptKind::kSubscript},
      {static_cast<std::uint32_t>(U'S'), ScriptKind::kNormal},
    };
    const std::vector<GlyphPlacement> left = LayoutGlyphRunAligned(
      inputs,
      config,
      0.0f,
      0.0f,
      2,
      LabelAlign::kLeft
    );
    assert(left[0].origin_x_px < left[1].origin_x_px);
    assert(left[1].origin_x_px < left[2].origin_x_px);
    assert(std::fabs(left[2].origin_x_px) < 0.01f);
  }

  {
    const std::vector<GlyphInput> inputs {
      {static_cast<std::uint32_t>(U'N'), ScriptKind::kNormal},
      {static_cast<std::uint32_t>(U'T'), ScriptKind::kNormal},
      {static_cast<std::uint32_t>(U's'), ScriptKind::kNormal},
    };
    const std::vector<GlyphPlacement> above = LayoutGlyphRunAligned(
      inputs,
      config,
      0.0f,
      0.0f,
      0,
      LabelAlign::kAbove
    );
    assert(above[1].background_box_px.y2 < above[0].background_box_px.y1);
    assert(std::fabs(above[1].origin_x_px - above[0].origin_x_px) < 0.01f);
  }

  {
    const std::vector<GlyphInput> inputs {
      {static_cast<std::uint32_t>(U'N'), ScriptKind::kNormal},
      {static_cast<std::uint32_t>(U'H'), ScriptKind::kNormal},
    };
    const std::vector<GlyphPlacement> above = LayoutGlyphRunAligned(
      inputs,
      config,
      0.0f,
      0.0f,
      0,
      LabelAlign::kAbove
    );
    const float stack_gap = above[0].background_box_px.y1 - above[1].background_box_px.y2;
    assert(stack_gap > 0.0f);
    assert(stack_gap < config.font_size_px * 0.03f);
    assert(std::fabs(above[1].origin_x_px - above[0].origin_x_px) < 0.01f);
  }

  {
    const ChemcoreGlyphInput inputs[] = {
      {static_cast<std::uint32_t>(U'O'), CHEMCORE_SCRIPT_NORMAL},
      {static_cast<std::uint32_t>(U'2'), CHEMCORE_SCRIPT_SUBSCRIPT},
      {static_cast<std::uint32_t>(U'S'), CHEMCORE_SCRIPT_NORMAL},
    };
    const ChemcoreLayoutConfig c_config = chemcore_default_layout_config();
    ChemcoreGlyphPlacement outputs[3] {};
    const size_t required = chemcore_layout_glyph_run_aligned(
      inputs,
      3,
      &c_config,
      0.0f,
      0.0f,
      2,
      CHEMCORE_LABEL_ALIGN_LEFT,
      outputs,
      3
    );
    assert(required == 3);
    assert(outputs[0].origin_x_px < outputs[1].origin_x_px);
    assert(outputs[1].origin_x_px < outputs[2].origin_x_px);
    const ChemcoreLabelAnchor anchor = chemcore_locate_glyph_run_aligned(
      inputs,
      3,
      &c_config,
      0.0f,
      0.0f,
      2,
      CHEMCORE_LABEL_ALIGN_LEFT
    );
    assert(anchor.valid == 1);
    assert(anchor.anchor_kind == CHEMCORE_ANCHOR_GLYPH_STANDARD_CENTER);
    assert(anchor.glyph_index == 2);
  }

  return 0;
}
