use chemsema_engine::{
    cdx_to_cdxml, document_to_cdx, document_to_svg, parse_cdxml_document, Engine, Point,
    PointerEvent, RenderBoundsScope, RenderPrimitive, RenderRole, ResourceData, Tool,
};
use serde_json::{json, Value};

const PNG_HEX: &str = "89504E470D0A1A0A0000000D49484452000000010000000108060000001F15C4890000000D4944415408D763F8FFFF3F030008FC02FEA7A6A00000000049454E44AE426082";
const PNG_BASE64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQI12P4//8/AwAI/AL+p6agAAAAAElFTkSuQmCC";

fn execute(engine: &mut Engine, command: Value) -> Value {
    serde_json::from_str(
        &engine
            .execute_command_json(&command.to_string())
            .expect("command executes"),
    )
    .expect("command result is JSON")
}

fn add_test_image(engine: &mut Engine) {
    let result = execute(
        engine,
        json!({
            "type": "add-image",
            "mimeType": "image/png",
            "dataBase64": PNG_BASE64,
            "pixelWidth": 1,
            "pixelHeight": 1,
            "position": { "x": 100.0, "y": 120.0 },
            "width": 80.0,
            "height": 60.0,
            "sourceName": "pixel.png"
        }),
    );
    assert_eq!(result["changed"], true);
}

#[test]
fn native_image_supports_selection_resize_rotation_copy_paste_and_undo() {
    let mut engine = Engine::new();
    add_test_image(&mut engine);

    assert!(engine
        .render_list()
        .iter()
        .any(|primitive| matches!(primitive, RenderPrimitive::Image { .. })));
    let saved = engine.document_json().expect("image document serializes");
    let mut reloaded = Engine::new();
    reloaded
        .load_document_json(&saved)
        .expect("image document reloads");
    assert!(reloaded
        .render_list()
        .iter()
        .any(|primitive| matches!(primitive, RenderPrimitive::Image { .. })));
    let before = engine
        .render_bounds(RenderBoundsScope::Selection)
        .expect("image selection has bounds");
    assert!(engine.begin_selection_resize("se", Point::new(before[2], before[3])));
    assert!(engine.finish_selection_resize(Point::new(before[2] + 40.0, before[3] + 20.0)));
    let resized = engine
        .render_bounds(RenderBoundsScope::Selection)
        .expect("resized image has bounds");
    assert!(resized[2] - resized[0] > before[2] - before[0]);
    assert!(resized[3] - resized[1] > before[3] - before[1]);

    let center = Point::new(
        (resized[0] + resized[2]) * 0.5,
        (resized[1] + resized[3]) * 0.5,
    );
    let start = Point::new(center.x, resized[1] - 12.0);
    assert!(engine.begin_selection_rotate(start));
    assert!(engine.finish_selection_rotate(Point::new(resized[2] + 12.0, center.y), true));
    let image = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "image")
        .expect("image object remains");
    assert!(image.transform.rotate.abs() > 1.0);

    assert!(engine.copy_selection());
    assert!(engine.paste_clipboard());
    assert_eq!(
        engine
            .state()
            .document
            .objects
            .iter()
            .filter(|object| object.object_type == "image")
            .count(),
        2
    );
    assert_eq!(
        engine
            .state()
            .document
            .resources
            .values()
            .filter(|resource| resource.resource_type == "image")
            .count(),
        2
    );
    assert!(engine.undo());
    assert_eq!(
        engine
            .state()
            .document
            .objects
            .iter()
            .filter(|object| object.object_type == "image")
            .count(),
        1
    );
}

#[test]
fn image_survives_cross_tab_document_paste_with_its_resource() {
    let mut source = Engine::new();
    add_test_image(&mut source);
    let source_json = source.document_json().expect("image document serializes");

    let mut target = Engine::new();
    assert!(target
        .paste_document_json(&source_json)
        .expect("image document should paste"));
    assert_eq!(
        target
            .state()
            .document
            .objects
            .iter()
            .filter(|object| object.object_type == "image")
            .count(),
        1
    );
    assert_eq!(
        target
            .state()
            .document
            .resources
            .values()
            .filter(|resource| resource.resource_type == "image")
            .count(),
        1
    );
}

