use crate::{
    Bond, BondLineStyles, BondLineWeights, BondStereo, ChemcoreDocument, DocumentInfo, FormatInfo,
    LabelRun, MoleculeFragment, Node, NodeLabel, ObjectPayload, Page, Point, Resource,
    ResourceData, SceneObject, Transform,
};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

#[derive(Debug, Clone, Default)]
struct XmlNode {
    name: String,
    attrs: BTreeMap<String, String>,
    text: String,
    children: Vec<XmlNode>,
}

#[derive(Debug, Clone, Copy)]
struct CdxmlDefaults {
    bond_length: f64,
    line_width: f64,
    bold_width: f64,
    hash_spacing: f64,
    bond_spacing: f64,
}

impl Default for CdxmlDefaults {
    fn default() -> Self {
        Self {
            bond_length: crate::DEFAULT_BOND_LENGTH,
            line_width: crate::DEFAULT_BOND_STROKE,
            bold_width: crate::BOLD_BOND_WIDTH_CM.value(),
            hash_spacing: crate::DEFAULT_HASH_SPACING_CM.value(),
            bond_spacing: crate::DEFAULT_BOND_SPACING_PERCENT,
        }
    }
}

pub fn parse_cdxml_document(cdxml: &str, title: Option<&str>) -> Result<ChemcoreDocument, String> {
    let root = parse_xml_tree(cdxml)?;
    let defaults = cdxml_defaults(&root);
    let colors = cdxml_color_table(&root);
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
    for (index, fragment) in fragments.iter().enumerate() {
        let Some(bbox) = parse_bbox(fragment.attr("BoundingBox")) else {
            continue;
        };
        let Some(resource) = normalize_fragment(fragment, bbox, defaults, &colors, &fonts) else {
            continue;
        };
        let resource_id = format!("mol_{:03}", index + 1);
        resources.insert(
            resource_id.clone(),
            Resource {
                resource_type: "molecule_fragment2d".to_string(),
                encoding: "chemcore.molecule.fragment2d".to_string(),
                data: ResourceData::Fragment(resource),
                meta: json!({
                    "import": { "cdxml": { "fragmentId": fragment.attr("id") } }
                }),
            },
        );
        objects.push(SceneObject {
            id: format!("obj_mol_{:03}", index + 1),
            object_type: "molecule".to_string(),
            name: format!("molecule {}", index + 1),
            visible: true,
            locked: false,
            z_index: parse_i32(fragment.attr("Z")).unwrap_or(10),
            transform: Transform {
                translate: [round2(bbox[0]), round2(bbox[1])],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: Some("style_molecule_default".to_string()),
            meta: json!({
                "source": "cdxml",
                "fragmentId": fragment.attr("id"),
            }),
            payload: ObjectPayload {
                resource_ref: Some(resource_id),
                bbox: Some([
                    0.0,
                    0.0,
                    round2(bbox[2] - bbox[0]),
                    round2(bbox[3] - bbox[1]),
                ]),
                extra: BTreeMap::new(),
            },
        });
    }
    append_line_objects(&root, &mut objects, &mut styles, defaults, &colors);
    append_shape_objects(&root, &mut objects, &mut styles, &colors);
    append_bracket_objects(&root, &mut objects, defaults);
    append_text_objects(
        &root,
        &mut objects,
        &mut styles,
        &colors,
        &fonts,
        &display_fragment_ids,
        &bonded_node_ids,
    );

    let page = page_from_objects(&objects);
    Ok(ChemcoreDocument {
        format: FormatInfo {
            name: "chemcore".to_string(),
            version: "0.1".to_string(),
            unit: "pt".to_string(),
        },
        document: DocumentInfo {
            id: "doc_cdxml_import".to_string(),
            title: title.unwrap_or("Imported CDXML").to_string(),
            page,
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
                        }
                    }
                },
            }),
        },
        styles,
        objects,
        resources,
    })
}

pub(crate) fn normalize_cdxml_document_for_editing(document: &mut ChemcoreDocument) {
    merge_molecule_objects_for_editing(&mut document.objects, &mut document.resources);
}

fn merge_molecule_objects_for_editing(
    objects: &mut Vec<SceneObject>,
    resources: &mut BTreeMap<String, Resource>,
) {
    let molecule_indices: Vec<usize> = objects
        .iter()
        .enumerate()
        .filter_map(|(index, object)| {
            (object.object_type == "molecule"
                && object
                    .payload
                    .resource_ref
                    .as_ref()
                    .and_then(|resource_ref| resources.get(resource_ref))
                    .and_then(|resource| resource.data.as_fragment())
                    .is_some())
            .then_some(index)
        })
        .collect();
    if molecule_indices.len() <= 1 {
        return;
    }

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut fragments = Vec::new();
    for (fragment_index, object_index) in molecule_indices.iter().copied().enumerate() {
        let object = &objects[object_index];
        let Some(resource_ref) = object.payload.resource_ref.as_ref() else {
            continue;
        };
        let Some(fragment) = resources
            .get(resource_ref)
            .and_then(|resource| resource.data.as_fragment())
            .cloned()
        else {
            continue;
        };
        let bbox = [
            object.transform.translate[0] + fragment.bbox[0],
            object.transform.translate[1] + fragment.bbox[1],
            object.transform.translate[0] + fragment.bbox[2],
            object.transform.translate[1] + fragment.bbox[3],
        ];
        min_x = min_x.min(bbox[0]);
        min_y = min_y.min(bbox[1]);
        max_x = max_x.max(bbox[2]);
        max_y = max_y.max(bbox[3]);
        fragments.push((
            fragment_index + 1,
            object.transform.translate,
            resource_ref.clone(),
            fragment,
        ));
    }
    if fragments.len() <= 1 || !min_x.is_finite() || !min_y.is_finite() {
        return;
    }

    let origin = [round2(min_x), round2(min_y)];
    let mut merged = MoleculeFragment {
        schema: "chemcore.molecule.fragment2d".to_string(),
        bbox: [0.0, 0.0, round2(max_x - min_x), round2(max_y - min_y)],
        nodes: Vec::new(),
        bonds: Vec::new(),
        meta: json!({
            "import": {
                "cdxml": {
                    "mergedFragments": fragments.len(),
                }
            }
        }),
    };

    for (fragment_number, translate, _resource_ref, fragment) in &fragments {
        let prefix = format!("f{fragment_number}_");
        for node in &fragment.nodes {
            let mut node = node.clone();
            let old_id = node.id.clone();
            node.id = format!("{prefix}{old_id}");
            let delta = [translate[0] - origin[0], translate[1] - origin[1]];
            node.position = [
                round2(node.position[0] + delta[0]),
                round2(node.position[1] + delta[1]),
            ];
            if let Some(label) = &mut node.label {
                translate_node_label_for_merge(label, delta);
            }
            merged.nodes.push(node);
        }
        for bond in &fragment.bonds {
            let mut bond = bond.clone();
            bond.id = format!("{prefix}{}", bond.id);
            bond.begin = format!("{prefix}{}", bond.begin);
            bond.end = format!("{prefix}{}", bond.end);
            merged.bonds.push(bond);
        }
    }

    let target_resource = "mol_cdxml_merged".to_string();
    resources.insert(
        target_resource.clone(),
        Resource {
            resource_type: "molecule_fragment2d".to_string(),
            encoding: "chemcore.molecule.fragment2d".to_string(),
            data: ResourceData::Fragment(merged),
            meta: json!({
                "import": {
                    "cdxml": {
                        "merged": true,
                    }
                }
            }),
        },
    );
    for (_, _, resource_ref, _) in &fragments {
        resources.remove(resource_ref);
    }

    let mut first = true;
    let molecule_index_set: BTreeSet<usize> = molecule_indices.into_iter().collect();
    let mut next_objects = Vec::with_capacity(objects.len());
    for (index, object) in objects.drain(..).enumerate() {
        if !molecule_index_set.contains(&index) {
            next_objects.push(object);
            continue;
        }
        if first {
            first = false;
            next_objects.push(SceneObject {
                id: "obj_cdxml_merged_molecule".to_string(),
                object_type: "molecule".to_string(),
                name: "molecule".to_string(),
                visible: true,
                locked: false,
                z_index: object.z_index,
                transform: Transform {
                    translate: origin,
                    rotate: 0.0,
                    scale: [1.0, 1.0],
                },
                style_ref: object.style_ref,
                meta: json!({
                    "source": "cdxml",
                    "mergedFragments": true,
                }),
                payload: ObjectPayload {
                    resource_ref: Some(target_resource.clone()),
                    bbox: Some([0.0, 0.0, round2(max_x - min_x), round2(max_y - min_y)]),
                    extra: BTreeMap::new(),
                },
            });
        }
    }
    *objects = next_objects;
}

fn translate_node_label_for_merge(label: &mut NodeLabel, delta: [f64; 2]) {
    if let Some(position) = &mut label.position {
        position[0] = round2(position[0] + delta[0]);
        position[1] = round2(position[1] + delta[1]);
    }
    if let Some(bbox) = &mut label.box_field {
        translate_bbox_in_place(bbox, delta);
    }
    if let Some(bbox) = &mut label.box_value {
        translate_bbox_in_place(bbox, delta);
    }
    for polygon in &mut label.glyph_polygons {
        for point in polygon {
            point[0] = round2(point[0] + delta[0]);
            point[1] = round2(point[1] + delta[1]);
        }
    }
}

fn translate_bbox_in_place(bbox: &mut [f64; 4], delta: [f64; 2]) {
    bbox[0] = round2(bbox[0] + delta[0]);
    bbox[1] = round2(bbox[1] + delta[1]);
    bbox[2] = round2(bbox[2] + delta[0]);
    bbox[3] = round2(bbox[3] + delta[1]);
}

pub fn document_to_cdxml(document: &ChemcoreDocument) -> String {
    CdxmlDocumentWriter::new(document).write()
}

