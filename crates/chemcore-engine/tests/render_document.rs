use chemcore_engine::{render_document, ChemcoreDocument, RenderPrimitive, RenderRole};
use serde_json::json;

fn fragment_document(nodes: serde_json::Value, bonds: serde_json::Value) -> ChemcoreDocument {
    serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": 0.85,
                "fontFamily": "Arial",
                "fontSize": 11.0
            }
        },
        "objects": [{
            "id": "obj_molecule_001",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_001" }
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 120.0, 120.0],
                    "nodes": nodes,
                    "bonds": bonds
                }
            }
        }
    }))
    .expect("document should deserialize")
}

fn polygon_area(points: &[chemcore_engine::Point]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        area += points[index].x * points[next].y - points[next].x * points[index].y;
    }
    area.abs() * 0.5
}

fn centered_bond_polygons(
    primitives: &[RenderPrimitive],
    center: chemcore_engine::Point,
) -> Vec<Vec<chemcore_engine::Point>> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001")
                && points.iter().any(|point| point.distance(center) <= 2.2) =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .collect()
}

fn object_bond_polygons(primitives: &[RenderPrimitive]) -> Vec<Vec<chemcore_engine::Point>> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .collect()
}

fn object_knockout_polygons(primitives: &[RenderPrimitive]) -> Vec<Vec<chemcore_engine::Point>> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentKnockout
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .collect()
}

fn object_bond_polygons_with_ids(
    primitives: &[RenderPrimitive],
) -> Vec<(String, Vec<chemcore_engine::Point>)> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                bond_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some((bond_id.clone().unwrap_or_default(), points.clone()))
            }
            _ => None,
        })
        .collect()
}

fn bond_axis_from_points(
    points: &[chemcore_engine::Point],
) -> Option<(chemcore_engine::Point, chemcore_engine::Point)> {
    if points.len() != 4 {
        return None;
    }
    Some((
        chemcore_engine::Point::new(
            (points[0].x + points[3].x) * 0.5,
            (points[0].y + points[3].y) * 0.5,
        ),
        chemcore_engine::Point::new(
            (points[1].x + points[2].x) * 0.5,
            (points[1].y + points[2].y) * 0.5,
        ),
    ))
}

fn bond_axis_length(points: &[chemcore_engine::Point]) -> Option<f64> {
    let (from, to) = bond_axis_from_points(points)?;
    Some(from.distance(to))
}

fn projection_range_on_axis(
    points: &[chemcore_engine::Point],
    axis_from: chemcore_engine::Point,
    axis_to: chemcore_engine::Point,
) -> Option<(f64, f64)> {
    let axis = chemcore_engine::Point::new(axis_to.x - axis_from.x, axis_to.y - axis_from.y);
    let length = (axis.x * axis.x + axis.y * axis.y).sqrt();
    if length <= 1.0e-6 {
        return None;
    }
    let unit_x = axis.x / length;
    let unit_y = axis.y / length;
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    for point in points {
        let projection = (point.x - axis_from.x) * unit_x + (point.y - axis_from.y) * unit_y;
        min_value = min_value.min(projection);
        max_value = max_value.max(projection);
    }
    Some((min_value, max_value))
}

fn object_bond_centerlines(
    primitives: &[RenderPrimitive],
) -> Vec<(chemcore_engine::Point, chemcore_engine::Point)> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Line {
                role,
                object_id,
                from,
                to,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some((*from, *to))
            }
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                bond_axis_from_points(points)
            }
            _ => None,
        })
        .collect()
}

fn shared_point_count(
    first: &[chemcore_engine::Point],
    second: &[chemcore_engine::Point],
    tolerance: f64,
) -> usize {
    first.iter()
        .filter(|point| second.iter().any(|other| point.distance(*other) <= tolerance))
        .count()
}

fn polygons_have_same_vertices(
    first: &[chemcore_engine::Point],
    second: &[chemcore_engine::Point],
    tolerance: f64,
) -> bool {
    first.len() == second.len()
        && first
            .iter()
            .all(|point| second.iter().any(|other| point.distance(*other) <= tolerance))
}

fn point_lies_on_segment(
    point: chemcore_engine::Point,
    from: chemcore_engine::Point,
    to: chemcore_engine::Point,
    tolerance: f64,
) -> bool {
    let cross =
        (point.x - from.x) * (to.y - from.y) - (point.y - from.y) * (to.x - from.x);
    if cross.abs() > tolerance {
        return false;
    }
    let dot =
        (point.x - from.x) * (to.x - from.x) + (point.y - from.y) * (to.y - from.y);
    if dot < -tolerance {
        return false;
    }
    let length_squared = (to.x - from.x).powi(2) + (to.y - from.y).powi(2);
    dot <= length_squared + tolerance
}

fn point_lies_on_polygon_boundary(
    point: chemcore_engine::Point,
    polygon: &[chemcore_engine::Point],
    tolerance: f64,
) -> bool {
    if polygon.len() < 2 {
        return false;
    }
    (0..polygon.len()).any(|index| {
        let next = (index + 1) % polygon.len();
        point_lies_on_segment(point, polygon[index], polygon[next], tolerance)
    })
}

fn average_closest_distance_to_point(
    points: &[chemcore_engine::Point],
    target: chemcore_engine::Point,
    count: usize,
) -> f64 {
    let mut distances: Vec<_> = points.iter().map(|point| point.distance(target)).collect();
    distances.sort_by(|a, b| a.total_cmp(b));
    distances.into_iter().take(count).sum::<f64>() / count as f64
}

