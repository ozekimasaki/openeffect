use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use image::codecs::gif::GifDecoder;
use exr::prelude::{ReadChannels, ReadLayers};
use image::{AnimationDecoder, DynamicImage, ImageFormat, ImageReader};
use model::BitsPerChannel;

use crate::catalog::{primary_kind, MediaKind};
use crate::dpx::{read_film_scan, FilmContainer};
use crate::error::MediaError;
use crate::interpret::AlphaMode;
use crate::raster::Raster;

#[derive(Clone, Debug, PartialEq)]
pub struct DecodedStill {
    pub raster: Raster,
    pub frame_count: u32,
    pub labeled_alpha: Option<AlphaMode>,
    pub extra_channels: Vec<String>,
}

pub fn decode_still(path: &Path) -> Result<DecodedStill, MediaError> {
    let kind = primary_kind(path).ok_or_else(|| MediaError::Invalid("拡張子を判定できない".into()))?;
    match kind {
        MediaKind::Png | MediaKind::Jpeg | MediaKind::Bmp | MediaKind::Targa => decode_image_still(path, kind),
        MediaKind::Tiff => decode_tiff(path),
        MediaKind::Gif => decode_gif(path),
        MediaKind::Radiance => decode_hdr(path),
        MediaKind::OpenExr => decode_exr(path),
        MediaKind::Psd => decode_psd(path),
        MediaKind::Svg => decode_svg(path),
        MediaKind::Dpx | MediaKind::Cineon => decode_film(path),
        MediaKind::Sgi
        | MediaKind::MayaIff
        | MediaKind::SoftimagePic
        | MediaKind::Pcx
        | MediaKind::Heif
        | MediaKind::CameraRaw
        | MediaKind::Pict
        | MediaKind::ElectricImage
        | MediaKind::QuickTime
        | MediaKind::Avi
        | MediaKind::Mpeg4
        | MediaKind::Hevc
        | MediaKind::Mpeg1
        | MediaKind::Mpeg2
        | MediaKind::Mxf
        | MediaKind::WindowsMedia
        | MediaKind::FlashVideo
        | MediaKind::ThreeGpp
        | MediaKind::Avchd
        | MediaKind::RedR3d
        | MediaKind::CanonCrm
        | MediaKind::Swf
        | MediaKind::Omf
        | MediaKind::Wav
        | MediaKind::Aiff
        | MediaKind::Mp3
        | MediaKind::Aac
        | MediaKind::Asnd
        | MediaKind::Project => Err(MediaError::Unsupported {
            kind: format!("{kind:?}"),
            detail: "純 Rust では画素を読まない",
        }),
    }
}

fn decode_image_still(path: &Path, kind: MediaKind) -> Result<DecodedStill, MediaError> {
    let format = image_format(kind);
    let file = File::open(path)?;
    let reader = match format {
        Some(format) => ImageReader::with_format(BufReader::new(file), format),
        None => ImageReader::open(path)?.with_guessed_format()?,
    };
    let image = reader.decode()?;
    Ok(from_dynamic(image, kind))
}

fn image_format(kind: MediaKind) -> Option<ImageFormat> {
    match kind {
        MediaKind::Png => Some(ImageFormat::Png),
        MediaKind::Jpeg => Some(ImageFormat::Jpeg),
        MediaKind::Bmp => Some(ImageFormat::Bmp),
        MediaKind::Targa => Some(ImageFormat::Tga),
        MediaKind::Tiff => Some(ImageFormat::Tiff),
        MediaKind::Radiance => Some(ImageFormat::Hdr),
        MediaKind::Gif
        | MediaKind::OpenExr
        | MediaKind::Psd
        | MediaKind::Svg
        | MediaKind::Dpx
        | MediaKind::Cineon
        | MediaKind::Sgi
        | MediaKind::MayaIff
        | MediaKind::SoftimagePic
        | MediaKind::Pcx
        | MediaKind::Heif
        | MediaKind::CameraRaw
        | MediaKind::Pict
        | MediaKind::ElectricImage
        | MediaKind::QuickTime
        | MediaKind::Avi
        | MediaKind::Mpeg4
        | MediaKind::Hevc
        | MediaKind::Mpeg1
        | MediaKind::Mpeg2
        | MediaKind::Mxf
        | MediaKind::WindowsMedia
        | MediaKind::FlashVideo
        | MediaKind::ThreeGpp
        | MediaKind::Avchd
        | MediaKind::RedR3d
        | MediaKind::CanonCrm
        | MediaKind::Swf
        | MediaKind::Omf
        | MediaKind::Wav
        | MediaKind::Aiff
        | MediaKind::Mp3
        | MediaKind::Aac
        | MediaKind::Asnd
        | MediaKind::Project => None,
    }
}

