use super::*;

pub(super) fn parse_cdxml_line_height(value: Option<&str>) -> Option<CdxmlLineHeight> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "variable" => Some(CdxmlLineHeight::Variable),
        "auto" => Some(CdxmlLineHeight::Auto),
        value => value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .map(CdxmlLineHeight::Fixed),
    }
}

pub(super) fn chemdraw_auto_run_line_height(run: &LabelRun, default_font_size: f64) -> f64 {
    let size = run
        .font_size
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(default_font_size);
    let family = run.font_family.as_deref().unwrap_or("Arial");
    let mut ratio = match family.to_ascii_lowercase().as_str() {
        // Measured from independent ChemDraw SVG baselines. These are font
        // metrics, not document- or molecule-specific exceptions.
        "times new roman" => 1.165,
        "calibri" => 1.225,
        _ => 1.15,
    };
    let bold = run.font_weight.unwrap_or(400) >= 600;
    let italic = run.font_style.as_deref() == Some("italic");
    if bold {
        ratio += 0.025;
    } else if italic {
        ratio -= 0.005;
    }
    match run.script.as_deref() {
        Some("superscript") => ratio += if bold { 0.27 } else { 0.265 },
        Some("subscript") => ratio += if bold { 0.17 } else { 0.165 },
        _ => {}
    }
    size * ratio
}

pub(super) fn chemdraw_auto_text_line_height(default_font_size: f64, runs: &[LabelRun]) -> f64 {
    runs.iter()
        .map(|run| chemdraw_auto_run_line_height(run, default_font_size))
        .fold(default_font_size * 1.15, f64::max)
}

pub(super) fn resolved_cdxml_label_line_spacing(
    text: &XmlNode,
    defaults: CdxmlDefaults,
    font_size: f64,
    runs: &[LabelRun],
    line_runs: &[Vec<LabelRun>],
) -> ResolvedCdxmlLineSpacing {
    let value = parse_cdxml_line_height(text.attr("LabelLineHeight"))
        .or_else(|| parse_cdxml_line_height(text.attr("LineHeight")))
        .or(defaults.label_line_height)
        .or(defaults.line_height)
        .unwrap_or(CdxmlLineHeight::Variable);
    match value {
        CdxmlLineHeight::Fixed(value) if value > 1.0 => ResolvedCdxmlLineSpacing {
            line_height: value,
            line_advances: Vec::new(),
            mode: "fixed",
        },
        CdxmlLineHeight::Auto => ResolvedCdxmlLineSpacing {
            line_height: chemdraw_auto_text_line_height(font_size, runs),
            line_advances: Vec::new(),
            mode: "auto",
        },
        _ => {
            let line_advances = crate::variable_text_line_advances(line_runs, font_size);
            ResolvedCdxmlLineSpacing {
                line_height: line_advances
                    .first()
                    .copied()
                    .unwrap_or_else(|| crate::molecule_label_line_advance(font_size)),
                line_advances,
                mode: "variable",
            }
        }
    }
}
