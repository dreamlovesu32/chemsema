use super::*;

#[test]
fn template_tool_hover_shows_label_anchor_snap_target_before_drag() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": { "id": "doc_test", "title": "test", "page": { "width": 400.0, "height": 300.0, "background": "#ffffff" } },
        "styles": {
            "style_molecule_default": { "kind": "molecule", "stroke": "#000000", "strokeWidth": 0.85, "fontFamily": "Arial", "fontSize": 11.0 }
        },
        "objects": [{
            "id": "obj_molecule_001",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_001", "bbox": [0.0, 0.0, 200.0, 120.0] }
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 200.0, 120.0],
                    "nodes": [{
                        "id": "n1",
                        "element": "C",
                        "atomicNumber": 6,
                        "position": [100.0, 60.0],
                        "charge": 0,
                        "numHydrogens": 0,
                        "label": {
                            "text": "Ph",
                            "position": [100.0, 63.5],
                            "box": [96.0, 54.0, 110.0, 66.0],
                            "glyphPolygons": [
                                [[96.0, 54.0], [102.0, 54.0], [102.0, 66.0], [96.0, 66.0]],
                                [[104.0, 54.0], [110.0, 54.0], [110.0, 66.0], [104.0, 66.0]]
                            ]
                        }
                    }],
                    "bonds": []
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("document should load");
    engine.set_tool_state(templates_tool("ring-6"));

    let hit_point = {
        let label = engine
            .state()
            .document
            .editable_fragment()
            .unwrap()
            .fragment
            .nodes[0]
            .label
            .as_ref()
            .unwrap();
        label_glyph_hit_point(label, 1)
    };

    engine.pointer_move(PointerEvent {
        x: hit_point.x,
        y: hit_point.y,
        button: None,
        alt_key: false,
    });

    let hover = engine
        .state()
        .overlay
        .hover_endpoint
        .as_ref()
        .expect("template tool should expose label glyph anchors like the bond tool");
    assert_eq!(hover.node_id, "n1");
    assert!(hover.label_anchor.is_some(), "{hover:?}");
}

#[test]
fn template_tool_hover_shows_bond_center_snap_target_before_drag() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.set_tool_state(templates_tool("ring-6"));
    engine.pointer_move(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: None,
        alt_key: false,
    });

    let hover = engine
        .state()
        .overlay
        .hover_bond_center
        .as_ref()
        .expect("template tool should expose bond-center snap hover before drag");
    assert!(
        hover
            .point
            .distance(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y))
            < px(3.0),
        "{hover:?}"
    );
}

#[test]
fn template_chain_drag_on_blank_canvas_creates_open_zigzag() {
    let mut engine = Engine::new();
    let start = px_point(300.0, 260.0);
    let target = start.translated(direction_from_angle(0.0).scaled(DEFAULT_BOND_LENGTH * 4.1));

    engine.set_tool_state(templates_tool("chain"));
    drag(&mut engine, start, target);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 5);
    assert_eq!(entry.fragment.bonds.len(), 4);
    for bond in &entry.fragment.bonds {
        let begin = node_world_point(&engine, &bond.begin);
        let end = node_world_point(&engine, &bond.end);
        assert!((begin.distance(end) - DEFAULT_BOND_LENGTH).abs() < 0.001);
    }
    assert!(!entry.fragment.bonds.iter().any(|bond| {
        (bond.begin == "n_1" && bond.end == "n_5") || (bond.begin == "n_5" && bond.end == "n_1")
    }));
    let first_angle = angle_between(
        node_world_point(&engine, "n_1"),
        node_world_point(&engine, "n_2"),
    );
    let second_angle = angle_between(
        node_world_point(&engine, "n_2"),
        node_world_point(&engine, "n_3"),
    );
    assert!(chemsema_engine::angular_distance(first_angle, 30.0) < 0.2);
    assert!(chemsema_engine::angular_distance(second_angle, 330.0) < 0.2);
}

#[test]
fn template_chain_drag_on_endpoint_reuses_anchor_node() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let endpoint = node_world_point(&engine, "n_2");
    let target = endpoint.translated(direction_from_angle(45.0).scaled(DEFAULT_BOND_LENGTH * 3.1));
    engine.set_tool_state(templates_tool("chain"));
    drag(&mut engine, endpoint, target);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 5);
    assert_eq!(entry.fragment.bonds.len(), 4);
    assert_eq!(attached_node_points(&engine, "n_2").len(), 2);
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_chain_click_without_drag_does_not_insert_chain() {
    let mut engine = Engine::new();
    engine.set_tool_state(templates_tool("chain"));

    click(&mut engine, px(300.0), px(260.0));

    assert_eq!(fragment_counts(&engine), (0, 0));
}

