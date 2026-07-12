use crate::{
    Bond, BondLineStyles, BondLineWeights, BondStereo, ChemcoreDocument, DocumentInfo,
    DocumentStyleInfo, DoubleBond, FormatInfo, LabelRun, MoleculeFragment, Node, NodeLabel,
    ObjectPayload, Page, Resource, ResourceData, SceneObject, Transform, EPSILON,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

mod colors;
mod export;
mod import_objects;
mod text_runs;
pub(crate) mod xml;

use self::colors::CdxmlColorTable;
pub use self::export::document_to_cdxml;
use self::import_objects::{
    append_bracket_objects, append_line_objects, append_orbital_shape_objects,
    append_shape_objects, append_table_shape_objects, append_text_objects,
    append_tlc_plate_shape_objects,
};
use self::text_runs::{label_display_runs, label_source_run};
pub(crate) use self::xml::parse_xml_tree;
use self::xml::{descendants, XmlNode};

#[derive(Debug, Clone, Copy)]
struct CdxmlDefaults {
    bond_length: f64,
    line_width: f64,
    bold_width: f64,
    hash_spacing: f64,
    bond_spacing: f64,
    margin_width: f64,
    label_size: f64,
    caption_size: f64,
    chain_angle: f64,
    label_font: u32,
    caption_font: u32,
    label_face: u32,
    caption_face: u32,
    label_justification: CdxmlJustification,
    caption_justification: CdxmlJustification,
    fractional_widths: bool,
    interpret_chemically: Option<bool>,
    show_atom_query: bool,
    show_atom_stereo: bool,
    show_atom_enhanced_stereo: bool,
    show_atom_number: bool,
    show_residue_id: bool,
    show_bond_query: bool,
    show_bond_rxn: bool,
    show_bond_stereo: bool,
    show_terminal_carbon_labels: bool,
    show_non_terminal_carbon_labels: bool,
    hide_implicit_hydrogens: bool,
    print_margins: [f64; 4],
    color: u32,
}

impl Default for CdxmlDefaults {
    fn default() -> Self {
        Self {
            bond_length: crate::DEFAULT_BOND_LENGTH,
            line_width: crate::DEFAULT_BOND_STROKE,
            bold_width: crate::BOLD_BOND_WIDTH_PT.value(),
            hash_spacing: crate::DEFAULT_HASH_SPACING_PT.value(),
            bond_spacing: crate::DEFAULT_BOND_SPACING_PERCENT,
            margin_width: crate::DEFAULT_BOND_MARGIN_WIDTH_PT.value(),
            label_size: crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT,
            caption_size: crate::DEFAULT_TEXT_FONT_SIZE_PT,
            chain_angle: 120.0,
            label_font: 3,
            caption_font: 3,
            label_face: 96,
            caption_face: 0,
            label_justification: CdxmlJustification::Auto,
            caption_justification: CdxmlJustification::Left,
            fractional_widths: true,
            interpret_chemically: None,
            show_atom_query: true,
            show_atom_stereo: false,
            show_atom_enhanced_stereo: true,
            show_atom_number: false,
            show_residue_id: false,
            show_bond_query: true,
            show_bond_rxn: true,
            show_bond_stereo: false,
            show_terminal_carbon_labels: false,
            show_non_terminal_carbon_labels: false,
            hide_implicit_hydrogens: false,
            print_margins: [36.0, 36.0, 36.0, 36.0],
            color: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CdxmlJustification {
    Auto,
    Left,
    Center,
    Right,
}

impl CdxmlJustification {
    fn as_cdxml(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Left => "Left",
            Self::Center => "Center",
            Self::Right => "Right",
        }
    }
}

pub fn parse_cdxml_document(cdxml: &str, title: Option<&str>) -> Result<ChemcoreDocument, String> {
    let root = parse_xml_tree(cdxml)?;
    let defaults = cdxml_defaults(&root);
    let colors = CdxmlColorTable::from_cdxml(&root);
    let fonts = cdxml_font_table(&root);
    let mut styles = default_cdxml_styles(defaults);
    let mut resources = BTreeMap::new();
    let mut objects = Vec::new();

    let fragments = display_fragments(&root);
    let display_fragment_ids: BTreeSet<String> = fragments
        .iter()
        .filter_map(|fragment| fragment.attr("id").map(ToString::to_string))
        .collect();
    let bonded_node_ids = cdxml_bonded_node_ids(&root);
    let mut molecule_index = 1usize;
    for fragment in &fragments {
        let Some(bbox) = parse_bbox(fragment.attr("BoundingBox")) else {
            continue;
        };
        let Some(resource) = normalize_fragment(fragment, bbox, defaults, &colors, &fonts) else {
            continue;
        };
        for component in split_cdxml_fragment_components(resource, bbox) {
            let resource_id = format!("mol_{:03}", molecule_index);
            let component_meta = cdxml_fragment_component_meta(
                fragment.attr("id"),
                component.component_index,
                component.component_count,
            );
            resources.insert(
                resource_id.clone(),
                Resource {
                    resource_type: "molecule_fragment2d".to_string(),
                    encoding: "chemcore.molecule.fragment2d".to_string(),
                    data: ResourceData::Fragment(component.fragment),
                    meta: component_meta.clone(),
                },
            );
            objects.push(SceneObject {
                id: format!("obj_mol_{:03}", molecule_index),
                object_type: "molecule".to_string(),
                name: format!("molecule {}", molecule_index),
                visible: true,
                locked: false,
                z_index: parse_i32(fragment.attr("Z")).unwrap_or(10),
                transform: Transform {
                    translate: [round2(component.bbox_abs[0]), round2(component.bbox_abs[1])],
                    rotate: 0.0,
                    scale: [1.0, 1.0],
                },
                style_ref: Some("style_molecule_default".to_string()),
                meta: component_meta,
                payload: ObjectPayload {
                    resource_ref: Some(resource_id),
                    bbox: Some([
                        0.0,
                        0.0,
                        round2(component.bbox_abs[2] - component.bbox_abs[0]),
                        round2(component.bbox_abs[3] - component.bbox_abs[1]),
                    ]),
                    extra: BTreeMap::new(),
                },
                children: Vec::new(),
            });
            molecule_index += 1;
        }
    }
    append_line_objects(&root, &mut objects, &mut styles, defaults, &colors);
    append_shape_objects(&root, &mut objects, &mut styles, defaults, &colors);
    append_orbital_shape_objects(&root, &mut objects, &mut styles, defaults, &colors);
    append_table_shape_objects(&root, &mut objects, &mut styles, defaults, &colors);
    append_tlc_plate_shape_objects(&root, &mut objects, &mut styles, defaults, &colors);
    append_bracket_objects(&root, &mut objects, defaults, &colors);
    append_text_objects(
        &root,
        &mut objects,
        &mut styles,
        defaults,
        &colors,
        &fonts,
        &display_fragment_ids,
        &bonded_node_ids,
    );
    apply_cdxml_groups(&root, &mut objects);
    let mut document = ChemcoreDocument {
        format: FormatInfo {
            name: "chemcore".to_string(),
            version: "0.1".to_string(),
            unit: "pt".to_string(),
        },
        document: DocumentInfo {
            id: "doc_cdxml_import".to_string(),
            title: title.unwrap_or("Imported CDXML").to_string(),
            page: page_from_objects(&objects, colors.background()),
            meta: json!({
                "createdBy": "chemcore",
                "sourceFormat": "cdxml",
                "nativeImport": true,
                "import": {
                    "cdxml": {
                        "defaults": {
                            "bondLength": defaults.bond_length,
                            "lineWidth": defaults.line_width,
                            "boldWidth": defaults.bold_width,
                            "hashSpacing": defaults.hash_spacing,
                            "bondSpacing": defaults.bond_spacing,
                            "marginWidth": defaults.margin_width,
                            "labelSize": defaults.label_size,
                            "captionSize": defaults.caption_size,
                            "chainAngle": defaults.chain_angle,
                            "labelFont": defaults.label_font,
                            "labelFace": defaults.label_face,
                            "captionFont": defaults.caption_font,
                            "captionFace": defaults.caption_face,
                            "labelJustification": defaults.label_justification.as_cdxml(),
                            "captionJustification": defaults.caption_justification.as_cdxml(),
                            "fractionalWidths": defaults.fractional_widths,
                            "interpretChemically": defaults.interpret_chemically,
                            "showAtomQuery": defaults.show_atom_query,
                            "showAtomStereo": defaults.show_atom_stereo,
                            "showAtomEnhancedStereo": defaults.show_atom_enhanced_stereo,
                            "showAtomNumber": defaults.show_atom_number,
                            "showResidueID": defaults.show_residue_id,
                            "showBondQuery": defaults.show_bond_query,
                            "showBondRxn": defaults.show_bond_rxn,
                            "showBondStereo": defaults.show_bond_stereo,
                            "showTerminalCarbonLabels": defaults.show_terminal_carbon_labels,
                            "showNonTerminalCarbonLabels": defaults.show_non_terminal_carbon_labels,
                            "hideImplicitHydrogens": defaults.hide_implicit_hydrogens,
                            "printMargins": defaults.print_margins,
                            "color": defaults.color,
                        }
                    }
                },
            }),
        },
        style: DocumentStyleInfo {
            preset: "default".to_string(),
            defaults: BTreeMap::from([
                ("bondLength".to_string(), defaults.bond_length),
                ("chainAngle".to_string(), defaults.chain_angle),
                ("lineWidth".to_string(), defaults.line_width),
                ("boldWidth".to_string(), defaults.bold_width),
                (
                    "wedgeWidth".to_string(),
                    cdxml_import_wedge_width(defaults.line_width, defaults.bold_width),
                ),
                ("hashSpacing".to_string(), defaults.hash_spacing),
                ("bondSpacing".to_string(), defaults.bond_spacing),
                ("marginWidth".to_string(), defaults.margin_width),
                ("graphicLineWidth".to_string(), defaults.line_width),
                ("labelFontSize".to_string(), defaults.label_size),
                ("textFontSize".to_string(), defaults.caption_size),
                ("labelFont".to_string(), defaults.label_font as f64),
                ("captionFont".to_string(), defaults.caption_font as f64),
                ("labelFace".to_string(), defaults.label_face as f64),
                ("captionFace".to_string(), defaults.caption_face as f64),
            ]),
        },
        styles,
        objects,
        resources,
    };
    crate::normalize_text_object_payloads(&mut document);
    crate::normalize_shape_object_payloads(&mut document);
    crate::normalize_arrow_object_payloads(&mut document);
    crate::normalize_fragment_label_payloads(&mut document);
    Ok(document)
}

fn apply_cdxml_groups(root: &XmlNode, objects: &mut Vec<SceneObject>) {
    let mut groups = Vec::new();
    let mut index = 1;
    collect_cdxml_groups(root, objects, &mut groups, &mut index);
    objects.extend(groups);
}

fn collect_cdxml_groups(
    node: &XmlNode,
    objects: &mut Vec<SceneObject>,
    groups: &mut Vec<SceneObject>,
    index: &mut usize,
) {
    for child in &node.children {
        if child.is("group") {
            if let Some(group) = cdxml_group_object(child, objects, index) {
                groups.push(group);
            }
        } else {
            collect_cdxml_groups(child, objects, groups, index);
        }
    }
}

fn cdxml_group_object(
    node: &XmlNode,
    objects: &mut Vec<SceneObject>,
    index: &mut usize,
) -> Option<SceneObject> {
    let group_number = *index;
    *index += 1;
    let mut children = Vec::new();
    for child in &node.children {
        if child.is("group") {
            if let Some(group) = cdxml_group_object(child, objects, index) {
                children.push(group);
            }
            continue;
        }
        children.extend(take_cdxml_child_objects(objects, child));
    }
    if children.is_empty() {
        return None;
    }
    let z_index = parse_i32(node.attr("Z")).unwrap_or_else(|| {
        children
            .iter()
            .map(|child| child.z_index)
            .min()
            .unwrap_or(0)
    });
    let payload_bbox = parse_bbox(node.attr("BoundingBox")).map(|bbox| {
        [
            round2(bbox[0]),
            round2(bbox[1]),
            round2(bbox[2] - bbox[0]),
            round2(bbox[3] - bbox[1]),
        ]
    });
    Some(SceneObject {
        id: format!("obj_group_{group_number:03}"),
        object_type: "group".to_string(),
        name: format!("group {group_number}"),
        visible: true,
        locked: false,
        z_index,
        transform: Transform::identity(),
        style_ref: None,
        meta: json!({
            "source": "cdxml",
            "groupId": node.attr("id"),
            "import": {
                "cdxml": {
                    "groupId": node.attr("id"),
                    "boundingBox": node.attr("BoundingBox"),
                }
            },
        }),
        payload: ObjectPayload {
            resource_ref: None,
            bbox: payload_bbox,
            extra: BTreeMap::new(),
        },
        children,
    })
}

fn take_cdxml_child_objects(objects: &mut Vec<SceneObject>, node: &XmlNode) -> Vec<SceneObject> {
    let Some(source_id) = node.attr("id") else {
        return Vec::new();
    };
    let mut taken = Vec::new();
    let mut index = 0;
    while index < objects.len() {
        if object_matches_cdxml_node(&objects[index], node, source_id) {
            taken.push(objects.remove(index));
        } else {
            index += 1;
        }
    }
    taken
}

fn object_matches_cdxml_node(object: &SceneObject, node: &XmlNode, source_id: &str) -> bool {
    match node.name.as_str() {
        "fragment" => object.meta.get("fragmentId").and_then(Value::as_str) == Some(source_id),
        "graphic" | "arrow" => {
            object.meta.get("graphicId").and_then(Value::as_str) == Some(source_id)
                || object
                    .meta
                    .get("graphicIds")
                    .and_then(Value::as_array)
                    .is_some_and(|ids| {
                        ids.iter()
                            .any(|id| id.as_str().is_some_and(|id| id == source_id))
                    })
        }
        "t" => object.meta.get("textId").and_then(Value::as_str) == Some(source_id),
        _ => false,
    }
}

pub(crate) fn normalize_cdxml_document_for_editing(document: &mut ChemcoreDocument) {
    scale_cdxml_document_for_editing(document);
}

const CDXML_EDITING_OUTPUT_SCALE: f64 = 1.0;

fn scale_cdxml_document_for_editing(document: &mut ChemcoreDocument) {
    if document
        .document
        .meta
        .pointer("/import/cdxml/editingScale")
        .is_some()
    {
        return;
    }

    let factor = CDXML_EDITING_OUTPUT_SCALE;
    document.document.page.width = round2(document.document.page.width * factor);
    document.document.page.height = round2(document.document.page.height * factor);
    for style in document.styles.values_mut() {
        scale_json_value_by_key("", style, factor);
    }
    scale_scene_objects_for_editing(&mut document.objects, factor);
    for resource in document.resources.values_mut() {
        if let ResourceData::Fragment(fragment) = &mut resource.data {
            scale_fragment_for_editing(fragment, factor);
        }
    }
    if let Some(cdxml_meta) = document
        .document
        .meta
        .get_mut("import")
        .and_then(|value| value.get_mut("cdxml"))
        .and_then(Value::as_object_mut)
    {
        cdxml_meta.insert("editingScale".to_string(), json!(factor));
    }
}

fn scale_scene_objects_for_editing(objects: &mut [SceneObject], factor: f64) {
    for object in objects {
        object.transform.translate[0] = round2(object.transform.translate[0] * factor);
        object.transform.translate[1] = round2(object.transform.translate[1] * factor);
        if let Some(bbox) = &mut object.payload.bbox {
            scale_bbox_for_editing(bbox, factor);
        }
        for (key, value) in &mut object.payload.extra {
            if key == "arrowHead" {
                continue;
            }
            scale_json_value_by_key(key, value, factor);
        }
        scale_scene_objects_for_editing(&mut object.children, factor);
    }
}

fn scale_fragment_for_editing(fragment: &mut MoleculeFragment, factor: f64) {
    scale_bbox_for_editing(&mut fragment.bbox, factor);
    for node in &mut fragment.nodes {
        scale_point_array_for_editing(&mut node.position, factor);
        if let Some(label) = &mut node.label {
            if let Some(position) = &mut label.position {
                scale_point_array_for_editing(position, factor);
            }
            if let Some(bbox) = &mut label.box_field {
                scale_bbox_for_editing(bbox, factor);
            }
            if let Some(bbox) = &mut label.box_value {
                scale_bbox_for_editing(bbox, factor);
            }
            if let Some(font_size) = &mut label.font_size {
                *font_size = round2(*font_size * factor);
            }
            for polygon in &mut label.glyph_polygons {
                for point in polygon {
                    scale_point_array_for_editing(point, factor);
                }
            }
            scale_label_runs_for_editing(&mut label.runs, factor);
            for runs in &mut label.line_runs {
                scale_label_runs_for_editing(runs, factor);
            }
        }
    }
    for bond in &mut fragment.bonds {
        bond.stroke_width = round2(bond.stroke_width * factor);
        scale_optional_number_for_editing(&mut bond.bold_width, factor);
        scale_optional_number_for_editing(&mut bond.wedge_width, factor);
        scale_optional_number_for_editing(&mut bond.label_clip_margin, factor);
        scale_optional_number_for_editing(&mut bond.hash_spacing, factor);
        scale_optional_number_for_editing(&mut bond.margin_width, factor);
    }
}

fn scale_label_runs_for_editing(runs: &mut [LabelRun], factor: f64) {
    for run in runs {
        if let Some(font_size) = &mut run.font_size {
            *font_size = round2(*font_size * factor);
        }
    }
}

fn scale_optional_number_for_editing(value: &mut Option<f64>, factor: f64) {
    if let Some(number) = value {
        *number = round2(*number * factor);
    }
}

fn scale_bbox_for_editing(bbox: &mut [f64; 4], factor: f64) {
    for value in bbox {
        *value = round2(*value * factor);
    }
}

fn scale_point_array_for_editing(point: &mut [f64; 2], factor: f64) {
    point[0] = round2(point[0] * factor);
    point[1] = round2(point[1] * factor);
}

fn scale_json_value_by_key(key: &str, value: &mut Value, factor: f64) {
    if scale_json_key_as_length_scalar(key) || scale_json_key_as_length_array(key) {
        scale_all_json_numbers(value, factor);
        return;
    }
    match value {
        Value::Array(items) => {
            for item in items {
                scale_json_value_by_key("", item, factor);
            }
        }
        Value::Object(map) => {
            for (child_key, child_value) in map {
                if child_key == "arrowHead" {
                    continue;
                }
                scale_json_value_by_key(child_key, child_value, factor);
            }
        }
        _ => {}
    }
}

fn scale_all_json_numbers(value: &mut Value, factor: f64) {
    match value {
        Value::Number(number) => {
            if let Some(raw) = number.as_f64() {
                *value = json!(round2(raw * factor));
            }
        }
        Value::Array(items) => {
            for item in items {
                scale_all_json_numbers(item, factor);
            }
        }
        Value::Object(map) => {
            for child_value in map.values_mut() {
                scale_all_json_numbers(child_value, factor);
            }
        }
        _ => {}
    }
}

fn scale_json_key_as_length_scalar(key: &str) -> bool {
    matches!(
        key,
        "width"
            | "height"
            | "x"
            | "y"
            | "rx"
            | "ry"
            | "radius"
            | "strokeWidth"
            | "fontSize"
            | "lineHeight"
            | "baselineOffset"
            | "wrapWidth"
            | "pad"
            | "padding"
            | "cornerRadius"
            | "shadowSize"
            | "dashSpacing"
    )
}

fn scale_json_key_as_length_array(key: &str) -> bool {
    matches!(
        key,
        "bbox"
            | "box"
            | "boxField"
            | "boundingBox"
            | "cdxmlBoundingBox"
            | "position"
            | "textPosition"
            | "translate"
            | "points"
            | "center"
            | "majorAxisEnd"
            | "minorAxisEnd"
            | "axisStart"
            | "axisEnd"
            | "anchorOffset"
            | "glyphPolygons"
            | "dashArray"
    )
}

fn cdxml_defaults(root: &XmlNode) -> CdxmlDefaults {
    let fallback = CdxmlDefaults::default();
    CdxmlDefaults {
        bond_length: parse_f64(root.attr("BondLength")).unwrap_or(crate::DEFAULT_BOND_LENGTH),
        line_width: parse_f64(root.attr("LineWidth")).unwrap_or(crate::DEFAULT_BOND_STROKE),
        bold_width: parse_f64(root.attr("BoldWidth")).unwrap_or(crate::BOLD_BOND_WIDTH_PT.value()),
        hash_spacing: parse_f64(root.attr("HashSpacing"))
            .unwrap_or(crate::DEFAULT_HASH_SPACING_PT.value()),
        bond_spacing: parse_f64(root.attr("BondSpacing"))
            .unwrap_or(crate::DEFAULT_BOND_SPACING_PERCENT),
        margin_width: parse_f64(root.attr("MarginWidth"))
            .unwrap_or(crate::DEFAULT_BOND_MARGIN_WIDTH_PT.value()),
        label_size: parse_f64(root.attr("LabelSize"))
            .unwrap_or(crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT),
        caption_size: parse_f64(root.attr("CaptionSize"))
            .unwrap_or(crate::DEFAULT_TEXT_FONT_SIZE_PT),
        chain_angle: parse_f64(root.attr("ChainAngle")).unwrap_or(fallback.chain_angle),
        label_font: parse_u32(root.attr("LabelFont")).unwrap_or(fallback.label_font),
        caption_font: parse_u32(root.attr("CaptionFont")).unwrap_or(fallback.caption_font),
        label_face: parse_u32(root.attr("LabelFace")).unwrap_or(fallback.label_face),
        caption_face: parse_u32(root.attr("CaptionFace")).unwrap_or(fallback.caption_face),
        label_justification: parse_cdxml_justification(root.attr("LabelJustification"))
            .unwrap_or(fallback.label_justification),
        caption_justification: parse_cdxml_justification(root.attr("CaptionJustification"))
            .unwrap_or(fallback.caption_justification),
        fractional_widths: parse_cdxml_bool(root.attr("FractionalWidths"))
            .unwrap_or(fallback.fractional_widths),
        interpret_chemically: parse_cdxml_bool(root.attr("InterpretChemically")),
        show_atom_query: parse_cdxml_bool(root.attr("ShowAtomQuery"))
            .unwrap_or(fallback.show_atom_query),
        show_atom_stereo: parse_cdxml_bool(root.attr("ShowAtomStereo"))
            .unwrap_or(fallback.show_atom_stereo),
        show_atom_enhanced_stereo: parse_cdxml_bool(root.attr("ShowAtomEnhancedStereo"))
            .unwrap_or(fallback.show_atom_enhanced_stereo),
        show_atom_number: parse_cdxml_bool(root.attr("ShowAtomNumber"))
            .unwrap_or(fallback.show_atom_number),
        show_residue_id: parse_cdxml_bool(root.attr("ShowResidueID"))
            .unwrap_or(fallback.show_residue_id),
        show_bond_query: parse_cdxml_bool(root.attr("ShowBondQuery"))
            .unwrap_or(fallback.show_bond_query),
        show_bond_rxn: parse_cdxml_bool(root.attr("ShowBondRxn")).unwrap_or(fallback.show_bond_rxn),
        show_bond_stereo: parse_cdxml_bool(root.attr("ShowBondStereo"))
            .unwrap_or(fallback.show_bond_stereo),
        show_terminal_carbon_labels: parse_cdxml_bool(root.attr("ShowTerminalCarbonLabels"))
            .unwrap_or(fallback.show_terminal_carbon_labels),
        show_non_terminal_carbon_labels: parse_cdxml_bool(root.attr("ShowNonTerminalCarbonLabels"))
            .unwrap_or(fallback.show_non_terminal_carbon_labels),
        hide_implicit_hydrogens: parse_cdxml_bool(root.attr("HideImplicitHydrogens"))
            .unwrap_or(fallback.hide_implicit_hydrogens),
        print_margins: parse_cdxml_margins(root.attr("PrintMargins"))
            .unwrap_or(fallback.print_margins),
        color: parse_u32(root.attr("color")).unwrap_or(fallback.color),
    }
}

fn parse_cdxml_bool(value: Option<&str>) -> Option<bool> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "yes" | "true" | "1" => Some(true),
        "no" | "false" | "0" => Some(false),
        _ => None,
    }
}

fn parse_cdxml_justification(value: Option<&str>) -> Option<CdxmlJustification> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "auto" => Some(CdxmlJustification::Auto),
        "left" | "start" => Some(CdxmlJustification::Left),
        "center" | "middle" => Some(CdxmlJustification::Center),
        "right" | "end" => Some(CdxmlJustification::Right),
        _ => None,
    }
}

