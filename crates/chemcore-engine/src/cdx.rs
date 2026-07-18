use crate::{document_to_cdxml, parse_cdxml_document, ChemcoreDocument};
use serde_json::json;
use std::collections::BTreeMap;
use std::fmt::Write;

const CDX_HEADER: &[u8; 22] = b"VjCD0100\x04\x03\x02\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
const CDX_COORD_FACTOR: f64 = 65_536.0;

pub fn parse_cdx_document(bytes: &[u8], title: Option<&str>) -> Result<ChemcoreDocument, String> {
    let cdxml = cdx_to_cdxml(bytes)?;
    let mut document = parse_cdxml_document(&cdxml, title)?;
    document.document.meta["sourceFormat"] = json!("cdx");
    if let Some(import) = document.document.meta.get_mut("import") {
        import["cdx"] = json!({ "nativeImport": true });
    }
    Ok(document)
}

pub fn document_to_cdx(document: &ChemcoreDocument) -> Result<Vec<u8>, String> {
    let cdxml = document_to_cdxml(document);
    cdxml_to_cdx(&cdxml)
}

pub fn cdx_to_cdxml(bytes: &[u8]) -> Result<String, String> {
    let tree = CdxReader::new(bytes).read()?;
    Ok(CdxmlWriter::new().write(&tree))
}

pub fn cdxml_to_cdx(cdxml: &str) -> Result<Vec<u8>, String> {
    let root = crate::cdxml::parse_xml_tree(cdxml)?;
    CdxWriter::new().write(&root)
}

#[derive(Debug, Clone)]
struct CdxNode {
    name: &'static str,
    attrs: BTreeMap<String, String>,
    text_runs: Vec<CdxTextRun>,
    text: Option<String>,
    children: Vec<CdxNode>,
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
}

impl<'a> CdxReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            font_table: None,
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
        let name = object_name(tag).unwrap_or("unknown");
        let mut node = CdxNode {
            name,
            attrs: BTreeMap::new(),
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
        if tag == 0x080A || tag == 0x080B {
            if let Some((font, face, size, color)) = decode_font_style(data) {
                let prefix = if tag == 0x080A { "Label" } else { "Caption" };
                node.attrs.insert(format!("{prefix}Font"), font.to_string());
                node.attrs.insert(format!("{prefix}Face"), face.to_string());
                node.attrs.insert(format!("{prefix}Size"), fmt_num(size));
                node.attrs
                    .insert(format!("{prefix}Color"), color.to_string());
            }
            return Ok(());
        }
        if let Some((name, value)) = decode_property(tag, data, self.font_table.as_ref()) {
            node.attrs.insert(name.to_string(), value);
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
        let tag_name = node.name;
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
            writeln!(
                out,
                "<s font=\"{}\" size=\"{}\" face=\"{}\" color=\"{}\">{}</s>",
                run.font,
                fmt_num(run.size),
                run.face,
                run.color,
                escape_text(&slice)
            )
            .expect("write styled text");
        }
    }

    fn write_indent(&self, out: &mut String, indent: usize) {
        for _ in 0..indent {
            out.push(' ');
        }
    }
}

struct CdxWriter {
    next_id: u32,
}

impl CdxWriter {
    fn new() -> Self {
        Self { next_id: 5000 }
    }

