use super::*;
use crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT;

#[path = "render_objects/arrows.rs"]
mod arrows;
#[path = "render_objects/graphics.rs"]
mod graphics;
#[path = "render_objects/text.rs"]
mod text;

pub(super) use arrows::render_line_object;
pub(super) use graphics::{render_bracket_object, render_shape_object};
pub(super) use text::render_text_object;

fn text_anchor(align: &str) -> String {
    match align {
        "center" => "middle".to_string(),
        "right" => "end".to_string(),
        _ => "start".to_string(),
    }
}

fn fragment_label_font_size(label: &crate::NodeLabel) -> f64 {
    let mut size = label.font_size;
    for run in &label.runs {
        if let Some(run_size) = run.font_size {
            size = Some(size.map_or(run_size, |current| current.max(run_size)));
        }
    }
    for run in label.line_runs.iter().flatten() {
        if let Some(run_size) = run.font_size {
            size = Some(size.map_or(run_size, |current| current.max(run_size)));
        }
    }
    size.unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT)
}

fn fragment_label_lines(label: &crate::NodeLabel) -> Vec<String> {
    if !label.lines.is_empty() {
        return label
            .lines
            .iter()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
    }
    if label.text.contains('\n') {
        return label
            .text
            .split('\n')
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect();
    }
    if label.text.trim().is_empty() {
        Vec::new()
    } else {
        vec![label.text.clone()]
    }
}

fn fragment_label_runs_for_line(
    label: &crate::NodeLabel,
    index: usize,
    line: &str,
) -> Vec<LabelRun> {
    if let Some(line_runs) = label.line_runs.get(index) {
        return line_runs.clone();
    }
    if index == 0 && !label.runs.is_empty() && !label.text.contains('\n') && label.lines.is_empty()
    {
        return label.runs.clone();
    }
    vec![LabelRun {
        text: line.to_string(),
        font_family: label.font_family.clone(),
        font_size: label.font_size,
        fill: label.fill.clone(),
        font_weight: None,
        font_style: None,
        underline: None,
        script: None,
    }]
}

fn fragment_label_position_world(label: &crate::NodeLabel, object: &SceneObject) -> Point {
    let position = label.position.unwrap_or([0.0, 0.0]);
    Point::new(
        object.transform.translate[0] + position[0],
        object.transform.translate[1] + position[1],
    )
}

fn polygon_list_bounds(polygons: &[Vec<Point>]) -> Option<(f64, f64, f64, f64)> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut found = false;
    for polygon in polygons {
        for point in polygon {
            found = true;
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }
    }
    found.then_some((min_x, min_y, max_x, max_y))
}

pub(super) fn render_molecule_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
) {
    let Some(resource_ref) = object.payload.resource_ref.as_ref() else {
        return;
    };
    let Some(resource) = document.resources.get(resource_ref) else {
        return;
    };
    match &resource.data {
        ResourceData::Fragment(fragment)
            if resource.resource_type == "molecule_fragment2d"
                || resource.encoding == "chemsema.molecule.fragment2d" =>
        {
            let node_map: BTreeMap<&str, &Node> = fragment
                .nodes
                .iter()
                .map(|node| (node.id.as_str(), node))
                .collect();
            let stroke = molecule_stroke(document, object);
            let object_id = Some(object.id.clone());
            let contact_kernel =
                build_main_bond_contact_kernel(document, object, &fragment.bonds, &node_map);

            for bond in &fragment.bonds {
                render_fragment_bond(
                    out,
                    document,
                    object,
                    &contact_kernel,
                    &fragment.bonds,
                    &node_map,
                    bond,
                    &stroke,
                    object_id.clone(),
                );
            }
            render_main_bond_contact_patches(out, &contact_kernel, &stroke, object_id.clone());

            for node in &fragment.nodes {
                render_fragment_label(out, document, object, node, object_id.clone());
                render_fragment_cdxml_node_markers(
                    out,
                    document,
                    object,
                    fragment,
                    node,
                    &stroke,
                    object_id.clone(),
                );
                render_fragment_atom_query_annotations(
                    out,
                    document,
                    object,
                    node,
                    object_id.clone(),
                );
                render_fragment_node_invalid_marker(out, object, node, object_id.clone());
            }
        }
        ResourceData::Text(molblock) => {
            render_legacy_molecule_object(out, document, object, molblock);
        }
        _ => {}
    }
}

