#pragma once

#include <cstddef>
#include <cstdint>
#include <vector>

namespace chemcore::glyph {

enum class ShapeKind : std::uint8_t {
  kRect = 0,
  kEllipse = 1,
  kRectCutTopRight = 2,
  kRectCutBottomRight = 3,
  kRectCutTopLeft = 4,
  kRectCutBottomLeft = 5,
};

enum class ScriptKind : std::uint8_t {
  kNormal = 0,
  kSubscript = 1,
  kSuperscript = 2,
};

enum class AnchorKind : std::uint8_t {
  kGlyphStandardCenter = 0,
};

enum class LabelAlign : std::uint8_t {
  kRight = 0,
  kLeft = 1,
  kAbove = 2,
  kBelow = 3,
};

inline constexpr std::size_t kDefaultAnchorGlyphIndex = static_cast<std::size_t>(-1);

struct Point {
  float x = 0.0f;
  float y = 0.0f;
};

struct Box {
  float x1 = 0.0f;
  float y1 = 0.0f;
  float x2 = 0.0f;
  float y2 = 0.0f;
};

struct GlyphProfile {
  std::uint32_t codepoint = 0;
  ShapeKind shape_kind = ShapeKind::kRect;
  float advance_em = 0.7f;
  float ink_left_em = 0.0f;
  float ink_top_em = -0.7f;
  float ink_right_em = 0.7f;
  float ink_bottom_em = 0.0f;
  float pad_x_em = 0.09f;
  float pad_y_em = 0.09f;
  bool visible = true;
};

struct GlyphInput {
  std::uint32_t codepoint = 0;
  ScriptKind script = ScriptKind::kNormal;
};

struct LayoutConfig {
  float font_size_px = 11.0f;
  float tracking_em = 0.0f;
  float subscript_scale = 0.78f;
  float superscript_scale = 0.78f;
  float subscript_shift_down_em = 0.30f;
  float superscript_shift_up_em = 0.28f;
};

struct GlyphShape {
  ShapeKind kind = ShapeKind::kRect;
  bool visible = false;
  Box box_px {};
  float cx_px = 0.0f;
  float cy_px = 0.0f;
  float rx_px = 0.0f;
  float ry_px = 0.0f;
};

struct GlyphPlacement {
  std::uint32_t codepoint = 0;
  ScriptKind script = ScriptKind::kNormal;
  bool visible = false;
  float font_size_px = 0.0f;
  float origin_x_px = 0.0f;
  float baseline_y_px = 0.0f;
  float advance_px = 0.0f;
  Box ink_box_px {};
  Box background_box_px {};
  GlyphShape shape_px {};
};

struct LabelAnchor {
  bool valid = false;
  AnchorKind kind = AnchorKind::kGlyphStandardCenter;
  std::size_t glyph_index = 0;
  Point point_px {};
};

const GlyphProfile& LookupGlyphProfile(std::uint32_t codepoint);

GlyphPlacement LayoutGlyph(
  const GlyphInput& glyph,
  const LayoutConfig& config,
  float origin_x_px,
  float baseline_y_px
);

std::vector<GlyphPlacement> LayoutGlyphRun(
  const std::vector<GlyphInput>& glyphs,
  const LayoutConfig& config,
  float start_x_px = 0.0f,
  float baseline_y_px = 0.0f
);

std::vector<GlyphPlacement> LayoutGlyphRunAligned(
  const std::vector<GlyphInput>& glyphs,
  const LayoutConfig& config,
  float anchor_origin_x_px = 0.0f,
  float anchor_baseline_y_px = 0.0f,
  std::size_t anchor_glyph_index = kDefaultAnchorGlyphIndex,
  LabelAlign align = LabelAlign::kRight
);

LabelAnchor LocateGlyphRun(
  const std::vector<GlyphInput>& glyphs,
  const std::vector<GlyphPlacement>& placements,
  const LayoutConfig& config,
  std::size_t anchor_glyph_index = kDefaultAnchorGlyphIndex
);

}  // namespace chemcore::glyph
