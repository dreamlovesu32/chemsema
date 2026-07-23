use super::*;

pub(crate) fn render_curve_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
) {
    let Some(values) = object
        .payload
        .extra
        .get("curvePoints")
        .and_then(JsonValue::as_array)
    else {
        return;
    };
    let points: Vec<_> = values
        .iter()
        .filter_map(|value| {
            let pair = value.as_array()?;
            Some(Point::new(
                pair.first()?.as_f64()? + object.transform.translate[0],
                pair.get(1)?.as_f64()? + object.transform.translate[1],
            ))
        })
        .collect();
    if points.len() < 6 || (points.len() - 3) % 3 != 0 {
        return;
    }
    let body = &points[1..points.len() - 1];
    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    let stroke = style
        .and_then(|value| style_string(value, "stroke"))
        .unwrap_or_else(|| "#000000".to_string());
    let stroke_width = style
        .and_then(|value| style_number(value, "strokeWidth"))
        .unwrap_or(crate::DEFAULT_BOND_STROKE);
    let dash_array = style
        .and_then(|value| style_number_array(value, "dashArray"))
        .unwrap_or_default();
    let mut d = format!("M {:.4} {:.4}", body[0].x, body[0].y);
    for segment in body[1..].chunks_exact(3) {
        d.push_str(&format!(
            " C {:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
            segment[0].x, segment[0].y, segment[1].x, segment[1].y, segment[2].x, segment[2].y,
        ));
    }
    if payload_bool(&object.payload, "closed").unwrap_or(false) {
        d.push_str(" Z");
    }
    out.push(RenderPrimitive::Path {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object.id.clone()),
        bond_id: None,
        d,
        points: body.to_vec(),
        stroke: stroke.clone(),
        stroke_width,
        dash_array,
        line_cap: Some("butt".to_string()),
        line_join: Some("round".to_string()),
        rotate: object.transform.rotate,
        rotate_center: None,
    });
    if !payload_string(&object.payload, "arrowheadType")
        .unwrap_or_else(|| "Solid".to_string())
        .eq_ignore_ascii_case("solid")
    {
        return;
    }
    let length = payload_number(&object.payload, "headLength")
        .unwrap_or(crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO);
    let center_length = payload_number(&object.payload, "headCenterLength")
        .unwrap_or(crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO * 0.875);
    let width = payload_number(&object.payload, "headWidth")
        .unwrap_or(crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO * 0.25);
    let head = payload_string(&object.payload, "head").unwrap_or_else(|| "none".to_string());
    if head != "none" {
        let end = *body.last().unwrap_or(&body[0]);
        let tangent = body[body.len() - 2];
        super::arrows::render_curve_solid_arrow_head(
            out,
            tangent,
            end,
            length,
            center_length,
            width,
            head == "half",
            stroke_width,
            &stroke,
            Some(object.id.clone()),
        );
    }
    let tail = payload_string(&object.payload, "tail").unwrap_or_else(|| "none".to_string());
    if tail != "none" {
        super::arrows::render_curve_solid_arrow_head(
            out,
            body[1],
            body[0],
            length,
            center_length,
            width,
            tail == "half",
            stroke_width,
            &stroke,
            Some(object.id.clone()),
        );
    }
}
