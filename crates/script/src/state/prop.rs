use model::{
    Keyframe as ModelKeyframe, Property as ModelProperty, PropertyBody, PropertyValue, TemporalInterpolation,
    Time,
};
use model::{LayerId, PropertyId};

use super::project::eval_expression;
use super::{App, ItemBody, KeyRec, LayerKind, LayerRec, Prop, Value};
use crate::catalog::MATCHES;
use crate::enums::*;
use crate::error::{ScriptError, ScriptResult};
use crate::text::TextDocument;
use crate::values::{MarkerValue, ScriptEase, ShapeValue};

impl App {
    pub(super) fn alloc_marker_property(&mut self) -> PropertyId {
        let id = self.alloc_prop();
        self.props.insert(id, marker_prop(id, None));
        id
    }

    pub(super) fn seed_common_properties(&mut self, layer: LayerId, time_remap: bool) -> ScriptResult<()> {
        let transform = self.add_root_group(layer, "Transform", "ADBE Transform Group", false)?;
        self.add_scalar_child(transform, "Anchor Point", "ADBE Anchor Point", &["anchorPoint"], [0.0, 0.0], true)?;
        self.add_scalar_child(transform, "Position", "ADBE Position", &["position"], [0.0, 0.0], true)?;
        self.add_scalar_child(transform, "Scale", "ADBE Scale", &["scale"], [100.0, 100.0], false)?;
        self.add_scalar_child(transform, "Rotation", "ADBE Rotate Z", &["rotation", "Z Rotation", "zRotation", "Rotation Z", "rotationZ"], [0.0, 0.0], false)?;
        let opacity = self.add_number_child(transform, "Opacity", "ADBE Opacity", &["opacity"], 100.0)?;
        {
            let prop = self.prop_mut(opacity)?;
            prop.has_min = true;
            prop.has_max = true;
            prop.min_value = 0.0;
            prop.max_value = 100.0;
            prop.units = "%".to_string();
        }
        self.prop_mut(self.find_layer_prop(layer, "Rotation").unwrap())?.value = Value::Scalar(0.0);
        self.prop_mut(self.find_layer_prop(layer, "Rotation").unwrap())?.value_type = PropertyValueType::OneD;
        self.add_root_group(layer, "Masks", "ADBE Mask Parade", true)?;
        let masks = self.find_layer_prop(layer, "ADBE Mask Parade").unwrap();
        self.prop_mut(masks)?.aliases.push("Masks".to_string());
        self.add_root_group(layer, "Effects", "ADBE Effect Parade", true)?;
        let effects = self.find_layer_prop(layer, "ADBE Effect Parade").unwrap();
        self.prop_mut(effects)?.aliases.push("Effects".to_string());
        self.add_root_group(layer, "Motion Trackers", "ADBE MTrackers", true)?;
        let trackers = self.find_layer_prop(layer, "ADBE MTrackers").unwrap();
        self.prop_mut(trackers)?.aliases.push("Motion Trackers".to_string());
        let marker = self.alloc_prop();
        self.props.insert(marker, marker_prop(marker, Some(layer)));
        self.layer_mut(layer)?.root_props.push(marker);
        if time_remap {
            self.attach_time_remap(layer)?;
        }
        let has_audio = self.layer(layer)?.has_audio;
        if has_audio {
            let audio = self.add_root_group(layer, "Audio", "ADBE Audio Group", false)?;
            self.add_number_child(audio, "Audio Levels", "ADBE Audio Levels", &["audioLevels"], 0.0)?;
        }
        Ok(())
    }

    pub(super) fn attach_time_remap(&mut self, layer: LayerId) -> ScriptResult<()> {
        if self.find_layer_prop(layer, "Time Remap").is_some() {
            return Ok(());
        }
        self.add_number_root(layer, "Time Remap", "ADBE Time Remapping", &["timeRemapEnabled"], 0.0)
    }

    pub(super) fn attach_text_property(&mut self, layer: LayerId) -> ScriptResult<()> {
        let text = self.layer(layer)?.text.clone().unwrap_or_else(|| TextDocument::point(""));
        let id = self.alloc_prop();
        let mut prop = value_prop(id, Some(layer), "Source Text", "ADBE Text Document", PropertyValueType::TextDocument, Value::Text(text));
        prop.aliases = vec!["source Text".to_string(), "Text".to_string(), "text".to_string()];
        self.props.insert(id, prop);
        self.layer_mut(layer)?.root_props.push(id);
        Ok(())
    }

    pub(super) fn attach_camera_properties(&mut self, layer: LayerId) -> ScriptResult<()> {
        let group = self.add_root_group(layer, "Camera Options", "ADBE Camera Options Group", false)?;
        self.add_number_child(group, "Zoom", "ADBE Camera Zoom", &["zoom"], 1000.0)?;
        self.add_bool_child(group, "Depth of Field", "ADBE Camera Depth of Field", &["depthOfField"], false)?;
        self.add_number_child(group, "Focus Distance", "ADBE Camera Focus Distance", &["focusDistance"], 1000.0)?;
        self.add_number_child(group, "Aperture", "ADBE Camera Aperture", &["aperture"], 10.0)?;
        self.add_number_child(group, "Blur Level", "ADBE Camera Blur Level", &["blurLevel"], 100.0)?;
        self.ensure_3d_properties(layer)
    }

    pub(super) fn attach_light_properties(&mut self, layer: LayerId) -> ScriptResult<()> {
        let group = self.add_root_group(layer, "Light Options", "ADBE Light Options Group", false)?;
        self.add_number_child(group, "Intensity", "ADBE Light Intensity", &["intensity"], 100.0)?;
        self.add_color_child(group, "Color", "ADBE Light Color", &["color"], [1.0, 1.0, 1.0, 1.0])?;
        self.add_number_child(group, "Cone Angle", "ADBE Light Cone Angle", &["coneAngle"], 90.0)?;
        self.add_number_child(group, "Cone Feather", "ADBE Light Cone Feather", &["coneFeather"], 0.0)?;
        self.add_number_child(group, "Shadow Darkness", "ADBE Light Shadow Darkness", &["shadowDarkness"], 0.0)?;
        self.add_number_child(group, "Shadow Diffusion", "ADBE Light Shadow Diffusion", &["shadowDiffusion"], 0.0)?;
        self.add_bool_child(group, "Casts Shadows", "ADBE Light Casts Shadows", &["castsShadows"], false)?;
        self.ensure_3d_properties(layer)
    }

    pub(super) fn ensure_3d_properties(&mut self, layer: LayerId) -> ScriptResult<()> {
        let transform = self
            .find_layer_prop(layer, "ADBE Transform Group")
            .ok_or_else(|| ScriptError::missing("Transform"))?;
        if self.find_layer_prop(layer, "X Rotation").is_none() {
            self.add_number_child(transform, "X Rotation", "ADBE Rotate X", &["xRotation", "Rotation X", "rotationX"], 0.0)?;
            self.add_number_child(transform, "Y Rotation", "ADBE Rotate Y", &["yRotation", "Rotation Y", "rotationY"], 0.0)?;
            self.add_color_child(transform, "Orientation", "ADBE Orientation", &["orientation"], [0.0, 0.0, 0.0, 1.0])?;
            if let Some(orientation) = self.find_layer_prop(layer, "Orientation") {
                self.prop_mut(orientation)?.value = Value::Vec3([0.0, 0.0, 0.0]);
                self.prop_mut(orientation)?.value_type = PropertyValueType::ThreeD;
            }
        }
        if self.find_layer_prop(layer, "ADBE Material Options Group").is_none()
            && !matches!(self.layer(layer)?.kind, LayerKind::Light | LayerKind::Camera)
        {
            let material = self.add_root_group(layer, "Material Options", "ADBE Material Options Group", false)?;
            self.add_bool_child(material, "Accepts Shadows", "ADBE Accepts Shadows", &["acceptsShadows"], true)?;
            self.add_bool_child(material, "Accepts Lights", "ADBE Accepts Lights", &["acceptsLights"], true)?;
            self.add_number_child(material, "Ambient", "ADBE Ambient Coefficient", &["ambient"], 100.0)?;
            self.add_number_child(material, "Diffuse", "ADBE Diffuse Coefficient", &["diffuse"], 50.0)?;
            self.add_number_child(material, "Specular", "ADBE Specular Coefficient", &["specular"], 50.0)?;
            self.add_number_child(material, "Shininess", "ADBE Shininess Coefficient", &["shininess"], 5.0)?;
            self.add_bool_child(material, "Casts Shadows", "ADBE Casts Shadows", &["castsShadows"], false)?;
            self.add_number_child(material, "Light Transmission", "ADBE Light Transmission", &["lightTransmission"], 0.0)?;
            self.add_number_child(material, "Metal", "ADBE Metal Coefficient", &["metal"], 0.0)?;
        }
        Ok(())
    }

