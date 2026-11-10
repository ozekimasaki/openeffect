//! 仕様の各チェックに対応する往復テスト。
//! 要確認のオフセットはクレートのフィールド関数にしない。
//! 合格条件がバイト範囲の一致である項目は、ここでその範囲を比較する。

use io_aep::fields::{cdat, cdta, idta, ldta, lhd3, nums, ppsn, tdmn, utf8};
use io_aep::{
    read_aep, read_aepx, read_aet, write_aep, write_aepx, write_aet, Chunk, Document, Error, FourCc,
};

fn fcc(bytes: &[u8; 4]) -> FourCc {
    FourCc::new(bytes)
}

fn data(id: &[u8; 4], payload: impl AsRef<[u8]>) -> Chunk {
    Chunk::data(fcc(id), payload)
}

fn list(kind: &[u8; 4], children: Vec<Chunk>) -> Chunk {
    Chunk::list(fcc(kind), children)
}

fn group(id: &[u8; 4], children: Vec<Chunk>) -> Chunk {
    Chunk::group(fcc(id), children)
}

fn opaque(kind: &[u8; 4], body: impl AsRef<[u8]>) -> Chunk {
    Chunk::opaque_list(fcc(kind), body)
}

fn document(chunks: Vec<Chunk>) -> Document {
    Document { chunks }
}

fn write_roundtrip(doc: &Document) -> (Document, Vec<u8>) {
    let bytes = write_aep(doc).expect("write aep");
    let back = read_aep(&bytes).expect("read aep");
    assert_eq!(&back, doc);
    assert_eq!(write_aep(&back).expect("rewrite"), bytes);
    (back, bytes)
}

fn rt_data(id: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let (back, _) = write_roundtrip(&document(vec![data(id, payload)]));
    match &back.chunks[0] {
        Chunk::Data { data, .. } => data.clone(),
        other => panic!("data chunk expected, got {other:?}"),
    }
}

fn patterned(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|index| seed.wrapping_add(index as u8))
        .collect()
}

fn push_chunk(buf: &mut Vec<u8>, id: &[u8; 4], payload: &[u8], pad: u8) {
    buf.extend_from_slice(id);
    buf.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    buf.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        buf.push(pad);
    }
}

fn rifx(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFX");
    out.extend_from_slice(&((body.len() + 4) as u32).to_be_bytes());
    out.extend_from_slice(b"Egg!");
    out.extend_from_slice(body);
    out
}

fn assert_raw_identity(file: &[u8]) -> Document {
    let doc = read_aep(file).expect("read raw");
    assert_eq!(write_aep(&doc).expect("write raw"), file);
    doc
}

fn top_level_ids(file: &[u8]) -> Vec<[u8; 4]> {
    let mut ids = Vec::new();
    let mut index = 12;
    while index + 8 <= file.len() {
        let mut id = [0; 4];
        id.copy_from_slice(&file[index..index + 4]);
        ids.push(id);
        let size = u32::from_be_bytes(file[index + 4..index + 8].try_into().unwrap()) as usize;
        index += 8 + size + usize::from(size % 2 == 1);
    }
    assert_eq!(index, file.len());
    ids
}

fn odd_even_file() -> Vec<u8> {
    let mut body = Vec::new();
    push_chunk(&mut body, b"nhed", &[1, 2, 3], 0x5A);
    push_chunk(&mut body, b"even", &[4, 5], 0xFF);
    rifx(&body)
}

fn idta_bytes(kind: u16, id: u32, marker: u8) -> Vec<u8> {
    let mut payload = patterned(64, 0x44);
    payload[0..2].copy_from_slice(&kind.to_be_bytes());
    payload[16..20].copy_from_slice(&id.to_be_bytes());
    payload[58] = marker;
    payload
}

fn cdta_payload_with(seed: u8) -> Vec<u8> {
    let mut payload = patterned(208, seed);
    payload[52] = 10;
    payload[53] = 20;
    payload[54] = 30;
    payload[140..142].copy_from_slice(&1920u16.to_be_bytes());
    payload[142..144].copy_from_slice(&1080u16.to_be_bytes());
    payload[156..158].copy_from_slice(&24u16.to_be_bytes());
    payload
}

fn cdta_payload() -> Vec<u8> {
    cdta_payload_with(0x21)
}

fn confirmed_cdta(index: usize) -> bool {
    (52..55).contains(&index) || (140..144).contains(&index) || (156..158).contains(&index)
}

fn assert_cdta_other_bytes(seed: u8) {
    let payload = cdta_payload_with(seed);
    let back = rt_data(b"cdta", &payload);
    for (index, (left, right)) in payload.iter().zip(&back).enumerate() {
        if !confirmed_cdta(index) {
            assert_eq!(left, right, "index {index}");
        }
    }
    assert_eq!(cdta::width(&back), Some(1920));
    assert_eq!(cdta::height(&back), Some(1080));
    assert_eq!(cdta::frame_rate(&back), Some(24));
    assert_eq!(cdta::background(&back), Some([10, 20, 30]));
    assert_eq!(back, payload);
}

fn ldta_with_known_fields(seed: u8) -> Vec<u8> {
    let mut payload = patterned(96, seed);
    payload[4..6].copy_from_slice(&ldta::BEST.to_be_bytes());
    payload[37] = 0b0100_0010;
    payload[38] = 0b0000_1110;
    payload[39] = 0b1110_1101;
    payload[40..44].copy_from_slice(&0x1122_3344u32.to_be_bytes());
    payload
}

fn confirmed_ldta(index: usize) -> bool {
    matches!(index, 4 | 5 | 37 | 38 | 39 | 40 | 41 | 42 | 43)
}

fn assert_unconfirmed_ldta(seed: u8) {
    let payload = ldta_with_known_fields(seed);
    let back = rt_data(b"ldta", &payload);
    for (index, (left, right)) in payload.iter().zip(&back).enumerate() {
        if !confirmed_ldta(index) {
            assert_eq!(left, right, "index {index}");
        }
    }
    assert_eq!(ldta::quality(&back), Some(ldta::BEST));
    assert_eq!(ldta::source_id(&back), Some(0x1122_3344));
    assert_eq!(back, payload);
}

fn assert_named_bit(place: (usize, u8)) {
    let (offset, bit) = place;
    let mut on = patterned(96, 0x3C);
    on[offset] |= 1 << bit;
    let back_on = rt_data(b"ldta", &on);
    assert_eq!(ldta::bit(&back_on, offset, bit), Some(true));
    assert_eq!(back_on[offset], on[offset]);
    assert_eq!(back_on, on);

    let mut off = on.clone();
    off[offset] &= !(1 << bit);
    let back_off = rt_data(b"ldta", &off);
    assert_eq!(ldta::bit(&back_off, offset, bit), Some(false));
    assert_eq!(back_off[offset], off[offset]);
}

fn each_data(doc: &Document, mut visit: impl FnMut(FourCc, &[u8])) {
    fn walk(chunks: &[Chunk], visit: &mut dyn FnMut(FourCc, &[u8])) {
        for chunk in chunks {
            match chunk {
                Chunk::Data { id, data, .. } => visit(*id, data),
                Chunk::List { children, .. }
                | Chunk::Group { children, .. }
                | Chunk::Xml { children, .. } => walk(children, visit),
                Chunk::OpaqueList { .. } => {}
            }
        }
    }
    walk(&doc.chunks, &mut visit);
}