#[test]
fn template_chain_drag_preview_shows_terminal_count_label() {
    let mut engine = Engine::new();
    let start = px_point(300.0, 260.0);
    let target = start.translated(direction_from_angle(0.0).scaled(DEFAULT_BOND_LENGTH * 3.1));
    engine.set_tool_state(templates_tool("chain"));

    engine.pointer_down(PointerEvent {
        x: start.x,
        y: start.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: target.x,
        y: target.y,
        button: None,
        alt_key: false,
    });

    let preview_label = engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                object_id: Some(object_id),
                text,
                font_size,
                ..
            } if object_id == "__preview_chain_count" => Some((text, font_size)),
            _ => None,
        })
        .expect("chain drag should show terminal count label");
    assert_eq!(preview_label.0, "4");
    assert_eq!(preview_label.1, 8.0);

    engine.pointer_up(PointerEvent {
        x: target.x,
        y: target.y,
        button: Some(0),
        alt_key: false,
    });
    assert!(engine.render_list().into_iter().all(|primitive| {
        !matches!(
            primitive,
            RenderPrimitive::Text {
                object_id: Some(object_id),
                ..
            } if object_id == "__preview_chain_count"
        )
    }));
}

#[test]
fn template_click_on_blank_canvas_creates_regular_ring() {
    let mut engine = Engine::new();
    let anchor = px_point(300.0, 260.0);

    engine.set_tool_state(templates_tool("ring-6"));
    click(&mut engine, anchor.x, anchor.y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 6);
    assert_eq!(entry.fragment.bonds.len(), 6);
    let ring_points = entry
        .fragment
        .nodes
        .iter()
        .map(|node| entry.world_point_for_node(node))
        .collect::<Vec<_>>();
    let center = Point::new(
        ring_points.iter().map(|point| point.x).sum::<f64>() / ring_points.len() as f64,
        ring_points.iter().map(|point| point.y).sum::<f64>() / ring_points.len() as f64,
    );
    assert!(center.distance(anchor) < 0.002, "{center:?} {anchor:?}");
    assert!(ring_points
        .iter()
        .all(|point| point.distance(anchor) > 0.01));
    for bond in &entry.fragment.bonds {
        let begin = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)
            .map(|node| entry.world_point_for_node(node))
            .unwrap();
        let end = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.end)
            .map(|node| entry.world_point_for_node(node))
            .unwrap();
        assert!(
            (begin.distance(end) - DEFAULT_BOND_LENGTH).abs() < 0.01,
            "{begin:?} {end:?}"
        );
    }
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_benzene_click_creates_alternating_double_bonds() {
    let mut engine = Engine::new();
    let anchor = px_point(300.0, 260.0);

    engine.set_tool_state(templates_tool("benzene"));
    click(&mut engine, anchor.x, anchor.y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 6);
    assert_eq!(entry.fragment.bonds.len(), 6);
    assert_eq!(
        entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.order == 2 && bond.double.is_some())
            .count(),
        3
    );
    assert_eq!(
        entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.order == 1 && bond.double.is_none())
            .count(),
        3
    );
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_benzene_click_on_bond_keeps_fused_side_and_adds_three_double_bonds() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.set_tool_state(templates_tool("benzene"));
    click(&mut engine, FIRST_CENTER_X, FIRST_CENTER_Y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 6);
    assert_eq!(entry.fragment.bonds.len(), 6);
    let original_bond = entry
        .fragment
        .bonds
        .iter()
        .find(|bond| bond.id == "b_3")
        .expect("clicked bond should be reused");
    assert_eq!(original_bond.order, 1);
    assert_eq!(
        entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.order == 2 && bond.double.is_some())
            .count(),
        3
    );
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_benzene_on_double_bond_reuses_shared_double_and_rekekulizes_ring() {
    let mut engine = Engine::new();
    engine.set_tool_state(double_bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let original_bond_id = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment
        .bonds[0]
        .id
        .clone();
    engine.set_tool_state(templates_tool("benzene"));
    click(&mut engine, FIRST_CENTER_X, FIRST_CENTER_Y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 6);
    assert_eq!(entry.fragment.bonds.len(), 6);
    assert_eq!(
        entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.order == 2)
            .count(),
        3,
        "the shared double must replace, not supplement, one aromatic double"
    );
    let original_bond = entry
        .fragment
        .bonds
        .iter()
        .find(|bond| bond.id == original_bond_id)
        .expect("the clicked double bond should be reused");
    assert_eq!(original_bond.order, 2);
    assert_ne!(
        original_bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center),
        "the reused double should move to the inside of the new ring"
    );
    for node in &entry.fragment.nodes {
        assert_eq!(
            entry
                .fragment
                .bonds
                .iter()
                .filter(|bond| {
                    bond.order == 2 && (bond.begin == node.id || bond.end == node.id)
                })
                .count(),
            1,
            "each benzene vertex should touch exactly one double bond"
        );
    }
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_benzene_fused_to_aromatic_double_reuses_one_double_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(templates_tool("benzene"));
    click(&mut engine, px(300.0), px(260.0));

    let (shared_bond_id, shared_center) = {
        let entry = engine.state().document.editable_fragment().unwrap();
        let shared = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.order == 2)
            .expect("benzene should contain a double bond");
        let begin = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == shared.begin)
            .map(|node| entry.world_point_for_node(node))
            .unwrap();
        let end = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == shared.end)
            .map(|node| entry.world_point_for_node(node))
            .unwrap();
        (
            shared.id.clone(),
            Point::new((begin.x + end.x) * 0.5, (begin.y + end.y) * 0.5),
        )
    };

    click(&mut engine, shared_center.x, shared_center.y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 10);
    assert_eq!(entry.fragment.bonds.len(), 11);
    assert_eq!(
        entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.order == 2)
            .count(),
        5,
        "two fused benzene rings should share one of their double bonds"
    );
    assert_eq!(
        entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.id == shared_bond_id)
            .count(),
        1,
        "the shared edge must not be duplicated"
    );
    for node in &entry.fragment.nodes {
        assert_eq!(
            entry
                .fragment
                .bonds
                .iter()
                .filter(|bond| {
                    bond.order == 2 && (bond.begin == node.id || bond.end == node.id)
                })
                .count(),
            1,
            "the fused Kekule layout should not place adjacent doubles at one vertex"
        );
    }
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_benzene_fused_to_aromatic_single_relayouts_both_rings() {
    let mut engine = Engine::new();
    engine.set_tool_state(templates_tool("benzene"));
    click(&mut engine, px(300.0), px(260.0));

    let (shared_bond_id, shared_center) = {
        let entry = engine.state().document.editable_fragment().unwrap();
        let shared = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.order == 1)
            .expect("benzene should contain a single bond");
        let begin = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == shared.begin)
            .map(|node| entry.world_point_for_node(node))
            .unwrap();
        let end = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == shared.end)
            .map(|node| entry.world_point_for_node(node))
            .unwrap();
        (
            shared.id.clone(),
            Point::new((begin.x + end.x) * 0.5, (begin.y + end.y) * 0.5),
        )
    };

    click(&mut engine, shared_center.x, shared_center.y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 10);
    assert_eq!(entry.fragment.bonds.len(), 11);
    assert_eq!(
        entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.order == 2)
            .count(),
        5
    );
    assert_eq!(
        entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.id == shared_bond_id)
            .map(|bond| bond.order),
        Some(2),
        "a fused aromatic ring should make the shared edge one of the reused doubles"
    );
    for node in &entry.fragment.nodes {
        assert_eq!(
            entry
                .fragment
                .bonds
                .iter()
                .filter(|bond| {
                    bond.order == 2 && (bond.begin == node.id || bond.end == node.id)
                })
                .count(),
            1
        );
    }
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_chair_click_on_blank_canvas_centers_shape() {
    let mut engine = Engine::new();
    let center = px_point(300.0, 260.0);

    engine.set_tool_state(templates_tool("chair-6-right"));
    click(&mut engine, center.x, center.y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 6);
    assert_eq!(entry.fragment.bonds.len(), 6);
    assert!(entry
        .fragment
        .bonds
        .iter()
        .all(|bond| bond.order == 1 && bond.double.is_none()));
    let points = entry
        .fragment
        .nodes
        .iter()
        .map(|node| entry.world_point_for_node(node))
        .collect::<Vec<_>>();
    let actual_center = Point::new(
        points.iter().map(|point| point.x).sum::<f64>() / points.len() as f64,
        points.iter().map(|point| point.y).sum::<f64>() / points.len() as f64,
    );
    assert!(
        actual_center.distance(center) < 0.005,
        "{actual_center:?} {center:?}"
    );
    let first_bond_angle = chemsema_engine::angle_between(points[0], points[1]);
    assert!(
        chemsema_engine::angular_distance(60.0, first_bond_angle) < 0.2,
        "{first_bond_angle}"
    );
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_chair_drag_on_blank_canvas_uses_initial_point_as_anchor() {
    let mut engine = Engine::new();
    let anchor = px_point(300.0, 260.0);
    let target = anchor.translated(direction_from_angle(22.0).scaled(DEFAULT_BOND_LENGTH * 2.0));

    engine.set_tool_state(templates_tool("chair-6-right"));
    drag(&mut engine, anchor, target);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 6);
    assert_eq!(entry.fragment.bonds.len(), 6);
    assert!(entry
        .fragment
        .nodes
        .iter()
        .any(|node| entry.world_point_for_node(node).distance(anchor) < 0.01));
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_chair_endpoint_click_aligns_anchor_bisector_with_existing_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    let existing_begin = node_world_point(&engine, "n_1");
    let endpoint = node_world_point(&engine, "n_2");

    engine.set_tool_state(templates_tool("chair-6-right"));
    click(&mut engine, endpoint.x, endpoint.y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 7);
    assert_eq!(entry.fragment.bonds.len(), 7);
    let new_neighbors = attached_node_points(&engine, "n_2")
        .into_iter()
        .filter(|point| point.distance(existing_begin) > 0.01)
        .collect::<Vec<_>>();
    assert_eq!(new_neighbors.len(), 2, "{new_neighbors:?}");
    let first = chemsema_engine::Vector::new(
        new_neighbors[0].x - endpoint.x,
        new_neighbors[0].y - endpoint.y,
    )
    .normalized();
    let second = chemsema_engine::Vector::new(
        new_neighbors[1].x - endpoint.x,
        new_neighbors[1].y - endpoint.y,
    )
    .normalized();
    let bisector = Point::new(
        endpoint.x + first.x + second.x,
        endpoint.y + first.y + second.y,
    );
    let expected_axis = chemsema_engine::angle_between(existing_begin, endpoint);
    let actual_axis = chemsema_engine::angle_between(endpoint, bisector);
    assert!(
        chemsema_engine::angular_distance(expected_axis, actual_axis) < 0.2,
        "{expected_axis} {actual_axis}"
    );
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_drag_on_blank_canvas_snaps_ring_axis_to_15_degrees() {
    let mut engine = Engine::new();
    let anchor = px_point(300.0, 260.0);
    let target = anchor.translated(direction_from_angle(22.0).scaled(DEFAULT_BOND_LENGTH * 2.0));

    engine.set_tool_state(templates_tool("ring-6"));
    engine.pointer_down(PointerEvent {
        x: anchor.x,
        y: anchor.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: target.x,
        y: target.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: target.x,
        y: target.y,
        button: Some(0),
        alt_key: false,
    });

    let ring_points = {
        let entry = engine.state().document.editable_fragment().unwrap();
        assert_eq!(entry.fragment.nodes.len(), 6);
        assert_eq!(entry.fragment.bonds.len(), 6);
        entry
            .fragment
            .nodes
            .iter()
            .map(|node| entry.world_point_for_node(node))
            .collect::<Vec<_>>()
    };
    let center = Point::new(
        ring_points.iter().map(|point| point.x).sum::<f64>() / ring_points.len() as f64,
        ring_points.iter().map(|point| point.y).sum::<f64>() / ring_points.len() as f64,
    );
    assert!(ring_points
        .iter()
        .any(|point| point.distance(anchor) < 0.01));
    assert!((chemsema_engine::angle_between(anchor, center) - 15.0).abs() < 0.2);
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_drag_on_blank_canvas_keeps_live_focus_on_initial_connection() {
    let mut engine = Engine::new();
    let anchor = px_point(300.0, 260.0);
    let target = anchor.translated(direction_from_angle(22.0).scaled(DEFAULT_BOND_LENGTH * 2.0));

    engine.set_tool_state(templates_tool("ring-6"));
    engine.pointer_down(PointerEvent {
        x: anchor.x,
        y: anchor.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: target.x,
        y: target.y,
        button: None,
        alt_key: false,
    });

    let hover = engine
        .state()
        .overlay
        .hover_endpoint
        .as_ref()
        .expect("template drag should keep live focus on the initial generated connection");
    assert!((hover.point.x - anchor.x).abs() < 0.001, "{hover:?}");
    assert!((hover.point.y - anchor.y).abs() < 0.001, "{hover:?}");
    assert!(hover.point.distance(target) > DEFAULT_BOND_LENGTH);
    let preview = engine
        .state()
        .overlay
        .preview
        .as_ref()
        .expect("template drag should keep the ring preview active");
    assert!((preview.end.x - anchor.x).abs() < 0.001, "{preview:?}");
    assert!((preview.end.y - anchor.y).abs() < 0.001, "{preview:?}");
    assert!(preview.end.distance(target) > DEFAULT_BOND_LENGTH);
}

#[test]
fn template_click_reuses_existing_endpoint_at_generated_ring_vertex() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let ring_points = fused_ring_points_for_bond(
        node_world_point(&engine, "n_1"),
        node_world_point(&engine, "n_2"),
        6,
        1.0,
    );
    let reusable_point = ring_points[2];
    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: reusable_point,
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: reusable_point
                .translated(direction_from_angle(37.0).scaled(DEFAULT_BOND_LENGTH * 1.7)),
            label_anchor: None,
        },
    );
    let reusable_id = node_id_at(&engine, reusable_point).expect("reusable node should exist");

    engine.set_tool_state(templates_tool("ring-6"));
    click(&mut engine, FIRST_CENTER_X, FIRST_CENTER_Y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 7);
    assert!(entry.fragment.bonds.iter().any(|bond| {
        (bond.begin == "n_2" && bond.end == reusable_id)
            || (bond.begin == reusable_id && bond.end == "n_2")
    }));
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn hover_focuses_existing_endpoint() {
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

    hover(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);

    let hover = engine.state().overlay.hover_endpoint.as_ref().unwrap();
    assert_eq!(hover.point.x, FIRST_END_X);
    assert_eq!(hover.point.y, FIRST_END_Y);
    let expected_radius = engine.options().bold_bond_width * 0.75;
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Circle {
            role: RenderRole::HoverEndpoint,
            radius,
            ..
        } if (*radius - expected_radius).abs() < 0.001
    )));
}

