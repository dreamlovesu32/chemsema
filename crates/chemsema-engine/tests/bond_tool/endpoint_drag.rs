use super::*;

#[test]
fn click_on_endpoint_prefers_candidate_closer_to_same_component_bond_geometry() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    click(&mut engine, FIRST_END_X, FIRST_END_Y);

    click(&mut engine, px(300.0), px(260.0));

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 4);
    let point = entry.fragment.nodes[3].position;
    assert!((point[0] - ROOT_SINGLE_BRANCH_X).abs() < 0.01, "{point:?}");
    assert!((point[1] - ROOT_SINGLE_BRANCH_Y).abs() < 0.01, "{point:?}");
}

#[test]
fn click_on_endpoint_ignores_disconnected_bond_geometry_when_choosing_direction() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    click(&mut engine, FIRST_END_X, FIRST_END_Y);

    click(&mut engine, ROOT_SINGLE_BRANCH_X, ROOT_SINGLE_BRANCH_Y);

    click(&mut engine, px(300.0), px(260.0));

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 5);
    let point = entry.fragment.nodes[4].position;
    assert!(
        (point[0] - ROOT_OPPOSITE_BRANCH_X).abs() < 0.01,
        "{point:?}"
    );
    assert!(
        (point[1] - ROOT_OPPOSITE_BRANCH_Y).abs() < 0.01,
        "{point:?}"
    );
}

#[test]
fn drag_from_endpoint_uses_fixed_length_and_angle_snap() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: px(370.0),
        y: px(292.0),
        button: None,
        alt_key: false,
    });
    assert!(engine.state().overlay.preview.is_some());
    engine.pointer_up(PointerEvent {
        x: px(370.0),
        y: px(292.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    let length = ((last.position[0] - FIRST_END_X).powi(2)
        + (last.position[1] - FIRST_END_Y).powi(2))
    .sqrt();
    assert!((length - DEFAULT_BOND_LENGTH).abs() < 0.01, "{length}");
    assert_eq!(fragment_counts(&engine), (3, 2));
}

#[test]
fn drag_from_endpoint_snaps_to_15_degree_increment() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: px(371.0),
        y: px(271.0),
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(371.0),
        y: px(271.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    let dx = last.position[0] - FIRST_END_X;
    let dy = last.position[1] - FIRST_END_Y;
    let angle = dy.atan2(dx).to_degrees().rem_euclid(360.0);

    assert!((angle - 45.0).abs() < 0.01, "{angle} {:?}", last.position);
    let length = (dx.powi(2) + dy.powi(2)).sqrt();
    assert!((length - DEFAULT_BOND_LENGTH).abs() < 0.01, "{length}");
}

#[test]
fn drag_preview_renders_document_geometry_instead_of_overlay_line() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: px(370.0),
        y: px(292.0),
        button: None,
        alt_key: false,
    });

    let render_list = engine.render_list();
    assert!(!render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Line { role, .. } if *role == RenderRole::PreviewBond
    )));
    assert!(render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Circle { role, .. } if *role == RenderRole::PreviewEnd
    )));
    assert!(
        render_list
            .iter()
            .filter(|primitive| matches!(
                primitive,
                RenderPrimitive::Polygon { role, .. } if *role == RenderRole::DocumentBond
            ))
            .count()
            >= 2,
        "{render_list:?}"
    );
}

#[test]
fn drag_preview_interaction_render_list_only_contains_preview_geometry() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: px(370.0),
        y: px(292.0),
        button: None,
        alt_key: false,
    });

    let interaction = engine.interaction_render_list();
    assert!(interaction.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Path { role, .. }
            | RenderPrimitive::Polygon { role, .. }
            | RenderPrimitive::FilledPath { role, .. }
            if *role == RenderRole::PreviewBond
    )));
    assert!(interaction.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Circle { role, .. } if *role == RenderRole::PreviewEnd
    )));
    assert!(interaction.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Circle {
            role: RenderRole::PreviewEnd,
            fill,
            ..
        } if fill == "rgba(47,111,237,0.82)"
    )));
    assert!(interaction.iter().all(|primitive| !matches!(
        primitive,
        RenderPrimitive::Path { role, .. }
            | RenderPrimitive::Polygon { role, .. }
            | RenderPrimitive::FilledPath { role, .. }
            if *role == RenderRole::DocumentBond
    )));
    assert!(
        interaction.len() <= 8,
        "preview interaction list should stay small: {interaction:?}"
    );
}