fn parse_cdxml_margins(value: Option<&str>) -> Option<[f64; 4]> {
    let parts: Vec<f64> = value?
        .split_whitespace()
        .take(4)
        .filter_map(|part| part.parse().ok())
        .collect();
    (parts.len() == 4).then_some([parts[0], parts[1], parts[2], parts[3]])
}

fn default_cdxml_styles(defaults: CdxmlDefaults) -> BTreeMap<String, Value> {
    BTreeMap::from([
        (
            "style_molecule_default".to_string(),
            json!({
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": defaults.line_width,
                "fontFamily": "Arial",
                "fontSize": defaults.label_size,
            }),
        ),
        (
            "style_text_default".to_string(),
            json!({
                "kind": "text",
                "fontFamily": "Arial",
                "fontSize": defaults.caption_size,
                "fontWeight": 400,
                "fill": "#000000",
                "stroke": null,
            }),
        ),
        (
            "style_arrow_default".to_string(),
            json!({
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": defaults.line_width,
                "lineCap": "butt",
                "lineJoin": "miter",
                "dashArray": [],
            }),
        ),
        (
            "style_line_default".to_string(),
            json!({
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": defaults.line_width,
                "lineCap": "round",
                "lineJoin": "round",
                "dashArray": [],
            }),
        ),
    ])
}