pub(super) fn render_molecule_object_targets(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    target_node_ids: &BTreeSet<String>,
    target_bond_ids: &BTreeSet<String>,
) {
    let Some(resource_ref) = object.payload.resource_ref.as_ref() else {
        return;
    };
    let Some(resource) = document.resources.get(resource_ref) else {
        return;
    };
    let ResourceData::Fragment(fragment) = &resource.data else {
        return;
    };
    if resource.resource_type != "molecule_fragment2d"
        && resource.encoding != "chemsema.molecule.fragment2d"
    {
        return;
    }

    let node_map: BTreeMap<&str, &Node> = fragment
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let stroke = molecule_stroke(document, object);
    let object_id = Some(object.id.clone());
    let mut target_render_bond_ids = BTreeSet::new();
    for bond in &fragment.bonds {
        let touches_target_node =
            target_node_ids.contains(&bond.begin) || target_node_ids.contains(&bond.end);
        if target_bond_ids.contains(&bond.id) || touches_target_node {
            target_render_bond_ids.insert(bond.id.clone());
        }
    }
    expand_target_render_bond_ids_for_contact_nodes(&mut target_render_bond_ids, &fragment.bonds);
    expand_target_render_bond_ids_for_crossings(
        &mut target_render_bond_ids,
        document,
        object,
        &fragment.bonds,
        &node_map,
    );
    let mut contact_node_ids = BTreeSet::new();
    for bond in &fragment.bonds {
        if target_render_bond_ids.contains(&bond.id) {
            contact_node_ids.insert(bond.begin.clone());
            contact_node_ids.insert(bond.end.clone());
        }
    }
    let contact_kernel = build_main_bond_contact_kernel_for_nodes(
        document,
        object,
        &fragment.bonds,
        &node_map,
        &contact_node_ids,
    );

    for bond in &fragment.bonds {
        if target_render_bond_ids.contains(&bond.id) {
            render_fragment_bond(
                out,
                document,
                object,
                &contact_kernel,
                &fragment.bonds,
                &node_map,
                bond,
                &stroke,
                object_id.clone(),
            );
        }
    }
    render_main_bond_contact_patches(out, &contact_kernel, &stroke, object_id.clone());

    let mut label_render_node_ids = target_node_ids.clone();
    label_render_node_ids.extend(contact_node_ids);
    for node in &fragment.nodes {
        if label_render_node_ids.contains(&node.id) {
            render_fragment_label(out, document, object, node, object_id.clone());
            render_fragment_cdxml_node_markers(
                out,
                document,
                object,
                fragment,
                node,
                &stroke,
                object_id.clone(),
            );
            render_fragment_atom_query_annotations(out, document, object, node, object_id.clone());
            render_fragment_node_invalid_marker(out, object, node, object_id.clone());
        }
    }
}

