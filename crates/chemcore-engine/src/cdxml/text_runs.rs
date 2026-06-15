use crate::LabelRun;
use std::collections::BTreeMap;

use super::colors::CdxmlColorTable;
use super::round2;

pub(super) fn label_source_run(
    text: &str,
    face: u32,
    font_id: &str,
    color_id: &str,
    font_size: f64,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
) -> LabelRun {
    let decoded_face = decode_cdxml_face(face);
    LabelRun {
        text: text.to_string(),
        font_family: Some(
            fonts
                .get(font_id)
                .cloned()
                .unwrap_or_else(|| "Arial".to_string()),
        ),
        font_size: Some(round2(font_size)),
        fill: Some(colors.resolve(Some(color_id))),
        font_weight: Some(if decoded_face.bold { 700 } else { 400 }),
        font_style: Some(
            if decoded_face.italic {
                "italic"
            } else {
                "normal"
            }
            .to_string(),
        ),
        underline: Some(decoded_face.underline),
        script: Some(decoded_face.script.to_string()),
    }
}

pub(super) fn label_display_runs(
    text: &str,
    face: u32,
    font_id: &str,
    color_id: &str,
    font_size: f64,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
) -> Vec<LabelRun> {
    let source = label_source_run(text, face, font_id, color_id, font_size, colors, fonts);
    if source.script.as_deref() == Some("chemical") {
        expand_cdxml_chemical_run(&source)
    } else {
        vec![source]
    }
}

struct CdxmlFace {
    bold: bool,
    italic: bool,
    underline: bool,
    script: &'static str,
}

fn decode_cdxml_face(face: u32) -> CdxmlFace {
    let has_subscript = face & 32 != 0;
    let has_superscript = face & 64 != 0;
    let script = match (has_subscript, has_superscript) {
        (true, true) => "chemical",
        (true, false) => "subscript",
        (false, true) => "superscript",
        (false, false) => "normal",
    };
    CdxmlFace {
        bold: face & 1 != 0,
        italic: face & 2 != 0,
        underline: face & 4 != 0,
        script,
    }
}

fn expand_cdxml_chemical_run(base: &LabelRun) -> Vec<LabelRun> {
    let chars: Vec<char> = base.text.chars().collect();
    let mut scripts = vec!["normal"; chars.len()];

    for index in 0..chars.len() {
        let character = chars[index];
        if character.is_ascii_digit()
            && index > 0
            && (chars[index - 1].is_ascii_alphabetic() || chars[index - 1] == ')')
        {
            scripts[index] = "subscript";
        }
        if is_cdxml_charge_marker(&chars, index) {
            scripts[index] = "superscript";
            if index > 0 && chars[index - 1].is_ascii_digit() {
                scripts[index - 1] = "superscript";
            }
        }
    }

    let mut out = Vec::new();
    let mut buffer = String::new();
    let mut active_script = "normal";
    for (index, character) in chars.into_iter().enumerate() {
        let script = scripts[index];
        if !buffer.is_empty() && script != active_script {
            let mut run = base.clone();
            run.text = std::mem::take(&mut buffer);
            run.script = Some(active_script.to_string());
            out.push(run);
        }
        active_script = script;
        buffer.push(character);
    }
    if !buffer.is_empty() {
        let mut run = base.clone();
        run.text = buffer;
        run.script = Some(active_script.to_string());
        out.push(run);
    }
    out
}

fn is_cdxml_charge_marker(chars: &[char], index: usize) -> bool {
    if !matches!(chars.get(index), Some('+' | '-')) {
        return false;
    }
    let previous = index.checked_sub(1).and_then(|offset| chars.get(offset));
    let next = chars.get(index + 1);
    if !matches!(
        previous,
        Some(character) if character.is_alphanumeric() || matches!(character, ')' | ']' | '}')
    ) {
        return false;
    }
    next.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chemical_run(text: &str) -> LabelRun {
        LabelRun {
            text: text.to_string(),
            script: Some("chemical".to_string()),
            ..LabelRun::default()
        }
    }

    #[test]
    fn chemical_runs_keep_internal_hyphen_normal() {
        let runs = expand_cdxml_chemical_run(&chemical_run("t-Bu"));
        assert_eq!(
            runs.iter()
                .map(|run| (run.text.as_str(), run.script.as_deref()))
                .collect::<Vec<_>>(),
            vec![("t-Bu", Some("normal"))]
        );
    }

    #[test]
    fn chemical_runs_keep_terminal_charge_suffix_superscript() {
        let runs = expand_cdxml_chemical_run(&chemical_run("Fe3+"));
        assert_eq!(
            runs.iter()
                .map(|run| (run.text.as_str(), run.script.as_deref()))
                .collect::<Vec<_>>(),
            vec![("Fe", Some("normal")), ("3+", Some("superscript"))]
        );
    }

    #[test]
    fn chemical_runs_keep_mid_label_hyphen_normal_even_after_digits() {
        let runs = expand_cdxml_chemical_run(&chemical_run("CH3-CH2"));
        assert_eq!(
            runs.iter()
                .map(|run| (run.text.as_str(), run.script.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("CH", Some("normal")),
                ("3", Some("subscript")),
                ("-CH", Some("normal")),
                ("2", Some("subscript"))
            ]
        );
    }
}
