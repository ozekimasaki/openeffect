use std::path::PathBuf;

use io_api::{OutputFormat, PostRenderAction, RenderOutput};
use model::ItemId;

use super::{App, ItemBody, OutputMod, RqItem};
use crate::enums::*;
use crate::error::{ScriptError, ScriptResult};

impl App {
    pub fn render_queue_len(&self) -> i32 {
        self.render.items.len() as i32
    }

    pub fn can_queue_in_ame(&self) -> bool {
        false
    }

    pub fn queue_notify(&self) -> bool {
        self.render.notify
    }

    pub fn set_queue_notify(&mut self, value: bool) {
        self.render.notify = value;
    }

    pub fn rendering(&self) -> bool {
        self.render.rendering
    }

    pub fn render_window_visible(&self) -> bool {
        self.render_panel
    }

    pub fn show_render_window(&mut self, show: bool) {
        self.render_panel = show;
    }

    pub fn queue_add(&mut self, comp: ItemId) -> ScriptResult<usize> {
        let _ = self.require_comp(comp)?;
        let duration = self.require_comp(comp)?.duration;
        let mut settings = self
            .render
            .render_templates
            .get("Best Settings")
            .cloned()
            .unwrap_or_default();
        settings.insert("Quality".to_string(), "Best".to_string());
        let module = OutputMod {
            file: None,
            include_source_xmp: true,
            name: "Lossless".to_string(),
            post_render: ScriptPostRenderAction::None,
            template: String::new(),
            settings: {
                let mut map = std::collections::BTreeMap::new();
                map.insert("Format".to_string(), "PNG".to_string());
                map
            },
        };
        self.render.items.push(RqItem {
            comp,
            log_type: LogType::ErrorsAndSettings,
            notify: false,
            render: true,
            skip_frames: 0,
            status: RqItemStatus::Queued,
            time_span_start: 0.0,
            time_span_duration: duration,
            elapsed: Some(0),
            started: None,
            template: "Best Settings".to_string(),
            settings,
            modules: vec![module],
            rendered_frames: Vec::new(),
        });
        Ok(self.render.items.len() - 1)
    }

    pub fn queue_item_comp(&self, index: usize) -> ScriptResult<ItemId> {
        Ok(self.rq(index)?.comp)
    }

    pub fn queue_log_type(&self, index: usize) -> ScriptResult<LogType> {
        Ok(self.rq(index)?.log_type)
    }

    pub fn set_queue_log_type(&mut self, index: usize, value: LogType) -> ScriptResult<()> {
        self.rq_mut(index)?.log_type = value;
        Ok(())
    }

    pub fn output_module_count(&self, index: usize) -> ScriptResult<i32> {
        Ok(self.rq(index)?.modules.len() as i32)
    }

    pub fn set_status_hook(&mut self, hook: Option<fn(RqItemStatus)>) {
        self.render.status_hook = hook;
    }

    pub fn queue_item_notify(&self, index: usize) -> ScriptResult<bool> {
        Ok(self.rq(index)?.notify)
    }

    pub fn set_queue_item_notify(&mut self, index: usize, value: bool) -> ScriptResult<()> {
        self.rq_mut(index)?.notify = value;
        Ok(())
    }

    pub fn queue_item_render_flag(&self, index: usize) -> ScriptResult<bool> {
        Ok(self.rq(index)?.render)
    }

    pub fn set_queue_item_render_flag(&mut self, index: usize, value: bool) -> ScriptResult<()> {
        self.rq_mut(index)?.render = value;
        Ok(())
    }

    pub fn skip_frames(&self, index: usize) -> ScriptResult<i32> {
        Ok(self.rq(index)?.skip_frames)
    }

    pub fn set_skip_frames(&mut self, index: usize, value: i32) -> ScriptResult<()> {
        if !(0..=99).contains(&value) {
            return Err(ScriptError::range("skipFrames"));
        }
        self.rq_mut(index)?.skip_frames = value;
        Ok(())
    }

    pub fn queue_elapsed(&self, index: usize) -> ScriptResult<i32> {
        Ok(self.rq(index)?.elapsed.unwrap_or(0))
    }

    pub fn queue_start_time(&self, index: usize) -> ScriptResult<Option<String>> {
        Ok(self.rq(index)?.started.clone())
    }

    pub fn queue_status(&self, index: usize) -> ScriptResult<RqItemStatus> {
        Ok(self.rq(index)?.status)
    }

    pub fn render_templates(&self, index: usize) -> ScriptResult<Vec<String>> {
        let _ = self.rq(index)?;
        Ok(self.render.render_templates.keys().cloned().collect())
    }

