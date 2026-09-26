use chemsema_engine::{
    document_to_cdxml, parse_cdxml_document, parse_document_json, ChemSemaDocument,
};
use serde_json::json;

const PNG_HEX: &str = "89504E470D0A1A0A0000000D49484452000000010000000108060000001F15C4890000000D4944415408D763F8FFFF3F030008FC02FEA7A6A00000000049454E44AE426082";

fn assert_group_contains(document: &ChemSemaDocument, expected: &[&str]) {
    assert_eq!(
        document.objects.len(),
        1,
        "no grouped object may escape to the page"
    );
    let group = &document.objects[0];
    assert_eq!(group.object_type, "group");
    let mut actual: Vec<_> = group
        .children
        .iter()
        .map(|child| child.object_type.as_str())
        .collect();
    actual.sort_unstable();
    let mut expected = expected.to_vec();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}

#[test]
fn nested_text_run_colors_and_fonts_survive_cdxml_export() {
    let source = r#"<CDXML><page id="1"><group id="2"><t id="3" p="30 30">
      <s font="3" size="10">AD</s></t></group></page></CDXML>"#;
    let mut document = parse_cdxml_document(source, None).unwrap();
    let text = &mut document.objects[0].children[0];
    text.payload.extra.insert(
        "runs".to_string(),
        json!([
            {"text": "A", "fontFamily": "Arial", "fontSize": 10, "fill": "#ff8000"},
            {"text": "D", "fontFamily": "Symbol", "fontSize": 10, "fill": "#ff8000"}
        ]),
    );

    let exported = document_to_cdxml(&document);
    let reopened = parse_cdxml_document(&exported, None).unwrap();
    let runs = &reopened.objects[0].children[0].payload.extra["runs"];
    assert_eq!(runs[0]["fill"], "#ff8000");
    assert_eq!(runs[1]["fill"], "#ff8000");
    assert_eq!(runs[1]["fontFamily"], "Symbol");
}

#[test]
fn imported_auto_line_height_does_not_gain_caption_line_height() {
    let source = r#"<CDXML><page id="1"><t id="2" p="30 30" LineHeight="auto">
      <s font="3" size="10">caption</s></t></page></CDXML>"#;
    let document = parse_cdxml_document(source, None).unwrap();
    let exported = document_to_cdxml(&document);
    let reopened = parse_cdxml_document(&exported, None).unwrap();
    assert!(reopened.objects[0].meta["import"]["cdxml"]["captionLineHeight"].is_null());
}

#[test]
fn first_bond_width_does_not_replace_document_default() {
    let source = r#"<CDXML LineWidth="0.6"><page id="1"><fragment id="2">
      <n id="3" p="20 20"/><n id="4" p="40 20"/>
      <b id="5" B="3" E="4" LineWidth="1.13"/>
    </fragment></page></CDXML>"#;
    let document = parse_cdxml_document(source, None).unwrap();
    let exported = document_to_cdxml(&document);
    let root = exported
        .lines()
        .find(|line| line.starts_with("<CDXML "))
        .unwrap();
    assert!(root.contains(" LineWidth=\"0.6\""), "{root}");
    assert!(exported.contains("LineWidth=\"1.13\""));
}

#[test]
fn grouped_image_and_fragment_graphics_survive_native_and_cdxml_round_trips() {
    let source = format!(
        r#"<CDXML><page id="1"><group id="2">
      <embeddedobject id="3" BoundingBox="20 20 40 40" PNG="{PNG_HEX}"/>
      <fragment id="4"><graphic id="5" GraphicType="Orbital" OrbitalType="lobe" BoundingBox="60 20 80 40"/></fragment>
      <fragment id="6"><n id="7" p="100 20"/><n id="8" p="120 20"/><b id="9" B="7" E="8"/>
        <t id="10" p="100 50"><s font="3" size="10">caption</s></t>
      </fragment>
    </group></page></CDXML>"#
    );
    let document = parse_cdxml_document(&source, None).unwrap();
    let expected = ["image", "shape", "molecule", "text"];
    assert_group_contains(&document, &expected);
    let saved = serde_json::to_string(&document).unwrap();
    let reopened = parse_document_json(&saved).unwrap();
    assert_group_contains(&reopened, &expected);
    let exported = document_to_cdxml(&reopened);
    assert_group_contains(&parse_cdxml_document(&exported, None).unwrap(), &expected);
}

#[test]
fn standalone_image_inside_nested_groups_keeps_both_groups() {
    let source = format!(
        r#"<CDXML><page id="1"><group id="2"><group id="3">
      <embeddedobject id="4" BoundingBox="20 20 40 40" PNG="{PNG_HEX}"/>
    </group></group></page></CDXML>"#
    );
    let document = parse_cdxml_document(&source, None).unwrap();
    assert_group_contains(&document, &["group"]);
    let inner = &document.objects[0].children[0];
    assert_eq!(inner.children.len(), 1);
    assert_eq!(inner.children[0].object_type, "image");
}

