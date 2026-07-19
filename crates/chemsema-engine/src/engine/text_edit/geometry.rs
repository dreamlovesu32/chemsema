use super::*;

pub(super) fn endpoint_label_editor_anchor_world(
    node: &crate::Node,
    object_translate: [f64; 2],
    connection_angles: &[f64],
) -> Option<Point> {
    let label = node.label.as_ref()?;
    let glyph_polygons = label.glyph_polygons();
    if !glyph_polygons.is_empty() {
        if let Some(anchor) = centered_label_anchor_world(label, &glyph_polygons, object_translate)
        {
            return Some(anchor);
        }
        let source_runs = source_runs_from_node_label(label);
        let source_text = if !source_runs.is_empty() {
            runs_text(&source_runs)
        } else {
            label
                .source_text
                .clone()
                .unwrap_or_else(|| label.text.clone())
        };
        let decision = label_layout_decision_for_text_mode(
            &source_text,
            connection_angles,
            source_runs_are_chemical(&source_runs),
        );
        let layout = layout_label_text(&source_text, &decision);
        let font_family = label
            .font_family
            .clone()
            .unwrap_or_else(|| DEFAULT_TEXT_FONT_FAMILY.to_string());
        let font_size = WorldPt(label.font_size.unwrap_or(DEFAULT_TEXT_FONT_SIZE)).value();
        let fill = label
            .fill
            .clone()
            .unwrap_or_else(|| DEFAULT_TEXT_FILL.to_string());
        let display_runs =
            display_runs_from_source_runs(&source_runs, &font_family, font_size, &fill);
        let (_, line_runs) = layout_display_runs(&display_runs, &decision);
        let anchor_index = label_anchor_index_for_layout(&line_runs, &layout);
        if let Some(anchor) = glyph_polygons
            .get(anchor_index)
            .and_then(|polygon| label_editor_anchor_point(label, anchor_index, polygon))
        {
            return Some(Point::new(
                anchor.x + object_translate[0],
                anchor.y + object_translate[1],
            ));
        }
    }
    let bbox = label.bbox()?;
    Some(Point::new(
        bbox[0] + object_translate[0],
        bbox[1] + object_translate[1],
    ))
}

fn centered_label_anchor_world(
    label: &crate::NodeLabel,
    glyph_polygons: &[Vec<Point>],
    object_translate: [f64; 2],
) -> Option<Point> {
    if !label_is_centered(label) {
        return None;
    }
    let bbox = label.bbox()?;
    let center_x = (bbox[0] + bbox[2]) * 0.5;
    let (_, glyph_box) = centered_label_glyph_box(glyph_polygons, center_x)?;
    Some(Point::new(
        center_x + object_translate[0],
        (glyph_box[1] + glyph_box[3]) * 0.5 + object_translate[1],
    ))
}

fn label_is_centered(label: &crate::NodeLabel) -> bool {
    label.layout.as_deref() == Some("attached-group-center")
        || (label.align.as_deref() == Some("center") && label.anchor.as_deref() == Some("middle"))
}

fn centered_label_glyph_box(
    glyph_polygons: &[Vec<Point>],
    center_x: f64,
) -> Option<(usize, [f64; 4])> {
    glyph_polygons
        .iter()
        .enumerate()
        .filter_map(|(index, polygon)| polygon_bounds(polygon).map(|bbox| (index, bbox)))
        .min_by(|(_, left), (_, right)| {
            glyph_center_distance_to_x(*left, center_x)
                .total_cmp(&glyph_center_distance_to_x(*right, center_x))
        })
}

fn glyph_center_distance_to_x(bbox: [f64; 4], x: f64) -> f64 {
    if x >= bbox[0] && x <= bbox[2] {
        0.0
    } else {
        (((bbox[0] + bbox[2]) * 0.5) - x).abs()
    }
}

fn polygon_bounds(polygon: &[Point]) -> Option<[f64; 4]> {
    if polygon.is_empty() {
        return None;
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for point in polygon {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Some([min_x, min_y, max_x, max_y])
}

pub(super) fn polygon_anchor_point(polygon: &[Point]) -> Option<Point> {
    if polygon.is_empty() {
        return None;
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for point in polygon {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Some(Point::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5))
}

fn label_editor_anchor_point(
    label: &crate::NodeLabel,
    glyph_index: usize,
    polygon: &[Point],
) -> Option<Point> {
    let anchor_y = label_line_anchor_y(label);
    let chars = label_visible_chars(label);
    if chars.len() == label.glyph_polygons.len()
        && chars
            .get(glyph_index)
            .copied()
            .is_some_and(crate::is_prime_anchor_suffix)
    {
        if let (Some(bounds), Some(natural_outset)) =
            (polygon_bounds(polygon), label_natural_outset_pt(label))
        {
            return Some(Point::new(
                bounds[2] - natural_outset,
                anchor_y.unwrap_or((bounds[1] + bounds[3]) * 0.5),
            ));
        }
    }
    polygon_anchor_point(polygon).map(|point| Point::new(point.x, anchor_y.unwrap_or(point.y)))
}

fn label_visible_chars(label: &crate::NodeLabel) -> Vec<char> {
    label
        .source_text
        .as_deref()
        .unwrap_or(label.text.as_str())
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn label_natural_outset_pt(label: &crate::NodeLabel) -> Option<f64> {
    [
        "/import/cdxml/naturalOutsetPt",
        "/import/cdxml/marginWidth",
        "/glyphClipProfile/naturalOutsetPt",
    ]
    .into_iter()
    .find_map(|path| label.meta.pointer(path).and_then(serde_json::Value::as_f64))
    .filter(|value| value.is_finite() && *value >= 0.0)
}

fn label_line_anchor_y(label: &crate::NodeLabel) -> Option<f64> {
    let position = label.position?;
    let font_size = label
        .font_size
        .unwrap_or(crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
    Some(position[1] - font_size * crate::MOLECULE_LABEL_ANCHOR_BASELINE_RATIO)
}

pub(super) fn current_node_label_editor_geometry(
    node: &crate::Node,
    object_translate: [f64; 2],
    connection_angles: &[f64],
) -> (Option<[f64; 2]>, Option<[f64; 4]>) {
    let Some(bounds) = endpoint_label_world_bounds(node, object_translate) else {
        return (None, None);
    };
    let anchor_offset =
        endpoint_label_editor_anchor_world(node, object_translate, connection_angles)
            .map(|anchor| [round6(anchor.x - bounds[0]), round6(anchor.y - bounds[1])]);
    (anchor_offset, Some(bounds))
}

pub(super) fn attached_node_label_anchor_world(
    fragment: &crate::MoleculeFragment,
    node_id: &str,
    object_translate: [f64; 2],
    _stroke_width: f64,
) -> Point {
    let Some(node) = fragment.nodes.iter().find(|node| node.id == node_id) else {
        return Point::new(object_translate[0], object_translate[1]);
    };
    Point::new(
        object_translate[0] + node.position[0],
        object_translate[1] + node.position[1],
    )
}