fn from_dynamic(image: DynamicImage, kind: MediaKind) -> DecodedStill {
    let (bits, channels, samples) = match &image {
        DynamicImage::ImageLuma8(buffer) => expand_gray(buffer.as_raw(), 1.0 / 255.0, false, BitsPerChannel::Bpc8),
        DynamicImage::ImageLumaA8(buffer) => expand_gray(buffer.as_raw(), 1.0 / 255.0, true, BitsPerChannel::Bpc8),
        DynamicImage::ImageRgb8(buffer) => expand_rgb(buffer.as_raw(), 1.0 / 255.0, 3, BitsPerChannel::Bpc8),
        DynamicImage::ImageRgba8(buffer) => expand_rgb(buffer.as_raw(), 1.0 / 255.0, 4, BitsPerChannel::Bpc8),
        DynamicImage::ImageLuma16(buffer) => {
            let raw: Vec<f32> = buffer.as_raw().iter().map(|v| f32::from(*v) / 65535.0).collect();
            (BitsPerChannel::Bpc16, 3, repeat_gray(&raw, false))
        }
        DynamicImage::ImageLumaA16(buffer) => {
            let raw: Vec<f32> = buffer.as_raw().iter().map(|v| f32::from(*v) / 65535.0).collect();
            (BitsPerChannel::Bpc16, 4, repeat_gray(&raw, true))
        }
        DynamicImage::ImageRgb16(buffer) => {
            let raw: Vec<f32> = buffer.as_raw().iter().map(|v| f32::from(*v) / 65535.0).collect();
            (BitsPerChannel::Bpc16, 3, raw)
        }
        DynamicImage::ImageRgba16(buffer) => {
            let raw: Vec<f32> = buffer.as_raw().iter().map(|v| f32::from(*v) / 65535.0).collect();
            (BitsPerChannel::Bpc16, 4, raw)
        }
        DynamicImage::ImageRgb32F(buffer) => (BitsPerChannel::Bpc32, 3, buffer.as_raw().clone()),
        DynamicImage::ImageRgba32F(buffer) => (BitsPerChannel::Bpc32, 4, buffer.as_raw().clone()),
        other => {
            let rgba = other.to_rgba8();
            expand_rgb(rgba.as_raw(), 1.0 / 255.0, 4, BitsPerChannel::Bpc8)
        }
    };
    let labeled_alpha = if kind == MediaKind::Png && channels == 4 {
        Some(AlphaMode::Straight)
    } else {
        None
    };
    DecodedStill {
        raster: Raster::new(image.width(), image.height(), channels, bits, samples),
        frame_count: 1,
        labeled_alpha,
        extra_channels: Vec::new(),
    }
}

fn expand_gray(raw: &[u8], scale: f32, alpha: bool, bits: BitsPerChannel) -> (BitsPerChannel, u8, Vec<f32>) {
    let step = if alpha { 2 } else { 1 };
    let mut samples = Vec::with_capacity(raw.len() / step * 4);
    for pixel in raw.chunks(step) {
        let value = f32::from(pixel[0]) * scale;
        let alpha = if alpha { f32::from(pixel[1]) * scale } else { 1.0 };
        samples.extend_from_slice(&[value, value, value, alpha]);
    }
    (bits, 4, samples)
}

fn expand_rgb(raw: &[u8], scale: f32, channels: u8, bits: BitsPerChannel) -> (BitsPerChannel, u8, Vec<f32>) {
    let mut samples = Vec::with_capacity(raw.len());
    for value in raw {
        samples.push(f32::from(*value) * scale);
    }
    (bits, channels, samples)
}

fn repeat_gray(raw: &[f32], alpha: bool) -> Vec<f32> {
    let step = if alpha { 2 } else { 1 };
    let mut samples = Vec::with_capacity(raw.len() / step * if alpha { 4 } else { 3 });
    for pixel in raw.chunks(step) {
        samples.extend_from_slice(&[pixel[0], pixel[0], pixel[0]]);
        if alpha {
            samples.push(pixel[1]);
        }
    }
    samples
}