    fn add_root_group(&mut self, layer: LayerId, name: &str, match_name: &str, indexed: bool) -> ScriptResult<PropertyId> {
        let id = self.alloc_prop();
        self.props.insert(id, group_prop(id, None, Some(layer), name, match_name, indexed));
        self.layer_mut(layer)?.root_props.push(id);
        Ok(id)
    }

    fn add_number_root(
        &mut self,
        layer: LayerId,
        name: &str,
        match_name: &str,
        aliases: &[&str],
        value: f64,
    ) -> ScriptResult<()> {
        let id = self.alloc_prop();
        let mut prop = value_prop(id, Some(layer), name, match_name, PropertyValueType::OneD, Value::Scalar(value));
        prop.aliases = aliases.iter().map(|alias| (*alias).to_string()).collect();
        prop.can_vary = true;
        prop.can_set_expression = true;
        self.props.insert(id, prop);
        self.layer_mut(layer)?.root_props.push(id);
        Ok(())
    }

    fn add_number_child(
        &mut self,
        parent: PropertyId,
        name: &str,
        match_name: &str,
        aliases: &[&str],
        value: f64,
    ) -> ScriptResult<PropertyId> {
        let layer = self.prop(parent)?.layer;
        let id = self.alloc_prop();
        let mut prop = value_prop(id, layer, name, match_name, PropertyValueType::OneD, Value::Scalar(value));
        prop.parent = Some(parent);
        prop.aliases = aliases.iter().map(|alias| (*alias).to_string()).collect();
        prop.can_vary = true;
        prop.can_set_expression = true;
        self.props.insert(id, prop);
        self.prop_mut(parent)?.children.push(id);
        Ok(id)
    }

    fn add_scalar_child(
        &mut self,
        parent: PropertyId,
        name: &str,
        match_name: &str,
        aliases: &[&str],
        value: [f64; 2],
        spatial: bool,
    ) -> ScriptResult<PropertyId> {
        let layer = self.prop(parent)?.layer;
        let id = self.alloc_prop();
        let value_type = if spatial {
            PropertyValueType::TwoDSpatial
        } else {
            PropertyValueType::TwoD
        };
        let mut prop = value_prop(id, layer, name, match_name, value_type, Value::Vec2(value));
        prop.parent = Some(parent);
        prop.aliases = aliases.iter().map(|alias| (*alias).to_string()).collect();
        prop.is_spatial = spatial;
        prop.can_vary = true;
        prop.can_set_expression = true;
        self.props.insert(id, prop);
        self.prop_mut(parent)?.children.push(id);
        Ok(id)
    }

    fn add_bool_child(
        &mut self,
        parent: PropertyId,
        name: &str,
        match_name: &str,
        aliases: &[&str],
        value: bool,
    ) -> ScriptResult<PropertyId> {
        let layer = self.prop(parent)?.layer;
        let id = self.alloc_prop();
        let mut prop = value_prop(id, layer, name, match_name, PropertyValueType::OneD, Value::Bool(value));
        prop.parent = Some(parent);
        prop.aliases = aliases.iter().map(|alias| (*alias).to_string()).collect();
        self.props.insert(id, prop);
        self.prop_mut(parent)?.children.push(id);
        Ok(id)
    }

    fn add_color_child(
        &mut self,
        parent: PropertyId,
        name: &str,
        match_name: &str,
        aliases: &[&str],
        value: [f64; 4],
    ) -> ScriptResult<PropertyId> {
        let layer = self.prop(parent)?.layer;
        let id = self.alloc_prop();
        let mut prop = value_prop(id, layer, name, match_name, PropertyValueType::Color, Value::Color(value));
        prop.parent = Some(parent);
        prop.aliases = aliases.iter().map(|alias| (*alias).to_string()).collect();
        prop.can_vary = true;
        self.props.insert(id, prop);
        self.prop_mut(parent)?.children.push(id);
        Ok(id)
    }

    pub fn find_layer_prop(&self, layer: LayerId, name: &str) -> Option<PropertyId> {
        let Ok(record) = self.layer(layer) else {
            return None;
        };
        for root in &record.root_props {
            if let Some(found) = self.find_named(*root, name) {
                return Some(found);
            }
        }
        None
    }

    fn find_named(&self, id: PropertyId, name: &str) -> Option<PropertyId> {
        let prop = self.props.get(&id)?;
        if prop.match_name == name || prop.name == name || prop.aliases.iter().any(|alias| alias == name) {
            return Some(id);
        }
        for child in &prop.children {
            if let Some(found) = self.find_named(*child, name) {
                return Some(found);
            }
        }
        None
    }

    pub fn property_by_name(&self, layer: LayerId, name: &str) -> Option<PropertyId> {
        self.find_layer_prop(layer, name)
    }

    pub fn host_property(&mut self, match_name: &str) -> ScriptResult<PropertyId> {
        self.ensure_catalog();
        self.catalog
            .get(match_name)
            .copied()
            .ok_or_else(|| ScriptError::missing(match_name))
    }

    fn ensure_catalog(&mut self) {
        if !self.catalog.is_empty() {
            return;
        }
        let mut groups: Vec<(String, PropertyId)> = Vec::new();
        for entry in MATCHES {
            let group = if let Some((_, id)) = groups.iter().find(|(heading, _)| heading == entry.heading) {
                *id
            } else {
                let id = self.alloc_prop();
                let name = if entry.heading.is_empty() {
                    "Match"
                } else {
                    entry.heading
                };
                self.props.insert(
                    id,
                    group_prop(id, None, None, name, entry.heading, false),
                );
                groups.push((entry.heading.to_string(), id));
                id
            };
            let id = self.alloc_prop();
            let mut prop = value_prop(
                id,
                None,
                entry.role,
                entry.match_name,
                PropertyValueType::OneD,
                Value::Scalar(0.0),
            );
            prop.parent = Some(group);
            self.props.insert(id, prop);
            self.prop_mut(group).unwrap().children.push(id);
            self.catalog.insert(entry.match_name.to_string(), id);
        }
    }

    pub fn match_name(&self, id: PropertyId) -> ScriptResult<String> {
        Ok(self.prop(id)?.match_name.clone())
    }

    pub fn property_name(&self, id: PropertyId) -> ScriptResult<String> {
        Ok(self.prop(id)?.name.clone())
    }

    pub fn set_property_name(&mut self, id: PropertyId, name: &str) -> ScriptResult<()> {
        if !self.prop(id)?.is_effect && self.prop(id)?.property_type == PropertyType::Property {
            return Err(ScriptError::readonly("name"));
        }
        self.prop_mut(id)?.name = name.to_string();
        Ok(())
    }

    pub fn property_type(&self, id: PropertyId) -> ScriptResult<PropertyType> {
        Ok(self.prop(id)?.property_type)
    }

    pub fn property_value_type(&self, id: PropertyId) -> ScriptResult<PropertyValueType> {
        Ok(self.prop(id)?.value_type)
    }

