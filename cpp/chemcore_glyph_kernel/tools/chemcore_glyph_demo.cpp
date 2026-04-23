#include "chemcore/glyph_kernel.hpp"

#include <iomanip>
#include <iostream>
#include <string>
#include <vector>

namespace {

using chemcore::glyph::GlyphInput;
using chemcore::glyph::kDefaultAnchorGlyphIndex;
using chemcore::glyph::LabelAnchor;
using chemcore::glyph::LabelAlign;
using chemcore::glyph::LayoutConfig;
using chemcore::glyph::LayoutGlyphRun;
using chemcore::glyph::LayoutGlyphRunAligned;
using chemcore::glyph::LocateGlyphRun;
using chemcore::glyph::AnchorKind;
using chemcore::glyph::ScriptKind;
using chemcore::glyph::ShapeKind;

bool IsDigit(char ch) {
  return ch >= '0' && ch <= '9';
}

bool IsChargeSign(char ch) {
  return ch == '+' || ch == '-';
}

struct PatternSpec {
  std::string text;
  std::size_t anchor_index = kDefaultAnchorGlyphIndex;
  LabelAlign align = LabelAlign::kRight;
};

bool IsUnsignedInteger(const std::string& text) {
  if (text.empty()) {
    return false;
  }
  for (const char ch : text) {
    if (!IsDigit(ch)) {
      return false;
    }
  }
  return true;
}

PatternSpec ParsePatternSpec(const std::string& arg) {
  PatternSpec spec {};
  std::string text = arg;

  const std::size_t align_marker = text.rfind('#');
  if (align_marker != std::string::npos && align_marker > 0) {
    const std::string suffix = text.substr(align_marker + 1);
    if (suffix == "left") {
      spec.align = LabelAlign::kLeft;
      text = text.substr(0, align_marker);
    } else if (suffix == "above") {
      spec.align = LabelAlign::kAbove;
      text = text.substr(0, align_marker);
    } else if (suffix == "below") {
      spec.align = LabelAlign::kBelow;
      text = text.substr(0, align_marker);
    } else if (suffix == "right") {
      spec.align = LabelAlign::kRight;
      text = text.substr(0, align_marker);
    }
  }

  const std::size_t marker = text.rfind('@');
  if (marker == std::string::npos || marker == 0) {
    spec.text = text;
    return spec;
  }

  const std::string suffix = text.substr(marker + 1);
  if (!IsUnsignedInteger(suffix)) {
    spec.text = text;
    return spec;
  }

  spec.text = text.substr(0, marker);
  spec.anchor_index = static_cast<std::size_t>(std::stoull(suffix));
  return spec;
}

std::vector<GlyphInput> ParsePattern(const std::string& pattern) {
  std::vector<GlyphInput> glyphs;
  ScriptKind pending_script = ScriptKind::kNormal;

  for (std::size_t index = 0; index < pattern.size(); index += 1) {
    const char ch = pattern[index];
    if (ch == '^') {
      pending_script = ScriptKind::kSuperscript;
      continue;
    }
    if (ch == '_') {
      pending_script = ScriptKind::kSubscript;
      continue;
    }
    GlyphInput input {};
    input.codepoint = static_cast<unsigned char>(ch);
    input.script = pending_script;
    glyphs.push_back(input);
    const char next_ch = index + 1 < pattern.size() ? pattern[index + 1] : '\0';
    if (pending_script != ScriptKind::kNormal && IsDigit(ch) && (IsDigit(next_ch) || IsChargeSign(next_ch))) {
      continue;
    }
    pending_script = ScriptKind::kNormal;
  }
  return glyphs;
}

const char* ShapeName(ShapeKind kind) {
  switch (kind) {
    case ShapeKind::kEllipse:
      return "ellipse";
    case ShapeKind::kRectCutTopRight:
      return "rect-cut-top-right";
    case ShapeKind::kRectCutBottomRight:
      return "rect-cut-bottom-right";
    case ShapeKind::kRectCutTopLeft:
      return "rect-cut-top-left";
    case ShapeKind::kRectCutBottomLeft:
      return "rect-cut-bottom-left";
    case ShapeKind::kRect:
    default:
      return "rect";
  }
}

const char* AnchorName(AnchorKind kind) {
  return kind == AnchorKind::kGlyphStandardCenter ? "glyph-standard-center" : "unknown";
}

const char* AlignName(LabelAlign align) {
  switch (align) {
    case LabelAlign::kLeft:
      return "left";
    case LabelAlign::kAbove:
      return "above";
    case LabelAlign::kBelow:
      return "below";
    case LabelAlign::kRight:
    default:
      return "right";
  }
}

void DumpPattern(const PatternSpec& spec, const LayoutConfig& config) {
  const auto inputs = ParsePattern(spec.text);
  const auto placements = LayoutGlyphRunAligned(inputs, config, 0.0f, 0.0f, spec.anchor_index, spec.align);
  const LabelAnchor anchor = LocateGlyphRun(inputs, placements, config, spec.anchor_index);
  std::cout << spec.text;
  if (spec.anchor_index != kDefaultAnchorGlyphIndex) {
    std::cout << " @" << spec.anchor_index;
  }
  std::cout << " align=" << AlignName(spec.align);
  std::cout << '\n';
  for (const auto& placement : placements) {
    const char printable = static_cast<char>(placement.codepoint);
    std::cout
      << "  " << printable
      << " script=" << static_cast<int>(placement.script)
      << " advance=" << std::fixed << std::setprecision(3) << placement.advance_px
      << " ink=[" << placement.ink_box_px.x1 << ", " << placement.ink_box_px.y1
      << ", " << placement.ink_box_px.x2 << ", " << placement.ink_box_px.y2 << "]"
      << " bg=[" << placement.background_box_px.x1 << ", " << placement.background_box_px.y1
      << ", " << placement.background_box_px.x2 << ", " << placement.background_box_px.y2 << "]"
      << " shape=" << ShapeName(placement.shape_px.kind)
      << '\n';
  }
  if (anchor.valid) {
    std::cout
      << "  anchor=" << AnchorName(anchor.kind)
      << " glyph_index=" << anchor.glyph_index
      << " point=[" << anchor.point_px.x << ", " << anchor.point_px.y << "]"
      << '\n';
  }
}

}  // namespace