fn export_cdxml_defaults(document: &ChemcoreDocument) -> CdxmlDefaults {
    let mut defaults = CdxmlDefaults::default();
    if let Some(import_defaults) = document
        .document
        .meta
        .get("import")
        .and_then(|value| value.get("cdxml"))
        .and_then(|value| value.get("defaults"))
    {
        if let Some(value) = import_defaults.get("bondLength").and_then(Value::as_f64) {
            defaults.bond_length = value;
        }
        if let Some(value) = import_defaults.get("lineWidth").and_then(Value::as_f64) {
            defaults.line_width = value;
        }
        if let Some(value) = import_defaults.get("boldWidth").and_then(Value::as_f64) {
            defaults.bold_width = value;
        }
        if let Some(value) = import_defaults.get("hashSpacing").and_then(Value::as_f64) {
            defaults.hash_spacing = value;
        }
        if let Some(value) = import_defaults.get("bondSpacing").and_then(Value::as_f64) {
            defaults.bond_spacing = value;
        }
    }
    if let Some(style) = document.styles.get("style_molecule_default") {
        if let Some(value) = style_number_value(style, "strokeWidth") {
            defaults.line_width = value;
        }
    }
    for resource in document.resources.values() {
        let ResourceData::Fragment(fragment) = &resource.data else {
            continue;
        };
        if let Some(bond) = fragment.bonds.first() {
            defaults.line_width = bond.stroke_width;
            if let Some(value) = bond.bold_width {
                defaults.bold_width = value;
            }
            if let Some(value) = bond.hash_spacing {
                defaults.hash_spacing = value;
            }
            break;
        }
    }
    if let Some(value) = document.objects.iter().find_map(|object| {
        (object.object_type == "symbol")
            .then(|| {
                object
                    .payload
                    .extra
                    .get("symbolLineWidth")
                    .and_then(Value::as_f64)
            })
            .flatten()
    }) {
        defaults.line_width = value;
    }
    defaults
}

struct CdxmlDocumentWriter<'a> {
    document: &'a ChemcoreDocument,
    next_id: u64,
    colors: CdxmlColorTable,
    defaults: CdxmlDefaults,
}

impl<'a> CdxmlDocumentWriter<'a> {
    fn new(document: &'a ChemcoreDocument) -> Self {
        let mut colors = CdxmlColorTable::default();
        collect_document_colors(document, &mut colors);
        Self {
            document,
            next_id: 1,
            colors,
            defaults: export_cdxml_defaults(document),
        }
    }

    fn write(mut self) -> String {
        let page = &self.document.document.page;
        let width = page.width.max(1.0);
        let height = page.height.max(1.0);
        let root_bbox = format!("0 0 {} {}", fmt_num(width), fmt_num(height));
        let mut out = String::new();
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n");
        out.push_str("<!DOCTYPE CDXML SYSTEM \"http://www.cambridgesoft.com/xml/cdxml.dtd\" >\n");
        write!(
            out,
            "<CDXML Name=\"{}\" BoundingBox=\"{}\" WindowPosition=\"0 0\" WindowSize=\"{} {}\" FractionalWidths=\"yes\" InterpretChemically=\"yes\" ShowAtomQuery=\"yes\" ShowBondQuery=\"yes\" LabelFont=\"3\" LabelSize=\"10\" CaptionFont=\"3\" CaptionSize=\"10\" LineWidth=\"{}\" BoldWidth=\"{}\" BondLength=\"{}\" BondSpacing=\"{}\" HashSpacing=\"{}\" color=\"{}\" bgcolor=\"{}\">\n",
            xml_escape_attr(&self.document.document.title),
            root_bbox,
            fmt_num(width),
            fmt_num(height),
            fmt_num(self.defaults.line_width),
            fmt_num(self.defaults.bold_width),
            fmt_num(self.defaults.bond_length),
            fmt_num(self.defaults.bond_spacing),
            fmt_num(self.defaults.hash_spacing),
            self.colors.id_for("#000000"),
            self.colors.id_for(&page.background),
        )
        .expect("writing CDXML root should not fail");
        self.write_color_table(&mut out);
        out.push_str("  <fonttable>\n");
        out.push_str("    <font id=\"3\" charset=\"iso-8859-1\" name=\"Arial\"/>\n");
        out.push_str("  </fonttable>\n");
        write!(
            out,
            "  <page id=\"{}\" BoundingBox=\"{}\" Width=\"{}\" Height=\"{}\">\n",
            self.alloc_id(),
            root_bbox,
            fmt_num(width),
            fmt_num(height)
        )
        .expect("writing CDXML page should not fail");

        let mut objects: Vec<&SceneObject> = self
            .document
            .objects
            .iter()
            .filter(|object| object.visible)
            .collect();
        objects.sort_by(|a, b| a.z_index.cmp(&b.z_index).then_with(|| a.id.cmp(&b.id)));
        for object in objects {
            match object.object_type.as_str() {
                "molecule" => self.write_molecule_object(&mut out, object),
                "line" => self.write_line_object(&mut out, object),
                "shape" => self.write_shape_object(&mut out, object),
                "bracket" | "symbol" => self.write_bracket_object(&mut out, object),
                "text" => self.write_text_object(&mut out, object),
                _ => {}
            }
        }

        out.push_str("  </page>\n");
        out.push_str("</CDXML>\n");
        out
    }

    fn write_color_table(&self, out: &mut String) {
        out.push_str("  <colortable>\n");
        for color in self.colors.colors() {
            let (r, g, b) = rgb_fractions(color);
            writeln!(
                out,
                "    <color r=\"{}\" g=\"{}\" b=\"{}\"/>",
                fmt_num(r),
                fmt_num(g),
                fmt_num(b)
            )
            .expect("writing color table should not fail");
        }
        out.push_str("  </colortable>\n");
    }

    fn write_molecule_object(&mut self, out: &mut String, object: &SceneObject) {
        let Some(fragment) = object
            .payload
            .resource_ref
            .as_ref()
            .and_then(|resource_ref| self.document.resources.get(resource_ref))
            .and_then(|resource| resource.data.as_fragment())
        else {
            return;
        };
        if fragment.nodes.is_empty() {
            return;
        }

        let fragment_id = self.alloc_id();
        let bbox = molecule_world_bbox(object, fragment).unwrap_or([
            object.transform.translate[0],
            object.transform.translate[1],
            object.transform.translate[0] + 1.0,
            object.transform.translate[1] + 1.0,
        ]);
        writeln!(
            out,
            "    <fragment id=\"{}\" BoundingBox=\"{}\" Z=\"{}\">",
            fragment_id,
            fmt_bbox(bbox),
            object.z_index
        )
        .expect("writing fragment should not fail");

        let mut node_ids = BTreeMap::new();
        for node in &fragment.nodes {
            node_ids.insert(node.id.clone(), self.alloc_id());
        }
        for node in &fragment.nodes {
            self.write_node(out, object, node, &node_ids[&node.id]);
        }
        for bond in &fragment.bonds {
            self.write_bond(out, bond, &node_ids);
        }
        out.push_str("    </fragment>\n");
    }

    fn write_node(&mut self, out: &mut String, object: &SceneObject, node: &Node, cdxml_id: &str) {
        let point = object_local_point(object, node.position);
        let label_text = node
            .label
            .as_ref()
            .and_then(|label| {
                label
                    .source_text
                    .as_ref()
                    .or(Some(&label.text))
                    .filter(|text| !text.trim().is_empty())
            })
            .cloned();
        let is_plain_carbon =
            node.atomic_number == 6 && label_text.is_none() && !node.is_placeholder;
        let is_nickname =
            node.is_placeholder || should_export_node_as_nickname(node, label_text.as_deref());
        let mut attrs = vec![("id", cdxml_id.to_string()), ("p", fmt_point(point))];
        if !is_plain_carbon && !is_nickname && node.atomic_number > 0 {
            attrs.push(("Element", node.atomic_number.to_string()));
        }
        if node.is_external_connection_point {
            attrs.push(("NodeType", "ExternalConnectionPoint".to_string()));
        } else if is_nickname {
            attrs.push(("NodeType", "Nickname".to_string()));
        }
        if node.charge != 0 {
            attrs.push(("Charge", node.charge.to_string()));
        }
        if node.num_hydrogens > 0 {
            attrs.push(("NumHydrogens", node.num_hydrogens.to_string()));
        }
        if let Some(label) = node.label.as_ref().filter(|label| label.has_visible_text()) {
            write_open_tag(out, 6, "n", attrs);
            self.write_node_label(out, object, node, label);
            out.push_str("      </n>\n");
        } else {
            write_empty_tag(out, 6, "n", attrs);
        }
    }

    fn write_node_label(
        &mut self,
        out: &mut String,
        object: &SceneObject,
        node: &Node,
        label: &NodeLabel,
    ) {
        let text = label.source_text.as_deref().unwrap_or(&label.text);
        let font_size = label.font_size.unwrap_or(10.0);
        let color_id = self
            .colors
            .id_for(label.fill.as_deref().unwrap_or("#000000"));
        let position = label
            .position
            .map(|position| object_local_point(object, position))
            .unwrap_or_else(|| object_local_point(object, node.position));
        let bbox = label
            .bbox()
            .map(|bbox| translate_bbox(bbox, object.transform.translate))
            .unwrap_or_else(|| text_bbox_estimate(position, text, font_size));
        let attrs = vec![
            ("id", self.alloc_id()),
            ("p", fmt_point(position)),
            ("BoundingBox", fmt_bbox(bbox)),
            ("LabelAlignment", "Auto".to_string()),
            (
                "Justification",
                cdxml_justification(label.align.as_deref()).to_string(),
            ),
            ("font", "3".to_string()),
            ("size", fmt_num(font_size)),
            ("color", color_id),
            ("UTF8Text", text.to_string()),
        ];
        write_open_tag(out, 8, "t", attrs);
        self.write_label_runs(out, 10, label, text, font_size);
        out.push_str("        </t>\n");
    }

