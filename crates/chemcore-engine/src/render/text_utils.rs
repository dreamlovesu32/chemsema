use super::*;

pub(super) fn split_runs_by_line(runs: &[LabelRun]) -> Vec<Vec<LabelRun>> {
    let mut out = vec![Vec::new()];
    for run in runs {
        let segments: Vec<&str> = run.text.split('\n').collect();
        for (index, segment) in segments.iter().enumerate() {
            if !segment.is_empty() {
                let mut next_run = run.clone();
                next_run.text = (*segment).to_string();
                out.last_mut()
                    .expect("line vector always exists")
                    .push(next_run);
            }
            if index + 1 < segments.len() {
                out.push(Vec::new());
            }
        }
    }
    out
}

pub(super) fn split_preserved_text_lines(text: &str) -> Vec<String> {
    text.split('\n')
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub(super) fn wrap_text_lines(text: &str, max_width: f64, font_size: f64) -> Vec<String> {
    let raw_lines: Vec<&str> = text.split('\n').collect();
    let max_chars = (max_width
        / crate::TEXT_WRAP_ESTIMATED_CHAR_WIDTH_CM
            .value()
            .max(font_size * 0.6))
    .floor()
    .max(8.0) as usize;
    let mut out = Vec::new();

    for raw_line in raw_lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.len() <= max_chars || !line.contains(' ') {
            out.push(line.to_string());
            continue;
        }
        let mut current = String::new();
        for word in line.split_whitespace() {
            let next = if current.is_empty() {
                word.to_string()
            } else {
                format!("{current} {word}")
            };
            if next.len() > max_chars && !current.is_empty() {
                out.push(current);
                current = word.to_string();
            } else {
                current = next;
            }
        }
        if !current.is_empty() {
            out.push(current);
        }
    }

    out
}