fn cdxml_font_table(root: &XmlNode) -> BTreeMap<String, String> {
    let mut fonts = BTreeMap::from([("3".to_string(), "Arial".to_string())]);
    if let Some(table) = descendants(root)
        .into_iter()
        .find(|node| node.is("fonttable"))
    {
        for font in table.direct_children("font") {
            if let (Some(id), Some(name)) = (font.attr("id"), font.attr("name")) {
                fonts.insert(id.to_string(), name.to_string());
            }
        }
    }
    fonts
}

fn display_fragments(root: &XmlNode) -> Vec<&XmlNode> {
    let mut fragments = Vec::new();
    collect_display_fragments(root, false, &mut fragments);
    fragments
}

fn collect_display_fragments<'a>(
    node: &'a XmlNode,
    inside_placeholder_node: bool,
    fragments: &mut Vec<&'a XmlNode>,
) {
    if !inside_placeholder_node && cdxml_node_is_display_fragment(node) {
        fragments.push(node);
    }
    let next_inside_placeholder = inside_placeholder_node || cdxml_node_has_cached_fragment(node);
    for child in &node.children {
        collect_display_fragments(child, next_inside_placeholder, fragments);
    }
}

fn cdxml_node_is_display_fragment(node: &XmlNode) -> bool {
    node.is("fragment")
        && node.attr("BoundingBox").is_some()
        && node.direct_children("n").count() >= 2
        && node.direct_children("b").next().is_some()
}

