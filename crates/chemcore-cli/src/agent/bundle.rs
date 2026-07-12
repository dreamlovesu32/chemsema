use super::*;
use sha2::Digest;

const BUNDLE_SCHEMA: &str = "chemcore.agent.bundle.v1";
const IDENTITY_MAP_SCHEMA: &str = "chemcore.identity-map.v1";
const PROVENANCE_SCHEMA: &str = "chemcore.agent.provenance.v1";

#[derive(Debug, Clone)]
pub(crate) struct BundleOptions {
    pub(crate) input: String,
    pub(crate) target: TargetSelector,
    pub(crate) out_dir: PathBuf,
    pub(crate) context_radius: f64,
    pub(crate) capture_format: CaptureFormat,
    pub(crate) raster: RasterOptions,
    pub(crate) subset_format: String,
    pub(crate) pretty: bool,
}

pub(crate) fn bundle_command(args: &[String]) -> Result<(), String> {
    let options = parse_bundle_args(args)?;
    let engine = load_engine_from_file(&options.input)?;
    let document = engine_document(&engine)?;
    let manifest = bundle_document(&engine, &document, &options)?;
    write_json_value(manifest, None, options.pretty)
}

pub(crate) fn parse_bundle_args(args: &[String]) -> Result<BundleOptions, String> {
    let mut input = None;
    let mut target = None;
    let mut out_dir = None;
    let mut context_radius = 40.0;
    let mut capture_format = CaptureFormat::Png;
    let mut raster = RasterOptions::default();
    let mut subset_format = "ccjs".to_string();
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--target" | "-t" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    parse_target_selector(
                        args.get(index)
                            .ok_or_else(|| "--target requires a selector.".to_string())?,
                    )?,
                )?;
            }
            "--targets" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    parse_target_selection_arg(args.get(index).ok_or_else(|| {
                        "--targets requires selectors separated by semicolons.".to_string()
                    })?)?,
                )?;
            }
            "--out-dir" | "--output-dir" => {
                index += 1;
                out_dir =
                    Some(PathBuf::from(args.get(index).ok_or_else(|| {
                        "--out-dir requires a directory.".to_string()
                    })?));
            }
            "--context-radius" | "--radius" => {
                index += 1;
                context_radius = parse_non_negative_f64(
                    "--context-radius",
                    args.get(index)
                        .ok_or_else(|| "--context-radius requires a number.".to_string())?,
                )?;
            }
            "--capture-format" => {
                index += 1;
                capture_format = parse_capture_format(
                    args.get(index)
                        .ok_or_else(|| "--capture-format requires png or svg.".to_string())?,
                )?;
            }
            "--capture-scale" | "--scale" => {
                index += 1;
                raster.scale = parse_positive_f64(
                    "--capture-scale",
                    args.get(index)
                        .ok_or_else(|| "--capture-scale requires a positive number.".to_string())?,
                )?;
            }
            "--capture-width" | "--width" => {
                index += 1;
                raster.width = Some(parse_positive_u32(
                    "--capture-width",
                    args.get(index).ok_or_else(|| {
                        "--capture-width requires a positive integer.".to_string()
                    })?,
                )?);
            }
            "--capture-height" | "--height" => {
                index += 1;
                raster.height = Some(parse_positive_u32(
                    "--capture-height",
                    args.get(index).ok_or_else(|| {
                        "--capture-height requires a positive integer.".to_string()
                    })?,
                )?);
            }
            "--subset-format" => {
                index += 1;
                subset_format = parse_subset_format(
                    args.get(index)
                        .ok_or_else(|| "--subset-format requires a format.".to_string())?,
                )?;
            }
            "--pretty" => pretty = true,
            value if input.is_none() => input = Some(value.to_string()),
            value => return Err(format!("Unexpected bundle argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "bundle requires an input file.".to_string())?;
    let target = target.ok_or_else(|| {
        "bundle requires --target <object:id|molecule:index|node:id|bond:id>, repeated --target, or --targets."
            .to_string()
    })?;
    if matches!(target, TargetSelector::All | TargetSelector::Bounds(_)) {
        return Err("bundle targets must be object, molecule, node, bond, or a selection of those selectors.".to_string());
    }
    ensure_bundle_target_is_editable(&target)?;
    let out_dir = out_dir.ok_or_else(|| "bundle requires --out-dir <directory>.".to_string())?;
    Ok(BundleOptions {
        input,
        target,
        out_dir,
        context_radius,
        capture_format,
        raster,
        subset_format,
        pretty,
    })
}

pub(crate) fn bundle_document(
    engine: &Engine,
    document: &ChemcoreDocument,
    options: &BundleOptions,
) -> Result<Value, String> {
    fs::create_dir_all(&options.out_dir).map_err(|error| {
        format!(
            "Failed to create bundle output directory {}: {error}",
            options.out_dir.display()
        )
    })?;
    if !options.out_dir.is_dir() {
        return Err(format!(
            "Bundle output path is not a directory: {}.",
            options.out_dir.display()
        ));
    }

    let target_bounds = target_bounds(document, &options.target)?;
    let expansion = CropExpansion::uniform_abs(options.context_radius);
    let visual_view_box = expanded_view_box(target_bounds, expansion);
    let visual_bounds = view_box_to_bounds(visual_view_box);
    let targets = flattened_targets(&options.target);
    let detail = bundle_target_detail(&options.input, document, &targets)?;
    let context = context_report(
        &options.input,
        document,
        &options.target,
        target_bounds,
        visual_bounds,
        expansion,
        200,
    )?;

    let editable_subset = export_document_for_target(document, &options.target)?;
    let integrity = referential_integrity(&editable_subset);
    if !integrity.all_resources_resolved || !integrity.all_styles_resolved {
        return Err(format!(
            "Bundle editable subset has unresolved references: resources={}, styles={}.",
            integrity.all_resources_resolved, integrity.all_styles_resolved
        ));
    }
    let source_hash = source_file_sha256(&options.input)?;
    let document_hash = crate::document_hash(engine)
        .ok_or_else(|| "Failed to compute source document hash for bundle.".to_string())?;
    let identity_map =
        identity_map_for_documents(document, &editable_subset, &document_hash, &source_hash)?;
    let provenance = provenance_for_bundle(
        engine,
        document,
        &editable_subset,
        options,
        target_bounds,
        visual_bounds,
        visual_view_box,
        &source_hash,
        &document_hash,
        identity_map
            .get("entries")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
    )?;

    let target_path = options.out_dir.join("target.json");
    let context_path = options.out_dir.join("context.json");
    let subset_name = format!("editable-subset.{}", options.subset_format);
    let subset_path = options.out_dir.join(&subset_name);
    let capture_name = format!("capture.{}", options.capture_format.as_str());
    let capture_path = options.out_dir.join(&capture_name);
    let identity_path = options.out_dir.join("identity-map.json");
    let provenance_path = options.out_dir.join("provenance.json");

    write_json_file(&target_path, &detail, options.pretty)?;
    write_json_file(&context_path, &context, options.pretty)?;
    write_subset_document(&editable_subset, &subset_path, &options.subset_format)?;
    let render = capture_render_primitives(document, &options.target, visual_view_box, false)?;
    let capture_output = write_capture_output(
        &render.primitives,
        visual_view_box,
        &capture_path.display().to_string(),
        options.capture_format,
        options.raster,
    )?;
    write_json_file(&identity_path, &identity_map, options.pretty)?;
    write_json_file(&provenance_path, &provenance, options.pretty)?;

    let artifacts = vec![
        artifact_status("detail", "target.json", &target_path, "json")?,
        artifact_status("context", "context.json", &context_path, "json")?,
        artifact_status(
            "editableSubset",
            &subset_name,
            &subset_path,
            &options.subset_format,
        )?,
        artifact_status(
            "capture",
            &capture_name,
            &capture_path,
            options.capture_format.as_str(),
        )?,
        artifact_status("identityMap", "identity-map.json", &identity_path, "json")?,
        artifact_status("provenance", "provenance.json", &provenance_path, "json")?,
    ];
    let manifest = json!({
        "schema": BUNDLE_SCHEMA,
        "ok": true,
        "source": {
            "path": privacy_preserving_source_path(&options.input),
            "fileName": Path::new(&options.input).file_name().and_then(|value| value.to_str()),
            "format": infer_format_from_path(&options.input),
            "sha256": source_hash,
            "documentHash": document_hash,
            "documentRevision": engine.revision(),
        },
        "target": {
            "selectors": targets.iter().map(TargetSelector::selector).collect::<Vec<_>>(),
            "selector": options.target.selector(),
            "bounds": bounds_json(target_bounds),
        },
        "editableScope": editable_scope_json(&editable_subset),
        "visualScope": {
            "bounds": bounds_json(visual_bounds),
            "viewBox": view_box_json(visual_view_box),
            "contextRadius": options.context_radius,
            "captureIncludesVisibleNonTargets": true,
            "editableOnly": false,
        },
        "artifacts": {
            "detail": "target.json",
            "context": "context.json",
            "editableSubset": subset_name,
            "capture": capture_name,
            "identityMap": "identity-map.json",
            "provenance": "provenance.json",
        },
        "artifactVerification": artifacts,
        "integrity": {
            "allResourcesResolved": integrity.all_resources_resolved,
            "allStylesResolved": integrity.all_styles_resolved,
            "captureVerified": capture_output.bytes > 0,
            "editableSubsetValid": true,
        },
        "capture": {
            "format": options.capture_format.as_str(),
            "bytes": capture_output.bytes,
            "pixelSize": capture_output.pixel_size.map(PixelSize::to_json),
            "render": {
                "mode": render.mode,
                "primitiveCount": render.primitives.len(),
                "targets": render.targets.to_json(),
            }
        },
        "provenance": {
            "schema": PROVENANCE_SCHEMA,
            "artifact": "provenance.json",
            "derivedFrom": {
                "documentHash": document_hash,
                "sourceFileSha256": source_hash,
                "selector": options.target.selector(),
                "selectors": targets.iter().map(TargetSelector::selector).collect::<Vec<_>>(),
                "sourceBounds": bounds_json(target_bounds),
                "visualBounds": bounds_json(visual_bounds),
                "translation": subset_translation_from_export_meta(&editable_subset),
            }
        }
    });
    let manifest_path = options.out_dir.join("manifest.json");
    write_json_file(&manifest_path, &manifest, options.pretty)?;
    let mut final_manifest = manifest;
    set_object_field(
        &mut final_manifest,
        "manifest",
        artifact_status("manifest", "manifest.json", &manifest_path, "json")?,
    );
    write_json_file(&manifest_path, &final_manifest, options.pretty)?;
    Ok(final_manifest)
}

pub(crate) fn parse_subset_format(value: &str) -> Result<String, String> {
    let normalized = value.trim().trim_start_matches('.').to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "ccjs" | "ccjz" | "cdxml" | "cdx" | "sdf"
    ) {
        Ok(normalized)
    } else {
        Err(format!(
            "Unsupported bundle --subset-format '{value}'. Expected ccjs, ccjz, cdxml, cdx, or sdf."
        ))
    }
}

