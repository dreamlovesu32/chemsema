use super::*;

pub fn anchor_from_point(document: &ChemcoreDocument, point: Point) -> Option<BondAnchor> {
    if let Some(hit) = hit_test_endpoint(document, point, ENDPOINT_HIT_RADIUS) {
        return Some(BondAnchor {
            node_id: Some(hit.node_id),
            object_id: Some(hit.object_id),
            point: hit.point,
            label_anchor: hit.label_anchor,
        });
    }
    document.editable_fragment()?;
    Some(BondAnchor {
        node_id: None,
        object_id: document
            .editable_fragment()
            .map(|entry| entry.object.id.clone()),
        point,
        label_anchor: None,
    })
}

fn editable_fragment_for_anchor<'a>(
    document: &'a ChemcoreDocument,
    anchor: &BondAnchor,
) -> Option<EditableFragment<'a>> {
    if let Some(object_id) = anchor.object_id.as_deref() {
        if let Some(entry) = document
            .editable_fragments()
            .into_iter()
            .find(|entry| entry.object.id == object_id)
        {
            return Some(entry);
        }
    }
    if let Some(node_id) = anchor.node_id.as_deref() {
        if let Some(entry) = document
            .editable_fragments()
            .into_iter()
            .find(|entry| entry.fragment.nodes.iter().any(|node| node.id == node_id))
        {
            return Some(entry);
        }
    }
    document.editable_fragment()
}

pub fn adjacent_directions(entry: &EditableFragment<'_>, node_id: &str) -> Vec<f64> {
    let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == node_id) else {
        return Vec::new();
    };
    let point = entry.world_point_for_node(node);
    let mut out = Vec::new();
    for bond in &entry.fragment.bonds {
        if bond.begin != node_id && bond.end != node_id {
            continue;
        }
        let other_id = if bond.begin == node_id {
            &bond.end
        } else {
            &bond.begin
        };
        let Some(other) = entry
            .fragment
            .nodes
            .iter()
            .find(|node| &node.id == other_id)
        else {
            continue;
        };
        out.push(angle_between(point, entry.world_point_for_node(other)));
    }
    out
}

pub(super) fn default_angle_for_anchor_with_single_neighbor_delta(
    document: &ChemcoreDocument,
    anchor: &BondAnchor,
    single_neighbor_delta: f64,
) -> f64 {
    let Some(node_id) = &anchor.node_id else {
        return BLANK_CANVAS_DEFAULT_ANGLE;
    };
    let Some(entry) = editable_fragment_for_anchor(document, anchor) else {
        return BLANK_CANVAS_DEFAULT_ANGLE;
    };
    let directions = adjacent_directions(&entry, node_id);
    match directions.len() {
        0 => 0.0,
        1 => {
            if (single_neighbor_delta - 180.0).abs() < 1.0e-9 {
                return normalize_angle(directions[0] + 180.0);
            }
            let a = normalize_angle(directions[0] + single_neighbor_delta);
            let b = normalize_angle(directions[0] - single_neighbor_delta);
            if connected_component_bond_count(&entry, node_id) <= 1 {
                right_preferred_angle(a, b)
            } else {
                preferred_continuation_angle(&entry, node_id, anchor.point, a, b)
            }
        }
        _ => largest_angular_gap(&directions).center,
    }
}

pub(super) fn right_preferred_angle(a: f64, b: f64) -> f64 {
    let da = direction_from_angle(a);
    let db = direction_from_angle(b);
    if da.x >= db.x {
        a
    } else {
        b
    }
}

pub(super) fn preferred_continuation_angle(
    entry: &EditableFragment<'_>,
    anchor_node_id: &str,
    anchor_point: Point,
    a: f64,
    b: f64,
) -> f64 {
    let component_node_ids = connected_component_node_ids(entry, anchor_node_id);
    let component_bonds = connected_component_bond_segments(entry, &component_node_ids);
    let a_distance = candidate_distance_to_other_bonds(
        entry,
        &component_node_ids,
        &component_bonds,
        anchor_node_id,
        anchor_point,
        a,
    );
    let b_distance = candidate_distance_to_other_bonds(
        entry,
        &component_node_ids,
        &component_bonds,
        anchor_node_id,
        anchor_point,
        b,
    );
    if (a_distance - b_distance).abs() <= 1.0e-9 {
        right_preferred_angle(a, b)
    } else if a_distance < b_distance {
        a
    } else {
        b
    }
}

pub(super) fn connected_component_bond_count(entry: &EditableFragment<'_>, node_id: &str) -> usize {
    let component_node_ids = connected_component_node_ids(entry, node_id);
    entry
        .fragment
        .bonds
        .iter()
        .filter(|bond| {
            component_node_ids.contains(bond.begin.as_str())
                && component_node_ids.contains(bond.end.as_str())
        })
        .count()
}

pub(super) fn connected_component_node_ids(
    entry: &EditableFragment<'_>,
    node_id: &str,
) -> HashSet<String> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue = VecDeque::new();
    visited.insert(node_id.to_string());
    queue.push_back(node_id.to_string());

    while let Some(current) = queue.pop_front() {
        for bond in &entry.fragment.bonds {
            let neighbor = if bond.begin == current {
                Some(bond.end.as_str())
            } else if bond.end == current {
                Some(bond.begin.as_str())
            } else {
                None
            };
            let Some(neighbor) = neighbor else {
                continue;
            };
            if visited.insert(neighbor.to_string()) {
                queue.push_back(neighbor.to_string());
            }
        }
    }

    visited
}

