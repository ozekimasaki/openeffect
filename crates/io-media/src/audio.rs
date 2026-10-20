use std::fs::File;
use std::io::Write;
use std::path::Path;

use model::Time;

use crate::catalog::{primary_kind, MediaKind};
use crate::error::MediaError;

/// インターリーブされた PCM。8 bit も符号付きへ寄せて持つ。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioClip {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits: u16,
    pub samples: Vec<i16>,
}

impl AudioClip {
    pub fn frame_count(&self) -> u64 {
        if self.channels == 0 {
            0
        } else {
            self.samples.len() as u64 / u64::from(self.channels)
        }
    }

    pub fn duration(&self) -> Time {
        if self.sample_rate == 0 {
            Time::ZERO
        } else {
            Time::from_seconds(self.frame_count() as f64 / f64::from(self.sample_rate))
        }
    }
}

pub fn read_audio(path: &Path) -> Result<AudioClip, MediaError> {
    match primary_kind(path) {
        Some(MediaKind::Wav) => read_wav(path),
        Some(MediaKind::Aiff) => read_aiff(path),
        Some(kind) => Err(MediaError::Unsupported {
            kind: format!("{kind:?}"),
            detail: "この音声は純 Rust では読まない",
        }),
        None => Err(MediaError::Invalid("音声の拡張子ではない".into())),
    }
}

pub fn write_wav(path: &Path, clip: &AudioClip) -> Result<(), MediaError> {
    let bits = if clip.bits <= 8 { 8 } else { 16 };
    let channels = clip.channels.max(1);
    let block = u16::from(channels) * (bits / 8);
    let data_bytes = clip.frame_count() as u32 * u32::from(block);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&clip.sample_rate.to_le_bytes());
    let byte_rate = clip.sample_rate * u32::from(block);
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&block.to_le_bytes());
    bytes.extend_from_slice(&bits.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());
    for sample in &clip.samples {
        if bits == 8 {
            bytes.push((*sample as i32 / 256 + 128).clamp(0, 255) as u8);
        } else {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
    }
    File::create(path)?.write_all(&bytes)?;
    Ok(())
}

pub fn write_aiff(path: &Path, clip: &AudioClip) -> Result<(), MediaError> {
    let channels = clip.channels.max(1);
    let frames = clip.frame_count() as u32;
    let mut comm = Vec::new();
    comm.extend_from_slice(&channels.to_be_bytes());
    comm.extend_from_slice(&frames.to_be_bytes());
    comm.extend_from_slice(&16u16.to_be_bytes());
    comm.extend_from_slice(&extended_80(clip.sample_rate));
    let ssnd_data = clip.samples.len() * 2;
    let ssnd_size = 8 + ssnd_data;
    let form_size = 4 + 8 + comm.len() + 8 + ssnd_size;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"FORM");
    bytes.extend_from_slice(&(form_size as u32).to_be_bytes());
    bytes.extend_from_slice(b"AIFF");
    bytes.extend_from_slice(b"COMM");
    bytes.extend_from_slice(&(comm.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&comm);
    bytes.extend_from_slice(b"SSND");
    bytes.extend_from_slice(&(ssnd_size as u32).to_be_bytes());
    bytes.extend_from_slice(&0u32.to_be_bytes());
    bytes.extend_from_slice(&0u32.to_be_bytes());
    for sample in &clip.samples {
        bytes.extend_from_slice(&sample.to_be_bytes());
    }
    File::create(path)?.write_all(&bytes)?;
    Ok(())
}

fn read_wav(path: &Path) -> Result<AudioClip, MediaError> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(MediaError::Decode("WAV ではない".into()));
    }
    let mut cursor = 12;
    let mut channels = 0u16;
    let mut rate = 0u32;
    let mut bits = 16u16;
    let mut data = None;
    while cursor + 8 <= bytes.len() {
        let id = &bytes[cursor..cursor + 4];
        let size = u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
        let start = cursor + 8;
        let end = start + size;
        if end > bytes.len() {
            break;
        }
        if id == b"fmt " && size >= 16 {
            channels = u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap());
            rate = u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap());
            bits = u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap());
        } else if id == b"data" {
            data = Some(&bytes[start..end]);
        }
        cursor = end + (size % 2);
    }
    let data = data.ok_or_else(|| MediaError::Decode("WAV に data が無い".into()))?;
    let samples = if bits <= 8 {
        data.iter().map(|sample| (i16::from(*sample) - 128) * 256).collect()
    } else {
        data.chunks_exact(2)
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
            .collect()
    };
    Ok(AudioClip {
        sample_rate: rate,
        channels,
        bits,
        samples,
    })
}

