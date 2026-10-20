use effect_api::Frame32;
use eval::BlendSample;
use model::{BitsPerChannel, Rgba, TrackMatteMode, Transform3d};

use crate::blend::{composite_pixel, luminance};
use crate::resolution::{CompResolution, ResolutionError};

/// タイムライン順。`layers[0]` が手前。
#[derive(Clone, Copy, Debug)]
pub struct LayerPlate<'a> {
    pub pixels: &'a Frame32,
    pub transform: Transform3d,
    pub blend: BlendSample,
    pub matte_mode: TrackMatteMode,
    pub matte_pixels: Option<&'a Frame32>,
    pub matte_transform: Transform3d,
    pub preserve_underlying_transparency: bool,
    pub three_d: bool,
    pub frame_index: u64,
    pub visible: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct CompositeRequest<'a> {
    pub width: u32,
    pub height: u32,
    pub bits: BitsPerChannel,
    pub background: Rgba,
    pub layers: &'a [LayerPlate<'a>],
}

impl<'a> LayerPlate<'a> {
    pub fn new(pixels: &'a Frame32) -> Self {
        Self {
            pixels,
            transform: Transform3d::identity(),
            blend: BlendSample::default(),
            matte_mode: TrackMatteMode::None,
            matte_pixels: None,
            matte_transform: Transform3d::identity(),
            preserve_underlying_transparency: false,
            three_d: false,
            frame_index: 0,
            visible: true,
        }
    }
}

pub fn composite(request: &CompositeRequest<'_>) -> Frame32 {
    composite_sized(request, request.width, request.height)
}

/// 書き出し。プレビューの解像度やズームは見ない。
pub fn composite_export(request: &CompositeRequest<'_>) -> Frame32 {
    composite(request)
}

pub fn preview_composite(
    request: &CompositeRequest<'_>,
    resolution: &CompResolution,
    view_zooms: &[f64],
) -> Result<Frame32, ResolutionError> {
    let (width, height) = resolution.buffer_size(request.width, request.height, view_zooms)?;
    Ok(composite_sized(request, width, height))
}

fn composite_sized(
    request: &CompositeRequest<'_>,
    buffer_width: u32,
    buffer_height: u32,
) -> Frame32 {
    let mut frame = Frame32::new(buffer_width, buffer_height, 4, 0.0);
    fill(&mut frame, request.background);
    if buffer_width == 0 || buffer_height == 0 {
        return frame;
    }
    for layer in request.layers.iter().rev() {
        if !layer.visible {
            continue;
        }
        paint_layer(&mut frame, request, layer);
    }
    frame
}

fn paint_layer(frame: &mut Frame32, request: &CompositeRequest<'_>, layer: &LayerPlate<'_>) {
    for y in 0..frame.height {
        for x in 0..frame.width {
            let comp_x = buffer_to_comp(x, frame.width, request.width);
            let comp_y = buffer_to_comp(y, frame.height, request.height);
            let mut src = sample_transformed(layer.pixels, &layer.transform, comp_x, comp_y);
            src[3] *= matte_factor(layer, comp_x, comp_y);
            let dst = read_pixel(frame, x, y);
            if layer.preserve_underlying_transparency {
                src[3] *= dst[3].clamp(0.0, 1.0);
            }
            let out = composite_pixel(
                layer.blend.mode,
                src,
                dst,
                layer.blend.opacity as f32,
                request.bits,
                layer.three_d,
                layer.frame_index,
                x,
                y,
            );
            write_pixel(frame, x, y, out);
        }
    }
}

