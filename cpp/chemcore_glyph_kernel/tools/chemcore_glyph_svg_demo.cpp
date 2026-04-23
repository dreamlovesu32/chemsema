#include "chemcore/glyph_kernel.hpp"

#include <algorithm>
#include <fstream>
#include <iomanip>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

namespace {

using chemcore::glyph::GlyphInput;
using chemcore::glyph::GlyphPlacement;
using chemcore::glyph::kDefaultAnchorGlyphIndex;
using chemcore::glyph::LabelAnchor;
using chemcore::glyph::LabelAlign;
using chemcore::glyph::LayoutConfig;
using chemcore::glyph::LayoutGlyphRun;
using chemcore::glyph::LayoutGlyphRunAligned;
using chemcore::glyph::LocateGlyphRun;
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

struct RowRender {
  std::string label;
  std::vector<GlyphInput> inputs;
  std::vector<GlyphPlacement> placements;
  LabelAnchor anchor;
  LabelAlign align = LabelAlign::kRight;
  float min_x = 0.0f;
  float max_x = 0.0f;
  float min_y = 0.0f;
  float max_y = 0.0f;
  float baseline_y = 0.0f;
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

std::string AlignName(LabelAlign align) {
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

std::string PatternLabel(const PatternSpec& spec) {
  std::string label = spec.text;
  if (spec.anchor_index != kDefaultAnchorGlyphIndex) {
    label += " @" + std::to_string(spec.anchor_index);
  }
  if (spec.align != LabelAlign::kRight) {
    label += " #" + AlignName(spec.align);
  }
  return label;
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

std::string EscapeXml(const std::string& text) {
  std::string out;
  out.reserve(text.size());
  for (char ch : text) {
    switch (ch) {
      case '&':
        out += "&amp;";
        break;
      case '<':
        out += "&lt;";
        break;
      case '>':
        out += "&gt;";
        break;
      case '"':
        out += "&quot;";
        break;
      case '\'':
        out += "&apos;";
        break;
      default:
        out.push_back(ch);
        break;
    }
  }
  return out;
}

std::string GlyphText(std::uint32_t codepoint) {
  return EscapeXml(std::string(1, static_cast<char>(codepoint)));
}

std::string FormatFloat(float value) {
  std::ostringstream stream;
  stream << std::fixed << std::setprecision(3) << value;
  return stream.str();
}

float CutCornerSizePx(const GlyphPlacement& placement) {
  const auto& box = placement.shape_px.box_px;
  const float width = box.x2 - box.x1;
  const float height = box.y2 - box.y1;
  return std::max(0.0f, std::min(width, height) * 0.42f);
}

std::string CutCornerPath(const GlyphPlacement& placement) {
  const auto& box = placement.shape_px.box_px;
  const float cut = CutCornerSizePx(placement);
  std::vector<std::pair<float, float>> points;

  switch (placement.shape_px.kind) {
    case ShapeKind::kRectCutTopRight:
      points = {{box.x1, box.y1}, {box.x2 - cut, box.y1}, {box.x2, box.y1 + cut}, {box.x2, box.y2}, {box.x1, box.y2}};
      break;
    case ShapeKind::kRectCutBottomRight:
      points = {{box.x1, box.y1}, {box.x2, box.y1}, {box.x2, box.y2 - cut}, {box.x2 - cut, box.y2}, {box.x1, box.y2}};
      break;
    case ShapeKind::kRectCutTopLeft:
      points = {{box.x1 + cut, box.y1}, {box.x2, box.y1}, {box.x2, box.y2}, {box.x1, box.y2}, {box.x1, box.y1 + cut}};
      break;
    case ShapeKind::kRectCutBottomLeft:
      points = {{box.x1, box.y1}, {box.x2, box.y1}, {box.x2, box.y2}, {box.x1 + cut, box.y2}, {box.x1, box.y2 - cut}};
      break;
    case ShapeKind::kRect:
    case ShapeKind::kEllipse:
    default:
      break;
  }

  std::ostringstream path;
  for (std::size_t index = 0; index < points.size(); index += 1) {
    path << (index == 0 ? "M " : " L ")
         << FormatFloat(points[index].first) << " " << FormatFloat(points[index].second);
  }
  path << " Z";
  return path.str();
}

bool IsCutCornerRect(ShapeKind kind) {
  return kind == ShapeKind::kRectCutTopRight
    || kind == ShapeKind::kRectCutBottomRight
    || kind == ShapeKind::kRectCutTopLeft
    || kind == ShapeKind::kRectCutBottomLeft;
}

std::string ShapeKindName(ShapeKind kind) {
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

RowRender MakeRow(const PatternSpec& spec, const LayoutConfig& config, float start_x_px, float baseline_y_px) {
  RowRender row {};
  row.label = PatternLabel(spec);
  row.align = spec.align;
  row.baseline_y = baseline_y_px;
  row.inputs = ParsePattern(spec.text);
  row.placements = LayoutGlyphRunAligned(row.inputs, config, start_x_px, baseline_y_px, spec.anchor_index, spec.align);
  row.anchor = LocateGlyphRun(row.inputs, row.placements, config, spec.anchor_index);
  row.min_x = start_x_px;
  row.max_x = start_x_px;
  row.min_y = baseline_y_px - config.font_size_px;
  row.max_y = baseline_y_px + config.font_size_px;

  for (const GlyphPlacement& placement : row.placements) {
    row.min_x = std::min(row.min_x, placement.background_box_px.x1);
    row.max_x = std::max(row.max_x, placement.background_box_px.x2);
    row.min_y = std::min(row.min_y, placement.background_box_px.y1);
    row.max_y = std::max(row.max_y, placement.background_box_px.y2);
  }
  return row;
}

std::string RenderSvg(const std::vector<PatternSpec>& patterns) {
  LayoutConfig config {};
  config.font_size_px = 28.0f;

  constexpr float kLeftMargin = 40.0f;
  constexpr float kTopMargin = 40.0f;
  constexpr float kRowGap = 44.0f;
  constexpr float kTitleGap = 26.0f;

  std::vector<RowRender> rows;
  rows.reserve(patterns.size());

  float baseline_y = kTopMargin + 52.0f;
  float min_x = 0.0f;
  float max_x = 0.0f;
  float min_y = 0.0f;
  float max_y = baseline_y;

  for (const PatternSpec& pattern : patterns) {
    RowRender row = MakeRow(pattern, config, kLeftMargin + 120.0f, baseline_y);
    min_x = rows.empty() ? row.min_x : std::min(min_x, row.min_x);
    max_x = rows.empty() ? row.max_x : std::max(max_x, row.max_x);
    min_y = rows.empty() ? row.min_y : std::min(min_y, row.min_y);
    max_y = std::max(max_y, row.max_y);
    rows.push_back(row);
    baseline_y = row.max_y + kRowGap;
  }

  const float width = std::max(760.0f, max_x + kLeftMargin);
  const float height = std::max(320.0f, baseline_y + kTopMargin);

  std::ostringstream svg;
  svg << "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"" << FormatFloat(width)
      << "\" height=\"" << FormatFloat(height) << "\" viewBox=\"0 0 "
      << FormatFloat(width) << " " << FormatFloat(height) << "\">\n";
  svg << "  <rect width=\"100%\" height=\"100%\" fill=\"#050505\"/>\n";
  svg << "  <text x=\"" << FormatFloat(kLeftMargin) << "\" y=\"" << FormatFloat(kTopMargin)
      << "\" fill=\"#f3f3f3\" font-size=\"24\" font-family=\"IBM Plex Sans, Arial, sans-serif\""
      << " dominant-baseline=\"hanging\">chemcore glyph kernel preview</text>\n";
  svg << "  <text x=\"" << FormatFloat(kLeftMargin) << "\" y=\"" << FormatFloat(kTopMargin + kTitleGap)
      << "\" fill=\"#a9a9a9\" font-size=\"13\" font-family=\"IBM Plex Sans, Arial, sans-serif\""
      << " dominant-baseline=\"hanging\">kernel geometry is deterministic; browser SVG text is only a quick preview</text>\n";

  for (const RowRender& row : rows) {
    svg << "  <text x=\"" << FormatFloat(kLeftMargin) << "\" y=\"" << FormatFloat(row.baseline_y)
        << "\" fill=\"#9f9f9f\" font-size=\"18\" font-family=\"IBM Plex Sans, Arial, sans-serif\""
        << " data-role=\"row-label\" dominant-baseline=\"alphabetic\">" << EscapeXml(row.label) << "</text>\n";
  }

  for (const RowRender& row : rows) {
    for (const GlyphPlacement& placement : row.placements) {
      if (!placement.visible) {
        continue;
      }
      if (placement.shape_px.kind == ShapeKind::kEllipse) {
        svg << "  <ellipse cx=\"" << FormatFloat(placement.shape_px.cx_px)
            << "\" cy=\"" << FormatFloat(placement.shape_px.cy_px)
            << "\" rx=\"" << FormatFloat(placement.shape_px.rx_px)
            << "\" ry=\"" << FormatFloat(placement.shape_px.ry_px)
            << "\" fill=\"#ffffff\" data-role=\"glyph-shape\" data-shape=\"ellipse\"/>\n";
      } else if (IsCutCornerRect(placement.shape_px.kind)) {
        svg << "  <path d=\"" << CutCornerPath(placement)
            << "\" fill=\"#ffffff\" data-role=\"glyph-shape\" data-shape=\""
            << ShapeKindName(placement.shape_px.kind) << "\"/>\n";
      } else {
        svg << "  <rect x=\"" << FormatFloat(placement.background_box_px.x1)
            << "\" y=\"" << FormatFloat(placement.background_box_px.y1)
            << "\" width=\"" << FormatFloat(placement.background_box_px.x2 - placement.background_box_px.x1)
            << "\" height=\"" << FormatFloat(placement.background_box_px.y2 - placement.background_box_px.y1)
            << "\" fill=\"#ffffff\" data-role=\"glyph-shape\" data-shape=\"rect\"/>\n";
      }
    }
  }

  for (const RowRender& row : rows) {
    for (const GlyphPlacement& placement : row.placements) {
      if (!placement.visible) {
        continue;
      }
      svg << "  <text x=\"" << FormatFloat(placement.origin_x_px)
          << "\" y=\"" << FormatFloat(placement.baseline_y_px)
          << "\" fill=\"#050505\" font-size=\"" << FormatFloat(placement.font_size_px)
          << "\" font-family=\"TeXGyreHeros, Arial, Helvetica, sans-serif\""
          << " data-role=\"glyph-text\" data-script=\"" << static_cast<int>(placement.script)
          << "\" dominant-baseline=\"alphabetic\">"
          << GlyphText(placement.codepoint) << "</text>\n";
    }
  }

  for (const RowRender& row : rows) {
    if (!row.anchor.valid) {
      continue;
    }
    svg << "  <circle cx=\"" << FormatFloat(row.anchor.point_px.x)
        << "\" cy=\"" << FormatFloat(row.anchor.point_px.y)
        << "\" r=\"3.200\" fill=\"#ffd400\" stroke=\"#050505\" stroke-width=\"0.900\""
        << " data-role=\"label-anchor\" data-anchor-index=\"" << row.anchor.glyph_index
        << "\" data-align=\"" << AlignName(row.align) << "\"/>\n";
  }

  svg << "</svg>\n";
  return svg.str();
}

}  // namespace

int main(int argc, char** argv) {
  std::string output_path = "tmp/chemcore_glyph_kernel_preview.svg";
  std::vector<PatternSpec> patterns {
    ParsePatternSpec("O"),
    ParsePatternSpec("Ca"),
    ParsePatternSpec("Ca@1"),
    ParsePatternSpec("Br"),
    ParsePatternSpec("Ph"),
    ParsePatternSpec("SO_2#right"),
    ParsePatternSpec("O_2S@2#left"),
    ParsePatternSpec("SO_2#above"),
    ParsePatternSpec("SO_2#below"),
    ParsePatternSpec("NH#above"),
    ParsePatternSpec("NH#below"),
    ParsePatternSpec("NTs#above"),
    ParsePatternSpec("NTs#below"),
    ParsePatternSpec("N^3"),
    ParsePatternSpec("Mg^2+"),
    ParsePatternSpec("SO_4^2-"),
    ParsePatternSpec("t-Bu"),
    ParsePatternSpec("HN"),
    ParsePatternSpec("CN"),
  };

  for (int index = 1; index < argc; index += 1) {
    const std::string arg = argv[index];
    if (arg == "--only") {
      patterns.clear();
      continue;
    }
    if (arg == "-o" && index + 1 < argc) {
      output_path = argv[++index];
      continue;
    }
    patterns.push_back(ParsePatternSpec(arg));
  }

  std::ofstream output(output_path, std::ios::out | std::ios::trunc);
  if (!output) {
    return 1;
  }
  output << RenderSvg(patterns);
  return 0;
}