fn side_double_outer_polygon_for_bond(
    polygons: &[(String, Vec<chemcore_engine::Point>)],
    bond_id: &str,
    shared_node: chemcore_engine::Point,
) -> Vec<chemcore_engine::Point> {
    polygons
        .iter()
        .filter(|(current_bond_id, _)| current_bond_id == bond_id)
        .max_by(|a, b| {
            average_closest_distance_to_point(&a.1, shared_node, 2).total_cmp(
                &average_closest_distance_to_point(&b.1, shared_node, 2),
            )
        })
        .map(|(_, points)| points.clone())
        .expect("side double outer polygon")
}

fn side_double_main_polygon_for_bond(
    polygons: &[(String, Vec<chemcore_engine::Point>)],
    bond_id: &str,
    shared_node: chemcore_engine::Point,
) -> Vec<chemcore_engine::Point> {
    polygons
        .iter()
        .filter(|(current_bond_id, _)| current_bond_id == bond_id)
        .min_by(|a, b| {
            average_closest_distance_to_point(&a.1, shared_node, 2).total_cmp(
                &average_closest_distance_to_point(&b.1, shared_node, 2),
            )
        })
        .map(|(_, points)| points.clone())
        .expect("side double main polygon")
}

fn closest_points_to_target(
    points: &[chemcore_engine::Point],
    target: chemcore_engine::Point,
    count: usize,
) -> Vec<chemcore_engine::Point> {
    let mut indexed: Vec<_> = points.iter().copied().collect();
    indexed.sort_by(|a, b| a.distance(target).total_cmp(&b.distance(target)));
    indexed.into_iter().take(count).collect()
}

#[test]
#[ignore = "backend debug output for bond vertices"]
fn debug_print_single_and_bold_bond_vertices() {
    let single_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
        ]),
    );
    let bold_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );

    let single_polygons = object_bond_polygons(&render_document(&single_document));
    let bold_polygons = object_bond_polygons(&render_document(&bold_document));

    let single = single_polygons
        .iter()
        .find(|points| points.len() == 4)
        .expect("single bond should render as one 4-point polygon");
    let bold = bold_polygons
        .iter()
        .find(|points| points.len() == 4)
        .expect("bold bond should render as one 4-point polygon");

    println!("single bond vertices:");
    for (index, point) in single.iter().enumerate() {
        println!("  p{} = ({:.6}, {:.6})", index + 1, point.x, point.y);
    }

    println!("bold bond vertices:");
    for (index, point) in bold.iter().enumerate() {
        println!("  p{} = ({:.6}, {:.6})", index + 1, point.x, point.y);
    }
}

#[test]
#[ignore = "backend debug output for joined bond vertices"]
fn debug_print_joined_single_and_bold_bond_vertices() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            },
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let polygons = object_bond_polygons(&render_document(&document));
    assert_eq!(polygons.len(), 2, "joined single+bold should render as two 4-point polygons");

    let first = polygons
        .iter()
        .find(|points| points.iter().any(|point| (point.x - 20.0).abs() < 0.001))
        .expect("first bond polygon");
    let second = polygons
        .iter()
        .find(|points| points.iter().any(|point| (point.x - 74.0).abs() < 1.0))
        .expect("second bond polygon");

    let mut shared = Vec::new();
    for first_point in first {
        for second_point in second {
            if first_point.distance(*second_point) <= 1.0e-6 {
                shared.push(*first_point);
            }
        }
    }

    println!("joined bold bond vertices:");
    for (index, point) in first.iter().enumerate() {
        println!("  p{} = ({:.6}, {:.6})", index + 1, point.x, point.y);
    }

    println!("joined single bond vertices:");
    for (index, point) in second.iter().enumerate() {
        println!("  p{} = ({:.6}, {:.6})", index + 1, point.x, point.y);
    }

    println!("shared vertices:");
    for (index, point) in shared.iter().enumerate() {
        println!("  s{} = ({:.6}, {:.6})", index + 1, point.x, point.y);
    }
}

#[test]
fn render_document_emits_arrow_line_primitives() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_arrow_default": {
                "kind": "stroke",
                "stroke": "#222222",
                "strokeWidth": 0.72,
                "lineCap": "butt",
                "lineJoin": "miter"
            }
        },
        "objects": [{
            "id": "obj_line_001",
            "type": "line",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_arrow_default",
            "payload": {
                "points": [[10.0, 20.0], [110.0, 20.0]],
                "head": "end",
                "arrowHead": {
                    "length": 22.5,
                    "width": 5.63,
                    "head": "full"
                }
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let shaft = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polyline {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic && object_id.as_deref() == Some("obj_line_001") => {
                Some(points.clone())
            }
            _ => None,
        })
        .expect("line shaft primitive");
    assert_eq!(shaft.len(), 2);
    assert!(shaft[1].x < 110.0);

    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Polygon {
            role,
            object_id,
            points,
            ..
        } if *role == RenderRole::DocumentGraphic
            && object_id.as_deref() == Some("obj_line_001")
            && points.len() == 4
    )));
}

