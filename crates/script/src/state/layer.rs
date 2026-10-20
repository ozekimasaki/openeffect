use model::{BlendMode, ItemId, LayerId, TrackMatte, TrackMatteMode};

use super::{check_int_range, check_range, App, ItemBody, LayerKind, LayerRec};
use crate::enums::*;
use crate::error::{ScriptError, ScriptResult};
use crate::text::TextDocument;
use crate::values::{Guide, GuideOptions};

impl LayerRec {
    pub(super) fn source_id(&self) -> Option<ItemId> {
        match self.kind {
            LayerKind::Av { source } => source,
            _ => None,
        }
    }

    pub(super) fn set_source(&mut self, source: ItemId) {
        if let LayerKind::Av { source: slot } = &mut self.kind {
            *slot = Some(source);
        }
    }
}

impl App {
    pub fn add_layer_from_item(&mut self, comp: ItemId, source: ItemId, duration: Option<f64>) -> ScriptResult<LayerId> {
        let (width, height, has_audio, has_video, span, still) = match &self.item(source)?.body {
            ItemBody::Footage(footage) => (
                footage.main.width,
                footage.main.height,
                footage.main.has_audio,
                footage.main.has_video,
                if footage.main.duration <= 0.0 {
                    self.require_comp(comp)?.duration
                } else {
                    footage.main.duration
                },
                footage.main.is_still,
            ),
            ItemBody::Comp(source_comp) => (
                source_comp.width,
                source_comp.height,
                false,
                true,
                source_comp.duration,
                false,
            ),
            ItemBody::Folder { .. } => return Err(ScriptError::invalid("フォルダはソースにできない")),
        };
        let duration = duration.unwrap_or(span);
        let name = self.item_name(source)?;
        let kind = match &self.item(source)?.body {
            ItemBody::Comp(_) => LayerKind::Av { source: Some(source) },
            _ => LayerKind::Av { source: Some(source) },
        };
        let _ = (width, height);
        self.push_layer(comp, &name, kind, duration, has_video, has_audio, !still)
    }

    pub fn add_null(&mut self, comp: ItemId, duration: Option<f64>) -> ScriptResult<LayerId> {
        let duration = duration.unwrap_or(self.require_comp(comp)?.duration);
        self.push_layer(comp, "Null", LayerKind::Null, duration, true, false, false)
    }

    pub fn add_shape(&mut self, comp: ItemId) -> ScriptResult<LayerId> {
        let duration = self.require_comp(comp)?.duration;
        self.push_layer(comp, "Shape", LayerKind::Shape, duration, true, false, false)
    }

    pub fn add_text_layer(&mut self, comp: ItemId, text: &str) -> ScriptResult<LayerId> {
        let duration = self.require_comp(comp)?.duration;
        let id = self.push_layer(comp, "Text", LayerKind::Text, duration, true, false, false)?;
        self.layer_mut(id)?.text = Some(TextDocument::point(text));
        self.attach_text_property(id)?;
        Ok(id)
    }

    pub fn add_box_text(&mut self, comp: ItemId, width: f64, height: f64, vertical: bool) -> ScriptResult<LayerId> {
        let duration = self.require_comp(comp)?.duration;
        let id = self.push_layer(comp, "Text", LayerKind::Text, duration, true, false, false)?;
        self.layer_mut(id)?.text = Some(TextDocument::box_text("", width, height, vertical));
        self.attach_text_property(id)?;
        Ok(id)
    }

    pub fn add_camera(&mut self, comp: ItemId, name: &str, center: [f64; 2]) -> ScriptResult<LayerId> {
        let duration = self.require_comp(comp)?.duration;
        let id = self.push_layer(comp, name, LayerKind::Camera, duration, false, false, false)?;
        self.layer_mut(id)?.point_of_interest = [center[0], center[1], 0.0];
        self.layer_mut(id)?.camera_nodes = 2;
        self.attach_camera_properties(id)?;
        self.set_vec2(self.find_layer_prop(id, "Position").unwrap(), center)?;
        Ok(id)
    }

    pub fn add_light(&mut self, comp: ItemId, name: &str, center: [f64; 2]) -> ScriptResult<LayerId> {
        let duration = self.require_comp(comp)?.duration;
        let id = self.push_layer(comp, name, LayerKind::Light, duration, false, false, false)?;
        self.layer_mut(id)?.light_type = LightType::Point;
        self.layer_mut(id)?.point_of_interest = [center[0], center[1], 0.0];
        self.attach_light_properties(id)?;
        self.set_vec2(self.find_layer_prop(id, "Position").unwrap(), center)?;
        Ok(id)
    }

    pub fn add_parametric_mesh(&mut self, comp: ItemId, name: &str, kind: ParametricMeshType) -> ScriptResult<LayerId> {
        let duration = self.require_comp(comp)?.duration;
        let id = self.push_layer(comp, name, LayerKind::Mesh, duration, true, false, false)?;
        self.layer_mut(id)?.mesh = kind;
        self.layer_mut(id)?.three_d = true;
        Ok(id)
    }

    pub fn add_solid_layer(
        &mut self,
        comp: ItemId,
        color: [f64; 3],
        name: &str,
        width: i32,
        height: i32,
        pixel_aspect: f64,
        duration: Option<f64>,
    ) -> ScriptResult<LayerId> {
        check_int_range(width, 4, 30000, "width")?;
        check_int_range(height, 4, 30000, "height")?;
        check_range(pixel_aspect, 0.01, 100.0, "pixelAspect")?;
        let comp_duration = self.require_comp(comp)?.duration;
        let rate = self.require_comp(comp)?.frame_rate;
        let footage = self.insert_solid(
            self.root,
            name,
            color,
            width as u32,
            height as u32,
            pixel_aspect,
            duration.unwrap_or(comp_duration),
            rate,
        )?;
        self.add_layer_from_item(comp, footage, duration)
    }

    pub fn add_audio_footage(&mut self, name: &str, duration: f64, frame_rate: f64) -> ScriptResult<ItemId> {
        let mut source = super::SourceRec::still(name, 0, 0, frame_rate);
        source.has_audio = true;
        source.has_video = false;
        source.is_still = false;
        source.duration = duration;
        source.native_duration = duration;
        self.insert_footage(self.root, name, source)
    }

    pub fn add_video_footage(
        &mut self,
        name: &str,
        width: u32,
        height: u32,
        duration: f64,
        frame_rate: f64,
        has_audio: bool,
    ) -> ScriptResult<ItemId> {
        let mut source = super::SourceRec::still(name, width, height, frame_rate);
        source.is_still = false;
        source.duration = duration;
        source.native_duration = duration;
        source.has_audio = has_audio;
        source.has_video = true;
        self.insert_footage(self.root, name, source)
    }

