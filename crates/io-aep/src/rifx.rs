use crate::chunk::{read_chunks, write_chunks};
use crate::{Chunk, Error, FourCc};

/// `Egg!` 直下のチャンク列。`.aep` と `.aet` は同じ並び。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    pub chunks: Vec<Chunk>,
}

/// `.aep` のバイト列を読む。フォームタイプが `Egg!` でないものは読まない。
pub fn read_aep(input: &[u8]) -> Result<Document, Error> {
    if input.len() < 12 {
        return Err(Error::Truncated);
    }
    if &input[0..4] != b"RIFX" {
        return Err(Error::BadMagic);
    }
    let declared = u32::from_be_bytes([input[4], input[5], input[6], input[7]]);
    let actual = (input.len() - 8) as u64;
    if u64::from(declared) != actual {
        return Err(Error::BadFileSize { declared, actual });
    }
    let form = FourCc([input[8], input[9], input[10], input[11]]);
    if form.0 != *b"Egg!" {
        return Err(Error::UnsupportedForm(form));
    }
    let chunks = read_chunks(&input[12..], 0)?;
    Ok(Document { chunks })
}

/// `.aep` を書く。先頭は `RIFX`、オフセット 8 は `Egg!`。
pub fn write_aep(document: &Document) -> Result<Vec<u8>, Error> {
    let mut body = Vec::new();
    write_chunks(&document.chunks, &mut body)?;
    let form_size = body.len().checked_add(4).ok_or(Error::TooLarge)?;
    if form_size > u32::MAX as usize {
        return Err(Error::TooLarge);
    }
    let mut out = Vec::with_capacity(8 + form_size);
    out.extend_from_slice(b"RIFX");
    out.extend_from_slice(&(form_size as u32).to_be_bytes());
    out.extend_from_slice(b"Egg!");
    out.extend_from_slice(&body);
    Ok(out)
}

/// `.aet` は中身が `.aep` と同じ RIFX。拡張子では分岐しない。
pub fn read_aet(input: &[u8]) -> Result<Document, Error> {
    read_aep(input)
}

/// `.aet` として書き戻す。バイト列は `.aep` と同じ。
pub fn write_aet(document: &Document) -> Result<Vec<u8>, Error> {
    write_aep(document)
}
