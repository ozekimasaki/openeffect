use crate::{Error, FourCc};

const MAX_DEPTH: usize = 512;

/// チャンク。未知の fourcc も、確定していない中身も、バイトのまま持つ。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Chunk {
    /// ペイロードを解釈しないデータチャンク。
    Data {
        id: FourCc,
        data: Vec<u8>,
        /// ペイロード長が奇数のときの 1 バイト。偶数のときは `None`。
        pad: Option<u8>,
    },
    /// `LIST`。`kind` はリストタイプ。
    List {
        kind: FourCc,
        children: Vec<Chunk>,
        pad: Option<u8>,
    },
    /// `tdsn` / `fnam` / `pdnm`。ヘッダは `LIST` にしない。
    Group {
        id: FourCc,
        children: Vec<Chunk>,
        pad: Option<u8>,
    },
    /// `LIST` だが、タイプのうしろを子チャンクに展開しない（`btdk`）。
    OpaqueList {
        kind: FourCc,
        body: Vec<u8>,
        pad: Option<u8>,
    },
    /// `.aepx` で fourcc にできない要素。名前は未確定なので、そのまま残す。
    Xml {
        name: String,
        attributes: Vec<(String, String)>,
        text: String,
        children: Vec<Chunk>,
    },
}

impl Chunk {
    pub fn data(id: FourCc, data: impl AsRef<[u8]>) -> Self {
        let data = data.as_ref().to_vec();
        let pad = synthetic_pad(data.len());
        Self::Data { id, data, pad }
    }

    pub fn list(kind: FourCc, children: Vec<Chunk>) -> Self {
        let pad = synthetic_pad(4 + wire_len(&children));
        Self::List {
            kind,
            children,
            pad,
        }
    }

    pub fn group(id: FourCc, children: Vec<Chunk>) -> Self {
        let pad = synthetic_pad(wire_len(&children));
        Self::Group { id, children, pad }
    }

    pub fn opaque_list(kind: FourCc, body: impl AsRef<[u8]>) -> Self {
        let body = body.as_ref().to_vec();
        let pad = synthetic_pad(4 + body.len());
        Self::OpaqueList { kind, body, pad }
    }
}

fn synthetic_pad(len: usize) -> Option<u8> {
    if len % 2 == 1 {
        Some(0)
    } else {
        None
    }
}

fn wire_len(chunks: &[Chunk]) -> usize {
    chunks.iter().map(one_wire_len).sum()
}

fn one_wire_len(chunk: &Chunk) -> usize {
    let payload = payload_len(chunk);
    8 + payload + usize::from(payload % 2 == 1)
}

fn payload_len(chunk: &Chunk) -> usize {
    match chunk {
        Chunk::Data { data, .. } => data.len(),
        Chunk::List { children, .. } => 4 + wire_len(children),
        Chunk::Group { children, .. } => wire_len(children),
        Chunk::OpaqueList { body, .. } => 4 + body.len(),
        Chunk::Xml { .. } => 0,
    }
}

pub(crate) fn is_group(id: FourCc) -> bool {
    id.0 == *b"tdsn" || id.0 == *b"fnam" || id.0 == *b"pdnm"
}

pub(crate) fn read_chunks(data: &[u8], depth: usize) -> Result<Vec<Chunk>, Error> {
    if depth > MAX_DEPTH {
        return Err(Error::TooDeep);
    }
    let mut rest = data;
    let mut chunks = Vec::new();
    while !rest.is_empty() {
        if rest.len() < 8 {
            return Err(Error::Truncated);
        }
        let id = FourCc([rest[0], rest[1], rest[2], rest[3]]);
        let size = u32::from_be_bytes([rest[4], rest[5], rest[6], rest[7]]) as usize;
        let payload_end = 8 + size;
        if rest.len() < payload_end {
            return Err(Error::Truncated);
        }
        let payload = &rest[8..payload_end];
        let (pad, next) = if size % 2 == 1 {
            if rest.len() < payload_end + 1 {
                return Err(Error::Truncated);
            }
            (Some(rest[payload_end]), payload_end + 1)
        } else {
            (None, payload_end)
        };
        rest = &rest[next..];
        chunks.push(decode_chunk(id, payload, pad, depth)?);
    }
    Ok(chunks)
}

fn decode_chunk(id: FourCc, payload: &[u8], pad: Option<u8>, depth: usize) -> Result<Chunk, Error> {
    if id.0 == *b"LIST" {
        if payload.len() < 4 {
            return Err(Error::BadList);
        }
        let kind = FourCc([payload[0], payload[1], payload[2], payload[3]]);
        let body = &payload[4..];
        if kind.0 == *b"btdk" {
            return Ok(Chunk::OpaqueList {
                kind,
                body: body.to_vec(),
                pad,
            });
        }
        let children = read_chunks(body, depth + 1)?;
        return Ok(Chunk::List {
            kind,
            children,
            pad,
        });
    }
    if is_group(id) {
        let children = read_chunks(payload, depth + 1)?;
        return Ok(Chunk::Group { id, children, pad });
    }
    Ok(Chunk::Data {
        id,
        data: payload.to_vec(),
        pad,
    })
}

pub(crate) fn write_chunks(chunks: &[Chunk], out: &mut Vec<u8>) -> Result<(), Error> {
    for chunk in chunks {
        write_chunk(chunk, out)?;
    }
    Ok(())
}

fn write_chunk(chunk: &Chunk, out: &mut Vec<u8>) -> Result<(), Error> {
    let (id, payload) = encode_payload(chunk)?;
    if payload.len() > u32::MAX as usize {
        return Err(Error::TooLarge);
    }
    out.extend_from_slice(&id.0);
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(&payload);
    if payload.len() % 2 == 1 {
        out.push(stored_pad(chunk).unwrap_or(0));
    }
    Ok(())
}

fn encode_payload(chunk: &Chunk) -> Result<(FourCc, Vec<u8>), Error> {
    match chunk {
        Chunk::Data { id, data, .. } => Ok((*id, data.clone())),
        Chunk::List { kind, children, .. } => {
            let mut payload = kind.0.to_vec();
            write_chunks(children, &mut payload)?;
            Ok((FourCc::new(b"LIST"), payload))
        }
        Chunk::Group { id, children, .. } => {
            let mut payload = Vec::new();
            write_chunks(children, &mut payload)?;
            Ok((*id, payload))
        }
        Chunk::OpaqueList { kind, body, .. } => {
            let mut payload = kind.0.to_vec();
            payload.extend_from_slice(body);
            Ok((FourCc::new(b"LIST"), payload))
        }
        Chunk::Xml { name, .. } => Err(Error::UnmappedXml(name.clone())),
    }
}

fn stored_pad(chunk: &Chunk) -> Option<u8> {
    match chunk {
        Chunk::Data { pad, .. }
        | Chunk::List { pad, .. }
        | Chunk::Group { pad, .. }
        | Chunk::OpaqueList { pad, .. } => *pad,
        Chunk::Xml { .. } => None,
    }
}