fn cdxml_node_has_cached_fragment(node: &XmlNode) -> bool {
    node.is("n")
        && matches!(
            node.attr("NodeType").unwrap_or(""),
            "Fragment" | "Nickname" | "GenericNickname" | "Unspecified"
        )
}

fn cdxml_bonded_node_ids(root: &XmlNode) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for bond in descendants(root).into_iter().filter(|node| node.is("b")) {
        if let Some(begin) = bond.attr("B") {
            ids.insert(begin.to_string());
        }
        if let Some(end) = bond.attr("E") {
            ids.insert(end.to_string());
        }
    }
    ids
}

fn normalize_fragment(
    fragment: &XmlNode,
    bbox: [f64; 4],
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
) -> Option<MoleculeFragment> {
    let origin = [bbox[0], bbox[1]];
    let node_ids: BTreeSet<String> = fragment
        .direct_children("n")
        .filter_map(|node| node.attr("id").map(ToString::to_string))
        .collect();
    let nodes: Vec<Node> = fragment
        .direct_children("n")
        .filter_map(|node| normalize_node(node, origin, colors, fonts, defaults))
        .collect();
    let bonds: Vec<Bond> = fragment
        .direct_children("b")
        .filter_map(|bond| normalize_bond(bond, &node_ids, defaults, colors))
        .collect();
    if nodes.len() < 2 || bonds.is_empty() {
        return None;
    }
    let mut fragment = MoleculeFragment {
        schema: "chemcore.molecule.fragment2d".to_string(),
        bbox: [
            0.0,
            0.0,
            round2(bbox[2] - bbox[0]),
            round2(bbox[3] - bbox[1]),
        ],
        nodes,
        bonds,
        meta: json!({
            "import": {
                "cdxml": {
                    "fragmentId": fragment.attr("id"),
                    "bboxAbs": bbox,
                    "z": parse_i32(fragment.attr("Z")),
                }
            }
        }),
    };
    crate::engine::refresh_attached_node_label_geometry_for_all_nodes_with_profile(
        &mut fragment,
        origin,
        defaults.line_width,
        Some(crate::GlyphClipProfile::from_margin_width(
            defaults.margin_width,
        )),
    );
    infer_cdxml_ring_double_bond_placements(&mut fragment);
    Some(fragment)
}

#[derive(Debug)]
struct CdxmlFragmentComponent {
    fragment: MoleculeFragment,
    bbox_abs: [f64; 4],
    component_index: usize,
    component_count: usize,
}

