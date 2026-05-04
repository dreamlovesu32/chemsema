use super::*;

pub(super) fn clear_select_hover_overlay(engine: &mut Engine) {
    engine.state.overlay.hover_bond_center = None;
    engine.state.overlay.hover_arrow = None;
    engine.state.overlay.hover_text_box = None;
    engine.state.overlay.hover_endpoint = None;
    engine.state.overlay.preview = None;
}

pub(super) fn render_selected_text_boxes(engine: &Engine, out: &mut Vec<RenderPrimitive>) {
    let selected_text_objects: BTreeSet<&str> = engine
        .state
        .selection
        .text_objects
        .iter()
        .map(String::as_str)
        .collect();
    for object in &engine.state.document.objects {
        if !selected_text_objects.contains(object.id.as_str()) {
            continue;
        }
        let Some(bounds) = text_object_world_bounds(object) else {
            continue;
        };
        push_selection_box(
            out,
            AxisBounds::from_array(bounds),
            RenderRole::SelectionTextBox,
        );
    }
}

pub(super) fn render_selected_arrow_handles(engine: &Engine, out: &mut Vec<RenderPrimitive>) {
    for object in &engine.state.document.objects {
        if !engine.state.selection.arrow_objects.contains(&object.id) {
            continue;
        }
        if let Some(bounds) = scene_object_selection_bounds(object) {
            push_selection_box(out, bounds, RenderRole::SelectionBox);
        }
    }
}

pub(super) fn scene_object_selection_bounds(object: &crate::SceneObject) -> Option<AxisBounds> {
    if matches!(object.object_type.as_str(), "bracket" | "symbol" | "shape") {
        return object_bbox_selection_bounds(object)
            .map(|bounds| bounds.expanded(crate::px_to_cm(3.0)));
    }
    arrow_object_selection_bounds(object)
}

pub(super) fn arrow_object_selection_bounds(object: &crate::SceneObject) -> Option<AxisBounds> {
    let points = line_object_points(object);
    if points.len() < 2 {
        return None;
    }
    let mut handles = arrow_object_handle_points(object, &points).into_iter();
    let first = handles.next()?;
    let mut bounds = AxisBounds::around_point(first, 0.0);
    for handle in handles {
        bounds.include_point(handle);
    }
    Some(bounds.expanded(crate::px_to_cm(4.0)))
}

pub(super) fn object_bbox_selection_bounds(object: &crate::SceneObject) -> Option<AxisBounds> {
    let [x, y, width, height] = object.payload.bbox?;
    if width <= crate::EPSILON || height <= crate::EPSILON {
        return None;
    }
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    Some(AxisBounds::new(
        tx + x,
        ty + y,
        tx + x + width,
        ty + y + height,
    ))
}

pub(super) fn render_selected_fragment_content(engine: &Engine, out: &mut Vec<RenderPrimitive>) {
    let Some(entry) = engine.state.document.editable_fragment() else {
        return;
    };

    for component in selected_component_summaries(engine) {
        let items = component_selection_items(&engine.state.document, &entry, &component);
        if items.is_empty() {
            continue;
        }
        if items.len() == 1 {
            let item = items[0];
            push_selection_item_box(out, item);
            push_selection_bond_dot(out, item.center);
            continue;
        }
        let group_bounds = items.iter().skip(1).fold(items[0].bounds, |mut acc, item| {
            acc.include_bounds(item.bounds);
            acc
        });
        push_selection_box(out, group_bounds, RenderRole::SelectionBox);
        for item in items {
            push_selection_bond_dot(out, item.center);
        }
    }
}

