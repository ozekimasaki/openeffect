use std::path::PathBuf;

use model::{BitsPerChannel, ItemId, LayerId};

use super::{check_int_range, check_range, App, ItemBody};
use crate::enums::*;
use crate::error::{ScriptError, ScriptResult};
use crate::values::ImportOptions;

impl App {
    pub fn num_items(&self) -> i32 {
        self.order.len() as i32
    }

    pub fn items_length(&self) -> i32 {
        self.num_items()
    }

    pub fn item_at(&self, index: i32) -> ScriptResult<ItemId> {
        if index < 1 {
            return Err(ScriptError::range("item index"));
        }
        self.order
            .get((index - 1) as usize)
            .copied()
            .ok_or_else(|| ScriptError::range("item index"))
    }

    pub fn collection_at(&self, index: i32) -> ScriptResult<ItemId> {
        self.item_at(index)
    }

    pub fn root_folder(&self) -> ItemId {
        self.root
    }

    pub fn item_by_id(&self, id: u64) -> ScriptResult<ItemId> {
        let id = ItemId::new(id);
        if self.items.contains_key(&id) && id != self.root {
            Ok(id)
        } else if id == self.root {
            Ok(id)
        } else {
            Err(ScriptError::missing("item id"))
        }
    }

    pub fn layer_by_id(&self, id: i64) -> ScriptResult<Option<LayerId>> {
        if id < 0 {
            return Err(ScriptError::range("layer id"));
        }
        let id = LayerId::new(id as u64);
        Ok(self.layers.contains_key(&id).then_some(id))
    }

    pub fn active_item(&self) -> Option<ItemId> {
        if let Some(viewer) = &self.viewer {
            if viewer.layer.is_none() {
                return Some(viewer.item);
            }
        }
        let selected = self.selection();
        if selected.len() == 1 {
            Some(selected[0])
        } else {
            None
        }
    }

    pub fn selection(&self) -> Vec<ItemId> {
        self.order
            .iter()
            .copied()
            .filter(|id| self.items.get(id).is_some_and(|item| item.selected))
            .collect()
    }

    pub fn bits_per_channel(&self) -> u8 {
        self.bits.bits()
    }

    pub fn set_bits_per_channel(&mut self, bits: u8) -> ScriptResult<()> {
        self.bits = BitsPerChannel::from_u8(bits).ok_or_else(|| ScriptError::invalid("bitsPerChannel"))?;
        self.touch();
        Ok(())
    }

    pub fn compensate_for_scene_referred_profiles(&self) -> bool {
        self.compensate_scene
    }

