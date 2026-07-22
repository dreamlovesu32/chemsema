use crate::{
    Bond, ChemSemaDocument, DocumentTextStyle, LabelRun, MoleculeFragment, Node, NodeLabel,
    ObjectPayload, Point, ResourceData, SceneObject,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Write;

use super::{
    colors::{rgb_fractions, CdxmlColorTable},
    CdxmlDefaults, CdxmlJustification,
};

pub fn document_to_cdxml(document: &ChemSemaDocument) -> String {
    CdxmlDocumentWriter::new(document).write()
}

fn cdxml_editing_scale(document: &ChemSemaDocument) -> f64 {
    document
        .document
        .meta
        .pointer("/import/cdxml/editingScale")
        .and_then(Value::as_f64)
        .filter(|value| *value > crate::EPSILON)
        .unwrap_or(1.0)
}

fn export_cdxml_defaults(document: &ChemSemaDocument) -> CdxmlDefaults {
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
        if let Some(value) = import_defaults.get("marginWidth").and_then(Value::as_f64) {
            defaults.margin_width = value;
        }
        if let Some(value) = import_defaults.get("chainAngle").and_then(Value::as_f64) {
            defaults.chain_angle = value;
        }
        if let Some(value) = import_defaults
            .get("labelJustification")
            .and_then(value_cdxml_justification)
        {
            defaults.label_justification = value;
        }
        if let Some(value) = import_defaults
            .get("captionJustification")
            .and_then(value_cdxml_justification)
        {
            defaults.caption_justification = value;
        }
        if let Some(value) = import_defaults
            .get("fractionalWidths")
            .and_then(Value::as_bool)
        {
            defaults.fractional_widths = value;
        }
        if let Some(value) = import_defaults
            .get("interpretChemically")
            .and_then(Value::as_bool)
        {
            defaults.interpret_chemically = Some(value);
        }
        if let Some(value) = import_defaults
            .get("showAtomQuery")
            .and_then(Value::as_bool)
        {
            defaults.show_atom_query = value;
        }
        if let Some(value) = import_defaults
            .get("showAtomStereo")
            .and_then(Value::as_bool)
        {
            defaults.show_atom_stereo = value;
        }
        if let Some(value) = import_defaults
            .get("showAtomEnhancedStereo")
            .and_then(Value::as_bool)
        {
            defaults.show_atom_enhanced_stereo = value;
        }
        if let Some(value) = import_defaults
            .get("showAtomNumber")
            .and_then(Value::as_bool)
        {
            defaults.show_atom_number = value;
        }
        if let Some(value) = import_defaults
            .get("showResidueID")
            .and_then(Value::as_bool)
        {
            defaults.show_residue_id = value;
        }
        if let Some(value) = import_defaults
            .get("showBondQuery")
            .and_then(Value::as_bool)
        {
            defaults.show_bond_query = value;
        }
        if let Some(value) = import_defaults.get("showBondRxn").and_then(Value::as_bool) {
            defaults.show_bond_rxn = value;
        }
        if let Some(value) = import_defaults
            .get("showBondStereo")
            .and_then(Value::as_bool)
        {
            defaults.show_bond_stereo = value;
        }
        if let Some(value) = import_defaults
            .get("showTerminalCarbonLabels")
            .and_then(Value::as_bool)
        {
            defaults.show_terminal_carbon_labels = value;
        }
        if let Some(value) = import_defaults
            .get("showNonTerminalCarbonLabels")
            .and_then(Value::as_bool)
        {
            defaults.show_non_terminal_carbon_labels = value;
        }
        if let Some(value) = import_defaults
            .get("hideImplicitHydrogens")
            .and_then(Value::as_bool)
        {
            defaults.hide_implicit_hydrogens = value;
        }
        if let Some(value) = import_defaults.get("printMargins").and_then(value_margins) {
            defaults.print_margins = value;
        }
    }
    if let Some(value) = document.style.defaults.get("bondLength") {
        defaults.bond_length = *value;
    }
    if let Some(value) = document.style.defaults.get("chainAngle") {
        defaults.chain_angle = *value;
    }
    if let Some(value) = document.style.defaults.get("lineWidth") {
        defaults.line_width = *value;
    }
    if let Some(value) = document.style.defaults.get("boldWidth") {
        defaults.bold_width = *value;
    }
    if let Some(value) = document.style.defaults.get("hashSpacing") {
        defaults.hash_spacing = *value;
    }
    if let Some(value) = document.style.defaults.get("bondSpacing") {
        defaults.bond_spacing = *value;
    }
    if let Some(value) = document.style.defaults.get("marginWidth") {
        defaults.margin_width = *value;
    }
    defaults.label_size = document.style.label_style.font_size;
    defaults.caption_size = document.style.caption_style.font_size;
    defaults.label_face = cdxml_face_for_document_text_style(&document.style.label_style);
    defaults.caption_face = cdxml_face_for_document_text_style(&document.style.caption_style);
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
            if let Some(value) = bond.margin_width {
                defaults.margin_width = value;
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

fn cdxml_face_for_document_text_style(style: &DocumentTextStyle) -> u32 {
    let mut face = 0;
    if style.font_weight >= 600 {
        face |= 1;
    }
    if style.font_style.eq_ignore_ascii_case("italic") {
        face |= 2;
    }
    if style.underline {
        face |= 4;
    }
    if style.outline {
        face |= 8;
    }
    if style.shadow {
        face |= 16;
    }
    face |= match style.script.trim().to_ascii_lowercase().as_str() {
        "subscript" => 32,
        "superscript" => 64,
        "chemical" => 96,
        _ => 0,
    };
    face
}

fn value_cdxml_justification(value: &Value) -> Option<CdxmlJustification> {
    match value.as_str()?.trim().to_ascii_lowercase().as_str() {
        "auto" => Some(CdxmlJustification::Auto),
        "left" | "start" => Some(CdxmlJustification::Left),
        "center" | "middle" => Some(CdxmlJustification::Center),
        "right" | "end" => Some(CdxmlJustification::Right),
        "full" => Some(CdxmlJustification::Full),
        "above" => Some(CdxmlJustification::Above),
        "below" => Some(CdxmlJustification::Below),
        "best" => Some(CdxmlJustification::Best),
        _ => None,
    }
}

fn value_margins(value: &Value) -> Option<[f64; 4]> {
    let values = value.as_array()?;
    Some([
        values.first()?.as_f64()?,
        values.get(1)?.as_f64()?,
        values.get(2)?.as_f64()?,
        values.get(3)?.as_f64()?,
    ])
}

struct CdxmlDocumentWriter<'a> {
    document: &'a ChemSemaDocument,
    next_id: u64,
    node_ids: BTreeMap<String, String>,
    bond_ids: BTreeMap<(String, String), String>,
    colors: CdxmlColorTable,
    fonts: CdxmlFontTable,
    defaults: CdxmlDefaults,
    editing_scale: f64,
}

impl<'a> CdxmlDocumentWriter<'a> {
    fn new(document: &'a ChemSemaDocument) -> Self {
        let mut colors = CdxmlColorTable::for_export(&document.document.page.background);
        collect_document_colors(document, &mut colors);
        let mut fonts = CdxmlFontTable::default();
        collect_document_fonts(document, &mut fonts);
        let mut defaults = export_cdxml_defaults(document);
        defaults.label_font = fonts
            .id_for(&document.style.label_style.font_family)
            .parse()
            .unwrap_or(3);
        defaults.caption_font = fonts
            .id_for(&document.style.caption_style.font_family)
            .parse()
            .unwrap_or(3);
        let foreground = document
            .document
            .meta
            .pointer("/import/cdxml/defaults/foregroundColor")
            .and_then(Value::as_str)
            .unwrap_or(&document.style.label_style.fill);
        defaults.color = colors.id_for(foreground).parse().unwrap_or(0);
        Self {
            document,
            next_id: 1,
            node_ids: BTreeMap::new(),
            bond_ids: BTreeMap::new(),
            colors,
            fonts,
            defaults,
            editing_scale: cdxml_editing_scale(document),
        }
    }

    fn write(mut self) -> String {
        self.prepare_bond_ids();
        let page = &self.document.document.page;
        let width = page.width.max(1.0);
        let height = page.height.max(1.0);
        let root_bbox = format!("0 0 {} {}", fmt_num(width), fmt_num(height));
        let mut out = String::new();
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n");
        out.push_str("<!DOCTYPE CDXML SYSTEM \"http://www.cambridgesoft.com/xml/cdxml.dtd\" >\n");
        write!(
            out,
            "<CDXML CreationProgram=\"ChemSema\" Name=\"{}\" BoundingBox=\"{}\" WindowPosition=\"0 0\" WindowSize=\"-32768 -32768\" WindowIsZoomed=\"yes\" FractionalWidths=\"{}\" InterpretChemically=\"{}\" ShowAtomQuery=\"{}\" ShowAtomStereo=\"{}\" ShowAtomEnhancedStereo=\"{}\" ShowAtomNumber=\"{}\" ShowResidueID=\"{}\" ShowBondQuery=\"{}\" ShowBondRxn=\"{}\" ShowBondStereo=\"{}\" ShowTerminalCarbonLabels=\"{}\" ShowNonTerminalCarbonLabels=\"{}\" HideImplicitHydrogens=\"{}\" LabelFont=\"{}\" LabelSize=\"{}\" LabelFace=\"{}\" CaptionFont=\"{}\" CaptionSize=\"{}\" CaptionFace=\"{}\" LineWidth=\"{}\" BoldWidth=\"{}\" BondLength=\"{}\" BondSpacing=\"{}\" HashSpacing=\"{}\" MarginWidth=\"{}\" ChainAngle=\"{}\" LabelJustification=\"{}\" CaptionJustification=\"{}\" PrintMargins=\"{}\" color=\"{}\" bgcolor=\"{}\"",
            xml_escape_attr(&self.document.document.title),
            root_bbox,
            fmt_cdxml_bool(self.defaults.fractional_widths),
            fmt_cdxml_bool(self.defaults.interpret_chemically.unwrap_or(true)),
            fmt_cdxml_bool(self.defaults.show_atom_query),
            fmt_cdxml_bool(self.defaults.show_atom_stereo),
            fmt_cdxml_bool(self.defaults.show_atom_enhanced_stereo),
            fmt_cdxml_bool(self.defaults.show_atom_number),
            fmt_cdxml_bool(self.defaults.show_residue_id),
            fmt_cdxml_bool(self.defaults.show_bond_query),
            fmt_cdxml_bool(self.defaults.show_bond_rxn),
            fmt_cdxml_bool(self.defaults.show_bond_stereo),
            fmt_cdxml_bool(self.defaults.show_terminal_carbon_labels),
            fmt_cdxml_bool(self.defaults.show_non_terminal_carbon_labels),
            fmt_cdxml_bool(self.defaults.hide_implicit_hydrogens),
            self.defaults.label_font,
            fmt_num(self.defaults.label_size),
            self.defaults.label_face,
            self.defaults.caption_font,
            fmt_num(self.defaults.caption_size),
            self.defaults.caption_face,
            fmt_num(self.defaults.line_width),
            fmt_num(self.defaults.bold_width),
            fmt_num(self.defaults.bond_length),
            fmt_num(self.defaults.bond_spacing),
            fmt_num(self.defaults.hash_spacing),
            fmt_num(self.defaults.margin_width),
            fmt_num(self.defaults.chain_angle),
            self.defaults.label_justification.as_cdxml(),
            self.defaults.caption_justification.as_cdxml(),
            fmt_margins(self.defaults.print_margins),
            self.defaults.color,
            self.colors.background_id(),
        )
        .expect("writing CDXML root should not fail");
        for (name, xml_name) in [
            ("lineHeight", "LineHeight"),
            ("labelLineHeight", "LabelLineHeight"),
            ("captionLineHeight", "CaptionLineHeight"),
        ] {
            if let Some(value) = self
                .document
                .document
                .meta
                .pointer(&format!("/import/cdxml/defaults/{name}"))
                .and_then(Value::as_str)
            {
                write!(out, " {xml_name}=\"{}\"", xml_escape_attr(value))
                    .expect("writing CDXML line-height default should not fail");
            }
        }
        out.push_str(">\n");
        self.write_color_table(&mut out);
        self.write_font_table(&mut out);
        write!(
            out,
            "  <page id=\"{}\" BoundingBox=\"{}\" HeaderPosition=\"36\" FooterPosition=\"36\" PrintTrimMarks=\"yes\" HeightPages=\"1\" WidthPages=\"1\" Width=\"{}\" Height=\"{}\">\n",
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
        self.write_scene_objects(&mut out, &objects);

        out.push_str("  </page>\n");
        out.push_str("</CDXML>\n");
        out
    }

    fn write_scene_object(&mut self, out: &mut String, object: &SceneObject) {
        match object.object_type.as_str() {
            "molecule" => self.write_molecule_object(out, object),
            "line" => self.write_line_object(out, object),
            "shape" => self.write_shape_object(out, object),
            "bracket" | "symbol" => self.write_bracket_object(out, object),
            "text" => self.write_text_object(out, object),
            "group" => self.write_group_object(out, object),
            _ => {}
        }
    }

    fn write_scene_objects(&mut self, out: &mut String, objects: &[&SceneObject]) {
        let mut emitted = std::collections::BTreeSet::new();
        for object in objects {
            if emitted.contains(&object.id) {
                continue;
            }
            if object.object_type == "molecule" {
                let scope = cdxml_bond_crossing_scope(object);
                if scope.starts_with("cdxml-fragment:") {
                    let components: Vec<_> = objects
                        .iter()
                        .copied()
                        .filter(|candidate| {
                            candidate.object_type == "molecule"
                                && cdxml_bond_crossing_scope(candidate) == scope
                        })
                        .collect();
                    if components.len() > 1 {
                        emitted.extend(components.iter().map(|component| component.id.clone()));
                        self.write_molecule_objects_as_fragment(out, &components);
                        continue;
                    }
                }
            }
            emitted.insert(object.id.clone());
            self.write_scene_object(out, object);
        }
    }

    fn write_group_object(&mut self, out: &mut String, object: &SceneObject) {
        if object.children.is_empty() {
            return;
        }
        if object.meta.get("kind").and_then(Value::as_str) == Some("bracket-group") {
            self.write_scene_object_children(out, object);
            return;
        }
        let mut scratch = self.document.clone();
        scratch.objects = object.children.clone();
        let bbox = crate::render_primitives_bounds(crate::render_document(&scratch).iter())
            .or(object.payload.bbox.map(|bbox| {
                [
                    object.transform.translate[0] + bbox[0],
                    object.transform.translate[1] + bbox[1],
                    object.transform.translate[0] + bbox[0] + bbox[2],
                    object.transform.translate[1] + bbox[1] + bbox[3],
                ]
            }))
            .unwrap_or([
                object.transform.translate[0],
                object.transform.translate[1],
                object.transform.translate[0] + 1.0,
                object.transform.translate[1] + 1.0,
            ]);
        writeln!(
            out,
            "    <group id=\"{}\" BoundingBox=\"{}\" Z=\"{}\">",
            self.alloc_id(),
            fmt_bbox(bbox),
            object.z_index
        )
        .expect("writing group should not fail");

        self.write_scene_object_children(out, object);
        out.push_str("    </group>\n");
    }

    fn write_scene_object_children(&mut self, out: &mut String, object: &SceneObject) {
        let mut children: Vec<&SceneObject> = object
            .children
            .iter()
            .filter(|child| child.visible)
            .collect();
        children.sort_by(|a, b| a.z_index.cmp(&b.z_index).then_with(|| a.id.cmp(&b.id)));
        self.write_scene_objects(out, &children);
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

    fn write_font_table(&self, out: &mut String) {
        out.push_str("  <fonttable>\n");
        for (id, name) in self.fonts.fonts() {
            writeln!(
                out,
                "    <font id=\"{}\" charset=\"iso-8859-1\" name=\"{}\"/>",
                id,
                xml_escape_attr(name),
            )
            .expect("writing font table should not fail");
        }
        out.push_str("  </fonttable>\n");
    }

    fn write_molecule_object(&mut self, out: &mut String, object: &SceneObject) {
        self.write_molecule_objects_as_fragment(out, &[object]);
    }

    fn write_molecule_objects_as_fragment(&mut self, out: &mut String, objects: &[&SceneObject]) {
        let components: Vec<_> = objects
            .iter()
            .filter_map(|object| {
                object
                    .payload
                    .resource_ref
                    .as_ref()
                    .and_then(|resource_ref| self.document.resources.get(resource_ref))
                    .and_then(|resource| resource.data.as_fragment())
                    .map(|fragment| (*object, fragment))
            })
            .filter(|(_, fragment)| !fragment.nodes.is_empty())
            .collect();
        if components.is_empty() {
            return;
        }

        let fragment_id = self.alloc_id();
        let bbox = components
            .iter()
            .filter_map(|(object, fragment)| molecule_world_bbox(object, fragment))
            .reduce(|left, right| {
                [
                    left[0].min(right[0]),
                    left[1].min(right[1]),
                    left[2].max(right[2]),
                    left[3].max(right[3]),
                ]
            })
            .unwrap_or([0.0, 0.0, 1.0, 1.0]);
        let z_index = components
            .iter()
            .map(|(object, _)| object.z_index)
            .min()
            .unwrap_or(10);
        writeln!(
            out,
            "    <fragment id=\"{}\" BoundingBox=\"{}\" Z=\"{}\">",
            fragment_id,
            fmt_bbox(bbox),
            z_index
        )
        .expect("writing fragment should not fail");

        let mut node_ids = BTreeMap::new();
        for (_, fragment) in &components {
            for node in &fragment.nodes {
                node_ids.insert(node.id.clone(), self.alloc_id());
            }
        }
        self.node_ids.extend(node_ids.clone());
        for (object, fragment) in &components {
            for node in &fragment.nodes {
                self.write_node(out, object, node, &node_ids[&node.id]);
            }
        }
        for (object, fragment) in &components {
            let crossing_scope = cdxml_bond_crossing_scope(object);
            for bond in &fragment.bonds {
                let Some(cdxml_id) = self
                    .bond_ids
                    .get(&(crossing_scope.clone(), bond.id.clone()))
                    .cloned()
                else {
                    continue;
                };
                self.write_bond(out, bond, &cdxml_id, &node_ids, &crossing_scope);
            }
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
        let is_nickname = node.is_placeholder;
        let mut attrs = vec![("id", cdxml_id.to_string()), ("p", fmt_point(point))];
        attrs.push(("Z", object.z_index.to_string()));
        if !is_plain_carbon && node.atomic_number > 0 && (!is_nickname || node.atomic_number != 6) {
            attrs.push(("Element", node.atomic_number.to_string()));
        }
        let imported_node_type = node
            .meta
            .pointer("/import/cdxml/nodeType")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty());
        if node.is_external_connection_point {
            attrs.push(("NodeType", "ExternalConnectionPoint".to_string()));
        } else if let Some(node_type) = imported_node_type {
            attrs.push(("NodeType", node_type.to_string()));
        } else if is_nickname {
            attrs.push(("NodeType", "Nickname".to_string()));
        }
        if let Some(element_list) = node
            .meta
            .pointer("/import/cdxml/elementList")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        {
            attrs.push(("ElementList", element_list.to_string()));
        }
        if let Some(label) = node.label.as_ref() {
            if let Some(display) = imported_cdxml_label_attr(label, "labelDisplay") {
                attrs.push(("LabelDisplay", display.to_string()));
            } else if label.layout.as_deref() == Some("attached-group-center")
                && label.meta.pointer("/import/cdxml").is_none()
            {
                attrs.push(("LabelDisplay", "Center".to_string()));
            }
        }
        if node.charge != 0 {
            attrs.push(("Charge", node.charge.to_string()));
        }
        if let Some(num_hydrogens) = cdxml_node_num_hydrogens_for_export(node) {
            attrs.push(("NumHydrogens", num_hydrogens.to_string()));
        }
        if let Some(implicit_hydrogens) = node
            .meta
            .pointer("/import/cdxml/implicitHydrogens")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        {
            attrs.push(("ImplicitHydrogens", implicit_hydrogens.to_string()));
        }
        attrs.push(("AS", "N".to_string()));
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
        let Some(font_size) = label.font_size else {
            return;
        };
        let position = label
            .position
            .map(|position| object_local_point(object, position))
            .unwrap_or_else(|| object_local_point(object, node.position));
        let Some(bbox) = label
            .bbox()
            .map(|bbox| translate_bbox(bbox, object.transform.translate))
        else {
            return;
        };
        let label_alignment = imported_cdxml_label_attr(label, "labelAlignment")
            .unwrap_or_else(|| cdxml_node_label_alignment(label));
        let label_justification = imported_cdxml_label_attr(label, "labelJustification")
            .unwrap_or_else(|| cdxml_justification(label.align.as_deref()));
        let mut attrs = vec![
            ("id", self.alloc_id()),
            ("p", fmt_point(position)),
            ("BoundingBox", fmt_bbox(bbox)),
            ("LabelAlignment", label_alignment.to_string()),
            ("LabelJustification", label_justification.to_string()),
            (
                "InterpretChemically",
                if cdxml_node_label_interpret_chemically(label) {
                    "yes".to_string()
                } else {
                    "no".to_string()
                },
            ),
            ("UTF8Text", text.to_string()),
        ];
        if let Some(justification) = imported_cdxml_label_attr(label, "justification") {
            attrs.push(("Justification", justification.to_string()));
        }
        for (name, xml_name) in [
            ("lineHeight", "LineHeight"),
            ("labelLineHeight", "LabelLineHeight"),
            ("wordWrapWidth", "WordWrapWidth"),
        ] {
            if let Some(value) = imported_cdxml_label_attr(label, name) {
                attrs.push((xml_name, value.to_string()));
            }
        }
        if let Some(line_starts) = imported_cdxml_label_attr(label, "lineStarts") {
            attrs.push(("LineStarts", line_starts.to_string()));
        } else if let Some(line_starts) = cdxml_label_line_starts(label) {
            attrs.push(("LineStarts", line_starts));
        }
        write_open_tag(out, 8, "t", attrs);
        self.write_label_runs(out, 10, label, text, font_size);
        out.push_str("        </t>\n");
    }

    fn write_bond(
        &mut self,
        out: &mut String,
        bond: &Bond,
        cdxml_id: &str,
        node_ids: &BTreeMap<String, String>,
        crossing_scope: &str,
    ) {
        let (Some(begin), Some(end)) = (node_ids.get(&bond.begin), node_ids.get(&bond.end)) else {
            return;
        };
        let mut attrs = vec![
            ("id", cdxml_id.to_string()),
            (
                "Z",
                bond.meta
                    .pointer("/import/cdxml/z")
                    .and_then(Value::as_i64)
                    .unwrap_or(1)
                    .to_string(),
            ),
            ("B", begin.clone()),
            ("E", end.clone()),
            (
                "Order",
                preserved_cdxml_bond_order(bond).unwrap_or_else(|| bond.order.max(1).to_string()),
            ),
            ("BS", "N".to_string()),
        ];
        let crossing_bonds: Vec<_> = imported_cdxml_crossing_bonds(bond)
            .filter_map(|source_id| {
                self.bond_ids
                    .get(&(crossing_scope.to_string(), source_id.to_string()))
                    .cloned()
            })
            .collect();
        if !crossing_bonds.is_empty() {
            attrs.push(("CrossingBonds", crossing_bonds.join(" ")));
        }
        if let Some(value) = bond_endpoint_attachment(bond, "begin") {
            attrs.push(("BeginAttach", value.to_string()));
        }
        if let Some(value) = bond_endpoint_attachment(bond, "end") {
            attrs.push(("EndAttach", value.to_string()));
        }
        if bond
            .meta
            .pointer("/import/cdxml/aromatic")
            .and_then(Value::as_bool)
            == Some(true)
        {
            attrs.push(("Display", "Dash".to_string()));
        } else if let Some(display) = cdxml_bond_display(bond, false) {
            attrs.push(("Display", display.to_string()));
        }
        if let Some(display2) = cdxml_bond_display(bond, true) {
            attrs.push(("Display2", display2.to_string()));
        }
        if let Some(stroke) = &bond.stroke {
            attrs.push(("color", self.colors.id_for(stroke)));
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
        if let Some(value) = bond.margin_width {
            attrs.push(("MarginWidth", fmt_num(value)));
        }
        write_empty_tag(out, 6, "b", attrs);
    }

    fn prepare_bond_ids(&mut self) {
        let mut keys = Vec::new();
        collect_cdxml_bond_export_keys(self.document, &self.document.objects, &mut keys);
        for key in keys {
            if !self.bond_ids.contains_key(&key) {
                let cdxml_id = self.alloc_id();
                self.bond_ids.insert(key, cdxml_id);
            }
        }
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
        let head_position = cdxml_arrow_endpoint_position(&object.payload, arrow, "head", "end");
        let tail_position = cdxml_arrow_endpoint_position(&object.payload, arrow, "tail", "start");
        let has_head = head_position != "None";
        let has_tail = tail_position != "None";
        let style = object_style(self.document, object);
        let stroke = style
            .and_then(|style| style_string_value(style, "stroke"))
            .unwrap_or_else(|| "#000000".to_string());
        let stroke_width = style
            .and_then(|style| style_number_value(style, "strokeWidth"))
            .unwrap_or(crate::DEFAULT_BOND_STROKE);
        let dashed = style
            .and_then(|style| style_number_array(style, "dashArray"))
            .is_some_and(|dash_array| !dash_array.is_empty());
        let mut attrs = vec![
            ("id", self.alloc_id()),
            ("Head3D", fmt_point3(head)),
            ("Tail3D", fmt_point3(tail)),
            ("LineWidth", fmt_num(stroke_width)),
            ("color", self.colors.id_for(&stroke)),
            ("Z", object.z_index.to_string()),
        ];
        let is_arrow = arrow.is_some()
            || object
                .meta
                .pointer("/import/cdxml/kind")
                .and_then(Value::as_str)
                == Some("arrow");
        if is_arrow || has_head || has_tail {
            let bold = arrow
                .and_then(|value| value.get("bold"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            match (bold, dashed) {
                (true, true) => attrs.push(("LineType", "Bold Dashed".to_string())),
                (true, false) => attrs.push(("LineType", "Bold".to_string())),
                (false, true) => attrs.push(("LineType", "Dashed".to_string())),
                (false, false) => {}
            }
            if let Some(fill_type) = arrow
                .and_then(|value| value.get("fillType").or_else(|| value.get("fill_type")))
                .and_then(Value::as_str)
                .and_then(cdxml_arrow_fill_type)
            {
                attrs.push(("FillType", fill_type.to_string()));
            }
            if let Some(bbox) =
                payload_nested_bbox_cdxml(&object.payload, "arrowGeometry", "boundingBox")
            {
                attrs.push((
                    "BoundingBox",
                    fmt_bbox(translate_bbox(bbox, object.transform.translate)),
                ));
            }
            if let Some(center) =
                payload_nested_point_cdxml(&object.payload, "arrowGeometry", "center")
            {
                attrs.push((
                    "Center3D",
                    fmt_point3(center.translated(crate::Vector::new(
                        object.transform.translate[0],
                        object.transform.translate[1],
                    ))),
                ));
            }
            if let Some(major) =
                payload_nested_point_cdxml(&object.payload, "arrowGeometry", "majorAxisEnd")
            {
                attrs.push((
                    "MajorAxisEnd3D",
                    fmt_point3(major.translated(crate::Vector::new(
                        object.transform.translate[0],
                        object.transform.translate[1],
                    ))),
                ));
            }
            if let Some(minor) =
                payload_nested_point_cdxml(&object.payload, "arrowGeometry", "minorAxisEnd")
            {
                attrs.push((
                    "MinorAxisEnd3D",
                    fmt_point3(minor.translated(crate::Vector::new(
                        object.transform.translate[0],
                        object.transform.translate[1],
                    ))),
                ));
            }
            attrs.push(("ArrowheadHead", head_position.to_string()));
            attrs.push(("ArrowheadTail", tail_position.to_string()));
            let arrow_kind = cdxml_arrow_kind(arrow);
            attrs.push((
                "ArrowheadType",
                cdxml_arrowhead_type_attr(arrow_kind).to_string(),
            ));
            if let Some(value) = arrow
                .and_then(|value| value.get("length"))
                .and_then(Value::as_f64)
            {
                attrs.push(("HeadSize", fmt_num(cdxml_arrow_size_attribute(value))));
            }
            if let Some(value) = arrow
                .and_then(|value| {
                    value
                        .get("centerLength")
                        .or_else(|| value.get("center_length"))
                })
                .and_then(Value::as_f64)
            {
                let value = cdxml_arrow_size_attribute(value);
                attrs.push(("ArrowheadCenterSize", fmt_num(value)));
            }
            if arrow_kind == "Equilibrium" {
                let value = arrow
                    .and_then(|value| {
                        value
                            .get("shaftSpacing")
                            .or_else(|| value.get("shaft_spacing"))
                    })
                    .and_then(Value::as_f64)
                    .unwrap_or(3.0);
                let value = cdxml_arrow_size_attribute(value);
                attrs.push(("ArrowShaftSpacing", fmt_num(value)));
                if let Some(value) = cdxml_arrow_equilibrium_ratio(arrow) {
                    attrs.push(("ArrowEquilibriumRatio", fmt_num(value * 100.0)));
                }
            } else if let Some(value) = arrow
                .and_then(|value| {
                    value
                        .get("shaftSpacing")
                        .or_else(|| value.get("shaft_spacing"))
                })
                .and_then(Value::as_f64)
                .filter(|value| value.is_finite() && *value >= 0.0)
            {
                attrs.push((
                    "ArrowShaftSpacing",
                    fmt_num(cdxml_arrow_size_attribute(value)),
                ));
            }
            if let Some(value) = arrow
                .and_then(|value| value.get("width"))
                .and_then(Value::as_f64)
            {
                attrs.push(("ArrowheadWidth", fmt_num(cdxml_arrow_size_attribute(value))));
            }
            if let Some(value) = arrow
                .and_then(|value| value.get("curve"))
                .and_then(Value::as_f64)
                .filter(|value| value.abs() > crate::EPSILON)
            {
                attrs.push(("AngularSize", fmt_num(value)));
            }
            if let Some(value) = arrow
                .and_then(|value| {
                    value
                        .get("curveSpacing")
                        .or_else(|| value.get("curve_spacing"))
                })
                .and_then(Value::as_f64)
                .filter(|value| value.is_finite() && *value >= 0.0)
            {
                attrs.push(("CurveSpacing", fmt_num(cdxml_arrow_size_attribute(value))));
            }
            if let Some(value) = arrow
                .and_then(|value| value.get("noGo").or_else(|| value.get("no_go")))
                .and_then(Value::as_str)
                .and_then(cdxml_arrow_no_go)
            {
                attrs.push(("NoGo", value.to_string()));
            }
            if arrow
                .and_then(|value| value.get("dipole"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                attrs.push(("Dipole", "yes".to_string()));
            }
            if arrow
                .and_then(|value| value.get("closed"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                attrs.push(("Closed", "yes".to_string()));
            }
            if let Some(value) = arrow
                .and_then(|value| value.get("source"))
                .and_then(cdxml_arrow_object_reference)
            {
                attrs.push(("ArrowSource", value));
            }
            if let Some(value) = arrow
                .and_then(|value| value.get("target"))
                .and_then(cdxml_arrow_object_reference)
            {
                attrs.push(("ArrowTarget", value));
            }
            write_empty_tag(out, 4, "arrow", attrs);
        } else {
            if dashed {
                attrs.push(("LineType", "Dashed".to_string()));
            }
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
            let Some(center) = payload_point_cdxml(&object.payload, "center") else {
                return;
            };
            let Some(major) = payload_point_cdxml(&object.payload, "majorAxisEnd") else {
                return;
            };
            let Some(minor) = payload_point_cdxml(&object.payload, "minorAxisEnd") else {
                return;
            };
            let bbox = [x, y, x + width, y + height];
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
        if kind == "orbital" {
            self.write_orbital_shape_object(out, object, color, style);
            return;
        }
        if kind == "crossTable" {
            let left = bbox[0];
            let top = bbox[1];
            let mid_x = left + width * 0.5;
            let mid_y = top + height * 0.5;
            let right = bbox[2];
            let bottom = bbox[3];
            let cell_bounds = [
                [left, top, mid_x, mid_y],
                [mid_x, top, right, mid_y],
                [left, mid_y, mid_x, bottom],
                [mid_x, mid_y, right, bottom],
            ];
            let table_id = self.alloc_id();
            let color_id = self.colors.id_for(color);
            write_open_tag(
                out,
                4,
                "table",
                vec![
                    ("id", table_id),
                    ("BoundingBox", fmt_bbox(bbox)),
                    ("color", color_id.clone()),
                    ("Z", object.z_index.to_string()),
                ],
            );
            for bounds in cell_bounds {
                write_empty_tag(
                    out,
                    6,
                    "page",
                    vec![
                        ("id", self.alloc_id()),
                        ("BoundingBox", fmt_bbox(bounds)),
                        ("HeaderPosition", "36".to_string()),
                        ("FooterPosition", "36".to_string()),
                        ("PrintTrimMarks", "yes".to_string()),
                        ("HeightPages", "1".to_string()),
                        ("WidthPages", "1".to_string()),
                        ("BoundsInParent", fmt_bbox(bounds)),
                    ],
                );
            }
            write_indent(out, 4);
            out.push_str("</table>\n");
            return;
        }
        if kind == "tlcPlate" {
            let plate_id = self.alloc_id();
            let color_id = self.colors.id_for(color);
            let origin_fraction = object
                .payload
                .extra
                .get("originFraction")
                .and_then(Value::as_f64)
                .unwrap_or(0.1);
            let solvent_fraction = object
                .payload
                .extra
                .get("solventFrontFraction")
                .and_then(Value::as_f64)
                .unwrap_or(0.1);
            let bool_attr = |key: &str, default_value: bool| {
                if object
                    .payload
                    .extra
                    .get(key)
                    .and_then(Value::as_bool)
                    .unwrap_or(default_value)
                {
                    "yes".to_string()
                } else {
                    "no".to_string()
                }
            };
            write_open_tag(
                out,
                4,
                "tlcplate",
                vec![
                    ("id", plate_id),
                    ("OriginFraction", fmt_num(origin_fraction)),
                    ("SolventFrontFraction", fmt_num(solvent_fraction)),
                    ("ShowOrigin", bool_attr("showOrigin", true)),
                    ("ShowSolventFront", bool_attr("showSolventFront", true)),
                    ("TopLeft", fmt_point(Point::new(bbox[0], bbox[1]))),
                    ("TopRight", fmt_point(Point::new(bbox[2], bbox[1]))),
                    ("BottomRight", fmt_point(Point::new(bbox[2], bbox[3]))),
                    ("BottomLeft", fmt_point(Point::new(bbox[0], bbox[3]))),
                    ("ShowBorders", bool_attr("showBorders", true)),
                    ("ShowSideTicks", bool_attr("showSideTicks", true)),
                    ("BoundingBox", fmt_bbox(bbox)),
                    ("Z", object.z_index.to_string()),
                    ("color", color_id.clone()),
                ],
            );
            if let Some(lanes) = object.payload.extra.get("lanes").and_then(Value::as_array) {
                for lane in lanes {
                    write_open_tag(out, 6, "tlclane", vec![("id", self.alloc_id())]);
                    if let Some(spots) = lane.get("spots").and_then(Value::as_array) {
                        for spot in spots {
                            let mut attrs = vec![
                                ("id", self.alloc_id()),
                                (
                                    "Rf",
                                    fmt_num(spot.get("rf").and_then(Value::as_f64).unwrap_or(0.15)),
                                ),
                                (
                                    "Tail",
                                    fmt_num(
                                        spot.get("tail").and_then(Value::as_f64).unwrap_or(0.0),
                                    ),
                                ),
                                (
                                    "Width",
                                    fmt_num(self.cdxml_tlc_spot_extent(
                                        spot.get("width").and_then(Value::as_f64),
                                    )),
                                ),
                                (
                                    "Height",
                                    fmt_num(self.cdxml_tlc_spot_extent(
                                        spot.get("height").and_then(Value::as_f64),
                                    )),
                                ),
                                (
                                    "CurveType",
                                    spot.get("curveType")
                                        .and_then(Value::as_i64)
                                        .unwrap_or(128)
                                        .to_string(),
                                ),
                                ("color", color_id.clone()),
                            ];
                            if spot.get("showRf").and_then(Value::as_bool).unwrap_or(false) {
                                attrs.push(("ShowRf", "yes".to_string()));
                            }
                            write_empty_tag(out, 8, "tlcspot", attrs);
                        }
                    }
                    write_indent(out, 6);
                    out.push_str("</tlclane>\n");
                }
            }
            write_indent(out, 4);
            out.push_str("</tlcplate>\n");
            return;
        }
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

    fn write_orbital_shape_object(
        &mut self,
        out: &mut String,
        object: &SceneObject,
        color: &str,
        style: Option<&Value>,
    ) {
        let template = payload_string_cdxml(&object.payload, "orbitalTemplate")
            .unwrap_or_else(|| "s".to_string());
        let render_style = payload_string_cdxml(&object.payload, "orbitalStyle")
            .unwrap_or_else(|| "hollow".to_string());
        let phase = payload_string_cdxml(&object.payload, "orbitalPhase")
            .unwrap_or_else(|| "plus".to_string());
        let orbital_type = cdxml_orbital_type(&template, &render_style, &phase);
        let mut attrs = vec![
            ("id", self.alloc_id()),
            ("GraphicType", "Orbital".to_string()),
            ("OrbitalType", orbital_type.to_string()),
            ("color", self.colors.id_for(color)),
            ("Z", object.z_index.to_string()),
        ];
        if matches!(template.as_str(), "s" | "oval") {
            let Some(center) = payload_point_cdxml(&object.payload, "center") else {
                return;
            };
            let Some(major) = payload_point_cdxml(&object.payload, "majorAxisEnd") else {
                return;
            };
            let Some(minor) = payload_point_cdxml(&object.payload, "minorAxisEnd") else {
                return;
            };
            let radius_x = center.distance(major);
            let radius_y = center.distance(minor);
            let bbox = [
                center.x - radius_x,
                center.y - radius_y,
                center.x + radius_x,
                center.y + radius_y,
            ];
            attrs.push(("BoundingBox", fmt_bbox(bbox)));
            attrs.push(("Center3D", fmt_point3(center)));
            attrs.push(("MajorAxisEnd3D", fmt_point3(major)));
            attrs.push(("MinorAxisEnd3D", fmt_point3(minor)));
            if template == "s" {
                let oval_type = match render_style.as_str() {
                    "shaded" => "Circle Shaded",
                    "filled" => "Circle Filled",
                    _ => "Circle",
                };
                attrs.push(("OvalType", oval_type.to_string()));
            } else {
                let oval_type = match render_style.as_str() {
                    "shaded" => "Shaded",
                    "filled" => "Filled",
                    _ => "",
                };
                if !oval_type.is_empty() {
                    attrs.push(("OvalType", oval_type.to_string()));
                }
            }
            write_empty_tag(out, 4, "graphic", attrs);
            return;
        }
        let Some(start) = payload_point_cdxml(&object.payload, "axisStart") else {
            return;
        };
        let Some(end) = payload_point_cdxml(&object.payload, "axisEnd") else {
            return;
        };
        attrs.push(("BoundingBox", fmt_bbox([end.x, end.y, start.x, start.y])));
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
            let color = payload_string_cdxml(&object.payload, "fill")
                .unwrap_or_else(|| "#000000".to_string());
            let color_id = self.colors.id_for(&color);
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
            let symbol_bbox =
                cdxml_symbol_anchor_bbox(center_x, center_y, anchor_width, anchor_height);
            let attrs = vec![
                ("id", self.alloc_id()),
                ("GraphicType", "Symbol".to_string()),
                ("SymbolType", symbol_type.to_string()),
                ("color", color_id),
                ("BoundingBox", fmt_bbox(symbol_bbox)),
                ("Z", object.z_index.to_string()),
            ];
            let represented_node = object
                .payload
                .extra
                .get("attachedAtomId")
                .and_then(Value::as_str)
                .and_then(|node_id| self.node_ids.get(node_id));
            let represented_attribute = object
                .payload
                .extra
                .get("representAttribute")
                .and_then(Value::as_str);
            if let (Some(node_id), Some(attribute)) = (represented_node, represented_attribute) {
                write_open_tag(out, 4, "graphic", attrs);
                write_empty_tag(
                    out,
                    6,
                    "represent",
                    vec![
                        ("attribute", attribute.to_string()),
                        ("object", node_id.clone()),
                    ],
                );
                out.push_str("    </graphic>\n");
            } else {
                write_empty_tag(out, 4, "graphic", attrs);
            }
            return;
        }

        let color = payload_string_cdxml(&object.payload, "stroke")
            .unwrap_or_else(|| "#000000".to_string());
        let color_id = self.colors.id_for(&color);
        let bracket_type = match kind.as_str() {
            "square" => "Square",
            "curly" => "Curly",
            _ => "Round",
        };
        if let Some(side) = object.payload.extra.get("side").and_then(Value::as_str) {
            let bracket_x = match (kind.as_str(), side) {
                ("round", "right") => bbox[0],
                ("round", _) => bbox[2],
                (_, "right") => bbox[2],
                _ => bbox[0],
            };
            let bracket_bbox = match side {
                "right" => [bracket_x, bbox[1], bracket_x, bbox[3]],
                _ => [bracket_x, bbox[3], bracket_x, bbox[1]],
            };
            write_empty_tag(
                out,
                4,
                "graphic",
                vec![
                    ("id", self.alloc_id()),
                    ("GraphicType", "Bracket".to_string()),
                    ("BracketType", bracket_type.to_string()),
                    ("color", color_id),
                    ("BoundingBox", fmt_bbox(bracket_bbox)),
                    ("LipSize", "60".to_string()),
                    ("Z", object.z_index.to_string()),
                ],
            );
            return;
        }
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
                ("color", color_id.clone()),
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
                ("color", color_id),
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
        let Some(font_size) = object
            .payload
            .extra
            .get("fontSize")
            .and_then(Value::as_f64)
            .or_else(|| style.and_then(|style| style_number_value(style, "fontSize")))
        else {
            return;
        };
        let color = style
            .and_then(|style| style_nullable_string_value(style, "fill"))
            .unwrap_or_else(|| "#000000".to_string());
        let font_family = style
            .and_then(|style| style_string_value(style, "fontFamily"))
            .unwrap_or_else(|| "Arial".to_string());
        let Some(box_value) = payload_bbox_cdxml(&object.payload, "box").or(object.payload.bbox)
        else {
            return;
        };
        let baseline_offset = object
            .payload
            .extra
            .get("baselineOffset")
            .and_then(Value::as_f64)
            .unwrap_or(font_size * 0.82);
        let anchor_offset_x = object
            .payload
            .extra
            .get("anchorOffsetX")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let anchor = Point::new(
            object.transform.translate[0] + anchor_offset_x,
            object.transform.translate[1] + baseline_offset,
        );
        let bbox = [
            object.transform.translate[0] + box_value[0],
            object.transform.translate[1] + box_value[1],
            object.transform.translate[0] + box_value[0] + box_value[2],
            object.transform.translate[1] + box_value[1] + box_value[3],
        ];
        let mut attrs = vec![
            ("id", self.alloc_id()),
            ("p", fmt_point(anchor)),
            ("BoundingBox", fmt_bbox(bbox)),
            (
                "CaptionJustification",
                cdxml_justification(payload_string_cdxml(&object.payload, "align").as_deref())
                    .to_string(),
            ),
            ("Z", object.z_index.to_string()),
            ("UTF8Text", text.clone()),
        ];
        for (name, xml_name) in [
            ("justification", "Justification"),
            ("lineHeight", "LineHeight"),
            ("captionLineHeight", "CaptionLineHeight"),
            ("wordWrapWidth", "WordWrapWidth"),
            ("lineStarts", "LineStarts"),
        ] {
            if let Some(value) = imported_cdxml_object_attr(object, name) {
                attrs.push((xml_name, value.to_string()));
            }
        }
        let inherited_caption_line_height = self
            .document
            .document
            .meta
            .pointer("/import/cdxml/defaults/captionLineHeight")
            .and_then(Value::as_str);
        let should_materialize_caption_line_height = object.meta.pointer("/import/cdxml").is_none()
            || (imported_cdxml_object_attr(object, "lineHeight").is_some()
                && inherited_caption_line_height.is_none());
        if imported_cdxml_object_attr(object, "captionLineHeight").is_none()
            && should_materialize_caption_line_height
        {
            if let Some(line_height) = object
                .payload
                .extra
                .get("lineHeight")
                .and_then(Value::as_f64)
            {
                attrs.push((
                    "CaptionLineHeight",
                    line_height.round().clamp(0.0, i16::MAX as f64).to_string(),
                ));
            }
        }
        write_open_tag(out, 4, "t", attrs);
        let runs = object
            .payload
            .extra
            .get("runs")
            .cloned()
            .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
            .unwrap_or_default();
        self.write_runs(out, 6, &runs, &text, font_size, &color, &font_family);
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
        let source_runs = label_source_runs_for_export(label);
        let runs = source_runs.as_deref().unwrap_or(&label.runs);
        self.write_runs(
            out,
            indent,
            runs,
            fallback,
            fallback_size,
            label.fill.as_deref().unwrap_or("#000000"),
            label.font_family.as_deref().unwrap_or("Arial"),
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
        fallback_font_family: &str,
    ) {
        if runs.is_empty() {
            let attrs = vec![
                ("font", self.fonts.id_for(fallback_font_family)),
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
            if run.outline.unwrap_or(false) {
                face |= 8;
            }
            if run.shadow.unwrap_or(false) {
                face |= 16;
            }
            match run.script.as_deref() {
                Some("subscript") => face |= 32,
                Some("superscript") => face |= 64,
                Some("chemical") => face |= 96,
                _ => {}
            }
            let mut attrs = vec![
                (
                    "font",
                    self.fonts
                        .id_for(run.font_family.as_deref().unwrap_or(fallback_font_family)),
                ),
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

    fn cdxml_tlc_spot_extent(&self, extent: Option<f64>) -> f64 {
        let Some(extent) = extent else {
            return 327680.0;
        };
        if extent > 1024.0 {
            return extent;
        }
        (extent / self.editing_scale.max(crate::EPSILON) * 65536.0).round()
    }
}

fn preserved_cdxml_bond_order(bond: &Bond) -> Option<String> {
    let source = bond
        .meta
        .pointer("/import/cdxml/order")
        .and_then(Value::as_str)?;
    let aromatic = bond
        .meta
        .pointer("/import/cdxml/aromatic")
        .and_then(Value::as_bool)
        == Some(true);
    if (aromatic && source == "1.5") || (bond.order == 1 && source.eq_ignore_ascii_case("dative")) {
        Some(source.to_string())
    } else {
        None
    }
}

#[derive(Debug, Clone)]
struct CdxmlFontTable {
    fonts: Vec<(String, String)>,
    ids: BTreeMap<String, String>,
    next_id: u64,
}

impl Default for CdxmlFontTable {
    fn default() -> Self {
        let mut table = Self {
            fonts: Vec::new(),
            ids: BTreeMap::new(),
            next_id: 4,
        };
        table.insert_with_id("3", "Arial");
        table
    }
}

impl CdxmlFontTable {
    fn normalize_name(name: &str) -> String {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            "Arial".to_string()
        } else {
            trimmed.to_string()
        }
    }

    fn insert_with_id(&mut self, id: &str, name: &str) {
        let normalized = Self::normalize_name(name);
        self.ids.insert(normalized.clone(), id.to_string());
        self.fonts.push((id.to_string(), normalized));
    }

    fn ensure(&mut self, name: &str) -> String {
        let normalized = Self::normalize_name(name);
        if let Some(id) = self.ids.get(&normalized) {
            return id.clone();
        }
        let id = self.next_id.to_string();
        self.next_id += 1;
        self.insert_with_id(&id, &normalized);
        id
    }

    fn id_for(&self, name: &str) -> String {
        let normalized = Self::normalize_name(name);
        self.ids
            .get(&normalized)
            .cloned()
            .unwrap_or_else(|| "3".to_string())
    }

    fn fonts(&self) -> &[(String, String)] {
        &self.fonts
    }
}

fn collect_document_colors(document: &ChemSemaDocument, colors: &mut CdxmlColorTable) {
    colors.ensure(&document.document.page.background);
    colors.ensure(&document.style.label_style.fill);
    colors.ensure(&document.style.caption_style.fill);
    if let Some(foreground) = document
        .document
        .meta
        .pointer("/import/cdxml/defaults/foregroundColor")
        .and_then(Value::as_str)
    {
        colors.ensure(foreground);
    }
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
        for bond in &fragment.bonds {
            if let Some(stroke) = &bond.stroke {
                colors.ensure(stroke);
            }
        }
    }
}

fn collect_document_fonts(document: &ChemSemaDocument, fonts: &mut CdxmlFontTable) {
    fonts.ensure(&document.style.label_style.font_family);
    fonts.ensure(&document.style.caption_style.font_family);
    for style in document.styles.values() {
        if let Some(font_family) = style_string_value(style, "fontFamily") {
            fonts.ensure(&font_family);
        }
    }
    for object in &document.objects {
        if object.object_type == "text" {
            if let Some(runs) = object
                .payload
                .extra
                .get("runs")
                .cloned()
                .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
            {
                for run in runs {
                    if let Some(font_family) = run.font_family {
                        fonts.ensure(&font_family);
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
            if let Some(font_family) = &label.font_family {
                fonts.ensure(font_family);
            }
            for run in &label.runs {
                if let Some(font_family) = &run.font_family {
                    fonts.ensure(font_family);
                }
            }
        }
    }
}

fn object_style<'a>(document: &'a ChemSemaDocument, object: &SceneObject) -> Option<&'a Value> {
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

fn payload_nested_point_cdxml(payload: &ObjectPayload, group: &str, key: &str) -> Option<Point> {
    let coords = payload.extra.get(group)?.get(key)?.as_array()?;
    Some(Point::new(
        coords.first()?.as_f64()?,
        coords.get(1)?.as_f64()?,
    ))
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

fn payload_nested_bbox_cdxml(payload: &ObjectPayload, group: &str, key: &str) -> Option<[f64; 4]> {
    let coords = payload.extra.get(group)?.get(key)?.as_array()?;
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

fn imported_cdxml_label_attr<'a>(label: &'a NodeLabel, name: &str) -> Option<&'a str> {
    label
        .meta
        .get("import")?
        .get("cdxml")?
        .get(name)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn imported_cdxml_object_attr<'a>(object: &'a SceneObject, name: &str) -> Option<&'a str> {
    object
        .meta
        .get("import")?
        .get("cdxml")?
        .get(name)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn bond_endpoint_attachment(bond: &Bond, endpoint: &str) -> Option<u64> {
    bond.meta
        .pointer(&format!("/endpointAttachments/{endpoint}"))
        .and_then(|attachment| attachment.get("characterIndex"))
        .and_then(Value::as_u64)
}

fn imported_cdxml_crossing_bonds(bond: &Bond) -> impl Iterator<Item = &str> {
    bond.meta
        .pointer("/import/cdxml/crossingBonds")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn cdxml_bond_crossing_scope(object: &SceneObject) -> String {
    object
        .meta
        .pointer("/import/cdxml/fragmentId")
        .and_then(Value::as_str)
        .map(|id| format!("cdxml-fragment:{id}"))
        .unwrap_or_else(|| format!("object:{}", object.id))
}

fn collect_cdxml_bond_export_keys(
    document: &ChemSemaDocument,
    objects: &[SceneObject],
    keys: &mut Vec<(String, String)>,
) {
    for object in objects {
        if !object.visible {
            continue;
        }
        if object.object_type == "molecule" {
            if let Some(fragment) = object
                .payload
                .resource_ref
                .as_ref()
                .and_then(|resource_ref| document.resources.get(resource_ref))
                .and_then(|resource| resource.data.as_fragment())
            {
                let scope = cdxml_bond_crossing_scope(object);
                keys.extend(
                    fragment
                        .bonds
                        .iter()
                        .map(|bond| (scope.clone(), bond.id.clone())),
                );
            }
        }
        collect_cdxml_bond_export_keys(document, &object.children, keys);
    }
}

fn cdxml_node_label_alignment(label: &NodeLabel) -> &'static str {
    if label.layout.as_deref() == Some("attached-group-above") {
        "Above"
    } else if label.layout.as_deref() == Some("attached-group-below") {
        "Below"
    } else if label.layout.as_deref() == Some("attached-group-center") {
        "Right"
    } else {
        "Auto"
    }
}

fn cdxml_node_label_interpret_chemically(label: &NodeLabel) -> bool {
    if let Some(value) = label.meta.get("defaultChemical").and_then(Value::as_bool) {
        return value;
    }
    label_source_runs_for_export(label)
        .unwrap_or_else(|| label.runs.clone())
        .iter()
        .any(|run| run.script.as_deref() == Some("chemical"))
}

fn cdxml_node_num_hydrogens_for_export(node: &Node) -> Option<u8> {
    if let Some(value) = crate::node_user_num_hydrogens_override(node) {
        return Some(value);
    }
    if let Some(value) = node
        .meta
        .pointer("/import/cdxml/explicitNumHydrogens")
        .and_then(Value::as_u64)
    {
        return Some(value.min(u64::from(u8::MAX)) as u8);
    }
    (node.num_hydrogens > 0).then_some(node.num_hydrogens)
}

fn cdxml_label_line_starts(label: &NodeLabel) -> Option<String> {
    let lines: Vec<String> = if !label.lines.is_empty() {
        label.lines.clone()
    } else if !label.line_runs.is_empty() {
        label
            .line_runs
            .iter()
            .map(|line| line.iter().map(|run| run.text.as_str()).collect())
            .collect()
    } else {
        Vec::new()
    };
    if lines.len() <= 1 {
        return None;
    }
    let mut offset = 0usize;
    Some(
        lines
            .iter()
            .map(|line| {
                offset += line.chars().count() + 1;
                offset.to_string()
            })
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn label_source_runs_for_export(label: &NodeLabel) -> Option<Vec<LabelRun>> {
    label
        .meta
        .get("sourceRuns")
        .cloned()
        .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
        .filter(|runs| !runs.is_empty())
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
        if stereo.kind == "hollow-wedge" {
            return Some(if stereo.wide_end == "end" {
                "HollowWedgeBegin"
            } else {
                "HollowWedgeEnd"
            });
        }
    }
    if second {
        let (line_style, line_weight) = match bond.double.as_ref().map(|double| double.placement) {
            Some(crate::DoubleBondPlacement::Left) => {
                (bond.line_styles.left, bond.line_weights.left)
            }
            _ => (bond.line_styles.right, bond.line_weights.right),
        };
        if line_style == crate::BondLinePattern::Dashed {
            return Some("Dash");
        }
        if line_weight == crate::BondLineWeight::Bold {
            return Some("Bold");
        }
        return None;
    }
    if bond.line_styles.main == crate::BondLinePattern::Dashed {
        return Some("Dash");
    }
    if bond.line_styles.main == crate::BondLinePattern::Wavy {
        return Some("Wavy");
    }
    if bond.line_weights.main == crate::BondLineWeight::Bold {
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
        "equilibrium" | "unequal-equilibrium" => "Equilibrium",
        _ => "Solid",
    }
}

fn cdxml_arrow_equilibrium_ratio(value: Option<&Value>) -> Option<f64> {
    let value = value?;
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("solid")
        .to_ascii_lowercase();
    let ratio = value
        .get("equilibriumRatio")
        .or_else(|| value.get("equilibrium_ratio"))
        .and_then(Value::as_f64)
        .filter(|ratio| ratio.is_finite() && *ratio > 1.0)
        .unwrap_or_else(|| {
            if kind == "unequal-equilibrium" {
                3.0
            } else {
                1.0
            }
        });
    (ratio > 1.0).then_some(ratio)
}

fn cdxml_arrowhead_type_attr(arrow_kind: &str) -> &str {
    if arrow_kind == "Equilibrium" {
        "Solid"
    } else {
        arrow_kind
    }
}

fn cdxml_arrow_endpoint_position(
    payload: &ObjectPayload,
    arrow: Option<&Value>,
    key: &str,
    legacy_enabled_value: &str,
) -> &'static str {
    if let Some(value) = arrow
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
        .and_then(cdxml_arrow_endpoint_style)
    {
        return value;
    }
    if payload_string_cdxml(payload, key)
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case(legacy_enabled_value))
    {
        "Full"
    } else {
        "None"
    }
}

fn cdxml_arrow_endpoint_style(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "full" => Some("Full"),
        "half-left" | "halfleft" | "left" | "top" => Some("HalfLeft"),
        "half-right" | "halfright" | "right" | "bottom" => Some("HalfRight"),
        "none" => Some("None"),
        _ => None,
    }
}

fn cdxml_arrow_size_attribute(value: f64) -> f64 {
    value * 100.0
}

fn cdxml_arrow_fill_type(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "none" => Some("None"),
        "solid" => Some("Solid"),
        "shaded" => Some("Shaded"),
        _ => None,
    }
}

fn cdxml_symbol_anchor_bbox(
    center_x: f64,
    center_y: f64,
    anchor_width: f64,
    anchor_height: f64,
) -> [f64; 4] {
    if anchor_width.abs() > crate::EPSILON {
        [center_x, center_y, center_x - anchor_width, center_y]
    } else if anchor_height.abs() > crate::EPSILON {
        [center_x, center_y, center_x, center_y + anchor_height]
    } else {
        [center_x, center_y, center_x, center_y]
    }
}

fn cdxml_arrow_no_go(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "cross" => Some("Cross"),
        "hash" => Some("Hash"),
        _ => None,
    }
}

fn cdxml_arrow_object_reference(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if !value.trim().is_empty() => Some(value.trim().to_string()),
        Value::Number(value) => Some(value.to_string()),
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

fn cdxml_orbital_type(template: &str, style: &str, phase: &str) -> &'static str {
    match (template, style, phase) {
        ("s", "shaded", _) => "sShaded",
        ("s", "filled", _) => "sFilled",
        ("s", _, _) => "s",
        ("p", "filled", _) => "pFilled",
        ("p", _, _) => "p",
        ("dxy", "filled", _) => "dxyFilled",
        ("dxy", _, _) => "dxy",
        ("oval", "shaded", _) => "ovalShaded",
        ("oval", "filled", _) => "ovalFilled",
        ("oval", _, _) => "oval",
        ("hybrid", "filled", "minus") => "hybridMinusFilled",
        ("hybrid", _, "minus") => "hybridMinus",
        ("hybrid", "filled", _) => "hybridPlusFilled",
        ("hybrid", _, _) => "hybridPlus",
        ("dz2", "filled", "minus") => "dz2MinusFilled",
        ("dz2", _, "minus") => "dz2Minus",
        ("dz2", "filled", _) => "dz2PlusFilled",
        ("dz2", _, _) => "dz2Plus",
        ("lobe", "shaded", _) => "lobeShaded",
        ("lobe", "filled", _) => "lobeFilled",
        ("lobe", _, _) => "lobe",
        _ => "s",
    }
}

fn cdxml_justification(value: Option<&str>) -> &'static str {
    match value.unwrap_or("").to_ascii_lowercase().as_str() {
        "center" | "middle" => "Center",
        "right" | "end" => "Right",
        "full" | "justify" => "Full",
        "above" => "Above",
        "below" => "Below",
        "auto" => "Auto",
        "best" => "Best",
        _ => "Left",
    }
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

fn fmt_cdxml_bool(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn fmt_margins(value: [f64; 4]) -> String {
    format!(
        "{} {} {} {}",
        fmt_num(value[0]),
        fmt_num(value[1]),
        fmt_num(value[2]),
        fmt_num(value[3])
    )
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
