//! コンテントアウェア塗り。穴は周囲の既知画素から埋めるオープンな手順。

use std::path::PathBuf;

use model::{BitsPerChannel, Ids, Layer, LayerId, LayerKind, Mask, Project, ShapePath, Time};

use crate::raster::Raster;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillRange {
    WorkArea,
    FullComposition,
}

#[derive(Clone, Debug)]
pub struct ContentFill {
    pub range: Option<FillRange>,
    pub alpha_expand: Option<f64>,
    pub directory_name: String,
    pub bits: BitsPerChannel,
}

#[derive(Clone, Debug)]
pub struct FillOutput {
    pub frames: Vec<Raster>,
    pub frames_read: Vec<i32>,
    pub layer_id: LayerId,
}

impl ContentFill {
    pub fn new(bits: BitsPerChannel) -> Self {
        Self {
            range: None,
            alpha_expand: None,
            directory_name: "Fills".into(),
            bits,
        }
    }

    pub fn directory(&self, project_dir: &std::path::Path) -> PathBuf {
        project_dir.join(&self.directory_name)
    }

    pub fn generate(
        &self,
        project: &mut Project,
        comp_id: model::ItemId,
        source: LayerId,
        frames: &[Raster],
        mask: &[[f64; 2]],
        ids: &mut Ids,
    ) -> Result<FillOutput, crate::error::ToolError> {
        let comp = project
            .composition_mut(comp_id)
            .ok_or_else(|| crate::error::ToolError::new("コンポジションがない"))?;
        let (start, end) = match self.range {
            Some(FillRange::WorkArea) | None => {
                let start = comp.work_area_start.to_frames(comp.frame_rate).round() as i32;
                let end = start + comp.work_area_duration.to_frames(comp.frame_rate).round() as i32;
                (start, end.max(start + 1))
            }
            Some(FillRange::FullComposition) => {
                let end = comp.duration.to_frames(comp.frame_rate).round() as i32;
                (0, end.max(1))
            }
        };
        let expand = self.alpha_expand.unwrap_or(0.0).round() as i32;
        let mut output = Vec::new();
        let mut read = Vec::new();
        for frame in start..end {
            if frame < 0 || frame as usize >= frames.len() {
                continue;
            }
            read.push(frame);
            output.push(inpaint(&frames[frame as usize], mask, expand));
        }
        let index = comp
            .layers
            .iter()
            .position(|layer| layer.id == source)
            .unwrap_or(0);
        let id = ids.layer();
        let mut layer = Layer::new(id, "塗り", LayerKind::Null);
        layer.properties.push(crate::persist::scalar(
            ids,
            "イン点",
            "in-frame",
            f64::from(start),
        ));
        layer.properties.push(crate::persist::scalar(
            ids,
            "アウト点",
            "out-frame",
            f64::from(end),
        ));
        layer.properties.push(crate::persist::text(
            ids,
            "深度",
            "fill-bits",
            match self.bits {
                BitsPerChannel::Bpc8 => "8",
                BitsPerChannel::Bpc16 => "16",
                BitsPerChannel::Bpc32 => "32",
            },
        ));
        comp.layers.insert(index, layer);
        let _ = Time::ZERO;
        Ok(FillOutput {
            frames: output,
            frames_read: read,
            layer_id: id,
        })
    }
}

pub fn mask_from_rect(ids: &mut Ids, x: f64, y: f64, w: f64, h: f64) -> Mask {
    let mut mask = Mask::new(ids.property(), "塗りマスク");
    let mut path = ShapePath::new();
    path.closed = true;
    path.vertices = vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
    path.in_tangents = vec![[0.0, 0.0]; 4];
    path.out_tangents = vec![[0.0, 0.0]; 4];
    mask.path = path;
    mask
}

