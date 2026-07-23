use crate::{
    document_to_cdxml, parse_cdxml_document, ChemSemaDocument, InterchangeDocument,
    InterchangeObject, InterchangeProperty,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::sync::OnceLock;

const CDX_HEADER: &[u8; 22] = b"VjCD0100\x04\x03\x02\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
const CDX_COORD_FACTOR: f64 = 65_536.0;

pub fn parse_cdx_document(bytes: &[u8], title: Option<&str>) -> Result<ChemSemaDocument, String> {
    let tree = CdxReader::new(bytes).read()?;
    let cdxml = CdxmlWriter::new().write(&tree);
    let interchange = interchange_object_from_cdx(&tree);
    let mut document = parse_cdxml_document(&cdxml, title)?;
    document.interchange.insert(
        "cdx".to_string(),
        InterchangeDocument {
            format: "cdx".to_string(),
            version: Some("0100".to_string()),
            root: interchange,
        },
    );
    document.document.meta["sourceFormat"] = json!("cdx");
    if let Some(import) = document.document.meta.get_mut("import") {
        import["cdx"] = json!({ "nativeImport": true });
    }
    Ok(document)
}

pub fn document_to_cdx(document: &ChemSemaDocument) -> Result<Vec<u8>, String> {
    let cdxml = document_to_cdxml(document);
    let mut root = crate::cdxml::parse_xml_tree(&cdxml)?;
    let source = document.interchange.get("cdx").map(|source| &source.root);
    if let Some(source) = source {
        overlay_unmodeled_cdx_values(&mut root, source);
    }
    CdxWriter::new(source).write(&root)
}

pub fn cdx_to_cdxml(bytes: &[u8]) -> Result<String, String> {
    let tree = CdxReader::new(bytes).read()?;
    Ok(CdxmlWriter::new().write(&tree))
}

pub fn cdxml_to_cdx(cdxml: &str) -> Result<Vec<u8>, String> {
    let root = crate::cdxml::parse_xml_tree(cdxml)?;
    CdxWriter::new(None).write(&root)
}

#[derive(Debug, Clone)]
struct CdxNode {
    name: String,
    tag: u16,
    id: u32,
    attrs: BTreeMap<String, String>,
    properties: Vec<CdxRawProperty>,
    text_runs: Vec<CdxTextRun>,
    text: Option<String>,
    children: Vec<CdxNode>,
}

#[derive(Debug, Clone)]
struct CdxRawProperty {
    tag: u16,
    data: Vec<u8>,
}

fn interchange_object_from_cdx(node: &CdxNode) -> InterchangeObject {
    let mut properties = BTreeMap::new();
    for (order, property) in node.properties.iter().enumerate() {
        let (official_name, cdx_type) = official_property_info(property.tag)
            .unwrap_or_else(|| (format!("tag_{:04X}", property.tag), "unknown".to_string()));
        let lexical = if property.tag == 0x0700 {
            node.text.clone().unwrap_or_default()
        } else {
            decode_property(property.tag, &property.data, None)
                .map(|(_, value)| value)
                .or_else(|| decode_official_lexical(&cdx_type, &property.data))
                .or_else(|| node.attrs.get(&official_name).cloned())
                .unwrap_or_default()
        };
        let storage_name = unique_property_storage_name(&properties, &official_name);
        properties.insert(
            storage_name,
            InterchangeProperty {
                name: official_name,
                order,
                value_type: Some(cdx_value_type(&cdx_type).to_string()),
                value: lexical,
                cdx_tag: Some(format!("0x{:04X}", property.tag)),
                cdx_type: Some(cdx_type),
                raw_base64: Some(BASE64.encode(&property.data)),
            },
        );
    }
    InterchangeObject {
        name: node.name.clone(),
        format_tag: Some(format!("0x{:04X}", node.tag)),
        id: Some(node.id.to_string()),
        properties,
        text: node.text.clone().unwrap_or_default(),
        children: node
            .children
            .iter()
            .map(interchange_object_from_cdx)
            .collect(),
    }
}

fn unique_property_storage_name(
    properties: &BTreeMap<String, InterchangeProperty>,
    name: &str,
) -> String {
    if !properties.contains_key(name) {
        return name.to_string();
    }
    let mut occurrence = 2usize;
    loop {
        let candidate = format!("{name}#{occurrence}");
        if !properties.contains_key(&candidate) {
            return candidate;
        }
        occurrence += 1;
    }
}

fn official_property_info(tag: u16) -> Option<(String, String)> {
    static SCHEMA: OnceLock<serde_json::Value> = OnceLock::new();
    let schema = SCHEMA.get_or_init(|| {
        serde_json::from_str(include_str!("../schemas/cdx-cdxml-official-v1.json"))
            .expect("embedded official CDX/CDXML schema must be valid JSON")
    });
    schema
        .pointer("/cdx/properties")?
        .as_array()?
        .iter()
        .find(|property| {
            property
                .get("tag")
                .and_then(serde_json::Value::as_str)
                .and_then(|value| u16::from_str_radix(value.trim_start_matches("0x"), 16).ok())
                == Some(tag)
        })
        .map(|property| {
            (
                property
                    .get("cdxmlName")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| property.get("sdkName").and_then(serde_json::Value::as_str))
                    .unwrap_or("unknown")
                    .trim_start_matches("kCDXProp_")
                    .to_string(),
                property
                    .get("cdxType")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
            )
        })
}

fn official_property_tag_and_type(name: &str) -> Option<(u16, String)> {
    static SCHEMA: OnceLock<serde_json::Value> = OnceLock::new();
    let schema = SCHEMA.get_or_init(|| {
        serde_json::from_str(include_str!("../schemas/cdx-cdxml-official-v1.json"))
            .expect("embedded official CDX/CDXML schema must be valid JSON")
    });
    let property = schema
        .pointer("/cdx/properties")?
        .as_array()?
        .iter()
        .find(|property| {
            property
                .get("cdxmlName")
                .and_then(serde_json::Value::as_str)
                == Some(name)
        })?;
    let tag = property
        .get("tag")
        .and_then(serde_json::Value::as_str)
        .and_then(parse_hex_u16)?;
    let cdx_type = property
        .get("cdxType")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    Some((tag, cdx_type))
}

fn cdx_value_type(cdx_type: &str) -> &'static str {
    match cdx_type {
        "CDXBoolean" | "CDXBooleanImplied" => "boolean",
        "INT8" | "UINT8" | "INT16" | "UINT16" | "INT32" | "UINT32" | "FLOAT64"
        | "CDXCoordinate" => "number",
        "CDXPoint2D" | "CDXPoint3D" | "CDXRectangle" | "CDXCurvePoints" | "CDXCurvePoints3D" => {
            "number-list"
        }
        "CDXDate" => "date-time-tuple",
        "CDXElementList" => "element-list",
        "CDXGenericList" => "string-list",
        "CDXObjectID" => "object-reference",
        "CDXObjectIDArray" | "CDXObjectIDArrayWithCounts" => "object-reference-list",
        "CDXRepresentsProperty" => "object-property-reference",
        "CDXString" => "string",
        _ => "binary",
    }
}

fn encode_hex_bytes(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 2);
    for byte in data {
        write!(&mut out, "{byte:02X}").expect("writing to a string cannot fail");
    }
    out
}

pub(crate) fn decode_hex_bytes(value: &str) -> Option<Vec<u8>> {
    let compact: String = value
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect();
    if compact.len() % 2 != 0 {
        return None;
    }
    (0..compact.len())
        .step_by(2)
        .map(|offset| u8::from_str_radix(&compact[offset..offset + 2], 16).ok())
        .collect()
}

fn decode_official_lexical(cdx_type: &str, data: &[u8]) -> Option<String> {
    Some(match cdx_type {
        "CDXString" => parse_cdx_string(data, None).text,
        "CDXBoolean" => bool_from_bytes(data),
        "CDXBooleanImplied" => "yes".to_string(),
        "INT8" => read_i8(data)?.to_string(),
        "UINT8" => read_u8(data)?.to_string(),
        "INT16" => read_i16(data)?.to_string(),
        "UINT16" => read_u16(data)?.to_string(),
        "INT32" => read_i32(data)?.to_string(),
        "UINT32" | "CDXObjectID" => read_u32(data)?.to_string(),
        "FLOAT64" => fmt_num(read_f64(data)?),
        "CDXCoordinate" => decode_coordinate(data)?,
        "CDXPoint2D" => decode_point2d(data)?,
        "CDXPoint3D" => decode_point3d(data)?,
        "CDXRectangle" => decode_rectangle(data)?,
        "CDXObjectIDArray" => decode_u32_array(data)?,
        "CDXObjectIDArrayWithCounts" => decode_u32_counted_array(data)?,
        "INT16ListWithCounts" => decode_i16_counted_list(data)?,
        "CDXElementList" => decode_element_list(data)?,
        "CDXCurvePoints" => decode_curve_points(data, 2)?,
        "CDXCurvePoints3D" => decode_curve_points(data, 3)?,
        "CDXDate" => decode_cdx_date(data)?,
        "CDXRepresentsProperty" => decode_represents_property(data)?,
        "CDXGenericList" => decode_generic_list(data)?,
        "Unformatted" => encode_hex_bytes(data),
        _ => return None,
    })
}

fn encode_official_lexical(cdx_type: &str, value: &str) -> Option<Vec<u8>> {
    Some(match cdx_type {
        "CDXString" => encode_plain_cdx_string(value),
        "CDXBoolean" => vec![if yes(value) { 1 } else { 0 }],
        "CDXBooleanImplied" if yes(value) => Vec::new(),
        "INT8" => vec![value.parse::<i8>().ok()? as u8],
        "UINT8" => vec![value.parse::<u8>().ok()?],
        "INT16" => value.parse::<i16>().ok()?.to_le_bytes().to_vec(),
        "UINT16" => value.parse::<u16>().ok()?.to_le_bytes().to_vec(),
        "INT32" => value.parse::<i32>().ok()?.to_le_bytes().to_vec(),
        "UINT32" | "CDXObjectID" => value.parse::<u32>().ok()?.to_le_bytes().to_vec(),
        "FLOAT64" => value.parse::<f64>().ok()?.to_le_bytes().to_vec(),
        "CDXCoordinate" => encode_coordinate(value)?,
        "CDXPoint2D" => encode_point2d(value)?,
        "CDXPoint3D" => encode_point3d(value)?,
        "CDXRectangle" => encode_rectangle(value)?,
        "CDXObjectIDArray" => encode_u32_array(value)?,
        "CDXObjectIDArrayWithCounts" => encode_u32_counted_array(value)?,
        "INT16ListWithCounts" => encode_i16_counted_list(value)?,
        "CDXElementList" => encode_element_list(value)?,
        "CDXCurvePoints" => encode_curve_points(value, 2)?,
        "CDXCurvePoints3D" => encode_curve_points(value, 3)?,
        "CDXDate" => encode_cdx_date(value)?,
        "CDXRepresentsProperty" => encode_represents_property(value)?,
        "CDXGenericList" => encode_generic_list(value)?,
        "Unformatted" => decode_hex_bytes(value)?,
        _ => return None,
    })
}

#[derive(Debug, Clone)]
struct CdxTextRun {
    start: usize,
    font: u16,
    face: u16,
    size: f64,
    color: u16,
}

struct CdxReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    font_table: Option<FontTable>,
    legacy_chemsema_object_tags: bool,
}

impl<'a> CdxReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            font_table: None,
            // ChemDraw 12 and ChemDraw 21 both write the long-established
            // 0x801B..0x802B object registry (Arrow=0x8021).  A previous
            // ChemSema beta followed the shifted static SDK table instead;
            // its explicit marker is handled as a compatibility dialect.
            legacy_chemsema_object_tags: true,
        }
    }

    fn read(mut self) -> Result<CdxNode, String> {
        if self.bytes.len() < CDX_HEADER.len() || &self.bytes[..CDX_HEADER.len()] != CDX_HEADER {
            return Err("invalid CDX header".to_string());
        }
        self.offset = CDX_HEADER.len();
        let root = self.read_object()?;
        self.consume_file_end_marker();
        Ok(root)
    }

    fn read_object(&mut self) -> Result<CdxNode, String> {
        let tag = self.read_u16()?;
        let id = self.read_u32()?;
        let name = if self.legacy_chemsema_object_tags {
            legacy_chemsema_object_name(tag).or_else(|| object_name(tag))
        } else {
            object_name(tag)
        }
        .unwrap_or("unknown")
        .to_string();
        let mut node = CdxNode {
            name,
            tag,
            id,
            attrs: BTreeMap::new(),
            properties: Vec::new(),
            text_runs: Vec::new(),
            text: None,
            children: Vec::new(),
        };
        node.attrs.insert("id".to_string(), id.to_string());

        loop {
            let tag = self.read_u16()?;
            if tag == 0 {
                break;
            }
            if is_object_tag(tag) {
                self.offset -= 2;
                node.children.push(self.read_object()?);
                continue;
            }
            let len = self.read_property_len()?;
            let data = self.read_bytes(len)?;
            node.properties.push(CdxRawProperty {
                tag,
                data: data.to_vec(),
            });
            self.apply_property(&mut node, tag, data)?;
        }
        Ok(node)
    }

    fn apply_property(&mut self, node: &mut CdxNode, tag: u16, data: &[u8]) -> Result<(), String> {
        if tag == 0x0100 {
            if let Some(table) = parse_font_table(data) {
                self.font_table = Some(table.clone());
                node.children.push(table.into_node());
            }
            return Ok(());
        }
        if tag == 0x0300 {
            if let Some(table) = parse_color_table(data) {
                node.children.push(table.into_node());
            }
            return Ok(());
        }
        if tag == 0x0700 {
            let text = parse_cdx_string(data, self.font_table.as_ref());
            node.text = Some(text.text);
            node.text_runs = text.runs;
            return Ok(());
        }
        if tag == 0x000E {
            if let Some(value) = decode_represents_property(data) {
                let mut parts = value.split_whitespace();
                if let (Some(object), Some(property_tag)) = (parts.next(), parts.next()) {
                    let attribute = parse_hex_u16(property_tag)
                        .and_then(official_property_info)
                        .map(|(name, _)| name)
                        .unwrap_or_else(|| property_tag.to_string());
                    let mut attrs = BTreeMap::new();
                    attrs.insert("object".to_string(), object.to_string());
                    attrs.insert("attribute".to_string(), attribute);
                    node.children.push(CdxNode {
                        name: "represent".to_string(),
                        tag,
                        id: 0,
                        attrs,
                        properties: Vec::new(),
                        text_runs: Vec::new(),
                        text: None,
                        children: Vec::new(),
                    });
                }
            }
            return Ok(());
        }
        if tag == 0x080A || tag == 0x080B {
            if let Some((font, face, size, color)) = decode_font_style(data) {
                let prefix = if tag == 0x080A { "Label" } else { "Caption" };
                if font != u16::MAX {
                    node.attrs.insert(format!("{prefix}Font"), font.to_string());
                }
                if face != u16::MAX {
                    node.attrs.insert(format!("{prefix}Face"), face.to_string());
                }
                if size != u16::MAX as f64 / 20.0 {
                    node.attrs.insert(format!("{prefix}Size"), fmt_num(size));
                }
                if color != u16::MAX {
                    node.attrs
                        .insert(format!("{prefix}Color"), color.to_string());
                }
            }
            return Ok(());
        }
        if let Some((name, value)) = decode_property(tag, data, self.font_table.as_ref()) {
            if tag == 0x0003 && value.trim() == "ChemSema" {
                self.legacy_chemsema_object_tags = true;
            }
            if tag == 0x0006
                && value.starts_with("ChemSema/")
                && value.contains("cdx-tags=official")
            {
                self.legacy_chemsema_object_tags = false;
            }
            if tag == 0x0006 && value.contains("cdx-tags=chemdraw") {
                self.legacy_chemsema_object_tags = true;
            }
            node.attrs.insert(name.to_string(), value);
        } else if let Some((name, cdx_type)) = official_property_info(tag) {
            if let Some(value) = decode_official_lexical(&cdx_type, data) {
                node.attrs.insert(name, value);
            }
        }
        Ok(())
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_property_len(&mut self) -> Result<usize, String> {
        let short = self.read_u16()?;
        if short == 0xFFFF {
            Ok(self.read_u32()? as usize)
        } else {
            Ok(short as usize)
        }
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], String> {
        if self.offset + len > self.bytes.len() {
            return Err("unexpected end of CDX stream".to_string());
        }
        let start = self.offset;
        self.offset += len;
        Ok(&self.bytes[start..start + len])
    }

    fn consume_file_end_marker(&mut self) {
        if self.offset + 2 <= self.bytes.len() {
            let marker = u16::from_le_bytes([self.bytes[self.offset], self.bytes[self.offset + 1]]);
            if marker == 0 {
                self.offset += 2;
            }
        }
    }
}

