use super::session::{handle_session_request, session_help_json, session_ready_json};
use super::*;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 0.000_001,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn infers_png_capture_format_from_output_path() {
    assert_eq!(
        infer_capture_format_from_path("capture.png"),
        Some(CaptureFormat::Png)
    );
    assert_eq!(
        infer_capture_format_from_path("capture.SVG"),
        Some(CaptureFormat::Svg)
    );
    assert!(parse_capture_format("jpeg").is_err());
}

#[test]
fn capture_output_defaults_to_temp_png() {
    let (path, format, defaulted) = resolve_capture_output(None, None).unwrap();
    assert_eq!(format, CaptureFormat::Png);
    assert!(defaulted);
    assert!(Path::new(&path).starts_with(default_output_dir()));
    assert_eq!(
        Path::new(&path)
            .extension()
            .and_then(|value| value.to_str()),
        Some("png")
    );
}

#[test]
fn explicit_capture_output_without_extension_requires_format() {
    assert!(resolve_capture_output(Some("capture".to_string()), None).is_err());
    let (path, format, defaulted) =
        resolve_capture_output(Some("capture".to_string()), Some(CaptureFormat::Svg)).unwrap();
    assert_eq!(path, "capture");
    assert_eq!(format, CaptureFormat::Svg);
    assert!(!defaulted);
}

#[test]
fn default_output_warnings_are_machine_readable() {
    let capture_warnings = default_capture_warnings(true, "capture.png");
    assert_eq!(capture_warnings[0]["kind"], "default_output_path");
    assert_eq!(capture_warnings[0]["path"], "capture.png");
    assert!(default_capture_warnings(false, "capture.png").is_empty());

    let payload_warnings = default_payload_warnings(true, Path::new("payload.json"));
    assert_eq!(payload_warnings[0]["kind"], "default_payload_path");
    assert_eq!(payload_warnings[0]["path"], "payload.json");
    assert!(default_payload_warnings(false, Path::new("payload.json")).is_empty());
}

#[test]
fn export_document_for_object_target_compacts_page() {
    let mut engine = Engine::new();
    let first: Value = serde_json::from_str(
        &engine
            .execute_command_json(
                &json!({
                    "type": "add-text",
                    "position": { "x": 100.0, "y": 120.0 },
                    "text": "A",
                    "box": [0.0, 0.0, 10.0, 10.0]
                })
                .to_string(),
            )
            .expect("add first text"),
    )
    .expect("first result");
    let first_id = first["created"]["objects"][0]
        .as_str()
        .expect("first id")
        .to_string();

    engine
        .execute_command_json(
            &json!({
                "type": "add-text",
                "position": { "x": 300.0, "y": 120.0 },
                "text": "B",
                "box": [0.0, 0.0, 10.0, 10.0]
            })
            .to_string(),
        )
        .expect("add second text");

    let document = engine_document(&engine).expect("document");
    let target = TargetSelector::Object(first_id.clone());
    let bounds = target_bounds(&document, &target).expect("target bounds");
    let exported = export_document_for_target(&document, &target).expect("export object");

    assert_eq!(exported.objects.len(), 1);
    assert_eq!(exported.objects[0].id, first_id);
    assert_close(exported.objects[0].transform.translate[0], 20.0);
    assert_close(exported.objects[0].transform.translate[1], 20.0);
    assert_close(exported.document.page.width, bounds[2] - bounds[0] + 40.0);
    assert_close(exported.document.page.height, bounds[3] - bounds[1] + 40.0);
    assert_eq!(
        exported.document.meta["export"]["selectionTarget"]["kind"],
        "object"
    );
}