#[test]
fn bond_tool_endpoint_hover_and_preview_handles_use_tinted_not_white_fill() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    hover(&mut engine, FIRST_END_X, FIRST_END_Y);
    let render_list = engine.render_list();
    assert!(render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Circle {
            role: RenderRole::HoverEndpoint,
            fill,
            ..
        } if fill == "rgba(47,111,237,0.82)"
    )));

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: px(370.0),
        y: px(292.0),
        button: None,
        alt_key: false,
    });
    let interaction = engine.interaction_render_list();
    assert!(interaction.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Circle {
            role: RenderRole::PreviewEnd,
            fill,
            ..
        } if fill == "rgba(47,111,237,0.82)"
    )));
}

#[test]
fn bond_tool_edits_endpoint_fragment_inside_nonfirst_molecule_object() {
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc",
            "title": "Two molecules",
            "page": { "width": 500.0, "height": 300.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": DEFAULT_BOND_STROKE,
                "fontFamily": "Arial",
                "fontSize": 10.0
            }
        },
        "objects": [
            {
                "id": "obj_first",
                "type": "molecule",
                "visible": true,
                "zIndex": 1,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_molecule_default",
                "payload": { "resourceRef": "mol_first", "bbox": [0.0, 0.0, 20.0, 20.0] }
            },
            {
                "id": "group_parent",
                "type": "group",
                "visible": true,
                "zIndex": 2,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": {},
                "children": [
                    {
                        "id": "obj_second",
                        "type": "molecule",
                        "visible": true,
                        "zIndex": 1,
                        "transform": { "translate": [100.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                        "styleRef": "style_molecule_default",
                        "payload": { "resourceRef": "mol_second", "bbox": [0.0, 0.0, 20.0, 20.0] }
                    }
                ]
            }
        ],
        "resources": {
            "mol_first": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 20.0, 20.0],
                    "nodes": [
                        { "id": "first_a", "element": "C", "atomicNumber": 6, "position": [0.0, 0.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": []
                }
            },
            "mol_second": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 20.0, 20.0],
                    "nodes": [
                        { "id": "second_a", "element": "C", "atomicNumber": 6, "position": [0.0, 0.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": []
                }
            }
        }
    });
    let mut engine = Engine::new();
    engine
        .load_document_json(&document.to_string())
        .expect("document should load");
    engine.set_tool_state(bond_tool());

    let anchor = Point::new(100.0, 20.0);
    engine.pointer_down(PointerEvent {
        x: anchor.x,
        y: anchor.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: anchor.x + 30.0,
        y: anchor.y,
        button: None,
        alt_key: false,
    });
    assert!(engine
        .interaction_render_list()
        .iter()
        .any(|primitive| matches!(
            primitive,
            RenderPrimitive::Path { role, .. }
                | RenderPrimitive::Polygon { role, .. }
                | RenderPrimitive::FilledPath { role, .. }
                if *role == RenderRole::PreviewBond
        )));
    engine.pointer_up(PointerEvent {
        x: anchor.x,
        y: anchor.y,
        button: Some(0),
        alt_key: false,
    });

    let document = parse_document_json(&engine.document_json().expect("document json"))
        .expect("document should parse");
    let first = document
        .resources
        .get("mol_first")
        .and_then(|resource| resource.data.as_fragment())
        .expect("first fragment");
    let second = document
        .resources
        .get("mol_second")
        .and_then(|resource| resource.data.as_fragment())
        .expect("second fragment");
    assert_eq!(first.bonds.len(), 0);
    assert_eq!(second.bonds.len(), 1);
    assert!(second
        .bonds
        .iter()
        .any(|bond| bond.begin == "second_a" || bond.end == "second_a"));
}