fn matte_factor(layer: &LayerPlate<'_>, comp_x: f64, comp_y: f64) -> f32 {
    if matches!(layer.matte_mode, TrackMatteMode::None) {
        return 1.0;
    }
    let sample = layer
        .matte_pixels
        .map(|pixels| sample_transformed(pixels, &layer.matte_transform, comp_x, comp_y));
    let pixel = sample.unwrap_or([0.0, 0.0, 0.0, 0.0]);
    let luma = luminance([pixel[0], pixel[1], pixel[2]]).clamp(0.0, 1.0);
    let alpha = pixel[3].clamp(0.0, 1.0);
    match layer.matte_mode {
        TrackMatteMode::None => 1.0,
        TrackMatteMode::Alpha => alpha,
        TrackMatteMode::AlphaInverted => 1.0 - alpha,
        TrackMatteMode::Luma => luma,
        TrackMatteMode::LumaInverted => 1.0 - luma,
    }
}

fn sample_transformed(
    frame: &Frame32,
    transform: &Transform3d,
    comp_x: f64,
    comp_y: f64,
) -> [f32; 4] {
    let Some((local_x, local_y)) = comp_to_local(comp_x, comp_y, transform) else {
        return [0.0; 4];
    };
    sample(frame, pixel_index(local_x), pixel_index(local_y))
}

/// 正の回転は時計回り。After Effects の向きに合わせる。
fn comp_to_local(x: f64, y: f64, transform: &Transform3d) -> Option<(f64, f64)> {
    let sx = transform.scale[0];
    let sy = transform.scale[1];
    if sx.abs() < 1.0e-12 || sy.abs() < 1.0e-12 {
        return None;
    }
    let (rx, ry) = rotate_clockwise(
        x - transform.position[0],
        y - transform.position[1],
        -transform.rotation_z,
    );
    Some((rx / sx + transform.anchor[0], ry / sy + transform.anchor[1]))
}

fn rotate_clockwise(x: f64, y: f64, degrees: f64) -> (f64, f64) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    (x * cos + y * sin, -x * sin + y * cos)
}

fn pixel_index(value: f64) -> i64 {
    if !value.is_finite() {
        return -1;
    }
    let nearest = value.round();
    if (value - nearest).abs() < 1.0e-6 {
        nearest as i64
    } else {
        value.floor() as i64
    }
}

fn buffer_to_comp(index: u32, buffer: u32, composition: u32) -> f64 {
    if buffer == composition {
        f64::from(index)
    } else if buffer == 0 {
        0.0
    } else {
        f64::from(index) * f64::from(composition) / f64::from(buffer)
    }
}

fn fill(frame: &mut Frame32, color: Rgba) {
    for pixel in frame.samples.chunks_mut(4) {
        if pixel.len() < 4 {
            break;
        }
        pixel[0] = color.r;
        pixel[1] = color.g;
        pixel[2] = color.b;
        pixel[3] = color.a;
    }
}

fn sample(frame: &Frame32, x: i64, y: i64) -> [f32; 4] {
    if frame.channels != 4 || x < 0 || y < 0 {
        return [0.0; 4];
    }
    let x = x as u32;
    let y = y as u32;
    if x >= frame.width || y >= frame.height {
        return [0.0; 4];
    }
    read_pixel(frame, x, y)
}

fn read_pixel(frame: &Frame32, x: u32, y: u32) -> [f32; 4] {
    let index = (y as usize * frame.width as usize + x as usize) * 4;
    if index + 3 >= frame.samples.len() {
        return [0.0; 4];
    }
    [
        frame.samples[index],
        frame.samples[index + 1],
        frame.samples[index + 2],
        frame.samples[index + 3],
    ]
}

fn write_pixel(frame: &mut Frame32, x: u32, y: u32, pixel: [f32; 4]) {
    let index = (y as usize * frame.width as usize + x as usize) * 4;
    if index + 3 >= frame.samples.len() {
        return;
    }
    frame.samples[index..index + 4].copy_from_slice(&pixel);
}

#[cfg(test)]
mod tests {
    use super::{composite, composite_export, preview_composite, CompositeRequest, LayerPlate};
    use crate::resolution::{CompResolution, Resolution};
    use effect_api::Frame32;
    use model::{BitsPerChannel, BlendMode, Rgba, TrackMatteMode};

