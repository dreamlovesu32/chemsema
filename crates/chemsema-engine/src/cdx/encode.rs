use super::*;

pub(super) fn encode_plain_cdx_string(value: &str) -> Vec<u8> {
    let mut out = Vec::new();
    write_u16(&mut out, 0);
    out.extend_from_slice(value.replace('\n', "\r").as_bytes());
    out
}

pub(super) fn encode_cdx_string(node: &crate::cdxml::xml::XmlNode) -> Vec<u8> {
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

pub(super) fn encode_bond_order(value: &str) -> Option<Vec<u8>> {
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

pub(super) fn encode_coordinate(value: &str) -> Option<Vec<u8>> {
    let coord = (value.parse::<f64>().ok()? * CDX_COORD_FACTOR).round() as i32;
    Some(coord.to_le_bytes().to_vec())
}

pub(super) fn encode_point2d(value: &str) -> Option<Vec<u8>> {
    let values = parse_numbers(value)?;
    if values.len() < 2 {
        return None;
    }
    let mut out = Vec::new();
    out.extend_from_slice(&((values[1] * CDX_COORD_FACTOR).round() as i32).to_le_bytes());
    out.extend_from_slice(&((values[0] * CDX_COORD_FACTOR).round() as i32).to_le_bytes());
    Some(out)
}

pub(super) fn encode_point3d(value: &str) -> Option<Vec<u8>> {
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

pub(super) fn encode_rectangle(value: &str) -> Option<Vec<u8>> {
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

pub(super) fn encode_u32_array(value: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    for part in value.split_whitespace() {
        write_u32(&mut out, part.parse().ok()?);
    }
    Some(out)
}

pub(super) fn encode_u32_counted_array(value: &str) -> Option<Vec<u8>> {
    let body = encode_u32_array(value)?;
    let count = u16::try_from(body.len() / 4).ok()?;
    let mut out = count.to_le_bytes().to_vec();
    out.extend_from_slice(&body);
    Some(out)
}

pub(super) fn encode_element_list(value: &str) -> Option<Vec<u8>> {
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

pub(super) fn encode_curve_points(value: &str, dimensions: usize) -> Option<Vec<u8>> {
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

pub(super) fn encode_cdx_date(value: &str) -> Option<Vec<u8>> {
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

pub(super) fn encode_represents_property(value: &str) -> Option<Vec<u8>> {
    let mut parts = value.split_whitespace();
    let object_id = parts.next()?.parse::<u32>().ok()?;
    let property_tag = parse_hex_u16(parts.next()?)?;
    let mut out = object_id.to_le_bytes().to_vec();
    out.extend_from_slice(&property_tag.to_le_bytes());
    Some(out)
}

pub(super) fn encode_generic_list(value: &str) -> Option<Vec<u8>> {
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

pub(super) fn encode_i16_counted_list(value: &str) -> Option<Vec<u8>> {
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

pub(super) fn encode_color_table(node: &crate::cdxml::xml::XmlNode) -> Vec<u8> {
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

pub(super) fn encode_font_table(node: &crate::cdxml::xml::XmlNode) -> Vec<u8> {
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