fn split_cdxml_fragment_components(
    fragment: MoleculeFragment,
    source_bbox_abs: [f64; 4],
) -> Vec<CdxmlFragmentComponent> {
    let components = fragment_connected_components(&fragment);
    if components.len() <= 1 {
        return vec![CdxmlFragmentComponent {
            fragment,
            bbox_abs: source_bbox_abs,
            component_index: 0,
            component_count: 1,
        }];
    }

    let component_count = components.len();
    components
        .into_iter()
        .enumerate()
        .filter_map(|(component_index, node_ids)| {
            let mut nodes: Vec<Node> = fragment
                .nodes
                .iter()
                .filter(|node| node_ids.contains(&node.id))
                .cloned()
                .collect();
            let bonds: Vec<Bond> = fragment
                .bonds
                .iter()
                .filter(|bond| node_ids.contains(&bond.begin) && node_ids.contains(&bond.end))
                .cloned()
                .collect();
            if !cdxml_component_has_visible_molecule_content(&nodes, &bonds) {
                return None;
            }

            let local_bounds = component_local_bounds(&nodes).unwrap_or([
                0.0,
                0.0,
                fragment.bbox[2].max(1.0),
                fragment.bbox[3].max(1.0),
            ]);
            let delta_x = -local_bounds[0];
            let delta_y = -local_bounds[1];
            for node in &mut nodes {
                node.position[0] = round2(node.position[0] + delta_x);
                node.position[1] = round2(node.position[1] + delta_y);
                if let Some(label) = &mut node.label {
                    translate_node_label_geometry(label, delta_x, delta_y);
                }
            }

            let bbox_abs = [
                round2(source_bbox_abs[0] + local_bounds[0]),
                round2(source_bbox_abs[1] + local_bounds[1]),
                round2(source_bbox_abs[0] + local_bounds[2]),
                round2(source_bbox_abs[1] + local_bounds[3]),
            ];
            let mut component_fragment = MoleculeFragment {
                schema: fragment.schema.clone(),
                bbox: [
                    0.0,
                    0.0,
                    round2((local_bounds[2] - local_bounds[0]).max(1.0)),
                    round2((local_bounds[3] - local_bounds[1]).max(1.0)),
                ],
                nodes,
                bonds,
                meta: fragment.meta.clone(),
            };
            annotate_cdxml_component_fragment_meta(
                &mut component_fragment,
                source_bbox_abs,
                bbox_abs,
                component_index,
                component_count,
            );
            Some(CdxmlFragmentComponent {
                fragment: component_fragment,
                bbox_abs,
                component_index,
                component_count,
            })
        })
        .collect()
}

