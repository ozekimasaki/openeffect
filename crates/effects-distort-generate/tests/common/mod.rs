use effect_api::{CpuApply, CpuInput, Frame, ParamValues};
use effects_distort_generate::{Image, Instance, Quality};
use model::{BitsPerChannel, Time};

pub fn solid(w: u32, h: u32, bits: BitsPerChannel, px: [f32; 4]) -> Frame {
    Image::new(w, h, 4, bits, px).to_frame()
}

pub fn paint(frame: &Frame, x: u32, y: u32, px: [f32; 4]) -> Frame {
    let mut image = Image::from_frame(frame);
    image.set(x as i32, y as i32, px);
    image.bits = frame.bits();
    image.to_frame()
}

pub fn view(frame: &Frame) -> Image {
    Image::from_frame(frame)
}

pub fn at(frame: &Frame, x: u32, y: u32) -> [f32; 4] {
    view(frame).at(x, y)
}

pub fn diff(a: &Frame, b: &Frame) -> f32 {
    let a = view(a);
    let b = view(b);
    let mut max = (a.width as i32 - b.width as i32).unsigned_abs() as f32;
    max += (a.height as i32 - b.height as i32).unsigned_abs() as f32;
    for (p, q) in a.px.iter().zip(b.px.iter()) {
        for c in 0..4 {
            max = max.max((p[c] - q[c]).abs());
        }
    }
    max
}

pub fn run(inst: &Instance, frame: &Frame, time: f64) -> Frame {
    effects_distort_generate::run(inst, frame, time).expect("render")
}

pub fn run_bits(
    inst: &Instance,
    frame: &Frame,
    bits: BitsPerChannel,
    time: f64,
) -> Result<Frame, String> {
    let params = ParamValues::new();
    let input = CpuInput {
        time: Time::from_seconds(time),
        bits,
        params: &params,
        frame,
    };
    let mut output = frame.clone();
    inst.apply_cpu(&input, &mut output)
        .map_err(|err| err.message)?;
    Ok(output)
}

pub fn best(inst: &mut Instance) {
    inst.host.quality = Quality::Best;
}

pub fn draft(inst: &mut Instance) {
    inst.host.quality = Quality::Draft;
}