#[test]
fn render_document_emits_text_lines_from_runs() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_text_001": {
                "kind": "text",
                "fontFamily": "Arial",
                "fontSize": 10.0,
                "fill": "#000000"
            }
        },
        "objects": [{
            "id": "obj_text_001",
            "type": "text",
            "visible": true,
            "zIndex": 20,
            "transform": { "translate": [30.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_text_001",
            "payload": {
                "text": "Na\nCl",
                "align": "center",
                "fontSize": 10.0,
                "lineHeight": 14.0,
                "preserveLines": true,
                "runs": [{
                    "text": "Na\nCl",
                    "fontFamily": "Arial",
                    "fontSize": 10.0,
                    "fill": "#000000",
                    "fontWeight": 400,
                    "fontStyle": "normal",
                    "script": "normal"
                }]
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let text_lines: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text {
                role,
                object_id,
                x,
                y,
                runs,
                text_anchor,
                ..
            } if *role == RenderRole::DocumentText && object_id.as_deref() == Some("obj_text_001") => {
                Some((*x, *y, runs.clone(), text_anchor.clone()))
            }
            _ => None,
        })
        .collect();

    assert_eq!(text_lines.len(), 2);
    assert!(text_lines.iter().all(|(x, _, _, _)| (*x - 30.0).abs() < 0.001));
    assert_eq!(text_lines[0].2[0].text, "Na");
    assert_eq!(text_lines[1].2[0].text, "Cl");
    assert!(text_lines[1].1 > text_lines[0].1);
    assert_eq!(text_lines[0].3.as_deref(), Some("middle"));
}

#[test]
fn render_document_emits_shape_rect_with_style() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape_001": {
                "kind": "shape",
                "fill": "#55f0f5",
                "stroke": "#000000",
                "strokeWidth": 0.7,
                "fillGradient": {
                    "type": "linear",
                    "x1": "0%",
                    "y1": "0%",
                    "x2": "0%",
                    "y2": "100%",
                    "stops": [
                        { "offset": "0%", "color": "#a2f7fa" },
                        { "offset": "100%", "color": "#4bd3d8" }
                    ]
                },
                "dashArray": [3.2, 2.8]
            }
        },
        "objects": [{
            "id": "obj_shape_001",
            "type": "shape",
            "visible": true,
            "zIndex": 5,
            "transform": { "translate": [18.77, 22.5], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_shape_001",
            "payload": {
                "kind": "roundRect",
                "bbox": [0.0, 0.0, 110.25, 81.0],
                "cornerRadius": 6.0
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let rect = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Rect {
                role,
                object_id,
                x,
                y,
                width,
                height,
                fill,
                stroke,
                stroke_width,
                rx,
                dash_array,
                fill_gradient,
                ..
            } if *role == RenderRole::DocumentGraphic && object_id.as_deref() == Some("obj_shape_001") => {
                Some((
                    *x,
                    *y,
                    *width,
                    *height,
                    fill.clone(),
                    stroke.clone(),
                    *stroke_width,
                    *rx,
                    dash_array.clone(),
                    fill_gradient.clone(),
                ))
            }
            _ => None,
        })
        .expect("shape rect primitive");

    assert!((rect.0 - 18.77).abs() < 0.001);
    assert!((rect.1 - 22.5).abs() < 0.001);
    assert!((rect.2 - 110.25).abs() < 0.001);
    assert!((rect.3 - 81.0).abs() < 0.001);
    assert_eq!(rect.4.as_deref(), Some("#55f0f5"));
    assert_eq!(rect.5.as_deref(), Some("#000000"));
    assert!((rect.6 - 0.7).abs() < 0.001);
    assert_eq!(rect.7, Some(6.0));
    assert_eq!(rect.8, vec![3.2, 2.8]);
    assert_eq!(
        rect.9
            .and_then(|value| value.get("stops").and_then(|stops| stops.as_array()).map(|stops| stops.len())),
        Some(2)
    );
}

#[test]
fn render_document_emits_fragment_label_text_and_knockout() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 200.0, "height": 120.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": 0.85,
                "fontFamily": "Arial",
                "fontSize": 11.0
            }
        },
        "objects": [{
            "id": "obj_molecule_001",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [12.0, 8.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_001" }
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 80.0, 40.0],
                    "nodes": [{
                        "id": "n1",
                        "element": "N",
                        "atomicNumber": 7,
                        "position": [20.0, 20.0],
                        "charge": 0,
                        "numHydrogens": 1,
                        "label": {
                            "text": "NH",
                            "sourceText": "NH",
                            "position": [16.4, 15.6],
                            "box": [16.4, 7.2, 23.6, 23.8],
                            "runs": [{
                                "text": "NH",
                                "fontFamily": "Arial",
                                "fontSize": 10.0,
                                "fill": "#000000",
                                "fontWeight": 400,
                                "fontStyle": "normal",
                                "script": "normal"
                            }],
                            "lines": ["H", "N"],
                            "lineRuns": [
                                [{
                                    "text": "H",
                                    "fontFamily": "Arial",
                                    "fontSize": 10.0,
                                    "fill": "#000000",
                                    "fontWeight": 400,
                                    "fontStyle": "normal",
                                    "script": "normal"
                                }],
                                [{
                                    "text": "N",
                                    "fontFamily": "Arial",
                                    "fontSize": 10.0,
                                    "fill": "#000000",
                                    "fontWeight": 400,
                                    "fontStyle": "normal",
                                    "script": "normal"
                                }]
                            ],
                            "align": "left",
                            "layout": "hetero-h-above",
                            "fontFamily": "Arial",
                            "fill": "#000000",
                            "fontSize": 10.0
                        }
                    }],
                    "bonds": []
                }
            }
        }
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect { role, object_id, fill, .. }
            if *role == RenderRole::DocumentKnockout
                && object_id.as_deref() == Some("obj_molecule_001")
                && fill.as_deref() == Some("#ffffff")
    )));

    let label_lines: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text {
                role,
                object_id,
                x,
                y,
                runs,
                ..
            } if *role == RenderRole::DocumentText && object_id.as_deref() == Some("obj_molecule_001") => {
                Some((*x, *y, runs.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(label_lines.len(), 2);
    assert!(label_lines.iter().all(|(x, _, _)| (*x - 28.4).abs() < 0.001));
    assert_eq!(label_lines[0].2[0].text, "H");
    assert_eq!(label_lines[1].2[0].text, "N");
    assert!(label_lines[1].1 > label_lines[0].1);
}

#[test]
fn render_document_emits_primitives_for_legacy_molblock_resource() {
    let molblock = concat!(
        "Legacy\n",
        "  Chemcore\n",
        "\n",
        "  2  1  0  0  0  0            999 V2000\n",
        "    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n",
        "    1.2000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n",
        "  1  2  1  0  0  0  0\n",
        "M  END\n"
    );
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 240.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": 0.85,
                "fontFamily": "Arial",
                "fontSize": 11.0
            }
        },
        "objects": [{
            "id": "obj_molecule_legacy",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [24.0, 18.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": {
                "resourceRef": "mol_legacy",
                "bbox": [0.0, 0.0, 96.0, 72.0]
            }
        }],
        "resources": {
            "mol_legacy": {
                "type": "molfile",
                "encoding": "chemical/x-mdl-molfile",
                "data": molblock
            }
        }
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Line { role, object_id, .. }
            if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_legacy")
    )));
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Text { role, object_id, text, .. }
            if *role == RenderRole::DocumentText
                && object_id.as_deref() == Some("obj_molecule_legacy")
                && text == "O"
    )));
}