fn signatures(doc: &Document) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    fn walk(chunks: &[Chunk], path: &str, out: &mut Vec<(String, Vec<u8>)>) {
        for (index, chunk) in chunks.iter().enumerate() {
            let here = format!("{path}/{index}");
            match chunk {
                Chunk::Data { id, data, pad } => {
                    out.push((format!("{here}/{id}"), data.clone()));
                    out.push((
                        format!("{here}/pad"),
                        pad.map(|byte| vec![byte]).unwrap_or_default(),
                    ));
                }
                Chunk::List {
                    kind,
                    children,
                    pad,
                } => {
                    out.push((
                        format!("{here}/LIST/{kind}"),
                        pad.map(|byte| vec![byte]).unwrap_or_default(),
                    ));
                    walk(children, &here, out);
                }
                Chunk::Group { id, children, pad } => {
                    out.push((
                        format!("{here}/GROUP/{id}"),
                        pad.map(|byte| vec![byte]).unwrap_or_default(),
                    ));
                    walk(children, &here, out);
                }
                Chunk::OpaqueList { kind, body, pad } => {
                    out.push((format!("{here}/OPAQUE/{kind}"), body.clone()));
                    out.push((
                        format!("{here}/pad"),
                        pad.map(|byte| vec![byte]).unwrap_or_default(),
                    ));
                }
                Chunk::Xml {
                    name,
                    text,
                    children,
                    ..
                } => {
                    out.push((format!("{here}/XML/{name}"), text.as_bytes().to_vec()));
                    walk(children, &here, out);
                }
            }
        }
    }
    walk(&doc.chunks, "", &mut out);
    out
}

fn assert_one_utf8_change(before: &Document, after: &Document, next: &[u8]) {
    let left = signatures(before);
    let right = signatures(after);
    assert_eq!(left.len(), right.len());
    let mut changed = 0;
    for (left_item, right_item) in left.iter().zip(&right) {
        assert_eq!(left_item.0, right_item.0);
        if left_item.1 != right_item.1 {
            changed += 1;
            assert!(left_item.0.contains("Utf8"), "{}", left_item.0);
            assert_eq!(right_item.1, next);
        }
    }
    assert_eq!(changed, 1);
}

#[test]
fn rifx_roundtrip() {
    let file = odd_even_file();
    assert_eq!(&file[0..4], b"RIFX");
    let doc = assert_raw_identity(&file);
    let again = write_aep(&doc).unwrap();
    assert_eq!(&again[0..4], b"RIFX");
}

#[test]
fn file_size_is_length_minus_eight() {
    let file = odd_even_file();
    let declared = u32::from_be_bytes(file[4..8].try_into().unwrap());
    assert_eq!(declared as usize, file.len() - 8);
    let doc = assert_raw_identity(&file);
    let again = write_aep(&doc).unwrap();
    let declared = u32::from_be_bytes(again[4..8].try_into().unwrap());
    assert_eq!(declared as usize, again.len() - 8);
    assert_eq!(again, file);

    let mut broken = file.clone();
    broken[7] = 0;
    assert!(matches!(read_aep(&broken), Err(Error::BadFileSize { .. })));
}

#[test]
fn egg_form_roundtrip_rejects_other_forms() {
    let file = odd_even_file();
    assert_eq!(&file[8..12], b"Egg!");
    let doc = assert_raw_identity(&file);
    assert_eq!(&write_aep(&doc).unwrap()[8..12], b"Egg!");

    let mut other = file.clone();
    other[8..12].copy_from_slice(b"WAVE");
    assert!(matches!(read_aep(&other), Err(Error::UnsupportedForm(_))));
    let mut riff = file.clone();
    riff[0..4].copy_from_slice(b"RIFF");
    assert!(matches!(read_aep(&riff), Err(Error::BadMagic)));
}