    fn write_bond(&mut self, out: &mut String, bond: &Bond, node_ids: &BTreeMap<String, String>) {
        let (Some(begin), Some(end)) = (node_ids.get(&bond.begin), node_ids.get(&bond.end)) else {
            return;
        };
        let mut attrs = vec![
            ("id", self.alloc_id()),
            ("B", begin.clone()),
            ("E", end.clone()),
            ("Order", bond.order.max(1).to_string()),
        ];
        if let Some(display) = cdxml_bond_display(bond, false) {
            attrs.push(("Display", display.to_string()));
        }
        if let Some(display2) = cdxml_bond_display(bond, true) {
            attrs.push(("Display2", display2.to_string()));
        }
        if let Some(double) = &bond.double {
            attrs.push((
                "DoublePosition",
                match double.placement {
                    crate::DoubleBondPlacement::Left => "Left",
                    crate::DoubleBondPlacement::Right => "Right",
                    crate::DoubleBondPlacement::Center => "Center",
                }
                .to_string(),
            ));
        }
        if bond.stroke_width > 0.0 {
            attrs.push(("LineWidth", fmt_num(bond.stroke_width)));
        }
        if let Some(value) = bond.bold_width {
            attrs.push(("BoldWidth", fmt_num(value)));
        }
        if let Some(value) = bond.hash_spacing {
            attrs.push(("HashSpacing", fmt_num(value)));
        }
        if let Some(value) = bond.bond_spacing {
            attrs.push(("BondSpacing", fmt_num(value)));
        }
        write_empty_tag(out, 6, "b", attrs);
    }

    fn write_line_object(&mut self, out: &mut String, object: &SceneObject) {
        let points = payload_points_cdxml(&object.payload, "points");
        if points.len() < 2 {
            return;
        }
        let tail = points[0].translated(crate::Vector::new(
            object.transform.translate[0],
            object.transform.translate[1],
        ));
        let head = points[points.len() - 1].translated(crate::Vector::new(
            object.transform.translate[0],
            object.transform.translate[1],
        ));
        let arrow = object.payload.extra.get("arrowHead");
        let has_head = payload_string_cdxml(&object.payload, "head").as_deref() == Some("end")
            || arrow
                .and_then(|value| value.get("head"))
                .and_then(Value::as_str)
                .is_some_and(|value| !value.eq_ignore_ascii_case("none"));
        let has_tail = payload_string_cdxml(&object.payload, "tail").as_deref() == Some("start")
            || arrow
                .and_then(|value| value.get("tail"))
                .and_then(Value::as_str)
                .is_some_and(|value| !value.eq_ignore_ascii_case("none"));
        let style = object_style(self.document, object);
        let stroke = style
            .and_then(|style| style_string_value(style, "stroke"))
            .unwrap_or_else(|| "#000000".to_string());
        let stroke_width = style
            .and_then(|style| style_number_value(style, "strokeWidth"))
            .unwrap_or(crate::DEFAULT_BOND_STROKE);
        let mut attrs = vec![
            ("id", self.alloc_id()),
            ("Head3D", fmt_point3(head)),
            ("Tail3D", fmt_point3(tail)),
            ("LineWidth", fmt_num(stroke_width)),
            ("color", self.colors.id_for(&stroke)),
            ("Z", object.z_index.to_string()),
        ];
        if has_head || has_tail {
            attrs.push((
                "ArrowheadHead",
                if has_head { "Full" } else { "None" }.to_string(),
            ));
            attrs.push((
                "ArrowheadTail",
                if has_tail { "Full" } else { "None" }.to_string(),
            ));
            attrs.push(("ArrowheadType", cdxml_arrow_kind(arrow).to_string()));
            if let Some(value) = arrow
                .and_then(|value| value.get("length"))
                .and_then(Value::as_f64)
            {
                attrs.push(("HeadSize", fmt_num(value * 100.0)));
            }
            write_empty_tag(out, 4, "arrow", attrs);
        } else {
            attrs.push(("GraphicType", "Line".to_string()));
            write_empty_tag(out, 4, "graphic", attrs);
        }
    }

    fn write_shape_object(&mut self, out: &mut String, object: &SceneObject) {
        let Some([x, y, width, height]) = object.payload.bbox else {
            return;
        };
        let kind =
            payload_string_cdxml(&object.payload, "kind").unwrap_or_else(|| "rect".to_string());
        let style = object_style(self.document, object);
        let stroke = style.and_then(|style| style_nullable_string_value(style, "stroke"));
        let fill = style.and_then(|style| style_nullable_string_value(style, "fill"));
        let color = fill.as_deref().or(stroke.as_deref()).unwrap_or("#000000");
        let filled = fill.is_some();
        if matches!(kind.as_str(), "circle" | "ellipse") {
            let center = payload_point_cdxml(&object.payload, "center").unwrap_or_else(|| {
                Point::new(
                    object.transform.translate[0] + x + width * 0.5,
                    object.transform.translate[1] + y + height * 0.5,
                )
            });
            let major = payload_point_cdxml(&object.payload, "majorAxisEnd")
                .unwrap_or_else(|| Point::new(center.x + width * 0.5, center.y));
            let minor = payload_point_cdxml(&object.payload, "minorAxisEnd")
                .unwrap_or_else(|| Point::new(center.x, center.y + height * 0.5));
            let bbox = [
                object.transform.translate[0] + x,
                object.transform.translate[1] + y,
                object.transform.translate[0] + x + width,
                object.transform.translate[1] + y + height,
            ];
            let mut attrs = vec![
                ("id", self.alloc_id()),
                ("GraphicType", "Oval".to_string()),
                ("BoundingBox", fmt_bbox(bbox)),
                ("Center3D", fmt_point3(center)),
                ("MajorAxisEnd3D", fmt_point3(major)),
                ("MinorAxisEnd3D", fmt_point3(minor)),
                (
                    "OvalType",
                    if kind == "circle" {
                        "Circle"
                    } else if filled {
                        "Filled"
                    } else {
                        ""
                    }
                    .to_string(),
                ),
                ("color", self.colors.id_for(color)),
                ("Z", object.z_index.to_string()),
            ];
            if let Some(stroke_width) =
                style.and_then(|style| style_number_value(style, "strokeWidth"))
            {
                attrs.push(("LineWidth", fmt_num(stroke_width)));
            }
            write_empty_tag(out, 4, "graphic", attrs);
            return;
        }
        let bbox = [
            object.transform.translate[0] + x,
            object.transform.translate[1] + y,
            object.transform.translate[0] + x + width,
            object.transform.translate[1] + y + height,
        ];
        let mut rectangle_type = String::new();
        if kind == "roundRect" {
            rectangle_type.push_str("RoundEdge");
        }
        if filled {
            if !rectangle_type.is_empty() {
                rectangle_type.push(' ');
            }
            rectangle_type.push_str("Filled");
        }
        let mut attrs = vec![
            ("id", self.alloc_id()),
            ("GraphicType", "Rectangle".to_string()),
            ("BoundingBox", fmt_bbox(bbox)),
            ("RectangleType", rectangle_type),
            ("color", self.colors.id_for(color)),
            ("Z", object.z_index.to_string()),
        ];
        if let Some(radius) = object
            .payload
            .extra
            .get("cornerRadius")
            .and_then(Value::as_f64)
        {
            attrs.push(("CornerRadius", fmt_num(radius * 100.0)));
        }
        if let Some(stroke_width) = style.and_then(|style| style_number_value(style, "strokeWidth"))
        {
            attrs.push(("LineWidth", fmt_num(stroke_width)));
        }
        write_empty_tag(out, 4, "graphic", attrs);
    }

    fn write_bracket_object(&mut self, out: &mut String, object: &SceneObject) {
        let Some([x, y, width, height]) = object.payload.bbox else {
            return;
        };
        let bbox = [
            object.transform.translate[0] + x,
            object.transform.translate[1] + y,
            object.transform.translate[0] + x + width,
            object.transform.translate[1] + y + height,
        ];
        let kind =
            payload_string_cdxml(&object.payload, "kind").unwrap_or_else(|| "round".to_string());
        if object.object_type == "symbol" {
            let symbol_type = match kind.as_str() {
                "double-dagger" => "DoubleDagger",
                "dagger" => "Dagger",
                "circle-plus" => "CirclePlus",
                "plus" => "Plus",
                "radical-cation" => "RadicalCation",
                "lone-pair" => "LonePair",
                "circle-minus" => "CircleMinus",
                "minus" => "Minus",
                "radical-anion" => "RadicalAnion",
                "electron" => "Electron",
                _ => "Dagger",
            };
            let style = object
                .payload
                .extra
                .get("symbolStyle")
                .and_then(Value::as_str)
                .map(crate::cdxml_symbol_style_from_name)
                .unwrap_or(crate::CdxmlSymbolStyle::Default);
            let anchor_width = object
                .payload
                .extra
                .get("symbolAnchorWidth")
                .and_then(Value::as_f64)
                .unwrap_or_else(|| crate::cdxml_symbol_anchor_width(&kind, style));
            let anchor_height = object
                .payload
                .extra
                .get("symbolAnchorHeight")
                .and_then(Value::as_f64)
                .unwrap_or_else(|| crate::cdxml_symbol_anchor_height(&kind));
            let center_x = (bbox[0] + bbox[2]) * 0.5;
            let center_y = (bbox[1] + bbox[3]) * 0.5;
            write_empty_tag(
                out,
                4,
                "graphic",
                vec![
                    ("id", self.alloc_id()),
                    ("GraphicType", "Symbol".to_string()),
                    ("SymbolType", symbol_type.to_string()),
                    (
                        "BoundingBox",
                        fmt_bbox([
                            center_x - anchor_width * 0.5,
                            center_y - anchor_height * 0.5,
                            center_x + anchor_width * 0.5,
                            center_y + anchor_height * 0.5,
                        ]),
                    ),
                    ("Z", object.z_index.to_string()),
                ],
            );
            return;
        }

        let bracket_type = match kind.as_str() {
            "square" => "Square",
            "curly" => "Curly",
            _ => "Round",
        };
        let left_x = bbox[0];
        let right_x = bbox[2];
        let top = bbox[1];
        let bottom = bbox[3];
        write_empty_tag(
            out,
            4,
            "graphic",
            vec![
                ("id", self.alloc_id()),
                ("GraphicType", "Bracket".to_string()),
                ("BracketType", bracket_type.to_string()),
                ("BoundingBox", fmt_bbox([left_x, bottom, left_x, top])),
                ("LipSize", "60".to_string()),
                ("Z", object.z_index.to_string()),
            ],
        );
        write_empty_tag(
            out,
            4,
            "graphic",
            vec![
                ("id", self.alloc_id()),
                ("GraphicType", "Bracket".to_string()),
                ("BracketType", bracket_type.to_string()),
                ("BoundingBox", fmt_bbox([right_x, top, right_x, bottom])),
                ("LipSize", "60".to_string()),
                ("Z", (object.z_index + 1).to_string()),
            ],
        );
    }

