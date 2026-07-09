use super::*;

pub(super) fn world_point(object: &SceneObject, node: &Node) -> Point {
    Point::new(
        object.transform.translate[0] + node.position[0],
        object.transform.translate[1] + node.position[1],
    )
}

pub(super) fn label_box_world(node: &Node, object: &SceneObject) -> Option<RectBox> {
    let label = node.label.as_ref()?;
    let bbox = label.bbox()?;
    Some(RectBox {
        x1: bbox[0] + object.transform.translate[0],
        y1: bbox[1] + object.transform.translate[1],
        x2: bbox[2] + object.transform.translate[0],
        y2: bbox[3] + object.transform.translate[1],
    })
}

pub(super) fn label_polygons_world(node: &Node, object: &SceneObject) -> Vec<Vec<Point>> {
    node.label
        .as_ref()
        .map(|label| {
            label
                .glyph_polygons()
                .into_iter()
                .map(|polygon| {
                    compact_polygon_points(
                        polygon
                            .into_iter()
                            .map(|point| {
                                Point::new(
                                    point.x + object.transform.translate[0],
                                    point.y + object.transform.translate[1],
                                )
                            })
                            .collect(),
                    )
                })
                .filter(|polygon| polygon.len() >= 3)
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn segment_intersection_fraction(
    start: Point,
    end: Point,
    first: Point,
    second: Point,
) -> Option<f64> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let edge = Vector::new(second.x - first.x, second.y - first.y);
    let denom = vector_cross(direction, edge);
    if denom.abs() <= EPSILON {
        let offset = Vector::new(first.x - start.x, first.y - start.y);
        if vector_cross(offset, direction).abs() > EPSILON {
            return None;
        }
        let length_sq = direction.x * direction.x + direction.y * direction.y;
        if length_sq <= EPSILON {
            return None;
        }
        let first_t =
            ((first.x - start.x) * direction.x + (first.y - start.y) * direction.y) / length_sq;
        let second_t =
            ((second.x - start.x) * direction.x + (second.y - start.y) * direction.y) / length_sq;
        let overlap_start = first_t.min(second_t).max(0.0);
        let overlap_end = first_t.max(second_t).min(1.0);
        return (overlap_end + EPSILON >= overlap_start).then_some(overlap_end);
    }
    let offset = Vector::new(first.x - start.x, first.y - start.y);
    let t = vector_cross(offset, edge) / denom;
    let u = vector_cross(offset, direction) / denom;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some(t)
    } else {
        None
    }
}

fn point_is_on_segment(point: Point, first: Point, second: Point) -> bool {
    let edge = Vector::new(second.x - first.x, second.y - first.y);
    let point_vector = Vector::new(point.x - first.x, point.y - first.y);
    if vector_cross(edge, point_vector).abs() > EPSILON {
        return false;
    }
    let dot = point_vector.x * edge.x + point_vector.y * edge.y;
    if dot < -EPSILON {
        return false;
    }
    dot <= edge.x * edge.x + edge.y * edge.y + EPSILON
}

fn point_is_inside_or_on_polygon(point: Point, polygon: &[Point]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    for index in 0..polygon.len() {
        let first = polygon[index];
        let second = polygon[(index + 1) % polygon.len()];
        if point_is_on_segment(point, first, second) {
            return true;
        }
        let crosses = (first.y > point.y) != (second.y > point.y);
        if crosses {
            let x_intersection =
                (second.x - first.x) * (point.y - first.y) / (second.y - first.y) + first.x;
            if x_intersection > point.x {
                inside = !inside;
            }
        }
    }
    inside
}

fn polygon_bounds(polygon: &[Point]) -> Option<RectBox> {
    let mut bounds = RectBox {
        x1: f64::INFINITY,
        y1: f64::INFINITY,
        x2: f64::NEG_INFINITY,
        y2: f64::NEG_INFINITY,
    };
    for point in polygon {
        bounds.x1 = bounds.x1.min(point.x);
        bounds.y1 = bounds.y1.min(point.y);
        bounds.x2 = bounds.x2.max(point.x);
        bounds.y2 = bounds.y2.max(point.y);
    }
    (bounds.x1.is_finite()
        && bounds.y1.is_finite()
        && bounds.x2.is_finite()
        && bounds.y2.is_finite()
        && bounds.x2 + EPSILON >= bounds.x1
        && bounds.y2 + EPSILON >= bounds.y1)
        .then_some(bounds)
}

fn rect_segment_exit_fraction(start: Point, end: Point, rect: RectBox) -> Option<f64> {
    let corners = [
        Point::new(rect.x1, rect.y1),
        Point::new(rect.x2, rect.y1),
        Point::new(rect.x2, rect.y2),
        Point::new(rect.x1, rect.y2),
    ];
    let start_inside = rect.contains(start);
    let mut best_t: Option<f64> = None;
    for index in 0..corners.len() {
        let next = (index + 1) % corners.len();
        let Some(t) = segment_intersection_fraction(start, end, corners[index], corners[next])
        else {
            continue;
        };
        if t <= EPSILON && !start_inside {
            continue;
        }
        if best_t.is_none_or(|current| t > current) {
            best_t = Some(t);
        }
    }
    if start_inside && best_t.is_none() {
        best_t = Some(0.0);
    }
    best_t
}

pub(super) fn clip_point_out_of_polygons(
    start: Point,
    end: Point,
    polygons: &[Vec<Point>],
) -> Option<Point> {
    let mut best_t: Option<f64> = None;
    for polygon in polygons {
        if polygon.len() < 3 {
            continue;
        }
        let start_inside = point_is_inside_or_on_polygon(start, polygon);
        let mut polygon_t: Option<f64> = None;
        for index in 0..polygon.len() {
            let next = (index + 1) % polygon.len();
            let Some(t) = segment_intersection_fraction(start, end, polygon[index], polygon[next])
            else {
                continue;
            };
            if t <= EPSILON && !start_inside {
                continue;
            }
            if polygon_t.is_none_or(|current| t > current) {
                polygon_t = Some(t);
            }
        }
        if start_inside && polygon_t.is_none() {
            polygon_t = Some(0.0);
        }
        if let Some(t) = polygon_t {
            if best_t.is_none_or(|current| t > current) {
                best_t = Some(t);
            }
        }
    }
    best_t.map(|t| {
        Point::new(
            start.x + (end.x - start.x) * t,
            start.y + (end.y - start.y) * t,
        )
    })
}

fn clip_point_out_of_expanded_polygon_bounds(
    start: Point,
    end: Point,
    polygons: &[Vec<Point>],
    margin: f64,
) -> Option<Point> {
    if margin <= EPSILON {
        return None;
    }
    let mut best_t: Option<f64> = None;
    for polygon in polygons {
        let Some(bounds) = polygon_bounds(polygon) else {
            continue;
        };
        let Some(t) = rect_segment_exit_fraction(start, end, bounds.expanded(margin)) else {
            continue;
        };
        if best_t.is_none_or(|current| t > current) {
            best_t = Some(t);
        }
    }
    best_t.map(|t| {
        Point::new(
            start.x + (end.x - start.x) * t,
            start.y + (end.y - start.y) * t,
        )
    })
}

pub(super) fn clip_point_out_of_box(
    start: Point,
    end: Point,
    rect: Option<RectBox>,
    margin: f64,
) -> Point {
    let Some(expanded) = rect.map(|box_value| box_value.expanded(margin)) else {
        return start;
    };
    if !expanded.contains(start) {
        return start;
    }
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let mut candidates = Vec::new();
    if dx.abs() > EPSILON {
        for x in [expanded.x1, expanded.x2] {
            let t = (x - start.x) / dx;
            let y = start.y + dy * t;
            if (0.0..=1.0).contains(&t) && y >= expanded.y1 && y <= expanded.y2 {
                candidates.push((t, Point::new(x, y)));
            }
        }
    }
    if dy.abs() > EPSILON {
        for y in [expanded.y1, expanded.y2] {
            let t = (y - start.y) / dy;
            let x = start.x + dx * t;
            if (0.0..=1.0).contains(&t) && x >= expanded.x1 && x <= expanded.x2 {
                candidates.push((t, Point::new(x, y)));
            }
        }
    }
    candidates
        .into_iter()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, point)| point)
        .unwrap_or(start)
}