pub(super) fn selected_component_summaries(engine: &Engine) -> Vec<ComponentSelection> {
    let Some(entry) = engine.state.document.editable_fragment() else {
        return Vec::new();
    };
    let selected_nodes: BTreeSet<&str> = engine
        .state
        .selection
        .nodes
        .iter()
        .map(String::as_str)
        .collect();
    let selected_bonds: BTreeSet<&str> = engine
        .state
        .selection
        .bonds
        .iter()
        .map(String::as_str)
        .collect();
    let selected_label_nodes: BTreeSet<&str> = engine
        .state
        .selection
        .label_nodes
        .iter()
        .map(String::as_str)
        .collect();
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut components = Vec::new();

    for node in &entry.fragment.nodes {
        if visited.contains(&node.id) {
            continue;
        }
        let component_node_ids = connected_component_node_ids(entry.fragment, &node.id);
        for node_id in &component_node_ids {
            visited.insert(node_id.clone());
        }
        let component_bond_ids: Vec<String> = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| {
                component_node_ids.contains(&bond.begin) && component_node_ids.contains(&bond.end)
            })
            .map(|bond| bond.id.clone())
            .collect();

        let component_selected_nodes: Vec<String> = component_node_ids
            .iter()
            .filter(|node_id| selected_nodes.contains(node_id.as_str()))
            .cloned()
            .collect();
        let component_selected_label_nodes: Vec<String> = component_node_ids
            .iter()
            .filter(|node_id| selected_label_nodes.contains(node_id.as_str()))
            .cloned()
            .collect();
        let component_selected_bonds: Vec<String> = component_bond_ids
            .iter()
            .filter(|bond_id| selected_bonds.contains(bond_id.as_str()))
            .cloned()
            .collect();
        if component_selected_nodes.is_empty()
            && component_selected_label_nodes.is_empty()
            && component_selected_bonds.is_empty()
        {
            continue;
        }
        components.push(ComponentSelection {
            node_ids: component_selected_nodes,
            label_node_ids: component_selected_label_nodes,
            bond_ids: component_selected_bonds,
        });
    }
    components
}

pub(super) fn bracket_object_ids_containing_component(
    document: &crate::ChemcoreDocument,
    entry: &crate::EditableFragment<'_>,
    component_node_ids: &[String],
) -> Vec<String> {
    let mut sample_points = Vec::new();
    for node_id in component_node_ids {
        if let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == *node_id) {
            sample_points.push(entry.world_point_for_node(node));
        }
    }
    for bond in &entry.fragment.bonds {
        if !component_node_ids.contains(&bond.begin) || !component_node_ids.contains(&bond.end) {
            continue;
        }
        let Some(begin) = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)
        else {
            continue;
        };
        let Some(end) = entry.fragment.nodes.iter().find(|node| node.id == bond.end) else {
            continue;
        };
        sample_points.push(midpoint(
            entry.world_point_for_node(begin),
            entry.world_point_for_node(end),
        ));
    }

    document
        .objects
        .iter()
        .filter(|object| object.object_type == "bracket" && object.visible)
        .filter_map(|object| {
            let bounds = object_bbox_selection_bounds(object)?;
            if sample_points
                .iter()
                .any(|point| point_in_bounds(*point, bounds))
            {
                Some(object.id.clone())
            } else {
                None
            }
        })
        .collect()
}

pub(super) fn component_selection_items(
    document: &crate::ChemcoreDocument,
    entry: &crate::EditableFragment<'_>,
    component: &ComponentSelection,
) -> Vec<FragmentSelectionItem> {
    let mut items = Vec::new();
    for node_id in &component.label_node_ids {
        let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == *node_id) else {
            continue;
        };
        let Some(bounds) = endpoint_label_world_bounds(node, entry.object.transform.translate)
        else {
            continue;
        };
        items.push(FragmentSelectionItem {
            kind: FragmentItemKind::Label,
            bounds: AxisBounds::from_array(bounds),
            center: Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5),
        });
    }
    for node_id in &component.node_ids {
        let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == *node_id) else {
            continue;
        };
        let center = entry.world_point_for_node(node);
        items.push(FragmentSelectionItem {
            kind: FragmentItemKind::Node,
            bounds: AxisBounds::around_point(center, SELECTION_NODE_BOX_SIZE / 2.0),
            center,
        });
    }
    for bond_id in &component.bond_ids {
        let Some(bond) = entry.fragment.bonds.iter().find(|bond| bond.id == *bond_id) else {
            continue;
        };
        let Some(begin) = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)
        else {
            continue;
        };
        let Some(end) = entry.fragment.nodes.iter().find(|node| node.id == bond.end) else {
            continue;
        };
        let begin_point = entry.world_point_for_node(begin);
        let end_point = entry.world_point_for_node(end);
        let bounds = fragment_bond_visual_bounds(document, entry.object, entry.fragment, bond)
            .map(AxisBounds::from_array)
            .unwrap_or_else(|| {
                AxisBounds::new(begin_point.x, begin_point.y, end_point.x, end_point.y)
            });
        items.push(FragmentSelectionItem {
            kind: FragmentItemKind::Bond,
            bounds,
            center: midpoint(begin_point, end_point),
        });
    }
    items
}