#[test]
fn image_focus_click_and_transient_drag_follow_select_tool_rules() {
    let mut engine = Engine::new();
    add_test_image(&mut engine);
    assert_eq!(engine.state().tool.active_tool, Tool::Select);
    assert_eq!(engine.state().selection.arrow_objects.len(), 1);

    assert!(engine.clear_selection());
    let center = Point::new(100.0, 120.0);
    engine.pointer_move(PointerEvent {
        x: center.x,
        y: center.y,
        button: None,
        alt_key: false,
    });
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::HoverObjectBox,
            x,
            y,
            width,
            height,
            ..
        } if (*x, *y, *width, *height) == (60.0, 90.0, 80.0, 60.0)
    )));

    assert!(engine.begin_selection_move_at_point(center, false, false));
    assert!(!engine.finish_selection_move(center, false));
    assert_eq!(engine.state().selection.arrow_objects.len(), 1);

    assert!(engine.clear_selection());
    assert!(engine.begin_selection_move_at_point(center, false, false));
    assert!(engine.update_selection_move(Point::new(120.0, 135.0), true));
    assert!(engine.finish_selection_move(Point::new(120.0, 135.0), true));
    assert!(engine.state().selection.is_empty());
    let moved = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "image")
        .expect("moved image remains");
    assert_eq!(moved.transform.translate, [80.0, 105.0]);
}

#[test]
fn cdxml_and_cdx_preserve_embedded_png_bytes_geometry_and_rotation() {
    let source = format!(
        r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML CreationProgram="ChemDraw 23.1.1.3" Name="image-test">
  <page id="1" BoundingBox="0 0 300 300">
    <embeddedobject id="2" BoundingBox="40 50 140 110" RotationAngle="22.5" Z="3" PNG="{PNG_HEX}"></embeddedobject>
  </page>
</CDXML>"#
    );
    let document = parse_cdxml_document(&source, Some("image-test")).expect("CDXML imports");
    let object = document
        .objects
        .iter()
        .find(|object| object.object_type == "image")
        .expect("embeddedobject becomes an image");
    assert_eq!(object.payload.bbox, Some([0.0, 0.0, 100.0, 60.0]));
    assert_eq!(object.transform.rotate, 22.5);
    let resource = document
        .resources
        .get(object.payload.resource_ref.as_ref().unwrap())
        .expect("image resource exists");
    assert!(matches!(resource.data, ResourceData::Json(_)));
    assert_eq!(resource.data.as_image().unwrap().data_base64, PNG_BASE64);

    let svg = document_to_svg(&document);
    assert!(svg.contains("<image"));
    assert!(svg.contains("data:image/png;base64,"));
    assert!(svg.contains("rotate(22.5"));

    let cdx = document_to_cdx(&document).expect("CDX exports");
    let roundtrip_cdxml = cdx_to_cdxml(&cdx).expect("CDX converts back to CDXML");
    assert!(roundtrip_cdxml.contains(PNG_HEX));
    assert!(roundtrip_cdxml.contains("RotationAngle=\"22.5\""));
}

#[test]
fn opaque_embedded_payload_is_preserved_and_never_renders_as_silent_blank() {
    let source = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML><page id="1" BoundingBox="0 0 300 300">
  <embeddedobject id="2" BoundingBox="10 20 110 80" Z="2" EnhancedMetafile="01020304"></embeddedobject>
</page></CDXML>"#;
    let document = parse_cdxml_document(source, Some("opaque")).expect("CDXML imports");
    let resource = document
        .resources
        .values()
        .find(|resource| resource.resource_type == "embedded-object")
        .expect("opaque resource is retained");
    assert!(matches!(resource.data, ResourceData::Json(_)));
    let svg = document_to_svg(&document);
    assert!(svg.contains("Embedded EnhancedMetafile"));
    let exported = chemsema_engine::document_to_cdxml(&document);
    assert!(exported.contains("EnhancedMetafile=\"01020304\""));
}

#[test]
fn image_command_rejects_mismatched_metadata_and_payload() {
    let mut engine = Engine::new();
    let result = execute(
        &mut engine,
        json!({
            "type": "add-image",
            "mimeType": "image/png",
            "dataBase64": PNG_BASE64,
            "pixelWidth": 2,
            "pixelHeight": 1,
            "position": { "x": 100.0, "y": 120.0 },
            "width": 80.0,
            "height": 60.0
        }),
    );
    assert_eq!(result["changed"], false);
    assert!(!engine
        .state()
        .document
        .objects
        .iter()
        .any(|object| object.object_type == "image"));
}
