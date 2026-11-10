use model::color::Rgba;
use model::composition::{CompPreset, Composition, Renderer};
use model::folder::FolderChild;
use model::footage::{solid_size_allowed, Footage, FootageKind, FootageProxy, Interpretation};
use model::id::{ItemId, LayerId};
use model::keyframe::TimedMarker;
use model::layer::{Layer, LayerKind};
use model::project::{LabelColor, Project, RememberedCompSettings, SOLIDS_FOLDER};
use model::property::{Property, PropertyValue};
use model::shape::{ShapeContents, ShapeKind, ShapeNode};
use model::text::Text;
use model::time::Time;

pub fn displayed_source_name<'a>(project: &'a Project, layer: &'a Layer) -> &'a str {
    layer
        .source_id()
        .and_then(|id| project.footage(id).or(None))
        .map(|footage| footage.name.as_str())
        .or_else(|| {
            layer.source_id().and_then(|id| {
                project
                    .composition(id)
                    .map(|composition| composition.name.as_str())
            })
        })
        .unwrap_or(layer.name.as_str())
}

pub fn import_solid(project: &mut Project, name: &str, width: u32, height: u32, color: Rgba) -> Option<ItemId> {
    if !solid_size_allowed(width, height) {
        return None;
    }
    let id = project.alloc_item();
    let mut footage = Footage::file(id, name, width, height, Time::ZERO, 1.0);
    footage.kind = FootageKind::Solid { color };
    footage.label = project.labels.defaults.solid;
    let folder_id = solids_folder(project);
    project.footage.push(footage);
    if let Some(folder) = project.root.child_folder_mut(SOLIDS_FOLDER) {
        folder.children.push(FolderChild::Footage(id));
    } else {
        let _ = folder_id;
        project.root.children.push(FolderChild::Footage(id));
    }
    Some(id)
}

fn solids_folder(project: &mut Project) -> ItemId {
    if project.root.child_folder(SOLIDS_FOLDER).is_none() {
        let id = project.alloc_item();
        project.root.children.push(FolderChild::Folder(model::folder::Folder::new(id, SOLIDS_FOLDER)));
        id
    } else {
        project.root.child_folder(SOLIDS_FOLDER).map(|folder| folder.id).unwrap_or(ItemId::new(0))
    }
}

pub fn new_solid_layer(
    project: &mut Project,
    comp: ItemId,
    name: &str,
    width: u32,
    height: u32,
    color: Rgba,
) -> Option<LayerId> {
    let footage_id = import_solid(project, name, width, height, color)?;
    let layer_id = project.alloc_layer();
    let mut layer = Layer::new(layer_id, name, LayerKind::Solid { source: footage_id });
    layer.label = project.labels.defaults.solid;
    layer.in_point = Time::ZERO;
    layer.out_point = Time::from_seconds(project.composition(comp)?.duration.seconds());
    project.composition_mut(comp)?.layers.insert(0, layer);
    Some(layer_id)
}

pub fn edit_solid(
    project: &mut Project,
    comp: ItemId,
    layer_id: LayerId,
    color: Rgba,
    affect_all: bool,
) -> Option<ItemId> {
    let source = project.composition(comp)?.layer(layer_id)?.source_id()?;
    if affect_all {
        if let FootageKind::Solid { color: slot } = &mut project.footage_mut(source)?.kind {
            *slot = color;
        }
        Some(source)
    } else {
        let original = project.footage(source)?.clone();
        let id = project.alloc_item();
        let mut copy = original;
        copy.id = id;
        copy.kind = FootageKind::Solid { color };
        project.footage.push(copy);
        project.root.children.push(FolderChild::Footage(id));
        if let LayerKind::Solid { source } = &mut project.composition_mut(comp)?.layer_mut(layer_id)?.kind {
            *source = id;
        }
        Some(id)
    }
}

pub fn import_placeholder(project: &mut Project, name: &str, width: u32, height: u32, duration: Time, fps: f64) -> ItemId {
    let id = project.alloc_item();
    let mut footage = Footage::file(id, name, width, height, duration, fps);
    footage.kind = FootageKind::Placeholder;
    footage.pixels = color_bars(width, height);
    project.add_footage(footage);
    id
}