#[test]
fn hover_bond_center_rect_uses_half_bond_length_and_bold_width() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    hover(&mut engine, FIRST_CENTER_X, FIRST_CENTER_Y);

    let expected_width = engine.options().bold_bond_width;
    let expected_length = Point::new(FIRST_START_X, FIRST_START_Y)
        .distance(Point::new(FIRST_END_X, FIRST_END_Y))
        * 0.5;
    let hover = engine.state().overlay.hover_bond_center.as_ref().unwrap();
    assert!((hover.width - expected_width).abs() < 0.001);

    let render_list = engine.render_list();
    let polygon = render_list
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role: RenderRole::HoverBondCenter,
                points,
                fill,
                stroke,
                ..
            } => Some((points.clone(), fill.as_str(), stroke.as_str())),
            _ => None,
        })
        .expect("hover bond center polygon should render");
    let (points, fill, stroke) = polygon;
    assert_eq!(fill, "rgba(47,111,237,0.72)");
    assert_eq!(stroke, "none");
    let mut lengths = polygon_edge_lengths(&points);
    lengths.sort_by(f64::total_cmp);

    assert!((lengths[0] - expected_width).abs() < 0.001, "{lengths:?}");
    assert!((lengths[1] - expected_width).abs() < 0.001, "{lengths:?}");
    assert!((lengths[2] - expected_length).abs() < 0.001, "{lengths:?}");
    assert!((lengths[3] - expected_length).abs() < 0.001, "{lengths:?}");
}