fn inpaint(source: &Raster, polygon: &[[f64; 2]], expand: i32) -> Raster {
    let mut hole = vec![false; source.pixels.len()];
    for y in 0..source.height as i32 {
        for x in 0..source.width as i32 {
            if point_in_polygon([f64::from(x) + 0.5, f64::from(y) + 0.5], polygon) {
                hole[(y * source.width as i32 + x) as usize] = true;
            }
        }
    }
    if expand > 0 {
        hole = dilate(&hole, source.width as i32, source.height as i32, expand);
    }
    let mut out = source.clone();
    let mut pending: Vec<(i32, i32)> = hole
        .iter()
        .enumerate()
        .filter(|(_, on)| **on)
        .map(|(index, _)| {
            (
                index as i32 % source.width as i32,
                index as i32 / source.width as i32,
            )
        })
        .collect();
    let mut known = hole.iter().map(|on| !on).collect::<Vec<_>>();
    while !pending.is_empty() {
        pending.sort_by_key(|(x, y)| {
            -known_neighbors(*x, *y, &known, source.width as i32, source.height as i32)
        });
        let (x, y) = pending.remove(0);
        if let Some(pixel) = nearest_known(&out, &known, x, y) {
            out.set(x, y, pixel);
        }
        known[(y * source.width as i32 + x) as usize] = true;
    }
    out
}

fn known_neighbors(x: i32, y: i32, known: &[bool], width: i32, height: i32) -> i32 {
    let mut count = 0;
    for oy in -1..=1 {
        for ox in -1..=1 {
            let px = x + ox;
            let py = y + oy;
            if px >= 0 && py >= 0 && px < width && py < height && known[(py * width + px) as usize]
            {
                count += 1;
            }
        }
    }
    count
}

fn nearest_known(source: &Raster, known: &[bool], x: i32, y: i32) -> Option<[f32; 4]> {
    let width = source.width as i32;
    let height = source.height as i32;
    for radius in 1..width.max(height) {
        for oy in -radius..=radius {
            for ox in -radius..=radius {
                if ox.abs() != radius && oy.abs() != radius {
                    continue;
                }
                let px = x + ox;
                let py = y + oy;
                if px >= 0
                    && py >= 0
                    && px < width
                    && py < height
                    && known[(py * width + px) as usize]
                {
                    return Some(source.get(px, py));
                }
            }
        }
    }
    None
}

fn dilate(mask: &[bool], width: i32, height: i32, radius: i32) -> Vec<bool> {
    let mut out = mask.to_vec();
    for y in 0..height {
        for x in 0..width {
            if !mask[(y * width + x) as usize] {
                continue;
            }
            for oy in -radius..=radius {
                for ox in -radius..=radius {
                    let px = x + ox;
                    let py = y + oy;
                    if px >= 0 && py >= 0 && px < width && py < height {
                        out[(py * width + px) as usize] = true;
                    }
                }
            }
        }
    }
    out
}