pub fn replace_with_placeholder(project: &mut Project, footage_id: ItemId) {
    if let Some(footage) = project.footage_mut(footage_id) {
        footage.kind = FootageKind::Placeholder;
        footage.pixels = color_bars(footage.width, footage.height);
    }
}

pub fn replace_with_file(project: &mut Project, footage_id: ItemId, name: &str, pixels: Vec<Rgba>, width: u32, height: u32) {
    if let Some(footage) = project.footage_mut(footage_id) {
        footage.kind = FootageKind::File;
        footage.name = name.to_string();
        footage.pixels = pixels;
        footage.width = width;
        footage.height = height;
        footage.missing = false;
    }
}

pub fn mark_missing(project: &mut Project, footage_id: ItemId) {
    if let Some(footage) = project.footage_mut(footage_id) {
        footage.missing = true;
        footage.pixels.clear();
    }
}

pub fn relink(project: &mut Project, footage_id: ItemId, pixels: Vec<Rgba>) {
    if let Some(footage) = project.footage_mut(footage_id) {
        footage.missing = false;
        footage.pixels = pixels;
    }
}

pub fn set_proxy(project: &mut Project, footage_id: ItemId, proxy: FootageProxy) {
    if let Some(footage) = project.footage_mut(footage_id) {
        footage.proxy = Some(proxy);
    }
}

pub fn set_proxy_enabled(project: &mut Project, footage_id: ItemId, enabled: bool) -> bool {
    match project.footage_mut(footage_id).and_then(|footage| footage.proxy.as_mut()) {
        Some(proxy) => {
            proxy.in_use = enabled;
            true
        }
        None => false,
    }
}

pub fn clear_proxy(project: &mut Project, footage_id: ItemId) {
    if let Some(footage) = project.footage_mut(footage_id) {
        footage.proxy = None;
    }
}

pub fn create_proxy_from_render(
    project: &mut Project,
    footage_id: ItemId,
    path: &str,
    pixels: Vec<Rgba>,
    still: bool,
) {
    let Some(footage) = project.footage(footage_id) else {
        return;
    };
    let proxy = FootageProxy {
        path: path.to_string(),
        width: footage.width,
        height: footage.height,
        pixels,
        in_use: true,
        still,
        interpretation: Interpretation::default(),
    };
    set_proxy(project, footage_id, proxy);
}

pub fn remember_interpretation(project: &mut Project, footage_id: ItemId) {
    if let Some(footage) = project.footage(footage_id) {
        project.remembered_interpretation = Some(footage.interpretation.clone());
    }
}

pub fn apply_interpretation(project: &mut Project, footage_ids: &[ItemId]) {
    let Some(remembered) = project.remembered_interpretation.clone() else {
        return;
    };
    for id in footage_ids {
        if let Some(footage) = project.footage_mut(*id) {
            footage.interpretation = remembered.clone();
        }
    }
}

pub fn import_media(project: &mut Project, kind: &str, duration: Time, frame_rate: f64) -> ItemId {
    let id = project.alloc_item();
    let mut footage = Footage::file(id, kind, 4, 4, duration, frame_rate);
    footage.label = match kind {
        "audio" => project.labels.defaults.audio,
        "still" => project.labels.defaults.still,
        _ => project.labels.defaults.video,
    };
    project.add_footage(footage);
    id
}

pub fn add_footage_layer(project: &mut Project, comp: ItemId, footage_id: ItemId) -> Option<LayerId> {
    let footage = project.footage(footage_id)?.clone();
    let layer_id = project.alloc_layer();
    let mut layer = Layer::new(layer_id, footage.name.clone(), LayerKind::Footage { source: footage_id });
    layer.in_point = Time::ZERO;
    layer.out_point = footage.duration;
    layer.label = footage.label.or(match footage.duration.seconds() {
        seconds if seconds <= footage.frame_rate.recip() => project.labels.defaults.still,
        _ => project.labels.defaults.video,
    });
    project.composition_mut(comp)?.layers.insert(0, layer);
    Some(layer_id)
}