#[test]
fn hovered_endpoint_can_be_replaced_with_element_label() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());

    click(&mut engine, px(300.0), px(260.0));
    hover(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);

    assert!(engine.replace_hovered_endpoint_label("N"));

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.position == [FIRST_END_X, FIRST_END_Y])
        .unwrap();
    assert_eq!(node.element, "N");
    assert_eq!(node.atomic_number, 7);
    assert_eq!(node.num_hydrogens, 2);
    assert!(!node.is_placeholder);
    assert_eq!(
        node.label.as_ref().map(|label| label.text.as_str()),
        Some("NH2")
    );
    assert_eq!(
        node.label.as_ref().and_then(|label| label.font_size),
        Some(chemsema_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT)
    );
    assert_eq!(
        node.label.as_ref().and_then(|label| label.align.as_deref()),
        Some("left")
    );
}

#[test]
fn shortcut_generated_hydrogen_label_preserves_manual_endpoint_text_edit() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());

    click(&mut engine, px(300.0), px(260.0));
    hover(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);
    assert!(engine.replace_hovered_endpoint_label("N"));

    let label_box = {
        let entry = engine.state().document.editable_fragment().unwrap();
        let node = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.position == [FIRST_END_X, FIRST_END_Y])
            .unwrap();
        assert_eq!(
            node.label
                .as_ref()
                .and_then(|label| label.source_text.as_deref()),
            Some("NH2")
        );
        assert!(
            node.meta.get("labelRecognition").is_none(),
            "generated one-connection NH2 should be valid"
        );
        node.label.as_ref().and_then(|label| label.bbox()).unwrap()
    };
    let session = engine
        .begin_text_edit(Point::new(
            (label_box[0] + label_box[2]) * 0.5,
            (label_box[1] + label_box[3]) * 0.5,
        ))
        .expect("label edit session should start");
    assert!(engine.apply_text_edit(chemsema_engine::TextEditSession {
        text: "NH".to_string(),
        source_runs: Vec::new(),
        ..session
    }));

    let label_box = {
        let entry = engine.state().document.editable_fragment().unwrap();
        let node = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.position == [FIRST_END_X, FIRST_END_Y])
            .unwrap();
        assert_eq!(node.element, "N");
        assert_eq!(node.num_hydrogens, 2);
        assert_eq!(
            node.label
                .as_ref()
                .and_then(|label| label.source_text.as_deref()),
            Some("NH")
        );
        assert_eq!(
            node.meta
                .get("implicitHydrogenLabel")
                .and_then(|value| value.get("userEdited"))
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert_eq!(
            node.label
                .as_ref()
                .and_then(|label| label.meta.get("implicitHydrogenLabel"))
                .and_then(|value| value.get("userEdited"))
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert_eq!(
            node.meta
                .get("labelRecognition")
                .and_then(|value| value.get("status"))
                .and_then(serde_json::Value::as_str),
            Some("invalid")
        );
        node.label.as_ref().and_then(|label| label.bbox()).unwrap()
    };

    let session = engine
        .begin_text_edit(Point::new(
            (label_box[0] + label_box[2]) * 0.5,
            (label_box[1] + label_box[3]) * 0.5,
        ))
        .expect("label edit session should restart");
    assert!(engine.apply_text_edit(chemsema_engine::TextEditSession {
        text: "NX".to_string(),
        source_runs: Vec::new(),
        ..session
    }));
    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.position == [FIRST_END_X, FIRST_END_Y])
        .unwrap();
    assert_eq!(
        node.meta
            .get("labelRecognition")
            .and_then(|value| value.get("status"))
            .and_then(serde_json::Value::as_str),
        Some("invalid")
    );
}