fn fragment_connected_components(fragment: &MoleculeFragment) -> Vec<BTreeSet<String>> {
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for node in &fragment.nodes {
        adjacency.entry(node.id.as_str()).or_default();
    }
    for bond in &fragment.bonds {
        adjacency
            .entry(bond.begin.as_str())
            .or_default()
            .push(bond.end.as_str());
        adjacency
            .entry(bond.end.as_str())
            .or_default()
            .push(bond.begin.as_str());
    }

    let mut visited = BTreeSet::new();
    let mut components = Vec::new();
    for node in &fragment.nodes {
        if visited.contains(node.id.as_str()) {
            continue;
        }
        let mut queue = VecDeque::from([node.id.as_str()]);
        let mut component = BTreeSet::new();
        while let Some(id) = queue.pop_front() {
            if !visited.insert(id) {
                continue;
            }
            component.insert(id.to_string());
            if let Some(neighbors) = adjacency.get(id) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        if !component.is_empty() {
            components.push(component);
        }
    }
    components
}

fn cdxml_component_has_visible_molecule_content(nodes: &[Node], bonds: &[Bond]) -> bool {
    !bonds.is_empty()
        || nodes.iter().any(|node| {
            node.atomic_number != 6
                || node
                    .label
                    .as_ref()
                    .is_some_and(|label| label.has_visible_text())
        })
}

fn component_local_bounds(nodes: &[Node]) -> Option<[f64; 4]> {
    let mut bounds = None;
    for node in nodes {
        include_point_in_bounds(&mut bounds, node.position);
        if let Some(label) = &node.label {
            if let Some(label_bounds) = label.bbox() {
                include_box_in_bounds(&mut bounds, label_bounds);
            }
            for polygon in &label.glyph_polygons {
                for point in polygon {
                    include_point_in_bounds(&mut bounds, *point);
                }
            }
        }
    }
    bounds.map(|mut bounds| {
        if (bounds[2] - bounds[0]).abs() < 1.0 {
            let center = (bounds[0] + bounds[2]) * 0.5;
            bounds[0] = center - 0.5;
            bounds[2] = center + 0.5;
        }
        if (bounds[3] - bounds[1]).abs() < 1.0 {
            let center = (bounds[1] + bounds[3]) * 0.5;
            bounds[1] = center - 0.5;
            bounds[3] = center + 0.5;
        }
        [
            round2(bounds[0]),
            round2(bounds[1]),
            round2(bounds[2]),
            round2(bounds[3]),
        ]
    })
}

fn include_point_in_bounds(bounds: &mut Option<[f64; 4]>, point: [f64; 2]) {
    if let Some(bounds) = bounds {
        bounds[0] = bounds[0].min(point[0]);
        bounds[1] = bounds[1].min(point[1]);
        bounds[2] = bounds[2].max(point[0]);
        bounds[3] = bounds[3].max(point[1]);
    } else {
        *bounds = Some([point[0], point[1], point[0], point[1]]);
    }
}

fn include_box_in_bounds(bounds: &mut Option<[f64; 4]>, bbox: [f64; 4]) {
    include_point_in_bounds(bounds, [bbox[0], bbox[1]]);
    include_point_in_bounds(bounds, [bbox[2], bbox[3]]);
}

fn translate_node_label_geometry(label: &mut NodeLabel, delta_x: f64, delta_y: f64) {
    if delta_x.abs() <= EPSILON && delta_y.abs() <= EPSILON {
        return;
    }
    if let Some(position) = &mut label.position {
        position[0] = round2(position[0] + delta_x);
        position[1] = round2(position[1] + delta_y);
    }
    if let Some(bbox) = &mut label.box_field {
        translate_bbox(bbox, delta_x, delta_y);
    }
    if let Some(bbox) = &mut label.box_value {
        translate_bbox(bbox, delta_x, delta_y);
    }
    for polygon in &mut label.glyph_polygons {
        for point in polygon {
            point[0] = round2(point[0] + delta_x);
            point[1] = round2(point[1] + delta_y);
        }
    }
}

fn translate_bbox(bbox: &mut [f64; 4], delta_x: f64, delta_y: f64) {
    bbox[0] = round2(bbox[0] + delta_x);
    bbox[1] = round2(bbox[1] + delta_y);
    bbox[2] = round2(bbox[2] + delta_x);
    bbox[3] = round2(bbox[3] + delta_y);
}

fn annotate_cdxml_component_fragment_meta(
    fragment: &mut MoleculeFragment,
    source_bbox_abs: [f64; 4],
    bbox_abs: [f64; 4],
    component_index: usize,
    component_count: usize,
) {
    let Some(cdxml_meta) = fragment
        .meta
        .get_mut("import")
        .and_then(|value| value.get_mut("cdxml"))
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    cdxml_meta.insert("sourceFragmentBboxAbs".to_string(), json!(source_bbox_abs));
    cdxml_meta.insert("bboxAbs".to_string(), json!(bbox_abs));
    cdxml_meta.insert("componentIndex".to_string(), json!(component_index));
    cdxml_meta.insert("componentCount".to_string(), json!(component_count));
}

fn cdxml_fragment_component_meta(
    fragment_id: Option<&str>,
    component_index: usize,
    component_count: usize,
) -> Value {
    let mut cdxml = serde_json::Map::new();
    cdxml.insert("fragmentId".to_string(), json!(fragment_id));
    if component_count > 1 {
        cdxml.insert("componentIndex".to_string(), json!(component_index));
        cdxml.insert("componentCount".to_string(), json!(component_count));
    }
    json!({
        "source": "cdxml",
        "import": { "cdxml": cdxml },
        "fragmentId": fragment_id,
    })
}

fn normalize_node(
    node: &XmlNode,
    origin: [f64; 2],
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
    defaults: CdxmlDefaults,
) -> Option<Node> {
    let id = node.attr("id")?.to_string();
    let position = parse_xy(node.attr("p"))?;
    let atomic_number = parse_u8(node.attr("Element")).unwrap_or(6);
    let node_type = node.attr("NodeType").unwrap_or("");
    let label = node_label(node, origin, colors, fonts, defaults);
    let is_bullet_carbon = atomic_number == 6
        && label
            .as_ref()
            .is_some_and(imported_cdxml_bullet_carbon_node_label);
    let radical_count = cdxml_radical_count(node.attr("Radical"));
    let mut meta = json!({
        "import": {
            "cdxml": {
                "z": parse_i32(node.attr("Z")),
                "nodeType": empty_as_null(node.attr("NodeType")),
                "labelDisplay": empty_as_null(node.attr("LabelDisplay")),
                "element": node.attr("Element"),
                "radical": empty_as_null(node.attr("Radical")),
            }
        }
    });
    if radical_count != 0 {
        meta["radicalCount"] = json!(radical_count);
    }
    Some(Node {
        id,
        element: element_symbol(atomic_number).to_string(),
        atomic_number,
        position: [
            round2(position[0] - origin[0]),
            round2(position[1] - origin[1]),
        ],
        charge: parse_i32(node.attr("Charge")).unwrap_or(0),
        num_hydrogens: parse_u8(node.attr("NumHydrogens")).unwrap_or(0),
        is_external_connection_point: node_type == "ExternalConnectionPoint",
        is_placeholder: matches!(
            node_type,
            "Fragment" | "Nickname" | "GenericNickname" | "Unspecified"
        ) && !is_bullet_carbon,
        label,
        meta,
    })
}

fn cdxml_radical_count(value: Option<&str>) -> i32 {
    match value.unwrap_or("").trim().to_ascii_lowercase().as_str() {
        "" | "none" => 0,
        "doublet" | "monovalent" | "radical" => 1,
        "singlet" | "triplet" | "divalent" | "divalentsinglet" | "divalenttriplet" => 2,
        other => other.parse::<i32>().unwrap_or(0).clamp(0, 9),
    }
}

fn imported_cdxml_bullet_carbon_node_label(label: &NodeLabel) -> bool {
    label.attachment.as_deref() == Some("node")
        && label.source_text.as_deref().unwrap_or(label.text.as_str()) == "•"
        && label.meta.pointer("/import/cdxml/boundingBox").is_some()
        && label.meta.pointer("/import/cdxml/textPosition").is_some()
}

fn node_label(
    node: &XmlNode,
    origin: [f64; 2],
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
    defaults: CdxmlDefaults,
) -> Option<NodeLabel> {
    let text_el = node.direct_children("t").next()?;
    let text = text_el
        .attr("UTF8Text")
        .map(ToString::to_string)
        .unwrap_or_else(|| text_el.full_text())
        .trim()
        .to_string();
    if text.is_empty() {
        return None;
    }
    let bbox = parse_bbox(text_el.attr("BoundingBox"));
    let explicit_interpret_chemically = parse_cdxml_bool(text_el.attr("InterpretChemically"))
        .or_else(|| parse_cdxml_bool(node.attr("InterpretChemically")))
        .or(defaults.interpret_chemically);
    let parent_face = parse_u32(text_el.attr("face")).unwrap_or(defaults.label_face);
    let run_has_chemical_face = text_el
        .direct_children("s")
        .any(|run| parse_u32(run.attr("face")).unwrap_or(parent_face) & 96 == 96);
    let interpret_chemically = explicit_interpret_chemically
        .unwrap_or_else(|| run_has_chemical_face || node.attr("Element").is_some());
    let default_label_font = defaults.label_font.to_string();
    let parent_font = text_el
        .attr("font")
        .or_else(|| {
            text_el
                .direct_children("s")
                .find_map(|run| run.attr("font"))
        })
        .unwrap_or(default_label_font.as_str());
    let parent_color = text_el
        .attr("color")
        .or_else(|| {
            text_el
                .direct_children("s")
                .find_map(|run| run.attr("color"))
        })
        .unwrap_or("0");
    let parent_size = parse_f64(text_el.attr("size")).unwrap_or_else(|| {
        text_el
            .direct_children("s")
            .find_map(|run| parse_f64(run.attr("size")))
            .unwrap_or(defaults.label_size)
    });
    let mut source_runs: Vec<LabelRun> = text_el
        .direct_children("s")
        .filter_map(|run| {
            let run_text = run.full_text();
            (!run_text.is_empty()).then(|| {
                label_source_run(
                    &run_text,
                    parse_u32(run.attr("face")).unwrap_or(parent_face),
                    run.attr("font").unwrap_or(parent_font),
                    run.attr("color").unwrap_or(parent_color),
                    parse_f64(run.attr("size")).unwrap_or(parent_size),
                    colors,
                    fonts,
                )
            })
        })
        .collect();
    for run in &mut source_runs {
        match (interpret_chemically, run.script.as_deref()) {
            (true, None | Some("normal")) => run.script = Some("chemical".to_string()),
            (false, Some("chemical")) => run.script = Some("normal".to_string()),
            _ => {}
        }
    }
    let runs: Vec<LabelRun> = text_el
        .direct_children("s")
        .flat_map(|run| {
            let run_text = run.full_text();
            if run_text.is_empty() {
                Vec::new()
            } else {
                label_display_runs(
                    &run_text,
                    parse_u32(run.attr("face")).unwrap_or(parent_face),
                    run.attr("font").unwrap_or(parent_font),
                    run.attr("color").unwrap_or(parent_color),
                    parse_f64(run.attr("size")).unwrap_or(parent_size),
                    colors,
                    fonts,
                )
            }
        })
        .collect();
    let text_position = parse_xy(text_el.attr("p")).or_else(|| parse_xy(node.attr("p")));
    let local_node_position = parse_xy(node.attr("p"))
        .map(|point| [round2(point[0] - origin[0]), round2(point[1] - origin[1])]);
    let label_display = node.attr("LabelDisplay");
    let label_justification = text_el
        .attr("Justification")
        .or_else(|| text_el.attr("LabelJustification"))
        .or(Some(defaults.label_justification.as_cdxml()));
    let inferred_align = infer_cdxml_label_align(
        label_display,
        label_justification,
        text_el.attr("LabelAlignment"),
    );
    let is_centered = inferred_align == "center";
    let layout = match cdxml_label_flow(label_display, text_el.attr("LabelAlignment")) {
        Some(crate::LabelFlow::StackAbove) => Some("attached-group-above".to_string()),
        Some(crate::LabelFlow::StackBelow) => Some("attached-group-below".to_string()),
        _ if is_centered => Some("attached-group-center".to_string()),
        _ => None,
    };
    Some(NodeLabel {
        text: text.clone(),
        source_text: Some(text.clone()),
        position: local_node_position,
        box_field: None,
        runs,
        line_runs: Vec::new(),
        lines: if text.contains('\n') {
            text.lines().map(ToString::to_string).collect()
        } else {
            Vec::new()
        },
        align: Some(inferred_align.to_string()),
        layout,
        attachment: Some("node".to_string()),
        anchor: Some(
            match inferred_align {
                "center" => "middle",
                "right" => "end",
                _ => "start",
            }
            .to_string(),
        ),
        font_family: Some(
            fonts
                .get(parent_font)
                .cloned()
                .unwrap_or_else(|| "Arial".to_string()),
        ),
        fill: Some(colors.resolve(Some(parent_color))),
        font_size: Some(parent_size),
        glyph_polygons: Vec::new(),
        box_value: None,
        meta: json!({
            "import": {
                "cdxml": {
                    "font": parent_font,
                    "face": parent_face,
                    "color": parent_color,
                    "textPosition": text_position,
                    "boundingBox": bbox,
                    "labelDisplay": empty_as_null(label_display),
                    "labelAlignment": empty_as_null(text_el.attr("LabelAlignment")),
                    "labelJustification": empty_as_null(text_el.attr("LabelJustification")),
                    "justification": empty_as_null(text_el.attr("Justification")),
                    "interpretChemically": interpret_chemically,
                    "interpretChemicallyExplicit": explicit_interpret_chemically.is_some(),
                    "marginWidth": defaults.margin_width,
                    "naturalOutsetPt": defaults.margin_width,
                    "circleRadiusPt": defaults.margin_width * 2.0,
                }
            },
            "defaultChemical": interpret_chemically,
            "implicitHydrogenLabel": {
                "source": "cdxml",
                "userEdited": true,
            },
            "sourceRuns": source_runs,
        }),
    })
}

fn attr_eq_ignore_ascii_case(value: Option<&str>, expected: &str) -> bool {
    value.is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

fn infer_cdxml_label_align(
    label_display: Option<&str>,
    label_justification: Option<&str>,
    label_alignment: Option<&str>,
) -> &'static str {
    if attr_eq_ignore_ascii_case(label_display, "Center") {
        "center"
    } else if attr_eq_ignore_ascii_case(label_display, "Right") {
        "right"
    } else if attr_eq_ignore_ascii_case(label_display, "Left") {
        "left"
    } else if attr_eq_ignore_ascii_case(label_justification, "Center") {
        "center"
    } else if attr_eq_ignore_ascii_case(label_justification, "Right") {
        "right"
    } else if attr_eq_ignore_ascii_case(label_justification, "Left") {
        "left"
    } else if attr_eq_ignore_ascii_case(label_alignment, "Center") {
        "center"
    } else if attr_eq_ignore_ascii_case(label_alignment, "Right") {
        "right"
    } else {
        "left"
    }
}

fn cdxml_label_flow(
    label_display: Option<&str>,
    label_alignment: Option<&str>,
) -> Option<crate::LabelFlow> {
    let value = label_display
        .filter(|value| matches!(*value, "Above" | "Below"))
        .or(label_alignment);
    match value {
        Some("Above") => Some(crate::LabelFlow::StackAbove),
        Some("Below") => Some(crate::LabelFlow::StackBelow),
        _ => None,
    }
}

fn normalize_bond(
    bond: &XmlNode,
    node_ids: &BTreeSet<String>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) -> Option<Bond> {
    let begin = bond.attr("B")?.to_string();
    let end = bond.attr("E")?.to_string();
    if !node_ids.contains(&begin) || !node_ids.contains(&end) {
        return None;
    }
    let display = bond.attr("Display").unwrap_or("");
    let stroke_width = parse_f64(bond.attr("LineWidth")).unwrap_or(defaults.line_width);
    let bold_width = parse_f64(bond.attr("BoldWidth")).unwrap_or(defaults.bold_width);
    let hash_spacing = parse_f64(bond.attr("HashSpacing")).unwrap_or(defaults.hash_spacing);
    let bond_spacing = parse_f64(bond.attr("BondSpacing")).unwrap_or(defaults.bond_spacing);
    let stereo = match display {
        "WedgeBegin" => Some(BondStereo {
            kind: "solid-wedge".to_string(),
            wide_end: "end".to_string(),
        }),
        "WedgeEnd" => Some(BondStereo {
            kind: "solid-wedge".to_string(),
            wide_end: "begin".to_string(),
        }),
        "WedgedHashBegin" => Some(BondStereo {
            kind: "hashed-wedge".to_string(),
            wide_end: "end".to_string(),
        }),
        "WedgedHashEnd" => Some(BondStereo {
            kind: "hashed-wedge".to_string(),
            wide_end: "begin".to_string(),
        }),
        "HollowWedgeBegin" => Some(BondStereo {
            kind: "hollow-wedge".to_string(),
            wide_end: "end".to_string(),
        }),
        "HollowWedgeEnd" => Some(BondStereo {
            kind: "hollow-wedge".to_string(),
            wide_end: "begin".to_string(),
        }),
        _ => None,
    };
    let order = cdxml_bond_order(bond.attr("Order"));
    let display2 = bond.attr("Display2").unwrap_or("");
    let mut line_styles = cdxml_bond_line_styles(order, display, display2);
    let mut line_weights = cdxml_bond_line_weights(order, display, display2);
    let placement = match bond
        .attr("DoublePosition")
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "left" => Some((crate::DoubleBondPlacement::Left, true)),
        "right" => Some((crate::DoubleBondPlacement::Right, true)),
        "center" => Some((crate::DoubleBondPlacement::Center, true)),
        _ => None,
    };
    if order >= 2 {
        if let Some((placement, _)) = placement {
            cdxml_apply_line_style_for_double_placement(
                order,
                display,
                display2,
                placement,
                &mut line_styles,
                &mut line_weights,
            );
        }
    }
    Some(Bond {
        id: bond.attr("id").unwrap_or("").to_string(),
        begin,
        end,
        order,
        double: placement.map(|(placement, frozen)| crate::DoubleBond {
            placement,
            center_exit_side: None,
            frozen,
        }),
        stereo,
        stroke_width,
        stroke: bond.attr("color").map(|color| colors.resolve(Some(color))),
        bold_width: Some(bold_width),
        wedge_width: Some(cdxml_import_wedge_width(stroke_width, bold_width)),
        label_clip_margin: None,
        hash_spacing: Some(hash_spacing),
        bond_spacing: Some(bond_spacing),
        margin_width: None,
        line_styles,
        line_weights,
        meta: json!({"import": {"cdxml": {"z": parse_i32(bond.attr("Z")), "display": empty_as_null(bond.attr("Display")), "display2": empty_as_null(bond.attr("Display2")), "doublePosition": empty_as_null(bond.attr("DoublePosition"))}}}),
    })
}