pub(crate) fn ensure_bundle_target_is_editable(target: &TargetSelector) -> Result<(), String> {
    match target {
        TargetSelector::Object(_)
        | TargetSelector::Molecule(_)
        | TargetSelector::Node(_)
        | TargetSelector::Bond(_) => Ok(()),
        TargetSelector::Selection(targets) => {
            for target in targets {
                ensure_bundle_target_is_editable(target)?;
            }
            Ok(())
        }
        TargetSelector::All | TargetSelector::Bounds(_) => Err(
            "bundle does not accept all or bounds targets for editable subset export.".to_string(),
        ),
    }
}

fn flattened_targets(target: &TargetSelector) -> Vec<TargetSelector> {
    let mut targets = Vec::new();
    collect_flattened_targets(target, &mut targets);
    targets
}

fn collect_flattened_targets(target: &TargetSelector, out: &mut Vec<TargetSelector>) {
    match target {
        TargetSelector::Selection(targets) => {
            for target in targets {
                collect_flattened_targets(target, out);
            }
        }
        target => out.push(target.clone()),
    }
}

fn bundle_target_detail(
    input: &str,
    document: &ChemcoreDocument,
    targets: &[TargetSelector],
) -> Result<Value, String> {
    let options = DetailOptions {
        include_raw: true,
        include_resource: true,
    };
    let mut details = Vec::new();
    for target in targets {
        let detail = detail_report(input, document, target, options)?;
        details.push(detail.get("detail").cloned().unwrap_or(Value::Null));
    }
    Ok(json!({
        "ok": true,
        "schema": BUNDLE_SCHEMA,
        "input": input,
        "targetCount": details.len(),
        "targets": details,
    }))
}

