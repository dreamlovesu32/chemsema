use super::*;
use crate::{
    DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM, DEFAULT_TEXT_FONT_SIZE_CM, DEFAULT_TEXT_LINE_HEIGHT_CM,
};

fn text_anchor(align: &str) -> String {
    match align {
        "center" => "middle".to_string(),
        "right" => "end".to_string(),
        _ => "start".to_string(),
    }
}

fn fragment_label_font_size(label: &crate::NodeLabel) -> f64 {
    let mut size = label
        .font_size
        .unwrap_or(0.0)
        .max(DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM);
    for run in &label.runs {
        size = size.max(run.font_size.unwrap_or(0.0));
    }
    size.max(DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM)
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

pub(super) fn render_molecule_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
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
                || resource.encoding == "chemcore.molecule.fragment2d" =>
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
            }
        }
        ResourceData::Text(molblock) => {
            render_legacy_molecule_object(out, document, object, molblock);
        }
        _ => {}
    }
}

pub(super) fn render_line_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
) {
    let points = payload_points(&object.payload, "points");
    if points.len() < 2 {
        return;
    }

    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    let stroke = style
        .and_then(|value| style_string(value, "stroke"))
        .unwrap_or_else(|| "#222222".to_string());
    let stroke_width = style
        .and_then(|value| {
            style_number(value, "strokeWidth").or_else(|| style_number(value, "stroke_width"))
        })
        .unwrap_or(px_to_cm(1.6));
    let line_cap = style
        .and_then(|value| style_string(value, "lineCap"))
        .unwrap_or_else(|| "round".to_string());
    let line_join = style
        .and_then(|value| style_string(value, "lineJoin"))
        .unwrap_or_else(|| "round".to_string());
    let dash_array = style
        .and_then(|value| style_number_array(value, "dashArray"))
        .unwrap_or_default();
    let object_id = Some(object.id.clone());
    let arrow_head = payload_arrow_head(&object.payload, "arrowHead");
    if let Some(arrow_head) = arrow_head.filter(|arrow_head| arrow_head.length > 0.0) {
        let head_style = payload_arrow_endpoint_style(&object.payload, "head", "end");
        let tail_style = payload_arrow_endpoint_style(&object.payload, "tail", "start");
        render_arrow_line_object(
            out,
            &points,
            &stroke,
            stroke_width,
            &line_cap,
            &line_join,
            arrow_head,
            head_style,
            tail_style,
            object_id,
        );
        return;
    }

    push_polyline(
        out,
        points,
        &stroke,
        stroke_width,
        dash_array,
        Some(line_cap),
        Some(line_join),
        RenderRole::DocumentGraphic,
        object_id,
    );
}

fn endpoint_flag_enabled(value: &str, expected: &str) -> bool {
    value.eq_ignore_ascii_case(expected) || value.eq_ignore_ascii_case("both")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderArrowEndpointStyle {
    None,
    Full,
    Left,
    Right,
}

impl RenderArrowEndpointStyle {
    fn enabled(self) -> bool {
        !matches!(self, Self::None)
    }
}

fn payload_arrow_endpoint_style(
    payload: &crate::ObjectPayload,
    key: &str,
    expected: &str,
) -> RenderArrowEndpointStyle {
    let explicit_enabled =
        payload_string(payload, key).is_some_and(|value| endpoint_flag_enabled(&value, expected));
    let arrow_style = payload
        .extra
        .get("arrowHead")
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_str)
        .map(render_arrow_endpoint_style)
        .unwrap_or(RenderArrowEndpointStyle::None);
    if arrow_style.enabled() {
        return arrow_style;
    }
    if explicit_enabled {
        RenderArrowEndpointStyle::Full
    } else {
        RenderArrowEndpointStyle::None
    }
}

