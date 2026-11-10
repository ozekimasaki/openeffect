//! 二つの調査で一致しているフィールドだけ。
//!
//! 要確認のオフセットは定数にも関数にもしない。呼び出し側はペイロード全体を比較する。

pub mod idta {
    /// アイテム種別。ペイロード先頭の uint16 ビッグエンディアン。
    pub const KIND: usize = 0;
    /// アイテム ID。ペイロード先頭から 16 バイト。
    pub const ITEM_ID: usize = 16;

    pub const FOLDER: u16 = 1;
    pub const COMPOSITION: u16 = 4;
    pub const FOOTAGE: u16 = 7;

    pub fn kind(data: &[u8]) -> Option<u16> {
        super::u16_be(data, KIND)
    }

    pub fn item_id(data: &[u8]) -> Option<u32> {
        super::u32_be(data, ITEM_ID)
    }
}

pub mod cdta {
    pub const BACKGROUND: usize = 52;
    pub const WIDTH: usize = 140;
    pub const HEIGHT: usize = 142;
    pub const FRAME_RATE: usize = 156;

    pub fn width(data: &[u8]) -> Option<u16> {
        super::u16_be(data, WIDTH)
    }

    pub fn height(data: &[u8]) -> Option<u16> {
        super::u16_be(data, HEIGHT)
    }

    pub fn frame_rate(data: &[u8]) -> Option<u16> {
        super::u16_be(data, FRAME_RATE)
    }

    pub fn background(data: &[u8]) -> Option<[u8; 3]> {
        let bytes = data.get(BACKGROUND..BACKGROUND + 3)?;
        Some([bytes[0], bytes[1], bytes[2]])
    }
}

pub mod ldta {
    pub const QUALITY: usize = 4;
    pub const SOURCE_ID: usize = 40;

    pub const WIREFRAME: u16 = 0;
    pub const DRAFT: u16 = 1;
    pub const BEST: u16 = 2;

    pub const BICUBIC: (usize, u8) = (37, 6);
    pub const GUIDE: (usize, u8) = (37, 1);
    pub const SOLO: (usize, u8) = (38, 3);
    pub const THREE_D: (usize, u8) = (38, 2);
    pub const ADJUSTMENT: (usize, u8) = (38, 1);
    pub const COLLAPSE: (usize, u8) = (39, 7);
    pub const SHY: (usize, u8) = (39, 6);
    pub const LOCKED: (usize, u8) = (39, 5);
    pub const MOTION_BLUR: (usize, u8) = (39, 3);
    pub const EFFECTS: (usize, u8) = (39, 2);
    pub const VIDEO: (usize, u8) = (39, 0);

    pub fn quality(data: &[u8]) -> Option<u16> {
        super::u16_be(data, QUALITY)
    }

    pub fn source_id(data: &[u8]) -> Option<u32> {
        super::u32_be(data, SOURCE_ID)
    }

    pub fn bit(data: &[u8], offset: usize, bit: u8) -> Option<bool> {
        if bit > 7 {
            return None;
        }
        Some(data.get(offset)? & (1 << bit) != 0)
    }
}

pub mod lhd3 {
    pub const COUNT: usize = 10;
    pub const ELEMENT_SIZE: usize = 18;

    pub fn count(data: &[u8]) -> Option<u16> {
        super::u16_be(data, COUNT)
    }

    pub fn element_size(data: &[u8]) -> Option<u16> {
        super::u16_be(data, ELEMENT_SIZE)
    }

    /// 個数 × 要素サイズ。`ldat` の使用範囲と突き合わせる。
    pub fn used_len(data: &[u8]) -> Option<usize> {
        let count = usize::from(count(data)?);
        let size = usize::from(element_size(data)?);
        Some(count * size)
    }
}

pub mod cdat {
    /// ペイロード全体を float64 ビッグエンディアンの列として読む。
    /// 成分数の取得元は未確定なので、`tdb4` は見ない。長さが 8 の倍数でないときは `None`。
    pub fn values(data: &[u8]) -> Option<Vec<f64>> {
        if data.len() % 8 != 0 {
            return None;
        }
        Some(
            data.chunks_exact(8)
                .map(|chunk| {
                    let mut bytes = [0; 8];
                    bytes.copy_from_slice(chunk);
                    f64::from_be_bytes(bytes)
                })
                .collect(),
        )
    }
}

pub mod utf8 {
    /// ペイロード全体。末尾 NUL は削らない。
    pub fn as_str(data: &[u8]) -> Option<&str> {
        std::str::from_utf8(data).ok()
    }
}

pub mod tdmn {
    /// 読み取り時だけ末尾の NUL を 1 つ外す。元のバイト列はチャンク側に残る。
    pub fn bytes(data: &[u8]) -> &[u8] {
        match data.split_last() {
            Some((0, head)) => head,
            _ => data,
        }
    }

    pub fn as_str(data: &[u8]) -> Option<&str> {
        std::str::from_utf8(bytes(data)).ok()
    }
}

pub mod nums {
    pub fn value(data: &[u8]) -> Option<u32> {
        let bytes: [u8; 4] = data.try_into().ok()?;
        Some(u32::from_be_bytes(bytes))
    }
}

pub mod ppsn {
    pub fn value(data: &[u8]) -> Option<f64> {
        let bytes: [u8; 8] = data.try_into().ok()?;
        Some(f64::from_be_bytes(bytes))
    }

    pub fn bits(data: &[u8]) -> Option<u64> {
        value(data).map(f64::to_bits)
    }
}

pub(crate) fn u16_be(data: &[u8], offset: usize) -> Option<u16> {
    let end = offset.checked_add(2)?;
    let bytes = data.get(offset..end)?;
    Some(u16::from_be_bytes([bytes[0], bytes[1]]))
}

pub(crate) fn u32_be(data: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let bytes = data.get(offset..end)?;
    Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

#[cfg(test)]
mod tests {
    use super::{cdat, cdta, idta, ldta, lhd3, nums, tdmn};

    #[test]
    fn short_payloads_are_absent() {
        assert!(idta::kind(&[0x00]).is_none());
        assert!(idta::item_id(&[0; 16]).is_none());
        assert!(cdta::width(&[0; 140]).is_none());
        assert!(cdta::background(&[0; 52]).is_none());
        assert!(ldta::bit(&[0; 37], 37, 6).is_none());
        assert!(ldta::bit(&[0xFF], 0, 8).is_none());
        assert!(lhd3::count(&[0; 10]).is_none());
        assert!(cdat::values(&[0; 4]).is_none());
        assert!(nums::value(&[0; 3]).is_none());
    }

    #[test]
    fn match_name_strips_one_trailing_nul() {
        assert_eq!(tdmn::bytes(b"ADBE\0"), b"ADBE");
        assert_eq!(tdmn::bytes(b"ADBE\0\0"), b"ADBE\0");
        assert_eq!(tdmn::bytes(b"ADBE"), b"ADBE");
    }
}