    fn push_layer(
        &mut self,
        comp: ItemId,
        name: &str,
        kind: LayerKind,
        duration: f64,
        has_video: bool,
        has_audio: bool,
        time_remap_property: bool,
    ) -> ScriptResult<LayerId> {
        let _ = self.require_comp(comp)?;
        let id = self.alloc_layer();
        let layer = LayerRec {
            name: name.to_string(),
            name_set: false,
            comp,
            kind,
            comment: String::new(),
            label: 0,
            locked: false,
            shy: false,
            solo: false,
            enabled: true,
            in_point: 0.0,
            out_point: duration,
            start_time: 0.0,
            source_span: duration,
            stretch: 100.0,
            selected: false,
            auto_orient: AutoOrientType::NoAutoOrient,
            parent: None,
            guides: Vec::new(),
            root_props: Vec::new(),
            blend: BlendMode::Normal,
            track_matte: TrackMatte::none(),
            matte_type: TrackMatteType::NoTrackMatte,
            preserve_transparency: false,
            collapse: false,
            motion_blur: false,
            three_d: false,
            three_d_per_char: false,
            adjustment: false,
            environment: false,
            guide_layer: false,
            frame_blending: false,
            frame_blending_type: FrameBlendingType::NoFrameBlend,
            effects_active: true,
            audio_enabled: true,
            quality: LayerQuality::Best,
            sampling: LayerSamplingQuality::Bilinear,
            time_remap: false,
            has_video,
            has_audio,
            light_type: LightType::Point,
            light_source: None,
            point_of_interest: [0.0, 0.0, 0.0],
            focus_area_width: 0.0,
            near_far_blur: 1.0,
            camera_nodes: 1,
            mesh: ParametricMeshType::Sphere,
            mesh_fields: Default::default(),
            bevel_fields: Default::default(),
            text: None,
        };
        self.layers.insert(id, layer);
        self.require_comp_mut(comp)?.layers.insert(0, id);
        self.seed_common_properties(id, time_remap_property)?;
        self.touch();
        Ok(id)
    }

    pub fn num_layers(&self, comp: ItemId) -> ScriptResult<i32> {
        Ok(self.require_comp(comp)?.layers.len() as i32)
    }

    pub fn layer_at(&self, comp: ItemId, index: i32) -> ScriptResult<LayerId> {
        if index < 1 {
            return Err(ScriptError::range("layer index"));
        }
        self.require_comp(comp)?
            .layers
            .get((index - 1) as usize)
            .copied()
            .ok_or_else(|| ScriptError::range("layer index"))
    }

    pub fn layer_by_name(&self, comp: ItemId, name: &str) -> ScriptResult<Option<LayerId>> {
        for layer in &self.require_comp(comp)?.layers {
            if self.layer(*layer)?.name == name {
                return Ok(Some(*layer));
            }
        }
        Ok(None)
    }

    pub fn layer_relative(&self, comp: ItemId, other: LayerId, rel: i32) -> ScriptResult<LayerId> {
        let index = self.layer_index(other)?;
        self.layer_at(comp, index + rel)
    }

    pub fn layer_index(&self, layer: LayerId) -> ScriptResult<i32> {
        let comp = self.layer(layer)?.comp;
        let position = self
            .require_comp(comp)?
            .layers
            .iter()
            .position(|id| *id == layer)
            .ok_or_else(|| ScriptError::missing("レイヤー"))?;
        Ok(position as i32 + 1)
    }

    pub fn containing_comp(&self, layer: LayerId) -> ScriptResult<ItemId> {
        Ok(self.layer(layer)?.comp)
    }

    pub fn layer_name(&self, layer: LayerId) -> ScriptResult<String> {
        Ok(self.layer(layer)?.name.clone())
    }

    pub fn set_layer_name(&mut self, layer: LayerId, name: &str) -> ScriptResult<()> {
        let record = self.layer_mut(layer)?;
        record.name = name.to_string();
        record.name_set = true;
        self.touch();
        Ok(())
    }

