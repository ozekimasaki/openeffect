use crate::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Element {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub nodes: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Node {
    Element(Element),
    Text(String),
}

struct Parser<'a> {
    bytes: &'a [u8],
    index: usize,
}

pub(crate) fn parse(input: &str) -> Result<Element, Error> {
    let mut parser = Parser {
        bytes: input.as_bytes(),
        index: 0,
    };
    if parser.bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        parser.index = 3;
    }
    loop {
        parser.skip_whitespace();
        if parser.starts_with(b"<?") {
            parser.skip_until(b"?>")?;
            continue;
        }
        if parser.starts_with(b"<!--") {
            parser.skip_until(b"-->")?;
            continue;
        }
        break;
    }
    let element = parser.parse_element()?;
    loop {
        parser.skip_whitespace();
        if parser.index >= parser.bytes.len() {
            break;
        }
        if parser.starts_with(b"<!--") {
            parser.skip_until(b"-->")?;
            continue;
        }
        return Err(Error::Xml("文書要素の後ろに余分なデータがあります".into()));
    }
    Ok(element)
}

impl Parser<'_> {
    fn parse_element(&mut self) -> Result<Element, Error> {
        self.expect(b"<")?;
        if self.starts_with(b"/") || self.starts_with(b"!") || self.starts_with(b"?") {
            return Err(Error::Xml("要素の開始ではありません".into()));
        }
        let name = self.parse_name()?;
        let mut attributes = Vec::new();
        let nodes = loop {
            self.skip_whitespace();
            if self.starts_with(b"/>") {
                self.index += 2;
                break Vec::new();
            }
            if self.starts_with(b">") {
                self.index += 1;
                break self.parse_content(&name)?;
            }
            let key = self.parse_name()?;
            self.skip_whitespace();
            self.expect(b"=")?;
            self.skip_whitespace();
            let value = self.parse_attribute()?;
            attributes.push((key, value));
        };
        Ok(Element {
            name,
            attributes,
            nodes,
        })
    }

    fn parse_content(&mut self, name: &str) -> Result<Vec<Node>, Error> {
        let mut nodes = Vec::new();
        loop {
            if self.index >= self.bytes.len() {
                return Err(Error::Xml(format!("要素 {name} が閉じていません")));
            }
            if self.starts_with(b"</") {
                self.index += 2;
                let end = self.parse_name()?;
                if end != name {
                    return Err(Error::Xml(format!(
                        "終了タグ {end} が {name} と一致しません"
                    )));
                }
                self.skip_whitespace();
                self.expect(b">")?;
                return Ok(nodes);
            }
            if self.starts_with(b"<!--") {
                self.skip_until(b"-->")?;
                continue;
            }
            if self.starts_with(b"<?") {
                self.skip_until(b"?>")?;
                continue;
            }
            if self.starts_with(b"<! ") || self.starts_with(b"<!D") || self.starts_with(b"<![") {
                return Err(Error::Xml("DTD と CDATA は読みません".into()));
            }
            if self.starts_with(b"<") {
                nodes.push(Node::Element(self.parse_element()?));
                continue;
            }
            let text = self.parse_text()?;
            if !text.is_empty() {
                nodes.push(Node::Text(text));
            }
        }
    }

    fn parse_text(&mut self) -> Result<String, Error> {
        let start = self.index;
        while self.index < self.bytes.len() && self.bytes[self.index] != b'<' {
            self.index += 1;
        }
        decode_entities(&self.bytes[start..self.index])
    }

    fn parse_attribute(&mut self) -> Result<String, Error> {
        let quote = self
            .bump()
            .ok_or_else(|| Error::Xml("属性値がありません".into()))?;
        if quote != b'"' && quote != b'\'' {
            return Err(Error::Xml("属性値の引用符がありません".into()));
        }
        let start = self.index;
        while self.index < self.bytes.len() && self.bytes[self.index] != quote {
            self.index += 1;
        }
        if self.index >= self.bytes.len() {
            return Err(Error::Xml("属性値が閉じていません".into()));
        }
        let raw = &self.bytes[start..self.index];
        self.index += 1;
        decode_entities(raw)
    }

    fn parse_name(&mut self) -> Result<String, Error> {
        let start = self.index;
        while self.index < self.bytes.len() {
            let byte = self.bytes[self.index];
            if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':') {
                self.index += 1;
            } else {
                break;
            }
        }
        if start == self.index {
            return Err(Error::Xml("要素名がありません".into()));
        }
        let name = std::str::from_utf8(&self.bytes[start..self.index])
            .map_err(|_| Error::Xml("要素名が UTF-8 ではありません".into()))?;
        Ok(name.to_string())
    }

    fn skip_whitespace(&mut self) {
        while self.index < self.bytes.len() && self.bytes[self.index].is_ascii_whitespace() {
            self.index += 1;
        }
    }

    fn skip_until(&mut self, marker: &[u8]) -> Result<(), Error> {
        let rest = &self.bytes[self.index..];
        let Some(found) = rest
            .windows(marker.len())
            .position(|window| window == marker)
        else {
            return Err(Error::Xml("閉じられていない制御です".into()));
        };
        self.index += found + marker.len();
        Ok(())
    }

    fn expect(&mut self, marker: &[u8]) -> Result<(), Error> {
        if self.starts_with(marker) {
            self.index += marker.len();
            Ok(())
        } else {
            Err(Error::Xml("予期しない記号です".into()))
        }
    }

    fn starts_with(&self, marker: &[u8]) -> bool {
        self.bytes[self.index..].starts_with(marker)
    }

    fn bump(&mut self) -> Option<u8> {
        let byte = *self.bytes.get(self.index)?;
        self.index += 1;
        Some(byte)
    }
}