    fn write_text_object(&mut self, out: &mut String, object: &SceneObject) {
        let text = payload_string_cdxml(&object.payload, "text").unwrap_or_default();
        if text.trim().is_empty() {
            return;
        }
        let style = object_style(self.document, object);
        let font_size = object
            .payload
            .extra
            .get("fontSize")
            .and_then(Value::as_f64)
            .or_else(|| style.and_then(|style| style_number_value(style, "fontSize")))
            .unwrap_or(10.0);
        let color = style
            .and_then(|style| style_nullable_string_value(style, "fill"))
            .unwrap_or_else(|| "#000000".to_string());
        let box_value = payload_bbox_cdxml(&object.payload, "box")
            .or(object.payload.bbox)
            .unwrap_or([
                0.0,
                0.0,
                (text.len() as f64 * font_size * 0.55).max(12.0),
                font_size * 1.4,
            ]);
        let anchor = Point::new(object.transform.translate[0], object.transform.translate[1]);
        let bbox = [
            object.transform.translate[0] + box_value[0],
            object.transform.translate[1] + box_value[1],
            object.transform.translate[0] + box_value[0] + box_value[2],
            object.transform.translate[1] + box_value[1] + box_value[3],
        ];
        let attrs = vec![
            ("id", self.alloc_id()),
            ("p", fmt_point(anchor)),
            ("BoundingBox", fmt_bbox(bbox)),
            (
                "Justification",
                cdxml_justification(payload_string_cdxml(&object.payload, "align").as_deref())
                    .to_string(),
            ),
            ("font", "3".to_string()),
            ("size", fmt_num(font_size)),
            ("color", self.colors.id_for(&color)),
            ("Z", object.z_index.to_string()),
            ("UTF8Text", text.clone()),
        ];
        write_open_tag(out, 4, "t", attrs);
        let runs = object
            .payload
            .extra
            .get("runs")
            .cloned()
            .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
            .unwrap_or_default();
        self.write_runs(out, 6, &runs, &text, font_size, &color);
        out.push_str("    </t>\n");
    }

    fn write_label_runs(
        &mut self,
        out: &mut String,
        indent: usize,
        label: &NodeLabel,
        fallback: &str,
        fallback_size: f64,
    ) {
        self.write_runs(
            out,
            indent,
            &label.runs,
            fallback,
            fallback_size,
            label.fill.as_deref().unwrap_or("#000000"),
        );
    }

    fn write_runs(
        &mut self,
        out: &mut String,
        indent: usize,
        runs: &[LabelRun],
        fallback: &str,
        fallback_size: f64,
        fallback_color: &str,
    ) {
        if runs.is_empty() {
            let attrs = vec![
                ("font", "3".to_string()),
                ("size", fmt_num(fallback_size)),
                ("color", self.colors.id_for(fallback_color)),
            ];
            write_text_tag(out, indent, "s", attrs, fallback);
            return;
        }
        for run in runs {
            if run.text.is_empty() {
                continue;
            }
            let mut face = 0;
            if run.font_weight.unwrap_or(400) >= 600 {
                face |= 1;
            }
            if run.font_style.as_deref() == Some("italic") {
                face |= 2;
            }
            match run.script.as_deref() {
                Some("subscript") => face |= 32,
                Some("superscript") => face |= 64,
                _ => {}
            }
            let mut attrs = vec![
                ("font", "3".to_string()),
                ("size", fmt_num(run.font_size.unwrap_or(fallback_size))),
                (
                    "color",
                    self.colors.id_for(run.fill.as_deref().unwrap_or("#000000")),
                ),
            ];
            if face != 0 {
                attrs.push(("face", face.to_string()));
            }
            write_text_tag(out, indent, "s", attrs, &run.text);
        }
    }

    fn alloc_id(&mut self) -> String {
        let id = self.next_id;
        self.next_id += 1;
        id.to_string()
    }
}

#[derive(Debug, Clone)]
struct CdxmlColorTable {
    colors: Vec<String>,
    ids: BTreeMap<String, String>,
}

impl Default for CdxmlColorTable {
    fn default() -> Self {
        let mut table = Self {
            colors: Vec::new(),
            ids: BTreeMap::new(),
        };
        table.ensure("#ffffff");
        table.ensure("#000000");
        table
    }
}

impl CdxmlColorTable {
    fn ensure(&mut self, color: &str) -> String {
        let normalized = normalize_hex_color(color).unwrap_or_else(|| "#000000".to_string());
        if let Some(id) = self.ids.get(&normalized) {
            return id.clone();
        }
        let id = (self.colors.len() + 1).to_string();
        self.colors.push(normalized.clone());
        self.ids.insert(normalized, id.clone());
        id
    }

    fn id_for(&self, color: &str) -> String {
        let normalized = normalize_hex_color(color).unwrap_or_else(|| "#000000".to_string());
        self.ids.get(&normalized).cloned().unwrap_or_else(|| {
            self.ids
                .get("#000000")
                .cloned()
                .unwrap_or_else(|| "2".to_string())
        })
    }

    fn colors(&self) -> &[String] {
        &self.colors
    }
}

fn collect_document_colors(document: &ChemcoreDocument, colors: &mut CdxmlColorTable) {
    colors.ensure(&document.document.page.background);
    for style in document.styles.values() {
        for key in ["stroke", "fill", "color", "background", "backgroundColor"] {
            if let Some(color) = style_nullable_string_value(style, key) {
                colors.ensure(&color);
            }
        }
    }
    for object in &document.objects {
        if let Some(style) = object_style(document, object) {
            for key in ["stroke", "fill", "color"] {
                if let Some(color) = style_nullable_string_value(style, key) {
                    colors.ensure(&color);
                }
            }
        }
        if object.object_type == "text" {
            if let Some(runs) = object
                .payload
                .extra
                .get("runs")
                .cloned()
                .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
            {
                for run in runs {
                    if let Some(fill) = run.fill {
                        colors.ensure(&fill);
                    }
                }
            }
        }
    }
    for resource in document.resources.values() {
        let Some(fragment) = resource.data.as_fragment() else {
            continue;
        };
        for node in &fragment.nodes {
            let Some(label) = &node.label else {
                continue;
            };
            if let Some(fill) = &label.fill {
                colors.ensure(fill);
            }
            for run in &label.runs {
                if let Some(fill) = &run.fill {
                    colors.ensure(fill);
                }
            }
        }
    }
}

fn object_style<'a>(document: &'a ChemcoreDocument, object: &SceneObject) -> Option<&'a Value> {
    object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
}

fn style_string_value(style: &Value, key: &str) -> Option<String> {
    style.get(key)?.as_str().map(ToString::to_string)
}

fn style_nullable_string_value(style: &Value, key: &str) -> Option<String> {
    let value = style.get(key)?;
    if value.is_null() {
        return None;
    }
    value.as_str().map(ToString::to_string)
}

fn style_number_value(style: &Value, key: &str) -> Option<f64> {
    style.get(key)?.as_f64()
}

fn payload_string_cdxml(payload: &ObjectPayload, key: &str) -> Option<String> {
    payload.extra.get(key)?.as_str().map(ToString::to_string)
}

fn payload_point_cdxml(payload: &ObjectPayload, key: &str) -> Option<Point> {
    let coords = payload.extra.get(key)?.as_array()?;
    Some(Point::new(
        coords.first()?.as_f64()?,
        coords.get(1)?.as_f64()?,
    ))
}

fn payload_points_cdxml(payload: &ObjectPayload, key: &str) -> Vec<Point> {
    payload
        .extra
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|point| {
            let coords = point.as_array()?;
            Some(Point::new(
                coords.first()?.as_f64()?,
                coords.get(1)?.as_f64()?,
            ))
        })
        .collect()
}

fn payload_bbox_cdxml(payload: &ObjectPayload, key: &str) -> Option<[f64; 4]> {
    let coords = payload.extra.get(key)?.as_array()?;
    Some([
        coords.first()?.as_f64()?,
        coords.get(1)?.as_f64()?,
        coords.get(2)?.as_f64()?,
        coords.get(3)?.as_f64()?,
    ])
}

fn object_local_point(object: &SceneObject, point: [f64; 2]) -> Point {
    Point::new(
        object.transform.translate[0] + point[0],
        object.transform.translate[1] + point[1],
    )
}

fn molecule_world_bbox(object: &SceneObject, fragment: &MoleculeFragment) -> Option<[f64; 4]> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut found = false;
    if let Some([x, y, width, height]) = object.payload.bbox {
        min_x = min_x.min(object.transform.translate[0] + x);
        min_y = min_y.min(object.transform.translate[1] + y);
        max_x = max_x.max(object.transform.translate[0] + x + width);
        max_y = max_y.max(object.transform.translate[1] + y + height);
        found = true;
    }
    if !found {
        for node in &fragment.nodes {
            let point = object_local_point(object, node.position);
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
            found = true;
            if let Some(label) = &node.label {
                if let Some(bbox) = label.bbox() {
                    let bbox = translate_bbox(bbox, object.transform.translate);
                    min_x = min_x.min(bbox[0]);
                    min_y = min_y.min(bbox[1]);
                    max_x = max_x.max(bbox[2]);
                    max_y = max_y.max(bbox[3]);
                }
            }
        }
        let pad = 12.0;
        min_x -= pad;
        min_y -= pad;
        max_x += pad;
        max_y += pad;
    }
    found.then_some([min_x, min_y, max_x, max_y])
}