fn render_fragment_cdxml_node_markers(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    fragment: &MoleculeFragment,
    node: &Node,
    stroke: &str,
    object_id: Option<String>,
) {
    let cdxml = node.meta.pointer("/import/cdxml");
    let h_dot = cdxml
        .and_then(|meta| meta.get("hDot"))
        .and_then(JsonValue::as_bool)
        == Some(true);
    let h_dash = cdxml
        .and_then(|meta| meta.get("hDash"))
        .and_then(JsonValue::as_bool)
        == Some(true);
    let is_unbonded_multi_attachment = cdxml
        .and_then(|meta| meta.get("nodeType"))
        .and_then(JsonValue::as_str)
        == Some("MultiAttachment")
        && !fragment
            .bonds
            .iter()
            .any(|bond| bond.begin == node.id || bond.end == node.id);
    if !h_dot && !h_dash && !is_unbonded_multi_attachment {
        return;
    }

    let center = world_point(object, node);
    let line_width = document
        .document
        .meta
        .pointer("/import/cdxml/defaults/lineWidth")
        .and_then(JsonValue::as_f64)
        .unwrap_or(DEFAULT_BOND_STROKE);
    let bold_width = document
        .document
        .meta
        .pointer("/import/cdxml/defaults/boldWidth")
        .and_then(JsonValue::as_f64)
        .unwrap_or(BOLD_BOND_WIDTH);
    let bond_length = document
        .document
        .meta
        .pointer("/import/cdxml/defaults/bondLength")
        .and_then(JsonValue::as_f64)
        .unwrap_or(crate::DEFAULT_BOND_LENGTH);

    if h_dot {
        out.push(RenderPrimitive::Circle {
            role: RenderRole::DocumentGraphic,
            object_id: object_id.clone(),
            node_id: Some(node.id.clone()),
            center,
            radius: bold_width * 0.5,
            fill: stroke.to_string(),
            stroke: stroke.to_string(),
            stroke_width: 0.0,
        });
    }
    if h_dash {
        let half_width = bold_width * 0.2625;
        for offset_y in [bold_width * 0.75, bold_width * 1.275] {
            push_cdxml_node_marker_line(
                out,
                object_id.clone(),
                Point::new(center.x - half_width, center.y + offset_y),
                Point::new(center.x + half_width, center.y + offset_y),
                stroke,
                line_width,
            );
        }
    }
    if is_unbonded_multi_attachment {
        // ChemDraw's unbonded MultiAttachment placeholder spans roughly 30%
        // of the document bond length (three full rays crossing at the node).
        let radius = bond_length * 0.15;
        for angle_degrees in [90.0_f64, 30.0, -30.0] {
            let angle = angle_degrees.to_radians();
            let dx = radius * angle.cos();
            let dy = radius * angle.sin();
            push_cdxml_node_marker_line(
                out,
                object_id.clone(),
                Point::new(center.x - dx, center.y - dy),
                Point::new(center.x + dx, center.y + dy),
                stroke,
                line_width,
            );
        }
    }
}

fn push_cdxml_node_marker_line(
    out: &mut Vec<RenderPrimitive>,
    object_id: Option<String>,
    from: Point,
    to: Point,
    stroke: &str,
    stroke_width: f64,
) {
    out.push(RenderPrimitive::Path {
        role: RenderRole::DocumentGraphic,
        object_id,
        bond_id: None,
        d: format!("M {:.4} {:.4} L {:.4} {:.4}", from.x, from.y, to.x, to.y),
        points: vec![from, to],
        stroke: stroke.to_string(),
        stroke_width,
        dash_array: Vec::new(),
        line_cap: Some("butt".to_string()),
        line_join: Some("miter".to_string()),
        rotate: 0.0,
        rotate_center: None,
    });
}

fn render_fragment_atom_query_annotations(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    node: &Node,
    object_id: Option<String>,
) {
    let show_atom_query = document
        .document
        .meta
        .pointer("/import/cdxml/defaults/showAtomQuery")
        .and_then(JsonValue::as_bool)
        .unwrap_or(true);
    if !show_atom_query
        || node
            .meta
            .pointer("/import/cdxml/restrictImplicitHydrogens")
            .and_then(JsonValue::as_bool)
            != Some(true)
    {
        return;
    }

    // CDXML's ImplicitHydrogens property is the atom-query restriction
    // kCDXProp_Atom_RestrictImplicitHydrogens. ChemDraw displays it as an
    // auxiliary H query marker; it is independent of both the authored atom
    // label and NumHydrogens, so it must not be folded into either one.
    let font_size = node
        .label
        .as_ref()
        .map(fragment_label_font_size)
        .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
    let font_family = node
        .label
        .as_ref()
        .and_then(|label| label.font_family.clone())
        .or_else(|| {
            object
                .style_ref
                .as_ref()
                .and_then(|style_ref| document.styles.get(style_ref))
                .and_then(|style| style_string(style, "fontFamily"))
        });
    let fill = node
        .label
        .as_ref()
        .and_then(|label| label.fill.clone())
        .or_else(|| {
            object
                .style_ref
                .as_ref()
                .and_then(|style_ref| document.styles.get(style_ref))
                .and_then(|style| style_string(style, "fill"))
        });
    let node_world = world_point(object, node);
    let x = node_world.x + font_size * 0.17;
    let baseline_y = label_box_world(node, object)
        .map(|label_box| label_box.y1 - font_size * 0.07)
        .unwrap_or(node_world.y - font_size * 0.55);
    push_text_for_node(
        out,
        x,
        baseline_y,
        Some(font_size * 0.82),
        String::new(),
        font_size,
        font_family.clone(),
        fill.clone(),
        Some("start".to_string()),
        vec![LabelRun {
            text: "H".to_string(),
            font_family,
            font_size: Some(font_size),
            fill,
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            script: Some("normal".to_string()),
        }],
        object_id,
        Some(node.id.clone()),
    );
}