#[test]
fn export_document_for_multi_object_selection_keeps_only_selected_objects() {
    let mut engine = Engine::new();
    let first: Value = serde_json::from_str(
        &engine
            .execute_command_json(
                &json!({
                    "type": "add-text",
                    "position": { "x": 100.0, "y": 120.0 },
                    "text": "A",
                    "box": [0.0, 0.0, 10.0, 10.0]
                })
                .to_string(),
            )
            .expect("add first text"),
    )
    .expect("first result");
    let second: Value = serde_json::from_str(
        &engine
            .execute_command_json(
                &json!({
                    "type": "add-text",
                    "position": { "x": 300.0, "y": 120.0 },
                    "text": "B",
                    "box": [0.0, 0.0, 10.0, 10.0]
                })
                .to_string(),
            )
            .expect("add second text"),
    )
    .expect("second result");
    engine
        .execute_command_json(
            &json!({
                "type": "add-text",
                "position": { "x": 500.0, "y": 120.0 },
                "text": "C",
                "box": [0.0, 0.0, 10.0, 10.0]
            })
            .to_string(),
        )
        .expect("add third text");
    let first_id = first["created"]["objects"][0]
        .as_str()
        .expect("first id")
        .to_string();
    let second_id = second["created"]["objects"][0]
        .as_str()
        .expect("second id")
        .to_string();

    let document = engine_document(&engine).expect("document");
    let target = TargetSelector::Selection(vec![
        TargetSelector::Object(first_id.clone()),
        TargetSelector::Object(second_id.clone()),
    ]);
    let bounds = target_bounds(&document, &target).expect("target bounds");
    let exported = export_document_for_target(&document, &target).expect("export selection");

    let ids = exported
        .objects
        .iter()
        .map(|object| object.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![first_id.as_str(), second_id.as_str()]);
    assert_close(exported.objects[0].transform.translate[0], 20.0);
    assert_close(exported.objects[1].transform.translate[0], 220.0);
    assert_close(exported.document.page.width, bounds[2] - bounds[0] + 40.0);
    assert_close(exported.document.page.height, bounds[3] - bounds[1] + 40.0);
    assert_eq!(
        exported.document.meta["export"]["selectionTarget"]["targetCount"],
        2
    );
}

#[test]
fn expands_view_box_with_absolute_and_relative_sides() {
    let view_box = expanded_view_box(
        [10.0, 20.0, 30.0, 60.0],
        CropExpansion {
            abs_left: 1.0,
            abs_top: 2.0,
            abs_right: 3.0,
            abs_bottom: 4.0,
            rel_left: 0.1,
            rel_top: 0.25,
            rel_right: 0.2,
            rel_bottom: 0.0,
        },
    );
    assert_close(view_box[0], 7.0);
    assert_close(view_box[1], 8.0);
    assert_close(view_box[2], 30.0);
    assert_close(view_box[3], 56.0);
}

#[test]
fn derives_png_height_from_fixed_width() {
    let pixel_size = pixel_size_for_view_box(
        [0.0, 0.0, 100.0, 50.0],
        RasterOptions {
            scale: 10.0,
            width: Some(1000),
            height: None,
        },
    )
    .unwrap();
    assert_eq!(pixel_size.width, 1000);
    assert_eq!(pixel_size.height, 500);
    assert_close(pixel_size.scale_x, 10.0);
    assert_close(pixel_size.scale_y, 10.0);
}

#[test]
fn png_capture_renders_svg_text() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 30"><text x="5" y="22" font-size="22" font-family="sans-serif" fill="#000000">CN</text></svg>"##;
    let pixmap = render_svg_png_pixmap(
        svg,
        [0.0, 0.0, 80.0, 30.0],
        PixelSize {
            width: 800,
            height: 300,
            scale_x: 10.0,
            scale_y: 10.0,
        },
    )
    .unwrap();
    let dark_pixels = pixmap
        .data()
        .chunks_exact(4)
        .filter(|pixel| pixel[0] < 240 || pixel[1] < 240 || pixel[2] < 240)
        .count();
    assert!(
        dark_pixels > 500,
        "text-only SVG should produce visible non-white PNG pixels, got {dark_pixels}"
    );
}

#[test]
fn detail_report_returns_raw_object_without_expanding_resource_by_default() {
    let document = ChemcoreDocument::blank();
    let report = detail_report(
        "blank.ccjs",
        &document,
        &TargetSelector::Object("obj_editor_molecule".to_string()),
        DetailOptions {
            include_raw: true,
            include_resource: false,
        },
    )
    .unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["detail"]["id"], "obj_editor_molecule");
    assert_eq!(
        report["detail"]["references"]["resource"]["id"],
        "mol_editor"
    );
    assert_eq!(
        report["detail"]["raw"]["object"]["id"],
        "obj_editor_molecule"
    );
    assert!(report["detail"]["raw"].get("resource").is_none());
}

