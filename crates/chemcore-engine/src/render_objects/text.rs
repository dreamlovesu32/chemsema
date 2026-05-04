use super::*;
use crate::{DEFAULT_TEXT_FONT_SIZE_CM, DEFAULT_TEXT_LINE_HEIGHT_CM};

pub(crate) fn render_text_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
) {
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    let font_size = payload_number(&object.payload, "fontSize")
        .or_else(|| {
            style.and_then(|value| {
                style_number(value, "fontSize").or_else(|| style_number(value, "font_size"))
            })
        })
        .unwrap_or(DEFAULT_TEXT_FONT_SIZE_CM);
    let line_height =
        payload_number(&object.payload, "lineHeight").unwrap_or(DEFAULT_TEXT_LINE_HEIGHT_CM);
    let align = payload_string(&object.payload, "align").unwrap_or_else(|| "left".to_string());
    let text_anchor = text_anchor(&align);
    let font_family = style
        .and_then(|value| style_string(value, "fontFamily"))
        .or_else(|| Some("Arial".to_string()));
    let fill = style.and_then(|value| style_string(value, "fill"));
    let object_id = Some(object.id.clone());

    if payload_bool(&object.payload, "preserveLines").unwrap_or(false) {
        let runs = payload_runs(&object.payload, "runs");
        if !runs.is_empty() {
            for (index, line_runs) in split_runs_by_line(&runs).into_iter().enumerate() {
                if line_runs.is_empty() {
                    continue;
                }
                push_text(
                    out,
                    tx,
                    ty + font_size * 0.82 + index as f64 * line_height,
                    String::new(),
                    font_size,
                    font_family.clone(),
                    fill.clone(),
                    Some(text_anchor.clone()),
                    line_runs,
                    object_id.clone(),
                );
            }
            return;
        }
        for (index, line) in
            split_preserved_text_lines(&payload_string(&object.payload, "text").unwrap_or_default())
                .into_iter()
                .enumerate()
        {
            push_text(
                out,
                tx,
                ty + font_size * 0.82 + index as f64 * line_height,
                line,
                font_size,
                font_family.clone(),
                fill.clone(),
                Some(text_anchor.clone()),
                Vec::new(),
                object_id.clone(),
            );
        }
        return;
    }

    let box_width = payload_box_width(&object.payload, "box").unwrap_or(px_to_cm(160.0));
    for (index, line) in wrap_text_lines(
        &payload_string(&object.payload, "text").unwrap_or_default(),
        box_width,
        font_size,
    )
    .into_iter()
    .enumerate()
    {
        push_text(
            out,
            tx,
            ty + font_size * 0.82 + index as f64 * line_height,
            line,
            font_size,
            font_family.clone(),
            fill.clone(),
            Some(text_anchor.clone()),
            Vec::new(),
            object_id.clone(),
        );
    }
}
