use quick_xml::escape::unescape;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub(super) struct XmlNode {
    pub(super) name: String,
    pub(super) attrs: BTreeMap<String, String>,
    pub(super) text: String,
    pub(super) children: Vec<XmlNode>,
}

pub(super) fn parse_xml_tree(xml: &str) -> Result<XmlNode, String> {
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
                    let decoded = text.xml_content().map_err(|error| error.to_string())?;
                    let unescaped = unescape(&decoded).map_err(|error| error.to_string())?;
                    node.text.push_str(&unescaped);
                }
            }
            Event::GeneralRef(reference) => {
                if let Some(node) = stack.last_mut() {
                    let name = reference.decode().map_err(|error| error.to_string())?;
                    let escaped = format!("&{name};");
                    match unescape(&escaped) {
                        Ok(unescaped) => node.text.push_str(&unescaped),
                        Err(_) => node.text.push_str(&escaped),
                    }
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
    pub(super) fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.get(key).map(String::as_str)
    }

    pub(super) fn is(&self, name: &str) -> bool {
        self.name == name
    }

    pub(super) fn direct_children<'a>(
        &'a self,
        name: &'a str,
    ) -> impl Iterator<Item = &'a XmlNode> {
        self.children.iter().filter(move |child| child.is(name))
    }

    pub(super) fn full_text(&self) -> String {
        let mut out = self.text.clone();
        for child in &self.children {
            out.push_str(&child.full_text());
        }
        out
    }
}

pub(super) fn descendants(node: &XmlNode) -> Vec<&XmlNode> {
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
