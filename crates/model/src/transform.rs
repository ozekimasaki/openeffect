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

fn add3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

impl Transform3d {
    /// 親が無いときの見た目の位置は、自分の位置そのもの。
    pub fn world_position(self, parent: Option<Transform3d>) -> [f64; 3] {
        match parent {
            None => self.position,
            Some(parent) => add3(parent.position, self.position),
        }
    }

    /// 見た目を保って親に付ける。以降の親の移動だけを受ける。
    pub fn parent_keeping_world(self, parent: Transform3d) -> Self {
        let mut child = self;
        child.position = sub3(self.position, parent.position);
        child
    }

    /// Alt で親に付ける。ローカル値は残し、見た目は親の分だけ飛ぶ。
    pub fn parent_taking_transform(self, _parent: Transform3d) -> Self {
        self
    }

    /// 親を外しても見た目を保つ。
    pub fn unparent_keeping_world(self, parent: Transform3d) -> Self {
        let mut child = self;
        child.position = add3(parent.position, self.position);
        child
    }

    /// Alt で親を外す。ローカル値をそのまま世界の位置にするので飛ぶ。
    pub fn unparent_releasing(self, _parent: Transform3d) -> Self {
        self
    }
}