fn write_json_file(path: &Path, value: &Value, pretty: bool) -> Result<u64, String> {
    ensure_output_parent_path(path)?;
    let text = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
    .map_err(|error| error.to_string())?;
    fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("Failed to write JSON {}: {error}", path.display()))?;
    verify_file_written(path, 1, "bundle JSON")
}

fn write_subset_document(
    document: &ChemcoreDocument,
    path: &Path,
    format: &str,
) -> Result<(), String> {
    let json = serde_json::to_string(document).map_err(|error| error.to_string())?;
    let mut engine = Engine::new();
    engine.load_document_json(&json)?;
    write_engine_output(&engine, &path.display().to_string(), Some(format))
}

fn artifact_status(
    key: &str,
    relative_path: &str,
    path: &Path,
    format: &str,
) -> Result<Value, String> {
    let bytes = verify_file_written(path, 1, key)?;
    Ok(json!({
        "key": key,
        "path": relative_path,
        "format": format,
        "verified": true,
        "bytes": bytes,
        "sha256": file_sha256(path)?,
    }))
}

#[derive(Debug)]
struct ReferentialIntegrity {
    all_resources_resolved: bool,
    all_styles_resolved: bool,
}

fn referential_integrity(document: &ChemcoreDocument) -> ReferentialIntegrity {
    let mut all_resources_resolved = true;
    let mut all_styles_resolved = true;
    for object in document.scene_objects() {
        if let Some(resource_ref) = object.payload.resource_ref.as_ref() {
            all_resources_resolved &= document.resources.contains_key(resource_ref);
        }
        if let Some(style_ref) = object.style_ref.as_ref() {
            all_styles_resolved &= document.styles.contains_key(style_ref);
        }
    }
    ReferentialIntegrity {
        all_resources_resolved,
        all_styles_resolved,
    }
}

