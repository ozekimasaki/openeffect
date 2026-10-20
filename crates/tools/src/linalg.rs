//! 追跡とカメラ解のための密な線形代数。

pub type Vec3 = [f64; 3];
pub type Mat3 = [[f64; 3]; 3];

pub fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn scale(a: Vec3, s: f64) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn length(a: Vec3) -> f64 {
    dot(a, a).sqrt()
}

pub fn normalize(a: Vec3) -> Vec3 {
    let n = length(a);
    if n < 1e-12 {
        [0.0, 0.0, 0.0]
    } else {
        scale(a, 1.0 / n)
    }
}

pub fn mul_mat_vec(m: Mat3, v: Vec3) -> Vec3 {
    [dot(m[0], v), dot(m[1], v), dot(m[2], v)]
}

pub fn mat_mul(a: Mat3, b: Mat3) -> Mat3 {
    let mut c = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            c[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    c
}

pub fn transpose(m: Mat3) -> Mat3 {
    [
        [m[0][0], m[1][0], m[2][0]],
        [m[0][1], m[1][1], m[2][1]],
        [m[0][2], m[1][2], m[2][2]],
    ]
}

pub fn identity() -> Mat3 {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

pub fn det(m: Mat3) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// yaw は Y、pitch は X、roll は Z。右手系でワールドからカメラへ。
pub fn rotation_ypr(yaw: f64, pitch: f64, roll: f64) -> Mat3 {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let (sr, cr) = roll.sin_cos();
    let ry = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
    let rx = [[1.0, 0.0, 0.0], [0.0, cp, -sp], [0.0, sp, cp]];
    let rz = [[cr, -sr, 0.0], [sr, cr, 0.0], [0.0, 0.0, 1.0]];
    mat_mul(rz, mat_mul(rx, ry))
}

pub fn gram(rows: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = rows.first().map(Vec::len).unwrap_or(0);
    let mut ata = vec![vec![0.0; n]; n];
    for row in rows {
        for i in 0..n {
            for j in 0..n {
                ata[i][j] += row[i] * row[j];
            }
        }
    }
    ata
}

/// 対称行列の最小固有値に対応する固有ベクトル。
pub fn smallest_eigenvector(matrix: &[Vec<f64>]) -> Vec<f64> {
    let n = matrix.len();
    let mut a = matrix.to_vec();
    let mut v = vec![vec![0.0; n]; n];
    for i in 0..n {
        v[i][i] = 1.0;
    }
    let sweeps = 8 * n * n.max(1);
    for _ in 0..sweeps {
        let mut p = 0;
        let mut q = 1.min(n.saturating_sub(1));
        let mut max = 0.0;
        for i in 0..n {
            for j in (i + 1)..n {
                let value = a[i][j].abs();
                if value > max {
                    max = value;
                    p = i;
                    q = j;
                }
            }
        }
        if max < 1e-14 || n < 2 {
            break;
        }
        let app = a[p][p];
        let aqq = a[q][q];
        let apq = a[p][q];
        let tau = (aqq - app) / (2.0 * apq);
        let t = if tau >= 0.0 {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        } else {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        };
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;
        a[p][p] = app - t * apq;
        a[q][q] = aqq + t * apq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;
        for k in 0..n {
            if k != p && k != q {
                let aik = a[p][k];
                let aqk = a[q][k];
                a[p][k] = c * aik - s * aqk;
                a[q][k] = s * aik + c * aqk;
                a[k][p] = a[p][k];
                a[k][q] = a[q][k];
            }
            let vip = v[k][p];
            let viq = v[k][q];
            v[k][p] = c * vip - s * viq;
            v[k][q] = s * vip + c * viq;
        }
    }
    let mut best = 0;
    let mut best_value = f64::MAX;
    for i in 0..n {
        if a[i][i] < best_value {
            best_value = a[i][i];
            best = i;
        }
    }
    (0..n).map(|i| v[i][best]).collect()
}

pub fn homography(src: &[[f64; 2]], dst: &[[f64; 2]]) -> Option<[f64; 9]> {
    if src.len() < 4 || src.len() != dst.len() {
        return None;
    }
    let mut rows = Vec::with_capacity(src.len() * 2);
    for (s, d) in src.iter().zip(dst) {
        rows.push(vec![
            s[0],
            s[1],
            1.0,
            0.0,
            0.0,
            0.0,
            -d[0] * s[0],
            -d[0] * s[1],
            -d[0],
        ]);
        rows.push(vec![
            0.0,
            0.0,
            0.0,
            s[0],
            s[1],
            1.0,
            -d[1] * s[0],
            -d[1] * s[1],
            -d[1],
        ]);
    }
    let mut h = smallest_eigenvector(&gram(&rows));
    if h.len() != 9 || h[8].abs() < 1e-12 {
        return None;
    }
    let scale = h[8];
    for value in &mut h {
        *value /= scale;
    }
    let mut out = [0.0; 9];
    out.copy_from_slice(&h);
    Some(out)
}

pub fn apply_homography(h: &[f64; 9], point: [f64; 2]) -> Option<[f64; 2]> {
    let w = h[6] * point[0] + h[7] * point[1] + h[8];
    if w.abs() < 1e-12 {
        return None;
    }
    Some([
        (h[0] * point[0] + h[1] * point[1] + h[2]) / w,
        (h[3] * point[0] + h[4] * point[1] + h[5]) / w,
    ])
}

fn eigen_symmetric_3(mut a: Mat3) -> (Vec3, Mat3) {
    let mut v = identity();
    for _ in 0..32 {
        let mut p = 0;
        let mut q = 1;
        let mut max = 0.0;
        for i in 0..3 {
            for j in (i + 1)..3 {
                let value = a[i][j].abs();
                if value > max {
                    max = value;
                    p = i;
                    q = j;
                }
            }
        }
        if max < 1e-14 {
            break;
        }
        let app = a[p][p];
        let aqq = a[q][q];
        let apq = a[p][q];
        let tau = (aqq - app) / (2.0 * apq);
        let t = if tau >= 0.0 {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        } else {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        };
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;
        a[p][p] = app - t * apq;
        a[q][q] = aqq + t * apq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;
        for k in 0..3 {
            if k != p && k != q {
                let aik = a[p][k];
                let aqk = a[q][k];
                a[p][k] = c * aik - s * aqk;
                a[q][k] = s * aik + c * aqk;
                a[k][p] = a[p][k];
                a[k][q] = a[q][k];
            }
            let vip = v[k][p];
            let viq = v[k][q];
            v[k][p] = c * vip - s * viq;
            v[k][q] = s * vip + c * viq;
        }
    }
    ([a[0][0], a[1][1], a[2][2]], v)
}

/// `m = u * diag(s) * v^T`。特異値は降順ではない。
pub fn svd_3x3(m: Mat3) -> (Mat3, Vec3, Mat3) {
    let mt = transpose(m);
    let (values, v) = eigen_symmetric_3(mat_mul(mt, m));
    let mut singular = [0.0; 3];
    let mut u_cols = [[0.0; 3]; 3];
    for i in 0..3 {
        let sigma = values[i].abs().sqrt();
        singular[i] = sigma;
        let column = [v[0][i], v[1][i], v[2][i]];
        let mapped = mul_mat_vec(m, column);
        u_cols[i] = if sigma < 1e-12 {
            [0.0, 0.0, 0.0]
        } else {
            scale(mapped, 1.0 / sigma)
        };
    }
    let mut u = [
        [u_cols[0][0], u_cols[1][0], u_cols[2][0]],
        [u_cols[0][1], u_cols[1][1], u_cols[2][1]],
        [u_cols[0][2], u_cols[1][2], u_cols[2][2]],
    ];
    if length(cross(
        [u[0][0], u[1][0], u[2][0]],
        [u[0][1], u[1][1], u[2][1]],
    )) < 1e-8
    {
        let c0 = normalize([u[0][0], u[1][0], u[2][0]]);
        let c1 = normalize([u[0][1], u[1][1], u[2][1]]);
        let c2 = cross(c0, c1);
        for row in 0..3 {
            u[row][0] = [c0, c1, c2][0][row];
            u[row][1] = [c0, c1, c2][1][row];
            u[row][2] = [c0, c1, c2][2][row];
        }
    }
    (u, singular, v)
}

pub fn rotation_between(src: &[Vec3], dst: &[Vec3]) -> Mat3 {
    let mut cov = [[0.0; 3]; 3];
    for (s, d) in src.iter().zip(dst) {
        for i in 0..3 {
            for j in 0..3 {
                cov[i][j] += d[i] * s[j];
            }
        }
    }
    let (u, _s, v) = svd_3x3(cov);
    let mut r = mat_mul(u, transpose(v));
    if det(r) < 0.0 {
        let mut u_fix = u;
        for row in 0..3 {
            u_fix[row][2] = -u_fix[row][2];
        }
        r = mat_mul(u_fix, transpose(v));
    }
    r
}

pub fn closest_point_on_rays(origin_a: Vec3, dir_a: Vec3, origin_b: Vec3, dir_b: Vec3) -> Vec3 {
    let w0 = sub(origin_a, origin_b);
    let a = dot(dir_a, dir_a);
    let b = dot(dir_a, dir_b);
    let c = dot(dir_b, dir_b);
    let d = dot(dir_a, w0);
    let e = dot(dir_b, w0);
    let denom = a * c - b * b;
    if denom.abs() < 1e-10 {
        return add(origin_a, dir_a);
    }
    let t = (b * e - c * d) / denom;
    let s = (a * e - b * d) / denom;
    let p1 = add(origin_a, scale(dir_a, t));
    let p2 = add(origin_b, scale(dir_b, s));
    scale(add(p1, p2), 0.5)
}