fn decode_entities(raw: &[u8]) -> Result<String, Error> {
    let mut out = Vec::new();
    let mut index = 0;
    while index < raw.len() {
        if raw[index] != b'&' {
            out.push(raw[index]);
            index += 1;
            continue;
        }
        let Some(end) = raw[index..].iter().position(|byte| *byte == b';') else {
            return Err(Error::Xml("文字参照が閉じていません".into()));
        };
        let token = &raw[index + 1..index + end];
        let character = entity_char(token)?;
        let mut encoded = [0; 4];
        out.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
        index += end + 1;
    }
    String::from_utf8(out).map_err(|_| Error::Xml("テキストが UTF-8 ではありません".into()))
}

fn entity_char(token: &[u8]) -> Result<char, Error> {
    let code = match token {
        b"lt" => '<' as u32,
        b"gt" => '>' as u32,
        b"amp" => '&' as u32,
        b"quot" => '"' as u32,
        b"apos" => '\'' as u32,
        _ if token.first() == Some(&b'#') => numeric_entity(token)?,
        _ => return Err(Error::Xml("未対応の文字参照です".into())),
    };
    let Some(character) = char::from_u32(code) else {
        return Err(Error::Xml("数値文字参照が不正です".into()));
    };
    if character == '\0' || (character.is_control() && !matches!(character, '\t' | '\n' | '\r')) {
        return Err(Error::Xml("XML に書けない文字です".into()));
    }
    Ok(character)
}

fn numeric_entity(token: &[u8]) -> Result<u32, Error> {
    let digits = if token.get(1) == Some(&b'x') || token.get(1) == Some(&b'X') {
        let text = std::str::from_utf8(&token[2..])
            .map_err(|_| Error::Xml("数値文字参照が不正です".into()))?;
        u32::from_str_radix(text, 16)
    } else {
        let text = std::str::from_utf8(&token[1..])
            .map_err(|_| Error::Xml("数値文字参照が不正です".into()))?;
        text.parse::<u32>()
    };
    digits.map_err(|_| Error::Xml("数値文字参照が不正です".into()))
}
