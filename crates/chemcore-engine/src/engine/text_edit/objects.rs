use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn update_text_object_fields(
    object: &mut crate::SceneObject,
    x: f64,
    y: f64,
    text: &str,
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    session: &TextEditSession,
    width: f64,
    height: f64,
) -> bool {
    let next_payload = make_text_payload(text, source_runs, display_runs, session, width, height);
    let changed =
        object.transform.translate != [x, y] || object.payload.extra != next_payload.extra;
    if !changed {
        return false;
    }
    object.transform.translate = [round2(x), round2(y)];
    object.payload = next_payload;
    object.style_ref = None;
    object.visible = true;
    object.locked = false;
    true
}

#[allow(clippy::too_many_arguments)]
pub(super) fn make_text_object(
    object_id: &str,
    x: f64,
    y: f64,
    text: &str,
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    session: &TextEditSession,
    width: f64,
    height: f64,
    z_index: i32,
) -> crate::SceneObject {
    crate::SceneObject {
        id: object_id.to_string(),
        object_type: "text".to_string(),
        name: "text".to_string(),
        visible: true,
        locked: false,
        z_index,
        transform: crate::Transform {
            translate: [round2(x), round2(y)],
            rotate: 0.0,
            scale: [1.0, 1.0],
        },
        style_ref: None,
        meta: Value::Null,
        payload: make_text_payload(text, source_runs, display_runs, session, width, height),
    }
}

pub(super) fn make_text_payload(
    text: &str,
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    session: &TextEditSession,
    width: f64,
    height: f64,
) -> crate::ObjectPayload {
    let mut extra = std::collections::BTreeMap::new();
    extra.insert("text".to_string(), Value::String(text.to_string()));
    extra.insert(
        "align".to_string(),
        Value::String(session.align.clone().unwrap_or_else(|| "left".to_string())),
    );
    extra.insert("valign".to_string(), Value::String("top".to_string()));
    extra.insert("preserveLines".to_string(), Value::Bool(true));
    extra.insert(
        "fontFamily".to_string(),
        Value::String(
            session
                .font_family
                .clone()
                .unwrap_or_else(|| "Arial".to_string()),
        ),
    );
    extra.insert(
        "fontSize".to_string(),
        json!(round6(
            session
                .font_size_world_cm()
                .unwrap_or(WorldCm(DEFAULT_TEXT_FONT_SIZE))
                .value()
        )),
    );
    extra.insert(
        "fill".to_string(),
        Value::String(
            session
                .fill
                .clone()
                .unwrap_or_else(|| "#000000".to_string()),
        ),
    );
    extra.insert(
        "lineHeight".to_string(),
        json!(round6(
            session
                .line_height
                .unwrap_or(DEFAULT_TEXT_BLOCK_LINE_HEIGHT)
        )),
    );
    extra.insert("box".to_string(), json!([0.0, 0.0, width, height]));
    extra.insert(
        "runs".to_string(),
        serde_json::to_value(display_runs).unwrap_or(Value::Array(Vec::new())),
    );
    extra.insert(
        "sourceRuns".to_string(),
        serde_json::to_value(source_runs).unwrap_or(Value::Array(Vec::new())),
    );
    crate::ObjectPayload {
        resource_ref: None,
        bbox: Some([0.0, 0.0, width, height]),
        extra,
    }
}

pub(crate) fn text_object_world_bounds(object: &crate::SceneObject) -> Option<[f64; 4]> {
    let local_box = payload_box(&object.payload).or(object
        .payload
        .bbox
        .map(|bbox| [bbox[0], bbox[1], bbox[2], bbox[3]]))?;
    let x = object.transform.translate[0] + local_box[0];
    let y = object.transform.translate[1] + local_box[1];
    Some([x, y, x + local_box[2], y + local_box[3]])
}

pub(crate) fn endpoint_label_world_bounds(
    node: &crate::Node,
    object_translate: [f64; 2],
) -> Option<[f64; 4]> {
    let bbox = node.label.as_ref()?.bbox()?;
    Some([
        round6(bbox[0] + object_translate[0]),
        round6(bbox[1] + object_translate[1]),
        round6(bbox[2] + object_translate[0]),
        round6(bbox[3] + object_translate[1]),
    ])
}
