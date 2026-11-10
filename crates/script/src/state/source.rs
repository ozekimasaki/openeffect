use super::{App, SourceKind};
use crate::enums::*;
use crate::error::{ScriptError, ScriptResult};
use model::ItemId;

fn source<'a>(app: &'a App, id: ItemId, proxy: bool) -> ScriptResult<&'a super::SourceRec> {
    let footage = app.require_footage(id)?;
    if proxy {
        footage.proxy.as_ref().ok_or_else(|| ScriptError::missing("proxy"))
    } else {
        Ok(&footage.main)
    }
}

impl App {
    pub fn alpha_mode(&self, id: ItemId, proxy: bool) -> ScriptResult<AlphaMode> {
        Ok(source(self, id, proxy)?.alpha_mode)
    }

    pub fn set_alpha_mode(&mut self, id: ItemId, proxy: bool, value: AlphaMode) -> ScriptResult<()> {
        active_source_mut(self, id, proxy)?.alpha_mode = value;
        if value == AlphaMode::Straight || value == AlphaMode::Premultiplied {
            active_source_mut(self, id, proxy)?.has_alpha = true;
        }
        Ok(())
    }

    pub fn conform_frame_rate(&self, id: ItemId, proxy: bool) -> ScriptResult<f64> {
        Ok(source(self, id, proxy)?.conform_frame_rate)
    }

    pub fn set_conform_frame_rate(&mut self, id: ItemId, proxy: bool, value: f64) -> ScriptResult<()> {
        if !(0.0..=99.0).contains(&value) {
            return Err(ScriptError::range("conformFrameRate"));
        }
        let source = active_source_mut(self, id, proxy)?;
        source.conform_frame_rate = value;
        if value > 0.0 {
            source.frame_rate = value;
        } else {
            source.frame_rate = source.native_frame_rate;
        }
        Ok(())
    }

    pub fn display_frame_rate(&self, id: ItemId, proxy: bool) -> ScriptResult<f64> {
        Ok(source(self, id, proxy)?.frame_rate)
    }

    pub fn field_separation(&self, id: ItemId, proxy: bool) -> ScriptResult<FieldSeparationType> {
        Ok(source(self, id, proxy)?.field_separation)
    }

    pub fn set_field_separation(&mut self, id: ItemId, proxy: bool, value: FieldSeparationType) -> ScriptResult<()> {
        active_source_mut(self, id, proxy)?.field_separation = value;
        Ok(())
    }

    pub fn source_has_alpha(&self, id: ItemId, proxy: bool) -> ScriptResult<bool> {
        let source = source(self, id, proxy)?;
        Ok(source.has_alpha && !source.invert_alpha || source.has_alpha)
    }

    pub fn high_quality_field_separation(&self, id: ItemId, proxy: bool) -> ScriptResult<bool> {
        Ok(source(self, id, proxy)?.high_quality_fields)
    }

    pub fn set_high_quality_field_separation(&mut self, id: ItemId, proxy: bool, value: bool) -> ScriptResult<()> {
        active_source_mut(self, id, proxy)?.high_quality_fields = value;
        Ok(())
    }

    pub fn invert_alpha(&self, id: ItemId, proxy: bool) -> ScriptResult<bool> {
        Ok(source(self, id, proxy)?.invert_alpha)
    }

    pub fn set_invert_alpha(&mut self, id: ItemId, proxy: bool, value: bool) -> ScriptResult<()> {
        active_source_mut(self, id, proxy)?.invert_alpha = value;
        Ok(())
    }

    pub fn alpha_is_inverted(&self, id: ItemId, proxy: bool) -> ScriptResult<bool> {
        Ok(source(self, id, proxy)?.invert_alpha)
    }

    pub fn is_still(&self, id: ItemId, proxy: bool) -> ScriptResult<bool> {
        Ok(source(self, id, proxy)?.is_still)
    }

    pub fn source_loop(&self, id: ItemId, proxy: bool) -> ScriptResult<i32> {
        Ok(source(self, id, proxy)?.loops)
    }

    pub fn set_source_loop(&mut self, id: ItemId, proxy: bool, value: i32) -> ScriptResult<()> {
        if !(1..=9999).contains(&value) {
            return Err(ScriptError::range("loop"));
        }
        let source = active_source_mut(self, id, proxy)?;
        source.loops = value;
        source.duration = source.native_duration * value as f64;
        if source.is_still {
            source.duration = 0.0;
        }
        Ok(())
    }

    pub fn native_frame_rate(&self, id: ItemId, proxy: bool) -> ScriptResult<f64> {
        Ok(source(self, id, proxy)?.native_frame_rate)
    }

    pub fn premul_color(&self, id: ItemId, proxy: bool) -> ScriptResult<[f64; 3]> {
        Ok(source(self, id, proxy)?.premul_color)
    }

    pub fn set_premul_color(&mut self, id: ItemId, proxy: bool, color: [f64; 3]) -> ScriptResult<()> {
        active_source_mut(self, id, proxy)?.premul_color = color;
        Ok(())
    }