fn infer_cdxml_ring_double_bond_placements(fragment: &mut MoleculeFragment) {
    infer_unspecified_cdxml_double_bond_placements(fragment);
}

fn infer_unspecified_cdxml_double_bond_placements(fragment: &mut MoleculeFragment) {
    let inferred: Vec<_> = fragment
        .bonds
        .iter()
        .enumerate()
        .filter_map(|(index, bond)| {
            if bond.order != 2
                || bond.double.is_some()
                || cdxml_bond_has_explicit_double_position(bond)
            {
                return None;
            }
            let placement = crate::engine::automatic_double_bond_placement_for_segment(
                fragment,
                &bond.begin,
                &bond.end,
                Some(&bond.id),
            );
            Some((index, placement))
        })
        .collect();
    for (index, placement) in inferred {
        cdxml_apply_imported_line_style_for_current_double_placement(
            &mut fragment.bonds[index],
            placement,
        );
        fragment.bonds[index].double = Some(DoubleBond {
            placement,
            center_exit_side: None,
            frozen: false,
        });
    }
}

fn cdxml_bond_has_explicit_double_position(bond: &Bond) -> bool {
    bond.meta
        .pointer("/import/cdxml/doublePosition")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
}

fn cdxml_apply_imported_line_style_for_current_double_placement(
    bond: &mut Bond,
    placement: crate::DoubleBondPlacement,
) {
    let display = bond
        .meta
        .pointer("/import/cdxml/display")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let display2 = bond
        .meta
        .pointer("/import/cdxml/display2")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    cdxml_apply_line_style_for_double_placement(
        bond.order,
        &display,
        &display2,
        placement,
        &mut bond.line_styles,
        &mut bond.line_weights,
    );
}

