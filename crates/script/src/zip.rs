//! ZIP の中の無圧縮ファイルを読む。MOGRT の JSON 列挙に使う。

use std::path::Path;

use crate::error::{ScriptError, ScriptResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZipEntry {
    pub name: String,
    pub bytes: Vec<u8>,
}

pub fn read_stored_zip(path: &Path) -> ScriptResult<Vec<ZipEntry>> {
    let bytes = std::fs::read(path).map_err(|error| ScriptError::invalid(error.to_string()))?;
    read_stored_bytes(&bytes)
}

pub fn read_stored_bytes(bytes: &[u8]) -> ScriptResult<Vec<ZipEntry>> {
    let mut entries = Vec::new();
    let mut index = 0usize;
    while index + 30 <= bytes.len() {
        if &bytes[index..index + 4] != b"PK\x03\x04" {
            break;
        }
        let method = u16::from_le_bytes([bytes[index + 8], bytes[index + 9]]);
        let comp_size = u32::from_le_bytes(bytes[index + 18..index + 22].try_into().unwrap()) as usize;
        let name_len = u16::from_le_bytes([bytes[index + 26], bytes[index + 27]]) as usize;
        let extra_len = u16::from_le_bytes([bytes[index + 28], bytes[index + 29]]) as usize;
        let name_start = index + 30;
        let name_end = name_start + name_len;
        let data_start = name_end + extra_len;
        let data_end = data_start + comp_size;
        if data_end > bytes.len() || name_end > bytes.len() {
            return Err(ScriptError::invalid("ZIP のローカルヘッダが短い"));
        }
        if method != 0 {
            return Err(ScriptError::unsupported("圧縮された ZIP メンバ"));
        }
        let name = String::from_utf8_lossy(&bytes[name_start..name_end]).into_owned();
        let data = bytes[data_start..data_end].to_vec();
        if !name.ends_with('/') {
            entries.push(ZipEntry { name, bytes: data });
        }
        index = data_end;
    }
    Ok(entries)
}