fn decode_gif(path: &Path) -> Result<DecodedStill, MediaError> {
    let file = BufReader::new(File::open(path)?);
    let decoder = GifDecoder::new(file)?;
    let frames = decoder.into_frames().collect_frames()?;
    let frame_count = frames.len().max(1) as u32;
    let first = frames
        .first()
        .ok_or_else(|| MediaError::Decode("GIF にフレームが無い".into()))?;
    let buffer = first.buffer();
    let still = from_dynamic(DynamicImage::ImageRgba8(buffer.clone()), MediaKind::Gif);
    Ok(DecodedStill {
        frame_count,
        labeled_alpha: None,
        ..still
    })
}

fn decode_hdr(path: &Path) -> Result<DecodedStill, MediaError> {
    let file = File::open(path)?;
    let image = ImageReader::with_format(BufReader::new(file), ImageFormat::Hdr).decode()?;
    Ok(from_dynamic(image, MediaKind::Radiance))
}

fn decode_tiff(path: &Path) -> Result<DecodedStill, MediaError> {
    let file = File::open(path)?;
    let mut decoder = tiff::decoder::Decoder::new(file)?;
    let (width, height) = decoder.dimensions()?;
    let color = decoder.colortype()?;
    let (channels, bits, gray) = match color {
        tiff::ColorType::RGB(8) => (3, BitsPerChannel::Bpc8, false),
        tiff::ColorType::RGBA(8) => (4, BitsPerChannel::Bpc8, false),
        tiff::ColorType::RGB(16) => (3, BitsPerChannel::Bpc16, false),
        tiff::ColorType::RGBA(16) => (4, BitsPerChannel::Bpc16, false),
        tiff::ColorType::RGB(32) => (3, BitsPerChannel::Bpc32, false),
        tiff::ColorType::RGBA(32) => (4, BitsPerChannel::Bpc32, false),
        tiff::ColorType::Gray(8) => (3, BitsPerChannel::Bpc8, true),
        tiff::ColorType::Gray(16) => (3, BitsPerChannel::Bpc16, true),
        tiff::ColorType::Gray(32) => (3, BitsPerChannel::Bpc32, true),
        other => {
            return Err(MediaError::Unsupported {
                kind: format!("{other:?}"),
                detail: "この TIFF の深度は読まない",
            })
        }
    };
    let decoded = decoder.read_image()?;
    let samples = tiff_samples(decoded, channels, bits, gray)?;
    Ok(DecodedStill {
        raster: Raster::new(width, height, channels, bits, samples),
        frame_count: 1,
        labeled_alpha: None,
        extra_channels: Vec::new(),
    })
}

fn tiff_samples(
    decoded: tiff::decoder::DecodingResult,
    channels: u8,
    bits: BitsPerChannel,
    gray: bool,
) -> Result<Vec<f32>, MediaError> {
    let scale = match bits {
        BitsPerChannel::Bpc8 => 255.0,
        BitsPerChannel::Bpc16 => 65535.0,
        BitsPerChannel::Bpc32 => 1.0,
    };
    let raw: Vec<f32> = match decoded {
        tiff::decoder::DecodingResult::U8(values) => values.into_iter().map(|v| f32::from(v) / scale).collect(),
        tiff::decoder::DecodingResult::U16(values) => values.into_iter().map(|v| f32::from(v) / scale).collect(),
        tiff::decoder::DecodingResult::F32(values) => values,
        _ => return Err(MediaError::Decode("TIFF のサンプル型を扱えない".into())),
    };
    if gray {
        let mut samples = Vec::with_capacity(raw.len() * 3);
        for value in raw {
            samples.extend_from_slice(&[value, value, value]);
        }
        return Ok(samples);
    }
    let _ = channels;
    Ok(raw)
}

