use super::App;
use crate::enums::*;
use crate::error::{ScriptError, ScriptResult};
use crate::template::{TemplateControl, TemplatePackage};
use model::{ItemId, LayerId, PropertyId};

impl App {
    pub fn comp_bg(&self, id: ItemId) -> ScriptResult<[f64; 3]> {
        Ok(self.require_comp(id)?.bg)
    }

    pub fn set_comp_bg(&mut self, id: ItemId, color: [f64; 3]) -> ScriptResult<()> {
        self.require_comp_mut(id)?.bg = color;
        self.touch();
        Ok(())
    }

    pub fn comp_counters(&self, id: ItemId) -> ScriptResult<bool> {
        Ok(self.require_comp(id)?.counters)
    }

    pub fn set_comp_counters(&mut self, id: ItemId, value: bool) -> ScriptResult<()> {
        self.require_comp_mut(id)?.counters = value;
        Ok(())
    }

    pub fn comp_display_start_frame(&self, id: ItemId) -> ScriptResult<i32> {
        Ok(self.require_comp(id)?.display_start_frame)
    }

    pub fn set_comp_display_start_frame(&mut self, id: ItemId, value: i32) -> ScriptResult<()> {
        self.require_comp_mut(id)?.display_start_frame = value;
        self.touch();
        Ok(())
    }

    pub fn displayed_first_frame(&self, id: ItemId) -> ScriptResult<i32> {
        Ok(self.require_comp(id)?.display_start_frame)
    }

    pub fn comp_display_start_time(&self, id: ItemId) -> ScriptResult<f64> {
        Ok(self.require_comp(id)?.display_start_time)
    }

    pub fn set_comp_display_start_time(&mut self, id: ItemId, value: f64) -> ScriptResult<()> {
        if !(-10800.0..=86339.0).contains(&value) {
            return Err(ScriptError::range("displayStartTime"));
        }
        self.require_comp_mut(id)?.display_start_time = value;
        Ok(())
    }

    pub fn draft_3d(&self, id: ItemId) -> ScriptResult<bool> {
        Ok(self.require_comp(id)?.draft3d)
    }

    pub fn set_draft_3d(&mut self, id: ItemId, value: bool) -> ScriptResult<()> {
        self.require_comp_mut(id)?.draft3d = value;
        Ok(())
    }

    pub fn drop_frame(&self, id: ItemId) -> ScriptResult<bool> {
        Ok(self.require_comp(id)?.drop_frame)
    }

    pub fn set_drop_frame(&mut self, id: ItemId, value: bool) -> ScriptResult<()> {
        self.require_comp_mut(id)?.drop_frame = value;
        Ok(())
    }

    pub fn comp_frame_blending(&self, id: ItemId) -> ScriptResult<bool> {
        Ok(self.require_comp(id)?.frame_blending)
    }

    pub fn set_comp_frame_blending(&mut self, id: ItemId, value: bool) -> ScriptResult<()> {
        self.require_comp_mut(id)?.frame_blending = value;
        Ok(())
    }

    pub fn hide_shy_layers(&self, id: ItemId) -> ScriptResult<bool> {
        Ok(self.require_comp(id)?.hide_shy)
    }

    pub fn set_hide_shy_layers(&mut self, id: ItemId, value: bool) -> ScriptResult<()> {
        self.require_comp_mut(id)?.hide_shy = value;
        Ok(())
    }

    pub fn comp_marker(&self, id: ItemId) -> ScriptResult<PropertyId> {
        Ok(self.require_comp(id)?.marker)
    }

    pub fn comp_motion_blur(&self, id: ItemId) -> ScriptResult<bool> {
        Ok(self.require_comp(id)?.motion_blur)
    }

    pub fn set_comp_motion_blur(&mut self, id: ItemId, value: bool) -> ScriptResult<()> {
        self.require_comp_mut(id)?.motion_blur = value;
        Ok(())
    }