fn expand_target_render_bond_ids_for_contact_nodes(
    target_render_bond_ids: &mut BTreeSet<String>,
    bonds: &[Bond],
) {
    if target_render_bond_ids.is_empty() {
        return;
    }

    let mut contact_node_ids = BTreeSet::new();
    for bond in bonds {
        if target_render_bond_ids.contains(&bond.id) {
            contact_node_ids.insert(bond.begin.clone());
            contact_node_ids.insert(bond.end.clone());
        }
    }

    for bond in bonds {
        if contact_node_ids.contains(&bond.begin) || contact_node_ids.contains(&bond.end) {
            target_render_bond_ids.insert(bond.id.clone());
        }
    }
}

fn expand_target_render_bond_ids_for_crossings(
    target_render_bond_ids: &mut BTreeSet<String>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
) {
    if target_render_bond_ids.is_empty() {
        return;
    }
    let mut extra = BTreeSet::new();
    let target_indices: Vec<usize> = bonds
        .iter()
        .enumerate()
        .filter_map(|(index, bond)| target_render_bond_ids.contains(&bond.id).then_some(index))
        .collect();
    for target_index in target_indices {
        for other_index in 0..bonds.len() {
            if target_index == other_index {
                continue;
            }
            let (under_bond, over_bond) = if target_index < other_index {
                (&bonds[target_index], &bonds[other_index])
            } else {
                (&bonds[other_index], &bonds[target_index])
            };
            if bonds_have_crossing_margin(document, object, node_map, over_bond, under_bond) {
                extra.insert(over_bond.id.clone());
            }
        }
    }
    target_render_bond_ids.extend(extra);
}

fn bonds_have_crossing_margin(
    document: &ChemSemaDocument,
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    over_bond: &Bond,
    under_bond: &Bond,
) -> bool {
    if bonds_share_endpoint(over_bond, under_bond) {
        return false;
    }
    let under_crossings = imported_cdxml_crossing_bonds(under_bond);
    let over_crossings = imported_cdxml_crossing_bonds(over_bond);
    if (under_crossings.is_some() || over_crossings.is_some())
        && !under_crossings
            .as_ref()
            .is_some_and(|ids| ids.contains(&over_bond.id))
        && !over_crossings
            .as_ref()
            .is_some_and(|ids| ids.contains(&under_bond.id))
    {
        return false;
    }
    let Some((over_start, over_end)) = bond_world_segment(object, node_map, over_bond) else {
        return false;
    };
    let Some((under_start, under_end)) = bond_world_segment(object, node_map, under_bond) else {
        return false;
    };
    let over_vector = Vector::new(over_end.x - over_start.x, over_end.y - over_start.y);
    let under_vector = Vector::new(under_end.x - under_start.x, under_end.y - under_start.y);
    if over_vector.length() <= EPSILON || under_vector.length() <= EPSILON {
        return false;
    }
    let crossing_sin = vector_cross(over_vector.normalized(), under_vector.normalized()).abs();
    if crossing_sin <= 0.1 {
        return false;
    }
    if interior_segment_intersection(over_start, over_end, under_start, under_end).is_some() {
        return true;
    }

    let under_stroke_width = bond_stroke_width(document, object, under_bond);
    let over_stroke_width = bond_stroke_width(document, object, over_bond);
    let margin_width = document_margin_width_for_bond(document, over_bond, over_stroke_width);
    if margin_width <= EPSILON {
        return false;
    }
    let under_envelope =
        document_bond_crossing_envelope(under_bond, under_start, under_end, under_stroke_width);
    let over_envelope =
        document_bond_crossing_envelope(over_bond, over_start, over_end, over_stroke_width);
    let Some(under_polygon) = crossing_strip_polygon_for_segment(
        under_start,
        under_end,
        under_envelope.silhouette_start,
        under_envelope.silhouette_end,
        0.05,
        0.0,
    ) else {
        return false;
    };
    let Some(over_polygon) = crossing_strip_polygon_for_segment(
        over_start,
        over_end,
        over_envelope.clearance_start,
        over_envelope.clearance_end,
        margin_width,
        margin_width,
    ) else {
        return false;
    };
    let overlap = intersect_convex_polygons(&under_polygon, &over_polygon);
    overlap.len() >= 3 && polygon_area_signed(&overlap).abs() > 1.0e-4
}