fn decode_exr(path: &Path) -> Result<DecodedStill, MediaError> {
    let image = exr::prelude::read()
        .no_deep_data()
        .largest_resolution_level()
        .all_channels()
        .first_valid_layer()
        .all_attributes()
        .from_file(path)?;
    let size = image.layer_data.size;
    let names: Vec<String> = image
        .layer_data
        .channel_data
        .list
        .iter()
        .map(|channel| channel.name.to_string())
        .collect();
    let mut samples = Vec::with_capacity(size.x() * size.y() * 4);
    let has_alpha = names.iter().any(|name| name == "A");
    let channels = if has_alpha { 4 } else { 3 };
    for y in 0..size.y() {
        for x in 0..size.x() {
            let pixel = image.layer_data.sample_vec_at(exr::prelude::Vec2(x, y));
            let value = |name: &str| -> f32 {
                names
                    .iter()
                    .position(|channel| channel == name)
                    .map(|index| sample_to_f32(pixel[index]))
                    .unwrap_or(0.0)
            };
            samples.push(value("R"));
            samples.push(value("G"));
            samples.push(value("B"));
            if has_alpha {
                samples.push(value("A"));
            }
        }
    }
    let extra = names
        .into_iter()
        .filter(|name| !matches!(name.as_str(), "R" | "G" | "B" | "A"))
        .collect();
    Ok(DecodedStill {
        raster: Raster::new(size.x() as u32, size.y() as u32, channels, BitsPerChannel::Bpc32, samples),
        frame_count: 1,
        labeled_alpha: None,
        extra_channels: extra,
    })
}

fn sample_to_f32(sample: exr::prelude::Sample) -> f32 {
    match sample {
        exr::prelude::Sample::F16(value) => value.to_f32(),
        exr::prelude::Sample::F32(value) => value,
        exr::prelude::Sample::U32(value) => value as f32,
    }
}

fn decode_psd(path: &Path) -> Result<DecodedStill, MediaError> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < 26 || &bytes[0..4] != b"8BPS" {
        return Err(MediaError::Decode("PSD ではない".into()));
    }
    let height = u32::from_be_bytes(bytes[14..18].try_into().unwrap());
    let width = u32::from_be_bytes(bytes[18..22].try_into().unwrap());
    let depth = u16::from_be_bytes(bytes[22..24].try_into().unwrap());
    let mode = u16::from_be_bytes(bytes[24..26].try_into().unwrap());
    let channels = u16::from_be_bytes(bytes[12..14].try_into().unwrap());
    if depth != 8 || mode != 3 {
        return Err(MediaError::Unsupported {
            kind: "psd".into(),
            detail: "統合画像は 8 bpc RGB だけを読む",
        });
    }
    let mut cursor = 26;
    for _ in 0..3 {
        if cursor + 4 > bytes.len() {
            return Err(MediaError::Decode("PSD のセクションが短い".into()));
        }
        let length = u32::from_be_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4 + length;
    }
    if cursor >= bytes.len() {
        return Err(MediaError::Decode("PSD に画像が無い".into()));
    }
    let compression = u16::from_be_bytes(bytes[cursor..cursor + 2].try_into().unwrap());
    cursor += 2;
    if compression != 0 {
        return Err(MediaError::Unsupported {
            kind: "psd".into(),
            detail: "非圧縮の統合画像だけを読む",
        });
    }
    let plane = width as usize * height as usize;
    let needed = plane * channels as usize;
    if cursor + needed > bytes.len() {
        return Err(MediaError::Decode("PSD の画素が足りない".into()));
    }
    let stored = channels.min(4) as u8;
    let mut samples = vec![0.0; plane * stored as usize];
    for channel in 0..stored as usize {
        for pixel in 0..plane {
            let value = f32::from(bytes[cursor + channel * plane + pixel]) / 255.0;
            samples[pixel * stored as usize + channel] = value;
        }
    }
    let bits = BitsPerChannel::Bpc8;
    Ok(DecodedStill {
        raster: Raster::new(width, height, stored, bits, samples),
        frame_count: 1,
        labeled_alpha: if stored == 4 { Some(AlphaMode::Straight) } else { None },
        extra_channels: Vec::new(),
    })
}