pub fn add_text_layer(project: &mut Project, comp: ItemId, text: &str) -> Option<LayerId> {
    let layer_id = project.alloc_layer();
    let mut layer = Layer::new(layer_id, "Text", LayerKind::Text);
    layer.text = Some(Text::point(text));
    layer.label = project.labels.defaults.text;
    insert_front(project, comp, layer)?;
    Some(layer_id)
}

fn insert_front(project: &mut Project, comp: ItemId, layer: Layer) -> Option<()> {
    project.composition_mut(comp)?.layers.insert(0, layer);
    Some(())
}

pub fn add_shape_rect(project: &mut Project, comp: ItemId, width: f64, height: f64, shift: bool) -> Option<(LayerId, f64, f64)> {
    let (width, height) = if shift {
        let side = width.abs().max(height.abs());
        (side, side)
    } else {
        (width, height)
    };
    let layer_id = project.alloc_layer();
    let mut layer = Layer::new(layer_id, "Shape", LayerKind::Shape);
    layer.label = project.labels.defaults.shape;
    let mut node = ShapeNode::new(project.alloc_property(), "Rectangle", ShapeKind::Path);
    node.properties.push(Property::value(
        project.alloc_property(),
        "Size",
        "Size",
        PropertyValue::TwoD([width, height]),
    ));
    layer.shape = Some(ShapeContents { nodes: vec![node] });
    insert_front(project, comp, layer)?;
    Some((layer_id, width, height))
}

pub fn add_null(project: &mut Project, comp: ItemId) -> Option<LayerId> {
    let layer_id = project.alloc_layer();
    let mut layer = Layer::new(layer_id, "Null", LayerKind::Null);
    layer.label = project.labels.defaults.null_object;
    insert_front(project, comp, layer)?;
    Some(layer_id)
}

pub fn add_adjustment(project: &mut Project, comp: ItemId) -> Option<LayerId> {
    let layer_id = project.alloc_layer();
    let mut layer = Layer::new(layer_id, "Adjustment", LayerKind::Adjustment);
    layer.adjustment = true;
    layer.label = project.labels.defaults.adjustment;
    insert_front(project, comp, layer)?;
    Some(layer_id)
}

pub fn add_camera(project: &mut Project, comp: ItemId, focal: Option<f64>) -> Option<LayerId> {
    let layer_id = project.alloc_layer();
    let mut layer = Layer::new(
        layer_id,
        "Camera",
        LayerKind::Camera {
            nodes: model::layer::CameraNodes::One,
        },
    );
    layer.label = project.labels.defaults.camera;
    layer.focal_length = focal;
    insert_front(project, comp, layer)?;
    Some(layer_id)
}

pub fn add_light(project: &mut Project, comp: ItemId, intensity: f64) -> Option<LayerId> {
    let layer_id = project.alloc_layer();
    let mut layer = Layer::new(
        layer_id,
        "Light",
        LayerKind::Light {
            kind: model::layer::LightKind::Point,
        },
    );
    layer.label = project.labels.defaults.light;
    layer.opacity = intensity;
    insert_front(project, comp, layer)?;
    Some(layer_id)
}

pub fn add_photoshop_layer(project: &mut Project, comp: ItemId, name: &str, width: u32, height: u32) -> Option<LayerId> {
    let id = project.alloc_item();
    let mut footage = Footage::file(id, name, width, height, Time::ZERO, 1.0);
    footage.kind = FootageKind::Photoshop;
    footage.pixels = vec![Rgba::white(); (width * height) as usize];
    project.add_footage(footage);
    let layer_id = project.alloc_layer();
    let layer = Layer::new(layer_id, name, LayerKind::Footage { source: id });
    insert_front(project, comp, layer)?;
    Some(layer_id)
}

