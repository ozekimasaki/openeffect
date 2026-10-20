use crate::chunk::is_group;
use crate::fields::{nums, ppsn};
use crate::xml::{self, Element, Node};
use crate::{Chunk, Document, Error, FourCc};

/// `.aepx` を読む。文書要素は `AfterEffectsProject`。
pub fn read_aepx(text: &str) -> Result<Document, Error> {
    let root = xml::parse(text)?;
    if root.name != "AfterEffectsProject" {
        return Err(Error::Xml(
            "文書要素が AfterEffectsProject ではありません".into(),
        ));
    }
    let mut chunks = Vec::new();
    for node in &root.nodes {
        if let Node::Element(element) = node {
            chunks.push(chunk_from_element(element)?);
        }
    }
    Ok(Document { chunks })
}

/// `.aepx` を書く。子の順序は `Egg!` 直下のチャンク順。
pub fn write_aepx(document: &Document) -> Result<String, Error> {
    let mut out =
        String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<AfterEffectsProject>\n");
    for chunk in &document.chunks {
        write_chunk(chunk, &mut out, 1)?;
    }
    out.push_str("</AfterEffectsProject>\n");
    Ok(out)
}

fn chunk_from_element(element: &Element) -> Result<Chunk, Error> {
    if element.name == "string" {
        return Ok(Chunk::data(
            FourCc::new(b"Utf8"),
            direct_text(element).into_bytes(),
        ));
    }
    if element.name == "ProjectXMPMetadata" {
        let data = if let Some(hex) = attribute(element, "bdata") {
            decode_hex(hex)?
        } else {
            direct_text(element).into_bytes()
        };
        return Ok(Chunk::data(FourCc::new(b"XMPM"), data));
    }
    if element.name == "numS"
        && child_elements(element).is_empty()
        && attribute(element, "bdata").is_none()
    {
        let value = direct_text(element)
            .trim()
            .parse::<u32>()
            .map_err(|_| Error::BadNumber)?;
        return Ok(Chunk::data(FourCc::new(b"numS"), value.to_be_bytes()));
    }
    if element.name == "ppSn"
        && child_elements(element).is_empty()
        && attribute(element, "bdata").is_none()
    {
        let value = direct_text(element)
            .trim()
            .parse::<f64>()
            .map_err(|_| Error::BadNumber)?;
        return Ok(Chunk::data(FourCc::new(b"ppSn"), value.to_be_bytes()));
    }

    let children = child_elements(element);
    if let Some(id) = fourcc_name(&element.name) {
        if is_group(id) {
            return Ok(Chunk::group(id, map_children(&children)?));
        }
        if id.0 == *b"btdk" && children.is_empty() {
            let body = match attribute(element, "bdata") {
                Some(hex) => decode_hex(hex)?,
                None => Vec::new(),
            };
            return Ok(Chunk::opaque_list(id, body));
        }
        if children.is_empty() {
            if let Some(hex) = attribute(element, "bdata") {
                return Ok(Chunk::data(id, decode_hex(hex)?));
            }
            if text_is_blank(element) {
                return Ok(Chunk::list(id, Vec::new()));
            }
            return Err(Error::Xml(format!(
                "要素 {} のテキストの置き場所が未確定です",
                element.name
            )));
        }
        return Ok(Chunk::list(id, map_children(&children)?));
    }

    Ok(Chunk::Xml {
        name: element.name.clone(),
        attributes: element.attributes.clone(),
        text: direct_text(element),
        children: map_children(&children)?,
    })
}

fn map_children(children: &[&Element]) -> Result<Vec<Chunk>, Error> {
    children
        .iter()
        .map(|child| chunk_from_element(child))
        .collect()
}

fn write_chunk(chunk: &Chunk, out: &mut String, depth: usize) -> Result<(), Error> {
    match chunk {
        Chunk::Data { id, data, .. } => write_data(*id, data, out, depth),
        Chunk::List { kind, children, .. } => {
            let name = xml_name(*kind)?;
            write_parent(&name, children, out, depth)
        }
        Chunk::Group { id, children, .. } => {
            let name = xml_name(*id)?;
            write_parent(&name, children, out, depth)
        }
        Chunk::OpaqueList { kind, body, .. } => {
            let name = xml_name(*kind)?;
            write_bdata(&name, body, out, depth);
            Ok(())
        }
        Chunk::Xml {
            name,
            attributes,
            text,
            children,
        } => {
            write_xml(name, attributes, text, children, out, depth)?;
            Ok(())
        }
    }
}

fn write_data(id: FourCc, data: &[u8], out: &mut String, depth: usize) -> Result<(), Error> {
    if id.0 == *b"Utf8" && is_xml_text(data) {
        let text = std::str::from_utf8(data).expect("is_xml_text checks UTF-8");
        write_text_element("string", text, out, depth);
        return Ok(());
    }
    if id.0 == *b"numS" {
        if let Some(value) = nums::value(data) {
            write_text_element("numS", &value.to_string(), out, depth);
            return Ok(());
        }
    }
    if id.0 == *b"ppSn" {
        if let Some(value) = ppsn::value(data) {
            let text = format_f64(value);
            if text
                .parse::<f64>()
                .ok()
                .is_some_and(|parsed| parsed.to_bits() == value.to_bits())
            {
                write_text_element("ppSn", &text, out, depth);
                return Ok(());
            }
        }
    }
    if id.0 == *b"XMPM" && is_xml_text(data) {
        let text = std::str::from_utf8(data).expect("is_xml_text checks UTF-8");
        write_text_element("ProjectXMPMetadata", text, out, depth);
        return Ok(());
    }
    let name = xml_name(id)?;
    write_bdata(&name, data, out, depth);
    Ok(())
}