#[test]
fn chunk_fourcc_and_order_roundtrip() {
    let doc = document(vec![
        data(b"nhed", [1, 2, 3, 4]),
        data(b"ZzZz", [9, 9]),
        list(b"Fold", vec![]),
    ]);
    let (back, bytes) = write_roundtrip(&doc);
    assert_eq!(top_level_ids(&bytes), vec![*b"nhed", *b"ZzZz", *b"LIST"]);
    match &back.chunks[2] {
        Chunk::List { kind, .. } => assert_eq!(kind.0, *b"Fold"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn chunk_size_is_payload_length() {
    let file = odd_even_file();
    let size = u32::from_be_bytes(file[16..20].try_into().unwrap());
    assert_eq!(size, 3);
    assert_eq!(&file[20..23], &[1, 2, 3]);
    assert_eq!(file[23], 0x5A);
    let doc = assert_raw_identity(&file);
    match &doc.chunks[0] {
        Chunk::Data { data, .. } => assert_eq!(data.len(), size as usize),
        other => panic!("{other:?}"),
    }
}

#[test]
fn odd_size_keeps_pad_even_size_has_none() {
    let file = odd_even_file();
    assert_eq!(file[23], 0x5A);
    assert_eq!(&file[24..28], b"even");
    assert_eq!(file.len(), 34);
    let doc = assert_raw_identity(&file);
    match &doc.chunks[0] {
        Chunk::Data { pad, .. } => assert_eq!(*pad, Some(0x5A)),
        other => panic!("{other:?}"),
    }
    match &doc.chunks[1] {
        Chunk::Data { data, pad, .. } => {
            assert_eq!(data, &[4, 5]);
            assert!(pad.is_none());
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn list_type_children_and_bytes_roundtrip() {
    let mut inner = Vec::new();
    push_chunk(&mut inner, b"idta", &[0x07], 0x11);
    push_chunk(&mut inner, b"Utf8", b"Ab", 0);
    let mut payload = b"Item".to_vec();
    payload.extend_from_slice(&inner);
    let mut body = Vec::new();
    push_chunk(&mut body, b"LIST", &payload, 0);
    let file = rifx(&body);
    let doc = assert_raw_identity(&file);
    match &doc.chunks[0] {
        Chunk::List { kind, children, .. } => {
            assert_eq!(kind.0, *b"Item");
            assert_eq!(children.len(), 2);
            match &children[0] {
                Chunk::Data { id, data, pad } => {
                    assert_eq!(id.0, *b"idta");
                    assert_eq!(data, &[0x07]);
                    assert_eq!(*pad, Some(0x11));
                }
                other => panic!("{other:?}"),
            }
            match &children[1] {
                Chunk::Data { id, data, pad } => {
                    assert_eq!(id.0, *b"Utf8");
                    assert_eq!(data, b"Ab");
                    assert!(pad.is_none());
                }
                other => panic!("{other:?}"),
            }
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn nhed_payload_roundtrip() {
    let payload = patterned(20, 0x10);
    assert_eq!(rt_data(b"nhed", &payload), payload);
}

#[test]
fn nhed_depth_byte_roundtrip() {
    for value in [0u8, 1, 2] {
        let payload = vec![value; 24];
        assert_eq!(rt_data(b"nhed", &payload), payload);
    }
}

#[test]
fn exen_list_roundtrip() {
    let doc = document(vec![list(
        b"ExEn",
        vec![data(b"bnry", b"extendscript"), data(b"tail", [1, 2, 3])],
    )]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List { kind, children, .. } => {
            assert_eq!(kind.0, *b"ExEn");
            assert_eq!(children.len(), 2);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn expression_engine_name_bytes_roundtrip() {
    let name = b"javascript-1.0";
    let doc = document(vec![list(b"ExEn", vec![data(b"bnry", name)])]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List { children, .. } => match &children[0] {
            Chunk::Data { data, .. } => assert_eq!(data, name),
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
}

#[test]
fn fold_children_roundtrip() {
    let doc = document(vec![list(
        b"Fold",
        vec![
            list(b"Item", vec![data(b"Utf8", b"A")]),
            list(b"Item", vec![data(b"Utf8", b"B")]),
            list(b"Item", vec![data(b"Utf8", b"C")]),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List { kind, children, .. } => {
            assert_eq!(kind.0, *b"Fold");
            let names: Vec<&[u8]> = children
                .iter()
                .map(|child| match child {
                    Chunk::List { children, .. } => match &children[0] {
                        Chunk::Data { data, .. } => data.as_slice(),
                        other => panic!("{other:?}"),
                    },
                    other => panic!("{other:?}"),
                })
                .collect();
            assert_eq!(
                names,
                vec![b"A".as_slice(), b"B".as_slice(), b"C".as_slice()]
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn fdta_stays_in_place() {
    let doc = document(vec![list(
        b"Fold",
        vec![
            list(b"Item", vec![data(b"Utf8", b"A")]),
            data(b"fdta", [0x01]),
            list(b"Item", vec![data(b"Utf8", b"B")]),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List { children, .. } => match &children[1] {
            Chunk::Data { id, data, .. } => {
                assert_eq!(id.0, *b"fdta");
                assert_eq!(data, &[0x01]);
            }
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
}

#[test]
fn item_idta_name_and_children_roundtrip() {
    let doc = document(vec![list(
        b"Item",
        vec![
            data(b"idta", idta_bytes(idta::FOLDER, 9, 1)),
            data(b"Utf8", b"Folder"),
            list(b"Sfdr", vec![list(b"Item", vec![data(b"Utf8", b"Child")])]),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List { kind, children, .. } => {
            assert_eq!(kind.0, *b"Item");
            assert_eq!(children.len(), 3);
            match &children[2] {
                Chunk::List { kind, children, .. } => {
                    assert_eq!(kind.0, *b"Sfdr");
                    assert_eq!(children.len(), 1);
                }
                other => panic!("{other:?}"),
            }
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn idta_payload_roundtrip() {
    let payload = idta_bytes(idta::COMPOSITION, 0x0102_0304, 3);
    assert_eq!(rt_data(b"idta", &payload), payload);
}

#[test]
fn idta_kind_values_roundtrip() {
    for kind in [idta::FOLDER, idta::COMPOSITION, idta::FOOTAGE, 9] {
        let payload = idta_bytes(kind, 1, 0);
        let back = rt_data(b"idta", &payload);
        assert_eq!(idta::kind(&back), Some(kind));
        assert_eq!(back, payload);
    }
}

#[test]
fn idta_id_matches_layer_source_id() {
    let id: u32 = 0x0A0B_0C0D;
    let mut ldta_bytes = patterned(96, 0x22);
    ldta_bytes[40..44].copy_from_slice(&id.to_be_bytes());
    let doc = document(vec![list(
        b"Fold",
        vec![
            list(
                b"Item",
                vec![
                    data(b"idta", idta_bytes(idta::FOOTAGE, id, 0)),
                    data(b"Utf8", b"clip"),
                ],
            ),
            list(
                b"Item",
                vec![
                    data(b"idta", idta_bytes(idta::COMPOSITION, 2, 0)),
                    list(b"Layr", vec![data(b"ldta", ldta_bytes)]),
                ],
            ),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    let mut item_id = None;
    let mut source_id = None;
    each_data(&back, |tag, bytes| {
        if tag.0 == *b"idta" && idta::kind(bytes) == Some(idta::FOOTAGE) {
            item_id = idta::item_id(bytes);
        }
        if tag.0 == *b"ldta" {
            source_id = ldta::source_id(bytes);
        }
    });
    assert_eq!(item_id, Some(id));
    assert_eq!(source_id, Some(id));
}

#[test]
fn idta_label_bytes_roundtrip() {
    let payload = idta_bytes(idta::FOOTAGE, 4, 0x5C);
    let back = rt_data(b"idta", &payload);
    assert_eq!(back, payload);
    assert_eq!(back[58], 0x5C);
}

#[test]
fn utf8_keeps_trailing_nul() {
    let payload = b"Name\0";
    let back = rt_data(b"Utf8", payload);
    assert_eq!(back, payload);
    assert_eq!(utf8::as_str(&back), Some("Name\0"));
}

#[test]
fn sfdr_item_order_roundtrip() {
    let doc = document(vec![list(
        b"Sfdr",
        vec![
            list(b"Item", vec![data(b"Utf8", b"One")]),
            list(b"Item", vec![data(b"Utf8", b"Two")]),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List { kind, children, .. } => {
            assert_eq!(kind.0, *b"Sfdr");
            assert_eq!(children.len(), 2);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn fiac_payload_roundtrip() {
    let payload = [0x01, 0x00, 0xFF];
    assert_eq!(rt_data(b"fiac", &payload), payload);
}

#[test]
fn item_name_roundtrip() {
    let doc = document(vec![list(
        b"Item",
        vec![
            data(b"idta", idta_bytes(idta::COMPOSITION, 1, 0)),
            data(b"Utf8", "コンポ".as_bytes()),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    let mut name = None;
    each_data(&back, |tag, bytes| {
        if tag.0 == *b"Utf8" {
            name = Some(bytes.to_vec());
        }
    });
    assert_eq!(name.as_deref(), Some("コンポ".as_bytes()));
}

#[test]
fn layer_name_empty_stays_empty() {
    let mut ldta_bytes = patterned(96, 2);
    ldta_bytes[40..44].copy_from_slice(&7u32.to_be_bytes());
    let doc = document(vec![list(
        b"Item",
        vec![
            data(b"idta", idta_bytes(idta::COMPOSITION, 1, 0)),
            list(b"Layr", vec![data(b"ldta", ldta_bytes), data(b"Utf8", b"")]),
            list(
                b"Item",
                vec![
                    data(b"idta", idta_bytes(idta::FOOTAGE, 7, 0)),
                    data(b"Utf8", b"clip"),
                ],
            ),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    let mut names = Vec::new();
    each_data(&back, |tag, bytes| {
        if tag.0 == *b"Utf8" {
            names.push(bytes.to_vec());
        }
    });
    assert_eq!(names, vec![Vec::<u8>::new(), b"clip".to_vec()]);
}

#[test]
fn folder_name_roundtrip() {
    let doc = document(vec![list(
        b"Item",
        vec![
            data(b"idta", idta_bytes(idta::FOLDER, 3, 0)),
            data(b"Utf8", "資料".as_bytes()),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    let mut found = false;
    each_data(&back, |tag, bytes| {
        if tag.0 == *b"idta" {
            assert_eq!(idta::kind(bytes), Some(idta::FOLDER));
        }
        if tag.0 == *b"Utf8" {
            assert_eq!(bytes, "資料".as_bytes());
            found = true;
        }
    });
    assert!(found);
}

#[test]
fn footage_custom_name_roundtrip() {
    let doc = document(vec![list(
        b"Item",
        vec![
            data(b"idta", idta_bytes(idta::FOOTAGE, 8, 0)),
            data(b"Utf8", b"Hero"),
        ],
    )]);
    let xml = write_aepx(&doc).unwrap();
    assert!(xml.contains("<string>Hero</string>"));
    let back = read_aepx(&xml).unwrap();
    assert_eq!(back, doc);
    let _ = write_roundtrip(&doc);
}

#[test]
fn solid_name_opti_roundtrip_adds_no_string() {
    let opti = b"Soli\x00\x00rest-of-opti";
    let doc = document(vec![list(
        b"Item",
        vec![
            data(b"idta", idta_bytes(idta::FOOTAGE, 8, 0)),
            data(b"opti", opti),
        ],
    )]);
    let xml = write_aepx(&doc).unwrap();
    assert!(!xml.contains("<string"));
    assert_eq!(read_aepx(&xml).unwrap(), doc);
    assert_eq!(rt_data(b"opti", opti), opti);
}

#[test]
fn tdsn_children_roundtrip() {
    let doc = document(vec![group(
        b"tdsn",
        vec![data(b"Utf8", b"Label"), data(b"ZzZz", [1, 2, 3, 4])],
    )]);
    let (_, bytes) = write_roundtrip(&doc);
    assert_eq!(&bytes[12..16], b"tdsn");
    assert_ne!(&bytes[12..16], b"LIST");
}

#[test]
fn tdsn_display_name_roundtrip() {
    for label in [b"Title".as_slice(), b"-_0_/-".as_slice()] {
        let doc = document(vec![group(b"tdsn", vec![data(b"Utf8", label)])]);
        let (back, _) = write_roundtrip(&doc);
        match &back.chunks[0] {
            Chunk::Group { children, .. } => match &children[0] {
                Chunk::Data { data, .. } => assert_eq!(data, label),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn fnam_name_and_unknown_child_roundtrip() {
    let doc = document(vec![group(
        b"fnam",
        vec![data(b"Utf8", b"Blur"), data(b"unkn", [9, 8, 7])],
    )]);
    let (back, bytes) = write_roundtrip(&doc);
    assert_eq!(&bytes[12..16], b"fnam");
    match &back.chunks[0] {
        Chunk::Group { children, .. } => {
            match &children[0] {
                Chunk::Data { data, .. } => assert_eq!(data, b"Blur"),
                other => panic!("{other:?}"),
            }
            match &children[1] {
                Chunk::Data { id, data, .. } => {
                    assert_eq!(id.0, *b"unkn");
                    assert_eq!(data, &[9, 8, 7]);
                }
                other => panic!("{other:?}"),
            }
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn pdnm_separator_bytes_roundtrip() {
    let payload = b"Red|Green|Blue";
    let doc = document(vec![group(b"pdnm", vec![data(b"Utf8", payload)])]);
    let (back, bytes) = write_roundtrip(&doc);
    assert_eq!(&bytes[12..16], b"pdnm");
    match &back.chunks[0] {
        Chunk::Group { children, .. } => match &children[0] {
            Chunk::Data { data, .. } => assert_eq!(data, payload),
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
}

#[test]
fn cmta_stays_in_place() {
    let doc = document(vec![
        data(b"nhed", [1, 2, 3, 4]),
        data(b"cmta", b"a note\0"),
        list(b"Fold", vec![]),
    ]);
    let (back, bytes) = write_roundtrip(&doc);
    assert_eq!(top_level_ids(&bytes)[1], *b"cmta");
    match &back.chunks[1] {
        Chunk::Data { id, data, .. } => {
            assert_eq!(id.0, *b"cmta");
            assert_eq!(data, b"a note\0");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn comment_string_edit_changes_only_that_chunk() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<AfterEffectsProject>
  <Item>
    <string>Comp</string>
    <string>note</string>
  </Item>
  <cmta bdata="636f6d6d656e74"/>
</AfterEffectsProject>
"#;
    let original = read_aepx(xml).unwrap();
    let edited = read_aepx(&xml.replace("<string>note</string>", "<string>text</string>")).unwrap();
    assert_one_utf8_change(&original, &edited, b"text");
    let original_aep = write_aep(&original).unwrap();
    let edited_aep = write_aep(&edited).unwrap();
    assert_ne!(original_aep, edited_aep);
    assert_eq!(read_aep(&edited_aep).unwrap(), edited);
    assert_eq!(read_aep(&original_aep).unwrap(), original);
}

#[test]
fn cdta_payload_roundtrip() {
    let payload = cdta_payload();
    let back = rt_data(b"cdta", &payload);
    assert_eq!(back, payload);
}

#[test]
fn comp_width_roundtrip() {
    let payload = cdta_payload();
    let back = rt_data(b"cdta", &payload);
    assert_eq!(cdta::width(&back), Some(1920));
    assert_eq!(&back[140..142], &payload[140..142]);
}

#[test]
fn comp_height_roundtrip() {
    let payload = cdta_payload();
    let back = rt_data(b"cdta", &payload);
    assert_eq!(cdta::height(&back), Some(1080));
    assert_eq!(&back[142..144], &payload[142..144]);
}

#[test]
fn comp_frame_rate_roundtrip() {
    let payload = cdta_payload();
    let back = rt_data(b"cdta", &payload);
    assert_eq!(cdta::frame_rate(&back), Some(24));
    assert_eq!(&back[156..158], &payload[156..158]);
}

#[test]
fn comp_background_roundtrip() {
    let payload = cdta_payload();
    let back = rt_data(b"cdta", &payload);
    assert_eq!(cdta::background(&back), Some([10, 20, 30]));
    assert_eq!(&back[52..55], &payload[52..55]);
}

#[test]
fn cdta_resolution_bytes_roundtrip() {
    let payload = cdta_payload();
    let back = rt_data(b"cdta", &payload);
    assert_eq!(&back[0..4], &payload[0..4]);
}

#[test]
fn cdta_frame_rate_fraction_bytes_roundtrip() {
    let payload = cdta_payload();
    let back = rt_data(b"cdta", &payload);
    assert_eq!(&back[4..12], &payload[4..12]);
    assert_eq!(&back[156..158], &payload[156..158]);
}

#[test]
fn cdta_duration_bytes_roundtrip() {
    assert_cdta_other_bytes(0x31);
}

#[test]
fn cdta_playhead_in_out_bytes_roundtrip() {
    assert_cdta_other_bytes(0x32);
}

#[test]
fn cdta_pixel_aspect_bytes_roundtrip() {
    let payload = cdta_payload();
    let back = rt_data(b"cdta", &payload);
    assert_eq!(&back[144..156], &payload[144..156]);
}

#[test]
fn cdta_shy_byte_roundtrip() {
    let mut payload = cdta_payload();
    payload[139] = 0b0000_0001;
    let back = rt_data(b"cdta", &payload);
    assert_eq!(back[139], payload[139]);
    assert_eq!(back, payload);
}

#[test]
fn cdta_motion_blur_byte_roundtrip() {
    let mut payload = cdta_payload();
    payload[139] = 0b0000_1000;
    let back = rt_data(b"cdta", &payload);
    assert_eq!(back[139], 0b0000_1000);
}

#[test]
fn cdta_frame_blend_byte_roundtrip() {
    let mut payload = cdta_payload();
    payload[139] = 0b0001_0000;
    let back = rt_data(b"cdta", &payload);
    assert_eq!(back[139], payload[139]);
}

#[test]
fn cdta_preserve_frame_rate_byte_roundtrip() {
    let mut payload = cdta_payload();
    payload[139] = 0b0010_0000;
    let back = rt_data(b"cdta", &payload);
    assert_eq!(back[139], payload[139]);
}

#[test]
fn cdta_preserve_resolution_byte_roundtrip() {
    let mut payload = cdta_payload();
    payload[139] = 0b1000_0000;
    let back = rt_data(b"cdta", &payload);
    assert_eq!(back[139], payload[139]);
}

#[test]
fn cdta_shutter_angle_bytes_roundtrip() {
    let payload = cdta_payload();
    let back = rt_data(b"cdta", &payload);
    assert_eq!(&back[174..176], &payload[174..176]);
}

#[test]
fn cdta_shutter_phase_bytes_roundtrip() {
    let payload = cdta_payload();
    let back = rt_data(b"cdta", &payload);
    assert_eq!(&back[176..180], &payload[176..180]);
}

#[test]
fn cdta_sample_limit_bytes_roundtrip() {
    let payload = cdta_payload();
    let back = rt_data(b"cdta", &payload);
    assert_eq!(&back[196..200], &payload[196..200]);
}

#[test]
fn cdta_samples_per_frame_bytes_roundtrip() {
    let payload = cdta_payload();
    let back = rt_data(b"cdta", &payload);
    assert_eq!(&back[200..204], &payload[200..204]);
}

#[test]
fn layr_order_roundtrip() {
    let doc = document(vec![
        list(b"Layr", vec![data(b"Utf8", b"Back")]),
        list(b"Layr", vec![data(b"Utf8", b"Front")]),
    ]);
    let (back, _) = write_roundtrip(&doc);
    let mut names = Vec::new();
    each_data(&back, |tag, bytes| {
        if tag.0 == *b"Utf8" {
            names.push(bytes.to_vec());
        }
    });
    assert_eq!(names, vec![b"Back".to_vec(), b"Front".to_vec()]);
}

#[test]
fn ldta_payload_roundtrip() {
    let payload = ldta_with_known_fields(0x55);
    assert_eq!(rt_data(b"ldta", &payload), payload);
}

#[test]
fn layer_source_id_roundtrip() {
    let mut payload = patterned(96, 0x60);
    payload[40..44].copy_from_slice(&0xABCDu32.to_be_bytes());
    let back = rt_data(b"ldta", &payload);
    assert_eq!(ldta::source_id(&back), Some(0xABCD));
    assert_eq!(&back[40..44], &payload[40..44]);
}

#[test]
fn layer_quality_roundtrip() {
    for quality in [ldta::WIREFRAME, ldta::DRAFT, ldta::BEST] {
        let mut payload = patterned(96, 0x61);
        payload[4..6].copy_from_slice(&quality.to_be_bytes());
        let back = rt_data(b"ldta", &payload);
        assert_eq!(ldta::quality(&back), Some(quality));
        assert_eq!(back, payload);
    }
}

#[test]
fn ldta_leading_bytes_roundtrip() {
    let payload = ldta_with_known_fields(0x10);
    let back = rt_data(b"ldta", &payload);
    assert_eq!(&back[..4], &payload[..4]);
}

#[test]
fn bicubic_bit_roundtrip() {
    assert_named_bit(ldta::BICUBIC);
}

#[test]
fn guide_layer_bit_roundtrip() {
    assert_named_bit(ldta::GUIDE);
}

#[test]
fn solo_bit_roundtrip() {
    assert_named_bit(ldta::SOLO);
}

#[test]
fn three_d_bit_roundtrip() {
    assert_named_bit(ldta::THREE_D);
}

#[test]
fn adjustment_bit_roundtrip() {
    assert_named_bit(ldta::ADJUSTMENT);
}

#[test]
fn collapse_bit_roundtrip() {
    assert_named_bit(ldta::COLLAPSE);
}

#[test]
fn layer_shy_bit_roundtrip() {
    assert_named_bit(ldta::SHY);
}

#[test]
fn lock_bit_roundtrip() {
    assert_named_bit(ldta::LOCKED);
}

#[test]
fn layer_motion_blur_bit_roundtrip() {
    assert_named_bit(ldta::MOTION_BLUR);
}

#[test]
fn effects_enabled_bit_roundtrip() {
    assert_named_bit(ldta::EFFECTS);
}

#[test]
fn video_switch_bit_roundtrip() {
    assert_named_bit(ldta::VIDEO);
}

#[test]
fn frame_blend_mode_byte_roundtrip() {
    let mut payload = ldta_with_known_fields(0x11);
    payload[37] = 0b1010_0100;
    let back = rt_data(b"ldta", &payload);
    assert_eq!(back[37], payload[37]);
}

#[test]
fn frame_blend_enabled_byte_roundtrip() {
    let mut payload = ldta_with_known_fields(0x12);
    payload[39] = 0b0001_0000;
    let back = rt_data(b"ldta", &payload);
    assert_eq!(back[39], payload[39]);
}

#[test]
fn audio_switch_byte_roundtrip() {
    let mut payload = ldta_with_known_fields(0x13);
    payload[39] = 0b0000_0010;
    let back = rt_data(b"ldta", &payload);
    assert_eq!(back[39], payload[39]);
}

#[test]
fn auto_orient_byte_roundtrip() {
    let mut payload = ldta_with_known_fields(0x14);
    payload[38] = 0b0000_0001;
    let back = rt_data(b"ldta", &payload);
    assert_eq!(back[38], payload[38]);
}

#[test]
fn null_layer_byte_roundtrip() {
    let mut payload = ldta_with_known_fields(0x15);
    payload[38] = 0b1000_0000;
    let back = rt_data(b"ldta", &payload);
    assert_eq!(back[38], payload[38]);
}

#[test]
fn parent_id_bytes_roundtrip() {
    assert_unconfirmed_ldta(0x21);
}

#[test]
fn track_matte_mode_bytes_roundtrip() {
    assert_unconfirmed_ldta(0x22);
}

#[test]
fn track_matte_id_bytes_roundtrip() {
    assert_unconfirmed_ldta(0x23);
}

#[test]
fn layer_kind_bytes_roundtrip() {
    assert_unconfirmed_ldta(0x24);
}

#[test]
fn layer_timing_bytes_roundtrip() {
    assert_unconfirmed_ldta(0x25);
}

fn view_list(kind: &[u8; 4]) {
    let doc = document(vec![list(
        kind,
        vec![data(b"ldta", [1, 2, 3, 4]), data(b"Utf8", b"view")],
    )]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List {
            kind: got,
            children,
            ..
        } => {
            assert_eq!(got.0, *kind);
            assert_eq!(children.len(), 2);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn secl_children_roundtrip() {
    view_list(b"SecL");
}

#[test]
fn clay_children_roundtrip() {
    view_list(b"CLay");
}

#[test]
fn dlay_children_roundtrip() {
    view_list(b"DLay");
}

#[test]
fn slay_children_roundtrip() {
    view_list(b"SLay");
}

#[test]
fn pin_list_type_keeps_trailing_space() {
    let mut inner = Vec::new();
    push_chunk(&mut inner, b"sspc", &[9, 9, 9, 9], 0);
    let mut payload = b"Pin ".to_vec();
    payload.extend_from_slice(&inner);
    let mut body = Vec::new();
    push_chunk(&mut body, b"LIST", &payload, 0);
    let file = rifx(&body);
    assert_eq!(&file[20..24], b"Pin ");
    let doc = assert_raw_identity(&file);
    match &doc.chunks[0] {
        Chunk::List { kind, children, .. } => {
            assert_eq!(&kind.0, b"Pin ");
            assert_eq!(children.len(), 1);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn sspc_payload_roundtrip() {
    let payload = patterned(32, 0x70);
    assert_eq!(rt_data(b"sspc", &payload), payload);
}

#[test]
fn footage_width_bytes_roundtrip() {
    let payload = patterned(32, 0x71);
    assert_eq!(rt_data(b"sspc", &payload), payload);
}

#[test]
fn footage_height_bytes_roundtrip() {
    let payload = patterned(32, 0x72);
    assert_eq!(rt_data(b"sspc", &payload), payload);
}

#[test]
fn opti_payload_roundtrip() {
    let payload = patterned(40, 0x73);
    assert_eq!(rt_data(b"opti", &payload), payload);
}

#[test]
fn soli_bytes_roundtrip() {
    let mut payload = patterned(40, 0x74);
    payload[..4].copy_from_slice(b"Soli");
    assert_eq!(rt_data(b"opti", &payload), payload);
}

#[test]
fn solid_color_bytes_roundtrip() {
    let mut payload = patterned(40, 0x75);
    payload[..4].copy_from_slice(b"Soli");
    payload[4..20].copy_from_slice(&[0x11; 16]);
    assert_eq!(rt_data(b"opti", &payload), payload);
}

#[test]
fn als2_children_roundtrip() {
    let doc = document(vec![list(
        b"Als2",
        vec![data(b"alas", br#"{"fullpath":"C:\\a.mov"}"#)],
    )]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List { kind, children, .. } => {
            assert_eq!(kind.0, *b"Als2");
            assert_eq!(children.len(), 1);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn alas_payload_roundtrip() {
    let payload = br#"{"fullpath":"C:\\footage\\clip.mov"}"#;
    assert_eq!(rt_data(b"alas", payload), payload);
}

#[test]
fn footage_fullpath_keeps_separators() {
    let payload = br#"{"fullpath":"C:\\footage\\clip.mov"}"#;
    let back = rt_data(b"alas", payload);
    assert_eq!(back, payload);
    assert!(back.windows(2).any(|window| window == br"\\"));
    assert!(!back.windows(1).any(|window| window == b"/"));
}

#[test]
fn proxy_path_stays_separate() {
    let source = br#"{"fullpath":"C:\\footage\\clip.mov"}"#;
    let proxy = br"D:\proxy\clip.mov";
    let doc = document(vec![list(
        b"Item",
        vec![
            data(b"idta", idta_bytes(idta::FOOTAGE, 1, 0)),
            data(b"alas", source),
            data(b"Utf8", proxy),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    let mut alas = None;
    let mut name = None;
    each_data(&back, |tag, bytes| {
        if tag.0 == *b"alas" {
            alas = Some(bytes.to_vec());
        }
        if tag.0 == *b"Utf8" {
            name = Some(bytes.to_vec());
        }
    });
    assert_eq!(alas.as_deref(), Some(source.as_slice()));
    assert_eq!(name.as_deref(), Some(proxy.as_slice()));
    assert_ne!(alas, name);
}

#[test]
fn mrst_children_roundtrip() {
    let doc = document(vec![list(
        b"mrst",
        vec![
            list(b"mrky", vec![list(b"Nmrd", vec![data(b"Utf8", b"hit")])]),
            data(b"tdbs", [1, 2]),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List { kind, children, .. } => {
            assert_eq!(kind.0, *b"mrst");
            assert_eq!(children.len(), 2);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn mrky_nmrd_order_roundtrip() {
    let doc = document(vec![list(
        b"mrky",
        vec![
            list(b"Nmrd", vec![data(b"Utf8", b"first")]),
            list(b"Nmrd", vec![data(b"Utf8", b"second")]),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    let mut names = Vec::new();
    each_data(&back, |tag, bytes| {
        if tag.0 == *b"Utf8" {
            names.push(bytes.to_vec());
        }
    });
    assert_eq!(names, vec![b"first".to_vec(), b"second".to_vec()]);
}

#[test]
fn nmrd_children_roundtrip() {
    let doc = document(vec![list(
        b"Nmrd",
        vec![data(b"Utf8", b"marker"), data(b"NmHd", [4, 5, 6, 7])],
    )]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List { kind, children, .. } => {
            assert_eq!(kind.0, *b"Nmrd");
            assert_eq!(children.len(), 2);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn marker_name_roundtrip() {
    let xml = "<AfterEffectsProject><Nmrd><string>コメント</string></Nmrd></AfterEffectsProject>";
    let doc = read_aepx(xml).unwrap();
    let aep = write_aep(&doc).unwrap();
    let back = read_aep(&aep).unwrap();
    let mut name = None;
    each_data(&back, |tag, bytes| {
        if tag.0 == *b"Utf8" {
            name = Some(bytes.to_vec());
        }
    });
    assert_eq!(name.as_deref(), Some("コメント".as_bytes()));
    assert_eq!(read_aepx(&write_aepx(&back).unwrap()).unwrap(), back);
}

#[test]
fn nmhd_bytes_roundtrip() {
    let payload = patterned(16, 0x80);
    assert_eq!(rt_data(b"NmHd", &payload), payload);
}

#[test]
fn chapter_text_roundtrip() {
    let xml = "<AfterEffectsProject><Nmrd><string>name</string><string>chapter=1</string><string>cue=2</string></Nmrd></AfterEffectsProject>";
    let doc = read_aepx(xml).unwrap();
    let edited = read_aepx(&xml.replace("chapter=1", "chapter=9")).unwrap();
    assert_one_utf8_change(&doc, &edited, b"chapter=9");
    let _ = write_roundtrip(&edited);
}

#[test]
fn cue_text_stays_separate() {
    let xml = "<AfterEffectsProject><Nmrd><string>name</string><string>chapter=1</string><string>cue=2</string></Nmrd></AfterEffectsProject>";
    let doc = read_aepx(xml).unwrap();
    let edited = read_aepx(&xml.replace("cue=2", "cue=8")).unwrap();
    assert_one_utf8_change(&doc, &edited, b"cue=8");
    let mut texts = Vec::new();
    each_data(&edited, |tag, bytes| {
        if tag.0 == *b"Utf8" {
            texts.push(bytes.to_vec());
        }
    });
    assert_eq!(
        texts,
        vec![b"name".to_vec(), b"chapter=1".to_vec(), b"cue=8".to_vec()]
    );
}

#[test]
fn tdgp_order_roundtrip() {
    let doc = document(vec![list(
        b"tdgp",
        vec![
            data(b"tdmn", b"ADBE Position\0"),
            list(b"tdbs", vec![data(b"cdat", 1.0f64.to_be_bytes())]),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List { kind, children, .. } => {
            assert_eq!(kind.0, *b"tdgp");
            match &children[0] {
                Chunk::Data { id, .. } => assert_eq!(id.0, *b"tdmn"),
                other => panic!("{other:?}"),
            }
            match &children[1] {
                Chunk::List { kind, .. } => assert_eq!(kind.0, *b"tdbs"),
                other => panic!("{other:?}"),
            }
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn tdmn_keeps_trailing_nul() {
    let payload = b"ADBE Position\0";
    let back = rt_data(b"tdmn", payload);
    assert_eq!(back, payload);
    assert_eq!(tdmn::as_str(&back), Some("ADBE Position"));
    assert_eq!(tdmn::bytes(&back), b"ADBE Position");
}

#[test]
fn tdsb_payload_roundtrip() {
    let payload = [0x00, 0x00, 0x00, 0x01];
    assert_eq!(rt_data(b"tdsb", &payload), payload);
}

#[test]
fn tdbs_child_order_roundtrip() {
    let doc = document(vec![list(
        b"tdbs",
        vec![
            data(b"tdsb", [1]),
            data(b"tdb4", [2, 2]),
            data(b"cdat", 2.0f64.to_be_bytes()),
            list(b"list", vec![]),
            data(b"Utf8", b"label"),
            data(b"tdpi", [3, 3, 3, 3]),
            data(b"tdps", [4, 4, 4, 4]),
            data(b"tdli", [5, 5, 5, 5]),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List { children, .. } => {
            let tags: Vec<[u8; 4]> = children
                .iter()
                .map(|child| match child {
                    Chunk::Data { id, .. } => id.0,
                    Chunk::List { kind, .. } => kind.0,
                    other => panic!("{other:?}"),
                })
                .collect();
            assert_eq!(
                tags,
                vec![
                    *b"tdsb", *b"tdb4", *b"cdat", *b"list", *b"Utf8", *b"tdpi", *b"tdps", *b"tdli"
                ]
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn tdb4_payload_roundtrip() {
    let payload = patterned(12, 0x90);
    assert_eq!(rt_data(b"tdb4", &payload), payload);
}

#[test]
fn cdat_f64_bytes_roundtrip() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&1.25f64.to_be_bytes());
    payload.extend_from_slice(&(-3.5f64).to_be_bytes());
    let back = rt_data(b"cdat", &payload);
    assert_eq!(back, payload);
    assert_eq!(cdat::values(&back), Some(vec![1.25, -3.5]));
    assert!(cdat::values(&[0, 1, 2, 3]).is_none());
    assert_eq!(rt_data(b"cdat", &[0, 1, 2, 3]), vec![0, 1, 2, 3]);
}

#[test]
fn list_children_roundtrip() {
    let doc = document(vec![list(
        b"list",
        vec![data(b"lhd3", [1, 2, 3, 4]), data(b"ldat", [5, 6, 7, 8])],
    )]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List { kind, children, .. } => {
            assert_eq!(kind.0, *b"list");
            assert_eq!(children.len(), 2);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn lhd3_count_matches_ldat_span() {
    let mut header = patterned(24, 0x40);
    let count = 3u16;
    let size = 8u16;
    header[10..12].copy_from_slice(&count.to_be_bytes());
    header[18..20].copy_from_slice(&size.to_be_bytes());
    let body = patterned(usize::from(count) * usize::from(size), 0x70);
    let doc = document(vec![list(
        b"list",
        vec![data(b"lhd3", header.clone()), data(b"ldat", body.clone())],
    )]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List { children, .. } => {
            let header_back = match &children[0] {
                Chunk::Data { data, .. } => data,
                other => panic!("{other:?}"),
            };
            let ldat = match &children[1] {
                Chunk::Data { data, .. } => data,
                other => panic!("{other:?}"),
            };
            assert_eq!(header_back, &header);
            assert_eq!(lhd3::count(header_back), Some(count));
            assert_eq!(lhd3::element_size(header_back), Some(size));
            assert_eq!(lhd3::used_len(header_back), Some(ldat.len()));
            assert_eq!(ldat, &body);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn ldat_payload_roundtrip() {
    let payload = patterned(24, 0x91);
    assert_eq!(rt_data(b"ldat", &payload), payload);
}

#[test]
fn tdpi_payload_roundtrip() {
    assert_eq!(rt_data(b"tdpi", &[1, 2, 3, 4]), vec![1, 2, 3, 4]);
}

#[test]
fn tdps_payload_roundtrip() {
    assert_eq!(rt_data(b"tdps", &[9, 8, 7, 6]), vec![9, 8, 7, 6]);
}

#[test]
fn tdli_payload_roundtrip() {
    assert_eq!(rt_data(b"tdli", &[4, 3, 2, 1]), vec![4, 3, 2, 1]);
}

#[test]
fn mkif_payload_roundtrip() {
    let payload = patterned(8, 0x92);
    assert_eq!(rt_data(b"mkif", &payload), payload);
}

#[test]
fn om_s_children_bytes_roundtrip() {
    let mut payload = Vec::new();
    push_chunk(&mut payload, b"omks", &[1, 2, 3, 4], 0);
    push_chunk(&mut payload, b"shap", &[5], 0x7E);
    let back = rt_data(b"om-s", &payload);
    assert_eq!(back, payload);
    let omks = back
        .windows(4)
        .position(|window| window == b"omks")
        .unwrap();
    let shap = back
        .windows(4)
        .position(|window| window == b"shap")
        .unwrap();
    assert!(omks < shap);
}

#[test]
fn omks_children_bytes_roundtrip() {
    let mut payload = Vec::new();
    push_chunk(&mut payload, b"shap", &[1, 2], 0);
    assert_eq!(rt_data(b"omks", &payload), payload);
}

#[test]
fn shap_children_roundtrip() {
    let doc = document(vec![list(
        b"shap",
        vec![
            data(b"shph", [1, 2, 3, 4]),
            list(b"list", vec![data(b"ldat", [5])]),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List { kind, children, .. } => {
            assert_eq!(kind.0, *b"shap");
            assert_eq!(children.len(), 2);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn shph_payload_roundtrip() {
    let payload = patterned(20, 0x93);
    assert_eq!(rt_data(b"shph", &payload), payload);
}

#[test]
fn gcst_gradient_xml_roundtrip() {
    let xml = b"<gradient><stop/></gradient>";
    let doc = document(vec![list(
        b"GCst",
        vec![list(b"GCky", vec![data(b"Utf8", xml)])],
    )]);
    let (back, _) = write_roundtrip(&doc);
    let mut found = None;
    each_data(&back, |tag, bytes| {
        if tag.0 == *b"Utf8" {
            found = Some(bytes.to_vec());
        }
    });
    assert_eq!(found.as_deref(), Some(xml.as_slice()));
}

#[test]
fn gcky_utf8_roundtrip() {
    let xml = b"<gradient>1</gradient>";
    let doc = document(vec![list(b"GCky", vec![data(b"Utf8", xml)])]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List { kind, children, .. } => {
            assert_eq!(kind.0, *b"GCky");
            match &children[0] {
                Chunk::Data { data, .. } => assert_eq!(data, xml),
                other => panic!("{other:?}"),
            }
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn btds_child_order_roundtrip() {
    let doc = document(vec![list(
        b"btds",
        vec![
            opaque(b"btdk", b"doc-bytes"),
            list(b"tdbs", vec![data(b"cdat", 0.5f64.to_be_bytes())]),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List { kind, children, .. } => {
            assert_eq!(kind.0, *b"btds");
            assert!(matches!(children[0], Chunk::OpaqueList { .. }));
            assert!(matches!(children[1], Chunk::List { .. }));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn btdk_body_roundtrip() {
    let body = b"Utf8\x00\x00\x00\x04name";
    let doc = document(vec![opaque(b"btdk", body)]);
    let (back, raw) = write_roundtrip(&doc);
    assert_eq!(&raw[12..16], b"LIST");
    assert_eq!(&raw[20..24], b"btdk");
    assert_eq!(&raw[24..24 + body.len()], body);
    match &back.chunks[0] {
        Chunk::OpaqueList {
            kind, body: got, ..
        } => {
            assert_eq!(kind.0, *b"btdk");
            assert_eq!(got, body);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn otst_bytes_roundtrip() {
    let mut payload = Vec::new();
    push_chunk(&mut payload, b"otky", &[1, 2, 3, 4], 0);
    assert_eq!(rt_data(b"otst", &payload), payload);
}

#[test]
fn otky_otda_bytes_roundtrip() {
    let mut payload = Vec::new();
    push_chunk(&mut payload, b"otda", &[0x11; 24], 0);
    let back = rt_data(b"otky", &payload);
    assert_eq!(back, payload);
    assert!(back.windows(4).any(|window| window == b"otda"));
}

#[test]
fn otda_bytes_roundtrip() {
    let payload = patterned(24, 0x94);
    assert_eq!(payload.len(), 24);
    assert_eq!(rt_data(b"otda", &payload), payload);
}

#[test]
fn ovg2_bytes_roundtrip() {
    assert_eq!(rt_data(b"OvG2", &[1, 2, 3, 4, 5]), vec![1, 2, 3, 4, 5]);
}

#[test]
fn blsi_bytes_roundtrip() {
    assert_eq!(rt_data(b"blsi", &[6, 7]), vec![6, 7]);
}

#[test]
fn blsv_bytes_roundtrip() {
    assert_eq!(rt_data(b"blsv", &[8, 9, 10]), vec![8, 9, 10]);
}

#[test]
fn efdg_efdf_order_roundtrip() {
    let doc = document(vec![list(
        b"EfdG",
        vec![
            list(b"EfDf", vec![data(b"tdmn", b"ADBE One\0")]),
            list(b"EfDf", vec![data(b"tdmn", b"ADBE Two\0")]),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    let mut names = Vec::new();
    each_data(&back, |tag, bytes| {
        if tag.0 == *b"tdmn" {
            names.push(bytes.to_vec());
        }
    });
    assert_eq!(names, vec![b"ADBE One\0".to_vec(), b"ADBE Two\0".to_vec()]);
}

#[test]
fn efdf_name_and_part_roundtrip() {
    let doc = document(vec![list(
        b"EfDf",
        vec![
            data(b"tdmn", b"ADBE Blur\0"),
            group(b"fnam", vec![data(b"Utf8", "ブラー".as_bytes())]),
            list(b"parT", vec![data(b"pard", [1, 2, 3, 4])]),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    let mut match_name = None;
    let mut display = None;
    let mut pard = None;
    each_data(&back, |tag, bytes| match &tag.0 {
        b"tdmn" => match_name = Some(bytes.to_vec()),
        b"Utf8" => display = Some(bytes.to_vec()),
        b"pard" => pard = Some(bytes.to_vec()),
        _ => {}
    });
    assert_eq!(match_name.as_deref(), Some(b"ADBE Blur\0".as_slice()));
    assert_eq!(
        tdmn::as_str(match_name.as_deref().unwrap()),
        Some("ADBE Blur")
    );
    assert_eq!(display.as_deref(), Some("ブラー".as_bytes()));
    assert_eq!(pard.as_deref(), Some(&[1, 2, 3, 4][..]));
}

#[test]
fn part_child_order_roundtrip() {
    let doc = document(vec![list(
        b"parT",
        vec![
            data(b"tdmn", b"Amount\0"),
            data(b"pard", [1, 0]),
            data(b"tdmn", b"Color\0"),
            data(b"pard", [2, 0]),
        ],
    )]);
    let (back, _) = write_roundtrip(&doc);
    match &back.chunks[0] {
        Chunk::List { kind, children, .. } => {
            assert_eq!(kind.0, *b"parT");
            assert_eq!(children.len(), 4);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn pard_payload_roundtrip() {
    let payload = patterned(18, 0x95);
    assert_eq!(rt_data(b"pard", &payload), payload);
}

#[test]
fn nums_text_matches_u32() {
    let xml = "<AfterEffectsProject><numS>305419896</numS></AfterEffectsProject>";
    let doc = read_aepx(xml).unwrap();
    match &doc.chunks[0] {
        Chunk::Data { id, data, .. } => {
            assert_eq!(id.0, *b"numS");
            assert_eq!(nums::value(data), Some(0x1234_5678));
            assert_eq!(data, &0x1234_5678u32.to_be_bytes());
        }
        other => panic!("{other:?}"),
    }
    let aep = write_aep(&doc).unwrap();
    let back = read_aep(&aep).unwrap();
    assert_eq!(back, doc);
    let again = read_aepx(&write_aepx(&back).unwrap()).unwrap();
    match &again.chunks[0] {
        Chunk::Data { data, .. } => assert_eq!(nums::value(data), Some(0x1234_5678)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn ppsn_text_matches_f64_bits() {
    for value in [0.1f64, -0.0, 30.0, -2.5, 1.5] {
        let doc = document(vec![data(b"ppSn", value.to_be_bytes())]);
        let xml = write_aepx(&doc).unwrap();
        let back = read_aepx(&xml).unwrap();
        match &back.chunks[0] {
            Chunk::Data { data, .. } => assert_eq!(ppsn::bits(data), Some(value.to_bits())),
            other => panic!("{other:?}"),
        }
        let _ = write_roundtrip(&doc);
    }
}

#[test]
fn xmpm_text_roundtrip() {
    let text = "<?xpacket a & b?>";
    let doc = document(vec![data(b"XMPM", text.as_bytes())]);
    let xml = write_aepx(&doc).unwrap();
    assert!(xml.contains("ProjectXMPMetadata"));
    let back = read_aepx(&xml).unwrap();
    match &back.chunks[0] {
        Chunk::Data { id, data, .. } => {
            assert_eq!(id.0, *b"XMPM");
            assert_eq!(data, text.as_bytes());
        }
        other => panic!("{other:?}"),
    }
    let _ = write_roundtrip(&back);
}

#[test]
fn unknown_chunk_file_roundtrip() {
    let mut body = Vec::new();
    push_chunk(&mut body, b"nhed", &[1, 2], 0);
    push_chunk(&mut body, b"ZzZz", &[3, 4, 5], 0xAB);
    push_chunk(&mut body, b"fdta", &[6], 0xCD);
    let file = rifx(&body);
    let doc = assert_raw_identity(&file);
    assert_eq!(top_level_ids(&file), vec![*b"nhed", *b"ZzZz", *b"fdta"]);
    match &doc.chunks[1] {
        Chunk::Data { id, data, pad } => {
            assert_eq!(id.0, *b"ZzZz");
            assert_eq!(data, &[3, 4, 5]);
            assert_eq!(*pad, Some(0xAB));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn aepx_root_matches_egg_children() {
    let doc = document(vec![
        data(b"nhed", [1, 2, 3, 4]),
        list(b"Fold", vec![data(b"Utf8", b"Root")]),
        data(b"XMPM", b"meta"),
    ]);
    let xml = write_aepx(&doc).unwrap();
    assert!(xml.contains("<AfterEffectsProject>"));
    let from_xml = read_aepx(&xml).unwrap();
    assert_eq!(from_xml, doc);
    let aep = write_aep(&from_xml).unwrap();
    assert_eq!(&aep[8..12], b"Egg!");
    assert_eq!(read_aep(&aep).unwrap(), doc);
    assert_eq!(top_level_ids(&aep), vec![*b"nhed", *b"LIST", *b"XMPM"]);
}

#[test]
fn aepx_string_matches_utf8() {
    let xml = "<AfterEffectsProject><string>コンポ</string></AfterEffectsProject>";
    let doc = read_aepx(xml).unwrap();
    match &doc.chunks[0] {
        Chunk::Data { id, data, .. } => {
            assert_eq!(id.0, *b"Utf8");
            assert_eq!(data, "コンポ".as_bytes());
        }
        other => panic!("{other:?}"),
    }
    let aep = write_aep(&doc).unwrap();
    assert_eq!(read_aep(&aep).unwrap(), doc);
}

#[test]
fn aepx_bdata_matches_payload() {
    let payload = vec![0x0A, 0xFF, 0x10];
    let doc = document(vec![data(b"cdta", payload.clone())]);
    let xml = write_aepx(&doc).unwrap();
    let from_xml = read_aepx(&xml).unwrap();
    match &from_xml.chunks[0] {
        Chunk::Data { data, .. } => assert_eq!(data, &payload),
        other => panic!("{other:?}"),
    }
    let aep = write_aep(&from_xml).unwrap();
    match &read_aep(&aep).unwrap().chunks[0] {
        Chunk::Data { data, .. } => assert_eq!(data, &payload),
        other => panic!("{other:?}"),
    }
    let spaced = r#"<AfterEffectsProject><cdta bdata="0A ff  10"/></AfterEffectsProject>"#;
    assert_eq!(read_aepx(spaced).unwrap(), doc);
}

#[test]
fn footage_without_custom_name_has_no_string() {
    let doc = document(vec![list(
        b"Item",
        vec![data(b"idta", idta_bytes(idta::FOOTAGE, 3, 0))],
    )]);
    let xml = write_aepx(&doc).unwrap();
    assert!(!xml.contains("<string"));
    assert_eq!(read_aepx(&xml).unwrap(), doc);

    let empty = "<AfterEffectsProject><Item><string></string></Item></AfterEffectsProject>";
    let read = read_aepx(empty).unwrap();
    match &read.chunks[0] {
        Chunk::List { children, .. } => match &children[0] {
            Chunk::Data { data, .. } => {
                assert!(data.is_empty());
                assert_ne!(data, b"clip.mov");
            }
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
}

#[test]
fn workspace_text_roundtrip() {
    let xml = "<AfterEffectsProject><Item><string>Comp</string></Item><ViewName>Custom Workspace</ViewName></AfterEffectsProject>";
    let doc = read_aepx(xml).unwrap();
    let again = read_aepx(&write_aepx(&doc).unwrap()).unwrap();
    assert_eq!(again, doc);
    match &doc.chunks[0] {
        Chunk::List { children, .. } => match &children[0] {
            Chunk::Data { data, .. } => assert_eq!(data, b"Comp"),
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
    match &doc.chunks[1] {
        Chunk::Xml { name, text, .. } => {
            assert_eq!(name, "ViewName");
            assert_eq!(text, "Custom Workspace");
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(write_aep(&doc), Err(Error::UnmappedXml(_))));
}

#[test]
fn fake_list_is_not_wrapped_in_list() {
    for id in [b"tdsn", b"fnam", b"pdnm"] {
        let doc = document(vec![group(id, vec![data(b"Utf8", b"Name")])]);
        let (_, bytes) = write_roundtrip(&doc);
        assert_eq!(&bytes[12..16], id);
        assert_ne!(&bytes[12..16], b"LIST");
        let xml = write_aepx(&doc).unwrap();
        assert!(!xml.contains("<LIST"));
        assert_eq!(read_aepx(&xml).unwrap(), doc);
    }
}

#[test]
fn aet_uses_the_rifx_reader() {
    let mut body = Vec::new();
    push_chunk(&mut body, b"nhed", &[1, 2, 3, 4], 0);
    push_chunk(&mut body, b"Utf8", b"Template", 0);
    let file = rifx(&body);
    let from_aet = read_aet(&file).unwrap();
    let from_aep = read_aep(&file).unwrap();
    assert_eq!(from_aet, from_aep);
    assert_eq!(write_aet(&from_aet).unwrap(), file);
    assert_eq!(&file[0..4], b"RIFX");
    assert_eq!(&file[8..12], b"Egg!");
}