fn translate_bbox(bbox: [f64; 4], translate: [f64; 2]) -> [f64; 4] {
    [
        bbox[0] + translate[0],
        bbox[1] + translate[1],
        bbox[2] + translate[0],
        bbox[3] + translate[1],
    ]
}

fn text_bbox_estimate(position: Point, text: &str, font_size: f64) -> [f64; 4] {
    let line_count = text.lines().count().max(1) as f64;
    let max_chars = text
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(1) as f64;
    let width = (max_chars * font_size * 0.58).max(font_size);
    let height = (line_count * font_size * 1.25).max(font_size);
    [
        position.x,
        position.y - font_size,
        position.x + width,
        position.y - font_size + height,
    ]
}

fn should_export_node_as_nickname(node: &Node, label_text: Option<&str>) -> bool {
    let Some(text) = label_text else {
        return false;
    };
    let trimmed = text.trim();
    !trimmed.is_empty()
        && !(node.atomic_number > 0 && trimmed == element_symbol(node.atomic_number))
        && !(node.atomic_number == 6 && trimmed == "C")
}

fn cdxml_bond_display(bond: &Bond, second: bool) -> Option<&'static str> {
    if let Some(stereo) = &bond.stereo {
        if stereo.kind == "solid-wedge" {
            return Some(if stereo.wide_end == "end" {
                "WedgeBegin"
            } else {
                "WedgeEnd"
            });
        }
        if stereo.kind == "hashed-wedge" {
            return Some(if stereo.wide_end == "end" {
                "WedgedHashBegin"
            } else {
                "WedgedHashEnd"
            });
        }
    }
    if second {
        if bond.line_styles.right == crate::BondLinePattern::Dashed {
            return Some("Dash");
        }
        if bond.line_weights.right == crate::BondLineWeight::Bold {
            return Some("Bold");
        }
        return None;
    }
    if bond.line_styles.main == crate::BondLinePattern::Dashed
        || bond.line_styles.left == crate::BondLinePattern::Dashed
    {
        return Some("Dash");
    }
    if bond.line_weights.main == crate::BondLineWeight::Bold
        || bond.line_weights.left == crate::BondLineWeight::Bold
    {
        return Some("Bold");
    }
    None
}

fn cdxml_arrow_kind(value: Option<&Value>) -> &'static str {
    match value
        .and_then(|value| value.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or("solid")
        .to_ascii_lowercase()
        .as_str()
    {
        "hollow" => "Hollow",
        "open" | "angle" | "retrosynthetic" => "Angle",
        _ => "Solid",
    }
}

fn cdxml_justification(value: Option<&str>) -> &'static str {
    match value.unwrap_or("").to_ascii_lowercase().as_str() {
        "center" | "middle" => "Center",
        "right" | "end" => "Right",
        _ => "Left",
    }
}

fn normalize_hex_color(color: &str) -> Option<String> {
    let color = color.trim();
    if !color.starts_with('#') {
        return None;
    }
    let hex = &color[1..];
    if hex.len() == 3 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        let mut out = String::from("#");
        for ch in hex.chars() {
            out.push(ch.to_ascii_lowercase());
            out.push(ch.to_ascii_lowercase());
        }
        return Some(out);
    }
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(format!("#{}", hex.to_ascii_lowercase()));
    }
    None
}

fn rgb_fractions(color: &str) -> (f64, f64, f64) {
    let normalized = normalize_hex_color(color).unwrap_or_else(|| "#000000".to_string());
    let r = u8::from_str_radix(&normalized[1..3], 16).unwrap_or(0) as f64 / 255.0;
    let g = u8::from_str_radix(&normalized[3..5], 16).unwrap_or(0) as f64 / 255.0;
    let b = u8::from_str_radix(&normalized[5..7], 16).unwrap_or(0) as f64 / 255.0;
    (r, g, b)
}

fn fmt_num(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let rounded = (value * 1000.0).round() / 1000.0;
    let mut out = format!("{rounded:.3}");
    while out.contains('.') && out.ends_with('0') {
        out.pop();
    }
    if out.ends_with('.') {
        out.pop();
    }
    if out == "-0" {
        "0".to_string()
    } else {
        out
    }
}

fn fmt_point(point: Point) -> String {
    format!("{} {}", fmt_num(point.x), fmt_num(point.y))
}

fn fmt_point3(point: Point) -> String {
    format!("{} {} 0", fmt_num(point.x), fmt_num(point.y))
}

fn fmt_bbox(bbox: [f64; 4]) -> String {
    format!(
        "{} {} {} {}",
        fmt_num(bbox[0]),
        fmt_num(bbox[1]),
        fmt_num(bbox[2]),
        fmt_num(bbox[3])
    )
}