    fn write(mut self, root: &crate::cdxml::xml::XmlNode) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        out.extend_from_slice(CDX_HEADER);
        self.write_object(root, &mut out)?;
        out.extend_from_slice(&[0, 0, 0, 0]);
        Ok(out)
    }

    fn write_object(
        &mut self,
        node: &crate::cdxml::xml::XmlNode,
        out: &mut Vec<u8>,
    ) -> Result<(), String> {
        let Some(tag) = object_tag(&node.name) else {
            return Ok(());
        };
        write_u16(out, tag);
        write_u32(out, self.xml_id(node));

        for (key, value) in &node.attrs {
            if key == "id" {
                continue;
            }
            if let Some((prop_tag, bytes)) = encode_property(key, value) {
                write_property(out, prop_tag, &bytes);
            }
        }

        if node.name == "CDXML" {
            if let Some(color_table) = node.direct_children("colortable").next() {
                write_property(out, 0x0300, &encode_color_table(color_table));
            }
            if let Some(font_table) = node.direct_children("fonttable").next() {
                write_property(out, 0x0100, &encode_font_table(font_table));
            }
        }

        if node.name == "t" {
            write_property(out, 0x0700, &encode_cdx_string(node));
        }

        for child in &node.children {
            if matches!(
                child.name.as_str(),
                "s" | "font" | "color" | "fonttable" | "colortable"
            ) {
                continue;
            }
            self.write_object(child, out)?;
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
                    name: "font",
                    attrs,
                    text_runs: Vec::new(),
                    text: None,
                    children: Vec::new(),
                }
            })
            .collect();
        CdxNode {
            name: "fonttable",
            attrs: BTreeMap::new(),
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
                    name: "color",
                    attrs,
                    text_runs: Vec::new(),
                    text: None,
                    children: Vec::new(),
                }
            })
            .collect();
        CdxNode {
            name: "colortable",
            attrs: BTreeMap::new(),
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
    let _charset = font_id
        .and_then(|id| font_table.and_then(|table| table.fonts.iter().find(|font| font.id == id)))
        .map(|font| font.charset)
        .unwrap_or(1252);
    String::from_utf8_lossy(data).replace('\r', "\n")
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
        PropertyKind::Point2D => decode_point2d(data)?,
        PropertyKind::Point3D => decode_point3d(data)?,
        PropertyKind::Rectangle => decode_rectangle(data)?,
        PropertyKind::Coordinate => decode_coordinate(data)?,
        PropertyKind::Int8 => read_i8(data)?.to_string(),
        PropertyKind::UInt8 => read_u8(data)?.to_string(),
        PropertyKind::Int16 => read_i16(data)?.to_string(),
        PropertyKind::UInt16 => read_u16(data)?.to_string(),
        PropertyKind::Int32 => read_i32(data)?.to_string(),
        PropertyKind::Fixed16_16 => fmt_num(read_i32(data)? as f64 / 65536.0),
        PropertyKind::UInt32 => read_u32(data)?.to_string(),
        PropertyKind::Float64 => read_f64(data)?.to_string(),
        PropertyKind::Boolean => bool_from_bytes(data),
        PropertyKind::BooleanImplied => "yes".to_string(),
        PropertyKind::BondOrder => decode_bond_order(data)?,
        PropertyKind::BondSpacing => (read_i16(data)? as f64 / 10.0).round().to_string(),
        PropertyKind::FontStyle => return None,
        PropertyKind::ObjectIdArray => decode_u32_array(data)?,
        PropertyKind::Int16ListWithCounts => decode_i16_counted_list(data)?,
        PropertyKind::Enum8(values) => enum_name(read_i8(data)? as i16, values).to_string(),
        PropertyKind::Enum(values) => enum_name(read_i16_lossy(data)?, values).to_string(),
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
    Point2D,
    Point3D,
    Rectangle,
    Coordinate,
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    Fixed16_16,
    UInt32,
    Float64,
    Boolean,
    BooleanImplied,
    BondOrder,
    BondSpacing,
    FontStyle,
    ObjectIdArray,
    Int16ListWithCounts,
    Enum8(&'static [(i16, &'static str)]),
    Enum(&'static [(i16, &'static str)]),
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
        0x0200 => ("p", PropertyKind::Point2D),
        0x0201 => ("xyz", PropertyKind::Point3D),
        0x0202 => ("extent", PropertyKind::Point2D),
        0x0204 => ("BoundingBox", PropertyKind::Rectangle),
        0x0205 => ("RotationAngle", PropertyKind::Int32),
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
        0x0421 => ("Charge", PropertyKind::Int8),
        0x042B => ("NumHydrogens", PropertyKind::UInt16),
        0x0437 => ("AS", PropertyKind::Enum(ATOM_STEREO)),
        0x0444 => ("HideImplicitHydrogens", PropertyKind::Boolean),
        0x0445 => ("ShowAtomEnhancedStereo", PropertyKind::Boolean),
        0x0504 => ("Weight", PropertyKind::Float64),
        0x0600 => ("Order", PropertyKind::BondOrder),
        0x0601 => ("Display", PropertyKind::Enum(BOND_DISPLAY)),
        0x0602 => ("Display2", PropertyKind::Enum(BOND_DISPLAY)),
        0x0603 => ("DoublePosition", PropertyKind::Enum(DOUBLE_POSITION)),
        0x0604 => ("B", PropertyKind::UInt32),
        0x0605 => ("E", PropertyKind::UInt32),
        0x0608 => ("BeginAttach", PropertyKind::UInt8),
        0x0609 => ("EndAttach", PropertyKind::UInt8),
        0x060A => ("BS", PropertyKind::Enum(BOND_STEREO)),
        0x0701 => ("Justification", PropertyKind::Enum8(JUSTIFICATION)),
        0x0704 => ("LineStarts", PropertyKind::Int16ListWithCounts),
        0x0705 => ("LabelAlignment", PropertyKind::Enum8(LABEL_ALIGNMENT)),
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
        0x0823 => ("LabelJustification", PropertyKind::Enum8(JUSTIFICATION)),
        0x0900 => ("WindowIsZoomed", PropertyKind::BooleanImplied),
        0x0901 => ("WindowPosition", PropertyKind::Point2D),
        0x0902 => ("WindowSize", PropertyKind::Point2D),
        0x0A00 => ("GraphicType", PropertyKind::Enum(GRAPHIC_TYPE)),
        0x0A01 => ("LineType", PropertyKind::Enum(LINE_TYPE)),
        0x0A02 => ("ArrowType", PropertyKind::Enum(ARROW_TYPE)),
        0x0A03 => ("RectangleType", PropertyKind::Int16),
        0x0A04 => ("OvalType", PropertyKind::Int16),
        0x0A05 => ("OrbitalType", PropertyKind::Enum(ORBITAL_TYPE)),
        0x0A06 => ("BracketType", PropertyKind::Int16),
        0x0A07 => ("SymbolType", PropertyKind::Int16),
        0x0A20 => ("HeadSize", PropertyKind::Int16),
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
        0x0A39 => ("Closed", PropertyKind::BooleanImplied),
        0x0A3A => ("Dipole", PropertyKind::Boolean),
        0x0A3B => ("NoGo", PropertyKind::Int8),
        0x0A3C => ("CornerRadius", PropertyKind::Int16),
        0x0A3E => ("ArrowSource", PropertyKind::UInt16),
        0x0A3F => ("ArrowTarget", PropertyKind::UInt16),
        0x0A70 => ("PNG", PropertyKind::String),
        0x0A71 => ("JPEG", PropertyKind::String),
        0x0AB1 => ("Tail", PropertyKind::Coordinate),
        0x0AF0 => ("TextFrame", PropertyKind::Rectangle),
        0x0B80 => ("ExternalConnectionID", PropertyKind::UInt32),
        0x0B81 => ("BracketedObjects", PropertyKind::ObjectIdArray),
        0x0B83 => ("RepeatPattern", PropertyKind::String),
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
        "Charge" => 0x0421,
        "NumHydrogens" => 0x042B,
        "AS" => 0x0437,
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
        "Justification" => 0x0701,
        "LineStarts" => 0x0704,
        "LabelAlignment" => 0x0705,
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
        "GraphicID" => 0x0A2B,
        "ArrowheadType" => 0x0A2F,
        "ArrowheadCenterSize" => 0x0A30,
        "ArrowheadWidth" => 0x0A31,
        "ArrowShaftSpacing" => 0x0A33,
        "ArrowEquilibriumRatio" => 0x0A34,
        "ArrowheadHead" => 0x0A35,
        "ArrowheadTail" => 0x0A36,
        "FillType" => 0x0A37,
        "Closed" => 0x0A39,
        "Dipole" => 0x0A3A,
        "NoGo" => 0x0A3B,
        "CornerRadius" => 0x0A3C,
        "ArrowSource" => 0x0A3E,
        "ArrowTarget" => 0x0A3F,
        "Tail" => 0x0AB1,
        "TextFrame" => 0x0AF0,
        "ExternalConnectionID" => 0x0B80,
        "BracketedObjects" => 0x0B81,
        "RepeatPattern" => 0x0B83,
        _ => return None,
    })
}