struct CdxmlWriter;

impl CdxmlWriter {
    fn new() -> Self {
        Self
    }

    fn write(&self, root: &CdxNode) -> String {
        let mut out = String::new();
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n");
        out.push_str("<!DOCTYPE CDXML SYSTEM \"http://www.cambridgesoft.com/xml/cdxml.dtd\" >\n");
        self.write_node(&mut out, root, 0);
        out
    }

    fn write_node(&self, out: &mut String, node: &CdxNode, indent: usize) {
        let tag_name = node.name.as_str();
        if tag_name == "fonttable" || tag_name == "colortable" {
            self.write_table_node(out, node, indent);
            return;
        }
        self.write_indent(out, indent);
        write!(out, "<{tag_name}").expect("write CDXML node");
        let mut attrs: Vec<_> = node.attrs.iter().collect();
        attrs.sort_by(|a, b| {
            attr_order(a.0)
                .cmp(&attr_order(b.0))
                .then_with(|| a.0.cmp(b.0))
        });
        for (key, value) in attrs {
            if key.starts_with("CDXRaw") {
                continue;
            }
            write!(out, " {key}=\"{}\"", escape_attr(value)).expect("write CDXML attr");
        }
        if node.children.is_empty() && node.text.is_none() {
            out.push_str("/>\n");
            return;
        }
        out.push_str(">\n");
        if let Some(text) = &node.text {
            self.write_text_runs(out, text, &node.text_runs, indent + 2);
        }
        for child in &node.children {
            self.write_node(out, child, indent + 2);
        }
        self.write_indent(out, indent);
        writeln!(out, "</{tag_name}>").expect("write CDXML node close");
    }

    fn write_table_node(&self, out: &mut String, node: &CdxNode, indent: usize) {
        self.write_indent(out, indent);
        writeln!(out, "<{}>", node.name).expect("write table open");
        for child in &node.children {
            self.write_node(out, child, indent + 2);
        }
        self.write_indent(out, indent);
        writeln!(out, "</{}>", node.name).expect("write table close");
    }

    fn write_text_runs(&self, out: &mut String, text: &str, runs: &[CdxTextRun], indent: usize) {
        if runs.is_empty() {
            self.write_indent(out, indent);
            writeln!(out, "{}", escape_text(text)).expect("write text");
            return;
        }
        let mut sorted = runs.to_vec();
        sorted.sort_by_key(|run| run.start);
        let char_count = text.chars().count();
        for (index, run) in sorted.iter().enumerate() {
            let start = run.start.min(char_count);
            let end = sorted
                .get(index + 1)
                .map(|next| next.start.min(char_count))
                .unwrap_or(char_count)
                .max(start);
            let slice: String = text.chars().skip(start).take(end - start).collect();
            self.write_indent(out, indent);
            out.push_str("<s");
            // CDXString uses 0xFFFF as an inheritance sentinel for each
            // style component. Emitting it as a literal face (65535) turns
            // every style bit on; omitting the CDXML attribute preserves the
            // containing text/document style, as ChemDraw does.
            if run.font != u16::MAX {
                write!(out, " font=\"{}\"", run.font).expect("write text font");
            }
            if run.size != u16::MAX as f64 / 20.0 {
                write!(out, " size=\"{}\"", fmt_num(run.size)).expect("write text size");
            }
            if run.face != u16::MAX {
                write!(out, " face=\"{}\"", run.face).expect("write text face");
            }
            if run.color != u16::MAX {
                write!(out, " color=\"{}\"", run.color).expect("write text color");
            }
            writeln!(out, ">{}</s>", escape_text(&slice)).expect("write styled text");
        }
    }

    fn write_indent(&self, out: &mut String, indent: usize) {
        for _ in 0..indent {
            out.push(' ');
        }
    }
}

struct CdxWriter<'a> {
    next_id: u32,
    source: Option<&'a InterchangeObject>,
}

impl<'a> CdxWriter<'a> {
    fn new(source: Option<&'a InterchangeObject>) -> Self {
        Self {
            next_id: 5000,
            source,
        }
    }

    fn write(mut self, root: &crate::cdxml::xml::XmlNode) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        out.extend_from_slice(CDX_HEADER);
        self.write_object(root, self.source, &mut out)?;
        out.extend_from_slice(&[0, 0, 0, 0]);
        Ok(out)
    }

    fn write_object(
        &mut self,
        node: &crate::cdxml::xml::XmlNode,
        source: Option<&InterchangeObject>,
        out: &mut Vec<u8>,
    ) -> Result<(), String> {
        let Some(tag) = object_tag(&node.name) else {
            return Ok(());
        };
        write_u16(out, tag);
        write_u32(out, self.xml_id(node));
        let mut written_properties = BTreeSet::new();

        for (key, value) in &node.attrs {
            if key == "id" {
                continue;
            }
            if let Some((prop_tag, bytes)) = encode_property(key, value) {
                write_property(out, prop_tag, &bytes);
                written_properties.insert(prop_tag);
            }
        }

        if node.name == "CDXML" {
            if let Some(color_table) = node.direct_children("colortable").next() {
                write_property(out, 0x0300, &encode_color_table(color_table));
                written_properties.insert(0x0300);
            }
            if let Some(font_table) = node.direct_children("fonttable").next() {
                write_property(out, 0x0100, &encode_font_table(font_table));
                written_properties.insert(0x0100);
            }
        }

        if node.name == "t" {
            write_property(out, 0x0700, &encode_cdx_string(node));
            written_properties.insert(0x0700);
        }
        for represent in node.direct_children("represent") {
            let Some(object) = represent.attr("object") else {
                continue;
            };
            let Some(attribute) = represent.attr("attribute") else {
                continue;
            };
            let Some((property_tag, _)) = official_property_tag_and_type(attribute) else {
                continue;
            };
            let lexical = format!("{object} 0x{property_tag:04X}");
            if let Some(bytes) = encode_represents_property(&lexical) {
                write_property(out, 0x000E, &bytes);
                written_properties.insert(0x000E);
            }
        }

        if let Some(source) = source {
            for property in ordered_interchange_properties(source) {
                let Some(prop_tag) = property.cdx_tag.as_deref().and_then(parse_hex_u16) else {
                    continue;
                };
                if written_properties.contains(&prop_tag) {
                    continue;
                }
                let encoded = (!property.value.is_empty())
                    .then(|| {
                        property
                            .cdx_type
                            .as_deref()
                            .and_then(|kind| encode_official_lexical(kind, &property.value))
                    })
                    .flatten();
                if let Some(bytes) = encoded.or_else(|| {
                    property
                        .raw_base64
                        .as_deref()
                        .and_then(|value| BASE64.decode(value).ok())
                }) {
                    write_property(out, prop_tag, &bytes);
                }
            }
        }

        let generated_children: Vec<_> = node
            .children
            .iter()
            .filter(|child| !is_cdx_helper_name(&child.name))
            .collect();
        let mut used_generated = BTreeSet::new();
        if let Some(source) = source {
            for source_child in source
                .children
                .iter()
                .filter(|child| !is_cdx_helper_name(&child.name))
            {
                let generated_index = generated_children
                    .iter()
                    .enumerate()
                    .find(|(index, child)| {
                        !used_generated.contains(index)
                            && interchange_matches_xml(source_child, child)
                    })
                    .map(|(index, _)| index)
                    .or_else(|| {
                        generated_children
                            .iter()
                            .enumerate()
                            .find(|(index, child)| {
                                !used_generated.contains(index) && source_child.name == child.name
                            })
                            .map(|(index, _)| index)
                    });
                if let Some(index) = generated_index
                    .filter(|index| object_tag(&generated_children[*index].name).is_some())
                {
                    used_generated.insert(index);
                    self.write_object(generated_children[index], Some(source_child), out)?;
                } else {
                    self.write_raw_interchange_object(source_child, out)?;
                }
            }
        }
        for (index, child) in generated_children.into_iter().enumerate() {
            if !used_generated.contains(&index) {
                self.write_object(child, None, out)?;
            }
        }
        write_u16(out, 0);
        Ok(())
    }

    fn write_raw_interchange_object(
        &mut self,
        object: &InterchangeObject,
        out: &mut Vec<u8>,
    ) -> Result<(), String> {
        let tag = object
            .format_tag
            .as_deref()
            .and_then(parse_hex_u16)
            .or_else(|| object_tag(&object.name))
            .ok_or_else(|| format!("CDX object '{}' has no writable object tag", object.name))?;
        write_u16(out, tag);
        let id = object
            .id
            .as_deref()
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| {
                let id = self.next_id;
                self.next_id += 1;
                id
            });
        write_u32(out, id);
        for property in ordered_interchange_properties(object) {
            let Some(prop_tag) = property.cdx_tag.as_deref().and_then(parse_hex_u16) else {
                continue;
            };
            let bytes = (!property.value.is_empty())
                .then(|| {
                    property
                        .cdx_type
                        .as_deref()
                        .and_then(|kind| encode_official_lexical(kind, &property.value))
                })
                .flatten()
                .or_else(|| {
                    property
                        .raw_base64
                        .as_deref()
                        .and_then(|value| BASE64.decode(value).ok())
                })
                .ok_or_else(|| {
                    format!(
                        "CDX property {} on object {} has no valid rawBase64 payload",
                        property.cdx_tag.as_deref().unwrap_or("unknown"),
                        object.name
                    )
                })?;
            write_property(out, prop_tag, &bytes);
        }
        for child in &object.children {
            if !matches!(
                child.name.as_str(),
                "s" | "font" | "color" | "fonttable" | "colortable" | "represent"
            ) {
                self.write_raw_interchange_object(child, out)?;
            }
        }
        write_u16(out, 0);
        Ok(())
    }

    fn xml_id(&mut self, node: &crate::cdxml::xml::XmlNode) -> u32 {
        if let Some(id) = node.attr("id").and_then(|value| value.parse::<u32>().ok()) {
            id
        } else {
            let id = self.next_id;
            self.next_id += 1;
            id
        }
    }
}

fn is_cdx_helper_name(name: &str) -> bool {
    matches!(
        name,
        "s" | "font" | "color" | "fonttable" | "colortable" | "represent"
    )
}

fn parse_hex_u16(value: &str) -> Option<u16> {
    u16::from_str_radix(value.trim().trim_start_matches("0x"), 16).ok()
}

fn ordered_interchange_properties(object: &InterchangeObject) -> Vec<&InterchangeProperty> {
    let mut properties: Vec<_> = object.properties.iter().collect();
    properties.sort_by(|(left_key, left), (right_key, right)| {
        left.order
            .cmp(&right.order)
            .then_with(|| left_key.cmp(right_key))
    });
    properties
        .into_iter()
        .map(|(_, property)| property)
        .collect()
}

fn interchange_matches_xml(
    source: &InterchangeObject,
    generated: &crate::cdxml::xml::XmlNode,
) -> bool {
    if source.name != generated.name {
        return false;
    }
    match (&source.id, generated.attr("id")) {
        (Some(source_id), Some(generated_id)) => source_id == generated_id,
        (None, None) => true,
        _ => false,
    }
}

fn overlay_unmodeled_cdx_values(
    generated: &mut crate::cdxml::xml::XmlNode,
    source: &InterchangeObject,
) {
    for property in source.properties.values() {
        if property_tag(&property.name).is_none() && !property.value.is_empty() {
            generated
                .attrs
                .insert(property.name.clone(), property.value.clone());
        }
    }
    let mut matched = BTreeSet::new();
    for child in &mut generated.children {
        let index = source
            .children
            .iter()
            .enumerate()
            .find(|(index, candidate)| {
                !matched.contains(index) && interchange_matches_xml(candidate, child)
            })
            .map(|(index, _)| index)
            .or_else(|| {
                source
                    .children
                    .iter()
                    .enumerate()
                    .find(|(index, candidate)| {
                        !matched.contains(index) && candidate.name == child.name
                    })
                    .map(|(index, _)| index)
            });
        if let Some(index) = index {
            matched.insert(index);
            overlay_unmodeled_cdx_values(child, &source.children[index]);
        }
    }
}

#[derive(Debug, Clone)]
struct FontRecord {
    id: u16,
    charset: u16,
    name: String,
}

#[derive(Debug, Clone)]
struct FontTable {
    fonts: Vec<FontRecord>,
}

impl FontTable {
    fn into_node(self) -> CdxNode {
        let children = self
            .fonts
            .into_iter()
            .map(|font| {
                let mut attrs = BTreeMap::new();
                attrs.insert("id".to_string(), font.id.to_string());
                attrs.insert(
                    "charset".to_string(),
                    charset_name(font.charset).to_string(),
                );
                attrs.insert("name".to_string(), font.name);
                CdxNode {
                    name: "font".to_string(),
                    tag: 0,
                    id: font.id as u32,
                    attrs,
                    properties: Vec::new(),
                    text_runs: Vec::new(),
                    text: None,
                    children: Vec::new(),
                }
            })
            .collect();
        CdxNode {
            name: "fonttable".to_string(),
            tag: 0x0100,
            id: 0,
            attrs: BTreeMap::new(),
            properties: Vec::new(),
            text_runs: Vec::new(),
            text: None,
            children,
        }
    }
}

struct ColorTable {
    colors: Vec<(u16, u16, u16)>,
}