    pub fn property_index(&self, id: PropertyId) -> ScriptResult<i32> {
        let parent = self.prop(id)?.parent;
        let layer = self.prop(id)?.layer;
        let siblings = if let Some(parent) = parent {
            self.prop(parent)?.children.clone()
        } else if let Some(layer) = layer {
            self.layer(layer)?.root_props.clone()
        } else {
            return Ok(1);
        };
        let position = siblings.iter().position(|prop| *prop == id).unwrap_or(0);
        Ok(position as i32 + 1)
    }

    pub fn property_depth(&self, id: PropertyId) -> ScriptResult<i32> {
        let mut depth = 1;
        let mut current = self.prop(id)?.parent;
        while let Some(parent) = current {
            depth += 1;
            current = self.prop(parent)?.parent;
        }
        Ok(depth)
    }

    pub fn parent_property(&self, id: PropertyId) -> ScriptResult<Option<PropertyId>> {
        Ok(self.prop(id)?.parent)
    }

    pub fn property_group(&self, id: PropertyId, count_up: i32) -> ScriptResult<Option<PropertyId>> {
        let mut current = Some(id);
        for _ in 0..count_up {
            current = current.and_then(|prop| self.props.get(&prop).and_then(|prop| prop.parent));
        }
        Ok(current)
    }

    pub fn num_properties(&self, id: PropertyId) -> ScriptResult<i32> {
        Ok(self.prop(id)?.children.len() as i32)
    }

    pub fn property_child(&self, id: PropertyId, index: i32) -> ScriptResult<PropertyId> {
        if index < 1 {
            return Err(ScriptError::range("property index"));
        }
        self.prop(id)?
            .children
            .get((index - 1) as usize)
            .copied()
            .ok_or_else(|| ScriptError::range("property index"))
    }

    pub fn property_child_by_name(&self, id: PropertyId, name: &str) -> Option<PropertyId> {
        self.find_named(id, name)
    }

    pub fn can_add_property(&self, group: PropertyId, name: &str) -> ScriptResult<bool> {
        let prop = self.prop(group)?;
        Ok(prop.property_type == PropertyType::IndexedGroup
            && (name == "ADBE Mask Atom" || name.starts_with("ADBE ") || name == "ADBE Fill"))
    }

    pub fn add_property(&mut self, group: PropertyId, name: &str) -> ScriptResult<PropertyId> {
        if !self.can_add_property(group, name)? && name != "ADBE Mask Atom" && !name.starts_with("ADBE") {
            return Err(ScriptError::invalid("足せないプロパティ"));
        }
        let layer = self.prop(group)?.layer;
        let match_name = self.prop(group)?.match_name.clone();
        if match_name == "ADBE Mask Parade" || name == "ADBE Mask Atom" {
            return self.add_mask(group, layer);
        }
        let id = self.alloc_prop();
        let mut prop = group_prop(id, Some(group), layer, name, name, false);
        prop.is_effect = true;
        prop.can_set_enabled = true;
        prop.enabled = true;
        prop.property_type = PropertyType::NamedGroup;
        self.props.insert(id, prop);
        self.prop_mut(group)?.children.push(id);
        self.touch();
        Ok(id)
    }

    fn add_mask(&mut self, group: PropertyId, layer: Option<LayerId>) -> ScriptResult<PropertyId> {
        let id = self.alloc_prop();
        let mut prop = group_prop(id, Some(group), layer, "Mask", "ADBE Mask Atom", false);
        prop.is_mask = true;
        prop.property_type = PropertyType::NamedGroup;
        prop.mask_mode = ScriptMaskMode::Add;
        self.props.insert(id, prop);
        self.prop_mut(group)?.children.push(id);
        self.add_shape_child(id, "Mask Path", "ADBE Mask Shape", &["maskShape", "maskPath"])?;
        self.add_number_child(id, "Mask Feather", "ADBE Mask Feather", &["maskFeather"], 0.0)?;
        self.add_number_child(id, "Mask Opacity", "ADBE Mask Opacity", &["maskOpacity"], 100.0)?;
        self.add_number_child(id, "Mask Expansion", "ADBE Mask Offset", &["maskOffset"], 0.0)?;
        Ok(id)
    }

    fn add_shape_child(&mut self, parent: PropertyId, name: &str, match_name: &str, aliases: &[&str]) -> ScriptResult<()> {
        let layer = self.prop(parent)?.layer;
        let id = self.alloc_prop();
        let mut prop = value_prop(id, layer, name, match_name, PropertyValueType::Shape, Value::Shape(ShapeValue::new()));
        prop.parent = Some(parent);
        prop.aliases = aliases.iter().map(|alias| (*alias).to_string()).collect();
        self.props.insert(id, prop);
        self.prop_mut(parent)?.children.push(id);
        Ok(())
    }

    pub fn add_variable_font_axis(&mut self, group: PropertyId, axis: &str) -> ScriptResult<PropertyId> {
        self.add_number_child(group, axis, axis, &[], 0.0)
    }

    pub fn effect_at(&self, layer: LayerId, index: i32) -> ScriptResult<PropertyId> {
        let effects = self
            .find_layer_prop(layer, "ADBE Effect Parade")
            .ok_or_else(|| ScriptError::missing("Effects"))?;
        if index >= 1 {
            return self.property_child(effects, index);
        }
        Err(ScriptError::range("effect"))
    }

    pub fn effect_by_name(&self, layer: LayerId, name: &str) -> ScriptResult<PropertyId> {
        let effects = self
            .find_layer_prop(layer, "ADBE Effect Parade")
            .ok_or_else(|| ScriptError::missing("Effects"))?;
        self.find_named(effects, name).ok_or_else(|| ScriptError::missing(name))
    }

    pub fn mask_at(&self, layer: LayerId, index: i32) -> ScriptResult<PropertyId> {
        let masks = self
            .find_layer_prop(layer, "ADBE Mask Parade")
            .ok_or_else(|| ScriptError::missing("Masks"))?;
        self.property_child(masks, index)
    }

