use chemsema_engine::{
    angular_distance, document_to_cdxml, document_to_svg, hit_test_bond_center,
    parse_cdxml_document, parse_document_json, render_document, render_primitives_bounds,
    ChemSemaDocument, Engine, Point, RenderPrimitive, RenderRole, ResourceData, Tool, ToolState,
};
use serde_json::json;
use serde_json::Map;
use std::collections::BTreeSet;

mod support;
use support::{cdxml_fixture_exists, read_cdxml_fixture, read_optional_cdxml_fixture};

const fn cdxml_cm_to_pt(value: f64) -> f64 {
    value * chemsema_engine::PT_PER_CM
}

const CDXML_EDIT_SCALE: f64 = 1.0;

fn assert_close(left: f64, right: f64) {
    assert!(
        (left - right).abs() < 1e-6,
        "expected {left:.6} to equal {right:.6}"
    );
}

fn assert_point_close(left: Point, right: Point) {
    assert_close(left.x, right.x);
    assert_close(left.y, right.y);
}

fn round_to_2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn select_tool_state() -> ToolState {
    ToolState {
        active_tool: Tool::Select,
        ..ToolState::default()
    }
}

fn object_is_bracket_group(object: &chemsema_engine::SceneObject) -> bool {
    object.object_type == "group"
        && object.meta.get("kind").and_then(|value| value.as_str()) == Some("bracket-group")
}

fn fragment_document(nodes: serde_json::Value, bonds: serde_json::Value) -> ChemSemaDocument {
    serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
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
            "payload": { "resourceRef": "mol_001", "bbox": [0.0, 0.0, 80.0, 40.0] }
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 120.0, 120.0],
                    "nodes": nodes,
                    "bonds": bonds
                }
            }
        }
    }))
    .expect("document should deserialize")
}

fn fragment_document_preserving_disconnected_components(
    nodes: serde_json::Value,
    bonds: serde_json::Value,
) -> ChemSemaDocument {
    let mut document = fragment_document(nodes, bonds);
    document.objects[0].meta = json!({ "preserveDisconnectedComponents": true });
    document
}

fn grouped_two_fragment_document() -> ChemSemaDocument {
    serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 300.0, "height": 200.0, "background": "#ffffff" }
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
            "id": "obj_molecule_first",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_first", "bbox": [0.0, 0.0, 40.0, 0.0] }
        }, {
            "id": "obj_group",
            "type": "group",
            "visible": true,
            "zIndex": 20,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "payload": { "bbox": [100.0, 20.0, 40.0, 0.0] },
            "children": [{
                "id": "obj_molecule_grouped",
                "type": "molecule",
                "visible": true,
                "zIndex": 30,
                "transform": { "translate": [100.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_molecule_default",
                "payload": { "resourceRef": "mol_grouped", "bbox": [0.0, 0.0, 40.0, 0.0] }
            }]
        }],
        "resources": {
            "mol_first": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 40.0, 0.0],
                    "nodes": [
                        { "id": "n_first_1", "element": "C", "atomicNumber": 6, "position": [0.0, 0.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n_first_2", "element": "C", "atomicNumber": 6, "position": [40.0, 0.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [{ "id": "b_first", "begin": "n_first_1", "end": "n_first_2", "order": 1 }]
                }
            },
            "mol_grouped": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 40.0, 0.0],
                    "nodes": [
                        { "id": "n_grouped_1", "element": "C", "atomicNumber": 6, "position": [0.0, 0.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n_grouped_2", "element": "C", "atomicNumber": 6, "position": [40.0, 0.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [{ "id": "b_grouped", "begin": "n_grouped_1", "end": "n_grouped_2", "order": 1 }]
                }
            }
        }
    }))
    .expect("document should deserialize")
}

fn grouped_labeled_molecule_document() -> ChemSemaDocument {
    serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_group_labeled_molecule",
            "title": "group labeled molecule",
            "page": { "width": 220.0, "height": 160.0, "background": "#ffffff" }
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
            "id": "grp_1",
            "type": "group",
            "zIndex": 30,
            "children": [{
                "id": "obj_molecule_001",
                "type": "molecule",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [10.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_molecule_default",
                "payload": { "resourceRef": "mol_001", "bbox": [0.0, 0.0, 80.0, 40.0] }
            }]
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 80.0, 40.0],
                    "nodes": [
                        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [10.0, 20.0], "charge": 0, "numHydrogens": 0 },
                        {
                            "id": "n2",
                            "element": "O",
                            "atomicNumber": 8,
                            "position": [60.0, 20.0],
                            "charge": 0,
                            "numHydrogens": 0,
                            "label": {
                                "text": "O",
                                "sourceText": "O",
                                "position": [60.0, 20.0],
                                "box": [56.0, 12.0, 66.0, 24.0],
                                "fontSize": 11.0
                            }
                        }
                    ],
                    "bonds": [
                        { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
                    ]
                }
            }
        }
    }))
    .expect("document should deserialize")
}

#[test]
fn hit_testing_checks_grouped_molecule_fragments() {
    let document = grouped_two_fragment_document();
    assert_eq!(document.editable_fragments().len(), 2);
    let first_hit = hit_test_bond_center(&document, Point::new(20.0, 0.0), 5.0)
        .expect("first molecule bond should be hoverable");
    assert_eq!(first_hit.bond_id, "b_first");
    let hit = hit_test_bond_center(&document, Point::new(120.0, 20.0), 5.0)
        .expect("grouped molecule bond should be hoverable");
    assert_eq!(hit.bond_id, "b_grouped");
}

fn primitive_polygon_bounds(points: &[Point]) -> [f64; 4] {
    points.iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |mut bounds, point| {
            bounds[0] = bounds[0].min(point.x);
            bounds[1] = bounds[1].min(point.y);
            bounds[2] = bounds[2].max(point.x);
            bounds[3] = bounds[3].max(point.y);
            bounds
        },
    )
}

fn comma_point_token(token: &str) -> Option<Point> {
    let token = token.trim_end_matches(|ch: char| ch == ',' || ch == ';');
    let (x, y) = token.split_once(',')?;
    Some(Point::new(x.parse().ok()?, y.parse().ok()?))
}

fn horizontal_path_span_at_y(d: &str, y: f64) -> Option<f64> {
    let xs = d
        .split_whitespace()
        .filter_map(comma_point_token)
        .filter(|point| (point.y - y).abs() < 0.01)
        .map(|point| point.x)
        .collect::<Vec<_>>();
    let min = xs.iter().copied().reduce(f64::min)?;
    let max = xs.iter().copied().reduce(f64::max)?;
    Some(max - min)
}

fn render_primitive_object_id(primitive: &RenderPrimitive) -> Option<&str> {
    match primitive {
        RenderPrimitive::Line { object_id, .. }
        | RenderPrimitive::Circle { object_id, .. }
        | RenderPrimitive::Polygon { object_id, .. }
        | RenderPrimitive::Rect { object_id, .. }
        | RenderPrimitive::Ellipse { object_id, .. }
        | RenderPrimitive::Polyline { object_id, .. }
        | RenderPrimitive::Path { object_id, .. }
        | RenderPrimitive::FilledPath { object_id, .. }
        | RenderPrimitive::Text { object_id, .. } => object_id.as_deref(),
    }
}

fn render_primitive_bond_id(primitive: &RenderPrimitive) -> Option<&str> {
    match primitive {
        RenderPrimitive::Line { bond_id, .. }
        | RenderPrimitive::Polygon { bond_id, .. }
        | RenderPrimitive::Polyline { bond_id, .. }
        | RenderPrimitive::Path { bond_id, .. }
        | RenderPrimitive::FilledPath { bond_id, .. } => bond_id.as_deref(),
        _ => None,
    }
}

fn render_primitive_text_content(primitive: &RenderPrimitive) -> Option<String> {
    match primitive {
        RenderPrimitive::Text { text, runs, .. } => {
            if runs.is_empty() {
                Some(text.clone())
            } else {
                Some(runs.iter().map(|run| run.text.as_str()).collect())
            }
        }
        _ => None,
    }
}

fn rects_with_role(engine: &Engine, role_filter: RenderRole) -> Vec<[f64; 4]> {
    engine
        .render_list()
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Rect {
                role,
                x,
                y,
                width,
                height,
                ..
            } if role == role_filter => Some([x, y, x + width, y + height]),
            _ => None,
        })
        .collect()
}

#[test]
fn render_document_adds_margin_knockout_for_later_crossing_bond() {
    let document = fragment_document_preserving_disconnected_components(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 60.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [100.0, 60.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [60.0, 20.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [60.0, 100.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b_under", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 1.0, "marginWidth": 2.0 },
            { "id": "b_over", "begin": "n3", "end": "n4", "order": 1, "strokeWidth": 1.0, "marginWidth": 2.0 }
        ]),
    );

    let primitives = render_document(&document);
    let knockout_index = primitives
        .iter()
        .position(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role: RenderRole::DocumentKnockout,
                    ..
                }
            )
        })
        .expect("crossing over-bond should insert a white margin knockout");
    let under_index = primitives
        .iter()
        .position(|primitive| matches!(primitive, RenderPrimitive::Polygon { role: RenderRole::DocumentBond, bond_id, .. } if bond_id.as_deref() == Some("b_under")))
        .expect("under bond should render");
    let over_index = primitives
        .iter()
        .position(|primitive| matches!(primitive, RenderPrimitive::Polygon { role: RenderRole::DocumentBond, bond_id, .. } if bond_id.as_deref() == Some("b_over")))
        .expect("over bond should render");
    assert!(under_index < knockout_index && knockout_index < over_index);

    let RenderPrimitive::Polygon { points, .. } = &primitives[knockout_index] else {
        unreachable!("knockout is a polygon");
    };
    let bounds = primitive_polygon_bounds(points);
    assert!((bounds[0] - 57.5).abs() < 0.001, "{bounds:?}");
    assert!((bounds[1] - 59.45).abs() < 0.001, "{bounds:?}");
    assert!((bounds[2] - 62.5).abs() < 0.001, "{bounds:?}");
    assert!((bounds[3] - 60.55).abs() < 0.001, "{bounds:?}");
}

#[test]
fn render_document_adds_wavy_margin_knockout_across_molecule_objects() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 140.0, "height": 120.0, "background": "#ffffff" }
        },
        "style": {
            "defaults": {
                "lineWidth": 0.85,
                "marginWidth": 2.0
            }
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
            "id": "obj_under",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_under", "bbox": [0.0, 0.0, 120.0, 80.0] }
        }, {
            "id": "obj_wavy",
            "type": "molecule",
            "visible": true,
            "zIndex": 20,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_wavy", "bbox": [0.0, 0.0, 120.0, 80.0] }
        }],
        "resources": {
            "mol_under": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 120.0, 80.0],
                    "nodes": [
                        { "id": "u1", "element": "C", "atomicNumber": 6, "position": [20.0, 60.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "u2", "element": "C", "atomicNumber": 6, "position": [100.0, 60.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b_under", "begin": "u1", "end": "u2", "order": 1, "strokeWidth": 0.85 }
                    ]
                }
            },
            "mol_wavy": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 120.0, 80.0],
                    "nodes": [
                        { "id": "w1", "element": "C", "atomicNumber": 6, "position": [60.0, 35.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "w2", "element": "C", "atomicNumber": 6, "position": [60.0, 85.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        {
                            "id": "b_wavy",
                            "begin": "w1",
                            "end": "w2",
                            "order": 1,
                            "strokeWidth": 0.85,
                            "lineStyles": { "main": "wavy" }
                        }
                    ]
                }
            }
        }
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let under_index = primitives
        .iter()
        .position(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role: RenderRole::DocumentBond,
                    bond_id,
                    ..
                } if bond_id.as_deref() == Some("b_under")
            )
        })
        .expect("under bond should render");
    let knockout_index = primitives
        .iter()
        .position(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role: RenderRole::DocumentKnockout,
                    bond_id,
                    ..
                } if bond_id.as_deref() == Some("b_wavy")
            )
        })
        .expect("wavy over-bond should insert a local crossing knockout");
    let wavy_index = primitives
        .iter()
        .position(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Path {
                    role: RenderRole::DocumentBond,
                    bond_id,
                    ..
                } if bond_id.as_deref() == Some("b_wavy")
            )
        })
        .expect("wavy bond should render");

    assert!(under_index < knockout_index && knockout_index < wavy_index);
    let RenderPrimitive::Polygon { points, .. } = &primitives[knockout_index] else {
        unreachable!("local wavy crossing knockout is a polygon");
    };
    let bounds = primitive_polygon_bounds(points);
    assert!(bounds[2] - bounds[0] < 12.0, "{bounds:?}");
    assert!(bounds[3] - bounds[1] < 2.0, "{bounds:?}");
}

#[test]
fn cdxml_crossing_knockouts_match_chemdraw_style_envelopes() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 320 110" LineWidth="0.60" BoldWidth="2.0"
 BondLength="35" BondSpacing="18" MarginWidth="1.60">
 <page id="1" BoundingBox="0 0 320 110">
  <fragment id="11"><n id="100" p="10 65"/><n id="101" p="70 65"/><n id="102" p="40 35"/><n id="103" p="40 95"/>
   <b id="110" Z="1" B="100" E="101" CrossingBonds="111"/><b id="111" Z="2" B="102" E="103" Order="2" DoublePosition="Center" CrossingBonds="110"/></fragment>
  <fragment id="21"><n id="120" p="90 65"/><n id="121" p="150 65"/><n id="122" p="120 35"/><n id="123" p="120 95"/>
   <b id="130" Z="1" B="120" E="121" CrossingBonds="131"/><b id="131" Z="2" B="122" E="123" Order="2" DoublePosition="Left" CrossingBonds="130"/></fragment>
  <fragment id="31"><n id="140" p="170 65"/><n id="141" p="230 65"/><n id="142" p="200 35"/><n id="143" p="200 95"/>
   <b id="150" Z="1" B="140" E="141" CrossingBonds="151"/><b id="151" Z="2" B="142" E="143" Display="WedgeBegin" CrossingBonds="150"/></fragment>
  <fragment id="41"><n id="160" p="250 65"/><n id="161" p="310 65"/><n id="162" p="280 35"/><n id="163" p="280 95"/>
   <b id="170" Z="1" B="160" E="161" CrossingBonds="171"/><b id="171" Z="2" B="162" E="163" Display="Wavy" CrossingBonds="170"/></fragment>
 </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("crossing style envelope"))
        .expect("crossing style matrix should parse");
    let mut bounds = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentKnockout,
                points,
                ..
            } => Some(primitive_polygon_bounds(&points)),
            _ => None,
        })
        .collect::<Vec<_>>();
    bounds.sort_by(|left, right| left[0].total_cmp(&right[0]));

    assert_eq!(bounds.len(), 4, "expected one local patch per crossing");
    let expected_x = [(33.0, 47.0), (107.6, 121.6), (197.5, 202.5), (277.4, 282.6)];
    for (bounds, (expected_min, expected_max)) in bounds.iter().zip(expected_x) {
        assert!((bounds[0] - expected_min).abs() < 0.001, "{bounds:?}");
        assert!((bounds[2] - expected_max).abs() < 0.001, "{bounds:?}");
        assert!((bounds[1] - 64.65).abs() < 0.001, "{bounds:?}");
        assert!((bounds[3] - 65.35).abs() < 0.001, "{bounds:?}");
    }
}

#[test]
fn cdxml_near_endpoint_crossings_use_finite_margin_caps() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 260 110" LineWidth="0.60" BoldWidth="2.0"
 BondLength="14.40" BondSpacing="18" MarginWidth="1.60">
 <page id="1" BoundingBox="0 0 260 110">
  <!-- Infinite lines meet 1.17 pt beyond the lower bond's end. -->
  <fragment id="11">
   <n id="100" p="43.15 43.94"/><n id="101" p="20.33 50.27"/>
   <n id="102" p="10.74 43.32"/><n id="103" p="30.30 60.10"/>
   <b id="110" Z="1" B="100" E="101"/><b id="111" Z="2" B="102" E="103"/>
  </fragment>
  <!-- Infinite lines meet 2.40 pt before the upper bond's begin cap. -->
  <fragment id="21">
   <n id="120" p="90.04 41.50"/><n id="121" p="93.21 72.94"/>
   <n id="122" p="94.35 63.04"/><n id="123" p="113.55 47.01"/>
   <b id="130" Z="1" B="120" E="121"/><b id="131" Z="2" B="122" E="123"/>
  </fragment>
  <!-- Same topology, but farther than the finite upper margin cap. -->
  <fragment id="31">
   <n id="140" p="170 25"/><n id="141" p="170 85"/>
   <n id="142" p="174 60"/><n id="143" p="200 30"/>
   <b id="150" Z="1" B="140" E="141"/><b id="151" Z="2" B="142" E="143"/>
  </fragment>
 </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("near endpoint crossing caps"))
        .expect("near-endpoint crossing matrix should parse");
    let knockouts = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentKnockout,
                bond_id,
                points,
                ..
            } => Some((bond_id, points)),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(knockouts.len(), 2, "{knockouts:?}");
    assert_eq!(knockouts[0].0.as_deref(), Some("111"));
    assert_eq!(knockouts[1].0.as_deref(), Some("131"));
    for (points, expected) in [
        (
            &knockouts[0].1,
            [20.236447, 49.695888, 21.730173, 50.607265],
        ),
        (
            &knockouts[1].1,
            [91.904096, 62.132698, 92.604508, 63.445850],
        ),
    ] {
        let bounds = primitive_polygon_bounds(points);
        for (actual, expected) in bounds.into_iter().zip(expected) {
            assert!((actual - expected).abs() < 1.0e-5, "{bounds:?}");
        }
    }
    assert!(
        knockouts
            .iter()
            .all(|(_, points)| polygon_area(points).abs() > 1.0e-4),
        "near-endpoint contacts must produce real finite overlap polygons: {knockouts:?}"
    );

    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load for target rendering");
    let target_primitives = engine.render_targets(
        &BTreeSet::new(),
        &BTreeSet::from(["110".to_string()]),
        &BTreeSet::new(),
    );
    assert!(
        target_primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentBond,
                bond_id,
                ..
            } if bond_id.as_deref() == Some("111")
        )),
        "near-endpoint upper bond must be an incremental-render dependency: {target_primitives:?}"
    );
    assert!(
        target_primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentKnockout,
                bond_id,
                ..
            } if bond_id.as_deref() == Some("111")
        )),
        "incremental rendering must retain the near-endpoint knockout: {target_primitives:?}"
    );
}

#[test]
fn explicit_crossing_bonds_are_authoritative_over_geometric_fallback() {
    let document = fragment_document_preserving_disconnected_components(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 60.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [100.0, 60.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [60.0, 20.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [60.0, 100.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b_under", "begin": "n1", "end": "n2", "order": 1,
                "strokeWidth": 1.0, "marginWidth": 2.0,
                "meta": { "import": { "cdxml": { "crossingBonds": [] } } }
            },
            { "id": "b_over", "begin": "n3", "end": "n4", "order": 1, "strokeWidth": 1.0, "marginWidth": 2.0 }
        ]),
    );

    assert!(
        !render_document(&document).iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentKnockout,
                ..
            }
        )),
        "an explicit empty crossing list must suppress geometric inference"
    );
}

#[test]
fn explicit_crossing_bond_ids_are_global_across_fragments() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 120 120" LineWidth="1" MarginWidth="2">
  <page id="1" BoundingBox="0 0 120 120">
    <fragment id="2">
      <n id="10" p="20 60"/><n id="11" p="100 60"/>
      <b id="20" Z="1" B="10" E="11" CrossingBonds="31"/>
    </fragment>
    <fragment id="3">
      <n id="30" p="60 20"/><n id="32" p="60 100"/>
      <b id="31" Z="2" B="30" E="32" CrossingBonds="20"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("cross-fragment crossings"))
        .expect("cross-fragment CDXML should parse");
    let primitives = render_document(&document);
    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentKnockout,
                bond_id,
                ..
            } if bond_id.as_deref() == Some("31")
        )),
        "explicit crossing IDs must resolve in document scope: {primitives:?}"
    );
}

#[test]
fn cdxml_crossing_bonds_round_trip_with_remapped_object_ids() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 120 120" LineWidth="0.6" MarginWidth="1.6">
  <page id="1" BoundingBox="0 0 120 120">
    <fragment id="2">
      <n id="10" p="20 60"/><n id="11" p="100 60"/>
      <n id="12" p="60 20"/><n id="13" p="60 100"/>
      <b id="20" Z="7" B="10" E="11" CrossingBonds="21"/>
      <b id="21" Z="8" B="12" E="13" CrossingBonds="20"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("crossings")).expect("CDXML should parse");
    let exported = document_to_cdxml(&document);
    assert!(exported.contains("CrossingBonds=\""), "{exported}");
    assert!(exported.contains("Z=\"7\""), "{exported}");
    assert!(exported.contains("Z=\"8\""), "{exported}");

    let reopened = parse_cdxml_document(&exported, Some("crossings reopened"))
        .expect("exported CDXML should parse");
    let bonds: Vec<_> = reopened
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .flat_map(|fragment| fragment.bonds.iter())
        .collect();
    assert_eq!(bonds.len(), 2);
    for (index, bond) in bonds.iter().enumerate() {
        let other = bonds[1 - index];
        let crossings = bond
            .meta
            .pointer("/import/cdxml/crossingBonds")
            .and_then(serde_json::Value::as_array)
            .expect("crossing list should survive");
        assert_eq!(crossings, &vec![json!(other.id)]);
    }
}

#[test]
fn coordinate_free_cdxml_chain_gets_deterministic_topology_layout() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML CreationProgram="CDXMLWriter"><page id="10"><fragment id="11">
  <n id="1" Element="6"/><n id="2" Element="6"/><n id="3" Element="6"/><n id="4" Element="6"/>
  <b B="1" E="2" id="5"/><b B="2" E="3" id="6"/><b B="3" E="4" id="7"/>
</fragment></page></CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("coordinate-free chain"))
        .expect("topology-only CDXML should import");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("coordinate-free fragment should survive");

    assert_eq!(fragment.nodes.len(), 4);
    assert_eq!(fragment.bonds.len(), 3);
    assert_eq!(fragment.nodes[0].position, [0.0, 0.0]);
    assert_eq!(fragment.nodes[1].position, [25.98, 15.0]);
    assert_eq!(fragment.nodes[2].position, [51.96, 0.0]);
    assert_eq!(fragment.nodes[3].position, [77.94, 15.0]);
    assert_eq!(
        render_document(&document)
            .iter()
            .filter(|primitive| render_primitive_bond_id(primitive).is_some())
            .count(),
        3
    );
}

#[test]
fn coordinate_free_cdxml_aromatic_ring_and_missing_bond_ids_remain_visible() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML CreationProgram="CDXMLWriter"><page><fragment>
  <n id="1"/><n id="2"/><n id="3"/><n id="4"/><n id="5"/><n id="6"/>
  <b B="1" E="2" Order="1.5" Display="Dash"/><b B="2" E="3" Order="1.5" Display="Dash"/>
  <b B="3" E="4" Order="1.5" Display="Dash"/><b B="4" E="5" Order="1.5" Display="Dash"/>
  <b B="5" E="6" Order="1.5" Display="Dash"/><b B="6" E="1" Order="1.5" Display="Dash"/>
</fragment></page></CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("coordinate-free aromatic ring"))
        .expect("topology-only aromatic ring should import");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("coordinate-free ring should survive");
    let ids = fragment
        .bonds
        .iter()
        .map(|bond| bond.id.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(fragment.nodes.len(), 6);
    assert_eq!(fragment.bonds.len(), 6);
    assert_eq!(ids.len(), 6);
    assert!(ids.iter().all(|id| id.starts_with("cdxml_bond_")));
    assert!(fragment.bonds.iter().all(|bond| {
        bond.order == 1
            && bond.line_styles.main == chemsema_engine::BondLinePattern::Solid
            && bond
                .meta
                .pointer("/import/cdxml/aromatic")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
    }));
    assert_eq!(
        render_document(&document)
            .iter()
            .filter(|primitive| render_primitive_bond_id(primitive).is_some())
            .count(),
        6
    );

    let exported = document_to_cdxml(&document);
    assert_eq!(exported.matches("Order=\"1.5\"").count(), 6, "{exported}");
    assert_eq!(
        exported.matches("Display=\"Dash\"").count(),
        6,
        "{exported}"
    );
}

#[test]
fn coordinate_free_cdxml_dative_chain_keeps_donor_hydrogen_and_arrowhead() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML CreationProgram="CDXMLWriter"><page><fragment>
  <n id="1" Element="6"/><n id="2" Element="8"/><n id="3" Element="6"/><n id="4" Element="6"/>
  <b id="5" B="1" E="2"/><b id="6" B="2" E="3" Order="dative"/><b id="7" B="3" E="4"/>
</fragment></page></CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("coordinate-free dative chain"))
        .expect("topology-only dative chain should import");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("coordinate-free dative chain should survive");
    let oxygen = fragment
        .nodes
        .iter()
        .find(|node| node.id == "2")
        .expect("oxygen donor should survive");

    assert_eq!(oxygen.num_hydrogens, 1);
    assert!(oxygen
        .label
        .as_ref()
        .is_some_and(|label| label.text.contains('H')));
    assert_eq!(
        render_document(&document)
            .iter()
            .filter(|primitive| render_primitive_bond_id(primitive) == Some("6"))
            .count(),
        2,
        "dative bond should render a shaft and one solid arrowhead"
    );
    assert!(document_to_cdxml(&document).contains("Order=\"dative\""));
}

#[test]
fn cdxml_restrict_implicit_hydrogens_renders_an_independent_atom_query_marker() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemDraw 6.0.1" LabelFont="3" LabelSize="10" LabelFace="96"
       BondLength="30" LineWidth="1" BoldWidth="4" MarginWidth="2">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page><fragment>
    <n id="22" p="131.35 270"/><n id="23" p="131.35 300"/>
    <n id="24" p="159.89 309.27"/><n id="25" p="177.52 285"/>
    <n id="26" p="159.89 260.73" NumHydrogens="1" Charge="-1"
       ImplicitHydrogens="yes">
      <t id="33" p="157.07 254.19" BoundingBox="158 247 163 265"
         LabelAlignment="Above" LineStarts="2 3">
        <s font="3" size="10" face="96">CH</s>
      </t>
    </n>
    <b id="27" B="22" E="23" Order="2"/><b id="28" B="23" E="24"/>
    <b id="29" B="24" E="25" Order="2"/><b id="30" B="25" E="26"/>
    <b id="31" B="26" E="22"/>
  </fragment></page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("legacy restricted hydrogen"))
        .expect("legacy CDXML should import");
    let carbon = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "26"))
        .expect("labeled carbon should survive");
    let label = carbon.label.as_ref().expect("carbon label should survive");

    assert_eq!(carbon.num_hydrogens, 1);
    assert_eq!(label.source_text.as_deref(), Some("CH"));
    assert_eq!(label.lines, vec!["H", "C"]);
    assert_eq!(label.text.matches('H').count(), 1);

    let node_h_positions = |document: &ChemSemaDocument| {
        render_document(document)
            .iter()
            .filter_map(|primitive| match primitive {
                RenderPrimitive::Text {
                    node_id: Some(node_id),
                    x,
                    runs,
                    ..
                } if node_id == "26" && runs.iter().any(|run| run.text.trim() == "H") => Some(*x),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let h_positions = node_h_positions(&document);
    assert_eq!(h_positions.len(), 2);
    assert!(
        h_positions.iter().any(|x| *x > 159.89),
        "the query H should sit independently to the upper right of the atom label: {h_positions:?}"
    );

    let without_num_hydrogens = parse_cdxml_document(
        &cdxml.replace(" NumHydrogens=\"1\"", ""),
        Some("query marker without NumHydrogens"),
    )
    .expect("query marker should not depend on NumHydrogens");
    assert_eq!(node_h_positions(&without_num_hydrogens).len(), 2);

    let hidden_query_marker = parse_cdxml_document(
        &cdxml.replace(
            "<CDXML CreationProgram=",
            "<CDXML ShowAtomQuery=\"no\" CreationProgram=",
        ),
        Some("hidden atom query marker"),
    )
    .expect("ShowAtomQuery=no should import");
    assert_eq!(
        node_h_positions(&hidden_query_marker).len(),
        1,
        "only the H authored inside CH should remain"
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("NumHydrogens=\"1\""), "{exported}");
    assert!(exported.contains("ImplicitHydrogens=\"yes\""), "{exported}");
    assert!(exported.contains(">CH</s>"), "{exported}");
}

#[test]
fn render_targets_for_under_crossing_bond_include_over_bond_dependency() {
    let document = fragment_document_preserving_disconnected_components(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 60.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [100.0, 60.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [60.0, 20.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [60.0, 100.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b_under", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 1.0, "marginWidth": 2.0 },
            { "id": "b_over", "begin": "n3", "end": "n4", "order": 1, "strokeWidth": 1.0, "marginWidth": 2.0 }
        ]),
    );
    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");

    let primitives = engine.render_targets(
        &BTreeSet::new(),
        &BTreeSet::from(["b_under".to_string()]),
        &BTreeSet::new(),
    );

    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role: RenderRole::DocumentBond,
                    bond_id,
                    ..
                } if bond_id.as_deref() == Some("b_over")
            )
        }),
        "targeting the lower crossing bond should also return the upper bond for desktop patching: {primitives:?}"
    );
    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role: RenderRole::DocumentKnockout,
                    bond_id,
                    ..
                } if bond_id.as_deref() == Some("b_over")
            )
        }),
        "upper-bond knockout depends on the lower crossing bond: {primitives:?}"
    );
}

#[test]
fn render_document_does_not_add_margin_knockout_for_shared_endpoint_bonds() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 60.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [60.0, 60.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [100.0, 60.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b_left", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 1.0, "marginWidth": 2.0 },
            { "id": "b_right", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 1.0, "marginWidth": 2.0 }
        ]),
    );

    let primitives = render_document(&document);
    assert!(
        !primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentKnockout,
                ..
            }
        )),
        "endpoint contact should stay in the existing contact kernel, not use crossing margin"
    );
}

#[test]
fn cdxml_group_import_preserves_tree_and_z_order() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="20" LineWidth="1" LabelSize="10" CaptionSize="10">
  <page id="1" BoundingBox="0 0 200 160" Width="200" Height="160">
    <group id="10" BoundingBox="10 10 80 50" Z="77">
      <graphic id="11" GraphicType="Rectangle" RectangleType="Plain" BoundingBox="10 10 40 30" Z="12"/>
      <graphic id="12" GraphicType="Rectangle" RectangleType="Plain" BoundingBox="50 20 80 50" Z="13"/>
    </group>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("group")).expect("cdxml should parse");
    let group = document
        .objects
        .iter()
        .find(|object| object.object_type == "group")
        .expect("group should import");
    assert_eq!(group.z_index, 77);
    assert_eq!(group.children.len(), 2);
    assert!(document
        .objects
        .iter()
        .all(|object| object.object_type != "shape"));

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("<group "));
    assert!(exported.contains("Z=\"77\""));
    let reimported =
        parse_cdxml_document(&exported, Some("group export")).expect("group export should parse");
    assert_eq!(
        reimported
            .objects
            .iter()
            .find(|object| object.object_type == "group")
            .map(|object| object.children.len()),
        Some(2)
    );
}

#[test]
fn grouped_scene_object_child_click_selects_child_not_group() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_group",
            "title": "group",
            "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape": {
                "kind": "shape",
                "stroke": "#000000",
                "strokeWidth": 1.0
            }
        },
        "objects": [{
            "id": "grp_1",
            "type": "group",
            "zIndex": 30,
            "children": [
                {
                    "id": "shape_a",
                    "type": "shape",
                    "zIndex": 10,
                    "transform": { "translate": [10.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "styleRef": "style_shape",
                    "payload": { "bbox": [0.0, 0.0, 20.0, 10.0], "kind": "rect" }
                },
                {
                    "id": "shape_b",
                    "type": "shape",
                    "zIndex": 20,
                    "transform": { "translate": [50.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "styleRef": "style_shape",
                    "payload": { "bbox": [0.0, 0.0, 30.0, 10.0], "kind": "rect" }
                }
            ]
        }],
        "resources": {}
    }))
    .expect("document should deserialize");
    assert_eq!(render_document(&document).len(), 2);

    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");
    engine.select_at_point(Point::new(20.0, 15.0), false);
    assert_eq!(engine.state().selection.arrow_objects, vec!["shape_a"]);
    let hit: serde_json::Value =
        serde_json::from_str(&engine.context_hit_test_json(Point::new(20.0, 15.0))).unwrap();
    assert_eq!(hit["objectId"], "shape_a");
    assert_eq!(hit["objectType"], "shape");
    let selection_boxes: Vec<_> = engine
        .render_list()
        .into_iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Rect {
                    role: RenderRole::SelectionBox,
                    ..
                }
            )
        })
        .collect();
    assert_eq!(selection_boxes.len(), 1);
    match &selection_boxes[0] {
        RenderPrimitive::Rect {
            x,
            y,
            width,
            height,
            ..
        } => {
            assert!((*x - 9.5).abs() < 0.1);
            assert!((*y - 9.5).abs() < 0.1);
            assert!((*width - 21.0).abs() < 0.2);
            assert!((*height - 11.0).abs() < 0.2);
        }
        _ => unreachable!(),
    }

    assert!(engine.select_component_at_point(Point::new(20.0, 15.0), false));
    assert_eq!(engine.state().selection.arrow_objects, vec!["grp_1"]);
}

#[test]
fn region_selection_collapses_group_box_only_when_all_children_are_selected() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_group_region",
            "title": "group region",
            "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape": {
                "kind": "shape",
                "stroke": "#000000",
                "strokeWidth": 1.0
            }
        },
        "objects": [{
            "id": "grp_1",
            "type": "group",
            "zIndex": 30,
            "children": [
                {
                    "id": "shape_a",
                    "type": "shape",
                    "zIndex": 10,
                    "transform": { "translate": [10.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "styleRef": "style_shape",
                    "payload": { "bbox": [0.0, 0.0, 20.0, 10.0], "kind": "rect" }
                },
                {
                    "id": "shape_b",
                    "type": "shape",
                    "zIndex": 20,
                    "transform": { "translate": [50.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "styleRef": "style_shape",
                    "payload": { "bbox": [0.0, 0.0, 30.0, 10.0], "kind": "rect" }
                }
            ]
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");
    engine.select_in_rect(Point::new(5.0, 5.0), Point::new(35.0, 25.0), false);
    let partial_boxes = rects_with_role(&engine, RenderRole::SelectionBox);
    assert_eq!(partial_boxes.len(), 1);
    assert!(
        partial_boxes[0][2] - partial_boxes[0][0] < 30.0,
        "partial group selection should show only the child box, got {partial_boxes:?}"
    );

    engine.select_in_rect(Point::new(0.0, 0.0), Point::new(90.0, 60.0), false);
    let complete_boxes = rects_with_role(&engine, RenderRole::SelectionBox);
    assert_eq!(complete_boxes.len(), 1);
    assert!(
        complete_boxes[0][2] - complete_boxes[0][0] > 60.0,
        "complete group selection should collapse to the group box, got {complete_boxes:?}"
    );
}

#[test]
fn region_selecting_grouped_molecule_moves_nodes_not_parent_group() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_group_region_molecule",
            "title": "group region molecule",
            "page": { "width": 220.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": 0.85,
                "fontFamily": "Arial",
                "fontSize": 11.0
            },
            "style_bracket": {
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": 1.0
            }
        },
        "objects": [{
            "id": "grp_1",
            "type": "group",
            "zIndex": 30,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "children": [
                {
                    "id": "obj_molecule_001",
                    "type": "molecule",
                    "visible": true,
                    "zIndex": 10,
                    "transform": { "translate": [10.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "styleRef": "style_molecule_default",
                    "payload": { "resourceRef": "mol_001", "bbox": [0.0, 0.0, 80.0, 40.0] }
                },
                {
                    "id": "bracket_1",
                    "type": "bracket",
                    "visible": true,
                    "zIndex": 20,
                    "transform": { "translate": [120.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "styleRef": "style_bracket",
                    "payload": { "bbox": [0.0, 0.0, 30.0, 60.0], "kind": "square" }
                }
            ]
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 80.0, 40.0],
                    "nodes": [
                        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [10.0, 20.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n2", "element": "C", "atomicNumber": 6, "position": [60.0, 20.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
                    ]
                }
            }
        }
    }))
    .expect("document should deserialize");

    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");

    engine.select_in_rect(Point::new(0.0, 0.0), Point::new(90.0, 60.0), false);
    assert!(
        !engine
            .state()
            .selection
            .arrow_objects
            .contains(&"grp_1".to_string()),
        "region selection must not directly select the parent group: {:?}",
        engine.state().selection
    );
    assert_eq!(engine.state().selection.bonds, vec!["b1"]);
    assert!(engine.state().selection.nodes.contains(&"n1".to_string()));
    assert!(engine.state().selection.nodes.contains(&"n2".to_string()));

    assert!(engine.begin_selection_move_at_point(Point::new(30.0, 30.0), false, false));
    assert!(engine.update_selection_move(Point::new(40.0, 30.0), false));
    assert!(engine.finish_selection_move(Point::new(40.0, 30.0), false));

    let group = engine
        .state()
        .document
        .find_scene_object("grp_1")
        .expect("group should remain");
    assert_eq!(group.transform.translate, [0.0, 0.0]);
    let bracket = engine
        .state()
        .document
        .find_scene_object("bracket_1")
        .expect("bracket should remain");
    assert_eq!(bracket.transform.translate, [120.0, 20.0]);
    let fragment = engine
        .state()
        .document
        .resources
        .get("mol_001")
        .and_then(|resource| resource.data.as_fragment())
        .expect("fragment should still exist");
    assert_eq!(fragment.nodes[0].position, [20.0, 20.0]);
    assert_eq!(fragment.nodes[1].position, [70.0, 20.0]);
}

#[test]
fn select_all_collapses_grouped_molecule_and_text_to_one_group_box() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_group_molecule",
            "title": "group molecule",
            "page": { "width": 220.0, "height": 160.0, "background": "#ffffff" }
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
            "id": "grp_1",
            "type": "group",
            "zIndex": 30,
            "children": [
                {
                    "id": "obj_molecule_001",
                    "type": "molecule",
                    "visible": true,
                    "zIndex": 10,
                    "transform": { "translate": [10.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "styleRef": "style_molecule_default",
                    "payload": { "resourceRef": "mol_001", "bbox": [0.0, 0.0, 80.0, 40.0] }
                },
                {
                    "id": "group_text",
                    "type": "text",
                    "visible": true,
                    "zIndex": 20,
                    "transform": { "translate": [100.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "payload": { "text": "Note", "bbox": [0.0, 0.0, 24.0, 12.0], "runs": [] }
                }
            ]
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 80.0, 40.0],
                    "nodes": [
                        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [10.0, 20.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n2", "element": "C", "atomicNumber": 6, "position": [60.0, 20.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
                    ]
                }
            }
        }
    }))
    .expect("document should deserialize");

    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");
    assert!(engine.select_all());

    let group_boxes = rects_with_role(&engine, RenderRole::SelectionBox);
    assert_eq!(group_boxes.len(), 1);
    assert!(
        group_boxes[0][0] <= 10.0
            && group_boxes[0][1] <= 10.0
            && group_boxes[0][2] >= 124.0
            && group_boxes[0][3] >= 50.0,
        "group box should cover both molecule and text, got {group_boxes:?}"
    );
    assert!(rects_with_role(&engine, RenderRole::SelectionTextBox).is_empty());
    assert!(rects_with_role(&engine, RenderRole::SelectionBond).is_empty());
    assert!(rects_with_role(&engine, RenderRole::SelectionNode).is_empty());
    let bond_dot_count = engine
        .render_list()
        .into_iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Circle {
                    role: RenderRole::SelectionBondDot,
                    ..
                }
            )
        })
        .count();
    assert_eq!(bond_dot_count, 0);
}

#[test]
fn moving_selected_grouped_molecule_does_not_move_nodes_twice() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_group_molecule_move",
            "title": "group molecule move",
            "page": { "width": 220.0, "height": 160.0, "background": "#ffffff" }
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
            "id": "grp_1",
            "type": "group",
            "zIndex": 30,
            "children": [{
                "id": "obj_molecule_001",
                "type": "molecule",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [10.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_molecule_default",
                "payload": { "resourceRef": "mol_001", "bbox": [0.0, 0.0, 80.0, 40.0] }
            }]
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 80.0, 40.0],
                    "nodes": [
                        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [10.0, 20.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n2", "element": "C", "atomicNumber": 6, "position": [60.0, 20.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
                    ]
                }
            }
        }
    }))
    .expect("document should deserialize");

    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");
    assert!(engine.select_all());
    assert!(engine.begin_selection_move_at_point(Point::new(20.0, 30.0), false, false));
    assert!(engine.update_selection_move(Point::new(30.0, 30.0), false));
    assert!(engine.finish_selection_move(Point::new(30.0, 30.0), false));

    let molecule = engine
        .state()
        .document
        .find_scene_object("obj_molecule_001")
        .expect("grouped molecule should still exist");
    assert_eq!(molecule.transform.translate, [20.0, 10.0]);
    let fragment = engine
        .state()
        .document
        .resources
        .get("mol_001")
        .and_then(|resource| resource.data.as_fragment())
        .expect("fragment should still exist");
    assert_eq!(fragment.nodes[0].position, [10.0, 20.0]);
    assert_eq!(fragment.nodes[1].position, [60.0, 20.0]);
}

#[test]
fn render_targets_for_selected_group_include_child_molecule_labels() {
    let document = grouped_labeled_molecule_document();
    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");

    let primitives = engine.render_targets(
        &BTreeSet::new(),
        &BTreeSet::new(),
        &BTreeSet::from(["grp_1".to_string()]),
    );

    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Text {
                    role: RenderRole::DocumentText,
                    object_id,
                    node_id,
                    ..
                } if object_id.as_deref() == Some("obj_molecule_001")
                    && node_id.as_deref() == Some("n2")
            )
        }),
        "rendering a selected group target must include child molecule label primitives"
    );
    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Line {
                    role: RenderRole::DocumentBond,
                    object_id,
                    bond_id,
                    ..
                }
                | RenderPrimitive::Polygon {
                    role: RenderRole::DocumentBond,
                    object_id,
                    bond_id,
                    ..
                } if object_id.as_deref() == Some("obj_molecule_001")
                    && bond_id.as_deref() == Some("b1")
            )
        }),
        "rendering a selected group target must include child molecule bond primitives"
    );
}

#[test]
fn moving_selected_grouped_molecule_moves_child_label_world_position() {
    let document = grouped_labeled_molecule_document();
    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");
    assert!(engine.select_component_at_point(Point::new(70.0, 30.0), false));
    assert_eq!(engine.state().selection.arrow_objects, vec!["grp_1"]);

    let before_x = engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                role: RenderRole::DocumentText,
                node_id,
                x,
                ..
            } if node_id.as_deref() == Some("n2") => Some(x),
            _ => None,
        })
        .expect("label should render before move");

    assert!(engine.begin_selection_move_at_point(Point::new(70.0, 30.0), false, false));
    assert!(engine.update_selection_move(Point::new(82.0, 35.0), false));
    assert!(engine.finish_selection_move(Point::new(82.0, 35.0), false));

    let after_x = engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                role: RenderRole::DocumentText,
                node_id,
                x,
                ..
            } if node_id.as_deref() == Some("n2") => Some(x),
            _ => None,
        })
        .expect("label should render after move");

    assert_close(after_x, before_x + 12.0);
}

#[test]
fn double_click_grouped_molecule_bond_selects_group() {
    let document = grouped_two_fragment_document();
    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");

    engine.select_at_point(Point::new(120.0, 20.0), false);
    assert_eq!(engine.state().selection.bonds, vec!["b_grouped"]);

    assert!(engine.select_component_at_point(Point::new(120.0, 20.0), false));
    assert_eq!(engine.state().selection.arrow_objects, vec!["obj_group"]);
    assert!(engine.state().selection.bonds.is_empty());
}

#[test]
fn engine_groups_and_ungroups_selected_scene_objects_without_geometry_drift() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_group_edit",
            "title": "group edit",
            "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape": {
                "kind": "shape",
                "stroke": "#000000",
                "strokeWidth": 1.0
            }
        },
        "objects": [
            {
                "id": "shape_a",
                "type": "shape",
                "zIndex": 10,
                "transform": { "translate": [10.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_shape",
                "payload": { "bbox": [0.0, 0.0, 20.0, 10.0], "kind": "rect" }
            },
            {
                "id": "shape_b",
                "type": "shape",
                "zIndex": 20,
                "transform": { "translate": [50.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_shape",
                "payload": { "bbox": [0.0, 0.0, 30.0, 10.0], "kind": "rect" }
            }
        ],
        "resources": {}
    }))
    .expect("document should deserialize");
    let before = render_primitives_bounds(render_document(&document).iter()).unwrap();

    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");
    engine.select_in_rect(Point::new(0.0, 0.0), Point::new(90.0, 60.0), false);
    assert!(engine.group_selection());
    let group = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "group")
        .expect("group should be created");
    assert_eq!(group.children.len(), 2);
    assert_eq!(group.z_index, 20);
    let grouped_bounds = render_primitives_bounds(render_document(&engine.state().document).iter())
        .expect("grouped document should render");
    assert_eq!(before, grouped_bounds);

    assert!(engine.ungroup_selection());
    assert!(engine
        .state()
        .document
        .objects
        .iter()
        .all(|object| object.object_type != "group"));
    let ungrouped_bounds =
        render_primitives_bounds(render_document(&engine.state().document).iter())
            .expect("ungrouped document should render");
    assert_eq!(before, ungrouped_bounds);
}

#[test]
fn context_hit_test_reports_object_without_mutating_selection() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_context_hit",
            "title": "context hit",
            "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape": {
                "kind": "shape",
                "stroke": "#000000",
                "strokeWidth": 1.0
            }
        },
        "objects": [{
            "id": "shape_a",
            "type": "shape",
            "zIndex": 10,
            "transform": { "translate": [10.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_shape",
            "payload": { "bbox": [0.0, 0.0, 20.0, 10.0], "kind": "rect" }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");
    let hit: serde_json::Value =
        serde_json::from_str(&engine.context_hit_test_json(Point::new(15.0, 15.0))).unwrap();
    assert_eq!(hit["kind"], "object");
    assert_eq!(hit["objectId"], "shape_a");
    assert_eq!(hit["objectType"], "shape");
    assert_eq!(hit["selected"], false);
    assert!(engine.state().selection.is_empty());

    engine.select_at_point(Point::new(15.0, 15.0), false);
    let selected_hit: serde_json::Value =
        serde_json::from_str(&engine.context_hit_test_json(Point::new(15.0, 15.0))).unwrap();
    assert_eq!(selected_hit["selected"], true);

    let canvas_hit: serde_json::Value =
        serde_json::from_str(&engine.context_hit_test_json(Point::new(150.0, 120.0))).unwrap();
    assert_eq!(canvas_hit["kind"], "canvas");
}

#[test]
fn complete_molecule_selection_suppresses_internal_selection_dots() {
    let mut engine = Engine::new();
    engine
        .load_document_json(
            &serde_json::to_string(&fragment_document(
                json!([
                    { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 20.0], "charge": 0, "numHydrogens": 0 },
                    { "id": "n2", "element": "C", "atomicNumber": 6, "position": [50.0, 20.0], "charge": 0, "numHydrogens": 0 }
                ]),
                json!([
                    { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
                ]),
            ))
            .unwrap(),
        )
        .expect("document should load");
    assert!(engine.select_component_at_point(Point::new(20.0, 20.0), false));
    let dot_count = engine
        .render_list()
        .into_iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Circle {
                    role: RenderRole::SelectionBondDot,
                    ..
                }
            )
        })
        .count();
    assert_eq!(dot_count, 0);
}

#[test]
fn select_all_selects_document_surface_without_expanding_groups() {
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_select_all",
            "title": "select all",
            "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape": {
                "kind": "shape",
                "stroke": "#000000",
                "strokeWidth": 1.0
            }
        },
        "objects": [
            {
                "id": "obj_molecule_001",
                "type": "molecule",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "resourceRef": "mol_001" }
            },
            {
                "id": "shape_1",
                "type": "shape",
                "visible": true,
                "zIndex": 20,
                "transform": { "translate": [80.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_shape",
                "payload": { "bbox": [0.0, 0.0, 20.0, 10.0], "kind": "rect" }
            },
            {
                "id": "text_1",
                "type": "text",
                "visible": true,
                "zIndex": 30,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "text": "Note", "bbox": [20.0, 70.0, 60.0, 90.0], "runs": [] }
            },
            {
                "id": "group_1",
                "type": "group",
                "visible": true,
                "zIndex": 40,
                "children": [
                    {
                        "id": "group_child_shape",
                        "type": "shape",
                        "visible": true,
                        "zIndex": 41,
                        "transform": { "translate": [110.0, 70.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                        "styleRef": "style_shape",
                        "payload": { "bbox": [0.0, 0.0, 20.0, 10.0], "kind": "rect" }
                    },
                    {
                        "id": "group_child_text",
                        "type": "text",
                        "visible": true,
                        "zIndex": 42,
                        "transform": { "translate": [138.0, 68.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                        "payload": { "text": "cond", "bbox": [0.0, 0.0, 26.0, 12.0], "runs": [] }
                    }
                ]
            }
        ],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 60.0, 40.0],
                    "nodes": [
                        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [10.0, 10.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n2", "element": "C", "atomicNumber": 6, "position": [40.0, 10.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
                    ]
                }
            }
        }
    });
    let mut engine = Engine::new();
    engine
        .load_document_json(&document.to_string())
        .expect("document should load");

    assert!(engine.select_all());
    let selection = &engine.state().selection;
    assert_eq!(selection.nodes, vec!["n1".to_string(), "n2".to_string()]);
    assert_eq!(selection.bonds, vec!["b1".to_string()]);
    assert_eq!(selection.text_objects, vec!["text_1".to_string()]);
    assert_eq!(
        selection.arrow_objects,
        vec!["shape_1".to_string(), "group_1".to_string()]
    );
    assert!(!selection
        .arrow_objects
        .contains(&"group_child_shape".to_string()));

    let selection_boxes: Vec<_> = engine
        .render_list()
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Rect {
                role: RenderRole::SelectionBox,
                x,
                y,
                width,
                height,
                ..
            } => Some((x, y, width, height)),
            _ => None,
        })
        .collect();
    assert!(selection_boxes.iter().any(|(x, y, width, height)| {
        *x <= 110.0
            && *x >= 100.0
            && *y <= 68.0
            && *y >= 60.0
            && (*x + *width) >= 164.0
            && (*y + *height) >= 80.0
    }));

    let internal_dot_count = engine
        .render_list()
        .into_iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Circle {
                    role: RenderRole::SelectionBondDot,
                    ..
                }
            )
        })
        .count();
    assert_eq!(internal_dot_count, 0);
    assert!(!engine.select_all());
}

#[test]
fn engine_reports_document_colors_from_document_model() {
    let mut engine = Engine::new();
    engine
        .load_document_json(
            &json!({
                "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
                "document": {
                    "id": "doc_colors",
                    "title": "colors",
                    "page": { "width": 200.0, "height": 160.0, "background": "#fff" }
                },
                "styles": {
                    "style_molecule_default": {
                        "kind": "molecule",
                        "stroke": "rgb(1, 2, 3)",
                        "fill": "#abc",
                        "strokeWidth": 0.85,
                        "fontSize": 11.0
                    }
                },
                "objects": [{
                    "id": "obj_molecule",
                    "type": "molecule",
                    "styleRef": "style_molecule_default",
                    "payload": { "resourceRef": "mol" }
                }],
                "resources": {
                    "mol": {
                        "type": "molecule_fragment2d",
                        "encoding": "chemsema.molecule.fragment2d",
                        "data": {
                            "schema": "chemsema.molecule.fragment2d",
                            "bbox": [0.0, 0.0, 200.0, 160.0],
                            "nodes": [{
                                "id": "n1",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [20.0, 20.0],
                                "charge": 0,
                                "numHydrogens": 0,
                                "label": { "text": "Me", "fill": "#00ff00" }
                            }, {
                                "id": "n2",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [50.0, 20.0],
                                "charge": 0,
                                "numHydrogens": 0
                            }],
                            "bonds": [{
                                "id": "b1",
                                "begin": "n1",
                                "end": "n2",
                                "order": 1,
                                "stroke": "#ff00ff",
                                "strokeWidth": 0.85
                            }]
                        }
                    }
                }
            })
            .to_string(),
        )
        .unwrap();

    let colors = engine.document_colors();
    assert!(colors.contains(&"#ffffff".to_string()));
    assert!(colors.contains(&"#010203".to_string()));
    assert!(colors.contains(&"#aabbcc".to_string()));
    assert!(colors.contains(&"#00ff00".to_string()));
    assert!(colors.contains(&"#ff00ff".to_string()));
    assert_eq!(colors.iter().filter(|color| *color == "#ffffff").count(), 1);
}

fn polygon_area(points: &[chemsema_engine::Point]) -> f64 {
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
    center: chemsema_engine::Point,
) -> Vec<Vec<chemsema_engine::Point>> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001")
                && points.iter().any(|point| point.distance(center) <= 4.0) =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .collect()
}

fn object_bond_polygons(primitives: &[RenderPrimitive]) -> Vec<Vec<chemsema_engine::Point>> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
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

fn object_knockout_polygons(primitives: &[RenderPrimitive]) -> Vec<Vec<chemsema_engine::Point>> {
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

fn document_bond_polygon_count_for_object(
    primitives: &[RenderPrimitive],
    object_id: &str,
) -> usize {
    document_bond_polygons_for_object(primitives, object_id).len()
}

fn document_bond_polygons_for_object(
    primitives: &[RenderPrimitive],
    object_id: &str,
) -> Vec<Vec<chemsema_engine::Point>> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id: primitive_object_id,
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                role,
                object_id: primitive_object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && primitive_object_id.as_deref() == Some(object_id) =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .collect()
}

fn document_knockout_count_for_object(primitives: &[RenderPrimitive], object_id: &str) -> usize {
    primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role,
                    object_id: primitive_object_id,
                    ..
                } if *role == RenderRole::DocumentKnockout
                    && primitive_object_id.as_deref() == Some(object_id)
            )
        })
        .count()
}

fn document_bond_axis_lengths_for_object(
    primitives: &[RenderPrimitive],
    object_id: &str,
) -> Vec<f64> {
    let mut lengths: Vec<_> = document_bond_polygons_for_object(primitives, object_id)
        .iter()
        .filter_map(|points| bond_axis_length(points))
        .collect();
    lengths.sort_by(f64::total_cmp);
    lengths
}

fn document_bond_axis_intervals_for_object(
    primitives: &[RenderPrimitive],
    object_id: &str,
) -> Vec<(f64, f64)> {
    let polygons = document_bond_polygons_for_object(primitives, object_id);
    let (axis_from, axis_to) = polygons
        .iter()
        .filter_map(|points| {
            let (axis_from, axis_to) = bond_axis_from_points(points)?;
            let length = axis_from.distance(axis_to);
            Some((length, axis_from, axis_to))
        })
        .max_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, axis_from, axis_to)| (axis_from, axis_to))
        .expect("object should have a bond axis");
    let mut intervals: Vec<_> = polygons
        .iter()
        .filter_map(|points| projection_range_on_axis(points, axis_from, axis_to))
        .collect();
    let origin = intervals
        .iter()
        .map(|(start, _)| *start)
        .fold(f64::INFINITY, f64::min);
    for (start, end) in &mut intervals {
        *start -= origin;
        *end -= origin;
    }
    intervals.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));
    intervals
}

fn render_roundtrip_signature(document: &ChemSemaDocument) -> Vec<String> {
    let primitives: Vec<_> = render_document(document)
        .into_iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Line {
                    role: RenderRole::DocumentBond | RenderRole::DocumentGraphic,
                    ..
                } | RenderPrimitive::Circle {
                    role: RenderRole::DocumentGraphic | RenderRole::DocumentText,
                    ..
                } | RenderPrimitive::Polygon {
                    role: RenderRole::DocumentBond
                        | RenderRole::DocumentGraphic
                        | RenderRole::DocumentKnockout,
                    ..
                } | RenderPrimitive::Rect {
                    role: RenderRole::DocumentGraphic | RenderRole::DocumentText,
                    ..
                } | RenderPrimitive::Ellipse {
                    role: RenderRole::DocumentGraphic,
                    ..
                } | RenderPrimitive::Polyline {
                    role: RenderRole::DocumentGraphic | RenderRole::DocumentBond,
                    ..
                } | RenderPrimitive::Path {
                    role: RenderRole::DocumentGraphic | RenderRole::DocumentBond,
                    ..
                } | RenderPrimitive::FilledPath {
                    role: RenderRole::DocumentGraphic,
                    ..
                } | RenderPrimitive::Text {
                    role: RenderRole::DocumentText,
                    ..
                }
            )
        })
        .collect();
    let (offset_x, offset_y) = render_signature_origin(&primitives);
    let mut signature: Vec<String> = primitives
        .iter()
        .map(|primitive| primitive_signature(primitive, offset_x, offset_y))
        .collect();
    signature.sort();
    signature
}

fn render_signature_origin(primitives: &[RenderPrimitive]) -> (f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    for primitive in primitives {
        for point in primitive_points(primitive) {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
        }
    }
    if min_x.is_finite() && min_y.is_finite() {
        (min_x, min_y)
    } else {
        (0.0, 0.0)
    }
}

fn primitive_points(primitive: &RenderPrimitive) -> Vec<Point> {
    match primitive {
        RenderPrimitive::Line { from, to, .. } => vec![*from, *to],
        RenderPrimitive::Circle { center, radius, .. } => vec![
            Point::new(center.x - radius, center.y - radius),
            Point::new(center.x + radius, center.y + radius),
        ],
        RenderPrimitive::Polygon { points, .. }
        | RenderPrimitive::Polyline { points, .. }
        | RenderPrimitive::Path { points, .. }
        | RenderPrimitive::FilledPath { points, .. } => points.clone(),
        RenderPrimitive::Rect {
            x,
            y,
            width,
            height,
            ..
        } => vec![Point::new(*x, *y), Point::new(*x + *width, *y + *height)],
        RenderPrimitive::Ellipse { center, rx, ry, .. } => vec![
            Point::new(center.x - rx, center.y - ry),
            Point::new(center.x + rx, center.y + ry),
        ],
        RenderPrimitive::Text { x, y, .. } => vec![Point::new(*x, *y)],
    }
}

fn primitive_signature(primitive: &RenderPrimitive, offset_x: f64, offset_y: f64) -> String {
    match primitive {
        RenderPrimitive::Line {
            role,
            from,
            to,
            stroke,
            stroke_width,
            dash_array,
            ..
        } => format!(
            "line:{role:?}:{}:{}:{stroke}:{}:{:?}",
            point_sig(*from, offset_x, offset_y),
            point_sig(*to, offset_x, offset_y),
            num_sig(*stroke_width),
            nums_sig(dash_array),
        ),
        RenderPrimitive::Circle {
            role,
            center,
            radius,
            fill,
            stroke,
            stroke_width,
            ..
        } => format!(
            "circle:{role:?}:{}:{}:{fill}:{stroke}:{}",
            point_sig(*center, offset_x, offset_y),
            num_sig(*radius),
            num_sig(*stroke_width),
        ),
        RenderPrimitive::Polygon {
            role,
            points,
            fill,
            stroke,
            stroke_width,
            ..
        } => format!(
            "polygon:{role:?}:{}:{fill}:{stroke}:{}",
            points_sig(points, offset_x, offset_y),
            num_sig(*stroke_width),
        ),
        RenderPrimitive::Rect {
            role,
            x,
            y,
            width,
            height,
            fill,
            stroke,
            stroke_width,
            rx,
            ry,
            dash_array,
            ..
        } => format!(
            "rect:{role:?}:{}:{}:{}:{}:{fill:?}:{stroke:?}:{}:{rx:?}:{ry:?}:{:?}",
            num_sig(*x - offset_x),
            num_sig(*y - offset_y),
            num_sig(*width),
            num_sig(*height),
            num_sig(*stroke_width),
            nums_sig(dash_array),
        ),
        RenderPrimitive::Ellipse {
            role,
            center,
            rx,
            ry,
            rotate,
            fill,
            stroke,
            stroke_width,
            dash_array,
            ..
        } => format!(
            "ellipse:{role:?}:{}:{}:{}:{}:{fill:?}:{stroke:?}:{}:{:?}",
            point_sig(*center, offset_x, offset_y),
            num_sig(*rx),
            num_sig(*ry),
            num_sig(*rotate),
            num_sig(*stroke_width),
            nums_sig(dash_array),
        ),
        RenderPrimitive::Polyline {
            role,
            points,
            stroke,
            stroke_width,
            dash_array,
            line_cap,
            line_join,
            ..
        } => format!(
            "polyline:{role:?}:{}:{stroke}:{}:{:?}:{line_cap:?}:{line_join:?}",
            points_sig(points, offset_x, offset_y),
            num_sig(*stroke_width),
            nums_sig(dash_array),
        ),
        RenderPrimitive::Path {
            role,
            d,
            points,
            stroke,
            stroke_width,
            dash_array,
            line_cap,
            line_join,
            ..
        } => format!(
            "path:{role:?}:{}:{d}:{stroke}:{}:{:?}:{line_cap:?}:{line_join:?}",
            points_sig(points, offset_x, offset_y),
            num_sig(*stroke_width),
            nums_sig(dash_array),
        ),
        RenderPrimitive::FilledPath {
            role,
            d,
            points,
            fill,
            fill_rule,
            ..
        } => format!(
            "filled-path:{role:?}:{}:{d}:{fill}:{fill_rule:?}",
            points_sig(points, offset_x, offset_y),
        ),
        RenderPrimitive::Text {
            role,
            x,
            y,
            text,
            font_size,
            font_family,
            fill,
            text_anchor,
            line_height,
            box_width,
            runs,
            ..
        } => format!(
            "text:{role:?}:{}:{}:{text}:{}:{font_family:?}:{fill:?}:{text_anchor:?}:{line_height:?}:{box_width:?}:{runs:?}",
            num_sig(*x - offset_x),
            num_sig(*y - offset_y),
            num_sig(*font_size),
        ),
    }
}

fn point_sig(point: Point, offset_x: f64, offset_y: f64) -> String {
    format!(
        "{} {}",
        num_sig(point.x - offset_x),
        num_sig(point.y - offset_y)
    )
}

fn points_sig(points: &[Point], offset_x: f64, offset_y: f64) -> String {
    points
        .iter()
        .map(|point| point_sig(*point, offset_x, offset_y))
        .collect::<Vec<_>>()
        .join(";")
}

fn nums_sig(values: &[f64]) -> Vec<String> {
    values.iter().map(|value| num_sig(*value)).collect()
}

fn num_sig(value: f64) -> String {
    format!("{:.2}", value)
}

fn object_knockout_rect_count(primitives: &[RenderPrimitive]) -> usize {
    primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Rect { role, object_id, .. }
                    if *role == RenderRole::DocumentKnockout
                        && object_id.as_deref() == Some("obj_molecule_001")
            )
        })
        .count()
}

fn object_bond_polygons_with_ids(
    primitives: &[RenderPrimitive],
) -> Vec<(String, Vec<chemsema_engine::Point>)> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                bond_id,
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
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

fn object_bond_points_for_id(
    primitives: &[RenderPrimitive],
    target_bond_id: &str,
) -> Vec<chemsema_engine::Point> {
    object_bond_polygons_with_ids(primitives)
        .into_iter()
        .filter(|(bond_id, _)| bond_id == target_bond_id)
        .flat_map(|(_, points)| points)
        .collect()
}

fn object_bond_centerlines_with_ids(
    primitives: &[RenderPrimitive],
    target_object_id: &str,
) -> Vec<(String, chemsema_engine::Point, chemsema_engine::Point)> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Line {
                role,
                object_id,
                bond_id,
                from,
                to,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some(target_object_id) =>
            {
                Some((bond_id.clone().unwrap_or_default(), *from, *to))
            }
            RenderPrimitive::Polygon {
                role,
                object_id,
                bond_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some(target_object_id) =>
            {
                let (from, to) = bond_axis_from_points(points)?;
                Some((bond_id.clone().unwrap_or_default(), from, to))
            }
            _ => None,
        })
        .collect()
}

fn bond_axis_from_points(
    points: &[chemsema_engine::Point],
) -> Option<(chemsema_engine::Point, chemsema_engine::Point)> {
    if points.len() != 4 {
        return None;
    }
    Some((
        chemsema_engine::Point::new(
            (points[0].x + points[3].x) * 0.5,
            (points[0].y + points[3].y) * 0.5,
        ),
        chemsema_engine::Point::new(
            (points[1].x + points[2].x) * 0.5,
            (points[1].y + points[2].y) * 0.5,
        ),
    ))
}

fn bond_polygon_normal_widths(points: &[chemsema_engine::Point]) -> Option<(f64, f64)> {
    let (from, to) = bond_axis_from_points(points)?;
    let axis = chemsema_engine::Point::new(to.x - from.x, to.y - from.y);
    let length = (axis.x.powi(2) + axis.y.powi(2)).sqrt();
    if length <= 1.0e-9 {
        return None;
    }
    let normal = chemsema_engine::Point::new(-axis.y / length, axis.x / length);
    let start_width =
        ((points[0].x - points[3].x) * normal.x + (points[0].y - points[3].y) * normal.y).abs();
    let end_width =
        ((points[1].x - points[2].x) * normal.x + (points[1].y - points[2].y) * normal.y).abs();
    Some((start_width, end_width))
}

fn bond_axis_length(points: &[chemsema_engine::Point]) -> Option<f64> {
    let (from, to) = bond_axis_from_points(points)?;
    Some(from.distance(to))
}

fn cdxml_shape_fills_by_z(document: &ChemSemaDocument) -> Vec<String> {
    let mut shapes: Vec<_> = document
        .objects
        .iter()
        .filter(|object| object.object_type == "shape")
        .collect();
    shapes.sort_by_key(|object| object.z_index);
    shapes
        .into_iter()
        .filter_map(|object| {
            let style_ref = object.style_ref.as_ref()?;
            document.styles.get(style_ref)?.get("fill")?.as_str()
        })
        .map(ToString::to_string)
        .collect()
}

#[test]
fn export_cdxml_emits_chemdraw_document_with_native_fragment() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "C",
                "atomicNumber": 6,
                "position": [30.0, 40.0],
                "charge": 0,
                "numHydrogens": 0
            },
            {
                "id": "n2",
                "element": "O",
                "atomicNumber": 8,
                "position": [70.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "O",
                    "sourceText": "O",
                    "position": [70.0, 40.0],
                    "box": [66.0, 28.0, 78.0, 44.0],
                    "fontSize": 10.0,
                    "fill": "#000000",
                    "attachment": "node",
                    "anchor": "start"
                }
            },
            {
                "id": "n3",
                "element": "C",
                "atomicNumber": 6,
                "position": [30.0, 80.0],
                "charge": 0,
                "numHydrogens": 0,
                "isPlaceholder": true,
                "label": {
                    "text": "CF3",
                    "sourceText": "CF3",
                    "position": [30.0, 80.0],
                    "box": [30.0, 70.0, 47.4, 82.5],
                    "fontSize": 10.0,
                    "fill": "#d61f1f",
                    "attachment": "node",
                    "anchor": "start"
                }
            }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "double": { "placement": "center", "frozen": false },
                "strokeWidth": 0.6,
                "bondSpacing": 18.0,
                "marginWidth": 1.6
            },
            {
                "id": "b2",
                "begin": "n1",
                "end": "n3",
                "order": 1,
                "stereo": { "kind": "solid-wedge", "wideEnd": "end" },
                "strokeWidth": 0.6
            }
        ]),
    );

    let cdxml = document_to_cdxml(&document);

    assert!(cdxml.contains("<!DOCTYPE CDXML"));
    assert!(cdxml.contains("<CDXML"));
    assert!(cdxml.contains("CreationProgram=\"ChemSema\""));
    assert!(cdxml.contains("LabelFace=\"96\""));
    assert!(cdxml.contains("CaptionFace=\"0\""));
    assert!(
        cdxml.contains("color=\"3\" bgcolor=\"1\""),
        "known black should reuse its color-table id: {cdxml}"
    );
    assert!(cdxml.contains("<page"));
    assert!(cdxml.contains("HeaderPosition=\"36\""));
    assert!(cdxml.contains("<fragment"));
    assert!(cdxml.contains("Order=\"2\""));
    assert!(cdxml.contains("BS=\"N\""));
    assert!(cdxml.contains("BondSpacing=\"18\""));
    assert!(cdxml.contains("MarginWidth=\"1.6\""));
    assert!(cdxml.contains("NodeType=\"Nickname\""));
    assert!(cdxml.contains("UTF8Text=\"CF3\""));
    assert!(!cdxml.contains("<t font="));
    assert!(!cdxml.contains("<t size="));
    assert!(!cdxml.contains("<t color="));
    assert!(cdxml.contains("<s font=\"3\" size=\"10\" color=\"3\""));

    let roundtripped =
        parse_cdxml_document(&cdxml, Some("roundtrip")).expect("export should parse");
    let fragment = roundtripped
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("roundtrip should create a molecule fragment");
    assert_eq!(fragment.nodes.len(), 3);
    assert_eq!(fragment.bonds.len(), 2);
    assert!(fragment.bonds.iter().any(|bond| bond.order == 2));
    assert!(fragment
        .nodes
        .iter()
        .any(|node| node.label.as_ref().is_some_and(|label| label.text == "CF3")));
    let cf3_label = fragment
        .nodes
        .iter()
        .find_map(|node| node.label.as_ref().filter(|label| label.text == "CF3"))
        .expect("CF3 label should roundtrip");
    assert_eq!(cf3_label.fill.as_deref(), Some("#d61f1f"));
}

#[test]
fn export_cdxml_preserves_text_run_style_across_reimport() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "text style",
            "page": { "width": 200.0, "height": 120.0, "background": "#ffffff" }
        },
        "styles": {
            "style_text_default": {
                "kind": "text",
                "fontFamily": "Times New Roman",
                "fontSize": 16.0,
                "fill": "#d61f1f",
                "stroke": null
            }
        },
        "objects": [{
            "id": "obj_text_001",
            "type": "text",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [24.0, 32.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_text_default",
            "payload": {
                "text": "RedBlue",
                "box": [0.0, 0.0, 80.0, 22.0],
                "align": "left",
                "fontSize": 16.0,
                "runs": [
                    {
                        "text": "Red",
                        "fontFamily": "Times New Roman",
                        "fontSize": 16.0,
                        "fill": "#d61f1f",
                        "fontWeight": 700,
                        "script": "normal"
                    },
                    {
                        "text": "Blue",
                        "fontFamily": "Arial",
                        "fontSize": 12.0,
                        "fill": "#1b32d8",
                        "fontStyle": "italic",
                        "script": "normal"
                    }
                ]
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let cdxml = document_to_cdxml(&document);
    let roundtripped =
        parse_cdxml_document(&cdxml, Some("text style")).expect("export should parse");
    let text_object = roundtripped
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should roundtrip");
    let runs: Vec<chemsema_engine::LabelRun> = serde_json::from_value(
        text_object
            .payload
            .extra
            .get("runs")
            .cloned()
            .expect("text runs should roundtrip"),
    )
    .expect("text runs should deserialize");

    assert_eq!(runs[0].text, "Red");
    assert_eq!(runs[0].fill.as_deref(), Some("#d61f1f"));
    assert_eq!(runs[0].font_family.as_deref(), Some("Times New Roman"));
    assert_eq!(runs[0].font_weight, Some(700));
    assert_eq!(runs[1].text, "Blue");
    assert_eq!(runs[1].fill.as_deref(), Some("#1b32d8"));
    assert_eq!(runs[1].font_family.as_deref(), Some("Arial"));
    assert_eq!(runs[1].font_style.as_deref(), Some("italic"));
}

#[test]
fn cdxml_single_atom_fragments_roundtrip_as_chemical_objects() {
    let source = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 160 80" LabelFont="3" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 160 80">
    <fragment id="2" BoundingBox="10 10 30 30"><n id="3" p="20 20" Element="1"/></fragment>
    <fragment id="4" BoundingBox="50 10 70 30"><n id="5" p="60 20" Element="19" Charge="1"/></fragment>
    <fragment id="6" BoundingBox="90 10 110 30"><n id="7" p="100 20" Element="55" Charge="1"/></fragment>
  </page>
</CDXML>"#;

    let imported = parse_cdxml_document(source, Some("single atoms")).expect("source imports");
    let exported = document_to_cdxml(&imported);
    let reopened =
        parse_cdxml_document(&exported, Some("single atoms reopened")).expect("export imports");
    let mut atoms: Vec<_> = reopened
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .filter_map(|fragment| fragment.nodes.first())
        .map(|node| (node.atomic_number, node.charge))
        .collect();
    atoms.sort_unstable();

    assert_eq!(atoms, vec![(1, 0), (19, 1), (55, 1)]);
    assert_eq!(
        reopened
            .objects
            .iter()
            .filter(|object| object.object_type == "molecule")
            .count(),
        3
    );
    assert!(!reopened
        .objects
        .iter()
        .any(|object| object.object_type == "text"));
}

#[test]
fn cdxml_custom_element_labels_preserve_atomic_identity() {
    let source = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 160 80" LabelFont="3" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 160 80">
    <fragment id="2" BoundingBox="10 10 130 50">
      <n id="3" p="20 30" Element="8" Charge="-1"><t p="16 34" BoundingBox="16 22 26 36" UTF8Text="O"><s font="3" size="10">O</s></t></n>
      <n id="4" p="70 30" Element="7"><t p="62 34" BoundingBox="62 18 78 38" UTF8Text="N&#10;H"><s font="3" size="10">N&#10;H</s></t></n>
      <n id="5" p="120 30" Element="62"><t p="104 34" BoundingBox="104 22 136 36" UTF8Text="SmIII"><s font="3" size="10">SmIII</s></t></n>
      <b id="6" B="3" E="4"/><b id="7" B="4" E="5"/>
    </fragment>
  </page>
</CDXML>"#;

    let imported = parse_cdxml_document(source, Some("custom elements")).expect("source imports");
    let exported = document_to_cdxml(&imported);
    assert!(exported.contains("Element=\"8\""));
    assert!(exported.contains("Element=\"7\""));
    assert!(exported.contains("Element=\"62\""));
    let reopened =
        parse_cdxml_document(&exported, Some("custom elements reopened")).expect("export imports");
    let fragment = reopened
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("molecule survives");
    let atomic_numbers: Vec<_> = fragment
        .nodes
        .iter()
        .map(|node| node.atomic_number)
        .collect();

    assert_eq!(atomic_numbers, vec![8, 7, 62]);
    assert_eq!(fragment.nodes[0].charge, -1);
    assert!(fragment.nodes.iter().all(|node| !node.is_placeholder));
}

#[test]
fn cdxml_headless_arrow_remains_an_arrow() {
    let source = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 160 80">
  <page id="1" BoundingBox="0 0 160 80">
    <arrow id="2" Tail3D="20 40 0" Head3D="140 40 0" ArrowheadHead="None" ArrowheadTail="None"/>
  </page>
</CDXML>"#;

    let imported = parse_cdxml_document(source, Some("headless arrow")).expect("source imports");
    let exported = document_to_cdxml(&imported);
    assert!(
        exported.contains("<arrow"),
        "arrow identity must survive: {exported}"
    );
    assert!(!exported.contains("GraphicType=\"Line\""));
    let reopened =
        parse_cdxml_document(&exported, Some("headless arrow reopened")).expect("export imports");
    let arrow = reopened
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow survives");
    let arrow_head = arrow
        .payload
        .extra
        .get("arrowHead")
        .expect("headless arrow keeps arrow payload");
    assert_eq!(
        arrow_head.get("head").and_then(|value| value.as_str()),
        Some("none")
    );
    assert_eq!(
        arrow_head.get("tail").and_then(|value| value.as_str()),
        Some("none")
    );
}

#[test]
fn cdxml_round_brackets_do_not_gain_groups_or_expand() {
    let source = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 160 100">
  <page id="1" BoundingBox="0 0 160 100">
    <graphic id="2" GraphicType="Bracket" BracketType="Round" BoundingBox="30 80 30 20"/>
    <graphic id="3" GraphicType="Bracket" BracketType="Round" BoundingBox="130 20 130 80"/>
  </page>
</CDXML>"#;

    let first = parse_cdxml_document(source, Some("round brackets")).expect("source imports");
    let first_group = first
        .objects
        .iter()
        .find(|object| object_is_bracket_group(object))
        .expect("bracket group exists");
    let first_positions: Vec<_> = first_group
        .children
        .iter()
        .map(|child| child.transform.translate)
        .collect();
    let exported = document_to_cdxml(&first);
    assert!(
        !exported.contains("<group"),
        "synthetic group must not be serialized"
    );
    let second = parse_cdxml_document(&exported, Some("round brackets second"))
        .expect("first export imports");
    let second_export = document_to_cdxml(&second);
    let third = parse_cdxml_document(&second_export, Some("round brackets third"))
        .expect("second export imports");

    for document in [&second, &third] {
        let groups: Vec<_> = document
            .objects
            .iter()
            .filter(|object| object_is_bracket_group(object))
            .collect();
        assert_eq!(groups.len(), 1);
        let positions: Vec<_> = groups[0]
            .children
            .iter()
            .map(|child| child.transform.translate)
            .collect();
        assert_eq!(positions, first_positions);
    }
}

#[test]
fn cdxml_drops_bonds_whose_normalized_endpoint_is_missing() {
    let source = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 100 60">
  <page id="1" BoundingBox="0 0 100 60">
    <fragment id="2" BoundingBox="10 10 90 50">
      <n id="3" p="20 30"/><n id="4" p="60 30"/><n id="5" p="not-a-point"/>
      <b id="6" B="3" E="4"/><b id="7" B="4" E="5"/>
    </fragment>
  </page>
</CDXML>"#;

    let document = parse_cdxml_document(source, Some("missing endpoint")).expect("source imports");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("valid component survives");
    assert_eq!(fragment.nodes.len(), 2);
    assert_eq!(fragment.bonds.len(), 1);
    assert_eq!(fragment.bonds[0].begin, "3");
    assert_eq!(fragment.bonds[0].end, "4");
}

#[test]
fn cdxml_import_export_import_is_render_stable_for_tmp_fixtures() {
    let fixtures = [
        "molecule.cdxml",
        "shape.cdxml",
        "kuohao.cdxml",
        "duibi.cdxml",
        "color.cdxml",
        "assets-acs.cdxml",
        "arrows-acs.cdxml",
    ];
    if fixtures
        .iter()
        .any(|fixture| !cdxml_fixture_exists(fixture))
    {
        eprintln!("skipping external CDXML roundtrip render suite; fixture set is incomplete");
        return;
    }
    for fixture in fixtures {
        let cdxml = read_cdxml_fixture(fixture);
        let imported = parse_cdxml_document(&cdxml, Some(fixture)).expect("fixture should import");
        let exported = document_to_cdxml(&imported);
        let reimported =
            parse_cdxml_document(&exported, Some(fixture)).expect("export should reimport");

        assert_eq!(
            render_roundtrip_signature(&reimported),
            render_roundtrip_signature(&imported),
            "{fixture} should be stable across import/export/import",
        );
    }
}

#[test]
fn cdxml_import_export_import_is_svg_stable_for_tmp_fixtures() {
    let fixtures = [
        "molecule.cdxml",
        "shape.cdxml",
        "kuohao.cdxml",
        "duibi.cdxml",
        "color.cdxml",
        "assets-acs.cdxml",
        "arrows-acs.cdxml",
    ];
    if fixtures
        .iter()
        .any(|fixture| !cdxml_fixture_exists(fixture))
    {
        eprintln!("skipping external CDXML roundtrip SVG suite; fixture set is incomplete");
        return;
    }
    for fixture in fixtures {
        let cdxml = read_cdxml_fixture(fixture);
        let imported = parse_cdxml_document(&cdxml, Some(fixture)).expect("fixture should import");
        let exported = document_to_cdxml(&imported);
        let reimported =
            parse_cdxml_document(&exported, Some(fixture)).expect("export should reimport");

        assert_eq!(
            document_to_svg(&reimported),
            document_to_svg(&imported),
            "{fixture} should keep the same SVG across import/export/import",
        );
    }
}

#[test]
fn public_cdxml_fixture_svg_golden_snapshots_match() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.join("../..");
    let fixture_dir = repo_root.join("fixtures").join("cdxml");
    let expected_dir = repo_root.join("fixtures").join("expected").join("svg");
    let mut fixtures = std::fs::read_dir(&fixture_dir)
        .unwrap_or_else(|error| panic!("{}: {error}", fixture_dir.display()))
        .map(|entry| entry.expect("fixture entry should be readable").path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("cdxml"))
        .collect::<Vec<_>>();
    fixtures.sort();
    assert!(
        !fixtures.is_empty(),
        "public CDXML fixture directory should contain regression cases"
    );

    for fixture_path in fixtures {
        let stem = fixture_path
            .file_stem()
            .and_then(|value| value.to_str())
            .expect("fixture file should have a UTF-8 stem");
        let expected_path = expected_dir.join(format!("{stem}.svg"));
        let cdxml = std::fs::read_to_string(&fixture_path)
            .unwrap_or_else(|error| panic!("{}: {error}", fixture_path.display()));
        let mut engine = Engine::new();
        engine
            .load_cdxml_document(&cdxml)
            .unwrap_or_else(|error| panic!("{stem}: {error}"));
        let actual = normalize_svg_snapshot(&engine.document_svg());
        let expected = normalize_svg_snapshot(
            &std::fs::read_to_string(&expected_path)
                .unwrap_or_else(|error| panic!("{}: {error}", expected_path.display())),
        );

        assert_eq!(actual, expected, "{stem} SVG golden snapshot changed");
    }
}

fn normalize_svg_snapshot(value: &str) -> String {
    let normalized = value.replace("\r\n", "\n");
    format!("{}\n", normalized.trim_end())
}

#[test]
fn cdxml_exported_arrow_fixtures_are_stable_after_first_save() {
    for fixture in ["assets-acs.cdxml", "arrows-acs.cdxml"] {
        let Some(cdxml) = read_optional_cdxml_fixture(fixture) else {
            continue;
        };
        let imported = parse_cdxml_document(&cdxml, Some(fixture)).expect("fixture should import");
        let first_export = document_to_cdxml(&imported);
        let first_reimport =
            parse_cdxml_document(&first_export, Some(fixture)).expect("first export should import");
        let second_export = document_to_cdxml(&first_reimport);
        let second_reimport = parse_cdxml_document(&second_export, Some(fixture))
            .expect("second export should import");

        assert_eq!(
            render_roundtrip_signature(&second_reimport),
            render_roundtrip_signature(&first_reimport),
            "{fixture} should not drift after the first save",
        );
    }
}

#[test]
fn export_svg_emits_rendered_document_primitives() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "C",
                "atomicNumber": 6,
                "position": [30.0, 40.0],
                "charge": 0,
                "numHydrogens": 0
            },
            {
                "id": "n2",
                "element": "O",
                "atomicNumber": 8,
                "position": [70.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "O",
                    "sourceText": "O",
                    "position": [70.0, 40.0],
                    "box": [66.0, 28.0, 78.0, 44.0],
                    "fontSize": 10.0,
                    "fill": "#000000",
                    "attachment": "node",
                    "anchor": "start"
                }
            }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "double": { "placement": "center", "frozen": false },
                "strokeWidth": 0.6,
                "bondSpacing": 18.0
            }
        ]),
    );

    let svg = document_to_svg(&document);

    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.contains("viewBox=\""));
    assert!(svg.contains("<polygon"));
    assert!(svg.contains("<text"));
    assert!(svg.contains(">O</"));
    assert!(!svg.contains("document-knockout"));
}

#[test]
fn load_cdxml_document_preserves_display_fragments_for_editing_hit_tests() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="18" LineWidth="0.6" BoldWidth="2" HashSpacing="2.5" BondSpacing="18">
  <page id="1" BoundingBox="0 0 120 80">
    <fragment id="10" BoundingBox="10 10 40 20">
      <n id="11" p="10 15"/>
      <n id="12" p="40 15"/>
      <b id="13" B="11" E="12" Order="1"/>
    </fragment>
    <fragment id="20" BoundingBox="70 10 100 20">
      <n id="21" p="70 15"/>
      <n id="22" p="100 15"/>
      <b id="23" B="21" E="22" Order="1"/>
    </fragment>
  </page>
</CDXML>"#;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("cdxml should load into editing engine");
    let document = &engine.state().document;
    let molecule_objects = document
        .objects
        .iter()
        .filter(|object| object.object_type == "molecule")
        .count();
    assert_eq!(molecule_objects, 2);
    let fragments = document.editable_fragments();
    assert_eq!(fragments.len(), 2);
    assert_eq!(fragments[0].fragment.bonds.len(), 1);
    assert_eq!(fragments[1].fragment.bonds.len(), 1);
    assert!(!document
        .objects
        .iter()
        .any(|object| object.id == "obj_cdxml_merged_molecule"));
    assert!(hit_test_bond_center(
        &document,
        Point::new(85.0 * CDXML_EDIT_SCALE, 15.0 * CDXML_EDIT_SCALE),
        30.0 * CDXML_EDIT_SCALE
    )
    .is_some());
}

#[test]
fn load_cdxml_document_splits_disconnected_components_inside_one_fragment() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="18" LineWidth="0.6" BoldWidth="2" HashSpacing="2.5" BondSpacing="18">
  <page id="1" BoundingBox="0 0 140 80">
    <fragment id="10" BoundingBox="10 10 112 20">
      <n id="11" p="10 15"/>
      <n id="12" p="40 15"/>
      <b id="13" B="11" E="12" Order="1"/>
      <n id="21" p="82 15"/>
      <n id="22" p="112 15"/>
      <b id="23" B="21" E="22" Order="1"/>
    </fragment>
  </page>
</CDXML>"#;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("cdxml should load into editing engine");
    let document = &engine.state().document;
    let molecule_objects = document
        .objects
        .iter()
        .filter(|object| object.object_type == "molecule")
        .count();
    assert_eq!(molecule_objects, 2);
    let fragments = document.editable_fragments();
    assert_eq!(fragments.len(), 2);
    assert_eq!(fragments[0].fragment.nodes.len(), 2);
    assert_eq!(fragments[0].fragment.bonds.len(), 1);
    assert_eq!(fragments[1].fragment.nodes.len(), 2);
    assert_eq!(fragments[1].fragment.bonds.len(), 1);
    assert_eq!(document.resources.len(), 2);
    assert!(document.resources.contains_key("mol_001"));
    assert!(document.resources.contains_key("mol_002"));
    assert!(hit_test_bond_center(
        document,
        Point::new(25.0 * CDXML_EDIT_SCALE, 15.0 * CDXML_EDIT_SCALE),
        30.0 * CDXML_EDIT_SCALE
    )
    .is_some());
    assert!(hit_test_bond_center(
        document,
        Point::new(97.0 * CDXML_EDIT_SCALE, 15.0 * CDXML_EDIT_SCALE),
        30.0 * CDXML_EDIT_SCALE
    )
    .is_some());
}

#[test]
fn load_cdxml_document_preserves_figure2_display_fragments() {
    let Some(cdxml) = read_optional_cdxml_fixture("figure2.cdxml") else {
        return;
    };
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(&cdxml)
        .expect("figure2 should load into editing engine");
    let document = &engine.state().document;
    let molecule_objects = document
        .objects
        .iter()
        .filter(|object| object.object_type == "molecule")
        .count();
    assert_eq!(molecule_objects, 7);
    assert_eq!(document.editable_fragments().len(), 7);
    assert!(!document
        .objects
        .iter()
        .any(|object| object.id == "obj_cdxml_merged_molecule"));
    assert!(!document.resources.contains_key("mol_cdxml_merged"));
}

#[test]
fn render_cdxml_fragment_node_labels_interleave_with_external_graphics_by_source_z() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="18" LineWidth="0.6" BoldWidth="2" HashSpacing="2.5" BondSpacing="18">
  <page id="1" BoundingBox="0 0 120 80">
    <fragment id="10" BoundingBox="0 0 30 20" Z="5">
      <n id="11" p="5 10" Z="5"/>
      <n id="12" p="25 10" Z="5"/>
      <b id="13" B="11" E="12" Order="1" Z="30"/>
    </fragment>
    <fragment id="20" BoundingBox="40 40 85 65" Z="5">
      <n id="21" p="52 50" Z="30" Element="18">
        <t p="54 54" BoundingBox="44 44 54 54" LabelJustification="Right">
          <s font="3" size="10" color="0">Ar</s>
        </t>
      </n>
      <n id="22" p="74 50" Z="10"/>
      <b id="23" B="21" E="22" Order="1" Z="10"/>
    </fragment>
    <graphic id="30"
      BoundingBox="58.64 50 50 50"
      Z="20"
      GraphicType="Orbital"
      OvalType="Circle Shaded"
      OrbitalType="sShaded"
      Center3D="50 50 0"
      MajorAxisEnd3D="58.64 50 0"
      MinorAxisEnd3D="50 58.64 0"/>
  </page>
</CDXML>"#;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("layered cdxml should load");
    let document = &engine.state().document;
    let primitives = render_document(&document);
    let orbital_last = primitives
        .iter()
        .enumerate()
        .filter(|(_, primitive)| {
            render_primitive_object_id(primitive) == Some("obj_shape_orbital_001")
        })
        .map(|(index, _)| index)
        .max()
        .expect("orbital should render");
    let ar_text = primitives
        .iter()
        .enumerate()
        .find_map(|(index, primitive)| {
            (render_primitive_text_content(primitive).as_deref() == Some("Ar")).then_some(index)
        })
        .expect("Ar node label should render");
    let high_bond = primitives
        .iter()
        .enumerate()
        .find_map(|(index, primitive)| {
            (render_primitive_bond_id(primitive) == Some("13")).then_some(index)
        })
        .unwrap_or_else(|| {
            panic!(
                "high-Z bond should render; bond ids: {:?}",
                primitives
                    .iter()
                    .filter_map(render_primitive_bond_id)
                    .collect::<Vec<_>>()
            )
        });

    assert!(
        orbital_last < ar_text,
        "source node Z should draw the Ar label above the external orbital"
    );
    assert!(
        orbital_last < high_bond,
        "source bond Z should draw the imported bond above the external orbital"
    );
}

#[test]
fn render_cdxml_group_children_keep_source_z_against_external_symbols() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="18" LineWidth="0.6" BondSpacing="18">
  <page id="1" BoundingBox="0 0 80 80">
    <group id="10" Z="30">
      <graphic id="11"
        BoundingBox="40 20 40 34"
        Z="1"
        GraphicType="Orbital"
        OrbitalType="lobe"/>
    </group>
    <graphic id="12"
      BoundingBox="40 26 40 38"
      Z="2"
      GraphicType="Symbol"
      SymbolType="Electron"/>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("group z")).expect("grouped orbital cdxml should parse");
    let primitives = render_document(&document);
    let lobe_last = primitives
        .iter()
        .enumerate()
        .filter(|(_, primitive)| {
            render_primitive_object_id(primitive) == Some("obj_shape_orbital_001")
        })
        .map(|(index, _)| index)
        .max()
        .expect("grouped lobe should render");
    let electron_first = primitives
        .iter()
        .enumerate()
        .find_map(|(index, primitive)| {
            (render_primitive_object_id(primitive) == Some("obj_symbol_001")).then_some(index)
        })
        .expect("external electron should render");

    assert!(
        lobe_last < electron_first,
        "CDXML group Z must not lift the white lobe fill above the higher-Z electron"
    );
}

#[test]
fn cdxml_generic_r_prime_labels_do_not_render_invalid_markers() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="18" LineWidth="0.6" BondSpacing="18">
  <page id="1" BoundingBox="0 0 120 50">
    <fragment id="10" BoundingBox="0 0 100 40">
      <n id="11" p="10 20" NodeType="GenericNickname" Z="1">
        <t p="10 20" BoundingBox="4 12 14 22"><s font="3" size="10">R&apos;</s></t>
      </n>
      <n id="12" p="40 20" Z="2"/>
      <n id="13" p="70 20" NodeType="GenericNickname" Z="3">
        <t p="70 20" BoundingBox="64 12 76 22"><s font="3" size="10">R&apos;&apos;</s></t>
      </n>
      <b id="14" B="11" E="12" Order="1" Z="4"/>
      <b id="15" B="12" E="13" Order="1" Z="5"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("r groups")).expect("generic R labels should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| match &resource.data {
            ResourceData::Fragment(fragment) => Some(fragment),
            _ => None,
        })
        .expect("fragment should be imported");
    for text in ["R'", "R''"] {
        let node = fragment
            .nodes
            .iter()
            .find(|node| {
                node.label
                    .as_ref()
                    .is_some_and(|label| label.source_text.as_deref() == Some(text))
            })
            .unwrap_or_else(|| panic!("{text} node should exist"));
        assert!(
            node.is_placeholder,
            "{text} GenericNickname should import as a placeholder"
        );
        assert_ne!(
            node.meta
                .get("labelRecognition")
                .and_then(|value| value.get("status"))
                .and_then(serde_json::Value::as_str),
            Some("invalid"),
            "{text} should not be an invalid chemical label"
        );
        assert_ne!(
            node.label
                .as_ref()
                .and_then(|label| label.meta.get("labelRecognition"))
                .and_then(|value| value.get("status"))
                .and_then(serde_json::Value::as_str),
            Some("invalid"),
            "{text} label should not be an invalid chemical label"
        );
    }
    assert!(!render_document(&document).iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::DocumentDiagnostic,
            stroke: Some(stroke),
            ..
        } if stroke == "#d32f2f"
    )));
}

#[test]
fn parse_cdxml_skips_cached_fragments_inside_placeholder_nodes() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="18" LineWidth="0.6" LabelSize="10">
  <page id="1" BoundingBox="0 0 140 60">
    <fragment id="visible" BoundingBox="0 0 120 50">
      <n id="n1" p="20 25" NodeType="Nickname">
        <fragment id="cached_bn" BoundingBox="-10 10 40 55">
          <n id="c1" p="0 20"/>
          <n id="c2" p="20 20"/>
          <b id="cb1" B="c1" E="c2"/>
        </fragment>
        <t p="20 29" BoundingBox="8 18 20 30" LabelJustification="Right" Justification="Right">
          <s font="3" size="10" face="97">Bn</s>
        </t>
      </n>
      <n id="n2" p="50 25"/>
      <n id="n3" p="80 25" NodeType="Fragment">
        <fragment id="cached_frag" BoundingBox="70 10 110 45">
          <n id="f1" p="80 20"/>
          <n id="f2" p="100 20"/>
          <b id="fb1" B="f1" E="f2"/>
        </fragment>
        <t p="80 29" BoundingBox="80 18 100 30" LabelJustification="Left">
          <s font="3" size="10" face="97">OMe</s>
        </t>
      </n>
      <b id="b1" B="n1" E="n2"/>
      <b id="b2" B="n2" E="n3"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("cached fragments")).expect("cdxml should parse");
    let fragments: Vec<_> = document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .collect();
    assert_eq!(
        fragments.len(),
        1,
        "cached child fragments under placeholder nodes should not import as visible molecules"
    );
    let fragment = fragments[0];
    assert_eq!(fragment.nodes.len(), 3);
    assert_eq!(fragment.bonds.len(), 2);
    assert!(fragment.nodes.iter().any(|node| {
        node.label
            .as_ref()
            .is_some_and(|label| label.source_text.as_deref() == Some("Bn"))
    }));
    assert!(fragment.nodes.iter().any(|node| {
        node.label
            .as_ref()
            .is_some_and(|label| label.source_text.as_deref() == Some("OMe"))
    }));
}

#[test]
fn load_cdxml_document_preserves_imported_acs_drawing_options() {
    let Some(cdxml) = read_optional_cdxml_fixture("db-acs.cdxml") else {
        return;
    };
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(&cdxml)
        .expect("cdxml should load into engine");

    assert!((engine.options().bond_length - 14.4).abs() < 0.05);
    assert!((engine.options().bond_stroke_width - 0.6).abs() < 0.01);
    assert!((engine.options().bold_bond_width - 2.0).abs() < 0.05);
    assert!((engine.options().wedge_width - 3.0).abs() < 0.05);
    assert!((engine.options().hash_spacing - 2.5).abs() < 0.05);
    assert!((engine.options().bond_spacing - 18.0).abs() < 0.05);
    assert!(engine.options().label_clip_margin.abs() < 0.01);
    assert!((engine.options().margin_width - 2.0).abs() < 0.05);
}

#[test]
fn load_cdxml_document_preserves_imported_label_font_size() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" LabelSize="10">
  <page id="p1" BoundingBox="0 0 40 24">
    <fragment id="f1" BoundingBox="0 0 40 24">
      <n id="n1" p="10 12" Element="7">
        <t p="6.40 15.90" BoundingBox="6.40 7.56 13.62 15.90" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">N</s>
        </t>
      </n>
      <n id="n2" p="24 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("cdxml should load into engine");

    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment")
        .fragment;
    let label = fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .and_then(|node| node.label.as_ref())
        .expect("imported N label");
    assert_eq!(label.font_family.as_deref(), Some("Arial"));
    assert_eq!(label.font_size, Some(10.0));
    assert_eq!(label.runs.first().and_then(|run| run.font_size), Some(10.0));

    let session = engine
        .begin_text_edit(Point::new(10.0, 12.0))
        .expect("clicking label should open a text edit session");
    assert_eq!(session.font_family.as_deref(), Some("Arial"));
    assert_eq!(session.font_size, Some(10.0));
    assert_eq!(
        session.source_runs.first().and_then(|run| run.font_size),
        Some(10.0)
    );
}

#[test]
fn load_cdxml_document_derives_wedge_width_from_imported_bold_width_multiplier() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" MarginWidth="1.7" LabelSize="10">
  <page id="p1" BoundingBox="0 0 100 100">
    <fragment id="f1" BoundingBox="10 10 40 20">
      <n id="n1" p="10 15"/>
      <n id="n2" p="24.4 15"/>
      <b id="b1" B="n1" E="n2" Display="WedgeBegin"/>
    </fragment>
  </page>
</CDXML>"#;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("cdxml should load into engine");

    assert!((engine.options().bond_length - 14.4).abs() < 0.05);
    assert!((engine.options().bond_stroke_width - 0.99).abs() < 0.01);
    assert!((engine.options().bold_bond_width - 2.01).abs() < 0.01);
    assert!((engine.options().wedge_width - 3.015).abs() < 0.01);
    assert!(engine.options().label_clip_margin.abs() < 0.01);
    assert!((engine.options().margin_width - 1.7).abs() < 0.01);

    let bond = &engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .bonds[0];
    assert!((bond.wedge_width.unwrap_or_default() - 3.015).abs() < 0.01);
    assert_eq!(bond.label_clip_margin, None);
    assert_eq!(bond.margin_width, None);
}

#[test]
fn load_cdxml_document_does_not_import_margin_width_as_label_retreat() {
    fn imported_label_clip_profile(
        line_width: f64,
        margin_width: f64,
    ) -> (f64, Option<(f64, f64)>) {
        let cdxml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="{line_width:.2}" BoldWidth="2.00" HashSpacing="2.50" BondSpacing="18" MarginWidth="{margin_width:.2}" LabelSize="10">
  <page id="p1" BoundingBox="0 0 50 30">
    <fragment id="f1" BoundingBox="0 0 50 30">
      <n id="n1" p="10 15"/>
      <n id="n2" p="24.4 15" Element="7">
        <t p="20.8 18.9" BoundingBox="20.8 10.56 28.02 18.9" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">N</s>
        </t>
      </n>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#
        );
        let mut engine = Engine::new();
        engine
            .load_cdxml_document(&cdxml)
            .expect("cdxml should load");
        let profile = engine
            .state()
            .document
            .editable_fragment()
            .and_then(|entry| {
                entry
                    .fragment
                    .nodes
                    .iter()
                    .find_map(|node| node.label.as_ref())
                    .and_then(|label| {
                        let meta = label.meta.pointer("/import/cdxml")?;
                        let natural = meta.get("naturalOutsetPt")?.as_f64()?;
                        let radius = meta.get("circleRadiusPt")?.as_f64()?;
                        Some((natural, radius))
                    })
            });
        (engine.options().label_clip_margin, profile)
    }

    let (normal, normal_profile) = imported_label_clip_profile(0.60, 1.60);
    let (wide_line, wide_line_profile) = imported_label_clip_profile(1.80, 1.60);
    let (wide_margin, wide_margin_profile) = imported_label_clip_profile(0.60, 5.00);

    assert!(normal.abs() < 0.01, "{normal}");
    assert_eq!(normal_profile, Some((1.6, 3.2)));
    assert_eq!(wide_line_profile, Some((1.6, 3.2)));
    assert_eq!(wide_margin_profile, Some((5.0, 10.0)));
    assert!(
        (wide_line - normal).abs() < 0.01,
        "CDXML MarginWidth should not mutate the legacy global label clip option: {normal} {wide_line}"
    );
    assert!((wide_margin - normal).abs() < 0.01, "{wide_margin}");
}

#[test]
fn cdxml_imported_bonds_use_engine_glyph_retreat() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" BondSpacing="18" MarginWidth="1.60" LabelSize="10">
  <page id="p1" BoundingBox="0 0 70 30">
    <fragment id="f1" BoundingBox="0 0 70 30">
      <n id="n1" p="10 15"/>
      <n id="n2" p="34.4 15" Element="7">
        <t p="30.8 18.9" BoundingBox="30.8 10.56 38.02 18.9" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">N</s>
        </t>
      </n>
      <b id="b1" B="n1" E="n2" Display="Bold"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("glyph retreat")).expect("cdxml should parse");
    let bond = imported_fragment_bond(&document, "obj_mol_001", "b1");
    assert_eq!(bond.label_clip_margin, None);
    assert_eq!(bond.margin_width, None);

    let polygon = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentBond,
                object_id,
                bond_id,
                points,
                ..
            } if object_id.as_deref() == Some("obj_mol_001")
                && bond_id.as_deref() == Some("b1") =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("bold bond polygon should render");
    let (from, to) = bond_axis_from_points(&polygon).expect("bond axis");
    let label_endpoint = if from.x > to.x { from } else { to };

    let retreat_from_text_origin = 30.8 - label_endpoint.x;
    assert!(
        (0.75..=1.05).contains(&retreat_from_text_origin),
        "imported bond should clip at the source-margin glyph polygon without adding a second MarginWidth retreat: {polygon:?}"
    );
}

#[test]
fn cdxml_imported_f_label_margin_expands_internal_bar_clip() {
    fn imported_f_endpoint_distance(margin_width: f64) -> (f64, f64, usize) {
        let cdxml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" BondSpacing="18" MarginWidth="{margin_width:.2}" LabelSize="10">
  <page id="p1" BoundingBox="0 0 70 40">
    <fragment id="f1" BoundingBox="0 0 70 40">
      <n id="n13" p="37.41 12.17"/>
      <n id="n14" p="24.94 4.97" Element="9" InterpretChemically="yes">
        <t p="21.91 8.87" BoundingBox="21.91 0.67 28.01 9.57" LabelJustification="Left" LabelAlignment="Auto">
          <s font="3" size="10" color="0" face="96">F</s>
        </t>
      </n>
      <b id="b28" B="n13" E="n14"/>
    </fragment>
  </page>
</CDXML>"#
        );
        let document =
            parse_cdxml_document(&cdxml, Some("imported F clip")).expect("cdxml should parse");
        let entry = document.editable_fragment().expect("editable fragment");
        let f_node = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == "n14")
            .expect("F node should import");
        let label = f_node.label.as_ref().expect("F label should import");
        let import_meta = label
            .meta
            .pointer("/import/cdxml")
            .expect("CDXML label import metadata should exist");
        assert_eq!(
            import_meta
                .get("naturalOutsetPt")
                .and_then(|value| value.as_f64()),
            Some(margin_width)
        );
        assert!(
            !label.glyph_polygons.is_empty(),
            "imported F label should carry glyph clip polygons"
        );

        let f_world = Point::new(
            entry.object.transform.translate[0] + f_node.position[0],
            entry.object.transform.translate[1] + f_node.position[1],
        );
        let polygon = render_document(&document)
            .into_iter()
            .find_map(|primitive| match primitive {
                RenderPrimitive::Polygon {
                    role: RenderRole::DocumentBond,
                    object_id,
                    bond_id,
                    points,
                    ..
                } if object_id.as_deref() == Some("obj_mol_001")
                    && bond_id.as_deref() == Some("b28") =>
                {
                    Some(points)
                }
                _ => None,
            })
            .expect("F bond polygon should render");
        let (from, to) = bond_axis_from_points(&polygon).expect("bond axis");
        let label_endpoint = if from.distance(f_world) < to.distance(f_world) {
            from
        } else {
            to
        };
        (
            label_endpoint.distance(f_world),
            import_meta
                .get("marginWidth")
                .and_then(|value| value.as_f64())
                .unwrap_or_default(),
            label.glyph_polygons[0].len(),
        )
    }

    let (one_pt_distance, one_pt_margin, one_pt_points) = imported_f_endpoint_distance(1.0);
    let (two_pt_distance, two_pt_margin, two_pt_points) = imported_f_endpoint_distance(2.0);

    assert_eq!(one_pt_margin, 1.0);
    assert_eq!(two_pt_margin, 2.0);
    assert!(one_pt_points > 8);
    assert!(two_pt_points > 8);
    assert!(
        two_pt_distance > one_pt_distance + 0.45,
        "imported F internal-bar clipping must expand with MarginWidth: {one_pt_distance} -> {two_pt_distance}"
    );
}

#[test]
fn render_document_does_not_join_bold_bond_at_labeled_endpoint() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [56.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [56.0, 45.0],
                    "box": [51.0, 34.0, 61.0, 46.0],
                    "glyphPolygons": [[
                        [51.0, 34.0],
                        [61.0, 34.0],
                        [61.0, 46.0],
                        [51.0, 46.0]
                    ]]
                }
            },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [62.0, 28.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineWeights": { "main": "bold" }
            },
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let polygon = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("bold bond polygon should render");
    let max_x = polygon
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);

    assert!(
        max_x <= 51.0 + 1.0e-6,
        "labeled endpoints should be clipped by glyphs and must not rejoin at the atom point: {polygon:?}"
    );
}

#[test]
fn render_document_retreats_bond_when_label_anchor_lies_on_glyph_boundary() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [50.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [50.0, 45.0],
                    "box": [50.0, 34.0, 60.0, 46.0],
                    "glyphPolygons": [[
                        [50.0, 34.0],
                        [60.0, 34.0],
                        [60.0, 46.0],
                        [50.0, 46.0]
                    ]]
                }
            }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let polygon = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("bond polygon should render");
    let max_x = polygon
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);

    assert!(
        max_x <= 50.0 + 1.0e-6,
        "a bond whose atom anchor lies on a glyph edge should still retreat outside the glyph: {polygon:?}"
    );
}

#[test]
fn render_document_allows_bond_between_close_labels_to_disappear() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "N",
                "atomicNumber": 7,
                "position": [50.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [50.0, 45.0],
                    "box": [45.0, 34.0, 55.0, 46.0],
                    "glyphPolygons": [[
                        [45.0, 34.0],
                        [55.0, 34.0],
                        [55.0, 46.0],
                        [45.0, 46.0]
                    ]]
                }
            },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [60.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [60.0, 45.0],
                    "box": [55.0, 34.0, 65.0, 46.0],
                    "glyphPolygons": [[
                        [55.0, 34.0],
                        [65.0, 34.0],
                        [65.0, 46.0],
                        [55.0, 46.0]
                    ]]
                }
            }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    assert!(
        !object_bond_polygons_with_ids(&render_document(&document))
            .iter()
            .any(|(bond_id, _)| bond_id == "b1"),
        "when label glyph retreats consume the whole segment, the bond should disappear instead of preserving a minimum visible length"
    );
}

#[test]
fn parse_cdxml_imports_assets_molecules_as_native_fragments() {
    let Some(cdxml) = read_optional_cdxml_fixture("assets-acs.cdxml") else {
        return;
    };
    let document = parse_cdxml_document(&cdxml, Some("assets")).expect("cdxml should parse");

    assert!(document
        .objects
        .iter()
        .any(|object| object.object_type == "molecule"));
    let molecule_count = document
        .objects
        .iter()
        .filter(|object| object.object_type == "molecule")
        .count();
    assert!(molecule_count >= 1);
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("import should create molecule fragment resource");
    assert!(fragment.nodes.len() >= 2);
    assert!(!fragment.bonds.is_empty());
    assert!(fragment
        .bonds
        .iter()
        .all(|bond| (bond.stroke_width - 0.6).abs() < 0.001));
    assert!(render_document(&document).iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Polygon {
            role: RenderRole::DocumentBond,
            ..
        }
    )));
}

#[test]
fn parse_cdxml_imports_arrows_shapes_and_text_objects() {
    let Some(arrows) = read_optional_cdxml_fixture("arrows-acs.cdxml") else {
        return;
    };
    let arrow_document =
        parse_cdxml_document(&arrows, Some("arrows")).expect("arrows should parse");
    assert!(arrow_document
        .objects
        .iter()
        .any(|object| object.object_type == "line"
            && object.payload.extra.get("arrowHead").is_some()));
    assert!(render_document(&arrow_document)
        .iter()
        .any(|primitive| matches!(
            primitive,
            RenderPrimitive::Path {
                role: RenderRole::DocumentGraphic,
                ..
            } | RenderPrimitive::Polygon {
                role: RenderRole::DocumentGraphic,
                ..
            }
        )));

    let Some(shapes) = read_optional_cdxml_fixture("shape.cdxml") else {
        return;
    };
    let shape_document = parse_cdxml_document(&shapes, Some("shape")).expect("shape should parse");
    assert!(shape_document
        .objects
        .iter()
        .any(|object| object.object_type == "shape"));
    assert!(render_document(&shape_document)
        .iter()
        .any(|primitive| matches!(
            primitive,
            RenderPrimitive::Path {
                role: RenderRole::DocumentGraphic,
                ..
            }
        )));
}

#[test]
fn parse_cdxml_preserves_shape_style_parameters() {
    let Some(shapes) = read_optional_cdxml_fixture("shape.cdxml") else {
        return;
    };
    let document = parse_cdxml_document(&shapes, Some("shape")).expect("shape should parse");

    let dashed_circle = document
        .objects
        .iter()
        .find(|object| {
            object.object_type == "shape"
                && object
                    .payload
                    .extra
                    .get("kind")
                    .and_then(|value| value.as_str())
                    == Some("circle")
                && object.style_ref.as_ref().is_some_and(|style_ref| {
                    document.styles[style_ref]
                        .get("dashArray")
                        .and_then(|value| value.as_array())
                        .is_some_and(|dash| !dash.is_empty())
                })
        })
        .expect("dashed circle should import");
    let dashed_style = &document.styles[dashed_circle.style_ref.as_ref().unwrap()];
    assert_eq!(
        dashed_style
            .get("strokeWidth")
            .and_then(|value| value.as_f64()),
        Some(0.6)
    );

    let shadowed_rect = document
        .objects
        .iter()
        .find(|object| {
            object.object_type == "shape"
                && object.style_ref.as_ref().is_some_and(|style_ref| {
                    document.styles[style_ref]
                        .get("shadow")
                        .and_then(|value| value.as_bool())
                        == Some(true)
                })
        })
        .expect("shadowed shape should import");
    let shadow_style = &document.styles[shadowed_rect.style_ref.as_ref().unwrap()];
    assert_eq!(
        shadow_style
            .get("shadowSize")
            .and_then(|value| value.as_f64()),
        Some(4.0)
    );
}

#[test]
fn export_cdxml_writes_shape_style_parameters() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 240.0, "background": "#ffffff" }
        },
        "styles": {
            "style_circle": {
                "kind": "shape",
                "fill": null,
                "stroke": "#000000",
                "strokeWidth": 0.6,
                "dashArray": [2.7]
            },
            "style_shadow": {
                "kind": "shape",
                "fill": null,
                "stroke": "#000000",
                "strokeWidth": 0.6,
                "dashArray": [],
                "shadow": true,
                "shadowSize": 4.0
            }
        },
        "objects": [{
            "id": "obj_shape_001",
            "type": "shape",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_circle",
            "payload": {
                "bbox": [40.0, 40.0, 40.0, 40.0],
                "kind": "circle",
                "center": [60.0, 60.0],
                "majorAxisEnd": [80.0, 60.0],
                "minorAxisEnd": [60.0, 80.0]
            }
        }, {
            "id": "obj_shape_002",
            "type": "shape",
            "visible": true,
            "zIndex": 11,
            "transform": { "translate": [100.0, 50.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_shadow",
            "payload": {
                "bbox": [0.0, 0.0, 50.0, 30.0],
                "kind": "rect"
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let cdxml = document_to_cdxml(&document);
    assert!(cdxml.contains("OvalType=\"Circle Dashed\""));
    assert!(cdxml.contains("RectangleType=\"Shadow\""));
    assert!(cdxml.contains("LineWidth=\"0.6\""));
    assert!(cdxml.contains("ShadowSize=\"400\""));
}

#[test]
fn parse_cdxml_preserves_arrow_geometry_modifiers() {
    let Some(assets) = read_optional_cdxml_fixture("assets-acs.cdxml") else {
        return;
    };
    let document = parse_cdxml_document(&assets, Some("assets")).expect("assets should parse");
    assert!(document.objects.iter().any(|object| {
        object.payload.extra.get("arrowHead").is_some_and(|arrow| {
            arrow.get("noGo").and_then(|value| value.as_str()) == Some("cross")
                && arrow.get("length").and_then(|value| value.as_f64()) == Some(22.5)
                && arrow.get("centerLength").and_then(|value| value.as_f64()) == Some(19.69)
                && arrow.get("width").and_then(|value| value.as_f64()) == Some(5.63)
        })
    }));
    assert!(document.objects.iter().any(|object| {
        object.payload.extra.get("arrowHead").is_some_and(|arrow| {
            arrow.get("curve").and_then(|value| value.as_f64()) == Some(-270.0)
                && arrow.get("length").and_then(|value| value.as_f64()) == Some(8.0)
                && arrow.get("width").and_then(|value| value.as_f64()) == Some(2.0)
        })
    }));
    assert!(document.objects.iter().any(|object| {
        object.payload.extra.get("arrowHead").is_some_and(|arrow| {
            arrow.get("head").and_then(|value| value.as_str()) == Some("half-left")
        })
    }));
    assert!(document.objects.iter().any(|object| {
        object
            .payload
            .extra
            .get("arrowGeometry")
            .is_some_and(|geometry| {
                geometry.get("center").is_some()
                    && geometry.get("majorAxisEnd").is_some()
                    && geometry.get("minorAxisEnd").is_some()
            })
    }));
}

#[test]
fn cdxml_arrow_head_dimensions_are_relative_to_line_width() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML LineWidth="0.6" BondLength="14.4" color="0" bgcolor="1">
  <page id="1" BoundingBox="0 0 160 80">
    <arrow id="2" Head3D="128.21 40 0" Tail3D="0 40 0" Z="1"
      FillType="None" ArrowheadType="Solid" ArrowheadHead="Full"
      HeadSize="2250" ArrowheadCenterSize="1969" ArrowheadWidth="563"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("arrow")).expect("cdxml should parse");
    let arrow = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow should import as line object");
    let arrow_head = arrow
        .payload
        .extra
        .get("arrowHead")
        .expect("arrow should keep cdxml arrow payload");
    assert_eq!(
        arrow_head.get("length").and_then(|value| value.as_f64()),
        Some(22.5)
    );
    assert_eq!(
        arrow_head
            .get("centerLength")
            .and_then(|value| value.as_f64()),
        Some(19.69)
    );
    assert_eq!(
        arrow_head.get("width").and_then(|value| value.as_f64()),
        Some(5.63)
    );

    let primitives = render_document(&document);
    let head_points = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some(arrow.id.as_str()) =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("solid arrow head should render as filled path");
    let head_min_x = head_points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let head_max_x = head_points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let head_min_y = head_points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let head_max_y = head_points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((head_max_x - head_min_x - 13.5).abs() <= 0.001);
    assert!((head_max_y - head_min_y - 6.856).abs() <= 0.001);
}

#[test]
fn cdxml_grouped_arrow_keeps_renderable_head_payload() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML LineWidth="0.6" BondLength="14.4" color="0" bgcolor="1">
  <page id="1" BoundingBox="0 0 180 80">
    <group id="group1">
      <graphic id="g1" SupersededBy="a1" GraphicType="Line" ArrowType="FullHead"
        HeadSize="2250" Head3D="150 40 0" Tail3D="20 40 0"/>
      <arrow id="a1" BoundingBox="20 36 150 44" Z="1" FillType="None"
        ArrowheadType="Solid" ArrowheadHead="Full" HeadSize="2250"
        ArrowheadCenterSize="1969" ArrowheadWidth="563"
        Head3D="150 40 0" Tail3D="20 40 0"/>
    </group>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("grouped arrow")).expect("cdxml should parse");
    let primitives = render_document(&document);
    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::FilledPath {
                role: RenderRole::DocumentGraphic,
                object_id,
                ..
            } if object_id.as_deref() == Some("obj_line_001")
        )),
        "grouped arrow should render a filled arrow head: {primitives:?}"
    );
}

#[test]
fn cdxml_arrow_line_width_scales_arrow_head_ratios() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML LineWidth="0.6" BondLength="14.4" color="0" bgcolor="1">
  <page id="1" BoundingBox="0 0 120 40">
    <arrow id="2" Head3D="100 20 0" Tail3D="0 20 0" Z="1"
      LineWidth="1.19" FillType="None" ArrowheadType="Solid" ArrowheadHead="Full"
      HeadSize="800" ArrowheadCenterSize="700" ArrowheadWidth="200"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("wide arrow")).expect("cdxml should parse");
    let arrow = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow should import as line object");
    let style = arrow
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .expect("arrow should use a line-width-specific style");
    assert_eq!(
        style.get("strokeWidth").and_then(|value| value.as_f64()),
        Some(1.19)
    );
    let arrow_head = arrow
        .payload
        .extra
        .get("arrowHead")
        .expect("arrow should keep cdxml arrow payload");
    assert_eq!(
        arrow_head.get("length").and_then(|value| value.as_f64()),
        Some(8.0)
    );
    assert_eq!(
        arrow_head.get("width").and_then(|value| value.as_f64()),
        Some(2.0)
    );

    let primitives = render_document(&document);
    let head_points = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some(arrow.id.as_str()) =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("solid arrow head should render as filled path");
    let head_min_x = head_points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let head_max_x = head_points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let head_min_y = head_points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let head_max_y = head_points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((head_max_x - head_min_x - 9.52).abs() <= 0.001);
    assert!((head_max_y - head_min_y - 4.86).abs() <= 0.001);
}

#[test]
fn cdxml_arrow_type_without_endpoint_does_not_enable_head() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML LineWidth="0.6" BondLength="14.4" color="0" bgcolor="1">
  <page id="1" BoundingBox="0 0 160 80">
    <arrow id="2" Head3D="128.21 40 0" Tail3D="0 40 0" Z="1"
      FillType="None" ArrowheadType="Solid"
      HeadSize="2250" ArrowheadCenterSize="1969" ArrowheadWidth="563"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("arrow")).expect("cdxml should parse");
    let arrow = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow should import as line object");
    let arrow_head = arrow
        .payload
        .extra
        .get("arrowHead")
        .expect("arrow should keep cdxml arrow payload");
    assert_eq!(
        arrow_head.get("head").and_then(|value| value.as_str()),
        Some("none")
    );

    let primitives = render_document(&document);
    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Polyline {
                role: RenderRole::DocumentGraphic,
                object_id,
                ..
            } if object_id.as_deref() == Some(arrow.id.as_str())
        )),
        "arrow element should still render its shaft"
    );
    assert!(
        !primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::FilledPath {
                role: RenderRole::DocumentGraphic,
                object_id,
                ..
            } if object_id.as_deref() == Some(arrow.id.as_str())
        )),
        "ArrowheadType alone describes the head kind, not an enabled endpoint"
    );
}

#[test]
fn cdxml_bold_line_uses_imported_bold_width_without_render_floor() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML LineWidth="0.57" BoldWidth="1.91" BondLength="13.78" color="0" bgcolor="1">
  <page id="1" BoundingBox="0 0 360 80">
    <arrow id="2" Head3D="340 40 0" Tail3D="20 40 0" Z="1"
      LineType="Bold" FillType="None" ArrowheadType="Solid"
      HeadSize="2000" ArrowheadCenterSize="1750" ArrowheadWidth="500"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("bold line")).expect("cdxml should parse");
    let line = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("bold line should import as line object");
    let style = line
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .expect("line should use imported bold-width style");
    assert_eq!(
        style.get("strokeWidth").and_then(|value| value.as_f64()),
        Some(1.91)
    );

    let primitives = render_document(&document);
    let shaft_width = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polyline {
                role,
                object_id,
                stroke_width,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some(line.id.as_str()) =>
            {
                Some(*stroke_width)
            }
            _ => None,
        })
        .expect("bold line shaft should render");
    assert!((shaft_width - 1.91).abs() <= 0.001);
    assert!(
        !primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::FilledPath {
                role: RenderRole::DocumentGraphic,
                object_id,
                ..
            } if object_id.as_deref() == Some(line.id.as_str())
        )),
        "line should not gain an arrowhead without ArrowheadHead or ArrowheadTail"
    );
}

#[test]
fn cdxml_bold_arrow_head_dimensions_scale_with_imported_line_width() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML LineWidth="0.6" BoldWidth="2" BondLength="14.4" color="0" bgcolor="1">
  <page id="1" BoundingBox="0 0 160 80">
    <arrow id="2" Head3D="128.21 40 0" Tail3D="0 40 0" Z="1"
      LineType="Bold" FillType="None" ArrowheadType="Solid" ArrowheadHead="Full"
      HeadSize="4500" ArrowheadCenterSize="3938" ArrowheadWidth="1125"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("bold arrow")).expect("cdxml should parse");
    let arrow = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow should import as line object");
    let primitives = render_document(&document);
    let head_points = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some(arrow.id.as_str()) =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("solid arrow head should render as filled path");
    let head_min_x = head_points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let head_max_x = head_points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let head_min_y = head_points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let head_max_y = head_points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((head_max_x - head_min_x - 90.0).abs() <= 0.001);
    assert!((head_max_y - head_min_y - 45.1).abs() <= 0.001);
}

#[test]
fn cdxml_arrow_head_rendering_does_not_apply_size_floor() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML LineWidth="0.6" BondLength="14.4" color="0" bgcolor="1">
  <page id="1" BoundingBox="0 0 40 20">
    <arrow id="2" Head3D="20 10 0" Tail3D="0 10 0" Z="1"
      FillType="None" ArrowheadType="Solid" ArrowheadHead="Full"
      HeadSize="600" ArrowheadCenterSize="525" ArrowheadWidth="150"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("small arrow")).expect("cdxml should parse");
    let arrow = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow should import as line object");
    let primitives = render_document(&document);
    let head_points = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some(arrow.id.as_str()) =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("solid arrow head should render as filled path");
    let head_min_x = head_points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let head_max_x = head_points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let head_min_y = head_points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let head_max_y = head_points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((head_max_x - head_min_x - 3.6).abs() <= 0.001);
    assert!((head_max_y - head_min_y - 1.9).abs() <= 0.001);
}

#[test]
fn export_cdxml_writes_arrow_geometry_modifiers() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_arrow_default": {
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": 1.0,
                "dashArray": [2.7]
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
                "tail": "none",
                "arrowHead": {
                    "kind": "solid",
                    "curve": -270.0,
                    "length": 8.0,
                    "centerLength": 7.0,
                    "width": 2.0,
                    "head": "half-left",
                    "tail": "none",
                    "fillType": "none",
                    "bold": true,
                    "noGo": "hash"
                },
                "arrowGeometry": {
                    "boundingBox": [10.0, 10.0, 120.0, 40.0],
                    "center": [65.0, 20.0],
                    "majorAxisEnd": [120.0, 20.0],
                    "minorAxisEnd": [65.0, 75.0]
                }
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let cdxml = document_to_cdxml(&document);
    assert!(cdxml.contains("HeadSize=\"800\""));
    assert!(cdxml.contains("ArrowheadCenterSize=\"700\""));
    assert!(cdxml.contains("ArrowheadWidth=\"200\""));
    assert!(cdxml.contains("ArrowheadHead=\"HalfLeft\""));
    assert!(cdxml.contains("AngularSize=\"-270\""));
    assert!(cdxml.contains("NoGo=\"Hash\""));
    assert!(cdxml.contains("LineType=\"Bold Dashed\""));
    assert!(cdxml.contains("FillType=\"None\""));
    assert!(cdxml.contains("Center3D=\"65 20 0\""));
    assert!(cdxml.contains("MajorAxisEnd3D=\"120 20 0\""));
    assert!(cdxml.contains("MinorAxisEnd3D=\"65 75 0\""));
}

#[test]
fn render_document_uses_arrow_geometry_for_elliptic_curved_arrows() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 80.0, "height": 60.0, "background": "#ffffff" }
        },
        "styles": {
            "style_arrow_default": {
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": 1.0
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
                "points": [[20.0, 0.0], [0.0, 10.0]],
                "head": "none",
                "tail": "none",
                "arrowHead": {
                    "kind": "curved",
                    "curve": -90.0,
                    "head": "none",
                    "tail": "none",
                    "length": 1.0,
                    "centerLength": 0.5,
                    "width": 0.2,
                    "bold": false,
                    "noGo": "none"
                },
                "arrowGeometry": {
                    "center": [0.0, 0.0],
                    "majorAxisEnd": [20.0, 0.0],
                    "minorAxisEnd": [0.0, 10.0]
                }
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let path_points = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Path {
                role: RenderRole::DocumentGraphic,
                object_id,
                points,
                ..
            } if object_id.as_deref() == Some("obj_line_001") => Some(points),
            _ => None,
        })
        .expect("elliptic arrow should render as a path");

    assert!(
        path_points
            .iter()
            .any(|point| point.x > 13.0 && point.x < 15.5 && point.y > 6.5 && point.y < 7.8),
        "{path_points:?}"
    );
}

#[test]
fn parse_document_json_fills_default_arrow_geometry_at_import_boundary() {
    let document = parse_document_json(
        &json!({
            "format": { "name": "chemsema", "version": "0.1" },
            "document": {
                "id": "doc_test",
                "title": "test",
                "page": { "width": 120.0, "height": 80.0, "background": "#ffffff" }
            },
            "styles": {
                "style_arrow_default": {
                    "kind": "stroke",
                    "stroke": "#000000",
                    "strokeWidth": 1.0
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
                    "points": [[10.0, 20.0], [90.0, 20.0]],
                    "head": "end",
                    "tail": "none",
                    "arrowHead": {
                        "kind": "curved",
                        "curve": -120.0,
                        "head": "full",
                        "tail": "none",
                        "length": 10.0,
                        "centerLength": 8.75,
                        "width": 2.5
                    }
                }
            }],
            "resources": {}
        })
        .to_string(),
    )
    .expect("document json should parse");

    let object = document
        .objects
        .iter()
        .find(|object| object.id == "obj_line_001")
        .expect("arrow object should exist");
    let geometry = object
        .payload
        .extra
        .get("arrowGeometry")
        .expect("legacy curved arrow should receive default arc geometry");
    let arrow_head = object
        .payload
        .extra
        .get("arrowHead")
        .expect("arrow head should be normalized at import boundary");
    assert_eq!(
        arrow_head.get("kind").and_then(|value| value.as_str()),
        Some("solid")
    );
    assert_eq!(
        arrow_head.get("bold").and_then(|value| value.as_bool()),
        Some(false)
    );
    assert_eq!(
        arrow_head.get("noGo").and_then(|value| value.as_str()),
        Some("none")
    );
    assert!(geometry.get("center").is_some());
    assert!(geometry.get("majorAxisEnd").is_some());
    assert!(geometry.get("minorAxisEnd").is_some());
    assert!(render_document(&document).iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            object_id,
            ..
        } if object_id.as_deref() == Some("obj_line_001")
    )));
}

#[test]
fn parse_document_json_normalizes_text_and_shape_payloads_at_import_boundary() {
    let document = parse_document_json(
        &json!({
            "format": { "name": "chemsema", "version": "0.1" },
            "document": {
                "id": "doc_test",
                "title": "test",
                "page": { "width": 120.0, "height": 80.0, "background": "#ffffff" }
            },
            "objects": [
                {
                    "id": "obj_text_001",
                    "type": "text",
                    "visible": true,
                    "zIndex": 10,
                    "transform": { "translate": [12.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "payload": { "text": "Note" }
                },
                {
                    "id": "obj_shape_001",
                    "type": "shape",
                    "visible": true,
                    "zIndex": 11,
                    "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "payload": {
                        "kind": "ellipse",
                        "bbox": [10.0, 20.0, 40.0, 0.0]
                    }
                }
            ],
            "resources": {}
        })
        .to_string(),
    )
    .expect("document json should parse");

    let text = document
        .objects
        .iter()
        .find(|object| object.id == "obj_text_001")
        .expect("text object should exist");
    assert_eq!(
        text.payload
            .extra
            .get("fontSize")
            .and_then(|value| value.as_f64()),
        Some(10.0)
    );
    assert!(text.payload.extra.get("lineHeight").is_some());
    assert!(text.payload.extra.get("box").is_some());
    assert_eq!(
        text.payload
            .extra
            .get("align")
            .and_then(|value| value.as_str()),
        Some("left")
    );

    let shape = document
        .objects
        .iter()
        .find(|object| object.id == "obj_shape_001")
        .expect("shape object should exist");
    assert_eq!(
        shape.payload.extra.get("center"),
        Some(&json!([30.0, 20.0]))
    );
    assert_eq!(
        shape.payload.extra.get("majorAxisEnd"),
        Some(&json!([50.0, 20.0]))
    );
    assert_eq!(
        shape.payload.extra.get("minorAxisEnd"),
        Some(&json!([30.0, 40.0]))
    );
}

#[test]
fn parse_cdxml_imports_free_text_object() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="10 20" BoundingBox="10 20 80 36" Justification="Left">
      <s font="3" size="12" face="33" color="0">H2O</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("text")).expect("text cdxml should parse");
    assert!(document
        .objects
        .iter()
        .any(|object| object.object_type == "text"));
    assert!(render_document(&document).iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Text {
            role: RenderRole::DocumentText,
            text,
            runs,
            ..
        } if text == "H2O" || runs.iter().any(|run| run.text == "H2O")
    )));
}

#[test]
fn parse_cdxml_text_auto_line_height_uses_chemdraw_import_compatibility() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="1" p="10 20" BoundingBox="10 10 90 45" Justification="Left" LineHeight="auto">
      <s font="3" size="10" face="0" color="0">Plain A
Plain B</s>
    </t>
    <t id="2" p="110 20" BoundingBox="110 10 210 45" Justification="Left" LineHeight="auto">
      <s font="3" size="10" face="97" color="0">H2O A
H2O B</s>
    </t>
    <t id="3" p="10 90" BoundingBox="10 80 110 125" Justification="Left" LineHeight="auto">
      <s font="3" size="10" face="1" color="0">H</s><s font="3" size="10" face="33" color="0">2</s><s font="3" size="10" face="1" color="0">O A
H</s><s font="3" size="10" face="33" color="0">2</s><s font="3" size="10" face="1" color="0">O B</s>
    </t>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("line height")).expect("text cdxml should parse");
    let mut line_heights: Vec<f64> = document
        .objects
        .iter()
        .filter(|object| object.object_type == "text")
        .filter_map(|object| {
            object
                .payload
                .extra
                .get("lineHeight")
                .and_then(|value| value.as_f64())
        })
        .collect();
    line_heights.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    assert_eq!(line_heights, vec![11.5, 13.45, 13.45]);
}

#[test]
fn parse_cdxml_unescapes_text_entities() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="10 20" BoundingBox="10 20 120 52" Justification="Left">
      <s font="3" size="12" face="0" color="0">d.r. &gt; 20:1 &amp; clean</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("entities")).expect("text cdxml should parse");
    let text_object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text should import");

    assert_eq!(
        text_object
            .payload
            .extra
            .get("text")
            .and_then(|value| value.as_str()),
        Some("d.r. > 20:1 & clean")
    );
}

#[test]
fn parse_cdxml_preserves_bracketusage_repeat_count() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <graphic id="2" BoundingBox="20 70 20 10" GraphicType="Bracket" BracketType="Square"/>
    <graphic id="3" BoundingBox="80 10 80 70" GraphicType="Bracket" BracketType="Square">
      <objecttag id="1" Name="bracketusage">
        <t p="0 0" BoundingBox="0 -6.30 4.17 0"><s font="3" size="7.5" color="0">2</s></t>
      </objecttag>
      <objecttag id="2" Name="parameterizedBracketLabel" Visible="no">
        <t p="84 74" BoundingBox="84 68 102 74" Visible="no"><s font="3" size="7.5" color="0">abc</s></t>
      </objecttag>
    </graphic>
    <bracketedgroup id="4" BracketUsage="MultipleGroup" RepeatCount="2">
      <bracketattachment id="5" GraphicID="2"/>
      <bracketattachment id="6" GraphicID="3"/>
    </bracketedgroup>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("bracket text")).expect("cdxml should parse");
    let bracket_group = document
        .scene_objects()
        .into_iter()
        .find(|object| object_is_bracket_group(object))
        .expect("paired bracket should import");
    assert_eq!(
        bracket_group
            .meta
            .get("repeatCount")
            .and_then(|value| value.as_u64()),
        Some(2)
    );
    let sides: Vec<_> = bracket_group
        .children
        .iter()
        .filter(|object| object.object_type == "bracket")
        .collect();
    assert_eq!(sides.len(), 2, "paired bracket should import as two sides");
    assert!(sides.iter().any(|object| {
        object
            .payload
            .extra
            .get("side")
            .and_then(|value| value.as_str())
            == Some("left")
    }));
    assert!(sides.iter().any(|object| {
        object
            .payload
            .extra
            .get("side")
            .and_then(|value| value.as_str())
            == Some("right")
    }));

    let text_objects: Vec<_> = document
        .objects
        .iter()
        .filter(|object| object.object_type == "text")
        .collect();
    let texts: Vec<_> = text_objects
        .iter()
        .filter_map(|object| {
            object
                .payload
                .extra
                .get("text")
                .and_then(|value| value.as_str())
        })
        .collect();
    assert_eq!(texts, vec!["abc"]);
    let roles: Vec<_> = text_objects
        .iter()
        .filter_map(|object| object.meta.get("role").and_then(|value| value.as_str()))
        .collect();
    assert_eq!(roles, vec!["parameterized_bracket_label"]);
}

#[test]
fn load_cdxml_dragging_unselected_bracket_side_does_not_move_other_side() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="20" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <graphic id="g1" BoundingBox="40 90 40 20" GraphicType="Bracket" BracketType="Square"/>
    <graphic id="g2" BoundingBox="150 20 150 90" GraphicType="Bracket" BracketType="Square"/>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("bracket pair cdxml should load");
    engine.set_tool_state(select_tool_state());

    let group = engine
        .state()
        .document
        .scene_objects()
        .into_iter()
        .find(|object| object_is_bracket_group(object))
        .expect("paired cdxml brackets should import as a bracket group");
    let left_id = group
        .children
        .iter()
        .find(|object| {
            object
                .payload
                .extra
                .get("side")
                .and_then(|value| value.as_str())
                == Some("left")
        })
        .map(|object| object.id.clone())
        .expect("left side should import");
    let right_id = group
        .children
        .iter()
        .find(|object| {
            object
                .payload
                .extra
                .get("side")
                .and_then(|value| value.as_str())
                == Some("right")
        })
        .map(|object| object.id.clone())
        .expect("right side should import");

    let left_before = engine
        .state()
        .document
        .find_scene_object(&left_id)
        .expect("left side should remain")
        .clone();
    let right_before = engine
        .state()
        .document
        .find_scene_object(&right_id)
        .expect("right side should remain")
        .clone();
    let left_height = left_before.payload.bbox.expect("left side bbox")[3];
    let start = Point::new(
        left_before.transform.translate[0] + 0.5,
        left_before.transform.translate[1] + left_height * 0.5,
    );
    let end = Point::new(start.x + 12.0, start.y + 6.0);

    assert!(engine.begin_selection_move_at_point(start, false, false));
    assert_eq!(
        engine.state().selection.arrow_objects,
        vec![left_id.clone()]
    );
    assert!(engine.update_selection_move(end, false));
    assert!(engine.finish_selection_move(end, false));

    let left_after = engine
        .state()
        .document
        .find_scene_object(&left_id)
        .expect("left side should remain");
    let right_after = engine
        .state()
        .document
        .find_scene_object(&right_id)
        .expect("right side should remain");
    assert_eq!(
        left_after.transform.translate,
        [
            round_to_2(left_before.transform.translate[0] + 12.0),
            round_to_2(left_before.transform.translate[1] + 6.0)
        ]
    );
    assert_eq!(
        right_after.transform.translate,
        right_before.transform.translate
    );
    assert_eq!(engine.state().selection.arrow_objects, vec![left_id]);
}

#[test]
fn load_cdxml_bracketusage_repeat_count_feeds_selection_summary() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="20" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <fragment id="f1" BoundingBox="0 -10 60 10">
      <n id="n1" p="0 0"/>
      <n id="n2" p="20 0"/>
      <n id="n3" p="40 0"/>
      <n id="n4" p="60 0"/>
      <b id="b1" B="n1" E="n2" Order="1"/>
      <b id="b2" B="n2" E="n3" Order="1"/>
      <b id="b3" B="n3" E="n4" Order="1"/>
    </fragment>
    <graphic id="g1" BoundingBox="15 10 15 -10" GraphicType="Bracket" BracketType="Square"/>
    <graphic id="g2" BoundingBox="45 -10 45 10" GraphicType="Bracket" BracketType="Square">
      <objecttag id="ot1" Name="bracketusage">
        <t p="0 0" BoundingBox="0 -6.30 4.17 0"><s font="3" size="7.5" color="0">3</s></t>
      </objecttag>
    </graphic>
    <bracketedgroup id="bg1" BracketUsage="MultipleGroup" RepeatCount="3">
      <bracketattachment id="ba1" GraphicID="g1"/>
      <bracketattachment id="ba2" GraphicID="g2"/>
    </bracketedgroup>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("bracketed cdxml should load");

    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment;
    let units = fragment
        .meta
        .get("repeatingUnits")
        .and_then(|value| value.as_array())
        .expect("repeat count should produce a repeating unit");
    assert_eq!(units.len(), 1);
    assert_eq!(units[0]["repeatCount"]["value"], 3);
    assert!(units[0]["countTextObjectId"].is_null());

    assert!(engine.select_all());
    let summary: serde_json::Value =
        serde_json::from_str(&engine.selection_chemistry_summary_json()).unwrap();
    assert_eq!(summary["formula"], "C8H18");
    assert_eq!(summary["atomCount"], 26);
    assert!((summary["formulaWeight"].as_f64().unwrap() - 114.232).abs() < 1.0e-9);
    assert!((summary["exactMass"].as_f64().unwrap() - 114.140_850_580_14).abs() < 1.0e-9);
}

#[test]
fn parse_cdxml_bracket_label_fixtures_match_chemdraw_offsets() {
    for fixture in [
        "manual/desktop/kuohao.cdxml",
        "manual/desktop/kuohao-acs.cdxml",
    ] {
        let cdxml = read_cdxml_fixture(fixture);
        let document = parse_cdxml_document(&cdxml, Some(fixture)).expect("fixture should parse");
        let bracket_groups: Vec<_> = document
            .scene_objects()
            .into_iter()
            .filter(|object| object_is_bracket_group(object))
            .collect();
        let labels: Vec<_> = document
            .scene_objects()
            .into_iter()
            .filter(|object| object.object_type == "text")
            .filter(|object| {
                object
                    .payload
                    .extra
                    .get("text")
                    .and_then(|value| value.as_str())
                    == Some("apple")
            })
            .collect();
        assert_eq!(
            bracket_groups.len(),
            3,
            "{fixture} should import three bracket pairs"
        );
        assert_eq!(
            labels.len(),
            3,
            "{fixture} should import three bracket labels"
        );

        for label in labels {
            let style = document
                .styles
                .get(
                    label
                        .style_ref
                        .as_ref()
                        .expect("label should have text style"),
                )
                .expect("label style should exist");
            assert_eq!(
                style.get("fontSize").and_then(|value| value.as_f64()),
                Some(7.5)
            );
            assert_eq!(
                label
                    .payload
                    .extra
                    .get("fontSize")
                    .and_then(|value| value.as_f64()),
                Some(7.5)
            );

            let label_anchor_y = label.transform.translate[1]
                + label
                    .payload
                    .extra
                    .get("baselineOffset")
                    .and_then(|value| value.as_f64())
                    .expect("label should keep CDXML baseline offset");
            let mut closest = None::<(f64, f64, f64)>;
            for bracket in &bracket_groups {
                let bbox = bracket.payload.bbox.expect("bracket should have bbox");
                let right = bracket.transform.translate[0] + bbox[0] + bbox[2];
                let bottom = bracket.transform.translate[1] + bbox[1] + bbox[3];
                let dx = label.transform.translate[0] - right;
                let dy = label_anchor_y - bottom;
                let score = (dx - 3.12).abs() + (dy - 2.4).abs();
                closest = match closest {
                    Some((best_score, _, _)) if best_score <= score => closest,
                    _ => Some((score, dx, dy)),
                };
            }
            let (_, dx, dy) = closest.expect("label should match a right bracket");
            assert!(
                (dx - 3.12).abs() < 0.02,
                "{fixture} bracket label x offset should be 3.12 pt, got {dx}"
            );
            assert!(
                (2.30..=2.50).contains(&dy),
                "{fixture} bracket label baseline y offset should be about 2.4 pt, got {dy}"
            );
        }
    }
}

#[test]
fn parse_cdxml_preserves_small_text_object_bbox() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="10 20" BoundingBox="10 20 18 28" Justification="Left" UTF8Text="x">
      <s font="3" size="6" face="0" color="0">x</s>
    </t>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("small text")).expect("text cdxml should parse");
    let text_object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text should import");
    let bbox: [f64; 4] = serde_json::from_value(
        text_object
            .payload
            .extra
            .get("box")
            .cloned()
            .expect("text object should preserve box"),
    )
    .expect("text box should deserialize");

    assert_eq!(bbox, [0.0, 0.0, 8.0, 8.0]);
}

#[test]
fn parse_cdxml_preserves_aligned_text_object_source_bbox() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="50 20" BoundingBox="30 10 70 30" Justification="Center" UTF8Text="center">
      <s font="3" size="10" face="0" color="0">center</s>
    </t>
    <t id="3" p="120 20" BoundingBox="80 10 120 30" Justification="Right" UTF8Text="right">
      <s font="3" size="10" face="0" color="0">right</s>
    </t>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("aligned text")).expect("text cdxml should parse");
    let text_objects: Vec<_> = document
        .scene_objects()
        .into_iter()
        .filter(|object| object.object_type == "text")
        .collect();

    let center = text_objects
        .iter()
        .find(|object| {
            object
                .payload
                .extra
                .get("text")
                .and_then(serde_json::Value::as_str)
                == Some("center")
        })
        .expect("center text should import");
    let center_box: [f64; 4] = serde_json::from_value(
        center
            .payload
            .extra
            .get("box")
            .cloned()
            .expect("center text should preserve box"),
    )
    .expect("center box should deserialize");
    assert_eq!(center.transform.translate, [50.0, 10.0]);
    assert_eq!(center_box, [-20.0, 0.0, 40.0, 20.0]);

    let right = text_objects
        .iter()
        .find(|object| {
            object
                .payload
                .extra
                .get("text")
                .and_then(serde_json::Value::as_str)
                == Some("right")
        })
        .expect("right text should import");
    let right_box: [f64; 4] = serde_json::from_value(
        right
            .payload
            .extra
            .get("box")
            .cloned()
            .expect("right text should preserve box"),
    )
    .expect("right box should deserialize");
    assert_eq!(right.transform.translate, [120.0, 10.0]);
    assert_eq!(right_box, [-40.0, 0.0, 40.0, 20.0]);
}

#[test]
fn load_cdxml_document_hit_tests_aligned_text_object_source_bbox() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="50 20" BoundingBox="30 10 70 30" Justification="Center" UTF8Text="center">
      <s font="3" size="10" face="0" color="0">center</s>
    </t>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("cdxml should load into engine");

    engine.select_at_point(
        Point::new(31.0 * CDXML_EDIT_SCALE, 20.0 * CDXML_EDIT_SCALE),
        false,
    );
    assert_eq!(engine.state().selection.text_objects, vec!["obj_text_001"]);

    engine.select_at_point(
        Point::new(71.0 * CDXML_EDIT_SCALE, 20.0 * CDXML_EDIT_SCALE),
        false,
    );
    assert!(engine.state().selection.text_objects.is_empty());
}

#[test]
fn parse_document_json_migrates_legacy_aligned_text_object_box() {
    let document = parse_document_json(
        &json!({
            "format": { "name": "chemsema", "version": "0.1" },
            "document": {
                "id": "doc_test",
                "title": "test",
                "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
            },
            "objects": [{
                "id": "obj_text_001",
                "type": "text",
                "transform": { "translate": [50.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": {
                    "text": "center",
                    "align": "center",
                    "box": [0.0, 0.0, 40.0, 20.0]
                }
            }]
        })
        .to_string(),
    )
    .expect("document json should parse");
    let text_object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should exist");
    let box_value: [f64; 4] = serde_json::from_value(
        text_object
            .payload
            .extra
            .get("box")
            .cloned()
            .expect("text object should preserve migrated box"),
    )
    .expect("text box should deserialize");

    assert_eq!(box_value, [-20.0, 0.0, 40.0, 20.0]);
}

#[test]
fn parse_cdxml_formula_face_expands_digits_to_subscript() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="10 20" BoundingBox="10 20 80 36" Justification="Left" UTF8Text="CF3">
      <s font="3" size="12" face="97" color="0">CF3</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("formula")).expect("text cdxml should parse");
    let text_object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("formula text should import");
    let runs: Vec<chemsema_engine::LabelRun> = serde_json::from_value(
        text_object
            .payload
            .extra
            .get("runs")
            .cloned()
            .expect("imported text should preserve runs"),
    )
    .expect("runs should deserialize");

    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].text, "CF");
    assert_eq!(runs[0].font_weight, Some(700));
    assert_eq!(runs[0].script.as_deref(), Some("normal"));
    assert_eq!(runs[1].text, "3");
    assert_eq!(runs[1].font_weight, Some(700));
    assert_eq!(runs[1].script.as_deref(), Some("subscript"));
}

#[test]
fn parse_cdxml_chemical_face_subscripts_group_multipliers() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="10 20" BoundingBox="10 20 120 36" Justification="Left" UTF8Text="ArB(OH)2">
      <s font="3" size="10" face="96" color="0">ArB(OH)2</s>
    </t>
    <t id="3" p="10 40" BoundingBox="10 40 160 56" Justification="Left" UTF8Text="Cu(CH3CN)4PF6">
      <s font="3" size="10" face="96" color="0">Cu(CH3CN)4PF6</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("formula")).expect("text cdxml should parse");
    let text_runs = |text: &str| -> Vec<chemsema_engine::LabelRun> {
        let object = document
            .objects
            .iter()
            .find(|object| {
                object
                    .payload
                    .extra
                    .get("text")
                    .and_then(serde_json::Value::as_str)
                    == Some(text)
            })
            .expect("formula text should import");
        serde_json::from_value(
            object
                .payload
                .extra
                .get("runs")
                .cloned()
                .expect("imported text should preserve runs"),
        )
        .expect("runs should deserialize")
    };

    let ar_boron = text_runs("ArB(OH)2");
    assert_eq!(ar_boron.last().map(|run| run.text.as_str()), Some("2"));
    assert_eq!(
        ar_boron.last().and_then(|run| run.script.as_deref()),
        Some("subscript")
    );

    let copper = text_runs("Cu(CH3CN)4PF6");
    let subscript_text: Vec<_> = copper
        .iter()
        .filter(|run| run.script.as_deref() == Some("subscript"))
        .map(|run| run.text.as_str())
        .collect();
    assert_eq!(subscript_text, vec!["3", "4", "6"]);
}

#[test]
fn parse_cdxml_decodes_face_bit_combinations() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="10 20" BoundingBox="10 20 120 36" Justification="Left" UTF8Text="A2B+NO2">
      <s font="3" size="12" face="39" color="0">A2</s>
      <s font="3" size="12" face="70" color="0">B+</s>
      <s font="3" size="12" face="103" color="0">NO2</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("faces")).expect("text cdxml should parse");
    let text_object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text should import");
    let runs: Vec<chemsema_engine::LabelRun> = serde_json::from_value(
        text_object
            .payload
            .extra
            .get("runs")
            .cloned()
            .expect("imported text should preserve runs"),
    )
    .expect("runs should deserialize");

    assert_eq!(runs[0].text, "A2");
    assert_eq!(runs[0].font_weight, Some(700));
    assert_eq!(runs[0].font_style.as_deref(), Some("italic"));
    assert_eq!(runs[0].underline, Some(true));
    assert_eq!(runs[0].script.as_deref(), Some("subscript"));

    assert_eq!(runs[1].text, "B+");
    assert_eq!(runs[1].font_weight, Some(400));
    assert_eq!(runs[1].font_style.as_deref(), Some("italic"));
    assert_eq!(runs[1].underline, Some(true));
    assert_eq!(runs[1].script.as_deref(), Some("superscript"));

    assert_eq!(
        runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(),
        vec!["A2", "B+", "NO", "2"]
    );
    assert_eq!(runs[2].font_weight, Some(700));
    assert_eq!(runs[2].font_style.as_deref(), Some("italic"));
    assert_eq!(runs[2].underline, Some(true));
    assert_eq!(runs[2].script.as_deref(), Some("normal"));
    assert_eq!(runs[3].font_weight, Some(700));
    assert_eq!(runs[3].font_style.as_deref(), Some("italic"));
    assert_eq!(runs[3].underline, Some(true));
    assert_eq!(runs[3].script.as_deref(), Some("subscript"));
}

#[test]
fn parse_cdxml_imports_table_lines_and_text_boxes() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <graphic id="2" GraphicType="Line" LineType="Dashed" Head3D="10 10 0" Tail3D="80 10 0"/>
    <t id="3" p="12 14" BoundingBox="12 14 60 30" Justification="Left">
      <s font="3" size="10" color="0">entry</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("table")).expect("table cdxml should parse");
    let line = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("plain table line should import");
    assert!(line.payload.extra.get("arrowHead").is_none());
    let line_style = document
        .styles
        .get(line.style_ref.as_deref().expect("line style ref"))
        .expect("line style should exist");
    assert_eq!(line_style["dashArray"], json!([2.5]));
    assert!(document
        .objects
        .iter()
        .any(|object| object.object_type == "text"));
    let primitives = render_document(&document);
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Polyline {
            role: RenderRole::DocumentGraphic,
            dash_array,
            ..
        } if !dash_array.is_empty()
    )));
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Text {
            role: RenderRole::DocumentText,
            text,
            runs,
            ..
        } if text == "entry" || runs.iter().any(|run| run.text == "entry")
    )));
}

#[test]
fn parse_cdxml_renders_acs_dashed_bond_patterns_like_chemdraw() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" BondSpacing="18" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <fragment id="2" BoundingBox="80 35 116 45">
      <n id="3" p="80 40"/>
      <n id="4" p="116 40"/>
      <b id="5" B="3" E="4" Display="Dash"/>
    </fragment>
    <fragment id="6" BoundingBox="80 65 116 80">
      <n id="7" p="80 70"/>
      <n id="8" p="116 70"/>
      <b id="9" B="7" E="8" Order="2" Display2="Dash"/>
    </fragment>
    <fragment id="10" BoundingBox="80 95 116 110">
      <n id="11" p="80 100"/>
      <n id="12" p="116 100"/>
      <b id="13" B="11" E="12" Order="2" Display="Dash" Display2="Dash"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("acs dash patterns")).expect("parse cdxml");
    let primitives = render_document(&document);

    assert_eq!(
        document_bond_polygon_count_for_object(&primitives, "obj_mol_001"),
        8
    );
    assert_eq!(
        document_knockout_count_for_object(&primitives, "obj_mol_001"),
        0
    );
    assert_eq!(
        document_bond_polygon_count_for_object(&primitives, "obj_mol_002"),
        9
    );
    assert_eq!(
        document_knockout_count_for_object(&primitives, "obj_mol_002"),
        0
    );
    assert_eq!(
        document_bond_polygon_count_for_object(&primitives, "obj_mol_003"),
        16
    );
    assert_eq!(
        document_knockout_count_for_object(&primitives, "obj_mol_003"),
        0
    );

    let single_segments = document_bond_axis_intervals_for_object(&primitives, "obj_mol_001");
    assert_eq!(single_segments.len(), 8, "{single_segments:?}");
    assert!(
        (single_segments[0].0 - 0.0).abs() < 0.01 && (single_segments[0].1 - 2.4).abs() < 0.01,
        "{single_segments:?}"
    );
    assert!(
        (single_segments[1].0 - 4.8).abs() < 0.01 && (single_segments[1].1 - 7.2).abs() < 0.01,
        "{single_segments:?}"
    );
    assert!(
        (single_segments[7].0 - 33.6).abs() < 0.01 && (single_segments[7].1 - 36.0).abs() < 0.01,
        "{single_segments:?}"
    );
    let solid_dash_lengths = document_bond_axis_lengths_for_object(&primitives, "obj_mol_002");
    assert!(
        solid_dash_lengths
            .iter()
            .filter(|length| (**length - 2.4).abs() < 0.01)
            .count()
            == 8
            && solid_dash_lengths.iter().any(|length| *length > 35.0),
        "{solid_dash_lengths:?}"
    );
    let double_dash_lengths = document_bond_axis_lengths_for_object(&primitives, "obj_mol_003");
    assert!(
        double_dash_lengths
            .iter()
            .all(|length| (*length - 2.4).abs() < 0.01),
        "{double_dash_lengths:?}"
    );
}

#[test]
fn parse_cdxml_imports_published_formula_face_node_labels_with_subscripts() {
    let Some(cdxml) = read_optional_cdxml_fixture("figure2.cdxml") else {
        return;
    };
    let document =
        parse_cdxml_document(&cdxml, Some("figure2")).expect("published cdxml should parse");
    let cf3_label = document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .flat_map(|fragment| fragment.nodes.iter())
        .filter_map(|node| node.label.as_ref())
        .find(|label| label.source_text.as_deref() == Some("CF3"))
        .expect("example should import CF3 node label");

    assert_eq!(cf3_label.text, "CF3");
    assert_eq!(
        cf3_label
            .runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>(),
        vec!["CF", "3"]
    );
    assert_eq!(cf3_label.runs[1].script.as_deref(), Some("subscript"));
    assert_eq!(
        cf3_label
            .meta
            .pointer("/sourceRuns/0/text")
            .and_then(serde_json::Value::as_str),
        Some("CF3")
    );
}

#[test]
fn load_cdxml_document_uses_internal_single_character_below_label_position() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" LabelSize="10" color="0" bgcolor="1">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
  </colortable>
  <fonttable>
    <font id="3" charset="iso-8859-1" name="Arial"/>
  </fonttable>
  <page id="p1" BoundingBox="238.76 122.79 310.06 156.07">
    <fragment id="f1" BoundingBox="238.76 122.79 310.06 156.07">
      <n id="n1" p="256.05 139.70"/>
      <n id="n2" p="270.45 139.70" NodeType="Fragment">
        <t id="t1" p="268.70 143.60" BoundingBox="268.70 138.06 272.20 143.60" LabelJustification="Left" LabelAlignment="Below" UTF8Text="•">
          <s font="3" size="10" color="0" face="96">•</s>
        </t>
      </n>
      <n id="n3" p="284.85 139.70"/>
      <b id="b1" B="n1" E="n2" Order="2"/>
      <b id="b2" B="n2" E="n3" Order="2"/>
    </fragment>
  </page>
</CDXML>"#;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("cdxml should load into engine");

    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("imported fragment should be editable");
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| {
            node.label
                .as_ref()
                .is_some_and(|label| label.source_text.as_deref() == Some("•"))
        })
        .expect("bullet label node should import");
    let label = node.label.as_ref().expect("node should have a label");
    let position = label.position.expect("bullet label should keep position");
    let world_position = [
        entry.object.transform.translate[0] + position[0],
        entry.object.transform.translate[1] + position[1],
    ];

    assert_eq!(node.element, "C");
    assert_eq!(node.atomic_number, 6);
    assert!(
        !node.is_placeholder,
        "the CDXML bullet is a visible carbon atom, not an invalid text placeholder"
    );
    assert!(
        node.meta.get("labelRecognition").is_none(),
        "the CDXML bullet carbon should not be marked as an invalid functional label"
    );
    assert_eq!(label.text, "•");
    assert!(
        label.meta.get("labelRecognition").is_none(),
        "the CDXML bullet carbon label should not carry invalid-label metadata"
    );
    assert_eq!(
        label
            .meta
            .pointer("/import/cdxml/labelAlignment")
            .and_then(serde_json::Value::as_str),
        Some("Below")
    );
    assert!(
        (world_position[0] - 268.72).abs() < 0.01,
        "single-character CDXML labels should use internal below-label x, got {world_position:?}"
    );
    assert!(
        (world_position[1] - 143.60).abs() < 0.01,
        "single-character CDXML labels should use internal below-label y, got {world_position:?}"
    );
}

#[test]
fn parse_cdxml_keeps_numeric_suffix_node_label_anchored_on_letter() {
    fn labeled_nodes(
        document: &ChemSemaDocument,
    ) -> Vec<(&chemsema_engine::Node, &chemsema_engine::NodeLabel)> {
        document
            .resources
            .values()
            .filter_map(|resource| resource.data.as_fragment())
            .flat_map(|fragment| fragment.nodes.iter())
            .filter_map(|node| node.label.as_ref().map(|label| (node, label)))
            .collect()
    }

    fn anchor_of(label: &chemsema_engine::NodeLabel, index: usize) -> Point {
        let polygon = label
            .glyph_polygons
            .get(index)
            .expect("glyph polygon should exist");
        let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
        let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for [x, y] in polygon {
            min_x = min_x.min(*x);
            min_y = min_y.min(*y);
            max_x = max_x.max(*x);
            max_y = max_y.max(*y);
        }
        Point::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5)
    }

    let invalid_cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" LabelSize="10">
  <page id="p1" BoundingBox="0 0 80 40">
    <fragment id="f1" BoundingBox="0 0 40 20">
      <n id="n1" p="10 10" NodeType="Nickname">
        <t id="t1" p="10 14" BoundingBox="4 4 16 16" UTF8Text="X3">
          <s font="3" size="10" face="96">X3</s>
        </t>
      </n>
      <n id="n2" p="24.4 10"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let invalid_imported =
        parse_cdxml_document(invalid_cdxml, Some("invalid")).expect("invalid label cdxml");
    let (invalid_node, invalid_label) = labeled_nodes(&invalid_imported)
        .into_iter()
        .find(|(_, label)| label.source_text.as_deref() == Some("X3"))
        .expect("invalid X3 label should import");
    assert_eq!(
        invalid_label
            .meta
            .get("labelRecognition")
            .and_then(|meta| meta.get("status"))
            .and_then(serde_json::Value::as_str),
        Some("invalid")
    );
    let invalid_anchor = anchor_of(invalid_label, 0);
    let invalid_line_anchor_y = invalid_label.position.expect("invalid label baseline")[1]
        - invalid_label.font_size.unwrap_or(10.0) * 0.39;
    assert!(
        (invalid_anchor.x - invalid_node.position[0]).abs() < 0.01
            && (invalid_line_anchor_y - invalid_node.position[1]).abs() < 0.01,
        "invalid labels should prefer non-script glyph x anchors and label-line y anchors over subscript/superscript glyphs: node={invalid_node:?}, label={invalid_label:?}"
    );
}

#[test]
fn parse_cdxml_uses_chemdraw_color_table_offset() {
    let Some(cdxml) = read_optional_cdxml_fixture("color.cdxml") else {
        return;
    };
    let document = parse_cdxml_document(&cdxml, Some("color")).expect("color cdxml should parse");

    let shape_fills = cdxml_shape_fills_by_z(&document);
    assert_eq!(
        shape_fills,
        vec![
            "#000000", "#ff0000", "#ffff00", "#00ff00", "#ffffff", "#00ffff", "#0000ff", "#ff00ff",
            "#804040", "#008000", "#0000a0", "#808080",
        ]
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("color=\"4\""), "{exported}");
    assert!(
        exported.contains("<color r=\"1\" g=\"0\" b=\"0\"/>"),
        "{exported}"
    );

    let reimported =
        parse_cdxml_document(&exported, Some("color export")).expect("export should parse");
    assert_eq!(cdxml_shape_fills_by_z(&reimported), shape_fills);
}

#[test]
fn cdxml_electron_symbol_uses_chemdraw_top_anchor_and_color() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" color="0" bgcolor="1">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
    <color r="1" g="0" b="0"/>
  </colortable>
  <page id="1">
    <graphic id="2" BoundingBox="285.19 130.29 285.19 141.94" Z="1" color="4" GraphicType="Symbol" SymbolType="Electron"/>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("electron symbol cdxml should load");
    let document = &engine.state().document;
    let symbol = document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("electron symbol should import");
    assert_eq!(
        symbol
            .payload
            .extra
            .get("kind")
            .and_then(|value| value.as_str()),
        Some("electron")
    );
    assert_eq!(
        symbol
            .payload
            .extra
            .get("fill")
            .and_then(|value| value.as_str()),
        Some("#ff0000")
    );
    let [_, _, width, height] = symbol.payload.bbox.expect("symbol should have bbox");
    let center = [
        symbol.transform.translate[0] + width * 0.5,
        symbol.transform.translate[1] + height * 0.5,
    ];
    let expected_diameter = 11.65 * 2.0 / 9.0;
    assert!(
        (width - expected_diameter).abs() < 0.01,
        "electron diameter should follow ChemDraw's anchor height ratio, got width={width}"
    );
    assert!(
        (height - expected_diameter).abs() < 0.01,
        "electron diameter should follow ChemDraw's anchor height ratio, got height={height}"
    );
    assert!(
        (center[0] - 285.19).abs() < 0.01,
        "electron center x should use the CDXML anchor x, got {center:?}"
    );
    assert!(
        (center[1] - 130.29).abs() < 0.01,
        "electron center y should use the top of the CDXML anchor bbox, got {center:?}"
    );
    let rendered_diameter = render_document(document)
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                object_id, d, fill, ..
            } if object_id.as_deref() == Some(symbol.id.as_str()) && fill == "#ff0000" => {
                horizontal_path_span_at_y(d, center[1])
            }
            _ => None,
        })
        .expect("electron should render as a filled path");
    assert!(
        (rendered_diameter - expected_diameter).abs() < 0.01,
        "rendered electron diameter should match imported geometry, got {rendered_diameter}"
    );

    let exported = document_to_cdxml(document);
    assert!(exported.contains("SymbolType=\"Electron\""), "{exported}");
    assert!(exported.contains("color=\"4\""), "{exported}");
    let reimported =
        parse_cdxml_document(&exported, Some("electron export")).expect("export should parse");
    let reimported_symbol = reimported
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("exported electron should reimport");
    let [_, _, re_width, re_height] = reimported_symbol
        .payload
        .bbox
        .expect("reimported symbol should have bbox");
    assert!((re_width - expected_diameter).abs() < 0.01, "{re_width}");
    assert!((re_height - expected_diameter).abs() < 0.01, "{re_height}");
    let re_center = [
        reimported_symbol.transform.translate[0] + re_width * 0.5,
        reimported_symbol.transform.translate[1] + re_height * 0.5,
    ];
    assert!((re_center[0] - 285.19).abs() < 0.01, "{re_center:?}");
    assert!((re_center[1] - 130.29).abs() < 0.01, "{re_center:?}");
}

#[test]
fn cdxml_charge_symbols_use_first_bbox_point_as_center() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" color="0" bgcolor="1">
  <page id="1">
    <graphic id="2" BoundingBox="94.25 102.47 86.75 102.47" Z="1" GraphicType="Symbol" SymbolType="Minus"/>
    <graphic id="3" BoundingBox="97.99 113.04 90.49 113.04" Z="2" GraphicType="Symbol" SymbolType="Plus"/>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("charge symbol cdxml should load");
    let document = &engine.state().document;
    assert_symbol_center(document, "minus", [94.25, 102.47]);
    assert_symbol_center(document, "plus", [97.99, 113.04]);

    let exported = document_to_cdxml(document);
    let reimported =
        parse_cdxml_document(&exported, Some("charge symbol export")).expect("export should parse");
    assert_symbol_center(&reimported, "minus", [94.25, 102.47]);
    assert_symbol_center(&reimported, "plus", [97.99, 113.04]);
}

fn assert_symbol_center(document: &ChemSemaDocument, kind: &str, expected: [f64; 2]) {
    let symbol = document
        .objects
        .iter()
        .find(|object| {
            object.object_type == "symbol"
                && object
                    .payload
                    .extra
                    .get("kind")
                    .and_then(|value| value.as_str())
                    == Some(kind)
        })
        .unwrap_or_else(|| panic!("{kind} symbol should import"));
    let [_, _, width, height] = symbol.payload.bbox.expect("symbol should have bbox");
    let center = [
        symbol.transform.translate[0] + width * 0.5,
        symbol.transform.translate[1] + height * 0.5,
    ];
    assert!(
        (center[0] - expected[0]).abs() < 0.01,
        "{kind} center x should use first CDXML bbox point, got {center:?}"
    );
    assert!(
        (center[1] - expected[1]).abs() < 0.01,
        "{kind} center y should use first CDXML bbox point, got {center:?}"
    );
}

#[test]
fn cdxml_represented_radical_symbol_does_not_double_count_node_radical() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" color="0" bgcolor="1">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 40 20">
      <n id="3" p="10 10" Element="7" Radical="Doublet"/>
      <n id="4" p="24.4 10"/>
      <b id="5" B="3" E="4"/>
      <graphic id="6" BoundingBox="10 13 2.5 13" GraphicType="Symbol" SymbolType="Electron">
        <represent attribute="Radical"/>
      </graphic>
    </fragment>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("radical cdxml should load");
    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist");
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "3")
        .expect("radical nitrogen should import");
    assert_eq!(
        node.meta
            .get("radicalCount")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    assert_eq!(node.num_hydrogens, 1);
    let attached = node
        .meta
        .get("attachedElectronSymbols")
        .and_then(|value| value.as_array())
        .expect("electron symbol should attach to the radical nitrogen");
    assert_eq!(
        attached
            .first()
            .and_then(|value| value.get("radicalDelta"))
            .and_then(|value| value.as_i64()),
        Some(0)
    );
}

#[test]
fn cdxml_represented_charge_symbol_roundtrips_without_accumulating_charge() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" color="0" bgcolor="1">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 40 20">
      <n id="3" p="10 10" Element="6" Charge="-1" NumHydrogens="1">
        <t id="4" p="10 10" BoundingBox="5 5 15 15" InterpretChemically="yes" UTF8Text="CH"><s face="96">CH</s></t>
      </n>
      <n id="5" p="24.4 10"/>
      <b id="6" B="3" E="5"/>
    </fragment>
    <graphic id="7" BoundingBox="10 15 10 5" GraphicType="Symbol" SymbolType="CircleMinus">
      <represent attribute="Charge" object="3"/>
    </graphic>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("represented charge should import");
    let exported = document_to_cdxml(&engine.state().document);
    assert!(
        exported.contains("<represent attribute=\"Charge\" object="),
        "{exported}"
    );
    let mut reimported = Engine::new();
    reimported
        .load_cdxml_document(&exported)
        .expect("represented charge export should import");
    let fragment = reimported
        .state()
        .document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should survive");
    assert_eq!(
        fragment
            .nodes
            .iter()
            .find(|node| node.charge != 0)
            .map(|node| node.charge),
        Some(-1)
    );
}

#[test]
fn cdxml_element_list_query_roundtrips_without_becoming_a_nickname() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" color="0" bgcolor="1">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 40 20">
      <n id="3" p="10 10" NodeType="ElementList" ElementList="6 7 15">
        <t id="4" p="10 10" BoundingBox="5 5 25 15" UTF8Text="[C,N,P]"><s face="96">[C,N,P]</s></t>
      </n>
      <n id="5" p="24.4 10"/>
      <b id="6" B="3" E="5"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("element list")).expect("query should import");
    let exported = document_to_cdxml(&document);
    assert!(exported.contains("NodeType=\"ElementList\""), "{exported}");
    assert!(exported.contains("ElementList=\"6 7 15\""), "{exported}");
    let reimported = parse_cdxml_document(&exported, Some("element list export"))
        .expect("query should reimport");
    let query_node = reimported
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.label.is_some()))
        .expect("query node should survive");
    assert!(!query_node.is_placeholder);
    assert_eq!(
        query_node
            .meta
            .pointer("/import/cdxml/elementList")
            .and_then(|value| value.as_str()),
        Some("6 7 15")
    );
}

#[test]
fn cdxml_left_dashed_double_bond_preserves_which_line_is_dashed() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" color="0" bgcolor="1">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 40 20">
      <n id="3" p="10 10"/><n id="4" p="24.4 10"/>
      <b id="5" B="3" E="4" Order="2" DoublePosition="Left" Display2="Dash"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("left dashed double")).expect("bond imports");
    let exported = document_to_cdxml(&document);
    assert!(exported.contains("Display2=\"Dash\""), "{exported}");
    assert!(!exported.contains(" Display=\"Dash\""), "{exported}");
    let reimported =
        parse_cdxml_document(&exported, Some("left dashed double export")).expect("bond reimports");
    let bond = reimported
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.bonds.first())
        .expect("bond survives");
    assert_eq!(
        bond.line_styles.main,
        chemsema_engine::BondLinePattern::Solid
    );
    assert_eq!(
        bond.line_styles.left,
        chemsema_engine::BondLinePattern::Dashed
    );
}

#[test]
fn parse_cdxml_color_table_keeps_duplicate_slots() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML color="0" bgcolor="1">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
    <color r="0" g="0" b="0"/>
    <color r="1" g="0" b="0"/>
  </colortable>
  <page id="1">
    <graphic id="2" BoundingBox="20 20 40 40" Z="1" color="5" GraphicType="Oval" OvalType="Circle Filled" Center3D="30 30 0" MajorAxisEnd3D="40 30 0" MinorAxisEnd3D="30 40 0"/>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("duplicate colors")).expect("cdxml should parse");

    assert_eq!(cdxml_shape_fills_by_z(&document), vec!["#ff0000"]);
}

#[test]
fn parse_cdxml_infers_benzene_double_bond_sides_and_bond_colors() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" BondSpacing="18" LineWidth="0.60" color="0" bgcolor="1">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
    <color r="1" g="0" b="0"/>
  </colortable>
  <page id="1">
    <fragment id="2" BoundingBox="7 10 33 39">
      <n id="n1" p="20.00 10.00"/>
      <n id="n2" p="32.47 17.20"/>
      <n id="n3" p="32.47 31.60"/>
      <n id="n4" p="20.00 38.80"/>
      <n id="n5" p="7.53 31.60"/>
      <n id="n6" p="7.53 17.20"/>
      <b id="b1" B="n1" E="n2" Order="2" BondCircularOrdering="b2 0 0 b6"/>
      <b id="b2" B="n2" E="n3"/>
      <b id="b3" B="n3" E="n4" Order="2" color="4" BondCircularOrdering="b4 0 0 b2"/>
      <b id="b4" B="n4" E="n5"/>
      <b id="b5" B="n5" E="n6" Order="2" BondCircularOrdering="b6 0 0 b4"/>
      <b id="b6" B="n6" E="n1"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("benzene")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should import");

    for bond_id in ["b1", "b3", "b5"] {
        let bond = fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id)
            .expect("benzene double bond should import");
        assert_eq!(
            bond.double.as_ref().map(|double| double.placement),
            Some(chemsema_engine::DoubleBondPlacement::Left),
            "{bond_id} should infer an inward side double placement"
        );
    }
    assert_eq!(
        fragment
            .bonds
            .iter()
            .find(|bond| bond.id == "b3")
            .and_then(|bond| bond.stroke.as_deref()),
        Some("#ff0000")
    );

    let primitives = render_document(&document);
    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Line {
                role: RenderRole::DocumentBond,
                bond_id: Some(id),
                stroke,
                ..
            } | RenderPrimitive::Polygon {
                role: RenderRole::DocumentBond,
                bond_id: Some(id),
                stroke,
                ..
            } if id == "b3" && stroke == "#ff0000"
        )),
        "colored CDXML bond should render with its imported stroke"
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("color=\"4\""), "{exported}");
    assert!(exported.contains("DoublePosition=\"Left\""), "{exported}");
    let reimported =
        parse_cdxml_document(&exported, Some("benzene export")).expect("export should parse");
    let reimported_fragment = reimported
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("reimported fragment should exist");
    assert!(reimported_fragment
        .bonds
        .iter()
        .any(|bond| bond.stroke.as_deref() == Some("#ff0000")));
}

#[test]
fn parse_cdxml_auto_dashed_double_bond_uses_ring_inside_side() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" BondSpacing="18" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50">
  <page id="1">
    <fragment id="2" BoundingBox="7 10 33 39">
      <n id="n1" p="20.00 10.00"/>
      <n id="n2" p="32.47 17.20"/>
      <n id="n3" p="32.47 31.60"/>
      <n id="n4" p="20.00 38.80"/>
      <n id="n5" p="7.53 31.60"/>
      <n id="n6" p="7.53 17.20"/>
      <b id="b1" B="n1" E="n2" Order="1.5" Display2="Dash"/>
      <b id="b2" B="n2" E="n3"/>
      <b id="b3" B="n3" E="n4" Order="2"/>
      <b id="b4" B="n4" E="n5"/>
      <b id="b5" B="n5" E="n6" Order="2"/>
      <b id="b6" B="n6" E="n1"/>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("auto dashed ring double")).expect("cdxml should parse");
    let bond = imported_fragment_bond(&document, "obj_mol_001", "b1");
    let double = bond
        .double
        .as_ref()
        .expect("auto dashed double should import as a double bond");

    assert_eq!(double.placement, chemsema_engine::DoubleBondPlacement::Left);
    assert!(
        !double.frozen,
        "no explicit DoublePosition means placement should remain auto"
    );
    assert_eq!(
        bond.line_styles.main,
        chemsema_engine::BondLinePattern::Solid
    );
    assert_eq!(
        bond.line_styles.left,
        chemsema_engine::BondLinePattern::Dashed
    );
    assert_eq!(
        bond.line_styles.right,
        chemsema_engine::BondLinePattern::Solid
    );
}

#[test]
fn parse_cdxml_auto_ring_double_prioritizes_ring_side_over_neighbor_double() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" BondSpacing="18" LineWidth="0.60">
  <page id="1">
    <fragment id="2" BoundingBox="7 10 33 39">
      <n id="n1" p="20.00 10.00"/>
      <n id="n2" p="32.47 17.20"/>
      <n id="n3" p="32.47 31.60"/>
      <n id="n4" p="20.00 38.80"/>
      <n id="n5" p="7.53 31.60"/>
      <n id="n6" p="7.53 17.20"/>
      <b id="b1" B="n1" E="n2" Order="2"/>
      <b id="b2" B="n2" E="n3" Order="2"/>
      <b id="b3" B="n3" E="n4"/>
      <b id="b4" B="n4" E="n5"/>
      <b id="b5" B="n5" E="n6"/>
      <b id="b6" B="n6" E="n1"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("ring priority")).expect("cdxml should parse");
    let bond = imported_fragment_bond(&document, "obj_mol_001", "b1");

    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(chemsema_engine::DoubleBondPlacement::Left),
        "ring membership should choose the inward side before adjacent-double centering"
    );
}

#[test]
fn cdxml_export_import_preserves_non_white_page_background() {
    let document = parse_document_json(
        &json!({
            "format": { "name": "chemsema", "version": "0.1" },
            "document": {
                "id": "doc_test",
                "title": "red background",
                "page": { "width": 120.0, "height": 80.0, "background": "#ff0000" }
            },
            "styles": {},
            "objects": [],
            "resources": {}
        })
        .to_string(),
    )
    .expect("document json should parse");

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("bgcolor=\"4\""), "{exported}");

    let reimported =
        parse_cdxml_document(&exported, Some("red background")).expect("export should parse");
    assert_eq!(reimported.document.page.background, "#ff0000");
}

#[test]
fn parse_cdxml_right_side_double_bonds_render_on_begin_to_end_right_side() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="20.00" BondSpacing="18" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" LabelSize="10" BoundingBox="0 0 40 50">
  <page id="p1" BoundingBox="0 0 40 50">
    <fragment id="f1" BoundingBox="0 0 40 50">
      <n id="n1" p="10 10"/>
      <n id="n2" p="10 30"/>
      <b id="b1" B="n1" E="n2" Order="2" DoublePosition="Right"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("right side")).expect("cdxml should parse");
    let object = document
        .objects
        .iter()
        .find(|object| object.id == "obj_mol_001")
        .expect("fragment should import as obj_mol_001");
    let resource_ref = object
        .payload
        .resource_ref
        .as_deref()
        .expect("molecule object should have resourceRef");
    let fragment = document
        .resources
        .get(resource_ref)
        .expect("molecule resource should exist")
        .data
        .as_fragment()
        .expect("molecule resource should have fragment data");

    let bond = fragment
        .bonds
        .iter()
        .find(|bond| bond.id == "b1")
        .expect("double bond should import");
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(chemsema_engine::DoubleBondPlacement::Right)
    );

    let primitives = render_document(&document);
    let centerlines = object_bond_centerlines_with_ids(&primitives, "obj_mol_001");
    let begin = chemsema_engine::Point::new(10.0, 10.0);
    let end = chemsema_engine::Point::new(10.0, 30.0);
    let dx = end.x - begin.x;
    let dy = end.y - begin.y;
    let length = dx.hypot(dy);
    let right_normal = chemsema_engine::Point::new(dy / length, -dx / length);
    let raw_mid = chemsema_engine::Point::new((begin.x + end.x) * 0.5, (begin.y + end.y) * 0.5);
    let max_rendered_projection = centerlines
        .iter()
        .filter(|(id, _, _)| id == "b1")
        .map(|(_, from, to)| {
            let mid = chemsema_engine::Point::new((from.x + to.x) * 0.5, (from.y + to.y) * 0.5);
            (mid.x - raw_mid.x) * right_normal.x + (mid.y - raw_mid.y) * right_normal.y
        })
        .max_by(|a, b| a.total_cmp(b))
        .expect("double bond should render centerlines");
    assert!(
        max_rendered_projection > 0.0,
        "outer line should render on B->E right side, got {max_rendered_projection}"
    );
}

#[test]
fn parse_cdxml_unspecified_alkene_double_bond_uses_automatic_side_placement() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 120 80">
    <fragment id="f1" BoundingBox="10 10 80 50">
      <n id="n1" p="10 40"/>
      <n id="n2" p="24.4 40"/>
      <n id="n3" p="38.8 40"/>
      <n id="n4" p="24.4 26"/>
      <b id="b1" B="n1" E="n2"/>
      <b id="b2" B="n2" E="n3" Order="2"/>
      <b id="b3" B="n2" E="n4"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("alkene")).expect("cdxml should parse");
    let fragment = document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment;
    let bond = fragment
        .bonds
        .iter()
        .find(|bond| bond.id == "b2")
        .expect("alkene double bond should import");

    let double = bond
        .double
        .as_ref()
        .expect("double bond state should be inferred");
    assert_ne!(
        double.placement,
        chemsema_engine::DoubleBondPlacement::Center
    );
    assert!(!double.frozen);
}

#[test]
fn parse_cdxml_auto_double_bond_matches_chemdraw_center_and_tie_rules() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="20.00" BondSpacing="18" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" LabelSize="10" BoundingBox="0 0 140 70">
  <page id="p1" BoundingBox="0 0 140 70">
    <fragment id="f1" BoundingBox="0 0 60 60">
      <n id="n1" p="30 25"/>
      <n id="n2" p="30 45"/>
      <n id="n3" p="12.68 15"/>
      <n id="n4" p="47.32 15"/>
      <b id="b1" B="n1" E="n2" Order="2"/>
      <b id="b2" B="n1" E="n3"/>
      <b id="b3" B="n1" E="n4"/>
    </fragment>
    <fragment id="f2" BoundingBox="60 0 140 60">
      <n id="m1" p="80 30"/>
      <n id="m2" p="100 30"/>
      <n id="m3" p="62.68 20"/>
      <n id="m4" p="117.32 40"/>
      <b id="m5" B="m1" E="m2" Order="2"/>
      <b id="m6" B="m1" E="m3"/>
      <b id="m7" B="m2" E="m4"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("auto double")).expect("cdxml should parse");
    let mut placements = Vec::new();
    for fragment in document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
    {
        for bond in &fragment.bonds {
            if bond.order == 2 {
                placements.push((
                    bond.id.as_str(),
                    bond.double.as_ref().map(|double| double.placement),
                    bond.double.as_ref().map(|double| double.frozen),
                ));
            }
        }
    }
    assert!(
        placements.contains(&(
            "b1",
            Some(chemsema_engine::DoubleBondPlacement::Center),
            Some(false)
        )),
        "{placements:?}"
    );
    assert!(
        placements.contains(&(
            "m5",
            Some(chemsema_engine::DoubleBondPlacement::Right),
            Some(false)
        )),
        "{placements:?}"
    );
}

#[test]
fn parse_cdxml_auto_double_bond_places_five_member_ring_inside() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="20.00" BondSpacing="18" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" LabelSize="10" BoundingBox="0 0 50 60">
  <page id="p1" BoundingBox="0 0 50 60">
    <fragment id="f1" BoundingBox="0 0 50 60">
      <n id="n1" p="10 20"/>
      <n id="n2" p="24.4 20"/>
      <n id="n3" p="31.25 33.65"/>
      <n id="n4" p="17.2 44"/>
      <n id="n5" p="3.15 33.65"/>
      <b id="b1" B="n1" E="n2" Order="2"/>
      <b id="b2" B="n2" E="n3"/>
      <b id="b3" B="n3" E="n4"/>
      <b id="b4" B="n4" E="n5"/>
      <b id="b5" B="n5" E="n1"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("cyclopentene")).expect("cdxml should parse");
    let fragment = document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment;
    let bond = fragment
        .bonds
        .iter()
        .find(|bond| bond.id == "b1")
        .expect("ring double bond should import");

    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(chemsema_engine::DoubleBondPlacement::Left)
    );
    assert_eq!(
        bond.double.as_ref().map(|double| double.frozen),
        Some(false)
    );
}

#[test]
fn parse_cdxml_auto_double_bond_prefers_alternating_ring_over_short_fused_cycle() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="40 240 90 305">
    <fragment id="f1" BoundingBox="40 240 90 305">
      <n id="a" p="62.90 267.82"/>
      <n id="b" p="76.43 272.76"/>
      <n id="c" p="51.87 277.07"/>
      <n id="d" p="54.35 291.25"/>
      <n id="e" p="67.88 296.19"/>
      <n id="f" p="78.92 286.94"/>
      <n id="g" p="63.42 253.43"/>
      <n id="h" p="77.27 249.48"/>
      <n id="i" p="85.31 261.42"/>
      <b id="target" B="a" E="b" Order="2"/>
      <b id="outer1" B="a" E="c"/>
      <b id="outer2" B="c" E="d" Order="2"/>
      <b id="outer3" B="d" E="e"/>
      <b id="outer4" B="e" E="f" Order="2"/>
      <b id="outer5" B="f" E="b"/>
      <b id="short1" B="a" E="g"/>
      <b id="short2" B="g" E="h"/>
      <b id="short3" B="h" E="i"/>
      <b id="short4" B="i" E="b"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("fused alternating")).expect("cdxml should parse");
    let fragment = document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment;
    let bond = fragment
        .bonds
        .iter()
        .find(|bond| bond.id == "target")
        .expect("target double bond should import");

    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(chemsema_engine::DoubleBondPlacement::Left)
    );
}

#[test]
fn parse_cdxml_attached_atom_label_rebuilds_active_bbox_from_glyph_metrics() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 40 24">
    <fragment id="f1" BoundingBox="0 0 40 24">
      <n id="n1" p="10 12" Element="7">
        <t p="6.40 15.90" BoundingBox="6.40 7.56 13.62 15.90" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">N</s>
        </t>
      </n>
      <n id="n2" p="24 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("atom label bbox")).expect("cdxml should parse");
    let label = document
        .resources
        .values()
        .find_map(|resource| {
            resource
                .data
                .as_fragment()
                .and_then(|fragment| fragment.nodes.iter().find(|node| node.atomic_number == 7))
        })
        .and_then(|node| node.label.as_ref())
        .expect("N label should import");
    let bbox = label.bbox().expect("N label should have an active bbox");
    let height = bbox[3] - bbox[1];
    assert!(
        (height - 8.9).abs() < 0.01,
        "attached CDXML labels should use the internal molecule-label line advance, got {bbox:?}"
    );
    assert_eq!(
        label.meta.pointer("/import/cdxml/boundingBox"),
        Some(&json!([6.4, 7.56, 13.62, 15.9])),
        "the original ChemDraw box should remain import evidence"
    );
    assert!(
        !label.glyph_polygons.is_empty(),
        "refresh should still populate glyph polygons for clipping"
    );

    let displaced_cdxml = cdxml
        .replace("p=\"6.40 15.90\"", "p=\"31.00 -17.00\"")
        .replace(
            "BoundingBox=\"6.40 7.56 13.62 15.90\"",
            "BoundingBox=\"-80 -60 140 190\"",
        );
    let displaced_document = parse_cdxml_document(&displaced_cdxml, Some("displaced atom label"))
        .expect("CDXML with displaced source geometry should parse");
    let displaced_label = displaced_document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "n1"))
        .and_then(|node| node.label.as_ref())
        .expect("displaced N label should import");
    assert_eq!(
        displaced_label.position, label.position,
        "source text position must not affect active node-label layout"
    );
    assert_eq!(
        displaced_label.bbox(),
        label.bbox(),
        "source BoundingBox must not affect active node-label layout"
    );
    assert_eq!(
        displaced_label.glyph_polygons, label.glyph_polygons,
        "source text geometry must not affect active clipping geometry"
    );
}

#[test]
fn parse_cdxml_right_aligned_attached_labels_use_line_anchor_y() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="30.00" LineWidth="1.00" BoldWidth="4.00" HashSpacing="2.70" BondSpacing="12" MarginWidth="2.00" LabelSize="10">
  <page id="p1" BoundingBox="0 0 120 170">
    <fragment id="f1" BoundingBox="20 20 80 150">
      <n id="c1" p="70 40" AS="N"/>
      <n id="rprime" p="40 40" NodeType="GenericNickname" GenericNickname="R" NumHydrogens="0" AS="N">
        <t p="40.95 43.90" BoundingBox="31.82 35.56 40.95 43.90" LabelJustification="Right" Justification="Right" LabelAlignment="Right">
          <s font="3" size="10" color="0">R&apos;</s>
        </t>
      </n>
      <b id="b1" B="c1" E="rprime"/>
      <n id="c2" p="70 90" AS="N"/>
      <n id="me" p="40 90" NodeType="GenericNickname" GenericNickname="Me" NumHydrogens="0" AS="N">
        <t p="42.78 93.90" BoundingBox="30.00 85.56 42.78 93.90" LabelJustification="Right" Justification="Right" LabelAlignment="Right">
          <s font="3" size="10" color="0">Me</s>
        </t>
      </n>
      <b id="b2" B="c2" E="me"/>
      <n id="c3" p="70 140" AS="N"/>
      <n id="ar" p="40 140" NodeType="GenericNickname" GenericNickname="Ar" NumHydrogens="0" AS="N">
        <t p="41.67 143.90" BoundingBox="30.00 135.56 41.67 143.90" LabelJustification="Right" Justification="Right" LabelAlignment="Right">
          <s font="3" size="10" color="0">Ar</s>
        </t>
      </n>
      <b id="b3" B="c3" E="ar"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("right attached labels")).expect("cdxml should parse");
    for node_id in ["rprime", "me", "ar"] {
        let node = document
            .resources
            .values()
            .filter_map(|resource| resource.data.as_fragment())
            .find_map(|fragment| fragment.nodes.iter().find(|node| node.id == node_id))
            .expect("node should import");
        let label = node.label.as_ref().expect("node label should import");
        let baseline = label.position.expect("label should have a baseline")[1];
        assert!(
            (baseline - node.position[1] - 3.9).abs() < 0.01,
            "{node_id} baseline should follow ChemDraw's line-anchor y, got node={:?} label={:?}",
            node.position,
            label.position
        );
    }
}

#[test]
fn render_cdxml_single_character_atom_label_uses_text_primitive() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 40 24">
    <fragment id="f1" BoundingBox="0 0 40 24">
      <n id="n1" p="10 12" Element="7">
        <t p="6.40 15.90" BoundingBox="6.40 7.56 13.62 15.90" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">N</s>
        </t>
      </n>
      <n id="n2" p="24 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("single atom label")).expect("cdxml should parse");
    let primitives = render_document(&document);
    let text = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                node_id,
                role,
                x,
                y,
                runs,
                text_anchor,
                ..
            } if node_id.as_deref() == Some("n1") && *role == RenderRole::DocumentText => {
                Some((*x, *y, runs.clone(), text_anchor.clone()))
            }
            _ => None,
        })
        .expect("N label should render as text");

    assert!((text.0 - 6.42).abs() < 0.001, "{text:?}");
    assert!((text.1 - 15.90).abs() < 0.001, "{text:?}");
    assert_eq!(
        text.2
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        "N"
    );
    assert_eq!(text.3.as_deref(), Some("start"));
    assert!(!primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::FilledPath {
            node_id,
            role,
            ..
        } if node_id.as_deref() == Some("n1") && *role == RenderRole::DocumentText
    )));
}

#[test]
fn parse_cdxml_node_label_preserves_explicit_nonchemical_semantics() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="14.40" LineWidth="0.60" MarginWidth="1.60">
  <page id="p1" BoundingBox="0 0 40 24">
    <fragment id="f1" BoundingBox="0 0 40 24">
      <n id="n1" p="10 12" Element="7">
        <t p="-20 40" BoundingBox="-80 -60 140 190" InterpretChemically="no">
          <s font="3" size="10" color="0" face="96">NH2</s>
        </t>
      </n>
      <n id="n2" p="24 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("nonchemical label")).expect("CDXML should parse");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.atomic_number == 7))
        .and_then(|node| node.label.as_ref())
        .expect("NH2 label should import");

    assert_eq!(label.text, "NH2");
    assert_eq!(label.meta.pointer("/defaultChemical"), Some(&json!(false)));
    assert_eq!(
        label.meta.pointer("/sourceRuns/0/script"),
        Some(&json!("normal"))
    );
    let exported = document_to_cdxml(&document);
    assert!(
        exported.contains("InterpretChemically=\"no\""),
        "{exported}"
    );
    assert!(exported.contains("BoundingBox="), "{exported}");
}

#[test]
fn parse_cdxml_node_label_subscripts_digits_across_style_run_boundaries() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="14.40" LineWidth="0.60" MarginWidth="1.60" InterpretChemically="yes">
  <page id="p1" BoundingBox="0 0 80 32">
    <fragment id="f1" BoundingBox="0 0 80 32">
      <n id="pd" p="20 16">
        <t p="20 20" BoundingBox="20 8 72 22" InterpretChemically="yes">
          <s font="3" size="10" color="0" face="97">Pd</s>
          <s font="3" size="10" color="0" face="65">IV</s>
          <s font="3" size="10" color="0" face="97">(OCF</s>
          <s font="3" size="10" color="0" face="97">3</s>
          <s font="3" size="10" color="0" face="97">)n</s>
        </t>
      </n>
      <n id="c1" p="40 16"/>
      <b id="b1" B="pd" E="c1"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("split formula run"))
        .expect("split formula CDXML should parse");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find_map(|node| node.label.as_ref()))
        .expect("Pd formula label should import");

    assert!(
        label
            .runs
            .iter()
            .any(|run| matches!(run.text.as_str(), "IV" | "VI")
                && run.script.as_deref() == Some("superscript")),
        "{:?}",
        label.runs
    );
    assert!(
        label
            .runs
            .iter()
            .any(|run| run.text == "3" && run.script.as_deref() == Some("subscript")),
        "{:?}",
        label.runs
    );
}

#[test]
fn parse_cdxml_preserves_document_drawing_defaults_without_using_cached_label_geometry() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML FractionalWidths="no" InterpretChemically="no" ShowTerminalCarbonLabels="yes" ShowNonTerminalCarbonLabels="yes" HideImplicitHydrogens="yes" LabelFont="4" LabelSize="11" LabelFace="98" CaptionFont="5" CaptionSize="9" CaptionFace="2" LineWidth="0.72" BoldWidth="3.20" BondLength="17.50" BondSpacing="21" HashSpacing="2.20" MarginWidth="1.60" ChainAngle="109.5" LabelJustification="Right" CaptionJustification="Center" PrintMargins="12 13 14 15" color="2">
  <fonttable>
    <font id="4" charset="iso-8859-1" name="Times New Roman"/>
    <font id="5" charset="iso-8859-1" name="Courier New"/>
  </fonttable>
  <page id="p1" BoundingBox="0 0 80 40">
    <fragment id="f1" BoundingBox="0 0 50 24">
      <n id="n1" p="10 12" Element="7">
        <t p="-20 40" BoundingBox="-80 -60 140 190">
          <s color="0">NH2</s>
        </t>
      </n>
      <n id="n2" p="28 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
    <t id="txt1" p="60 14" BoundingBox="45 5 75 18">
      <s color="0">note</s>
    </t>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("defaults")).expect("CDXML should parse");
    let defaults = document
        .document
        .meta
        .pointer("/import/cdxml/defaults")
        .expect("CDXML defaults should be preserved");

    assert_eq!(defaults.get("chainAngle"), Some(&json!(109.5)));
    assert_eq!(
        defaults.pointer("/labelStyle/fontFamily"),
        Some(&json!("Times New Roman"))
    );
    assert_eq!(defaults.pointer("/labelStyle/fontSize"), Some(&json!(11.0)));
    assert_eq!(
        defaults.pointer("/labelStyle/fontStyle"),
        Some(&json!("italic"))
    );
    assert_eq!(
        defaults.pointer("/labelStyle/script"),
        Some(&json!("chemical"))
    );
    assert_eq!(
        defaults.pointer("/labelStyle/fill"),
        Some(&json!("#ffffff"))
    );
    assert_eq!(
        defaults.pointer("/captionStyle/fontFamily"),
        Some(&json!("Courier New"))
    );
    assert_eq!(
        defaults.pointer("/captionStyle/fontSize"),
        Some(&json!(9.0))
    );
    assert_eq!(
        defaults.pointer("/captionStyle/fontStyle"),
        Some(&json!("italic"))
    );
    assert_eq!(
        defaults.pointer("/captionStyle/script"),
        Some(&json!("normal"))
    );
    assert_eq!(defaults.get("foregroundColor"), Some(&json!("#ffffff")));
    for opaque_key in [
        "labelFont",
        "labelFace",
        "captionFont",
        "captionFace",
        "color",
    ] {
        assert!(
            defaults.get(opaque_key).is_none(),
            "opaque {opaque_key} leaked into CCJS"
        );
    }
    assert_eq!(defaults.get("labelJustification"), Some(&json!("Right")));
    assert_eq!(defaults.get("captionJustification"), Some(&json!("Center")));
    assert_eq!(defaults.get("fractionalWidths"), Some(&json!(false)));
    assert_eq!(defaults.get("interpretChemically"), Some(&json!(false)));
    assert_eq!(defaults.get("showTerminalCarbonLabels"), Some(&json!(true)));
    assert_eq!(
        defaults.get("showNonTerminalCarbonLabels"),
        Some(&json!(true))
    );
    assert_eq!(defaults.get("hideImplicitHydrogens"), Some(&json!(true)));
    assert_eq!(
        defaults.get("printMargins"),
        Some(&json!([12.0, 13.0, 14.0, 15.0]))
    );
    assert_eq!(
        document.style.defaults.get("chainAngle").copied(),
        Some(109.5)
    );
    assert_eq!(document.style.label_style.font_family, "Times New Roman");
    assert_eq!(document.style.label_style.font_size, 11.0);
    assert_eq!(document.style.label_style.font_style, "italic");
    assert_eq!(document.style.label_style.script, "chemical");
    assert_eq!(document.style.caption_style.font_family, "Courier New");
    assert_eq!(document.style.caption_style.font_size, 9.0);

    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "n1"))
        .and_then(|node| node.label.as_ref())
        .expect("node label should import");
    assert_eq!(label.font_family.as_deref(), Some("Times New Roman"));
    assert_eq!(label.font_size, Some(11.0));
    assert_eq!(label.align.as_deref(), Some("right"));
    assert_eq!(label.meta.pointer("/defaultChemical"), Some(&json!(false)));
    for opaque_key in ["font", "face", "color"] {
        assert!(
            label
                .meta
                .pointer(&format!("/import/cdxml/{opaque_key}"))
                .is_none(),
            "opaque label {opaque_key} leaked into CCJS"
        );
    }
    assert_eq!(
        label.meta.pointer("/sourceRuns/0/fontStyle"),
        Some(&json!("italic"))
    );
    assert_eq!(
        label.meta.pointer("/sourceRuns/0/script"),
        Some(&json!("normal")),
        "root InterpretChemically=no keeps the visible text non-chemical even when LabelFace has chemical bits"
    );
    assert_ne!(
        label.bbox(),
        Some([-80.0, -60.0, 140.0, 190.0]),
        "source BoundingBox must remain evidence, not active node-label geometry"
    );

    let text_object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("free text should import");
    let style = document
        .styles
        .get(text_object.style_ref.as_deref().expect("text style ref"))
        .expect("text style should exist");
    assert_eq!(style.get("fontFamily"), Some(&json!("Courier New")));
    assert_eq!(style.get("fontSize"), Some(&json!(9.0)));
    assert_eq!(
        text_object.payload.extra.get("align"),
        Some(&json!("center"))
    );
    assert_eq!(
        text_object
            .payload
            .extra
            .get("runs")
            .and_then(|runs| runs.pointer("/0/fontStyle")),
        Some(&json!("italic"))
    );

    let exported = document_to_cdxml(&document);
    for expected in [
        "FractionalWidths=\"no\"",
        "InterpretChemically=\"no\"",
        "ShowTerminalCarbonLabels=\"yes\"",
        "ShowNonTerminalCarbonLabels=\"yes\"",
        "HideImplicitHydrogens=\"yes\"",
        "LabelFont=\"4\"",
        "LabelFace=\"98\"",
        "CaptionFont=\"5\"",
        "CaptionFace=\"2\"",
        "ChainAngle=\"109.5\"",
        "LabelJustification=\"Right\"",
        "CaptionJustification=\"Center\"",
        "PrintMargins=\"12 13 14 15\"",
        "color=\"2\"",
    ] {
        assert!(
            exported.contains(expected),
            "missing {expected} in {exported}"
        );
    }
}

#[test]
fn cdxml_centered_text_anchor_is_stable_after_first_save() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LabelFont="3" LabelSize="10" LabelFace="96" CaptionFont="3" CaptionSize="10" CaptionFace="0">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 300 200" Width="300" Height="200">
    <t id="2" p="135.75 535.25" BoundingBox="113.59 526 157.92 655.25" Justification="Center" Z="1" UTF8Text="Acid"><s font="3" size="10" color="3" face="0">Acid</s></t>
  </page>
</CDXML>"#;
    let imported = parse_cdxml_document(cdxml, Some("centered-text")).expect("CDXML imports");
    let first = document_to_cdxml(&imported);
    let reimported = parse_cdxml_document(&first, Some("centered-text")).expect("export imports");
    let second = document_to_cdxml(&reimported);

    assert_eq!(
        second, first,
        "centered text must not drift after first save"
    );
}

#[test]
fn render_cdxml_imported_atom_label_uses_text_primitive() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 48 24">
    <fragment id="f1" BoundingBox="0 0 48 24">
      <n id="n1" p="10 12" Element="7">
        <t p="6.40 15.90" BoundingBox="6.40 7.56 21.60 15.90" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">NH</s>
        </t>
      </n>
      <n id="n2" p="30 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("multi atom label")).expect("cdxml");
    let primitives = render_document(&document);
    let text = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                node_id,
                role,
                runs,
                ..
            } if node_id.as_deref() == Some("n1") && *role == RenderRole::DocumentText => {
                Some(runs.clone())
            }
            _ => None,
        })
        .expect("HN label should render as text");

    assert_eq!(
        text.iter().map(|run| run.text.as_str()).collect::<String>(),
        "HN"
    );
    assert!(!primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::FilledPath {
            node_id,
            role,
            ..
        } if node_id.as_deref() == Some("n1") && *role == RenderRole::DocumentText
    )));
}

#[test]
fn parse_cdxml_right_aligned_chemical_node_label_reverses_visible_groups() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 44 24">
    <fragment id="f1" BoundingBox="0 0 44 24">
      <n id="n1" p="22 12" Element="6">
        <t p="22.00 15.90" BoundingBox="10.00 7.56 22.00 15.90" LabelJustification="Right">
          <s font="3" size="10" color="0" face="96">OCF3</s>
        </t>
      </n>
      <n id="n2" p="34 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("right aligned label")).expect("cdxml should parse");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "n1"))
        .and_then(|node| node.label.as_ref())
        .expect("OCF3 label should import");

    assert_eq!(label.align.as_deref(), Some("right"));
    assert_eq!(label.source_text.as_deref(), Some("OCF3"));
    assert_eq!(label.text, "F3CO");
    let display_text: String = label.runs.iter().map(|run| run.text.as_str()).collect();
    assert_eq!(display_text, "F3CO");
    assert_eq!(
        label
            .meta
            .pointer("/sourceRuns/0/text")
            .and_then(serde_json::Value::as_str),
        Some("OCF3")
    );
}

#[test]
fn parse_cdxml_right_aligned_labels_reverse_groups_independent_of_validity() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 80 84">
    <fragment id="f1" BoundingBox="0 0 80 84">
      <n id="tfa" p="30 12" Element="6">
        <t p="30.00 15.90" BoundingBox="-2.00 7.56 30.00 15.90" LabelJustification="Right" Justification="Right" LabelAlignment="Right">
          <s font="3" size="10" color="0" face="96">OTFA</s>
        </t>
      </n>
      <n id="tfa2" p="48 12"/>
      <b id="btfa" B="tfa" E="tfa2"/>
      <n id="xyz" p="30 40" Element="6">
        <t p="30.00 43.90" BoundingBox="-2.00 35.56 30.00 43.90" LabelJustification="Right" Justification="Right" LabelAlignment="Right">
          <s font="3" size="10" color="0" face="96">OXYZ</s>
        </t>
      </n>
      <n id="xyz2" p="48 40"/>
      <b id="bxyz" B="xyz" E="xyz2"/>
      <n id="nme" p="30 68" Element="6">
        <t p="30.00 71.90" BoundingBox="-2.00 63.56 30.00 71.90" LabelJustification="Right" Justification="Right" LabelAlignment="Right">
          <s font="3" size="10" color="0" face="96">NMe4</s>
        </t>
      </n>
      <n id="nme2" p="48 68"/>
      <b id="bnme" B="nme" E="nme2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("right aligned display tokens"))
        .expect("cdxml should parse");
    let label_for = |node_id: &str| {
        document
            .resources
            .values()
            .filter_map(|resource| resource.data.as_fragment())
            .flat_map(|fragment| fragment.nodes.iter())
            .find(|node| node.id == node_id)
            .and_then(|node| node.label.as_ref())
            .unwrap_or_else(|| panic!("{node_id} label should import"))
    };

    let tfa = label_for("tfa");
    assert_eq!(tfa.source_text.as_deref(), Some("OTFA"));
    assert_eq!(tfa.text, "TFAO");
    assert_eq!(
        tfa.meta
            .pointer("/labelRecognition/status")
            .and_then(serde_json::Value::as_str),
        Some("recognized")
    );

    let xyz = label_for("xyz");
    assert_eq!(xyz.source_text.as_deref(), Some("OXYZ"));
    assert_eq!(xyz.text, "ZYXO");
    assert_eq!(
        xyz.meta
            .pointer("/labelRecognition/diagnostic")
            .and_then(serde_json::Value::as_str),
        Some("uninterpretable-label")
    );

    let nme = label_for("nme");
    assert_eq!(nme.source_text.as_deref(), Some("NMe4"));
    assert_eq!(nme.text, "Me4N");
    assert_eq!(
        nme.meta
            .pointer("/labelRecognition/diagnostic")
            .and_then(serde_json::Value::as_str),
        Some("invalid-valence")
    );
}

#[test]
fn parse_cdxml_attached_chemical_label_preserves_visible_spaces() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" MarginWidth="1.60" LabelSize="10">
  <page id="p1" BoundingBox="0 0 100 36">
    <fragment id="f1" BoundingBox="0 0 100 36">
      <n id="n1" p="20 18"/>
      <n id="n2" p="34 18" NodeType="Nickname">
        <t p="34 22" BoundingBox="34 8 93 24" LabelJustification="Left" LabelAlignment="Left" InterpretChemically="yes" UTF8Text="MgBr CuI Bipy">
          <s font="3" size="10" color="0" face="96">MgBr CuI Bipy</s>
        </t>
      </n>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("spaced chemical label")).expect("cdxml");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "n2"))
        .and_then(|node| node.label.as_ref())
        .expect("spaced label should import");

    assert_eq!(label.source_text.as_deref(), Some("MgBr CuI Bipy"));
    assert_eq!(label.text, "MgBr CuI Bipy");
    let display_text: String = label.runs.iter().map(|run| run.text.as_str()).collect();
    assert_eq!(display_text, "MgBr CuI Bipy");
    assert_eq!(
        label
            .meta
            .pointer("/sourceRuns/0/text")
            .and_then(serde_json::Value::as_str),
        Some("MgBr CuI Bipy")
    );

    let first_export = document_to_cdxml(&document);
    assert!(
        first_export.contains("UTF8Text=\"MgBr CuI Bipy\""),
        "export should keep visible/source spaces: {first_export}"
    );
    assert!(
        first_export.contains(">MgBr CuI Bipy</s>"),
        "exported text run should keep spaces: {first_export}"
    );
    let reimported = parse_cdxml_document(&first_export, Some("spaced chemical label"))
        .expect("reimport should parse");
    let second_export = document_to_cdxml(&reimported);
    assert_eq!(
        second_export, first_export,
        "CDXML spaced chemical label roundtrip should stabilize"
    );
}

#[test]
fn parse_cdxml_normal_face_attached_label_uses_connection_aware_group_layout() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 84 44">
    <fragment id="f1" BoundingBox="0 0 84 44">
      <n id="n1" p="20 12" NodeType="Fragment">
        <t p="16.40 15.90" BoundingBox="16.40 4.40 35.20 15.90" LabelJustification="Left" LabelAlignment="Above" UTF8Text="NTs">
          <s font="3" size="10" color="0" face="1">NTs</s>
        </t>
      </n>
      <n id="n2" p="8 24"/>
      <n id="n3" p="32 24"/>
      <b id="b1" B="n1" E="n2"/>
      <b id="b2" B="n1" E="n3"/>
      <n id="n4" p="64 12" NodeType="Fragment">
        <t p="64.00 15.90" BoundingBox="45.20 7.56 64.00 15.90" LabelJustification="Right" Justification="Right" LabelAlignment="Right" UTF8Text="NTs">
          <s font="3" size="10" color="0" face="1">NTs</s>
        </t>
      </n>
      <n id="n5" p="76 12"/>
      <n id="n6" p="60 24"/>
      <b id="b3" B="n4" E="n5"/>
      <b id="b4" B="n4" E="n6"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("normal face labels")).expect("cdxml should parse");
    let fragments: Vec<_> = document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .collect();
    assert_eq!(fragments.len(), 2);
    let stacked = fragments
        .iter()
        .flat_map(|fragment| fragment.nodes.iter())
        .find(|node| node.id == "n1")
        .and_then(|node| node.label.as_ref())
        .expect("stacked NTs label should import");
    assert_eq!(stacked.text, "Ts\nN");
    assert_eq!(stacked.lines, vec!["Ts", "N"]);
    assert_eq!(
        stacked
            .meta
            .pointer("/sourceRuns/0/script")
            .and_then(serde_json::Value::as_str),
        Some("normal")
    );
    assert_eq!(
        stacked
            .meta
            .pointer("/labelRecognition/canonicalLabel")
            .and_then(serde_json::Value::as_str),
        Some("NTs")
    );

    let mixed_direction = fragments
        .iter()
        .flat_map(|fragment| fragment.nodes.iter())
        .find(|node| node.id == "n4")
        .and_then(|node| node.label.as_ref())
        .expect("mixed-direction NTs label should import");
    assert_eq!(mixed_direction.text, "NTs");
    assert_eq!(
        mixed_direction
            .meta
            .pointer("/sourceRuns/0/script")
            .and_then(serde_json::Value::as_str),
        Some("normal")
    );
    assert_eq!(
        mixed_direction
            .meta
            .pointer("/labelRecognition/components/1/label")
            .and_then(serde_json::Value::as_str),
        Some("Ts")
    );
}

#[test]
fn parse_cdxml_parenthesized_attached_label_reverses_inner_groups() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 92 44">
    <fragment id="f1" BoundingBox="0 0 92 44">
      <n id="n1" p="20 16" NodeType="Fragment">
        <t p="20.00 19.90" BoundingBox="-34.00 8.40 20.00 19.90" LabelJustification="Right" Justification="Right" LabelAlignment="Right" UTF8Text="N(PhSO2)2">
          <s font="3" size="10" color="0" face="96">N(PhSO2)2</s>
        </t>
      </n>
      <n id="n2" p="36 16"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("parenthesized label")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should import");
    let node = fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .expect("N(PhSO2)2 node should import");
    let label = node.label.as_ref().expect("N(PhSO2)2 label should import");

    assert_eq!(label.source_text.as_deref(), Some("N(PhSO2)2"));
    assert_eq!(label.text, "(O2SPh)2N");
    let display_text: String = label.runs.iter().map(|run| run.text.as_str()).collect();
    assert_eq!(display_text, "(O2SPh)2N");
    assert_eq!(
        label
            .meta
            .pointer("/labelRecognition/canonicalLabel")
            .and_then(serde_json::Value::as_str),
        Some("N(PhSO2)2")
    );
    assert_eq!(
        label
            .meta
            .pointer("/labelRecognition/anchorAtom")
            .and_then(serde_json::Value::as_str),
        Some("N")
    );
    assert_eq!(label.align.as_deref(), Some("right"));
    assert_eq!(label.anchor.as_deref(), Some("end"));
}

#[test]
fn parse_cdxml_fixed_right_hydrocarbon_formula_preserves_order_and_subscripts() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="14.40" LineWidth="0.60" MarginWidth="1.60" LabelSize="10">
  <page id="p1" BoundingBox="0 0 80 36">
    <fragment id="f1" BoundingBox="0 0 80 36">
      <n id="alkyl" p="32 18" NodeType="Nickname">
        <t p="32 22" BoundingBox="-8 8 32 24" LabelJustification="Right" Justification="Right" LabelAlignment="Right" UTF8Text="C10H21">
          <s font="3" size="10" color="0" face="96">C10H21</s>
        </t>
      </n>
      <n id="c1" p="50 18"/>
      <b id="b1" B="alkyl" E="c1"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("fixed right C10H21")).expect("cdxml");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "alkyl"))
        .and_then(|node| node.label.as_ref())
        .expect("C10H21 label should import");

    assert_eq!(label.text, "C10H21");
    assert_eq!(label.source_text.as_deref(), Some("C10H21"));
    let subscript_text: String = label
        .runs
        .iter()
        .filter(|run| run.script.as_deref() == Some("subscript"))
        .map(|run| run.text.as_str())
        .collect();
    assert_eq!(subscript_text, "1021");

    let first_export = document_to_cdxml(&document);
    for expected in [
        "LabelJustification=\"Right\"",
        "Justification=\"Right\"",
        "LabelAlignment=\"Right\"",
        ">C10H21</s>",
    ] {
        assert!(
            first_export.contains(expected),
            "missing {expected}: {first_export}"
        );
    }
    let reimported =
        parse_cdxml_document(&first_export, Some("fixed right C10H21")).expect("reimport");
    let second_export = document_to_cdxml(&reimported);
    assert_eq!(
        second_export, first_export,
        "CDXML open/save must stabilize after export"
    );
}

#[test]
fn parse_cdxml_auto_right_alignment_reverses_hydrocarbon_as_one_group() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="14.40" LineWidth="0.60" MarginWidth="1.60" LabelSize="10">
  <page id="p1" BoundingBox="0 0 90 36">
    <fragment id="f1" BoundingBox="0 0 90 36">
      <n id="alkoxy" p="40 18" NodeType="Nickname">
        <t p="40 22" BoundingBox="-12 8 40 24" LabelJustification="Auto" Justification="Right" LabelAlignment="Right" UTF8Text="C10H21O3">
          <s font="3" size="10" color="0" face="96">C10H21O3</s>
        </t>
      </n>
      <n id="c1" p="58 18"/>
      <b id="b1" B="alkoxy" E="c1"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("auto C10H21O3")).expect("cdxml");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "alkoxy"))
        .and_then(|node| node.label.as_ref())
        .expect("C10H21O3 label should import");

    assert_eq!(label.source_text.as_deref(), Some("C10H21O3"));
    assert_eq!(label.text, "O3C10H21");
    assert_ne!(label.text, "O3H21C10");
}

#[test]
fn parse_cdxml_centered_multichar_label_uses_internal_center_anchor() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" LabelSize="10" color="0" bgcolor="1">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
    <color r="1" g="0" b="0"/>
  </colortable>
  <page id="p1" BoundingBox="0 0 60 30">
    <fragment id="f1" BoundingBox="0 0 60 30">
      <n id="n1" p="10 12" NodeType="Fragment" LabelDisplay="Center">
        <t p="10 16" BoundingBox="-80 -60 140 190" LabelJustification="Center" Justification="Center" LabelAlignment="Right" UTF8Text="CF3">
          <s font="3" size="10" color="2" face="96">CF3</s>
        </t>
      </n>
      <n id="n2" p="26 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("centered CF3 label"))
        .expect("centered CF3 CDXML should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should import");
    let node = fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .expect("CF3 node should import");
    let label = node.label.as_ref().expect("CF3 label should import");
    let bbox = label.bbox().expect("CF3 label should have a rebuilt bbox");

    assert_eq!(label.text, "CF3");
    assert_eq!(label.source_text.as_deref(), Some("CF3"));
    assert_eq!(label.align.as_deref(), Some("center"));
    assert_eq!(label.anchor.as_deref(), Some("middle"));
    assert_eq!(label.layout.as_deref(), Some("attached-group-center"));
    assert!(
        ((bbox[0] + bbox[2]) * 0.5 - node.position[0]).abs() < 0.01,
        "CDXML LabelDisplay=Center must center the internally rebuilt label box on the node: bbox={bbox:?}, node={node:?}"
    );
    assert_ne!(
        bbox,
        [-80.0, -60.0, 140.0, 190.0],
        "source BoundingBox must remain provenance, not active label geometry"
    );
    assert_eq!(
        label.meta.pointer("/import/cdxml/labelDisplay"),
        Some(&json!("Center"))
    );
}

#[test]
fn parse_cdxml_inferred_centered_metal_label_uses_the_same_baseline_as_neighboring_atoms() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM="http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" LabelSize="10" MarginWidth="1.6" color="0" bgcolor="1">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="p1" BoundingBox="0 0 80 40">
    <fragment id="f1" BoundingBox="0 0 80 40">
      <n id="left" p="16 20" Element="7" NumHydrogens="0">
        <t p="12.4 23.9" BoundingBox="12.4 15.7 19.6 24.6" LabelJustification="Left" UTF8Text="N">
          <s font="3" size="10" color="0" face="96">N</s>
        </t>
      </n>
      <n id="metal" p="30.4 20" Element="46" NumHydrogens="0">
        <t p="30.4 23.9" BoundingBox="24.3 14.9 36.5 26.4" LabelJustification="Center" Justification="Center" LabelAlignment="Center" UTF8Text="Pd">
          <s font="3" size="10" color="0" face="96">Pd</s>
        </t>
      </n>
      <n id="right" p="44.8 20" Element="7" NumHydrogens="0">
        <t p="41.2 23.9" BoundingBox="41.2 15.7 48.4 24.6" LabelJustification="Left" UTF8Text="N">
          <s font="3" size="10" color="0" face="96">N</s>
        </t>
      </n>
      <b id="b1" B="left" E="metal"/>
      <b id="b2" B="metal" E="right"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("inferred centered Pd label"))
        .expect("inferred centered Pd CDXML should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should import");
    let label_for = |id: &str| {
        fragment
            .nodes
            .iter()
            .find(|node| node.id == id)
            .and_then(|node| node.label.as_ref())
            .expect("node label should import")
    };
    let left = label_for("left");
    let metal = label_for("metal");
    let right = label_for("right");
    let left_baseline = left.position.expect("left baseline")[1];
    let metal_baseline = metal.position.expect("metal baseline")[1];
    let right_baseline = right.position.expect("right baseline")[1];

    assert_eq!(metal.align.as_deref(), Some("center"));
    assert_eq!(metal.anchor.as_deref(), Some("middle"));
    assert_eq!(metal.layout.as_deref(), Some("attached-group-center"));
    assert_eq!(
        metal.meta.pointer("/import/cdxml/labelDisplay"),
        Some(&serde_json::Value::Null),
        "center alignment inferred from justification must not invent LabelDisplay"
    );
    assert_close(left_baseline, 23.9);
    assert_close(metal_baseline, 23.9);
    assert_close(right_baseline, 23.9);
}

#[test]
fn parse_cdxml_right_aligned_metal_oxidation_label_reverses_visible_order() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 90 60">
    <fragment id="f1" BoundingBox="0 0 90 60">
      <n id="cu" p="40 24" Element="29" NumHydrogens="0">
        <t p="42.00 27.90" BoundingBox="16.00 16.40 42.00 27.90" LabelAlignment="Right" LabelJustification="Unspecified">
          <s font="3" size="10" color="0" face="96">Cu(II)</s>
        </t>
      </n>
      <n id="n1" p="58 12" Element="7"/>
      <n id="n2" p="58 24" Element="7"/>
      <n id="n3" p="58 36" Element="7"/>
      <b id="b1" B="cu" E="n1"/>
      <b id="b2" B="cu" E="n2"/>
      <b id="b3" B="cu" E="n3"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("metal oxidation label")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should import");
    let copper = fragment
        .nodes
        .iter()
        .find(|node| node.id == "cu")
        .expect("copper node should import");
    let label = copper.label.as_ref().expect("Cu(II) label should import");

    assert_eq!(label.source_text.as_deref(), Some("Cu(II)"));
    assert_eq!(label.text, "(II)Cu");
    assert_eq!(
        copper
            .meta
            .pointer("/labelRecognition/status")
            .and_then(serde_json::Value::as_str),
        Some("recognized")
    );
    assert_eq!(
        copper
            .meta
            .pointer("/labelRecognition/source")
            .and_then(serde_json::Value::as_str),
        Some("element-oxidation-state-label")
    );
    assert_eq!(copper.num_hydrogens, 0);
}

#[test]
fn parse_cdxml_metal_containing_chemical_label_does_not_mark_invalid() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 80 40">
    <fragment id="f1" BoundingBox="0 0 80 40">
      <n id="cu" p="30 20" Element="29" NumHydrogens="0">
        <t p="30.00 23.90" BoundingBox="30.00 12.40 76.00 23.90" LabelAlignment="Left" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">Cu(NO3)2</s>
        </t>
      </n>
      <n id="n1" p="54 20" Element="7"/>
      <b id="b1" B="cu" E="n1"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("metal salt label")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should import");
    let copper = fragment
        .nodes
        .iter()
        .find(|node| node.id == "cu")
        .expect("copper node should import");

    assert_eq!(
        copper
            .meta
            .pointer("/labelRecognition/status")
            .and_then(serde_json::Value::as_str),
        Some("recognized")
    );
    assert_eq!(
        copper
            .meta
            .pointer("/labelRecognition/source")
            .and_then(serde_json::Value::as_str),
        Some("metal-containing-chemical-text")
    );
    assert_eq!(
        copper
            .meta
            .pointer("/labelRecognition/groupKind")
            .and_then(serde_json::Value::as_str),
        Some("chemical-text")
    );
    assert!(copper.meta.pointer("/labelRecognition/expansion").is_none());
}

#[test]
fn parse_cdxml_label_display_overrides_auto_reversal_without_losing_chemistry() {
    let cdxml = include_str!("fixtures/label-display-modes.cdxml");
    let document = parse_cdxml_document(cdxml, Some("label display modes"))
        .expect("label display mode CDXML should parse");
    let mut labels: std::collections::BTreeMap<
        String,
        Vec<(&chemsema_engine::Node, &chemsema_engine::NodeLabel)>,
    > = std::collections::BTreeMap::new();
    for resource in document.resources.values() {
        if let Some(fragment) = resource.data.as_fragment() {
            for node in &fragment.nodes {
                if let Some(label) = &node.label {
                    if label.source_text.as_deref() == Some("CF3") {
                        let display = label
                            .meta
                            .pointer("/import/cdxml/labelDisplay")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("Auto");
                        labels
                            .entry(display.to_string())
                            .or_default()
                            .push((node, label));
                    }
                }
            }
        }
    }

    let auto_label = labels["Auto"]
        .iter()
        .map(|(_, label)| *label)
        .find(|label| label.text == "F3C")
        .expect("a right-aligned CF3 without LabelDisplay should use chemical group reversal");
    assert_eq!(auto_label.source_text.as_deref(), Some("CF3"));

    for (_, right_label) in &labels["Right"] {
        assert_eq!(right_label.source_text.as_deref(), Some("CF3"));
        assert_eq!(right_label.text, "CF3");
        assert_eq!(right_label.align.as_deref(), Some("right"));
        assert_eq!(right_label.anchor.as_deref(), Some("end"));
    }

    let (center_node, center_label) = labels["Center"]
        .iter()
        .find(|(_, label)| label.text == "CF3")
        .copied()
        .expect("LabelDisplay=Center CF3 should import");
    let center_box = center_label.bbox().expect("center label box");
    assert_eq!(center_label.source_text.as_deref(), Some("CF3"));
    assert_eq!(center_label.text, "CF3");
    assert_eq!(center_label.align.as_deref(), Some("center"));
    assert_eq!(center_label.anchor.as_deref(), Some("middle"));
    assert!(
        ((center_box[0] + center_box[2]) * 0.5 - center_node.position[0]).abs() < 0.01,
        "LabelDisplay=Center should center the rebuilt label box on the node"
    );

    for (_, left_label) in &labels["Left"] {
        assert_eq!(left_label.source_text.as_deref(), Some("CF3"));
        assert_eq!(left_label.text, "CF3");
        assert_eq!(left_label.align.as_deref(), Some("left"));
        assert_eq!(left_label.anchor.as_deref(), Some("start"));
    }
}

#[test]
fn parse_cdxml_attached_sulfur_label_uses_elliptical_clip_geometry() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 40 24">
    <fragment id="f1" BoundingBox="0 0 40 24">
      <n id="n1" p="10 12" Element="16">
        <t p="6.40 15.90" BoundingBox="6.40 7.56 13.10 15.90" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">S</s>
        </t>
      </n>
      <n id="n2" p="24 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("sulfur ellipse clip")).expect("cdxml should parse");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "n1"))
        .and_then(|node| node.label.as_ref())
        .expect("S label should import");

    assert!(
        !label.glyph_polygons.is_empty(),
        "sulfur label should populate glyph polygons"
    );
    assert!(
        label
            .glyph_polygons
            .iter()
            .any(|polygon| polygon.len() >= 16),
        "sulfur clipping should include an ellipse-like polygon for S; text={:?}, polygons={:?}",
        label.text,
        label.glyph_polygons
    );
}

#[test]
fn parse_cdxml_double_bond_spacing_uses_bond_spacing_percent() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" BondSpacing="18" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <fragment id="2" BoundingBox="9 8 26 12">
      <n id="3" p="10 10"/>
      <n id="4" p="24.4 10"/>
      <b id="5" B="3" E="4" Order="2"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("spacing")).expect("cdxml should parse");
    let primitives = render_document(&document);
    let mut center_ys: Vec<f64> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_mol_001") =>
            {
                bond_axis_from_points(points).map(|(from, to)| (from.y + to.y) * 0.5)
            }
            _ => None,
        })
        .collect();
    center_ys.sort_by(f64::total_cmp);

    assert_eq!(center_ys.len(), 2, "{center_ys:?}");
    let center_distance = center_ys[1] - center_ys[0];
    let expected_center_distance = 14.4 * 0.18;
    assert!(
        (center_distance - expected_center_distance).abs() < 0.001,
        "{center_distance}"
    );
}

#[test]
fn parse_cdxml_node_labels_use_internal_attached_layout() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 120 120">
      <n id="3" p="50 50" Element="7">
        <t id="30" p="0 0" BoundingBox="0 0 100 100" UTF8Text="NH">
          <s font="3" size="10" color="0">NH</s>
        </t>
      </n>
      <n id="4" p="42 65"/>
      <n id="5" p="58 65"/>
      <b id="6" B="3" E="4"/>
      <b id="7" B="3" E="5"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("labels")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("import should create molecule fragment resource");
    let node = fragment
        .nodes
        .iter()
        .find(|node| node.id == "3")
        .expect("labeled node should import");
    let label = node.label.as_ref().expect("node label should import");

    assert_eq!(node.position, [50.0, 50.0]);
    assert_eq!(label.attachment.as_deref(), Some("node"));
    assert_eq!(label.anchor.as_deref(), Some("start"));
    assert_eq!(label.lines, vec!["H".to_string(), "N".to_string()]);
    assert_eq!(label.layout.as_deref(), Some("attached-group-above"));
    assert!(
        !label.glyph_polygons.is_empty(),
        "internal glyph geometry should be generated"
    );
    let glyph_center = |index: usize| {
        let polygon = label
            .glyph_polygons
            .get(index)
            .expect("expected glyph polygon");
        let bounds = polygon.iter().fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |mut bounds, point| {
                bounds[0] = bounds[0].min(point[0]);
                bounds[1] = bounds[1].min(point[1]);
                bounds[2] = bounds[2].max(point[0]);
                bounds[3] = bounds[3].max(point[1]);
                bounds
            },
        );
        Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5)
    };
    let hydrogen_center = glyph_center(0);
    let nitrogen_center = glyph_center(1);
    assert!(
        (nitrogen_center.x - node.position[0]).abs() < 0.01,
        "stacked NH labels should anchor the original first atom glyph horizontally to the node: H={hydrogen_center:?}, N={nitrogen_center:?}, node={:?}",
        node.position
    );
    assert!(
        hydrogen_center.y < nitrogen_center.y,
        "hydrogen should render above nitrogen for an above-stacked NH label"
    );
    let box_value = label.box_value.expect("internal label box should exist");
    assert!(
        box_value[2] - box_value[0] < 30.0 && box_value[3] - box_value[1] < 30.0,
        "{box_value:?}"
    );
    assert_ne!(box_value, [0.0, 0.0, 100.0, 100.0]);

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("Element=\"7\""), "{exported}");
    assert!(exported.contains("NumHydrogens=\"1\""), "{exported}");
    assert!(exported.contains("LabelAlignment=\"Above\""), "{exported}");
    assert!(exported.contains("LineStarts=\"2 4\""), "{exported}");
    assert!(exported.contains("BoundingBox="), "{exported}");
    assert!(
        exported.contains("InterpretChemically=\"yes\""),
        "{exported}"
    );
    assert!(exported.contains("face=\"96\""), "{exported}");

    let reimported =
        parse_cdxml_document(&exported, Some("labels export")).expect("export should parse");
    let reimported_fragment = reimported
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("reimport should create molecule fragment resource");
    let reimported_node = reimported_fragment
        .nodes
        .iter()
        .find(|node| node.atomic_number == 7)
        .expect("nitrogen node should reimport as nitrogen");
    let reimported_label = reimported_node
        .label
        .as_ref()
        .expect("nitrogen label should reimport");
    assert_eq!(reimported_node.num_hydrogens, 1);
    assert_eq!(
        reimported_label.lines,
        vec!["H".to_string(), "N".to_string()]
    );
    assert_eq!(
        reimported_label.layout.as_deref(),
        Some("attached-group-above")
    );
}

#[test]
fn parse_cdxml_left_justification_does_not_override_connection_aware_label_layout() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50" LabelJustification="Auto">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 180 80">
      <n id="right" p="20 40" Element="7" NumHydrogens="1">
        <t id="right_text" p="20 44" BoundingBox="14 32 26 46" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">NH</s>
        </t>
      </n>
      <n id="right_up" p="34 32"/>
      <n id="right_down" p="34 48"/>
      <b id="right_bond_up" B="right" E="right_up"/>
      <b id="right_bond_down" B="right" E="right_down"/>

      <n id="below" p="80 24" Element="7" NumHydrogens="1">
        <t id="below_text" p="80 28" BoundingBox="74 16 86 30" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">NH</s>
        </t>
      </n>
      <n id="below_left" p="72 40"/>
      <n id="below_right" p="88 40"/>
      <b id="below_bond_left" B="below" E="below_left"/>
      <b id="below_bond_right" B="below" E="below_right"/>

      <n id="above" p="140 48" Element="7" NumHydrogens="1">
        <t id="above_text" p="140 52" BoundingBox="134 40 146 54" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">NH</s>
        </t>
      </n>
      <n id="above_left" p="132 32"/>
      <n id="above_right" p="148 32"/>
      <b id="above_bond_left" B="above" E="above_left"/>
      <b id="above_bond_right" B="above" E="above_right"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("left-justified automatic labels"))
        .expect("CDXML should parse");
    let label = |node_id: &str| {
        document
            .resources
            .iter()
            .filter_map(|(_, resource)| resource.data.as_fragment())
            .flat_map(|fragment| fragment.nodes.iter())
            .find(|node| node.id == node_id)
            .and_then(|node| node.label.as_ref())
            .unwrap_or_else(|| panic!("missing label for {node_id}"))
    };

    let reversed = label("right");
    assert_eq!(reversed.source_text.as_deref(), Some("NH"));
    assert_eq!(reversed.text, "HN");
    assert_eq!(reversed.layout.as_deref(), Some("attached-group"));
    assert_eq!(
        reversed
            .meta
            .pointer("/import/cdxml/labelJustification")
            .and_then(serde_json::Value::as_str),
        Some("Left")
    );

    let stacked_above = label("below");
    assert_eq!(stacked_above.lines, ["H", "N"]);
    assert_eq!(
        stacked_above.layout.as_deref(),
        Some("attached-group-above")
    );

    let stacked_below = label("above");
    assert_eq!(stacked_below.lines, ["N", "H"]);
    assert_eq!(
        stacked_below.layout.as_deref(),
        Some("attached-group-below")
    );
}

#[test]
fn parse_cdxml_explicit_label_display_still_overrides_connection_aware_layout() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" LabelJustification="Auto">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 60 40">
      <n id="fixed" p="16 20" Element="7" NumHydrogens="1" LabelDisplay="Left">
        <t id="fixed_text" p="16 24" BoundingBox="10 12 22 26" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">NH</s>
        </t>
      </n>
      <n id="right_up" p="32 12"/>
      <n id="right_down" p="32 28"/>
      <b id="bond_up" B="fixed" E="right_up"/>
      <b id="bond_down" B="fixed" E="right_down"/>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("fixed left label display")).expect("CDXML should parse");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "fixed"))
        .and_then(|node| node.label.as_ref())
        .expect("fixed label should import");

    assert_eq!(label.source_text.as_deref(), Some("NH"));
    assert_eq!(label.text, "NH");
    assert_eq!(label.align.as_deref(), Some("left"));
    assert_eq!(label.anchor.as_deref(), Some("start"));
}

#[test]
fn parse_cdxml_label_fields_keep_their_official_layout_roles() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" LabelJustification="Auto">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 190 110">
      <n id="alignment_right" p="42 20" Element="7" NumHydrogens="1">
        <t p="42 24" LabelAlignment="Right"><s font="3" size="10" face="96">NH</s></t>
      </n>
      <n id="ar_up" p="26 12"/><n id="ar_down" p="26 28"/>
      <b id="ar_b1" B="alignment_right" E="ar_up"/><b id="ar_b2" B="alignment_right" E="ar_down"/>

      <n id="alignment_above" p="76 20" Element="7" NumHydrogens="1">
        <t p="76 24" LabelAlignment="Above"><s font="3" size="10" face="96">NH</s></t>
      </n>
      <n id="aa_up" p="92 12"/><n id="aa_down" p="92 28"/>
      <b id="aa_b1" B="alignment_above" E="aa_up"/><b id="aa_b2" B="alignment_above" E="aa_down"/>

      <n id="display_above" p="112 20" Element="7" NumHydrogens="1" LabelDisplay="Above">
        <t p="112 12" LabelAlignment="Above"><s font="3" size="10" face="96">NH</s></t>
      </n>
      <n id="da_up" p="128 12"/><n id="da_down" p="128 28"/>
      <b id="da_b1" B="display_above" E="da_up"/><b id="da_b2" B="display_above" E="da_down"/>

      <n id="authored_lines" p="154 20" NodeType="Fragment">
        <t p="154 24" LineStarts="4 7"><s font="3" size="10" face="96">Cl2&#10;Zr</s></t>
      </n>
      <n id="ml_up" p="170 12"/><n id="ml_down" p="170 28"/>
      <b id="ml_b1" B="authored_lines" E="ml_up"/><b id="ml_b2" B="authored_lines" E="ml_down"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("label field roles")).expect("CDXML");
    let label = |node_id: &str| {
        document
            .resources
            .values()
            .filter_map(|resource| resource.data.as_fragment())
            .flat_map(|fragment| fragment.nodes.iter())
            .find(|node| node.id == node_id)
            .and_then(|node| node.label.as_ref())
            .unwrap_or_else(|| panic!("missing {node_id}"))
    };

    assert_eq!(label("alignment_right").text, "NH");
    assert_eq!(label("alignment_above").text, "HN");
    assert!(label("alignment_above").lines.is_empty());
    assert_eq!(label("display_above").text, "NH");
    assert!(label("display_above").lines.is_empty());
    assert_eq!(label("authored_lines").text, "Cl2\nZr");
    assert_eq!(label("authored_lines").lines, ["Cl2", "Zr"]);
}

#[test]
fn cdxml_caption_fields_override_obsolete_text_fields_and_roundtrip() {
    let source = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML CaptionJustification="Left">
  <page id="1" BoundingBox="0 0 120 60">
    <t id="2" p="80 30" BoundingBox="20 10 80 34"
       CaptionJustification="Right" Justification="Left" LabelJustification="Center"
       LineHeight="9" CaptionLineHeight="auto" WordWrapWidth="72" LineStarts="6 12">
      <s font="3" size="10">alpha&#10;beta</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(source, Some("caption fields")).expect("CDXML");
    let text = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object");
    assert_eq!(text.payload.extra.get("align"), Some(&json!("right")));
    assert_eq!(
        text.meta.pointer("/import/cdxml/captionLineHeight"),
        Some(&json!("auto"))
    );

    let exported = document_to_cdxml(&document);
    for expected in [
        "CaptionJustification=\"Right\"",
        "Justification=\"Left\"",
        "CaptionLineHeight=\"auto\"",
        "LineHeight=\"9\"",
        "WordWrapWidth=\"72\"",
        "LineStarts=\"6 12\"",
    ] {
        assert!(
            exported.contains(expected),
            "missing {expected}: {exported}"
        );
    }
    assert!(!exported.contains("LabelJustification=\"Center\""));
}

#[test]
fn parse_cdxml_preserves_explicit_zero_hydrogens_on_imported_nitrogen() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50" LabelSize="10">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 80 40">
      <n id="1" p="20 20"/>
      <n id="2" p="40 20" Element="7" NumHydrogens="0">
        <t id="20" p="36 24" BoundingBox="36 16 44 25" LabelAlignment="Left" LabelJustification="Left">
          <s font="3" size="10" face="96" color="0">N</s>
        </t>
      </n>
      <n id="3" p="60 20"/>
      <b id="4" B="1" E="2"/>
      <b id="5" B="2" E="3"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("explicit h0")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("import should create molecule fragment resource");
    let nitrogen = fragment
        .nodes
        .iter()
        .find(|node| node.id == "2")
        .expect("nitrogen node should import");

    assert_eq!(nitrogen.num_hydrogens, 0);
    assert_eq!(
        nitrogen
            .meta
            .pointer("/import/cdxml/explicitNumHydrogens")
            .and_then(|value| value.as_u64()),
        Some(0)
    );
    assert_eq!(
        nitrogen
            .meta
            .get("labelRecognition")
            .and_then(|meta| meta.get("status"))
            .and_then(|status| status.as_str()),
        None
    );
    assert_eq!(
        nitrogen.label.as_ref().map(|label| label.text.as_str()),
        Some("N")
    );
}

#[test]
fn neutral_second_period_nitrogen_does_not_use_five_valence_to_add_hydrogen() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50" LabelSize="10">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 90 60">
      <n id="1" p="20 20"/>
      <n id="2" p="40 20" Element="7">
        <t id="20" p="36 24" BoundingBox="36 16 44 25" LabelAlignment="Left" LabelJustification="Left">
          <s font="3" size="10" face="96" color="0">N</s>
        </t>
      </n>
      <n id="3" p="60 20"/>
      <n id="4" p="40 40"/>
      <b id="5" B="1" E="2" Order="2"/>
      <b id="6" B="2" E="3"/>
      <b id="7" B="2" E="4"/>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("neutral tetravalent n")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("import should create molecule fragment resource");
    let nitrogen = fragment
        .nodes
        .iter()
        .find(|node| node.id == "2")
        .expect("nitrogen node should import");

    assert_eq!(nitrogen.num_hydrogens, 0);
    assert_eq!(
        nitrogen
            .meta
            .get("labelRecognition")
            .and_then(|meta| meta.get("status"))
            .and_then(|status| status.as_str()),
        Some("invalid")
    );
}

#[test]
fn neutral_second_period_boron_four_connection_label_is_invalid() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50" LabelSize="10">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 100 80">
      <n id="1" p="20 40"/>
      <n id="2" p="40 40" Element="5">
        <t id="20" p="36 44" BoundingBox="36 34 44 45" LabelAlignment="Left" LabelJustification="Left">
          <s font="3" size="10" face="96" color="0">B</s>
        </t>
      </n>
      <n id="3" p="60 40"/>
      <n id="4" p="40 20"/>
      <n id="5" p="40 60"/>
      <b id="6" B="1" E="2"/>
      <b id="7" B="2" E="3"/>
      <b id="8" B="2" E="4"/>
      <b id="9" B="2" E="5"/>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("neutral tetravalent b")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("import should create molecule fragment resource");
    let boron = fragment
        .nodes
        .iter()
        .find(|node| node.id == "2")
        .expect("boron node should import");

    assert_eq!(
        boron
            .meta
            .get("labelRecognition")
            .and_then(|meta| meta.get("status"))
            .and_then(|status| status.as_str()),
        Some("invalid")
    );
}

#[test]
fn second_period_carbon_label_five_connection_is_invalid() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50" LabelSize="10">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 120 80">
      <n id="1" p="20 40"/>
      <n id="2" p="50 40" Element="6">
        <t id="20" p="46 44" BoundingBox="46 34 54 45" LabelAlignment="Left" LabelJustification="Left">
          <s font="3" size="10" face="96" color="0">C</s>
        </t>
      </n>
      <n id="3" p="80 40"/>
      <n id="4" p="50 15"/>
      <n id="5" p="50 65"/>
      <n id="6" p="70 60"/>
      <b id="7" B="1" E="2"/>
      <b id="8" B="2" E="3"/>
      <b id="9" B="2" E="4"/>
      <b id="10" B="2" E="5"/>
      <b id="11" B="2" E="6"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("pentavalent c")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("import should create molecule fragment resource");
    let carbon = fragment
        .nodes
        .iter()
        .find(|node| node.id == "2")
        .expect("carbon node should import");

    assert_eq!(
        carbon
            .meta
            .get("labelRecognition")
            .and_then(|meta| meta.get("status"))
            .and_then(|status| status.as_str()),
        Some("invalid")
    );
}

#[test]
fn metal_coordination_does_not_create_implicit_hydrogen_on_pyridine_nitrogen() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50" LabelSize="10">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 110 60">
      <n id="1" p="20 20"/>
      <n id="2" p="40 20" Element="7">
        <t id="20" p="36 24" BoundingBox="36 16 44 25" LabelAlignment="Left" LabelJustification="Left">
          <s font="3" size="10" face="96" color="0">N</s>
        </t>
      </n>
      <n id="3" p="60 20"/>
      <n id="4" p="40 40" Element="29">
        <t id="21" p="38 44" BoundingBox="38 34 50 45" LabelAlignment="Center" LabelJustification="Center">
          <s font="3" size="10" face="96" color="0">Cu</s>
        </t>
      </n>
      <b id="5" B="1" E="2" Order="2"/>
      <b id="6" B="2" E="3"/>
      <b id="7" B="2" E="4"/>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("coordinated pyridine n")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("import should create molecule fragment resource");
    let nitrogen = fragment
        .nodes
        .iter()
        .find(|node| node.id == "2")
        .expect("nitrogen node should import");

    assert_eq!(nitrogen.num_hydrogens, 0);
    assert_eq!(
        nitrogen
            .meta
            .get("labelRecognition")
            .and_then(|meta| meta.get("status"))
            .and_then(|status| status.as_str()),
        None
    );
}

#[test]
fn parse_cdxml_matches_default_and_acs_double_bond_spacing_samples() {
    for (fixture, expected_normal, expected_bold, expected_widths) in [
        ("db.cdxml", 3.6, 5.1, [1.0, 4.0]),
        ("db-acs.cdxml", 2.592, 3.292, [0.6, 2.0]),
    ] {
        let Some(cdxml) = read_optional_cdxml_fixture(fixture) else {
            continue;
        };
        let document = parse_cdxml_document(&cdxml, Some(fixture)).expect("cdxml should parse");
        let primitives = render_document(&document);

        let normal = imported_vertical_line_metrics(&primitives, "obj_mol_001");
        assert_line_spacing(&normal, expected_normal, fixture);
        assert_line_widths(&normal, expected_widths[0], expected_widths[0], fixture);

        let dashed_solid = imported_vertical_line_metrics(&primitives, "obj_mol_002");
        assert_line_spacing(&dashed_solid, expected_normal, fixture);
        let dashed_solid_bond = imported_fragment_bond(&document, "obj_mol_002", "9");
        assert_eq!(dashed_solid_bond.order, 2);
        assert_eq!(
            dashed_solid_bond.line_styles.right,
            chemsema_engine::BondLinePattern::Dashed
        );

        let bold = imported_vertical_line_metrics(&primitives, "obj_mol_003");
        assert_line_spacing(&bold, expected_bold, fixture);
        assert_line_widths(&bold, expected_widths[0], expected_widths[1], fixture);

        let dashed = imported_vertical_line_metrics(&primitives, "obj_mol_004");
        assert_line_spacing(&dashed, expected_normal, fixture);
        let dashed_bond = imported_fragment_bond(&document, "obj_mol_004", "17");
        assert_eq!(dashed_bond.order, 2);
        assert_eq!(
            dashed_bond.line_styles.left,
            chemsema_engine::BondLinePattern::Dashed
        );
        assert_eq!(
            dashed_bond.line_styles.right,
            chemsema_engine::BondLinePattern::Dashed
        );
    }
}

#[test]
fn parse_cdxml_double_bond_spacing_uses_chemdraw_line_width_floor() {
    for (name, line_width, bond_length, bond_spacing, expected_center_distance) in [
        ("acs", 0.60, 14.40, 18.0, 2.592),
        ("default", 1.00, 30.00, 12.0, 3.600),
        ("thick-short", 1.98, 22.68, 12.0, 4.950),
    ] {
        let end_x = 100.0 + bond_length;
        let cdxml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML CreationProgram="ChemDraw 22.2.0.3300" FractionalWidths="yes" LineWidth="{line_width:.2}" BoldWidth="4.00" BondLength="{bond_length:.2}" BondSpacing="{bond_spacing:.0}" HashSpacing="2.70" MarginWidth="2.00" LabelSize="10">
  <page id="1" BoundingBox="0 0 200 100">
    <fragment id="2" BoundingBox="90 90 140 110">
      <n id="3" p="100.00 100.00"/>
      <n id="4" p="{end_x:.2} 100.00"/>
      <b id="5" B="3" E="4" Order="2"/>
    </fragment>
  </page>
</CDXML>"#
        );
        let document = parse_cdxml_document(&cdxml, Some(name)).expect("cdxml should parse");
        let rendered = imported_double_bond_center_spacing(&document, "obj_mol_001");
        let formula = imported_double_bond_formula_spacing(&document, "obj_mol_001");

        assert!(
            (rendered - expected_center_distance).abs() < 0.01,
            "{name}: expected {expected_center_distance}, rendered {rendered}"
        );
        assert!(
            (formula - expected_center_distance).abs() < 0.01,
            "{name}: expected {expected_center_distance}, formula {formula}"
        );
    }
}

#[test]
fn parse_cdxml_recognizes_fractional_dashed_double_bond() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 50 50">
    <fragment id="f1" BoundingBox="0 0 50 50">
      <n id="n1" p="24 10"/>
      <n id="n2" p="24 34"/>
      <b id="b1" B="n1" E="n2" Order="1.5" Display2="Dash"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("fractional dashed double")).expect("cdxml should parse");
    let bond = imported_fragment_bond(&document, "obj_mol_001", "b1");

    assert_eq!(bond.order, 2);
    let double = bond
        .double
        .as_ref()
        .expect("fractional bond should render as a double bond");
    assert_eq!(
        double.placement,
        chemsema_engine::DoubleBondPlacement::Center
    );
    assert!(
        !double.frozen,
        "Display2 without DoublePosition should keep automatic placement"
    );
    assert_eq!(
        bond.line_styles.right,
        chemsema_engine::BondLinePattern::Dashed
    );
    assert_eq!(
        bond.meta
            .pointer("/import/cdxml/display2")
            .and_then(serde_json::Value::as_str),
        Some("Dash")
    );

    let primitives = render_document(&document);
    let bond_polygons: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentBond,
                object_id,
                bond_id,
                points,
                ..
            } if object_id.as_deref() == Some("obj_mol_001")
                && bond_id.as_deref() == Some("b1") =>
            {
                Some(points)
            }
            _ => None,
        })
        .collect();
    assert!(
        bond_polygons.len() > 2,
        "virtual/solid double bond should render one solid line plus black dash segments: {bond_polygons:?}"
    );
    let lengths: Vec<_> = bond_polygons
        .iter()
        .filter_map(|points| bond_axis_length(points))
        .collect();
    assert!(
        lengths.iter().any(|length| *length > 18.0)
            && lengths.iter().any(|length| *length > 2.0 && *length < 3.0),
        "Display2=\"Dash\" should use the same evenly distributed black segments as dashed bonds: {lengths:?}"
    );
    assert!(
        !primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentKnockout,
                object_id,
                node_id: None,
                ..
            } if object_id.as_deref() == Some("obj_mol_001")
        )),
        "dashed double bonds should draw black dash segments directly, not a solid line with knockout gaps: {primitives:?}"
    );
    let exported = document_to_cdxml(&document);
    assert!(exported.contains("Display2=\"Dash\""), "{exported}");
}

#[test]
fn parse_cdxml_double_bond_spacing_scales_with_actual_bond_length() {
    for (fixture, expected_spacings) in [
        (
            "db-chang.cdxml",
            [
                ("obj_mol_001", 9.0002),
                ("obj_mol_002", 12.8413),
                ("obj_mol_003", 14.5250),
                ("obj_mol_004", 9.5205),
            ],
        ),
        (
            "db-acs-chang.cdxml",
            [
                ("obj_mol_001", 4.7411),
                ("obj_mol_002", 5.7277),
                ("obj_mol_003", 5.9441),
                ("obj_mol_004", 5.2895),
            ],
        ),
    ] {
        let Some(cdxml) = read_optional_cdxml_fixture(fixture) else {
            continue;
        };
        let document = parse_cdxml_document(&cdxml, Some(fixture)).expect("cdxml should parse");

        for (object_id, expected) in expected_spacings {
            let rendered = imported_double_bond_center_spacing(&document, object_id);
            let formula = imported_double_bond_formula_spacing(&document, object_id);
            assert!(
                (rendered - expected).abs() < 0.01,
                "{fixture} {object_id}: expected {expected}, rendered {rendered}"
            );
            assert!(
                (formula - expected).abs() < 0.01,
                "{fixture} {object_id}: expected {expected}, formula {formula}"
            );
        }
    }
}

fn imported_fragment_bond<'a>(
    document: &'a ChemSemaDocument,
    object_id: &str,
    bond_id: &str,
) -> &'a chemsema_engine::Bond {
    let object = document
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .expect("imported molecule object should exist");
    let resource_ref = object
        .payload
        .resource_ref
        .as_deref()
        .expect("molecule object should reference fragment resource");
    let fragment = document
        .resources
        .get(resource_ref)
        .and_then(|resource| resource.data.as_fragment())
        .expect("fragment resource should exist");
    fragment
        .bonds
        .iter()
        .find(|bond| bond.id == bond_id)
        .expect("bond should exist")
}

fn imported_double_bond_center_spacing(document: &ChemSemaDocument, object_id: &str) -> f64 {
    let object = document
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .expect("imported molecule object should exist");
    let resource_ref = object
        .payload
        .resource_ref
        .as_deref()
        .expect("molecule object should reference fragment resource");
    let fragment = document
        .resources
        .get(resource_ref)
        .and_then(|resource| resource.data.as_fragment())
        .expect("fragment resource should exist");
    let bond = fragment
        .bonds
        .first()
        .expect("fixture fragment has one bond");
    let begin = fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.begin)
        .expect("begin node should exist");
    let end = fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.end)
        .expect("end node should exist");
    let begin = chemsema_engine::Point::new(
        object.transform.translate[0] + begin.position[0],
        object.transform.translate[1] + begin.position[1],
    );
    let end = chemsema_engine::Point::new(
        object.transform.translate[0] + end.position[0],
        object.transform.translate[1] + end.position[1],
    );
    let dx = end.x - begin.x;
    let dy = end.y - begin.y;
    let length = dx.hypot(dy);
    let normal = chemsema_engine::Point::new(-dy / length, dx / length);
    let mut projections: Vec<f64> = render_document(document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id: primitive_object_id,
                points,
                ..
            } if role == RenderRole::DocumentBond
                && primitive_object_id.as_deref() == Some(object_id) =>
            {
                let projection = points
                    .iter()
                    .map(|point| point.x * normal.x + point.y * normal.y)
                    .sum::<f64>()
                    / points.len() as f64;
                Some(projection)
            }
            _ => None,
        })
        .collect();
    projections.sort_by(f64::total_cmp);
    assert!(
        projections.len() >= 2,
        "{object_id}: expected at least two rendered bond line polygons"
    );
    let split = projections
        .windows(2)
        .enumerate()
        .max_by(|(_, left), (_, right)| {
            (left[1] - left[0])
                .abs()
                .total_cmp(&(right[1] - right[0]).abs())
        })
        .map(|(index, _)| index + 1)
        .expect("projection split should exist");
    let first = projections[..split].iter().sum::<f64>() / split as f64;
    let second = projections[split..].iter().sum::<f64>() / (projections.len() - split) as f64;
    (second - first).abs()
}

fn imported_double_bond_formula_spacing(document: &ChemSemaDocument, object_id: &str) -> f64 {
    let object = document
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .expect("imported molecule object should exist");
    let resource_ref = object
        .payload
        .resource_ref
        .as_deref()
        .expect("molecule object should reference fragment resource");
    let fragment = document
        .resources
        .get(resource_ref)
        .and_then(|resource| resource.data.as_fragment())
        .expect("fragment resource should exist");
    let bond = fragment
        .bonds
        .first()
        .expect("fixture fragment has one bond");
    let begin = fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.begin)
        .expect("begin node should exist");
    let end = fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.end)
        .expect("end node should exist");
    let length = chemsema_engine::Point::new(begin.position[0], begin.position[1]).distance(
        chemsema_engine::Point::new(end.position[0], end.position[1]),
    );
    let ratio = bond
        .bond_spacing
        .expect("cdxml fixture should import bond spacing")
        / 100.0;
    let stroke_width = bond.stroke_width;
    let line_width = |weight| {
        if weight == chemsema_engine::BondLineWeight::Bold {
            bond.bold_width.unwrap_or(stroke_width).max(stroke_width)
        } else {
            stroke_width
        }
    };
    let first_width = line_width(bond.line_weights.left);
    let second_width = line_width(bond.line_weights.right);
    (length * ratio - stroke_width).max(stroke_width * 1.5) + 0.5 * (first_width + second_width)
}

fn imported_vertical_line_metrics(
    primitives: &[RenderPrimitive],
    object_id: &str,
) -> Vec<(f64, f64)> {
    let mut metrics: Vec<(f64, f64)> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id: primitive_object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && primitive_object_id.as_deref() == Some(object_id) =>
            {
                let (from, to) = bond_axis_from_points(points)?;
                let center_x = (from.x + to.x) * 0.5;
                let min_x = points
                    .iter()
                    .map(|point| point.x)
                    .fold(f64::INFINITY, f64::min);
                let max_x = points
                    .iter()
                    .map(|point| point.x)
                    .fold(f64::NEG_INFINITY, f64::max);
                Some((center_x, max_x - min_x))
            }
            _ => None,
        })
        .collect();
    metrics.sort_by(|a, b| a.0.total_cmp(&b.0));
    metrics
}

fn assert_line_spacing(metrics: &[(f64, f64)], expected: f64, context: &str) {
    assert_eq!(metrics.len(), 2, "{context}: {metrics:?}");
    let actual = metrics[1].0 - metrics[0].0;
    assert!(
        (actual - expected).abs() < 0.001,
        "{context}: expected {expected}, got {actual}, metrics={metrics:?}"
    );
}

fn assert_line_widths(
    metrics: &[(f64, f64)],
    expected_left: f64,
    expected_right: f64,
    context: &str,
) {
    assert_eq!(metrics.len(), 2, "{context}: {metrics:?}");
    assert!(
        (metrics[0].1 - expected_left).abs() < 0.001,
        "{context}: expected left width {expected_left}, got {}, metrics={metrics:?}",
        metrics[0].1
    );
    assert!(
        (metrics[1].1 - expected_right).abs() < 0.001,
        "{context}: expected right width {expected_right}, got {}, metrics={metrics:?}",
        metrics[1].1
    );
}

fn projection_range_on_axis(
    points: &[chemsema_engine::Point],
    axis_from: chemsema_engine::Point,
    axis_to: chemsema_engine::Point,
) -> Option<(f64, f64)> {
    let axis = chemsema_engine::Point::new(axis_to.x - axis_from.x, axis_to.y - axis_from.y);
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
) -> Vec<(chemsema_engine::Point, chemsema_engine::Point)> {
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
            }
            | RenderPrimitive::FilledPath {
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
    first: &[chemsema_engine::Point],
    second: &[chemsema_engine::Point],
    tolerance: f64,
) -> usize {
    first
        .iter()
        .filter(|point| {
            second
                .iter()
                .any(|other| point.distance(*other) <= tolerance)
        })
        .count()
}

fn point_lies_on_segment(
    point: chemsema_engine::Point,
    from: chemsema_engine::Point,
    to: chemsema_engine::Point,
    tolerance: f64,
) -> bool {
    let cross = (point.x - from.x) * (to.y - from.y) - (point.y - from.y) * (to.x - from.x);
    if cross.abs() > tolerance {
        return false;
    }
    let dot = (point.x - from.x) * (to.x - from.x) + (point.y - from.y) * (to.y - from.y);
    if dot < -tolerance {
        return false;
    }
    let length_squared = (to.x - from.x).powi(2) + (to.y - from.y).powi(2);
    dot <= length_squared + tolerance
}

fn polygons_have_same_vertices(
    first: &[chemsema_engine::Point],
    second: &[chemsema_engine::Point],
    tolerance: f64,
) -> bool {
    first.len() == second.len()
        && first.iter().all(|point| {
            second
                .iter()
                .any(|other| point.distance(*other) <= tolerance)
        })
}

fn point_lies_on_polygon_boundary(
    point: chemsema_engine::Point,
    polygon: &[chemsema_engine::Point],
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
    points: &[chemsema_engine::Point],
    target: chemsema_engine::Point,
    count: usize,
) -> f64 {
    let mut distances: Vec<_> = points.iter().map(|point| point.distance(target)).collect();
    distances.sort_by(|a, b| a.total_cmp(b));
    distances.into_iter().take(count).sum::<f64>() / count as f64
}

fn side_double_outer_polygon_for_bond(
    polygons: &[(String, Vec<chemsema_engine::Point>)],
    bond_id: &str,
    shared_node: chemsema_engine::Point,
) -> Vec<chemsema_engine::Point> {
    polygons
        .iter()
        .filter(|(current_bond_id, _)| current_bond_id == bond_id)
        .max_by(|a, b| {
            average_closest_distance_to_point(&a.1, shared_node, 2)
                .total_cmp(&average_closest_distance_to_point(&b.1, shared_node, 2))
        })
        .map(|(_, points)| points.clone())
        .expect("side double outer polygon")
}

fn side_double_main_polygon_for_bond(
    polygons: &[(String, Vec<chemsema_engine::Point>)],
    bond_id: &str,
    shared_node: chemsema_engine::Point,
) -> Vec<chemsema_engine::Point> {
    polygons
        .iter()
        .filter(|(current_bond_id, _)| current_bond_id == bond_id)
        .min_by(|a, b| {
            average_closest_distance_to_point(&a.1, shared_node, 2)
                .total_cmp(&average_closest_distance_to_point(&b.1, shared_node, 2))
        })
        .map(|(_, points)| points.clone())
        .expect("side double main polygon")
}

fn closest_points_to_target(
    points: &[chemsema_engine::Point],
    target: chemsema_engine::Point,
    count: usize,
) -> Vec<chemsema_engine::Point> {
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
    assert_eq!(
        polygons.len(),
        2,
        "joined single+bold should render as two 4-point polygons"
    );

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
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
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
                "tail": "none",
                "arrowHead": {
                    "kind": "solid",
                    "length": 22.5,
                    "centerLength": 19.69,
                    "width": 5.63,
                    "curve": 0.0,
                    "head": "full",
                    "tail": "full",
                    "bold": false,
                    "noGo": "none"
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
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_line_001") =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .expect("line shaft primitive");
    assert_eq!(shaft.len(), 2);
    assert!(shaft[1].x < 110.0);

    let arrow_head_paths: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                d,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_line_001")
                && points.len() == 6 =>
            {
                Some((points.clone(), d.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(arrow_head_paths.len(), 2);
    assert!(arrow_head_paths[0].1.contains(" C "));
    let head_width = arrow_head_paths[0]
        .0
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max)
        - arrow_head_paths[0]
            .0
            .iter()
            .map(|point| point.y)
            .fold(f64::INFINITY, f64::min);
    assert!((head_width - 8.2072).abs() <= 0.001);
}

#[test]
fn render_document_rounds_inner_curved_half_arrow_heads() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 100.0, "height": 60.0, "background": "#ffffff" }
        },
        "styles": {
            "style_arrow_default": {
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": 1.0,
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
                "points": [[40.0, 20.0], [60.0, 20.0]],
                "head": "end",
                "tail": "none",
                "arrowHead": {
                    "kind": "solid",
                    "length": 10.0,
                    "centerLength": 8.75,
                    "width": 2.5,
                    "curve": -120.0,
                    "head": "half-right",
                    "tail": "none",
                    "bold": false,
                    "noGo": "none"
                },
                "arrowGeometry": {
                    "center": [50.0, 25.77],
                    "majorAxisEnd": [61.55, 25.77],
                    "minorAxisEnd": [50.0, 37.32]
                }
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let shaft_end = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Path {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_line_001") =>
            {
                points.last().copied()
            }
            _ => None,
        })
        .expect("inner curved half arrow shaft path");
    let half_head_points = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_line_001")
                && points.len() == 4 =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .expect("inner curved half arrow head path");

    let cut_edge = half_head_points[3];
    assert!(
        shaft_end.distance(cut_edge) <= 0.65,
        "inner curved half-arrow shaft should stop at the head cut edge, shaft={shaft_end:?}, head={half_head_points:?}"
    );
}

#[test]
fn render_document_uses_open_arrow_width_as_extra_head_width() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
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
                "tail": "none",
                "arrowHead": {
                    "kind": "hollow",
                    "length": 12.0,
                    "centerLength": 12.0,
                    "width": 3.0,
                    "curve": 0.0,
                    "head": "full",
                    "tail": "none",
                    "bold": false,
                    "noGo": "none"
                }
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let outline = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_line_001")
                && points.len() > 4 =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .expect("hollow arrow outline polygon");
    let outline_width = outline
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max)
        - outline
            .iter()
            .map(|point| point.y)
            .fold(f64::INFINITY, f64::min);
    assert!((outline_width - 17.28).abs() <= 0.001);
}

#[test]
fn render_document_respects_thin_open_and_hollow_arrow_stroke_width() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_arrow_thin": {
                "kind": "stroke",
                "stroke": "#222222",
                "strokeWidth": 0.6,
                "lineCap": "butt",
                "lineJoin": "miter"
            }
        },
        "objects": [
            {
                "id": "obj_hollow",
                "type": "line",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_arrow_thin",
                "payload": {
                    "points": [[10.0, 20.0], [110.0, 20.0]],
                    "head": "end",
                    "tail": "none",
                    "arrowHead": {
                        "kind": "hollow",
                        "length": 12.0,
                        "centerLength": 12.0,
                        "width": 3.0,
                        "curve": 0.0,
                        "head": "full",
                        "tail": "none",
                        "bold": false,
                        "noGo": "none"
                    }
                }
            },
            {
                "id": "obj_open",
                "type": "line",
                "visible": true,
                "zIndex": 11,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_arrow_thin",
                "payload": {
                    "points": [[10.0, 80.0], [110.0, 80.0]],
                    "head": "end",
                    "tail": "none",
                    "arrowHead": {
                        "kind": "open",
                        "length": 12.0,
                        "centerLength": 12.0,
                        "width": 3.0,
                        "curve": 0.0,
                        "head": "full",
                        "tail": "none",
                        "bold": false,
                        "noGo": "none"
                    }
                }
            }
        ],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let hollow_width = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                stroke_width,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_hollow") =>
            {
                Some(*stroke_width)
            }
            _ => None,
        })
        .expect("hollow arrow outline");
    assert!((hollow_width - 0.6).abs() <= 1.0e-6, "{hollow_width}");

    let open_widths: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polyline {
                role,
                object_id,
                stroke_width,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_open") =>
            {
                Some(*stroke_width)
            }
            _ => None,
        })
        .collect();
    assert!(!open_widths.is_empty());
    assert!(
        open_widths
            .iter()
            .all(|width| (*width - 0.6).abs() <= 1.0e-6),
        "{open_widths:?}"
    );
}

#[test]
fn cdxml_acs_hollow_and_open_arrows_keep_chemdraw_head_width() {
    let Some(arrows) = read_optional_cdxml_fixture("arrows-acs.cdxml") else {
        return;
    };
    let document = parse_cdxml_document(&arrows, Some("arrows")).expect("arrows should parse");
    let primitives = render_document(&document);

    for (object_id, expected_height) in [
        ("obj_line_004", 14.4),
        ("obj_line_005", 7.2),
        ("obj_line_006", 14.4),
        ("obj_line_007", 7.2),
    ] {
        let height = primitives
            .iter()
            .filter_map(|primitive| match primitive {
                RenderPrimitive::Polygon {
                    role,
                    object_id: Some(id),
                    points,
                    ..
                }
                | RenderPrimitive::Polyline {
                    role,
                    object_id: Some(id),
                    points,
                    ..
                } if *role == RenderRole::DocumentGraphic && id == object_id => Some(
                    points
                        .iter()
                        .map(|point| point.y)
                        .fold(f64::NEG_INFINITY, f64::max)
                        - points
                            .iter()
                            .map(|point| point.y)
                            .fold(f64::INFINITY, f64::min),
                ),
                _ => None,
            })
            .fold(0.0, f64::max);

        assert!(
            (height - expected_height).abs() <= 0.001,
            "{object_id} height {height}"
        );
    }
}

#[test]
fn cdxml_import_preserves_hollow_and_open_arrow_dimensions() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="0.60">
  <page id="1">
    <arrow id="1" ArrowheadHead="Full" ArrowheadType="Hollow" HeadSize="1200" ArrowheadCenterSize="1200" ArrowheadWidth="300" ArrowShaftSpacing="1200" Head3D="110 20 0" Tail3D="10 20 0"/>
    <arrow id="2" ArrowheadHead="Full" ArrowheadType="Hollow" HeadSize="600" ArrowheadCenterSize="600" ArrowheadWidth="150" ArrowShaftSpacing="600" Head3D="110 50 0" Tail3D="10 50 0"/>
    <arrow id="3" ArrowheadHead="Full" ArrowheadType="Hollow" HeadSize="900" ArrowheadCenterSize="875" ArrowheadWidth="225" ArrowShaftSpacing="875" Head3D="110 80 0" Tail3D="10 80 0"/>
    <arrow id="4" ArrowheadHead="Full" ArrowheadType="Angle" HeadSize="1200" ArrowheadCenterSize="1200" ArrowheadWidth="300" ArrowShaftSpacing="1200" Head3D="110 110 0" Tail3D="10 110 0"/>
    <arrow id="5" ArrowheadHead="Full" ArrowheadType="Angle" HeadSize="600" ArrowheadCenterSize="600" ArrowheadWidth="150" ArrowShaftSpacing="600" Head3D="110 140 0" Tail3D="10 140 0"/>
    <arrow id="6" ArrowheadHead="Full" ArrowheadType="Angle" HeadSize="900" ArrowheadCenterSize="875" ArrowheadWidth="225" ArrowShaftSpacing="875" Head3D="110 170 0" Tail3D="10 170 0"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("hollow-open-sizes"))
        .expect("CDXML hollow/open arrows should parse");
    let arrow_head_for = |object_id: &str| {
        document
            .objects
            .iter()
            .find(|object| object.id == object_id)
            .and_then(|object| object.payload.extra.get("arrowHead"))
            .cloned()
            .expect("arrowHead payload")
    };
    for (object_id, expected_kind, expected_length, expected_center_length, expected_width) in [
        ("obj_line_001", "hollow", 12.0, 12.0, 3.0),
        ("obj_line_002", "hollow", 6.0, 6.0, 1.5),
        ("obj_line_003", "hollow", 9.0, 8.75, 2.25),
        ("obj_line_004", "open", 12.0, 12.0, 3.0),
        ("obj_line_005", "open", 6.0, 6.0, 1.5),
        ("obj_line_006", "open", 9.0, 8.75, 2.25),
    ] {
        let arrow_head = arrow_head_for(object_id);
        assert_eq!(
            arrow_head.get("kind").and_then(serde_json::Value::as_str),
            Some(expected_kind),
            "{object_id}"
        );
        assert_eq!(
            arrow_head.get("length").and_then(serde_json::Value::as_f64),
            Some(expected_length),
            "{object_id}"
        );
        assert_eq!(
            arrow_head
                .get("centerLength")
                .and_then(serde_json::Value::as_f64),
            Some(expected_center_length),
            "{object_id}"
        );
        assert_eq!(
            arrow_head.get("width").and_then(serde_json::Value::as_f64),
            Some(expected_width),
            "{object_id}"
        );
    }
}

#[test]
fn cdxml_imports_exports_and_renders_equilibrium_arrows() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="0 0 140 60">
    <arrow id="1" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="1500" ArrowheadCenterSize="1313" ArrowheadWidth="375" ArrowShaftSpacing="300"
      Head3D="110 30 0" Tail3D="10 30 0"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("equilibrium arrow"))
        .expect("CDXML equilibrium arrow should parse");
    let arrow = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow should import as line");
    let arrow_head = arrow
        .payload
        .extra
        .get("arrowHead")
        .expect("equilibrium arrow should carry arrowHead payload");
    assert_eq!(
        arrow_head.get("kind").and_then(serde_json::Value::as_str),
        Some("equilibrium")
    );
    assert_eq!(
        arrow_head.get("head").and_then(serde_json::Value::as_str),
        Some("half-left")
    );
    assert_eq!(
        arrow_head.get("tail").and_then(serde_json::Value::as_str),
        Some("half-left")
    );
    assert_eq!(
        arrow_head.get("length").and_then(serde_json::Value::as_f64),
        Some(15.0)
    );
    assert_eq!(
        arrow_head
            .get("centerLength")
            .and_then(serde_json::Value::as_f64),
        Some(13.13)
    );
    assert_eq!(
        arrow_head.get("width").and_then(serde_json::Value::as_f64),
        Some(3.75)
    );
    assert_eq!(
        arrow_head
            .get("shaftSpacing")
            .and_then(serde_json::Value::as_f64),
        Some(3.0)
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("ArrowheadType=\"Solid\""));
    assert!(exported.contains("ArrowShaftSpacing=\"300\""));
    assert!(!exported.contains("ArrowheadType=\"Equilibrium\""));

    let primitives: Vec<_> = render_document(&document)
        .into_iter()
        .filter(|primitive| match primitive {
            RenderPrimitive::Polyline { object_id, .. }
            | RenderPrimitive::FilledPath { object_id, .. } => {
                object_id.as_deref() == Some(&arrow.id)
            }
            _ => false,
        })
        .collect();
    assert_eq!(primitives.len(), 4);
    assert_eq!(
        primitives
            .iter()
            .filter(|primitive| matches!(primitive, RenderPrimitive::Polyline { .. }))
            .count(),
        2
    );
    assert_eq!(
        primitives
            .iter()
            .filter(|primitive| matches!(primitive, RenderPrimitive::FilledPath { .. }))
            .count(),
        2
    );
}

fn right_arrow_head_width_from_cdxml(cdxml: &str) -> f64 {
    let document =
        parse_cdxml_document(cdxml, Some("equilibrium arrow")).expect("CDXML arrow should parse");
    render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::FilledPath { points, .. } => {
                let bounds = primitive_polygon_bounds(&points);
                Some((bounds[2], bounds[2] - bounds[0]))
            }
            _ => None,
        })
        .max_by(|left, right| left.0.partial_cmp(&right.0).unwrap())
        .map(|(_, width)| (width * 100.0).round() / 100.0)
        .expect("arrow should render a filled head")
}

fn rounded_pair(points: &[Point]) -> ([f64; 2], [f64; 2]) {
    fn round2(value: f64) -> f64 {
        (value * 100.0).round() / 100.0
    }
    (
        [round2(points[0].x), round2(points[0].y)],
        [round2(points[1].x), round2(points[1].y)],
    )
}

#[test]
fn cdxml_equilibrium_arrow_heads_scale_with_axis_length_like_chemdraw() {
    let regular_short = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="0 0 240 80">
    <arrow id="1" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="2250" ArrowheadCenterSize="1969" ArrowheadWidth="563" ArrowShaftSpacing="300"
      Head3D="194.66 94.13 0" Tail3D="183.79 94.13 0"/>
  </page>
</CDXML>"#;
    let regular_full = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="0 0 300 80">
    <arrow id="1" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="2250" ArrowheadCenterSize="1969" ArrowheadWidth="563" ArrowShaftSpacing="300"
      Head3D="234.50 161.63 0" Tail3D="183.79 161.63 0"/>
  </page>
</CDXML>"#;
    let unequal_short = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="0 0 260 80">
    <arrow id="1" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="2250" ArrowheadCenterSize="1969" ArrowheadWidth="563" ArrowShaftSpacing="300"
      ArrowEquilibriumRatio="300" Head3D="208.54 370.50 0" Tail3D="195.79 370.50 0"/>
  </page>
</CDXML>"#;

    assert_eq!(right_arrow_head_width_from_cdxml(regular_short), 9.25);
    assert_eq!(right_arrow_head_width_from_cdxml(regular_full), 22.5);
    assert_eq!(right_arrow_head_width_from_cdxml(unequal_short), 8.5);
}

#[test]
fn cdxml_unequal_equilibrium_arrow_layout_matches_chemdraw() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="180 450 340 490">
    <arrow id="47" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="2250" ArrowheadCenterSize="1969" ArrowheadWidth="563" ArrowShaftSpacing="300"
      ArrowEquilibriumRatio="300" Head3D="314.80 468.75 0" Tail3D="198.79 468.75 0"/>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("unequal equilibrium arrow")).expect("CDXML arrow parses");
    let primitives = render_document(&document);
    let mut polylines: Vec<([f64; 2], [f64; 2])> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polyline { points, .. } => Some(rounded_pair(points)),
            _ => None,
        })
        .collect();
    polylines.sort_by(|left, right| {
        left.0[1]
            .partial_cmp(&right.0[1])
            .unwrap()
            .then(left.0[0].partial_cmp(&right.0[0]).unwrap())
    });
    assert_eq!(
        polylines,
        vec![
            ([198.79, 467.25], [296.11, 467.25]),
            ([282.36, 470.25], [249.92, 470.25]),
        ]
    );

    let mut head_bounds: Vec<[f64; 4]> = primitives
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::FilledPath { points, .. } => {
                let bounds = primitive_polygon_bounds(&points);
                Some([
                    (bounds[0] * 100.0).round() / 100.0,
                    (bounds[1] * 100.0).round() / 100.0,
                    (bounds[2] * 100.0).round() / 100.0,
                    (bounds[3] * 100.0).round() / 100.0,
                ])
            }
            _ => None,
        })
        .collect();
    head_bounds.sort_by(|left, right| left[0].partial_cmp(&right[0]).unwrap());
    assert_eq!(
        head_bounds,
        vec![
            [231.23, 469.75, 253.73, 475.88],
            [292.3, 461.62, 314.8, 467.75],
        ]
    );
}

#[test]
fn cdxml_imports_exports_and_renders_unequal_equilibrium_arrows() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="0 0 140 60">
    <arrow id="1" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="1500" ArrowheadCenterSize="1313" ArrowheadWidth="375" ArrowShaftSpacing="300"
      ArrowEquilibriumRatio="300" Head3D="110 30 0" Tail3D="10 30 0"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("unequal equilibrium arrow"))
        .expect("CDXML unequal equilibrium arrow should parse");
    let arrow = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow should import as line");
    let arrow_head = arrow
        .payload
        .extra
        .get("arrowHead")
        .expect("unequal equilibrium arrow should carry arrowHead payload");
    assert_eq!(
        arrow_head.get("kind").and_then(serde_json::Value::as_str),
        Some("unequal-equilibrium")
    );
    assert_eq!(
        arrow_head
            .get("equilibriumRatio")
            .and_then(serde_json::Value::as_f64),
        Some(3.0)
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("ArrowheadType=\"Solid\""));
    assert!(exported.contains("ArrowShaftSpacing=\"300\""));
    assert!(exported.contains("ArrowEquilibriumRatio=\"300\""));

    let mut branch_lengths: Vec<f64> = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polyline {
                object_id, points, ..
            } if object_id.as_deref() == Some(&arrow.id) => Some(
                points
                    .windows(2)
                    .map(|pair| pair[0].distance(pair[1]))
                    .sum::<f64>(),
            ),
            _ => None,
        })
        .collect();
    branch_lengths.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(branch_lengths.len(), 2);
    assert!(
        branch_lengths[0] < branch_lengths[1] * 0.45,
        "unequal equilibrium reverse branch should be much shorter: {branch_lengths:?}"
    );
}

#[test]
fn render_document_emits_arrow_no_go_marks_at_current_head_size() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
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
        "objects": [
            {
                "id": "obj_line_001",
                "type": "line",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_arrow_default",
                "payload": {
                    "points": [[10.0, 20.0], [110.0, 20.0]],
                    "head": "end",
                    "tail": "none",
                    "arrowHead": {
                        "kind": "solid",
                        "length": 10.0,
                        "centerLength": 8.75,
                        "width": 2.5,
                        "curve": 0.0,
                        "head": "full",
                        "tail": "none",
                        "bold": false,
                        "noGo": "hash"
                    }
                }
            },
            {
                "id": "obj_line_002",
                "type": "line",
                "visible": true,
                "zIndex": 11,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_arrow_default",
                "payload": {
                    "points": [[10.0, 60.0], [110.0, 60.0]],
                    "head": "end",
                    "tail": "none",
                    "arrowHead": {
                        "kind": "solid",
                        "length": 10.0,
                        "centerLength": 8.75,
                        "width": 2.5,
                        "curve": 0.0,
                        "head": "full",
                        "tail": "none",
                        "bold": false,
                        "noGo": "cross"
                    }
                }
            }
        ],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let mark_lines_for = |object_id: &str| -> Vec<(Point, Point, f64)> {
        primitives
            .iter()
            .filter_map(|primitive| match primitive {
                RenderPrimitive::Line {
                    role,
                    object_id: primitive_object_id,
                    from,
                    to,
                    stroke_width,
                    ..
                } if *role == RenderRole::DocumentGraphic
                    && primitive_object_id.as_deref() == Some(object_id) =>
                {
                    Some((*from, *to, *stroke_width))
                }
                _ => None,
            })
            .collect()
    };

    let hash_marks = mark_lines_for("obj_line_001");
    assert_eq!(hash_marks.len(), 2);
    for (from, to, stroke_width) in &hash_marks {
        assert_close(*stroke_width, 0.72);
        assert_close(from.distance(*to), 10.0 * 0.72 * 5.0_f64.sqrt() * 0.5);
    }
    let mut hash_centers: Vec<Point> = hash_marks
        .iter()
        .map(|(from, to, _)| Point::new((from.x + to.x) * 0.5, (from.y + to.y) * 0.5))
        .collect();
    hash_centers.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    assert_point_close(hash_centers[0], Point::new(60.0 - 10.0 * 0.72 * 0.25, 20.0));
    assert_point_close(hash_centers[1], Point::new(60.0 + 10.0 * 0.72 * 0.25, 20.0));
    assert_close(hash_centers[0].distance(hash_centers[1]), 10.0 * 0.72 * 0.5);

    let cross_marks = mark_lines_for("obj_line_002");
    assert_eq!(cross_marks.len(), 2);
    for (from, to, stroke_width) in &cross_marks {
        assert_close(*stroke_width, 0.72);
        assert_close(from.distance(*to), 10.0 * 0.72 * std::f64::consts::SQRT_2);
        assert_point_close(
            Point::new((from.x + to.x) * 0.5, (from.y + to.y) * 0.5),
            Point::new(60.0, 60.0),
        );
    }
}

#[test]
fn render_document_emits_text_lines_from_runs() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
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
            } if *role == RenderRole::DocumentText
                && object_id.as_deref() == Some("obj_text_001") =>
            {
                Some((*x, *y, runs.clone(), text_anchor.clone()))
            }
            _ => None,
        })
        .collect();

    assert_eq!(text_lines.len(), 2);
    assert!(text_lines
        .iter()
        .all(|(x, _, _, _)| (*x - 30.0).abs() < 0.001));
    assert_eq!(text_lines[0].2[0].text, "Na");
    assert_eq!(text_lines[1].2[0].text, "Cl");
    assert!(text_lines[1].1 > text_lines[0].1);
    assert_eq!(text_lines[0].3.as_deref(), Some("middle"));
}

#[test]
fn render_document_emits_shape_rect_with_style() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
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
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_shape_001") =>
            {
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
        rect.9.and_then(|value| value
            .get("stops")
            .and_then(|stops| stops.as_array())
            .map(|stops| stops.len())),
        Some(2)
    );
}

#[test]
fn render_document_emits_cdxml_oval_shape_with_zero_bbox_height() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape_001": {
                "kind": "shape",
                "fill": null,
                "stroke": "#000000",
                "strokeWidth": 1.0,
                "dashArray": [2.7]
            }
        },
        "objects": [{
            "id": "obj_shape_001",
            "type": "shape",
            "visible": true,
            "zIndex": 5,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_shape_001",
            "payload": {
                "kind": "ellipse",
                "bbox": [158.16, 247.50, 36.12, 0.0],
                "center": [158.16, 247.50],
                "majorAxisEnd": [194.28, 247.50],
                "minorAxisEnd": [158.16, 261.95]
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            object_id,
            stroke_width,
            dash_array,
            d,
            ..
        } if object_id.as_deref() == Some("obj_shape_001")
            && (*stroke_width - 1.0).abs() < 0.001
            && dash_array == &vec![2.7]
            && d.starts_with("M ")
            && d.contains(" C ")
    )));
}

#[test]
fn render_document_emits_all_shape_geometry_style_combinations() {
    let mut styles = Map::new();
    styles.insert(
        "solid".to_string(),
        json!({"kind": "shape", "fill": null, "stroke": "#000000", "strokeWidth": 1.0}),
    );
    styles.insert(
        "dashed".to_string(),
        json!({"kind": "shape", "fill": null, "stroke": "#000000", "strokeWidth": 1.0, "dashArray": [2.7]}),
    );
    styles.insert(
        "filled".to_string(),
        json!({"kind": "shape", "fill": "#000000", "stroke": null, "strokeWidth": 1.0}),
    );
    styles.insert(
        "shaded".to_string(),
        json!({"kind": "shape", "fill": null, "stroke": "#000000", "strokeWidth": 1.0, "shaded": true}),
    );
    styles.insert(
        "shadowed".to_string(),
        json!({"kind": "shape", "fill": null, "stroke": "#000000", "strokeWidth": 1.0, "shadow": true}),
    );

    let mut objects = Vec::new();
    let shapes = ["circle", "ellipse", "roundRect", "rect"];
    let style_ids = ["solid", "dashed", "filled", "shaded", "shadowed"];
    for (shape_index, shape) in shapes.iter().enumerate() {
        for (style_index, style_id) in style_ids.iter().enumerate() {
            let id = format!("obj_{shape}_{style_id}");
            let x = 20.0 + style_index as f64 * 40.0;
            let y = 20.0 + shape_index as f64 * 40.0;
            let payload = match *shape {
                "circle" => json!({
                    "kind": "circle",
                    "bbox": [x - 10.0, y, 20.0, 0.0],
                    "center": [x, y],
                    "majorAxisEnd": [x + 10.0, y],
                    "minorAxisEnd": [x, y + 10.0]
                }),
                "ellipse" => json!({
                    "kind": "ellipse",
                    "bbox": [x - 14.0, y, 28.0, 0.0],
                    "center": [x, y],
                    "majorAxisEnd": [x + 14.0, y],
                    "minorAxisEnd": [x, y + 6.0]
                }),
                "roundRect" => json!({
                    "kind": "roundRect",
                    "bbox": [0.0, 0.0, 28.0, 18.0],
                    "cornerRadius": 6.0
                }),
                _ => json!({
                    "kind": "rect",
                    "bbox": [0.0, 0.0, 28.0, 18.0]
                }),
            };
            objects.push(json!({
                "id": id,
                "type": "shape",
                "visible": true,
                "zIndex": shape_index * style_ids.len() + style_index,
                "transform": { "translate": [x, y], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": style_id,
                "payload": payload
            }));
        }
    }

    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 240.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": styles,
        "objects": objects,
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    for shape in shapes {
        for style_id in style_ids {
            let id = format!("obj_{shape}_{style_id}");
            assert!(
                primitives.iter().any(|primitive| match primitive {
                    RenderPrimitive::Rect { object_id, .. }
                    | RenderPrimitive::Path { object_id, .. }
                    | RenderPrimitive::FilledPath { object_id, .. } =>
                        object_id.as_deref() == Some(id.as_str()),
                    _ => false,
                }),
                "missing rendered primitive for {id}"
            );
        }
    }
}

#[test]
fn render_document_emits_fragment_label_text_and_knockout() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
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
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
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
            } if *role == RenderRole::DocumentText
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some((*x, *y, runs.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(label_lines.len(), 2);
    assert!(label_lines
        .iter()
        .all(|(x, _, _)| (*x - 28.4).abs() < 0.001));
    assert_eq!(label_lines[0].2[0].text, "H");
    assert_eq!(label_lines[1].2[0].text, "N");
    assert!(label_lines[1].1 > label_lines[0].1);
}

#[test]
fn render_document_respects_explicit_small_fragment_label_font_size() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "N",
                "atomicNumber": 7,
                "position": [20.0, 20.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "sourceText": "N",
                    "position": [18.0, 20.0],
                    "box": [18.0, 14.0, 22.0, 20.0],
                    "fontSize": 6.0,
                    "runs": [{
                        "text": "N",
                        "fontFamily": "Arial",
                        "fontSize": 6.0,
                        "fill": "#000000",
                        "fontWeight": 400,
                        "fontStyle": "normal",
                        "script": "normal"
                    }]
                }
            }
        ]),
        json!([]),
    );

    let font_size = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                role, font_size, ..
            } if role == RenderRole::DocumentText => Some(font_size),
            _ => None,
        })
        .expect("fragment label text");

    assert!((font_size - 6.0).abs() <= 1.0e-6, "{font_size}");
}

#[test]
fn render_document_draws_imported_cdxml_invalid_marker_as_non_focusable_diagnostic() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "N",
                "atomicNumber": 7,
                "position": [20.0, 20.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "sourceText": "N",
                    "position": [18.0, 20.0],
                    "box": [18.0, 14.0, 22.0, 20.0],
                    "fontSize": 10.0,
                    "runs": [{
                        "text": "N",
                        "fontFamily": "Arial",
                        "fontSize": 10.0,
                        "fill": "#000000",
                        "fontWeight": 400,
                        "fontStyle": "normal",
                        "script": "normal"
                    }],
                    "meta": {
                        "import": { "cdxml": { "textPosition": [18.0, 20.0] } },
                        "labelRecognition": { "status": "invalid" }
                    }
                }
            }
        ]),
        json!([]),
    );

    let marker = render_document(&document)
        .into_iter()
        .find(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Rect {
                    role: RenderRole::DocumentDiagnostic,
                    stroke: Some(stroke),
                    ..
                } if stroke == "#d32f2f"
            )
        })
        .expect("imported invalid label should still render a diagnostic marker");

    assert!(matches!(
        marker,
        RenderPrimitive::Rect {
            object_id: None,
            node_id: Some(node_id),
            stroke_width,
            ..
        } if node_id == "n1" && (stroke_width - 0.5).abs() < 1.0e-6
    ));
}

#[test]
fn export_svg_omits_invalid_label_diagnostics() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "N",
                "atomicNumber": 7,
                "position": [20.0, 20.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "sourceText": "N",
                    "position": [18.0, 20.0],
                    "box": [18.0, 14.0, 22.0, 20.0],
                    "fontSize": 10.0,
                    "runs": [{
                        "text": "N",
                        "fontFamily": "Arial",
                        "fontSize": 10.0,
                        "fill": "#000000",
                        "fontWeight": 400,
                        "fontStyle": "normal",
                        "script": "normal"
                    }],
                    "meta": {
                        "labelRecognition": { "status": "invalid" }
                    }
                }
            }
        ]),
        json!([]),
    );

    let primitives = render_document(&document);
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::DocumentDiagnostic,
            stroke: Some(stroke),
            ..
        } if stroke == "#d32f2f"
    )));

    let svg = document_to_svg(&document);
    assert!(svg.contains(">N</"));
    assert!(!svg.contains("#d32f2f"));
}

#[test]
fn render_document_scales_small_bracket_geometry_without_fixed_minimums() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 80.0, "height": 80.0, "background": "#ffffff" }
        },
        "styles": {},
        "objects": [{
            "id": "obj_bracket_001",
            "type": "bracket",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "payload": {
                "kind": "square",
                "bbox": [0.0, 0.0, 2.0, 6.0],
                "stroke": "#000000",
                "strokeWidth": 0.6
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let d = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Path {
                role, object_id, d, ..
            } if role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_bracket_001") =>
            {
                Some(d)
            }
            _ => None,
        })
        .expect("bracket path");
    let first_x = d
        .trim_start_matches("M ")
        .split(',')
        .next()
        .and_then(|value| value.parse::<f64>().ok())
        .expect("first bracket x coordinate");

    assert!(
        first_x > 0.0 && first_x < 0.5,
        "small bracket lip should scale with bbox instead of using a fixed 1pt minimum: {d}"
    );
}

#[test]
fn render_document_uses_label_glyph_polygons_for_knockout_and_endpoint_clipping() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "N",
                "atomicNumber": 7,
                "position": [19.0, 20.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "NH",
                    "position": [18.0, 23.9],
                    "box": [18.0, 16.0, 25.0, 24.0],
                    "glyphPolygons": [
                        [[18.0, 17.0], [20.0, 17.0], [20.0, 23.0], [18.0, 23.0]],
                        [[21.0, 17.0], [23.0, 17.0], [23.0, 23.0], [21.0, 23.0]]
                    ]
                }
            },
            {
                "id": "n2",
                "element": "C",
                "atomicNumber": 6,
                "position": [60.0, 20.0],
                "charge": 0,
                "numHydrogens": 0
            }
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
    let knockouts = object_knockout_polygons(&primitives);
    assert_eq!(knockouts.len(), 2, "{knockouts:?}");
    assert_eq!(object_knockout_rect_count(&primitives), 0);
    assert!(primitives.iter().all(|primitive| match primitive {
        RenderPrimitive::Polygon { role, node_id, .. } if *role == RenderRole::DocumentKnockout => {
            node_id.as_deref() == Some("n1")
        }
        _ => true,
    }));
    let svg = document_to_svg(&document);
    assert!(
        !svg.contains("fill=\"#ffffff\""),
        "label clipping geometry should not paint a white knockout in document SVG: {svg}"
    );

    let centerlines = object_bond_centerlines(&primitives);
    assert_eq!(centerlines.len(), 1, "{centerlines:?}");
    let start_x = centerlines[0].0.x.min(centerlines[0].1.x);
    assert!(
        (start_x - 23.0).abs() < 0.02,
        "endpoint should clip at the source-margin glyph polygon without adding a second margin retreat: {centerlines:?}"
    );
}

#[test]
fn render_document_emits_primitives_for_legacy_molblock_resource() {
    let molblock = concat!(
        "Legacy\n",
        "  ChemSema\n",
        "\n",
        "  2  1  0  0  0  0            999 V2000\n",
        "    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n",
        "    1.2000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n",
        "  1  2  1  0  0  0  0\n",
        "M  END\n"
    );
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
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

    assert_eq!(polygons.len(), 14);
    assert!(polygons.iter().all(|points| points.len() == 4));
    assert!(knockouts.is_empty(), "{knockouts:?}");
    let lengths: Vec<_> = polygons
        .iter()
        .filter_map(|points| bond_axis_length(points))
        .collect();
    assert!(
        lengths
            .iter()
            .all(|length| (*length - 36.0 / 13.0).abs() < 0.01),
        "{lengths:?}"
    );
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
            } if role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
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
    let polygons = centered_bond_polygons(&primitives, chemsema_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 2);
    assert!(polygons.iter().all(|points| points.len() == 5));
    assert!(
        polygons.iter().all(|points| polygon_area(points) > 0.01),
        "{polygons:?}"
    );

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
    let polygons = centered_bond_polygons(&primitives, chemsema_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 2);
    assert!(polygons.iter().all(|points| points.len() == 5));
    assert!(
        polygons.iter().all(|points| polygon_area(points) > 0.01),
        "{polygons:?}"
    );

    let total_bond_polygons: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
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
        .collect();
    assert_eq!(total_bond_polygons.len(), 2);
}

#[test]
fn render_document_emits_equal_length_cross_segments_for_bold_dashed_bond() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [50.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 1.0,
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

    assert_eq!(polygons.len(), 11);
    assert!(polygons.iter().all(|points| points.len() == 4));
    assert!(knockouts.is_empty(), "{knockouts:?}");
    let black_segments: Vec<_> = polygons
        .iter()
        .filter_map(|points| bond_axis_length(points))
        .collect();
    let first_black = black_segments[0];
    assert!(
        black_segments
            .iter()
            .all(|length| (length - first_black).abs() < 0.02),
        "{black_segments:?}"
    );
    assert!((first_black - 1.0).abs() < 0.02, "{black_segments:?}");
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
    let polygons = centered_bond_polygons(&primitives, chemsema_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 2);
    assert!(
        polygons.iter().all(|points| points.len() == 5),
        "the dashed terminal stripe must absorb its contact profile: {polygons:?}"
    );
    assert!(object_knockout_polygons(&primitives).is_empty());
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
            }
            | RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points.len())
            }
            _ => None,
        })
        .collect();

    assert!(
        bond_polygons.iter().all(|count| *count == 4),
        "{bond_polygons:?}"
    );
    assert!(bond_polygons.len() >= 2);
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
    let polygons = centered_bond_polygons(&primitives, chemsema_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 2);
    assert_eq!(
        polygons.iter().filter(|points| points.len() == 4).count(),
        2
    );

    let wedge_polygon = primitives
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
        .max_by(|a, b| polygon_area(a).total_cmp(&polygon_area(b)))
        .expect("solid wedge polygon");
    assert_eq!(wedge_polygon.len(), 4);
    assert_eq!(
        wedge_polygon
            .iter()
            .filter(|point| point.distance(chemsema_engine::Point::new(56.0, 40.0)) <= 4.0)
            .count(),
        2
    );
}

#[test]
fn render_document_retreats_hashed_wedge_against_connected_single_bond() {
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
    let connected_primitives = render_document(&connected);
    let connected_polygons = object_bond_polygons_with_ids(&connected_primitives);
    let hashed_wedge = object_bond_points_for_id(&connected_primitives, "b1");
    assert!(!hashed_wedge.is_empty(), "hashed wedge polygons");
    let branch = connected_polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b2").then_some(points.clone()))
        .expect("branch polygon");
    let connected_end =
        closest_points_to_target(&hashed_wedge, chemsema_engine::Point::new(56.0, 40.0), 2);

    assert!(
        connected_end.iter().all(|point| point.x < 55.0),
        "{hashed_wedge:?}"
    );
    assert!(
        average_closest_distance_to_point(&branch, chemsema_engine::Point::new(56.0, 40.0), 2)
            < 0.6,
        "{branch:?}"
    );
}

#[test]
fn render_document_retreats_hashed_wedge_at_both_connected_endpoints() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [2.0, 12.0], "charge": 0, "numHydrogens": 0 },
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
            { "id": "b2", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n2", "end": "n4", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let primitives = render_document(&document);
    let hashed_wedge = object_bond_points_for_id(&primitives, "b1");
    assert!(!hashed_wedge.is_empty(), "hashed wedge polygons");
    let begin_end =
        closest_points_to_target(&hashed_wedge, chemsema_engine::Point::new(20.0, 40.0), 2);
    let wide_end =
        closest_points_to_target(&hashed_wedge, chemsema_engine::Point::new(56.0, 40.0), 2);

    assert!(
        begin_end.iter().all(|point| point.x > 21.0),
        "{hashed_wedge:?}"
    );
    assert!(
        wide_end.iter().all(|point| point.x < 55.0),
        "{hashed_wedge:?}"
    );
}

#[test]
fn render_document_keeps_hashed_wedge_label_clip_without_extra_hash_retreat() {
    let labeled_nodes = json!([
        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
        {
            "id": "n2",
            "element": "N",
            "atomicNumber": 7,
            "position": [56.0, 40.0],
            "charge": 0,
            "numHydrogens": 0,
            "label": {
                "text": "N",
                "position": [52.0, 44.0],
                "box": [50.0, 34.0, 62.0, 46.0],
                "runs": [{ "text": "N", "fontFamily": "Arial", "fontSize": 10.0, "fill": "#000000" }]
            }
        }
    ]);
    let isolated = fragment_document(
        labeled_nodes.clone(),
        json!([{
            "id": "b1",
            "begin": "n1",
            "end": "n2",
            "order": 1,
            "strokeWidth": 0.85,
            "stereo": { "kind": "hashed-wedge", "wideEnd": "end" }
        }]),
    );
    let connected = fragment_document(
        json!([
            labeled_nodes[0].clone(),
            labeled_nodes[1].clone(),
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 58.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": { "kind": "hashed-wedge", "wideEnd": "end" }
            },
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 }
        ]),
    );
    let isolated_wedge = object_bond_polygons_with_ids(&render_document(&isolated))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("isolated hashed wedge polygon");
    let connected_wedge = object_bond_polygons_with_ids(&render_document(&connected))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("connected hashed wedge polygon");

    assert!(polygons_have_same_vertices(
        &isolated_wedge,
        &connected_wedge,
        1.0e-4,
    ));
}

#[test]
fn render_document_retreats_hash_bond_against_connected_single_bond() {
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
    let connected_primitives = render_document(&connected);
    let connected_polygons = object_bond_polygons_with_ids(&connected_primitives);
    let hash_bond = object_bond_points_for_id(&connected_primitives, "b1");
    let branch = connected_polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b2").then_some(points.clone()))
        .expect("branch polygon");
    let connected_end =
        closest_points_to_target(&hash_bond, chemsema_engine::Point::new(56.0, 40.0), 2);

    assert!(!hash_bond.is_empty(), "hash bond segments");
    assert!(
        connected_end.iter().all(|point| point.x < 55.0),
        "{hash_bond:?}"
    );
    assert!(
        average_closest_distance_to_point(&branch, chemsema_engine::Point::new(56.0, 40.0), 2)
            < 0.6,
        "{branch:?}"
    );
    let knockouts = object_knockout_polygons(&connected_primitives);
    assert!(knockouts.is_empty(), "{knockouts:?}");
}

#[test]
fn render_document_keeps_hash_bond_label_clip_without_extra_hash_retreat() {
    let labeled_nodes = json!([
        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
        {
            "id": "n2",
            "element": "N",
            "atomicNumber": 7,
            "position": [56.0, 40.0],
            "charge": 0,
            "numHydrogens": 0,
            "label": {
                "text": "N",
                "position": [52.0, 44.0],
                "box": [50.0, 34.0, 62.0, 46.0],
                "runs": [{ "text": "N", "fontFamily": "Arial", "fontSize": 10.0, "fill": "#000000" }]
            }
        }
    ]);
    let hash_bond = json!({
        "id": "b1",
        "begin": "n1",
        "end": "n2",
        "order": 1,
        "strokeWidth": 0.85,
        "lineStyles": { "main": "dashed", "left": "solid", "right": "solid" },
        "lineWeights": { "main": "bold", "left": "normal", "right": "normal" }
    });
    let isolated = fragment_document(labeled_nodes.clone(), json!([hash_bond.clone()]));
    let connected = fragment_document(
        json!([
            labeled_nodes[0].clone(),
            labeled_nodes[1].clone(),
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 58.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            hash_bond,
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 }
        ]),
    );
    let isolated_hash = object_bond_polygons_with_ids(&render_document(&isolated))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("isolated hash polygon");
    let connected_hash = object_bond_polygons_with_ids(&render_document(&connected))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("connected hash polygon");

    assert!(polygons_have_same_vertices(
        &isolated_hash,
        &connected_hash,
        1.0e-4,
    ));
}

#[test]
fn render_document_retreats_hash_bond_segments_against_center_double_outer_line() {
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
    let hash_bond = object_bond_points_for_id(&primitives, "b2");
    assert!(!hash_bond.is_empty(), "hash bond segments");
    let connected_end =
        closest_points_to_target(&hash_bond, chemsema_engine::Point::new(56.0, 40.0), 2);
    let unit = chemsema_engine::Point::new(18.0, -28.0);
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
    assert!(
        projections.iter().all(|projection| *projection > 0.05),
        "{hash_bond:?} {projections:?}"
    );
    assert!(object_knockout_polygons(&primitives).is_empty());
}

#[test]
fn render_document_retreats_hashed_wedge_stripes_against_center_double_outer_line() {
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
    let hashed_wedge = object_bond_points_for_id(&primitives, "b2");
    assert!(!hashed_wedge.is_empty(), "hashed wedge polygons");
    let connected_end =
        closest_points_to_target(&hashed_wedge, chemsema_engine::Point::new(56.0, 40.0), 2);
    let unit = chemsema_engine::Point::new(18.0, -28.0);
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
}

#[test]
fn render_document_retreats_hash_bond_and_ignores_it_for_other_bond_contacts() {
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

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let hash_bond = polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points.clone()))
        .expect("hash bond polygon");

    assert!(
        average_closest_distance_to_point(&hash_bond, chemsema_engine::Point::new(56.0, 40.0), 2)
            > 1.0,
        "{hash_bond:?}"
    );
}

#[test]
fn render_document_retreats_hashed_wedge_and_ignores_it_for_other_bond_contacts() {
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

    let primitives = render_document(&document);
    let hashed_wedge = object_bond_points_for_id(&primitives, "b1");
    assert!(!hashed_wedge.is_empty(), "hashed wedge polygons");

    assert!(
        average_closest_distance_to_point(
            &hashed_wedge,
            chemsema_engine::Point::new(56.0, 40.0),
            2
        ) > 1.0,
        "{hashed_wedge:?}"
    );
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
    let hash_bond = object_bond_points_for_id(&primitives, "b2");
    assert!(!hash_bond.is_empty(), "hash bond segments");
    let connected_end =
        closest_points_to_target(&hash_bond, chemsema_engine::Point::new(56.0, 40.0), 2);
    let unit = chemsema_engine::Point::new(18.0, -28.0);
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
    assert!(
        projections.iter().all(|projection| *projection > 0.05),
        "{hash_bond:?} {projections:?}"
    );
    assert!(object_knockout_polygons(&primitives).is_empty());
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
    let hashed_wedge = object_bond_points_for_id(&primitives, "b2");
    assert!(!hashed_wedge.is_empty(), "hashed wedge polygons");
    let connected_end =
        closest_points_to_target(&hashed_wedge, chemsema_engine::Point::new(56.0, 40.0), 2);
    let unit = chemsema_engine::Point::new(18.0, -28.0);
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
}

#[test]
fn render_document_uses_length_percent_with_line_width_floor_for_side_double_offset() {
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
    let expected_short_offset = (36.0_f64 * 0.12).max(2.5 * 0.85);
    let expected_long_offset = (72.0_f64 * 0.12).max(2.5 * 0.85);
    assert!(
        (short_offset - expected_short_offset).abs() < 0.05,
        "{short_offset}"
    );
    assert!(
        (long_offset - expected_long_offset).abs() < 0.05,
        "{long_offset}"
    );
}

#[test]
fn render_document_increases_side_double_offset_for_bold_main_line() {
    let normal_document = fragment_document(
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
                "double": { "placement": "right" }
            }
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
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" },
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );

    let normal_lines = object_bond_centerlines(&render_document(&normal_document));
    let bold_lines = object_bond_centerlines(&render_document(&bold_document));

    let normal_offset = normal_lines
        .iter()
        .map(|(from, to)| ((from.y + to.y) / 2.0 - 40.0).abs())
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();
    let bold_offset = bold_lines
        .iter()
        .map(|(from, to)| ((from.y + to.y) / 2.0 - 40.0).abs())
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();

    assert!(
        bold_offset > normal_offset + 0.01,
        "{normal_offset} {bold_offset}"
    );
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

    assert!(
        (short_length - long_length).abs() < 0.05,
        "{short_length} {long_length}"
    );
    assert!(
        (short_axis.0.x - 20.0).abs() < 0.05 && (short_axis.1.x - 56.0).abs() < 0.05,
        "{short_axis:?}"
    );
    assert!(
        (long_axis.0.x - 20.0).abs() < 0.05 && (long_axis.1.x - 56.0).abs() < 0.05,
        "{long_axis:?}"
    );
}

#[test]
fn render_document_keeps_side_double_outer_line_full_length_when_only_opposite_side_single_is_attached(
) {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 68.0], "charge": 0, "numHydrogens": 0 }
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
    assert!(
        (short_length - long_length).abs() < 0.05,
        "short={short_length} long={long_length}"
    );
    assert!(
        (short_axis.0.x - 20.0).abs() < 0.05 && (short_axis.1.x - 56.0).abs() < 0.05,
        "{short_axis:?}"
    );
    assert!(
        (long_axis.0.x - 20.0).abs() < 0.05 && (long_axis.1.x - 56.0).abs() < 0.05,
        "{long_axis:?}"
    );
}

#[test]
fn render_document_keeps_same_side_single_attached_side_double_outer_line_shortened() {
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

    let short_length = indexed_lengths[0].1;
    let long_length = indexed_lengths[1].1;
    assert!(
        short_length < long_length - 0.05,
        "{short_length} {long_length}"
    );
}

#[test]
fn render_document_recomputes_triple_outer_line_retreat_from_current_bond_length() {
    let short_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [8.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [68.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 3, "strokeWidth": 0.85 },
            { "id": "b2", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n2", "end": "n4", "order": 1, "strokeWidth": 0.85 }
        ]),
    );
    let long_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [92.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [8.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [104.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 3, "strokeWidth": 0.85 },
            { "id": "b2", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n2", "end": "n4", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let retreat_for = |document: &chemsema_engine::ChemSemaDocument| {
        let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(document))
            .into_iter()
            .filter(|(bond_id, _)| bond_id == "b1")
            .map(|(_, points)| points)
            .collect();
        assert_eq!(polygons.len(), 3);

        let mut lengths: Vec<_> = polygons
            .iter()
            .map(|points| bond_axis_length(points).expect("bond axis length"))
            .collect();
        lengths.sort_by(|a, b| a.total_cmp(b));
        let outer_length = lengths[0];
        let main_length = lengths[2];
        main_length - outer_length
    };

    let short_retreat = retreat_for(&short_document);
    let long_retreat = retreat_for(&long_document);

    assert!(
        long_retreat > short_retreat + 0.05,
        "short_retreat={short_retreat} long_retreat={long_retreat}"
    );
    assert!(short_retreat > 0.0, "{short_retreat}");
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
fn render_document_keeps_center_double_joined_line_normal_width() {
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
                "strokeWidth": 0.6,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.6
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);

    for polygon in &polygons {
        let (start_width, end_width) =
            bond_polygon_normal_widths(polygon).expect("center double polygon width");
        assert!(
            (start_width - 0.6).abs() <= 1.0e-6 && (end_width - 0.6).abs() <= 1.0e-6,
            "{polygon:?} start={start_width} end={end_width}"
        );
    }
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
fn render_document_uses_larger_individual_label_retreat_for_both_center_double_lines() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "C",
                "atomicNumber": 6,
                "position": [20.0, 60.0],
                "charge": 0,
                "numHydrogens": 0
            },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [56.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [56.0, 45.0],
                    "box": [51.0, 34.0, 61.0, 46.0]
                }
            }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);

    let axis = chemsema_engine::Point::new(20.0 - 56.0, 60.0 - 40.0);
    let axis_length = (axis.x * axis.x + axis.y * axis.y).sqrt();
    let unit_x = axis.x / axis_length;
    let unit_y = axis.y / axis_length;
    let endpoint_retreats: Vec<_> = polygons
        .iter()
        .map(|polygon| {
            let (from, to) = bond_axis_from_points(polygon).expect("bond axis");
            let endpoint = if from.distance(chemsema_engine::Point::new(56.0, 40.0))
                <= to.distance(chemsema_engine::Point::new(56.0, 40.0))
            {
                from
            } else {
                to
            };
            (endpoint.x - 56.0) * unit_x + (endpoint.y - 40.0) * unit_y
        })
        .collect();

    assert!(
        (endpoint_retreats[0] - endpoint_retreats[1]).abs() <= 1.0e-4,
        "parallel center-double lines should apply the larger of their independently computed label retreats: {polygons:?} {endpoint_retreats:?}"
    );
    assert!(
        endpoint_retreats.iter().all(|retreat| *retreat > 0.0),
        "{polygons:?} {endpoint_retreats:?}"
    );
    let axis_lengths: Vec<_> = polygons
        .iter()
        .map(|polygon| {
            let (from, to) = bond_axis_from_points(polygon).expect("bond axis");
            from.distance(to)
        })
        .collect();
    assert!(
        (axis_lengths[0] - axis_lengths[1]).abs() <= 1.0e-4,
        "center-double strokes must remain equal length after label retreat: {polygons:?} {axis_lengths:?}"
    );
}

#[test]
fn render_document_side_double_uses_anchor_glyph_retreat_once_and_keeps_lines_equal() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "P",
                "atomicNumber": 15,
                "position": [62.0, 12.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "P(OPh)",
                    "position": [58.0, 15.9],
                    "box": [58.0, 6.0, 94.0, 22.0],
                    "runs": [{ "text": "P(OPh)", "fontFamily": "Arial", "fontSize": 10.0, "script": "normal" }],
                    "glyphPolygons": [
                        [[58.0, 8.0], [66.0, 8.0], [66.0, 18.0], [58.0, 18.0]],
                        [[67.0, 6.0], [69.0, 6.0], [69.0, 22.0], [67.0, 22.0]],
                        [[71.0, 8.0], [77.0, 8.0], [77.0, 18.0], [71.0, 18.0]],
                        [[79.0, 8.0], [85.0, 8.0], [85.0, 18.0], [79.0, 18.0]],
                        [[90.0, 6.0], [92.0, 6.0], [92.0, 22.0], [90.0, 22.0]]
                    ]
                }
            },
            {
                "id": "n2",
                "element": "O",
                "atomicNumber": 8,
                "position": [62.0, 36.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "O",
                    "position": [56.0, 39.9],
                    "box": [56.0, 32.0, 64.0, 42.0],
                    "glyphPolygons": [[[56.0, 32.0], [64.0, 32.0], [64.0, 42.0], [56.0, 42.0]]]
                }
            }
        ]),
        json!([{
            "id": "b1",
            "begin": "n1",
            "end": "n2",
            "order": 2,
            "strokeWidth": 0.6,
            "double": { "placement": "left" }
        }]),
    );

    let axes: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| bond_axis_from_points(&points).expect("bond axis"))
        .collect();
    assert_eq!(axes.len(), 2);
    let main_axis = axes
        .iter()
        .min_by(|left, right| {
            let left_x = (left.0.x + left.1.x) * 0.5;
            let right_x = (right.0.x + right.1.x) * 0.5;
            (left_x - 62.0).abs().total_cmp(&(right_x - 62.0).abs())
        })
        .expect("main axis");
    assert!(
        ((main_axis.0.x + main_axis.1.x) * 0.5 - 62.0).abs() <= 0.02,
        "without EndAttach the side-double main line must use the structural node even when cached glyph geometry is shifted: {axes:?}"
    );
    let lengths: Vec<_> = axes.iter().map(|(from, to)| from.distance(*to)).collect();
    assert!(
        (lengths[0] - lengths[1]).abs() <= 1.0e-4,
        "side-double lines must share the larger single-pass glyph retreat: {axes:?}"
    );
    for (from, to) in axes {
        let label_exit_y = from.y.min(to.y);
        assert!(
            (label_exit_y - 18.0).abs() <= 0.02,
            "the synthetic internal-row rectangle must not over-clip a bond anchored on P: {from:?} {to:?}"
        );
    }
}

#[test]
fn parse_cdxml_side_double_terminal_label_stays_on_main_bond_node() {
    let source = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="70 80 140 130" BondLength="14.4" LabelFont="3" LabelSize="10" MarginWidth="1.6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="70 80 140 130">
    <fragment id="2" BoundingBox="80 90 135 125">
      <n id="3" p="85 100"/>
      <n id="4" p="100 100" NodeType="Nickname">
        <t id="5" BoundingBox="96.7 94 134.3 105" p="96.7 103.9" LabelAlignment="Left" LabelJustification="Left" InterpretChemically="yes">
          <s font="3" size="10" face="0" color="0">P(OPh)</s>
          <s font="3" size="10" face="96" color="0">2</s>
        </t>
      </n>
      <n id="6" p="100 114.4" Element="8" NumHydrogens="0">
        <t id="7" BoundingBox="94.8 109.9 102.6 118.8" p="94.8 118.1" LabelJustification="Left" InterpretChemically="yes">
          <s font="3" size="10" face="0" color="0">O</s>
        </t>
      </n>
      <b id="8" B="3" E="4" Order="1"/>
      <b id="9" B="4" E="6" Order="2" DoublePosition="Left"/>
    </fragment>
  </page>
</CDXML>"#;

    let document = parse_cdxml_document(source, Some("side-double terminal label"))
        .expect("CDXML should import");
    let fragment = document
        .editable_fragments()
        .into_iter()
        .next()
        .expect("fragment");
    let oxygen = fragment
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "6")
        .expect("oxygen node");
    let oxygen_polygon = oxygen
        .label
        .as_ref()
        .and_then(|label| label.glyph_polygons.first())
        .expect("oxygen glyph polygon");
    let min_x = oxygen_polygon
        .iter()
        .map(|point| point[0])
        .fold(f64::INFINITY, f64::min);
    let max_x = oxygen_polygon
        .iter()
        .map(|point| point[0])
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        (((min_x + max_x) * 0.5) - oxygen.position[0]).abs() <= 0.05,
        "terminal O glyph must stay on the structural node/main bond axis: {oxygen:?}"
    );

    let axes: Vec<_> = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                bond_id: Some(bond_id),
                points,
                ..
            } if role == RenderRole::DocumentBond && bond_id == "9" => {
                bond_axis_from_points(&points)
            }
            _ => None,
        })
        .collect();
    assert_eq!(axes.len(), 2, "{axes:?}");
    for (from, to) in &axes {
        assert!(
            (from.x - to.x).abs() <= 0.05,
            "side-double lines must remain vertical on the source node axis: {axes:?}"
        );
    }
    assert!(
        (axes[0].0.distance(axes[0].1) - axes[1].0.distance(axes[1].1)).abs() <= 1.0e-4,
        "terminal side-double lines must remain equal after label retreat: {axes:?}"
    );
}

#[test]
fn parse_cdxml_begin_attach_uses_internal_label_glyph_and_round_trips_stably() {
    let source = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="300 590 370 630" BondLength="14.4" LabelFont="3" LabelSize="10" MarginWidth="1.6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="300 590 370 630">
    <fragment id="2" BoundingBox="309.75 598.802 362.063 624.05">
      <n id="3" p="311.625 617.025" NodeType="Fragment">
        <t id="4" BoundingBox="309.75 612.55 362.063 624.05" p="309.75 621.5" LabelAlignment="Left" LabelJustification="Left" InterpretChemically="yes">
          <s font="3" size="10" face="0" color="0">(PhO)</s>
          <s font="3" size="10" face="96" color="0">2</s>
          <s font="3" size="10" face="0" color="0">POH</s>
        </t>
      </n>
      <n id="5" p="343.569 603.527" Element="8" NumHydrogens="0">
        <t id="6" BoundingBox="339.663 598.802 347.413 607.302" p="339.663 607.202" LabelJustification="Left">
          <s font="3" size="10" face="96" color="0">O</s>
        </t>
      </n>
      <b id="7" B="3" BeginAttach="6" E="5" Order="2"/>
    </fragment>
  </page>
</CDXML>"#;

    let document = parse_cdxml_document(source, Some("internal label attachment"))
        .expect("CDXML should import");
    let fragment = document
        .editable_fragments()
        .into_iter()
        .next()
        .expect("fragment");
    assert_eq!(
        fragment.fragment.bonds[0]
            .meta
            .pointer("/endpointAttachments/begin/characterIndex")
            .and_then(serde_json::Value::as_u64),
        Some(6)
    );
    assert_eq!(
        fragment.fragment.bonds[0]
            .meta
            .pointer("/endpointAttachments/begin/character"),
        Some(&json!("P"))
    );

    let polygons: Vec<_> = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                bond_id: Some(bond_id),
                points,
                ..
            } if role == RenderRole::DocumentBond && bond_id == "7" => Some(points),
            _ => None,
        })
        .collect();
    assert_eq!(polygons.len(), 2);
    let axes: Vec<_> = polygons
        .iter()
        .map(|polygon| bond_axis_from_points(polygon).expect("bond axis"))
        .collect();
    for (from, to) in &axes {
        assert!(
            (from.x - to.x).abs() <= 0.15,
            "BeginAttach=6 must make the P=O center double vertical: {axes:?}"
        );
    }
    assert!(
        (axes[0].0.distance(axes[0].1) - axes[1].0.distance(axes[1].1)).abs() <= 1.0e-4,
        "the two P=O strokes must remain equal length: {axes:?}"
    );

    let first_export = document_to_cdxml(&document);
    assert!(first_export.contains("BeginAttach=\"6\""), "{first_export}");
    let reopened = parse_cdxml_document(&first_export, Some("internal label attachment"))
        .expect("exported CDXML should reopen");
    let second_export = document_to_cdxml(&reopened);
    assert_eq!(
        second_export, first_export,
        "attachment export must stabilize"
    );
}

#[test]
fn render_document_clips_center_double_lines_against_glyph_polygon_only() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "C",
                "atomicNumber": 6,
                "position": [50.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "isPlaceholder": true,
                "label": {
                    "text": "•",
                    "position": [49.0, 42.5],
                    "box": [49.4, 36.5, 50.6, 43.5],
                    "glyphPolygons": [[
                        [49.4, 36.5],
                        [50.6, 36.5],
                        [50.6, 43.5],
                        [49.4, 43.5]
                    ]]
                }
            },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [80.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.6,
                "labelClipMargin": 9.0,
                "bondSpacing": 18.0,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 2,
                "strokeWidth": 0.6,
                "labelClipMargin": 9.0,
                "bondSpacing": 18.0,
                "double": { "placement": "center" }
            }
        ]),
    );

    let polygons = object_bond_polygons_with_ids(&render_document(&document));
    let b1_axes: Vec<_> = polygons
        .iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| bond_axis_from_points(points).expect("b1 axis"))
        .collect();
    let b2_axes: Vec<_> = polygons
        .iter()
        .filter(|(bond_id, _)| bond_id == "b2")
        .map(|(_, points)| bond_axis_from_points(points).expect("b2 axis"))
        .collect();

    assert_eq!(b1_axes.len(), 2, "{polygons:?}");
    assert_eq!(b2_axes.len(), 2, "{polygons:?}");
    for (from, to) in b1_axes {
        let label_endpoint_x = from.x.max(to.x);
        assert!(
            label_endpoint_x <= 49.45,
            "left center-double line should stop at the dot glyph polygon, ignoring legacy labelClipMargin: {polygons:?}"
        );
    }
    for (from, to) in b2_axes {
        let label_endpoint_x = from.x.min(to.x);
        assert!(
            label_endpoint_x >= 50.55,
            "right center-double line should stop at the dot glyph polygon, ignoring legacy labelClipMargin: {polygons:?}"
        );
    }
}

#[test]
fn render_document_keeps_terminal_side_double_offset_with_label_retreat() {
    let unlabeled = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "N", "atomicNumber": 7, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            }
        ]),
    );
    let labeled = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [56.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [56.0, 45.0],
                    "box": [51.0, 34.0, 61.0, 46.0]
                }
            }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            }
        ]),
    );

    let unlabeled_polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&unlabeled))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    let labeled_polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&labeled))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(unlabeled_polygons.len(), 2);
    assert_eq!(labeled_polygons.len(), 2);

    let unlabeled_axes: Vec<_> = unlabeled_polygons
        .iter()
        .map(|polygon| bond_axis_from_points(polygon).expect("bond axis"))
        .collect();
    let labeled_axes: Vec<_> = labeled_polygons
        .iter()
        .map(|polygon| bond_axis_from_points(polygon).expect("bond axis"))
        .collect();
    let unlabeled_gap = ((unlabeled_axes[0].0.y + unlabeled_axes[0].1.y) * 0.5
        - (unlabeled_axes[1].0.y + unlabeled_axes[1].1.y) * 0.5)
        .abs();
    let labeled_gap = ((labeled_axes[0].0.y + labeled_axes[0].1.y) * 0.5
        - (labeled_axes[1].0.y + labeled_axes[1].1.y) * 0.5)
        .abs();

    assert!(
        (unlabeled_gap - labeled_gap).abs() <= 1.0e-4,
        "{unlabeled_gap} {labeled_gap}"
    );
}

#[test]
fn render_document_ignores_legacy_label_clip_margin_for_glyph_polygons() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [56.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [56.0, 45.0],
                    "box": [51.0, 34.0, 61.0, 46.0],
                    "glyphPolygons": [[
                        [51.0, 34.0],
                        [61.0, 34.0],
                        [61.0, 46.0],
                        [51.0, 46.0]
                    ]]
                }
            }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85, "labelClipMargin": 9.0 }
        ]),
    );

    let polygon = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("bond polygon should render");
    let (from, to) = bond_axis_from_points(&polygon).expect("bond axis");
    let label_endpoint = if from.x > to.x { from } else { to };

    assert!(
        (label_endpoint.x - 51.0).abs() < 0.02,
        "glyph polygon clipping should ignore legacy labelClipMargin and avoid adding a second margin retreat: {polygon:?}"
    );
}

#[test]
fn render_document_treats_horizontal_label_interior_as_rectangular_clip() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [27.0, 40.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "C",
                "atomicNumber": 6,
                "position": [27.0, 16.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "Ph",
                    "position": [27.0, 20.0],
                    "box": [20.0, 10.0, 34.0, 22.0],
                    "runs": [{ "text": "Ph", "fontFamily": "Arial", "fontSize": 10.0, "script": "normal" }],
                    "glyphPolygons": [
                        [[20.0, 10.0], [24.0, 10.0], [24.0, 22.0], [20.0, 22.0]],
                        [[30.0, 10.0], [34.0, 10.0], [34.0, 22.0], [30.0, 22.0]]
                    ]
                }
            }
        ]),
        json!([{ "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }]),
    );

    let polygon = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("bond polygon should render");
    let label_edge_y = polygon
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);

    assert!(
        (label_edge_y - 22.0).abs() < 0.02,
        "horizontal multi-character labels should bridge only the overlapping internal glyph gap without adding a second margin retreat: {polygon:?}"
    );
}

#[test]
fn render_document_does_not_rectangularize_vertically_separated_label_glyphs() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [45.0, 20.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [22.0, 20.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "NH",
                    "position": [22.0, 24.0],
                    "box": [20.0, 6.0, 34.0, 26.0],
                    "runs": [
                        { "text": "N", "fontFamily": "Arial", "fontSize": 10.0, "script": "normal" },
                        { "text": "H", "fontFamily": "Arial", "fontSize": 7.0, "script": "superscript" }
                    ],
                    "glyphPolygons": [
                        [[20.0, 14.0], [24.0, 14.0], [24.0, 26.0], [20.0, 26.0]],
                        [[30.0, 6.0], [34.0, 6.0], [34.0, 12.0], [30.0, 12.0]]
                    ]
                }
            }
        ]),
        json!([{ "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }]),
    );

    let polygon = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("bond polygon should render");
    let (from, to) = bond_axis_from_points(&polygon).expect("bond axis");
    let label_endpoint = if from.x < to.x { from } else { to };

    assert!(
        (label_endpoint.x - 24.0).abs() < 0.02,
        "vertically separated or superscript glyphs should not rectangularize or add a second margin retreat: {polygon:?}"
    );
}

#[test]
fn render_document_clips_solid_wedge_wide_endpoint_against_outline_lines() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [42.0, 54.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "C",
                "atomicNumber": 6,
                "position": [27.0, 70.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "t-Bu",
                    "position": [27.0, 73.5],
                    "box": [12.0, 65.0, 29.3, 72.4],
                    "glyphPolygons": [
                        [[12.0, 65.0], [16.0, 65.0], [16.0, 72.4], [12.0, 72.4]],
                        [[17.0, 65.0], [21.0, 65.0], [21.0, 72.4], [17.0, 72.4]],
                        [[22.0, 65.0], [25.0, 65.0], [25.0, 72.4], [22.0, 72.4]],
                        [[26.0, 65.0], [29.3, 65.0], [29.3, 72.4], [26.0, 72.4]]
                    ]
                }
            }
        ]),
        json!([{
            "id": "b1",
            "begin": "n1",
            "end": "n2",
            "order": 1,
            "strokeWidth": 0.6,
            "wedgeWidth": 2.0,
            "stereo": { "kind": "solid-wedge", "wideEnd": "end" }
        }]),
    );

    let polygon = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("solid wedge should render");
    let cap_center = Point::new(
        (polygon[1].x + polygon[2].x) * 0.5,
        (polygon[1].y + polygon[2].y) * 0.5,
    );

    assert!(
        polygon[1].x >= 29.3 - 0.02 && polygon[2].x > 30.0 && cap_center.x > 29.9,
        "solid wedge should use the most conservative label retreat from its center and outline lines: {polygon:?}"
    );
}

#[test]
fn render_document_clips_hashed_wedge_wide_endpoint_against_outline_lines() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [42.0, 54.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "C",
                "atomicNumber": 6,
                "position": [27.0, 70.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "t-Bu",
                    "position": [27.0, 73.5],
                    "box": [12.0, 65.0, 29.3, 72.4],
                    "glyphPolygons": [
                        [[12.0, 65.0], [16.0, 65.0], [16.0, 72.4], [12.0, 72.4]],
                        [[17.0, 65.0], [21.0, 65.0], [21.0, 72.4], [17.0, 72.4]],
                        [[22.0, 65.0], [25.0, 65.0], [25.0, 72.4], [22.0, 72.4]],
                        [[26.0, 65.0], [29.3, 65.0], [29.3, 72.4], [26.0, 72.4]]
                    ]
                }
            }
        ]),
        json!([{
            "id": "b1",
            "begin": "n1",
            "end": "n2",
            "order": 1,
            "strokeWidth": 0.6,
            "wedgeWidth": 2.0,
            "stereo": { "kind": "hashed-wedge", "wideEnd": "end" }
        }]),
    );

    let points = object_bond_points_for_id(&render_document(&document), "b1");
    assert!(!points.is_empty(), "hashed wedge should render stripes");
    let cap_points = closest_points_to_target(&points, Point::new(27.0, 70.0), 2);
    let cap_center_x =
        cap_points.iter().map(|point| point.x).sum::<f64>() / cap_points.len() as f64;

    assert!(
        cap_center_x > 29.9,
        "hashed wedge label clipping should use the same outline-aware retreat as solid wedges: {points:?}"
    );
}

#[test]
fn render_document_acs_template_does_not_add_label_clip_margin() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [56.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [56.0, 45.0],
                    "box": [51.0, 34.0, 61.0, 46.0],
                    "glyphPolygons": [[
                        [51.0, 34.0],
                        [61.0, 34.0],
                        [61.0, 46.0],
                        [51.0, 46.0]
                    ]]
                }
            }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.6,
                "boldWidth": 2.0,
                "hashSpacing": 2.5,
                "bondSpacing": 18.0
            }
        ]),
    );

    let polygon = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("bond polygon should render");
    let (from, to) = bond_axis_from_points(&polygon).expect("bond axis");
    let label_endpoint = if from.x > to.x { from } else { to };
    let margin = 51.0 - label_endpoint.x;

    assert!(
        margin.abs() < 0.02,
        "ACS label clipping should use the source-margin glyph polygon without adding a second margin retreat: {margin} {polygon:?}"
    );
}

#[test]
fn render_document_keeps_center_double_parallel_with_branches_and_labeled_endpoint() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [44.0, 22.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [44.0, 58.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n4",
                "element": "N",
                "atomicNumber": 7,
                "position": [92.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [92.0, 45.0],
                    "box": [87.0, 34.0, 97.0, 46.0]
                }
            },
            { "id": "n5", "element": "C", "atomicNumber": 6, "position": [110.0, 22.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n2", "end": "n4", "order": 2, "strokeWidth": 0.85, "double": { "placement": "center" } },
            { "id": "b2", "begin": "n2", "end": "n1", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b4", "begin": "n4", "end": "n5", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2, "{polygons:?}");

    let axes: Vec<_> = polygons
        .iter()
        .map(|polygon| bond_axis_from_points(polygon).expect("bond axis"))
        .collect();
    let first_direction =
        chemsema_engine::Point::new(axes[0].1.x - axes[0].0.x, axes[0].1.y - axes[0].0.y);
    let second_direction =
        chemsema_engine::Point::new(axes[1].1.x - axes[1].0.x, axes[1].1.y - axes[1].0.y);
    let first_angle = first_direction.y.atan2(first_direction.x).to_degrees();
    let second_angle = second_direction.y.atan2(second_direction.x).to_degrees();

    assert!(
        angular_distance(first_angle, second_angle) <= 1.0e-4,
        "{polygons:?} {first_angle} {second_angle}"
    );
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
    assert!(center_double.iter().any(|polygon| {
        [polygon[1], polygon[2]]
            .iter()
            .all(|point| point_lies_on_polygon_boundary(*point, &branch_up, 1.0e-4))
    }));
    assert!(center_double.iter().any(|polygon| {
        [polygon[1], polygon[2]]
            .iter()
            .all(|point| point_lies_on_polygon_boundary(*point, &branch_down, 1.0e-4))
    }));
    assert!(shared_point_count(&branch_up, &branch_down, 1.0e-4) >= 2);
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
        chemsema_engine::Point::new(56.0, 40.0),
    );
    let b2_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b2",
        chemsema_engine::Point::new(56.0, 40.0),
    );
    let b1_main =
        side_double_main_polygon_for_bond(&polygons, "b1", chemsema_engine::Point::new(56.0, 40.0));
    let b2_main =
        side_double_main_polygon_for_bond(&polygons, "b2", chemsema_engine::Point::new(56.0, 40.0));

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
        chemsema_engine::Point::new(56.0, 40.0),
    );
    let b2_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b2",
        chemsema_engine::Point::new(56.0, 40.0),
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
        chemsema_engine::Point::new(56.0, 40.0),
    );
    let b2_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b2",
        chemsema_engine::Point::new(56.0, 40.0),
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
        chemsema_engine::Point::new(56.0, 40.0),
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
        chemsema_engine::Point::new(56.0, 40.0),
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
        chemsema_engine::Point::new(56.0, 40.0),
    );
    let centered_double: Vec<_> = polygons
        .iter()
        .filter(|(bond_id, _)| bond_id == "b2")
        .collect();

    assert!(centered_double
        .iter()
        .all(|(_, points)| { shared_point_count(&side_double_outer, points, 1.0e-4) == 0 }));
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

    assert!(
        (long_offset - short_offset * 2.0).abs() < 0.05,
        "{short_offset} {long_offset}"
    );
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
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("short wedge polygon");
    let long_polygon = render_document(&long_document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
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

    assert!(
        (short_width - long_width).abs() < 0.05,
        "{short_width} {long_width}"
    );
}

#[test]
fn render_document_uses_explicit_solid_wedge_wide_and_tip_widths() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [34.4, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.6,
                "boldWidth": 2.0,
                "wedgeWidth": 2.0,
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "end"
                }
            }
        ]),
    );

    let polygon = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("solid wedge polygon");

    let tip_width =
        ((polygon[0].x - polygon[3].x).powi(2) + (polygon[0].y - polygon[3].y).powi(2)).sqrt();
    let wide_width =
        ((polygon[1].x - polygon[2].x).powi(2) + (polygon[1].y - polygon[2].y).powi(2)).sqrt();

    assert!((tip_width - 0.6).abs() < 0.01, "{tip_width}");
    assert!((wide_width - 2.0).abs() < 0.01, "{wide_width}");
}

#[test]
fn render_document_uses_acs_template_wedge_width_for_legacy_json_without_wedge_width() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [34.4, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.6,
                "boldWidth": 2.0,
                "hashSpacing": 2.5,
                "bondSpacing": 18.0,
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "end"
                }
            }
        ]),
    );

    let polygon = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("solid wedge polygon");

    let wide_width =
        ((polygon[1].x - polygon[2].x).powi(2) + (polygon[1].y - polygon[2].y).powi(2)).sqrt();

    assert!((wide_width - 3.0).abs() < 0.01, "{wide_width}");
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
    let polygons = centered_bond_polygons(&primitives, chemsema_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 3);
    assert_eq!(
        polygons.iter().filter(|points| points.len() == 5).count(),
        3
    );
    assert!(polygons.iter().all(|points| polygon_area(points) > 0.01));
}

#[test]
fn render_document_clips_solid_wedge_in_three_way_main_contact() {
    for (begin, end, wide_end) in [("n1", "n3", "begin"), ("n3", "n1", "end")] {
        let document = fragment_document(
            json!([
                { "id": "n1", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
                { "id": "n2", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
                { "id": "n3", "element": "C", "atomicNumber": 6, "position": [96.0, 4.0], "charge": 0, "numHydrogens": 0 },
                { "id": "n4", "element": "C", "atomicNumber": 6, "position": [68.0, 88.0], "charge": 0, "numHydrogens": 0 }
            ]),
            json!([
                {
                    "id": "b1",
                    "begin": begin,
                    "end": end,
                    "order": 1,
                    "strokeWidth": 0.85,
                    "stereo": {
                        "kind": "solid-wedge",
                        "wideEnd": wide_end
                    }
                },
                { "id": "b2", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 },
                { "id": "b3", "begin": "n1", "end": "n4", "order": 1, "strokeWidth": 0.85 }
            ]),
        );

        let primitives = render_document(&document);
        let polygons = object_bond_polygons_with_ids(&primitives);
        let wedge = polygons
            .iter()
            .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
            .expect("solid wedge polygon");
        assert_eq!(wedge.len(), 5, "{wide_end} {wedge:?}");
        assert!(wedge
            .iter()
            .any(|point| point.distance(chemsema_engine::Point::new(56.0, 40.0)) <= 0.001));

        let centered = centered_bond_polygons(&primitives, chemsema_engine::Point::new(56.0, 40.0));
        assert_eq!(centered.len(), 3, "{wide_end} {centered:?}");
        assert!(centered.iter().all(|points| polygon_area(points) > 0.01));
        let center_patches = polygons
            .iter()
            .filter_map(|(bond_id, points)| bond_id.is_empty().then_some(points))
            .collect::<Vec<_>>();
        assert!(center_patches.is_empty(), "{wide_end} {polygons:?}");
    }
}

#[test]
fn render_document_uses_extended_intersections_for_solid_wedge_three_way_contact() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [cdxml_cm_to_pt(7.5), cdxml_cm_to_pt(6.5)], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [cdxml_cm_to_pt(6.45), cdxml_cm_to_pt(6.5)], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [cdxml_cm_to_pt(7.682330586550277), cdxml_cm_to_pt(5.465951859337181)], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [cdxml_cm_to_pt(7.859121150491952), cdxml_cm_to_pt(7.486677251825204)], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b_left", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": cdxml_cm_to_pt(0.035) },
            { "id": "b_up", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": cdxml_cm_to_pt(0.035) },
            {
                "id": "b_wedge",
                "begin": "n1",
                "end": "n4",
                "order": 1,
                "strokeWidth": cdxml_cm_to_pt(0.035),
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "begin"
                }
            }
        ]),
    );

    let expected_up_wedge_intersection =
        chemsema_engine::Point::new(214.1234207734643, 178.46000173212423);
    let contact_center = chemsema_engine::Point::new(cdxml_cm_to_pt(7.5), cdxml_cm_to_pt(6.5));
    let polygons = object_bond_polygons_with_ids(&render_document(&document));
    let up = polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b_up").then_some(points))
        .expect("upper single polygon");
    let wedge = polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b_wedge").then_some(points))
        .expect("solid wedge polygon");

    assert!(
        up.iter()
            .any(|point| point.distance(expected_up_wedge_intersection) <= cdxml_cm_to_pt(0.001)),
        "{up:?}"
    );
    assert!(
        wedge
            .iter()
            .any(|point| point.distance(expected_up_wedge_intersection) <= cdxml_cm_to_pt(0.001)),
        "{wedge:?}"
    );
    let has_edge = |points: &[chemsema_engine::Point],
                    first: chemsema_engine::Point,
                    second: chemsema_engine::Point| {
        (0..points.len()).any(|index| {
            let next = (index + 1) % points.len();
            (points[index].distance(first) <= cdxml_cm_to_pt(0.001)
                && points[next].distance(second) <= cdxml_cm_to_pt(0.001))
                || (points[index].distance(second) <= cdxml_cm_to_pt(0.001)
                    && points[next].distance(first) <= cdxml_cm_to_pt(0.001))
        })
    };
    assert!(
        has_edge(up, expected_up_wedge_intersection, contact_center),
        "{up:?}"
    );
    assert!(
        has_edge(wedge, expected_up_wedge_intersection, contact_center),
        "{wedge:?}"
    );
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
    let polygons = centered_bond_polygons(&primitives, chemsema_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 4);
    assert_eq!(
        polygons.iter().filter(|points| points.len() == 5).count(),
        4
    );
    assert!(polygons.iter().all(|points| polygon_area(points) > 0.01));
}