impl ColorTable {
    fn into_node(self) -> CdxNode {
        let children = self
            .colors
            .into_iter()
            .map(|(r, g, b)| {
                let mut attrs = BTreeMap::new();
                attrs.insert("r".to_string(), fmt_num(r as f64 / 65_535.0));
                attrs.insert("g".to_string(), fmt_num(g as f64 / 65_535.0));
                attrs.insert("b".to_string(), fmt_num(b as f64 / 65_535.0));
                CdxNode {
                    name: "color".to_string(),
                    tag: 0,
                    id: 0,
                    attrs,
                    properties: Vec::new(),
                    text_runs: Vec::new(),
                    text: None,
                    children: Vec::new(),
                }
            })
            .collect();
        CdxNode {
            name: "colortable".to_string(),
            tag: 0x0300,
            id: 0,
            attrs: BTreeMap::new(),
            properties: Vec::new(),
            text_runs: Vec::new(),
            text: None,
            children,
        }
    }
}

struct ParsedText {
    text: String,
    runs: Vec<CdxTextRun>,
}

fn parse_cdx_string(data: &[u8], font_table: Option<&FontTable>) -> ParsedText {
    if data.len() >= 2 {
        let run_count = u16::from_le_bytes([data[0], data[1]]) as usize;
        let run_bytes = 2 + run_count * 10;
        if run_bytes <= data.len() {
            let mut runs = Vec::new();
            let mut offset = 2;
            for _ in 0..run_count {
                let start = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
                let font = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
                let face = u16::from_le_bytes([data[offset + 4], data[offset + 5]]);
                let size = u16::from_le_bytes([data[offset + 6], data[offset + 7]]) as f64 / 20.0;
                let color = u16::from_le_bytes([data[offset + 8], data[offset + 9]]);
                runs.push(CdxTextRun {
                    start,
                    font,
                    face,
                    size,
                    color,
                });
                offset += 10;
            }
            return ParsedText {
                text: decode_text(
                    &data[run_bytes..],
                    runs.first().map(|run| run.font),
                    font_table,
                ),
                runs,
            };
        }
    }
    ParsedText {
        text: decode_text(data, None, font_table),
        runs: Vec::new(),
    }
}

fn decode_text(data: &[u8], font_id: Option<u16>, font_table: Option<&FontTable>) -> String {
    let charset = font_id
        .and_then(|id| font_table.and_then(|table| table.fonts.iter().find(|font| font.id == id)))
        .map(|font| font.charset)
        .unwrap_or(1252);
    let decoded = if charset == 65001 || std::str::from_utf8(data).is_ok() {
        String::from_utf8_lossy(data).into_owned()
    } else {
        encoding_rs::WINDOWS_1252.decode(data).0.into_owned()
    };
    decoded.replace('\r', "\n")
}

fn parse_font_table(data: &[u8]) -> Option<FontTable> {
    if data.len() < 4 {
        return None;
    }
    let count = u16::from_le_bytes([data[2], data[3]]) as usize;
    let mut offset = 4;
    let mut fonts = Vec::new();
    for _ in 0..count {
        if offset + 6 > data.len() {
            return None;
        }
        let id = u16::from_le_bytes([data[offset], data[offset + 1]]);
        let charset = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
        let len = u16::from_le_bytes([data[offset + 4], data[offset + 5]]) as usize;
        offset += 6;
        if offset + len > data.len() {
            return None;
        }
        let name = String::from_utf8_lossy(&data[offset..offset + len]).to_string();
        offset += len;
        fonts.push(FontRecord { id, charset, name });
    }
    Some(FontTable { fonts })
}

fn parse_color_table(data: &[u8]) -> Option<ColorTable> {
    if data.len() < 2 {
        return None;
    }
    let count = u16::from_le_bytes([data[0], data[1]]) as usize;
    let mut offset = 2;
    let mut colors = Vec::new();
    for _ in 0..count {
        if offset + 6 > data.len() {
            return None;
        }
        colors.push((
            u16::from_le_bytes([data[offset], data[offset + 1]]),
            u16::from_le_bytes([data[offset + 2], data[offset + 3]]),
            u16::from_le_bytes([data[offset + 4], data[offset + 5]]),
        ));
        offset += 6;
    }
    Some(ColorTable { colors })
}

fn decode_property(
    tag: u16,
    data: &[u8],
    font_table: Option<&FontTable>,
) -> Option<(&'static str, String)> {
    let schema = property_schema(tag)?;
    let value = match schema.kind {
        PropertyKind::String => parse_cdx_string(data, font_table).text,
        PropertyKind::Binary => encode_hex_bytes(data),
        PropertyKind::Point2D => decode_point2d(data)?,
        PropertyKind::Point3D => decode_point3d(data)?,
        PropertyKind::Rectangle => decode_rectangle(data)?,
        PropertyKind::Coordinate => decode_coordinate(data)?,
        PropertyKind::Int8 => read_i8(data)?.to_string(),
        PropertyKind::UInt8 => read_u8(data)?.to_string(),
        PropertyKind::Int16 => read_i16_lossy(data)?.to_string(),
        PropertyKind::UInt16 => read_u16_lossy(data)?.to_string(),
        PropertyKind::LineHeightInt16 => decode_line_height(read_i16(data)? as i64),
        PropertyKind::LineHeightUInt16 => decode_line_height(read_u16(data)? as i64),
        PropertyKind::Fixed16_16 => fmt_num(read_i32(data)? as f64 / 65536.0),
        PropertyKind::UInt32 => read_u32(data)?.to_string(),
        PropertyKind::Float64 => read_f64(data)?.to_string(),
        PropertyKind::Boolean => bool_from_bytes(data),
        PropertyKind::BooleanImplied => "yes".to_string(),
        PropertyKind::BondOrder => decode_bond_order(data)?,
        // CDX stores BondSpacing in tenths of a percent.  Preserve that
        // fractional digit: rounding here changes the distance between the
        // strokes of every multiple bond in documents that use values such as
        // 12.5%, and the error grows with the bond length.
        PropertyKind::BondSpacing => fmt_num(read_i16(data)? as f64 / 10.0),
        PropertyKind::AngleTenths => fmt_num(read_i16(data)? as f64 / 10.0),
        PropertyKind::FontStyle => return None,
        PropertyKind::ObjectIdArray => decode_u32_array(data)?,
        PropertyKind::Int16ListWithCounts => decode_i16_counted_list(data)?,
        PropertyKind::Enum8(values) => enum_name(read_i8(data)? as i16, values).to_string(),
        PropertyKind::Enum(values) => enum_name(read_i16_lossy(data)?, values).to_string(),
        PropertyKind::BitFlags(values) => decode_bit_flags(read_i16_lossy(data)?, values),
    };
    Some((schema.name, value))
}

#[derive(Clone, Copy)]
struct PropertySchema {
    name: &'static str,
    kind: PropertyKind,
}

#[derive(Clone, Copy)]
enum PropertyKind {
    String,
    Binary,
    Point2D,
    Point3D,
    Rectangle,
    Coordinate,
    Int8,
    UInt8,
    Int16,
    UInt16,
    LineHeightInt16,
    LineHeightUInt16,
    Fixed16_16,
    UInt32,
    Float64,
    Boolean,
    BooleanImplied,
    BondOrder,
    BondSpacing,
    AngleTenths,
    FontStyle,
    ObjectIdArray,
    Int16ListWithCounts,
    Enum8(&'static [(i16, &'static str)]),
    Enum(&'static [(i16, &'static str)]),
    BitFlags(&'static [(i16, &'static str)]),
}

fn decode_line_height(value: i64) -> String {
    match value {
        0 => "variable".to_string(),
        1 => "auto".to_string(),
        _ => fmt_num(value as f64 / 20.0),
    }
}

fn encode_line_height(value: &str, unsigned: bool) -> Option<Vec<u8>> {
    let numeric = match value.trim().to_ascii_lowercase().as_str() {
        "variable" => 0,
        "auto" => 1,
        value => (value.parse::<f64>().ok()? * 20.0).round() as i32,
    };
    if unsigned {
        Some(u16::try_from(numeric).ok()?.to_le_bytes().to_vec())
    } else {
        Some(i16::try_from(numeric).ok()?.to_le_bytes().to_vec())
    }
}

fn property_schema(tag: u16) -> Option<PropertySchema> {
    let schema = match tag {
        0x0001 => ("CreationUserName", PropertyKind::String),
        0x0003 => ("CreationProgram", PropertyKind::String),
        0x0004 => ("ModificationUserName", PropertyKind::String),
        0x0006 => ("ModificationProgram", PropertyKind::String),
        0x0008 => ("Name", PropertyKind::String),
        0x0009 => ("Comment", PropertyKind::String),
        0x000A => ("Z", PropertyKind::Int16),
        0x0011 => ("Visible", PropertyKind::Boolean),
        0x0012 | 0x0013 => ("SupersededBy", PropertyKind::UInt32),
        0x0200 => ("p", PropertyKind::Point2D),
        0x0201 => ("xyz", PropertyKind::Point3D),
        0x0202 => ("extent", PropertyKind::Point2D),
        0x0204 => ("BoundingBox", PropertyKind::Rectangle),
        0x0205 => ("RotationAngle", PropertyKind::Fixed16_16),
        0x0207 => ("Head3D", PropertyKind::Point3D),
        0x0208 => ("Tail3D", PropertyKind::Point3D),
        0x0209 => ("TopLeft", PropertyKind::Point2D),
        0x020A => ("TopRight", PropertyKind::Point2D),
        0x020B => ("BottomRight", PropertyKind::Point2D),
        0x020C => ("BottomLeft", PropertyKind::Point2D),
        0x020D => ("Center3D", PropertyKind::Point3D),
        0x020E => ("MajorAxisEnd3D", PropertyKind::Point3D),
        0x020F => ("MinorAxisEnd3D", PropertyKind::Point3D),
        0x0301 => ("color", PropertyKind::UInt16),
        0x0302 => ("bgcolor", PropertyKind::Int16),
        0x0400 => ("NodeType", PropertyKind::Enum(NODE_TYPE)),
        0x0401 => ("LabelDisplay", PropertyKind::Enum8(LABEL_DISPLAY)),
        0x0402 => ("Element", PropertyKind::Int16),
        0x0420 => ("Isotope", PropertyKind::Int16),
        0x0421 => ("Charge", PropertyKind::Int8),
        0x0422 => ("Radical", PropertyKind::Enum8(ATOM_RADICAL)),
        0x0424 => ("ImplicitHydrogens", PropertyKind::BooleanImplied),
        0x042B => ("NumHydrogens", PropertyKind::UInt16),
        0x0433 => ("GenericNickname", PropertyKind::String),
        0x0437 => ("AS", PropertyKind::Enum(ATOM_STEREO)),
        0x0439 => ("AtomNumber", PropertyKind::String),
        0x043B => ("ShowAtomStereo", PropertyKind::Boolean),
        0x043C => ("ShowAtomNumber", PropertyKind::Boolean),
        0x043F => ("IsotopicAbundance", PropertyKind::Enum8(ISOTOPIC_ABUNDANCE)),
        0x0444 => ("HideImplicitHydrogens", PropertyKind::Boolean),
        0x0445 => ("ShowAtomEnhancedStereo", PropertyKind::Boolean),
        0x0504 => ("Weight", PropertyKind::Float64),
        0x0505 => ("ConnectionOrder", PropertyKind::ObjectIdArray),
        0x0600 => ("Order", PropertyKind::BondOrder),
        0x0601 => ("Display", PropertyKind::Enum(BOND_DISPLAY)),
        0x0602 => ("Display2", PropertyKind::Enum(BOND_DISPLAY)),
        0x0603 => ("DoublePosition", PropertyKind::Enum(DOUBLE_POSITION)),
        0x0604 => ("B", PropertyKind::UInt32),
        0x0605 => ("E", PropertyKind::UInt32),
        0x0608 => ("BeginAttach", PropertyKind::UInt8),
        0x0609 => ("EndAttach", PropertyKind::UInt8),
        0x060A => ("BS", PropertyKind::Enum(BOND_STEREO)),
        0x060B => ("BondCircularOrdering", PropertyKind::ObjectIdArray),
        0x060E => ("CrossingBonds", PropertyKind::ObjectIdArray),
        0x0701 => ("Justification", PropertyKind::Enum8(JUSTIFICATION)),
        0x0702 => ("LineHeight", PropertyKind::LineHeightUInt16),
        0x0703 => ("WordWrapWidth", PropertyKind::Int16),
        0x0704 => ("LineStarts", PropertyKind::Int16ListWithCounts),
        0x0705 => ("LabelAlignment", PropertyKind::Enum8(LABEL_ALIGNMENT)),
        0x0706 => ("LabelLineHeight", PropertyKind::LineHeightInt16),
        0x0707 => ("CaptionLineHeight", PropertyKind::LineHeightInt16),
        0x0708 => ("InterpretChemically", PropertyKind::Boolean),
        0x0709 => ("UTF8Text", PropertyKind::String),
        0x0802 => ("PrintMargins", PropertyKind::Rectangle),
        0x0803 => ("ChainAngle", PropertyKind::Fixed16_16),
        0x0804 => ("BondSpacing", PropertyKind::BondSpacing),
        0x0805 => ("BondLength", PropertyKind::Coordinate),
        0x0806 => ("BoldWidth", PropertyKind::Coordinate),
        0x0807 => ("LineWidth", PropertyKind::Coordinate),
        0x0808 => ("MarginWidth", PropertyKind::Coordinate),
        0x0809 => ("HashSpacing", PropertyKind::Coordinate),
        0x080A => ("LabelStyle", PropertyKind::FontStyle),
        0x080B => ("CaptionStyle", PropertyKind::FontStyle),
        0x080C => ("CaptionJustification", PropertyKind::Enum8(JUSTIFICATION)),
        0x080D => ("FractionalWidths", PropertyKind::Boolean),
        0x080F => ("WidthPages", PropertyKind::UInt16),
        0x0810 => ("HeightPages", PropertyKind::UInt16),
        0x0812 => ("Width", PropertyKind::Coordinate),
        0x0813 => ("Height", PropertyKind::Coordinate),
        0x0815 => ("Header", PropertyKind::String),
        0x0816 => ("HeaderPosition", PropertyKind::Coordinate),
        0x0817 => ("Footer", PropertyKind::String),
        0x0818 => ("FooterPosition", PropertyKind::Coordinate),
        0x0819 => ("PrintTrimMarks", PropertyKind::BooleanImplied),
        0x081A => ("LabelFont", PropertyKind::Int16),
        0x081B => ("CaptionFont", PropertyKind::Int16),
        0x081C => ("LabelSize", PropertyKind::Int16),
        0x081D => ("CaptionSize", PropertyKind::Int16),
        0x081E => ("LabelFace", PropertyKind::Int16),
        0x081F => ("CaptionFace", PropertyKind::Int16),
        0x0822 => ("BondSpacingAbs", PropertyKind::Coordinate),
        0x0823 => ("LabelJustification", PropertyKind::Enum8(JUSTIFICATION)),
        0x0900 => ("WindowIsZoomed", PropertyKind::BooleanImplied),
        0x0901 => ("WindowPosition", PropertyKind::Point2D),
        0x0902 => ("WindowSize", PropertyKind::Point2D),
        0x0A00 => ("GraphicType", PropertyKind::Enum(GRAPHIC_TYPE)),
        0x0A01 => ("LineType", PropertyKind::BitFlags(LINE_TYPE)),
        0x0A02 => ("ArrowType", PropertyKind::BitFlags(ARROW_TYPE)),
        0x0A03 => ("RectangleType", PropertyKind::BitFlags(RECTANGLE_TYPE)),
        0x0A04 => ("OvalType", PropertyKind::BitFlags(OVAL_TYPE)),
        0x0A05 => ("OrbitalType", PropertyKind::Enum(ORBITAL_TYPE)),
        0x0A06 => ("BracketType", PropertyKind::Enum(BRACKET_TYPE)),
        0x0A07 => ("SymbolType", PropertyKind::Enum(SYMBOL_TYPE)),
        0x0A20 => ("HeadSize", PropertyKind::Int16),
        0x0A21 => ("AngularSize", PropertyKind::AngleTenths),
        0x0A22 => ("LipSize", PropertyKind::Int16),
        0x0A27 => ("BracketedObjectIDs", PropertyKind::ObjectIdArray),
        0x0A28 => ("RepeatCount", PropertyKind::Float64),
        0x0A2B => ("GraphicID", PropertyKind::UInt32),
        0x0A2F => ("ArrowheadType", PropertyKind::Enum(ARROW_HEAD_TYPE)),
        0x0A30 => ("ArrowheadCenterSize", PropertyKind::UInt16),
        0x0A31 => ("ArrowheadWidth", PropertyKind::UInt16),
        0x0A33 => ("ArrowShaftSpacing", PropertyKind::UInt16),
        0x0A34 => ("ArrowEquilibriumRatio", PropertyKind::UInt16),
        0x0A35 => ("ArrowheadHead", PropertyKind::Enum(ARROW_HEAD_POSITION)),
        0x0A36 => ("ArrowheadTail", PropertyKind::Enum(ARROW_HEAD_POSITION)),
        0x0A37 => ("FillType", PropertyKind::Enum(FILL_TYPE)),
        0x0A38 => ("CurveSpacing", PropertyKind::UInt16),
        0x0A39 => ("Closed", PropertyKind::BooleanImplied),
        0x0A3A => ("Dipole", PropertyKind::BooleanImplied),
        0x0A3B => ("NoGo", PropertyKind::Enum8(NO_GO)),
        0x0A3C => ("CornerRadius", PropertyKind::Int16),
        0x0A3E => ("ArrowSource", PropertyKind::UInt16),
        0x0A3F => ("ArrowTarget", PropertyKind::UInt16),
        0x0A60 => ("Edition", PropertyKind::Binary),
        0x0A61 => ("EditionAlias", PropertyKind::Binary),
        0x0A62 => ("MacPICT", PropertyKind::Binary),
        0x0A63 => ("WindowsMetafile", PropertyKind::Binary),
        0x0A64 => ("OLEObject", PropertyKind::Binary),
        0x0A65 => ("EnhancedMetafile", PropertyKind::Binary),
        0x0A6E => ("GIF", PropertyKind::Binary),
        0x0A6F => ("TIFF", PropertyKind::Binary),
        0x0A70 => ("PNG", PropertyKind::Binary),
        0x0A71 => ("JPEG", PropertyKind::Binary),
        0x0A72 => ("BMP", PropertyKind::Binary),
        0x0AB1 => ("Tail", PropertyKind::Coordinate),
        0x0B00 => ("TextFrame", PropertyKind::Rectangle),
        0x0B80 => ("GeometricFeature", PropertyKind::Int8),
        0x0B81 => ("RelationValue", PropertyKind::Float64),
        0x0B82 => ("BasisObjects", PropertyKind::ObjectIdArray),
        0x0B83 => ("ConstraintType", PropertyKind::Int8),
        0x0D06 => ("PositioningType", PropertyKind::Enum8(POSITIONING_TYPE)),
        0x0D07 => ("PositioningAngle", PropertyKind::Fixed16_16),
        0x0D08 => ("PositioningOffset", PropertyKind::Point2D),
        _ => return None,
    };
    Some(PropertySchema {
        name: schema.0,
        kind: schema.1,
    })
}