pub fn precompose(project: &mut Project, comp: ItemId, layer_ids: &[LayerId], name: &str) -> Option<ItemId> {
    let composition = project.composition(comp)?.clone();
    let mut moving = Vec::new();
    for layer in composition.layers.iter().rev() {
        if layer_ids.contains(&layer.id) {
            moving.push(layer.clone());
        }
    }
    moving.reverse();
    let child_id = project.alloc_item();
    let mut child = Composition::new(
        child_id,
        name,
        composition.width,
        composition.height,
        composition.frame_rate,
        composition.duration,
    );
    child.layers = moving;
    child.background = Rgba::transparent();
    project.add_composition(child);
    let duration = composition.duration;
    let insert_at = {
        let parent = project.composition_mut(comp)?;
        let insert_at = parent
            .layers
            .iter()
            .position(|layer| layer_ids.contains(&layer.id))
            .unwrap_or(0);
        parent.layers.retain(|layer| !layer_ids.contains(&layer.id));
        insert_at
    };
    let layer_id = project.alloc_layer();
    let mut layer = Layer::new(layer_id, name, LayerKind::Precomp { source: child_id });
    layer.in_point = Time::ZERO;
    layer.out_point = duration;
    layer.transform.scale = [1.0, 1.0, 1.0];
    project.composition_mut(comp)?.layers.insert(insert_at, layer);
    Some(child_id)
}

pub fn confirm_comp_settings(project: &mut Project, comp: ItemId) {
    let Some(composition) = project.composition(comp) else {
        return;
    };
    project.last_comp_settings = Some(RememberedCompSettings {
        width: composition.width,
        height: composition.height,
        pixel_aspect: composition.pixel_aspect,
        frame_rate: composition.frame_rate,
        resolution: composition.resolution,
        start: composition.start,
        duration: composition.duration,
        background: composition.background,
        motion_blur: composition.motion_blur,
        preserve_frame_rate: composition.preserve_frame_rate,
        preserve_resolution: composition.preserve_resolution,
        renderer: composition.renderer,
        drop_frame: composition.drop_frame,
        anchor: composition.anchor,
        cinema4d: composition.cinema4d,
    });
}

pub fn new_comp_from_last(project: &mut Project, name: &str) -> Option<ItemId> {
    let settings = project.last_comp_settings.clone()?;
    let id = project.alloc_item();
    let mut composition = Composition::new(
        id,
        name,
        settings.width,
        settings.height,
        settings.frame_rate,
        settings.duration,
    );
    composition.pixel_aspect = settings.pixel_aspect;
    composition.resolution = settings.resolution;
    composition.start = settings.start;
    composition.background = settings.background;
    composition.motion_blur = settings.motion_blur;
    composition.renderer = settings.renderer;
    composition.drop_frame = settings.drop_frame;
    composition.anchor = settings.anchor;
    composition.cinema4d = settings.cinema4d;
    composition.preserve_frame_rate = false;
    composition.preserve_resolution = false;
    composition.label = project.labels.defaults.composition;
    project.add_composition(composition);
    Some(id)
}

pub fn save_preset(project: &mut Project, comp: ItemId, name: &str) -> Option<()> {
    let composition = project.composition(comp)?;
    project.presets.save_user(CompPreset {
        name: name.to_string(),
        width: composition.width,
        height: composition.height,
        pixel_aspect: composition.pixel_aspect,
        frame_rate: composition.frame_rate,
        bundled: false,
    });
    Some(())
}

pub fn apply_preset(project: &mut Project, comp: ItemId, name: &str) -> Option<()> {
    let preset = project.presets.all().find(|preset| preset.name == name)?.clone();
    let composition = project.composition_mut(comp)?;
    let duration = composition.duration;
    let resolution = composition.resolution;
    let start = composition.start;
    let preserve_frame_rate = composition.preserve_frame_rate;
    let preserve_resolution = composition.preserve_resolution;
    composition.width = preset.width;
    composition.height = preset.height;
    composition.pixel_aspect = preset.pixel_aspect;
    composition.frame_rate = preset.frame_rate;
    composition.duration = duration;
    composition.resolution = resolution;
    composition.start = start;
    composition.preserve_frame_rate = preserve_frame_rate;
    composition.preserve_resolution = preserve_resolution;
    Some(())
}