    pub fn motion_blur_adaptive_sample_limit(&self, id: ItemId) -> ScriptResult<u32> {
        Ok(self.require_comp(id)?.motion_blur_adaptive)
    }

    pub fn set_motion_blur_adaptive_sample_limit(&mut self, id: ItemId, value: u32) -> ScriptResult<()> {
        if !(16..=256).contains(&value) {
            return Err(ScriptError::range("motionBlurAdaptiveSampleLimit"));
        }
        self.require_comp_mut(id)?.motion_blur_adaptive = value;
        Ok(())
    }

    pub fn motion_blur_samples_per_frame(&self, id: ItemId) -> ScriptResult<u32> {
        Ok(self.require_comp(id)?.motion_blur_samples)
    }

    pub fn set_motion_blur_samples_per_frame(&mut self, id: ItemId, value: u32) -> ScriptResult<()> {
        if !(2..=64).contains(&value) {
            return Err(ScriptError::range("motionBlurSamplesPerFrame"));
        }
        self.require_comp_mut(id)?.motion_blur_samples = value;
        Ok(())
    }

    pub fn shutter_angle(&self, id: ItemId) -> ScriptResult<f64> {
        Ok(self.require_comp(id)?.shutter_angle)
    }

    pub fn set_shutter_angle(&mut self, id: ItemId, value: f64) -> ScriptResult<()> {
        if !(0.0..=720.0).contains(&value) {
            return Err(ScriptError::range("shutterAngle"));
        }
        self.require_comp_mut(id)?.shutter_angle = value;
        Ok(())
    }

    pub fn shutter_phase(&self, id: ItemId) -> ScriptResult<f64> {
        Ok(self.require_comp(id)?.shutter_phase)
    }

    pub fn set_shutter_phase(&mut self, id: ItemId, value: f64) -> ScriptResult<()> {
        if !(-360.0..=360.0).contains(&value) {
            return Err(ScriptError::range("shutterPhase"));
        }
        self.require_comp_mut(id)?.shutter_phase = value;
        Ok(())
    }

    pub fn template_controller_count(&self, id: ItemId) -> ScriptResult<i32> {
        Ok(self.require_comp(id)?.controllers.len() as i32)
    }

    pub fn template_name(&self, id: ItemId) -> ScriptResult<String> {
        Ok(self.require_comp(id)?.template_name.clone())
    }

    pub fn set_template_name(&mut self, id: ItemId, name: &str) -> ScriptResult<()> {
        self.require_comp_mut(id)?.template_name = name.to_string();
        Ok(())
    }

    pub fn template_controller_name(&self, id: ItemId, index: i32) -> ScriptResult<String> {
        self.require_comp(id)?
            .controllers
            .get(index as usize - 1)
            .map(|controller| controller.name.clone())
            .ok_or_else(|| ScriptError::range("controller"))
    }

    pub fn set_template_controller_name(&mut self, id: ItemId, index: i32, name: &str) -> ScriptResult<String> {
        let controller = self
            .require_comp_mut(id)?
            .controllers
            .get_mut(index as usize - 1)
            .ok_or_else(|| ScriptError::range("controller"))?;
        controller.name = name.to_string();
        Ok(name.to_string())
    }

    pub fn preserve_nested_frame_rate(&self, id: ItemId) -> ScriptResult<bool> {
        Ok(self.require_comp(id)?.preserve_nested_frame_rate)
    }

    pub fn set_preserve_nested_frame_rate(&mut self, id: ItemId, value: bool) -> ScriptResult<()> {
        self.require_comp_mut(id)?.preserve_nested_frame_rate = value;
        Ok(())
    }

    pub fn preserve_nested_resolution(&self, id: ItemId) -> ScriptResult<bool> {
        Ok(self.require_comp(id)?.preserve_nested_resolution)
    }

    pub fn set_preserve_nested_resolution(&mut self, id: ItemId, value: bool) -> ScriptResult<()> {
        self.require_comp_mut(id)?.preserve_nested_resolution = value;
        Ok(())
    }