fn identity_map_for_documents(
    source: &ChemcoreDocument,
    subset: &ChemcoreDocument,
    source_document_hash: &str,
    source_file_sha256: &str,
) -> Result<Value, String> {
    let source_selectors = document_selector_kinds(source);
    let subset_selectors = document_selector_kinds(subset);
    let entries = source_selectors
        .iter()
        .filter(|(selector, _)| subset_selectors.contains_key(*selector))
        .map(|(selector, kind)| {
            json!({
                "sourceSelector": selector,
                "bundleSelector": selector,
                "kind": kind,
                "includedBecause": identity_inclusion_reason(kind),
                "sourceDocumentHash": source_document_hash,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "schema": IDENTITY_MAP_SCHEMA,
        "sourceDocumentHash": source_document_hash,
        "sourceFileSha256": source_file_sha256,
        "selectorIdentityStable": true,
        "entries": entries,
    }))
}

fn document_selector_kinds(document: &ChemcoreDocument) -> BTreeMap<String, String> {
    let mut selectors = BTreeMap::new();
    for object in document.scene_objects() {
        selectors.insert(format!("object:{}", object.id), "object".to_string());
        if let Some(resource_ref) = object.payload.resource_ref.as_ref() {
            selectors.insert(format!("resource:{resource_ref}"), "resource".to_string());
        }
        if let Some(style_ref) = object.style_ref.as_ref() {
            selectors.insert(format!("style:{style_ref}"), "style".to_string());
        }
    }
    for entry in document.editable_fragments() {
        for node in &entry.fragment.nodes {
            selectors.insert(format!("node:{}", node.id), "node".to_string());
        }
        for bond in &entry.fragment.bonds {
            selectors.insert(format!("bond:{}", bond.id), "bond".to_string());
        }
    }
    selectors
}

fn identity_inclusion_reason(kind: &str) -> &'static str {
    match kind {
        "object" => "editable-target-or-required-ancestor",
        "resource" => "referenced-resource",
        "style" => "referenced-style",
        "node" | "bond" => "referenced-molecule-entity",
        _ => "included-in-editable-subset",
    }
}