    pub fn is_effect(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.is_effect)
    }

    pub fn is_mask(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.is_mask)
    }

    pub fn property_enabled(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.enabled)
    }

    pub fn set_property_enabled(&mut self, id: PropertyId, value: bool) -> ScriptResult<()> {
        self.prop_mut(id)?.enabled = value;
        Ok(())
    }

    pub fn can_set_enabled(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.can_set_enabled || self.prop(id)?.is_effect)
    }

    pub fn property_active(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.enabled)
    }

    pub fn property_elided(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.elided)
    }

    pub fn property_selected(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.selected)
    }

    pub fn set_property_selected(&mut self, id: PropertyId, value: bool) -> ScriptResult<()> {
        self.prop_mut(id)?.selected = value;
        Ok(())
    }

    pub fn selected_properties_on_layer(&self, layer: LayerId) -> ScriptResult<Vec<PropertyId>> {
        Ok(self
            .layer_prop_ids(layer)?
            .into_iter()
            .filter(|id| self.props.get(id).is_some_and(|prop| prop.selected))
            .collect())
    }

    pub(super) fn layer_prop_ids(&self, layer: LayerId) -> ScriptResult<Vec<PropertyId>> {
        let mut ids = Vec::new();
        for root in &self.layer(layer)?.root_props {
            self.collect_props(*root, &mut ids);
        }
        Ok(ids)
    }

    fn collect_props(&self, id: PropertyId, into: &mut Vec<PropertyId>) {
        into.push(id);
        if let Some(prop) = self.props.get(&id) {
            for child in &prop.children {
                self.collect_props(*child, into);
            }
        }
    }

    pub fn is_modified(&self, id: PropertyId) -> ScriptResult<bool> {
        let prop = self.prop(id)?;
        Ok(!prop.keys.is_empty() || !prop.expression.is_empty() || prop.value != default_for(&prop.value))
    }

    pub fn duplicate_property(&mut self, id: PropertyId) -> ScriptResult<PropertyId> {
        let parent = self.prop(id)?.parent.ok_or_else(|| ScriptError::invalid("複製できない"))?;
        let clone = self.clone_prop(id, parent)?;
        self.prop_mut(parent)?.children.push(clone);
        Ok(clone)
    }

    fn clone_prop(&mut self, id: PropertyId, parent: PropertyId) -> ScriptResult<PropertyId> {
        let mut copy = self.prop(id)?.clone_data();
        let new_id = self.alloc_prop();
        let children = copy.children.clone();
        copy.children.clear();
        copy.parent = Some(parent);
        self.props.insert(new_id, copy);
        for child in children {
            let cloned = self.clone_prop(child, new_id)?;
            self.prop_mut(new_id)?.children.push(cloned);
        }
        Ok(new_id)
    }

    pub fn move_property_to(&mut self, id: PropertyId, new_index: i32) -> ScriptResult<()> {
        let parent = self.prop(id)?.parent.ok_or_else(|| ScriptError::invalid("移動できない"))?;
        let children = &mut self.prop_mut(parent)?.children;
        let current = children.iter().position(|prop| *prop == id).ok_or_else(|| ScriptError::missing("プロパティ"))?;
        let id = children.remove(current);
        let index = (new_index.max(1) as usize - 1).min(children.len());
        children.insert(index, id);
        Ok(())
    }

    pub fn remove_property(&mut self, id: PropertyId) -> ScriptResult<()> {
        let children = self.prop(id)?.children.clone();
        for child in children {
            self.remove_property(child)?;
        }
        if let Some(parent) = self.prop(id)?.parent {
            self.prop_mut(parent)?.children.retain(|prop| *prop != id);
        } else if let Some(layer) = self.prop(id)?.layer {
            self.layer_mut(layer)?.root_props.retain(|prop| *prop != id);
        }
        self.props.remove(&id);
        Ok(())
    }

    pub fn num_keys(&self, id: PropertyId) -> ScriptResult<i32> {
        Ok(self.prop(id)?.keys.len() as i32)
    }

    pub fn add_key(&mut self, id: PropertyId, time: f64) -> ScriptResult<i32> {
        if let Some(index) = self.prop(id)?.keys.iter().position(|key| (key.time - time).abs() < 1e-9) {
            return Ok(index as i32 + 1);
        }
        let value = self.sample_property(id, time)?;
        self.prop_mut(id)?.keys.push(KeyRec::at(time, value));
        self.prop_mut(id)?.keys.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
        let index = self.prop(id)?.keys.iter().position(|key| (key.time - time).abs() < 1e-9).unwrap_or(0);
        self.touch();
        Ok(index as i32 + 1)
    }

    pub fn remove_key(&mut self, id: PropertyId, index: i32) -> ScriptResult<()> {
        let keys = &mut self.prop_mut(id)?.keys;
        if index < 1 || index as usize > keys.len() {
            return Err(ScriptError::range("key"));
        }
        keys.remove(index as usize - 1);
        Ok(())
    }

    pub fn key_time(&self, id: PropertyId, index: i32) -> ScriptResult<f64> {
        Ok(self.key(id, index)?.time)
    }

    pub fn key_time_by_comment(&self, id: PropertyId, comment: &str) -> ScriptResult<f64> {
        self.prop(id)?
            .keys
            .iter()
            .find(|key| matches!(&key.value, Value::Marker(marker) if marker.marker.comment == comment))
            .map(|key| key.time)
            .ok_or_else(|| ScriptError::missing("marker"))
    }

    pub fn key_value(&self, id: PropertyId, index: i32) -> ScriptResult<Value> {
        Ok(self.key(id, index)?.value.clone())
    }

    pub fn set_value(&mut self, id: PropertyId, value: Value) -> ScriptResult<()> {
        if self.prop(id)?.keys.is_empty() {
            self.prop_mut(id)?.value = value;
        } else {
            let time = self.property_time(id);
            self.set_value_at_time(id, time, value)?;
        }
        self.touch();
        Ok(())
    }

    pub fn set_value_at_key(&mut self, id: PropertyId, index: i32, value: Value) -> ScriptResult<()> {
        self.key_mut(id, index)?.value = value;
        Ok(())
    }

    pub fn set_value_at_time(&mut self, id: PropertyId, time: f64, value: Value) -> ScriptResult<()> {
        if let Some(index) = self.prop(id)?.keys.iter().position(|key| (key.time - time).abs() < 1e-9) {
            self.prop_mut(id)?.keys[index].value = value;
        } else {
            self.prop_mut(id)?.keys.push(KeyRec::at(time, value));
            self.prop_mut(id)?.keys.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
        }
        self.touch();
        Ok(())
    }

    pub fn set_values_at_times(&mut self, id: PropertyId, times: &[f64], values: &[Value]) -> ScriptResult<()> {
        if times.len() != values.len() {
            return Err(ScriptError::invalid("時刻と値の数が違う"));
        }
        for (time, value) in times.iter().zip(values.iter()) {
            self.set_value_at_time(id, *time, value.clone())?;
        }
        Ok(())
    }

    pub fn property_value(&self, id: PropertyId) -> ScriptResult<Value> {
        let time = self.property_time(id);
        self.value_at_time(id, time, false)
    }

    pub fn value_at_time(&self, id: PropertyId, time: f64, pre_expression: bool) -> ScriptResult<Value> {
        let sampled = self.sample_property(id, time)?;
        let prop = self.prop(id)?;
        if pre_expression || !prop.expression_enabled || prop.expression.is_empty() {
            return Ok(sampled);
        }
        match eval_expression(&prop.expression, &sampled) {
            Ok(value) => Ok(value),
            Err(_) => Ok(sampled),
        }
    }

    pub(super) fn sample_property(&self, id: PropertyId, time: f64) -> ScriptResult<Value> {
        let prop = self.prop(id)?;
        if prop.keys.is_empty() {
            return Ok(prop.value.clone());
        }
        let keys = &prop.keys;
        let mut left = None;
        let mut right = None;
        for (index, key) in keys.iter().enumerate() {
            if key.time <= time + 1e-9 {
                left = Some(index);
            } else if right.is_none() {
                right = Some(index);
            }
        }
        if let (Some(left_index), Some(right_index)) = (left, right) {
            let left = &keys[left_index];
            let right = &keys[right_index];
            if left.out_type == KeyframeInterpolationType::Hold || (right.time - left.time).abs() < 1e-9 {
                return Ok(hold_sample(prop, time));
            }
            let amount = (time - left.time) / (right.time - left.time);
            return Ok(lerp_value(&left.value, &right.value, amount));
        }
        if let Some(index) = left.or(right) {
            return Ok(keys[index].value.clone());
        }
        Ok(prop.value.clone())
    }

    pub(super) fn property_time(&self, id: PropertyId) -> f64 {
        let Some(layer) = self.prop(id).ok().and_then(|prop| prop.layer) else {
            return 0.0;
        };
        let Ok(record) = self.layer(layer) else {
            return 0.0;
        };
        self.items
            .get(&record.comp)
            .and_then(|item| match &item.body {
                ItemBody::Comp(comp) => Some(comp.time),
                _ => None,
            })
            .unwrap_or(0.0)
    }

    pub fn expression(&self, id: PropertyId) -> ScriptResult<String> {
        Ok(self.prop(id)?.expression.clone())
    }

    pub fn set_expression(&mut self, id: PropertyId, source: &str) -> ScriptResult<()> {
        if !self.prop(id)?.can_set_expression && self.prop(id)?.property_type != PropertyType::Property {
            return Err(ScriptError::invalid("エクスプレッションを付けられない"));
        }
        let prop = self.prop_mut(id)?;
        prop.expression = source.to_string();
        prop.expression_enabled = true;
        prop.expression_error.clear();
        let value = self.prop(id)?.value.clone();
        if let Err(error) = eval_expression(source, &value) {
            if source != "value" {
                self.prop_mut(id)?.expression_error = error;
            }
        }
        self.touch();
        Ok(())
    }

    pub fn expression_enabled(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.expression_enabled)
    }

    pub fn set_expression_enabled(&mut self, id: PropertyId, value: bool) -> ScriptResult<()> {
        self.prop_mut(id)?.expression_enabled = value;
        Ok(())
    }

    pub fn expression_error(&self, id: PropertyId) -> ScriptResult<String> {
        let prop = self.prop(id)?;
        if prop.expression_enabled && !prop.expression.is_empty() {
            let value = prop.value.clone();
            if let Err(error) = eval_expression(&prop.expression, &value) {
                return Ok(error);
            }
        }
        Ok(prop.expression_error.clone())
    }

    pub fn can_set_expression(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.can_set_expression)
    }

    pub fn can_vary_over_time(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.can_vary)
    }

    pub fn is_time_varying(&self, id: PropertyId) -> ScriptResult<bool> {
        let prop = self.prop(id)?;
        Ok(!prop.keys.is_empty() || (prop.expression_enabled && !prop.expression.is_empty()))
    }

    pub fn is_spatial(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.is_spatial)
    }

    pub fn has_min(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.has_min)
    }

    pub fn has_max(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.has_max)
    }

    pub fn min_value(&self, id: PropertyId) -> ScriptResult<f64> {
        Ok(self.prop(id)?.min_value)
    }

    pub fn max_value(&self, id: PropertyId) -> ScriptResult<f64> {
        Ok(self.prop(id)?.max_value)
    }

    pub fn units_text(&self, id: PropertyId) -> ScriptResult<String> {
        Ok(self.prop(id)?.units.clone())
    }

    pub fn value_text(&self, id: PropertyId) -> ScriptResult<String> {
        Ok(match self.property_value(id)? {
            Value::Scalar(value) => format!("{value}"),
            Value::Vec2(value) => format!("{},{}", value[0], value[1]),
            Value::Vec3(value) => format!("{},{},{}", value[0], value[1], value[2]),
            Value::Bool(value) => value.to_string(),
            Value::Text(doc) => doc.text,
            _ => String::new(),
        })
    }

    pub fn dimensions_separated(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.dimensions_separated)
    }

    pub fn set_dimensions_separated(&mut self, id: PropertyId, value: bool) -> ScriptResult<()> {
        self.prop_mut(id)?.dimensions_separated = value;
        self.prop_mut(id)?.is_leader = value;
        if value && self.prop(id)?.children.is_empty() {
            let current = match self.prop(id)?.value.clone() {
                Value::Vec2(value) => value,
                Value::Vec3(value) => [value[0], value[1]],
                Value::Scalar(value) => [value, 0.0],
                _ => [0.0, 0.0],
            };
            let x = self.add_number_child(id, "X Position", "ADBE Position_0", &[], current[0])?;
            let y = self.add_number_child(id, "Y Position", "ADBE Position_1", &[], current[1])?;
            self.prop_mut(x)?.is_follower = true;
            self.prop_mut(x)?.separation_dimension = 0;
            self.prop_mut(x)?.separation_leader = Some(id);
            self.prop_mut(y)?.is_follower = true;
            self.prop_mut(y)?.separation_dimension = 1;
            self.prop_mut(y)?.separation_leader = Some(id);
        }
        Ok(())
    }

    pub fn is_separation_leader(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.is_leader)
    }

    pub fn is_separation_follower(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.is_follower)
    }

    pub fn separation_dimension(&self, id: PropertyId) -> ScriptResult<i32> {
        Ok(self.prop(id)?.separation_dimension)
    }

    pub fn separation_leader(&self, id: PropertyId) -> ScriptResult<Option<PropertyId>> {
        Ok(self.prop(id)?.separation_leader)
    }

    pub fn separation_follower(&self, id: PropertyId, dim: i32) -> ScriptResult<PropertyId> {
        self.prop(id)?
            .children
            .get(dim as usize)
            .copied()
            .ok_or_else(|| ScriptError::range("separation"))
    }

    pub fn is_dropdown(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.dropdown)
    }

    pub fn property_parameters(&self, id: PropertyId) -> ScriptResult<Vec<String>> {
        Ok(self.prop(id)?.choices.clone())
    }

    pub fn set_property_parameters(&mut self, id: PropertyId, items: Vec<String>) -> ScriptResult<PropertyId> {
        let prop = self.prop_mut(id)?;
        prop.dropdown = true;
        prop.choices = items;
        Ok(id)
    }

    pub fn alternate_source(&self, id: PropertyId) -> ScriptResult<Option<model::ItemId>> {
        Ok(self.prop(id)?.alternate)
    }

    pub fn can_set_alternate_source(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.can_set_alternate || self.prop(id)?.property_type == PropertyType::Property)
    }

    pub fn set_alternate_source(&mut self, id: PropertyId, source: model::ItemId) -> ScriptResult<()> {
        let prop = self.prop_mut(id)?;
        prop.alternate = Some(source);
        prop.can_set_alternate = true;
        Ok(())
    }

    pub fn input_layer_and_stage(&self, id: PropertyId) -> ScriptResult<Vec<i32>> {
        let prop = self.prop(id)?;
        match (prop.input_layer, prop.input_stage) {
            (Some(layer), Some(stage)) => Ok(vec![layer, stage]),
            _ => Ok(Vec::new()),
        }
    }

    pub fn set_input_layer_and_stage(&mut self, id: PropertyId, layer_index: i32, stage: i32) -> ScriptResult<()> {
        let prop = self.prop_mut(id)?;
        prop.input_layer = Some(layer_index);
        prop.input_stage = Some(stage);
        Ok(())
    }

    pub fn layer_input_stage(&self, id: PropertyId) -> ScriptResult<Option<i32>> {
        Ok(self.prop(id)?.input_stage)
    }

    pub fn set_layer_input_stage(&mut self, id: PropertyId, stage: i32) -> ScriptResult<()> {
        self.prop_mut(id)?.input_stage = Some(stage);
        Ok(())
    }

    pub fn input_stage_cycle_limit(&self, id: PropertyId) -> ScriptResult<i32> {
        let _ = self.prop(id)?;
        Ok(LayerInputStageType::Effects.index())
    }

    pub fn nearest_key_index(&self, id: PropertyId, time: f64) -> ScriptResult<i32> {
        let keys = &self.prop(id)?.keys;
        if keys.is_empty() {
            return Err(ScriptError::missing("キーが無い"));
        }
        let mut best = 0;
        let mut best_distance = f64::MAX;
        for (index, key) in keys.iter().enumerate() {
            let distance = (key.time - time).abs();
            if distance < best_distance {
                best = index;
                best_distance = distance;
            }
        }
        Ok(best as i32 + 1)
    }

    pub fn key_in_interpolation(&self, id: PropertyId, index: i32) -> ScriptResult<KeyframeInterpolationType> {
        Ok(self.key(id, index)?.in_type)
    }

    pub fn key_out_interpolation(&self, id: PropertyId, index: i32) -> ScriptResult<KeyframeInterpolationType> {
        Ok(self.key(id, index)?.out_type)
    }

    pub fn set_interpolation_at_key(
        &mut self,
        id: PropertyId,
        index: i32,
        in_type: KeyframeInterpolationType,
        out_type: KeyframeInterpolationType,
    ) -> ScriptResult<()> {
        let key = self.key_mut(id, index)?;
        key.in_type = in_type;
        key.out_type = out_type;
        Ok(())
    }

    pub fn interpolation_valid(&self, id: PropertyId, kind: KeyframeInterpolationType, _incoming: bool) -> ScriptResult<bool> {
        let _ = self.prop(id)?;
        Ok(matches!(
            kind,
            KeyframeInterpolationType::Linear | KeyframeInterpolationType::Bezier | KeyframeInterpolationType::Hold
        ))
    }

    pub fn key_in_spatial_tangent(&self, id: PropertyId, index: i32) -> ScriptResult<Vec<f64>> {
        if !self.prop(id)?.is_spatial {
            return Err(ScriptError::invalid("空間プロパティではない"));
        }
        Ok(self.key(id, index)?.in_tangent.clone())
    }

    pub fn key_out_spatial_tangent(&self, id: PropertyId, index: i32) -> ScriptResult<Vec<f64>> {
        if !self.prop(id)?.is_spatial {
            return Err(ScriptError::invalid("空間プロパティではない"));
        }
        Ok(self.key(id, index)?.out_tangent.clone())
    }

    pub fn set_spatial_tangents(
        &mut self,
        id: PropertyId,
        index: i32,
        incoming: Vec<f64>,
        outgoing: Vec<f64>,
    ) -> ScriptResult<()> {
        if !self.prop(id)?.is_spatial {
            return Err(ScriptError::invalid("空間プロパティではない"));
        }
        let key = self.key_mut(id, index)?;
        key.in_tangent = incoming;
        key.out_tangent = outgoing;
        Ok(())
    }

    pub fn key_in_ease(&self, id: PropertyId, index: i32) -> ScriptResult<Vec<ScriptEase>> {
        Ok(self.key(id, index)?.ease_in.clone())
    }

    pub fn key_out_ease(&self, id: PropertyId, index: i32) -> ScriptResult<Vec<ScriptEase>> {
        Ok(self.key(id, index)?.ease_out.clone())
    }

    pub fn set_temporal_ease(
        &mut self,
        id: PropertyId,
        index: i32,
        incoming: Vec<ScriptEase>,
        outgoing: Vec<ScriptEase>,
    ) -> ScriptResult<()> {
        let key = self.key_mut(id, index)?;
        key.ease_in = incoming;
        key.ease_out = outgoing;
        Ok(())
    }

    pub fn key_label(&self, id: PropertyId, index: i32) -> ScriptResult<i32> {
        Ok(self.key(id, index)?.label)
    }

    pub fn set_key_label(&mut self, id: PropertyId, index: i32, label: i32) -> ScriptResult<()> {
        if !(0..=16).contains(&label) {
            return Err(ScriptError::range("label"));
        }
        self.key_mut(id, index)?.label = label;
        Ok(())
    }

    pub fn key_roving(&self, id: PropertyId, index: i32) -> ScriptResult<bool> {
        Ok(self.key(id, index)?.roving)
    }

    pub fn set_key_roving(&mut self, id: PropertyId, index: i32, value: bool) -> ScriptResult<()> {
        self.key_mut(id, index)?.roving = value;
        if value {
            self.retime_roving(id, index)?;
        }
        Ok(())
    }

    fn retime_roving(&mut self, id: PropertyId, index: i32) -> ScriptResult<()> {
        let keys = &self.prop(id)?.keys;
        let current = (index as usize).saturating_sub(1);
        if current == 0 || current + 1 >= keys.len() {
            return Ok(());
        }
        let left = &keys[current - 1];
        let mid = &keys[current];
        let right = &keys[current + 1];
        let d0 = value_distance(&left.value, &mid.value);
        let d1 = value_distance(&mid.value, &right.value);
        let total = (d0 + d1).max(1e-6);
        let time = left.time + (right.time - left.time) * (d0 / total);
        self.key_mut(id, index)?.time = time;
        Ok(())
    }

    pub fn key_selected(&self, id: PropertyId, index: i32) -> ScriptResult<bool> {
        Ok(self.key(id, index)?.selected)
    }

    pub fn set_key_selected(&mut self, id: PropertyId, index: i32, value: bool) -> ScriptResult<()> {
        self.key_mut(id, index)?.selected = value;
        Ok(())
    }

    pub fn selected_keys(&self, id: PropertyId) -> ScriptResult<Vec<i32>> {
        Ok(self
            .prop(id)?
            .keys
            .iter()
            .enumerate()
            .filter(|(_, key)| key.selected)
            .map(|(index, _)| index as i32 + 1)
            .collect())
    }

    pub fn key_spatial_auto_bezier(&self, id: PropertyId, index: i32) -> ScriptResult<bool> {
        Ok(self.key(id, index)?.spatial_auto)
    }

    pub fn set_key_spatial_auto_bezier(&mut self, id: PropertyId, index: i32, value: bool) -> ScriptResult<()> {
        self.key_mut(id, index)?.spatial_auto = value;
        Ok(())
    }

    pub fn key_spatial_continuous(&self, id: PropertyId, index: i32) -> ScriptResult<bool> {
        Ok(self.key(id, index)?.spatial_continuous)
    }

    pub fn set_key_spatial_continuous(&mut self, id: PropertyId, index: i32, value: bool) -> ScriptResult<()> {
        self.key_mut(id, index)?.spatial_continuous = value;
        Ok(())
    }

    pub fn key_temporal_auto_bezier(&self, id: PropertyId, index: i32) -> ScriptResult<bool> {
        Ok(self.key(id, index)?.temporal_auto)
    }

    pub fn set_key_temporal_auto_bezier(&mut self, id: PropertyId, index: i32, value: bool) -> ScriptResult<()> {
        self.key_mut(id, index)?.temporal_auto = value;
        Ok(())
    }

    pub fn key_temporal_continuous(&self, id: PropertyId, index: i32) -> ScriptResult<bool> {
        Ok(self.key(id, index)?.temporal_continuous)
    }

    pub fn set_key_temporal_continuous(&mut self, id: PropertyId, index: i32, value: bool) -> ScriptResult<()> {
        self.key_mut(id, index)?.temporal_continuous = value;
        Ok(())
    }

    pub fn vec2(&self, id: PropertyId) -> ScriptResult<[f64; 2]> {
        match &self.prop(id)?.value {
            Value::Vec2(value) => Ok(*value),
            Value::Vec3(value) => Ok([value[0], value[1]]),
            Value::Scalar(value) => Ok([*value, 0.0]),
            _ => Err(ScriptError::invalid("2次元ではない")),
        }
    }

    pub fn set_vec2(&mut self, id: PropertyId, value: [f64; 2]) -> ScriptResult<()> {
        self.set_value(id, Value::Vec2(value))
    }

    pub fn set_vec3(&mut self, id: PropertyId, value: [f64; 3]) -> ScriptResult<()> {
        self.set_value(id, Value::Vec3(value))
    }

    pub(super) fn sample_vec2(&self, layer: LayerId, name: &str, time: f64) -> ScriptResult<Option<[f64; 2]>> {
        let Some(prop) = self.find_layer_prop(layer, name) else {
            return Ok(None);
        };
        Ok(Some(match self.sample_property(prop, time)? {
            Value::Vec2(value) => value,
            Value::Vec3(value) => [value[0], value[1]],
            Value::Scalar(value) => [value, 0.0],
            _ => [0.0, 0.0],
        }))
    }

    pub(super) fn sample_scalar(&self, layer: LayerId, name: &str, time: f64) -> ScriptResult<Option<f64>> {
        let Some(prop) = self.find_layer_prop(layer, name) else {
            return Ok(None);
        };
        Ok(match self.sample_property(prop, time)? {
            Value::Scalar(value) => Some(value),
            _ => None,
        })
    }

    fn key(&self, id: PropertyId, index: i32) -> ScriptResult<&KeyRec> {
        self.prop(id)?
            .keys
            .get(index as usize - 1)
            .ok_or_else(|| ScriptError::range("key"))
    }

    fn key_mut(&mut self, id: PropertyId, index: i32) -> ScriptResult<&mut KeyRec> {
        self.prop_mut(id)?
            .keys
            .get_mut(index as usize - 1)
            .ok_or_else(|| ScriptError::range("key"))
    }

    pub fn mask_color(&self, id: PropertyId) -> ScriptResult<[f64; 3]> {
        Ok(self.prop(id)?.mask_color)
    }

    pub fn set_mask_color(&mut self, id: PropertyId, color: [f64; 3]) -> ScriptResult<()> {
        self.prop_mut(id)?.mask_color = color;
        Ok(())
    }

    pub fn mask_inverted(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.mask_inverted)
    }

    pub fn set_mask_inverted(&mut self, id: PropertyId, value: bool) -> ScriptResult<()> {
        self.prop_mut(id)?.mask_inverted = value;
        Ok(())
    }

    pub fn mask_locked(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.mask_locked)
    }

    pub fn set_mask_locked(&mut self, id: PropertyId, value: bool) -> ScriptResult<()> {
        self.prop_mut(id)?.mask_locked = value;
        Ok(())
    }

    pub fn mask_ui_can_edit_path(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(!self.prop(id)?.mask_locked)
    }

    pub fn mask_feather_falloff(&self, id: PropertyId) -> ScriptResult<MaskFeatherFalloff> {
        Ok(self.prop(id)?.mask_falloff)
    }

    pub fn set_mask_feather_falloff(&mut self, id: PropertyId, value: MaskFeatherFalloff) -> ScriptResult<()> {
        self.prop_mut(id)?.mask_falloff = value;
        Ok(())
    }

    pub fn mask_mode(&self, id: PropertyId) -> ScriptResult<ScriptMaskMode> {
        Ok(self.prop(id)?.mask_mode)
    }

    pub fn set_mask_mode(&mut self, id: PropertyId, value: ScriptMaskMode) -> ScriptResult<()> {
        self.prop_mut(id)?.mask_mode = value;
        Ok(())
    }

    pub fn mask_motion_blur(&self, id: PropertyId) -> ScriptResult<MaskMotionBlur> {
        Ok(self.prop(id)?.mask_motion_blur)
    }

    pub fn set_mask_motion_blur(&mut self, id: PropertyId, value: MaskMotionBlur) -> ScriptResult<()> {
        self.prop_mut(id)?.mask_motion_blur = value;
        Ok(())
    }

    pub fn roto_bezier(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.roto_bezier)
    }

    pub fn set_roto_bezier(&mut self, id: PropertyId, value: bool) -> ScriptResult<()> {
        self.prop_mut(id)?.roto_bezier = value;
        Ok(())
    }

    pub fn mask_curve_from_vertices_only(&self, id: PropertyId) -> ScriptResult<bool> {
        Ok(self.prop(id)?.roto_bezier)
    }

    pub fn add_to_template(&mut self, prop: PropertyId, comp: model::ItemId) -> ScriptResult<bool> {
        self.add_to_template_as(prop, comp, &self.prop(prop)?.name.clone())
    }

    pub fn add_to_template_as(&mut self, prop: PropertyId, comp: model::ItemId, name: &str) -> ScriptResult<bool> {
        let _ = self.require_comp(comp)?;
        let default_number = match &self.prop(prop)?.value {
            Value::Scalar(value) => *value,
            _ => 0.0,
        };
        let type_name = match self.prop(prop)?.value_type {
            PropertyValueType::OneD | PropertyValueType::TwoD | PropertyValueType::ThreeD => "number",
            _ => "number",
        };
        let layer = self.prop(prop)?.layer;
        self.require_comp_mut(comp)?.controllers.push(super::Controller {
            name: name.to_string(),
            property: Some(prop),
            layer,
            type_name: type_name.to_string(),
            default_number,
        });
        Ok(true)
    }

    pub fn can_add_to_template(&self, prop: PropertyId, comp: model::ItemId) -> ScriptResult<bool> {
        let _ = (prop, self.require_comp(comp)?);
        Ok(true)
    }

    pub fn layer_add_to_template(&mut self, layer: LayerId, comp: model::ItemId) -> ScriptResult<bool> {
        let name = self.layer_name(layer)?;
        self.layer_add_to_template_as(layer, comp, &name)
    }

    pub fn layer_add_to_template_as(&mut self, layer: LayerId, comp: model::ItemId, name: &str) -> ScriptResult<bool> {
        let _ = self.require_comp(comp)?;
        self.require_comp_mut(comp)?.controllers.push(super::Controller {
            name: name.to_string(),
            property: None,
            layer: Some(layer),
            type_name: "layer".to_string(),
            default_number: 0.0,
        });
        Ok(true)
    }

    pub fn layer_can_add_to_template(&self, _layer: LayerId, comp: model::ItemId) -> ScriptResult<bool> {
        let _ = self.require_comp(comp)?;
        Ok(true)
    }

    pub(super) fn clone_layer_record(&mut self, layer: LayerId) -> ScriptResult<LayerRec> {
        let mut copy = self.layer(layer)?.clone_shallow();
        copy.root_props.clear();
        copy.name_set = self.layer(layer)?.name_set;
        Ok(copy)
    }

    pub(super) fn rebind_layer_props(&mut self, from: LayerId, to: LayerId) -> ScriptResult<()> {
        let roots = self.layer(from)?.root_props.clone();
        for root in roots {
            let cloned = self.clone_prop_tree(root, None, Some(to))?;
            self.layer_mut(to)?.root_props.push(cloned);
        }
        Ok(())
    }

    fn clone_prop_tree(&mut self, id: PropertyId, parent: Option<PropertyId>, layer: Option<LayerId>) -> ScriptResult<PropertyId> {
        let mut copy = self.prop(id)?.clone_data();
        let children = copy.children.clone();
        copy.children.clear();
        copy.parent = parent;
        copy.layer = layer;
        let new_id = self.alloc_prop();
        self.props.insert(new_id, copy);
        for child in children {
            let cloned = self.clone_prop_tree(child, Some(new_id), layer)?;
            self.prop_mut(new_id)?.children.push(cloned);
        }
        Ok(new_id)
    }

    pub fn source_text(&self, layer: LayerId) -> ScriptResult<TextDocument> {
        if let Some(prop) = self.find_layer_prop(layer, "Source Text") {
            if let Value::Text(doc) = &self.prop(prop)?.value {
                return Ok(doc.clone());
            }
        }
        self.layer(layer)?
            .text
            .clone()
            .ok_or_else(|| ScriptError::invalid("テキストではない"))
    }

    pub fn set_source_text(&mut self, layer: LayerId, mut doc: TextDocument) -> ScriptResult<()> {
        doc.mark_applied();
        self.layer_mut(layer)?.text = Some(doc.clone());
        if let Some(prop) = self.find_layer_prop(layer, "Source Text") {
            self.prop_mut(prop)?.value = Value::Text(doc);
        }
        self.touch();
        Ok(())
    }
}

