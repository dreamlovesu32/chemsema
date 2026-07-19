use super::*;

pub(super) fn payload_text(payload: &crate::ObjectPayload) -> String {
    payload
        .extra
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub(super) fn payload_box(payload: &crate::ObjectPayload) -> Option<[f64; 4]> {
    payload
        .extra
        .get("box")
        .cloned()
        .and_then(|value| serde_json::from_value::<[f64; 4]>(value).ok())
}

pub(super) fn payload_runs_or_text(payload: &crate::ObjectPayload) -> Vec<LabelRun> {
    if let Some(value) = payload.extra.get("runs").cloned() {
        if let Ok(runs) = serde_json::from_value::<Vec<LabelRun>>(value) {
            if !runs.is_empty() {
                return runs;
            }
        }
    }
    let text = payload_text(payload);
    if text.is_empty() {
        Vec::new()
    } else {
        vec![LabelRun {
            text,
            font_family: payload
                .extra
                .get("fontFamily")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            font_size: payload.extra.get("fontSize").and_then(Value::as_f64),
            fill: payload
                .extra
                .get("fill")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            script: Some("normal".to_string()),
        }]
    }
}

pub(super) fn runs_text(runs: &[LabelRun]) -> String {
    runs.iter().map(|run| run.text.as_str()).collect()
}

pub(super) fn normalize_source_runs(session: &TextEditSession, text: &str) -> Vec<LabelRun> {
    let source_runs = if !session.source_runs.is_empty() {
        session.source_runs.clone()
    } else if text.is_empty() {
        Vec::new()
    } else {
        vec![LabelRun {
            text: text.to_string(),
            font_family: session.font_family.clone(),
            font_size: session.font_size,
            fill: session.fill.clone(),
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            script: Some(if session.default_chemical {
                "chemical".to_string()
            } else {
                "normal".to_string()
            }),
        }]
    };
    source_runs
        .into_iter()
        .filter(|run| !run.text.is_empty())
        .map(|mut run| {
            if run.font_family.is_none() {
                run.font_family = session.font_family.clone();
            }
            if run.font_size.is_none() {
                run.font_size = session.font_size;
            }
            if run.fill.is_none() {
                run.fill = session.fill.clone();
            }
            if run.font_weight.is_none() {
                run.font_weight = Some(400);
            }
            if run.font_style.is_none() {
                run.font_style = Some("normal".to_string());
            }
            if run.underline.is_none() {
                run.underline = Some(false);
            }
            if run.script.is_none() {
                run.script = Some(if session.default_chemical {
                    "chemical".to_string()
                } else {
                    "normal".to_string()
                });
            }
            run
        })
        .collect()
}

pub(super) fn source_runs_are_chemical(source_runs: &[LabelRun]) -> bool {
    source_runs
        .iter()
        .any(|run| run.script.as_deref() == Some("chemical"))
}

pub(super) fn display_runs_from_source_runs(
    source_runs: &[LabelRun],
    fallback_font_family: &str,
    fallback_font_size: f64,
    fallback_fill: &str,
) -> Vec<LabelRun> {
    let mut out = Vec::new();
    let mut index = 0;
    while index < source_runs.len() {
        let run = &source_runs[index];
        if run.text.is_empty() {
            index += 1;
            continue;
        }
        match run.script.as_deref().unwrap_or("normal") {
            "chemical" => {
                let start = index;
                while index < source_runs.len()
                    && source_runs[index].script.as_deref() == Some("chemical")
                {
                    index += 1;
                }
                let bases = source_runs[start..index]
                    .iter()
                    .filter(|run| !run.text.is_empty())
                    .map(|run| {
                        display_run_base(
                            run,
                            fallback_font_family,
                            fallback_font_size,
                            fallback_fill,
                        )
                    })
                    .collect::<Vec<_>>();
                out.extend(expand_chemical_runs(&bases));
                continue;
            }
            "subscript" | "superscript" => {
                let mut base =
                    display_run_base(run, fallback_font_family, fallback_font_size, fallback_fill);
                base.script = run.script.clone();
                out.push(base);
            }
            _ => {
                out.push(display_run_base(
                    run,
                    fallback_font_family,
                    fallback_font_size,
                    fallback_fill,
                ));
            }
        }
        index += 1;
    }
    merge_adjacent_runs(out)
}

fn display_run_base(
    run: &LabelRun,
    fallback_font_family: &str,
    fallback_font_size: f64,
    fallback_fill: &str,
) -> LabelRun {
    LabelRun {
        text: run.text.clone(),
        font_family: Some(
            run.font_family
                .clone()
                .unwrap_or_else(|| fallback_font_family.to_string()),
        ),
        font_size: Some(run.font_size.unwrap_or(fallback_font_size)),
        fill: Some(
            run.fill
                .clone()
                .unwrap_or_else(|| fallback_fill.to_string()),
        ),
        font_weight: Some(run.font_weight.unwrap_or(400)),
        font_style: Some(
            run.font_style
                .clone()
                .unwrap_or_else(|| "normal".to_string()),
        ),
        underline: Some(run.underline.unwrap_or(false)),
        script: Some("normal".to_string()),
    }
}

pub(super) fn merge_adjacent_runs(runs: Vec<LabelRun>) -> Vec<LabelRun> {
    let mut merged: Vec<LabelRun> = Vec::new();
    for run in runs {
        if let Some(previous) = merged.last_mut() {
            if previous.font_family == run.font_family
                && previous.font_size == run.font_size
                && previous.fill == run.fill
                && previous.font_weight == run.font_weight
                && previous.font_style == run.font_style
                && previous.script == run.script
            {
                previous.text.push_str(&run.text);
                continue;
            }
        }
        merged.push(run);
    }
    merged
}

#[cfg(test)]
pub(super) fn expand_chemical_run(base: &LabelRun, text: &str) -> Vec<LabelRun> {
    let mut base = base.clone();
    base.text = text.to_string();
    expand_chemical_runs(&[base])
}

fn expand_chemical_runs(base_runs: &[LabelRun]) -> Vec<LabelRun> {
    let chars: Vec<char> = base_runs.iter().flat_map(|run| run.text.chars()).collect();
    let mut scripts = vec!["normal"; chars.len()];

    let mut index = 0usize;
    while index < chars.len() {
        if !chars[index].is_ascii_digit() {
            if is_charge_marker(&chars, index) {
                scripts[index] = "superscript";
            }
            index += 1;
            continue;
        }
        let start = index;
        while index < chars.len() && chars[index].is_ascii_digit() {
            index += 1;
        }
        if index < chars.len() && is_charge_marker(&chars, index) {
            scripts[start..=index].fill("superscript");
            index += 1;
        } else if start > 0 && (chars[start - 1].is_ascii_alphabetic() || chars[start - 1] == ')') {
            scripts[start..index].fill("subscript");
        }
    }

    let mut out = Vec::new();
    let mut char_index = 0;
    for base in base_runs {
        let mut buffer = String::new();
        let mut active_script = "normal";
        for character in base.text.chars() {
            let script = scripts[char_index];
            char_index += 1;
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
    }
    out
}

fn is_charge_marker(chars: &[char], index: usize) -> bool {
    if !matches!(chars.get(index), Some('+' | '-')) {
        return false;
    }
    let previous = index.checked_sub(1).and_then(|offset| chars.get(offset));
    if !matches!(
        previous,
        Some(character) if character.is_alphanumeric() || matches!(character, ')' | ']' | '}')
    ) {
        return false;
    }
    chars.get(index + 1).is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chemical_base() -> LabelRun {
        LabelRun {
            font_family: Some("Arial".to_string()),
            font_size: Some(10.0),
            fill: Some("#000000".to_string()),
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            script: Some("normal".to_string()),
            ..LabelRun::default()
        }
    }

    #[test]
    fn expand_chemical_run_keeps_internal_hyphen_normal() {
        let runs = expand_chemical_run(&chemical_base(), "t-Bu");
        assert_eq!(
            runs.iter()
                .map(|run| (run.text.as_str(), run.script.as_deref()))
                .collect::<Vec<_>>(),
            vec![("t-Bu", Some("normal"))]
        );
    }

    #[test]
    fn expand_chemical_run_keeps_terminal_charge_superscript() {
        let runs = expand_chemical_run(&chemical_base(), "Fe3+");
        assert_eq!(
            runs.iter()
                .map(|run| (run.text.as_str(), run.script.as_deref()))
                .collect::<Vec<_>>(),
            vec![("Fe", Some("normal")), ("3+", Some("superscript"))]
        );
    }

    #[test]
    fn display_runs_classify_subscripts_across_source_run_boundaries() {
        let source_runs = [
            LabelRun {
                text: "OCF".to_string(),
                fill: Some("#ff0000".to_string()),
                script: Some("chemical".to_string()),
                ..LabelRun::default()
            },
            LabelRun {
                text: "3".to_string(),
                fill: Some("#ff0000".to_string()),
                script: Some("chemical".to_string()),
                ..LabelRun::default()
            },
        ];

        let runs = display_runs_from_source_runs(&source_runs, "Arial", 10.0, "#000000");

        assert_eq!(
            runs.iter()
                .map(|run| (
                    run.text.as_str(),
                    run.script.as_deref(),
                    run.fill.as_deref()
                ))
                .collect::<Vec<_>>(),
            vec![
                ("OCF", Some("normal"), Some("#ff0000")),
                ("3", Some("subscript"), Some("#ff0000"))
            ]
        );
    }

    #[test]
    fn expand_chemical_run_styles_complete_multi_digit_counts_and_charges() {
        let counts = expand_chemical_run(&chemical_base(), "C10H21");
        assert_eq!(
            counts
                .iter()
                .map(|run| (run.text.as_str(), run.script.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("C", Some("normal")),
                ("10", Some("subscript")),
                ("H", Some("normal")),
                ("21", Some("subscript")),
            ]
        );

        let charge = expand_chemical_run(&chemical_base(), "Fe10+");
        assert_eq!(
            charge
                .iter()
                .map(|run| (run.text.as_str(), run.script.as_deref()))
                .collect::<Vec<_>>(),
            vec![("Fe", Some("normal")), ("10+", Some("superscript"))]
        );
    }
}
