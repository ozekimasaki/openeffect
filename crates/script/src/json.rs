//! テンプレートと MOGRT が使う、オブジェクトと配列だけの JSON。

use crate::error::{ScriptError, ScriptResult};

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    pub fn object(pairs: Vec<(&str, Json)>) -> Self {
        Self::Object(
            pairs
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        )
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Self::Object(pairs) => pairs.iter().find(|(name, _)| name == key).map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn write(&self) -> String {
        let mut out = String::new();
        write_json(&mut out, self);
        out
    }
}

fn write_json(out: &mut String, value: &Json) {
    match value {
        Json::Null => out.push_str("null"),
        Json::Bool(true) => out.push_str("true"),
        Json::Bool(false) => out.push_str("false"),
        Json::Number(number) => {
            if number.fract() == 0.0 && number.abs() < 1e15 {
                out.push_str(&format!("{}", *number as i64));
            } else {
                out.push_str(&format!("{number}"));
            }
        }
        Json::String(text) => {
            out.push('"');
            for ch in text.chars() {
                match ch {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    _ => out.push(ch),
                }
            }
            out.push('"');
        }
        Json::Array(values) => {
            out.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_json(out, value);
            }
            out.push(']');
        }
        Json::Object(pairs) => {
            out.push('{');
            for (index, (key, value)) in pairs.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_json(out, &Json::String(key.clone()));
                out.push(':');
                write_json(out, value);
            }
            out.push('}');
        }
    }
}

pub fn parse_json(input: &str) -> ScriptResult<Json> {
    let mut parser = Parser {
        bytes: input.as_bytes(),
        index: 0,
    };
    let value = parser.parse_value()?;
    parser.skip();
    if parser.index != parser.bytes.len() {
        return Err(ScriptError::invalid("JSON の末尾に余りがある"));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> Parser<'a> {
    fn skip(&mut self) {
        while matches!(self.bytes.get(self.index), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.index += 1;
        }
    }

    fn parse_value(&mut self) -> ScriptResult<Json> {
        self.skip();
        match self.bytes.get(self.index).copied() {
            Some(b'n') => self.consume_ident("null").map(|()| Json::Null),
            Some(b't') => self.consume_ident("true").map(|()| Json::Bool(true)),
            Some(b'f') => self.consume_ident("false").map(|()| Json::Bool(false)),
            Some(b'"') => self.parse_string().map(Json::String),
            Some(b'[') => self.parse_array(),
            Some(b'{') => self.parse_object(),
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            _ => Err(ScriptError::invalid("JSON の値が読めない")),
        }
    }

    fn consume_ident(&mut self, ident: &str) -> ScriptResult<()> {
        let end = self.index + ident.len();
        if self.bytes.get(self.index..end) == Some(ident.as_bytes()) {
            self.index = end;
            Ok(())
        } else {
            Err(ScriptError::invalid("JSON のリテラルが違う"))
        }
    }

    fn parse_string(&mut self) -> ScriptResult<String> {
        self.index += 1;
        let mut raw = Vec::new();
        while let Some(byte) = self.bytes.get(self.index).copied() {
            self.index += 1;
            match byte {
                b'"' => {
                    let text = String::from_utf8(raw)
                        .map_err(|_| ScriptError::invalid("JSON の文字列が UTF-8 ではない"))?;
                    return Ok(text);
                }
                b'\\' => {
                    let escaped = self
                        .bytes
                        .get(self.index)
                        .copied()
                        .ok_or_else(|| ScriptError::invalid("JSON の文字列が途中で終わった"))?;
                    self.index += 1;
                    match escaped {
                        b'"' => raw.push(b'"'),
                        b'\\' => raw.push(b'\\'),
                        b'n' => raw.push(b'\n'),
                        b'r' => raw.push(b'\r'),
                        b't' => raw.push(b'\t'),
                        b'/' => raw.push(b'/'),
                        _ => return Err(ScriptError::invalid("JSON のエスケープが未対応")),
                    }
                }
                _ => raw.push(byte),
            }
        }
        Err(ScriptError::invalid("JSON の文字列が閉じない"))
    }

    fn parse_number(&mut self) -> ScriptResult<Json> {
        let start = self.index;
        if self.bytes.get(self.index) == Some(&b'-') {
            self.index += 1;
        }
        while matches!(self.bytes.get(self.index), Some(b'0'..=b'9')) {
            self.index += 1;
        }
        if self.bytes.get(self.index) == Some(&b'.') {
            self.index += 1;
            while matches!(self.bytes.get(self.index), Some(b'0'..=b'9')) {
                self.index += 1;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.index])
            .map_err(|_| ScriptError::invalid("JSON の数値が読めない"))?;
        let number = text
            .parse::<f64>()
            .map_err(|_| ScriptError::invalid("JSON の数値が読めない"))?;
        Ok(Json::Number(number))
    }

    fn parse_array(&mut self) -> ScriptResult<Json> {
        self.index += 1;
        let mut values = Vec::new();
        loop {
            self.skip();
            if self.bytes.get(self.index) == Some(&b']') {
                self.index += 1;
                return Ok(Json::Array(values));
            }
            if !values.is_empty() {
                if self.bytes.get(self.index) != Some(&b',') {
                    return Err(ScriptError::invalid("JSON 配列の区切りが無い"));
                }
                self.index += 1;
            }
            values.push(self.parse_value()?);
        }
    }

    fn parse_object(&mut self) -> ScriptResult<Json> {
        self.index += 1;
        let mut pairs = Vec::new();
        loop {
            self.skip();
            if self.bytes.get(self.index) == Some(&b'}') {
                self.index += 1;
                return Ok(Json::Object(pairs));
            }
            if !pairs.is_empty() {
                if self.bytes.get(self.index) != Some(&b',') {
                    return Err(ScriptError::invalid("JSON オブジェクトの区切りが無い"));
                }
                self.index += 1;
                self.skip();
            }
            if self.bytes.get(self.index) != Some(&b'"') {
                return Err(ScriptError::invalid("JSON のキーが文字列ではない"));
            }
            let key = self.parse_string()?;
            self.skip();
            if self.bytes.get(self.index) != Some(&b':') {
                return Err(ScriptError::invalid("JSON のコロンが無い"));
            }
            self.index += 1;
            let value = self.parse_value()?;
            pairs.push((key, value));
        }
    }
}