    pub fn time_span_duration(&self, index: usize) -> ScriptResult<f64> {
        Ok(self.rq(index)?.time_span_duration)
    }

    pub fn set_time_span_duration(&mut self, index: usize, value: f64) -> ScriptResult<()> {
        self.rq_mut(index)?.time_span_duration = value;
        Ok(())
    }

    pub fn time_span_start(&self, index: usize) -> ScriptResult<f64> {
        Ok(self.rq(index)?.time_span_start)
    }

    pub fn set_time_span_start(&mut self, index: usize, value: f64) -> ScriptResult<()> {
        self.rq_mut(index)?.time_span_start = value;
        Ok(())
    }

    pub fn apply_render_template(&mut self, index: usize, name: &str) -> ScriptResult<()> {
        let settings = self
            .render
            .render_templates
            .get(name)
            .cloned()
            .ok_or_else(|| ScriptError::missing("テンプレート"))?;
        let item = self.rq_mut(index)?;
        item.template = name.to_string();
        item.settings = settings;
        Ok(())
    }

    pub fn duplicate_queue_item(&mut self, index: usize) -> ScriptResult<usize> {
        let copy = self.rq(index)?.clone_item();
        self.render.items.push(copy);
        Ok(self.render.items.len() - 1)
    }

    pub fn queue_get_setting(&self, index: usize, key: &str) -> ScriptResult<String> {
        self.rq(index)?
            .settings
            .get(key)
            .cloned()
            .ok_or_else(|| ScriptError::unconfirmed(key))
    }