impl Prop {
    fn clone_data(&self) -> Self {
        Self {
            parent: self.parent,
            children: self.children.clone(),
            layer: self.layer,
            name: self.name.clone(),
            match_name: self.match_name.clone(),
            aliases: self.aliases.clone(),
            property_type: self.property_type,
            value_type: self.value_type,
            value: self.value.clone(),
            keys: self.keys.clone(),
            expression: self.expression.clone(),
            expression_enabled: self.expression_enabled,
            expression_error: self.expression_error.clone(),
            selected: false,
            enabled: self.enabled,
            can_set_enabled: self.can_set_enabled,
            elided: self.elided,
            is_effect: self.is_effect,
            is_mask: self.is_mask,
            can_set_expression: self.can_set_expression,
            can_vary: self.can_vary,
            is_spatial: self.is_spatial,
            has_min: self.has_min,
            has_max: self.has_max,
            min_value: self.min_value,
            max_value: self.max_value,
            units: self.units.clone(),
            dimensions_separated: self.dimensions_separated,
            separation_leader: self.separation_leader,
            separation_dimension: self.separation_dimension,
            is_leader: self.is_leader,
            is_follower: self.is_follower,
            dropdown: self.dropdown,
            choices: self.choices.clone(),
            alternate: self.alternate,
            can_set_alternate: self.can_set_alternate,
            input_layer: self.input_layer,
            input_stage: self.input_stage,
            mask_color: self.mask_color,
            mask_locked: self.mask_locked,
            mask_inverted: self.mask_inverted,
            mask_falloff: self.mask_falloff,
            mask_mode: self.mask_mode,
            mask_motion_blur: self.mask_motion_blur,
            roto_bezier: self.roto_bezier,
        }
    }
}

