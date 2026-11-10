//! CPU で合成したフレームを winit の窓へ出す。
//!
//! 画素は CPU で決める。wgpu はそのバッファを窓の表面へコピーする。
//! Windows のインスタンスは [`gpu::instance_descriptor`] に従い DX12 だけを使う。
//! テストは画素の配置だけを見て、ウィンドウは開かない。

use std::sync::Arc;

use effect_api::Frame32;
use wgpu::rwh::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

/// 余白。コンポジションの外は暗い。
const FIELD: [u8; 4] = [24, 24, 26, 255];
const WINDOW_WIDTH: f64 = 640.0;
const WINDOW_HEIGHT: f64 = 360.0;
const ROW_ALIGN: usize = 256;

const PRESENT_SHADER: &str = r#"
fn corner(index: u32) -> vec2<f32> {
    let x = select(0.0, 1.0, index == 1u || index == 4u || index == 5u);
    let y = select(0.0, 1.0, index == 2u || index == 3u || index == 5u);
    return vec2<f32>(x, y);
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs(@builtin(vertex_index) index: u32) -> VsOut {
    let uv = corner(index);
    var out: VsOut;
    out.clip = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@group(0) @binding(0)
var image: texture_2d<f32>;

@group(0) @binding(1)
var image_sampler: sampler;

@fragment
fn fs(input: VsOut) -> @location(0) vec4<f32> {
    return textureSample(image, image_sampler, input.uv);
}
"#;

/// ヘッドレス結合と同じ 4×2 の赤い平面を、暗い窓の中央に出す。
///
/// 窓を閉じるまで戻らない。テストからは呼ばない。
pub fn open_window() {
    let (project, comp) = crate::small_composition();
    let image = crate::evaluate_frame(&project, comp).expect("1 フレーム");
    let frame = crate::composite_on_cpu(&image, project.bits_per_channel);
    let event_loop = EventLoop::new().expect("イベントループ");
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut viewer = Viewer {
        frame: Some(frame),
        window: None,
        gpu: None,
        presented: false,
    };
    event_loop.run_app(&mut viewer).expect("ウィンドウ");
}

/// コンポジションを整数倍して窓の中央に置き、余白を暗く塗った RGBA8。
pub(crate) fn present_rgba(frame: &Frame32, width: u32, height: u32) -> Vec<u8> {
    let len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|count| count.checked_mul(4))
        .expect("窓の大きさ");
    let mut pixels = vec![0_u8; len];
    for texel in pixels.chunks_exact_mut(4) {
        texel.copy_from_slice(&FIELD);
    }
    if width == 0 || height == 0 || frame.width == 0 || frame.height == 0 {
        return pixels;
    }
    let placed = placement(frame.width, frame.height, width, height);
    for y in 0..frame.height {
        for x in 0..frame.width {
            let color = texel(frame, x, y);
            let x0 = placed.x + x.saturating_mul(placed.scale);
            let y0 = placed.y + y.saturating_mul(placed.scale);
            stamp(&mut pixels, width, height, x0, y0, placed.scale, color);
        }
    }
    pixels
}

struct Placement {
    x: u32,
    y: u32,
    scale: u32,
}

/// コンポジションが窓の 7 割に収まる最大の整数倍。1 未満にはしない。
fn placement(frame_w: u32, frame_h: u32, win_w: u32, win_h: u32) -> Placement {
    let budget_w = (win_w * 7 / 10).max(1);
    let budget_h = (win_h * 7 / 10).max(1);
    let scale = (budget_w / frame_w).min(budget_h / frame_h).max(1);
    let drawn_w = frame_w.saturating_mul(scale);
    let drawn_h = frame_h.saturating_mul(scale);
    Placement {
        x: win_w.saturating_sub(drawn_w) / 2,
        y: win_h.saturating_sub(drawn_h) / 2,
        scale,
    }
}

fn texel(frame: &Frame32, x: u32, y: u32) -> [u8; 4] {
    let channels = usize::from(frame.channels.max(1));
    let index = (y as usize * frame.width as usize + x as usize) * channels;
    let mut rgba = [0.0_f32; 4];
    rgba[3] = 1.0;
    for channel in 0..channels.min(4) {
        if let Some(value) = frame.samples.get(index + channel) {
            rgba[channel] = *value;
        }
    }
    [
        unit_u8(rgba[0]),
        unit_u8(rgba[1]),
        unit_u8(rgba[2]),
        unit_u8(rgba[3]),
    ]
}

fn unit_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn stamp(pixels: &mut [u8], width: u32, height: u32, x0: u32, y0: u32, scale: u32, color: [u8; 4]) {
    for dy in 0..scale {
        let y = y0 + dy;
        if y >= height {
            break;
        }
        for dx in 0..scale {
            let x = x0 + dx;
            if x >= width {
                break;
            }
            let index = (y as usize * width as usize + x as usize) * 4;
            pixels[index..index + 4].copy_from_slice(&color);
        }
    }
}

struct Viewer {
    frame: Option<Frame32>,
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    presented: bool,
}

impl ApplicationHandler for Viewer {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let Some(frame) = self.frame.take() else {
            return;
        };
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("OpenEffect")
                        .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
                        .with_visible(false),
                )
                .expect("ウィンドウ"),
        );
        let gpu = Gpu::new(Arc::clone(&window), frame);
        let size = physical_size(&window);
        self.gpu = Some(gpu);
        if let Some(gpu) = &mut self.gpu {
            gpu.resize(size.width, size.height);
        }
        self.window = Some(window);
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(window) = &self.window else {
                    return;
                };
                let Some(gpu) = &mut self.gpu else {
                    return;
                };
                if gpu.draw(window) {
                    window.set_visible(true);
                    self.presented = true;
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.presented {
            return;
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

struct SharedWindow(Arc<Window>);

impl HasWindowHandle for SharedWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.0.window_handle()
    }
}