#[test]
fn hovered_endpoint_can_be_replaced_with_abbreviation_label() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());

    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(delete_tool());
    engine.set_tool_state(bond_tool());
    hover(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);

    assert!(engine.replace_hovered_endpoint_label("Ph"));

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.position == [FIRST_END_X, FIRST_END_Y])
        .unwrap();
    assert_eq!(node.element, "C");
    assert_eq!(node.atomic_number, 6);
    assert!(node.is_placeholder);
    assert_eq!(
        node.label.as_ref().map(|label| label.text.as_str()),
        Some("Ph")
    );
    assert_eq!(
        node.label.as_ref().and_then(|label| label.font_size),
        Some(chemsema_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT)
    );
    assert_eq!(
        node.label.as_ref().and_then(|label| label.align.as_deref()),
        Some("left")
    );

    engine.set_tool_state(select_tool());
    assert_eq!(engine.state().selection.nodes.len(), 2);
    assert_eq!(engine.state().selection.bonds.len(), 1);
}

#[test]
fn labeled_node_center_no_longer_focuses_plain_endpoint() {
    let mut engine = Engine::new();
    let glyph_polygons = vec![
        rect_polygon(304.0, 256.0, 310.0, 264.0),
        rect_polygon(312.0, 256.0, 318.0, 264.0),
        rect_polygon(320.0, 256.0, 326.0, 264.0),
        rect_polygon(328.0, 256.0, 334.0, 264.0),
    ];
    let bonds = json!([{
        "id": "b1",
        "begin": "n0",
        "end": "n1",
        "order": 1
    }]);
    load_label_document(&mut engine, "CuF3", glyph_polygons, bonds);
    engine.set_tool_state(bond_tool());

    hover(&mut engine, px(300.0), px(260.0));
    assert!(
        engine
            .state()
            .overlay
            .hover_endpoint
            .as_ref()
            .is_none_or(|hit| hit.label_anchor.is_some()),
        "a labeled node center may hit its real glyph outline, but must never fall through to a plain endpoint"
    );
}