    pub fn renderer(&self, id: ItemId) -> ScriptResult<String> {
        Ok(self.require_comp(id)?.renderer.clone())
    }

    pub fn set_renderer(&mut self, id: ItemId, name: &str) -> ScriptResult<()> {
        if !self.renderers(id)?.iter().any(|item| item == name) {
            return Err(ScriptError::invalid("レンダラー"));
        }
        self.require_comp_mut(id)?.renderer = name.to_string();
        Ok(())
    }

    pub fn renderers(&self, id: ItemId) -> ScriptResult<Vec<String>> {
        let _ = self.require_comp(id)?;
        Ok(vec!["Classic 3D".to_string(), "Advanced 3D".to_string()])
    }

    pub fn resolution_factor(&self, id: ItemId) -> ScriptResult<[i32; 2]> {
        Ok(self.require_comp(id)?.resolution)
    }

    pub fn set_resolution_factor(&mut self, id: ItemId, value: [i32; 2]) -> ScriptResult<()> {
        if !(1..=99).contains(&value[0]) || !(1..=99).contains(&value[1]) {
            return Err(ScriptError::range("resolutionFactor"));
        }
        self.require_comp_mut(id)?.resolution = value;
        Ok(())
    }

    pub fn selected_layers(&self, id: ItemId) -> ScriptResult<Vec<LayerId>> {
        let mut layers = Vec::new();
        for layer in &self.require_comp(id)?.layers.clone() {
            if self.layer_selected(*layer) {
                layers.push(*layer);
            }
        }
        Ok(layers)
    }

    pub fn layer_selected(&self, layer: LayerId) -> bool {
        self.layers.get(&layer).is_some_and(|layer| layer.selected)
    }

    pub fn set_layer_selected(&mut self, layer: LayerId, selected: bool) -> ScriptResult<()> {
        self.layer_mut(layer)?.selected = selected;
        Ok(())
    }

    pub fn selected_properties(&self, id: ItemId) -> ScriptResult<Vec<PropertyId>> {
        let mut props = Vec::new();
        for layer in self.require_comp(id)?.layers.clone() {
            props.extend(self.selected_properties_on_layer(layer)?);
        }
        Ok(props)
    }

    pub fn work_area_duration(&self, id: ItemId) -> ScriptResult<f64> {
        Ok(self.require_comp(id)?.work_duration)
    }

    pub fn set_work_area_duration(&mut self, id: ItemId, value: f64) -> ScriptResult<()> {
        self.require_comp_mut(id)?.work_duration = value;
        Ok(())
    }

    pub fn work_area_start(&self, id: ItemId) -> ScriptResult<f64> {
        Ok(self.require_comp(id)?.work_start)
    }

    pub fn set_work_area_start(&mut self, id: ItemId, value: f64) -> ScriptResult<()> {
        self.require_comp_mut(id)?.work_start = value;
        Ok(())
    }

    pub fn duplicate_comp(&mut self, id: ItemId) -> ScriptResult<ItemId> {
        let name = self.item_name(id)?;
        let width = self.av_width(id)?;
        let height = self.av_height(id)?;
        let pixel = self.av_pixel_aspect(id)?;
        let duration = self.av_duration(id)?;
        let rate = self.av_frame_rate(id)?;
        let copy = self.add_comp_in(self.item(id)?.parent, &format!("{name} 2"), width, height, pixel, duration, rate)?;
        let layers = self.require_comp(id)?.layers.clone();
        for layer in layers.into_iter().rev() {
            self.copy_layer_to_comp(layer, copy)?;
        }
        Ok(copy)
    }

    pub fn export_as_motion_graphics_template(&self, _id: ItemId, _overwrite: bool, _path: Option<&str>) -> ScriptResult<bool> {
        Err(ScriptError::unsupported("exportAsMotionGraphicsTemplate"))
    }