fn point_in_polygon(point: [f64; 2], polygon: &[[f64; 2]]) -> bool {
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[j];
        if (a[1] > point[1]) != (b[1] > point[1])
            && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1] + 1e-9) + a[0]
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use model::{BitsPerChannel, Composition, Project, Time};

    use super::*;

    fn scene() -> (Project, model::ItemId, LayerId, Vec<Raster>) {
        let mut project = Project::new(BitsPerChannel::Bpc16);
        let comp_id = project.alloc_item();
        let mut comp = Composition::new(
            comp_id,
            "comp",
            16,
            12,
            24.0,
            Time::from_seconds(4.0 / 24.0),
        );
        comp.work_area_start = Time::from_frames(1.0, 24.0);
        comp.work_area_duration = Time::from_frames(2.0, 24.0);
        let source = project.alloc_layer();
        comp.layers
            .push(model::Layer::new(source, "source", LayerKind::Null));
        project.add_composition(comp);
        let mut frames = Vec::new();
        for _ in 0..4 {
            let mut image = Raster::new(16, 12, [0.1, 0.6, 0.2, 1.0]);
            image.fill_rect(6, 4, 4, 4, [0.8, 0.1, 0.1, 1.0]);
            frames.push(image);
        }
        (project, comp_id, source, frames)
    }

    #[test]
    fn mask_limits_the_fill_to_the_inside() {
        let (mut project, comp, source, frames) = scene();
        let mut ids = model::Ids::new();
        let fill = ContentFill::new(project.bits_per_channel);
        let polygon = [[6.0, 4.0], [10.0, 4.0], [10.0, 8.0], [6.0, 8.0]];
        let mut tool = fill;
        tool.range = Some(FillRange::FullComposition);
        tool.alpha_expand = Some(0.0);
        let output = tool
            .generate(&mut project, comp, source, &frames, &polygon, &mut ids)
            .unwrap();
        let image = &output.frames[0];
        assert!(image.get(8, 6)[0] < frames[0].get(8, 6)[0]);
        assert_eq!(image.get(1, 1), frames[0].get(1, 1));
    }

    #[test]
    fn work_area_limits_length_and_frames_read() {
        let (mut project, comp, source, frames) = scene();
        let mut ids = model::Ids::new();
        let mut tool = ContentFill::new(project.bits_per_channel);
        tool.range = Some(FillRange::WorkArea);
        tool.alpha_expand = Some(0.0);
        let output = tool
            .generate(
                &mut project,
                comp,
                source,
                &frames,
                &[[6.0, 4.0], [10.0, 4.0], [10.0, 8.0], [6.0, 8.0]],
                &mut ids,
            )
            .unwrap();
        assert_eq!(output.frames.len(), 2);
        assert!(output
            .frames_read
            .iter()
            .all(|frame| *frame >= 1 && *frame < 3));
    }

    #[test]
    fn full_composition_spans_the_duration() {
        let (mut project, comp, source, frames) = scene();
        let mut ids = model::Ids::new();
        let mut tool = ContentFill::new(project.bits_per_channel);
        tool.range = Some(FillRange::FullComposition);
        tool.alpha_expand = Some(0.0);
        let output = tool
            .generate(
                &mut project,
                comp,
                source,
                &frames,
                &[[6.0, 4.0], [10.0, 4.0], [10.0, 8.0], [6.0, 8.0]],
                &mut ids,
            )
            .unwrap();
        assert_eq!(output.frames.len(), 4);
    }

    #[test]
    fn alpha_expand_widens_the_hole() {
        let (mut project, comp, source, mut frames) = scene();
        frames[0].fill_rect(4, 2, 8, 8, [0.9, 0.9, 0.1, 1.0]);
        frames[0].fill_rect(6, 4, 4, 4, [0.8, 0.1, 0.1, 1.0]);
        let polygon = [[6.0, 4.0], [10.0, 4.0], [10.0, 8.0], [6.0, 8.0]];
        let original = frames[0].clone();
        let mut tight = ContentFill::new(project.bits_per_channel);
        tight.range = Some(FillRange::FullComposition);
        tight.alpha_expand = Some(0.0);
        let mut ids = model::Ids::new();
        let a = tight
            .generate(&mut project, comp, source, &frames, &polygon, &mut ids)
            .unwrap();
        let (mut project, comp, source, _) = scene();
        let mut wide = ContentFill::new(project.bits_per_channel);
        wide.range = Some(FillRange::FullComposition);
        wide.alpha_expand = Some(2.0);
        let b = wide
            .generate(&mut project, comp, source, &frames, &polygon, &mut ids)
            .unwrap();
        let changed = |image: &Raster, original: &Raster| {
            image
                .pixels
                .iter()
                .zip(original.pixels.iter())
                .filter(|(left, right)| *left != *right)
                .count()
        };
        assert!(changed(&b.frames[0], &original) > changed(&a.frames[0], &original));
    }

    #[test]
    fn generated_layer_is_above_and_inherits_depth() {
        let (mut project, comp, source, frames) = scene();
        let mut ids = model::Ids::new();
        let tool = ContentFill::new(project.bits_per_channel);
        assert_eq!(tool.bits, BitsPerChannel::Bpc16);
        assert_eq!(
            tool.directory(std::path::Path::new("/proj"))
                .file_name()
                .unwrap(),
            "Fills"
        );
        let mut tool = tool;
        tool.range = Some(FillRange::WorkArea);
        tool.alpha_expand = Some(1.0);
        let output = tool
            .generate(
                &mut project,
                comp,
                source,
                &frames,
                &[[6.0, 4.0], [10.0, 4.0], [10.0, 8.0], [6.0, 8.0]],
                &mut ids,
            )
            .unwrap();
        let composition = project.composition(comp).unwrap();
        let fill_index = composition
            .layers
            .iter()
            .position(|layer| layer.id == output.layer_id)
            .unwrap();
        let source_index = composition
            .layers
            .iter()
            .position(|layer| layer.id == source)
            .unwrap();
        assert!(fill_index < source_index);
        assert!(output.frames[0].get(8, 6)[1] > output.frames[0].get(8, 6)[0]);
    }
}
