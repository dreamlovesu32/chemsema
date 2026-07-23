use super::*;

pub(super) fn fmt_num(value: f64) -> String {
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

pub(super) fn fmt_cdxml_bool(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

pub(super) fn fmt_margins(value: [f64; 4]) -> String {
    format!(
        "{} {} {} {}",
        fmt_num(value[0]),
        fmt_num(value[1]),
        fmt_num(value[2]),
        fmt_num(value[3])
    )
}

pub(super) fn fmt_point(point: Point) -> String {
    format!("{} {}", fmt_num(point.x), fmt_num(point.y))
}

pub(super) fn fmt_point3(point: Point) -> String {
    format!("{} {} 0", fmt_num(point.x), fmt_num(point.y))
}

pub(super) fn fmt_bbox(bbox: [f64; 4]) -> String {
    format!(
        "{} {} {} {}",
        fmt_num(bbox[0]),
        fmt_num(bbox[1]),
        fmt_num(bbox[2]),
        fmt_num(bbox[3])
    )
}

pub(super) fn encode_hex_bytes(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut out, "{byte:02X}").expect("writing to a string cannot fail");
    }
    out
}

pub(super) fn write_open_tag(
    out: &mut String,
    indent: usize,
    name: &str,
    attrs: Vec<(&str, String)>,
) {
    write_indent(out, indent);
    write!(out, "<{name}").expect("writing tag should not fail");
    for (key, value) in attrs {
        write!(out, " {key}=\"{}\"", xml_escape_attr(&value))
            .expect("writing tag attr should not fail");
    }
    out.push_str(">\n");
}

pub(super) fn write_empty_tag(
    out: &mut String,
    indent: usize,
    name: &str,
    attrs: Vec<(&str, String)>,
) {
    write_indent(out, indent);
    write!(out, "<{name}").expect("writing tag should not fail");
    for (key, value) in attrs {
        write!(out, " {key}=\"{}\"", xml_escape_attr(&value))
            .expect("writing tag attr should not fail");
    }
    out.push_str("/>\n");
}

pub(super) fn write_text_tag(
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

pub(super) fn write_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push(' ');
    }
}

pub(super) fn xml_escape_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub(super) fn xml_escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