fn bond_world_segment(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
) -> Option<(Point, Point)> {
    let begin = world_point(object, node_map.get(bond.begin.as_str()).copied()?);
    let end = world_point(object, node_map.get(bond.end.as_str()).copied()?);
    Some((begin, end))
}

fn bonds_share_endpoint(first: &Bond, second: &Bond) -> bool {
    first.begin == second.begin
        || first.begin == second.end
        || first.end == second.begin
        || first.end == second.end
}

fn interior_segment_intersection(a1: Point, a2: Point, b1: Point, b2: Point) -> Option<Point> {
    let a = Vector::new(a2.x - a1.x, a2.y - a1.y);
    let b = Vector::new(b2.x - b1.x, b2.y - b1.y);
    let denom = vector_cross(a, b);
    if denom.abs() <= EPSILON {
        return None;
    }
    let offset = Vector::new(b1.x - a1.x, b1.y - a1.y);
    let t = vector_cross(offset, b) / denom;
    let u = vector_cross(offset, a) / denom;
    if t <= 1.0e-6 || t >= 1.0 - 1.0e-6 || u <= 1.0e-6 || u >= 1.0 - 1.0e-6 {
        return None;
    }
    Some(Point::new(a1.x + a.x * t, a1.y + a.y * t))
}

fn render_fragment_node_invalid_marker(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    node: &Node,
    object_id: Option<String>,
) {
    if chemical_check_disabled(&node.meta) {
        return;
    }
    if !crate::node_has_charge_symbol_invalid(node) {
        return;
    }
    let center = Point::new(
        object.transform.translate[0] + node.position[0],
        object.transform.translate[1] + node.position[1],
    );
    out.push(RenderPrimitive::Circle {
        role: RenderRole::DocumentDiagnostic,
        object_id,
        node_id: Some(node.id.clone()),
        center,
        radius: crate::ENDPOINT_FOCUS_RADIUS,
        fill: "none".to_string(),
        stroke: "#d32f2f".to_string(),
        stroke_width: 0.5,
    });
}