#[test]
fn detail_report_can_suppress_raw_or_include_molecule_fragment() {
    let document = ChemcoreDocument::blank();
    let summary = detail_report(
        "blank.ccjs",
        &document,
        &TargetSelector::Object("obj_editor_molecule".to_string()),
        DetailOptions {
            include_raw: false,
            include_resource: false,
        },
    )
    .unwrap();
    assert!(summary["detail"].get("raw").is_none());

    let molecule = detail_report(
        "blank.ccjs",
        &document,
        &TargetSelector::Molecule(0),
        DetailOptions {
            include_raw: true,
            include_resource: false,
        },
    )
    .unwrap();
    assert_eq!(molecule["detail"]["kind"], "molecule");
    assert_eq!(
        molecule["detail"]["raw"]["fragment"]["schema"],
        "chemcore.molecule.fragment2d"
    );
}

#[test]
fn session_reports_canonical_protocol_id() {
    let ready = session_ready_json(None);
    assert_eq!(ready["protocol"], crate::protocol::SESSION_PROTOCOL_VERSION);

    let help = session_help_json();
    assert_eq!(help["protocol"], crate::protocol::SESSION_PROTOCOL_VERSION);
    assert!(help["operations"]["execute"]["description"]
        .as_str()
        .expect("execute description")
        .contains("select-targets"));
}

#[test]
fn session_execute_selection_commands_drive_arrange() {
    let mut engine = Engine::new();
    let first: Value = serde_json::from_str(
        &engine
            .execute_command_json(
                &json!({
                    "type": "add-text",
                    "position": { "x": 10.0, "y": 10.0 },
                    "text": "A",
                    "box": [0.0, 0.0, 10.0, 10.0]
                })
                .to_string(),
            )
            .expect("add first text"),
    )
    .expect("first result");
    let second: Value = serde_json::from_str(
        &engine
            .execute_command_json(
                &json!({
                    "type": "add-text",
                    "position": { "x": 40.0, "y": 30.0 },
                    "text": "B",
                    "box": [0.0, 0.0, 10.0, 10.0]
                })
                .to_string(),
            )
            .expect("add second text"),
    )
    .expect("second result");
    let first_id = first["created"]["objects"][0].as_str().expect("first id");
    let second_id = second["created"]["objects"][0].as_str().expect("second id");

    let input = std::env::temp_dir().join(format!(
        "chemcore-cli-session-selection-input-{}.ccjs",
        std::process::id()
    ));
    let output = std::env::temp_dir().join(format!(
        "chemcore-cli-session-selection-output-{}.ccjs",
        std::process::id()
    ));
    fs::write(&input, engine.document_json().expect("document json")).expect("write input");

    let mut session = None;
    let (opened, exit) = handle_session_request(
        &mut session,
        json!({
            "id": 1,
            "op": "open",
            "input": input.to_string_lossy()
        }),
    );
    assert!(!exit);
    assert_eq!(opened["ok"], true);

    let (executed, exit) = handle_session_request(
        &mut session,
        json!({
            "id": 2,
            "op": "execute",
            "commands": [
                {
                    "type": "select-targets",
                    "targets": { "objects": [first_id, second_id] }
                },
                {
                    "type": "apply-selection-arrange",
                    "command": "align-left"
                }
            ]
        }),
    );
    assert!(!exit);
    assert_eq!(executed["ok"], true);
    assert_eq!(
        executed["result"]["results"][0]["commandType"],
        "select-targets"
    );
    assert_eq!(
        executed["result"]["results"][0]["result"]["output"]["counts"]["textObjects"],
        2
    );
    assert_eq!(
        executed["result"]["results"][1]["commandType"],
        "apply-selection-arrange"
    );
    assert_eq!(executed["result"]["results"][1]["changed"], true);

    let (saved, exit) = handle_session_request(
        &mut session,
        json!({
            "id": 3,
            "op": "save",
            "out": output.to_string_lossy()
        }),
    );
    assert!(!exit);
    assert_eq!(saved["ok"], true);

    let document: Value =
        serde_json::from_str(&fs::read_to_string(&output).expect("saved document"))
            .expect("saved json");
    let text_x = document["objects"]
        .as_array()
        .expect("objects")
        .iter()
        .filter(|object| object["type"].as_str() == Some("text"))
        .map(|object| object["transform"]["translate"][0].as_f64().expect("x"))
        .collect::<Vec<_>>();
    assert_eq!(text_x, vec![10.0, 10.0]);

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}