    pub fn remove_pulldown(&self, id: ItemId, proxy: bool) -> ScriptResult<PulldownPhase> {
        Ok(source(self, id, proxy)?.pulldown)
    }

    pub fn set_remove_pulldown(&mut self, id: ItemId, proxy: bool, value: PulldownPhase) -> ScriptResult<()> {
        active_source_mut(self, id, proxy)?.pulldown = value;
        Ok(())
    }

    pub fn guess_alpha_mode(&mut self, id: ItemId, proxy: bool) -> ScriptResult<()> {
        let has_alpha = source(self, id, proxy)?.has_alpha;
        let mode = if has_alpha {
            AlphaMode::Straight
        } else {
            AlphaMode::Ignore
        };
        self.set_alpha_mode(id, proxy, mode)
    }

    pub fn guess_pulldown(&mut self, id: ItemId, proxy: bool, _method: Option<PulldownMethod>) -> ScriptResult<()> {
        let rate = source(self, id, proxy)?.native_frame_rate;
        let phase = if (rate - 23.976).abs() < 0.1 {
            PulldownPhase::Wssww
        } else {
            PulldownPhase::Off
        };
        self.set_remove_pulldown(id, proxy, phase)
    }

    pub fn replace_footage(&mut self, id: ItemId, file: &std::path::Path) -> ScriptResult<()> {
        let _ = self.require_footage(id)?;
        let info = std::fs::read(file).ok().as_deref().and_then(crate::values::png_info);
        let (width, height, alpha) = info.unwrap_or((1920, 1080, false));
        let source = active_source_mut(self, id, false)?;
        source.path = Some(file.to_path_buf());
        source.kind = SourceKind::File;
        source.width = width;
        source.height = height;
        source.has_alpha = alpha;
        source.missing = !file.exists();
        source.name = file.file_name().and_then(|name| name.to_str()).unwrap_or("footage").to_string();
        Ok(())
    }

    pub fn replace_with_placeholder(
        &mut self,
        id: ItemId,
        name: &str,
        width: i32,
        height: i32,
        frame_rate: f64,
        duration: f64,
    ) -> ScriptResult<()> {
        let source = active_source_mut(self, id, false)?;
        source.kind = SourceKind::Placeholder;
        source.name = name.to_string();
        source.width = width as u32;
        source.height = height as u32;
        source.frame_rate = frame_rate;
        source.duration = duration;
        source.missing = true;
        source.path = None;
        let item = self.item_mut(id)?;
        item.name = name.to_string();
        Ok(())
    }

    pub fn replace_with_sequence(&mut self, id: ItemId, file: &std::path::Path, force_alphabetical: bool) -> ScriptResult<()> {
        let mut options = crate::values::ImportOptions::new(Some(file.to_path_buf()));
        options.sequence = true;
        options.force_alphabetical = force_alphabetical;
        let paths = super::item::sequence_paths(file, &options);
        let source = active_source_mut(self, id, false)?;
        source.kind = SourceKind::Sequence;
        source.sequence = true;
        source.path = Some(file.to_path_buf());
        source.sequence_paths = paths;
        source.is_still = false;
        source.frame_rate = if source.frame_rate <= 0.0 { 30.0 } else { source.frame_rate };
        source.duration = source.sequence_paths.len().max(1) as f64 / source.frame_rate;
        Ok(())
    }

    pub fn replace_with_solid(
        &mut self,
        id: ItemId,
        color: [f64; 3],
        name: &str,
        width: i32,
        height: i32,
        pixel_aspect: f64,
    ) -> ScriptResult<()> {
        let bits = self.bits;
        let source = active_source_mut(self, id, false)?;
        source.kind = SourceKind::Solid;
        source.color = color;
        source.name = name.to_string();
        source.width = width as u32;
        source.height = height as u32;
        source.pixel_aspect = pixel_aspect;
        source.path = None;
        source.missing = false;
        source.is_still = true;
        let width = source.width;
        let height = source.height;
        self.require_footage_mut(id)?.eval = super::item::solid_frame(bits, width, height, color);
        self.item_mut(id)?.name = name.to_string();
        Ok(())
    }

    pub fn sequence_frame_rate(&self, id: ItemId) -> ScriptResult<f64> {
        Ok(self.require_footage(id)?.main.frame_rate)
    }

    pub fn footage_file(&self, id: ItemId) -> ScriptResult<Option<std::path::PathBuf>> {
        let footage = self.require_footage(id)?;
        if footage.main.kind == SourceKind::Solid {
            return Ok(None);
        }
        Ok(footage.main.path.clone())
    }
}

pub(super) fn active_source_mut(app: &mut App, id: ItemId, proxy: bool) -> ScriptResult<&mut super::SourceRec> {
    let footage = app.require_footage_mut(id)?;
    if proxy {
        footage.proxy.as_mut().ok_or_else(|| ScriptError::missing("proxy"))
    } else {
        Ok(&mut footage.main)
    }
}
