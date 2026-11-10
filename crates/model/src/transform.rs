/// 2D 変換。スケール 1 が 100%。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform2d {
    pub anchor: [f64; 2],
    pub position: [f64; 2],
    pub scale: [f64; 2],
    pub rotation_degrees: f64,
}

impl Transform2d {
    pub const fn identity() -> Self {
        Self {
            anchor: [0.0, 0.0],
            position: [0.0, 0.0],
            scale: [1.0, 1.0],
            rotation_degrees: 0.0,
        }
    }
}

impl Default for Transform2d {
    fn default() -> Self {
        Self::identity()
    }
}

/// 親子に渡す 3D 変換。2D では z を 0 にする。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform3d {
    pub anchor: [f64; 3],
    pub position: [f64; 3],
    pub scale: [f64; 3],
    pub orientation: [f64; 3],
    pub rotation_x: f64,
    pub rotation_y: f64,
    pub rotation_z: f64,
}

impl Transform3d {
    pub const fn identity() -> Self {
        Self {
            anchor: [0.0, 0.0, 0.0],
            position: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            orientation: [0.0, 0.0, 0.0],
            rotation_x: 0.0,
            rotation_y: 0.0,
            rotation_z: 0.0,
        }
    }
}

impl Default for Transform3d {
    fn default() -> Self {
        Self::identity()
    }
}