pub fn render_order(project: &Project, comp: ItemId) -> Vec<ItemId> {
    let mut order = Vec::new();
    push_order(project, comp, &mut order);
    order
}

fn push_order(project: &Project, comp: ItemId, order: &mut Vec<ItemId>) {
    let Some(composition) = project.composition(comp) else {
        return;
    };
    for layer in &composition.layers {
        if let Some(source) = layer.source_id() {
            if let Some(footage) = project.footage(source) {
                if let FootageKind::OutputPlaceholder { comp: dependency } = footage.kind {
                    push_order(project, dependency, order);
                }
            } else if project.composition(source).is_some() {
                push_order(project, source, order);
            }
        }
    }
    if !order.contains(&comp) {
        order.push(comp);
    }
}

pub fn assign_parent(composition: &mut Composition, child_id: LayerId, parent_id: LayerId, alt: bool) {
    let Some(parent_transform) = composition.layer(parent_id).map(|layer| layer.transform) else {
        return;
    };
    let Some(child) = composition.layer_mut(child_id) else {
        return;
    };
    if alt {
        child.transform = child.transform.parent_taking_transform(parent_transform);
    } else {
        child.transform = child.transform.parent_keeping_world(parent_transform);
    }
    child.parent = Some(parent_id);
}

pub fn clear_parent(composition: &mut Composition, child_id: LayerId, alt: bool) {
    let Some(parent_id) = composition.layer(child_id).and_then(|layer| layer.parent) else {
        return;
    };
    let Some(parent_transform) = composition.layer(parent_id).map(|layer| layer.transform) else {
        return;
    };
    let Some(child) = composition.layer_mut(child_id) else {
        return;
    };
    if alt {
        child.transform = child.transform.unparent_releasing(parent_transform);
    } else {
        child.transform = child.transform.unparent_keeping_world(parent_transform);
    }
    child.parent = None;
}

pub fn world_position(composition: &Composition, layer_id: LayerId) -> Option<[f64; 3]> {
    let layer = composition.layer(layer_id)?;
    let parent = layer
        .parent
        .and_then(|parent| composition.layer(parent))
        .map(|parent| parent.transform);
    Some(layer.transform.world_position(parent))
}

pub fn select_children(composition: &Composition, parent_id: LayerId) -> Vec<LayerId> {
    composition
        .layers
        .iter()
        .filter(|layer| layer.parent == Some(parent_id))
        .map(|layer| layer.id)
        .collect()
}

pub fn select_label_group(composition: &Composition, label: u8) -> Vec<LayerId> {
    composition
        .layers
        .iter()
        .filter(|layer| layer.label == Some(label))
        .map(|layer| layer.id)
        .collect()
}

pub fn search_layers(project: &Project, composition: &Composition, query: &str) -> Vec<LayerId> {
    let query = query.to_ascii_lowercase();
    composition
        .layers
        .iter()
        .filter(|layer| {
            if !timeline_lists(composition, layer) {
                return false;
            }
            layer.name.to_ascii_lowercase().contains(&query)
                || layer.comment.to_ascii_lowercase().contains(&query)
                || label_name(project, layer.label).to_ascii_lowercase().contains(&query)
        })
        .map(|layer| layer.id)
        .collect()
}

pub fn timeline_lists(composition: &Composition, layer: &Layer) -> bool {
    !(composition.hide_shy_layers && layer.shy)
}

pub fn label_name(project: &Project, label: Option<u8>) -> &str {
    match label {
        Some(index) if index >= 1 && (index as usize) <= project.labels.labels.len() => {
            project.labels.labels[index as usize - 1].name.as_str()
        }
        _ => "",
    }
}

pub fn handle_color(project: &Project, layer: &Layer) -> LabelColor {
    if project.labels.use_for_handles == Some(true) {
        if let Some(index) = layer.label {
            if let Some(color) = project
                .labels
                .labels
                .get(index as usize - 1)
                .and_then(|label| label.color)
            {
                return color;
            }
        }
    }
    project.common_handle_color
}