impl HasDisplayHandle for SharedWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.0.display_handle()
    }
}

struct Gpu {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    bind_layout: wgpu::BindGroupLayout,
    frame: Frame32,
    texture: Option<wgpu::Texture>,
    bind: Option<wgpu::BindGroup>,
}

impl Gpu {
    fn new(window: Arc<Window>, frame: Frame32) -> Self {
        let instance = wgpu::Instance::new(gpu::instance_descriptor());
        let surface = instance
            .create_surface(SharedWindow(Arc::clone(&window)))
            .expect("表面");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("描画アダプタ");
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("openeffect"),
                required_features: wgpu::Features::empty(),
                required_limits:
                    wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
            },
            None,
        ))
        .expect("デバイス");
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|format| !format.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .expect("表面の形式");
        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
            wgpu::PresentMode::Fifo
        } else {
            caps.present_modes
                .first()
                .copied()
                .unwrap_or(wgpu::PresentMode::Fifo)
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: 1,
            height: 1,
            desired_maximum_frame_latency: 2,
            present_mode,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
        };
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("present"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(PRESENT_SHADER)),
        });
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("present"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("present"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("present"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("present"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            sampler,
            bind_layout,
            frame,
            texture: None,
            bind: None,
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.upload(width, height);
    }

    fn upload(&mut self, width: u32, height: u32) {
        let pixels = present_rgba(&self.frame, width, height);
        let (padded, stride) = pad_rows(&pixels, width, height);
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("composition"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &padded,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composition"),
            layout: &self.bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        self.texture = Some(texture);
        self.bind = Some(bind);
    }

    fn draw(&mut self, window: &Window) -> bool {
        if self.bind.is_none() || self.config.width == 0 || self.config.height == 0 {
            return false;
        }
        match self.surface.get_current_texture() {
            Ok(surface) => {
                let suboptimal = self.paint(window, surface);
                if suboptimal {
                    self.surface.configure(&self.device, &self.config);
                }
                true
            }
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                false
            }
            Err(wgpu::SurfaceError::Timeout) => false,
            Err(wgpu::SurfaceError::OutOfMemory) => {
                eprintln!("フレームを確保できない");
                false
            }
        }
    }

    fn paint(&self, window: &Window, surface: wgpu::SurfaceTexture) -> bool {
        let Some(bind) = &self.bind else {
            return false;
        };
        let view = surface
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("present"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("present"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(field_color()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, bind, &[]);
            pass.draw(0..6, 0..1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        window.pre_present_notify();
        let suboptimal = surface.suboptimal;
        surface.present();
        suboptimal
    }
}

fn physical_size(window: &Window) -> PhysicalSize<u32> {
    let size = window.inner_size();
    if size.width > 0 && size.height > 0 {
        return size;
    }
    let scale = window.scale_factor();
    PhysicalSize::new(
        (WINDOW_WIDTH * scale).round().max(1.0) as u32,
        (WINDOW_HEIGHT * scale).round().max(1.0) as u32,
    )
}

fn field_color() -> wgpu::Color {
    wgpu::Color {
        r: f64::from(FIELD[0]) / 255.0,
        g: f64::from(FIELD[1]) / 255.0,
        b: f64::from(FIELD[2]) / 255.0,
        a: 1.0,
    }
}

fn pad_rows(pixels: &[u8], width: u32, height: u32) -> (Vec<u8>, u32) {
    let row = width as usize * 4;
    let stride = row.div_ceil(ROW_ALIGN) * ROW_ALIGN;
    if stride == row {
        return (pixels.to_vec(), stride as u32);
    }
    let mut padded = vec![0_u8; stride * height as usize];
    for y in 0..height as usize {
        let source = y * row;
        let dest = y * stride;
        padded[dest..dest + row].copy_from_slice(&pixels[source..source + row]);
    }
    (padded, stride as u32)
}

#[cfg(test)]
mod tests {
    use super::{present_rgba, FIELD};
    use crate::{composite_on_cpu, evaluate_frame, small_composition};
    use effect_api::Frame32;

    #[test]
    fn red_plane_sits_on_a_dark_field_without_a_window() {
        let (project, comp) = small_composition();
        let image = evaluate_frame(&project, comp).expect("frame");
        let frame = composite_on_cpu(&image, project.bits_per_channel);
        let width = 40;
        let height = 20;
        let pixels = present_rgba(&frame, width, height);
        // 予算は 28×14。4×2 なので 7 倍、描画は 28×14、原点は (6, 3)。
        let red = [255, 0, 0, 255];
        assert_eq!(pixels.len(), (width * height * 4) as usize);
        assert_eq!(at(&pixels, width, 0, 0), FIELD);
        assert_eq!(at(&pixels, width, 5, 3), FIELD);
        assert_eq!(at(&pixels, width, 6, 2), FIELD);
        assert_eq!(at(&pixels, width, 6, 3), red);
        assert_eq!(at(&pixels, width, 33, 16), red);
        assert_eq!(at(&pixels, width, 34, 16), FIELD);
        assert_eq!(at(&pixels, width, 33, 17), FIELD);
        let mut red_count = 0_u32;
        for y in 0..height {
            for x in 0..width {
                let pixel = at(&pixels, width, x, y);
                if (6..34).contains(&x) && (3..17).contains(&y) {
                    assert_eq!(pixel, red);
                    red_count += 1;
                } else {
                    assert_eq!(pixel, FIELD);
                }
            }
        }
        assert_eq!(red_count, 28 * 14);
    }

    #[test]
    fn present_rgba_copies_cpu_samples() {
        let mut frame = Frame32::new(1, 1, 4, 0.0);
        frame.samples.copy_from_slice(&[0.0, 1.0, 0.0, 1.0]);
        let pixels = present_rgba(&frame, 10, 10);
        // 予算は 7×7。1×1 なので 7 倍、原点は (1, 1)。
        assert_eq!(at(&pixels, 10, 0, 0), FIELD);
        assert_eq!(at(&pixels, 10, 1, 1), [0, 255, 0, 255]);
        assert_eq!(at(&pixels, 10, 7, 7), [0, 255, 0, 255]);
        assert_eq!(at(&pixels, 10, 8, 7), FIELD);
    }

    fn at(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let index = (y as usize * width as usize + x as usize) * 4;
        pixels[index..index + 4].try_into().expect("texel")
    }
}