impl LayerRecShallow for super::LayerRec {
    fn clone_shallow(&self) -> Self {
        Self {
            name: self.name.clone(),
            name_set: self.name_set,
            comp: self.comp,
            kind: match self.kind {
                super::LayerKind::Av { source } => super::LayerKind::Av { source },
                super::LayerKind::Null => super::LayerKind::Null,
                super::LayerKind::Camera => super::LayerKind::Camera,
                super::LayerKind::Light => super::LayerKind::Light,
                super::LayerKind::Text => super::LayerKind::Text,
                super::LayerKind::Shape => super::LayerKind::Shape,
                super::LayerKind::Mesh => super::LayerKind::Mesh,
            },
            comment: self.comment.clone(),
            label: self.label,
            locked: self.locked,
            shy: self.shy,
            solo: false,
            enabled: self.enabled,
            in_point: self.in_point,
            out_point: self.out_point,
            start_time: self.start_time,
            source_span: self.source_span,
            stretch: self.stretch,
            selected: false,
            auto_orient: self.auto_orient,
            parent: None,
            guides: self.guides.clone(),
            root_props: Vec::new(),
            blend: self.blend,
            track_matte: model::TrackMatte::none(),
            matte_type: TrackMatteType::NoTrackMatte,
            preserve_transparency: self.preserve_transparency,
            collapse: self.collapse,
            motion_blur: self.motion_blur,
            three_d: self.three_d,
            three_d_per_char: self.three_d_per_char,
            adjustment: self.adjustment,
            environment: self.environment,
            guide_layer: self.guide_layer,
            frame_blending: self.frame_blending,
            frame_blending_type: self.frame_blending_type,
            effects_active: self.effects_active,
            audio_enabled: self.audio_enabled,
            quality: self.quality,
            sampling: self.sampling,
            time_remap: self.time_remap,
            has_video: self.has_video,
            has_audio: self.has_audio,
            light_type: self.light_type,
            light_source: None,
            point_of_interest: self.point_of_interest,
            focus_area_width: self.focus_area_width,
            near_far_blur: self.near_far_blur,
            camera_nodes: self.camera_nodes,
            mesh: self.mesh,
            mesh_fields: self.mesh_fields.clone(),
            bevel_fields: self.bevel_fields.clone(),
            text: self.text.clone(),
        }
    }
}

