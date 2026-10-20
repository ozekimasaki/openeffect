use effect_api::Frame32;
use model::Rgba;
use serde::{Deserialize, Serialize};

/// Adobe の `.guides` キー名は要確認。これは OpenEffect の JSON で、拡張子は `.guides`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuideUnit {
    Pixels,
    Percent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuideStyle {
    Lines,
    Dashed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RulerEdge {
    Top,
    Left,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Guide {
    pub orientation: Orientation,
    pub position: f64,
    pub unit: GuideUnit,
    pub edge: Edge,
    /// 無いときはビューのガイド色を使う。
    pub color: Option<Rgba>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuideAppearance {
    pub color: Rgba,
    pub style: GuideStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ViewGuides {
    pub visible: bool,
    pub locked: bool,
    pub guides: Vec<Guide>,
    pub appearance: GuideAppearance,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuideFileError {
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuideSnap {
    pub enabled: bool,
    /// 吸着距離。ヘルプの数値は要確認なので、呼び出し側が渡す。
    pub distance: f64,
}

impl ViewGuides {
    /// 表示とロックの初期値は仕様に数値が無い。新しいビューは表示オン、ロックオフ。
    pub fn new(appearance: GuideAppearance) -> Self {
        Self {
            visible: true,
            locked: false,
            guides: Vec::new(),
            appearance,
        }
    }

    pub fn toggle_visible(&mut self) {
        self.visible = !self.visible;
    }

    pub fn toggle_locked(&mut self) {
        self.locked = !self.locked;
    }

    pub fn clear(&mut self) {
        self.guides.clear();
    }

    pub fn push_from_ruler(&mut self, edge: RulerEdge, position: f64) -> Guide {
        let guide = guide_from_ruler(edge, position);
        self.guides.push(guide);
        guide
    }

    pub fn move_guide(&mut self, index: usize, position: f64) -> bool {
        if self.locked {
            return false;
        }
        let Some(guide) = self.guides.get_mut(index) else {
            return false;
        };
        guide.position = position;
        true
    }

    pub fn effective_color(&self, guide: &Guide) -> Rgba {
        guide.color.unwrap_or(self.appearance.color)
    }

    pub fn effective_style(&self, guide: &Guide) -> GuideStyle {
        let _ = guide;
        self.appearance.style
    }
}

pub fn guide_from_ruler(edge: RulerEdge, position: f64) -> Guide {
    match edge {
        RulerEdge::Top => Guide {
            orientation: Orientation::Horizontal,
            position,
            unit: GuideUnit::Pixels,
            edge: Edge::Top,
            color: None,
        },
        RulerEdge::Left => Guide {
            orientation: Orientation::Vertical,
            position,
            unit: GuideUnit::Pixels,
            edge: Edge::Left,
            color: None,
        },
    }
}

impl Guide {
    pub fn axis_pixel(&self, frame_width: u32, frame_height: u32) -> f64 {
        let span = match self.orientation {
            Orientation::Horizontal => f64::from(frame_height),
            Orientation::Vertical => f64::from(frame_width),
        };
        let along = match self.unit {
            GuideUnit::Pixels => self.position,
            GuideUnit::Percent => self.position / 100.0 * span,
        };
        if self.from_end() {
            span - along
        } else {
            along
        }
    }

    fn from_end(&self) -> bool {
        match self.orientation {
            Orientation::Horizontal => matches!(self.edge, Edge::Bottom | Edge::Right),
            Orientation::Vertical => matches!(self.edge, Edge::Right | Edge::Bottom),
        }
    }
}

pub fn export_json(guides: &[Guide]) -> Result<String, GuideFileError> {
    let document = GuidesDocument {
        guides: guides.iter().copied().map(GuideRecord::from).collect(),
    };
    serde_json::to_string(&document).map_err(|error| GuideFileError {
        message: error.to_string(),
    })
}

/// 既存のガイドは残し、ファイルのガイドを後ろへ足す。
/// `premiere` のガイドは現在のフレームサイズのピクセル位置になり、個別色を捨てる。
pub fn import_json(
    guides: &mut Vec<Guide>,
    json: &str,
    frame_width: u32,
    frame_height: u32,
) -> Result<(), GuideFileError> {
    let document: GuidesDocument = serde_json::from_str(json).map_err(|error| GuideFileError {
        message: error.to_string(),
    })?;
    for record in document.guides {
        let premiere = record.premiere;
        let mut guide = record.into_guide()?;
        if premiere {
            let pixel = guide.axis_pixel(frame_width, frame_height);
            guide.position = pixel;
            guide.unit = GuideUnit::Pixels;
            guide.edge = match guide.orientation {
                Orientation::Horizontal => Edge::Top,
                Orientation::Vertical => Edge::Left,
            };
            guide.color = None;
        }
        guides.push(guide);
    }
    Ok(())
}

pub fn snap_edge(
    position: f64,
    orientation: Orientation,
    guides: &ViewGuides,
    frame_width: u32,
    frame_height: u32,
    snap: GuideSnap,
) -> f64 {
    if !snap.enabled || !snap.distance.is_finite() || snap.distance < 0.0 {
        return position;
    }
    let mut best: Option<(f64, f64)> = None;
    for guide in &guides.guides {
        if guide.orientation != orientation {
            continue;
        }
        let line = guide.axis_pixel(frame_width, frame_height);
        let delta = (line - position).abs();
        if delta <= snap.distance {
            let replace = match best {
                None => true,
                Some((best_delta, best_line)) => {
                    delta < best_delta || ((delta - best_delta).abs() < 1.0e-9 && line < best_line)
                }
            };
            if replace {
                best = Some((delta, line));
            }
        }
    }
    best.map(|(_, line)| line).unwrap_or(position)
}

/// ガイドはプレビューの上に重ねる。合成結果そのものは渡されたフレームのコピーから描く。
pub fn overlay_guides(frame: &Frame32, guides: &ViewGuides) -> Frame32 {
    let mut output = frame.clone();
    if !guides.visible || frame.channels != 4 {
        return output;
    }
    for guide in &guides.guides {
        let color = guides.effective_color(guide);
        let style = guides.effective_style(guide);
        let axis = guide.axis_pixel(frame.width, frame.height).round();
        if !axis.is_finite() {
            continue;
        }
        let axis = axis as i64;
        match guide.orientation {
            Orientation::Horizontal => {
                if axis < 0 || axis >= i64::from(frame.height) {
                    continue;
                }
                for x in 0..frame.width {
                    if style == GuideStyle::Dashed && x % 2 == 1 {
                        continue;
                    }
                    write_overlay(&mut output, x, axis as u32, color);
                }
            }
            Orientation::Vertical => {
                if axis < 0 || axis >= i64::from(frame.width) {
                    continue;
                }
                for y in 0..frame.height {
                    if style == GuideStyle::Dashed && y % 2 == 1 {
                        continue;
                    }
                    write_overlay(&mut output, axis as u32, y, color);
                }
            }
        }
    }
    output
}

fn write_overlay(frame: &mut Frame32, x: u32, y: u32, color: Rgba) {
    let index = (y as usize * frame.width as usize + x as usize) * 4;
    if index + 3 >= frame.samples.len() {
        return;
    }
    frame.samples[index] = color.r;
    frame.samples[index + 1] = color.g;
    frame.samples[index + 2] = color.b;
    frame.samples[index + 3] = color.a;
}

#[derive(Serialize, Deserialize)]
struct GuidesDocument {
    guides: Vec<GuideRecord>,
}

#[derive(Serialize, Deserialize)]
struct GuideRecord {
    orientation: String,
    position: f64,
    unit: String,
    edge: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<[f32; 4]>,
    #[serde(default, skip_serializing_if = "is_false")]
    premiere: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl From<Guide> for GuideRecord {
    fn from(guide: Guide) -> Self {
        Self {
            orientation: match guide.orientation {
                Orientation::Horizontal => "horizontal",
                Orientation::Vertical => "vertical",
            }
            .to_string(),
            position: guide.position,
            unit: match guide.unit {
                GuideUnit::Pixels => "pixels",
                GuideUnit::Percent => "percent",
            }
            .to_string(),
            edge: match guide.edge {
                Edge::Left => "left",
                Edge::Right => "right",
                Edge::Top => "top",
                Edge::Bottom => "bottom",
            }
            .to_string(),
            color: guide
                .color
                .map(|color| [color.r, color.g, color.b, color.a]),
            premiere: false,
        }
    }
}

impl GuideRecord {
    fn into_guide(self) -> Result<Guide, GuideFileError> {
        let orientation = match self.orientation.as_str() {
            "horizontal" => Orientation::Horizontal,
            "vertical" => Orientation::Vertical,
            _ => {
                return Err(GuideFileError {
                    message: "unknown orientation".to_string(),
                })
            }
        };
        let unit = match self.unit.as_str() {
            "pixels" => GuideUnit::Pixels,
            "percent" => GuideUnit::Percent,
            _ => {
                return Err(GuideFileError {
                    message: "unknown unit".to_string(),
                })
            }
        };
        let edge = match self.edge.as_str() {
            "left" => Edge::Left,
            "right" => Edge::Right,
            "top" => Edge::Top,
            "bottom" => Edge::Bottom,
            _ => {
                return Err(GuideFileError {
                    message: "unknown edge".to_string(),
                })
            }
        };
        Ok(Guide {
            orientation,
            position: self.position,
            unit,
            edge,
            color: self.color.map(|[r, g, b, a]| Rgba::new(r, g, b, a)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        export_json, guide_from_ruler, import_json, overlay_guides, snap_edge, Edge, Guide,
        GuideAppearance, GuideSnap, GuideStyle, GuideUnit, Orientation, RulerEdge, ViewGuides,
    };
    use effect_api::Frame32;
    use model::Rgba;

    fn appearance(style: GuideStyle) -> GuideAppearance {
        GuideAppearance {
            color: Rgba::new(0.0, 1.0, 0.0, 1.0),
            style,
        }
    }

    #[test]
    fn hiding_guides_keeps_their_positions() {
        let mut guides = ViewGuides::new(appearance(GuideStyle::Lines));
        guides.push_from_ruler(RulerEdge::Top, 4.0);
        guides.toggle_visible();
        assert!(!guides.visible);
        assert_eq!(guides.guides.len(), 1);
        assert_eq!(guides.guides[0].position, 4.0);
        let frame = Frame32::new(4, 4, 4, 0.0);
        let over = overlay_guides(&frame, &guides);
        assert_eq!(over.samples, frame.samples);
    }

    #[test]
    fn lock_stops_the_selection_tool_from_moving_a_guide() {
        let mut guides = ViewGuides::new(appearance(GuideStyle::Lines));
        guides.push_from_ruler(RulerEdge::Left, 3.0);
        guides.toggle_locked();
        assert!(!guides.move_guide(0, 9.0));
        assert_eq!(guides.guides[0].position, 3.0);
        guides.toggle_locked();
        assert!(guides.move_guide(0, 9.0));
        assert_eq!(guides.guides[0].position, 9.0);
    }

    #[test]
    fn clear_removes_only_the_current_view() {
        let mut current = ViewGuides::new(appearance(GuideStyle::Lines));
        let mut other = ViewGuides::new(appearance(GuideStyle::Lines));
        current.push_from_ruler(RulerEdge::Top, 1.0);
        other.push_from_ruler(RulerEdge::Left, 2.0);
        current.clear();
        assert!(current.guides.is_empty());
        assert_eq!(other.guides.len(), 1);
    }

    #[test]
    fn ruler_drags_choose_orientation() {
        let horizontal = guide_from_ruler(RulerEdge::Top, 8.0);
        let vertical = guide_from_ruler(RulerEdge::Left, 5.0);
        assert_eq!(horizontal.orientation, Orientation::Horizontal);
        assert_eq!(vertical.orientation, Orientation::Vertical);
    }

    #[test]
    fn percent_and_far_edges_follow_the_frame() {
        let percent = Guide {
            orientation: Orientation::Horizontal,
            position: 10.0,
            unit: GuideUnit::Percent,
            edge: Edge::Top,
            color: None,
        };
        assert_eq!(percent.axis_pixel(100, 100), 10.0);
        assert_eq!(percent.axis_pixel(100, 200), 20.0);
        assert_eq!(percent.position, 10.0);

        let bottom = Guide {
            orientation: Orientation::Horizontal,
            position: 10.0,
            unit: GuideUnit::Pixels,
            edge: Edge::Bottom,
            color: None,
        };
        assert_eq!(bottom.axis_pixel(50, 100), 90.0);
        assert_eq!(bottom.axis_pixel(50, 200), 190.0);

        let right = Guide {
            orientation: Orientation::Vertical,
            position: 15.0,
            unit: GuideUnit::Pixels,
            edge: Edge::Right,
            color: None,
        };
        assert_eq!(right.axis_pixel(100, 40), 85.0);
        assert_eq!(right.axis_pixel(180, 40), 165.0);
    }

    #[test]
    fn import_appends_and_export_round_trips_the_count() {
        let mut guides = vec![guide_from_ruler(RulerEdge::Top, 2.0)];
        let extra = Guide {
            orientation: Orientation::Vertical,
            position: 25.0,
            unit: GuideUnit::Percent,
            edge: Edge::Right,
            color: Some(Rgba::new(1.0, 0.0, 0.0, 1.0)),
        };
        let json = export_json(&[extra]).unwrap();
        import_json(&mut guides, &json, 200, 100).unwrap();
        assert_eq!(guides.len(), 2);
        assert_eq!(guides[0].position, 2.0);
        assert_eq!(guides[1], extra);

        let exported = export_json(&guides).unwrap();
        let mut empty = Vec::new();
        import_json(&mut empty, &exported, 200, 100).unwrap();
        assert_eq!(empty.len(), guides.len());
        assert_eq!(empty, guides);
    }

    #[test]
    fn uncolored_guides_use_the_preference_color_and_style() {
        let mut guides = ViewGuides::new(appearance(GuideStyle::Lines));
        guides.push_from_ruler(RulerEdge::Top, 0.0);
        let colored = Guide {
            orientation: Orientation::Vertical,
            position: 1.0,
            unit: GuideUnit::Pixels,
            edge: Edge::Left,
            color: Some(Rgba::new(1.0, 0.0, 0.0, 1.0)),
        };
        guides.guides.push(colored);
        assert_eq!(
            guides.effective_color(&guides.guides[0]),
            guides.appearance.color
        );
        assert_eq!(
            guides.effective_color(&guides.guides[1]),
            Rgba::new(1.0, 0.0, 0.0, 1.0)
        );
        assert_eq!(guides.effective_style(&guides.guides[0]), GuideStyle::Lines);
        guides.appearance.style = GuideStyle::Dashed;
        assert_eq!(
            guides.effective_style(&guides.guides[0]),
            GuideStyle::Dashed
        );
        let frame = Frame32::new(4, 2, 4, 0.0);
        guides.guides.truncate(1);
        guides.appearance.style = GuideStyle::Lines;
        let lines = overlay_guides(&frame, &guides);
        guides.appearance.style = GuideStyle::Dashed;
        let dashed = overlay_guides(&frame, &guides);
        assert_ne!(lines.samples, dashed.samples);
        assert_eq!(lines.samples[5], 1.0);
        assert_eq!(dashed.samples[1], 1.0);
        assert_eq!(dashed.samples[5], 0.0);
    }

    #[test]
    fn premiere_guides_resolve_to_pixels_and_drop_their_color() {
        let json = r#"{"guides":[{"orientation":"vertical","position":25.0,"unit":"percent","edge":"right","color":[1.0,0.0,0.0,1.0],"premiere":true}]}"#;
        let mut guides = vec![guide_from_ruler(RulerEdge::Top, 1.0)];
        import_json(&mut guides, json, 200, 80).unwrap();
        assert_eq!(guides.len(), 2);
        assert_eq!(guides[1].position, 150.0);
        assert_eq!(guides[1].unit, GuideUnit::Pixels);
        assert_eq!(guides[1].edge, Edge::Left);
        assert!(guides[1].color.is_none());
    }

    #[test]
    fn snap_pulls_an_edge_onto_a_guide_only_while_enabled() {
        let mut guides = ViewGuides::new(appearance(GuideStyle::Lines));
        guides.push_from_ruler(RulerEdge::Left, 10.0);
        let on = GuideSnap {
            enabled: true,
            distance: 5.0,
        };
        let off = GuideSnap {
            enabled: false,
            distance: 5.0,
        };
        assert_eq!(
            snap_edge(12.0, Orientation::Vertical, &guides, 100, 80, on),
            10.0
        );
        assert_eq!(
            snap_edge(12.0, Orientation::Vertical, &guides, 100, 80, off),
            12.0
        );
    }
}