#[test]
fn render_document_keeps_terminal_triple_outer_lines_full_length() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 3, "strokeWidth": 0.85 }
        ]),
    );

    let centerlines = object_bond_centerlines(&render_document(&document));

    assert_eq!(centerlines.len(), 3);
    for (from, to) in centerlines {
        assert!((from.x - 20.0).abs() < 0.001, "{from:?}");
        assert!((to.x - 56.0).abs() < 0.001, "{to:?}");
    }
}

#[test]
fn render_document_preserves_dashed_double_line_styles() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "double": { "placement": "left" },
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "dashed",
                    "left": "dashed",
                    "right": "solid"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons(&primitives);
    let knockouts = object_knockout_polygons(&primitives);

    assert_eq!(polygons.len(), 2);
    assert!(polygons.iter().all(|points| points.len() == 4));
    assert!(knockouts.len() >= 2, "{knockouts:?}");
    assert!(!primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Line { role, object_id, .. }
            if *role == RenderRole::DocumentBond && object_id.as_deref() == Some("obj_molecule_001")
    )));
}

#[test]
fn render_document_emits_polygon_for_bold_single_bond() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );

    let polygons: Vec<_> = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if role == RenderRole::DocumentBond && object_id.as_deref() == Some("obj_molecule_001") => {
                Some(points)
            }
            _ => None,
        })
        .collect();

    assert_eq!(polygons.len(), 1);
    assert_eq!(polygons[0].len(), 4);
    assert!(polygons[0].iter().any(|point| (point.y - 40.0).abs() > 0.5));
}

#[test]
fn render_document_emits_polygon_for_plain_single_bond() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons(&primitives);

    assert_eq!(polygons.len(), 1);
    assert_eq!(polygons[0].len(), 4);
    assert!(!primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Line { role, object_id, .. }
            if *role == RenderRole::DocumentBond && object_id.as_deref() == Some("obj_molecule_001")
    )));
}

#[test]
fn render_document_emits_main_contact_patches_for_connected_bold_and_single_bonds() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = centered_bond_polygons(&primitives, chemcore_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 2);
    assert!(polygons.iter().all(|points| points.len() == 4));
    assert!(polygons.iter().all(|points| polygon_area(points) > 0.01), "{polygons:?}");

    assert!(!primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Line {
            role,
            object_id,
            ..
        } if *role == RenderRole::DocumentBond && object_id.as_deref() == Some("obj_molecule_001")
    )));
}

#[test]
fn render_document_emits_two_way_main_contact_patches_for_plain_singles() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 },
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = centered_bond_polygons(&primitives, chemcore_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 2);
    assert!(polygons.iter().all(|points| points.len() == 4));
    assert!(polygons.iter().all(|points| polygon_area(points) > 0.01), "{polygons:?}");

    let total_bond_polygons: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond && object_id.as_deref() == Some("obj_molecule_001") => {
                Some(points.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(total_bond_polygons.len(), 2);
}

#[test]
fn render_document_emits_equal_length_cross_segments_for_bold_dashed_bond() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "dashed",
                    "left": "solid",
                    "right": "solid"
                },
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons(&primitives);
    let knockouts = object_knockout_polygons(&primitives);

    assert_eq!(polygons.len(), 1);
    assert_eq!(polygons[0].len(), 4);
    assert!(polygon_area(&polygons[0]) > 40.0, "{polygons:?}");
    assert!(knockouts.len() >= 2, "{knockouts:?}");
    assert!(knockouts
        .iter()
        .all(|points| points.iter().any(|point| point.y > 40.0) && points.iter().any(|point| point.y < 40.0)));
    let (axis_from, axis_to) = bond_axis_from_points(&polygons[0]).expect("hash bond axis");
    let (bond_start, bond_end) =
        projection_range_on_axis(&polygons[0], axis_from, axis_to).expect("hash bond range");
    let mut gaps: Vec<_> = knockouts
        .iter()
        .filter_map(|points| projection_range_on_axis(points, axis_from, axis_to))
        .collect();
    gaps.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut black_segments = Vec::new();
    let mut cursor = bond_start;
    for (gap_start, gap_end) in gaps {
        if gap_start > cursor + 1.0e-6 {
            black_segments.push(gap_start - cursor);
        }
        cursor = gap_end;
    }
    if bond_end > cursor + 1.0e-6 {
        black_segments.push(bond_end - cursor);
    }
    assert!(black_segments.len() >= 2, "{black_segments:?}");
    let first_black = black_segments[0];
    assert!(
        black_segments
            .iter()
            .all(|length| (length - first_black).abs() < 0.02),
        "{black_segments:?}"
    );
    assert!(!primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Line { role, object_id, .. }
            if *role == RenderRole::DocumentBond && object_id.as_deref() == Some("obj_molecule_001")
    )));
}

#[test]
fn render_document_emits_main_contact_patches_for_connected_bold_and_dashed_single_bonds() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "dashed",
                    "left": "solid",
                    "right": "solid"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = centered_bond_polygons(&primitives, chemcore_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 2);
    assert!(polygons.iter().all(|points| points.len() == 4));
    assert!(object_knockout_polygons(&primitives).len() >= 1);
}

