use super::*;

pub(super) fn is_cdx_helper_name(name: &str) -> bool {
    matches!(
        name,
        "s" | "font" | "color" | "fonttable" | "colortable" | "represent"
    )
}

pub(super) fn parse_hex_u16(value: &str) -> Option<u16> {
    u16::from_str_radix(value.trim().trim_start_matches("0x"), 16).ok()
}

pub(super) fn ordered_interchange_properties(
    object: &InterchangeObject,
) -> Vec<&InterchangeProperty> {
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

pub(super) fn interchange_matches_xml(
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

pub(super) fn overlay_unmodeled_cdx_values(
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

pub(super) fn parse_cdx_string(data: &[u8], font_table: Option<&FontTable>) -> ParsedText {
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

pub(super) fn decode_text(
    data: &[u8],
    font_id: Option<u16>,
    font_table: Option<&FontTable>,
) -> String {
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

pub(super) fn parse_font_table(data: &[u8]) -> Option<FontTable> {
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

pub(super) fn parse_color_table(data: &[u8]) -> Option<ColorTable> {
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

pub(super) fn decode_property(
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