    pub fn set_compensate_for_scene_referred_profiles(&mut self, value: bool) {
        self.compensate_scene = value;
        self.touch();
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_dirty(&self, _value: bool) -> ScriptResult<()> {
        Err(ScriptError::readonly("dirty"))
    }

    pub fn display_start_frame(&self) -> i32 {
        self.display_start_frame
    }

    pub fn set_display_start_frame(&mut self, value: i32) -> ScriptResult<()> {
        check_int_range(value, 0, 1, "displayStartFrame")?;
        self.display_start_frame = value;
        self.frames_count = if value == 0 {
            FramesCountType::Start0
        } else {
            FramesCountType::Start1
        };
        self.touch();
        Ok(())
    }

    pub fn frame_origin(&self) -> i32 {
        match self.frames_count {
            FramesCountType::Start0 | FramesCountType::TimecodeConversion => 0,
            FramesCountType::Start1 => 1,
        }
    }

    pub fn expression_engine(&self) -> &str {
        &self.expression_engine
    }

    pub fn set_expression_engine(&mut self, value: &str) -> ScriptResult<()> {
        if value != "extendscript" && value != "javascript-1.0" {
            return Err(ScriptError::invalid("expressionEngine"));
        }
        self.expression_engine = value.to_string();
        self.touch();
        Ok(())
    }

    pub fn feet_frames_film_type(&self) -> FeetFramesFilmType {
        self.feet_film
    }

    pub fn set_feet_frames_film_type(&mut self, value: FeetFramesFilmType) {
        self.feet_film = value;
        self.touch();
    }

    pub fn project_file(&self) -> Option<&PathBuf> {
        self.file.as_ref()
    }

    pub fn footage_timecode_display_start_type(&self) -> FootageTimecodeDisplayStartType {
        self.timecode_start
    }

    pub fn set_footage_timecode_display_start_type(&mut self, value: FootageTimecodeDisplayStartType) {
        self.timecode_start = value;
        self.touch();
    }

    pub fn frames_count_type(&self) -> FramesCountType {
        self.frames_count
    }

    pub fn set_frames_count_type(&mut self, value: FramesCountType) {
        self.frames_count = value;
        if value == FramesCountType::TimecodeConversion || value == FramesCountType::Start0 {
            self.display_start_frame = 0;
        } else {
            self.display_start_frame = 1;
        }
        self.touch();
    }

    pub fn frames_use_feet_frames(&self) -> bool {
        self.feet_frames
    }

    pub fn set_frames_use_feet_frames(&mut self, value: bool) {
        self.feet_frames = value;
        self.touch();
    }

    pub fn time_display_is_frames(&self) -> bool {
        self.time_display == TimeDisplayType::Frames && !self.feet_frames
    }

    pub fn time_display_is_feet_frames(&self) -> bool {
        self.feet_frames
    }

    pub fn gpu_accel_type(&self) -> GpuAccelType {
        self.gpu
    }

    pub fn set_gpu_accel_type(&mut self, value: GpuAccelType) -> ScriptResult<()> {
        if self.platform == Platform::Windows && value == GpuAccelType::Metal {
            return Err(ScriptError::invalid("Windows では Metal を選べない"));
        }
        self.gpu = value;
        self.touch();
        Ok(())
    }

    pub fn linear_blending(&self) -> bool {
        self.linear_blending
    }

    pub fn set_linear_blending(&mut self, value: bool) {
        self.linear_blending = value;
        self.touch();
    }

    pub fn compositing_is_linear(&self) -> bool {
        self.linear_blending || self.linearize_working_space
    }

    pub fn linearize_working_space(&self) -> bool {
        self.linearize_working_space
    }

    pub fn set_linearize_working_space(&mut self, value: bool) {
        self.linearize_working_space = value;
        self.touch();
    }

    pub fn revision(&self) -> i64 {
        self.revision
    }

    pub fn time_display_type(&self) -> TimeDisplayType {
        self.time_display
    }

    pub fn set_time_display_type(&mut self, value: TimeDisplayType) {
        self.time_display = value;
        if value == TimeDisplayType::Frames {
            self.feet_frames = false;
        }
        self.touch();
    }

    pub fn tool_type(&self) -> ToolType {
        self.tool
    }

    pub fn set_tool_type(&mut self, value: ToolType) {
        self.tool = value;
        self.touch();
    }

    pub fn transparency_grid_thumbnails(&self) -> bool {
        self.transparency_grid
    }

    pub fn set_transparency_grid_thumbnails(&mut self, value: bool) {
        self.transparency_grid = value;
        self.touch();
    }

    pub fn working_gamma(&self) -> f64 {
        self.working_gamma
    }

    pub fn set_working_gamma(&mut self, value: f64) -> ScriptResult<()> {
        if (value - 2.2).abs() > 1e-6 && (value - 2.4).abs() > 1e-6 {
            return Err(ScriptError::invalid("workingGamma"));
        }
        self.working_gamma = value;
        self.touch();
        Ok(())
    }

    pub fn working_space(&self) -> &str {
        &self.working_space
    }

    pub fn set_working_space(&mut self, value: &str) -> ScriptResult<()> {
        if !value.is_empty() && !Self::color_profiles().contains(&value) {
            return Err(ScriptError::invalid("不明なカラースペース"));
        }
        self.working_space = value.to_string();
        self.touch();
        Ok(())
    }

    pub fn list_color_profiles(&self) -> Vec<String> {
        Self::color_profiles().iter().map(|name| (*name).to_string()).collect()
    }

    pub fn xmp_packet(&self) -> &str {
        &self.xmp
    }

    pub fn set_xmp_packet(&mut self, value: &str) {
        self.xmp = value.to_string();
        self.touch();
    }

    pub fn project_window_visible(&self) -> bool {
        self.project_panel
    }

    pub fn show_window(&mut self, show: bool) {
        self.project_panel = show;
    }

    pub fn set_default_import_folder(&mut self, folder: Option<PathBuf>) -> bool {
        self.default_import_folder = folder;
        true
    }

    pub fn clear_default_import_folder(&mut self) -> bool {
        self.default_import_folder = None;
        true
    }

    pub fn default_import_folder(&self) -> Option<&PathBuf> {
        self.default_import_folder.as_ref()
    }

    pub fn import_dialog_folder(&self) -> Option<&PathBuf> {
        self.default_import_folder.as_ref()
    }

    pub fn set_next_import(&mut self, files: Option<Vec<PathBuf>>) {
        self.import_dialog = files;
    }

    pub fn set_next_save(&mut self, file: Option<PathBuf>) {
        self.save_dialog = file;
        self.save_dialog_set = true;
    }

    pub fn close(&mut self, options: CloseOptions) -> ScriptResult<bool> {
        match options {
            CloseOptions::DoNotSaveChanges => {
                *self = Self::with_platform(self.platform);
                Ok(true)
            }
            CloseOptions::SaveChanges => {
                self.save(None)?;
                let platform = self.platform;
                *self = Self::with_platform(platform);
                Ok(true)
            }
            CloseOptions::PromptToSaveChanges => {
                if !self.save_dialog_set {
                    return Err(ScriptError::invalid("保存ダイアログの応答が無い"));
                }
                self.save_dialog_set = false;
                if let Some(path) = self.save_dialog.clone() {
                    self.save(Some(path))?;
                    let platform = self.platform;
                    *self = Self::with_platform(platform);
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }

    pub fn import_file(&mut self, options: &ImportOptions) -> ScriptResult<ItemId> {
        let path = options
            .file
            .clone()
            .ok_or_else(|| ScriptError::invalid("ImportOptions.file が無い"))?;
        match options.import_as {
            ImportAsType::Footage => self.import_footage_file(&path, options),
            ImportAsType::Comp | ImportAsType::CompCroppedLayers => {
                if !options.can_import_as(options.import_as) {
                    return Err(ScriptError::invalid("コンポジションとして読めない"));
                }
                self.add_comp_in(self.root, "comp", 1920, 1080, 1.0, 10.0, 30.0)
            }
            ImportAsType::Project => Err(ScriptError::unsupported("プロジェクトの読み込みは open を使う")),
        }
    }

    pub fn import_file_with_dialog(&mut self) -> ScriptResult<Option<Vec<ItemId>>> {
        let Some(files) = self.import_dialog.clone() else {
            return Ok(None);
        };
        self.import_dialog = None;
        let mut ids = Vec::new();
        for file in files {
            let options = ImportOptions::new(Some(file));
            ids.push(self.import_file(&options)?);
        }
        Ok(Some(ids))
    }

    pub fn import_placeholder(
        &mut self,
        name: &str,
        width: i32,
        height: i32,
        frame_rate: f64,
        duration: f64,
    ) -> ScriptResult<ItemId> {
        check_int_range(width, 4, 30000, "width")?;
        check_int_range(height, 4, 30000, "height")?;
        check_range(frame_rate, 1.0, 99.0, "frameRate")?;
        check_range(duration, 0.0, 10800.0, "duration")?;
        self.add_placeholder(self.root, name, width as u32, height as u32, frame_rate, duration)
    }

    fn unsupported_team(&self, name: &str) -> ScriptResult<()> {
        let _ = self.num_items();
        Err(ScriptError::unsupported(name))
    }

    pub fn new_team_project(&self, _name: &str, _description: &str) -> ScriptResult<bool> {
        self.unsupported_team("newTeamProject")?;
        Ok(false)
    }

    pub fn open_team_project(&self, _name: &str) -> ScriptResult<bool> {
        self.unsupported_team("openTeamProject")?;
        Ok(false)
    }

    pub fn share_team_project(&self, _comment: &str) -> ScriptResult<bool> {
        self.unsupported_team("shareTeamProject")?;
        Ok(false)
    }

    pub fn sync_team_project(&self) -> ScriptResult<bool> {
        self.unsupported_team("syncTeamProject")?;
        Ok(false)
    }

    pub fn close_team_project(&self) -> ScriptResult<bool> {
        self.unsupported_team("closeTeamProject")?;
        Ok(false)
    }

    pub fn convert_team_project_to_project(&self, _file: Option<PathBuf>) -> ScriptResult<bool> {
        self.unsupported_team("convertTeamProjectToProject")?;
        Ok(false)
    }

    pub fn list_team_projects(&self) -> ScriptResult<Vec<String>> {
        self.unsupported_team("listTeamProjects")?;
        Ok(Vec::new())
    }

    pub fn is_team_project_open(&self, _name: &str) -> ScriptResult<bool> {
        self.unsupported_team("isTeamProjectOpen")?;
        Ok(false)
    }

    pub fn is_any_team_project_open(&self) -> ScriptResult<bool> {
        self.unsupported_team("isAnyTeamProjectOpen")?;
        Ok(false)
    }

    pub fn is_team_project_enabled(&self) -> ScriptResult<bool> {
        self.unsupported_team("isTeamProjectEnabled")?;
        Ok(false)
    }

    pub fn is_logged_in_to_team_project(&self) -> ScriptResult<bool> {
        self.unsupported_team("isLoggedInToTeamProject")?;
        Ok(false)
    }

    pub fn is_sync_command_enabled(&self) -> ScriptResult<bool> {
        self.unsupported_team("isSyncCommandEnabled")?;
        Ok(false)
    }

    pub fn is_share_command_enabled(&self) -> ScriptResult<bool> {
        self.unsupported_team("isShareCommandEnabled")?;
        Ok(false)
    }

    pub fn is_resolve_command_enabled(&self) -> ScriptResult<bool> {
        self.unsupported_team("isResolveCommandEnabled")?;
        Ok(false)
    }

    pub fn resolve_conflict(&self, _kind: ResolveType) -> ScriptResult<bool> {
        self.unsupported_team("resolveConflict")?;
        Ok(false)
    }

    pub fn consolidate_footage(&mut self) -> ScriptResult<i32> {
        let mut groups: Vec<(String, Vec<ItemId>)> = Vec::new();
        let ids: Vec<ItemId> = self.order.clone();
        for id in ids {
            let path = match &self.item(id)?.body {
                ItemBody::Footage(footage) => footage.main.path.clone(),
                _ => None,
            };
            let Some(path) = path else { continue };
            let key = path.display().to_string();
            if let Some(group) = groups.iter_mut().find(|(existing, _)| existing == &key) {
                group.1.push(id);
            } else {
                groups.push((key, vec![id]));
            }
        }
        let mut removed = 0;
        for (_, mut group) in groups {
            if group.len() < 2 {
                continue;
            }
            let keep = group.remove(0);
            for duplicate in group {
                self.retarget_footage(duplicate, keep)?;
                self.remove_item(duplicate)?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    pub fn remove_unused_footage(&mut self) -> ScriptResult<i32> {
        let used = self.used_footage();
        let ids: Vec<ItemId> = self.order.clone();
        let mut removed = 0;
        for id in ids {
            let is_footage = matches!(self.item(id)?.body, ItemBody::Footage(_));
            if is_footage && !used.contains(&id) {
                self.remove_item(id)?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    pub fn reduce_project(&mut self, keep: &[ItemId]) -> ScriptResult<i32> {
        let mut needed = keep.to_vec();
        let mut index = 0;
        while index < needed.len() {
            let id = needed[index];
            index += 1;
            if let Ok(item) = self.item(id) {
                let parent = item.parent;
                if parent != self.root && parent != id && !needed.contains(&parent) {
                    needed.push(parent);
                }
                if let ItemBody::Comp(comp) = &item.body {
                    let layers = comp.layers.clone();
                    for layer in layers {
                        if let Some(source) = self.layer(layer)?.source_id() {
                            if !needed.contains(&source) {
                                needed.push(source);
                            }
                        }
                    }
                }
            }
        }
        let before = self.num_items();
        let ids: Vec<ItemId> = self.order.clone();
        for id in ids {
            if !needed.contains(&id) {
                self.remove_item(id)?;
            }
        }
        Ok(before - self.num_items())
    }

    pub fn replace_font(
        &mut self,
        from: &crate::text::FontObject,
        to: &crate::text::FontObject,
        no_font_locking: bool,
    ) -> ScriptResult<bool> {
        let layers: Vec<LayerId> = self.layers.keys().copied().collect();
        let mut changed = false;
        for layer in layers {
            let Some(mut text) = self.layer(layer)?.text.clone() else {
                continue;
            };
            if text.replace_font(&from.post_script_name, &to.post_script_name, no_font_locking) {
                changed = true;
                self.layer_mut(layer)?.text = Some(text.clone());
                if let Some(prop) = self.find_layer_prop(layer, "Source Text") {
                    if let crate::state::Value::Text(doc) = &mut self.prop_mut(prop)?.value {
                        *doc = text;
                    }
                }
            }
        }
        if changed {
            self.touch();
        }
        Ok(changed)
    }

    pub fn used_fonts(&self) -> Vec<crate::text::FontObject> {
        let mut fonts = Vec::new();
        for layer in self.layers.values() {
            if let Some(text) = &layer.text {
                for ch in &text.chars {
                    if !fonts.iter().any(|font: &crate::text::FontObject| {
                        font.post_script_name == ch.font.post_script_name
                    }) {
                        fonts.push(ch.font.clone());
                    }
                }
            }
        }
        fonts
    }

    pub fn auto_fix_expressions(&mut self, old_text: &str, new_text: &str) -> ScriptResult<()> {
        let ids: Vec<model::PropertyId> = self.props.keys().copied().collect();
        for id in ids {
            let expression = self.prop(id)?.expression.clone();
            if !expression.contains(old_text) {
                continue;
            }
            let replaced = expression.replace(old_text, new_text);
            let value = self.sample_property(id, self.property_time(id))?;
            if eval_expression(&replaced, &value).is_ok() {
                let prop = self.prop_mut(id)?;
                prop.expression = replaced;
                prop.expression_error.clear();
            }
        }
        self.touch();
        Ok(())
    }
}

pub(super) fn eval_expression(source: &str, value: &super::Value) -> Result<super::Value, String> {
    let tokens = tokenize(source)?;
    if tokens.is_empty() {
        return Err("空の式".to_string());
    }
    let mut parser = ExprParser { tokens, index: 0 };
    let result = parser.parse_add(value)?;
    if parser.index != parser.tokens.len() {
        return Err("式の余り".to_string());
    }
    Ok(result)
}

struct ExprParser {
    tokens: Vec<Tok>,
    index: usize,
}

enum Tok {
    Num(f64),
    Value,
    Op(char),
    LParen,
    RParen,
}

fn tokenize(source: &str) -> Result<Vec<Tok>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            index += 1;
            continue;
        }
        if ch == '+' || ch == '-' || ch == '*' || ch == '/' {
            tokens.push(Tok::Op(ch));
            index += 1;
            continue;
        }
        if ch == '(' {
            tokens.push(Tok::LParen);
            index += 1;
            continue;
        }
        if ch == ')' {
            tokens.push(Tok::RParen);
            index += 1;
            continue;
        }
        if ch.is_ascii_digit() || ch == '.' {
            let start = index;
            index += 1;
            while index < chars.len() && (chars[index].is_ascii_digit() || chars[index] == '.') {
                index += 1;
            }
            let text: String = chars[start..index].iter().collect();
            let number = text.parse::<f64>().map_err(|_| "数値".to_string())?;
            tokens.push(Tok::Num(number));
            continue;
        }
        if ch.is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < chars.len() && (chars[index].is_ascii_alphanumeric() || chars[index] == '_') {
                index += 1;
            }
            let text: String = chars[start..index].iter().collect();
            if text == "value" {
                tokens.push(Tok::Value);
            } else {
                return Err(format!("未知の識別子 {text}"));
            }
            continue;
        }
        return Err(format!("読めない文字 {ch}"));
    }
    Ok(tokens)
}

impl ExprParser {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.index)
    }

    fn parse_add(&mut self, value: &super::Value) -> Result<super::Value, String> {
        let mut left = self.parse_mul(value)?;
        while let Some(Tok::Op('+') | Tok::Op('-')) = self.peek() {
            let op = match self.tokens[self.index] {
                Tok::Op(op) => op,
                _ => break,
            };
            self.index += 1;
            let right = self.parse_mul(value)?;
            left = apply_op(left, right, op)?;
        }
        Ok(left)
    }

    fn parse_mul(&mut self, value: &super::Value) -> Result<super::Value, String> {
        let mut left = self.parse_unary(value)?;
        while let Some(Tok::Op('*') | Tok::Op('/')) = self.peek() {
            let op = match self.tokens[self.index] {
                Tok::Op(op) => op,
                _ => break,
            };
            self.index += 1;
            let right = self.parse_unary(value)?;
            left = apply_op(left, right, op)?;
        }
        Ok(left)
    }

    fn parse_unary(&mut self, value: &super::Value) -> Result<super::Value, String> {
        if let Some(Tok::Op('-')) = self.peek() {
            self.index += 1;
            let inner = self.parse_unary(value)?;
            return apply_op(super::Value::Scalar(0.0), inner, '-');
        }
        self.parse_primary(value)
    }

    fn parse_primary(&mut self, value: &super::Value) -> Result<super::Value, String> {
        let Some(token) = self.tokens.get(self.index) else {
            return Err("式が途中で終わった".to_string());
        };
        match token {
            Tok::Num(number) => {
                let number = *number;
                self.index += 1;
                Ok(super::Value::Scalar(number))
            }
            Tok::Value => {
                self.index += 1;
                Ok(value.clone())
            }
            Tok::LParen => {
                self.index += 1;
                let inner = self.parse_add(value)?;
                if !matches!(self.tokens.get(self.index), Some(Tok::RParen)) {
                    return Err("括弧が閉じない".to_string());
                }
                self.index += 1;
                Ok(inner)
            }
            _ => Err("式の項が無い".to_string()),
        }
    }
}

fn apply_op(left: super::Value, right: super::Value, op: char) -> Result<super::Value, String> {
    let (Some(a), Some(b)) = (as_scalar(&left), as_scalar(&right)) else {
        return Err("数値ではない".to_string());
    };
    let value = match op {
        '+' => a + b,
        '-' => a - b,
        '*' => a * b,
        '/' => a / b,
        _ => return Err("演算子".to_string()),
    };
    Ok(super::Value::Scalar(value))
}

fn as_scalar(value: &super::Value) -> Option<f64> {
    match value {
        super::Value::Scalar(number) => Some(*number),
        _ => None,
    }
}