#[test]
fn grouping_existing_special_object_fixtures_captures_every_page_object() {
    for source in [
        include_str!("fixtures/cdxml/rest.cdxml"),
        include_str!("fixtures/cdxml/gel-electrophoresis.cdxml"),
        include_str!("fixtures/cdxml/plasmid-map.cdxml"),
    ] {
        let original = parse_cdxml_document(source, None).unwrap();
        let page_start = source.find("<page").unwrap();
        let content_start = page_start + source[page_start..].find('>').unwrap() + 1;
        let content_end = source.rfind("</page>").unwrap();
        let grouped = format!(
            "{}<group id=\"900000001\">{}</group>{}",
            &source[..content_start],
            &source[content_start..content_end],
            &source[content_end..]
        );
        let document = parse_cdxml_document(&grouped, None).unwrap();
        let expected: Vec<_> = original
            .objects
            .iter()
            .map(|obj| obj.object_type.as_str())
            .collect();
        assert_group_contains(&document, &expected);
        assert_eq!(document.resources.len(), original.resources.len());
    }
}

#[test]
fn grouped_electron_symbols_keep_atom_links_and_charge_on_repeated_reload() {
    for (symbol, expected_charge) in [
        ("Plus", 1),
        ("Minus", -1),
        ("RadicalCation", 1),
        ("RadicalAnion", -1),
    ] {
        for depth in 0..=2 {
            let mut body = format!(
                r#"<fragment id="2"><n id="3" p="30 30" Element="7"/>
              <graphic id="4" GraphicType="Symbol" SymbolType="{symbol}" BoundingBox="32 24 38 24"><represent object="3" attribute="Charge"/></graphic>
            </fragment>"#
            );
            for level in 0..depth {
                body = format!("<group id=\"{}\">{body}</group>", 100 + level);
            }
            let source = format!("<CDXML><page id=\"1\">{body}</page></CDXML>");
            let mut engine = chemsema_engine::Engine::new();
            engine.load_cdxml_document(&source).unwrap();
            for _ in 0..3 {
                let saved = engine.document_json().unwrap();
                let document = parse_document_json(&saved).unwrap();
                let node = document.editable_fragments()[0]
                    .fragment
                    .nodes
                    .iter()
                    .find(|node| node.id == "3")
                    .unwrap();
                assert_eq!(
                    node.charge, expected_charge,
                    "{symbol} at group depth {depth}"
                );
                assert_eq!(
                    chemsema_engine::node_radical_count(node),
                    i32::from(symbol.starts_with("Radical"))
                );
                let symbol_object = document
                    .scene_objects()
                    .into_iter()
                    .find(|object| object.object_type == "symbol")
                    .unwrap();
                assert_eq!(symbol_object.payload.extra["attachedAtomId"], "3");
                assert!(document.links.iter().any(|link| link.kind == "atom-symbol"
                    && link
                        .endpoints
                        .iter()
                        .any(|endpoint| endpoint.entity_id == symbol_object.id)
                    && link
                        .endpoints
                        .iter()
                        .any(|endpoint| endpoint.entity_id == "3")));
                engine.load_document_json(&saved).unwrap();
            }
        }
    }
}

#[test]
fn compound_symbol_representation_does_not_count_existing_atom_attributes_twice() {
    for (symbol, charge) in [("RadicalCation", 1), ("RadicalAnion", -1)] {
        for attribute in ["Charge", "Radical"] {
            for grouped in [false, true] {
                let body = format!(
                    r#"<fragment id="2"><n id="3" p="30 30" Element="7" Charge="{charge}" Radical="Doublet"/>
                  <graphic id="4" GraphicType="Symbol" SymbolType="{symbol}" BoundingBox="32 24 38 24"><represent object="3" attribute="{attribute}"/></graphic>
                </fragment>"#
                );
                let body = if grouped {
                    format!("<group id=\"5\">{body}</group>")
                } else {
                    body
                };
                let mut engine = chemsema_engine::Engine::new();
                engine
                    .load_cdxml_document(&format!("<CDXML><page id=\"1\">{body}</page></CDXML>"))
                    .unwrap();
                for _ in 0..3 {
                    let saved = engine.document_json().unwrap();
                    let document = parse_document_json(&saved).unwrap();
                    let node = &document.editable_fragments()[0].fragment.nodes[0];
                    assert_eq!(node.charge, charge, "{symbol} via {attribute}");
                    assert_eq!(
                        chemsema_engine::node_radical_count(node),
                        1,
                        "{symbol} via {attribute}"
                    );
                    assert_eq!(
                        chemsema_engine::document_to_svg(&document)
                            .matches('•')
                            .count(),
                        0,
                        "the explicit symbol supplies the dot; do not add an atom annotation"
                    );
                    engine.load_document_json(&saved).unwrap();
                }
            }
        }
    }
}

#[test]
fn represented_electron_suppresses_only_the_redundant_radical_annotation() {
    for (symbol, expected_annotations) in [("Electron", 0), ("LonePair", 1)] {
        let source = format!(
            r#"<CDXML><page id="1"><group id="5"><fragment id="2">
          <n id="3" p="30 30" Element="7" Radical="Doublet"/>
          <graphic id="4" GraphicType="Symbol" SymbolType="{symbol}" BoundingBox="32 24 38 24"><represent object="3" attribute="Radical"/></graphic>
        </fragment></group></page></CDXML>"#
        );
        let mut engine = chemsema_engine::Engine::new();
        engine.load_cdxml_document(&source).unwrap();
        let document = parse_document_json(&engine.document_json().unwrap()).unwrap();
        assert_eq!(
            chemsema_engine::node_radical_count(
                &document.editable_fragments()[0].fragment.nodes[0]
            ),
            1
        );
        assert_eq!(
            chemsema_engine::document_to_svg(&document)
                .matches('•')
                .count(),
            expected_annotations,
            "a lone pair is not an unpaired-electron symbol"
        );
    }
}