fn encode_property(name: &str, value: &str) -> Option<(u16, Vec<u8>)> {
    let tag = property_tag(name)?;
    let schema = property_schema(tag)?;
    let bytes = match schema.kind {
        PropertyKind::String => encode_plain_cdx_string(value),
        PropertyKind::Point2D => encode_point2d(value)?,
        PropertyKind::Point3D => encode_point3d(value)?,
        PropertyKind::Rectangle => encode_rectangle(value)?,
        PropertyKind::Coordinate => encode_coordinate(value)?,
        PropertyKind::Int8 => vec![value.parse::<i8>().ok()? as u8],
        PropertyKind::UInt8 => vec![value.parse::<u8>().ok()?],
        PropertyKind::Int16 => value.parse::<i16>().ok()?.to_le_bytes().to_vec(),
        PropertyKind::UInt16 => value.parse::<u16>().ok()?.to_le_bytes().to_vec(),
        PropertyKind::Int32 => value.parse::<i32>().ok()?.to_le_bytes().to_vec(),
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
        PropertyKind::FontStyle => return None,
        PropertyKind::ObjectIdArray => encode_u32_array(value)?,
        PropertyKind::Int16ListWithCounts => encode_i16_counted_list(value)?,
        PropertyKind::Enum8(values) => vec![enum_value(value, values)? as i8 as u8],
        PropertyKind::Enum(values) => enum_value(value, values)?.to_le_bytes().to_vec(),
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
        0x800D => "scheme",
        0x800E => "step",
        0x8011 => "objecttag",
        0x8016 => "table",
        0x8017 => "bracketedgroup",
        0x8018 => "bracketattachment",
        0x801B => "geometry",
        0x801C => "constraint",
        0x801D => "tlcplate",
        0x801E => "tlclane",
        0x801F => "tlcspot",
        0x8020 => "chemicalproperty",
        0x8021 => "arrow",
        0x8025 => "bioshape",
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
        "scheme" => 0x800D,
        "step" => 0x800E,
        "objecttag" => 0x8011,
        "table" => 0x8016,
        "bracketedgroup" => 0x8017,
        "bracketattachment" => 0x8018,
        "geometry" => 0x801B,
        "constraint" => 0x801C,
        "tlcplate" => 0x801D,
        "tlclane" => 0x801E,
        "tlcspot" => 0x801F,
        "chemicalproperty" => 0x8020,
        "arrow" => 0x8021,
        "bioshape" => 0x8025,
        "annotation" => 0x802B,
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
const BOND_STEREO: &[(i16, &str)] = &[(0, "U"), (1, "N"), (2, "E"), (3, "Z")];
const NODE_TYPE: &[(i16, &str)] = &[
    (0, "Unspecified"),
    (1, "Element"),
    (4, "Nickname"),
    (5, "Fragment"),
    (12, "ExternalConnectionPoint"),
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
const LINE_TYPE: &[(i16, &str)] = &[
    (0, "Solid"),
    (1, "Dashed"),
    (2, "Bold"),
    (3, "Wavy"),
    (4, "Bold Dashed"),
];
const ARROW_TYPE: &[(i16, &str)] = &[
    (0, "NoHead"),
    (1, "HalfHead"),
    (2, "FullHead"),
    (4, "Resonance"),
    (8, "Equilibrium"),
    (16, "Hollow"),
    (32, "RetroSynthetic"),
];
const ARROW_HEAD_TYPE: &[(i16, &str)] = &[
    (0, "Unspecified"),
    (1, "Solid"),
    (2, "Hollow"),
    (3, "Angle"),
];
const ARROW_HEAD_POSITION: &[(i16, &str)] =
    &[(0, "None"), (1, "Full"), (2, "HalfLeft"), (3, "HalfRight")];
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
    fn cdx_roundtrip_imports_basic_molecule() {
        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML CreationProgram="ChemCore" BoundingBox="0 0 120 80" LabelFont="3" LabelSize="10" LabelFace="96" CaptionFont="3" CaptionSize="10" LineWidth="1" BoldWidth="4" BondLength="18" BondSpacing="18" HashSpacing="2.5" MarginWidth="2">
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
            "CDX must stabilize after the first ChemCore save"
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
}