#[test]
fn hovered_endpoint_can_be_cleared_back_to_carbon() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());

    click(&mut engine, px(300.0), px(260.0));
    hover(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);
    assert!(engine.replace_hovered_endpoint_label("Me"));
    hover(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);

    assert!(engine.replace_hovered_endpoint_label("C"));

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.position == [FIRST_END_X, FIRST_END_Y])
        .unwrap();
    assert_eq!(node.element, "C");
    assert_eq!(node.atomic_number, 6);
    assert!(!node.is_placeholder);
    assert!(node.label.is_none());
}

#[test]
fn hover_focuses_label_glyph_anchor() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    load_label_document(
        &mut engine,
        "CuF3",
        vec![
            rect_polygon(294.0, 256.0, 300.0, 264.0),
            rect_polygon(302.0, 256.0, 308.0, 264.0),
            rect_polygon(310.0, 256.0, 316.0, 264.0),
            rect_polygon(318.0, 256.0, 324.0, 264.0),
        ],
        json!([]),
    );

    let (hit_point, expected_anchor, expected_glyph_box) = {
        let label = engine
            .state()
            .document
            .editable_fragment()
            .unwrap()
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == "n1")
            .unwrap()
            .label
            .as_ref()
            .unwrap();
        (
            label_glyph_hit_point(label, 1),
            label_glyph_anchor(label, 1),
            label_glyph_box(label, 1),
        )
    };
    engine.pointer_move(PointerEvent {
        x: hit_point.x,
        y: hit_point.y,
        button: None,
        alt_key: false,
    });

    let hover = engine.state().overlay.hover_endpoint.as_ref().unwrap();
    assert_eq!(hover.node_id, "n1");
    assert!(hover.point.distance(expected_anchor) < 0.001, "{hover:?}");
    let anchor = hover
        .label_anchor
        .as_ref()
        .expect("label anchor should exist");
    for (actual, expected) in anchor.glyph_box.iter().zip(expected_glyph_box) {
        assert!((*actual - expected).abs() < 0.001, "{anchor:?}");
    }

    let render_list = engine.render_list();
    assert!(render_list.iter().any(|primitive| {
        matches!(
            primitive,
            RenderPrimitive::Rect {
                role: RenderRole::HoverLabelGlyph,
                x,
                y,
                width,
                height,
                ..
            } if (*x - expected_glyph_box[0]).abs() < 0.001
                && (*y - expected_glyph_box[1]).abs() < 0.001
                && (*width - (expected_glyph_box[2] - expected_glyph_box[0])).abs() < 0.001
                && (*height - (expected_glyph_box[3] - expected_glyph_box[1])).abs() < 0.001
        )
    }));
}

