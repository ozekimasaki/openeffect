use std::fs::File;
use std::path::Path;

use exr::image::pixel_vec::PixelVec;
use exr::prelude::{Encoding, Image, SpecificChannels, Vec2, WritableImage};
use image::codecs::hdr::HdrEncoder;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder, ImageFormat, Rgb};
use model::BitsPerChannel;

use crate::catalog::{extension_of, format_info, primary_kind, MediaKind, PureRust};
use crate::error::MediaError;
use crate::raster::Raster;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PngCompression {
    Fast,
    Best,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TiffCompression {
    None,
    Lzw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExrCompression {
    None,
    Zip,
}

#[derive(Clone, Debug)]
pub struct EncodeOptions {
    pub bits: BitsPerChannel,
    pub include_alpha: bool,
    pub straight: bool,
    pub png_compression: PngCompression,
    pub jpeg_quality: u8,
    pub tiff_compression: TiffCompression,
    pub exr_compression: ExrCompression,
    pub extra_channel: Option<Vec<f32>>,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            bits: BitsPerChannel::Bpc8,
            include_alpha: true,
            straight: true,
            png_compression: PngCompression::Fast,
            jpeg_quality: 90,
            tiff_compression: TiffCompression::None,
            exr_compression: ExrCompression::Zip,
            extra_channel: None,
        }
    }
}

pub fn encode_still(path: &Path, raster: &Raster, options: &EncodeOptions) -> Result<(), MediaError> {
    let kind = primary_kind(path).ok_or_else(|| MediaError::Invalid("拡張子を判定できない".into()))?;
    let info = format_info(kind);
    if !info.export || !matches!(info.pure_rust, PureRust::ReadWrite) {
        return Err(MediaError::Unsupported {
            kind: format!("{kind:?}"),
            detail: "この形式は純 Rust では書き出さない",
        });
    }
    match kind {
        MediaKind::Png => write_png(path, raster, options),
        MediaKind::Jpeg => write_jpeg(path, raster, options),
        MediaKind::Tiff => write_tiff(path, raster, options),
        MediaKind::Targa => write_tga(path, raster, options),
        MediaKind::Radiance => write_hdr(path, raster),
        MediaKind::OpenExr => write_exr(path, raster, options),
        MediaKind::Dpx | MediaKind::Cineon => write_film(path, raster, options),
        MediaKind::Bmp
        | MediaKind::Gif
        | MediaKind::Psd
        | MediaKind::Sgi
        | MediaKind::MayaIff
        | MediaKind::SoftimagePic
        | MediaKind::Pcx
        | MediaKind::Heif
        | MediaKind::CameraRaw
        | MediaKind::Pict
        | MediaKind::ElectricImage
        | MediaKind::Svg
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
            detail: "書き出しに対応していない",
        }),
    }
}

fn prepared(raster: &Raster, options: &EncodeOptions) -> Vec<[f32; 4]> {
    let mut pixels = Vec::with_capacity(raster.width as usize * raster.height as usize);
    for y in 0..raster.height {
        for x in 0..raster.width {
            let mut pixel = raster.get(x, y);
            if !options.include_alpha {
                pixel[3] = 1.0;
            } else if !options.straight {
                pixel[0] *= pixel[3];
                pixel[1] *= pixel[3];
                pixel[2] *= pixel[3];
            }
            pixels.push(pixel);
        }
    }
    pixels
}

fn write_png(path: &Path, raster: &Raster, options: &EncodeOptions) -> Result<(), MediaError> {
    let pixels = prepared(raster, options);
    let file = File::create(path)?;
    let compression = match options.png_compression {
        PngCompression::Fast => CompressionType::Fast,
        PngCompression::Best => CompressionType::Best,
    };
    let encoder = PngEncoder::new_with_quality(file, compression, FilterType::Adaptive);
    let include_alpha = options.include_alpha && raster.has_alpha();
    if options.bits == BitsPerChannel::Bpc16 {
        let mut raw = Vec::new();
        for pixel in &pixels {
            let channels = if include_alpha { 4 } else { 3 };
            for channel in 0..channels {
                let value = (pixel[channel].clamp(0.0, 1.0) * 65535.0).round() as u16;
                raw.extend_from_slice(&value.to_ne_bytes());
            }
        }
        let color = if include_alpha {
            ExtendedColorType::Rgba16
        } else {
            ExtendedColorType::Rgb16
        };
        encoder.write_image(&raw, raster.width, raster.height, color)?;
    } else {
        let mut raw = Vec::new();
        for pixel in &pixels {
            let channels = if include_alpha { 4 } else { 3 };
            for channel in 0..channels {
                raw.push((pixel[channel].clamp(0.0, 1.0) * 255.0).round() as u8);
            }
        }
        let color = if include_alpha {
            ExtendedColorType::Rgba8
        } else {
            ExtendedColorType::Rgb8
        };
        encoder.write_image(&raw, raster.width, raster.height, color)?;
    }
    Ok(())
}