    pub fn active_camera(&self, id: ItemId) -> ScriptResult<Option<LayerId>> {
        let time = self.require_comp(id)?.time;
        for layer in &self.require_comp(id)?.layers.clone() {
            if self.layer_is_camera(*layer)? && self.layer_active_at(*layer, time).unwrap_or(false) {
                return Ok(Some(*layer));
            }
            if self.layer_is_camera(*layer)? {
                return Ok(Some(*layer));
            }
        }
        Ok(None)
    }

    pub fn template_package(&self, id: ItemId) -> ScriptResult<TemplatePackage> {
        let comp = self.require_comp(id)?;
        let mut layers = Vec::new();
        for layer in &comp.layers {
            let record = self.layer(*layer)?;
            let kind = match record.kind {
                super::LayerKind::Av { .. } => "av",
                super::LayerKind::Null => "null",
                super::LayerKind::Camera => "camera",
                super::LayerKind::Light => "light",
                super::LayerKind::Text => "text",
                super::LayerKind::Shape => "shape",
                super::LayerKind::Mesh => "mesh",
            };
            layers.push(crate::template::TemplateLayer {
                name: record.name.clone(),
                kind: kind.to_string(),
            });
        }
        let controls = comp
            .controllers
            .iter()
            .map(|controller| TemplateControl {
                name: controller.name.clone(),
                type_name: controller.type_name.clone(),
                default_number: Some(controller.default_number),
            })
            .collect();
        Ok(TemplatePackage {
            name: if comp.template_name.is_empty() {
                self.item_name(id)?
            } else {
                comp.template_name.clone()
            },
            width: comp.width,
            height: comp.height,
            duration: comp.duration,
            frame_rate: comp.frame_rate,
            layers,
            controls,
        })
    }

    pub fn import_template(&mut self, path: &std::path::Path) -> ScriptResult<ItemId> {
        let package = crate::template::oe_template_import(path)?;
        let id = self.add_comp(
            &package.name,
            package.width as i32,
            package.height as i32,
            1.0,
            package.duration,
            package.frame_rate,
        )?;
        for layer in &package.layers {
            match layer.kind.as_str() {
                "text" => {
                    self.add_text_layer(id, &layer.name)?;
                }
                "shape" => {
                    self.add_shape(id)?;
                }
                "null" => {
                    self.add_null(id, None)?;
                }
                "camera" => {
                    self.add_camera(id, &layer.name, [0.0, 0.0])?;
                }
                "light" => {
                    self.add_light(id, &layer.name, [0.0, 0.0])?;
                }
                "mesh" => {
                    self.add_parametric_mesh(id, &layer.name, ParametricMeshType::Sphere)?;
                }
                _ => {
                    self.add_null(id, None)?;
                }
            }
        }
        for control in &package.controls {
            self.require_comp_mut(id)?.controllers.push(super::Controller {
                name: control.name.clone(),
                property: None,
                layer: None,
                type_name: control.type_name.clone(),
                default_number: control.default_number.unwrap_or(0.0),
            });
        }
        Ok(id)
    }

    pub fn apply_mogrt_controls(&mut self, controls: &[TemplateControl]) -> ScriptResult<ItemId> {
        let id = self.add_comp("mogrt", 1920, 1080, 1.0, 10.0, 30.0)?;
        for control in controls {
            if control.type_name == "number" {
                self.require_comp_mut(id)?.controllers.push(super::Controller {
                    name: control.name.clone(),
                    property: None,
                    layer: None,
                    type_name: "number".to_string(),
                    default_number: control.default_number.unwrap_or(0.0),
                });
            }
        }
        Ok(id)
    }

    pub fn controller_is_number(&self, id: ItemId, name: &str) -> ScriptResult<bool> {
        Ok(self
            .require_comp(id)?
            .controllers
            .iter()
            .any(|controller| controller.name == name && controller.type_name == "number"))
    }
}