#[test]
fn render_document_emits_primitives_for_wedge_and_hashed_wedge() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [92.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [128.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "end"
                }
            },
            {
                "id": "b2",
                "begin": "n3",
                "end": "n4",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "hashed-wedge",
                    "wideEnd": "end"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let bond_polygons: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond && object_id.as_deref() == Some("obj_molecule_001") => {
                Some(points.len())
            }
            _ => None,
        })
        .collect();
    let knockout_polygons = object_knockout_polygons(&primitives);

    assert!(bond_polygons.iter().all(|count| *count == 4), "{bond_polygons:?}");
    assert!(bond_polygons.len() >= 2);
    assert!(knockout_polygons.len() >= 1);
}

#[test]
fn render_document_emits_main_contact_patches_for_connected_single_and_solid_wedge() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "end"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = centered_bond_polygons(&primitives, chemcore_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 2);
    assert_eq!(polygons.iter().filter(|points| points.len() == 4).count(), 2);

    let wedge_polygon = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond && object_id.as_deref() == Some("obj_molecule_001") => {
                Some(points.clone())
            }
            _ => None,
        })
        .max_by(|a, b| polygon_area(a).total_cmp(&polygon_area(b)))
        .expect("solid wedge polygon");
    assert_eq!(wedge_polygon.len(), 4);
    assert_eq!(
        wedge_polygon
            .iter()
            .filter(|point| point.distance(chemcore_engine::Point::new(56.0, 40.0)) <= 2.2)
            .count(),
        2
    );
}

#[test]
fn render_document_keeps_hashed_wedge_mother_polygon_original_against_connected_single_bond() {
    let isolated = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "hashed-wedge",
                    "wideEnd": "end"
                }
            }
        ]),
    );
    let connected = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "hashed-wedge",
                    "wideEnd": "end"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );
    let isolated_polygon = object_bond_polygons_with_ids(&render_document(&isolated))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("isolated hashed wedge polygon");
    let connected_primitives = render_document(&connected);
    let connected_polygons = object_bond_polygons_with_ids(&connected_primitives);
    let hashed_wedge = connected_polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points.clone()))
        .expect("hashed wedge polygon");

    assert_eq!(hashed_wedge.len(), 4);
    assert!(polygons_have_same_vertices(
        &isolated_polygon,
        &hashed_wedge,
        1.0e-4,
    ));
    assert!(object_knockout_polygons(&connected_primitives).len() >= 1);
}

#[test]
fn render_document_keeps_hash_bond_mother_polygon_original_against_connected_single_bond() {
    let isolated = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "dashed",
                    "left": "solid",
                    "right": "solid"
                },
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );
    let connected = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "dashed",
                    "left": "solid",
                    "right": "solid"
                },
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );
    let isolated_polygon = object_bond_polygons_with_ids(&render_document(&isolated))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("isolated hash bond polygon");
    let connected_primitives = render_document(&connected);
    let hash_bond = object_bond_polygons_with_ids(&connected_primitives)
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("connected hash bond polygon");

    assert_eq!(hash_bond.len(), 4);
    assert!(polygons_have_same_vertices(
        &isolated_polygon,
        &hash_bond,
        1.0e-4,
    ));
    assert!(object_knockout_polygons(&connected_primitives).len() >= 1);
}

#[test]
fn render_document_retreats_hash_bond_mother_polygon_against_center_double_outer_line() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "dashed",
                    "left": "solid",
                    "right": "solid"
                },
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let hash_bond = object_bond_polygons_with_ids(&primitives)
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b2").then_some(points))
        .expect("hash bond polygon");
    let connected_end = closest_points_to_target(&hash_bond, chemcore_engine::Point::new(56.0, 40.0), 2);
    let unit = chemcore_engine::Point::new(18.0, -28.0);
    let unit_length = (unit.x * unit.x + unit.y * unit.y).sqrt();
    let unit_x = unit.x / unit_length;
    let unit_y = unit.y / unit_length;
    let projections: Vec<_> = connected_end
        .iter()
        .map(|point| (point.x - 56.0) * unit_x + (point.y - 40.0) * unit_y)
        .collect();

    assert_eq!(connected_end.len(), 2);
    assert!(
        (projections[0] - projections[1]).abs() <= 1.0e-4,
        "{hash_bond:?} {projections:?}"
    );
    assert!(projections.iter().all(|projection| *projection > 0.05), "{hash_bond:?} {projections:?}");
    assert!(object_knockout_polygons(&primitives).len() >= 1);
}

#[test]
fn render_document_retreats_hashed_wedge_mother_polygon_against_center_double_outer_line() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "hashed-wedge",
                    "wideEnd": "begin"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let hashed_wedge = object_bond_polygons_with_ids(&primitives)
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b2").then_some(points))
        .expect("hashed wedge polygon");
    let connected_end =
        closest_points_to_target(&hashed_wedge, chemcore_engine::Point::new(56.0, 40.0), 2);
    let unit = chemcore_engine::Point::new(18.0, -28.0);
    let unit_length = (unit.x * unit.x + unit.y * unit.y).sqrt();
    let unit_x = unit.x / unit_length;
    let unit_y = unit.y / unit_length;
    let projections: Vec<_> = connected_end
        .iter()
        .map(|point| (point.x - 56.0) * unit_x + (point.y - 40.0) * unit_y)
        .collect();

    assert_eq!(connected_end.len(), 2);
    assert!(
        (projections[0] - projections[1]).abs() <= 1.0e-4,
        "{hashed_wedge:?} {projections:?}"
    );
    assert!(
        projections.iter().all(|projection| *projection > 0.05),
        "{hashed_wedge:?} {projections:?}"
    );
    assert!(object_knockout_polygons(&primitives).len() >= 1);
}