pub(super) fn connected_component_bond_segments(
    entry: &EditableFragment<'_>,
    component_node_ids: &HashSet<String>,
) -> Vec<(String, String, Point, Point)> {
    entry
        .fragment
        .bonds
        .iter()
        .filter_map(|bond| {
            if !component_node_ids.contains(bond.begin.as_str())
                || !component_node_ids.contains(bond.end.as_str())
            {
                return None;
            }
            let begin = node_by_id(&entry.fragment.nodes, &bond.begin)?;
            let end = node_by_id(&entry.fragment.nodes, &bond.end)?;
            Some((
                bond.begin.clone(),
                bond.end.clone(),
                entry.world_point_for_node(begin),
                entry.world_point_for_node(end),
            ))
        })
        .collect()
}

pub(super) fn candidate_distance_to_other_bonds(
    entry: &EditableFragment<'_>,
    component_node_ids: &HashSet<String>,
    component_bonds: &[(String, String, Point, Point)],
    anchor_node_id: &str,
    anchor_point: Point,
    candidate_angle: f64,
) -> f64 {
    let candidate_endpoint =
        anchor_point.translated(direction_from_angle(candidate_angle).scaled(DEFAULT_BOND_LENGTH));
    let snapped_target = component_node_ids
        .iter()
        .filter(|node_id| node_id.as_str() != anchor_node_id)
        .filter_map(|node_id| node_by_id(&entry.fragment.nodes, node_id))
        .find_map(|node| {
            let point = entry.world_point_for_node(node);
            if point.distance(candidate_endpoint) <= 1.0e-6
                && node.element == "C"
                && node.atomic_number == 6
                && !node.is_placeholder
            {
                Some(point)
            } else {
                None
            }
        })
        .unwrap_or(candidate_endpoint);

    component_bonds
        .iter()
        .filter(|(begin_id, end_id, _, _)| begin_id != anchor_node_id && end_id != anchor_node_id)
        .map(|(_, _, begin, end)| point_to_segment_distance(snapped_target, *begin, *end))
        .min_by(|left, right| left.total_cmp(right))
        .unwrap_or(f64::INFINITY)
}

pub fn default_angle_for_anchor(document: &ChemcoreDocument, anchor: &BondAnchor) -> f64 {
    default_angle_for_anchor_with_single_neighbor_delta(document, anchor, 120.0)
}

pub fn default_angle_for_anchor_for_variant(
    document: &ChemcoreDocument,
    anchor: &BondAnchor,
    bond_variant: BondVariant,
) -> f64 {
    if bond_variant == BondVariant::Triple {
        if let Some(node_id) = &anchor.node_id {
            if let Some(entry) = editable_fragment_for_anchor(document, anchor) {
                let directions = adjacent_directions(&entry, node_id);
                if directions.len() == 1 {
                    return normalize_angle(directions[0] + 180.0);
                }
            }
        }
    }
    let single_neighbor_delta = if bond_variant == BondVariant::Triple {
        180.0
    } else {
        120.0
    };
    default_angle_for_anchor_with_single_neighbor_delta(document, anchor, single_neighbor_delta)
}

pub fn snapped_angle_for_anchor(
    document: &ChemcoreDocument,
    anchor: &BondAnchor,
    mouse: Point,
) -> f64 {
    let mouse_angle = angle_between(anchor.point, mouse);
    let directions = anchor
        .node_id
        .as_ref()
        .and_then(|node_id| {
            editable_fragment_for_anchor(document, anchor)
                .map(|entry| adjacent_directions(&entry, node_id))
        })
        .unwrap_or_default();

    if directions.is_empty() {
        return nearest_angle(mouse_angle, GLOBAL_SNAP_ANGLES);
    }

    let mut candidates = HashSet::new();
    for angle in GLOBAL_SNAP_ANGLES {
        candidates.insert((*angle * 1000.0).round() as i32);
    }
    for base in &directions {
        for relative in RELATIVE_BOND_ANGLES {
            candidates.insert((normalize_angle(base + relative) * 1000.0).round() as i32);
            candidates.insert((normalize_angle(base - relative) * 1000.0).round() as i32);
        }
    }

    let gap = largest_angular_gap(&directions);
    let mut best = 0.0;
    let mut best_score = f64::INFINITY;
    for candidate_key in candidates {
        let candidate = candidate_key as f64 / 1000.0;
        let mut score = angular_distance(candidate, mouse_angle);
        if directions.len() >= 2 && !angle_in_clockwise_arc(candidate, gap.start, gap.end) {
            score += 25.0;
        }
        if directions.len() >= 2 {
            let satisfied = directions
                .iter()
                .filter(|direction| {
                    RELATIVE_BOND_ANGLES.iter().any(|allowed| {
                        (angular_distance(candidate, **direction) - allowed).abs() < 0.001
                    })
                })
                .count();
            score += (directions.len() - satisfied) as f64 * 8.0;
        }
        if score < best_score {
            best_score = score;
            best = candidate;
        }
    }
    normalize_angle(best)
}

pub fn endpoint_from_angle(anchor: &BondAnchor, angle: f64, length: f64) -> Point {
    anchor
        .point
        .translated(direction_from_angle(angle).scaled(length))
}

pub fn endpoint_from_angle_for_document(
    document: &ChemcoreDocument,
    anchor: &BondAnchor,
    angle: f64,
    length: f64,
) -> Point {
    resolved_anchor_point_for_angle(document, anchor, angle)
        .translated(direction_from_angle(angle).scaled(length))
}

pub fn nearest_angle(target: f64, candidates: &[f64]) -> f64 {
    candidates
        .iter()
        .copied()
        .min_by(|a, b| angular_distance(*a, target).total_cmp(&angular_distance(*b, target)))
        .unwrap_or(0.0)
}

pub fn node_by_id<'a>(nodes: &'a [Node], node_id: &str) -> Option<&'a Node> {
    nodes.iter().find(|node| node.id == node_id)
}
