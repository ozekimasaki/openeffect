use std::path::{Path, PathBuf};

use effect_api::{Frame, Frame16, Frame32, Frame8};
use io_api::FootageReference;
use model::{BitsPerChannel, ItemId, Time};

use super::{check_int_range, check_range, App, CompRec, FootageRec, ItemBody, ItemRec, SourceKind, SourceRec};
use crate::enums::*;
use crate::error::{ScriptError, ScriptResult};
use crate::enums::GuidePositionType;
use crate::values::{png_info, Guide, GuideOptions};

impl App {
    pub fn add_comp(
        &mut self,
        name: &str,
        width: i32,
        height: i32,
        pixel_aspect: f64,
        duration: f64,
        frame_rate: f64,
    ) -> ScriptResult<ItemId> {
        self.add_comp_in(self.root, name, width, height, pixel_aspect, duration, frame_rate)
    }

    pub fn add_comp_in(
        &mut self,
        folder: ItemId,
        name: &str,
        width: i32,
        height: i32,
        pixel_aspect: f64,
        duration: f64,
        frame_rate: f64,
    ) -> ScriptResult<ItemId> {
        check_int_range(width, 4, 30000, "width")?;
        check_int_range(height, 4, 30000, "height")?;
        check_range(pixel_aspect, 0.01, 100.0, "pixelAspect")?;
        check_range(duration, 0.0, 10800.0, "duration")?;
        check_range(frame_rate, 1.0, 99.0, "frameRate")?;
        let id = self.alloc_item();
        let marker = self.alloc_marker_property();
        let comp = CompRec {
            width: width as u32,
            height: height as u32,
            pixel_aspect,
            frame_rate,
            duration,
            time: 0.0,
            work_start: 0.0,
            work_duration: duration,
            layers: Vec::new(),
            bg: [0.0, 0.0, 0.0],
            display_start_frame: self.display_start_frame,
            display_start_time: 0.0,
            draft3d: false,
            drop_frame: false,
            frame_blending: false,
            hide_shy: false,
            motion_blur: false,
            motion_blur_adaptive: 128,
            motion_blur_samples: 16,
            shutter_angle: 180.0,
            shutter_phase: 0.0,
            preserve_nested_frame_rate: false,
            preserve_nested_resolution: false,
            renderer: "Classic 3D".to_string(),
            resolution: [1, 1],
            counters: false,
            template_name: name.to_string(),
            controllers: Vec::new(),
            marker,
        };
        self.insert_item(
            id,
            folder,
            ItemRec {
                name: name.to_string(),
                comment: String::new(),
                label: 0,
                selected: false,
                parent: folder,
                guides: Vec::new(),
                body: ItemBody::Comp(comp),
            },
        )?;
        Ok(id)
    }

    pub fn add_folder(&mut self, name: &str) -> ScriptResult<ItemId> {
        self.add_folder_in(self.root, name)
    }

    pub fn add_folder_in(&mut self, folder: ItemId, name: &str) -> ScriptResult<ItemId> {
        let id = self.alloc_item();
        self.insert_item(
            id,
            folder,
            ItemRec {
                name: name.to_string(),
                comment: String::new(),
                label: 0,
                selected: false,
                parent: folder,
                guides: Vec::new(),
                body: ItemBody::Folder { children: Vec::new() },
            },
        )?;
        Ok(id)
    }

    pub(super) fn add_placeholder(
        &mut self,
        folder: ItemId,
        name: &str,
        width: u32,
        height: u32,
        frame_rate: f64,
        duration: f64,
    ) -> ScriptResult<ItemId> {
        let mut source = SourceRec::still(name, width, height, frame_rate);
        source.kind = SourceKind::Placeholder;
        source.duration = duration;
        source.native_duration = duration;
        source.is_still = duration <= 0.0;
        source.missing = true;
        source.has_video = true;
        self.insert_footage(folder, name, source)
    }