#[test]
fn render_document_keeps_hash_bond_original_and_retreats_other_bonds_in_multi_bond_node() {
    let isolated = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "dashed",
                    "left": "solid",
                    "right": "solid"
                },
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [74.0, 68.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "dashed",
                    "left": "solid",
                    "right": "solid"
                },
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            },
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n2", "end": "n4", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let isolated_hash = object_bond_polygons_with_ids(&render_document(&isolated))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("isolated hash bond polygon");
    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let hash_bond = polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points.clone()))
        .expect("hash bond polygon");
    let branch_one = polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b2").then_some(points.clone()))
        .expect("branch one polygon");
    let branch_two = polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b3").then_some(points.clone()))
        .expect("branch two polygon");

    assert!(polygons_have_same_vertices(&isolated_hash, &hash_bond, 1.0e-4));
    assert!(
        average_closest_distance_to_point(&branch_one, chemcore_engine::Point::new(56.0, 40.0), 2) > 0.5,
        "{branch_one:?}"
    );
    assert!(
        average_closest_distance_to_point(&branch_two, chemcore_engine::Point::new(56.0, 40.0), 2) > 0.5,
        "{branch_two:?}"
    );
}

#[test]
fn render_document_keeps_hashed_wedge_original_and_retreats_other_bonds_in_multi_bond_node() {
    let isolated = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "hashed-wedge",
                    "wideEnd": "end"
                }
            }
        ]),
    );
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [74.0, 68.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "hashed-wedge",
                    "wideEnd": "end"
                }
            },
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n2", "end": "n4", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let isolated_wedge = object_bond_polygons_with_ids(&render_document(&isolated))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("isolated hashed wedge polygon");
    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let hashed_wedge = polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points.clone()))
        .expect("hashed wedge polygon");
    let branch_one = polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b2").then_some(points.clone()))
        .expect("branch one polygon");
    let branch_two = polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b3").then_some(points.clone()))
        .expect("branch two polygon");

    assert!(polygons_have_same_vertices(
        &isolated_wedge,
        &hashed_wedge,
        1.0e-4,
    ));
    assert!(
        average_closest_distance_to_point(&branch_one, chemcore_engine::Point::new(56.0, 40.0), 2) > 0.5,
        "{branch_one:?}"
    );
    assert!(
        average_closest_distance_to_point(&branch_two, chemcore_engine::Point::new(56.0, 40.0), 2) > 0.5,
        "{branch_two:?}"
    );
    assert!(object_knockout_polygons(&primitives).len() >= 1);
}

#[test]
fn render_document_retreats_hash_bond_against_solid_dashed_center_double_outer_line() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" },
                "lineStyles": {
                    "left": "solid",
                    "right": "dashed"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "dashed",
                    "left": "solid",
                    "right": "solid"
                },
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let hash_bond = object_bond_polygons_with_ids(&primitives)
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b2").then_some(points))
        .expect("hash bond polygon");
    let connected_end = closest_points_to_target(&hash_bond, chemcore_engine::Point::new(56.0, 40.0), 2);
    let unit = chemcore_engine::Point::new(18.0, -28.0);
    let unit_length = (unit.x * unit.x + unit.y * unit.y).sqrt();
    let unit_x = unit.x / unit_length;
    let unit_y = unit.y / unit_length;
    let projections: Vec<_> = connected_end
        .iter()
        .map(|point| (point.x - 56.0) * unit_x + (point.y - 40.0) * unit_y)
        .collect();

    assert_eq!(connected_end.len(), 2);
    assert!(
        (projections[0] - projections[1]).abs() <= 1.0e-4,
        "{hash_bond:?} {projections:?}"
    );
    assert!(projections.iter().all(|projection| *projection > 0.05), "{hash_bond:?} {projections:?}");
    assert!(object_knockout_polygons(&primitives).len() >= 1);
}

#[test]
fn render_document_retreats_hashed_wedge_against_double_dashed_center_double_outer_line() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" },
                "lineStyles": {
                    "left": "dashed",
                    "right": "dashed"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "hashed-wedge",
                    "wideEnd": "begin"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let hashed_wedge = object_bond_polygons_with_ids(&primitives)
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b2").then_some(points))
        .expect("hashed wedge polygon");
    let connected_end =
        closest_points_to_target(&hashed_wedge, chemcore_engine::Point::new(56.0, 40.0), 2);
    let unit = chemcore_engine::Point::new(18.0, -28.0);
    let unit_length = (unit.x * unit.x + unit.y * unit.y).sqrt();
    let unit_x = unit.x / unit_length;
    let unit_y = unit.y / unit_length;
    let projections: Vec<_> = connected_end
        .iter()
        .map(|point| (point.x - 56.0) * unit_x + (point.y - 40.0) * unit_y)
        .collect();

    assert_eq!(connected_end.len(), 2);
    assert!(
        (projections[0] - projections[1]).abs() <= 1.0e-4,
        "{hashed_wedge:?} {projections:?}"
    );
    assert!(
        projections.iter().all(|projection| *projection > 0.05),
        "{hashed_wedge:?} {projections:?}"
    );
    assert!(object_knockout_polygons(&primitives).len() >= 1);
}

#[test]
fn render_document_scales_side_double_offset_with_bond_length() {
    let short_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": {
                    "placement": "right"
                }
            }
        ]),
    );
    let long_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 80.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [92.0, 80.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": {
                    "placement": "right"
                }
            }
        ]),
    );

    let short_lines = object_bond_centerlines(&render_document(&short_document));
    let long_lines = object_bond_centerlines(&render_document(&long_document));

    let short_offset = short_lines
        .iter()
        .map(|(from, to)| ((from.y + to.y) / 2.0 - 40.0).abs())
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();
    let long_offset = long_lines
        .iter()
        .map(|(from, to)| ((from.y + to.y) / 2.0 - 80.0).abs())
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();

    assert!((long_offset - short_offset * 2.0).abs() < 0.05, "{short_offset} {long_offset}");
}

