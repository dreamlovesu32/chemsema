use super::*;

pub(super) fn terminal_node_drag_axis(
    entry: crate::EditableFragment<'_>,
    node_id: &str,
) -> Option<(Point, f64)> {
    let incident: Vec<_> = entry
        .fragment
        .bonds
        .iter()
        .filter(|bond| bond.begin == node_id || bond.end == node_id)
        .collect();
    if incident.len() != 1 {
        return None;
    }
    let bond = incident[0];
    let other_id = if bond.begin == node_id {
        &bond.end
    } else {
        &bond.begin
    };
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == node_id)?;
    let other = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == *other_id)?;
    let node_point = entry.world_point_for_node(node);
    let pivot = entry.world_point_for_node(other);
    let length = if pivot.distance(node_point) <= crate::EPSILON {
        DEFAULT_BOND_LENGTH
    } else {
        pivot.distance(node_point)
    };
    Some((pivot, length))
}

pub(super) fn selection_drag_changes_document(
    drag: &SelectionMoveDrag,
    point: Point,
    alt_key: bool,
) -> bool {
    match &drag.mode {
        SelectionMoveMode::TerminalNode { pivot, length, .. } => {
            if point.distance(drag.start) <= crate::EPSILON {
                return false;
            }
            let target = terminal_drag_target(*pivot, *length, point, alt_key);
            drag.node_originals.iter().any(|node| {
                (node.position[0] - target.x).abs() > 1.0e-9
                    || (node.position[1] - target.y).abs() > 1.0e-9
            })
        }
        SelectionMoveMode::Translate => point.distance(drag.start) > crate::EPSILON,
    }
}

pub(super) fn apply_selection_drag_to_document(
    engine: &mut Engine,
    drag: &SelectionMoveDrag,
    point: Point,
    alt_key: bool,
) {
    let delta_x = point.x - drag.start.x;
    let delta_y = point.y - drag.start.y;

    for original in &drag.text_originals {
        let Some(object) = engine
            .state
            .document
            .objects
            .iter_mut()
            .find(|object| object.id == original.object_id)
        else {
            continue;
        };
        if matches!(drag.mode, SelectionMoveMode::Translate) {
            object.transform.translate = [
                round2(original.translate[0] + delta_x),
                round2(original.translate[1] + delta_y),
            ];
        }
    }

    let stroke_width = engine.options.bond_stroke_world_cm().value();
    let Some(mut entry) = engine.state.document.editable_fragment_mut() else {
        return;
    };
    let object_translate = entry.object.transform.translate;
    match &drag.mode {
        SelectionMoveMode::Translate => {
            for original in &drag.node_originals {
                if let Some(node) = entry
                    .fragment
                    .nodes
                    .iter_mut()
                    .find(|node| node.id == original.node_id)
                {
                    node.position = [
                        round2(original.position[0] + delta_x),
                        round2(original.position[1] + delta_y),
                    ];
                }
            }
        }
        SelectionMoveMode::TerminalNode {
            node_id,
            pivot,
            length,
        } => {
            let target = terminal_drag_target(*pivot, *length, point, alt_key);
            if let Some(node) = entry
                .fragment
                .nodes
                .iter_mut()
                .find(|node| node.id == *node_id)
            {
                node.position = [
                    round2(target.x - object_translate[0]),
                    round2(target.y - object_translate[1]),
                ];
            }
        }
    }
    refresh_attached_node_label_geometry_for_all_nodes(
        entry.fragment,
        object_translate,
        stroke_width,
    );
    entry.update_bounds();
}

pub(super) fn selection_rotate_delta_degrees(
    drag: &SelectionRotateDrag,
    point: Point,
    alt_key: bool,
) -> f64 {
    let raw = signed_angle_delta(drag.start_angle, angle_between(drag.center, point));
    if alt_key {
        return raw;
    }
    (raw / 15.0).round() * 15.0
}

pub(super) fn signed_angle_delta(start: f64, end: f64) -> f64 {
    let mut delta = (end - start) % 360.0;
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta <= -180.0 {
        delta += 360.0;
    }
    delta
}

pub(super) fn rotate_point_around(point: Point, center: Point, degrees: f64) -> Point {
    let radians = degrees.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    Point::new(
        center.x + dx * cos - dy * sin,
        center.y + dx * sin + dy * cos,
    )
}

pub(super) fn apply_selection_rotation_to_document(
    engine: &mut Engine,
    drag: &SelectionRotateDrag,
    angle: f64,
) {
    for original in &drag.text_originals {
        let Some(object) = engine
            .state
            .document
            .objects
            .iter_mut()
            .find(|object| object.id == original.object_id)
        else {
            continue;
        };
        let next = rotate_point_around(
            Point::new(original.translate[0], original.translate[1]),
            drag.center,
            angle,
        );
        object.transform.translate = [round2(next.x), round2(next.y)];
    }

    let stroke_width = engine.options.bond_stroke_world_cm().value();
    let Some(mut entry) = engine.state.document.editable_fragment_mut() else {
        return;
    };
    let object_translate = entry.object.transform.translate;
    for original in &drag.node_originals {
        let original_world = Point::new(
            object_translate[0] + original.position[0],
            object_translate[1] + original.position[1],
        );
        let next = rotate_point_around(original_world, drag.center, angle);
        if let Some(node) = entry
            .fragment
            .nodes
            .iter_mut()
            .find(|node| node.id == original.node_id)
        {
            node.position = [
                round2(next.x - object_translate[0]),
                round2(next.y - object_translate[1]),
            ];
        }
    }
    refresh_attached_node_label_geometry_for_all_nodes(
        entry.fragment,
        object_translate,
        stroke_width,
    );
    entry.update_bounds();
}

pub(super) fn terminal_drag_target(
    pivot: Point,
    length: f64,
    point: Point,
    alt_key: bool,
) -> Point {
    if alt_key {
        return point;
    }
    let angle = nearest_angle(angle_between(pivot, point), GLOBAL_SNAP_ANGLES);
    pivot.translated(direction_from_angle(angle).scaled(length))
}