fn property_tag(name: &str) -> Option<u16> {
    Some(match name {
        "CreationUserName" => 0x0001,
        "CreationProgram" => 0x0003,
        "ModificationUserName" => 0x0004,
        "ModificationProgram" => 0x0006,
        "Name" => 0x0008,
        "Comment" => 0x0009,
        "Z" => 0x000A,
        "Visible" => 0x0011,
        "SupersededBy" => 0x0012,
        "p" => 0x0200,
        "xyz" => 0x0201,
        "extent" => 0x0202,
        "BoundingBox" => 0x0204,
        "RotationAngle" => 0x0205,
        "Head3D" => 0x0207,
        "Tail3D" => 0x0208,
        "TopLeft" => 0x0209,
        "TopRight" => 0x020A,
        "BottomRight" => 0x020B,
        "BottomLeft" => 0x020C,
        "Center3D" => 0x020D,
        "MajorAxisEnd3D" => 0x020E,
        "MinorAxisEnd3D" => 0x020F,
        "color" => 0x0301,
        "bgcolor" => 0x0302,
        "NodeType" => 0x0400,
        "LabelDisplay" => 0x0401,
        "Element" => 0x0402,
        "Isotope" => 0x0420,
        "Charge" => 0x0421,
        "Radical" => 0x0422,
        "ImplicitHydrogens" => 0x0424,
        "NumHydrogens" => 0x042B,
        "AS" => 0x0437,
        "AtomNumber" => 0x0439,
        "ShowAtomStereo" => 0x043B,
        "ShowAtomNumber" => 0x043C,
        "IsotopicAbundance" => 0x043F,
        "HideImplicitHydrogens" => 0x0444,
        "ShowAtomEnhancedStereo" => 0x0445,
        "Order" => 0x0600,
        "Display" => 0x0601,
        "Display2" => 0x0602,
        "DoublePosition" => 0x0603,
        "B" => 0x0604,
        "E" => 0x0605,
        "BeginAttach" => 0x0608,
        "EndAttach" => 0x0609,
        "BS" => 0x060A,
        "BondCircularOrdering" => 0x060B,
        "GenericNickname" => 0x0433,
        "ConnectionOrder" => 0x0505,
        "CrossingBonds" => 0x060E,
        "Justification" => 0x0701,
        "LineHeight" => 0x0702,
        "WordWrapWidth" => 0x0703,
        "LineStarts" => 0x0704,
        "LabelAlignment" => 0x0705,
        "LabelLineHeight" => 0x0706,
        "CaptionLineHeight" => 0x0707,
        "InterpretChemically" => 0x0708,
        "UTF8Text" => 0x0709,
        "PrintMargins" => 0x0802,
        "ChainAngle" => 0x0803,
        "BondSpacing" => 0x0804,
        "BondLength" => 0x0805,
        "BoldWidth" => 0x0806,
        "LineWidth" => 0x0807,
        "MarginWidth" => 0x0808,
        "HashSpacing" => 0x0809,
        "CaptionJustification" => 0x080C,
        "FractionalWidths" => 0x080D,
        "WidthPages" => 0x080F,
        "HeightPages" => 0x0810,
        "Width" => 0x0812,
        "Height" => 0x0813,
        "Header" => 0x0815,
        "HeaderPosition" => 0x0816,
        "Footer" => 0x0817,
        "FooterPosition" => 0x0818,
        "PrintTrimMarks" => 0x0819,
        "LabelFont" => 0x081A,
        "CaptionFont" => 0x081B,
        "LabelSize" => 0x081C,
        "CaptionSize" => 0x081D,
        "LabelFace" => 0x081E,
        "CaptionFace" => 0x081F,
        "BondSpacingAbs" => 0x0822,
        "LabelJustification" => 0x0823,
        "WindowIsZoomed" => 0x0900,
        "WindowPosition" => 0x0901,
        "WindowSize" => 0x0902,
        "GraphicType" => 0x0A00,
        "LineType" => 0x0A01,
        "ArrowType" => 0x0A02,
        "RectangleType" => 0x0A03,
        "OvalType" => 0x0A04,
        "OrbitalType" => 0x0A05,
        "BracketType" => 0x0A06,
        "SymbolType" => 0x0A07,
        "HeadSize" => 0x0A20,
        "AngularSize" => 0x0A21,
        "LipSize" => 0x0A22,
        "BracketedObjectIDs" => 0x0A27,
        "GraphicID" => 0x0A2B,
        "ArrowheadType" => 0x0A2F,
        "ArrowheadCenterSize" => 0x0A30,
        "ArrowheadWidth" => 0x0A31,
        "ArrowShaftSpacing" => 0x0A33,
        "ArrowEquilibriumRatio" => 0x0A34,
        "ArrowheadHead" => 0x0A35,
        "ArrowheadTail" => 0x0A36,
        "FillType" => 0x0A37,
        "CurveSpacing" => 0x0A38,
        "Closed" => 0x0A39,
        "Dipole" => 0x0A3A,
        "NoGo" => 0x0A3B,
        "CornerRadius" => 0x0A3C,
        "ArrowSource" => 0x0A3E,
        "ArrowTarget" => 0x0A3F,
        "Tail" => 0x0AB1,
        "TextFrame" => 0x0B00,
        "GeometricFeature" => 0x0B80,
        "RelationValue" => 0x0B81,
        "BasisObjects" => 0x0B82,
        "ConstraintType" => 0x0B83,
        "PositioningType" => 0x0D06,
        "PositioningAngle" => 0x0D07,
        "PositioningOffset" => 0x0D08,
        _ => return None,
    })
}

fn encode_property(name: &str, value: &str) -> Option<(u16, Vec<u8>)> {
    let (tag, official_type) = match property_tag(name) {
        Some(tag) => (tag, None),
        None => {
            let (tag, kind) = official_property_tag_and_type(name)?;
            (tag, Some(kind))
        }
    };
    let Some(schema) = property_schema(tag) else {
        return Some((
            tag,
            encode_official_lexical(official_type.as_deref()?, value)?,
        ));
    };
    let bytes = match schema.kind {
        PropertyKind::String => encode_plain_cdx_string(value),
        PropertyKind::Binary => decode_hex_bytes(value)?,
        PropertyKind::Point2D => encode_point2d(value)?,
        PropertyKind::Point3D => encode_point3d(value)?,
        PropertyKind::Rectangle => encode_rectangle(value)?,
        PropertyKind::Coordinate => encode_coordinate(value)?,
        PropertyKind::Int8 => vec![value.parse::<i8>().ok()? as u8],
        PropertyKind::UInt8 => vec![value.parse::<u8>().ok()?],
        PropertyKind::Int16 => value.parse::<i16>().ok()?.to_le_bytes().to_vec(),
        PropertyKind::UInt16 => value.parse::<u16>().ok()?.to_le_bytes().to_vec(),
        PropertyKind::LineHeightInt16 => encode_line_height(value, false)?,
        PropertyKind::LineHeightUInt16 => encode_line_height(value, true)?,
        PropertyKind::Fixed16_16 => ((value.parse::<f64>().ok()? * 65536.0).round() as i32)
            .to_le_bytes()
            .to_vec(),
        PropertyKind::UInt32 => value.parse::<u32>().ok()?.to_le_bytes().to_vec(),
        PropertyKind::Float64 => value.parse::<f64>().ok()?.to_le_bytes().to_vec(),
        PropertyKind::Boolean => vec![if yes(value) { 1 } else { 0 }],
        PropertyKind::BooleanImplied => {
            if yes(value) {
                Vec::new()
            } else {
                return None;
            }
        }
        PropertyKind::BondOrder => encode_bond_order(value)?,
        PropertyKind::BondSpacing => ((value.parse::<f64>().ok()? * 10.0).round() as i16)
            .to_le_bytes()
            .to_vec(),
        PropertyKind::AngleTenths => ((value.parse::<f64>().ok()? * 10.0).round() as i16)
            .to_le_bytes()
            .to_vec(),
        PropertyKind::FontStyle => return None,
        PropertyKind::ObjectIdArray => encode_u32_array(value)?,
        PropertyKind::Int16ListWithCounts => encode_i16_counted_list(value)?,
        PropertyKind::Enum8(values) => vec![enum_value(value, values)? as i8 as u8],
        PropertyKind::Enum(values) => enum_value(value, values)?.to_le_bytes().to_vec(),
        PropertyKind::BitFlags(values) => encode_bit_flags(value, values)?.to_le_bytes().to_vec(),
    };
    Some((tag, bytes))
}

fn object_name(tag: u16) -> Option<&'static str> {
    Some(match tag {
        0x8000 => "CDXML",
        0x8001 => "page",
        0x8002 => "group",
        0x8003 => "fragment",
        0x8004 => "n",
        0x8005 => "b",
        0x8006 => "t",
        0x8007 => "graphic",
        0x8008 => "curve",
        0x8009 => "embeddedobject",
        0x800A => "altgroup",
        0x800B => "templategrid",
        0x800C => "regnum",
        0x800D => "scheme",
        0x800E => "step",
        0x8010 => "spectrum",
        0x8011 => "objecttag",
        0x8013 => "sequence",
        0x8014 => "crossreference",
        0x8015 => "splitter",
        0x8016 => "table",
        0x8017 => "bracketedgroup",
        0x8018 => "bracketattachment",
        0x8019 => "crossingbond",
        0x8020 => "border",
        0x8021 => "geometry",
        0x8022 => "constraint",
        0x8023 => "tlcplate",
        0x8024 => "tlclane",
        0x8025 => "tlcspot",
        0x8026 => "chemicalproperty",
        0x8027 => "arrow",
        _ => return None,
    })
}

fn legacy_chemsema_object_name(tag: u16) -> Option<&'static str> {
    Some(match tag {
        0x801B => "geometry",
        0x801C => "constraint",
        0x801D => "tlcplate",
        0x801E => "tlclane",
        0x801F => "tlcspot",
        0x8020 => "chemicalproperty",
        0x8021 => "arrow",
        0x8025 => "bioshape",
        0x802A => "border",
        0x802B => "annotation",
        _ => return None,
    })
}

fn object_tag(name: &str) -> Option<u16> {
    Some(match name {
        "CDXML" => 0x8000,
        "page" => 0x8001,
        "group" => 0x8002,
        "fragment" => 0x8003,
        "n" => 0x8004,
        "b" => 0x8005,
        "t" => 0x8006,
        "graphic" => 0x8007,
        "curve" => 0x8008,
        "embeddedobject" => 0x8009,
        "altgroup" => 0x800A,
        "templategrid" => 0x800B,
        "regnum" => 0x800C,
        "scheme" => 0x800D,
        "step" => 0x800E,
        "spectrum" => 0x8010,
        "objecttag" => 0x8011,
        "sequence" => 0x8013,
        "crossreference" => 0x8014,
        "splitter" => 0x8015,
        "table" => 0x8016,
        "bracketedgroup" => 0x8017,
        "bracketattachment" => 0x8018,
        "crossingbond" => 0x8019,
        "geometry" => 0x801B,
        "constraint" => 0x801C,
        "tlcplate" => 0x801D,
        "tlclane" => 0x801E,
        "tlcspot" => 0x801F,
        "chemicalproperty" => 0x8020,
        "arrow" => 0x8021,
        "border" => 0x802A,
        _ => return None,
    })
}

fn is_object_tag(tag: u16) -> bool {
    tag & 0x8000 != 0
}