    pub fn layer_is_name_set(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.name_set)
    }

    pub fn layer_is_name_from_source(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(!self.layer(layer)?.name_set && self.layer(layer)?.source_id().is_some())
    }

    pub fn layer_id_value(&self, layer: LayerId) -> u64 {
        layer.get()
    }

    pub fn layer_comment(&self, layer: LayerId) -> ScriptResult<String> {
        Ok(self.layer(layer)?.comment.clone())
    }

    pub fn set_layer_comment(&mut self, layer: LayerId, comment: &str) -> ScriptResult<()> {
        self.layer_mut(layer)?.comment = comment.to_string();
        self.touch();
        Ok(())
    }

    pub fn layer_label(&self, layer: LayerId) -> ScriptResult<i32> {
        Ok(self.layer(layer)?.label)
    }

    pub fn set_layer_label(&mut self, layer: LayerId, label: i32) -> ScriptResult<()> {
        check_int_range(label, 0, 16, "label")?;
        self.layer_mut(layer)?.label = label;
        self.touch();
        Ok(())
    }

    pub fn layer_locked(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.locked)
    }

    pub fn set_layer_locked(&mut self, layer: LayerId, locked: bool) -> ScriptResult<()> {
        self.layer_mut(layer)?.locked = locked;
        Ok(())
    }

    pub fn ui_can_move_layer(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(!self.layer(layer)?.locked)
    }

    pub fn nudge_layer_from_ui(&mut self, layer: LayerId, delta: [f64; 2]) -> ScriptResult<()> {
        if self.layer(layer)?.locked {
            return Err(ScriptError::invalid("ロック中は UI から動かせない"));
        }
        let prop = self
            .find_layer_prop(layer, "Position")
            .ok_or_else(|| ScriptError::missing("Position"))?;
        let mut value = self.vec2(prop)?;
        value[0] += delta[0];
        value[1] += delta[1];
        self.set_vec2(prop, value)
    }

    pub fn layer_shy(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.shy)
    }

    pub fn set_layer_shy(&mut self, layer: LayerId, shy: bool) -> ScriptResult<()> {
        self.layer_mut(layer)?.shy = shy;
        Ok(())
    }

    pub fn layer_hidden_by_shy(&self, layer: LayerId) -> ScriptResult<bool> {
        let record = self.layer(layer)?;
        Ok(record.shy && self.require_comp(record.comp)?.hide_shy)
    }

    pub fn layer_solo(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.solo)
    }

    pub fn set_layer_solo(&mut self, layer: LayerId, solo: bool) -> ScriptResult<()> {
        self.layer_mut(layer)?.solo = solo;
        Ok(())
    }

    pub fn layer_enabled(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.enabled)
    }

    pub fn set_layer_enabled(&mut self, layer: LayerId, enabled: bool) -> ScriptResult<()> {
        self.layer_mut(layer)?.enabled = enabled;
        Ok(())
    }

    pub fn layer_in_point(&self, layer: LayerId) -> ScriptResult<f64> {
        Ok(self.layer(layer)?.in_point)
    }

    pub fn set_layer_in_point(&mut self, layer: LayerId, value: f64) -> ScriptResult<()> {
        check_range(value, -10800.0, 10800.0, "inPoint")?;
        self.layer_mut(layer)?.in_point = value;
        self.touch();
        Ok(())
    }

    pub fn layer_out_point(&self, layer: LayerId) -> ScriptResult<f64> {
        Ok(self.layer(layer)?.out_point)
    }

    pub fn set_layer_out_point(&mut self, layer: LayerId, value: f64) -> ScriptResult<()> {
        check_range(value, -10800.0, 10800.0, "outPoint")?;
        self.layer_mut(layer)?.out_point = value;
        self.touch();
        Ok(())
    }

    pub fn layer_start_time(&self, layer: LayerId) -> ScriptResult<f64> {
        Ok(self.layer(layer)?.start_time)
    }

    pub fn set_layer_start_time(&mut self, layer: LayerId, value: f64) -> ScriptResult<()> {
        check_range(value, -10800.0, 10800.0, "startTime")?;
        let record = self.layer_mut(layer)?;
        let delta = value - record.start_time;
        record.start_time = value;
        record.in_point += delta;
        record.out_point += delta;
        self.touch();
        Ok(())
    }

    pub fn layer_stretch(&self, layer: LayerId) -> ScriptResult<f64> {
        Ok(self.layer(layer)?.stretch)
    }

    pub fn set_layer_stretch(&mut self, layer: LayerId, value: f64) -> ScriptResult<()> {
        check_range(value, -9900.0, 9900.0, "stretch")?;
        if value == 0.0 {
            return Err(ScriptError::range("stretch"));
        }
        let record = self.layer_mut(layer)?;
        record.stretch = value;
        record.out_point = record.in_point + record.source_span * (value.abs() / 100.0);
        self.touch();
        Ok(())
    }

    pub fn layer_time(&self, layer: LayerId) -> ScriptResult<f64> {
        let record = self.layer(layer)?;
        let comp_time = self.require_comp(record.comp)?.time;
        Ok(comp_time - record.start_time)
    }

    pub fn layer_has_video(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.has_video)
    }

    pub fn layer_null(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(matches!(self.layer(layer)?.kind, LayerKind::Null))
    }

    pub fn layer_is_camera(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(matches!(self.layer(layer)?.kind, LayerKind::Camera))
    }

    pub fn layer_is_light(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(matches!(self.layer(layer)?.kind, LayerKind::Light))
    }

    pub fn layer_is_text(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(matches!(self.layer(layer)?.kind, LayerKind::Text))
    }

    pub fn layer_is_shape(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(matches!(self.layer(layer)?.kind, LayerKind::Shape))
    }

    pub fn layer_is_mesh(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(matches!(self.layer(layer)?.kind, LayerKind::Mesh))
    }

    pub fn layer_active(&self, layer: LayerId) -> ScriptResult<bool> {
        self.layer_active_at(layer, self.require_comp(self.layer(layer)?.comp)?.time)
    }

    pub fn layer_active_at(&self, layer: LayerId, time: f64) -> ScriptResult<bool> {
        let record = self.layer(layer)?;
        if !record.enabled || !record.has_video {
            return Ok(false);
        }
        if time < record.in_point || time >= record.out_point {
            return Ok(false);
        }
        let comp = record.comp;
        let any_solo = self.require_comp(comp)?.layers.iter().any(|id| {
            self.layers.get(id).is_some_and(|other| other.solo && other.has_video)
        });
        if any_solo && !record.solo {
            return Ok(false);
        }
        Ok(true)
    }

    pub fn auto_orient(&self, layer: LayerId) -> ScriptResult<AutoOrientType> {
        Ok(self.layer(layer)?.auto_orient)
    }

    pub fn set_auto_orient(&mut self, layer: LayerId, value: AutoOrientType) -> ScriptResult<()> {
        self.layer_mut(layer)?.auto_orient = value;
        Ok(())
    }

    pub fn layer_parent(&self, layer: LayerId) -> ScriptResult<Option<LayerId>> {
        Ok(self.layer(layer)?.parent)
    }

    pub fn set_layer_parent(&mut self, layer: LayerId, parent: Option<LayerId>) -> ScriptResult<()> {
        if let Some(parent) = parent {
            if parent == layer || self.parent_chain_contains(parent, layer)? {
                return Err(ScriptError::invalid("親子の循環"));
            }
        }
        self.layer_mut(layer)?.parent = parent;
        self.touch();
        Ok(())
    }

    pub fn set_parent_with_jump(&mut self, layer: LayerId, parent: Option<LayerId>) -> ScriptResult<()> {
        let before = self.source_point_to_comp(layer, self.layer_time(layer)?, self.anchor(layer)?)?;
        self.set_layer_parent(layer, parent)?;
        if parent.is_some() {
            let parent = parent.unwrap();
            let parent_point = self.comp_point_to_source(parent, self.layer_time(parent)?, before)?;
            let anchor = self.anchor(layer)?;
            let prop = self
                .find_layer_prop(layer, "Position")
                .ok_or_else(|| ScriptError::missing("Position"))?;
            self.set_vec2(prop, [parent_point[0], parent_point[1]])?;
            let _ = anchor;
        }
        Ok(())
    }

    fn parent_chain_contains(&self, start: LayerId, target: LayerId) -> ScriptResult<bool> {
        let mut current = Some(start);
        for _ in 0..32 {
            let Some(id) = current else { return Ok(false) };
            if id == target {
                return Ok(true);
            }
            current = self.layer(id)?.parent;
        }
        Ok(true)
    }

    pub fn duplicate_layer(&mut self, layer: LayerId) -> ScriptResult<LayerId> {
        let copy = self.clone_layer_record(layer)?;
        let comp = copy.comp;
        let id = self.alloc_layer();
        let index = self.layer_index(layer)? as usize;
        self.layers.insert(id, copy);
        self.rebind_layer_props(layer, id)?;
        self.require_comp_mut(comp)?.layers.insert(index, id);
        self.touch();
        Ok(id)
    }

    pub fn copy_layer_to_comp(&mut self, layer: LayerId, dest: ItemId) -> ScriptResult<LayerId> {
        let _ = self.require_comp(dest)?;
        let copy = self.clone_layer_record(layer)?;
        let id = self.alloc_layer();
        self.layers.insert(id, copy);
        self.layer_mut(id)?.comp = dest;
        self.rebind_layer_props(layer, id)?;
        self.require_comp_mut(dest)?.layers.insert(0, id);
        self.touch();
        Ok(id)
    }

    pub fn remove_layer(&mut self, layer: LayerId) -> ScriptResult<()> {
        let comp = self.layer(layer)?.comp;
        let roots = self.layer(layer)?.root_props.clone();
        for prop in roots {
            self.remove_property(prop)?;
        }
        self.require_comp_mut(comp)?.layers.retain(|id| *id != layer);
        self.layers.remove(&layer);
        self.touch();
        Ok(())
    }

    pub fn move_layer_to_beginning(&mut self, layer: LayerId) -> ScriptResult<()> {
        self.move_layer_to(layer, 0)
    }

    pub fn move_layer_to_end(&mut self, layer: LayerId) -> ScriptResult<()> {
        let comp = self.layer(layer)?.comp;
        let len = self.require_comp(comp)?.layers.len();
        self.move_layer_to(layer, len.saturating_sub(1))
    }

    pub fn move_layer_before(&mut self, layer: LayerId, before: LayerId) -> ScriptResult<()> {
        let index = self.layer_index(before)? as usize - 1;
        self.move_layer_to(layer, index)
    }

    pub fn move_layer_after(&mut self, layer: LayerId, after: LayerId) -> ScriptResult<()> {
        let mut index = self.layer_index(after)? as usize;
        let current = self.layer_index(layer)? as usize - 1;
        if current < index {
            index -= 1;
        }
        self.move_layer_to(layer, index)
    }

    fn move_layer_to(&mut self, layer: LayerId, index: usize) -> ScriptResult<()> {
        let comp = self.layer(layer)?.comp;
        let layers = &mut self.require_comp_mut(comp)?.layers;
        let current = layers.iter().position(|id| *id == layer).ok_or_else(|| ScriptError::missing("レイヤー"))?;
        let id = layers.remove(current);
        let index = index.min(layers.len());
        layers.insert(index, id);
        Ok(())
    }

    pub fn precompose(&mut self, comp: ItemId, indices: &[i32], name: &str, _move_all: bool) -> ScriptResult<ItemId> {
        let width = self.require_comp(comp)?.width as i32;
        let height = self.require_comp(comp)?.height as i32;
        let rate = self.require_comp(comp)?.frame_rate;
        let duration = self.require_comp(comp)?.duration;
        let pixel = self.require_comp(comp)?.pixel_aspect;
        let mut layers = Vec::new();
        for index in indices {
            layers.push(self.layer_at(comp, *index)?);
        }
        let new_comp = self.add_comp_in(self.root, name, width, height, pixel, duration, rate)?;
        for (offset, layer) in layers.iter().enumerate() {
            self.remove_from_comp_list(comp, *layer)?;
            self.layer_mut(*layer)?.comp = new_comp;
            self.require_comp_mut(new_comp)?.layers.push(*layer);
            let _ = offset;
        }
        let precomp = self.push_layer(comp, name, LayerKind::Av { source: Some(new_comp) }, duration, true, false, false)?;
        let _ = precomp;
        Ok(new_comp)
    }

    fn remove_from_comp_list(&mut self, comp: ItemId, layer: LayerId) -> ScriptResult<()> {
        self.require_comp_mut(comp)?.layers.retain(|id| *id != layer);
        Ok(())
    }

    pub fn layer_guides(&self, layer: LayerId) -> ScriptResult<Vec<Guide>> {
        Ok(self.layer(layer)?.guides.clone())
    }

    pub fn add_layer_guide(&mut self, layer: LayerId, orientation: i32, position: f64) -> ScriptResult<i32> {
        let record = self.layer_mut(layer)?;
        record.guides.push(Guide::at(orientation, position));
        Ok(record.guides.len() as i32 - 1)
    }

    pub fn add_layer_guide_options(&mut self, layer: LayerId, options: &GuideOptions) -> ScriptResult<i32> {
        let orientation = match options.orientation.unwrap_or(GuideOrientationType::Horizontal) {
            GuideOrientationType::Horizontal => 0,
            GuideOrientationType::Vertical => 1,
        };
        let position = options.position.unwrap_or(0.0);
        let index = self.add_layer_guide(layer, orientation, position)?;
        let guide = &mut self.layer_mut(layer)?.guides[index as usize];
        if let Some(color) = options.color {
            guide.color = color;
        }
        if let Some(pinned) = options.pinned {
            guide.pinned = pinned;
        }
        Ok(index)
    }

    pub fn layer_guide(&self, layer: LayerId, index: i32) -> ScriptResult<Guide> {
        self.layer(layer)?
            .guides
            .get(index as usize)
            .cloned()
            .ok_or_else(|| ScriptError::range("guide"))
    }

    pub fn remove_layer_guide(&mut self, layer: LayerId, index: i32) -> ScriptResult<()> {
        let guides = &mut self.layer_mut(layer)?.guides;
        if index < 0 || index as usize >= guides.len() {
            return Err(ScriptError::range("guide"));
        }
        guides.remove(index as usize);
        Ok(())
    }

    pub fn set_layer_guide(&mut self, layer: LayerId, position: f64, index: i32) -> ScriptResult<()> {
        let guide = self
            .layer_mut(layer)?
            .guides
            .get_mut(index as usize)
            .ok_or_else(|| ScriptError::range("guide"))?;
        guide.position = position;
        Ok(())
    }

    pub fn apply_preset(&mut self, layer: LayerId, path: &std::path::Path) -> ScriptResult<()> {
        let text = std::fs::read_to_string(path).map_err(|error| ScriptError::invalid(error.to_string()))?;
        for line in text.lines() {
            let line = line.trim();
            if let Some(name) = line.strip_prefix("effect ") {
                let effects = self
                    .find_layer_prop(layer, "ADBE Effect Parade")
                    .ok_or_else(|| ScriptError::missing("Effects"))?;
                self.add_property(effects, name)?;
            }
        }
        Ok(())
    }

    pub fn scene_edit_detection(&mut self, layer: LayerId, mode: SceneEditDetectionMode) -> ScriptResult<Vec<f64>> {
        let record = self.layer(layer)?;
        if !record.has_video {
            return Err(ScriptError::invalid("映像が無い"));
        }
        if record.time_remap {
            return Err(ScriptError::invalid("タイムリマップ中"));
        }
        let duration = record.out_point - record.in_point;
        let mut times = Vec::new();
        if duration >= 2.0 {
            let mut time = 1.0;
            while time < duration {
                times.push(time);
                time += 1.0;
            }
        }
        if mode == SceneEditDetectionMode::Markers {
            if let Some(marker) = self.find_layer_prop(layer, "Marker") {
                for time in &times {
                    self.set_value_at_time(marker, *time, super::Value::Marker(crate::values::MarkerValue::new("scene")))?;
                }
            }
        }
        Ok(times)
    }

    pub fn adjustment_layer(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.adjustment)
    }

    pub fn set_adjustment_layer(&mut self, layer: LayerId, value: bool) -> ScriptResult<()> {
        self.layer_mut(layer)?.adjustment = value;
        Ok(())
    }

    pub fn adjustment_affects_below(&self, layer: LayerId) -> ScriptResult<bool> {
        let record = self.layer(layer)?;
        Ok(record.adjustment && record.effects_active && record.enabled)
    }

    pub fn audio_enabled(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.audio_enabled)
    }

    pub fn set_audio_enabled(&mut self, layer: LayerId, value: bool) -> ScriptResult<()> {
        self.layer_mut(layer)?.audio_enabled = value;
        Ok(())
    }

    pub fn audio_active(&self, layer: LayerId) -> ScriptResult<bool> {
        self.audio_active_at(layer, self.require_comp(self.layer(layer)?.comp)?.time)
    }

    pub fn audio_active_at(&self, layer: LayerId, time: f64) -> ScriptResult<bool> {
        let record = self.layer(layer)?;
        if !record.has_audio || !record.audio_enabled || !record.enabled {
            return Ok(false);
        }
        if time < record.in_point || time >= record.out_point {
            return Ok(false);
        }
        let comp = record.comp;
        let any_solo = self
            .require_comp(comp)?
            .layers
            .iter()
            .any(|id| self.layers.get(id).is_some_and(|other| other.solo));
        if any_solo && !record.solo {
            return Ok(false);
        }
        Ok(true)
    }

    pub fn blending_mode(&self, layer: LayerId) -> ScriptResult<BlendMode> {
        Ok(self.layer(layer)?.blend)
    }

    pub fn set_blending_mode(&mut self, layer: LayerId, mode: BlendMode) -> ScriptResult<()> {
        self.layer_mut(layer)?.blend = mode;
        Ok(())
    }

    pub fn can_set_collapse(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(matches!(
            self.layer(layer)?.kind,
            LayerKind::Av { source: Some(_) } | LayerKind::Shape | LayerKind::Text
        ) && self.layer_source_is_comp(layer))
    }

    fn layer_source_is_comp(&self, layer: LayerId) -> bool {
        match self.layer(layer).ok().and_then(|layer| layer.source_id()) {
            Some(source) => matches!(self.items.get(&source).map(|item| &item.body), Some(ItemBody::Comp(_))),
            None => matches!(self.layers.get(&layer).map(|layer| &layer.kind), Some(LayerKind::Shape | LayerKind::Text)),
        }
    }

    pub fn collapse_transformation(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.collapse)
    }

    pub fn set_collapse_transformation(&mut self, layer: LayerId, value: bool) -> ScriptResult<()> {
        if value && !self.can_set_collapse(layer)? && !matches!(self.layer(layer)?.kind, LayerKind::Shape | LayerKind::Text) {
            return Err(ScriptError::invalid("コラップスできない"));
        }
        self.layer_mut(layer)?.collapse = value;
        Ok(())
    }

    pub fn nested_transform_kept(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.collapse && self.layer_source_is_comp(layer))
    }

    pub fn can_set_time_remap(&self, layer: LayerId) -> ScriptResult<bool> {
        let record = self.layer(layer)?;
        if matches!(record.kind, LayerKind::Text) {
            return Ok(false);
        }
        let Some(source) = record.source_id() else {
            return Ok(false);
        };
        match &self.item(source)?.body {
            ItemBody::Footage(footage) => Ok(!footage.main.is_still && footage.main.duration > 0.0),
            ItemBody::Comp(comp) => Ok(comp.duration > 0.0),
            ItemBody::Folder { .. } => Ok(false),
        }
    }

    pub fn time_remap_enabled(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.time_remap)
    }

    pub fn set_time_remap_enabled(&mut self, layer: LayerId, value: bool) -> ScriptResult<()> {
        if value && !self.can_set_time_remap(layer)? {
            return Err(ScriptError::invalid("タイムリマップできない"));
        }
        self.layer_mut(layer)?.time_remap = value;
        if value && self.find_layer_prop(layer, "Time Remap").is_none() {
            self.attach_time_remap(layer)?;
        }
        Ok(())
    }

    pub fn effects_active(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.effects_active)
    }

    pub fn set_effects_active(&mut self, layer: LayerId, value: bool) -> ScriptResult<()> {
        self.layer_mut(layer)?.effects_active = value;
        Ok(())
    }

    pub fn effects_apply(&self, layer: LayerId) -> ScriptResult<bool> {
        let record = self.layer(layer)?;
        Ok(record.effects_active && record.enabled)
    }

    pub fn environment_layer(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.environment)
    }

    pub fn set_environment_layer(&mut self, layer: LayerId, value: bool) -> ScriptResult<()> {
        self.layer_mut(layer)?.environment = value;
        Ok(())
    }

    pub fn frame_blending(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.frame_blending)
    }

    pub fn set_frame_blending(&mut self, layer: LayerId, value: bool) -> ScriptResult<()> {
        self.layer_mut(layer)?.frame_blending = value;
        Ok(())
    }

    pub fn frame_blend_active(&self, layer: LayerId) -> ScriptResult<bool> {
        let record = self.layer(layer)?;
        Ok(record.frame_blending && self.require_comp(record.comp)?.frame_blending)
    }

    pub fn frame_blending_type(&self, layer: LayerId) -> ScriptResult<FrameBlendingType> {
        Ok(self.layer(layer)?.frame_blending_type)
    }

    pub fn set_frame_blending_type(&mut self, layer: LayerId, value: FrameBlendingType) -> ScriptResult<()> {
        self.layer_mut(layer)?.frame_blending_type = value;
        if value != FrameBlendingType::NoFrameBlend {
            self.layer_mut(layer)?.frame_blending = true;
        }
        Ok(())
    }

    pub fn guide_layer(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.guide_layer)
    }

    pub fn set_guide_layer(&mut self, layer: LayerId, value: bool) -> ScriptResult<()> {
        self.layer_mut(layer)?.guide_layer = value;
        Ok(())
    }

    pub fn layer_in_final_render(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(!self.layer(layer)?.guide_layer && self.layer_active(layer)?)
    }

    pub fn layer_has_audio(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.has_audio)
    }

    pub fn layer_width(&self, layer: LayerId) -> ScriptResult<f64> {
        if let Some(source) = self.layer(layer)?.source_id() {
            return Ok(self.av_width(source)? as f64);
        }
        match &self.layer(layer)?.kind {
            LayerKind::Text => Ok(self.layer(layer)?.text.as_ref().map(|text| text.box_size[0]).unwrap_or(0.0)),
            _ => Ok(self.require_comp(self.layer(layer)?.comp)?.width as f64),
        }
    }

    pub fn layer_height(&self, layer: LayerId) -> ScriptResult<f64> {
        if let Some(source) = self.layer(layer)?.source_id() {
            return Ok(self.av_height(source)? as f64);
        }
        match &self.layer(layer)?.kind {
            LayerKind::Text => Ok(self.layer(layer)?.text.as_ref().map(|text| text.box_size[1]).unwrap_or(0.0)),
            _ => Ok(self.require_comp(self.layer(layer)?.comp)?.height as f64),
        }
    }

    pub fn motion_blur(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.motion_blur)
    }

    pub fn set_motion_blur(&mut self, layer: LayerId, value: bool) -> ScriptResult<()> {
        self.layer_mut(layer)?.motion_blur = value;
        Ok(())
    }

    pub fn motion_blur_active(&self, layer: LayerId) -> ScriptResult<bool> {
        let record = self.layer(layer)?;
        Ok(record.motion_blur && self.require_comp(record.comp)?.motion_blur)
    }

    pub fn preserve_transparency(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.preserve_transparency)
    }

    pub fn set_preserve_transparency(&mut self, layer: LayerId, value: bool) -> ScriptResult<()> {
        self.layer_mut(layer)?.preserve_transparency = value;
        Ok(())
    }

    pub fn layer_quality(&self, layer: LayerId) -> ScriptResult<LayerQuality> {
        Ok(self.layer(layer)?.quality)
    }

    pub fn set_layer_quality(&mut self, layer: LayerId, value: LayerQuality) -> ScriptResult<()> {
        self.layer_mut(layer)?.quality = value;
        Ok(())
    }

    pub fn sampling_quality(&self, layer: LayerId) -> ScriptResult<LayerSamplingQuality> {
        Ok(self.layer(layer)?.sampling)
    }

    pub fn set_sampling_quality(&mut self, layer: LayerId, value: LayerSamplingQuality) -> ScriptResult<()> {
        self.layer_mut(layer)?.sampling = value;
        Ok(())
    }

    pub fn layer_source(&self, layer: LayerId) -> ScriptResult<Option<ItemId>> {
        Ok(self.layer(layer)?.source_id())
    }

    pub fn replace_source(&mut self, layer: LayerId, source: ItemId, fix_expressions: bool) -> ScriptResult<()> {
        let old_name = self
            .layer(layer)?
            .source_id()
            .and_then(|id| self.items.get(&id).map(|item| item.name.clone()))
            .unwrap_or_default();
        let new_name = self.item_name(source)?;
        self.layer_mut(layer)?.set_source(source);
        if fix_expressions && !old_name.is_empty() {
            let props = self.layer_prop_ids(layer)?;
            for prop in props {
                let expression = self.prop(prop)?.expression.clone();
                if expression.contains(&old_name) {
                    self.prop_mut(prop)?.expression = expression.replace(&old_name, &new_name);
                }
            }
        }
        self.touch();
        Ok(())
    }

    pub fn three_d_layer(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.three_d)
    }

    pub fn set_three_d_layer(&mut self, layer: LayerId, value: bool) -> ScriptResult<()> {
        self.layer_mut(layer)?.three_d = value;
        if value {
            self.ensure_3d_properties(layer)?;
            if let Some(position) = self.find_layer_prop(layer, "Position") {
                if let super::Value::Vec2(value) = self.prop(position)?.value.clone() {
                    self.prop_mut(position)?.value = super::Value::Vec3([value[0], value[1], 0.0]);
                    self.prop_mut(position)?.value_type = PropertyValueType::ThreeDSpatial;
                }
            }
        }
        Ok(())
    }

    pub fn three_d_per_char(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.three_d_per_char)
    }

    pub fn set_three_d_per_char(&mut self, layer: LayerId, value: bool) -> ScriptResult<()> {
        self.layer_mut(layer)?.three_d_per_char = value;
        Ok(())
    }

    pub fn track_matte_type(&self, layer: LayerId) -> ScriptResult<TrackMatteType> {
        Ok(self.layer(layer)?.matte_type)
    }

    pub fn set_track_matte_type(&mut self, layer: LayerId, value: TrackMatteType) -> ScriptResult<()> {
        let above = self.layer_above(layer)?;
        self.layer_mut(layer)?.matte_type = value;
        let mode = matte_mode(value);
        let source = if value == TrackMatteType::NoTrackMatte { None } else { above };
        let record = self.layer_mut(layer)?;
        record.track_matte = TrackMatte { source, mode };
        Ok(())
    }

    pub fn track_matte_layer(&self, layer: LayerId) -> ScriptResult<Option<LayerId>> {
        Ok(self.layer(layer)?.track_matte.source)
    }

    pub fn has_track_matte(&self, layer: LayerId) -> ScriptResult<bool> {
        Ok(self.layer(layer)?.matte_type != TrackMatteType::NoTrackMatte)
    }

    pub fn is_track_matte(&self, layer: LayerId) -> ScriptResult<bool> {
        if matches!(self.layer(layer)?.kind, LayerKind::Text) {
            return Err(ScriptError::invalid("テキストレイヤーでは使わない"));
        }
        let comp = self.layer(layer)?.comp;
        for other in &self.require_comp(comp)?.layers.clone() {
            if self.layer(*other)?.track_matte.source == Some(layer) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn set_track_matte(&mut self, layer: LayerId, matte: LayerId, kind: TrackMatteType) -> ScriptResult<()> {
        self.layer_mut(layer)?.matte_type = kind;
        self.layer_mut(layer)?.track_matte = TrackMatte {
            source: Some(matte),
            mode: matte_mode(kind),
        };
        Ok(())
    }

    pub fn remove_track_matte(&mut self, layer: LayerId) -> ScriptResult<()> {
        self.layer_mut(layer)?.matte_type = TrackMatteType::NoTrackMatte;
        self.layer_mut(layer)?.track_matte = TrackMatte::none();
        Ok(())
    }

    fn layer_above(&self, layer: LayerId) -> ScriptResult<Option<LayerId>> {
        let index = self.layer_index(layer)?;
        if index <= 1 {
            return Ok(None);
        }
        let comp = self.layer(layer)?.comp;
        Ok(Some(self.layer_at(comp, index - 1)?))
    }

    pub fn light_type(&self, layer: LayerId) -> ScriptResult<LightType> {
        if !self.layer_is_light(layer)? {
            return Err(ScriptError::invalid("ライトではない"));
        }
        Ok(self.layer(layer)?.light_type)
    }

    pub fn set_light_type(&mut self, layer: LayerId, value: LightType) -> ScriptResult<()> {
        if !self.layer_is_light(layer)? {
            return Err(ScriptError::invalid("ライトではない"));
        }
        self.layer_mut(layer)?.light_type = value;
        Ok(())
    }

    pub fn light_source(&self, layer: LayerId) -> ScriptResult<Option<LayerId>> {
        Ok(self.layer(layer)?.light_source)
    }

    pub fn set_light_source(&mut self, layer: LayerId, source: Option<LayerId>) -> ScriptResult<()> {
        if !self.layer_is_light(layer)? {
            return Err(ScriptError::invalid("ライトではない"));
        }
        if self.layer(layer)?.light_type != LightType::Environment {
            return Err(ScriptError::invalid("環境ライトではない"));
        }
        if let Some(source) = source {
            if self.layer(source)?.three_d {
                return Err(ScriptError::invalid("3D レイヤーは光源にできない"));
            }
        }
        self.layer_mut(layer)?.light_source = source;
        Ok(())
    }

    pub fn point_of_interest(&self, layer: LayerId) -> ScriptResult<[f64; 3]> {
        Ok(self.layer(layer)?.point_of_interest)
    }

    pub fn set_point_of_interest(&mut self, layer: LayerId, value: [f64; 3]) -> ScriptResult<()> {
        self.layer_mut(layer)?.point_of_interest = value;
        let position = self
            .find_layer_prop(layer, "Position")
            .and_then(|prop| self.vec2(prop).ok())
            .unwrap_or([0.0, 0.0]);
        let dx = value[0] - position[0];
        let dy = value[1] - position[1];
        let angle = dy.atan2(dx).to_degrees();
        if let Some(orientation) = self.find_layer_prop(layer, "Orientation") {
            self.set_vec3(orientation, [0.0, 0.0, angle])?;
        }
        Ok(())
    }

    pub fn focus_area_width(&self, layer: LayerId) -> ScriptResult<f64> {
        self.require_advanced_camera(layer)?;
        Ok(self.layer(layer)?.focus_area_width)
    }

    pub fn set_focus_area_width(&mut self, layer: LayerId, value: f64) -> ScriptResult<()> {
        self.require_advanced_camera(layer)?;
        self.layer_mut(layer)?.focus_area_width = value;
        Ok(())
    }

    pub fn near_far_blur_multiplier(&self, layer: LayerId) -> ScriptResult<f64> {
        self.require_advanced_camera(layer)?;
        Ok(self.layer(layer)?.near_far_blur)
    }

    pub fn set_near_far_blur_multiplier(&mut self, layer: LayerId, value: f64) -> ScriptResult<()> {
        self.require_advanced_camera(layer)?;
        self.layer_mut(layer)?.near_far_blur = value;
        Ok(())
    }

    fn require_advanced_camera(&self, layer: LayerId) -> ScriptResult<()> {
        if !self.layer_is_camera(layer)? {
            return Err(ScriptError::invalid("カメラではない"));
        }
        let comp = self.layer(layer)?.comp;
        if self.require_comp(comp)?.renderer != "Advanced 3D" {
            return Err(ScriptError::unsupported("クラシック 3D"));
        }
        Ok(())
    }

    pub fn mesh_type(&self, layer: LayerId) -> ScriptResult<ParametricMeshType> {
        if !self.layer_is_mesh(layer)? {
            return Err(ScriptError::invalid("メッシュではない"));
        }
        Ok(self.layer(layer)?.mesh)
    }

    pub fn set_mesh_type(&mut self, layer: LayerId, value: ParametricMeshType) -> ScriptResult<()> {
        if !self.layer_is_mesh(layer)? {
            return Err(ScriptError::invalid("メッシュではない"));
        }
        self.layer_mut(layer)?.mesh = value;
        Ok(())
    }

    pub fn mesh_field(&self, layer: LayerId, field: &str) -> ScriptResult<f64> {
        let kind = self.mesh_type(layer)?;
        if !mesh_allows(kind, field) {
            return Err(ScriptError::invalid("このメッシュ種類では読まない"));
        }
        Ok(self.layer(layer)?.mesh_fields.get(field).copied().unwrap_or(0.0))
    }

    pub fn set_mesh_field(&mut self, layer: LayerId, field: &str, value: f64) -> ScriptResult<()> {
        let kind = self.mesh_type(layer)?;
        if !mesh_allows(kind, field) {
            return Err(ScriptError::invalid("このメッシュ種類では書かない"));
        }
        self.layer_mut(layer)?.mesh_fields.insert(field.to_string(), value);
        Ok(())
    }

    pub fn bevel_field(&self, layer: LayerId, field: &str) -> ScriptResult<f64> {
        let kind = self.mesh_type(layer)?;
        if !kind.allows_bevel() || !bevel_allows(kind, field) {
            return Err(ScriptError::invalid("このメッシュ種類ではベベルを読まない"));
        }
        Ok(self.layer(layer)?.bevel_fields.get(field).copied().unwrap_or(0.0))
    }

    pub fn set_bevel_field(&mut self, layer: LayerId, field: &str, value: f64) -> ScriptResult<()> {
        let kind = self.mesh_type(layer)?;
        if !kind.allows_bevel() || !bevel_allows(kind, field) {
            return Err(ScriptError::invalid("このメッシュ種類ではベベルを書かない"));
        }
        self.layer_mut(layer)?.bevel_fields.insert(field.to_string(), value);
        Ok(())
    }

    pub fn source_rect_at_time(&self, layer: LayerId, _time: f64, _extents: bool) -> ScriptResult<[f64; 4]> {
        let width = self.layer_width(layer)?;
        let height = self.layer_height(layer)?;
        Ok([0.0, 0.0, width.max(1.0), height.max(1.0)])
    }

    pub fn anchor(&self, layer: LayerId) -> ScriptResult<[f64; 2]> {
        let prop = self
            .find_layer_prop(layer, "Anchor Point")
            .ok_or_else(|| ScriptError::missing("Anchor Point"))?;
        self.vec2(prop)
    }

    pub fn source_point_to_comp(&self, layer: LayerId, time: f64, point: [f64; 2]) -> ScriptResult<[f64; 2]> {
        let local = self.apply_layer_transform(layer, time, point)?;
        self.map_through_parents(layer, time, local)
    }

    pub fn comp_point_to_source(&self, layer: LayerId, time: f64, point: [f64; 2]) -> ScriptResult<[f64; 2]> {
        let world = self.unmap_through_parents(layer, time, point)?;
        self.invert_layer_transform(layer, time, world)
    }

    pub fn layer_to_world(&self, layer: LayerId, time: f64, point: [f64; 2]) -> ScriptResult<[f64; 2]> {
        self.source_point_to_comp(layer, time, point)
    }

    pub fn world_to_layer(&self, layer: LayerId, time: f64, point: [f64; 2]) -> ScriptResult<[f64; 2]> {
        self.comp_point_to_source(layer, time, point)
    }

    pub fn layer_source_size_at(&self, layer: LayerId, _time: f64) -> ScriptResult<(u32, u32)> {
        Ok((self.layer_width(layer)? as u32, self.layer_height(layer)? as u32))
    }

    fn apply_layer_transform(&self, layer: LayerId, time: f64, point: [f64; 2]) -> ScriptResult<[f64; 2]> {
        let anchor = self.sample_vec2(layer, "Anchor Point", time)?.unwrap_or([0.0, 0.0]);
        let position = self.sample_vec2(layer, "Position", time)?.unwrap_or([0.0, 0.0]);
        let scale = self.sample_vec2(layer, "Scale", time)?.unwrap_or([100.0, 100.0]);
        let rotation = self.sample_scalar(layer, "Rotation", time)?.unwrap_or(0.0);
        let mut local = [point[0] - anchor[0], point[1] - anchor[1]];
        local[0] *= scale[0] / 100.0;
        local[1] *= scale[1] / 100.0;
        let radians = rotation.to_radians();
        let (sin, cos) = (radians.sin(), radians.cos());
        let rotated = [cos * local[0] - sin * local[1], sin * local[0] + cos * local[1]];
        Ok([rotated[0] + position[0], rotated[1] + position[1]])
    }

    fn invert_layer_transform(&self, layer: LayerId, time: f64, point: [f64; 2]) -> ScriptResult<[f64; 2]> {
        let anchor = self.sample_vec2(layer, "Anchor Point", time)?.unwrap_or([0.0, 0.0]);
        let position = self.sample_vec2(layer, "Position", time)?.unwrap_or([0.0, 0.0]);
        let scale = self.sample_vec2(layer, "Scale", time)?.unwrap_or([100.0, 100.0]);
        let rotation = self.sample_scalar(layer, "Rotation", time)?.unwrap_or(0.0);
        let mut local = [point[0] - position[0], point[1] - position[1]];
        let radians = -rotation.to_radians();
        let (sin, cos) = (radians.sin(), radians.cos());
        local = [cos * local[0] - sin * local[1], sin * local[0] + cos * local[1]];
        let sx = if scale[0].abs() < 1e-9 { 1.0 } else { scale[0] / 100.0 };
        let sy = if scale[1].abs() < 1e-9 { 1.0 } else { scale[1] / 100.0 };
        Ok([local[0] / sx + anchor[0], local[1] / sy + anchor[1]])
    }

    fn map_through_parents(&self, layer: LayerId, time: f64, mut point: [f64; 2]) -> ScriptResult<[f64; 2]> {
        let mut parent = self.layer(layer)?.parent;
        let mut guard = 0;
        while let Some(id) = parent {
            point = self.apply_layer_transform(id, time, point)?;
            parent = self.layer(id)?.parent;
            guard += 1;
            if guard > 32 {
                break;
            }
        }
        Ok(point)
    }

    fn unmap_through_parents(&self, layer: LayerId, time: f64, point: [f64; 2]) -> ScriptResult<[f64; 2]> {
        let mut chain = Vec::new();
        let mut parent = self.layer(layer)?.parent;
        while let Some(id) = parent {
            chain.push(id);
            parent = self.layer(id)?.parent;
            if chain.len() > 32 {
                break;
            }
        }
        let mut point = point;
        for id in chain.into_iter().rev() {
            point = self.invert_layer_transform(id, time, point)?;
        }
        Ok(point)
    }

    pub fn calculate_transform_from_points(
        &self,
        layer: LayerId,
        top_left: [f64; 3],
        top_right: [f64; 3],
        bottom_right: [f64; 3],
    ) -> ScriptResult<TransformFit> {
        let width = self.layer_width(layer)?.max(1.0);
        let height = self.layer_height(layer)?.max(1.0);
        let anchor = self.anchor(layer)?;
        let dx = [top_right[0] - top_left[0], top_right[1] - top_left[1]];
        let dy = [bottom_right[0] - top_right[0], bottom_right[1] - top_right[1]];
        let rotation = dx[1].atan2(dx[0]).to_degrees();
        let scale_x = (dx[0] * dx[0] + dx[1] * dx[1]).sqrt() / width * 100.0;
        let scale_y = (dy[0] * dy[0] + dy[1] * dy[1]).sqrt() / height * 100.0;
        let radians = rotation.to_radians();
        let (sin, cos) = (radians.sin(), radians.cos());
        let ax = anchor[0] * scale_x / 100.0;
        let ay = anchor[1] * scale_y / 100.0;
        let position = [
            top_left[0] + cos * ax - sin * ay,
            top_left[1] + sin * ax + cos * ay,
        ];
        Ok(TransformFit {
            position,
            scale: [scale_x, scale_y],
            rotation,
        })
    }

    pub fn open_comp_viewer(&mut self, comp: ItemId) -> ScriptResult<()> {
        let _ = self.require_comp(comp)?;
        self.viewer = Some(crate::values::Viewer {
            item: comp,
            layer: None,
            kind: crate::values::ViewerKind::Composition,
        });
        Ok(())
    }

    pub fn open_footage_viewer(&mut self, item: ItemId) -> ScriptResult<()> {
        let _ = self.require_footage(item)?;
        self.viewer = Some(crate::values::Viewer {
            item,
            layer: None,
            kind: crate::values::ViewerKind::Footage,
        });
        Ok(())
    }

    pub fn open_layer_viewer(&mut self, layer: LayerId) -> ScriptResult<()> {
        let comp = self.layer(layer)?.comp;
        self.viewer = Some(crate::values::Viewer {
            item: comp,
            layer: Some(layer),
            kind: crate::values::ViewerKind::Layer,
        });
        Ok(())
    }

    pub fn viewer(&self) -> Option<&crate::values::Viewer> {
        self.viewer.as_ref()
    }

    pub fn open_in_essential_graphics(&mut self, comp: ItemId) -> ScriptResult<()> {
        let _ = self.require_comp(comp)?;
        self.essential = Some(comp);
        Ok(())
    }

    pub fn essential_graphics_comp(&self) -> Option<ItemId> {
        self.essential
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformFit {
    pub position: [f64; 2],
    pub scale: [f64; 2],
    pub rotation: f64,
}

fn matte_mode(kind: TrackMatteType) -> TrackMatteMode {
    match kind {
        TrackMatteType::Alpha => TrackMatteMode::Alpha,
        TrackMatteType::AlphaInverted => TrackMatteMode::AlphaInverted,
        TrackMatteType::Luma => TrackMatteMode::Luma,
        TrackMatteType::LumaInverted => TrackMatteMode::LumaInverted,
        TrackMatteType::NoTrackMatte => TrackMatteMode::None,
    }
}

fn mesh_allows(kind: ParametricMeshType, field: &str) -> bool {
    let fields: &[&str] = match kind {
        ParametricMeshType::Cube => &["width", "height", "depth", "smoothingAngle"],
        ParametricMeshType::Sphere => &["radius", "sides", "sliceCaps", "sliceStart", "sliceEnd", "smoothingAngle"],
        ParametricMeshType::Plane => &["width", "length", "cornerRadius", "cornerSides"],
        ParametricMeshType::Torus => &[
            "ringRadius",
            "pipeRadius",
            "ringSides",
            "pipeSides",
            "caps",
            "sliceStart",
            "sliceEnd",
            "smoothingAngle",
        ],
        ParametricMeshType::Cone => &[
            "topRadius",
            "bottomRadius",
            "height",
            "sides",
            "topCap",
            "bottomCap",
            "sliceCaps",
            "sliceStart",
            "sliceEnd",
            "smoothingAngle",
        ],
        ParametricMeshType::Cylinder => &[
            "radius",
            "height",
            "sides",
            "topCap",
            "bottomCap",
            "sliceCaps",
            "sliceStart",
            "sliceEnd",
            "smoothingAngle",
        ],
    };
    fields.contains(&field)
}

fn bevel_allows(kind: ParametricMeshType, field: &str) -> bool {
    let fields: &[&str] = match kind {
        ParametricMeshType::Cube => &["radius", "sides"],
        ParametricMeshType::Cone => &["topRadius", "topSides", "bottomRadius", "bottomSides"],
        ParametricMeshType::Cylinder => &["radius", "sides"],
        _ => &[],
    };
    fields.contains(&field)
}