trait LayerRecShallow {
    fn clone_shallow(&self) -> Self;
}

impl KeyRec {
    fn at(time: f64, value: Value) -> Self {
        let _model = ModelKeyframe::at(Time::from_seconds(time), PropertyValue::Scalar(0.0));
        let _ = _model.temporal_out;
        Self {
            time,
            value,
            in_type: KeyframeInterpolationType::Linear,
            out_type: KeyframeInterpolationType::Linear,
            ease_in: vec![ScriptEase::default()],
            ease_out: vec![ScriptEase::default()],
            roving: false,
            selected: false,
            label: 0,
            in_tangent: vec![0.0, 0.0],
            out_tangent: vec![0.0, 0.0],
            spatial_auto: true,
            spatial_continuous: true,
            temporal_auto: false,
            temporal_continuous: false,
        }
    }
}

fn hold_sample(prop: &Prop, time: f64) -> Value {
    let mut model_keys = Vec::new();
    for key in &prop.keys {
        if let Value::Scalar(value) = key.value {
            let mut model = ModelKeyframe::at(Time::from_seconds(key.time), PropertyValue::Scalar(value));
            if key.out_type == KeyframeInterpolationType::Hold {
                model.temporal_out = TemporalInterpolation::Hold;
            }
            model_keys.push(model);
        }
    }
    if !model_keys.is_empty() {
        let base = match &prop.value {
            Value::Scalar(value) => PropertyValue::Scalar(*value),
            _ => PropertyValue::Scalar(0.0),
        };
        let sampled = eval::sample_keyframes(&base, &model_keys, Time::from_seconds(time));
        if let PropertyValue::Scalar(value) = sampled {
            if prop.keys.iter().all(|key| matches!(key.value, Value::Scalar(_))) {
                return Value::Scalar(value);
            }
        }
    }
    prop.keys
        .iter()
        .rev()
        .find(|key| key.time <= time + 1e-9)
        .map(|key| key.value.clone())
        .unwrap_or_else(|| prop.value.clone())
}