const BOND_DISPLAY: &[(i16, &str)] = &[
    (0, "Solid"),
    (1, "Dash"),
    (2, "Hash"),
    (3, "WedgedHashBegin"),
    (4, "WedgedHashEnd"),
    (5, "Bold"),
    (6, "WedgeBegin"),
    (7, "WedgeEnd"),
    (8, "Wavy"),
    (9, "HollowWedgeBegin"),
    (10, "HollowWedgeEnd"),
    (11, "WavyWedgeBegin"),
    (12, "WavyWedgeEnd"),
    (13, "Dot"),
    (14, "DashDot"),
];
const DOUBLE_POSITION: &[(i16, &str)] = &[
    (0, "Center"),
    (1, "Right"),
    (2, "Left"),
    (256, "Center"),
    (257, "Right"),
    (258, "Left"),
];
const ATOM_STEREO: &[(i16, &str)] = &[
    (0, "U"),
    (1, "N"),
    (2, "R"),
    (3, "S"),
    (4, "r"),
    (5, "s"),
    (6, "u"),
];
const ATOM_RADICAL: &[(i16, &str)] = &[(0, "None"), (1, "Singlet"), (2, "Doublet"), (3, "Triplet")];
const ISOTOPIC_ABUNDANCE: &[(i16, &str)] = &[
    (0, "Unspecified"),
    (1, "Any"),
    (2, "Natural"),
    (3, "Enriched"),
    (4, "Deficient"),
    (5, "Nonnatural"),
];
const BOND_STEREO: &[(i16, &str)] = &[(0, "U"), (1, "N"), (2, "E"), (3, "Z")];
const NODE_TYPE: &[(i16, &str)] = &[
    (0, "Unspecified"),
    (1, "Element"),
    (2, "ElementList"),
    (3, "ElementListNickname"),
    (4, "Nickname"),
    (5, "Fragment"),
    (6, "Formula"),
    (7, "GenericNickname"),
    (8, "AnonymousAlternativeGroup"),
    (9, "NamedAlternativeGroup"),
    (10, "MultiAttachment"),
    (11, "VariableAttachment"),
    (12, "ExternalConnectionPoint"),
    (13, "LinkNode"),
];
const LABEL_DISPLAY: &[(i16, &str)] = &[
    (0, "Auto"),
    (1, "Left"),
    (2, "Center"),
    (3, "Right"),
    (4, "Above"),
    (5, "Below"),
];
const JUSTIFICATION: &[(i16, &str)] = &[
    (-1, "Right"),
    (0, "Left"),
    (1, "Center"),
    (2, "Full"),
    (3, "Above"),
    (4, "Below"),
    (5, "Auto"),
    (6, "Best"),
];
const LABEL_ALIGNMENT: &[(i16, &str)] = &[
    (0, "Auto"),
    (1, "Left"),
    (2, "Center"),
    (3, "Right"),
    (4, "Above"),
    (5, "Below"),
    (6, "Best"),
];
const GRAPHIC_TYPE: &[(i16, &str)] = &[
    (1, "Line"),
    (2, "Arc"),
    (3, "Rectangle"),
    (4, "Oval"),
    (5, "Orbital"),
    (6, "Bracket"),
    (7, "Symbol"),
];
const BRACKET_TYPE: &[(i16, &str)] = &[
    (0, "RoundPair"),
    (1, "SquarePair"),
    (2, "CurlyPair"),
    (3, "Square"),
    (4, "Curly"),
    (5, "Round"),
];
const POSITIONING_TYPE: &[(i16, &str)] =
    &[(0, "auto"), (1, "angle"), (2, "offset"), (3, "absolute")];
const SYMBOL_TYPE: &[(i16, &str)] = &[
    (0, "LonePair"),
    (1, "Electron"),
    (2, "RadicalCation"),
    (3, "RadicalAnion"),
    (4, "CirclePlus"),
    (5, "CircleMinus"),
    (6, "Dagger"),
    (7, "DoubleDagger"),
    (8, "Plus"),
    (9, "Minus"),
    (10, "Racemic"),
    (11, "Absolute"),
    (12, "Relative"),
];
const LINE_TYPE: &[(i16, &str)] = &[(0, "Solid"), (1, "Dashed"), (2, "Bold"), (4, "Wavy")];
const RECTANGLE_TYPE: &[(i16, &str)] = &[
    (0, "Plain"),
    (1, "RoundEdge"),
    (2, "Shadow"),
    (4, "Shaded"),
    (8, "Filled"),
    (16, "Dashed"),
    (32, "Bold"),
];
const OVAL_TYPE: &[(i16, &str)] = &[
    (0, "Plain"),
    (1, "Circle"),
    (2, "Shaded"),
    (4, "Filled"),
    (8, "Dashed"),
    (16, "Bold"),
    (32, "Shadowed"),
];
const ARROW_TYPE: &[(i16, &str)] = &[
    (0, "NoHead"),
    (1, "HalfHead"),
    (2, "FullHead"),
    (4, "Resonance"),
    (8, "Equilibrium"),
    (16, "Hollow"),
    (32, "RetroSynthetic"),
    (64, "NoGo"),
    (128, "Dipole"),
];
const ARROW_HEAD_TYPE: &[(i16, &str)] = &[
    (0, "Unspecified"),
    (1, "Solid"),
    (2, "Hollow"),
    (3, "Angle"),
];
// Modern Arrow objects use a different enum from legacy Graphic/ArrowType.
// Values 2..4 are confirmed by ChemDraw's own CDXML -> CDX round trip.
const ARROW_HEAD_POSITION: &[(i16, &str)] = &[
    (0, "None"),
    (1, "Unspecified"),
    (2, "Full"),
    (3, "HalfLeft"),
    (4, "HalfRight"),
];
const NO_GO: &[(i16, &str)] = &[(0, "None"), (1, "None"), (2, "Cross"), (3, "Hash")];
const FILL_TYPE: &[(i16, &str)] = &[(0, "Unspecified"), (1, "None"), (2, "Solid"), (3, "Shaded")];
const ORBITAL_TYPE: &[(i16, &str)] = &[
    (0, "s"),
    (1, "oval"),
    (2, "lobe"),
    (3, "p"),
    (4, "hybridPlus"),
    (5, "hybridMinus"),
    (6, "dz2Plus"),
    (7, "dz2Minus"),
    (8, "dxy"),
    (256, "sFilled"),
    (257, "ovalFilled"),
    (258, "lobeFilled"),
    (259, "pFilled"),
    (260, "hybridPlusFilled"),
    (261, "hybridMinusFilled"),
    (262, "dz2PlusFilled"),
    (263, "dz2MinusFilled"),
    (264, "dxyFilled"),
    (512, "sShaded"),
    (513, "ovalShaded"),
    (514, "lobeShaded"),
];

fn enum_name(value: i16, values: &'static [(i16, &'static str)]) -> &'static str {
    values
        .iter()
        .find_map(|(candidate, name)| (*candidate == value).then_some(*name))
        .unwrap_or("Unspecified")
}

fn enum_value(name: &str, values: &'static [(i16, &'static str)]) -> Option<i16> {
    values
        .iter()
        .find_map(|(value, candidate)| candidate.eq_ignore_ascii_case(name).then_some(*value))
}

fn decode_bit_flags(value: i16, values: &'static [(i16, &'static str)]) -> String {
    if value == 0 {
        return values
            .iter()
            .find_map(|(flag, name)| (*flag == 0).then_some(*name))
            .unwrap_or("0")
            .to_string();
    }
    let names = values
        .iter()
        .filter_map(|(flag, name)| (*flag != 0 && value & *flag == *flag).then_some(*name))
        .collect::<Vec<_>>();
    if names.is_empty() {
        value.to_string()
    } else {
        names.join(" ")
    }
}

fn encode_bit_flags(value: &str, values: &'static [(i16, &'static str)]) -> Option<i16> {
    if let Ok(numeric) = value.parse::<i16>() {
        return Some(numeric);
    }
    let mut encoded = 0_i16;
    let mut matched = false;
    for token in value.split_whitespace() {
        let flag = enum_value(token, values)?;
        encoded |= flag;
        matched = true;
    }
    matched.then_some(encoded)
}

fn read_i8(data: &[u8]) -> Option<i8> {
    data.first().map(|value| *value as i8)
}

fn read_u8(data: &[u8]) -> Option<u8> {
    data.first().copied()
}

fn read_i16(data: &[u8]) -> Option<i16> {
    (data.len() >= 2).then(|| i16::from_le_bytes([data[0], data[1]]))
}

fn read_u16(data: &[u8]) -> Option<u16> {
    (data.len() >= 2).then(|| u16::from_le_bytes([data[0], data[1]]))
}

fn read_i32(data: &[u8]) -> Option<i32> {
    (data.len() >= 4).then(|| i32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

fn read_u32(data: &[u8]) -> Option<u32> {
    (data.len() >= 4).then(|| u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

fn read_f64(data: &[u8]) -> Option<f64> {
    (data.len() >= 8).then(|| {
        f64::from_le_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ])
    })
}

fn read_i16_lossy(data: &[u8]) -> Option<i16> {
    if data.len() == 1 {
        Some(data[0] as i8 as i16)
    } else {
        read_i16(data)
    }
}

fn read_u16_lossy(data: &[u8]) -> Option<u16> {
    if data.len() == 1 {
        Some(data[0] as u16)
    } else {
        read_u16(data)
    }
}

fn decode_coordinate(data: &[u8]) -> Option<String> {
    Some(fmt_num(read_i32(data)? as f64 / CDX_COORD_FACTOR))
}

fn decode_point2d(data: &[u8]) -> Option<String> {
    if data.len() < 8 {
        return None;
    }
    let y = i32::from_le_bytes([data[0], data[1], data[2], data[3]]) as f64 / CDX_COORD_FACTOR;
    let x = i32::from_le_bytes([data[4], data[5], data[6], data[7]]) as f64 / CDX_COORD_FACTOR;
    Some(format!("{} {}", fmt_num(x), fmt_num(y)))
}

fn decode_point3d(data: &[u8]) -> Option<String> {
    if data.len() < 12 {
        return None;
    }
    let x = i32::from_le_bytes([data[0], data[1], data[2], data[3]]) as f64 / CDX_COORD_FACTOR;
    let y = i32::from_le_bytes([data[4], data[5], data[6], data[7]]) as f64 / CDX_COORD_FACTOR;
    let z = i32::from_le_bytes([data[8], data[9], data[10], data[11]]) as f64 / CDX_COORD_FACTOR;
    Some(format!("{} {} {}", fmt_num(x), fmt_num(y), fmt_num(z)))
}

fn decode_rectangle(data: &[u8]) -> Option<String> {
    if data.len() < 16 {
        return None;
    }
    let top = i32::from_le_bytes([data[0], data[1], data[2], data[3]]) as f64 / CDX_COORD_FACTOR;
    let left = i32::from_le_bytes([data[4], data[5], data[6], data[7]]) as f64 / CDX_COORD_FACTOR;
    let bottom =
        i32::from_le_bytes([data[8], data[9], data[10], data[11]]) as f64 / CDX_COORD_FACTOR;
    let right =
        i32::from_le_bytes([data[12], data[13], data[14], data[15]]) as f64 / CDX_COORD_FACTOR;
    Some(format!(
        "{} {} {} {}",
        fmt_num(left),
        fmt_num(top),
        fmt_num(right),
        fmt_num(bottom)
    ))
}

fn bool_from_bytes(data: &[u8]) -> String {
    if data.first().copied().unwrap_or(1) == 0 {
        "no".to_string()
    } else {
        "yes".to_string()
    }
}

fn decode_bond_order(data: &[u8]) -> Option<String> {
    const ORDERS: [&str; 16] = [
        "1",
        "2",
        "3",
        "4",
        "5",
        "6",
        "0.5",
        "1.5",
        "2.5",
        "3.5",
        "4.5",
        "5.5",
        "dative",
        "ionic",
        "hydrogen",
        "threecenter",
    ];
    let value = read_u16(data)?;
    if value == 0 || value == 0xFFFF {
        return Some(String::new());
    }
    let parts: Vec<&str> = ORDERS
        .iter()
        .enumerate()
        .filter_map(|(index, order)| ((value & (1 << index)) != 0).then_some(*order))
        .collect();
    Some(parts.join(" "))
}

fn decode_font_style(data: &[u8]) -> Option<(u16, u16, f64, u16)> {
    if data.len() < 8 {
        return None;
    }
    let font = u16::from_le_bytes([data[0], data[1]]);
    let face = u16::from_le_bytes([data[2], data[3]]);
    let size = u16::from_le_bytes([data[4], data[5]]) as f64 / 20.0;
    let color = u16::from_le_bytes([data[6], data[7]]);
    Some((font, face, size, color))
}

fn decode_u32_array(data: &[u8]) -> Option<String> {
    if data.len() % 4 != 0 {
        return None;
    }
    Some(
        data.chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]).to_string())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn decode_u32_counted_array(data: &[u8]) -> Option<String> {
    let count = read_u16(data)? as usize;
    let body = data.get(2..2 + count * 4)?;
    decode_u32_array(body)
}

fn decode_element_list(data: &[u8]) -> Option<String> {
    let signed_count = read_i16(data)?;
    let count = signed_count.unsigned_abs() as usize;
    let body = data.get(2..2 + count * 2)?;
    let values = body
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]).to_string())
        .collect::<Vec<_>>()
        .join(" ");
    Some(if signed_count < 0 {
        format!("NOT {values}")
    } else {
        values
    })
}

fn decode_curve_points(data: &[u8], dimensions: usize) -> Option<String> {
    let count = read_u16(data)? as usize;
    let body = data.get(2..2 + count * dimensions * 4)?;
    let mut values = Vec::with_capacity(count * dimensions);
    for point in body.chunks_exact(dimensions * 4) {
        if dimensions == 2 {
            values.extend(
                decode_point2d(point)?
                    .split_whitespace()
                    .map(ToString::to_string),
            );
        } else {
            values.extend(
                decode_point3d(point)?
                    .split_whitespace()
                    .map(ToString::to_string),
            );
        }
    }
    Some(values.join(" "))
}

fn decode_cdx_date(data: &[u8]) -> Option<String> {
    if data.len() < 14 {
        return None;
    }
    Some(
        data[..14]
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]).to_string())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn decode_represents_property(data: &[u8]) -> Option<String> {
    if data.len() < 6 {
        return None;
    }
    Some(format!(
        "{} 0x{:04X}",
        read_u32(data)?,
        u16::from_le_bytes([data[4], data[5]])
    ))
}

fn decode_generic_list(data: &[u8]) -> Option<String> {
    let signed_count = read_i16(data)?;
    let count = signed_count.unsigned_abs() as usize;
    let mut offset = 2usize;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let len = read_u16(data.get(offset..)?)? as usize;
        offset += 2;
        let item = data.get(offset..offset + len)?;
        offset += len;
        values.push(parse_cdx_string(item, None).text);
    }
    let joined = values.join(" ");
    Some(if signed_count < 0 {
        format!("NOT {joined}")
    } else {
        joined
    })
}

fn decode_i16_counted_list(data: &[u8]) -> Option<String> {
    if data.len() < 2 {
        return None;
    }
    let count = u16::from_le_bytes([data[0], data[1]]) as usize;
    if data.len() < 2 + count * 2 {
        return None;
    }
    Some(
        data[2..2 + count * 2]
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]).to_string())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn encode_plain_cdx_string(value: &str) -> Vec<u8> {
    let mut out = Vec::new();
    write_u16(&mut out, 0);
    out.extend_from_slice(value.replace('\n', "\r").as_bytes());
    out
}

