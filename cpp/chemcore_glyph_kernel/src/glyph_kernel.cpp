#include "chemcore/glyph_kernel.hpp"

#include <algorithm>
#include <array>
#include <stdexcept>
#include <unordered_map>

namespace chemcore::glyph {
namespace {

GlyphProfile MakeProfile(
  std::uint32_t codepoint,
  ShapeKind shape_kind,
  float advance_em,
  float ink_left_em,
  float ink_top_em,
  float ink_right_em,
  float ink_bottom_em,
  float pad_x_em = 0.09f,
  float pad_y_em = 0.09f,
  bool visible = true
) {
  GlyphProfile profile {};
  profile.codepoint = codepoint;
  profile.shape_kind = shape_kind;
  profile.advance_em = advance_em;
  profile.ink_left_em = ink_left_em;
  profile.ink_top_em = ink_top_em;
  profile.ink_right_em = ink_right_em;
  profile.ink_bottom_em = ink_bottom_em;
  profile.pad_x_em = pad_x_em;
  profile.pad_y_em = pad_y_em;
  profile.visible = visible;
  return profile;
}

const std::unordered_map<std::uint32_t, GlyphProfile>& ExplicitProfiles() {
  static const auto* profiles = new std::unordered_map<std::uint32_t, GlyphProfile> {
    {U' ', MakeProfile(U' ', ShapeKind::kRect, 0.28f, 0.00f, 0.00f, 0.00f, 0.00f, 0.00f, 0.00f, false)},
    {U'+', MakeProfile(U'+', ShapeKind::kRect, 0.58f, 0.00f, -0.47f, 0.58f, 0.00f)},
    {U'-', MakeProfile(U'-', ShapeKind::kRect, 0.33f, 0.00f, -0.32f, 0.33f, 0.00f)},
    {U'(', MakeProfile(U'(', ShapeKind::kRect, 0.33f, 0.00f, -0.73f, 0.33f, 0.21f)},
    {U')', MakeProfile(U')', ShapeKind::kRect, 0.33f, 0.00f, -0.73f, 0.33f, 0.21f)},
    {U'[', MakeProfile(U'[', ShapeKind::kRect, 0.28f, 0.00f, -0.73f, 0.28f, 0.22f)},
    {U']', MakeProfile(U']', ShapeKind::kRect, 0.28f, 0.00f, -0.73f, 0.28f, 0.22f)},
    {U'.', MakeProfile(U'.', ShapeKind::kRect, 0.28f, 0.00f, -0.11f, 0.28f, 0.00f)},
    {U',', MakeProfile(U',', ShapeKind::kRect, 0.28f, 0.00f, -0.11f, 0.28f, 0.15f)},
    {U'/', MakeProfile(U'/', ShapeKind::kRect, 0.28f, -0.01f, -0.73f, 0.29f, 0.02f)},
    {U'∙', MakeProfile(U'∙', ShapeKind::kEllipse, 0.28f, 0.00f, -0.31f, 0.28f, 0.00f)},
    {U'•', MakeProfile(U'•', ShapeKind::kEllipse, 0.35f, 0.00f, -0.47f, 0.35f, 0.00f)},
    {U'A', MakeProfile(U'A', ShapeKind::kRect, 0.67f, 0.00f, -0.73f, 0.67f, 0.00f)},
    {U'B', MakeProfile(U'B', ShapeKind::kRect, 0.67f, 0.00f, -0.73f, 0.67f, 0.01f)},
    {U'C', MakeProfile(U'C', ShapeKind::kEllipse, 0.72f, 0.00f, -0.75f, 0.72f, 0.02f)},
    {U'D', MakeProfile(U'D', ShapeKind::kRect, 0.72f, 0.00f, -0.73f, 0.72f, 0.00f)},
    {U'E', MakeProfile(U'E', ShapeKind::kRect, 0.67f, 0.00f, -0.73f, 0.67f, 0.00f)},
    {U'F', MakeProfile(U'F', ShapeKind::kRectCutBottomRight, 0.61f, 0.00f, -0.73f, 0.61f, 0.00f)},
    {U'G', MakeProfile(U'G', ShapeKind::kEllipse, 0.78f, 0.00f, -0.75f, 0.78f, 0.02f)},
    {U'H', MakeProfile(U'H', ShapeKind::kRect, 0.72f, 0.00f, -0.73f, 0.72f, 0.00f)},
    {U'I', MakeProfile(U'I', ShapeKind::kRect, 0.28f, 0.00f, -0.73f, 0.28f, 0.00f)},
    {U'J', MakeProfile(U'J', ShapeKind::kRect, 0.50f, 0.00f, -0.73f, 0.50f, 0.02f)},
    {U'K', MakeProfile(U'K', ShapeKind::kRect, 0.67f, 0.00f, -0.73f, 0.67f, 0.00f)},
    {U'L', MakeProfile(U'L', ShapeKind::kRectCutTopRight, 0.56f, 0.00f, -0.73f, 0.56f, 0.00f)},
    {U'M', MakeProfile(U'M', ShapeKind::kRect, 0.83f, 0.00f, -0.73f, 0.83f, 0.00f)},
    {U'N', MakeProfile(U'N', ShapeKind::kRect, 0.72f, 0.00f, -0.73f, 0.72f, 0.00f)},
    {U'O', MakeProfile(U'O', ShapeKind::kEllipse, 0.78f, 0.00f, -0.75f, 0.78f, 0.02f)},
    {U'P', MakeProfile(U'P', ShapeKind::kRectCutBottomRight, 0.67f, 0.00f, -0.73f, 0.67f, 0.00f)},
    {U'Q', MakeProfile(U'Q', ShapeKind::kEllipse, 0.78f, 0.00f, -0.75f, 0.78f, 0.06f)},
    {U'R', MakeProfile(U'R', ShapeKind::kRect, 0.72f, 0.00f, -0.73f, 0.72f, 0.00f)},
    {U'S', MakeProfile(U'S', ShapeKind::kRect, 0.67f, 0.00f, -0.75f, 0.67f, 0.02f)},
    {U'T', MakeProfile(U'T', ShapeKind::kRect, 0.61f, 0.00f, -0.73f, 0.61f, 0.00f)},
    {U'U', MakeProfile(U'U', ShapeKind::kRect, 0.72f, 0.00f, -0.73f, 0.72f, 0.02f)},
    {U'V', MakeProfile(U'V', ShapeKind::kRect, 0.67f, 0.00f, -0.73f, 0.67f, 0.00f)},
    {U'W', MakeProfile(U'W', ShapeKind::kRect, 0.94f, 0.00f, -0.73f, 0.94f, 0.00f)},
    {U'X', MakeProfile(U'X', ShapeKind::kRect, 0.67f, 0.00f, -0.73f, 0.67f, 0.00f)},
    {U'Y', MakeProfile(U'Y', ShapeKind::kRect, 0.67f, 0.00f, -0.73f, 0.67f, 0.00f)},
    {U'Z', MakeProfile(U'Z', ShapeKind::kRect, 0.61f, 0.00f, -0.73f, 0.61f, 0.00f)},
    {U'a', MakeProfile(U'a', ShapeKind::kRect, 0.56f, 0.00f, -0.54f, 0.56f, 0.02f)},
    {U'b', MakeProfile(U'b', ShapeKind::kRectCutTopRight, 0.56f, 0.00f, -0.73f, 0.56f, 0.02f)},
    {U'c', MakeProfile(U'c', ShapeKind::kEllipse, 0.50f, 0.00f, -0.54f, 0.50f, 0.02f)},
    {U'd', MakeProfile(U'd', ShapeKind::kRectCutTopLeft, 0.56f, 0.00f, -0.73f, 0.56f, 0.02f)},
    {U'e', MakeProfile(U'e', ShapeKind::kEllipse, 0.56f, 0.00f, -0.54f, 0.56f, 0.02f)},
    {U'f', MakeProfile(U'f', ShapeKind::kRect, 0.28f, 0.00f, -0.73f, 0.28f, 0.00f)},
    {U'g', MakeProfile(U'g', ShapeKind::kEllipse, 0.56f, 0.00f, -0.54f, 0.56f, 0.23f)},
    {U'h', MakeProfile(U'h', ShapeKind::kRectCutTopRight, 0.56f, 0.00f, -0.73f, 0.56f, 0.00f)},
    {U'i', MakeProfile(U'i', ShapeKind::kRect, 0.22f, 0.00f, -0.71f, 0.22f, 0.00f)},
    {U'j', MakeProfile(U'j', ShapeKind::kRect, 0.22f, -0.02f, -0.71f, 0.22f, 0.22f)},
    {U'k', MakeProfile(U'k', ShapeKind::kRect, 0.50f, 0.00f, -0.73f, 0.51f, 0.00f)},
    {U'l', MakeProfile(U'l', ShapeKind::kRect, 0.22f, 0.00f, -0.73f, 0.22f, 0.00f)},
    {U'm', MakeProfile(U'm', ShapeKind::kRect, 0.83f, 0.00f, -0.54f, 0.83f, 0.00f)},
    {U'n', MakeProfile(U'n', ShapeKind::kRect, 0.56f, 0.00f, -0.54f, 0.56f, 0.00f)},
    {U'o', MakeProfile(U'o', ShapeKind::kEllipse, 0.56f, 0.00f, -0.54f, 0.56f, 0.02f)},
    {U'p', MakeProfile(U'p', ShapeKind::kRect, 0.56f, 0.00f, -0.54f, 0.56f, 0.22f)},
    {U'q', MakeProfile(U'q', ShapeKind::kRectCutBottomLeft, 0.56f, 0.00f, -0.54f, 0.56f, 0.23f)},
    {U'r', MakeProfile(U'r', ShapeKind::kRect, 0.33f, 0.00f, -0.54f, 0.33f, 0.00f)},
    {U's', MakeProfile(U's', ShapeKind::kRect, 0.50f, 0.00f, -0.54f, 0.50f, 0.02f)},
    {U't', MakeProfile(U't', ShapeKind::kRect, 0.28f, 0.00f, -0.66f, 0.28f, 0.01f)},
    {U'u', MakeProfile(U'u', ShapeKind::kRect, 0.56f, 0.00f, -0.52f, 0.56f, 0.02f)},
    {U'v', MakeProfile(U'v', ShapeKind::kRect, 0.50f, 0.00f, -0.52f, 0.50f, 0.00f)},
    {U'w', MakeProfile(U'w', ShapeKind::kRect, 0.72f, 0.00f, -0.52f, 0.72f, 0.00f)},
    {U'x', MakeProfile(U'x', ShapeKind::kRect, 0.50f, 0.00f, -0.52f, 0.50f, 0.00f)},
    {U'y', MakeProfile(U'y', ShapeKind::kRect, 0.50f, 0.00f, -0.52f, 0.50f, 0.22f)},
    {U'z', MakeProfile(U'z', ShapeKind::kRect, 0.50f, 0.00f, -0.52f, 0.50f, 0.00f)},
    {U'0', MakeProfile(U'0', ShapeKind::kEllipse, 0.56f, 0.00f, -0.71f, 0.56f, 0.02f)},
    {U'1', MakeProfile(U'1', ShapeKind::kRect, 0.56f, 0.00f, -0.71f, 0.56f, 0.00f)},
    {U'2', MakeProfile(U'2', ShapeKind::kRect, 0.56f, 0.00f, -0.71f, 0.56f, 0.00f)},
    {U'3', MakeProfile(U'3', ShapeKind::kRect, 0.56f, 0.00f, -0.71f, 0.56f, 0.02f)},
    {U'4', MakeProfile(U'4', ShapeKind::kRect, 0.56f, 0.00f, -0.71f, 0.56f, 0.00f)},
    {U'5', MakeProfile(U'5', ShapeKind::kRect, 0.56f, 0.00f, -0.70f, 0.56f, 0.02f)},
    {U'6', MakeProfile(U'6', ShapeKind::kEllipse, 0.56f, 0.00f, -0.71f, 0.56f, 0.02f)},
    {U'7', MakeProfile(U'7', ShapeKind::kRect, 0.56f, 0.00f, -0.70f, 0.56f, 0.00f)},
    {U'8', MakeProfile(U'8', ShapeKind::kEllipse, 0.56f, 0.00f, -0.71f, 0.56f, 0.02f)},
    {U'9', MakeProfile(U'9', ShapeKind::kEllipse, 0.56f, 0.00f, -0.71f, 0.56f, 0.02f)},
  };
  return *profiles;
}

bool IsAsciiUpper(std::uint32_t codepoint) {
  return codepoint >= U'A' && codepoint <= U'Z';
}

bool IsAsciiLower(std::uint32_t codepoint) {
  return codepoint >= U'a' && codepoint <= U'z';
}

bool IsDigit(std::uint32_t codepoint) {
  return codepoint >= U'0' && codepoint <= U'9';
}

bool IsChargeSign(std::uint32_t codepoint) {
  return codepoint == U'+' || codepoint == U'-';
}

const GlyphProfile& DefaultUpperProfile();

float StandardGlyphCenterYOffsetPx(const LayoutConfig& config) {
  const GlyphProfile& profile = DefaultUpperProfile();
  return (profile.ink_top_em + profile.ink_bottom_em) * 0.5f * config.font_size_px;
}

std::size_t ResolveAnchorGlyphIndex(
  const std::vector<GlyphPlacement>& placements,
  std::size_t requested_index
) {
  if (
    requested_index != kDefaultAnchorGlyphIndex
    && requested_index < placements.size()
    && placements[requested_index].visible
  ) {
    return requested_index;
  }

  for (std::size_t index = 0; index < placements.size(); index += 1) {
    if (placements[index].visible) {
      return index;
    }
  }
  return kDefaultAnchorGlyphIndex;
}

const GlyphProfile& DefaultUpperProfile() {
  static const GlyphProfile profile = MakeProfile(0, ShapeKind::kRect, 0.72f, 0.00f, -0.73f, 0.72f, 0.00f);
  return profile;
}

const GlyphProfile& DefaultLowerProfile() {
  static const GlyphProfile profile = MakeProfile(0, ShapeKind::kRect, 0.56f, 0.00f, -0.54f, 0.56f, 0.00f);
  return profile;
}

const GlyphProfile& DefaultDigitProfile() {
  static const GlyphProfile profile = MakeProfile(0, ShapeKind::kRect, 0.56f, 0.00f, -0.71f, 0.56f, 0.00f);
  return profile;
}

const GlyphProfile& DefaultPunctuationProfile() {
  static const GlyphProfile profile = MakeProfile(0, ShapeKind::kRect, 0.45f, 0.03f, -0.40f, 0.38f, 0.02f);
  return profile;
}

float ScriptScale(const LayoutConfig& config, ScriptKind script) {
  switch (script) {
    case ScriptKind::kSubscript:
      return config.subscript_scale;
    case ScriptKind::kSuperscript:
      return config.superscript_scale;
    case ScriptKind::kNormal:
    default:
      return 1.0f;
  }
}

float ScriptBaselineShiftPx(const LayoutConfig& config, ScriptKind script) {
  switch (script) {
    case ScriptKind::kSubscript:
      return config.subscript_shift_down_em * config.font_size_px;
    case ScriptKind::kSuperscript:
      return -config.superscript_shift_up_em * config.font_size_px;
    case ScriptKind::kNormal:
    default:
      return 0.0f;
  }
}

float ChargeSignBaselineAdjustmentPx(const GlyphProfile& profile, const LayoutConfig& config, ScriptKind script) {
  if (script == ScriptKind::kNormal) {
    return 0.0f;
  }
  const GlyphProfile& digit_profile = DefaultDigitProfile();
  const float digit_center_em = (digit_profile.ink_top_em + digit_profile.ink_bottom_em) * 0.5f;
  const float sign_center_em = (profile.ink_top_em + profile.ink_bottom_em) * 0.5f;
  return (digit_center_em - sign_center_em) * config.font_size_px * ScriptScale(config, script);
}

Box ExpandBox(const Box& box, float pad_x_px, float pad_y_px) {
  return Box {
    box.x1 - pad_x_px,
    box.y1 - pad_y_px,
    box.x2 + pad_x_px,
    box.y2 + pad_y_px,
  };
}

GlyphShape BuildShape(ShapeKind kind, bool visible, const Box& background_box) {
  GlyphShape shape {};
  shape.kind = kind;
  shape.visible = visible;
  shape.box_px = background_box;
  if (!visible) {
    return shape;
  }
  if (kind == ShapeKind::kEllipse) {
    shape.cx_px = (background_box.x1 + background_box.x2) * 0.5f;
    shape.cy_px = (background_box.y1 + background_box.y2) * 0.5f;
    shape.rx_px = std::max(0.1f, (background_box.x2 - background_box.x1) * 0.5f);
    shape.ry_px = std::max(0.1f, (background_box.y2 - background_box.y1) * 0.5f);
  }
  return shape;
}

Box VisibleBounds(const std::vector<GlyphPlacement>& placements) {
  bool has_box = false;
  Box bounds {};
  for (const GlyphPlacement& placement : placements) {
    if (!placement.visible) {
      continue;
    }
    const Box& box = placement.background_box_px;
    if (!has_box) {
      bounds = box;
      has_box = true;
      continue;
    }
    bounds.x1 = std::min(bounds.x1, box.x1);
    bounds.y1 = std::min(bounds.y1, box.y1);
    bounds.x2 = std::max(bounds.x2, box.x2);
    bounds.y2 = std::max(bounds.y2, box.y2);
  }
  return bounds;
}

void TranslateBox(Box& box, float dx, float dy) {
  box.x1 += dx;
  box.x2 += dx;
  box.y1 += dy;
  box.y2 += dy;
}

void TranslatePlacement(GlyphPlacement& placement, float dx, float dy) {
  placement.origin_x_px += dx;
  placement.baseline_y_px += dy;
  TranslateBox(placement.ink_box_px, dx, dy);
  TranslateBox(placement.background_box_px, dx, dy);
  TranslateBox(placement.shape_px.box_px, dx, dy);
  placement.shape_px.cx_px += dx;
  placement.shape_px.cy_px += dy;
}

}  // namespace

const GlyphProfile& LookupGlyphProfile(std::uint32_t codepoint) {
  const auto& explicit_profiles = ExplicitProfiles();
  const auto it = explicit_profiles.find(codepoint);
  if (it != explicit_profiles.end()) {
    return it->second;
  }
  if (IsAsciiUpper(codepoint)) {
    return DefaultUpperProfile();
  }
  if (IsAsciiLower(codepoint)) {
    return DefaultLowerProfile();
  }
  if (IsDigit(codepoint)) {
    return DefaultDigitProfile();
  }
  return DefaultPunctuationProfile();
}

GlyphPlacement LayoutGlyph(
  const GlyphInput& glyph,
  const LayoutConfig& config,
  float origin_x_px,
  float baseline_y_px
) {
  if (config.font_size_px <= 0.0f) {
    throw std::invalid_argument("font_size_px must be positive");
  }

  const GlyphProfile& profile = LookupGlyphProfile(glyph.codepoint);
  const float scale = config.font_size_px * ScriptScale(config, glyph.script);
  float baseline_y = baseline_y_px + ScriptBaselineShiftPx(config, glyph.script);
  if (IsChargeSign(glyph.codepoint)) {
    baseline_y += ChargeSignBaselineAdjustmentPx(profile, config, glyph.script);
  }

  GlyphPlacement placement {};
  placement.codepoint = glyph.codepoint;
  placement.script = glyph.script;
  placement.visible = profile.visible;
  placement.font_size_px = scale;
  placement.origin_x_px = origin_x_px;
  placement.baseline_y_px = baseline_y;
  placement.advance_px = (profile.advance_em + config.tracking_em) * scale;

  placement.ink_box_px = Box {
    origin_x_px + profile.ink_left_em * scale,
    baseline_y + profile.ink_top_em * scale,
    origin_x_px + profile.ink_right_em * scale,
    baseline_y + profile.ink_bottom_em * scale,
  };
  placement.background_box_px = profile.visible
    ? ExpandBox(placement.ink_box_px, profile.pad_x_em * scale, profile.pad_y_em * scale)
    : Box {};
  placement.shape_px = BuildShape(profile.shape_kind, profile.visible, placement.background_box_px);
  return placement;
}

std::vector<GlyphPlacement> LayoutGlyphRun(
  const std::vector<GlyphInput>& glyphs,
  const LayoutConfig& config,
  float start_x_px,
  float baseline_y_px
) {
  std::vector<GlyphPlacement> placements;
  placements.reserve(glyphs.size());

  float cursor_x = start_x_px;
  for (const GlyphInput& glyph : glyphs) {
    GlyphPlacement placement = LayoutGlyph(glyph, config, cursor_x, baseline_y_px);
    cursor_x += placement.advance_px;
    placements.push_back(placement);
  }
  return placements;
}

std::vector<GlyphPlacement> LayoutGlyphRunAligned(
  const std::vector<GlyphInput>& glyphs,
  const LayoutConfig& config,
  float anchor_origin_x_px,
  float anchor_baseline_y_px,
  std::size_t anchor_glyph_index,
  LabelAlign align
) {
  if (glyphs.empty()) {
    return {};
  }

  const std::vector<GlyphPlacement> probe = LayoutGlyphRun(glyphs, config, 0.0f, anchor_baseline_y_px);
  const std::size_t resolved_anchor_index = ResolveAnchorGlyphIndex(probe, anchor_glyph_index);
  if (resolved_anchor_index == kDefaultAnchorGlyphIndex) {
    return probe;
  }

  if (align == LabelAlign::kRight || align == LabelAlign::kLeft) {
    std::vector<GlyphPlacement> placements = probe;
    const float dx = anchor_origin_x_px - placements[resolved_anchor_index].origin_x_px;
    for (GlyphPlacement& placement : placements) {
      TranslatePlacement(placement, dx, 0.0f);
    }
    return placements;
  }

  std::vector<GlyphPlacement> placements(glyphs.size());
  GlyphPlacement anchor = LayoutGlyph(glyphs[resolved_anchor_index], config, anchor_origin_x_px, anchor_baseline_y_px);
  placements[resolved_anchor_index] = anchor;

  std::vector<std::size_t> other_indices;
  std::vector<GlyphInput> other_glyphs;
  other_indices.reserve(glyphs.size() - 1);
  other_glyphs.reserve(glyphs.size() - 1);
  for (std::size_t index = 0; index < glyphs.size(); index += 1) {
    if (index == resolved_anchor_index) {
      continue;
    }
    other_indices.push_back(index);
    other_glyphs.push_back(glyphs[index]);
  }

  if (other_glyphs.empty()) {
    return placements;
  }

  std::vector<GlyphPlacement> other_placements = LayoutGlyphRun(other_glyphs, config, anchor_origin_x_px, anchor_baseline_y_px);
  const float stack_gap_px = config.font_size_px * 0.02f;

  float dy = 0.0f;

  switch (align) {
    case LabelAlign::kAbove: {
      const Box anchor_bounds = VisibleBounds(std::vector<GlyphPlacement> {anchor});
      const Box other_bounds = VisibleBounds(other_placements);
      dy = anchor_bounds.y1 - stack_gap_px - other_bounds.y2;
      break;
    }
    case LabelAlign::kBelow: {
      const Box anchor_bounds = VisibleBounds(std::vector<GlyphPlacement> {anchor});
      const Box other_bounds = VisibleBounds(other_placements);
      dy = anchor_bounds.y2 + stack_gap_px - other_bounds.y1;
      break;
    }
    case LabelAlign::kLeft:
    case LabelAlign::kRight:
    default:
      break;
  }

  for (std::size_t index = 0; index < other_placements.size(); index += 1) {
    TranslatePlacement(other_placements[index], 0.0f, dy);
    placements[other_indices[index]] = other_placements[index];
  }

  return placements;
}

LabelAnchor LocateGlyphRun(
  const std::vector<GlyphInput>& glyphs,
  const std::vector<GlyphPlacement>& placements,
  const LayoutConfig& config,
  std::size_t anchor_glyph_index
) {
  LabelAnchor anchor {};
  if (glyphs.empty() || placements.empty()) {
    return anchor;
  }

  const std::size_t resolved_index = ResolveAnchorGlyphIndex(placements, anchor_glyph_index);
  if (resolved_index == kDefaultAnchorGlyphIndex) {
    return anchor;
  }

  const GlyphPlacement& placement = placements[resolved_index];
  anchor.valid = true;
  anchor.kind = AnchorKind::kGlyphStandardCenter;
  anchor.glyph_index = resolved_index;
  anchor.point_px = Point {
    (placement.background_box_px.x1 + placement.background_box_px.x2) * 0.5f,
    placement.baseline_y_px + StandardGlyphCenterYOffsetPx(config),
  };
  return anchor;
}

}  // namespace chemcore::glyph
