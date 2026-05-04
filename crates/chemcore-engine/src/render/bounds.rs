use super::*;

pub fn render_primitives_bounds<'a>(
    primitives: impl IntoIterator<Item = &'a RenderPrimitive>,
) -> Option<[f64; 4]> {
    let mut bounds: Option<[f64; 4]> = None;
    for primitive in primitives {
        let Some([min_x, min_y, max_x, max_y]) = render_primitive_bounds(primitive) else {
            continue;
        };
        bounds = Some(match bounds {
            Some([current_min_x, current_min_y, current_max_x, current_max_y]) => [
                f64::min(current_min_x, min_x),
                f64::min(current_min_y, min_y),
                f64::max(current_max_x, max_x),
                f64::max(current_max_y, max_y),
            ],
            None => [min_x, min_y, max_x, max_y],
        });
    }
    bounds
}

pub(crate) fn fragment_bond_visual_bounds(
    document: &ChemcoreDocument,
    object: &SceneObject,
    fragment: &MoleculeFragment,
    bond: &Bond,
) -> Option<[f64; 4]> {
    let node_map: BTreeMap<&str, &Node> = fragment
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let contact_kernel =
        build_main_bond_contact_kernel(document, object, &fragment.bonds, &node_map);
    let mut out = Vec::new();
    render_fragment_bond(
        &mut out,
        document,
        object,
        &contact_kernel,
        &fragment.bonds,
        &node_map,
        bond,
        &molecule_stroke(document, object),
        None,
    );

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut found = false;

    for primitive in out {
        if !primitive_matches_bond(&primitive, &bond.id) {
            continue;
        }
        let Some([x1, y1, x2, y2]) = render_primitive_bounds(&primitive) else {
            continue;
        };
        min_x = min_x.min(x1);
        min_y = min_y.min(y1);
        max_x = max_x.max(x2);
        max_y = max_y.max(y2);
        found = true;
    }

    found.then_some([min_x, min_y, max_x, max_y])
}

pub(crate) fn primitive_matches_bond(primitive: &RenderPrimitive, bond_id: &str) -> bool {
    match primitive {
        RenderPrimitive::Line {
            bond_id: Some(current),
            ..
        }
        | RenderPrimitive::Polygon {
            bond_id: Some(current),
            ..
        }
        | RenderPrimitive::Polyline {
            bond_id: Some(current),
            ..
        }
        | RenderPrimitive::Path {
            bond_id: Some(current),
            ..
        } => current == bond_id,
        _ => false,
    }
}

pub fn render_primitive_bounds(primitive: &RenderPrimitive) -> Option<[f64; 4]> {
    match primitive {
        RenderPrimitive::Line {
            from,
            to,
            stroke_width,
            ..
        } => {
            let half_width = stroke_width * 0.5;
            Some([
                from.x.min(to.x) - half_width,
                from.y.min(to.y) - half_width,
                from.x.max(to.x) + half_width,
                from.y.max(to.y) + half_width,
            ])
        }
        RenderPrimitive::Polygon {
            points,
            stroke_width,
            ..
        }
        | RenderPrimitive::Polyline {
            points,
            stroke_width,
            ..
        }
        | RenderPrimitive::Path {
            points,
            stroke_width,
            ..
        } => point_list_bounds(points, *stroke_width * 0.5),
        RenderPrimitive::FilledPath { points, .. } => point_list_bounds(points, 0.0),
        RenderPrimitive::Rect {
            x,
            y,
            width,
            height,
            stroke_width,
            ..
        } => {
            let half_width = stroke_width * 0.5;
            Some([
                *x - half_width,
                *y - half_width,
                *x + *width + half_width,
                *y + *height + half_width,
            ])
        }
        RenderPrimitive::Ellipse {
            center,
            rx,
            ry,
            stroke_width,
            ..
        } => {
            let half_width = stroke_width * 0.5;
            Some([
                center.x - rx - half_width,
                center.y - ry - half_width,
                center.x + rx + half_width,
                center.y + ry + half_width,
            ])
        }
        RenderPrimitive::Circle { center, radius, .. } => Some([
            center.x - radius,
            center.y - radius,
            center.x + radius,
            center.y + radius,
        ]),
        RenderPrimitive::Text {
            x,
            y,
            font_size,
            line_height,
            box_width,
            text,
            runs,
            text_anchor,
            ..
        } => {
            let width = box_width.unwrap_or_else(|| estimate_text_width(text, runs, *font_size));
            let line_count = text.lines().count().max(1) as f64;
            let height = line_height.unwrap_or(*font_size * 1.2).max(*font_size) * line_count;
            let min_x = match text_anchor.as_deref() {
                Some("middle") => x - width * 0.5,
                Some("end") => x - width,
                _ => *x,
            };
            Some([
                min_x,
                y - font_size * 0.86,
                min_x + width,
                y - font_size * 0.86 + height,
            ])
        }
    }
}

fn estimate_text_width(text: &str, runs: &[LabelRun], fallback_font_size: f64) -> f64 {
    if !runs.is_empty() {
        return runs
            .iter()
            .map(|run| {
                let font_size = run.font_size.unwrap_or(fallback_font_size)
                    * crate::shared_script_scale_factor(run.script.as_deref());
                run.text.chars().count() as f64 * font_size * 0.56
            })
            .sum();
    }
    text.lines()
        .map(|line| line.chars().count() as f64 * fallback_font_size * 0.56)
        .fold(0.0, f64::max)
}

fn point_list_bounds(points: &[Point], margin: f64) -> Option<[f64; 4]> {
    let mut iter = points.iter().copied();
    let first = iter.next()?;
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x;
    let mut max_y = first.y;
    for point in iter {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Some([
        min_x - margin,
        min_y - margin,
        max_x + margin,
        max_y + margin,
    ])
}