    pub fn queue_get_settings(&self, index: usize) -> ScriptResult<Vec<(String, String)>> {
        Ok(self.rq(index)?.settings.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    }

    pub fn queue_set_setting(&mut self, index: usize, key: &str, value: &str) -> ScriptResult<()> {
        if !self.rq(index)?.settings.contains_key(key) && key != "Quality" && key != "Resolution" {
            return Err(ScriptError::unconfirmed(key));
        }
        self.rq_mut(index)?.settings.insert(key.to_string(), value.to_string());
        Ok(())
    }

    pub fn queue_set_settings(&mut self, index: usize, settings: Vec<(String, String)>) -> ScriptResult<()> {
        for (key, value) in settings {
            self.queue_set_setting(index, &key, &value)?;
        }
        Ok(())
    }

    pub fn output_module(&self, item: usize, module: usize) -> ScriptResult<()> {
        let _ = self.module(item, module)?;
        Ok(())
    }

    pub fn remove_queue_item(&mut self, index: usize) -> ScriptResult<()> {
        if index >= self.render.items.len() {
            return Err(ScriptError::range("queue"));
        }
        self.render.items.remove(index);
        Ok(())
    }

    pub fn save_render_template(&mut self, index: usize, name: &str) -> ScriptResult<()> {
        let settings = self.rq(index)?.settings.clone();
        self.render.render_templates.insert(name.to_string(), settings);
        Ok(())
    }

    pub fn add_output_module(&mut self, index: usize) -> ScriptResult<usize> {
        self.rq_mut(index)?.modules.push(OutputMod {
            file: None,
            include_source_xmp: false,
            name: "Module".to_string(),
            post_render: ScriptPostRenderAction::None,
            template: String::new(),
            settings: {
                let mut map = std::collections::BTreeMap::new();
                map.insert("Format".to_string(), "PNG".to_string());
                map
            },
        });
        Ok(self.rq(index)?.modules.len() - 1)
    }

    pub fn output_file(&self, item: usize, module: usize) -> ScriptResult<Option<PathBuf>> {
        Ok(self.module(item, module)?.file.clone())
    }

    pub fn set_output_file(&mut self, item: usize, module: usize, file: PathBuf) -> ScriptResult<()> {
        self.module_mut(item, module)?.file = Some(file);
        Ok(())
    }

    pub fn include_source_xmp(&self, item: usize, module: usize) -> ScriptResult<bool> {
        Ok(self.module(item, module)?.include_source_xmp)
    }

    pub fn set_include_source_xmp(&mut self, item: usize, module: usize, value: bool) -> ScriptResult<()> {
        self.module_mut(item, module)?.include_source_xmp = value;
        Ok(())
    }

    pub fn output_name(&self, item: usize, module: usize) -> ScriptResult<String> {
        Ok(self.module(item, module)?.name.clone())
    }

    pub fn set_output_name(&mut self, item: usize, module: usize, name: &str) -> ScriptResult<()> {
        self.module_mut(item, module)?.name = name.to_string();
        Ok(())
    }

    pub fn post_render_action(&self, item: usize, module: usize) -> ScriptResult<ScriptPostRenderAction> {
        Ok(self.module(item, module)?.post_render)
    }

    pub fn set_post_render_action(
        &mut self,
        item: usize,
        module: usize,
        value: ScriptPostRenderAction,
    ) -> ScriptResult<()> {
        self.module_mut(item, module)?.post_render = value;
        Ok(())
    }

    pub fn output_templates(&self, item: usize, module: usize) -> ScriptResult<Vec<String>> {
        let _ = self.module(item, module)?;
        Ok(self.render.output_templates.keys().cloned().collect())
    }

    pub fn apply_output_template(&mut self, item: usize, module: usize, name: &str) -> ScriptResult<()> {
        let format = self
            .render
            .output_templates
            .get(name)
            .cloned()
            .ok_or_else(|| ScriptError::missing("出力テンプレート"))?;
        let module = self.module_mut(item, module)?;
        module.template = name.to_string();
        module.settings.insert("Format".to_string(), format);
        Ok(())
    }

    pub fn output_get_setting(&self, item: usize, module: usize, key: &str) -> ScriptResult<String> {
        self.module(item, module)?
            .settings
            .get(key)
            .cloned()
            .ok_or_else(|| ScriptError::unconfirmed(key))
    }

    pub fn output_get_settings(&self, item: usize, module: usize) -> ScriptResult<Vec<(String, String)>> {
        Ok(self
            .module(item, module)?
            .settings
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect())
    }

    pub fn output_set_setting(&mut self, item: usize, module: usize, key: &str, value: &str) -> ScriptResult<()> {
        if key != "Format" && !self.module(item, module)?.settings.contains_key(key) {
            return Err(ScriptError::unconfirmed(key));
        }
        self.module_mut(item, module)?.settings.insert(key.to_string(), value.to_string());
        Ok(())
    }

    pub fn output_set_settings(&mut self, item: usize, module: usize, settings: Vec<(String, String)>) -> ScriptResult<()> {
        for (key, value) in settings {
            self.output_set_setting(item, module, &key, &value)?;
        }
        Ok(())
    }

    pub fn remove_output_module(&mut self, item: usize, module: usize) -> ScriptResult<()> {
        let modules = &mut self.rq_mut(item)?.modules;
        if modules.len() <= 1 {
            return Err(ScriptError::invalid("最後の出力モジュールは消せない"));
        }
        if module >= modules.len() {
            return Err(ScriptError::range("output module"));
        }
        modules.remove(module);
        Ok(())
    }

    pub fn save_output_template(&mut self, item: usize, module: usize, name: &str) -> ScriptResult<()> {
        let format = self
            .module(item, module)?
            .settings
            .get("Format")
            .cloned()
            .unwrap_or_else(|| "PNG".to_string());
        self.render.output_templates.insert(name.to_string(), format);
        Ok(())
    }

    pub fn pause_rendering(&mut self) {
        self.render.rendering = false;
        self.render.paused = true;
        for item in &mut self.render.items {
            if item.status == RqItemStatus::Rendering {
                item.status = RqItemStatus::UserStopped;
            }
        }
    }

    pub fn stop_rendering(&mut self) {
        self.render.rendering = false;
        self.render.paused = false;
        for item in &mut self.render.items {
            if item.status == RqItemStatus::Rendering || item.status == RqItemStatus::Queued && item.render {
                if item.status != RqItemStatus::Done {
                    item.status = RqItemStatus::UserStopped;
                }
            }
        }
    }

    pub fn queue_in_ame(&mut self, _immediate: bool) -> ScriptResult<()> {
        Err(ScriptError::unsupported("queueInAME"))
    }

    pub fn render_queue(&mut self) -> ScriptResult<()> {
        self.render.rendering = true;
        let count = self.render.items.len();
        for index in 0..count {
            if !self.render.items[index].render {
                continue;
            }
            if self.render.paused {
                self.render.items[index].status = RqItemStatus::UserStopped;
                continue;
            }
            self.render.items[index].status = RqItemStatus::Rendering;
            self.render.items[index].started = Some("now".to_string());
            let comp = self.render.items[index].comp;
            let start = self.render.items[index].time_span_start;
            let duration = self.render.items[index].time_span_duration;
            let skip = self.render.items[index].skip_frames;
            let rate = self.require_comp(comp)?.frame_rate;
            let step = (skip + 1) as f64;
            let mut frame = (start * rate).round() as i32;
            let end = ((start + duration) * rate).round() as i32;
            let mut frames = Vec::new();
            while frame < end {
                let guide_only = false;
                if !guide_only {
                    frames.push(frame);
                }
                frame += step as i32;
            }
            let modules = self.render.items[index].modules.len();
            for module in 0..modules {
                if let Some(path) = self.render.items[index].modules[module].file.clone() {
                    let body = format!(
                        "OpenEffect render\nframes {}\n",
                        frames.iter().map(|frame| frame.to_string()).collect::<Vec<_>>().join(" ")
                    );
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    std::fs::write(&path, body).map_err(|error| ScriptError::invalid(error.to_string()))?;
                    let format = match self.render.items[index].modules[module]
                        .settings
                        .get("Format")
                        .map(String::as_str)
                    {
                        Some("PNG") => OutputFormat::Png,
                        Some("TIFF") => OutputFormat::Tiff,
                        _ => OutputFormat::Png,
                    };
                    let post = match self.render.items[index].modules[module].post_render {
                        ScriptPostRenderAction::None => PostRenderAction::None,
                        ScriptPostRenderAction::Import => PostRenderAction::Import,
                        ScriptPostRenderAction::ImportAndReplaceUsage => PostRenderAction::ImportAndReplace,
                        ScriptPostRenderAction::SetProxy => PostRenderAction::SetProxy,
                    };
                    let mut output = RenderOutput::new(comp, &path, format, self.bits);
                    output.post_render = post;
                    self.io_queue.push(output);
                    self.apply_post_render(index, module, &path)?;
                }
            }
            self.render.items[index].rendered_frames = frames;
            self.render.items[index].status = RqItemStatus::Done;
            self.render.items[index].elapsed = Some(1);
            if let Some(hook) = self.render.status_hook {
                hook(RqItemStatus::Done);
            }
        }
        self.render.rendering = false;
        Ok(())
    }

    fn apply_post_render(&mut self, _item: usize, module_index: usize, path: &std::path::Path) -> ScriptResult<()> {
        let action = self.render.items[_item].modules[module_index].post_render;
        match action {
            ScriptPostRenderAction::None => Ok(()),
            ScriptPostRenderAction::Import => {
                let _ = self.import_footage_file(path, &crate::values::ImportOptions::new(Some(path.to_path_buf())))?;
                Ok(())
            }
            ScriptPostRenderAction::ImportAndReplaceUsage => {
                let footage = self.import_footage_file(path, &crate::values::ImportOptions::new(Some(path.to_path_buf())))?;
                let comp = self.render.items[_item].comp;
                if let ItemBody::Comp(comp) = &self.item(comp)?.body {
                    let layers = comp.layers.clone();
                    for layer in layers {
                        if self.layer(layer)?.source_id().is_some() {
                            self.layer_mut(layer)?.set_source(footage);
                            break;
                        }
                    }
                }
                Ok(())
            }
            ScriptPostRenderAction::SetProxy => Ok(()),
        }
    }

    pub fn rendered_frames(&self, index: usize) -> ScriptResult<Vec<i32>> {
        Ok(self.rq(index)?.rendered_frames.clone())
    }

    fn rq(&self, index: usize) -> ScriptResult<&RqItem> {
        self.render.items.get(index).ok_or_else(|| ScriptError::range("queue"))
    }

    fn rq_mut(&mut self, index: usize) -> ScriptResult<&mut RqItem> {
        self.render.items.get_mut(index).ok_or_else(|| ScriptError::range("queue"))
    }

    fn module(&self, item: usize, module: usize) -> ScriptResult<&OutputMod> {
        self.rq(item)?
            .modules
            .get(module)
            .ok_or_else(|| ScriptError::range("output module"))
    }

    fn module_mut(&mut self, item: usize, module: usize) -> ScriptResult<&mut OutputMod> {
        self.rq_mut(item)?
            .modules
            .get_mut(module)
            .ok_or_else(|| ScriptError::range("output module"))
    }
}

impl RqItem {
    fn clone_item(&self) -> Self {
        Self {
            comp: self.comp,
            log_type: self.log_type,
            notify: self.notify,
            render: self.render,
            skip_frames: self.skip_frames,
            status: RqItemStatus::Queued,
            time_span_start: self.time_span_start,
            time_span_duration: self.time_span_duration,
            elapsed: Some(0),
            started: None,
            template: self.template.clone(),
            settings: self.settings.clone(),
            modules: self.modules.clone(),
            rendered_frames: Vec::new(),
        }
    }
}

impl Clone for OutputMod {
    fn clone(&self) -> Self {
        Self {
            file: self.file.clone(),
            include_source_xmp: self.include_source_xmp,
            name: self.name.clone(),
            post_render: self.post_render,
            template: self.template.clone(),
            settings: self.settings.clone(),
        }
    }
}