fn write_open_tag(out: &mut String, indent: usize, name: &str, attrs: Vec<(&'static str, String)>) {
    write_indent(out, indent);
    write!(out, "<{name}").expect("writing tag should not fail");
    for (key, value) in attrs {
        write!(out, " {key}=\"{}\"", xml_escape_attr(&value))
            .expect("writing tag attr should not fail");
    }
    out.push_str(">\n");
}

fn write_empty_tag(
    out: &mut String,
    indent: usize,
    name: &str,
    attrs: Vec<(&'static str, String)>,
) {
    write_indent(out, indent);
    write!(out, "<{name}").expect("writing tag should not fail");
    for (key, value) in attrs {
        write!(out, " {key}=\"{}\"", xml_escape_attr(&value))
            .expect("writing tag attr should not fail");
    }
    out.push_str("/>\n");
}

fn write_text_tag(
    out: &mut String,
    indent: usize,
    name: &str,
    attrs: Vec<(&'static str, String)>,
    text: &str,
) {
    write_indent(out, indent);
    write!(out, "<{name}").expect("writing tag should not fail");
    for (key, value) in attrs {
        write!(out, " {key}=\"{}\"", xml_escape_attr(&value))
            .expect("writing tag attr should not fail");
    }
    writeln!(out, ">{}</{name}>", xml_escape_text(text)).expect("writing text tag should not fail");
}

fn write_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push(' ');
    }
}

fn xml_escape_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn xml_escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn parse_xml_tree(xml: &str) -> Result<XmlNode, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut stack: Vec<XmlNode> = Vec::new();
    loop {
        match reader.read_event().map_err(|error| error.to_string())? {
            Event::Start(start) => stack.push(xml_node_from_start(&reader, &start)?),
            Event::Empty(start) => {
                let node = xml_node_from_start(&reader, &start)?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    return Ok(node);
                }
            }
            Event::Text(text) => {
                if let Some(node) = stack.last_mut() {
                    node.text
                        .push_str(&text.xml_content().map_err(|error| error.to_string())?);
                }
            }
            Event::CData(text) => {
                if let Some(node) = stack.last_mut() {
                    node.text
                        .push_str(&text.xml_content().map_err(|error| error.to_string())?);
                }
            }
            Event::End(_) => {
                let Some(node) = stack.pop() else {
                    return Err("unexpected XML end tag".to_string());
                };
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    return Ok(node);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Err("empty CDXML document".to_string())
}

fn xml_node_from_start(reader: &Reader<&[u8]>, start: &BytesStart<'_>) -> Result<XmlNode, String> {
    let mut attrs = BTreeMap::new();
    for attr in start.attributes() {
        let attr = attr.map_err(|error| error.to_string())?;
        let key = local_name(std::str::from_utf8(attr.key.as_ref()).map_err(|e| e.to_string())?);
        let value = attr
            .decode_and_unescape_value(reader.decoder())
            .map_err(|error| error.to_string())?
            .into_owned();
        attrs.insert(key, value);
    }
    Ok(XmlNode {
        name: local_name(std::str::from_utf8(start.name().as_ref()).map_err(|e| e.to_string())?),
        attrs,
        text: String::new(),
        children: Vec::new(),
    })
}

fn local_name(value: &str) -> String {
    value
        .rsplit_once('}')
        .map(|(_, name)| name)
        .unwrap_or(value)
        .rsplit_once(':')
        .map(|(_, name)| name)
        .unwrap_or(value)
        .to_string()
}

impl XmlNode {
    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.get(key).map(String::as_str)
    }

    fn is(&self, name: &str) -> bool {
        self.name == name
    }

    fn direct_children<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a XmlNode> {
        self.children.iter().filter(move |child| child.is(name))
    }

    fn full_text(&self) -> String {
        let mut out = self.text.clone();
        for child in &self.children {
            out.push_str(&child.full_text());
        }
        out
    }
}

fn cdxml_defaults(root: &XmlNode) -> CdxmlDefaults {
    CdxmlDefaults {
        bond_length: parse_f64(root.attr("BondLength")).unwrap_or(crate::DEFAULT_BOND_LENGTH),
        line_width: parse_f64(root.attr("LineWidth")).unwrap_or(crate::DEFAULT_BOND_STROKE),
        bold_width: parse_f64(root.attr("BoldWidth")).unwrap_or(crate::BOLD_BOND_WIDTH_CM.value()),
        hash_spacing: parse_f64(root.attr("HashSpacing"))
            .unwrap_or(crate::DEFAULT_HASH_SPACING_CM.value()),
        bond_spacing: parse_f64(root.attr("BondSpacing"))
            .unwrap_or(crate::DEFAULT_BOND_SPACING_PERCENT),
    }
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
                "fontSize": crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM,
            }),
        ),
        (
            "style_text_default".to_string(),
            json!({
                "kind": "text",
                "fontFamily": "Arial",
                "fontSize": 10.0,
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

fn cdxml_color_table(root: &XmlNode) -> BTreeMap<String, String> {
    let mut colors = BTreeMap::from([("0".to_string(), "#000000".to_string())]);
    if let Some(table) = descendants(root)
        .into_iter()
        .find(|node| node.is("colortable"))
    {
        for (index, color) in table.direct_children("color").enumerate() {
            let Some(r) = parse_f64(color.attr("r")) else {
                continue;
            };
            let Some(g) = parse_f64(color.attr("g")) else {
                continue;
            };
            let Some(b) = parse_f64(color.attr("b")) else {
                continue;
            };
            colors.insert(
                (index + 1).to_string(),
                format!(
                    "#{:02x}{:02x}{:02x}",
                    (r * 255.0).round().clamp(0.0, 255.0) as u8,
                    (g * 255.0).round().clamp(0.0, 255.0) as u8,
                    (b * 255.0).round().clamp(0.0, 255.0) as u8
                ),
            );
        }
    }
    // ChemDraw commonly writes the first colortable entries as white/black,
    // while document objects still use legacy palette ids for semantic colors.
    // These ids are visible in exported SVG from the same CDXML fixtures.
    for (id, color) in [
        ("0", "#000000"),
        ("3", "#000000"),
        ("4", "#d61f1f"),
        ("5", "#fff24a"),
        ("7", "#55f0f5"),
        ("8", "#1b32d8"),
        ("10", "#cfcfcf"),
    ] {
        colors.insert(id.to_string(), color.to_string());
    }
    colors
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
    descendants(root)
        .into_iter()
        .filter(|node| {
            node.is("fragment")
                && node.attr("BoundingBox").is_some()
                && node.direct_children("n").count() >= 2
                && node.direct_children("b").next().is_some()
        })
        .collect()
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
    colors: &BTreeMap<String, String>,
    fonts: &BTreeMap<String, String>,
) -> Option<MoleculeFragment> {
    let origin = [bbox[0], bbox[1]];
    let node_ids: BTreeSet<String> = fragment
        .direct_children("n")
        .filter_map(|node| node.attr("id").map(ToString::to_string))
        .collect();
    let nodes: Vec<Node> = fragment
        .direct_children("n")
        .filter_map(|node| normalize_node(node, origin, colors, fonts))
        .collect();
    let bonds: Vec<Bond> = fragment
        .direct_children("b")
        .filter_map(|bond| normalize_bond(bond, &node_ids, defaults))
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
    crate::engine::refresh_attached_node_label_geometry_for_all_nodes(
        &mut fragment,
        origin,
        defaults.line_width,
    );
    Some(fragment)
}

fn normalize_node(
    node: &XmlNode,
    origin: [f64; 2],
    colors: &BTreeMap<String, String>,
    fonts: &BTreeMap<String, String>,
) -> Option<Node> {
    let id = node.attr("id")?.to_string();
    let position = parse_xy(node.attr("p"))?;
    let atomic_number = parse_u8(node.attr("Element")).unwrap_or(6);
    let node_type = node.attr("NodeType").unwrap_or("");
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
        is_placeholder: matches!(node_type, "Fragment" | "Nickname" | "Unspecified"),
        label: node_label(node, origin, colors, fonts),
        meta: json!({
            "import": {
                "cdxml": {
                    "nodeType": empty_as_null(node.attr("NodeType")),
                    "labelDisplay": empty_as_null(node.attr("LabelDisplay")),
                    "element": node.attr("Element"),
                }
            }
        }),
    })
}

fn node_label(
    node: &XmlNode,
    origin: [f64; 2],
    colors: &BTreeMap<String, String>,
    fonts: &BTreeMap<String, String>,
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
    let parent_font = text_el.attr("font").unwrap_or("3");
    let parent_color = text_el.attr("color").unwrap_or("0");
    let parent_size = parse_f64(text_el.attr("size")).unwrap_or(10.0);
    let runs: Vec<LabelRun> = text_el
        .direct_children("s")
        .filter_map(|run| {
            let run_text = run.full_text();
            (!run_text.is_empty()).then(|| {
                label_run(
                    &run_text,
                    parse_u32(run.attr("face")).unwrap_or(0),
                    run.attr("font").unwrap_or(parent_font),
                    run.attr("color").unwrap_or(parent_color),
                    parse_f64(run.attr("size")).unwrap_or(parent_size),
                    colors,
                    fonts,
                )
            })
        })
        .collect();
    Some(NodeLabel {
        text: text.clone(),
        source_text: Some(text.clone()),
        position: parse_xy(node.attr("p"))
            .map(|point| [round2(point[0] - origin[0]), round2(point[1] - origin[1])]),
        box_field: None,
        runs,
        line_runs: Vec::new(),
        lines: if text.contains('\n') {
            text.lines().map(ToString::to_string).collect()
        } else {
            Vec::new()
        },
        align: Some("left".to_string()),
        layout: None,
        attachment: Some("node".to_string()),
        anchor: Some("start".to_string()),
        font_family: Some(
            fonts
                .get(parent_font)
                .cloned()
                .unwrap_or_else(|| "Arial".to_string()),
        ),
        fill: Some(
            colors
                .get(parent_color)
                .cloned()
                .unwrap_or_else(|| "#000000".to_string()),
        ),
        font_size: Some(parent_size),
        glyph_polygons: Vec::new(),
        box_value: None,
        meta: json!({
            "import": {
                "cdxml": {
                    "font": parent_font,
                    "color": parent_color,
                    "textPosition": parse_xy(text_el.attr("p")),
                    "boundingBox": bbox,
                    "labelAlignment": empty_as_null(text_el.attr("LabelAlignment")),
                    "labelJustification": empty_as_null(text_el.attr("LabelJustification")),
                    "justification": empty_as_null(text_el.attr("Justification")),
                }
            }
        }),
    })
}

fn normalize_bond(
    bond: &XmlNode,
    node_ids: &BTreeSet<String>,
    defaults: CdxmlDefaults,
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
        _ => None,
    };
    let order = cdxml_bond_order(bond.attr("Order"));
    let placement = match bond
        .attr("DoublePosition")
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "left" => Some(crate::DoubleBondPlacement::Left),
        "right" => Some(crate::DoubleBondPlacement::Right),
        _ if order == 2 => Some(crate::DoubleBondPlacement::Center),
        _ => None,
    };
    Some(Bond {
        id: bond.attr("id").unwrap_or("").to_string(),
        begin,
        end,
        order,
        double: placement.map(|placement| crate::DoubleBond {
            placement,
            center_exit_side: None,
            frozen: true,
        }),
        stereo,
        stroke_width,
        bold_width: Some(bold_width),
        hash_spacing: Some(hash_spacing),
        bond_spacing: Some(bond_spacing),
        line_styles: cdxml_bond_line_styles(order, display, bond.attr("Display2").unwrap_or("")),
        line_weights: cdxml_bond_line_weights(order, display, bond.attr("Display2").unwrap_or("")),
        meta: json!({"import": {"cdxml": {"display": empty_as_null(bond.attr("Display")), "doublePosition": empty_as_null(bond.attr("DoublePosition"))}}}),
    })
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

fn append_line_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &BTreeMap<String, String>,
) {
    let mut index = 1;
    for node in descendants(root) {
        if !(node.is("arrow") || (node.is("graphic") && node.attr("GraphicType") == Some("Line"))) {
            continue;
        }
        if node.attr("SupersededBy").is_some() {
            continue;
        }
        let head = parse_xyz2(node.attr("Head3D"));
        let tail = parse_xyz2(node.attr("Tail3D"));
        let (Some(tail), Some(head)) = (tail, head) else {
            continue;
        };
        let is_arrow = node.is("arrow") || has_arrow_attrs(node);
        let head_enabled = arrow_endpoint_enabled(node.attr("ArrowheadHead"))
            || node
                .attr("ArrowType")
                .is_some_and(|value| value == "FullHead");
        let tail_enabled = arrow_endpoint_enabled(node.attr("ArrowheadTail"));
        let mut extra = BTreeMap::new();
        extra.insert("kind".to_string(), json!("line"));
        extra.insert(
            "points".to_string(),
            json!([
                [round2(tail[0]), round2(tail[1])],
                [round2(head[0]), round2(head[1])]
            ]),
        );
        if is_arrow {
            extra.insert(
                "head".to_string(),
                json!(if head_enabled { "end" } else { "none" }),
            );
            extra.insert(
                "tail".to_string(),
                json!(if tail_enabled { "start" } else { "none" }),
            );
            extra.insert(
                "arrowHead".to_string(),
                json!({
                    "kind": node.attr("ArrowheadType").or_else(|| node.attr("ArrowType")).unwrap_or("Solid").to_ascii_lowercase(),
                    "head": node.attr("ArrowheadHead").unwrap_or(if head_enabled { "Full" } else { "None" }).to_ascii_lowercase(),
                    "tail": node.attr("ArrowheadTail").unwrap_or(if tail_enabled { "Full" } else { "None" }).to_ascii_lowercase(),
                    "length": parse_scaled_100(node.attr("HeadSize")).unwrap_or(defaults.bond_length * 0.7),
                    "centerLength": parse_scaled_100(node.attr("ArrowheadCenterSize")).unwrap_or(defaults.bond_length * 0.45),
                    "width": parse_scaled_100(node.attr("ArrowheadWidth")).unwrap_or(defaults.bond_length * 0.25),
                }),
            );
        }
        let style_ref = cdxml_line_style_ref(node, is_arrow, styles, defaults, colors);
        objects.push(SceneObject {
            id: format!("obj_line_{index:03}"),
            object_type: "line".to_string(),
            name: format!("line {index}"),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(if is_arrow { 20 } else { 18 }),
            transform: Transform::identity(),
            style_ref: Some(style_ref),
            meta: json!({"source": "cdxml", "graphicId": node.attr("id"), "import": {"cdxml": {"kind": if is_arrow { "arrow" } else { "line" }, "lineType": empty_as_null(node.attr("LineType"))}}}),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: None,
                extra,
            },
        });
        index += 1;
    }
}

