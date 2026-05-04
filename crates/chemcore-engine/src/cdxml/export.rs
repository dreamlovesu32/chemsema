use crate::{
    Bond, ChemcoreDocument, LabelRun, MoleculeFragment, Node, NodeLabel, ObjectPayload, Point,
    ResourceData, SceneObject,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Write;

use super::{element_symbol, CdxmlDefaults};

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
        if let Some(value) = import_defaults.get("labelSize").and_then(Value::as_f64) {
            defaults.label_size = value;
        }
        if let Some(value) = import_defaults.get("captionSize").and_then(Value::as_f64) {
            defaults.caption_size = value;
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
            "<CDXML Name=\"{}\" BoundingBox=\"{}\" WindowPosition=\"0 0\" WindowSize=\"{} {}\" FractionalWidths=\"yes\" InterpretChemically=\"yes\" ShowAtomQuery=\"yes\" ShowBondQuery=\"yes\" LabelFont=\"3\" LabelSize=\"{}\" CaptionFont=\"3\" CaptionSize=\"{}\" LineWidth=\"{}\" BoldWidth=\"{}\" BondLength=\"{}\" BondSpacing=\"{}\" HashSpacing=\"{}\" color=\"{}\" bgcolor=\"{}\">\n",
            xml_escape_attr(&self.document.document.title),
            root_bbox,
            fmt_num(width),
            fmt_num(height),
            fmt_num(self.defaults.label_size),
            fmt_num(self.defaults.caption_size),
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
            if arrow
                .and_then(|value| value.get("bold"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                attrs.push(("LineType", "Bold".to_string()));
            }
            attrs.push((
                "ArrowheadHead",
                if has_head { "Full" } else { "None" }.to_string(),
            ));
            attrs.push((
                "ArrowheadTail",
                if has_tail { "Full" } else { "None" }.to_string(),
            ));
            let arrow_kind = cdxml_arrow_kind(arrow);
            attrs.push(("ArrowheadType", arrow_kind.to_string()));
            if let Some(value) = arrow
                .and_then(|value| value.get("length"))
                .and_then(Value::as_f64)
            {
                attrs.push(("HeadSize", fmt_num(value * 100.0)));
            }
            if let Some(value) = arrow
                .and_then(|value| {
                    value
                        .get("centerLength")
                        .or_else(|| value.get("center_length"))
                })
                .and_then(Value::as_f64)
            {
                attrs.push(("ArrowheadCenterSize", fmt_num(value * 100.0)));
                if matches!(arrow_kind, "Hollow" | "Angle") {
                    attrs.push(("ArrowShaftSpacing", fmt_num(value * 100.0)));
                }
            }
            if let Some(value) = arrow
                .and_then(|value| value.get("width"))
                .and_then(Value::as_f64)
            {
                attrs.push(("ArrowheadWidth", fmt_num(value * 100.0)));
            }
            if let Some(value) = arrow
                .and_then(|value| value.get("curve"))
                .and_then(Value::as_f64)
                .filter(|value| value.abs() > crate::EPSILON)
            {
                attrs.push(("AngularSize", fmt_num(value)));
            }
            if let Some(value) = arrow
                .and_then(|value| value.get("noGo").or_else(|| value.get("no_go")))
                .and_then(Value::as_str)
                .and_then(cdxml_arrow_no_go)
            {
                attrs.push(("NoGo", value.to_string()));
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
        let filled = fill.is_some() && stroke.is_none();
        let shaded = style
            .and_then(|style| style.get("shaded"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let shadowed = style
            .and_then(|style| style.get("shadow"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let dashed = style
            .and_then(|style| style_number_array(style, "dashArray"))
            .is_some_and(|dash| !dash.is_empty());
        let shadow_size = style
            .and_then(|style| style_number_value(style, "shadowSize"))
            .unwrap_or(4.0);
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
            let mut oval_type = String::new();
            if kind == "circle" {
                oval_type.push_str("Circle");
            }
            push_cdxml_shape_type_flag(&mut oval_type, dashed, "Dashed");
            push_cdxml_shape_type_flag(&mut oval_type, shaded, "Shaded");
            push_cdxml_shape_type_flag(&mut oval_type, filled, "Filled");
            push_cdxml_shape_type_flag(&mut oval_type, shadowed, "Shadowed");
            let mut attrs = vec![
                ("id", self.alloc_id()),
                ("GraphicType", "Oval".to_string()),
                ("BoundingBox", fmt_bbox(bbox)),
                ("Center3D", fmt_point3(center)),
                ("MajorAxisEnd3D", fmt_point3(major)),
                ("MinorAxisEnd3D", fmt_point3(minor)),
                ("OvalType", oval_type),
                ("color", self.colors.id_for(color)),
                ("Z", object.z_index.to_string()),
            ];
            if let Some(stroke_width) =
                style.and_then(|style| style_number_value(style, "strokeWidth"))
            {
                attrs.push(("LineWidth", fmt_num(stroke_width)));
            }
            if shadowed {
                attrs.push(("ShadowSize", fmt_num(shadow_size * 100.0)));
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
        if kind == "rect" && !dashed && !shaded && !filled && !shadowed {
            rectangle_type.push_str("Plain");
        }
        push_cdxml_shape_type_flag(&mut rectangle_type, dashed, "Dashed");
        push_cdxml_shape_type_flag(&mut rectangle_type, shaded, "Shaded");
        push_cdxml_shape_type_flag(&mut rectangle_type, filled, "Filled");
        push_cdxml_shape_type_flag(&mut rectangle_type, shadowed, "Shadow");
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
        if shadowed {
            attrs.push(("ShadowSize", fmt_num(shadow_size * 100.0)));
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
            if run.underline.unwrap_or(false) {
                face |= 4;
            }
            match run.script.as_deref() {
                Some("subscript") => face |= 32,
                Some("superscript") => face |= 64,
                Some("chemical") => face |= 96,
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

fn style_number_array(style: &Value, key: &str) -> Option<Vec<f64>> {
    Some(
        style
            .get(key)?
            .as_array()?
            .iter()
            .filter_map(Value::as_f64)
            .collect(),
    )
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

fn cdxml_arrow_no_go(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "cross" => Some("Cross"),
        "hash" => Some("Hash"),
        _ => None,
    }
}

fn push_cdxml_shape_type_flag(out: &mut String, enabled: bool, flag: &str) {
    if !enabled {
        return;
    }
    if !out.is_empty() {
        out.push(' ');
    }
    out.push_str(flag);
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