fn write_parent(
    name: &str,
    children: &[Chunk],
    out: &mut String,
    depth: usize,
) -> Result<(), Error> {
    indent(out, depth);
    out.push('<');
    out.push_str(name);
    out.push_str(">\n");
    for child in children {
        write_chunk(child, out, depth + 1)?;
    }
    indent(out, depth);
    out.push_str("</");
    out.push_str(name);
    out.push_str(">\n");
    Ok(())
}

fn write_text_element(name: &str, text: &str, out: &mut String, depth: usize) {
    indent(out, depth);
    out.push('<');
    out.push_str(name);
    out.push('>');
    out.push_str(&escape_text(text));
    out.push_str("</");
    out.push_str(name);
    out.push_str(">\n");
}

fn write_bdata(name: &str, data: &[u8], out: &mut String, depth: usize) {
    indent(out, depth);
    out.push('<');
    out.push_str(name);
    out.push_str(" bdata=\"");
    out.push_str(&encode_hex(data));
    out.push_str("\"/>\n");
}

fn write_xml(
    name: &str,
    attributes: &[(String, String)],
    text: &str,
    children: &[Chunk],
    out: &mut String,
    depth: usize,
) -> Result<(), Error> {
    indent(out, depth);
    out.push('<');
    out.push_str(name);
    for (key, value) in attributes {
        out.push(' ');
        out.push_str(key);
        out.push_str("=\"");
        out.push_str(&escape_attr(value));
        out.push('"');
    }
    if children.is_empty() && text.is_empty() {
        out.push_str("/>\n");
        return Ok(());
    }
    out.push('>');
    if children.is_empty() {
        out.push_str(&escape_text(text));
        out.push_str("</");
        out.push_str(name);
        out.push_str(">\n");
        return Ok(());
    }
    out.push('\n');
    for child in children {
        write_chunk(child, out, depth + 1)?;
    }
    if !text.is_empty() {
        indent(out, depth + 1);
        out.push_str(&escape_text(text));
        out.push('\n');
    }
    indent(out, depth);
    out.push_str("</");
    out.push_str(name);
    out.push_str(">\n");
    Ok(())
}

fn xml_name(id: FourCc) -> Result<String, Error> {
    if !is_xml_name(id) {
        return Err(Error::XmlName(id));
    }
    let name = std::str::from_utf8(&id.0).map_err(|_| Error::XmlName(id))?;
    Ok(name.to_string())
}

fn is_xml_name(id: FourCc) -> bool {
    let [first, second, third, fourth] = id.0;
    start_name(first) && continue_name(second) && continue_name(third) && continue_name(fourth)
}

fn start_name(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn continue_name(byte: u8) -> bool {
    start_name(byte) || byte.is_ascii_digit() || byte == b'-' || byte == b'.'
}

fn is_xml_text(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    text.chars().all(|character| {
        character == '\t' || character == '\n' || character == '\r' || !character.is_control()
    })
}

fn format_f64(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    if value == 0.0 && value.is_sign_negative() {
        return "-0".to_string();
    }
    let text = value.to_string();
    if text
        .parse::<f64>()
        .ok()
        .is_some_and(|parsed| parsed.to_bits() == value.to_bits())
    {
        return text;
    }
    format!("{value:.16e}")
}

fn encode_hex(data: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(data.len() * 2);
    for byte in data {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub(crate) fn decode_hex(text: &str) -> Result<Vec<u8>, Error> {
    let digits: String = text
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect();
    if digits.len() % 2 != 0 {
        return Err(Error::BadHex);
    }
    let bytes = digits.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut index = 0;
    while index < bytes.len() {
        let high = hex_value(bytes[index])?;
        let low = hex_value(bytes[index + 1])?;
        out.push((high << 4) | low);
        index += 2;
    }
    Ok(out)
}

fn hex_value(byte: u8) -> Result<u8, Error> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(Error::BadHex),
    }
}

fn child_elements(element: &Element) -> Vec<&Element> {
    element
        .nodes
        .iter()
        .filter_map(|node| match node {
            Node::Element(child) => Some(child),
            Node::Text(_) => None,
        })
        .collect()
}

fn direct_text(element: &Element) -> String {
    let mut text = String::new();
    for node in &element.nodes {
        if let Node::Text(part) = node {
            text.push_str(part);
        }
    }
    text
}

fn text_is_blank(element: &Element) -> bool {
    direct_text(element).chars().all(char::is_whitespace)
}

fn attribute<'a>(element: &'a Element, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

fn fourcc_name(name: &str) -> Option<FourCc> {
    let bytes = name.as_bytes();
    if bytes.len() != 4 {
        return None;
    }
    Some(FourCc([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn escape_text(text: &str) -> String {
    let mut out = String::new();
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(character),
        }
    }
    out
}

fn escape_attr(text: &str) -> String {
    let mut out = String::new();
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(character),
        }
    }
    out
}
