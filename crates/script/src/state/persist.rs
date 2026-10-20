use std::path::Path;

use super::{App, ItemBody, SourceKind};
use crate::error::{ScriptError, ScriptResult};
use crate::json::{parse_json, Json};

impl App {
    pub fn save(&mut self, file: Option<std::path::PathBuf>) -> ScriptResult<()> {
        let path = file.or_else(|| self.file.clone()).ok_or_else(|| ScriptError::invalid("保存先が無い"))?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        let json = self.to_json();
        std::fs::write(&path, json.write()).map_err(|error| ScriptError::invalid(error.to_string()))?;
        self.file = Some(path);
        self.dirty = false;
        self.revision += 1;
        Ok(())
    }

    pub fn save_with_dialog(&mut self) -> ScriptResult<bool> {
        if !self.save_dialog_set {
            return Err(ScriptError::invalid("保存ダイアログの応答が無い"));
        }
        self.save_dialog_set = false;
        if let Some(path) = self.save_dialog.clone() {
            self.save(Some(path))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn open(path: &Path) -> ScriptResult<Self> {
        let text = std::fs::read_to_string(path).map_err(|error| ScriptError::invalid(error.to_string()))?;
        let json = parse_json(&text)?;
        let mut app = Self::new();
        app.read_json(&json)?;
        app.file = Some(path.to_path_buf());
        app.dirty = false;
        Ok(app)
    }

    fn to_json(&self) -> Json {
        let mut items = Vec::new();
        for id in &self.order {
            let item = &self.items[id];
            let mut pairs = vec![
                ("id", Json::Number(id.get() as f64)),
                ("name", Json::String(item.name.clone())),
                ("comment", Json::String(item.comment.clone())),
                ("label", Json::Number(item.label as f64)),
                ("parent", Json::Number(item.parent.get() as f64)),
            ];
            match &item.body {
                ItemBody::Folder { .. } => pairs.push(("kind", Json::String("folder".to_string()))),
                ItemBody::Comp(comp) => {
                    pairs.push(("kind", Json::String("comp".to_string())));
                    pairs.push(("width", Json::Number(comp.width as f64)));
                    pairs.push(("height", Json::Number(comp.height as f64)));
                    pairs.push(("frameRate", Json::Number(comp.frame_rate)));
                    pairs.push(("duration", Json::Number(comp.duration)));
                    pairs.push(("pixelAspect", Json::Number(comp.pixel_aspect)));
                }
                ItemBody::Footage(footage) => {
                    pairs.push(("kind", Json::String("footage".to_string())));
                    pairs.push(("width", Json::Number(footage.main.width as f64)));
                    pairs.push(("height", Json::Number(footage.main.height as f64)));
                    pairs.push(("frameRate", Json::Number(footage.main.frame_rate)));
                    pairs.push(("duration", Json::Number(footage.main.duration)));
                    pairs.push((
                        "path",
                        Json::String(
                            footage
                                .main
                                .path
                                .as_ref()
                                .map(|path| path.display().to_string())
                                .unwrap_or_default(),
                        ),
                    ));
                    pairs.push(("still", Json::Bool(footage.main.is_still)));
                    pairs.push((
                        "sourceKind",
                        Json::String(format!("{:?}", footage.main.kind).to_ascii_lowercase()),
                    ));
                }
            }
            items.push(Json::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()));
        }
        Json::object(vec![
            ("revision", Json::Number(self.revision as f64)),
            ("xmp", Json::String(self.xmp.clone())),
            ("bits", Json::Number(self.bits.bits() as f64)),
            ("workingSpace", Json::String(self.working_space.clone())),
            ("workingGamma", Json::Number(self.working_gamma)),
            ("expressionEngine", Json::String(self.expression_engine.clone())),
            ("items", Json::Array(items)),
        ])
    }

    fn read_json(&mut self, json: &Json) -> ScriptResult<()> {
        if let Some(value) = json.get("xmp").and_then(Json::as_str) {
            self.xmp = value.to_string();
        }
        if let Some(value) = json.get("revision").and_then(Json::as_f64) {
            self.revision = value as i64;
        }
        if let Some(value) = json.get("bits").and_then(Json::as_f64) {
            if let Some(bits) = model::BitsPerChannel::from_u8(value as u8) {
                self.bits = bits;
            }
        }
        if let Some(value) = json.get("workingSpace").and_then(Json::as_str) {
            self.working_space = value.to_string();
        }
        if let Some(value) = json.get("workingGamma").and_then(Json::as_f64) {
            self.working_gamma = value;
        }
        if let Some(value) = json.get("expressionEngine").and_then(Json::as_str) {
            self.expression_engine = value.to_string();
        }
        let Some(items) = json.get("items").and_then(Json::as_array) else {
            return Ok(());
        };
        for item in items {
            let id_num = item.get("id").and_then(Json::as_f64).unwrap_or(0.0) as u64;
            let name = item.get("name").and_then(Json::as_str).unwrap_or("").to_string();
            let kind = item.get("kind").and_then(Json::as_str).unwrap_or("");
            let comment = item.get("comment").and_then(Json::as_str).unwrap_or("").to_string();
            let label = item.get("label").and_then(Json::as_f64).unwrap_or(0.0) as i32;
            let id = model::ItemId::new(id_num);
            match kind {
                "comp" => {
                    let created = self.add_comp_in(
                        self.root,
                        &name,
                        item.get("width").and_then(Json::as_f64).unwrap_or(1920.0) as i32,
                        item.get("height").and_then(Json::as_f64).unwrap_or(1080.0) as i32,
                        item.get("pixelAspect").and_then(Json::as_f64).unwrap_or(1.0),
                        item.get("duration").and_then(Json::as_f64).unwrap_or(0.0),
                        item.get("frameRate").and_then(Json::as_f64).unwrap_or(30.0),
                    )?;
                    let _ = created;
                    if let Some(record) = self.items.get_mut(&created) {
                        record.comment = comment;
                        record.label = label;
                    }
                    let _ = id;
                }
                "footage" => {
                    let path = item.get("path").and_then(Json::as_str).unwrap_or("");
                    let options = crate::values::ImportOptions::new(if path.is_empty() {
                        None
                    } else {
                        Some(std::path::PathBuf::from(path))
                    });
                    if path.is_empty() {
                        let created = self.add_placeholder(
                            self.root,
                            &name,
                            item.get("width").and_then(Json::as_f64).unwrap_or(1.0) as u32,
                            item.get("height").and_then(Json::as_f64).unwrap_or(1.0) as u32,
                            item.get("frameRate").and_then(Json::as_f64).unwrap_or(30.0),
                            item.get("duration").and_then(Json::as_f64).unwrap_or(0.0),
                        )?;
                        if let Some(record) = self.items.get_mut(&created) {
                            record.comment = comment;
                            record.label = label;
                            record.name = name;
                        }
                    } else {
                        let created = self.import_footage_file(Path::new(path), &options)?;
                        if let Some(record) = self.items.get_mut(&created) {
                            record.comment = comment;
                            record.label = label;
                            record.name = name;
                        }
                    }
                }
                "folder" => {
                    let created = self.add_folder_in(self.root, &name)?;
                    if let Some(record) = self.items.get_mut(&created) {
                        record.comment = comment;
                        record.label = label;
                    }
                }
                _ => {}
            }
        }
        let _ = SourceKind::File;
        Ok(())
    }
}