fn read_aiff(path: &Path) -> Result<AudioClip, MediaError> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < 12 || &bytes[0..4] != b"FORM" {
        return Err(MediaError::Decode("AIFF ではない".into()));
    }
    let mut cursor = 12;
    let mut channels = 0u16;
    let mut frames = 0u32;
    let mut bits = 16u16;
    let mut rate = 0u32;
    let mut samples = Vec::new();
    while cursor + 8 <= bytes.len() {
        let id = &bytes[cursor..cursor + 4];
        let size = u32::from_be_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
        let start = cursor + 8;
        let end = (start + size).min(bytes.len());
        if id == b"COMM" && end >= start + 18 {
            channels = u16::from_be_bytes(bytes[start..start + 2].try_into().unwrap());
            frames = u32::from_be_bytes(bytes[start + 2..start + 6].try_into().unwrap());
            bits = u16::from_be_bytes(bytes[start + 6..start + 8].try_into().unwrap());
            rate = extended_80_to_rate(&bytes[start + 8..start + 18]);
        } else if id == b"SSND" && end >= start + 8 {
            samples = bytes[start + 8..end]
                .chunks_exact(2)
                .map(|pair| i16::from_be_bytes([pair[0], pair[1]]))
                .collect();
        }
        cursor = end + (size % 2);
    }
    if samples.is_empty() && frames > 0 {
        return Err(MediaError::Decode("AIFF にサンプルが無い".into()));
    }
    let _ = frames;
    Ok(AudioClip {
        sample_rate: rate,
        channels,
        bits,
        samples,
    })
}

fn extended_80(rate: u32) -> [u8; 10] {
    if rate == 0 {
        return [0; 10];
    }
    let mut mantissa = u64::from(rate);
    let mut exponent = 16383 + 63;
    while (mantissa & (1u64 << 63)) == 0 {
        mantissa <<= 1;
        exponent -= 1;
    }
    let mut out = [0u8; 10];
    out[0] = (exponent >> 8) as u8;
    out[1] = (exponent & 0xff) as u8;
    out[2..10].copy_from_slice(&mantissa.to_be_bytes());
    out
}

fn extended_80_to_rate(bytes: &[u8]) -> u32 {
    let exponent = (u32::from(bytes[0] & 0x7f) << 8) | u32::from(bytes[1]);
    let mantissa = u64::from_be_bytes(bytes[2..10].try_into().unwrap());
    if mantissa == 0 {
        return 0;
    }
    let unbiased = exponent as i32 - 16383;
    let value = if unbiased >= 63 {
        mantissa << (unbiased - 63)
    } else {
        mantissa >> (63 - unbiased)
    };
    value as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    #[test]
    fn wav_reports_duration_and_channel_count() {
        let dir = temp_dir("wav");
        let wav_path = dir.join("tone.wav");
        let bwf_path = dir.join("tone.bwf");
        let clip = AudioClip {
            sample_rate: 48_000,
            channels: 2,
            bits: 16,
            samples: vec![0; 48_000 * 2],
        };
        write_wav(&wav_path, &clip).unwrap();
        std::fs::copy(&wav_path, &bwf_path).unwrap();
        let wav = read_audio(&wav_path).unwrap();
        let bwf = read_audio(&bwf_path).unwrap();
        assert_eq!(wav.channels, 2);
        assert!((wav.duration().seconds() - 1.0).abs() < 1e-6);
        assert_eq!(bwf.channels, 2);
        assert!((bwf.duration().seconds() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn aiff_reports_duration() {
        let dir = temp_dir("aiff");
        let aif = dir.join("tone.aif");
        let aiff = dir.join("tone.aiff");
        let clip = AudioClip {
            sample_rate: 48_000,
            channels: 1,
            bits: 16,
            samples: vec![0; 24_000],
        };
        write_aiff(&aif, &clip).unwrap();
        std::fs::copy(&aif, &aiff).unwrap();
        assert!((read_audio(&aif).unwrap().duration().seconds() - 0.5).abs() < 1e-4);
        assert!((read_audio(&aiff).unwrap().duration().seconds() - 0.5).abs() < 1e-4);
    }

    #[test]
    fn stereo_sixteen_bit_wav_roundtrip() {
        let dir = temp_dir("wav16");
        let path = dir.join("mix.wav");
        let clip = AudioClip {
            sample_rate: 48_000,
            channels: 2,
            bits: 16,
            samples: vec![1000, -1000, 2000, -2000],
        };
        write_wav(&path, &clip).unwrap();
        let read = read_audio(&path).unwrap();
        assert_eq!(read.bits, 16);
        assert_eq!(read.channels, 2);
        assert_eq!(read.samples, clip.samples);
    }
}