fn cdxml_line_style_ref(
    node: &XmlNode,
    is_arrow: bool,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &BTreeMap<String, String>,
) -> String {
    let line_type = node.attr("LineType").unwrap_or("");
    let bold = line_type.contains("Bold");
    let dashed = line_type.contains("Dashed");
    let color = colors
        .get(node.attr("color").unwrap_or("0"))
        .cloned()
        .unwrap_or_else(|| "#000000".to_string());
    let base = if is_arrow { "arrow" } else { "line" };
    if !bold && !dashed && color == "#000000" {
        return format!("style_{base}_default");
    }
    let style_id = format!(
        "style_{base}_{}{}{}",
        if bold { "bold" } else { "regular" },
        if dashed { "_dashed" } else { "" },
        if color == "#000000" {
            String::new()
        } else {
            format!("_{}", color.trim_start_matches('#'))
        }
    );
    styles.entry(style_id.clone()).or_insert_with(|| {
        json!({
            "kind": "stroke",
            "stroke": color,
            "strokeWidth": if bold { defaults.bold_width } else { defaults.line_width },
            "lineCap": if is_arrow { "butt" } else { "round" },
            "lineJoin": if is_arrow { "miter" } else { "round" },
            "dashArray": if dashed { json!([2.7]) } else { json!([]) },
        })
    });
    style_id
}

fn append_shape_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    colors: &BTreeMap<String, String>,
) {
    let mut index = 1;
    for node in descendants(root) {
        if !node.is("graphic") || node.attr("SupersededBy").is_some() {
            continue;
        }
        let graphic_type = node.attr("GraphicType").unwrap_or("");
        if !matches!(graphic_type, "Rectangle" | "Oval") {
            continue;
        }
        let Some(bbox) = parse_bbox(node.attr("BoundingBox")) else {
            continue;
        };
        let type_value = node
            .attr(if graphic_type == "Rectangle" {
                "RectangleType"
            } else {
                "OvalType"
            })
            .unwrap_or("");
        let color = colors
            .get(node.attr("color").unwrap_or("0"))
            .cloned()
            .unwrap_or_else(|| "#000000".to_string());
        let filled = type_value.contains("Filled");
        let shaded = type_value.contains("Shaded");
        let shadow = type_value.contains("Shadow");
        let style_id = format!("style_shape_{index:03}");
        styles.insert(
            style_id.clone(),
            json!({
                "kind": "shape",
                "fill": if filled || shaded { json!(color) } else { Value::Null },
                "stroke": if filled { Value::Null } else { json!(color) },
                "strokeWidth": if filled { 0.0 } else { 1.0 },
                "dashArray": if type_value.contains("Dashed") { json!([2.7]) } else { json!([]) },
                "shaded": if shaded { json!(true) } else { Value::Null },
                "shadow": if shadow { json!(true) } else { Value::Null },
            }),
        );
        let (transform, payload) = if graphic_type == "Oval" {
            let (Some(center), Some(major), Some(minor)) = (
                parse_xyz2(node.attr("Center3D")),
                parse_xyz2(node.attr("MajorAxisEnd3D")),
                parse_xyz2(node.attr("MinorAxisEnd3D")),
            ) else {
                continue;
            };
            let mut extra = BTreeMap::new();
            extra.insert(
                "kind".to_string(),
                json!(if type_value.contains("Circle") {
                    "circle"
                } else {
                    "ellipse"
                }),
            );
            extra.insert(
                "center".to_string(),
                json!([round2(center[0]), round2(center[1])]),
            );
            extra.insert(
                "majorAxisEnd".to_string(),
                json!([round2(major[0]), round2(major[1])]),
            );
            extra.insert(
                "minorAxisEnd".to_string(),
                json!([round2(minor[0]), round2(minor[1])]),
            );
            (
                Transform::identity(),
                ObjectPayload {
                    resource_ref: None,
                    bbox: Some([
                        round2(bbox[0]),
                        round2(bbox[1]),
                        round2(bbox[2] - bbox[0]),
                        round2(bbox[3] - bbox[1]),
                    ]),
                    extra,
                },
            )
        } else {
            let mut extra = BTreeMap::new();
            extra.insert(
                "kind".to_string(),
                json!(if type_value.contains("RoundEdge") {
                    "roundRect"
                } else {
                    "rect"
                }),
            );
            extra.insert(
                "cornerRadius".to_string(),
                json!(parse_scaled_100(node.attr("CornerRadius")).unwrap_or(0.0)),
            );
            (
                Transform {
                    translate: [round2(bbox[0]), round2(bbox[1])],
                    rotate: 0.0,
                    scale: [1.0, 1.0],
                },
                ObjectPayload {
                    resource_ref: None,
                    bbox: Some([
                        0.0,
                        0.0,
                        round2(bbox[2] - bbox[0]),
                        round2(bbox[3] - bbox[1]),
                    ]),
                    extra,
                },
            )
        };
        objects.push(SceneObject {
            id: format!("obj_shape_{index:03}"),
            object_type: "shape".to_string(),
            name: format!("shape {index}"),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(15),
            transform,
            style_ref: Some(style_id),
            meta: json!({"source": "cdxml", "graphicId": node.attr("id")}),
            payload,
        });
        index += 1;
    }
}

#[derive(Clone)]
struct PendingCdxmlBracket {
    kind: String,
    bbox: [f64; 4],
    z_index: i32,
    graphic_id: Option<String>,
}

fn append_bracket_objects(root: &XmlNode, objects: &mut Vec<SceneObject>, defaults: CdxmlDefaults) {
    let mut brackets = Vec::new();
    let mut symbol_index = 1;
    for node in descendants(root) {
        if !node.is("graphic") || node.attr("SupersededBy").is_some() {
            continue;
        }
        match node.attr("GraphicType").unwrap_or("") {
            "Bracket" => {
                let Some(bbox) = parse_bbox(node.attr("BoundingBox")) else {
                    continue;
                };
                brackets.push(PendingCdxmlBracket {
                    kind: match node.attr("BracketType").unwrap_or("") {
                        "Square" => "square",
                        "Curly" => "curly",
                        _ => "round",
                    }
                    .to_string(),
                    bbox,
                    z_index: parse_i32(node.attr("Z")).unwrap_or(15),
                    graphic_id: node.attr("id").map(ToString::to_string),
                });
            }
            "Symbol" => {
                let symbol_type = node.attr("SymbolType").unwrap_or("");
                let Some(kind) = cdxml_symbol_kind(symbol_type) else {
                    continue;
                };
                let Some(raw_bbox) = parse_bbox(node.attr("BoundingBox")) else {
                    continue;
                };
                let cx = (raw_bbox[0] + raw_bbox[2]) * 0.5;
                let cy = (raw_bbox[1] + raw_bbox[3]) * 0.5;
                let style = crate::cdxml_symbol_style_from_line_width(defaults.line_width);
                let metrics =
                    crate::cdxml_symbol_metrics_from_bbox(kind, raw_bbox, defaults.line_width);
                let (width, height) = (metrics.width, metrics.height);
                let mut extra = BTreeMap::new();
                extra.insert("kind".to_string(), json!(kind));
                extra.insert("fill".to_string(), json!("#000000"));
                extra.insert(
                    "symbolStyle".to_string(),
                    json!(crate::cdxml_symbol_style_name(style)),
                );
                extra.insert(
                    "symbolAnchorWidth".to_string(),
                    json!(metrics.cdxml_anchor_width),
                );
                extra.insert(
                    "symbolAnchorHeight".to_string(),
                    json!(metrics.cdxml_anchor_height),
                );
                extra.insert("symbolLineWidth".to_string(), json!(metrics.line_width));
                extra.insert("cdxmlBoundingBox".to_string(), json!(raw_bbox));
                if let Some(stroke_width) = metrics.stroke_width {
                    extra.insert("strokeWidth".to_string(), json!(stroke_width));
                }
                objects.push(SceneObject {
                    id: format!("obj_symbol_{symbol_index:03}"),
                    object_type: "symbol".to_string(),
                    name: format!("symbol {symbol_index}"),
                    visible: true,
                    locked: false,
                    z_index: parse_i32(node.attr("Z")).unwrap_or(15),
                    transform: Transform {
                        translate: [round2(cx - width * 0.5), round2(cy - height * 0.5)],
                        rotate: 0.0,
                        scale: [1.0, 1.0],
                    },
                    style_ref: None,
                    meta: json!({"source": "cdxml", "graphicId": node.attr("id")}),
                    payload: ObjectPayload {
                        resource_ref: None,
                        bbox: Some([0.0, 0.0, width, height]),
                        extra,
                    },
                });
                symbol_index += 1;
            }
            _ => {}
        }
    }

    let mut used = vec![false; brackets.len()];
    let mut object_index = 1;
    for left_index in 0..brackets.len() {
        if used[left_index] {
            continue;
        }
        let left = &brackets[left_index];
        let left_bounds = normalized_bbox(left.bbox);
        let mut best_index = None;
        let mut best_dx = f64::INFINITY;
        for right_index in 0..brackets.len() {
            if left_index == right_index || used[right_index] {
                continue;
            }
            let right = &brackets[right_index];
            if right.kind != left.kind {
                continue;
            }
            let right_bounds = normalized_bbox(right.bbox);
            if (center_y(left_bounds) - center_y(right_bounds)).abs() > 2.0
                || (height_of(left_bounds) - height_of(right_bounds)).abs() > 2.0
            {
                continue;
            }
            let dx = (center_x(right_bounds) - center_x(left_bounds)).abs();
            if dx > crate::EPSILON && dx < best_dx {
                best_dx = dx;
                best_index = Some(right_index);
            }
        }
        let Some(right_index) = best_index else {
            continue;
        };
        used[left_index] = true;
        used[right_index] = true;
        let right = &brackets[right_index];
        let lb = normalized_bbox(left.bbox);
        let rb = normalized_bbox(right.bbox);
        let min_x = lb[0].min(rb[0]);
        let min_y = lb[1].min(rb[1]);
        let max_x = lb[2].max(rb[2]);
        let max_y = lb[3].max(rb[3]);
        let mut extra = BTreeMap::new();
        extra.insert("kind".to_string(), json!(left.kind));
        extra.insert("stroke".to_string(), json!("#000000"));
        extra.insert("strokeWidth".to_string(), json!(1.0));
        extra.insert("lipSize".to_string(), json!(60));
        objects.push(SceneObject {
            id: format!("obj_bracket_{object_index:03}"),
            object_type: "bracket".to_string(),
            name: format!("bracket {object_index}"),
            visible: true,
            locked: false,
            z_index: left.z_index.min(right.z_index),
            transform: Transform {
                translate: [round2(min_x), round2(min_y)],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: None,
            meta: json!({
                "source": "cdxml",
                "graphicIds": [left.graphic_id, right.graphic_id],
            }),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: Some([0.0, 0.0, round2(max_x - min_x), round2(max_y - min_y)]),
                extra,
            },
        });
        object_index += 1;
    }
}