    pub(super) fn import_footage_file(&mut self, path: &Path, options: &crate::values::ImportOptions) -> ScriptResult<ItemId> {
        let info = std::fs::read(path).ok().as_deref().and_then(png_info);
        let (width, height, alpha) = info.unwrap_or((1920, 1080, false));
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("footage")
            .to_string();
        let mut source = SourceRec::still(&name, width, height, 30.0);
        source.path = Some(path.to_path_buf());
        source.reference = Some(if path.exists() {
            FootageReference::linked(ItemId::new(0), path)
        } else {
            source.missing = true;
            FootageReference::unlinked(ItemId::new(0), path)
        });
        source.has_alpha = alpha;
        source.missing = !path.exists();
        if options.sequence || crate::values::numbered_name(&path.display().to_string()).numbered && options.sequence {
            source.sequence = true;
            source.kind = SourceKind::Sequence;
            source.sequence_paths = sequence_paths(path, options);
            source.is_still = false;
            let frames = source.sequence_paths.len().max(1) as f64;
            source.duration = frames / source.frame_rate;
            source.native_duration = source.duration;
            if let (Some(start), Some(end)) = (options.range_start, options.range_end) {
                let count = (end - start + 1).max(0) as f64;
                source.duration = count / source.frame_rate;
                source.native_duration = source.duration;
            }
        }
        self.insert_footage(self.root, &name, source)
    }

    pub(crate) fn insert_footage(&mut self, folder: ItemId, name: &str, source: SourceRec) -> ScriptResult<ItemId> {
        let id = self.alloc_item();
        if let Some(reference) = &mut source_reference_fix(id, &source) {
            let _ = reference;
        }
        let mut source = source;
        if let Some(path) = &source.path {
            source.reference = Some(if source.missing {
                FootageReference::unlinked(id, path)
            } else {
                FootageReference::linked(id, path)
            });
        }
        let eval = solid_frame(self.bits, source.width, source.height, source.color);
        self.insert_item(
            id,
            folder,
            ItemRec {
                name: name.to_string(),
                comment: String::new(),
                label: 0,
                selected: false,
                parent: folder,
                guides: Vec::new(),
                body: ItemBody::Footage(FootageRec {
                    main: source,
                    proxy: None,
                    use_proxy: false,
                    eval,
                }),
            },
        )?;
        Ok(id)
    }

    pub(super) fn insert_solid(
        &mut self,
        folder: ItemId,
        name: &str,
        color: [f64; 3],
        width: u32,
        height: u32,
        pixel_aspect: f64,
        duration: f64,
        frame_rate: f64,
    ) -> ScriptResult<ItemId> {
        let mut source = SourceRec::still(name, width, height, frame_rate);
        source.kind = SourceKind::Solid;
        source.color = color;
        source.duration = duration;
        source.native_duration = duration;
        source.is_still = true;
        source.pixel_aspect = pixel_aspect;
        source.has_video = true;
        source.has_audio = false;
        let id = self.alloc_item();
        let eval = solid_frame(self.bits, width, height, color);
        self.insert_item(
            id,
            folder,
            ItemRec {
                name: name.to_string(),
                comment: String::new(),
                label: 0,
                selected: false,
                parent: folder,
                guides: Vec::new(),
                body: ItemBody::Footage(FootageRec {
                    main: source,
                    proxy: None,
                    use_proxy: false,
                    eval,
                }),
            },
        )?;
        Ok(id)
    }

    fn insert_item(&mut self, id: ItemId, folder: ItemId, item: ItemRec) -> ScriptResult<()> {
        if !matches!(self.item(folder)?.body, ItemBody::Folder { .. }) && folder != self.root {
            return Err(ScriptError::invalid("親がフォルダではない"));
        }
        self.items.insert(id, item);
        self.order.push(id);
        if let ItemBody::Folder { children } = &mut self.item_mut(folder)?.body {
            children.push(id);
        }
        self.touch();
        Ok(())
    }

    pub fn item_name(&self, id: ItemId) -> ScriptResult<String> {
        Ok(self.item(id)?.name.clone())
    }

    pub fn set_item_name(&mut self, id: ItemId, name: &str) -> ScriptResult<()> {
        self.item_mut(id)?.name = name.to_string();
        if let ItemBody::Comp(comp) = &mut self.item_mut(id)?.body {
            if comp.template_name.is_empty() {
                comp.template_name = name.to_string();
            }
        }
        self.touch();
        Ok(())
    }

    pub fn item_comment(&self, id: ItemId) -> ScriptResult<String> {
        Ok(self.item(id)?.comment.clone())
    }

    pub fn set_item_comment(&mut self, id: ItemId, comment: &str) -> ScriptResult<()> {
        self.item_mut(id)?.comment = comment.to_string();
        self.touch();
        Ok(())
    }

    pub fn item_id_value(&self, id: ItemId) -> u64 {
        id.get()
    }

    pub fn dynamic_link_guid(&self, _id: ItemId) -> ScriptResult<String> {
        Ok(String::new())
    }

