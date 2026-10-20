use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use crate::error::MediaError;

const HEADER_BYTES: usize = 2048;
const DATA_OFFSET: u32 = 2048;
const DPX_MAGIC: u32 = 0x5344_5058;
const CINEON_MAGIC: u32 = 0x802A_5FD7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilmContainer {
    Dpx,
    Cineon45,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilmScan {
    pub container: FilmContainer,
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub bits: u16,
    pub codes: Vec<u16>,
}

pub fn write_film_scan(path: &Path, scan: &FilmScan) -> Result<(), MediaError> {
    let mut header = vec![0u8; HEADER_BYTES];
    let magic = match scan.container {
        FilmContainer::Dpx => DPX_MAGIC,
        FilmContainer::Cineon45 => CINEON_MAGIC,
    };
    write_u32(&mut header, 0, magic);
    write_u32(&mut header, 4, DATA_OFFSET);
    write_u32(&mut header, 8, scan.width);
    write_u32(&mut header, 12, scan.height);
    header[16] = scan.channels;
    header[17] = scan.bits.clamp(0, 255) as u8;
    header[18] = if scan.channels >= 4 { 51 } else { 50 };
    if matches!(scan.container, FilmContainer::Dpx) {
        write_u32(&mut header, 772, scan.width);
        write_u32(&mut header, 776, scan.height);
        header[800] = if scan.channels >= 4 { 51 } else { 50 };
        header[803] = scan.bits.clamp(0, 255) as u8;
    }
    let mut file = File::create(path)?;
    file.write_all(&header)?;
    for code in &scan.codes {
        if scan.bits <= 8 {
            file.write_all(&[*code as u8])?;
        } else {
            file.write_all(&code.to_be_bytes())?;
        }
    }
    Ok(())
}

pub fn read_film_scan(path: &Path) -> Result<FilmScan, MediaError> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    if bytes.len() < HEADER_BYTES {
        return Err(MediaError::Decode("DPX ヘッダが短い".into()));
    }
    let magic = read_u32(&bytes, 0);
    let container = match magic {
        DPX_MAGIC => FilmContainer::Dpx,
        CINEON_MAGIC => FilmContainer::Cineon45,
        _ => return Err(MediaError::Decode("DPX / Cineon のマジックが違う".into())),
    };
    let offset = read_u32(&bytes, 4) as usize;
    let width = read_u32(&bytes, 8);
    let height = read_u32(&bytes, 12);
    let channels = bytes[16];
    let bits = u16::from(bytes[17]);
    let count = width as usize * height as usize * channels as usize;
    let mut codes = Vec::with_capacity(count);
    let mut cursor = offset;
    for _ in 0..count {
        if bits <= 8 {
            let value = *bytes.get(cursor).ok_or_else(|| MediaError::Decode("DPX が途中で終わった".into()))?;
            codes.push(u16::from(value));
            cursor += 1;
        } else {
            if cursor + 2 > bytes.len() {
                return Err(MediaError::Decode("DPX が途中で終わった".into()));
            }
            codes.push(u16::from_be_bytes([bytes[cursor], bytes[cursor + 1]]));
            cursor += 2;
        }
    }
    Ok(FilmScan {
        container,
        width,
        height,
        channels,
        bits,
        codes,
    })
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    #[test]
    fn dpx_and_cineon_roundtrip_codes() {
        let dir = temp_dir("film");
        let dpx_path = dir.join("frame.dpx");
        let cin_path = dir.join("frame.cin");
        let dpx = FilmScan {
            container: FilmContainer::Dpx,
            width: 2,
            height: 1,
            channels: 4,
            bits: 16,
            codes: vec![1, 2, 3, 4, 5, 6, 7, 8],
        };
        let mut cin = dpx.clone();
        cin.container = FilmContainer::Cineon45;
        cin.bits = 10;
        cin.channels = 3;
        cin.codes = vec![95, 400, 685];
        cin.width = 1;
        cin.height = 1;
        write_film_scan(&dpx_path, &dpx).unwrap();
        write_film_scan(&cin_path, &cin).unwrap();
        let read_dpx = read_film_scan(&dpx_path).unwrap();
        let read_cin = read_film_scan(&cin_path).unwrap();
        assert_eq!(read_dpx.width, 2);
        assert_eq!(read_dpx.channels, 4);
        assert_eq!(read_dpx.codes, dpx.codes);
        assert_eq!(read_cin.container, FilmContainer::Cineon45);
        assert_eq!(read_cin.bits, 10);
        assert_eq!(read_cin.codes, vec![95, 400, 685]);
    }
}
