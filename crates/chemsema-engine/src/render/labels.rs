use super::*;
use crate::NodeLabel;

pub(super) fn world_point(object: &SceneObject, node: &Node) -> Point {
    Point::new(
        object.transform.translate[0] + node.position[0],
        object.transform.translate[1] + node.position[1],
    )
}

pub(super) fn attached_label_glyph_anchor_world(
    object: &SceneObject,
    node: &Node,
    authored_character_index: usize,
) -> Option<Point> {
    let label = node.label.as_ref()?;
    let source = label.source_text.as_deref().unwrap_or(&label.text);
    let glyph_index = authored_character_glyph_index(source, authored_character_index)?;
    let polygons = label.glyph_polygons();
    let bounds = polygon_bounds(polygons.get(glyph_index)?)?;
    let anchor_y = if source.contains('\r') || source.contains('\n') {
        (bounds.y1 + bounds.y2) * 0.5
    } else {
        let baseline_y = label.position?[1];
        let font_size = label
            .font_size
            .unwrap_or(crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
        baseline_y - font_size * crate::MOLECULE_LABEL_ANCHOR_BASELINE_RATIO
    };
    Some(Point::new(
        (bounds.x1 + bounds.x2) * 0.5 + object.transform.translate[0],
        anchor_y + object.transform.translate[1],
    ))
}

fn authored_character_glyph_index(source: &str, authored_character_index: usize) -> Option<usize> {
    let mut visible_index = 0usize;
    for (index, character) in source.chars().enumerate() {
        if index == authored_character_index {
            return if matches!(character, '\r' | '\n') {
                visible_index.checked_sub(1)
            } else {
                Some(visible_index)
            };
        }
        if !matches!(character, '\r' | '\n') {
            visible_index += 1;
        }
    }
    None
}

#[cfg(test)]
mod attachment_tests {
    use super::authored_character_glyph_index;

    #[test]
    fn authored_multiline_attachment_indices_map_to_visible_glyphs() {
        assert_eq!(authored_character_glyph_index("H+\nN", 2), Some(1));
        assert_eq!(authored_character_glyph_index("H+\nN", 3), Some(2));
    }

    #[test]
    fn authored_single_line_attachment_indices_are_unchanged() {
        assert_eq!(authored_character_glyph_index("(PhO)2POH", 6), Some(6));
    }
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
                .map(|polygon| polygon_to_world(polygon, object))
                .filter(|polygon| polygon.len() >= 3)
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn label_clip_polygons_world(node: &Node, object: &SceneObject) -> Vec<Vec<Point>> {
    node.label
        .as_ref()
        .map(|label| {
            label_clip_polygons(label)
                .into_iter()
                .map(|polygon| polygon_to_world(polygon, object))
                .filter(|polygon| polygon.len() >= 3)
                .collect()
        })
        .unwrap_or_default()
}

fn polygon_to_world(polygon: Vec<Point>, object: &SceneObject) -> Vec<Point> {
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
}

#[derive(Debug, Clone, Copy)]
struct GlyphClipInfo {
    index: usize,
    bounds: RectBox,
    center_x: f64,
    center_y: f64,
    height: f64,
}

fn label_clip_polygons(label: &NodeLabel) -> Vec<Vec<Point>> {
    let mut polygons = label.glyph_clip_polygons();
    let glyph_polygons = label.glyph_polygons();
    let mut glyphs: Vec<GlyphClipInfo> = glyph_polygons
        .iter()
        .enumerate()
        .filter_map(|(index, polygon)| {
            let bounds = polygon_bounds(polygon)?;
            Some(GlyphClipInfo {
                index,
                bounds,
                center_x: (bounds.x1 + bounds.x2) * 0.5,
                center_y: (bounds.y1 + bounds.y2) * 0.5,
                height: (bounds.y2 - bounds.y1).max(0.0),
            })
        })
        .filter(|glyph| label_glyph_can_join_horizontal_clip(label, glyph.index))
        .collect();
    if glyphs.len() < 2 {
        return polygons;
    }

    glyphs.sort_by(|left, right| {
        left.center_y
            .total_cmp(&right.center_y)
            .then_with(|| left.center_x.total_cmp(&right.center_x))
    });

    let mut rows: Vec<Vec<GlyphClipInfo>> = Vec::new();
    for glyph in glyphs {
        if let Some(row) = rows.iter_mut().find(|row| {
            row.last()
                .is_some_and(|previous| horizontal_clip_glyphs_share_row(*previous, glyph))
        }) {
            row.push(glyph);
        } else {
            rows.push(vec![glyph]);
        }
    }

    for mut row in rows {
        if row.len() < 2 {
            continue;
        }
        row.sort_by(|left, right| left.center_x.total_cmp(&right.center_x));
        polygons.extend(horizontal_label_internal_clip_polygons(&row));
    }

    polygons
}

fn label_glyph_can_join_horizontal_clip(label: &NodeLabel, glyph_index: usize) -> bool {
    !matches!(
        label_glyph_script(label, glyph_index),
        Some("subscript" | "superscript")
    )
}

fn label_glyph_script(label: &NodeLabel, glyph_index: usize) -> Option<&str> {
    let mut remaining = glyph_index;
    let runs = if !label.line_runs.is_empty() {
        label.line_runs.iter().flatten().collect::<Vec<_>>()
    } else {
        label.runs.iter().collect::<Vec<_>>()
    };
    for run in runs {
        let count = run.text.chars().count();
        if remaining < count {
            return run.script.as_deref();
        }
        remaining = remaining.saturating_sub(count);
    }
    None
}

fn horizontal_clip_glyphs_share_row(left: GlyphClipInfo, right: GlyphClipInfo) -> bool {
    let vertical_overlap =
        (left.bounds.y2.min(right.bounds.y2) - left.bounds.y1.max(right.bounds.y1)).max(0.0);
    let min_height = left.height.min(right.height);
    if min_height <= EPSILON || vertical_overlap < min_height * 0.45 {
        return false;
    }
    let max_height = left.height.max(right.height);
    if (left.center_y - right.center_y).abs() > max_height * 0.45 {
        return false;
    }
    let gap = right.bounds.x1 - left.bounds.x2;
    gap <= max_height * 0.65
}

fn horizontal_label_internal_clip_polygons(row: &[GlyphClipInfo]) -> Vec<Vec<Point>> {
    let mut rectangles = Vec::new();
    let last_index = row.len().saturating_sub(1);

    // Keep the outer half of the first and last glyph as real outline. Their
    // inward halves, and every middle glyph, are rectangularized using that
    // glyph's own bounds. A low parenthesis therefore cannot drag the P-side
    // clipping edge down to the parenthesis baseline.
    for (index, glyph) in row.iter().enumerate() {
        let x1 = if index == 0 {
            glyph.center_x
        } else {
            glyph.bounds.x1
        };
        let x2 = if index == last_index {
            glyph.center_x
        } else {
            glyph.bounds.x2
        };
        if let Some(rectangle) = clip_rectangle(x1, glyph.bounds.y1, x2, glyph.bounds.y2) {
            rectangles.push(rectangle);
        }
    }

    // Bridge only the vertical overlap of adjacent glyphs. This fills an
    // internal character gap without flattening the whole row to a shared
    // top or bottom.
    for pair in row.windows(2) {
        let left = pair[0];
        let right = pair[1];
        let y1 = left.bounds.y1.max(right.bounds.y1);
        let y2 = left.bounds.y2.min(right.bounds.y2);
        if let Some(rectangle) = clip_rectangle(left.bounds.x2, y1, right.bounds.x1, y2) {
            rectangles.push(rectangle);
        }
    }

    rectangles
}

fn clip_rectangle(x1: f64, y1: f64, x2: f64, y2: f64) -> Option<Vec<Point>> {
    if x2 <= x1 + EPSILON || y2 <= y1 + EPSILON {
        return None;
    }
    Some(vec![
        Point::new(x1, y1),
        Point::new(x2, y1),
        Point::new(x2, y2),
        Point::new(x1, y2),
    ])
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

pub(super) fn polygon_bounds(polygon: &[Point]) -> Option<RectBox> {
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
) -> Point {
    // Prefer per-glyph polygons when they exist; bounding boxes are only a
    // fallback for imported or legacy labels without glyph geometry.
    if polygons.is_empty() {
        return clip_point_out_of_box(start, end, rect, 0.0);
    }
    clip_point_out_of_polygons(start, end, polygons).unwrap_or(start)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn clip_body_segment_out_of_label_geometry(
    start: Point,
    end: Point,
    start_rect: Option<RectBox>,
    start_polygons: &[Vec<Point>],
    start_half_width: f64,
    end_rect: Option<RectBox>,
    end_polygons: &[Vec<Point>],
    end_half_width: f64,
) -> Option<(Point, Point)> {
    let (start_retreat, end_retreat) = body_segment_label_retreats(
        start,
        end,
        start_rect,
        start_polygons,
        start_half_width,
        end_rect,
        end_polygons,
        end_half_width,
    )?;
    let (clipped_start, clipped_end) =
        apply_segment_endpoint_retreats(start, end, start_retreat, end_retreat);
    (clipped_start.distance(clipped_end) > EPSILON).then_some((clipped_start, clipped_end))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn body_segment_label_retreats(
    start: Point,
    end: Point,
    start_rect: Option<RectBox>,
    start_polygons: &[Vec<Point>],
    start_half_width: f64,
    end_rect: Option<RectBox>,
    end_polygons: &[Vec<Point>],
    end_half_width: f64,
) -> Option<(f64, f64)> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return None;
    }
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let start_retreat = wedge_endpoint_label_retreat(
        start,
        end,
        unit,
        normal,
        start_rect,
        start_polygons,
        start_half_width,
        length,
    );
    let end_retreat = wedge_endpoint_label_retreat(
        end,
        start,
        Vector::new(-unit.x, -unit.y),
        normal,
        end_rect,
        end_polygons,
        end_half_width,
        length,
    );
    Some((start_retreat, end_retreat))
}

#[allow(clippy::too_many_arguments)]
fn wedge_endpoint_label_retreat(
    endpoint: Point,
    opposite: Point,
    axis_from_endpoint: Vector,
    normal: Vector,
    rect: Option<RectBox>,
    polygons: &[Vec<Point>],
    endpoint_half_width: f64,
    axis_length: f64,
) -> f64 {
    let mut retreat: f64 = 0.0;
    for side in [0.0, 1.0, -1.0] {
        let endpoint_offset = endpoint_half_width * side;
        let ray_start = Point::new(
            endpoint.x + normal.x * endpoint_offset,
            endpoint.y + normal.y * endpoint_offset,
        );
        let ray_end = Point::new(
            opposite.x + normal.x * endpoint_offset,
            opposite.y + normal.y * endpoint_offset,
        );
        let clipped = clip_point_out_of_label_geometry(ray_start, ray_end, rect, polygons);
        let projected = (clipped.x - ray_start.x) * axis_from_endpoint.x
            + (clipped.y - ray_start.y) * axis_from_endpoint.y;
        retreat = retreat.max(projected.clamp(0.0, axis_length));
    }
    retreat
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_fragment_line(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
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
        document,
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
    _document: &ChemSemaDocument,
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
        .map(|node| label_clip_polygons_world(node, object))
        .unwrap_or_default();
    let end_polygons = node_map
        .get(bond.end.as_str())
        .map(|node| label_clip_polygons_world(node, object))
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
        let half_width = line_weight_stroke_width_for_bond(bond, stroke_width, line_weight) * 0.5;
        clip_body_segment_out_of_label_geometry(
            start,
            end,
            start_box,
            &start_polygons,
            half_width,
            end_box,
            &end_polygons,
            half_width,
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
            dashed_bond_segment_polygons_with_profiles(
                clipped_start,
                clipped_end,
                visual_width,
                &dash_array,
                start_endpoint_profile.as_deref(),
                end_endpoint_profile.as_deref(),
            )
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
