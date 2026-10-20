/// 3 次元の差、内積、回転。軸は X が右、Y が下、Z が手前。
pub fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn scale(a: [f64; 3], factor: f64) -> [f64; 3] {
    [a[0] * factor, a[1] * factor, a[2] * factor]
}

pub fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn length(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

pub fn normalize(a: [f64; 3]) -> Option<[f64; 3]> {
    let len = length(a);
    if len <= f64::EPSILON {
        None
    } else {
        Some(scale(a, 1.0 / len))
    }
}

pub fn reject(vector: [f64; 3], unit_axis: [f64; 3]) -> [f64; 3] {
    sub(vector, scale(unit_axis, dot(vector, unit_axis)))
}

pub fn rotate_x(v: [f64; 3], degrees: f64) -> [f64; 3] {
    let radians = degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    [v[0], v[1] * cos - v[2] * sin, v[1] * sin + v[2] * cos]
}

pub fn rotate_y(v: [f64; 3], degrees: f64) -> [f64; 3] {
    let radians = degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    [v[0] * cos + v[2] * sin, v[1], -v[0] * sin + v[2] * cos]
}

pub fn rotate_z(v: [f64; 3], degrees: f64) -> [f64; 3] {
    let radians = degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    [v[0] * cos - v[1] * sin, v[0] * sin + v[1] * cos, v[2]]
}

pub fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6
}

pub fn close3(a: [f64; 3], b: [f64; 3]) -> bool {
    close(a[0], b[0]) && close(a[1], b[1]) && close(a[2], b[2])
}

/// コンポジション中心。目標点の初期位置。
pub fn composition_center(width: u32, height: u32) -> [f64; 3] {
    [f64::from(width) / 2.0, f64::from(height) / 2.0, 0.0]
}

/// 公式のフォーカス距離式 `length(position, pointOfInterest)`。
pub const FOCUS_DISTANCE_EXPRESSION: &str = "length(position, pointOfInterest)";

pub fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    length(sub(a, b))
}