int main(int argc, char** argv) {
  LayoutConfig config {};

  if (argc <= 1) {
    DumpPattern(ParsePatternSpec("O"), config);
    DumpPattern(ParsePatternSpec("Ca"), config);
    DumpPattern(ParsePatternSpec("Ca@1"), config);
    DumpPattern(ParsePatternSpec("Br"), config);
    DumpPattern(ParsePatternSpec("Ph"), config);
    DumpPattern(ParsePatternSpec("SO_2#right"), config);
    DumpPattern(ParsePatternSpec("O_2S@2#left"), config);
    DumpPattern(ParsePatternSpec("SO_2#above"), config);
    DumpPattern(ParsePatternSpec("SO_2#below"), config);
    DumpPattern(ParsePatternSpec("NH#above"), config);
    DumpPattern(ParsePatternSpec("NH#below"), config);
    DumpPattern(ParsePatternSpec("NTs#above"), config);
    DumpPattern(ParsePatternSpec("NTs#below"), config);
    DumpPattern(ParsePatternSpec("N^3"), config);
    DumpPattern(ParsePatternSpec("Mg^2+"), config);
    DumpPattern(ParsePatternSpec("SO_4^2-"), config);
    DumpPattern(ParsePatternSpec("t-Bu"), config);
    return 0;
  }

  for (int index = 1; index < argc; index += 1) {
    DumpPattern(ParsePatternSpec(argv[index]), config);
  }
  return 0;
}