    fn solid(width: u32, height: u32, pixel: [f32; 4]) -> Frame32 {
        let mut frame = Frame32::new(width, height, 4, 0.0);
        for chunk in frame.samples.chunks_mut(4) {
            chunk.copy_from_slice(&pixel);
        }
        frame
    }

    fn request<'a>(
        width: u32,
        height: u32,
        bits: BitsPerChannel,
        layers: &'a [LayerPlate<'a>],
    ) -> CompositeRequest<'a> {
        CompositeRequest {
            width,
            height,
            bits,
            background: Rgba::transparent(),
            layers,
        }
    }

    fn pixel(frame: &Frame32, x: u32, y: u32) -> [f32; 4] {
        let index = ((y * frame.width + x) * 4) as usize;
        frame.samples[index..index + 4].try_into().unwrap()
    }

    fn close(pixel: [f32; 4], expected: [f32; 4]) -> bool {
        pixel
            .iter()
            .zip(expected)
            .all(|(got, want)| (got - want).abs() < 1.0e-4)
    }

    #[test]
    fn opaque_red_over_blue_stays_red() {
        let red = solid(2, 2, [1.0, 0.0, 0.0, 1.0]);
        let blue = solid(2, 2, [0.0, 0.0, 1.0, 1.0]);
        let layers = [LayerPlate::new(&red), LayerPlate::new(&blue)];
        let frame = composite(&request(2, 2, BitsPerChannel::Bpc8, &layers));
        assert_eq!(pixel(&frame, 0, 0), [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn partial_opacity_mixes_over_the_lower_color() {
        let red = solid(1, 1, [1.0, 0.0, 0.0, 1.0]);
        let blue = solid(1, 1, [0.0, 0.0, 1.0, 1.0]);
        let mut front = LayerPlate::new(&red);
        front.blend.opacity = 0.5;
        let layers = [front, LayerPlate::new(&blue)];
        let frame = composite(&request(1, 1, BitsPerChannel::Bpc8, &layers));
        assert_eq!(pixel(&frame, 0, 0), [0.5, 0.0, 0.5, 1.0]);
    }

    #[test]
    fn multiply_and_screen_and_add_follow_the_documented_math() {
        let gray = solid(1, 1, [0.5, 0.5, 0.5, 1.0]);
        let mut multiply = LayerPlate::new(&gray);
        multiply.blend.mode = BlendMode::Multiply;
        let layers = [multiply, LayerPlate::new(&gray)];
        let frame = composite(&request(1, 1, BitsPerChannel::Bpc8, &layers));
        assert_eq!(pixel(&frame, 0, 0), [0.25, 0.25, 0.25, 1.0]);

        let mut screen = LayerPlate::new(&gray);
        screen.blend.mode = BlendMode::Screen;
        let layers = [screen, LayerPlate::new(&gray)];
        let frame = composite(&request(1, 1, BitsPerChannel::Bpc8, &layers));
        assert_eq!(pixel(&frame, 0, 0), [0.75, 0.75, 0.75, 1.0]);

        let bright = solid(1, 1, [0.75, 0.75, 0.75, 1.0]);
        let mut add = LayerPlate::new(&bright);
        add.blend.mode = BlendMode::Add;
        let layers = [add, LayerPlate::new(&bright)];
        let eight = composite(&request(1, 1, BitsPerChannel::Bpc8, &layers));
        let thirty_two = composite(&request(1, 1, BitsPerChannel::Bpc32, &layers));
        assert_eq!(pixel(&eight, 0, 0)[0], 1.0);
        assert_eq!(pixel(&thirty_two, 0, 0)[0], 1.5);
    }

    #[test]
    fn darken_is_per_channel_and_darker_color_picks_one_rgb() {
        let red = solid(1, 1, [1.0, 0.0, 0.0, 1.0]);
        let cyan = solid(1, 1, [0.0, 1.0, 1.0, 1.0]);
        let mut darken = LayerPlate::new(&red);
        darken.blend.mode = BlendMode::Darken;
        let layers = [darken, LayerPlate::new(&cyan)];
        let frame = composite(&request(1, 1, BitsPerChannel::Bpc8, &layers));
        assert_eq!(pixel(&frame, 0, 0), [0.0, 0.0, 0.0, 1.0]);

        let mut darker = LayerPlate::new(&red);
        darker.blend.mode = BlendMode::DarkerColor;
        let layers = [darker, LayerPlate::new(&cyan)];
        let frame = composite(&request(1, 1, BitsPerChannel::Bpc8, &layers));
        let got = pixel(&frame, 0, 0);
        assert!(got == [1.0, 0.0, 0.0, 1.0] || got == [0.0, 1.0, 1.0, 1.0]);
        assert_ne!(got, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn documented_color_modes_keep_their_edge_cases() {
        let white = solid(1, 1, [1.0, 1.0, 1.0, 1.0]);
        let black = solid(1, 1, [0.0, 0.0, 0.0, 1.0]);
        let mid = solid(1, 1, [0.4, 0.4, 0.4, 1.0]);
        let gray = solid(1, 1, [0.5, 0.5, 0.5, 1.0]);
        let red = solid(1, 1, [1.0, 0.0, 0.0, 1.0]);

        let mut burn = LayerPlate::new(&white);
        burn.blend.mode = BlendMode::ColorBurn;
        let layers = [burn, LayerPlate::new(&mid)];
        let frame = composite(&request(1, 1, BitsPerChannel::Bpc8, &layers));
        assert!(close(pixel(&frame, 0, 0), [0.4, 0.4, 0.4, 1.0]));

        let mut linear = LayerPlate::new(&black);
        linear.blend.mode = BlendMode::LinearBurn;
        let layers = [linear, LayerPlate::new(&mid)];
        let frame = composite(&request(1, 1, BitsPerChannel::Bpc8, &layers));
        assert_eq!(pixel(&frame, 0, 0)[0], 0.0);

        let mut overlay = LayerPlate::new(&gray);
        overlay.blend.mode = BlendMode::Overlay;
        let layers = [overlay, LayerPlate::new(&mid)];
        let frame = composite(&request(1, 1, BitsPerChannel::Bpc8, &layers));
        assert!(close(pixel(&frame, 0, 0), [0.4, 0.4, 0.4, 1.0]));

        let mut soft = LayerPlate::new(&white);
        soft.blend.mode = BlendMode::SoftLight;
        let layers = [soft, LayerPlate::new(&black)];
        let frame = composite(&request(1, 1, BitsPerChannel::Bpc8, &layers));
        assert!(pixel(&frame, 0, 0)[0] < 1.0);

        let mut hue = LayerPlate::new(&red);
        hue.blend.mode = BlendMode::Hue;
        let layers = [hue, LayerPlate::new(&gray)];
        let frame = composite(&request(1, 1, BitsPerChannel::Bpc8, &layers));
        let got = pixel(&frame, 0, 0);
        assert!((got[0] - got[1]).abs() < 1.0e-4);
        assert!((got[1] - got[2]).abs() < 1.0e-4);
    }

    #[test]
    fn transform_offset_scale_and_clockwise_rotation() {
        let red = solid(2, 2, [1.0, 0.0, 0.0, 1.0]);
        let mut moved = LayerPlate::new(&red);
        moved.transform.position = [2.0, 1.0, 0.0];
        let layers = [moved];
        let frame = composite(&request(4, 3, BitsPerChannel::Bpc8, &layers));
        assert_eq!(pixel(&frame, 2, 1), [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(pixel(&frame, 0, 0), [0.0, 0.0, 0.0, 0.0]);

        let mut scaled = solid(2, 1, [0.0, 0.0, 0.0, 0.0]);
        scaled.samples[0..4].copy_from_slice(&[1.0, 0.0, 0.0, 1.0]);
        scaled.samples[4..8].copy_from_slice(&[0.0, 1.0, 0.0, 1.0]);
        let mut layer = LayerPlate::new(&scaled);
        layer.transform.scale = [2.0, 1.0, 1.0];
        let layers = [layer];
        let frame = composite(&request(4, 1, BitsPerChannel::Bpc8, &layers));
        assert_eq!(pixel(&frame, 0, 0)[0], 1.0);
        assert_eq!(pixel(&frame, 1, 0)[0], 1.0);
        assert_eq!(pixel(&frame, 2, 0)[1], 1.0);
        assert_eq!(pixel(&frame, 3, 0)[1], 1.0);

        let mut rotated_source = solid(2, 1, [0.0, 0.0, 0.0, 0.0]);
        rotated_source.samples[0..4].copy_from_slice(&[1.0, 0.0, 0.0, 1.0]);
        rotated_source.samples[4..8].copy_from_slice(&[0.0, 1.0, 0.0, 1.0]);
        let mut rotated = LayerPlate::new(&rotated_source);
        rotated.transform.position = [0.0, 1.0, 0.0];
        rotated.transform.rotation_z = 90.0;
        let layers = [rotated];
        let frame = composite(&request(2, 2, BitsPerChannel::Bpc8, &layers));
        assert_eq!(pixel(&frame, 0, 0)[1], 1.0);
        assert_eq!(pixel(&frame, 0, 1)[0], 1.0);
    }

    #[test]
    fn track_mattes_keep_only_the_matte_shape() {
        let fill = solid(2, 1, [1.0, 0.0, 0.0, 1.0]);
        let mut matte = solid(2, 1, [0.0, 0.0, 0.0, 0.0]);
        matte.samples[3] = 1.0;
        matte.samples[0] = 1.0;
        let mut alpha = LayerPlate::new(&fill);
        alpha.matte_mode = TrackMatteMode::Alpha;
        alpha.matte_pixels = Some(&matte);
        let layers = [alpha];
        let frame = composite(&request(2, 1, BitsPerChannel::Bpc8, &layers));
        assert_eq!(pixel(&frame, 0, 0)[3], 1.0);
        assert_eq!(pixel(&frame, 1, 0)[3], 0.0);

        let mut inverted = LayerPlate::new(&fill);
        inverted.matte_mode = TrackMatteMode::AlphaInverted;
        inverted.matte_pixels = Some(&matte);
        let layers = [inverted];
        let frame = composite(&request(2, 1, BitsPerChannel::Bpc8, &layers));
        assert_eq!(pixel(&frame, 0, 0)[3], 0.0);
        assert_eq!(pixel(&frame, 1, 0)[3], 1.0);

        let mut luma_matte = solid(2, 1, [0.0, 0.0, 0.0, 1.0]);
        luma_matte.samples[0..4].copy_from_slice(&[1.0, 1.0, 1.0, 1.0]);
        let mut luma = LayerPlate::new(&fill);
        luma.matte_mode = TrackMatteMode::Luma;
        luma.matte_pixels = Some(&luma_matte);
        let layers = [luma];
        let frame = composite(&request(2, 1, BitsPerChannel::Bpc8, &layers));
        assert_eq!(pixel(&frame, 0, 0)[0], 1.0);
        assert_eq!(pixel(&frame, 1, 0)[3], 0.0);

        let mut luma_inv = LayerPlate::new(&fill);
        luma_inv.matte_mode = TrackMatteMode::LumaInverted;
        luma_inv.matte_pixels = Some(&luma_matte);
        let layers = [luma_inv];
        let frame = composite(&request(2, 1, BitsPerChannel::Bpc8, &layers));
        assert_eq!(pixel(&frame, 0, 0)[3], 0.0);
        assert_eq!(pixel(&frame, 1, 0)[0], 1.0);
    }

    #[test]
    fn preserve_underlying_transparency_hides_empty_areas() {
        let mut lower = solid(2, 1, [0.0, 0.0, 0.0, 0.0]);
        lower.samples[0..4].copy_from_slice(&[1.0, 0.0, 0.0, 1.0]);
        let upper = solid(2, 1, [0.0, 1.0, 0.0, 1.0]);
        let mut front = LayerPlate::new(&upper);
        front.preserve_underlying_transparency = true;
        let layers = [front, LayerPlate::new(&lower)];
        let frame = composite(&request(2, 1, BitsPerChannel::Bpc8, &layers));
        assert_eq!(pixel(&frame, 0, 0), [0.0, 1.0, 0.0, 1.0]);
        assert_eq!(pixel(&frame, 1, 0)[3], 0.0);
    }

    #[test]
    fn stencil_alpha_hides_every_layer_below() {
        let red = solid(2, 1, [1.0, 0.0, 0.0, 1.0]);
        let green = solid(2, 1, [0.0, 1.0, 0.0, 1.0]);
        let mut stencil_pixels = solid(2, 1, [1.0, 1.0, 1.0, 0.0]);
        stencil_pixels.samples[7] = 1.0;
        let mut stencil = LayerPlate::new(&stencil_pixels);
        stencil.blend.mode = BlendMode::StencilAlpha;
        let layers = [stencil, LayerPlate::new(&green), LayerPlate::new(&red)];
        let frame = composite(&request(2, 1, BitsPerChannel::Bpc8, &layers));
        assert_eq!(pixel(&frame, 0, 0)[3], 0.0);
        assert_eq!(pixel(&frame, 1, 0), [0.0, 1.0, 0.0, 1.0]);
    }

    #[test]
    fn alpha_add_is_more_opaque_than_normal() {
        let red = solid(1, 1, [1.0, 0.0, 0.0, 0.5]);
        let blue = solid(1, 1, [0.0, 0.0, 1.0, 0.5]);
        let normal = composite(&request(
            1,
            1,
            BitsPerChannel::Bpc8,
            &[LayerPlate::new(&red), LayerPlate::new(&blue)],
        ));
        let mut front = LayerPlate::new(&red);
        front.blend.mode = BlendMode::AlphaAdd;
        let added = composite(&request(
            1,
            1,
            BitsPerChannel::Bpc8,
            &[front, LayerPlate::new(&blue)],
        ));
        assert!(pixel(&added, 0, 0)[3] > pixel(&normal, 0, 0)[3]);
        assert_eq!(pixel(&normal, 0, 0)[3], 0.75);
        assert_eq!(pixel(&added, 0, 0)[3], 1.0);
    }

    #[test]
    fn dissolve_dithers_and_three_d_matches_normal() {
        let red = solid(16, 16, [1.0, 0.0, 0.0, 1.0]);
        let blue = solid(16, 16, [0.0, 0.0, 1.0, 1.0]);
        let mut front = LayerPlate::new(&red);
        front.blend.mode = BlendMode::Dissolve;
        front.blend.opacity = 0.5;
        let layers = [front, LayerPlate::new(&blue)];
        let frame = composite(&request(16, 16, BitsPerChannel::Bpc8, &layers));
        let mut source = 0;
        let mut lower = 0;
        for y in 0..16 {
            for x in 0..16 {
                let got = pixel(&frame, x, y);
                if got == [1.0, 0.0, 0.0, 1.0] {
                    source += 1;
                } else if got == [0.0, 0.0, 1.0, 1.0] {
                    lower += 1;
                }
            }
        }
        assert!(source > 0 && lower > 0);
        assert_eq!(source + lower, 16 * 16);
        let again = composite(&request(16, 16, BitsPerChannel::Bpc8, &layers));
        assert_eq!(again.samples, frame.samples);
        front.frame_index = 5;
        let later = composite(&request(
            16,
            16,
            BitsPerChannel::Bpc8,
            &[front, LayerPlate::new(&blue)],
        ));
        assert_eq!(later.samples, frame.samples);

        let mut dancing = front;
        dancing.blend.mode = BlendMode::DancingDissolve;
        dancing.frame_index = 0;
        let first = composite(&request(
            16,
            16,
            BitsPerChannel::Bpc8,
            &[dancing, LayerPlate::new(&blue)],
        ));
        dancing.frame_index = 1;
        let moved = composite(&request(
            16,
            16,
            BitsPerChannel::Bpc8,
            &[dancing, LayerPlate::new(&blue)],
        ));
        assert_ne!(moved.samples, first.samples);

        let mut dimensional = front;
        dimensional.three_d = true;
        dimensional.blend.mode = BlendMode::Dissolve;
        let mut normal = LayerPlate::new(&red);
        normal.blend.opacity = 0.5;
        let got = composite(&request(
            16,
            16,
            BitsPerChannel::Bpc8,
            &[dimensional, LayerPlate::new(&blue)],
        ));
        let expect = composite(&request(
            16,
            16,
            BitsPerChannel::Bpc8,
            &[normal, LayerPlate::new(&blue)],
        ));
        assert_eq!(got.samples, expect.samples);
    }

    #[test]
    fn difference_subtract_and_divide() {
        let red = solid(1, 1, [1.0, 0.0, 0.0, 1.0]);
        let mut difference = LayerPlate::new(&red);
        difference.blend.mode = BlendMode::Difference;
        let frame = composite(&request(
            1,
            1,
            BitsPerChannel::Bpc8,
            &[difference, LayerPlate::new(&red)],
        ));
        assert_eq!(pixel(&frame, 0, 0), [0.0, 0.0, 0.0, 1.0]);

        let white = solid(1, 1, [1.0, 1.0, 1.0, 1.0]);
        let gray = solid(1, 1, [0.25, 0.25, 0.25, 1.0]);
        let mut subtract = LayerPlate::new(&white);
        subtract.blend.mode = BlendMode::Subtract;
        let low = composite(&request(
            1,
            1,
            BitsPerChannel::Bpc8,
            &[subtract, LayerPlate::new(&gray)],
        ));
        let high = composite(&request(
            1,
            1,
            BitsPerChannel::Bpc32,
            &[subtract, LayerPlate::new(&gray)],
        ));
        assert_eq!(pixel(&low, 0, 0)[0], 0.0);
        assert!(pixel(&high, 0, 0)[0] < 0.0);

        let mut divide = LayerPlate::new(&gray);
        divide.blend.mode = BlendMode::Divide;
        let full = solid(1, 1, [1.0, 1.0, 1.0, 1.0]);
        let clipped = composite(&request(
            1,
            1,
            BitsPerChannel::Bpc8,
            &[divide, LayerPlate::new(&full)],
        ));
        let wide = composite(&request(
            1,
            1,
            BitsPerChannel::Bpc32,
            &[divide, LayerPlate::new(&full)],
        ));
        assert_eq!(pixel(&clipped, 0, 0)[0], 1.0);
        assert!(pixel(&wide, 0, 0)[0] > 1.0);
        let mut white_src = LayerPlate::new(&full);
        white_src.blend.mode = BlendMode::Divide;
        let unchanged = composite(&request(
            1,
            1,
            BitsPerChannel::Bpc32,
            &[white_src, LayerPlate::new(&gray)],
        ));
        assert_eq!(pixel(&unchanged, 0, 0)[0], 0.25);
    }

    #[test]
    fn hidden_layers_are_skipped_and_export_ignores_preview_resolution() {
        let red = solid(4, 2, [1.0, 0.0, 0.0, 1.0]);
        let blue = solid(4, 2, [0.0, 0.0, 1.0, 1.0]);
        let mut hidden = LayerPlate::new(&red);
        hidden.visible = false;
        let layers = [hidden, LayerPlate::new(&blue)];
        let built = request(4, 2, BitsPerChannel::Bpc8, &layers);
        let frame = composite(&built);
        assert_eq!(pixel(&frame, 0, 0), [0.0, 0.0, 1.0, 1.0]);
        let resolution = CompResolution {
            value: Resolution::Half,
            preserve_when_nested: false,
        };
        let preview = preview_composite(&built, &resolution, &[1.0]).unwrap();
        assert_eq!((preview.width, preview.height), (2, 1));
        let exported = composite_export(&built);
        assert_eq!((exported.width, exported.height), (4, 2));
        assert_eq!(exported.samples, frame.samples);
    }
}