#[test]
fn render_document_keeps_terminal_side_double_outer_line_equal_length() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": {
                    "placement": "right"
                }
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);

    let mut indexed_lengths: Vec<_> = polygons
        .iter()
        .enumerate()
        .map(|(index, points)| (index, bond_axis_length(points).expect("bond axis length")))
        .collect();
    indexed_lengths.sort_by(|(_, a), (_, b)| a.total_cmp(b));

    let (short_index, short_length) = indexed_lengths[0];
    let (long_index, long_length) = indexed_lengths[1];
    let short_axis = bond_axis_from_points(&polygons[short_index]).expect("short axis");
    let long_axis = bond_axis_from_points(&polygons[long_index]).expect("long axis");

    assert!((short_length - long_length).abs() < 0.05, "{short_length} {long_length}");
    assert!((short_axis.0.x - 20.0).abs() < 0.05 && (short_axis.1.x - 56.0).abs() < 0.05, "{short_axis:?}");
    assert!((long_axis.0.x - 20.0).abs() < 0.05 && (long_axis.1.x - 56.0).abs() < 0.05, "{long_axis:?}");
}

#[test]
fn render_document_shortens_non_terminal_side_double_by_offset_times_sqrt3_over_3() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": {
                    "placement": "right"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);

    let mut indexed_lengths: Vec<_> = polygons
        .iter()
        .enumerate()
        .map(|(index, points)| (index, bond_axis_length(points).expect("bond axis length")))
        .collect();
    indexed_lengths.sort_by(|(_, a), (_, b)| a.total_cmp(b));

    let (short_index, short_length) = indexed_lengths[0];
    let (long_index, long_length) = indexed_lengths[1];
    let short_axis = bond_axis_from_points(&polygons[short_index]).expect("short axis");
    let long_axis = bond_axis_from_points(&polygons[long_index]).expect("long axis");
    let short_mid_y = (short_axis.0.y + short_axis.1.y) * 0.5;
    let long_mid_y = (long_axis.0.y + long_axis.1.y) * 0.5;
    let offset_distance = (short_mid_y - long_mid_y).abs();
    let expected_short_length = long_length - offset_distance * (3.0f64).sqrt() / 3.0;

    assert!(
        (short_length - expected_short_length).abs() < 0.05,
        "short={short_length} expected={expected_short_length} long={long_length} offset={offset_distance}"
    );
}

#[test]
fn render_document_joins_center_double_only_on_occupied_side() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);

    let mut indexed_extensions: Vec<_> = polygons
        .iter()
        .enumerate()
        .map(|(index, polygon)| (index, polygon[1].x.max(polygon[2].x)))
        .collect();
    indexed_extensions.sort_by(|(_, a), (_, b)| a.total_cmp(b));
    let unchanged = &polygons[indexed_extensions[0].0];
    let extended = &polygons[indexed_extensions[1].0];

    assert!(
        (unchanged[1].x - 56.0).abs() < 0.001 && (unchanged[2].x - 56.0).abs() < 0.001,
        "{unchanged:?}"
    );
    assert!(
        (extended[1].x - 56.0).abs() > 0.05 || (extended[2].x - 56.0).abs() > 0.05,
        "{extended:?}"
    );
}

#[test]
fn render_document_keeps_center_double_original_for_straight_through_180_degrees() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [92.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);
    for polygon in polygons {
        assert!((polygon[1].x - 56.0).abs() < 0.001, "{polygon:?}");
        assert!((polygon[2].x - 56.0).abs() < 0.001, "{polygon:?}");
    }
}

#[test]
fn render_document_keeps_center_double_original_for_angles_over_162_degrees() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [91.45, 33.75], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);
    for polygon in polygons {
        assert!((polygon[1].x - 56.0).abs() < 0.05, "{polygon:?}");
        assert!((polygon[2].x - 56.0).abs() < 0.05, "{polygon:?}");
    }
}

#[test]
fn render_document_extends_center_double_lines_to_branch_bonds_and_branches_join_each_other() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [74.0, 68.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            },
            {
                "id": "b3",
                "begin": "n2",
                "end": "n4",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let polygons = object_bond_polygons_with_ids(&render_document(&document));
    let center_double: Vec<_> = polygons
        .iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points.clone())
        .collect();
    assert_eq!(center_double.len(), 2);
    let branch_up = polygons
        .iter()
        .find(|(bond_id, _)| bond_id == "b2")
        .map(|(_, points)| points.clone())
        .expect("upper branch polygon");
    let branch_down = polygons
        .iter()
        .find(|(bond_id, _)| bond_id == "b3")
        .map(|(_, points)| points.clone())
        .expect("lower branch polygon");

    assert!(
        center_double.iter().all(|polygon| {
            let end_points = [polygon[1], polygon[2]];
            end_points
                .iter()
                .all(|point| point_lies_on_polygon_boundary(*point, &branch_up, 1.0e-4))
                || end_points
                    .iter()
                    .all(|point| point_lies_on_polygon_boundary(*point, &branch_down, 1.0e-4))
        }),
        "{center_double:?} {branch_up:?} {branch_down:?}"
    );
    assert!(
        center_double.iter().any(|polygon| {
            [polygon[1], polygon[2]]
                .iter()
                .all(|point| point_lies_on_polygon_boundary(*point, &branch_up, 1.0e-4))
        })
    );
    assert!(
        center_double.iter().any(|polygon| {
            [polygon[1], polygon[2]]
                .iter()
                .all(|point| point_lies_on_polygon_boundary(*point, &branch_down, 1.0e-4))
        })
    );
    assert_eq!(shared_point_count(&branch_up, &branch_down, 1.0e-4), 2);
}