pub(super) fn render_fragment_label(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    node: &Node,
    object_id: Option<String>,
) {
    let Some(label) = node.label.as_ref() else {
        return;
    };
    if !label.has_visible_text() {
        return;
    }

    let font_size = fragment_label_font_size(label);
    let text_anchor = text_anchor(label.align.as_deref().unwrap_or("left"));
    let font_family = label.font_family.clone().or_else(|| {
        object
            .style_ref
            .as_ref()
            .and_then(|style_ref| document.styles.get(style_ref))
            .and_then(|style| style_string(style, "fontFamily"))
    });
    let fill = label.fill.clone().or_else(|| {
        object
            .style_ref
            .as_ref()
            .and_then(|style_ref| document.styles.get(style_ref))
            .and_then(|style| style_string(style, "fill"))
    });
    let knockout_polygons = label_polygons_world(node, object);
    if knockout_polygons.is_empty() {
        if let Some(box_value) = label_box_world(node, object) {
            out.push(RenderPrimitive::Rect {
                role: RenderRole::DocumentKnockout,
                object_id: object_id.clone(),
                node_id: Some(node.id.clone()),
                x: box_value.x1,
                y: box_value.y1,
                width: (box_value.x2 - box_value.x1).max(0.0),
                height: (box_value.y2 - box_value.y1).max(0.0),
                fill: Some(document.document.page.background.clone()),
                stroke: None,
                stroke_width: 0.0,
                rx: None,
                ry: None,
                dash_array: Vec::new(),
                fill_gradient: None,
            });
        }
    } else {
        for polygon in knockout_polygons {
            push_label_knockout_polygon(out, polygon, object_id.clone(), node.id.clone());
        }
    }
    if fragment_label_is_invalid(label) {
        let invalid_box = polygon_list_bounds(&label_polygons_world(node, object))
            .map(|(x1, y1, x2, y2)| RectBox { x1, y1, x2, y2 })
            .or_else(|| label_box_world(node, object));
        if let Some(box_value) = invalid_box {
            out.push(RenderPrimitive::Rect {
                role: RenderRole::DocumentDiagnostic,
                object_id: None,
                node_id: Some(node.id.clone()),
                x: box_value.x1,
                y: box_value.y1,
                width: (box_value.x2 - box_value.x1).max(0.0),
                height: (box_value.y2 - box_value.y1).max(0.0),
                fill: Some("none".to_string()),
                stroke: Some("#d32f2f".to_string()),
                stroke_width: 0.5,
                rx: None,
                ry: None,
                dash_array: Vec::new(),
                fill_gradient: None,
            });
        }
    }

    let lines = fragment_label_lines(label);
    if lines.is_empty() {
        return;
    }
    let world_position = fragment_label_position_world(label, object);
    if lines.len() == 1 {
        let primitive = RenderPrimitive::Text {
            role: RenderRole::DocumentText,
            object_id,
            node_id: Some(node.id.clone()),
            x: world_position.x,
            y: world_position.y,
            baseline_offset: Some(font_size * 0.82),
            dominant_baseline: None,
            text: String::new(),
            font_size,
            font_family,
            fill,
            text_anchor: Some(text_anchor),
            line_height: None,
            preserve_lines: false,
            box_width: None,
            runs: fragment_label_runs_for_line(label, 0, &lines[0]),
            rotate: 0.0,
            rotate_center: None,
        };
        out.push(primitive);
        return;
    }

    let label_box = label_box_world(node, object);
    let line_height = crate::molecule_label_line_advance(font_size);
    let box_top = label_box
        .map(|box_value| box_value.y1)
        .unwrap_or(world_position.y - line_height * 0.82);
    for (index, line) in lines.iter().enumerate() {
        let baseline_y = box_top + line_height * index as f64 + font_size * 0.82;
        push_text_for_node(
            out,
            world_position.x,
            baseline_y,
            Some(font_size * 0.82),
            String::new(),
            font_size,
            font_family.clone(),
            fill.clone(),
            Some(text_anchor.clone()),
            fragment_label_runs_for_line(label, index, line),
            object_id.clone(),
            Some(node.id.clone()),
        );
    }
}

fn fragment_label_is_invalid(label: &crate::NodeLabel) -> bool {
    if chemical_check_disabled(&label.meta) {
        return false;
    }
    label
        .meta
        .get("labelRecognition")
        .and_then(|value| value.get("status"))
        .and_then(serde_json::Value::as_str)
        == Some("invalid")
}

fn chemical_check_disabled(meta: &serde_json::Value) -> bool {
    if meta
        .get("defaultChemical")
        .and_then(serde_json::Value::as_bool)
        == Some(false)
    {
        return true;
    }
    meta.get("chemicalCheck")
        .and_then(serde_json::Value::as_bool)
        == Some(false)
}