#[test]
fn click_on_label_glyph_uses_rightmost_group_anchor_for_default_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    load_label_document(
        &mut engine,
        "CuF3",
        vec![
            rect_polygon(294.0, 256.0, 300.0, 264.0),
            rect_polygon(302.0, 256.0, 308.0, 264.0),
            rect_polygon(310.0, 256.0, 316.0, 264.0),
            rect_polygon(318.0, 256.0, 324.0, 264.0),
        ],
        json!([]),
    );

    let (hit_point, right_group_anchor) = {
        let label = engine
            .state()
            .document
            .editable_fragment()
            .unwrap()
            .fragment
            .nodes[0]
            .label
            .as_ref()
            .unwrap();
        (
            label_glyph_hit_point(label, 1),
            label_glyph_anchor(label, 2),
        )
    };
    click(&mut engine, hit_point.x, hit_point.y);

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    let expected = endpoint_from_anchor(right_group_anchor, 0.0);
    assert!(
        (last.position[0] - expected.x).abs() < 0.01,
        "{:?}",
        last.position
    );
    assert!(
        (last.position[1] - expected.y).abs() < 0.01,
        "{:?}",
        last.position
    );
}

#[test]
fn single_group_label_right_anchor_uses_terminal_letter_but_not_digit() {
    for (label, expected_anchor_index) in [("Ph", 1), ("N3", 0)] {
        let mut engine = Engine::new();
        engine.set_tool_state(bond_tool());
        load_label_document(
            &mut engine,
            label,
            vec![
                rect_polygon(294.0, 256.0, 300.0, 264.0),
                rect_polygon(302.0, 256.0, 308.0, 264.0),
            ],
            json!([]),
        );

        let (hit_point, expected_anchor) = {
            let node_label = engine
                .state()
                .document
                .editable_fragment()
                .unwrap()
                .fragment
                .nodes[0]
                .label
                .as_ref()
                .unwrap();
            (
                label_glyph_hit_point(node_label, 1),
                label_glyph_anchor(node_label, expected_anchor_index),
            )
        };
        hover(&mut engine, hit_point.x, hit_point.y);

        let anchor = engine
            .state()
            .overlay
            .hover_endpoint
            .as_ref()
            .and_then(|hover| hover.label_anchor.as_ref())
            .expect("label anchor should exist");
        let right_group_point = anchor
            .right_group_point
            .expect("right group anchor should exist");
        assert!(
            right_group_point.distance(expected_anchor) < 0.01,
            "{label}: {right_group_point:?}"
        );
    }
}