#[allow(clippy::too_many_arguments)]
fn provenance_for_bundle(
    engine: &Engine,
    source_document: &ChemcoreDocument,
    subset: &ChemcoreDocument,
    options: &BundleOptions,
    target_bounds: [f64; 4],
    visual_bounds: [f64; 4],
    visual_view_box: [f64; 4],
    source_file_sha256: &str,
    source_document_hash: &str,
    identity_entry_count: usize,
) -> Result<Value, String> {
    let targets = flattened_targets(&options.target);
    Ok(json!({
        "schema": PROVENANCE_SCHEMA,
        "ok": true,
        "source": {
            "path": privacy_preserving_source_path(&options.input),
            "fileName": Path::new(&options.input).file_name().and_then(|value| value.to_str()),
            "format": infer_format_from_path(&options.input),
            "sha256": source_file_sha256,
            "documentHash": source_document_hash,
            "documentRevision": engine.revision(),
            "privacy": {
                "pathPolicy": "file-name-only",
                "absolutePathStored": false,
            }
        },
        "derivedFrom": {
            "documentHash": source_document_hash,
            "sourceFileSha256": source_file_sha256,
            "selector": options.target.selector(),
            "selectors": targets.iter().map(TargetSelector::selector).collect::<Vec<_>>(),
            "sourceBounds": bounds_json(target_bounds),
            "visualBounds": bounds_json(visual_bounds),
            "visualViewBox": view_box_json(visual_view_box),
            "translation": subset_translation_from_export_meta(subset),
        },
        "editableSubset": {
            "documentId": subset.document.id,
            "documentTitle": subset.document.title,
            "format": options.subset_format,
            "objectCount": subset.scene_objects().len(),
            "topLevelObjectCount": subset.objects.len(),
            "resourceCount": subset.resources.len(),
            "styleCount": subset.styles.len(),
            "scope": editable_scope_json(subset),
        },
        "identityMap": {
            "artifact": "identity-map.json",
            "schema": IDENTITY_MAP_SCHEMA,
            "entryCount": identity_entry_count,
            "selectorIdentityStable": true,
        },
        "sourceDocument": {
            "objectCount": source_document.scene_objects().len(),
            "resourceCount": source_document.resources.len(),
            "styleCount": source_document.styles.len(),
        }
    }))
}

fn subset_translation_from_export_meta(subset: &ChemcoreDocument) -> Value {
    let Some(bounds) = subset.document.meta.pointer("/export/selectionBounds") else {
        return Value::Null;
    };
    let min_x = bounds.get("minX").and_then(Value::as_f64).unwrap_or(0.0);
    let min_y = bounds.get("minY").and_then(Value::as_f64).unwrap_or(0.0);
    let margin = subset
        .document
        .meta
        .pointer("/export/selectionMargin")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    json!({
        "dx": margin - min_x,
        "dy": margin - min_y,
        "reason": "editable-subset-compaction",
    })
}

fn editable_scope_json(document: &ChemcoreDocument) -> Value {
    let object_selectors = document
        .scene_objects()
        .into_iter()
        .map(|object| format!("object:{}", object.id))
        .collect::<Vec<_>>();
    let resource_ids = document.resources.keys().cloned().collect::<Vec<_>>();
    let style_ids = document.styles.keys().cloned().collect::<Vec<_>>();
    let mut node_selectors = Vec::new();
    let mut bond_selectors = Vec::new();
    for entry in document.editable_fragments() {
        for node in &entry.fragment.nodes {
            node_selectors.push(format!("node:{}", node.id));
        }
        for bond in &entry.fragment.bonds {
            bond_selectors.push(format!("bond:{}", bond.id));
        }
    }
    json!({
        "objects": object_selectors,
        "resources": resource_ids,
        "styles": style_ids,
        "nodes": node_selectors,
        "bonds": bond_selectors,
        "note": "The editable subset is the only modification scope. Visual context may contain additional non-target objects."
    })
}

fn source_file_sha256(path: &str) -> Result<String, String> {
    file_sha256(Path::new(path))
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    Ok(sha256_bytes(&bytes))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = sha2::Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn privacy_preserving_source_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.to_string())
}