fn cdxml_apply_line_style_for_double_placement(
    order: u8,
    display: &str,
    display2: &str,
    placement: crate::DoubleBondPlacement,
    line_styles: &mut BondLineStyles,
    line_weights: &mut BondLineWeights,
) {
    if order < 2 {
        return;
    }
    if placement == crate::DoubleBondPlacement::Center {
        *line_styles = cdxml_bond_line_styles(order, display, display2);
        *line_weights = cdxml_bond_line_weights(order, display, display2);
        return;
    }

    *line_styles = BondLineStyles::default();
    *line_weights = BondLineWeights::default();
    if matches!(display, "Dash" | "Hash") {
        line_styles.main = crate::BondLinePattern::Dashed;
    } else if display == "Wavy" {
        line_styles.main = crate::BondLinePattern::Wavy;
    }
    if display == "Bold" {
        line_weights.main = crate::BondLineWeight::Bold;
    }

    let outer_style = match placement {
        crate::DoubleBondPlacement::Left => &mut line_styles.left,
        crate::DoubleBondPlacement::Right => &mut line_styles.right,
        crate::DoubleBondPlacement::Center => unreachable!(),
    };
    if matches!(display2, "Dash" | "Hash") {
        *outer_style = crate::BondLinePattern::Dashed;
    }

    let outer_weight = match placement {
        crate::DoubleBondPlacement::Left => &mut line_weights.left,
        crate::DoubleBondPlacement::Right => &mut line_weights.right,
        crate::DoubleBondPlacement::Center => unreachable!(),
    };
    if display2 == "Bold" {
        *outer_weight = crate::BondLineWeight::Bold;
    }
}

fn cdxml_import_wedge_width(_stroke_width: f64, bold_width: f64) -> f64 {
    (bold_width * crate::WEDGE_BOLD_WIDTH_MULTIPLIER).max(crate::DEFAULT_BOND_STROKE)
}

fn cdxml_bond_order(value: Option<&str>) -> u8 {
    let order = parse_f64(value).unwrap_or(1.0);
    if order >= 2.5 {
        3
    } else if order >= 1.5 {
        2
    } else {
        1
    }
}

fn cdxml_bond_line_styles(order: u8, display: &str, display2: &str) -> BondLineStyles {
    let mut styles = BondLineStyles::default();
    if matches!(display, "Dash" | "Hash") {
        styles.main = crate::BondLinePattern::Dashed;
        if order >= 2 {
            styles.left = crate::BondLinePattern::Dashed;
        }
    } else if display == "Wavy" {
        styles.main = crate::BondLinePattern::Wavy;
    }
    if order >= 2 && matches!(display2, "Dash" | "Hash") {
        styles.right = crate::BondLinePattern::Dashed;
    }
    styles
}

fn cdxml_bond_line_weights(order: u8, display: &str, display2: &str) -> BondLineWeights {
    let mut weights = BondLineWeights::default();
    if display == "Bold" {
        weights.main = crate::BondLineWeight::Bold;
        if order >= 2 {
            weights.left = crate::BondLineWeight::Bold;
        }
    }
    if order >= 2 && display2 == "Bold" {
        weights.right = crate::BondLineWeight::Bold;
    }
    weights
}

fn page_from_objects(objects: &[SceneObject], background: &str) -> Page {
    let mut max_x: f64 = 640.0;
    let mut max_y: f64 = 480.0;
    for object in objects {
        let tx = object.transform.translate[0];
        let ty = object.transform.translate[1];
        if let Some([x, y, w, h]) = object.payload.bbox {
            max_x = max_x.max(tx + x + w);
            max_y = max_y.max(ty + y + h);
        }
        if let Some(points) = object.payload.extra.get("points").and_then(Value::as_array) {
            for point in points {
                if let Some(coords) = point.as_array() {
                    if let (Some(x), Some(y)) = (
                        coords.first().and_then(Value::as_f64),
                        coords.get(1).and_then(Value::as_f64),
                    ) {
                        max_x = max_x.max(tx + x);
                        max_y = max_y.max(ty + y);
                    }
                }
            }
        }
    }
    Page {
        width: round2(max_x + 24.0),
        height: round2(max_y + 24.0),
        background: background.to_string(),
    }
}

fn parse_xy(value: Option<&str>) -> Option<[f64; 2]> {
    let mut parts = value?.split_whitespace();
    Some([parts.next()?.parse().ok()?, parts.next()?.parse().ok()?])
}

fn parse_xyz2(value: Option<&str>) -> Option<[f64; 2]> {
    parse_xy(value)
}

fn parse_bbox(value: Option<&str>) -> Option<[f64; 4]> {
    let nums: Vec<f64> = value?
        .split_whitespace()
        .take(4)
        .filter_map(|part| part.parse().ok())
        .collect();
    (nums.len() == 4).then(|| {
        [
            nums[0].min(nums[2]),
            nums[1].min(nums[3]),
            nums[0].max(nums[2]),
            nums[1].max(nums[3]),
        ]
    })
}

fn parse_f64(value: Option<&str>) -> Option<f64> {
    value?.parse().ok()
}

fn parse_i32(value: Option<&str>) -> Option<i32> {
    value?.parse().ok()
}

fn parse_u8(value: Option<&str>) -> Option<u8> {
    value?.parse().ok()
}

fn parse_u32(value: Option<&str>) -> Option<u32> {
    value?.parse().ok()
}

fn parse_scaled_100(value: Option<&str>) -> Option<f64> {
    parse_f64(value).map(|value| value / 100.0)
}

fn round2(value: f64) -> f64 {
    crate::round2(value)
}

fn has_arrow_attrs(node: &XmlNode) -> bool {
    [
        "ArrowheadHead",
        "ArrowheadTail",
        "ArrowType",
        "ArrowheadType",
    ]
    .into_iter()
    .any(|key| arrow_endpoint_enabled(node.attr(key)))
}

fn arrow_endpoint_enabled(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        let normalized = value.trim().to_ascii_lowercase();
        !normalized.is_empty() && !matches!(normalized.as_str(), "none" | "0" | "false")
    })
}

fn empty_as_null(value: Option<&str>) -> Value {
    match value.filter(|value| !value.is_empty()) {
        Some(value) => json!(value),
        None => Value::Null,
    }
}

fn element_symbol(atomic_number: u8) -> &'static str {
    const SYMBOLS: [&str; 119] = [
        "", "H", "He", "Li", "Be", "B", "C", "N", "O", "F", "Ne", "Na", "Mg", "Al", "Si", "P", "S",
        "Cl", "Ar", "K", "Ca", "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn", "Ga",
        "Ge", "As", "Se", "Br", "Kr", "Rb", "Sr", "Y", "Zr", "Nb", "Mo", "Tc", "Ru", "Rh", "Pd",
        "Ag", "Cd", "In", "Sn", "Sb", "Te", "I", "Xe", "Cs", "Ba", "La", "Ce", "Pr", "Nd", "Pm",
        "Sm", "Eu", "Gd", "Tb", "Dy", "Ho", "Er", "Tm", "Yb", "Lu", "Hf", "Ta", "W", "Re", "Os",
        "Ir", "Pt", "Au", "Hg", "Tl", "Pb", "Bi", "Po", "At", "Rn", "Fr", "Ra", "Ac", "Th", "Pa",
        "U", "Np", "Pu", "Am", "Cm", "Bk", "Cf", "Es", "Fm", "Md", "No", "Lr", "Rf", "Db", "Sg",
        "Bh", "Hs", "Mt", "Ds", "Rg", "Cn", "Nh", "Fl", "Mc", "Lv", "Ts", "Og",
    ];
    SYMBOLS.get(atomic_number as usize).copied().unwrap_or("C")
}