#[test]
fn drag_from_label_glyph_uses_focused_glyph_for_vertical_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    load_label_document(
        &mut engine,
        "CuF3",
        vec![
            rect_polygon(294.0, 256.0, 300.0, 264.0),
            rect_polygon(302.0, 256.0, 308.0, 264.0),
            rect_polygon(310.0, 256.0, 316.0, 264.0),
            rect_polygon(318.0, 256.0, 324.0, 264.0),
        ],
        json!([]),
    );

    let (hit_point, clicked_anchor) = {
        let label = engine
            .state()
            .document
            .editable_fragment()
            .unwrap()
            .fragment
            .nodes[0]
            .label
            .as_ref()
            .unwrap();
        (
            label_glyph_hit_point(label, 1),
            label_glyph_anchor(label, 1),
        )
    };
    engine.pointer_down(PointerEvent {
        x: hit_point.x,
        y: hit_point.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: hit_point.x,
        y: hit_point.y - px(40.0),
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: hit_point.x,
        y: hit_point.y - px(40.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    let expected = endpoint_from_anchor_toward(
        clicked_anchor,
        Point::new(hit_point.x, hit_point.y - px(40.0)),
    );
    assert!(
        (last.position[0] - expected.x).abs() < 0.01,
        "{:?}",
        last.position
    );
    assert!(
        (last.position[1] - expected.y).abs() < 0.01,
        "{:?}",
        last.position
    );
}

#[test]
fn drag_from_connected_label_uses_rightmost_group_uppercase_anchor() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    load_label_document(
        &mut engine,
        "CuF3",
        vec![
            rect_polygon(294.0, 256.0, 300.0, 264.0),
            rect_polygon(302.0, 256.0, 308.0, 264.0),
            rect_polygon(310.0, 256.0, 316.0, 264.0),
            rect_polygon(318.0, 256.0, 324.0, 264.0),
        ],
        json!([{
            "id": "b0",
            "begin": "n0",
            "end": "n1",
            "order": 1,
            "strokeWidth": DEFAULT_BOND_STROKE
        }]),
    );

    let (hit_point, right_group_anchor) = {
        let label = engine
            .state()
            .document
            .editable_fragment()
            .unwrap()
            .fragment
            .nodes[0]
            .label
            .as_ref()
            .unwrap();
        (
            label_glyph_hit_point(label, 1),
            label_glyph_anchor(label, 2),
        )
    };
    engine.pointer_down(PointerEvent {
        x: hit_point.x,
        y: hit_point.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: hit_point.x + px(55.0),
        y: hit_point.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: hit_point.x + px(55.0),
        y: hit_point.y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    let expected = endpoint_from_anchor(right_group_anchor, 0.0);
    assert!(
        Point::new(last.position[0], last.position[1]).distance(expected) < 0.01,
        "{:?}",
        last.position
    );
}

#[test]
fn drag_from_middle_label_glyph_uses_leftmost_anchor_for_leftward_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    load_label_document(
        &mut engine,
        "CuF3",
        vec![
            rect_polygon(294.0, 256.0, 300.0, 264.0),
            rect_polygon(302.0, 256.0, 308.0, 264.0),
            rect_polygon(310.0, 256.0, 316.0, 264.0),
            rect_polygon(318.0, 256.0, 324.0, 264.0),
        ],
        json!([]),
    );

    let (hit_point, left_anchor) = {
        let label = engine
            .state()
            .document
            .editable_fragment()
            .unwrap()
            .fragment
            .nodes[0]
            .label
            .as_ref()
            .unwrap();
        (
            label_glyph_hit_point(label, 1),
            label_glyph_anchor(label, 0),
        )
    };
    engine.pointer_down(PointerEvent {
        x: hit_point.x,
        y: hit_point.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: hit_point.x - px(80.0),
        y: hit_point.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: hit_point.x - px(80.0),
        y: hit_point.y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    let expected = endpoint_from_anchor(left_anchor, 180.0);
    assert!(
        (last.position[0] - expected.x).abs() < 0.01,
        "{:?}",
        last.position
    );
    assert!(
        (last.position[1] - expected.y).abs() < 0.01,
        "{:?}",
        last.position
    );
}

#[test]
fn drag_from_rightmost_label_glyph_keeps_clicked_glyph_for_rightward_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    load_label_document(
        &mut engine,
        "CuF3",
        vec![
            rect_polygon(294.0, 256.0, 300.0, 264.0),
            rect_polygon(302.0, 256.0, 308.0, 264.0),
            rect_polygon(310.0, 256.0, 316.0, 264.0),
            rect_polygon(318.0, 256.0, 324.0, 264.0),
        ],
        json!([]),
    );

    let (hit_point, clicked_anchor) = {
        let label = engine
            .state()
            .document
            .editable_fragment()
            .unwrap()
            .fragment
            .nodes[0]
            .label
            .as_ref()
            .unwrap();
        (
            label_glyph_hit_point(label, 3),
            label_glyph_anchor(label, 3),
        )
    };
    let target = Point::new(hit_point.x + px(39.0), hit_point.y);
    engine.pointer_down(PointerEvent {
        x: hit_point.x,
        y: hit_point.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: target.x,
        y: target.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: target.x,
        y: target.y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    let expected = endpoint_from_anchor(clicked_anchor, 0.0);
    assert!(
        (last.position[0] - expected.x).abs() < 0.01,
        "{:?}",
        last.position
    );
    assert!(
        (last.position[1] - expected.y).abs() < 0.01,
        "{:?}",
        last.position
    );
}

#[test]
fn click_on_single_bond_endpoint_extends_at_120_degrees() {
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
    engine.pointer_up(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 3);
    let point = entry.fragment.nodes[2].position;
    assert!(
        (point[0] - FIRST_END_SINGLE_EXTEND_X).abs() < 0.01,
        "{point:?}"
    );
    assert!(
        (point[1] - FIRST_END_SINGLE_EXTEND_Y).abs() < 0.01,
        "{point:?}"
    );
}

#[test]
fn click_draw_clears_hover_after_commit() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });

    assert!(engine.state().overlay.hover_endpoint.is_none());
    assert!(engine.state().overlay.hover_bond_center.is_none());
    assert!(engine.state().overlay.preview.is_none());
}

#[test]
fn dragged_bond_clears_hover_after_commit() {
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
        x: FIRST_END_SINGLE_EXTEND_X,
        y: FIRST_END_SINGLE_EXTEND_Y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: FIRST_END_SINGLE_EXTEND_X,
        y: FIRST_END_SINGLE_EXTEND_Y,
        button: Some(0),
        alt_key: false,
    });

    assert!(engine.state().overlay.hover_endpoint.is_none());
    assert!(engine.state().overlay.hover_bond_center.is_none());
    assert!(engine.state().overlay.preview.is_none());
}

#[test]
fn click_on_single_bond_endpoint_prefers_rightward_120_degree_branch_for_single_bond_component() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    click(&mut engine, px(300.0), px(260.0));

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 3);
    let point = entry.fragment.nodes[2].position;
    assert!((point[0] - ROOT_SINGLE_BRANCH_X).abs() < 0.01, "{point:?}");
    assert!((point[1] - ROOT_SINGLE_BRANCH_Y).abs() < 0.01, "{point:?}");
}