pub fn write_merged_psd(path: &Path, raster: &Raster) -> Result<(), MediaError> {
    let channels: u16 = if raster.has_alpha() { 4 } else { 3 };
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"8BPS");
    bytes.extend_from_slice(&1u16.to_be_bytes());
    bytes.extend_from_slice(&[0; 6]);
    bytes.extend_from_slice(&channels.to_be_bytes());
    bytes.extend_from_slice(&raster.height.to_be_bytes());
    bytes.extend_from_slice(&raster.width.to_be_bytes());
    bytes.extend_from_slice(&8u16.to_be_bytes());
    bytes.extend_from_slice(&3u16.to_be_bytes());
    bytes.extend_from_slice(&0u32.to_be_bytes());
    bytes.extend_from_slice(&0u32.to_be_bytes());
    bytes.extend_from_slice(&0u32.to_be_bytes());
    bytes.extend_from_slice(&0u16.to_be_bytes());
    let plane = raster.width as usize * raster.height as usize;
    for channel in 0..channels as usize {
        for pixel in 0..plane {
            let sample = raster.samples()[pixel * raster.channels as usize + channel];
            bytes.push((sample.clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

fn decode_svg(path: &Path) -> Result<DecodedStill, MediaError> {
    let text = std::fs::read_to_string(path)?;
    let (width, height) = svg_size(&text).ok_or_else(|| MediaError::Decode("SVG の寸法が無い".into()))?;
    Ok(DecodedStill {
        raster: Raster::solid(width, height, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 0.0]),
        frame_count: 1,
        labeled_alpha: Some(AlphaMode::Straight),
        extra_channels: Vec::new(),
    })
}

fn svg_size(text: &str) -> Option<(u32, u32)> {
    let width = attr_number(text, "width");
    let height = attr_number(text, "height");
    if let (Some(width), Some(height)) = (width, height) {
        return Some((width.ceil().max(1.0) as u32, height.ceil().max(1.0) as u32));
    }
    let view_box = attr(text, "viewBox")?;
    let mut parts = view_box.split_whitespace();
    let _min_x = parts.next()?;
    let _min_y = parts.next()?;
    let width = parts.next()?.parse::<f64>().ok()?;
    let height = parts.next()?.parse::<f64>().ok()?;
    Some((width.ceil().max(1.0) as u32, height.ceil().max(1.0) as u32))
}

fn attr<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let pattern = format!("{name}=\"");
    let start = text.find(&pattern)? + pattern.len();
    let end = text[start..].find('"')? + start;
    Some(&text[start..end])
}

fn attr_number(text: &str, name: &str) -> Option<f64> {
    let raw = attr(text, name)?;
    let number: String = raw
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '.' || *ch == '-')
        .collect();
    number.parse().ok()
}

fn decode_film(path: &Path) -> Result<DecodedStill, MediaError> {
    let scan = read_film_scan(path)?;
    let bits = match scan.bits {
        0..=8 => BitsPerChannel::Bpc8,
        9..=16 => BitsPerChannel::Bpc16,
        _ => BitsPerChannel::Bpc32,
    };
    let scale = if scan.bits <= 8 {
        255.0
    } else if scan.bits <= 10 {
        1023.0
    } else {
        65535.0
    };
    let samples: Vec<f32> = scan.codes.iter().map(|code| f32::from(*code) / scale).collect();
    let channels = if scan.channels >= 4 { 4 } else { 3 };
    let _ = FilmContainer::Dpx;
    Ok(DecodedStill {
        raster: Raster::new(scan.width, scan.height, channels, bits, samples),
        frame_count: 1,
        labeled_alpha: if channels == 4 { Some(AlphaMode::Straight) } else { None },
        extra_channels: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    #[test]
    fn merged_psd_reports_dimensions_and_alpha() {
        let dir = temp_dir("psd");
        let path = dir.join("flat.psd");
        let raster = Raster::solid(3, 2, BitsPerChannel::Bpc8, [0.2, 0.4, 0.6, 0.5]);
        write_merged_psd(&path, &raster).unwrap();
        let decoded = decode_still(&path).unwrap();
        assert_eq!((decoded.raster.width, decoded.raster.height), (3, 2));
        assert!(decoded.raster.has_alpha());
    }

    #[test]
    fn svg_reports_dimensions() {
        let dir = temp_dir("svg");
        let path = dir.join("mark.svg");
        std::fs::write(&path, r#"<svg width="120" height="40" xmlns="http://www.w3.org/2000/svg"></svg>"#).unwrap();
        let decoded = decode_still(&path).unwrap();
        assert_eq!((decoded.raster.width, decoded.raster.height), (120, 40));
        assert!(decoded.raster.has_alpha());
    }
}