fn encode_cdx_string(node: &crate::cdxml::xml::XmlNode) -> Vec<u8> {
    let runs: Vec<_> = node.direct_children("s").collect();
    let mut out = Vec::new();
    write_u16(&mut out, runs.len() as u16);
    let mut text = String::new();
    let mut starts = Vec::new();
    for run in &runs {
        starts.push(text.chars().count() as u16);
        text.push_str(&run.full_text());
    }
    if runs.is_empty() {
        text.push_str(&node.full_text());
    }
    for (index, run) in runs.iter().enumerate() {
        write_u16(&mut out, starts[index]);
        write_u16(
            &mut out,
            run.attr("font").and_then(|v| v.parse().ok()).unwrap_or(3),
        );
        write_u16(
            &mut out,
            run.attr("face").and_then(|v| v.parse().ok()).unwrap_or(0),
        );
        write_u16(
            &mut out,
            (run.attr("size")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(12.0)
                * 20.0)
                .round() as u16,
        );
        write_u16(
            &mut out,
            run.attr("color").and_then(|v| v.parse().ok()).unwrap_or(0),
        );
    }
    out.extend_from_slice(text.replace('\n', "\r").as_bytes());
    out
}

fn encode_bond_order(value: &str) -> Option<Vec<u8>> {
    let mut encoded = 0u16;
    for part in value.split_whitespace() {
        encoded |= match part {
            "1" => 0x0001,
            "2" => 0x0002,
            "3" => 0x0004,
            "4" => 0x0008,
            "5" => 0x0010,
            "6" => 0x0020,
            "0.5" => 0x0040,
            "1.5" => 0x0080,
            "2.5" => 0x0100,
            "3.5" => 0x0200,
            "4.5" => 0x0400,
            "5.5" => 0x0800,
            "dative" => 0x1000,
            "ionic" => 0x2000,
            "hydrogen" => 0x4000,
            "threecenter" => 0x8000,
            _ => return None,
        };
    }
    if encoded == 0 {
        encoded = 0xFFFF;
    }
    Some(encoded.to_le_bytes().to_vec())
}

fn encode_coordinate(value: &str) -> Option<Vec<u8>> {
    let coord = (value.parse::<f64>().ok()? * CDX_COORD_FACTOR).round() as i32;
    Some(coord.to_le_bytes().to_vec())
}

fn encode_point2d(value: &str) -> Option<Vec<u8>> {
    let values = parse_numbers(value)?;
    if values.len() < 2 {
        return None;
    }
    let mut out = Vec::new();
    out.extend_from_slice(&((values[1] * CDX_COORD_FACTOR).round() as i32).to_le_bytes());
    out.extend_from_slice(&((values[0] * CDX_COORD_FACTOR).round() as i32).to_le_bytes());
    Some(out)
}

fn encode_point3d(value: &str) -> Option<Vec<u8>> {
    let values = parse_numbers(value)?;
    if values.len() < 3 {
        return None;
    }
    let mut out = Vec::new();
    for value in values.iter().take(3) {
        out.extend_from_slice(&((value * CDX_COORD_FACTOR).round() as i32).to_le_bytes());
    }
    Some(out)
}

fn encode_rectangle(value: &str) -> Option<Vec<u8>> {
    let values = parse_numbers(value)?;
    if values.len() < 4 {
        return None;
    }
    let mut out = Vec::new();
    for value in [values[1], values[0], values[3], values[2]] {
        out.extend_from_slice(&((value * CDX_COORD_FACTOR).round() as i32).to_le_bytes());
    }
    Some(out)
}

fn encode_u32_array(value: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    for part in value.split_whitespace() {
        write_u32(&mut out, part.parse().ok()?);
    }
    Some(out)
}

fn encode_u32_counted_array(value: &str) -> Option<Vec<u8>> {
    let body = encode_u32_array(value)?;
    let count = u16::try_from(body.len() / 4).ok()?;
    let mut out = count.to_le_bytes().to_vec();
    out.extend_from_slice(&body);
    Some(out)
}

fn encode_element_list(value: &str) -> Option<Vec<u8>> {
    let mut tokens = value.split_whitespace();
    let excluded = tokens
        .clone()
        .next()
        .is_some_and(|token| token.eq_ignore_ascii_case("NOT"));
    if excluded {
        tokens.next();
    }
    let values: Option<Vec<u16>> = tokens.map(|token| token.parse().ok()).collect();
    let values = values?;
    let count = i16::try_from(values.len()).ok()?;
    let signed_count = if excluded { -count } else { count };
    let mut out = signed_count.to_le_bytes().to_vec();
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    Some(out)
}

fn encode_curve_points(value: &str, dimensions: usize) -> Option<Vec<u8>> {
    let values = parse_numbers(value)?;
    if values.len() % dimensions != 0 {
        return None;
    }
    let count = u16::try_from(values.len() / dimensions).ok()?;
    let mut out = count.to_le_bytes().to_vec();
    for point in values.chunks_exact(dimensions) {
        let lexical = point
            .iter()
            .map(|value| fmt_num(*value))
            .collect::<Vec<_>>()
            .join(" ");
        let encoded = if dimensions == 2 {
            encode_point2d(&lexical)?
        } else {
            encode_point3d(&lexical)?
        };
        out.extend_from_slice(&encoded);
    }
    Some(out)
}

fn encode_cdx_date(value: &str) -> Option<Vec<u8>> {
    let values: Option<Vec<i16>> = value
        .split_whitespace()
        .map(|part| part.parse().ok())
        .collect();
    let values = values?;
    if values.len() != 7 {
        return None;
    }
    let mut out = Vec::with_capacity(14);
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    Some(out)
}

fn encode_represents_property(value: &str) -> Option<Vec<u8>> {
    let mut parts = value.split_whitespace();
    let object_id = parts.next()?.parse::<u32>().ok()?;
    let property_tag = parse_hex_u16(parts.next()?)?;
    let mut out = object_id.to_le_bytes().to_vec();
    out.extend_from_slice(&property_tag.to_le_bytes());
    Some(out)
}

fn encode_generic_list(value: &str) -> Option<Vec<u8>> {
    let mut tokens = value.split_whitespace();
    let excluded = tokens
        .clone()
        .next()
        .is_some_and(|token| token.eq_ignore_ascii_case("NOT"));
    if excluded {
        tokens.next();
    }
    let values: Vec<&str> = tokens.collect();
    let count = i16::try_from(values.len()).ok()?;
    let mut out = (if excluded { -count } else { count })
        .to_le_bytes()
        .to_vec();
    for value in values {
        let encoded = encode_plain_cdx_string(value);
        write_u16(&mut out, u16::try_from(encoded.len()).ok()?);
        out.extend_from_slice(&encoded);
    }
    Some(out)
}

fn encode_i16_counted_list(value: &str) -> Option<Vec<u8>> {
    let values: Option<Vec<i16>> = value
        .split_whitespace()
        .map(|part| part.parse::<i16>().ok())
        .collect();
    let values = values?;
    let mut out = Vec::new();
    write_u16(&mut out, values.len() as u16);
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    Some(out)
}

fn encode_color_table(node: &crate::cdxml::xml::XmlNode) -> Vec<u8> {
    let colors: Vec<_> = node.direct_children("color").collect();
    let mut out = Vec::new();
    write_u16(&mut out, colors.len() as u16);
    for color in colors {
        for key in ["r", "g", "b"] {
            let value = color
                .attr(key)
                .and_then(|value| value.parse::<f64>().ok())
                .unwrap_or(0.0);
            write_u16(&mut out, (value.clamp(0.0, 1.0) * 65_535.0).round() as u16);
        }
    }
    out
}

fn encode_font_table(node: &crate::cdxml::xml::XmlNode) -> Vec<u8> {
    let fonts: Vec<_> = node.direct_children("font").collect();
    let mut out = Vec::new();
    write_u16(&mut out, 1);
    write_u16(&mut out, fonts.len() as u16);
    for font in fonts {
        let name = font.attr("name").unwrap_or("Arial");
        write_u16(
            &mut out,
            font.attr("id")
                .and_then(|value| value.parse().ok())
                .unwrap_or(3),
        );
        write_u16(
            &mut out,
            charset_id(font.attr("charset").unwrap_or("iso-8859-1")),
        );
        write_u16(&mut out, name.len() as u16);
        out.extend_from_slice(name.as_bytes());
    }
    out
}

fn parse_numbers(value: &str) -> Option<Vec<f64>> {
    let values: Option<Vec<f64>> = value
        .split_whitespace()
        .map(|part| part.parse::<f64>().ok())
        .collect();
    values
}

fn write_property(out: &mut Vec<u8>, tag: u16, data: &[u8]) {
    write_u16(out, tag);
    if data.len() > 65_534 {
        write_u16(out, 0xFFFF);
        write_u32(out, data.len() as u32);
    } else {
        write_u16(out, data.len() as u16);
    }
    out.extend_from_slice(data);
}

fn write_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn yes(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "yes" | "true" | "1"
    )
}

fn fmt_num(value: f64) -> String {
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

fn escape_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn attr_order(key: &str) -> usize {
    match key {
        "id" => 0,
        "BoundingBox" => 1,
        "p" => 2,
        "Z" => 3,
        _ => 10,
    }
}

fn charset_name(id: u16) -> &'static str {
    match id {
        0 => "iso-8859-1",
        1 => "iso-8859-1",
        65001 => "UTF-8",
        1252 => "iso-8859-1",
        _ => "iso-8859-1",
    }
}

