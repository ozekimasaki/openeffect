//! OpenEffect テンプレートの読み書きと、MOGRT JSON から読める項目。

use std::fs;
use std::path::Path;

use crate::error::{ScriptError, ScriptResult};
use crate::json::{parse_json, Json};
use crate::zip::read_stored_zip;

#[derive(Clone, Debug, PartialEq)]
pub struct TemplateControl {
    pub name: String,
    pub type_name: String,
    pub default_number: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TemplateLayer {
    pub name: String,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TemplatePackage {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub duration: f64,
    pub frame_rate: f64,
    pub layers: Vec<TemplateLayer>,
    pub controls: Vec<TemplateControl>,
}

impl TemplatePackage {
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }
}

pub fn oe_template_manifest(package: &TemplatePackage) -> Json {
    Json::object(vec![
        ("name", Json::String(package.name.clone())),
        ("duration", Json::Number(package.duration)),
        ("frameRate", Json::Number(package.frame_rate)),
        ("width", Json::Number(package.width as f64)),
        ("height", Json::Number(package.height as f64)),
        ("layerCount", Json::Number(package.layer_count() as f64)),
    ])
}

pub fn oe_template_controls(package: &TemplatePackage) -> Vec<TemplateControl> {
    package.controls.clone()
}

pub fn oe_template_export(package: &TemplatePackage, path: &Path) -> ScriptResult<()> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|error| ScriptError::invalid(error.to_string()))?;
    }
    fs::create_dir_all(path).map_err(|error| ScriptError::invalid(error.to_string()))?;
    let manifest = oe_template_manifest(package);
    fs::write(path.join("manifest.json"), manifest.write())
        .map_err(|error| ScriptError::invalid(error.to_string()))?;
    let mut layers = Vec::new();
    for layer in &package.layers {
        layers.push(Json::object(vec![
            ("name", Json::String(layer.name.clone())),
            ("kind", Json::String(layer.kind.clone())),
        ]));
    }
    let mut controls = Vec::new();
    for control in &package.controls {
        controls.push(Json::object(vec![
            ("name", Json::String(control.name.clone())),
            ("type", Json::String(control.type_name.clone())),
            (
                "default",
                match control.default_number {
                    Some(value) => Json::Number(value),
                    None => Json::Null,
                },
            ),
        ]));
    }
    let snapshot = Json::object(vec![
        ("name", Json::String(package.name.clone())),
        ("width", Json::Number(package.width as f64)),
        ("height", Json::Number(package.height as f64)),
        ("duration", Json::Number(package.duration)),
        ("frameRate", Json::Number(package.frame_rate)),
        ("layers", Json::Array(layers)),
        ("controls", Json::Array(controls)),
    ]);
    fs::write(path.join("snapshot.json"), snapshot.write())
        .map_err(|error| ScriptError::invalid(error.to_string()))?;
    Ok(())
}

pub fn oe_template_import(path: &Path) -> ScriptResult<TemplatePackage> {
    let manifest_text = fs::read_to_string(path.join("manifest.json"))
        .map_err(|error| ScriptError::invalid(error.to_string()))?;
    let snapshot_text = fs::read_to_string(path.join("snapshot.json"))
        .map_err(|error| ScriptError::invalid(error.to_string()))?;
    let manifest = parse_json(&manifest_text)?;
    let snapshot = parse_json(&snapshot_text)?;
    let _ = manifest;
    read_package(&snapshot)
}

fn read_package(snapshot: &Json) -> ScriptResult<TemplatePackage> {
    let name = snapshot
        .get("name")
        .and_then(Json::as_str)
        .ok_or_else(|| ScriptError::invalid("テンプレート名が無い"))?
        .to_string();
    let width = snapshot.get("width").and_then(Json::as_f64).unwrap_or(0.0) as u32;
    let height = snapshot.get("height").and_then(Json::as_f64).unwrap_or(0.0) as u32;
    let duration = snapshot.get("duration").and_then(Json::as_f64).unwrap_or(0.0);
    let frame_rate = snapshot.get("frameRate").and_then(Json::as_f64).unwrap_or(0.0);
    let mut layers = Vec::new();
    if let Some(items) = snapshot.get("layers").and_then(Json::as_array) {
        for item in items {
            let name = item.get("name").and_then(Json::as_str).unwrap_or("").to_string();
            let kind = item.get("kind").and_then(Json::as_str).unwrap_or("").to_string();
            layers.push(TemplateLayer { name, kind });
        }
    }
    let controls = read_controls(snapshot);
    Ok(TemplatePackage {
        name,
        width,
        height,
        duration,
        frame_rate,
        layers,
        controls,
    })
}

pub fn oe_mogrt_list_json(path: &Path) -> ScriptResult<Vec<String>> {
    let entries = read_stored_zip(path)?;
    let mut names = Vec::new();
    for entry in entries {
        if entry.name.rsplit('.').next().is_some_and(|ext| ext.eq_ignore_ascii_case("json")) {
            names.push(entry.name);
        }
    }
    Ok(names)
}

pub fn oe_mogrt_read_name(json: &Json) -> ScriptResult<String> {
    match json.get("name").and_then(Json::as_str) {
        Some(name) => Ok(name.to_string()),
        None => Err(ScriptError::unconfirmed("名前キーが無い")),
    }
}

pub fn oe_mogrt_read_controls(json: &Json) -> Vec<TemplateControl> {
    read_controls(json)
}

fn read_controls(json: &Json) -> Vec<TemplateControl> {
    let Some(items) = json.get("controls").and_then(Json::as_array) else {
        return Vec::new();
    };
    let mut controls = Vec::new();
    for item in items {
        let Some(name) = item.get("name").and_then(Json::as_str) else {
            continue;
        };
        let Some(type_name) = item.get("type").and_then(Json::as_str) else {
            continue;
        };
        if !known_control_type(type_name) {
            continue;
        }
        let Some(default_number) = item.get("default").and_then(Json::as_f64) else {
            if type_name != "number" && type_name != "float" {
                continue;
            }
            continue;
        };
        controls.push(TemplateControl {
            name: name.to_string(),
            type_name: canonical_type(type_name).to_string(),
            default_number: Some(default_number),
        });
    }
    controls
}

fn known_control_type(type_name: &str) -> bool {
    matches!(type_name, "number" | "float" | "bool" | "color" | "text" | "point")
}

fn canonical_type(type_name: &str) -> &str {
    match type_name {
        "float" => "number",
        other => other,
    }
}
