use std::collections::HashMap;

use model::{
    BitsPerChannel, Composition, Footage, FootageKind, ItemId, Layer, LayerId, LayerKind,
    LightKind, Marker, Project, Property, PropertyBody, PropertyId, PropertyValue, Rgba,
    ShapeContents, ShapeKind, ShapePath, Time,
};

use model::shape::ShapeNode;

use crate::value::{PathData, PropRef};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineMode {
    Javascript,
    Legacy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeDisplay {
    Timecode,
    Frames,
    FeetAndFrames,
    Ntsc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Timeline,
    Composition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rect {
    pub top: f64,
    pub left: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[f64; 4]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompAttr {
    pub ntsc_drop_frame: bool,
    pub display_start_time: f64,
    pub bg_color: [f64; 4],
    pub markers: Vec<(f64, Marker)>,
}

impl Default for CompAttr {
    fn default() -> Self {
        Self {
            ntsc_drop_frame: false,
            display_start_time: 0.0,
            bg_color: [1.0, 1.0, 1.0, 1.0],
            markers: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayerAttr {
    pub in_point: f64,
    pub out_point: f64,
    pub start_time: f64,
    pub enabled: bool,
    pub audio_enabled: bool,
    pub has_audio: bool,
    pub has_video: bool,
    pub comment: String,
    pub stretch: f64,
    pub width: f64,
    pub height: f64,
    pub source_rect: Rect,
    pub extents: f64,
    pub auto_orient: bool,
    pub image: Option<Image>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PropExtra {
    pub enabled: bool,
    pub dropdown_items: Vec<String>,
}

impl Default for PropExtra {
    fn default() -> Self {
        Self {
            enabled: true,
            dropdown_items: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reveal {
    pub comp: ItemId,
    pub layer: LayerId,
    pub property: Option<PropertyId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayerError {
    pub comp: ItemId,
    pub layer: LayerId,
    pub property: PropertyId,
    pub message: String,
    pub fix: Option<Reveal>,
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub project: Project,
    pub full_path: String,
    pub linear_blending: bool,
    pub engine_mode: EngineMode,
    pub time_display: TimeDisplay,
    pub compact_english: bool,
    pub show_banners: bool,
    pub banner_open: bool,
    pub banner_index: usize,
    pub comp_attrs: HashMap<ItemId, CompAttr>,
    pub layer_attrs: HashMap<LayerId, LayerAttr>,
    pub prop_extras: HashMap<PropertyId, PropExtra>,
    pub errors: Vec<LayerError>,
    pub open_comp: Option<ItemId>,
    pub revealed: Option<Reveal>,
    pub shown_properties: Option<Vec<PropertyId>>,
    pub(crate) layer_comp: HashMap<LayerId, ItemId>,
}

impl Scene {
    pub fn new(bits: BitsPerChannel) -> Self {
        Self {
            project: Project::new(bits),
            full_path: String::new(),
            linear_blending: false,
            engine_mode: EngineMode::Javascript,
            time_display: TimeDisplay::Timecode,
            compact_english: true,
            show_banners: true,
            banner_open: true,
            banner_index: 0,
            comp_attrs: HashMap::new(),
            layer_attrs: HashMap::new(),
            prop_extras: HashMap::new(),
            errors: Vec::new(),
            open_comp: None,
            revealed: None,
            shown_properties: None,
            layer_comp: HashMap::new(),
        }
    }

    pub fn add_comp(
        &mut self,
        name: &str,
        width: u32,
        height: u32,
        fps: f64,
        seconds: f64,
    ) -> ItemId {
        let id = self.project.alloc_item();
        self.project.add_composition(Composition::new(
            id,
            name,
            width,
            height,
            fps,
            Time::from_seconds(seconds),
        ));
        self.comp_attrs.insert(id, CompAttr::default());
        id
    }

    pub fn add_footage(&mut self, name: &str, width: u32, height: u32) -> ItemId {
        let id = self.project.alloc_item();
        self.project.add_footage(Footage::file(
            id,
            name,
            width,
            height,
            Time::from_seconds(1.0),
            25.0,
        ));
        id
    }

    pub fn add_solid_footage(
        &mut self,
        name: &str,
        width: u32,
        height: u32,
        color: Rgba,
    ) -> ItemId {
        let id = self.project.alloc_item();
        self.project.add_footage(Footage {
            id,
            name: name.into(),
            width,
            height,
            duration: Time::from_seconds(1.0),
            frame_rate: 25.0,
            kind: FootageKind::Solid { color },
        });
        id
    }

    pub fn add_layer(&mut self, comp: ItemId, name: &str, kind: LayerKind) -> LayerId {
        let id = self.project.alloc_layer();
        let (width, height) = self.source_size(kind);
        let duration = self
            .project
            .composition(comp)
            .map(|composition| composition.duration.seconds())
            .unwrap_or(0.0);
        let layer = Layer::new(id, name, kind);
        self.project
            .composition_mut(comp)
            .expect("comp")
            .layers
            .push(layer);
        self.layer_comp.insert(id, comp);
        self.layer_attrs.insert(
            id,
            LayerAttr {
                in_point: 0.0,
                out_point: duration,
                start_time: 0.0,
                enabled: true,
                audio_enabled: false,
                has_audio: false,
                has_video: true,
                comment: String::new(),
                stretch: 1.0,
                width,
                height,
                source_rect: Rect {
                    top: 0.0,
                    left: 0.0,
                    width,
                    height,
                },
                extents: 0.0,
                auto_orient: false,
                image: None,
            },
        );
        id
    }

    fn source_size(&self, kind: LayerKind) -> (f64, f64) {
        match kind {
            LayerKind::Footage { source }
            | LayerKind::Solid { source }
            | LayerKind::Placeholder { source } => self
                .project
                .footage(source)
                .map(|footage| (f64::from(footage.width), f64::from(footage.height)))
                .unwrap_or((0.0, 0.0)),
            LayerKind::Precomp { source } => self
                .project
                .composition(source)
                .map(|comp| (f64::from(comp.width), f64::from(comp.height)))
                .unwrap_or((0.0, 0.0)),
            LayerKind::Null
            | LayerKind::Adjustment
            | LayerKind::Camera { .. }
            | LayerKind::Light { .. }
            | LayerKind::Text
            | LayerKind::Shape
            | LayerKind::Guide => (0.0, 0.0),
        }
    }

    pub fn add_solid(&mut self, comp: ItemId, name: &str, width: u32, height: u32) -> LayerId {
        let footage = self.add_solid_footage(name, width, height, Rgba::white());
        self.add_layer(comp, name, LayerKind::Solid { source: footage })
    }

    pub fn add_camera(&mut self, comp: ItemId, name: &str) -> LayerId {
        let layer = self.add_layer(
            comp,
            name,
            LayerKind::Camera {
                nodes: model::CameraNodes::One,
            },
        );
        if let Some(found) = self.layer_mut(comp, layer) {
            found.three_d = true;
        }
        layer
    }

    pub fn add_light(&mut self, comp: ItemId, name: &str, kind: LightKind) -> LayerId {
        let layer = self.add_layer(comp, name, LayerKind::Light { kind });
        if let Some(found) = self.layer_mut(comp, layer) {
            found.three_d = true;
        }
        layer
    }

    pub fn add_text_layer(&mut self, comp: ItemId, name: &str, source: &str) -> LayerId {
        let layer = self.add_layer(comp, name, LayerKind::Text);
        if let Some(found) = self.layer_mut(comp, layer) {
            found.text = Some(model::Text::point(source));
        }
        layer
    }

    pub fn add_shape_layer(&mut self, comp: ItemId, name: &str) -> LayerId {
        let layer = self.add_layer(comp, name, LayerKind::Shape);
        if let Some(found) = self.layer_mut(comp, layer) {
            found.shape = Some(ShapeContents::new());
        }
        layer
    }

    pub fn layer(&self, comp: ItemId, layer: LayerId) -> Option<&Layer> {
        self.project.composition(comp)?.layer(layer)
    }

    pub fn layer_mut(&mut self, comp: ItemId, layer: LayerId) -> Option<&mut Layer> {
        self.project.composition_mut(comp)?.layer_mut(layer)
    }

    pub fn layer_attr(&self, layer: LayerId) -> Option<&LayerAttr> {
        self.layer_attrs.get(&layer)
    }

    pub fn layer_attr_mut(&mut self, layer: LayerId) -> Option<&mut LayerAttr> {
        self.layer_attrs.get_mut(&layer)
    }

    pub fn comp_attr(&self, comp: ItemId) -> Option<&CompAttr> {
        self.comp_attrs.get(&comp)
    }

    pub fn comp_attr_mut(&mut self, comp: ItemId) -> Option<&mut CompAttr> {
        self.comp_attrs.get_mut(&comp)
    }

    pub fn prop_extra(&self, id: PropertyId) -> PropExtra {
        self.prop_extras.get(&id).cloned().unwrap_or_default()
    }

    pub fn prop_extra_mut(&mut self, id: PropertyId) -> &mut PropExtra {
        self.prop_extras.entry(id).or_default()
    }

    pub fn comp_of_layer(&self, layer: LayerId) -> Option<ItemId> {
        self.layer_comp.get(&layer).copied()
    }

    pub fn set_parent(&mut self, comp: ItemId, layer: LayerId, parent: Option<LayerId>) {
        if let Some(found) = self.layer_mut(comp, layer) {
            found.parent = parent;
        }
    }

    pub fn set_3d(&mut self, comp: ItemId, layer: LayerId, enabled: bool) {
        if let Some(found) = self.layer_mut(comp, layer) {
            found.three_d = enabled;
        }
    }

    pub fn add_group(
        &mut self,
        comp: ItemId,
        layer: LayerId,
        parent: Option<PropertyId>,
        name: &str,
        match_name: &str,
    ) -> PropertyId {
        let id = self.project.alloc_property();
        self.insert(comp, layer, parent, Property::group(id, name, match_name));
        self.prop_extras.insert(id, PropExtra::default());
        id
    }

    pub fn add_value(
        &mut self,
        comp: ItemId,
        layer: LayerId,
        parent: Option<PropertyId>,
        name: &str,
        match_name: &str,
        value: PropertyValue,
    ) -> PropertyId {
        let id = self.project.alloc_property();
        self.insert(
            comp,
            layer,
            parent,
            Property::value(id, name, match_name, value),
        );
        self.prop_extras.insert(id, PropExtra::default());
        id
    }

    pub fn add_key(
        &mut self,
        comp: ItemId,
        layer: LayerId,
        prop: PropertyId,
        time: f64,
        value: PropertyValue,
    ) -> bool {
        let Some(property) = self.property_mut(comp, layer, prop) else {
            return false;
        };
        property.push_keyframe(model::Keyframe::at(Time::from_seconds(time), value))
    }

    pub fn set_expression(
        &mut self,
        comp: ItemId,
        layer: LayerId,
        prop: PropertyId,
        source: Option<String>,
        enabled: bool,
    ) -> bool {
        let Some(property) = self.property_mut(comp, layer, prop) else {
            return false;
        };
        match &mut property.body {
            PropertyBody::Value {
                expression,
                expression_enabled,
                ..
            } => {
                *expression = source;
                *expression_enabled = enabled;
                true
            }
            PropertyBody::Group { .. } => false,
        }
    }

    pub fn expression_of(
        &self,
        comp: ItemId,
        layer: LayerId,
        prop: PropertyId,
    ) -> Option<(String, bool)> {
        let property = self.property(comp, layer, prop)?;
        match &property.body {
            PropertyBody::Value {
                expression,
                expression_enabled,
                ..
            } => expression
                .as_ref()
                .map(|source| (source.clone(), *expression_enabled)),
            PropertyBody::Group { .. } => None,
        }
    }

    fn insert(
        &mut self,
        comp: ItemId,
        layer: LayerId,
        parent: Option<PropertyId>,
        property: Property,
    ) {
        let Some(layer) = self.layer_mut(comp, layer) else {
            return;
        };
        if let Some(parent) = parent {
            if let Some(parent) = find_in_list(&mut layer.properties, parent) {
                parent.push_child(property);
            }
        } else {
            layer.properties.push(property);
        }
    }

    pub fn property(&self, comp: ItemId, layer: LayerId, id: PropertyId) -> Option<&Property> {
        let layer = self.layer(comp, layer)?;
        find_in_list_ref(&layer.properties, id)
    }

    pub fn property_mut(
        &mut self,
        comp: ItemId,
        layer: LayerId,
        id: PropertyId,
    ) -> Option<&mut Property> {
        let layer = self.layer_mut(comp, layer)?;
        find_in_list(&mut layer.properties, id)
    }

    pub fn add_mask(&mut self, comp: ItemId, layer: LayerId, name: &str, path: ShapePath) -> usize {
        let id = self.project.alloc_property();
        let mut mask = model::Mask::new(id, name);
        mask.path = path;
        let Some(layer) = self.layer_mut(comp, layer) else {
            return 0;
        };
        layer.masks.push(mask);
        layer.masks.len() - 1
    }

    pub fn set_shape_path(
        &mut self,
        comp: ItemId,
        layer: LayerId,
        group: &str,
        path_name: &str,
        path: ShapePath,
    ) {
        let group_id = self.project.alloc_property();
        let path_id = self.project.alloc_property();
        let prop_id = self.project.alloc_property();
        let mut path_node = ShapeNode::new(path_id, path_name, ShapeKind::Path);
        path_node.properties.push(Property::value(
            prop_id,
            "Path",
            "path",
            PropertyValue::path(path),
        ));
        let mut group_node = ShapeNode::new(group_id, group, ShapeKind::Group);
        group_node.children.push(path_node);
        if let Some(layer) = self.layer_mut(comp, layer) {
            let shape = layer.shape.get_or_insert_with(ShapeContents::new);
            shape.nodes.push(group_node);
        }
    }

    pub fn clear_error(&mut self, property: PropertyId) {
        self.errors.retain(|error| error.property != property);
    }

    pub fn record_error(&mut self, target: crate::Target, message: String) {
        self.errors
            .retain(|error| error.property != target.property);
        self.errors.push(LayerError {
            comp: target.comp,
            layer: target.layer,
            property: target.property,
            message,
            fix: None,
        });
    }

    pub fn set_fix(&mut self, property: PropertyId, fix: Reveal) {
        if let Some(error) = self
            .errors
            .iter_mut()
            .find(|error| error.property == property)
        {
            error.fix = Some(fix);
        }
    }

    pub fn errors_for(&self, layer: LayerId) -> Vec<&LayerError> {
        self.errors
            .iter()
            .filter(|error| error.layer == layer)
            .collect()
    }
}

fn contains(property: &Property, id: PropertyId) -> bool {
    if property.id == id {
        return true;
    }
    match &property.body {
        PropertyBody::Group { children } => children.iter().any(|child| contains(child, id)),
        PropertyBody::Value { .. } => false,
    }
}

fn find_in_list(list: &mut [Property], id: PropertyId) -> Option<&mut Property> {
    let mut index = 0;
    while index < list.len() {
        if list[index].id == id {
            return Some(&mut list[index]);
        }
        if contains(&list[index], id) {
            return match &mut list[index].body {
                PropertyBody::Group { children } => find_in_list(children, id),
                PropertyBody::Value { .. } => None,
            };
        }
        index += 1;
    }
    None
}

fn find_in_list_ref(list: &[Property], id: PropertyId) -> Option<&Property> {
    for property in list {
        if property.id == id {
            return Some(property);
        }
        if let PropertyBody::Group { children } = &property.body {
            if let Some(found) = find_in_list_ref(children, id) {
                return Some(found);
            }
        }
    }
    None
}

pub fn path_from_shape(path: &ShapePath) -> PathData {
    PathData {
        points: path.vertices.clone(),
        in_tangents: path.in_tangents.clone(),
        out_tangents: path.out_tangents.clone(),
        closed: path.closed,
    }
}

pub fn prop_ref(comp: ItemId, layer: LayerId, id: PropertyId) -> PropRef {
    PropRef { comp, layer, id }
}