fn render_arrow_endpoint_style(value: &str) -> RenderArrowEndpointStyle {
    match value.to_ascii_lowercase().as_str() {
        "full" => RenderArrowEndpointStyle::Full,
        "half-left" | "halfleft" | "left" | "top" => RenderArrowEndpointStyle::Left,
        "half-right" | "halfright" | "right" | "bottom" => RenderArrowEndpointStyle::Right,
        _ => RenderArrowEndpointStyle::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn render_arrow_line_object(
    out: &mut Vec<RenderPrimitive>,
    points: &[Point],
    stroke: &str,
    stroke_width: f64,
    line_cap: &str,
    line_join: &str,
    arrow_head: ArrowHeadGeometry,
    head_style: RenderArrowEndpointStyle,
    tail_style: RenderArrowEndpointStyle,
    object_id: Option<String>,
) {
    if points.len() < 2 {
        return;
    }
    let start = points[0];
    let end = *points.last().unwrap_or(&start);
    if arrow_head.curve.abs() > crate::EPSILON && arrow_head.kind == ArrowHeadKind::Solid {
        render_curved_solid_arrow_line(
            out,
            start,
            end,
            stroke,
            stroke_width,
            arrow_head,
            head_style,
            tail_style,
            object_id,
        );
        return;
    }
    match arrow_head.kind {
        ArrowHeadKind::Solid => render_solid_arrow_line(
            out,
            start,
            end,
            stroke,
            stroke_width,
            arrow_head,
            head_style,
            tail_style,
            object_id,
        ),
        ArrowHeadKind::Hollow => render_hollow_arrow_line(
            out,
            start,
            end,
            stroke,
            stroke_width,
            arrow_head,
            head_style.enabled(),
            tail_style.enabled(),
            object_id,
        ),
        ArrowHeadKind::Open => render_open_arrow_line(
            out,
            start,
            end,
            stroke,
            stroke_width,
            line_cap,
            line_join,
            arrow_head,
            head_style.enabled(),
            tail_style.enabled(),
            object_id,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_curved_solid_arrow_line(
    out: &mut Vec<RenderPrimitive>,
    start: Point,
    end: Point,
    stroke: &str,
    stroke_width: f64,
    arrow_head: ArrowHeadGeometry,
    head_style: RenderArrowEndpointStyle,
    tail_style: RenderArrowEndpointStyle,
    object_id: Option<String>,
) {
    let points = curved_arrow_points(start, end, arrow_head.curve);
    if points.len() < 2 {
        render_solid_arrow_line(
            out,
            start,
            end,
            stroke,
            stroke_width,
            arrow_head,
            head_style,
            tail_style,
            object_id,
        );
        return;
    }
    let line_width = if arrow_head.bold { 4.0 } else { stroke_width };
    let start_trim = arrow_endpoint_shaft_trim(tail_style, arrow_head);
    let end_trim = arrow_endpoint_shaft_trim(head_style, arrow_head);
    if let Some(path) = curved_arrow_path(start, end, arrow_head.curve, start_trim, end_trim) {
        push_path(
            out,
            path.d,
            path.points,
            stroke,
            line_width,
            Vec::new(),
            Some("butt".to_string()),
            Some("round".to_string()),
            RenderRole::DocumentGraphic,
            object_id.clone(),
        );
    }
    render_arrow_no_go_on_points(out, &points, stroke, arrow_head, object_id.clone());
    if head_style.enabled() {
        let tangent_from = point_at_distance_from_end(&points, arrow_head.center_length)
            .unwrap_or_else(|| points[points.len().saturating_sub(2)]);
        push_polygon(
            out,
            solid_arrow_head_points(tangent_from, end, arrow_head, head_style),
            stroke,
            stroke,
            0.0,
            RenderRole::DocumentGraphic,
            object_id.clone(),
        );
    }
    if tail_style.enabled() {
        let tangent_from = point_at_distance_from_start(&points, arrow_head.center_length)
            .unwrap_or_else(|| *points.get(1).unwrap_or(&end));
        push_polygon(
            out,
            solid_arrow_head_points(tangent_from, start, arrow_head, tail_style),
            stroke,
            stroke,
            0.0,
            RenderRole::DocumentGraphic,
            object_id,
        );
    }
}

fn curved_arrow_points(start: Point, end: Point, sweep_degrees: f64) -> Vec<Point> {
    let Some(arc) = curved_arrow_arc(start, end, sweep_degrees) else {
        return vec![start, end];
    };
    let steps = ((sweep_degrees.abs() / 12.0).ceil() as usize).clamp(8, 32);
    (0..=steps)
        .map(|index| {
            let t = index as f64 / steps as f64;
            arc.point_at(arc.start_angle + arc.sweep * t)
        })
        .collect()
}

struct CurvedArrowArc {
    center: Point,
    radius: f64,
    start_angle: f64,
    sweep: f64,
}

impl CurvedArrowArc {
    fn point_at(&self, angle: f64) -> Point {
        Point::new(
            self.center.x + angle.cos() * self.radius,
            self.center.y + angle.sin() * self.radius,
        )
    }
}

struct CurvedArrowPath {
    d: String,
    points: Vec<Point>,
}

fn curved_arrow_arc(start: Point, end: Point, sweep_degrees: f64) -> Option<CurvedArrowArc> {
    let chord = Vector::new(end.x - start.x, end.y - start.y);
    let chord_length = chord.length();
    if chord_length <= crate::EPSILON || sweep_degrees.abs() <= crate::EPSILON {
        return None;
    }
    let sweep = -sweep_degrees.to_radians();
    let half = sweep.abs() * 0.5;
    let sin_half = half.sin().abs();
    if sin_half <= crate::EPSILON {
        return None;
    }
    let unit = chord.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let radius = chord_length / (2.0 * sin_half);
    let offset = radius * half.cos() * sweep.signum();
    let center = Point::new(
        (start.x + end.x) * 0.5 + normal.x * offset,
        (start.y + end.y) * 0.5 + normal.y * offset,
    );
    Some(CurvedArrowArc {
        center,
        radius,
        start_angle: (start.y - center.y).atan2(start.x - center.x),
        sweep,
    })
}

fn curved_arrow_path(
    start: Point,
    end: Point,
    sweep_degrees: f64,
    start_trim: f64,
    end_trim: f64,
) -> Option<CurvedArrowPath> {
    let arc = curved_arrow_arc(start, end, sweep_degrees)?;
    let total = arc.radius * arc.sweep.abs();
    if start_trim + end_trim >= total - crate::EPSILON {
        return None;
    }
    let sign = arc.sweep.signum();
    let start_angle = arc.start_angle + sign * start_trim / arc.radius;
    let end_angle = arc.start_angle + arc.sweep - sign * end_trim / arc.radius;
    let sweep = end_angle - start_angle;
    let start_point = arc.point_at(start_angle);
    let mut d = format!("M {} {}", start_point.x, start_point.y);
    let mut angle = start_angle;
    let segments = (sweep.abs() / (std::f64::consts::FRAC_PI_2)).ceil() as usize;
    let segments = segments.max(1);
    let delta = sweep / segments as f64;
    let mut points = Vec::with_capacity(33);
    let sample_steps = ((sweep.abs().to_degrees() / 6.0).ceil() as usize).clamp(8, 64);
    for index in 0..=sample_steps {
        let t = index as f64 / sample_steps as f64;
        points.push(arc.point_at(start_angle + sweep * t));
    }
    for _ in 0..segments {
        let next = angle + delta;
        let p0 = arc.point_at(angle);
        let p3 = arc.point_at(next);
        let k = 4.0 / 3.0 * (delta * 0.25).tan();
        let c1 = Point::new(
            p0.x - k * arc.radius * angle.sin(),
            p0.y + k * arc.radius * angle.cos(),
        );
        let c2 = Point::new(
            p3.x + k * arc.radius * next.sin(),
            p3.y - k * arc.radius * next.cos(),
        );
        d.push_str(&format!(
            " C {} {}, {} {}, {} {}",
            c1.x, c1.y, c2.x, c2.y, p3.x, p3.y
        ));
        angle = next;
    }
    Some(CurvedArrowPath { d, points })
}

fn polyline_length(points: &[Point]) -> f64 {
    points
        .windows(2)
        .map(|pair| pair[0].distance(pair[1]))
        .sum()
}

fn point_at_distance_from_start(points: &[Point], distance: f64) -> Option<Point> {
    if points.len() < 2 {
        return None;
    }
    if distance <= 0.0 {
        return points.first().copied();
    }
    let mut remaining = distance;
    for pair in points.windows(2) {
        let segment = pair[0].distance(pair[1]);
        if remaining <= segment {
            let t = if segment <= crate::EPSILON {
                0.0
            } else {
                remaining / segment
            };
            return Some(Point::new(
                pair[0].x + (pair[1].x - pair[0].x) * t,
                pair[0].y + (pair[1].y - pair[0].y) * t,
            ));
        }
        remaining -= segment;
    }
    points.last().copied()
}

fn point_at_distance_from_end(points: &[Point], distance: f64) -> Option<Point> {
    if points.len() < 2 {
        return None;
    }
    let total = polyline_length(points);
    point_at_distance_from_start(points, (total - distance).max(0.0))
}

#[allow(clippy::too_many_arguments)]
fn render_solid_arrow_line(
    out: &mut Vec<RenderPrimitive>,
    start: Point,
    end: Point,
    stroke: &str,
    stroke_width: f64,
    arrow_head: ArrowHeadGeometry,
    head_style: RenderArrowEndpointStyle,
    tail_style: RenderArrowEndpointStyle,
    object_id: Option<String>,
) {
    let Some((unit, _normal, length)) = arrow_axis(start, end) else {
        return;
    };
    let line_width = if arrow_head.bold { 4.0 } else { stroke_width };
    let start_shaft = start.translated(
        unit.scaled(arrow_endpoint_shaft_trim(tail_style, arrow_head).min(length * 0.45)),
    );
    let end_shaft = end.translated(
        unit.scaled(-arrow_endpoint_shaft_trim(head_style, arrow_head).min(length * 0.45)),
    );
    if start_shaft.distance(end_shaft) > crate::EPSILON {
        push_polyline(
            out,
            vec![start_shaft, end_shaft],
            stroke,
            line_width,
            Vec::new(),
            Some("butt".to_string()),
            Some("miter".to_string()),
            RenderRole::DocumentGraphic,
            object_id.clone(),
        );
    }
    render_arrow_no_go_on_axis(out, start, end, stroke, arrow_head, object_id.clone());
    if head_style.enabled() {
        push_polygon(
            out,
            solid_arrow_head_points(start, end, arrow_head, head_style),
            stroke,
            stroke,
            0.0,
            RenderRole::DocumentGraphic,
            object_id.clone(),
        );
    }
    if tail_style.enabled() {
        push_polygon(
            out,
            solid_arrow_head_points(end, start, arrow_head, tail_style),
            stroke,
            stroke,
            0.0,
            RenderRole::DocumentGraphic,
            object_id,
        );
    }
}

fn render_arrow_no_go_on_points(
    out: &mut Vec<RenderPrimitive>,
    points: &[Point],
    stroke: &str,
    arrow_head: ArrowHeadGeometry,
    object_id: Option<String>,
) {
    if points.len() < 2 {
        return;
    }
    let total = polyline_length(points);
    if total <= crate::EPSILON {
        return;
    }
    let center = point_at_distance_from_start(points, total * 0.5).unwrap_or(points[0]);
    let before =
        point_at_distance_from_start(points, (total * 0.5 - 0.1).max(0.0)).unwrap_or(points[0]);
    let after = point_at_distance_from_start(points, (total * 0.5 + 0.1).min(total))
        .unwrap_or_else(|| *points.last().unwrap_or(&points[0]));
    render_arrow_no_go_mark(out, center, before, after, stroke, arrow_head, object_id);
}

fn render_arrow_no_go_on_axis(
    out: &mut Vec<RenderPrimitive>,
    start: Point,
    end: Point,
    stroke: &str,
    arrow_head: ArrowHeadGeometry,
    object_id: Option<String>,
) {
    let center = Point::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5);
    render_arrow_no_go_mark(out, center, start, end, stroke, arrow_head, object_id);
}

fn render_arrow_no_go_mark(
    out: &mut Vec<RenderPrimitive>,
    center: Point,
    tangent_from: Point,
    tangent_to: Point,
    stroke: &str,
    arrow_head: ArrowHeadGeometry,
    object_id: Option<String>,
) {
    if arrow_head.no_go == ArrowNoGoGeometry::None {
        return;
    }
    let Some((unit, normal, _length)) = arrow_axis(tangent_from, tangent_to) else {
        return;
    };
    let bar_width =
        (arrow_head.length * 0.14).clamp(1.4, 2.6) * if arrow_head.bold { 1.25 } else { 1.0 };
    match arrow_head.no_go {
        ArrowNoGoGeometry::None => {}
        ArrowNoGoGeometry::Cross => {
            let bar_length = arrow_head.length * 2.05;
            for axis in [
                Vector::new(unit.x + normal.x, unit.y + normal.y).normalized(),
                Vector::new(unit.x - normal.x, unit.y - normal.y).normalized(),
            ] {
                push_polygon(
                    out,
                    no_go_bar_points(center, axis, bar_length, bar_width),
                    stroke,
                    stroke,
                    0.0,
                    RenderRole::DocumentGraphic,
                    object_id.clone(),
                );
            }
        }
        ArrowNoGoGeometry::Hash => {
            let bar_length = arrow_head.length * 2.25;
            let axis = Vector::new(unit.x - normal.x * 2.0, unit.y - normal.y * 2.0).normalized();
            let offset = unit.scaled(arrow_head.length * 0.36);
            for center in [
                center.translated(offset.scaled(-0.5)),
                center.translated(offset.scaled(0.5)),
            ] {
                push_polygon(
                    out,
                    no_go_bar_points(center, axis, bar_length, bar_width),
                    stroke,
                    stroke,
                    0.0,
                    RenderRole::DocumentGraphic,
                    object_id.clone(),
                );
            }
        }
    }
}

fn no_go_bar_points(center: Point, axis: Vector, length: f64, width: f64) -> Vec<Point> {
    let normal = Vector::new(-axis.y, axis.x);
    let half_length = length * 0.5;
    let half_width = width * 0.5;
    vec![
        center
            .translated(axis.scaled(-half_length))
            .translated(normal.scaled(-half_width)),
        center
            .translated(axis.scaled(half_length))
            .translated(normal.scaled(-half_width)),
        center
            .translated(axis.scaled(half_length))
            .translated(normal.scaled(half_width)),
        center
            .translated(axis.scaled(-half_length))
            .translated(normal.scaled(half_width)),
    ]
}

fn solid_arrow_head_points(
    from: Point,
    to: Point,
    arrow_head: ArrowHeadGeometry,
    style: RenderArrowEndpointStyle,
) -> Vec<Point> {
    if style == RenderArrowEndpointStyle::Full {
        return arrow_head_points(from, to, arrow_head);
    }
    let Some((unit, normal, _length)) = arrow_axis(from, to) else {
        return Vec::new();
    };
    let head_length = arrow_head
        .length
        .max(crate::ARROW_SHAPE_MIN_HEAD_LENGTH_CM.value());
    let head_half_width =
        (arrow_head.width + 0.05).max(crate::ARROW_SHAPE_MIN_HEAD_WIDTH_CM.value() * 0.5);
    let notch_length = arrow_head
        .center_length
        .max(crate::ARROW_SHAPE_MIN_NOTCH_LENGTH_CM.value())
        .min(head_length - crate::ARROW_SHAPE_MIN_HEAD_TO_NOTCH_GAP_CM.value());
    let notch = Point::new(to.x - unit.x * notch_length, to.y - unit.y * notch_length);
    let right = Point::new(
        to.x - unit.x * head_length + normal.x * head_half_width,
        to.y - unit.y * head_length + normal.y * head_half_width,
    );
    let left = Point::new(
        to.x - unit.x * head_length - normal.x * head_half_width,
        to.y - unit.y * head_length - normal.y * head_half_width,
    );
    let inner_half_width = head_half_width * 0.53;
    let right_inner = Point::new(
        notch.x + normal.x * inner_half_width,
        notch.y + normal.y * inner_half_width,
    );
    let left_inner = Point::new(
        notch.x - normal.x * inner_half_width,
        notch.y - normal.y * inner_half_width,
    );
    match style {
        RenderArrowEndpointStyle::Left => vec![to, notch, left_inner, left],
        RenderArrowEndpointStyle::Right => vec![to, right, right_inner, notch],
        RenderArrowEndpointStyle::Full | RenderArrowEndpointStyle::None => Vec::new(),
    }
}

fn arrow_endpoint_shaft_trim(
    style: RenderArrowEndpointStyle,
    arrow_head: ArrowHeadGeometry,
) -> f64 {
    match style {
        RenderArrowEndpointStyle::Full => arrow_head.center_length,
        RenderArrowEndpointStyle::Left | RenderArrowEndpointStyle::Right => arrow_head.length,
        RenderArrowEndpointStyle::None => 0.0,
    }
}

#[allow(clippy::too_many_arguments)]
fn render_hollow_arrow_line(
    out: &mut Vec<RenderPrimitive>,
    start: Point,
    end: Point,
    stroke: &str,
    stroke_width: f64,
    arrow_head: ArrowHeadGeometry,
    has_head: bool,
    has_tail: bool,
    object_id: Option<String>,
) {
    let Some(points) = hollow_arrow_outline_points(start, end, arrow_head, has_head, has_tail)
    else {
        return;
    };
    push_polygon(
        out,
        points,
        "none",
        stroke,
        if arrow_head.bold {
            2.0
        } else {
            stroke_width.max(1.0)
        },
        RenderRole::DocumentGraphic,
        object_id,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_open_arrow_line(
    out: &mut Vec<RenderPrimitive>,
    start: Point,
    end: Point,
    stroke: &str,
    stroke_width: f64,
    line_cap: &str,
    line_join: &str,
    arrow_head: ArrowHeadGeometry,
    has_head: bool,
    has_tail: bool,
    object_id: Option<String>,
) {
    let Some((unit, normal, length)) = arrow_axis(start, end) else {
        return;
    };
    let line_width = if arrow_head.bold {
        2.0
    } else {
        stroke_width.max(1.0)
    };
    let shaft_half_width = open_arrow_shaft_half_width(arrow_head);
    let head_length = arrow_head.length.min(length * 0.45);
    let neck_offset = (head_length * 0.5).min(length * 0.3);
    let head_half_width = open_arrow_head_half_width(arrow_head);
    let start_neck = if has_tail {
        start.translated(unit.scaled(neck_offset))
    } else {
        start
    };
    let end_neck = if has_head {
        end.translated(unit.scaled(-neck_offset))
    } else {
        end
    };

    if has_tail {
        push_polyline(
            out,
            open_arrow_head_outline_points(
                start,
                unit.scaled(-1.0),
                normal,
                head_length,
                neck_offset,
                shaft_half_width,
                head_half_width,
            ),
            stroke,
            line_width,
            Vec::new(),
            Some(line_cap.to_string()),
            Some(line_join.to_string()),
            RenderRole::DocumentGraphic,
            object_id.clone(),
        );
    }
    if start_neck.distance(end_neck) > crate::EPSILON {
        push_polyline(
            out,
            vec![
                start_neck.translated(normal.scaled(shaft_half_width)),
                end_neck.translated(normal.scaled(shaft_half_width)),
            ],
            stroke,
            line_width,
            Vec::new(),
            Some(line_cap.to_string()),
            Some(line_join.to_string()),
            RenderRole::DocumentGraphic,
            object_id.clone(),
        );
        push_polyline(
            out,
            vec![
                start_neck.translated(normal.scaled(-shaft_half_width)),
                end_neck.translated(normal.scaled(-shaft_half_width)),
            ],
            stroke,
            line_width,
            Vec::new(),
            Some(line_cap.to_string()),
            Some(line_join.to_string()),
            RenderRole::DocumentGraphic,
            object_id.clone(),
        );
    }
    if has_head {
        push_polyline(
            out,
            open_arrow_head_outline_points(
                end,
                unit,
                normal,
                head_length,
                neck_offset,
                shaft_half_width,
                head_half_width,
            ),
            stroke,
            line_width,
            Vec::new(),
            Some(line_cap.to_string()),
            Some(line_join.to_string()),
            RenderRole::DocumentGraphic,
            object_id.clone(),
        );
    }
}

fn open_arrow_shaft_half_width(arrow_head: ArrowHeadGeometry) -> f64 {
    let width = arrow_head.center_length.max(arrow_head.length) * 0.5;
    if arrow_head.bold {
        width * 1.15
    } else {
        width
    }
}

fn open_arrow_head_half_width(arrow_head: ArrowHeadGeometry) -> f64 {
    let width = arrow_head.center_length.max(arrow_head.length);
    if arrow_head.bold {
        width * 1.15
    } else {
        width
    }
}

fn open_arrow_head_outline_points(
    tip: Point,
    outward: crate::Vector,
    normal: crate::Vector,
    head_length: f64,
    neck_offset: f64,
    shaft_half_width: f64,
    head_half_width: f64,
) -> Vec<Point> {
    vec![
        tip.translated(outward.scaled(-neck_offset))
            .translated(normal.scaled(shaft_half_width)),
        tip.translated(outward.scaled(-head_length))
            .translated(normal.scaled(head_half_width)),
        tip,
        tip.translated(outward.scaled(-head_length))
            .translated(normal.scaled(-head_half_width)),
        tip.translated(outward.scaled(-neck_offset))
            .translated(normal.scaled(-shaft_half_width)),
    ]
}

pub(super) fn render_text_object(
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

pub(super) fn render_shape_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
) {
    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    let style = ShapeStyleSpec::from_style(style);
    let Some(geometry) = ShapeGeometry::from_object(object) else {
        return;
    };
    render_shape_geometry(out, &object.id, &geometry, style);
}

struct ShapeStyleSpec {
    fill: Option<String>,
    stroke: Option<String>,
    stroke_width: f64,
    dash_array: Vec<f64>,
    fill_gradient: Option<JsonValue>,
    render_style: ShapeRenderStyle,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShapeRenderStyle {
    Solid,
    Dashed,
    Filled,
    Shaded,
    Shadowed,
    Custom,
}

impl ShapeStyleSpec {
    fn from_style(style: Option<&JsonValue>) -> Self {
        let fill = style.and_then(|value| style_nullable_string(value, "fill"));
        let stroke = style.and_then(|value| style_nullable_string(value, "stroke"));
        let stroke_width = style
            .and_then(|value| {
                style_number(value, "strokeWidth").or_else(|| style_number(value, "stroke_width"))
            })
            .unwrap_or(px_to_cm(1.0));
        let dash_array = style
            .and_then(|value| style_number_array(value, "dashArray"))
            .unwrap_or_default();
        let fill_gradient = style
            .and_then(|value| value.get("fillGradient").cloned())
            .filter(|value| !value.is_null());
        let shaded = style
            .and_then(|value| value.get("shaded"))
            .and_then(JsonValue::as_bool)
            .unwrap_or(false);
        let shadowed = style
            .and_then(|value| value.get("shadow"))
            .and_then(JsonValue::as_bool)
            .unwrap_or(false);
        let render_style = if shaded {
            ShapeRenderStyle::Shaded
        } else if shadowed {
            ShapeRenderStyle::Shadowed
        } else if fill.is_some() && stroke.is_none() && fill_gradient.is_none() {
            ShapeRenderStyle::Filled
        } else if fill.is_none() && stroke.is_some() && !dash_array.is_empty() {
            ShapeRenderStyle::Dashed
        } else if fill.is_none() && stroke.is_some() && dash_array.is_empty() {
            ShapeRenderStyle::Solid
        } else {
            ShapeRenderStyle::Custom
        };
        Self {
            fill,
            stroke,
            stroke_width,
            dash_array,
            fill_gradient,
            render_style,
        }
    }

    fn base_color(&self) -> &str {
        self.stroke
            .as_deref()
            .or(self.fill.as_deref())
            .unwrap_or("#000000")
    }
}

enum ShapeGeometry {
    Oval {
        center: Point,
        rx: f64,
        ry: f64,
        rotate: f64,
        ellipse: bool,
    },
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        corner_radius: Option<f64>,
        rounded: bool,
    },
}

impl ShapeGeometry {
    fn from_object(object: &SceneObject) -> Option<Self> {
        let [tx, ty] = object.transform.translate;
        let kind = payload_string(&object.payload, "kind").unwrap_or_else(|| "rect".to_string());
        if matches!(kind.as_str(), "circle" | "ellipse") {
            let center = payload_point(&object.payload, "center")?;
            let major_axis_end = payload_point(&object.payload, "majorAxisEnd")?;
            let minor_axis_end = payload_point(&object.payload, "minorAxisEnd")?;
            let rx = center.distance(major_axis_end);
            let ry = center.distance(minor_axis_end);
            if rx <= crate::EPSILON || ry <= crate::EPSILON {
                return None;
            }
            return Some(Self::Oval {
                center,
                rx,
                ry,
                rotate: crate::angle_between(center, major_axis_end),
                ellipse: kind == "ellipse",
            });
        }

        let [_, _, width, height] = object.payload.bbox?;
        if width <= 0.0 || height <= 0.0 {
            return None;
        }
        let corner_radius =
            payload_number(&object.payload, "cornerRadius").filter(|value| *value > 0.0);
        Some(Self::Rect {
            x: tx,
            y: ty,
            width,
            height,
            corner_radius,
            rounded: kind == "roundRect",
        })
    }

    fn fill_path_d(&self) -> String {
        match *self {
            Self::Oval {
                center,
                rx,
                ry,
                rotate,
                ellipse,
            } => oval_path_d(center, rx, ry, rotate, ellipse),
            Self::Rect {
                x,
                y,
                width,
                height,
                corner_radius,
                rounded,
            } => {
                if rounded {
                    rounded_rect_path_d(x, y, width, height, corner_radius.unwrap_or(0.0))
                } else {
                    rect_path_d(x, y, width, height)
                }
            }
        }
    }

    fn outline_path_d(&self, dash_array: &[f64]) -> String {
        match *self {
            Self::Oval {
                center,
                rx,
                ry,
                rotate,
                ellipse,
            } => oval_path_d(center, rx, ry, rotate, ellipse || !dash_array.is_empty()),
            _ => self.fill_path_d(),
        }
    }

    fn shifted_fill_path_d(&self, dx: f64, dy: f64) -> String {
        match *self {
            Self::Oval {
                center,
                rx,
                ry,
                rotate,
                ellipse,
            } => oval_path_d(
                center.translated(crate::Vector::new(dx, dy)),
                rx,
                ry,
                rotate,
                ellipse,
            ),
            Self::Rect {
                x,
                y,
                width,
                height,
                corner_radius,
                rounded,
            } => {
                if rounded {
                    rounded_rect_path_d(x + dx, y + dy, width, height, corner_radius.unwrap_or(0.0))
                } else {
                    rect_path_d(x + dx, y + dy, width, height)
                }
            }
        }
    }

    fn bounds_points(&self) -> Vec<Point> {
        match *self {
            Self::Oval { center, rx, ry, .. } => ellipse_bounds_points(center, rx, ry),
            Self::Rect {
                x,
                y,
                width,
                height,
                ..
            } => vec![
                Point::new(x, y),
                Point::new(x + width, y),
                Point::new(x + width, y + height),
                Point::new(x, y + height),
            ],
        }
    }

    fn shadow_bounds_points(&self, offset: f64) -> Vec<Point> {
        match *self {
            Self::Oval { center, rx, ry, .. } => vec![
                Point::new(center.x - rx, center.y - ry),
                Point::new(center.x + rx + offset, center.y + ry + offset),
            ],
            Self::Rect {
                x,
                y,
                width,
                height,
                ..
            } => vec![
                Point::new(x, y),
                Point::new(x + width + offset, y + height + offset),
            ],
        }
    }
}

fn render_shape_geometry(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    geometry: &ShapeGeometry,
    style: ShapeStyleSpec,
) {
    match style.render_style {
        ShapeRenderStyle::Solid | ShapeRenderStyle::Dashed => {
            if let Some(stroke) = style.stroke {
                push_shape_outline(
                    out,
                    object_id,
                    geometry,
                    stroke,
                    style.stroke_width,
                    style.dash_array,
                );
            }
        }
        ShapeRenderStyle::Filled => {
            push_shape_fill(
                out,
                object_id,
                geometry,
                style.fill.unwrap_or_else(|| "#000000".to_string()),
            );
            if matches!(
                geometry,
                ShapeGeometry::Rect { .. } | ShapeGeometry::Oval { ellipse: true, .. }
            ) {
                push_shape_outline(
                    out,
                    object_id,
                    geometry,
                    "#000000".to_string(),
                    0.05,
                    Vec::new(),
                );
            }
        }
        ShapeRenderStyle::Shaded => {
            push_shape_shaded_layers(out, object_id, geometry, style.base_color());
            if let Some(stroke) = style.stroke {
                if matches!(geometry, ShapeGeometry::Rect { .. }) {
                    push_shape_outline(out, object_id, geometry, stroke.clone(), 0.05, Vec::new());
                }
                let stroke_width = match geometry {
                    ShapeGeometry::Oval { ellipse: true, .. } => 0.05,
                    _ => style.stroke_width,
                };
                push_shape_outline(
                    out,
                    object_id,
                    geometry,
                    stroke,
                    stroke_width,
                    style.dash_array,
                );
            }
        }
        ShapeRenderStyle::Shadowed => {
            push_shape_shadow_path(
                out,
                object_id,
                geometry.shifted_fill_path_d(4.0, 4.0),
                geometry.fill_path_d(),
                shape_shadow_fill(style.stroke.as_deref(), style.fill.as_deref()),
                geometry.shadow_bounds_points(4.0),
            );
            if let Some(fill) = style.fill {
                push_shape_fill(out, object_id, geometry, fill);
            }
            if let Some(stroke) = style.stroke {
                push_shape_outline(
                    out,
                    object_id,
                    geometry,
                    stroke,
                    style.stroke_width,
                    style.dash_array,
                );
            }
        }
        ShapeRenderStyle::Custom => push_shape_custom(out, object_id, geometry, style),
    }
}

fn push_shape_fill(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    geometry: &ShapeGeometry,
    fill: String,
) {
    out.push(RenderPrimitive::FilledPath {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object_id.to_string()),
        d: geometry.fill_path_d(),
        points: geometry.bounds_points(),
        fill,
        fill_rule: None,
        clip_path_d: None,
        clip_rule: None,
    });
}

fn push_shape_outline(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    geometry: &ShapeGeometry,
    stroke: String,
    stroke_width: f64,
    dash_array: Vec<f64>,
) {
    out.push(RenderPrimitive::Path {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object_id.to_string()),
        bond_id: None,
        d: geometry.outline_path_d(&dash_array),
        points: geometry.bounds_points(),
        stroke,
        stroke_width,
        dash_array,
        line_cap: match geometry {
            ShapeGeometry::Rect { .. } => Some("butt".to_string()),
            ShapeGeometry::Oval { .. } => None,
        },
        line_join: match geometry {
            ShapeGeometry::Rect { .. } => Some("miter".to_string()),
            ShapeGeometry::Oval { .. } => None,
        },
    });
}

fn push_shape_shaded_layers(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    geometry: &ShapeGeometry,
    base_color: &str,
) {
    match *geometry {
        ShapeGeometry::Oval {
            center,
            rx,
            ry,
            rotate,
            ellipse,
        } => {
            push_shaded_ellipse_layers(out, object_id, ellipse, center, rx, ry, rotate, base_color)
        }
        ShapeGeometry::Rect {
            x,
            y,
            width,
            height,
            corner_radius,
            rounded,
        } => push_shaded_rect_layers(
            out,
            object_id,
            x,
            y,
            width,
            height,
            corner_radius,
            rounded,
            base_color,
        ),
    }
}

fn push_shape_custom(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    geometry: &ShapeGeometry,
    style: ShapeStyleSpec,
) {
    match geometry {
        ShapeGeometry::Rect {
            x,
            y,
            width,
            height,
            corner_radius,
            ..
        } => out.push(RenderPrimitive::Rect {
            role: RenderRole::DocumentGraphic,
            object_id: Some(object_id.to_string()),
            node_id: None,
            x: *x,
            y: *y,
            width: *width,
            height: *height,
            fill: style.fill,
            stroke: style.stroke,
            stroke_width: style.stroke_width,
            rx: *corner_radius,
            ry: *corner_radius,
            dash_array: style.dash_array,
            fill_gradient: style.fill_gradient,
        }),
        ShapeGeometry::Oval { .. } => {
            if let Some(fill) = style.fill {
                push_shape_fill(out, object_id, geometry, fill);
            }
            if let Some(stroke) = style.stroke {
                push_shape_outline(
                    out,
                    object_id,
                    geometry,
                    stroke,
                    style.stroke_width,
                    style.dash_array,
                );
            }
        }
    }
}

fn push_shape_shadow_path(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    shifted_shape_path: String,
    original_shape_path: String,
    fill: String,
    points: Vec<Point>,
) {
    let clip_path = shape_shadow_clip_path(&points, &original_shape_path);
    out.push(RenderPrimitive::FilledPath {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object_id.to_string()),
        d: shifted_shape_path,
        points,
        fill,
        fill_rule: None,
        clip_path_d: Some(clip_path),
        clip_rule: Some("evenodd".to_string()),
    });
}

fn push_shape_ellipse_fill(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    center: Point,
    rx: f64,
    ry: f64,
    rotate: f64,
    use_cubic: bool,
    fill: String,
) {
    out.push(RenderPrimitive::FilledPath {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object_id.to_string()),
        d: oval_path_d(center, rx, ry, rotate, use_cubic),
        points: ellipse_bounds_points(center, rx, ry),
        fill,
        fill_rule: None,
        clip_path_d: None,
        clip_rule: None,
    });
}

fn ellipse_bounds_points(center: Point, rx: f64, ry: f64) -> Vec<Point> {
    vec![
        Point::new(center.x - rx, center.y - ry),
        Point::new(center.x + rx, center.y + ry),
    ]
}

fn shape_shadow_clip_path(points: &[Point], original_shape_path: &str) -> String {
    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let padding = 5.0;
    let left = min_x - padding;
    let top = min_y - padding;
    let right = max_x + padding;
    let bottom = max_y + padding;
    format!(
        "M {left},{top} L {right},{top} L {right},{bottom} L {left},{bottom} L {left},{top} {original_shape_path}"
    )
}

#[allow(clippy::too_many_arguments)]
fn push_shape_rect_fill(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    corner_radius: Option<f64>,
    fill: String,
) {
    let d = if corner_radius.is_some_and(|radius| radius > crate::EPSILON) {
        rounded_rect_path_d(x, y, width, height, corner_radius.unwrap_or(0.0))
    } else {
        rect_path_d(x, y, width, height)
    };
    out.push(RenderPrimitive::FilledPath {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object_id.to_string()),
        d,
        points: vec![Point::new(x, y), Point::new(x + width, y + height)],
        fill,
        fill_rule: None,
        clip_path_d: None,
        clip_rule: None,
    });
}

const SHADED_LEVELS: &[&str] = &[
    "#000000", "#0f0f0f", "#1e1e1e", "#2d2d2d", "#3b3b3b", "#494949", "#565656", "#636363",
    "#6f6f6f", "#7b7b7b", "#868686", "#919191", "#9b9b9b", "#a5a5a5", "#aeaeae", "#b7b7b7",
    "#bfbfbf", "#c7c7c7", "#cecece", "#d5d5d5", "#dbdbdb", "#e1e1e1", "#e6e6e6", "#ebebeb",
    "#efefef", "#f3f3f3", "#f6f6f6", "#f9f9f9", "#fbfbfb", "#fdfdfd", "#fefefe", "#ffffff",
];

const CIRCLE_SHADED_LEVELS: &[&str] = &[
    "#000000", "#0f0f0f", "#1e1e1e", "#2d2d2d", "#3b3b3b", "#494949", "#565656", "#636363",
    "#6f6f6f", "#7b7b7b", "#868686", "#919191", "#9b9b9b", "#a5a5a5", "#aeaeae", "#b7b7b7",
    "#bfbfbf", "#c6c6c6", "#cecece", "#d4d4d4", "#dbdbdb", "#e0e0e0", "#e6e6e6", "#eaeaea",
    "#efefef", "#f2f2f2", "#f6f6f6", "#f8f8f8", "#fbfbfb", "#fcfcfc", "#fefefe", "#fefefe",
];

const CIRCLE_SHADED_REMAIN_RATIO: f64 = 0.152_470_445_589_572_57;
const CIRCLE_SHADED_CENTER_SHIFT_RATIO: f64 = 0.484_377_144_287_654_77;
const ELLIPSE_SHADED_REMAIN_RATIO: f64 = 0.111_974_358_974_358_58;
const ELLIPSE_SHADED_CENTER_SHIFT_RATIO: f64 = 0.484_730_769_230_768_24;
const RECT_SHADED_INSET_RATIO: f64 = 0.058_648_052_902_278_19;
const ROUND_RECT_SHADED_INSET_RATIO: f64 = 0.127_129_977_460_556;
const RECT_SHADED_REMAIN_RATIO: f64 = 0.111_976_487_876_561_09;

#[allow(clippy::too_many_arguments)]
fn push_shaded_ellipse_layers(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    use_cubic: bool,
    center: Point,
    rx: f64,
    ry: f64,
    rotate: f64,
    base_color: &str,
) {
    let is_circle = (rx - ry).abs() <= crate::EPSILON;
    let levels = if is_circle {
        CIRCLE_SHADED_LEVELS
    } else {
        SHADED_LEVELS
    };
    let remain_ratio = if is_circle {
        CIRCLE_SHADED_REMAIN_RATIO
    } else {
        ELLIPSE_SHADED_REMAIN_RATIO
    };
    let shift_ratio = if is_circle {
        CIRCLE_SHADED_CENTER_SHIFT_RATIO
    } else {
        ELLIPSE_SHADED_CENTER_SHIFT_RATIO
    };
    let max_index = (levels.len() - 1) as f64;
    for (index, level) in levels.iter().enumerate() {
        let t = index as f64 / max_index;
        let layer_rx = rx * (1.0 - (1.0 - remain_ratio) * t);
        let layer_ry = ry * (1.0 - (1.0 - remain_ratio) * t);
        let layer_center = center.translated(crate::Vector::new(
            -shift_ratio * rx * t,
            -shift_ratio * ry * t,
        ));
        push_shape_ellipse_fill(
            out,
            object_id,
            layer_center,
            layer_rx,
            layer_ry,
            rotate,
            use_cubic,
            shaded_level_color(base_color, level, t),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_shaded_rect_layers(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    corner_radius: Option<f64>,
    rounded: bool,
    base_color: &str,
) {
    let inset_ratio = if rounded {
        ROUND_RECT_SHADED_INSET_RATIO
    } else {
        RECT_SHADED_INSET_RATIO
    };
    let max_index = (SHADED_LEVELS.len() - 1) as f64;
    for (index, level) in SHADED_LEVELS.iter().enumerate() {
        let t = index as f64 / max_index;
        let layer_x = x + width * inset_ratio * t;
        let layer_y = y + height * inset_ratio * t;
        let layer_width = width * (1.0 - (1.0 - RECT_SHADED_REMAIN_RATIO) * t);
        let layer_height = height * (1.0 - (1.0 - RECT_SHADED_REMAIN_RATIO) * t);
        let layer_radius = corner_radius.map(|radius| {
            radius
                .min(layer_width * 0.5)
                .min(layer_height * 0.5)
                .max(0.0)
        });
        push_shape_rect_fill(
            out,
            object_id,
            layer_x,
            layer_y,
            layer_width,
            layer_height,
            layer_radius,
            shaded_level_color(base_color, level, t),
        );
    }
}

fn shaded_level_color(base_color: &str, gray: &str, t: f64) -> String {
    let Some((r, g, b)) = parse_hex_color(base_color) else {
        return gray.to_string();
    };
    if r == 0 && g == 0 && b == 0 {
        return gray.to_string();
    }
    let mix = |channel: u8| -> u8 { (channel as f64 + (255.0 - channel as f64) * t).round() as u8 };
    format!("#{:02x}{:02x}{:02x}", mix(r), mix(g), mix(b))
}

fn parse_hex_color(value: &str) -> Option<(u8, u8, u8)> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    Some((
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ))
}

fn rounded_rect_path_d(x: f64, y: f64, width: f64, height: f64, radius: f64) -> String {
    let r = radius.min(width * 0.5).min(height * 0.5).max(0.0);
    if r <= crate::EPSILON {
        return rect_path_d(x, y, width, height);
    }
    let right = x + width;
    let bottom = y + height;
    let k = r * 0.552_284_749_830_793_6;
    format!(
        "M {x},{bottom_start} C {x},{bottom_start} {x},{top_left_c1} {x},{top_left_start} C {x},{top_left_c2} {top_left_c3},{y} {top_left_end},{y} C {top_left_end},{y} {top_right_start},{y} {top_right_start},{y} C {top_right_c1},{y} {right},{top_left_c2} {right},{top_left_start} C {right},{top_left_start} {right},{bottom_start} {right},{bottom_start} C {right},{bottom_c1} {top_right_c1},{bottom} {top_right_start},{bottom} C {top_right_start},{bottom} {top_left_end},{bottom} {top_left_end},{bottom} C {top_left_c3},{bottom} {x},{bottom_c1} {x},{bottom_start}",
        top_left_start = y + r,
        top_left_c1 = y + r,
        top_left_c2 = y + r - k,
        top_left_c3 = x + r - k,
        top_left_end = x + r,
        top_right_start = right - r,
        top_right_c1 = right - r + k,
        bottom_start = bottom - r,
        bottom_c1 = bottom - r + k,
    )
}

fn rect_path_d(x: f64, y: f64, width: f64, height: f64) -> String {
    let right = x + width;
    let bottom = y + height;
    format!(
        "M {right},{bottom} C {right},{bottom} {right},{y} {right},{y} C {right},{y} {x},{y} {x},{y} C {x},{y} {x},{bottom} {x},{bottom} C {x},{bottom} {right},{bottom} {right},{bottom}"
    )
}

fn oval_path_d(center: Point, rx: f64, ry: f64, rotate: f64, use_cubic: bool) -> String {
    if use_cubic {
        return ellipse_cubic_path_d(center, rx, ry, rotate);
    }
    ellipse_path_d(center, rx, ry, rotate)
}

fn ellipse_cubic_path_d(center: Point, rx: f64, ry: f64, rotate: f64) -> String {
    let k = 0.552_284_749_830_793_6;
    let major = crate::direction_from_angle(rotate);
    let minor = crate::direction_from_angle(rotate + 90.0);
    let left = center.translated(major.scaled(-rx));
    let right = center.translated(major.scaled(rx));
    let bottom = center.translated(minor.scaled(ry));
    let top = center.translated(minor.scaled(-ry));
    let c1 = left.translated(minor.scaled(k * ry));
    let c2 = bottom.translated(major.scaled(-k * rx));
    let c3 = bottom.translated(major.scaled(k * rx));
    let c4 = right.translated(minor.scaled(k * ry));
    let c5 = right.translated(minor.scaled(-k * ry));
    let c6 = top.translated(major.scaled(k * rx));
    let c7 = top.translated(major.scaled(-k * rx));
    let c8 = left.translated(minor.scaled(-k * ry));
    format!(
        "M {},{} C {},{} {},{} {},{} C {},{} {},{} {},{} C {},{} {},{} {},{} C {},{} {},{} {},{}",
        left.x,
        left.y,
        c1.x,
        c1.y,
        c2.x,
        c2.y,
        bottom.x,
        bottom.y,
        c3.x,
        c3.y,
        c4.x,
        c4.y,
        right.x,
        right.y,
        c5.x,
        c5.y,
        c6.x,
        c6.y,
        top.x,
        top.y,
        c7.x,
        c7.y,
        c8.x,
        c8.y,
        left.x,
        left.y
    )
}

fn ellipse_path_d(center: Point, rx: f64, ry: f64, rotate: f64) -> String {
    let unit = crate::direction_from_angle(rotate);
    let start = center.translated(unit.scaled(-rx));
    let end = center.translated(unit.scaled(rx));
    format!(
        "M {},{} A {rx},{ry} {rotate} 1 0 {},{} A {rx},{ry} {rotate} 1 0 {},{} Z",
        start.x, start.y, end.x, end.y, start.x, start.y
    )
}

fn payload_point(payload: &ObjectPayload, key: &str) -> Option<Point> {
    let coords = payload.extra.get(key)?.as_array()?;
    Some(Point::new(
        coords.first()?.as_f64()?,
        coords.get(1)?.as_f64()?,
    ))
}

fn shape_shadow_fill(stroke: Option<&str>, fill: Option<&str>) -> String {
    let color = fill.or(stroke).unwrap_or("#000000");
    if color.eq_ignore_ascii_case("#000000") {
        return "rgba(0,0,0,0.247059)".to_string();
    }
    let Some((r, g, b)) = parse_hex_color(color) else {
        return color.to_string();
    };
    format!("rgba({r},{g},{b},0.247059)")
}

pub(super) fn render_fragment_label(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
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
            push_knockout_polygon(out, polygon, object_id.clone());
        }
    }
    if fragment_label_is_invalid(label) {
        if let Some(box_value) = label_box_world(node, object) {
            out.push(RenderPrimitive::Rect {
                role: RenderRole::DocumentGraphic,
                object_id: object_id.clone(),
                node_id: Some(node.id.clone()),
                x: box_value.x1,
                y: box_value.y1,
                width: (box_value.x2 - box_value.x1).max(0.0),
                height: (box_value.y2 - box_value.y1).max(0.0),
                fill: Some("none".to_string()),
                stroke: Some("#d32f2f".to_string()),
                stroke_width: 1.0,
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
        push_text_for_node(
            out,
            world_position.x,
            world_position.y,
            String::new(),
            font_size,
            font_family,
            fill,
            Some(text_anchor),
            fragment_label_runs_for_line(label, 0, &lines[0]),
            object_id,
            Some(node.id.clone()),
        );
        return;
    }

    let label_box = label_box_world(node, object);
    let line_height = label_box
        .map(|box_value| (box_value.y2 - box_value.y1) / lines.len() as f64)
        .unwrap_or(font_size * 1.05);
    let box_top = label_box
        .map(|box_value| box_value.y1)
        .unwrap_or(world_position.y - line_height * 0.82);
    for (index, line) in lines.iter().enumerate() {
        let baseline_y = box_top + line_height * index as f64 + line_height * 0.82;
        push_text_for_node(
            out,
            world_position.x,
            baseline_y,
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
    label
        .meta
        .get("labelRecognition")
        .and_then(|value| value.get("status"))
        .and_then(serde_json::Value::as_str)
        == Some("invalid")
}