fn normalized_bbox(bbox: [f64; 4]) -> [f64; 4] {
    [
        bbox[0].min(bbox[2]),
        bbox[1].min(bbox[3]),
        bbox[0].max(bbox[2]),
        bbox[1].max(bbox[3]),
    ]
}

fn center_x(bbox: [f64; 4]) -> f64 {
    (bbox[0] + bbox[2]) * 0.5
}

fn center_y(bbox: [f64; 4]) -> f64 {
    (bbox[1] + bbox[3]) * 0.5
}

fn height_of(bbox: [f64; 4]) -> f64 {
    bbox[3] - bbox[1]
}

fn cdxml_symbol_kind(symbol_type: &str) -> Option<&'static str> {
    Some(match symbol_type {
        "DoubleDagger" => "double-dagger",
        "Dagger" => "dagger",
        "CirclePlus" => "circle-plus",
        "Plus" => "plus",
        "RadicalCation" => "radical-cation",
        "LonePair" => "lone-pair",
        "CircleMinus" => "circle-minus",
        "Minus" => "minus",
        "RadicalAnion" => "radical-anion",
        "Electron" => "electron",
        _ => return None,
    })
}

fn append_text_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    colors: &BTreeMap<String, String>,
    fonts: &BTreeMap<String, String>,
    display_fragment_ids: &BTreeSet<String>,
    bonded_node_ids: &BTreeSet<String>,
) {
    let mut index = 1;
    append_text_objects_recursive(
        root,
        false,
        0,
        None,
        &mut index,
        objects,
        styles,
        colors,
        fonts,
        display_fragment_ids,
        bonded_node_ids,
    );
}

fn append_text_objects_recursive(
    node: &XmlNode,
    skip_text: bool,
    placeholder_depth: usize,
    inherited_z: Option<i32>,
    index: &mut usize,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    colors: &BTreeMap<String, String>,
    fonts: &BTreeMap<String, String>,
    display_fragment_ids: &BTreeSet<String>,
    bonded_node_ids: &BTreeSet<String>,
) {
    let next_skip_text = skip_text
        || (node.is("fragment")
            && node
                .attr("id")
                .is_some_and(|id| display_fragment_ids.contains(id)))
        || (node.is("n")
            && node.attr("Element").is_some()
            && node
                .attr("id")
                .map_or(true, |id| bonded_node_ids.contains(id)));
    let next_placeholder_depth = if node.is("n")
        && matches!(
            node.attr("NodeType").unwrap_or(""),
            "Fragment" | "Nickname" | "Unspecified"
        ) {
        1
    } else if placeholder_depth > 0 {
        placeholder_depth + 1
    } else {
        0
    };
    let current_z = parse_i32(node.attr("Z")).or(inherited_z);
    if node.is("t") && !skip_text && placeholder_depth <= 1 {
        if let Some(object) =
            text_object(node, *index, current_z.unwrap_or(30), styles, colors, fonts)
        {
            objects.push(object);
            *index += 1;
        }
    }
    for child in &node.children {
        append_text_objects_recursive(
            child,
            next_skip_text,
            next_placeholder_depth,
            current_z,
            index,
            objects,
            styles,
            colors,
            fonts,
            display_fragment_ids,
            bonded_node_ids,
        );
    }
}

fn text_object(
    node: &XmlNode,
    index: usize,
    z_index: i32,
    styles: &mut BTreeMap<String, Value>,
    colors: &BTreeMap<String, String>,
    fonts: &BTreeMap<String, String>,
) -> Option<SceneObject> {
    let text = node
        .attr("UTF8Text")
        .map(ToString::to_string)
        .unwrap_or_else(|| node.full_text())
        .trim()
        .to_string();
    if text.is_empty() {
        return None;
    }
    let bbox = parse_bbox(node.attr("BoundingBox"));
    let point = parse_xy(node.attr("p")).or_else(|| bbox.map(|bbox| [bbox[0], bbox[1]]))?;
    let align = node
        .attr("Justification")
        .or_else(|| node.attr("LabelJustification"))
        .unwrap_or("Left")
        .to_ascii_lowercase();
    let font_id = node.attr("font").unwrap_or("3");
    let color_id = node.attr("color").unwrap_or("0");
    let font_size = parse_f64(node.attr("size")).unwrap_or_else(|| {
        node.direct_children("s")
            .find_map(|run| parse_f64(run.attr("size")))
            .unwrap_or(10.0)
    });
    let style_id = format!("style_text_{index:03}");
    styles.entry(style_id.clone()).or_insert_with(|| {
        json!({
            "kind": "text",
            "fontFamily": fonts.get(font_id).cloned().unwrap_or_else(|| "Arial".to_string()),
            "fontSize": font_size,
            "fontWeight": 400,
            "fill": colors.get(color_id).cloned().unwrap_or_else(|| "#000000".to_string()),
            "stroke": null,
        })
    });
    let runs: Vec<LabelRun> = node
        .direct_children("s")
        .filter_map(|run| {
            let run_text = run.full_text();
            (!run_text.is_empty()).then(|| {
                label_run(
                    &run_text,
                    parse_u32(run.attr("face")).unwrap_or(0),
                    run.attr("font").unwrap_or(font_id),
                    run.attr("color").unwrap_or(color_id),
                    parse_f64(run.attr("size")).unwrap_or(font_size),
                    colors,
                    fonts,
                )
            })
        })
        .collect();
    let width = bbox
        .map(|bbox| (bbox[2] - bbox[0]).abs())
        .unwrap_or(160.0)
        .max(24.0);
    let height = bbox
        .map(|bbox| (bbox[3] - bbox[1]).abs())
        .unwrap_or(font_size * 1.4)
        .max(14.0);
    let translate = if let Some(bbox) = bbox {
        let x = match align.as_str() {
            "center" => (bbox[0] + bbox[2]) * 0.5,
            "right" => bbox[2],
            _ => bbox[0],
        };
        [round2(x), round2(bbox[1])]
    } else {
        [round2(point[0]), round2(point[1])]
    };
    let mut extra = BTreeMap::new();
    extra.insert("text".to_string(), json!(text));
    extra.insert(
        "box".to_string(),
        json!([0.0, 0.0, round2(width), round2(height)]),
    );
    extra.insert("align".to_string(), json!(align));
    extra.insert("valign".to_string(), json!("top"));
    extra.insert("lineHeight".to_string(), json!(round2(font_size * 1.2)));
    extra.insert("fontSize".to_string(), json!(round2(font_size)));
    extra.insert("preserveLines".to_string(), json!(true));
    if !runs.is_empty() {
        extra.insert("runs".to_string(), serde_json::to_value(runs).ok()?);
    }
    Some(SceneObject {
        id: format!("obj_text_{index:03}"),
        object_type: "text".to_string(),
        name: format!("text {index}"),
        visible: true,
        locked: false,
        z_index,
        transform: Transform {
            translate,
            rotate: 0.0,
            scale: [1.0, 1.0],
        },
        style_ref: Some(style_id),
        meta: json!({"source": "cdxml", "role": "free_text"}),
        payload: ObjectPayload {
            resource_ref: None,
            bbox: None,
            extra,
        },
    })
}

fn label_run(
    text: &str,
    face: u32,
    font_id: &str,
    color_id: &str,
    font_size: f64,
    colors: &BTreeMap<String, String>,
    fonts: &BTreeMap<String, String>,
) -> LabelRun {
    LabelRun {
        text: text.to_string(),
        font_family: Some(
            fonts
                .get(font_id)
                .cloned()
                .unwrap_or_else(|| "Arial".to_string()),
        ),
        font_size: Some(round2(font_size)),
        fill: Some(
            colors
                .get(color_id)
                .cloned()
                .unwrap_or_else(|| "#000000".to_string()),
        ),
        font_weight: Some(if face & 1 != 0 { 700 } else { 400 }),
        font_style: Some(if face & 2 != 0 { "italic" } else { "normal" }.to_string()),
        underline: None,
        script: Some(
            if face & 32 != 0 && face & 64 == 0 {
                "subscript"
            } else if face & 64 != 0 && face & 32 == 0 {
                "superscript"
            } else {
                "normal"
            }
            .to_string(),
        ),
    }
}

fn page_from_objects(objects: &[SceneObject]) -> Page {
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
        background: "#ffffff".to_string(),
    }
}

fn descendants(node: &XmlNode) -> Vec<&XmlNode> {
    let mut out = Vec::new();
    collect_descendants(node, &mut out);
    out
}

fn collect_descendants<'a>(node: &'a XmlNode, out: &mut Vec<&'a XmlNode>) {
    out.push(node);
    for child in &node.children {
        collect_descendants(child, out);
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