fn write_jpeg(path: &Path, raster: &Raster, options: &EncodeOptions) -> Result<(), MediaError> {
    let pixels = prepared(raster, options);
    let mut raw = Vec::with_capacity(pixels.len() * 3);
    for pixel in pixels {
        for channel in 0..3 {
            raw.push((pixel[channel].clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }
    let file = File::create(path)?;
    let mut encoder = JpegEncoder::new_with_quality(file, options.jpeg_quality.clamp(1, 100));
    encoder.encode(&raw, raster.width, raster.height, ExtendedColorType::Rgb8)?;
    Ok(())
}

fn write_tiff(path: &Path, raster: &Raster, options: &EncodeOptions) -> Result<(), MediaError> {
    let pixels = prepared(raster, options);
    let include_alpha = options.include_alpha && raster.has_alpha();
    let file = File::create(path)?;
    let mut encoder = tiff::encoder::TiffEncoder::new(file)?;
    match (options.bits, include_alpha, options.tiff_compression) {
        (BitsPerChannel::Bpc32, true, TiffCompression::None) => {
            let data = float_samples(&pixels, 4);
            encoder.write_image_with_compression::<tiff::encoder::colortype::RGBA32Float, _>(
                raster.width,
                raster.height,
                tiff::encoder::compression::Uncompressed,
                &data,
            )?;
        }
        (BitsPerChannel::Bpc32, false, TiffCompression::None) => {
            let data = float_samples(&pixels, 3);
            encoder.write_image_with_compression::<tiff::encoder::colortype::RGB32Float, _>(
                raster.width,
                raster.height,
                tiff::encoder::compression::Uncompressed,
                &data,
            )?;
        }
        (BitsPerChannel::Bpc32, true, TiffCompression::Lzw) => {
            let data = float_samples(&pixels, 4);
            encoder.write_image_with_compression::<tiff::encoder::colortype::RGBA32Float, _>(
                raster.width,
                raster.height,
                tiff::encoder::compression::Lzw,
                &data,
            )?;
        }
        (BitsPerChannel::Bpc32, false, TiffCompression::Lzw) => {
            let data = float_samples(&pixels, 3);
            encoder.write_image_with_compression::<tiff::encoder::colortype::RGB32Float, _>(
                raster.width,
                raster.height,
                tiff::encoder::compression::Lzw,
                &data,
            )?;
        }
        (BitsPerChannel::Bpc16, true, TiffCompression::None) => {
            let data = u16_samples(&pixels, 4);
            encoder.write_image_with_compression::<tiff::encoder::colortype::RGBA16, _>(
                raster.width,
                raster.height,
                tiff::encoder::compression::Uncompressed,
                &data,
            )?;
        }
        (BitsPerChannel::Bpc16, false, TiffCompression::None) => {
            let data = u16_samples(&pixels, 3);
            encoder.write_image_with_compression::<tiff::encoder::colortype::RGB16, _>(
                raster.width,
                raster.height,
                tiff::encoder::compression::Uncompressed,
                &data,
            )?;
        }
        (BitsPerChannel::Bpc16, true, TiffCompression::Lzw) => {
            let data = u16_samples(&pixels, 4);
            encoder.write_image_with_compression::<tiff::encoder::colortype::RGBA16, _>(
                raster.width,
                raster.height,
                tiff::encoder::compression::Lzw,
                &data,
            )?;
        }
        (BitsPerChannel::Bpc16, false, TiffCompression::Lzw) => {
            let data = u16_samples(&pixels, 3);
            encoder.write_image_with_compression::<tiff::encoder::colortype::RGB16, _>(
                raster.width,
                raster.height,
                tiff::encoder::compression::Lzw,
                &data,
            )?;
        }
        (_, true, TiffCompression::None) => {
            let data = u8_samples(&pixels, 4);
            encoder.write_image_with_compression::<tiff::encoder::colortype::RGBA8, _>(
                raster.width,
                raster.height,
                tiff::encoder::compression::Uncompressed,
                &data,
            )?;
        }
        (_, false, TiffCompression::None) => {
            let data = u8_samples(&pixels, 3);
            encoder.write_image_with_compression::<tiff::encoder::colortype::RGB8, _>(
                raster.width,
                raster.height,
                tiff::encoder::compression::Uncompressed,
                &data,
            )?;
        }
        (_, true, TiffCompression::Lzw) => {
            let data = u8_samples(&pixels, 4);
            encoder.write_image_with_compression::<tiff::encoder::colortype::RGBA8, _>(
                raster.width,
                raster.height,
                tiff::encoder::compression::Lzw,
                &data,
            )?;
        }
        (_, false, TiffCompression::Lzw) => {
            let data = u8_samples(&pixels, 3);
            encoder.write_image_with_compression::<tiff::encoder::colortype::RGB8, _>(
                raster.width,
                raster.height,
                tiff::encoder::compression::Lzw,
                &data,
            )?;
        }
    }
    Ok(())
}

fn u8_samples(pixels: &[[f32; 4]], channels: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(pixels.len() * channels);
    for pixel in pixels {
        for channel in 0..channels {
            data.push((pixel[channel].clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }
    data
}

fn u16_samples(pixels: &[[f32; 4]], channels: usize) -> Vec<u16> {
    let mut data = Vec::with_capacity(pixels.len() * channels);
    for pixel in pixels {
        for channel in 0..channels {
            data.push((pixel[channel].clamp(0.0, 1.0) * 65535.0).round() as u16);
        }
    }
    data
}

fn float_samples(pixels: &[[f32; 4]], channels: usize) -> Vec<f32> {
    let mut data = Vec::with_capacity(pixels.len() * channels);
    for pixel in pixels {
        for channel in 0..channels {
            data.push(pixel[channel]);
        }
    }
    data
}

fn write_tga(path: &Path, raster: &Raster, options: &EncodeOptions) -> Result<(), MediaError> {
    let pixels = prepared(raster, options);
    let mut raw = Vec::with_capacity(pixels.len() * 4);
    for pixel in pixels {
        for channel in 0..4 {
            raw.push((pixel[channel].clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }
    let image = image::RgbaImage::from_raw(raster.width, raster.height, raw)
        .ok_or_else(|| MediaError::Encode("TGA の寸法が合わない".into()))?;
    image
        .save_with_format(path, ImageFormat::Tga)
        .map_err(|error| MediaError::Encode(error.to_string()))?;
    Ok(())
}

fn write_hdr(path: &Path, raster: &Raster) -> Result<(), MediaError> {
    let mut pixels = Vec::new();
    for y in 0..raster.height {
        for x in 0..raster.width {
            let pixel = raster.get(x, y);
            pixels.push(Rgb([pixel[0], pixel[1], pixel[2]]));
        }
    }
    let file = File::create(path)?;
    HdrEncoder::new(file).encode(&pixels, raster.width as usize, raster.height as usize)?;
    Ok(())
}

fn write_exr(path: &Path, raster: &Raster, options: &EncodeOptions) -> Result<(), MediaError> {
    let encoding = match options.exr_compression {
        ExrCompression::None => Encoding::UNCOMPRESSED,
        ExrCompression::Zip => Encoding::SMALL_LOSSLESS,
    };
    let width = raster.width as usize;
    let height = raster.height as usize;
    if let Some(extra) = &options.extra_channel {
        let mut pixels = Vec::with_capacity(width * height);
        for y in 0..raster.height {
            for x in 0..raster.width {
                let pixel = raster.get(x, y);
                let index = y as usize * width + x as usize;
                let z = extra.get(index).copied().unwrap_or(0.0);
                pixels.push((pixel[0], pixel[1], pixel[2], pixel[3], z));
            }
        }
        let channels = SpecificChannels::build()
            .with_channel("R")
            .with_channel("G")
            .with_channel("B")
            .with_channel("A")
            .with_channel("Z")
            .with_pixels(PixelVec::new((width, height), pixels));
        let mut image = Image::from_encoded_channels((width, height), encoding, channels);
        image.layer_data.encoding = encoding;
        image.write().to_file(path).map_err(MediaError::from)?;
    } else if options.include_alpha {
        let channels = SpecificChannels::rgba(|Vec2(x, y)| {
            let pixel = raster.get(x as u32, y as u32);
            (pixel[0], pixel[1], pixel[2], pixel[3])
        });
        let image = Image::from_encoded_channels((width, height), encoding, channels);
        image.write().to_file(path).map_err(MediaError::from)?;
    } else {
        let channels = SpecificChannels::rgb(|Vec2(x, y)| {
            let pixel = raster.get(x as u32, y as u32);
            (pixel[0], pixel[1], pixel[2])
        });
        let image = Image::from_encoded_channels((width, height), encoding, channels);
        image.write().to_file(path).map_err(MediaError::from)?;
    }
    Ok(())
}

fn write_film(path: &Path, raster: &Raster, options: &EncodeOptions) -> Result<(), MediaError> {
    use crate::dpx::{write_film_scan, FilmContainer, FilmScan};
    let extension = extension_of(path).unwrap_or_default();
    let container = if extension == "cin" {
        FilmContainer::Cineon45
    } else {
        FilmContainer::Dpx
    };
    let bits = if container == FilmContainer::Cineon45 { 10 } else { 16 };
    let scale = if bits == 10 { 1023.0 } else { 65535.0 };
    let channels: u8 = if options.include_alpha && raster.has_alpha() { 4 } else { 3 };
    let mut codes = Vec::new();
    for y in 0..raster.height {
        for x in 0..raster.width {
            let pixel = raster.get(x, y);
            for channel in 0..usize::from(channels) {
                codes.push((pixel[channel].clamp(0.0, 1.0) * scale).round() as u16);
            }
        }
    }
    write_film_scan(
        path,
        &FilmScan {
            container,
            width: raster.width,
            height: raster.height,
            channels,
            bits,
            codes,
        },
    )
}

pub fn quantize(value: f32, bits: BitsPerChannel) -> f32 {
    match bits {
        BitsPerChannel::Bpc8 => (value.clamp(0.0, 1.0) * 255.0).round() / 255.0,
        BitsPerChannel::Bpc16 => (value.clamp(0.0, 1.0) * 65535.0).round() / 65535.0,
        BitsPerChannel::Bpc32 => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_still;
    use crate::test_support::temp_dir;
    fn pattern() -> Raster {
        let mut raster = Raster::solid(8, 8, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
        for y in 0..8 {
            for x in 0..8 {
                let n = (x * 17 + y * 29) as f32 / 255.0;
                raster.set(x, y, [n, 1.0 - n, (x as f32) / 7.0, if x == 0 { 0.5 } else { 1.0 }]);
            }
        }
        raster
    }

    fn assert_size(path: &Path, width: u32, height: u32) {
        let decoded = decode_still(path).unwrap();
        assert_eq!((decoded.raster.width, decoded.raster.height), (width, height));
    }

    #[test]
    fn png_roundtrip_reports_dimensions_and_alpha() {
        let dir = temp_dir("png");
        let path = dir.join("still.png");
        let raster = pattern();
        encode_still(&path, &raster, &EncodeOptions::default()).unwrap();
        let decoded = decode_still(&path).unwrap();
        assert_eq!((decoded.raster.width, decoded.raster.height), (8, 8));
        assert!(decoded.raster.has_alpha());
        assert_eq!(decoded.labeled_alpha, Some(crate::interpret::AlphaMode::Straight));
        let edge = decoded.raster.get(0, 0);
        assert!((edge[0] - quantize(raster.get(0, 0)[0], BitsPerChannel::Bpc8)).abs() < 0.01);
        assert!((edge[3] - 0.5).abs() < 0.02);
    }

    #[test]
    fn png_sixteen_bit_roundtrip_keeps_depth() {
        let dir = temp_dir("png16");
        let path = dir.join("deep.png");
        let mut options = EncodeOptions::default();
        options.bits = BitsPerChannel::Bpc16;
        encode_still(&path, &pattern(), &options).unwrap();
        let decoded = decode_still(&path).unwrap();
        assert_eq!(decoded.raster.bits, BitsPerChannel::Bpc16);
    }

    #[test]
    fn png_compression_changes_file_size() {
        let dir = temp_dir("pngc");
        let noisy = pattern();
        let fast_path = dir.join("fast.png");
        let best_path = dir.join("best.png");
        let mut fast = EncodeOptions::default();
        fast.png_compression = PngCompression::Fast;
        fast.include_alpha = false;
        let mut best = fast.clone();
        best.png_compression = PngCompression::Best;
        encode_still(&fast_path, &noisy, &fast).unwrap();
        encode_still(&best_path, &noisy, &best).unwrap();
        let fast_len = std::fs::metadata(&fast_path).unwrap().len();
        let best_len = std::fs::metadata(&best_path).unwrap().len();
        assert_ne!(fast_len, best_len);
    }

    #[test]
    fn jpeg_extensions_roundtrip_dimensions() {
        let dir = temp_dir("jpeg");
        for name in ["still.jpg", "still.jpe"] {
            let path = dir.join(name);
            encode_still(&path, &pattern(), &EncodeOptions::default()).unwrap();
            let decoded = decode_still(&path).unwrap();
            assert_eq!((decoded.raster.width, decoded.raster.height), (8, 8));
            assert!(!decoded.raster.has_alpha());
        }
    }

    #[test]
    fn jpeg_quality_changes_file_size() {
        let dir = temp_dir("jpegq");
        let low = dir.join("low.jpg");
        let high = dir.join("high.jpg");
        let mut poor = EncodeOptions::default();
        poor.jpeg_quality = 10;
        let mut fine = poor.clone();
        fine.jpeg_quality = 95;
        encode_still(&low, &pattern(), &poor).unwrap();
        encode_still(&high, &pattern(), &fine).unwrap();
        assert!(std::fs::metadata(&low).unwrap().len() < std::fs::metadata(&high).unwrap().len());
    }

    #[test]
    fn tiff_extensions_and_depths_roundtrip() {
        let dir = temp_dir("tiff");
        let raster = pattern();
        for name in ["still.tif", "still.tiff"] {
            let path = dir.join(name);
            encode_still(&path, &raster, &EncodeOptions::default()).unwrap();
            assert_size(&path, 8, 8);
        }
        let mut bits16 = EncodeOptions::default();
        bits16.bits = BitsPerChannel::Bpc16;
        bits16.straight = true;
        let path16 = dir.join("deep.tif");
        encode_still(&path16, &raster, &bits16).unwrap();
        let decoded = decode_still(&path16).unwrap();
        assert_eq!(decoded.raster.bits, BitsPerChannel::Bpc16);
        assert!((decoded.raster.get(1, 1)[0] - raster.get(1, 1)[0]).abs() < 0.01);

        let mut bits32 = EncodeOptions::default();
        bits32.bits = BitsPerChannel::Bpc32;
        let path32 = dir.join("float.tif");
        encode_still(&path32, &raster, &bits32).unwrap();
        assert_eq!(decode_still(&path32).unwrap().raster.bits, BitsPerChannel::Bpc32);
    }

    #[test]
    fn tiff_compression_changes_file_size() {
        let dir = temp_dir("tiffc");
        let plain = dir.join("plain.tif");
        let packed = dir.join("lzw.tif");
        let mut none = EncodeOptions::default();
        none.tiff_compression = TiffCompression::None;
        let mut lzw = none.clone();
        lzw.tiff_compression = TiffCompression::Lzw;
        encode_still(&plain, &pattern(), &none).unwrap();
        encode_still(&packed, &pattern(), &lzw).unwrap();
        assert_ne!(
            std::fs::metadata(&plain).unwrap().len(),
            std::fs::metadata(&packed).unwrap().len()
        );
    }

    #[test]
    fn targa_family_roundtrip_dimensions() {
        let dir = temp_dir("tga");
        let source = dir.join("still.tga");
        encode_still(&source, &pattern(), &EncodeOptions::default()).unwrap();
        for name in ["still.vda", "still.icb", "still.vst"] {
            let path = dir.join(name);
            std::fs::copy(&source, &path).unwrap();
            assert_size(&path, 8, 8);
        }
    }

    #[test]
    fn radiance_family_roundtrip_dimensions() {
        let dir = temp_dir("hdr");
        let source = dir.join("still.hdr");
        let mut raster = pattern();
        raster.bits = BitsPerChannel::Bpc32;
        encode_still(&source, &raster, &EncodeOptions::default()).unwrap();
        for name in ["still.rgbe", "still.xyze"] {
            let path = dir.join(name);
            std::fs::copy(&source, &path).unwrap();
            let decoded = decode_still(&path).unwrap();
            assert_eq!((decoded.raster.width, decoded.raster.height), (8, 8));
            assert_eq!(decoded.raster.bits, BitsPerChannel::Bpc32);
        }
    }

    #[test]
    fn openexr_family_is_thirty_two_bpc() {
        let dir = temp_dir("exr");
        let path = dir.join("still.exr");
        let mut options = EncodeOptions::default();
        options.bits = BitsPerChannel::Bpc8;
        options.extra_channel = Some(vec![0.25; 64]);
        let mut raster = pattern();
        raster.bits = BitsPerChannel::Bpc32;
        encode_still(&path, &raster, &options).unwrap();
        let decoded = decode_still(&path).unwrap();
        assert_eq!(decoded.raster.bits, BitsPerChannel::Bpc32);
        assert_eq!((decoded.raster.width, decoded.raster.height), (8, 8));
        assert!(decoded.extra_channels.iter().any(|name| name == "Z"));
        for name in ["still.sxr", "still.mxr"] {
            let alias = dir.join(name);
            std::fs::copy(&path, &alias).unwrap();
            assert_size(&alias, 8, 8);
        }
    }

    #[test]
    fn exr_compression_changes_file_size() {
        let dir = temp_dir("exrc");
        let plain = dir.join("plain.exr");
        let zipped = dir.join("zip.exr");
        let mut raster = Raster::solid(32, 32, BitsPerChannel::Bpc32, [0.2, 0.4, 0.6, 1.0]);
        for y in 0..32 {
            for x in 0..32 {
                let n = ((x * 13 + y * 7) % 17) as f32 / 17.0;
                raster.set(x, y, [n, n * 0.5, 1.0 - n, 1.0]);
            }
        }
        let mut none = EncodeOptions::default();
        none.exr_compression = ExrCompression::None;
        let mut zip = none.clone();
        zip.exr_compression = ExrCompression::Zip;
        encode_still(&plain, &raster, &none).unwrap();
        encode_still(&zipped, &raster, &zip).unwrap();
        assert_ne!(
            std::fs::metadata(&plain).unwrap().len(),
            std::fs::metadata(&zipped).unwrap().len()
        );
    }

    #[test]
    fn bmp_extensions_report_dimensions() {
        let dir = temp_dir("bmp");
        let raster = pattern();
        let mut raw = Vec::new();
        for y in 0..raster.height {
            for x in 0..raster.width {
                let pixel = raster.get(x, y);
                raw.push((pixel[2] * 255.0) as u8);
                raw.push((pixel[1] * 255.0) as u8);
                raw.push((pixel[0] * 255.0) as u8);
            }
        }
        let image = image::RgbImage::from_raw(8, 8, raw).unwrap();
        let bmp = dir.join("still.bmp");
        image.save_with_format(&bmp, ImageFormat::Bmp).unwrap();
        for name in ["still.rle", "still.dib"] {
            std::fs::copy(&bmp, dir.join(name)).unwrap();
        }
        for name in ["still.bmp", "still.rle", "still.dib"] {
            assert_size(&dir.join(name), 8, 8);
        }
    }

    #[test]
    fn static_and_animated_gif_report_frames() {
        let dir = temp_dir("gif");
        let still = dir.join("still.gif");
        let rgba = image::RgbaImage::from_pixel(4, 3, image::Rgba([10, 20, 30, 255]));
        rgba.save_with_format(&still, ImageFormat::Gif).unwrap();
        let decoded = decode_still(&still).unwrap();
        assert_eq!((decoded.raster.width, decoded.raster.height), (4, 3));
        assert_eq!(decoded.frame_count, 1);

        let animated = dir.join("anim.gif");
        let file = File::create(&animated).unwrap();
        let mut encoder = image::codecs::gif::GifEncoder::new(file);
        let frame = image::Frame::from_parts(
            rgba,
            0,
            0,
            image::Delay::from_numer_denom_ms(100, 1),
        );
        encoder.encode_frames(vec![frame.clone(), frame]).unwrap();
        drop(encoder);
        assert_eq!(decode_still(&animated).unwrap().frame_count, 2);
    }

    #[test]
    fn film_scan_roundtrip_keeps_alpha_dimensions() {
        let dir = temp_dir("filmio");
        let dpx = dir.join("scan.dpx");
        let cin = dir.join("scan.cin");
        encode_still(&dpx, &pattern(), &EncodeOptions::default()).unwrap();
        encode_still(&cin, &pattern(), &EncodeOptions::default()).unwrap();
        let dpx_image = decode_still(&dpx).unwrap();
        let cin_image = decode_still(&cin).unwrap();
        assert!(dpx_image.raster.has_alpha());
        assert_eq!((dpx_image.raster.width, dpx_image.raster.height), (8, 8));
        assert_eq!(cin_image.frame_count, 1);
    }
}
