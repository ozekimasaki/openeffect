use model::{LayerKind, Property, PropertyBody, PropertyValue, Rgba, TemporalInterpolation};

use crate::ast::BinOp;
use crate::eval::Ctx;
use crate::ops::{
    self, add, cross, degrees_to_radians, div, dot, frames_number, hex_to_rgb, hsl_to_rgb, length,
    look_at, mul, noise1, noise2, noise3, normalize, radians_to_degrees, rgb_to_hsl, sub,
    unit_random, KeySample, LoopMode, Xform,
};
use crate::scene::{path_from_shape, prop_ref, Image, Rect};
use crate::value::{array_of, numbers_of, PathData, PropRef, Value};

const FORBIDDEN: &[&str] = &[
    "source",
    "effect",
    "mask",
    "width",
    "height",
    "anchorPoint",
    "scale",
    "opacity",
    "audioLevels",
    "timeRemap",
    "lightTransmission",
    "castsShadows",
    "acceptsShadows",
    "acceptsLights",
    "ambient",
    "diffuse",
    "specularIntensity",
    "specularShininess",
    "metal",
];

pub fn needs_prefix(name: &str) -> bool {
    matches!(
        name,
        "comp"
            | "footage"
            | "thisProject"
            | "time"
            | "colorDepth"
            | "posterizeTime"
            | "timeToFrames"
            | "framesToTime"
            | "timeToTimecode"
            | "timeToNTSCTimecode"
            | "timeToFeetAndFrames"
            | "timeToCurrentFormat"
            | "add"
            | "sub"
            | "mul"
            | "div"
            | "clamp"
            | "dot"
            | "cross"
            | "normalize"
            | "length"
            | "lookAt"
            | "seedRandom"
            | "random"
            | "gaussRandom"
            | "noise"
            | "linear"
            | "ease"
            | "easeIn"
            | "easeOut"
            | "rgbToHsl"
            | "hslToRgb"
            | "hexToRgb"
            | "degreesToRadians"
            | "radiansToDegrees"
    )
}

pub fn need_num(value: &Value) -> Result<f64, String> {
    value.number().ok_or_else(|| "number expected".into())
}

pub fn unwrap_property(ctx: &Ctx<'_>, value: Value) -> Result<Value, String> {
    match value {
        Value::Property(prop) => sample_prop(ctx, &prop, ctx.now()),
        other => Ok(other),
    }
}

pub fn ident(ctx: &Ctx<'_>, name: &str) -> Result<Value, String> {
    if let Some(value) = special(ctx, name)? {
        return Ok(value);
    }
    match try_layer_name(ctx, ctx.target.comp, ctx.target.layer, name)? {
        Some(value) => Ok(value),
        None => Err(format!("unknown name {name}")),
    }
}

pub fn member(ctx: &Ctx<'_>, object: Value, name: &str) -> Result<Value, String> {
    match object {
        Value::Comp(comp) => member_comp(ctx, comp, name),
        Value::Layer { comp, layer } => match try_layer_name(ctx, comp, layer, name)? {
            Some(value) => Ok(value),
            None => Err(format!("unknown layer member {name}")),
        },
        Value::Footage(id) => member_footage(ctx, id, name),
        Value::Property(prop) => member_property(ctx, &prop, name),
        Value::Group(prop) => member_group(ctx, &prop, name),
        Value::Project => member_project(ctx, name),
        Value::Dollar => member_dollar(name),
        Value::MarkerProp { comp, layer } => member_marker_prop(ctx, comp, layer, name),
        Value::MarkerKey { comp, layer, index } => member_marker_key(ctx, comp, layer, index, name),
        Value::KeyRef { prop, index } => member_key(ctx, &prop, index, name),
        Value::Rect {
            top,
            left,
            width,
            height,
        } => match name {
            "top" => Ok(Value::Number(top)),
            "left" => Ok(Value::Number(left)),
            "width" => Ok(Value::Number(width)),
            "height" => Ok(Value::Number(height)),
            _ => Err(format!("unknown rect member {name}")),
        },
        Value::Content { comp, layer, names } => member_content(ctx, comp, layer, &names, name),
        Value::TextObject { comp, layer } => {
            if name == "sourceText" {
                Ok(Value::SourceText { comp, layer })
            } else {
                Err(format!("unknown text member {name}"))
            }
        }
        Value::SourceText { comp, layer } => {
            if name == "value" {
                let text = ctx
                    .scene
                    .layer(comp, layer)
                    .and_then(|layer| layer.text.as_ref())
                    .map(|text| text.source.clone())
                    .unwrap_or_default();
                Ok(Value::String(text))
            } else {
                Err(format!("unknown sourceText member {name}"))
            }
        }
        Value::Path(_) => Err(format!("path member {name} is a call")),
        Value::MaskRef { comp, layer, index } => {
            let layer = ctx
                .scene
                .layer(comp, layer)
                .ok_or_else(|| "missing layer".to_string())?;
            let mask = layer
                .masks
                .get(index)
                .ok_or_else(|| "missing mask".to_string())?;
            match name {
                "path" | "maskPath" => Ok(Value::Path(path_from_shape(&mask.path))),
                "name" => Ok(Value::String(mask.name.clone())),
                _ => Err(format!("unknown mask member {name}")),
            }
        }
        other => Err(format!("no member {name} on {other:?}")),
    }
}

pub fn index(ctx: &Ctx<'_>, object: Value, index: Value) -> Result<Value, String> {
    if matches!(object, Value::SourceText { .. }) {
        return Err("text.sourceText[i] is not evaluated".into());
    }
    let object = unwrap_property(ctx, object)?;
    match object {
        Value::Array(items) => {
            let slot = need_num(&index)? as isize;
            items
                .get(slot as usize)
                .cloned()
                .ok_or_else(|| "index out of range".into())
        }
        Value::String(text) => {
            let slot = need_num(&index)? as usize;
            text.chars()
                .nth(slot)
                .map(|ch| Value::String(ch.to_string()))
                .ok_or_else(|| "index out of range".into())
        }
        Value::Map(map) => {
            let key = index.text().unwrap_or("").to_string();
            Ok(map.get(&key).cloned().unwrap_or(Value::Null))
        }
        _ => Err("value is not indexable".into()),
    }
}

pub fn call_global(ctx: &mut Ctx<'_>, name: &str, args: &[Value]) -> Result<Value, String> {
    if matches!(name, "effect" | "mask")
        && layer_forbidden(ctx, ctx.target.comp, ctx.target.layer, name)
    {
        return Err(format!("{name} is not available"));
    }
    dispatch(ctx, name, args, None)
}

pub fn call_method(
    ctx: &mut Ctx<'_>,
    object: Value,
    name: &str,
    args: &[Value],
) -> Result<Value, String> {
    if let Value::Layer { comp, layer } = &object {
        if layer_forbidden(ctx, *comp, *layer, name) {
            return Err(format!("{name} is not available"));
        }
    }
    dispatch(ctx, name, args, Some(object))
}

pub fn call_value(ctx: &mut Ctx<'_>, callee: Value, args: &[Value]) -> Result<Value, String> {
    match callee {
        Value::This => Err("this is not a function".into()),
        Value::Layer { comp, layer } => call_layer_index(ctx, comp, layer, args),
        Value::Group(prop) | Value::Property(prop) => call_group_index(ctx, &prop, args),
        _ => Err("not a function".into()),
    }
}

pub fn binary(op: BinOp, left: &Value, right: &Value) -> Result<Value, String> {
    match op {
        BinOp::Add => add_values(left, right),
        BinOp::Sub => sub_values(left, right),
        BinOp::Mul => mul_values(left, right),
        BinOp::Div => div_values(left, right),
        BinOp::Rem => Ok(Value::Number(need_num(left)? % need_num(right)?)),
        BinOp::Eq | BinOp::EqStrict => Ok(Value::Bool(values_equal(left, right))),
        BinOp::NotEq | BinOp::NotEqStrict => Ok(Value::Bool(!values_equal(left, right))),
        BinOp::Lt => Ok(Value::Bool(need_num(left)? < need_num(right)?)),
        BinOp::Gt => Ok(Value::Bool(need_num(left)? > need_num(right)?)),
        BinOp::Le => Ok(Value::Bool(need_num(left)? <= need_num(right)?)),
        BinOp::Ge => Ok(Value::Bool(need_num(left)? >= need_num(right)?)),
        BinOp::And | BinOp::Or => Err("logical op handled earlier".into()),
    }
}