pub(super) fn clip_point_out_of_label_geometry(
    start: Point,
    end: Point,
    rect: Option<RectBox>,
    polygons: &[Vec<Point>],
    _margin: f64,
) -> Point {
    // Prefer per-glyph polygons when they exist; bounding boxes are only a
    // fallback for imported or legacy labels without glyph geometry.
    if polygons.is_empty() {
        return clip_point_out_of_box(start, end, rect, 0.0);
    }
    clip_point_out_of_polygons(start, end, polygons)
        .or_else(|| clip_point_out_of_expanded_polygon_bounds(start, end, polygons, 0.0))
        .unwrap_or(start)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn clip_segment_out_of_label_geometry(
    start: Point,
    end: Point,
    start_rect: Option<RectBox>,
    start_polygons: &[Vec<Point>],
    end_rect: Option<RectBox>,
    end_polygons: &[Vec<Point>],
    margin: f64,
) -> Option<(Point, Point)> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length_sq = direction.x * direction.x + direction.y * direction.y;
    if length_sq <= EPSILON {
        return None;
    }

    let clipped_start =
        clip_point_out_of_label_geometry(start, end, start_rect, start_polygons, margin);
    let clipped_end = clip_point_out_of_label_geometry(end, start, end_rect, end_polygons, margin);
    let start_t = ((clipped_start.x - start.x) * direction.x
        + (clipped_start.y - start.y) * direction.y)
        / length_sq;
    let end_t = ((clipped_end.x - start.x) * direction.x + (clipped_end.y - start.y) * direction.y)
        / length_sq;

    (end_t > start_t + EPSILON).then_some((clipped_start, clipped_end))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_fragment_line(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    start_box: Option<RectBox>,
    end_box: Option<RectBox>,
    allow_bold_contacts: bool,
    stroke: &str,
    stroke_width: f64,
    dash_array: Vec<f64>,
    line_weight: BondLineWeight,
    object_id: Option<String>,
) {
    render_fragment_line_with_profiles(
        out,
        object,
        contact_kernel,
        bonds,
        node_map,
        bond,
        start,
        end,
        start_box,
        end_box,
        allow_bold_contacts,
        stroke,
        stroke_width,
        dash_array,
        line_weight,
        object_id,
        true,
        true,
        true,
        true,
        None,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_fragment_line_with_profiles(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    start_box: Option<RectBox>,
    end_box: Option<RectBox>,
    allow_bold_contacts: bool,
    stroke: &str,
    stroke_width: f64,
    dash_array: Vec<f64>,
    line_weight: BondLineWeight,
    object_id: Option<String>,
    clip_against_label_geometry: bool,
    allow_start_join: bool,
    allow_end_join: bool,
    inherit_kernel_profiles: bool,
    start_endpoint_profile_override: Option<Vec<Point>>,
    end_endpoint_profile_override: Option<Vec<Point>>,
) {
    let start_polygons = node_map
        .get(bond.begin.as_str())
        .map(|node| label_polygons_world(node, object))
        .unwrap_or_default();
    let end_polygons = node_map
        .get(bond.end.as_str())
        .map(|node| label_polygons_world(node, object))
        .unwrap_or_default();
    let start_has_label = node_map
        .get(bond.begin.as_str())
        .and_then(|node| node.label.as_ref())
        .is_some_and(|label| label.has_visible_text());
    let end_has_label = node_map
        .get(bond.end.as_str())
        .and_then(|node| node.label.as_ref())
        .is_some_and(|label| label.has_visible_text());
    let allow_start_join = allow_start_join && !start_has_label;
    let allow_end_join = allow_end_join && !end_has_label;
    let start_endpoint_profile_override = if start_has_label {
        None
    } else {
        start_endpoint_profile_override
    };
    let end_endpoint_profile_override = if end_has_label {
        None
    } else {
        end_endpoint_profile_override
    };
    let Some((clipped_start, clipped_end)) = (if clip_against_label_geometry {
        let label_clip_margin = label_clip_margin_for_bond(bond, stroke_width);
        clip_segment_out_of_label_geometry(
            start,
            end,
            start_box,
            &start_polygons,
            end_box,
            &end_polygons,
            label_clip_margin,
        )
    } else {
        Some((start, end))
    }) else {
        return;
    };
    let mut start_retreat = if start_has_label {
        0.0
    } else {
        contact_kernel.endpoint_retreat(&bond.id, &bond.begin)
    };
    let mut end_retreat = if end_has_label {
        0.0
    } else {
        contact_kernel.endpoint_retreat(&bond.id, &bond.end)
    };
    if is_hash_bond(bond) && line_weight == BondLineWeight::Bold && !dash_array.is_empty() {
        let retreat = hash_contact_retreat_distance_for_bond(bond, stroke_width);
        if !start_has_label && endpoint_has_other_bond(bonds, bond, &bond.begin) {
            start_retreat = start_retreat.max(retreat);
        }
        if !end_has_label && endpoint_has_other_bond(bonds, bond, &bond.end) {
            end_retreat = end_retreat.max(retreat);
        }
    }
    let (clipped_start, clipped_end) =
        apply_segment_endpoint_retreats(clipped_start, clipped_end, start_retreat, end_retreat);
    let mut start_endpoint_profile = start_endpoint_profile_override.or_else(|| {
        if inherit_kernel_profiles {
            contact_kernel.endpoint_profile(&bond.id, &bond.begin)
        } else {
            None
        }
    });
    let mut end_endpoint_profile = end_endpoint_profile_override.or_else(|| {
        if inherit_kernel_profiles {
            contact_kernel.endpoint_profile(&bond.id, &bond.end)
        } else {
            None
        }
    });
    if start_retreat > EPSILON {
        start_endpoint_profile = None;
    }
    if end_retreat > EPSILON {
        end_endpoint_profile = None;
    }
    let use_start_contact_kernel = !start_has_label
        && (contact_kernel.uses_endpoint(&bond.id, &bond.begin)
            || start_endpoint_profile.is_some());
    let use_end_contact_kernel = !end_has_label
        && (contact_kernel.uses_endpoint(&bond.id, &bond.end) || end_endpoint_profile.is_some());
    if line_weight == BondLineWeight::Normal && dash_array.is_empty() {
        let allow_main_line_join =
            is_joinable_main_line_render(bond, allow_bold_contacts, line_weight);
        if let Some(points) = main_line_polygon_points(
            object,
            bonds,
            node_map,
            bond,
            clipped_start,
            clipped_end,
            stroke_width,
            allow_main_line_join && allow_start_join && !use_start_contact_kernel,
            allow_main_line_join && allow_end_join && !use_end_contact_kernel,
            start_endpoint_profile.clone(),
            end_endpoint_profile.clone(),
        ) {
            push_bond_polygon(out, &bond.id, points, stroke, stroke, 0.0, object_id);
            return;
        }
    }
    if !dash_array.is_empty() {
        let visual_width = line_weight_stroke_width_for_bond(bond, stroke_width, line_weight);
        let segment_polygons = if line_weight == BondLineWeight::Bold && is_hash_bond(bond) {
            hash_bond_segment_polygons(clipped_start, clipped_end, visual_width, stroke_width)
        } else {
            dashed_bond_segment_polygons(clipped_start, clipped_end, visual_width, &dash_array)
        };
        if !segment_polygons.is_empty() {
            for points in segment_polygons {
                push_bond_polygon(
                    out,
                    &bond.id,
                    points,
                    stroke,
                    stroke,
                    0.0,
                    object_id.clone(),
                );
            }
            return;
        }
        if let Some(points) =
            simple_main_line_polygon_points(clipped_start, clipped_end, visual_width)
        {
            push_bond_polygon(
                out,
                &bond.id,
                points,
                stroke,
                stroke,
                0.0,
                object_id.clone(),
            )
        }
        return;
    }
    if line_weight == BondLineWeight::Bold && dash_array.is_empty() {
        let direction = Vector::new(
            clipped_end.x - clipped_start.x,
            clipped_end.y - clipped_start.y,
        );
        if direction.length() > EPSILON {
            if allow_start_join && !use_start_contact_kernel {
                if let Some(points) = bold_main_line_join_polygon(
                    object,
                    bonds,
                    node_map,
                    bond,
                    &bond.begin,
                    clipped_start,
                    direction,
                    stroke_width,
                ) {
                    push_bond_polygon(
                        out,
                        &bond.id,
                        points,
                        stroke,
                        stroke,
                        0.0,
                        object_id.clone(),
                    );
                }
            }
            if allow_end_join && !use_end_contact_kernel {
                if let Some(points) = bold_main_line_join_polygon(
                    object,
                    bonds,
                    node_map,
                    bond,
                    &bond.end,
                    clipped_end,
                    Vector::new(-direction.x, -direction.y),
                    stroke_width,
                ) {
                    push_bond_polygon(
                        out,
                        &bond.id,
                        points,
                        stroke,
                        stroke,
                        0.0,
                        object_id.clone(),
                    );
                }
            }
        }
        push_bond_polygon(
            out,
            &bond.id,
            compute_bold_bond_points(
                object,
                bonds,
                node_map,
                bond,
                clipped_start,
                clipped_end,
                stroke_width,
                allow_bold_contacts && allow_start_join && !use_start_contact_kernel,
                allow_bold_contacts && allow_end_join && !use_end_contact_kernel,
                start_endpoint_profile,
                end_endpoint_profile,
            ),
            stroke,
            stroke,
            0.0,
            object_id,
        );
        return;
    }
    if let Some(points) = simple_main_line_polygon_points(
        clipped_start,
        clipped_end,
        line_weight_stroke_width_for_bond(bond, stroke_width, line_weight),
    ) {
        push_bond_polygon(out, &bond.id, points, stroke, stroke, 0.0, object_id);
    }
}