    pub fn item_label(&self, id: ItemId) -> ScriptResult<i32> {
        Ok(self.item(id)?.label)
    }

    pub fn set_item_label(&mut self, id: ItemId, label: i32) -> ScriptResult<()> {
        check_int_range(label, 0, 16, "label")?;
        self.item_mut(id)?.label = label;
        self.touch();
        Ok(())
    }

    pub fn item_selected(&self, id: ItemId) -> ScriptResult<bool> {
        Ok(self.item(id)?.selected)
    }

    pub fn set_item_selected(&mut self, id: ItemId, selected: bool) -> ScriptResult<()> {
        self.item_mut(id)?.selected = selected;
        if selected {
            self.viewer = Some(crate::values::Viewer {
                item: id,
                layer: None,
                kind: match &self.item(id)?.body {
                    ItemBody::Comp(_) => crate::values::ViewerKind::Composition,
                    ItemBody::Footage(_) => crate::values::ViewerKind::Footage,
                    ItemBody::Folder { .. } => crate::values::ViewerKind::Composition,
                },
            });
        }
        Ok(())
    }

    pub fn item_type_name(&self, id: ItemId) -> ScriptResult<&'static str> {
        Ok(match &self.item(id)?.body {
            ItemBody::Comp(_) => ItemKindName::Composition.ja(),
            ItemBody::Folder { .. } => ItemKindName::Folder.ja(),
            ItemBody::Footage(_) => ItemKindName::Footage.ja(),
        })
    }

    pub fn parent_folder(&self, id: ItemId) -> ScriptResult<ItemId> {
        if id == self.root {
            return Ok(self.root);
        }
        Ok(self.item(id)?.parent)
    }

    pub fn set_parent_folder(&mut self, id: ItemId, folder: ItemId) -> ScriptResult<()> {
        if id == self.root {
            return Err(ScriptError::invalid("ルートは移動できない"));
        }
        if id == folder || self.is_descendant(folder, id) {
            return Err(ScriptError::invalid("フォルダの循環"));
        }
        if !matches!(self.item(folder)?.body, ItemBody::Folder { .. }) {
            return Err(ScriptError::invalid("親がフォルダではない"));
        }
        let old = self.item(id)?.parent;
        if let ItemBody::Folder { children } = &mut self.item_mut(old)?.body {
            children.retain(|child| *child != id);
        }
        self.item_mut(id)?.parent = folder;
        if let ItemBody::Folder { children } = &mut self.item_mut(folder)?.body {
            children.push(id);
        }
        self.touch();
        Ok(())
    }

    fn is_descendant(&self, id: ItemId, ancestor: ItemId) -> bool {
        let mut current = id;
        for _ in 0..64 {
            if current == ancestor {
                return true;
            }
            let parent = self.items.get(&current).map(|item| item.parent).unwrap_or(current);
            if parent == current {
                return false;
            }
            current = parent;
        }
        false
    }

    pub fn guides(&self, id: ItemId) -> ScriptResult<Vec<Guide>> {
        Ok(self.item(id)?.guides.clone())
    }

    pub fn add_guide(&mut self, id: ItemId, orientation: i32, position: f64) -> ScriptResult<i32> {
        let guide = Guide::at(orientation, position);
        let item = self.item_mut(id)?;
        item.guides.push(guide);
        let index = item.guides.len() as i32 - 1;
        self.touch();
        Ok(index)
    }

    pub fn add_guide_options(&mut self, id: ItemId, options: &GuideOptions) -> ScriptResult<i32> {
        let orientation = match options.orientation.unwrap_or(GuideOrientationType::Horizontal) {
            GuideOrientationType::Horizontal => 0,
            GuideOrientationType::Vertical => 1,
        };
        let mut position = options.position.unwrap_or(0.0);
        let position_type = options.position_type.unwrap_or(GuidePositionType::Pixels);
        if position_type == GuidePositionType::Percentage {
            let span = self.item_span(id, orientation)?;
            position = span * position / 100.0;
        }
        let index = self.add_guide(id, orientation, position)?;
        let guide = &mut self.item_mut(id)?.guides[index as usize];
        guide.position_type = position_type;
        if let Some(color) = options.color {
            guide.color = color;
        }
        if let Some(pinned) = options.pinned {
            let span = self.item_span(id, orientation).unwrap_or(0.0);
            let guide = &mut self.item_mut(id)?.guides[index as usize];
            guide.pinned = pinned;
            guide.pin_anchor = span - guide.position;
        }
        Ok(index)
    }

    fn item_span(&self, id: ItemId, orientation: i32) -> ScriptResult<f64> {
        match &self.item(id)?.body {
            ItemBody::Comp(comp) => Ok(if orientation == 1 {
                comp.width as f64
            } else {
                comp.height as f64
            }),
            ItemBody::Footage(footage) => Ok(if orientation == 1 {
                footage.main.width as f64
            } else {
                footage.main.height as f64
            }),
            ItemBody::Folder { .. } => Ok(0.0),
        }
    }

    pub fn guide_at(&self, id: ItemId, index: i32) -> ScriptResult<Guide> {
        self.item(id)?
            .guides
            .get(index as usize)
            .cloned()
            .ok_or_else(|| ScriptError::range("guide"))
    }

    pub fn remove_guide(&mut self, id: ItemId, index: i32) -> ScriptResult<()> {
        let guides = &mut self.item_mut(id)?.guides;
        if index < 0 || index as usize >= guides.len() {
            return Err(ScriptError::range("guide"));
        }
        guides.remove(index as usize);
        self.touch();
        Ok(())
    }

    pub fn set_guide(&mut self, id: ItemId, position: f64, index: i32) -> ScriptResult<()> {
        let guide = self
            .item_mut(id)?
            .guides
            .get_mut(index as usize)
            .ok_or_else(|| ScriptError::range("guide"))?;
        guide.position = position;
        self.touch();
        Ok(())
    }

    pub fn set_guide_options(&mut self, id: ItemId, index: i32, options: &GuideOptions) -> ScriptResult<()> {
        if let Some(position) = options.position {
            self.set_guide(id, position, index)?;
        }
        let guide = self
            .item_mut(id)?
            .guides
            .get_mut(index as usize)
            .ok_or_else(|| ScriptError::range("guide"))?;
        if let Some(orientation) = options.orientation {
            guide.orientation = match orientation {
                GuideOrientationType::Horizontal => 0,
                GuideOrientationType::Vertical => 1,
            };
        }
        if let Some(color) = options.color {
            guide.color = color;
        }
        if let Some(pinned) = options.pinned {
            guide.pinned = pinned;
        }
        if let Some(position_type) = options.position_type {
            guide.position_type = position_type;
        }
        self.touch();
        Ok(())
    }

    pub fn remove_item(&mut self, id: ItemId) -> ScriptResult<()> {
        if id == self.root {
            return Err(ScriptError::invalid("ルートは消せない"));
        }
        let children = match &self.item(id)?.body {
            ItemBody::Folder { children } => children.clone(),
            _ => Vec::new(),
        };
        for child in children {
            self.remove_item(child)?;
        }
        if let ItemBody::Comp(comp) = &self.item(id)?.body {
            let layers = comp.layers.clone();
            for layer in layers {
                self.remove_layer(layer)?;
            }
        }
        let parent = self.item(id)?.parent;
        if let Some(ItemBody::Folder { children }) = self.items.get_mut(&parent).map(|item| &mut item.body) {
            children.retain(|child| *child != id);
        }
        self.order.retain(|item| *item != id);
        self.items.remove(&id);
        self.touch();
        Ok(())
    }

    pub fn folder_num_items(&self, id: ItemId) -> ScriptResult<i32> {
        match &self.item(id)?.body {
            ItemBody::Folder { children } => Ok(children.len() as i32),
            _ => Err(ScriptError::invalid("フォルダではない")),
        }
    }

    pub fn folder_item(&self, id: ItemId, index: i32) -> ScriptResult<ItemId> {
        match &self.item(id)?.body {
            ItemBody::Folder { children } => {
                if index < 1 {
                    return Err(ScriptError::range("folder item"));
                }
                children
                    .get((index - 1) as usize)
                    .copied()
                    .ok_or_else(|| ScriptError::range("folder item"))
            }
            _ => Err(ScriptError::invalid("フォルダではない")),
        }
    }

    pub fn folder_children(&self, id: ItemId) -> ScriptResult<Vec<ItemId>> {
        match &self.item(id)?.body {
            ItemBody::Folder { children } => Ok(children.clone()),
            _ => Err(ScriptError::invalid("フォルダではない")),
        }
    }

    pub(super) fn retarget_footage(&mut self, from: ItemId, to: ItemId) -> ScriptResult<()> {
        let layers: Vec<model::LayerId> = self.layers.keys().copied().collect();
        for layer in layers {
            if self.layer(layer)?.source_id() == Some(from) {
                self.layer_mut(layer)?.set_source(to);
            }
        }
        Ok(())
    }

    pub(super) fn used_footage(&self) -> Vec<ItemId> {
        let mut used = Vec::new();
        for layer in self.layers.values() {
            if let Some(source) = layer.source_id() {
                if !used.contains(&source) {
                    used.push(source);
                }
            }
        }
        for item in self.items.values() {
            if let ItemBody::Footage(footage) = &item.body {
                if let Some(proxy) = &footage.proxy {
                    let _ = proxy;
                }
            }
        }
        used
    }

    pub fn av_width(&self, id: ItemId) -> ScriptResult<i32> {
        Ok(match &self.item(id)?.body {
            ItemBody::Comp(comp) => comp.width as i32,
            ItemBody::Footage(footage) => active_source(footage).width as i32,
            ItemBody::Folder { .. } => return Err(ScriptError::invalid("幅が無い")),
        })
    }

    pub fn set_av_width(&mut self, id: ItemId, width: i32) -> ScriptResult<()> {
        check_int_range(width, 4, 30000, "width")?;
        let old_width = self.av_width(id)? as f64;
        match &mut self.item_mut(id)?.body {
            ItemBody::Comp(comp) => {
                comp.width = width as u32;
            }
            ItemBody::Footage(_) => return Err(ScriptError::readonly("フッテージの幅")),
            ItemBody::Folder { .. } => return Err(ScriptError::invalid("幅が無い")),
        }
        self.rescale_pinned_guides(id, 1, old_width, width as f64);
        self.touch();
        Ok(())
    }

    pub fn av_height(&self, id: ItemId) -> ScriptResult<i32> {
        Ok(match &self.item(id)?.body {
            ItemBody::Comp(comp) => comp.height as i32,
            ItemBody::Footage(footage) => active_source(footage).height as i32,
            ItemBody::Folder { .. } => return Err(ScriptError::invalid("高さが無い")),
        })
    }

    pub fn set_av_height(&mut self, id: ItemId, height: i32) -> ScriptResult<()> {
        check_int_range(height, 4, 30000, "height")?;
        let old_height = self.av_height(id)? as f64;
        match &mut self.item_mut(id)?.body {
            ItemBody::Comp(comp) => {
                comp.height = height as u32;
            }
            ItemBody::Footage(_) => return Err(ScriptError::readonly("フッテージの高さ")),
            ItemBody::Folder { .. } => return Err(ScriptError::invalid("高さが無い")),
        }
        self.rescale_pinned_guides(id, 0, old_height, height as f64);
        self.touch();
        Ok(())
    }

    fn rescale_pinned_guides(&mut self, id: ItemId, orientation: i32, _old: f64, new_span: f64) {
        if let Ok(item) = self.item_mut(id) {
            for guide in &mut item.guides {
                if guide.pinned && guide.orientation == orientation {
                    guide.position = new_span - guide.pin_anchor;
                }
            }
        }
    }

    pub fn av_duration(&self, id: ItemId) -> ScriptResult<f64> {
        Ok(match &self.item(id)?.body {
            ItemBody::Comp(comp) => comp.duration,
            ItemBody::Footage(footage) => active_source(footage).duration,
            ItemBody::Folder { .. } => return Err(ScriptError::invalid("尺が無い")),
        })
    }

    pub fn set_comp_duration(&mut self, id: ItemId, duration: f64) -> ScriptResult<()> {
        check_range(duration, 0.0, 10800.0, "duration")?;
        self.require_comp_mut(id)?.duration = duration;
        self.touch();
        Ok(())
    }

    pub fn av_frame_rate(&self, id: ItemId) -> ScriptResult<f64> {
        Ok(match &self.item(id)?.body {
            ItemBody::Comp(comp) => comp.frame_rate,
            ItemBody::Footage(footage) => active_source(footage).frame_rate,
            ItemBody::Folder { .. } => return Err(ScriptError::invalid("フレームレートが無い")),
        })
    }

    pub fn set_comp_frame_rate(&mut self, id: ItemId, frame_rate: f64) -> ScriptResult<()> {
        check_range(frame_rate, 1.0, 99.0, "frameRate")?;
        self.require_comp_mut(id)?.frame_rate = frame_rate;
        self.touch();
        Ok(())
    }

    pub fn av_frame_duration(&self, id: ItemId) -> ScriptResult<f64> {
        let rate = self.av_frame_rate(id)?;
        Ok(if rate <= 0.0 { 0.0 } else { 1.0 / rate })
    }

    pub fn set_comp_frame_duration(&mut self, id: ItemId, frame_duration: f64) -> ScriptResult<()> {
        if frame_duration <= 0.0 {
            return Err(ScriptError::range("frameDuration"));
        }
        self.set_comp_frame_rate(id, 1.0 / frame_duration)
    }

    pub fn av_pixel_aspect(&self, id: ItemId) -> ScriptResult<f64> {
        Ok(match &self.item(id)?.body {
            ItemBody::Comp(comp) => comp.pixel_aspect,
            ItemBody::Footage(footage) => active_source(footage).pixel_aspect,
            ItemBody::Folder { .. } => return Err(ScriptError::invalid("ピクセルアスペクトが無い")),
        })
    }

    pub fn set_comp_pixel_aspect(&mut self, id: ItemId, value: f64) -> ScriptResult<()> {
        check_range(value, 0.01, 100.0, "pixelAspect")?;
        self.require_comp_mut(id)?.pixel_aspect = value;
        self.touch();
        Ok(())
    }

    pub fn footage_missing(&self, id: ItemId) -> ScriptResult<bool> {
        match &self.item(id)?.body {
            ItemBody::Footage(footage) => Ok(active_source(footage).missing || active_source(footage).kind == SourceKind::Placeholder),
            ItemBody::Comp(_) => Ok(false),
            ItemBody::Folder { .. } => Err(ScriptError::invalid("フッテージではない")),
        }
    }

    pub fn has_audio(&self, id: ItemId) -> ScriptResult<bool> {
        match &self.item(id)?.body {
            ItemBody::Footage(footage) => Ok(active_source(footage).has_audio),
            ItemBody::Comp(_) => Ok(false),
            ItemBody::Folder { .. } => Ok(false),
        }
    }

    pub fn has_video(&self, id: ItemId) -> ScriptResult<bool> {
        match &self.item(id)?.body {
            ItemBody::Footage(footage) => Ok(active_source(footage).has_video),
            ItemBody::Comp(_) => Ok(true),
            ItemBody::Folder { .. } => Ok(false),
        }
    }

    pub fn is_media_replacement_compatible(&self, id: ItemId) -> ScriptResult<bool> {
        Ok(matches!(self.item(id)?.body, ItemBody::Footage(_)))
    }

    pub fn item_time(&self, id: ItemId) -> ScriptResult<f64> {
        match &self.item(id)?.body {
            ItemBody::Comp(comp) => Ok(comp.time),
            ItemBody::Footage(_) => Ok(0.0),
            ItemBody::Folder { .. } => Err(ScriptError::invalid("時刻が無い")),
        }
    }

    pub fn set_item_time(&mut self, id: ItemId, time: f64) -> ScriptResult<()> {
        self.require_comp_mut(id)?.time = time;
        Ok(())
    }

    pub fn used_in(&self, id: ItemId) -> Vec<ItemId> {
        let mut comps = Vec::new();
        for (item_id, item) in &self.items {
            if let ItemBody::Comp(comp) = &item.body {
                let referenced = comp.layers.iter().any(|layer| {
                    self.layers.get(layer).and_then(|layer| layer.source_id()) == Some(id)
                });
                if referenced {
                    comps.push(*item_id);
                }
            }
        }
        comps
    }

    pub fn use_proxy(&self, id: ItemId) -> ScriptResult<bool> {
        match &self.item(id)?.body {
            ItemBody::Footage(footage) => Ok(footage.use_proxy),
            _ => Err(ScriptError::invalid("プロキシが無い")),
        }
    }

    pub fn set_use_proxy(&mut self, id: ItemId, value: bool) -> ScriptResult<()> {
        let footage = self.require_footage_mut(id)?;
        if value && footage.proxy.is_none() {
            return Err(ScriptError::invalid("プロキシが無い"));
        }
        footage.use_proxy = value;
        self.touch();
        Ok(())
    }

    pub fn proxy_is_some(&self, id: ItemId) -> ScriptResult<bool> {
        Ok(self.require_footage(id)?.proxy.is_some())
    }

    pub fn set_proxy_file(&mut self, id: ItemId, file: &Path) -> ScriptResult<()> {
        let info = std::fs::read(file).ok().as_deref().and_then(png_info);
        let (width, height, alpha) = info.unwrap_or((64, 64, false));
        let mut source = SourceRec::still("proxy", width, height, 30.0);
        source.path = Some(file.to_path_buf());
        source.has_alpha = alpha;
        source.missing = !file.exists();
        source.reference = Some(FootageReference::linked(id, file));
        let footage = self.require_footage_mut(id)?;
        footage.proxy = Some(source);
        footage.use_proxy = true;
        self.touch();
        Ok(())
    }

    pub fn set_proxy_to_none(&mut self, id: ItemId) -> ScriptResult<()> {
        let footage = self.require_footage_mut(id)?;
        footage.proxy = None;
        footage.use_proxy = false;
        self.touch();
        Ok(())
    }

    pub fn set_proxy_placeholder(
        &mut self,
        id: ItemId,
        name: &str,
        width: i32,
        height: i32,
        frame_rate: f64,
        duration: f64,
    ) -> ScriptResult<()> {
        check_int_range(width, 4, 30000, "width")?;
        check_int_range(height, 4, 30000, "height")?;
        let mut source = SourceRec::still(name, width as u32, height as u32, frame_rate);
        source.kind = SourceKind::Placeholder;
        source.duration = duration;
        source.missing = true;
        let footage = self.require_footage_mut(id)?;
        footage.proxy = Some(source);
        footage.use_proxy = true;
        self.touch();
        Ok(())
    }

    pub fn set_proxy_sequence(&mut self, id: ItemId, file: &Path, force_alphabetical: bool) -> ScriptResult<()> {
        let mut source = SourceRec::still("proxy", 32, 32, 30.0);
        source.sequence = true;
        source.kind = SourceKind::Sequence;
        source.path = Some(file.to_path_buf());
        let mut paths = sequence_paths(
            file,
            &crate::values::ImportOptions {
                force_alphabetical,
                sequence: true,
                ..crate::values::ImportOptions::new(Some(file.to_path_buf()))
            },
        );
        if force_alphabetical {
            paths.sort();
        }
        source.sequence_paths = paths;
        source.is_still = false;
        source.duration = source.sequence_paths.len().max(1) as f64 / source.frame_rate;
        let footage = self.require_footage_mut(id)?;
        footage.proxy = Some(source);
        footage.use_proxy = true;
        self.touch();
        Ok(())
    }

    pub fn set_proxy_solid(
        &mut self,
        id: ItemId,
        color: [f64; 3],
        name: &str,
        width: i32,
        height: i32,
        pixel_aspect: f64,
    ) -> ScriptResult<()> {
        check_int_range(width, 4, 30000, "width")?;
        check_int_range(height, 4, 30000, "height")?;
        check_range(pixel_aspect, 0.01, 100.0, "pixelAspect")?;
        let mut source = SourceRec::still(name, width as u32, height as u32, 30.0);
        source.kind = SourceKind::Solid;
        source.color = color;
        source.pixel_aspect = pixel_aspect;
        let footage = self.require_footage_mut(id)?;
        footage.proxy = Some(source);
        footage.use_proxy = true;
        self.touch();
        Ok(())
    }

    pub fn main_source_kind(&self, id: ItemId) -> ScriptResult<SourceKind> {
        Ok(self.require_footage(id)?.main.kind)
    }

    pub fn main_source_file(&self, id: ItemId) -> ScriptResult<Option<PathBuf>> {
        Ok(self.require_footage(id)?.main.path.clone())
    }

    pub fn source_file(&self, id: ItemId, proxy: bool) -> ScriptResult<Option<PathBuf>> {
        let footage = self.require_footage(id)?;
        let source = if proxy {
            footage.proxy.as_ref().ok_or_else(|| ScriptError::missing("proxy"))?
        } else {
            &footage.main
        };
        Ok(source.path.clone())
    }

    pub fn missing_footage_path(&self, id: ItemId, proxy: bool) -> ScriptResult<String> {
        let footage = self.require_footage(id)?;
        let source = if proxy {
            footage.proxy.as_ref().ok_or_else(|| ScriptError::missing("proxy"))?
        } else {
            &footage.main
        };
        if source.missing {
            Ok(source.path.as_ref().map(|path| path.display().to_string()).unwrap_or_default())
        } else {
            Ok(String::new())
        }
    }

    pub fn reload_source(&mut self, id: ItemId, proxy: bool) -> ScriptResult<()> {
        let path = self.source_file(id, proxy)?.ok_or_else(|| ScriptError::missing("file"))?;
        if let Some((width, height, alpha)) = std::fs::read(&path).ok().as_deref().and_then(png_info) {
            let footage = self.require_footage_mut(id)?;
            let source = if proxy { footage.proxy.as_mut().unwrap() } else { &mut footage.main };
            source.width = width;
            source.height = height;
            source.has_alpha = alpha;
            source.missing = false;
        }
        self.touch();
        Ok(())
    }

    pub fn eval_bits(&self, id: ItemId) -> ScriptResult<u8> {
        Ok(self.require_footage(id)?.eval.bits().bits())
    }

    pub fn solid_color(&self, id: ItemId) -> ScriptResult<[f64; 3]> {
        Ok(self.require_footage(id)?.main.color)
    }

    pub fn set_solid_color(&mut self, id: ItemId, color: [f64; 3]) -> ScriptResult<()> {
        let bits = self.bits;
        let footage = self.require_footage_mut(id)?;
        if footage.main.kind != SourceKind::Solid {
            return Err(ScriptError::invalid("ソリッドではない"));
        }
        footage.main.color = color;
        let width = footage.main.width;
        let height = footage.main.height;
        footage.eval = solid_frame(bits, width, height, color);
        self.touch();
        Ok(())
    }
}