fn special(ctx: &Ctx<'_>, name: &str) -> Result<Option<Value>, String> {
    let value = match name {
        "time" => Value::Number(ctx.now()),
        "value" => sample_prop(ctx, &target_prop(ctx), ctx.now())?,
        "thisComp" => Value::Comp(ctx.target.comp),
        "thisLayer" => Value::Layer {
            comp: ctx.target.comp,
            layer: ctx.target.layer,
        },
        "thisProperty" => Value::Property(target_prop(ctx)),
        "thisProject" => Value::Project,
        "colorDepth" => Value::Number(f64::from(ctx.scene.project.bits_per_channel.bits())),
        "propertyIndex" => {
            return Ok(Some(Value::Number(
                property_index(ctx, &target_prop(ctx)) as f64
            )))
        }
        "numKeys" => {
            let prop = target_prop(ctx);
            let count = if marker_property(ctx, &prop) {
                markers(ctx, prop.comp, Some(prop.layer)).len()
            } else {
                ctx.scene
                    .property(prop.comp, prop.layer, prop.id)
                    .map(property_keys)
                    .unwrap_or_default()
                    .len()
            };
            return Ok(Some(Value::Number(count as f64)));
        }
        "velocity" => return velocity_at(ctx, &target_prop(ctx), ctx.now()).map(Some),
        "speed" => {
            return Ok(Some(Value::Number(speed_of(&velocity_at(
                ctx,
                &target_prop(ctx),
                ctx.now(),
            )?))))
        }
        "$" => Value::Dollar,
        "this" => Value::This,
        "duration" | "numLayers" | "frameDuration" | "shutterAngle" | "shutterPhase"
        | "pixelAspect" | "ntscDropFrame" | "displayStartTime" | "bgColor" | "activeCamera" => {
            return member_comp(ctx, ctx.target.comp, name).map(Some);
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn target_prop(ctx: &Ctx<'_>) -> PropRef {
    prop_ref(ctx.target.comp, ctx.target.layer, ctx.target.property)
}

fn layer_forbidden(ctx: &Ctx<'_>, comp: model::ItemId, layer: model::LayerId, name: &str) -> bool {
    let Some(layer) = ctx.scene.layer(comp, layer) else {
        return false;
    };
    let blocked = matches!(
        layer.kind,
        LayerKind::Camera { .. } | LayerKind::Light { .. }
    );
    blocked && FORBIDDEN.contains(&name)
}

fn try_layer_name(
    ctx: &Ctx<'_>,
    comp: model::ItemId,
    layer: model::LayerId,
    name: &str,
) -> Result<Option<Value>, String> {
    if layer_forbidden(ctx, comp, layer, name) {
        return Err(format!("{name} is not available"));
    }
    if let Some(value) = layer_intrinsic(ctx, comp, layer, name)? {
        return Ok(Some(value));
    }
    if let Some(prop) = find_named(ctx, comp, layer, name) {
        return Ok(Some(classify(ctx, &prop)));
    }
    Ok(None)
}

fn layer_intrinsic(
    ctx: &Ctx<'_>,
    comp: model::ItemId,
    layer_id: model::LayerId,
    name: &str,
) -> Result<Option<Value>, String> {
    let Some(layer) = ctx.scene.layer(comp, layer_id) else {
        return Ok(None);
    };
    let attr = ctx.scene.layer_attr(layer_id);
    let value = match name {
        "name" => Value::String(layer.name.clone()),
        "index" => Value::Number(layer_index(ctx, comp, layer_id).unwrap_or(0.0)),
        "hasParent" => Value::Bool(layer.parent.is_some()),
        "parent" => {
            let Some(parent) = layer.parent else {
                return Err("layer has no parent".into());
            };
            Value::Layer {
                comp,
                layer: parent,
            }
        }
        "inPoint" => Value::Number(attr.map(|attr| attr.in_point).unwrap_or(0.0)),
        "outPoint" => Value::Number(attr.map(|attr| attr.out_point).unwrap_or(0.0)),
        "startTime" => Value::Number(attr.map(|attr| attr.start_time).unwrap_or(0.0)),
        "hasVideo" => Value::Bool(attr.map(|attr| attr.has_video).unwrap_or(true)),
        "hasAudio" => Value::Bool(attr.map(|attr| attr.has_audio).unwrap_or(false)),
        "enabled" => Value::Bool(attr.map(|attr| attr.enabled).unwrap_or(true)),
        "audioActive" => {
            let on = attr
                .map(|attr| {
                    attr.audio_enabled && ctx.now() >= attr.in_point && ctx.now() < attr.out_point
                })
                .unwrap_or(false);
            Value::Bool(on)
        }
        "active" => {
            if matches!(layer.kind, LayerKind::Camera { .. }) {
                Value::Bool(active_camera(ctx, comp, ctx.now()) == Some(layer_id))
            } else {
                let on = attr
                    .map(|attr| {
                        attr.enabled && ctx.now() >= attr.in_point && ctx.now() < attr.out_point
                    })
                    .unwrap_or(false);
                Value::Bool(on)
            }
        }
        "width" => Value::Number(attr.map(|attr| attr.width).unwrap_or(0.0)),
        "height" => Value::Number(attr.map(|attr| attr.height).unwrap_or(0.0)),
        "source" => layer_source(ctx, comp, layer_id)?,
        "marker" => Value::MarkerProp {
            comp,
            layer: Some(layer_id),
        },
        "text" if layer.text.is_some() => Value::TextObject {
            comp,
            layer: layer_id,
        },
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn layer_source(
    ctx: &Ctx<'_>,
    comp: model::ItemId,
    layer: model::LayerId,
) -> Result<Value, String> {
    let kind = ctx
        .scene
        .layer(comp, layer)
        .map(|layer| layer.kind)
        .ok_or_else(|| "missing layer".to_string())?;
    match kind {
        LayerKind::Footage { source }
        | LayerKind::Solid { source }
        | LayerKind::Placeholder { source } => Ok(Value::Footage(source)),
        LayerKind::Precomp { source } => Ok(Value::Comp(source)),
        LayerKind::Camera { .. } | LayerKind::Light { .. } => Err("no source".into()),
        LayerKind::Null
        | LayerKind::Adjustment
        | LayerKind::Text
        | LayerKind::Shape
        | LayerKind::Guide => Err("no source".into()),
    }
}

fn member_comp(ctx: &Ctx<'_>, comp: model::ItemId, name: &str) -> Result<Value, String> {
    let composition = ctx
        .scene
        .project
        .composition(comp)
        .ok_or_else(|| "missing comp".to_string())?;
    let attr = ctx.scene.comp_attr(comp);
    match name {
        "name" => Ok(Value::String(composition.name.clone())),
        "width" => Ok(Value::Number(f64::from(composition.width))),
        "height" => Ok(Value::Number(f64::from(composition.height))),
        "duration" => Ok(Value::Number(composition.duration.seconds())),
        "numLayers" => Ok(Value::Number(composition.layers.len() as f64)),
        "frameDuration" => {
            let fps = composition.frame_rate;
            Ok(Value::Number(if fps == 0.0 { 0.0 } else { 1.0 / fps }))
        }
        "shutterAngle" => Ok(Value::Number(composition.motion_blur.shutter_angle_degrees)),
        "shutterPhase" => Ok(Value::Number(composition.motion_blur.shutter_phase_degrees)),
        "pixelAspect" => Ok(Value::Number(composition.pixel_aspect)),
        "ntscDropFrame" => Ok(Value::Bool(
            attr.map(|attr| attr.ntsc_drop_frame).unwrap_or(false),
        )),
        "displayStartTime" => Ok(Value::Number(
            attr.map(|attr| attr.display_start_time).unwrap_or(0.0),
        )),
        "bgColor" => Ok(array_of(
            attr.map(|attr| attr.bg_color)
                .unwrap_or([1.0, 1.0, 1.0, 1.0]),
        )),
        "marker" => Ok(Value::MarkerProp { comp, layer: None }),
        "activeCamera" => {
            let camera = active_camera(ctx, comp, ctx.now())
                .ok_or_else(|| "no active camera".to_string())?;
            Ok(Value::Layer {
                comp,
                layer: camera,
            })
        }
        _ => Err(format!("unknown comp member {name}")),
    }
}

fn member_footage(ctx: &Ctx<'_>, id: model::ItemId, name: &str) -> Result<Value, String> {
    let footage = ctx
        .scene
        .project
        .footage(id)
        .ok_or_else(|| "missing footage".to_string())?;
    match name {
        "name" => Ok(Value::String(footage.name.clone())),
        "width" => Ok(Value::Number(f64::from(footage.width))),
        "height" => Ok(Value::Number(f64::from(footage.height))),
        _ => Err(format!("unknown footage member {name}")),
    }
}

fn member_project(ctx: &Ctx<'_>, name: &str) -> Result<Value, String> {
    match name {
        "fullPath" => Ok(Value::String(ctx.scene.full_path.clone())),
        "bitsPerChannel" => Ok(Value::Number(f64::from(
            ctx.scene.project.bits_per_channel.bits(),
        ))),
        "linearBlending" => Ok(Value::Bool(ctx.scene.linear_blending)),
        _ => Err(format!("unknown project member {name}")),
    }
}

fn member_dollar(name: &str) -> Result<Value, String> {
    match name {
        "build" => Ok(Value::String(String::new())),
        "engineName" => Ok(Value::String(String::new())),
        "global" => Ok(Value::GlobalObject),
        _ => Err(format!("$.{name} is not supported")),
    }
}

fn member_property(ctx: &Ctx<'_>, prop: &PropRef, name: &str) -> Result<Value, String> {
    let property = ctx
        .scene
        .property(prop.comp, prop.layer, prop.id)
        .cloned()
        .ok_or_else(|| "missing property".to_string())?;
    match name {
        "value" => sample_cloned(ctx, prop, &property, ctx.now()),
        "name" => {
            if let PropertyBody::Value {
                value: PropertyValue::Layer(Some(id)),
                ..
            } = &property.body
            {
                let comp = ctx.scene.comp_of_layer(*id).unwrap_or(prop.comp);
                let layer_name = ctx
                    .scene
                    .layer(comp, *id)
                    .map(|layer| layer.name.clone())
                    .unwrap_or_default();
                return Ok(Value::String(layer_name));
            }
            Ok(Value::String(property.name.clone()))
        }
        "propertyIndex" => Ok(Value::Number(property_index(ctx, prop) as f64)),
        "active" => Ok(Value::Bool(property_active(ctx, prop, &property))),
        "enabled" => Ok(Value::Bool(ctx.scene.prop_extra(prop.id).enabled)),
        "numKeys" => Ok(Value::Number(property_keys(&property).len() as f64)),
        "numProperties" => Ok(Value::Number(0.0)),
        "text" => Ok(Value::String(dropdown_label(
            ctx,
            prop,
            &property,
            ctx.now(),
        )?)),
        "items" => Ok(Value::Array(
            ctx.scene
                .prop_extra(prop.id)
                .dropdown_items
                .into_iter()
                .map(Value::String)
                .collect(),
        )),
        _ => Err(format!("unknown property member {name}")),
    }
}

fn member_group(ctx: &Ctx<'_>, prop: &PropRef, name: &str) -> Result<Value, String> {
    let property = ctx
        .scene
        .property(prop.comp, prop.layer, prop.id)
        .cloned()
        .ok_or_else(|| "missing group".to_string())?;
    match name {
        "name" => Ok(Value::String(property.name.clone())),
        "numProperties" => {
            let count = match &property.body {
                PropertyBody::Group { children } => children.len(),
                PropertyBody::Value { .. } => 0,
            };
            Ok(Value::Number(count as f64))
        }
        "enabled" => Ok(Value::Bool(ctx.scene.prop_extra(prop.id).enabled)),
        "propertyIndex" => Ok(Value::Number(property_index(ctx, prop) as f64)),
        "active" => Ok(Value::Bool(property_active(ctx, prop, &property))),
        other => child_named(ctx, prop, other)
            .map(|child| classify(ctx, &child))
            .ok_or_else(|| format!("unknown group member {other}")),
    }
}

fn member_content(
    ctx: &Ctx<'_>,
    comp: model::ItemId,
    layer: model::LayerId,
    names: &[String],
    name: &str,
) -> Result<Value, String> {
    if name != "path" {
        return Err(format!("unknown content member {name}"));
    }
    let layer = ctx
        .scene
        .layer(comp, layer)
        .ok_or_else(|| "missing layer".to_string())?;
    let root = layer.shape.as_ref().ok_or_else(|| "no shape".to_string())?;
    let node = find_shape(&root.nodes, names).ok_or_else(|| "content not found".to_string())?;
    for property in &node.properties {
        if let PropertyBody::Value {
            value: PropertyValue::Path(path),
            ..
        } = &property.body
        {
            return Ok(Value::Path(path_from_shape(path.as_ref())));
        }
    }
    Err("path not found".into())
}

fn find_shape<'a>(
    nodes: &'a [model::shape::ShapeNode],
    names: &[String],
) -> Option<&'a model::shape::ShapeNode> {
    let (first, rest) = names.split_first()?;
    let node = nodes.iter().find(|node| node.name == *first)?;
    if rest.is_empty() {
        Some(node)
    } else {
        find_shape(&node.children, rest)
    }
}

fn dispatch(
    ctx: &mut Ctx<'_>,
    name: &str,
    args: &[Value],
    recv: Option<Value>,
) -> Result<Value, String> {
    match name {
        "add" => map_vec2(args, add),
        "sub" => map_vec2(args, sub),
        "mul" => Ok(array_of(mul(&need_vec(&args[0])?, need_num(&args[1])?))),
        "div" => Ok(array_of(div(&need_vec(&args[0])?, need_num(&args[1])?))),
        "dot" => Ok(Value::Number(dot(
            &need_vec(&args[0])?,
            &need_vec(&args[1])?,
        ))),
        "cross" => Ok(array_of(cross(&need_vec(&args[0])?, &need_vec(&args[1])?))),
        "normalize" => Ok(array_of(normalize(&need_vec(&args[0])?))),
        "length" => {
            if args.len() >= 2 {
                Ok(Value::Number(length(&sub(
                    &need_vec(&args[0])?,
                    &need_vec(&args[1])?,
                ))))
            } else {
                Ok(Value::Number(length(&need_vec(&args[0])?)))
            }
        }
        "clamp" => clamp_call(args),
        "lookAt" => Ok(array_of(look_at(
            as3(&need_vec(&args[0])?),
            as3(&need_vec(&args[1])?),
        ))),
        "degreesToRadians" => Ok(Value::Number(degrees_to_radians(need_num(&args[0])?))),
        "radiansToDegrees" => Ok(Value::Number(radians_to_degrees(need_num(&args[0])?))),
        "rgbToHsl" => Ok(array_of(rgb_to_hsl(as4(&need_vec(&args[0])?)))),
        "hslToRgb" => Ok(array_of(hsl_to_rgb(as4(&need_vec(&args[0])?)))),
        "hexToRgb" => {
            let text = args.first().and_then(Value::text).unwrap_or("");
            Ok(array_of(
                hex_to_rgb(text).ok_or_else(|| "bad hex".to_string())?,
            ))
        }
        "seedRandom" => {
            ctx.seed_offset = need_num(&args[0])?;
            ctx.timeless = args.get(1).and_then(Value::bool).unwrap_or(false);
            Ok(Value::Null)
        }
        "random" => random_call(ctx, args),
        "gaussRandom" => gauss_call(ctx, args),
        "noise" => noise_call(args),
        "linear" => ease_call(args, |u| u),
        "ease" => ease_call(args, ops::smoothstep),
        "easeIn" => ease_call(args, ops::ease_in_curve),
        "easeOut" => ease_call(args, ops::ease_out_curve),
        "posterizeTime" => {
            let fps = need_num(&args[0])?;
            ctx.poster_fps = Some(fps);
            Ok(Value::Number(fps))
        }
        "timeToFrames" => Ok(Value::Number(frames_number(
            arg_num(ctx, args, 0, default_t(ctx))?,
            arg_num(ctx, args, 1, default_fps(ctx))?,
            arg_bool(args, 2, false)?,
        ))),
        "framesToTime" => {
            let frames = need_num(&args[0])?;
            let fps = arg_num(ctx, args, 1, default_fps(ctx))?;
            Ok(Value::Number(if fps == 0.0 { 0.0 } else { frames / fps }))
        }
        "timeToTimecode" => Ok(Value::String(ops::format_timecode(
            arg_num(ctx, args, 0, default_t(ctx))?,
            arg_num(ctx, args, 1, 30.0)?,
            false,
            arg_bool(args, 2, false)?,
        ))),
        "timeToNTSCTimecode" => Ok(Value::String(ops::format_timecode(
            arg_num(ctx, args, 0, default_t(ctx))?,
            30.0,
            arg_bool(args, 1, false)?,
            arg_bool(args, 2, false)?,
        ))),
        "timeToFeetAndFrames" => Ok(Value::String(ops::format_feet(
            arg_num(ctx, args, 0, default_t(ctx))?,
            arg_num(ctx, args, 1, default_fps(ctx))?,
            arg_num(ctx, args, 2, 16.0)?,
            arg_bool(args, 3, false)?,
        ))),
        "timeToCurrentFormat" => current_format(ctx, args),
        "comp" => comp_named(ctx, args.first().and_then(Value::text).unwrap_or("")),
        "footage" => footage_named(ctx, args.first().and_then(Value::text).unwrap_or("")),
        "valueAtTime" => {
            let prop = receiver_prop(ctx, recv.as_ref())?;
            sample_prop(ctx, &prop, need_num(&args[0])?)
        }
        "velocity" => velocity_at(ctx, &receiver_prop(ctx, recv.as_ref())?, ctx.now()),
        "velocityAtTime" => velocity_at(
            ctx,
            &receiver_prop(ctx, recv.as_ref())?,
            need_num(&args[0])?,
        ),
        "speed" => Ok(Value::Number(speed_of(&velocity_at(
            ctx,
            &receiver_prop(ctx, recv.as_ref())?,
            ctx.now(),
        )?))),
        "speedAtTime" => Ok(Value::Number(speed_of(&velocity_at(
            ctx,
            &receiver_prop(ctx, recv.as_ref())?,
            need_num(&args[0])?,
        )?))),
        "wiggle" => wiggle(ctx, recv.as_ref(), args, false),
        "temporalWiggle" => wiggle(ctx, recv.as_ref(), args, true),
        "smooth" => smooth(ctx, recv.as_ref(), args),
        "loopIn" => loop_call(ctx, recv.as_ref(), args, false, false),
        "loopOut" => loop_call(ctx, recv.as_ref(), args, true, false),
        "loopInDuration" => loop_call(ctx, recv.as_ref(), args, false, true),
        "loopOutDuration" => loop_call(ctx, recv.as_ref(), args, true, true),
        "key" => key_call(ctx, recv.as_ref(), args),
        "nearestKey" => nearest_key_call(ctx, recv.as_ref(), args),
        "nextKey" => neighbor_key(ctx, recv.as_ref(), need_num(&args[0])?, true),
        "previousKey" => neighbor_key(ctx, recv.as_ref(), need_num(&args[0])?, false),
        "propertyGroup" => property_group(ctx, recv.as_ref(), args),
        "textAtTime" => {
            let prop = receiver_prop(ctx, recv.as_ref())?;
            let property = ctx
                .scene
                .property(prop.comp, prop.layer, prop.id)
                .cloned()
                .ok_or_else(|| "missing property".to_string())?;
            Ok(Value::String(dropdown_label(
                ctx,
                &prop,
                &property,
                need_num(&args[0])?,
            )?))
        }
        "effect" => effect_call(ctx, recv.as_ref(), args),
        "mask" => mask_call(ctx, recv.as_ref(), args),
        "param" => param_call(ctx, recv.as_ref(), args),
        "sourceTime" => {
            let (comp, layer) = receiver_layer(ctx, recv.as_ref());
            let t = arg_num(ctx, args, 0, ctx.now())?;
            Ok(Value::Number(source_time(ctx, comp, layer, t)))
        }
        "sourceRectAtTime" => source_rect(ctx, recv.as_ref(), args),
        "sampleImage" => sample_image_call(ctx, recv.as_ref(), args),
        "toComp" => space_point(ctx, recv.as_ref(), args, Space::ToComp),
        "fromComp" => space_point(ctx, recv.as_ref(), args, Space::FromComp),
        "toWorld" => space_point(ctx, recv.as_ref(), args, Space::ToWorld),
        "fromWorld" => space_point(ctx, recv.as_ref(), args, Space::FromWorld),
        "toCompVec" => space_vec(ctx, recv.as_ref(), args, Space::ToComp),
        "fromCompVec" => space_vec(ctx, recv.as_ref(), args, Space::FromComp),
        "toWorldVec" => space_vec(ctx, recv.as_ref(), args, Space::ToWorld),
        "fromWorldVec" => space_vec(ctx, recv.as_ref(), args, Space::FromWorld),
        "fromCompToSurface" => {
            let point = space_point(ctx, recv.as_ref(), args, Space::FromComp)?;
            let coords = numbers_of(&point).unwrap_or_default();
            Ok(array_of(coords.into_iter().take(2)))
        }
        "content" => {
            let fallback = Value::Layer {
                comp: ctx.target.comp,
                layer: ctx.target.layer,
            };
            content_call(recv.as_ref().or(Some(&fallback)), args)
        }
        "layer" => {
            let Value::Comp(comp) = recv.unwrap_or(Value::Comp(ctx.target.comp)) else {
                return Err("layer() needs a comp".into());
            };
            comp_layer(ctx, comp, args)
        }
        "layerByComment" => {
            let Value::Comp(comp) = recv.unwrap_or(Value::Comp(ctx.target.comp)) else {
                return Err("layerByComment needs a comp".into());
            };
            let comment = args.first().and_then(Value::text).unwrap_or("");
            let composition = ctx
                .scene
                .project
                .composition(comp)
                .ok_or_else(|| "missing comp".to_string())?;
            let found = composition.layers.iter().find(|layer| {
                ctx.scene
                    .layer_attr(layer.id)
                    .map(|attr| attr.comment == comment)
                    .unwrap_or(false)
            });
            let layer = found.ok_or_else(|| "comment not found".to_string())?;
            Ok(Value::Layer {
                comp,
                layer: layer.id,
            })
        }
        "points" | "inTangents" | "outTangents" | "isClosed" | "pointOnPath" | "tangentOnPath"
        | "normalOnPath" | "createPath" => path_call(name, recv.as_ref(), args),
        "eval" => {
            let source = args.first().and_then(Value::text).unwrap_or("");
            ctx.eval_nested(source)
        }
        "numKeys" => {
            if let Some(Value::MarkerProp { comp, layer }) = recv {
                return Ok(Value::Number(markers(ctx, comp, layer).len() as f64));
            }
            let prop = receiver_prop(ctx, recv.as_ref())?;
            let property = ctx
                .scene
                .property(prop.comp, prop.layer, prop.id)
                .ok_or_else(|| "missing property".to_string())?;
            Ok(Value::Number(property_keys(property).len() as f64))
        }
        _ => Err(format!("unknown function {name}")),
    }
}

fn map_vec2(args: &[Value], op: fn(&[f64], &[f64]) -> Vec<f64>) -> Result<Value, String> {
    Ok(array_of(op(&need_vec(&args[0])?, &need_vec(&args[1])?)))
}

fn need_vec(value: &Value) -> Result<Vec<f64>, String> {
    if let Some(number) = value.number() {
        return Ok(vec![number]);
    }
    numbers_of(value).ok_or_else(|| "array expected".into())
}

fn as3(values: &[f64]) -> [f64; 3] {
    [
        values.first().copied().unwrap_or(0.0),
        values.get(1).copied().unwrap_or(0.0),
        values.get(2).copied().unwrap_or(0.0),
    ]
}

fn as4(values: &[f64]) -> [f64; 4] {
    [
        as3(values)[0],
        as3(values)[1],
        as3(values)[2],
        values.get(3).copied().unwrap_or(1.0),
    ]
}

fn clamp_call(args: &[Value]) -> Result<Value, String> {
    if args[0].number().is_some() && args[1].number().is_some() && args[2].number().is_some() {
        return Ok(Value::Number(ops::clamp_num(
            need_num(&args[0])?,
            need_num(&args[1])?,
            need_num(&args[2])?,
        )));
    }
    let value = need_vec(&args[0])?;
    let a = need_vec(&args[1])?;
    let b = need_vec(&args[2])?;
    let n = value.len().max(a.len()).max(b.len());
    let value = ops::pad(value, n);
    let a = ops::pad(a, n);
    let b = ops::pad(b, n);
    Ok(array_of(
        value
            .into_iter()
            .zip(a)
            .zip(b)
            .map(|((v, a), b)| ops::clamp_num(v, a, b)),
    ))
}

fn random_call(ctx: &mut Ctx<'_>, args: &[Value]) -> Result<Value, String> {
    match args.len() {
        0 => Ok(Value::Number(unit(ctx))),
        1 => scale_random(ctx, &Value::Number(0.0), &args[0]),
        _ => scale_random(ctx, &args[0], &args[1]),
    }
}

fn scale_random(ctx: &mut Ctx<'_>, min: &Value, max: &Value) -> Result<Value, String> {
    if min.number().is_some() && max.number().is_some() {
        let min = need_num(min)?;
        let max = need_num(max)?;
        return Ok(Value::Number(min + unit(ctx) * (max - min)));
    }
    let min = need_vec(min)?;
    let max = need_vec(max)?;
    let n = min.len().max(max.len());
    let min = ops::pad(min, n);
    let max = ops::pad(max, n);
    let mut out = Vec::new();
    for (min, max) in min.into_iter().zip(max) {
        out.push(min + unit(ctx) * (max - min));
    }
    Ok(array_of(out))
}

fn gauss_call(ctx: &mut Ctx<'_>, args: &[Value]) -> Result<Value, String> {
    match args.len() {
        0 => Ok(Value::Number(gauss_unit(ctx))),
        1 => scale_gauss(ctx, &Value::Number(0.0), &args[0]),
        _ => scale_gauss(ctx, &args[0], &args[1]),
    }
}

fn scale_gauss(ctx: &mut Ctx<'_>, min: &Value, max: &Value) -> Result<Value, String> {
    if min.number().is_some() && max.number().is_some() {
        let min = need_num(min)?;
        let max = need_num(max)?;
        return Ok(Value::Number(min + gauss_unit(ctx) * (max - min)));
    }
    let min = need_vec(min)?;
    let max = need_vec(max)?;
    let n = min.len().max(max.len());
    let min = ops::pad(min, n);
    let max = ops::pad(max, n);
    let mut out = Vec::new();
    for (min, max) in min.into_iter().zip(max) {
        out.push(min + gauss_unit(ctx) * (max - min));
    }
    Ok(array_of(out))
}

fn unit(ctx: &mut Ctx<'_>) -> f64 {
    ctx.rng_counter += 1;
    let time = if ctx.timeless { 0 } else { ctx.now().to_bits() };
    unit_random(&[
        ctx.target.layer.get(),
        ctx.target.property.get(),
        time,
        ctx.seed_offset.to_bits(),
        ctx.rng_counter,
    ])
}

fn gauss_unit(ctx: &mut Ctx<'_>) -> f64 {
    let u1 = unit(ctx).clamp(1e-12, 1.0 - 1e-12);
    let u2 = unit(ctx);
    let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
    0.5 + z * (0.5 / 1.6448536269514722)
}

fn noise_call(args: &[Value]) -> Result<Value, String> {
    let value = if args[0].number().is_some() {
        noise1(need_num(&args[0])?)
    } else {
        let coords = need_vec(&args[0])?;
        match coords.len() {
            0 | 1 => noise1(coords.first().copied().unwrap_or(0.0)),
            2 => noise2(coords[0], coords[1]),
            _ => noise3(coords[0], coords[1], coords[2]),
        }
    };
    Ok(Value::Number(value))
}

fn ease_call(args: &[Value], curve: fn(f64) -> f64) -> Result<Value, String> {
    if args.len() >= 5 {
        ops::interp(
            need_num(&args[0])?,
            need_num(&args[1])?,
            need_num(&args[2])?,
            &args[3],
            &args[4],
            curve,
        )
        .ok_or_else(|| "cannot interpolate".into())
    } else {
        ops::interp(need_num(&args[0])?, 0.0, 1.0, &args[1], &args[2], curve)
            .ok_or_else(|| "cannot interpolate".into())
    }
}

fn arg_num(ctx: &Ctx<'_>, args: &[Value], index: usize, default: f64) -> Result<f64, String> {
    let _ = ctx;
    match args.get(index) {
        Some(value) => need_num(value),
        None => Ok(default),
    }
}

fn arg_bool(args: &[Value], index: usize, default: bool) -> Result<bool, String> {
    match args.get(index) {
        Some(Value::Bool(value)) => Ok(*value),
        Some(Value::Number(value)) => Ok(*value != 0.0),
        Some(_) => Err("boolean expected".into()),
        None => Ok(default),
    }
}

fn default_t(ctx: &Ctx<'_>) -> f64 {
    ctx.now()
        + ctx
            .scene
            .comp_attr(ctx.target.comp)
            .map(|attr| attr.display_start_time)
            .unwrap_or(0.0)
}

fn default_fps(ctx: &Ctx<'_>) -> f64 {
    let duration = ctx
        .scene
        .project
        .composition(ctx.target.comp)
        .map(|comp| comp.frame_rate)
        .unwrap_or(0.0);
    if duration == 0.0 {
        0.0
    } else {
        duration
    }
}

fn current_format(ctx: &Ctx<'_>, args: &[Value]) -> Result<Value, String> {
    let t = arg_num(ctx, args, 0, default_t(ctx))?;
    let fps = arg_num(ctx, args, 1, default_fps(ctx))?;
    let is_duration = arg_bool(args, 2, false)?;
    let text = match ctx.scene.time_display {
        crate::scene::TimeDisplay::Timecode => ops::format_timecode(t, 30.0, false, is_duration),
        crate::scene::TimeDisplay::Frames => frames_number(t, fps, is_duration).to_string(),
        crate::scene::TimeDisplay::FeetAndFrames => ops::format_feet(
            t,
            fps,
            arg_num(ctx, args, 3, 16.0).unwrap_or(16.0),
            is_duration,
        ),
        crate::scene::TimeDisplay::Ntsc => {
            let drop = if args.len() > 3 {
                arg_bool(args, 3, false)?
            } else {
                ctx.scene
                    .comp_attr(ctx.target.comp)
                    .map(|attr| attr.ntsc_drop_frame)
                    .unwrap_or(false)
            };
            ops::format_timecode(t, 30.0, drop, is_duration)
        }
    };
    Ok(Value::String(text))
}

fn comp_named(ctx: &Ctx<'_>, name: &str) -> Result<Value, String> {
    ctx.scene
        .project
        .compositions
        .iter()
        .find(|comp| comp.name == name)
        .map(|comp| Value::Comp(comp.id))
        .ok_or_else(|| format!("comp {name} not found"))
}

fn footage_named(ctx: &Ctx<'_>, name: &str) -> Result<Value, String> {
    ctx.scene
        .project
        .footage
        .iter()
        .find(|footage| footage.name == name)
        .map(|footage| Value::Footage(footage.id))
        .ok_or_else(|| format!("footage {name} not found"))
}

fn comp_layer(ctx: &Ctx<'_>, comp: model::ItemId, args: &[Value]) -> Result<Value, String> {
    if args.len() >= 2 {
        let Value::Layer { layer, .. } = &args[0] else {
            return Err("layer expected".into());
        };
        let base = layer_index(ctx, comp, *layer).ok_or_else(|| "layer not in comp".to_string())?;
        let index = base + need_num(&args[1])?;
        return layer_at(ctx, comp, index);
    }
    match &args[0] {
        Value::Number(index) => layer_at(ctx, comp, *index),
        Value::String(name) => {
            let composition = ctx
                .scene
                .project
                .composition(comp)
                .ok_or_else(|| "missing comp".to_string())?;
            composition
                .layers
                .iter()
                .find(|layer| layer.name == *name)
                .map(|layer| Value::Layer {
                    comp,
                    layer: layer.id,
                })
                .ok_or_else(|| format!("layer {name} not found"))
        }
        _ => Err("layer selector expected".into()),
    }
}

fn layer_at(ctx: &Ctx<'_>, comp: model::ItemId, index: f64) -> Result<Value, String> {
    let slot = index.round() as isize;
    if slot < 1 {
        return Err("layer index starts at 1".into());
    }
    let composition = ctx
        .scene
        .project
        .composition(comp)
        .ok_or_else(|| "missing comp".to_string())?;
    composition
        .layers
        .get((slot as usize) - 1)
        .map(|layer| Value::Layer {
            comp,
            layer: layer.id,
        })
        .ok_or_else(|| "layer index out of range".into())
}

fn layer_index(ctx: &Ctx<'_>, comp: model::ItemId, layer: model::LayerId) -> Option<f64> {
    let composition = ctx.scene.project.composition(comp)?;
    let position = composition
        .layers
        .iter()
        .position(|item| item.id == layer)?;
    Some((position + 1) as f64)
}

fn active_camera(ctx: &Ctx<'_>, comp: model::ItemId, time: f64) -> Option<model::LayerId> {
    let composition = ctx.scene.project.composition(comp)?;
    for layer in &composition.layers {
        if !matches!(layer.kind, LayerKind::Camera { .. }) {
            continue;
        }
        let Some(attr) = ctx.scene.layer_attr(layer.id) else {
            continue;
        };
        if attr.enabled && time >= attr.in_point && time < attr.out_point {
            return Some(layer.id);
        }
    }
    None
}

fn receiver_layer(ctx: &Ctx<'_>, recv: Option<&Value>) -> (model::ItemId, model::LayerId) {
    match recv {
        Some(Value::Layer { comp, layer }) => (*comp, *layer),
        _ => (ctx.target.comp, ctx.target.layer),
    }
}

fn receiver_prop(ctx: &Ctx<'_>, recv: Option<&Value>) -> Result<PropRef, String> {
    match recv {
        Some(Value::Property(prop) | Value::Group(prop)) => Ok(prop.clone()),
        None => Ok(target_prop(ctx)),
        _ => Err("property expected".into()),
    }
}

fn effect_call(ctx: &Ctx<'_>, recv: Option<&Value>, args: &[Value]) -> Result<Value, String> {
    let (comp, layer) = receiver_layer(ctx, recv);
    if layer_forbidden(ctx, comp, layer, "effect") {
        return Err("effect is not available".into());
    }
    let group = find_named(ctx, comp, layer, "effects").ok_or_else(|| "no effects".to_string())?;
    call_group_index(ctx, &group, args)
}

fn mask_call(ctx: &Ctx<'_>, recv: Option<&Value>, args: &[Value]) -> Result<Value, String> {
    let (comp, layer_id) = receiver_layer(ctx, recv);
    if layer_forbidden(ctx, comp, layer_id, "mask") {
        return Err("mask is not available".into());
    }
    let layer = ctx
        .scene
        .layer(comp, layer_id)
        .ok_or_else(|| "missing layer".to_string())?;
    let index = match &args[0] {
        Value::Number(index) => {
            let slot = *index as isize;
            if slot < 1 || slot as usize > layer.masks.len() {
                return Err("mask index out of range".into());
            }
            (slot as usize) - 1
        }
        Value::String(name) => layer
            .masks
            .iter()
            .position(|mask| mask.name == *name)
            .ok_or_else(|| format!("mask {name} not found"))?,
        _ => return Err("mask selector expected".into()),
    };
    Ok(Value::MaskRef {
        comp,
        layer: layer_id,
        index,
    })
}

fn param_call(ctx: &Ctx<'_>, recv: Option<&Value>, args: &[Value]) -> Result<Value, String> {
    let Some(Value::Group(prop) | Value::Property(prop)) = recv else {
        return Err("param needs an effect".into());
    };
    let child = call_group_index(ctx, prop, args)?;
    unwrap_property(ctx, child)
}

fn content_call(recv: Option<&Value>, args: &[Value]) -> Result<Value, String> {
    let name = args.first().and_then(Value::text).unwrap_or("").to_string();
    match recv {
        Some(Value::Layer { comp, layer }) => Ok(Value::Content {
            comp: *comp,
            layer: *layer,
            names: vec![name],
        }),
        Some(Value::Content { comp, layer, names }) => {
            let mut names = names.clone();
            names.push(name);
            Ok(Value::Content {
                comp: *comp,
                layer: *layer,
                names,
            })
        }
        _ => Err("content needs a layer".into()),
    }
}

fn call_layer_index(
    ctx: &Ctx<'_>,
    comp: model::ItemId,
    layer: model::LayerId,
    args: &[Value],
) -> Result<Value, String> {
    let layer = ctx
        .scene
        .layer(comp, layer)
        .ok_or_else(|| "missing layer".to_string())?;
    select_child(&layer.properties, comp, layer.id, args)
}

fn call_group_index(ctx: &Ctx<'_>, prop: &PropRef, args: &[Value]) -> Result<Value, String> {
    let property = ctx
        .scene
        .property(prop.comp, prop.layer, prop.id)
        .ok_or_else(|| "missing group".to_string())?;
    let children = match &property.body {
        PropertyBody::Group { children } => children,
        PropertyBody::Value { .. } => return Err("not a group".into()),
    };
    select_child(children, prop.comp, prop.layer, args)
}

fn select_child(
    children: &[Property],
    comp: model::ItemId,
    layer: model::LayerId,
    args: &[Value],
) -> Result<Value, String> {
    let property = match &args[0] {
        Value::Number(index) => {
            let slot = *index as isize;
            if slot < 1 {
                return Err("property index starts at 1".into());
            }
            children.get((slot as usize) - 1)
        }
        Value::String(name) => children
            .iter()
            .find(|child| child.name == *name || child.match_name == *name),
        _ => None,
    };
    let property = property.ok_or_else(|| "property not found".to_string())?;
    Ok(classify_property(comp, layer, property))
}

fn classify(ctx: &Ctx<'_>, prop: &PropRef) -> Value {
    let grouped = ctx
        .scene
        .property(prop.comp, prop.layer, prop.id)
        .map(|property| matches!(property.body, PropertyBody::Group { .. }))
        .unwrap_or(false);
    if grouped {
        Value::Group(prop.clone())
    } else {
        Value::Property(prop.clone())
    }
}

fn classify_property(comp: model::ItemId, layer: model::LayerId, property: &Property) -> Value {
    let prop = prop_ref(comp, layer, property.id);
    if matches!(property.body, PropertyBody::Group { .. }) {
        Value::Group(prop)
    } else {
        Value::Property(prop)
    }
}

fn find_named(
    ctx: &Ctx<'_>,
    comp: model::ItemId,
    layer: model::LayerId,
    name: &str,
) -> Option<PropRef> {
    let layer = ctx.scene.layer(comp, layer)?;
    find_id(&layer.properties, name).map(|id| prop_ref(comp, layer.id, id))
}

fn find_id(properties: &[Property], name: &str) -> Option<model::PropertyId> {
    for property in properties {
        if property.match_name == name || property.name == name {
            return Some(property.id);
        }
        if let PropertyBody::Group { children } = &property.body {
            if let Some(id) = find_id(children, name) {
                return Some(id);
            }
        }
    }
    None
}

fn child_named(ctx: &Ctx<'_>, prop: &PropRef, name: &str) -> Option<PropRef> {
    let property = ctx.scene.property(prop.comp, prop.layer, prop.id)?;
    let PropertyBody::Group { children } = &property.body else {
        return None;
    };
    children
        .iter()
        .find(|child| child.name == name || child.match_name == name)
        .map(|child| prop_ref(prop.comp, prop.layer, child.id))
}

fn sample_prop(ctx: &Ctx<'_>, prop: &PropRef, time: f64) -> Result<Value, String> {
    let property = ctx
        .scene
        .property(prop.comp, prop.layer, prop.id)
        .cloned()
        .ok_or_else(|| "missing property".to_string())?;
    sample_cloned(ctx, prop, &property, time)
}

fn sample_cloned(
    ctx: &Ctx<'_>,
    prop: &PropRef,
    property: &Property,
    time: f64,
) -> Result<Value, String> {
    let sampled = sample_body(property, time);
    Ok(property_to_value(ctx, prop, &sampled))
}

fn sample_body(property: &Property, time: f64) -> PropertyValue {
    let PropertyBody::Value {
        value, keyframes, ..
    } = &property.body
    else {
        return PropertyValue::Empty;
    };
    if keyframes.is_empty() {
        return value.clone();
    }
    let mut keys = keyframes.clone();
    keys.sort_by(|a, b| {
        a.time
            .seconds()
            .partial_cmp(&b.time.seconds())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if time <= keys[0].time.seconds() {
        return keys[0].value.clone();
    }
    if time >= keys[keys.len() - 1].time.seconds() {
        return keys[keys.len() - 1].value.clone();
    }
    let right = keys
        .iter()
        .position(|key| key.time.seconds() >= time)
        .unwrap_or(keys.len() - 1);
    let left = right - 1;
    let t0 = keys[left].time.seconds();
    let t1 = keys[right].time.seconds();
    let u = if (t1 - t0).abs() < 1e-12 {
        0.0
    } else {
        (time - t0) / (t1 - t0)
    };
    match keys[left].temporal_out {
        TemporalInterpolation::Hold => keys[left].value.clone(),
        TemporalInterpolation::Linear
        | TemporalInterpolation::Bezier
        | TemporalInterpolation::ContinuousBezier
        | TemporalInterpolation::AutoBezier => {
            lerp_property(&keys[left].value, &keys[right].value, u)
        }
    }
}

fn lerp_property(left: &PropertyValue, right: &PropertyValue, u: f64) -> PropertyValue {
    match (left, right) {
        (PropertyValue::Scalar(a), PropertyValue::Scalar(b)) => {
            PropertyValue::Scalar(ops::lerp(*a, *b, u))
        }
        (PropertyValue::TwoD(a), PropertyValue::TwoD(b)) => {
            PropertyValue::TwoD([ops::lerp(a[0], b[0], u), ops::lerp(a[1], b[1], u)])
        }
        (PropertyValue::ThreeD(a), PropertyValue::ThreeD(b)) => PropertyValue::ThreeD([
            ops::lerp(a[0], b[0], u),
            ops::lerp(a[1], b[1], u),
            ops::lerp(a[2], b[2], u),
        ]),
        (PropertyValue::Color(a), PropertyValue::Color(b)) => PropertyValue::Color(Rgba::new(
            ops::lerp(f64::from(a.r), f64::from(b.r), u) as f32,
            ops::lerp(f64::from(a.g), f64::from(b.g), u) as f32,
            ops::lerp(f64::from(a.b), f64::from(b.b), u) as f32,
            ops::lerp(f64::from(a.a), f64::from(b.a), u) as f32,
        )),
        _ => left.clone(),
    }
}

fn property_to_value(ctx: &Ctx<'_>, prop: &PropRef, value: &PropertyValue) -> Value {
    match value {
        PropertyValue::Empty => Value::Null,
        PropertyValue::Scalar(number) => Value::Number(*number),
        PropertyValue::TwoD(values) => array_of(*values),
        PropertyValue::ThreeD(values) => array_of(*values),
        PropertyValue::Color(color) => array_of([
            f64::from(color.r),
            f64::from(color.g),
            f64::from(color.b),
            f64::from(color.a),
        ]),
        PropertyValue::Boolean(value) => Value::Bool(*value),
        PropertyValue::Index(value) => Value::Number(f64::from(*value)),
        PropertyValue::Text(value) => Value::String(value.clone()),
        PropertyValue::Path(path) => Value::Path(path_from_shape(path.as_ref())),
        PropertyValue::Marker(_) => Value::Null,
        PropertyValue::Layer(Some(id)) => {
            let comp = ctx.scene.comp_of_layer(*id).unwrap_or(prop.comp);
            Value::Layer { comp, layer: *id }
        }
        PropertyValue::Layer(None) => Value::Null,
    }
}

fn property_keys(property: &Property) -> Vec<(f64, PropertyValue)> {
    let PropertyBody::Value { keyframes, .. } = &property.body else {
        return Vec::new();
    };
    let mut keys: Vec<_> = keyframes
        .iter()
        .map(|key| (key.time.seconds(), key.value.clone()))
        .collect();
    keys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    keys
}

fn property_index(ctx: &Ctx<'_>, prop: &PropRef) -> usize {
    let Some(layer) = ctx.scene.layer(prop.comp, prop.layer) else {
        return 0;
    };
    index_in(&layer.properties, prop.id).unwrap_or(0)
}

fn index_in(properties: &[Property], id: model::PropertyId) -> Option<usize> {
    if let Some(position) = properties.iter().position(|property| property.id == id) {
        return Some(position + 1);
    }
    for property in properties {
        if let PropertyBody::Group { children } = &property.body {
            if contains_id(children, id) {
                return index_in(children, id);
            }
        }
    }
    None
}

fn contains_id(properties: &[Property], id: model::PropertyId) -> bool {
    properties.iter().any(|property| {
        property.id == id
            || match &property.body {
                PropertyBody::Group { children } => contains_id(children, id),
                PropertyBody::Value { .. } => false,
            }
    })
}

fn property_active(ctx: &Ctx<'_>, prop: &PropRef, property: &Property) -> bool {
    let three_d = ctx
        .scene
        .layer(prop.comp, prop.layer)
        .map(|layer| layer.three_d)
        .unwrap_or(false);
    if matches!(property.match_name.as_str(), "rotationX" | "rotationY") && !three_d {
        return false;
    }
    true
}

fn dropdown_label(
    ctx: &Ctx<'_>,
    prop: &PropRef,
    property: &Property,
    time: f64,
) -> Result<String, String> {
    let value = sample_cloned(ctx, prop, property, time)?;
    let index = value.number().unwrap_or(1.0).round() as isize;
    let items = ctx.scene.prop_extra(prop.id).dropdown_items;
    if index < 1 {
        return Ok(String::new());
    }
    Ok(items.get((index as usize) - 1).cloned().unwrap_or_default())
}

fn velocity_at(ctx: &Ctx<'_>, prop: &PropRef, time: f64) -> Result<Value, String> {
    let property = ctx
        .scene
        .property(prop.comp, prop.layer, prop.id)
        .cloned()
        .ok_or_else(|| "missing property".to_string())?;
    let keys = property_keys(&property);
    if keys.len() < 2 || time <= keys[0].0 || time >= keys[keys.len() - 1].0 {
        return zero_like(&sample_cloned(ctx, prop, &property, time)?);
    }
    let right = keys
        .iter()
        .position(|(stamp, _)| *stamp >= time)
        .unwrap_or(keys.len() - 1);
    let left = right - 1;
    let dt = keys[right].0 - keys[left].0;
    let left_v = property_to_value(ctx, prop, &keys[left].1);
    let right_v = property_to_value(ctx, prop, &keys[right].1);
    sub_values(&right_v, &left_v).and_then(|delta| div_values(&delta, &Value::Number(dt)))
}

fn zero_like(value: &Value) -> Result<Value, String> {
    match value {
        Value::Number(_) => Ok(Value::Number(0.0)),
        Value::Array(items) => Ok(array_of(std::iter::repeat_n(0.0, items.len()))),
        _ => Ok(Value::Number(0.0)),
    }
}

fn speed_of(value: &Value) -> f64 {
    match value {
        Value::Number(number) => number.abs(),
        Value::Array(_) => numbers_of(value)
            .map(|coords| length(&coords))
            .unwrap_or(0.0),
        _ => 0.0,
    }
}

fn wiggle(
    ctx: &mut Ctx<'_>,
    recv: Option<&Value>,
    args: &[Value],
    temporal: bool,
) -> Result<Value, String> {
    let prop = receiver_prop(ctx, recv)?;
    let freq = need_num(&args[0])?;
    let amp = need_num(&args[1])?;
    let octaves = args.get(2).and_then(Value::number).unwrap_or(1.0);
    let amp_mult = args.get(3).and_then(Value::number).unwrap_or(0.5);
    let t = args
        .get(4)
        .and_then(Value::number)
        .unwrap_or_else(|| ctx.now());
    if temporal {
        let shift = wobble(ctx, t, freq, 1.0, octaves, amp_mult, 4.0).clamp(-1.0, 1.0) * amp;
        return sample_prop(ctx, &prop, t + shift);
    }
    let base = sample_prop(ctx, &prop, t)?;
    match base {
        Value::Number(value) => Ok(Value::Number(
            value + wobble(ctx, t, freq, amp, octaves, amp_mult, 0.0),
        )),
        Value::Array(items) => {
            let mut out = Vec::new();
            for (index, item) in items.iter().enumerate() {
                let value = item.number().unwrap_or(0.0);
                out.push(value + wobble(ctx, t, freq, amp, octaves, amp_mult, index as f64));
            }
            Ok(array_of(out))
        }
        other => Ok(other),
    }
}

fn wobble(
    ctx: &Ctx<'_>,
    t: f64,
    freq: f64,
    amp: f64,
    octaves: f64,
    amp_mult: f64,
    salt: f64,
) -> f64 {
    let mut total = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = freq;
    let count = octaves.max(1.0).floor() as i32;
    for index in 0..count.max(1) {
        let phase = ctx.seed_offset + salt + f64::from(index);
        total += amplitude * (frequency * t + phase).sin();
        amplitude *= amp_mult;
        frequency *= 2.0;
    }
    amp * total
}

fn smooth(ctx: &Ctx<'_>, recv: Option<&Value>, args: &[Value]) -> Result<Value, String> {
    let prop = receiver_prop(ctx, recv)?;
    let width = args.get(0).and_then(Value::number).unwrap_or(0.2);
    let samples = args
        .get(1)
        .and_then(Value::number)
        .unwrap_or(5.0)
        .round()
        .max(1.0) as i32;
    let t = args
        .get(2)
        .and_then(Value::number)
        .unwrap_or_else(|| ctx.now());
    if samples <= 1 {
        return sample_prop(ctx, &prop, t);
    }
    let mut acc: Option<Value> = None;
    for index in 0..samples {
        let u = t - width / 2.0 + width * f64::from(index) / f64::from(samples - 1);
        let sample = sample_prop(ctx, &prop, u)?;
        acc = Some(match acc {
            Some(current) => add_values(&current, &sample)?,
            None => sample,
        });
    }
    div_values(
        &acc.unwrap_or(Value::Number(0.0)),
        &Value::Number(f64::from(samples)),
    )
}

fn loop_call(
    ctx: &Ctx<'_>,
    recv: Option<&Value>,
    args: &[Value],
    outbound: bool,
    duration_form: bool,
) -> Result<Value, String> {
    let prop = receiver_prop(ctx, recv)?;
    let property = ctx
        .scene
        .property(prop.comp, prop.layer, prop.id)
        .cloned()
        .ok_or_else(|| "missing property".to_string())?;
    let mode = ops::parse_loop_mode(args.first().and_then(Value::text).unwrap_or("cycle"))
        .ok_or_else(|| "unknown loop type".to_string())?;
    let second = args.get(1).and_then(Value::number).unwrap_or(0.0);
    let samples = key_samples(&property);
    if samples.len() < 2 {
        return sample_cloned(ctx, &prop, &property, ctx.now());
    }
    let segment = if mode == LoopMode::Continue {
        if outbound {
            let end = samples.len() - 1;
            Some((samples[end - 1].time, samples[end].time))
        } else {
            Some((samples[0].time, samples[1].time))
        }
    } else if duration_form {
        let attr = ctx.scene.layer_attr(prop.layer);
        if outbound {
            let last = samples[samples.len() - 1].time;
            let start = if second > 0.0 {
                last - second
            } else {
                attr.map(|attr| attr.in_point).unwrap_or(samples[0].time)
            };
            Some((start, last))
        } else {
            let first = samples[0].time;
            let end = if second > 0.0 {
                first + second
            } else {
                attr.map(|attr| attr.out_point)
                    .unwrap_or(samples[samples.len() - 1].time)
            };
            Some((first, end))
        }
    } else if second <= 0.0 {
        Some((samples[0].time, samples[samples.len() - 1].time))
    } else {
        let intervals = second.round() as usize;
        if outbound {
            let end = samples.len() - 1;
            let start = end.saturating_sub(intervals);
            Some((samples[start].time, samples[end].time))
        } else {
            let end = intervals.min(samples.len() - 1);
            Some((samples[0].time, samples[end].time))
        }
    };
    let scalar = matches!(sample_body(&property, ctx.now()), PropertyValue::Scalar(_));
    let sampled = ops::loop_sample(&samples, ctx.now(), mode, outbound, segment);
    if scalar {
        Ok(Value::Number(sampled.first().copied().unwrap_or(0.0)))
    } else {
        Ok(array_of(sampled))
    }
}

fn key_samples(property: &Property) -> Vec<KeySample> {
    property_keys(property)
        .into_iter()
        .map(|(time, value)| KeySample {
            time,
            value: match &value {
                PropertyValue::Scalar(number) => vec![*number],
                PropertyValue::TwoD(values) => values.to_vec(),
                PropertyValue::ThreeD(values) => values.to_vec(),
                PropertyValue::Color(color) => vec![
                    f64::from(color.r),
                    f64::from(color.g),
                    f64::from(color.b),
                    f64::from(color.a),
                ],
                _ => Vec::new(),
            },
        })
        .collect()
}

fn key_call(ctx: &Ctx<'_>, recv: Option<&Value>, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::MarkerProp { comp, layer }) = recv {
        return marker_key(ctx, *comp, *layer, &args[0]);
    }
    let prop = receiver_prop(ctx, recv)?;
    if marker_property(ctx, &prop) {
        return marker_key(ctx, prop.comp, Some(prop.layer), &args[0]);
    }
    let property = ctx
        .scene
        .property(prop.comp, prop.layer, prop.id)
        .ok_or_else(|| "missing property".to_string())?;
    let keys = property_keys(property);
    let index = need_num(&args[0])? as isize;
    if index < 1 || index as usize > keys.len() {
        return Err("key index out of range".into());
    }
    Ok(Value::KeyRef {
        prop,
        index: (index as usize) - 1,
    })
}

fn nearest_key_call(ctx: &Ctx<'_>, recv: Option<&Value>, args: &[Value]) -> Result<Value, String> {
    let time = need_num(args.first().unwrap_or(&Value::Number(ctx.now())))?;
    if let Some(Value::MarkerProp { comp, layer }) = recv {
        return nearest_marker(ctx, *comp, *layer, time);
    }
    let prop = receiver_prop(ctx, recv)?;
    if marker_property(ctx, &prop) {
        return nearest_marker(ctx, prop.comp, Some(prop.layer), time);
    }
    let property = ctx
        .scene
        .property(prop.comp, prop.layer, prop.id)
        .ok_or_else(|| "missing property".to_string())?;
    let keys = property_keys(property);
    let index = nearest_index(
        &keys.iter().map(|(time, _)| *time).collect::<Vec<_>>(),
        time,
    )?;
    Ok(Value::KeyRef { prop, index })
}

fn neighbor_key(
    ctx: &Ctx<'_>,
    recv: Option<&Value>,
    time: f64,
    next: bool,
) -> Result<Value, String> {
    let prop = receiver_prop(ctx, recv)?;
    let property = ctx
        .scene
        .property(prop.comp, prop.layer, prop.id)
        .ok_or_else(|| "missing property".to_string())?;
    let keys = property_keys(property);
    if keys.is_empty() {
        return Err("no keys".into());
    }
    let index = if next {
        keys.iter()
            .position(|(stamp, _)| *stamp > time)
            .unwrap_or(keys.len() - 1)
    } else {
        keys.iter()
            .rposition(|(stamp, _)| *stamp < time)
            .unwrap_or(0)
    };
    Ok(Value::KeyRef { prop, index })
}

fn property_group(ctx: &Ctx<'_>, recv: Option<&Value>, args: &[Value]) -> Result<Value, String> {
    let prop = receiver_prop(ctx, recv)?;
    let steps = args
        .first()
        .and_then(Value::number)
        .unwrap_or(1.0)
        .round()
        .max(1.0) as usize;
    let layer = ctx
        .scene
        .layer(prop.comp, prop.layer)
        .ok_or_else(|| "missing layer".to_string())?;
    let mut current = prop.id;
    for _ in 0..steps {
        current =
            parent_of(&layer.properties, current).ok_or_else(|| "no property group".to_string())?;
    }
    Ok(Value::Group(prop_ref(prop.comp, prop.layer, current)))
}

fn parent_of(properties: &[Property], id: model::PropertyId) -> Option<model::PropertyId> {
    for property in properties {
        if property.id == id {
            return None;
        }
        if let PropertyBody::Group { children } = &property.body {
            if children.iter().any(|child| child.id == id) {
                return Some(property.id);
            }
            if contains_id(children, id) {
                return parent_of(children, id);
            }
        }
    }
    None
}

fn marker_property(ctx: &Ctx<'_>, prop: &PropRef) -> bool {
    ctx.scene
        .property(prop.comp, prop.layer, prop.id)
        .map(|property| property.match_name == "marker" || property.name == "Marker")
        .unwrap_or(false)
}

fn markers(
    ctx: &Ctx<'_>,
    comp: model::ItemId,
    layer: Option<model::LayerId>,
) -> Vec<(f64, model::Marker)> {
    if let Some(layer) = layer {
        let Some(prop) = find_named(ctx, comp, layer, "marker") else {
            return Vec::new();
        };
        let Some(property) = ctx.scene.property(comp, layer, prop.id) else {
            return Vec::new();
        };
        return property_keys(property)
            .into_iter()
            .filter_map(|(time, value)| match value {
                PropertyValue::Marker(marker) => Some((time, *marker)),
                _ => None,
            })
            .collect();
    }
    let mut markers = ctx
        .scene
        .comp_attr(comp)
        .map(|attr| attr.markers.clone())
        .unwrap_or_default();
    markers.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    markers
}

fn marker_key(
    ctx: &Ctx<'_>,
    comp: model::ItemId,
    layer: Option<model::LayerId>,
    arg: &Value,
) -> Result<Value, String> {
    let markers = markers(ctx, comp, layer);
    let index = match arg {
        Value::Number(index) => {
            let slot = *index as isize;
            if slot < 1 || slot as usize > markers.len() {
                return Err("marker index out of range".into());
            }
            (slot as usize) - 1
        }
        Value::String(name) => markers
            .iter()
            .position(|(_, marker)| marker.comment == *name)
            .ok_or_else(|| format!("marker {name} not found"))?,
        _ => return Err("marker selector expected".into()),
    };
    Ok(Value::MarkerKey { comp, layer, index })
}

fn nearest_marker(
    ctx: &Ctx<'_>,
    comp: model::ItemId,
    layer: Option<model::LayerId>,
    time: f64,
) -> Result<Value, String> {
    let markers = markers(ctx, comp, layer);
    let index = nearest_index(
        &markers.iter().map(|(time, _)| *time).collect::<Vec<_>>(),
        time,
    )?;
    Ok(Value::MarkerKey { comp, layer, index })
}

fn nearest_index(times: &[f64], time: f64) -> Result<usize, String> {
    times
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (time - **a)
                .abs()
                .partial_cmp(&(time - **b).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(index, _)| index)
        .ok_or_else(|| "no keys".into())
}

fn member_marker_prop(
    ctx: &Ctx<'_>,
    comp: model::ItemId,
    layer: Option<model::LayerId>,
    name: &str,
) -> Result<Value, String> {
    if name == "numKeys" {
        return Ok(Value::Number(markers(ctx, comp, layer).len() as f64));
    }
    Err(format!("unknown marker member {name}"))
}

fn member_marker_key(
    ctx: &Ctx<'_>,
    comp: model::ItemId,
    layer: Option<model::LayerId>,
    index: usize,
    name: &str,
) -> Result<Value, String> {
    let markers = markers(ctx, comp, layer);
    let (time, marker) = markers
        .get(index)
        .ok_or_else(|| "missing marker".to_string())?;
    match name {
        "duration" => Ok(Value::Number(marker.duration_seconds)),
        "comment" => Ok(Value::String(marker.comment.clone())),
        "chapter" => Ok(Value::String(marker.chapter.clone())),
        "url" => Ok(Value::String(marker.url.clone())),
        "frameTarget" => Ok(Value::String(marker.frame_target.clone())),
        "eventCuePoint" => Ok(Value::Bool(marker.event_cue_point)),
        "cuePointName" => Ok(Value::String(marker.cue_point_name.clone())),
        "parameters" => {
            let mut map = std::collections::BTreeMap::new();
            for (key, value) in &marker.parameters {
                map.insert(key.clone(), Value::String(value.clone()));
            }
            Ok(Value::Map(map))
        }
        "time" => Ok(Value::Number(*time)),
        "index" => Ok(Value::Number((index + 1) as f64)),
        "protectedRegion" => Ok(Value::Bool(marker.protected_region)),
        _ => Err(format!("unknown marker key member {name}")),
    }
}

fn member_key(ctx: &Ctx<'_>, prop: &PropRef, index: usize, name: &str) -> Result<Value, String> {
    let property = ctx
        .scene
        .property(prop.comp, prop.layer, prop.id)
        .ok_or_else(|| "missing property".to_string())?;
    let keys = property_keys(property);
    let (time, value) = keys.get(index).ok_or_else(|| "missing key".to_string())?;
    match name {
        "time" => Ok(Value::Number(*time)),
        "index" => Ok(Value::Number((index + 1) as f64)),
        "value" => Ok(property_to_value(ctx, prop, value)),
        _ => Err(format!("unknown key member {name}")),
    }
}

fn source_time(ctx: &Ctx<'_>, comp: model::ItemId, layer: model::LayerId, time: f64) -> f64 {
    if let Some(prop) = find_named(ctx, comp, layer, "timeRemap") {
        if let Ok(Value::Number(value)) = sample_prop(ctx, &prop, time) {
            return value;
        }
    }
    let Some(attr) = ctx.scene.layer_attr(layer) else {
        return time;
    };
    attr.start_time + (time - attr.in_point) * attr.stretch
}

fn source_rect(ctx: &Ctx<'_>, recv: Option<&Value>, args: &[Value]) -> Result<Value, String> {
    let (_comp, layer) = receiver_layer(ctx, recv);
    let include = arg_bool(args, 1, false)?;
    let attr = ctx.scene.layer_attr(layer);
    let rect = attr.map(|attr| attr.source_rect.clone()).unwrap_or(Rect {
        top: 0.0,
        left: 0.0,
        width: 0.0,
        height: 0.0,
    });
    let extra = if include {
        attr.map(|attr| attr.extents).unwrap_or(0.0)
    } else {
        0.0
    };
    Ok(Value::Rect {
        top: rect.top,
        left: rect.left,
        width: rect.width + extra,
        height: rect.height + extra,
    })
}

fn sample_image_call(ctx: &Ctx<'_>, recv: Option<&Value>, args: &[Value]) -> Result<Value, String> {
    let (_comp, layer) = receiver_layer(ctx, recv);
    let point = need_vec(&args[0])?;
    let radius = if args.len() > 1 {
        need_vec(&args[1])?
    } else {
        vec![0.5, 0.5]
    };
    let image = ctx
        .scene
        .layer_attr(layer)
        .and_then(|attr| attr.image.clone());
    Ok(array_of(average_image(image.as_ref(), &point, &radius)))
}

fn average_image(image: Option<&Image>, point: &[f64], radius: &[f64]) -> [f64; 4] {
    let Some(image) = image else {
        return [0.0, 0.0, 0.0, 0.0];
    };
    let rx = radius.first().copied().unwrap_or(0.5);
    let ry = radius.get(1).copied().unwrap_or(0.5);
    let x0 = (point.first().copied().unwrap_or(0.0) - rx).floor() as i32;
    let y0 = (point.get(1).copied().unwrap_or(0.0) - ry).floor() as i32;
    let width = (2.0 * rx).round().max(1.0) as i32;
    let height = (2.0 * ry).round().max(1.0) as i32;
    let mut acc = [0.0; 4];
    let mut count = 0.0;
    for y in y0..y0 + height {
        for x in x0..x0 + width {
            if x < 0 || y < 0 || x as u32 >= image.width || y as u32 >= image.height {
                continue;
            }
            let pixel = image.pixels[(y as u32 * image.width + x as u32) as usize];
            for channel in 0..4 {
                acc[channel] += pixel[channel];
            }
            count += 1.0;
        }
    }
    if count == 0.0 {
        [0.0, 0.0, 0.0, 0.0]
    } else {
        acc.map(|channel| channel / count)
    }
}

#[derive(Clone, Copy)]
enum Space {
    ToComp,
    FromComp,
    ToWorld,
    FromWorld,
}

fn space_point(
    ctx: &Ctx<'_>,
    recv: Option<&Value>,
    args: &[Value],
    space: Space,
) -> Result<Value, String> {
    let (comp, layer) = receiver_layer(ctx, recv);
    let point = need_vec(&args[0])?;
    let time = arg_num(ctx, args, 1, ctx.now())?;
    let chain = xform_chain(ctx, comp, layer, time);
    let kind = ctx.scene.layer(comp, layer).map(|layer| layer.kind);
    let three_d = ctx
        .scene
        .layer(comp, layer)
        .map(|layer| layer.three_d)
        .unwrap_or(false);
    let skip_camera = matches!(
        kind,
        Some(model::LayerKind::Camera { .. } | model::LayerKind::Light { .. })
    );
    let camera = if three_d {
        camera_position(ctx, comp, time)
    } else {
        [0.0, 0.0, 0.0]
    };
    let result = match space {
        Space::ToWorld => apply_chain(&chain, &point, false),
        Space::FromWorld => invert_chain(&chain, &point, false),
        Space::ToComp => {
            let mut world = apply_chain(&chain, &point, false);
            if three_d && !skip_camera {
                for (component, camera) in world.iter_mut().zip(camera) {
                    *component -= camera;
                }
            }
            world
        }
        Space::FromComp => {
            let mut input = point.clone();
            if three_d && !skip_camera {
                for (component, camera) in input.iter_mut().zip(camera) {
                    *component += camera;
                }
            }
            invert_chain(&chain, &input, false)
        }
    };
    Ok(array_of(result))
}

fn space_vec(
    ctx: &Ctx<'_>,
    recv: Option<&Value>,
    args: &[Value],
    space: Space,
) -> Result<Value, String> {
    let (comp, layer) = receiver_layer(ctx, recv);
    let vector = need_vec(&args[0])?;
    let time = arg_num(ctx, args, 1, ctx.now())?;
    let chain = xform_chain(ctx, comp, layer, time);
    let result = match space {
        Space::ToWorld | Space::ToComp => apply_chain(&chain, &vector, true),
        Space::FromWorld | Space::FromComp => invert_chain(&chain, &vector, true),
    };
    Ok(array_of(result))
}

fn xform_chain(
    ctx: &Ctx<'_>,
    comp: model::ItemId,
    layer: model::LayerId,
    time: f64,
) -> Vec<(Xform, bool)> {
    let mut chain = Vec::new();
    let mut current = Some(layer);
    let mut guard = 0;
    while let Some(id) = current {
        if guard > 32 {
            break;
        }
        guard += 1;
        let two_d = !ctx
            .scene
            .layer(comp, id)
            .map(|layer| layer.three_d)
            .unwrap_or(false);
        chain.push((xform_of(ctx, comp, id, time), two_d));
        current = ctx.scene.layer(comp, id).and_then(|layer| layer.parent);
    }
    chain
}

fn xform_of(ctx: &Ctx<'_>, comp: model::ItemId, layer: model::LayerId, time: f64) -> Xform {
    let mut form = Xform::identity();
    if let Some(anchor) = named_vec(ctx, comp, layer, "anchorPoint", time) {
        form.anchor = as3(&anchor);
    }
    if let Some(position) = named_vec(ctx, comp, layer, "position", time) {
        form.position = as3(&position);
    }
    if let Some(scale) = named_vec(ctx, comp, layer, "scale", time) {
        form.scale = [
            scale.first().copied().unwrap_or(100.0),
            scale.get(1).copied().unwrap_or(100.0),
            scale.get(2).copied().unwrap_or(100.0),
        ];
    }
    if let Some(rotation) = named_num(ctx, comp, layer, "rotation", time) {
        form.rotation[2] = rotation;
    }
    if let Some(orientation) = named_vec(ctx, comp, layer, "orientation", time) {
        form.rotation[0] += orientation.first().copied().unwrap_or(0.0);
        form.rotation[1] += orientation.get(1).copied().unwrap_or(0.0);
        form.rotation[2] += orientation.get(2).copied().unwrap_or(0.0);
    }
    if let Some(rotation) = named_num(ctx, comp, layer, "rotationX", time) {
        form.rotation[0] += rotation;
    }
    if let Some(rotation) = named_num(ctx, comp, layer, "rotationY", time) {
        form.rotation[1] += rotation;
    }
    if let Some(rotation) = named_num(ctx, comp, layer, "rotationZ", time) {
        form.rotation[2] += rotation;
    }
    form
}

fn named_num(
    ctx: &Ctx<'_>,
    comp: model::ItemId,
    layer: model::LayerId,
    name: &str,
    time: f64,
) -> Option<f64> {
    let prop = find_named(ctx, comp, layer, name)?;
    sample_prop(ctx, &prop, time).ok()?.number()
}

fn named_vec(
    ctx: &Ctx<'_>,
    comp: model::ItemId,
    layer: model::LayerId,
    name: &str,
    time: f64,
) -> Option<Vec<f64>> {
    let prop = find_named(ctx, comp, layer, name)?;
    numbers_of(&sample_prop(ctx, &prop, time).ok()?)
}

fn apply_chain(chain: &[(Xform, bool)], point: &[f64], vector: bool) -> Vec<f64> {
    let mut current = point.to_vec();
    for (form, two_d) in chain {
        current = if vector {
            ops::apply_vec(form, &current, *two_d)
        } else {
            ops::apply_point(form, &current, *two_d)
        };
    }
    current
}

fn invert_chain(chain: &[(Xform, bool)], point: &[f64], vector: bool) -> Vec<f64> {
    let mut current = point.to_vec();
    for (form, two_d) in chain.iter().rev() {
        current = if vector {
            ops::invert_vec(form, &current, *two_d)
        } else {
            ops::invert_point(form, &current, *two_d)
        };
    }
    current
}

fn camera_position(ctx: &Ctx<'_>, comp: model::ItemId, time: f64) -> [f64; 3] {
    let Some(camera) = active_camera(ctx, comp, time) else {
        return [0.0, 0.0, 0.0];
    };
    named_vec(ctx, comp, camera, "position", time)
        .map(|position| as3(&position))
        .unwrap_or([0.0, 0.0, 0.0])
}

fn path_call(name: &str, recv: Option<&Value>, args: &[Value]) -> Result<Value, String> {
    if name == "createPath" {
        return Ok(Value::Path(build_path(args)));
    }
    let Some(Value::Path(path)) = recv else {
        return Err("path expected".into());
    };
    match name {
        "points" => Ok(points_array(&path.points)),
        "inTangents" => Ok(points_array(&pad_tangents(
            &path.in_tangents,
            path.points.len(),
        ))),
        "outTangents" => Ok(points_array(&pad_tangents(
            &path.out_tangents,
            path.points.len(),
        ))),
        "isClosed" => Ok(Value::Bool(path.closed)),
        "pointOnPath" => {
            let percent = args.first().and_then(Value::number).unwrap_or(0.5);
            Ok(array_of(ops::point_on_path(
                &path.points,
                path.closed,
                percent,
            )))
        }
        "tangentOnPath" => {
            let percent = args.first().and_then(Value::number).unwrap_or(0.5);
            Ok(array_of(ops::tangent_on_path(
                &path.points,
                path.closed,
                percent,
            )))
        }
        "normalOnPath" => {
            let percent = args.first().and_then(Value::number).unwrap_or(0.5);
            Ok(array_of(ops::normal_on_path(
                &path.points,
                path.closed,
                percent,
            )))
        }
        _ => Err(format!("unknown path call {name}")),
    }
}

fn pad_tangents(tangents: &[[f64; 2]], count: usize) -> Vec<[f64; 2]> {
    let mut out = tangents.to_vec();
    out.resize(count, [0.0, 0.0]);
    out
}

fn points_array(points: &[[f64; 2]]) -> Value {
    Value::Array(
        points
            .iter()
            .map(|point| array_of([round4(point[0]), round4(point[1])]))
            .collect(),
    )
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn build_path(args: &[Value]) -> PathData {
    if args.is_empty() {
        return ops::default_path();
    }
    let points = pair_list(args.first());
    let in_tangents = if args.len() > 1 {
        pair_list(args.get(1))
    } else {
        Vec::new()
    };
    let out_tangents = if args.len() > 2 {
        pair_list(args.get(2))
    } else {
        Vec::new()
    };
    let closed = args.get(3).and_then(Value::bool).unwrap_or(true);
    PathData {
        points,
        in_tangents,
        out_tangents,
        closed,
    }
}

fn pair_list(value: Option<&Value>) -> Vec<[f64; 2]> {
    let Some(Value::Array(items)) = value else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let coords = numbers_of(item)?;
            Some([
                coords.first().copied().unwrap_or(0.0),
                coords.get(1).copied().unwrap_or(0.0),
            ])
        })
        .collect()
}

fn add_values(left: &Value, right: &Value) -> Result<Value, String> {
    if left.number().is_some() && right.number().is_some() {
        return Ok(Value::Number(need_num(left)? + need_num(right)?));
    }
    Ok(array_of(add(&need_vec(left)?, &need_vec(right)?)))
}

fn sub_values(left: &Value, right: &Value) -> Result<Value, String> {
    if left.number().is_some() && right.number().is_some() {
        return Ok(Value::Number(need_num(left)? - need_num(right)?));
    }
    Ok(array_of(sub(&need_vec(left)?, &need_vec(right)?)))
}

fn mul_values(left: &Value, right: &Value) -> Result<Value, String> {
    match (left.number(), right.number()) {
        (Some(left), Some(right)) => Ok(Value::Number(left * right)),
        (None, Some(amount)) => Ok(array_of(mul(&need_vec(left)?, amount))),
        (Some(amount), None) => Ok(array_of(mul(&need_vec(right)?, amount))),
        (None, None) => {
            let left = need_vec(left)?;
            let right = need_vec(right)?;
            let n = left.len().max(right.len());
            let left = ops::pad(left, n);
            let right = ops::pad(right, n);
            Ok(array_of(left.into_iter().zip(right).map(|(a, b)| a * b)))
        }
    }
}

fn div_values(left: &Value, right: &Value) -> Result<Value, String> {
    match (left.number(), right.number()) {
        (Some(left), Some(right)) => Ok(Value::Number(left / right)),
        (None, Some(amount)) => Ok(array_of(div(&need_vec(left)?, amount))),
        _ => {
            let left = need_vec(left)?;
            let right = need_vec(right)?;
            let n = left.len().max(right.len());
            let left = ops::pad(left, n);
            let right = ops::pad(right, n);
            Ok(array_of(left.into_iter().zip(right).map(|(a, b)| a / b)))
        }
    }
}

fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left == right,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len() && left.iter().zip(right).all(|(a, b)| values_equal(a, b))
        }
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}