fn charset_id(name: &str) -> u16 {
    match name.to_ascii_lowercase().as_str() {
        "utf-8" | "utf8" => 65001,
        "iso-8859-1" | "latin1" | "windows-1252" | "cp1252" => 1252,
        _ => 1252,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cdx_string_ffff_style_components_remain_inherited() {
        let text = CdxNode {
            name: "t".to_string(),
            tag: 0x8006,
            id: 2,
            attrs: BTreeMap::from([("id".to_string(), "2".to_string())]),
            properties: Vec::new(),
            text_runs: vec![CdxTextRun {
                start: 0,
                font: u16::MAX,
                face: u16::MAX,
                size: 20.0,
                color: u16::MAX,
            }],
            text: Some("BINAP".to_string()),
            children: Vec::new(),
        };
        let root = CdxNode {
            name: "CDXML".to_string(),
            tag: 0x8000,
            id: 1,
            attrs: BTreeMap::from([("id".to_string(), "1".to_string())]),
            properties: Vec::new(),
            text_runs: Vec::new(),
            text: None,
            children: vec![text],
        };
        let decoded = CdxmlWriter::new().write(&root);
        assert!(decoded.contains("<s size=\"20\">BINAP</s>"));
        assert!(!decoded.contains("65535"));
    }

    #[test]
    fn cdx_font_style_ffff_components_remain_inherited() {
        let mut cdx = CDX_HEADER.to_vec();
        write_u16(&mut cdx, 0x8000);
        write_u32(&mut cdx, 1);
        write_property(
            &mut cdx,
            0x080B,
            &[0xff, 0xff, 0xff, 0xff, 0xc8, 0x00, 0xff, 0xff],
        );
        write_u16(&mut cdx, 0);
        write_u16(&mut cdx, 0);

        let decoded = cdx_to_cdxml(&cdx).expect("font style should decode");
        assert!(decoded.contains("CaptionSize=\"10\""));
        assert!(!decoded.contains("CaptionFont="));
        assert!(!decoded.contains("CaptionFace="));
        assert!(!decoded.contains("CaptionColor="));
        assert!(!decoded.contains("65535"));
    }

    #[test]
    fn cdx_int16_properties_accept_legacy_single_byte_storage() {
        assert_eq!(
            decode_property(0x0402, &[7], None),
            Some(("Element", "7".to_string()))
        );
        assert_eq!(
            encode_property("Element", "7"),
            Some((0x0402, 7_i16.to_le_bytes().to_vec()))
        );
    }

    #[test]
    fn cdx_roundtrip_imports_basic_molecule() {
        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML CreationProgram="ChemSema" BoundingBox="0 0 120 80" LabelFont="3" LabelSize="10" LabelFace="96" CaptionFont="3" CaptionSize="10" LineWidth="1" BoldWidth="4" BondLength="18" BondSpacing="18" HashSpacing="2.5" MarginWidth="2">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
  </colortable>
  <fonttable>
    <font id="3" charset="iso-8859-1" name="Arial"/>
  </fonttable>
  <page id="1" BoundingBox="0 0 120 80" Width="120" Height="80">
    <fragment id="2" BoundingBox="10 10 60 20">
      <n id="3" p="10 10" Element="6"/>
      <n id="4" p="60 20" Element="8" NumHydrogens="1"/>
      <b id="5" B="3" E="4" Order="1" Display="Dash"/>
    </fragment>
  </page>
</CDXML>
"#;
        let cdx = cdxml_to_cdx(cdxml).expect("CDXML should encode to CDX");
        let decoded = cdx_to_cdxml(&cdx).expect("CDX should decode to CDXML");
        assert!(decoded.contains("<fragment"));
        assert!(decoded.contains("Display=\"Dash\""));
        let doc = parse_cdx_document(&cdx, Some("basic")).expect("CDX should import");
        assert_eq!(doc.resources.len(), 1);
    }

    #[test]
    fn cdx_symbol_type_uses_official_enum_names_and_values() {
        let encoded = encode_property("SymbolType", "Plus").expect("plus symbol should encode");
        assert_eq!(encoded.0, 0x0A07);
        assert_eq!(encoded.1, 8_i16.to_le_bytes());
        let (_, decoded) =
            decode_property(encoded.0, &encoded.1, None).expect("plus symbol should decode");
        assert_eq!(decoded, "Plus");

        let cdxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LineWidth="0.6" BondLength="14.4">
  <page id="1">
    <graphic id="2" GraphicType="Symbol" SymbolType="Plus"
      BoundingBox="20 20 20 30"/>
  </page>
</CDXML>"#;
        let cdx = cdxml_to_cdx(cdxml).expect("symbol CDXML should encode");
        let decoded_cdxml = cdx_to_cdxml(&cdx).expect("symbol CDX should decode");
        assert!(
            decoded_cdxml.contains("SymbolType=\"Plus\""),
            "{decoded_cdxml}"
        );
        let document = parse_cdx_document(&cdx, Some("plus symbol"))
            .expect("symbol CDX should import into the document model");
        assert!(document.scene_objects().iter().any(|object| {
            object.object_type == "symbol"
                && object
                    .payload
                    .extra
                    .get("kind")
                    .and_then(serde_json::Value::as_str)
                    == Some("plus")
        }));
    }

    #[test]
    fn cdx_restrict_implicit_hydrogens_uses_official_implied_boolean_tag() {
        let encoded = encode_property("ImplicitHydrogens", "yes")
            .expect("implicit-hydrogen restriction should encode");
        assert_eq!(encoded.0, 0x0424);
        assert!(encoded.1.is_empty());
        let (_, decoded) = decode_property(encoded.0, &encoded.1, None)
            .expect("implicit-hydrogen restriction should decode");
        assert_eq!(decoded, "yes");

        let cdxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CDXML><page><fragment>
  <n id="1" p="10 10" Element="6" ImplicitHydrogens="yes">
    <t p="10 10" UTF8Text="C"/>
  </n>
  <n id="2" p="30 10" Element="6"/>
  <b id="3" B="1" E="2"/>
</fragment></page></CDXML>"#;
        let cdx = cdxml_to_cdx(cdxml).expect("query CDXML should encode to CDX");
        let decoded_cdxml = cdx_to_cdxml(&cdx).expect("query CDX should decode");
        assert!(
            decoded_cdxml.contains("ImplicitHydrogens=\"yes\""),
            "{decoded_cdxml}"
        );
        let document = parse_cdx_document(&cdx, Some("atom query"))
            .expect("query CDX should import into the document model");
        let node = document
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "1"))
            .expect("query node should survive");
        assert_eq!(
            node.meta
                .pointer("/import/cdxml/restrictImplicitHydrogens")
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn cdx_crossing_bonds_use_official_object_id_array_tag_and_round_trip() {
        let encoded =
            encode_property("CrossingBonds", "20 21").expect("CrossingBonds should encode");
        assert_eq!(encoded.0, 0x060E);
        let (_, decoded) =
            decode_property(encoded.0, &encoded.1, None).expect("CrossingBonds should decode");
        assert_eq!(decoded, "20 21");

        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 120 120" LineWidth="0.6" MarginWidth="1.6">
  <page id="1" BoundingBox="0 0 120 120">
    <fragment id="2">
      <n id="10" p="20 60"/><n id="11" p="100 60"/>
      <n id="12" p="60 20"/><n id="13" p="60 100"/>
      <b id="20" Z="7" B="10" E="11" CrossingBonds="21"/>
      <b id="21" Z="8" B="12" E="13" CrossingBonds="20"/>
    </fragment>
  </page>
</CDXML>"#;
        let cdx = cdxml_to_cdx(cdxml).expect("crossing CDXML should encode");
        let decoded_cdxml = cdx_to_cdxml(&cdx).expect("crossing CDX should decode");
        assert!(
            decoded_cdxml.contains("CrossingBonds=\"21\""),
            "{decoded_cdxml}"
        );
        assert!(
            decoded_cdxml.contains("CrossingBonds=\"20\""),
            "{decoded_cdxml}"
        );

        let imported = parse_cdx_document(&cdx, Some("crossings")).expect("CDX should import");
        let first = document_to_cdx(&imported).expect("crossing CDX should export");
        let reopened = parse_cdx_document(&first, Some("crossings"))
            .expect("exported crossing CDX should reopen");
        let second = document_to_cdx(&reopened).expect("crossing CDX should stabilize");
        assert_eq!(second, first);
    }

    #[test]
    fn cdx_text_preserves_style_runs_as_cdxml_s_elements() {
        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 120 80">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 120 80">
    <t id="2" p="10 10" BoundingBox="10 10 60 25"><s font="3" size="12" face="1" color="0">Hello</s><s font="3" size="8" face="0" color="0">2</s></t>
  </page>
</CDXML>
"#;
        let cdx = cdxml_to_cdx(cdxml).expect("CDXML should encode to CDX");
        let decoded = cdx_to_cdxml(&cdx).expect("CDX should decode to CDXML");
        assert!(decoded.contains("<s font=\"3\" size=\"12\" face=\"1\" color=\"0\">Hello</s>"));
        assert!(decoded.contains("<s font=\"3\" size=\"8\" face=\"0\" color=\"0\">2</s>"));
    }

    #[test]
    fn cdx_justification_enums_use_signed_single_byte_values() {
        for (name, encoded) in [
            ("Right", 0xff),
            ("Left", 0x00),
            ("Center", 0x01),
            ("Full", 0x02),
            ("Above", 0x03),
            ("Below", 0x04),
            ("Auto", 0x05),
            ("Best", 0x06),
        ] {
            let (tag, bytes) =
                encode_property("LabelJustification", name).expect("justification should encode");
            assert_eq!(tag, 0x0823);
            assert_eq!(bytes, vec![encoded], "{name}");
            let (_, decoded) =
                decode_property(tag, &bytes, None).expect("justification should decode");
            assert_eq!(decoded, name);
        }
    }

    #[test]
    fn cdx_label_display_and_alignment_use_single_byte_values() {
        for (property, expected_tag) in [("LabelDisplay", 0x0401), ("LabelAlignment", 0x0705)] {
            let (tag, bytes) = encode_property(property, "Right").expect("enum should encode");
            assert_eq!(tag, expected_tag);
            assert_eq!(bytes, vec![3]);
            let (_, decoded) = decode_property(tag, &bytes, None).expect("enum should decode");
            assert_eq!(decoded, "Right");
        }
    }

    #[test]
    fn cdx_open_save_stabilizes_label_layout_fields() {
        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 80 40" BondLength="14.4" LabelFont="3" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 80 40">
    <fragment id="2" BoundingBox="0 0 80 40">
      <n id="3" p="30 20" NodeType="Nickname">
        <t id="4" p="30 24" BoundingBox="0 10 30 26" LabelJustification="Right" Justification="Right" LabelAlignment="Right" UTF8Text="C10H21">
          <s font="3" size="10" face="96" color="0">C10H21</s>
        </t>
      </n>
      <n id="5" p="48 20"/>
      <b id="6" B="3" E="5"/>
    </fragment>
  </page>
</CDXML>"#;
        let source = cdxml_to_cdx(cdxml).expect("source CDX should encode");
        let imported = parse_cdx_document(&source, Some("stable CDX")).expect("source CDX import");
        let first = document_to_cdx(&imported).expect("first CDX export");
        let reopened = parse_cdx_document(&first, Some("stable CDX")).expect("reopen CDX");
        let second = document_to_cdx(&reopened).expect("second CDX export");

        assert_eq!(
            second, first,
            "CDX must stabilize after the first ChemSema save"
        );
        let decoded = cdx_to_cdxml(&first).expect("saved CDX should decode");
        for expected in [
            "LabelJustification=\"Right\"",
            "Justification=\"Right\"",
            "LabelAlignment=\"Right\"",
        ] {
            assert!(decoded.contains(expected), "missing {expected}: {decoded}");
        }
    }

    #[test]
    fn cdx_right_aligned_chemical_label_stays_reversed_across_open_save() {
        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 80 40" BondLength="14.4" LabelFont="3" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 80 40">
    <fragment id="2" BoundingBox="0 0 80 40">
      <n id="3" p="30 20" NodeType="Nickname">
        <t id="4" p="30 24" BoundingBox="0 10 30 26" LabelJustification="Right" Justification="Right" LabelAlignment="Right" UTF8Text="OCF3">
          <s font="3" size="10" face="96" color="0">OCF3</s>
        </t>
      </n>
      <n id="5" p="48 20"/>
      <b id="6" B="3" E="5"/>
    </fragment>
  </page>
</CDXML>"#;
        let source = cdxml_to_cdx(cdxml).expect("source CDX should encode");
        let imported =
            parse_cdx_document(&source, Some("right-aligned OCF3")).expect("source CDX import");
        let imported_label = imported
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .and_then(|fragment| fragment.nodes.iter().find_map(|node| node.label.as_ref()))
            .expect("imported OCF3 label");
        assert_eq!(imported_label.source_text.as_deref(), Some("OCF3"));
        assert_eq!(imported_label.text, "F3CO");

        let first = document_to_cdx(&imported).expect("first CDX export");
        let reopened = parse_cdx_document(&first, Some("right-aligned OCF3")).expect("reopen CDX");
        let reopened_label = reopened
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .and_then(|fragment| fragment.nodes.iter().find_map(|node| node.label.as_ref()))
            .expect("reopened OCF3 label");
        assert_eq!(reopened_label.source_text.as_deref(), Some("OCF3"));
        assert_eq!(reopened_label.text, "F3CO");

        let second = document_to_cdx(&reopened).expect("second CDX export");
        assert_eq!(second, first, "right-aligned OCF3 CDX must stabilize");
        let decoded = cdx_to_cdxml(&first).expect("saved CDX should decode");
        assert!(decoded.contains("LabelJustification=\"Right\""));
        assert!(decoded.contains("Justification=\"Right\""));
        assert!(decoded.contains("LabelAlignment=\"Right\""));
        assert!(
            !decoded.contains("LabelDisplay=\"Right\""),
            "alignment fields must not be promoted to a fixed LabelDisplay: {decoded}"
        );
    }

    #[test]
    fn cdx_right_aligned_hyphenated_label_token_stays_whole() {
        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 80 40" BondLength="14.4" LabelFont="3" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 80 40">
    <fragment id="2" BoundingBox="0 0 80 40">
      <n id="3" p="30 20" NodeType="Nickname">
        <t id="4" p="30 24" BoundingBox="0 10 30 26" LabelJustification="Right" Justification="Right" LabelAlignment="Right" UTF8Text="2-Np">
          <s font="3" size="10" face="0" color="0">2-Np</s>
        </t>
      </n>
      <n id="5" p="48 20"/>
      <b id="6" B="3" E="5"/>
    </fragment>
  </page>
</CDXML>"#;
        let source = cdxml_to_cdx(cdxml).expect("source CDX should encode");
        let imported =
            parse_cdx_document(&source, Some("right-aligned 2-Np")).expect("source CDX import");
        let imported_label = imported
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .and_then(|fragment| fragment.nodes.iter().find_map(|node| node.label.as_ref()))
            .expect("imported 2-Np label");
        assert_eq!(imported_label.source_text.as_deref(), Some("2-Np"));
        assert_eq!(imported_label.text, "2-Np");

        let first = document_to_cdx(&imported).expect("first CDX export");
        let reopened = parse_cdx_document(&first, Some("right-aligned 2-Np")).expect("reopen CDX");
        let reopened_label = reopened
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .and_then(|fragment| fragment.nodes.iter().find_map(|node| node.label.as_ref()))
            .expect("reopened 2-Np label");
        assert_eq!(reopened_label.source_text.as_deref(), Some("2-Np"));
        assert_eq!(reopened_label.text, "2-Np");

        let second = document_to_cdx(&reopened).expect("second CDX export");
        assert_eq!(second, first, "right-aligned 2-Np CDX must stabilize");
    }

    #[test]
    fn cdx_inferred_centered_metal_label_keeps_vertical_anchor_and_stable_fields() {
        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 80 40" BondLength="14.4" LabelFont="3" LabelSize="10" MarginWidth="1.6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 80 40">
    <fragment id="2" BoundingBox="0 0 80 40">
      <n id="3" p="30 20" Element="46" NumHydrogens="0">
        <t id="4" p="30 23.9" BoundingBox="23.9 14.9 36.1 26.4" LabelJustification="Center" Justification="Center" LabelAlignment="Center" UTF8Text="Pd">
          <s font="3" size="10" face="96" color="0">Pd</s>
        </t>
      </n>
      <n id="5" p="44.4 20" Element="7" NumHydrogens="0">
        <t id="6" p="40.8 23.9" BoundingBox="40.8 15.7 48 24.6" LabelJustification="Left" UTF8Text="N">
          <s font="3" size="10" face="96" color="0">N</s>
        </t>
      </n>
      <b id="7" B="3" E="5"/>
    </fragment>
  </page>
</CDXML>"#;
        let source = cdxml_to_cdx(cdxml).expect("source CDX should encode");
        let imported =
            parse_cdx_document(&source, Some("inferred centered Pd")).expect("source CDX import");
        let imported_fragment = imported
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .expect("imported fragment");
        let imported_metal = imported_fragment
            .nodes
            .iter()
            .find(|node| node.element == "Pd")
            .expect("imported Pd node");
        let imported_label = imported_metal.label.as_ref().expect("imported Pd label");
        assert_eq!(imported_label.align.as_deref(), Some("center"));
        assert_eq!(imported_label.anchor.as_deref(), Some("middle"));
        assert_eq!(
            imported_label.layout.as_deref(),
            Some("attached-group-center")
        );
        assert_eq!(
            imported_label.meta.pointer("/import/cdxml/labelDisplay"),
            Some(&serde_json::Value::Null)
        );
        assert!(
            (imported_label.position.expect("Pd baseline")[1]
                - imported_metal.position[1]
                - 3.9)
                .abs()
                < 0.01,
            "inferred centered Pd baseline must use the ChemDraw 0.39 font-size anchor: node={imported_metal:?}"
        );

        let first = document_to_cdx(&imported).expect("first CDX export");
        let decoded = cdx_to_cdxml(&first).expect("saved CDX should decode");
        for expected in [
            "LabelJustification=\"Center\"",
            "Justification=\"Center\"",
            "LabelAlignment=\"Center\"",
        ] {
            assert!(decoded.contains(expected), "missing {expected}: {decoded}");
        }
        assert!(
            !decoded.contains("LabelDisplay=\"Center\""),
            "inferred center fields must not be promoted to LabelDisplay: {decoded}"
        );

        let reopened =
            parse_cdx_document(&first, Some("inferred centered Pd")).expect("reopen CDX");
        let reopened_fragment = reopened
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .expect("reopened fragment");
        let reopened_metal = reopened_fragment
            .nodes
            .iter()
            .find(|node| node.element == "Pd")
            .expect("reopened Pd node");
        let reopened_label = reopened_metal.label.as_ref().expect("reopened Pd label");
        assert!(
            (reopened_label.position.expect("reopened Pd baseline")[1]
                - reopened_metal.position[1]
                - 3.9)
                .abs()
                < 0.01
        );
        let second = document_to_cdx(&reopened).expect("second CDX export");
        assert_eq!(second, first, "inferred centered Pd CDX must stabilize");
    }

    #[test]
    fn cdx_internal_fragment_attachment_round_trips_stably() {
        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 80 40" BondLength="14.4" LabelFont="3" LabelSize="10" MarginWidth="1.6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 80 40">
    <fragment id="2" BoundingBox="0 0 80 40">
      <n id="3" p="10 30" NodeType="Fragment">
        <t id="4" p="8 34" BoundingBox="8 22 62 36" LabelAlignment="Left" LabelJustification="Left" InterpretChemically="yes">
          <s font="3" size="10" face="0" color="0">(PhO)</s>
          <s font="3" size="10" face="96" color="0">2</s>
          <s font="3" size="10" face="0" color="0">POH</s>
        </t>
      </n>
      <n id="5" p="42 16" Element="8" NumHydrogens="0">
        <t id="6" p="38 20" BoundingBox="38 10 46 21"><s font="3" size="10" face="96" color="0">O</s></t>
      </n>
      <b id="7" B="3" BeginAttach="6" E="5" Order="2"/>
    </fragment>
  </page>
</CDXML>"#;
        let source = cdxml_to_cdx(cdxml).expect("source CDX should encode");
        let source_decoded = cdx_to_cdxml(&source).expect("source CDX should decode");
        assert!(
            source_decoded.contains("BeginAttach=\"6\""),
            "{source_decoded}"
        );
        let imported = parse_cdx_document(&source, Some("internal attachment"))
            .expect("source CDX should import");
        let imported_bond = imported
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .and_then(|fragment| fragment.bonds.first())
            .expect("imported bond");
        assert_eq!(
            imported_bond
                .meta
                .pointer("/endpointAttachments/begin/characterIndex")
                .and_then(serde_json::Value::as_u64),
            Some(6)
        );
        assert_eq!(
            imported_bond
                .meta
                .pointer("/endpointAttachments/begin/character"),
            Some(&serde_json::json!("P"))
        );

        let first = document_to_cdx(&imported).expect("first CDX export");
        let decoded = cdx_to_cdxml(&first).expect("saved CDX should decode");
        assert!(decoded.contains("BeginAttach=\"6\""), "{decoded}");
        let reopened = parse_cdx_document(&first, Some("internal attachment")).expect("reopen CDX");
        let second = document_to_cdx(&reopened).expect("second CDX export");
        assert_eq!(second, first, "internal attachment CDX must stabilize");
    }

    #[test]
    fn cdx_chain_angle_uses_readable_degrees_and_fixed_point_binary() {
        let encoded = encode_property("ChainAngle", "120").expect("chain angle should encode");
        assert_eq!(encoded.1, (120_i32 * 65536).to_le_bytes());
        let (_, decoded) =
            decode_property(encoded.0, &encoded.1, None).expect("chain angle should decode");
        assert_eq!(decoded, "120");
    }

    #[test]
    fn cdx_geometry_and_constraint_properties_use_the_official_tags_and_types() {
        for (name, value, tag, bytes) in [
            ("GeometricFeature", "3", 0x0B80, vec![3]),
            (
                "RelationValue",
                "12.5",
                0x0B81,
                12.5_f64.to_le_bytes().to_vec(),
            ),
            (
                "BasisObjects",
                "17 23",
                0x0B82,
                [17_u32.to_le_bytes(), 23_u32.to_le_bytes()].concat(),
            ),
            ("ConstraintType", "2", 0x0B83, vec![2]),
        ] {
            let encoded = encode_property(name, value).expect("property should encode");
            assert_eq!(encoded, (tag, bytes.clone()), "{name}={value}");
            assert_eq!(
                decode_property(tag, &bytes, None),
                Some((name, value.to_string())),
                "{name}={value}"
            );
        }

        assert_eq!(property_tag("ExternalConnectionID"), None);
        assert_eq!(property_tag("BracketedObjects"), None);
        assert_eq!(property_tag("RepeatPattern"), None);
    }

    #[test]
    fn cdx_arrow_properties_follow_chemdraws_binary_enums_and_units() {
        for (name, value, tag, bytes) in [
            ("ArrowheadHead", "None", 0x0A35, vec![0, 0]),
            ("ArrowheadHead", "Full", 0x0A35, vec![2, 0]),
            ("ArrowheadHead", "HalfLeft", 0x0A35, vec![3, 0]),
            ("ArrowheadHead", "HalfRight", 0x0A35, vec![4, 0]),
            ("ArrowheadTail", "Full", 0x0A36, vec![2, 0]),
            ("NoGo", "Cross", 0x0A3B, vec![2]),
            ("NoGo", "Hash", 0x0A3B, vec![3]),
            ("AngularSize", "90", 0x0A21, vec![0x84, 0x03]),
            ("CurveSpacing", "777", 0x0A38, vec![0x09, 0x03]),
        ] {
            let encoded = encode_property(name, value).expect("arrow property should encode");
            assert_eq!(encoded, (tag, bytes.clone()), "{name}={value}");
            assert_eq!(
                decode_property(tag, &bytes, None),
                Some((name, value.to_string())),
                "{name}={value}"
            );
        }

        assert_eq!(encode_property("Dipole", "yes"), Some((0x0A3A, Vec::new())));
        assert_eq!(
            decode_property(0x0A3A, &[], None),
            Some(("Dipole", "yes".to_string()))
        );
    }

    #[test]
    fn cdx_legacy_arrow_type_preserves_base_type_and_modifiers() {
        let encoded = encode_property("ArrowType", "FullHead NoGo Dipole")
            .expect("legacy arrow flags should encode");
        assert_eq!(encoded, (0x0A02, vec![0xC2, 0x00]));
        assert_eq!(
            decode_property(encoded.0, &encoded.1, None),
            Some(("ArrowType", "FullHead NoGo Dipole".to_string()))
        );
        assert_eq!(
            decode_property(0x0A02, &[2], None),
            Some(("ArrowType", "FullHead".to_string())),
            "ChemDraw 8 wrote this INT16 property as a single byte"
        );
    }

    #[test]
    fn cdx_text_layout_properties_follow_the_official_tags_and_special_values() {
        for (name, value, tag, bytes, decoded) in [
            ("LineHeight", "variable", 0x0702, vec![0, 0], "variable"),
            ("LineHeight", "auto", 0x0702, vec![1, 0], "auto"),
            ("WordWrapWidth", "144", 0x0703, vec![144, 0], "144"),
            ("LabelLineHeight", "12", 0x0706, vec![240, 0], "12"),
            ("CaptionLineHeight", "8.25", 0x0707, vec![165, 0], "8.25"),
            ("CaptionLineHeight", "auto", 0x0707, vec![1, 0], "auto"),
            ("BondSpacing", "12.5", 0x0804, vec![125, 0], "12.5"),
            ("BondSpacingAbs", "1.25", 0x0822, vec![0, 64, 1, 0], "1.25"),
            ("BracketType", "Square", 0x0A06, vec![3, 0], "Square"),
            ("BracketType", "Round", 0x0A06, vec![5, 0], "Round"),
            (
                "OvalType",
                "Circle Shaded",
                0x0A04,
                vec![3, 0],
                "Circle Shaded",
            ),
            (
                "RectangleType",
                "RoundEdge Shadow",
                0x0A03,
                vec![3, 0],
                "RoundEdge Shadow",
            ),
            ("LineType", "Bold Dashed", 0x0A01, vec![3, 0], "Dashed Bold"),
            ("LipSize", "60", 0x0A22, vec![60, 0], "60"),
            ("PositioningType", "absolute", 0x0D06, vec![3], "absolute"),
            (
                "NodeType",
                "GenericNickname",
                0x0400,
                vec![7, 0],
                "GenericNickname",
            ),
        ] {
            let encoded = encode_property(name, value).expect("property should encode");
            assert_eq!(encoded.0, tag, "{name}");
            assert_eq!(encoded.1, bytes, "{name}");
            let (decoded_name, decoded_value) =
                decode_property(tag, &encoded.1, None).expect("property should decode");
            assert_eq!(decoded_name, name);
            assert_eq!(decoded_value, decoded);
        }

        let best = encode_property("LabelAlignment", "Best").expect("Best should encode");
        assert_eq!(best, (0x0705, vec![6]));
        assert_eq!(
            decode_property(best.0, &best.1, None),
            Some(("LabelAlignment", "Best".to_string()))
        );
    }

    #[test]
    fn chemdraw_8_one_byte_bracket_type_uses_the_documented_enum() {
        assert_eq!(
            decode_property(0x0A06, &[3], None),
            Some(("BracketType", "Square".to_string()))
        );
    }

    #[test]
    fn cdx_superseded_by_reads_the_legacy_alias_and_writes_the_official_tag() {
        let encoded = encode_property("SupersededBy", "203").expect("property should encode");
        assert_eq!(encoded, (0x0012, 203_u32.to_le_bytes().to_vec()));

        for tag in [0x0012, 0x0013] {
            assert_eq!(
                decode_property(tag, &203_u32.to_le_bytes(), None),
                Some(("SupersededBy", "203".to_string()))
            );
        }
    }

    #[test]
    fn cdx_text_uses_utf8_when_valid_and_windows_1252_for_legacy_bytes() {
        assert_eq!(decode_text("11 °F".as_bytes(), None, None), "11 °F");
        assert_eq!(decode_text(b"11 \xB0F", None, None), "11 °F");
    }

    #[test]
    fn chemdraw_object_tags_override_the_shifted_static_registry() {
        assert_eq!(object_tag("geometry"), Some(0x801B));
        assert_eq!(object_tag("constraint"), Some(0x801C));
        assert_eq!(object_tag("tlcplate"), Some(0x801D));
        assert_eq!(object_tag("tlclane"), Some(0x801E));
        assert_eq!(object_tag("tlcspot"), Some(0x801F));
        assert_eq!(object_tag("chemicalproperty"), Some(0x8020));
        assert_eq!(object_tag("arrow"), Some(0x8021));
        assert_eq!(object_tag("border"), Some(0x802A));
        assert_eq!(legacy_chemsema_object_name(0x8021), Some("arrow"));

        let chemdraw = r#"<CDXML CreationProgram="ChemSema" ModificationProgram="ChemSema/1.0.0-beta.1;cdx-tags=chemdraw"><geometry id="2" /></CDXML>"#;
        let chemdraw_tree = CdxReader::new(&cdxml_to_cdx(chemdraw).unwrap())
            .read()
            .unwrap();
        assert_eq!(chemdraw_tree.children[0].name, "geometry");

        let mut shifted_beta = CDX_HEADER.to_vec();
        write_u16(&mut shifted_beta, 0x8000);
        write_u32(&mut shifted_beta, 1);
        write_property(
            &mut shifted_beta,
            0x0006,
            &encode_plain_cdx_string("ChemSema/1.0.0-beta.1;cdx-tags=official"),
        );
        write_u16(&mut shifted_beta, 0x8027);
        write_u32(&mut shifted_beta, 2);
        write_u16(&mut shifted_beta, 0);
        write_u16(&mut shifted_beta, 0);
        write_u16(&mut shifted_beta, 0);
        assert_eq!(
            CdxReader::new(&shifted_beta).read().unwrap().children[0].name,
            "arrow"
        );
    }

    #[test]
    fn public_complex_cdx_types_follow_the_official_binary_layouts() {
        for (kind, lexical) in [
            ("CDXDate", "2026 7 23 9 30 45 125"),
            ("CDXElementList", "9 17 35"),
            ("CDXElementList", "NOT 9 17 35"),
            ("CDXObjectIDArrayWithCounts", "1 2 3 4"),
            ("CDXCurvePoints", "1 2 3 4"),
            ("CDXCurvePoints3D", "1 2 3 4 5 6"),
            ("CDXRepresentsProperty", "6 0x0421"),
            ("CDXGenericList", "R X A"),
            ("CDXGenericList", "NOT R X A"),
        ] {
            let bytes = encode_official_lexical(kind, lexical).expect("complex type encodes");
            assert_eq!(
                decode_official_lexical(kind, &bytes).as_deref(),
                Some(lexical),
                "{kind}"
            );
        }
        assert_eq!(
            encode_official_lexical("CDXElementList", "NOT 9 17 35").unwrap(),
            vec![0xFD, 0xFF, 9, 0, 17, 0, 35, 0]
        );
        assert_eq!(
            encode_official_lexical("CDXObjectIDArrayWithCounts", "1 2 3 4").unwrap()[..2],
            [4, 0]
        );
        assert_eq!(
            encode_official_lexical("CDXGenericList", "R X A").unwrap(),
            vec![3, 0, 3, 0, 0, 0, b'R', 3, 0, 0, 0, b'X', 3, 0, 0, 0, b'A']
        );
    }

    #[test]
    fn repeated_cdx_properties_keep_order_values_and_distinct_json_keys() {
        let mut cdx = CDX_HEADER.to_vec();
        write_u16(&mut cdx, 0x8000);
        write_u32(&mut cdx, 1);
        write_property(&mut cdx, 0x0A86, &1.25_f64.to_le_bytes());
        write_property(&mut cdx, 0x0A86, &2.5_f64.to_le_bytes());
        write_u16(&mut cdx, 0);
        write_u16(&mut cdx, 0);

        let document = parse_cdx_document(&cdx, Some("repeated")).unwrap();
        let properties = &document.interchange["cdx"].root.properties;
        assert_eq!(properties["Spectrum_DataPoint"].value, "1.25");
        assert_eq!(properties["Spectrum_DataPoint#2"].value, "2.5");
        assert_eq!(
            properties["Spectrum_DataPoint#2"].name,
            "Spectrum_DataPoint"
        );

        let saved = document_to_cdx(&document).unwrap();
        let reopened = parse_cdx_document(&saved, Some("repeated")).unwrap();
        let properties = &reopened.interchange["cdx"].root.properties;
        assert_eq!(properties["Spectrum_DataPoint"].value, "1.25");
        assert_eq!(properties["Spectrum_DataPoint#2"].value, "2.5");
    }

    #[test]
    fn unknown_cdx_objects_keep_their_position_tag_and_payload() {
        let mut cdx = CDX_HEADER.to_vec();
        write_u16(&mut cdx, 0x8000);
        write_u32(&mut cdx, 1);
        for (tag, id) in [(0x8002, 2), (0xC001, 3), (0x8001, 4)] {
            write_u16(&mut cdx, tag);
            write_u32(&mut cdx, id);
            if tag == 0xC001 {
                write_property(&mut cdx, 0x4001, &[1, 2, 3, 4]);
            }
            write_u16(&mut cdx, 0);
        }
        write_u16(&mut cdx, 0);
        write_u16(&mut cdx, 0);

        let document = parse_cdx_document(&cdx, Some("unknown object")).unwrap();
        let saved = document_to_cdx(&document).unwrap();
        let reopened = parse_cdx_document(&saved, Some("unknown object")).unwrap();
        let children: Vec<_> = reopened.interchange["cdx"]
            .root
            .children
            .iter()
            .filter(|child| {
                child
                    .format_tag
                    .as_deref()
                    .and_then(parse_hex_u16)
                    .is_some_and(is_object_tag)
            })
            .collect();
        assert_eq!(
            children
                .iter()
                .map(|child| child.name.as_str())
                .collect::<Vec<_>>(),
            vec!["group", "unknown", "page"]
        );
        assert_eq!(children[1].format_tag.as_deref(), Some("0xC001"));
        assert_eq!(
            children[1].properties["tag_4001"].raw_base64.as_deref(),
            Some("AQIDBA==")
        );
    }

    #[test]
    fn cdx_official_properties_and_objects_are_editable_and_lossless_in_ccjs() {
        let mut cdx = CDX_HEADER.to_vec();
        write_u16(&mut cdx, 0x8000);
        write_u32(&mut cdx, 1);
        write_property(&mut cdx, 0x000B, b"CAS-1");
        write_u16(&mut cdx, 0x800C);
        write_u32(&mut cdx, 2);
        write_property(&mut cdx, 0x000C, b"CAS");
        write_u16(&mut cdx, 0);
        write_u16(&mut cdx, 0);
        write_u16(&mut cdx, 0);

        let mut document = parse_cdx_document(&cdx, Some("registry")).expect("CDX parses");
        let source = document.interchange.get("cdx").expect("CDX tree");
        assert_eq!(source.root.properties["RegistryNumber"].value, "CAS-1");
        assert_eq!(source.root.children[0].name, "regnum");
        assert_eq!(
            source.root.children[0].properties["RegistryAuthority"].value,
            "CAS"
        );
        document
            .set_interchange_property("cdx", &[], "RegistryNumber", "CAS-2")
            .expect("public interchange edit API updates a CDX field");

        let saved = document_to_cdx(&document).expect("edited CDX saves");
        let reopened = CdxReader::new(&saved).read().expect("saved CDX reopens");
        assert_eq!(
            reopened.attrs.get("RegistryNumber").map(String::as_str),
            Some("CAS-2")
        );
        assert!(reopened.children.iter().any(|child| child.name == "regnum"));
    }
}