fn source_reference_fix(id: ItemId, source: &SourceRec) -> Option<FootageReference> {
    let path = source.path.clone()?;
    Some(if source.missing {
        FootageReference::unlinked(id, path)
    } else {
        FootageReference::linked(id, path)
    })
}

fn active_source(footage: &FootageRec) -> &SourceRec {
    if footage.use_proxy {
        footage.proxy.as_ref().unwrap_or(&footage.main)
    } else {
        &footage.main
    }
}

pub(crate) fn solid_frame(bits: BitsPerChannel, width: u32, height: u32, _color: [f64; 3]) -> Frame {
    match bits {
        BitsPerChannel::Bpc8 => Frame::Bpc8(Frame8::new(width, height, 4, 255)),
        BitsPerChannel::Bpc16 => Frame::Bpc16(Frame16::new(width, height, 4, 32768)),
        BitsPerChannel::Bpc32 => Frame::Bpc32(Frame32::new(width, height, 4, 1.0)),
    }
}

impl SourceRec {
    pub(super) fn still(name: &str, width: u32, height: u32, frame_rate: f64) -> Self {
        Self {
            kind: SourceKind::File,
            reference: None,
            path: None,
            width,
            height,
            duration: 0.0,
            native_duration: 0.0,
            frame_rate,
            native_frame_rate: frame_rate,
            conform_frame_rate: 0.0,
            pixel_aspect: 1.0,
            color: [0.0, 0.0, 0.0],
            has_audio: false,
            has_video: true,
            has_alpha: false,
            alpha_mode: AlphaMode::Ignore,
            field_separation: FieldSeparationType::Off,
            high_quality_fields: false,
            invert_alpha: false,
            is_still: true,
            loops: 1,
            premul_color: [0.0, 0.0, 0.0],
            pulldown: PulldownPhase::Off,
            sequence: false,
            sequence_paths: Vec::new(),
            missing: false,
            name: name.to_string(),
        }
    }
}

