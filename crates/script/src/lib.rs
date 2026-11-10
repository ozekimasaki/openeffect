//! OpenEffect のスクリプティングホスト。
//! ガイドのオブジェクト構成に対応する。Adobe のプラグイン SDK は使わない。

#![forbid(unsafe_code)]

mod catalog;
mod effect;
mod enums;
mod error;
mod json;
mod state;
mod template;
mod text;
mod values;
mod zip;

pub use effect::{
    oe_fx_checkout_input, oe_fx_comp_to_layer, oe_fx_convert_16_to_32, oe_fx_convert_32_to_8,
    oe_fx_convert_8_to_16, oe_fx_define_angle, oe_fx_define_checkbox, oe_fx_define_color,
    oe_fx_define_float, oe_fx_define_layer, oe_fx_define_point, oe_fx_define_popup, oe_fx_frame_16,
    oe_fx_frame_32, oe_fx_frame_32_with, oe_fx_frame_8, oe_fx_layer_to_comp, oe_fx_layer_to_world,
    oe_fx_param_count, oe_fx_param_get, oe_fx_release_input, oe_fx_render, oe_fx_set_param,
    oe_fx_set_param_key, oe_fx_world_to_layer, FxEffect,
};
pub use enums::*;
pub use error::{ScriptError, ScriptResult};
pub use model::{ItemId, LayerId, PropertyId};
pub use effect::{
    channel_one_from_16, channel_white_16_from_8, channel_white_8, copy_identity, oe_fx_checked_size,
    oe_fx_slider_visible,
};
pub use template::{
    oe_mogrt_list_json, oe_mogrt_read_controls, oe_mogrt_read_name, oe_template_controls,
    oe_template_export, oe_template_import, oe_template_manifest, TemplateControl, TemplatePackage,
};
pub use text::{CharacterRange, FontObject, LineRange, ParagraphRange, TextDocument};
pub use values::{
    Guide, GuideOptions, ImportOptions, MarkerValue, NumberedName, ScriptEase, ScriptFile,
    ScriptFolder, ShapeValue, Viewer, ViewerKind,
};

pub use state::App;

#[cfg(test)]
mod tests;