pub fn tab_color(project: &Project, composition: &Composition) -> Option<LabelColor> {
    if project.labels.use_for_tabs != Some(true) {
        return None;
    }
    let index = composition.label?;
    project.labels.labels.get(index as usize - 1).and_then(|label| label.color)
}

pub fn try_move(layer: &mut Layer, delta: [f64; 3]) -> bool {
    if layer.locked {
        return false;
    }
    layer.transform.position[0] += delta[0];
    layer.transform.position[1] += delta[1];
    layer.transform.position[2] += delta[2];
    true
}

pub fn solo_only(composition: &mut Composition, layer_id: LayerId) {
    for layer in &mut composition.layers {
        layer.solo = layer.id == layer_id;
    }
}

pub fn cycle_blend(layer: &mut Layer, forward: bool) {
    layer.blend_mode = if forward {
        layer.blend_mode.next()
    } else {
        layer.blend_mode.previous()
    };
}

pub fn sequence_layers(composition: &mut Composition, ids: &[LayerId]) {
    let mut cursor = composition
        .layer(ids[0])
        .map(|layer| layer.in_point.seconds())
        .unwrap_or(0.0);
    for id in ids {
        let Some(layer) = composition.layer_mut(*id) else {
            continue;
        };
        let span = layer.out_point.seconds() - layer.in_point.seconds();
        layer.in_point = Time::from_seconds(cursor);
        layer.out_point = Time::from_seconds(cursor + span);
        cursor += span;
    }
}

pub fn visible_in_preview(composition: &Composition, layer: &Layer) -> bool {
    if !layer.video && layer.is_visual() && !layer.is_adjustment() {
        return false;
    }
    let visual_solo = composition.layers.iter().any(|item| item.solo && item.is_visual());
    if visual_solo && layer.is_visual() && !layer.solo {
        return false;
    }
    true
}

pub fn included_in_render(composition: &Composition, layer: &Layer) -> bool {
    if layer.is_guide() {
        return false;
    }
    visible_in_preview(composition, layer)
}

pub fn color_bars(width: u32, height: u32) -> Vec<Rgba> {
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            let band = if width == 0 { 0 } else { x * 7 / width };
            let color = match band {
                0 => Rgba::new(1.0, 1.0, 1.0, 1.0),
                1 => Rgba::new(1.0, 1.0, 0.0, 1.0),
                2 => Rgba::new(0.0, 1.0, 1.0, 1.0),
                3 => Rgba::new(0.0, 1.0, 0.0, 1.0),
                4 => Rgba::new(1.0, 0.0, 1.0, 1.0),
                5 => Rgba::new(1.0, 0.0, 0.0, 1.0),
                _ => Rgba::new(0.0, 0.0, 1.0, 1.0),
            };
            let _ = y;
            pixels.push(color);
        }
    }
    pixels
}

pub fn classic_is_default(composition: &Composition) -> bool {
    composition.renderer == Renderer::Classic3d
}

pub fn enable_3d(layer: &mut Layer, label: Option<u8>) {
    layer.three_d = true;
    if layer.transform.position[2] == 0.0 {
        layer.transform.position[2] = 0.0;
    }
    if layer.label.is_none() {
        layer.label = label;
    }
}

pub fn new_other_3d(project: &mut Project, comp: ItemId) -> Option<LayerId> {
    let layer_id = add_null(project, comp)?;
    let label = project.labels.defaults.other_3d;
    if let Some(layer) = project.composition_mut(comp)?.layer_mut(layer_id) {
        enable_3d(layer, label);
    }
    Some(layer_id)
}

pub fn new_folder(project: &mut Project, name: &str) -> ItemId {
    let id = project.alloc_item();
    let mut folder = model::folder::Folder::new(id, name);
    folder.label = project.labels.defaults.folder;
    project.add_folder(folder);
    id
}

pub fn marker_interval(a: &TimedMarker, b: &TimedMarker) -> Time {
    Time::from_seconds((b.time.seconds() - a.time.seconds()).abs())
}

pub fn drag_marker(marker: &mut TimedMarker, time: Time, locked: bool) -> bool {
    if locked {
        return false;
    }
    marker.time = time;
    true
}