#[test]
fn render_document_joins_same_side_double_outer_polygons() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "left" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "left" }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let b1_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemcore_engine::Point::new(56.0, 40.0),
    );
    let b2_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b2",
        chemcore_engine::Point::new(56.0, 40.0),
    );
    let b1_main = side_double_main_polygon_for_bond(
        &polygons,
        "b1",
        chemcore_engine::Point::new(56.0, 40.0),
    );
    let b2_main = side_double_main_polygon_for_bond(
        &polygons,
        "b2",
        chemcore_engine::Point::new(56.0, 40.0),
    );

    assert_eq!(shared_point_count(&b1_outer, &b2_outer, 1.0e-4), 2);
    assert_eq!(shared_point_count(&b1_outer, &b1_main, 1.0e-4), 0);
    assert_eq!(shared_point_count(&b2_outer, &b2_main, 1.0e-4), 0);
}

#[test]
fn render_document_keeps_opposite_side_double_outer_polygons_inset() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "left" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let b1_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemcore_engine::Point::new(56.0, 40.0),
    );
    let b2_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b2",
        chemcore_engine::Point::new(56.0, 40.0),
    );

    assert_eq!(shared_point_count(&b1_outer, &b2_outer, 1.0e-4), 0);
}

#[test]
fn render_document_joins_side_double_outer_polygon_for_straight_through_180_degrees() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [92.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let b1_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemcore_engine::Point::new(56.0, 40.0),
    );
    let b2_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b2",
        chemcore_engine::Point::new(56.0, 40.0),
    );

    assert_eq!(shared_point_count(&b1_outer, &b2_outer, 1.0e-4), 2);
}

#[test]
fn render_document_joins_inner_side_double_outer_polygon_against_triple_outer_polygon() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 3,
                "strokeWidth": 0.85
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let b1_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemcore_engine::Point::new(56.0, 40.0),
    );
    let triple_shared = polygons
        .iter()
        .filter(|(bond_id, _)| bond_id == "b2")
        .any(|(_, points)| shared_point_count(&b1_outer, points, 1.0e-4) == 2);

    assert!(triple_shared, "{polygons:?}");
}

#[test]
fn render_document_retreats_side_double_outer_polygon_for_acute_single_bond_angles() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [30.0, 60.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let b1_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemcore_engine::Point::new(56.0, 40.0),
    );
    let single_shared = polygons
        .iter()
        .filter(|(bond_id, _)| bond_id == "b2")
        .any(|(_, points)| shared_point_count(&b1_outer, points, 1.0e-4) == 2);

    assert!(!single_shared, "{polygons:?}");
}

#[test]
fn render_document_retreats_side_double_outer_polygon_against_center_double_reference_axis() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [30.0, 60.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let side_double_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemcore_engine::Point::new(56.0, 40.0),
    );
    let centered_double: Vec<_> = polygons
        .iter()
        .filter(|(bond_id, _)| bond_id == "b2")
        .collect();

    assert!(centered_double.iter().all(|(_, points)| {
        shared_point_count(&side_double_outer, points, 1.0e-4) == 0
    }));
}

#[test]
fn render_document_scales_triple_offset_with_bond_length() {
    let short_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 3, "strokeWidth": 0.85 }
        ]),
    );
    let long_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 80.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [92.0, 80.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 3, "strokeWidth": 0.85 }
        ]),
    );

    let short_lines = object_bond_centerlines(&render_document(&short_document));
    let long_lines = object_bond_centerlines(&render_document(&long_document));

    let short_offset = short_lines
        .iter()
        .map(|(from, to)| ((from.y + to.y) / 2.0 - 40.0).abs())
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();
    let long_offset = long_lines
        .iter()
        .map(|(from, to)| ((from.y + to.y) / 2.0 - 80.0).abs())
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();

    assert!((long_offset - short_offset * 2.0).abs() < 0.05, "{short_offset} {long_offset}");
}

#[test]
fn render_document_keeps_solid_wedge_cap_width_constant_when_bond_is_longer() {
    let short_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "end"
                }
            }
        ]),
    );
    let long_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 80.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [92.0, 80.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "end"
                }
            }
        ]),
    );

    let short_polygon = render_document(&short_document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon { role, object_id, points, .. }
                if role == RenderRole::DocumentBond && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("short wedge polygon");
    let long_polygon = render_document(&long_document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon { role, object_id, points, .. }
                if role == RenderRole::DocumentBond && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("long wedge polygon");

    let short_width = ((short_polygon[1].x - short_polygon[2].x).powi(2)
        + (short_polygon[1].y - short_polygon[2].y).powi(2))
    .sqrt();
    let long_width = ((long_polygon[1].x - long_polygon[2].x).powi(2)
        + (long_polygon[1].y - long_polygon[2].y).powi(2))
    .sqrt();

    assert!((short_width - long_width).abs() < 0.05, "{short_width} {long_width}");
}

#[test]
fn render_document_emits_three_way_main_contact_patches() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 8.82], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [74.0, 71.18], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 },
            { "id": "b2", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n1", "end": "n4", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = centered_bond_polygons(&primitives, chemcore_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 3);
    assert_eq!(polygons.iter().filter(|points| points.len() == 5).count(), 3);
    assert!(polygons.iter().all(|points| polygon_area(points) > 0.01));
}

#[test]
fn render_document_emits_four_way_main_contact_patches() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [92.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [56.0, 4.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n5", "element": "C", "atomicNumber": 6, "position": [56.0, 76.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 },
            { "id": "b2", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n1", "end": "n4", "order": 1, "strokeWidth": 0.85 },
            { "id": "b4", "begin": "n1", "end": "n5", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = centered_bond_polygons(&primitives, chemcore_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 4);
    assert_eq!(polygons.iter().filter(|points| points.len() == 5).count(), 4);
    assert!(polygons.iter().all(|points| polygon_area(points) > 0.01));
}