pub(crate) fn sequence_paths(path: &Path, options: &crate::values::ImportOptions) -> Vec<PathBuf> {
    let Some(parent) = path.parent() else {
        return vec![path.to_path_buf()];
    };
    let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
    let numbered = crate::values::numbered_name(file_name);
    if !numbered.numbered {
        return vec![path.to_path_buf()];
    }
    let stem = file_name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(file_name);
    let digits = stem.chars().rev().take_while(|ch| ch.is_ascii_digit()).count();
    let prefix: String = stem.chars().take(stem.len() - digits).collect();
    let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    let mut paths = Vec::new();
    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(&prefix) && (ext.is_empty() || name.ends_with(ext)) {
                let number = crate::values::numbered_name(&name);
                if number.numbered {
                    let in_range = match (options.range_start, options.range_end) {
                        (Some(start), Some(end)) => number.number >= start && number.number <= end,
                        (Some(start), None) => number.number >= start,
                        (None, Some(end)) => number.number <= end,
                        (None, None) => true,
                    };
                    if in_range {
                        paths.push(entry.path());
                    }
                }
            }
        }
    }
    if options.force_alphabetical {
        paths.sort();
    }
    if paths.is_empty() {
        paths.push(path.to_path_buf());
    }
    let _ = Time::from_seconds(0.0);
    paths
}
