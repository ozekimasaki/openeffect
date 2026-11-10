//! カメラ、ライト、マテリアル、クラシック 3D と Advanced 3D の計算。
//! 公式ヘルプに無い数値は `Confirmed::Unconfirmed` のままにする。

#![forbid(unsafe_code)]

pub mod camera;
pub mod confirmed;
pub mod draft;
pub mod environment;
pub mod geometry;
pub mod light;
pub mod material;
pub mod math;
pub mod renderer;
pub mod shade;
pub mod shadow;
pub mod tool;

pub use camera::{
    active_camera, angle_of_view_degrees, apparent_scale, Camera, CameraPreset, Optics,
};
pub use confirmed::{project_file_location, Confirmed, Keyframes};
pub use light::{Falloff, Light, SceneLightKind};
pub use material::Material;
pub use renderer::{Renderer, ShadowResolution, ShadowSmoothness};