fn lerp_value(left: &Value, right: &Value, amount: f64) -> Value {
    match (left, right) {
        (Value::Scalar(a), Value::Scalar(b)) => Value::Scalar(a + (b - a) * amount),
        (Value::Vec2(a), Value::Vec2(b)) => Value::Vec2([
            a[0] + (b[0] - a[0]) * amount,
            a[1] + (b[1] - a[1]) * amount,
        ]),
        (Value::Vec3(a), Value::Vec3(b)) => Value::Vec3([
            a[0] + (b[0] - a[0]) * amount,
            a[1] + (b[1] - a[1]) * amount,
            a[2] + (b[2] - a[2]) * amount,
        ]),
        _ => left.clone(),
    }
}

fn value_distance(left: &Value, right: &Value) -> f64 {
    match (left, right) {
        (Value::Vec2(a), Value::Vec2(b)) => {
            let dx = a[0] - b[0];
            let dy = a[1] - b[1];
            (dx * dx + dy * dy).sqrt()
        }
        (Value::Scalar(a), Value::Scalar(b)) => (a - b).abs(),
        _ => 1.0,
    }
}

fn default_for(value: &Value) -> Value {
    match value {
        Value::Scalar(_) => Value::Scalar(0.0),
        Value::Vec2(_) => Value::Vec2([0.0, 0.0]),
        _ => value.clone(),
    }
}

fn group_prop(
    _id: PropertyId,
    parent: Option<PropertyId>,
    layer: Option<LayerId>,
    name: &str,
    match_name: &str,
    indexed: bool,
) -> Prop {
    Prop {
        parent,
        children: Vec::new(),
        layer,
        name: name.to_string(),
        match_name: match_name.to_string(),
        aliases: Vec::new(),
        property_type: if indexed {
            PropertyType::IndexedGroup
        } else {
            PropertyType::NamedGroup
        },
        value_type: PropertyValueType::NoValue,
        value: Value::Empty,
        keys: Vec::new(),
        expression: String::new(),
        expression_enabled: false,
        expression_error: String::new(),
        selected: false,
        enabled: true,
        can_set_enabled: false,
        elided: false,
        is_effect: false,
        is_mask: false,
        can_set_expression: false,
        can_vary: false,
        is_spatial: false,
        has_min: false,
        has_max: false,
        min_value: 0.0,
        max_value: 0.0,
        units: String::new(),
        dimensions_separated: false,
        separation_leader: None,
        separation_dimension: 0,
        is_leader: false,
        is_follower: false,
        dropdown: false,
        choices: Vec::new(),
        alternate: None,
        can_set_alternate: false,
        input_layer: None,
        input_stage: None,
        mask_color: [0.0, 0.6, 1.0],
        mask_locked: false,
        mask_inverted: false,
        mask_falloff: MaskFeatherFalloff::Linear,
        mask_mode: ScriptMaskMode::Add,
        mask_motion_blur: MaskMotionBlur::SameAsLayer,
        roto_bezier: false,
    }
}

fn value_prop(
    id: PropertyId,
    layer: Option<LayerId>,
    name: &str,
    match_name: &str,
    value_type: PropertyValueType,
    value: Value,
) -> Prop {
    let mut prop = group_prop(id, None, layer, name, match_name, false);
    prop.property_type = PropertyType::Property;
    prop.value_type = value_type;
    prop.value = value;
    prop.can_set_expression = true;
    prop.can_vary = true;
    prop
}

fn marker_prop(id: PropertyId, layer: Option<LayerId>) -> Prop {
    let mut prop = value_prop(
        id,
        layer,
        "Marker",
        "ADBE Marker",
        PropertyValueType::Marker,
        Value::Marker(MarkerValue::new("")),
    );
    prop.aliases = vec!["marker".to_string()];
    prop
}

impl Value {
    pub(crate) fn as_scalar(&self) -> Option<f64> {
        match self {
            Self::Scalar(value) => Some(*value),
            _ => None,
        }
    }
}

fn _use_model_property() -> ModelProperty {
    ModelProperty::value(
        PropertyId::new(0),
        "Opacity",
        "ADBE Opacity",
        PropertyValue::Scalar(100.0),
    )
}

fn _use_property_body(property: &ModelProperty) -> bool {
    matches!(property.body, PropertyBody::Value { .. })
}
