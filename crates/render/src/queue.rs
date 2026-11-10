use io_api::{PostRenderAction, RenderOutput, RenderQueue};
use io_media::{import_paths, ImportPrefs, MediaProject};
use model::{Composition, ItemId, LayerKind};

use crate::output::OutputModule;
use crate::settings::{RenderSettings, RenderTemplate};
use crate::video::MOVIE_DEFAULT_NAME;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueStatus {
    Queued,
    Unqueued,
    WillContinue,
    Done,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderItem {
    pub comp: ItemId,
    pub settings: RenderSettings,
    pub modules: Vec<OutputModule>,
    pub status: QueueStatus,
    pub replace_target: Option<ItemId>,
}

#[derive(Clone, Debug)]
pub struct ProjectQueue {
    pub items: Vec<RenderItem>,
    pub default_template: RenderTemplate,
    pub default_output_name: String,
}

impl ProjectQueue {
    pub fn after_reset() -> Self {
        Self {
            items: Vec::new(),
            default_template: RenderTemplate::Best,
            default_output_name: MOVIE_DEFAULT_NAME.into(),
        }
    }

    pub fn add_comp(&mut self, comp: &Composition) -> usize {
        let settings = RenderSettings::from_template(self.default_template);
        let module = if self.default_output_name == MOVIE_DEFAULT_NAME {
            OutputModule::movie_default(format!("{}.mp4", comp.name), comp.width, comp.height)
        } else {
            OutputModule::high_quality(format!("{}.png", comp.name))
        };
        self.items.push(RenderItem {
            comp: comp.id,
            settings,
            modules: vec![module],
            status: QueueStatus::Queued,
            replace_target: None,
        });
        self.items.len() - 1
    }

    pub fn set_default_template(&mut self, template: RenderTemplate) {
        self.default_template = template;
    }

    pub fn set_default_output(&mut self, name: impl Into<String>) {
        self.default_output_name = name.into();
    }

    pub fn add_output(&mut self, index: usize, module: OutputModule) {
        self.items[index].modules.push(module);
    }

    pub fn remove_output(&mut self, item: usize, module: usize) -> Option<OutputModule> {
        if module < self.items[item].modules.len() {
            Some(self.items[item].modules.remove(module))
        } else {
            None
        }
    }

    pub fn duplicate_output(&mut self, item: usize, module: usize) {
        let mut copy = self.items[item].modules[module].clone();
        let stem = copy
            .path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("output");
        let extension = copy.extension();
        copy.path = copy.path.with_file_name(format!("{stem}_copy.{extension}"));
        self.items[item].modules.push(copy);
    }

    pub fn add_output_to_selected(&mut self, indices: &[usize], module: &OutputModule) {
        for index in indices {
            let mut copy = module.clone();
            copy.path = copy.path.with_file_name(format!("item{index}.{}", copy.extension()));
            self.items[*index].modules.push(copy);
        }
    }

    pub fn contract(&self) -> RenderQueue {
        let mut queue = RenderQueue::new();
        for item in &self.items {
            for module in &item.modules {
                queue.push(RenderOutput::new(
                    item.comp,
                    module.path.clone(),
                    module.format,
                    module.depth,
                ));
            }
        }
        queue
    }
}

pub fn pre_render(queue: &mut ProjectQueue, comp: &Composition, target: ItemId) -> usize {
    let index = queue.add_comp(comp);
    queue.items[index].modules[0].post_render = PostRenderAction::ImportAndReplace;
    queue.items[index].replace_target = Some(target);
    index
}

pub fn apply_post_render(
    media: &mut MediaProject,
    item: &RenderItem,
    module_index: usize,
) -> Result<(), io_media::MediaError> {
    let module = &item.modules[module_index];
    match module.post_render {
        PostRenderAction::None => Ok(()),
        PostRenderAction::Import => {
            let mut prefs = ImportPrefs::default();
            prefs.sequence.sequence = false;
            import_paths(media, &[module.path.clone()], &prefs).map(|_| ())
        }
        PostRenderAction::ImportAndReplace => {
            let before = item.replace_target;
            let mut prefs = ImportPrefs::default();
            prefs.sequence.sequence = false;
            let ids = import_paths(media, &[module.path.clone()], &prefs)?;
            if let (Some(from), Some(to)) = (before, ids.first().copied()) {
                retarget(&mut media.project, from, to);
            }
            Ok(())
        }
        PostRenderAction::SetProxy => {
            if let Some(target) = item.replace_target {
                if let Some(reference) = media.reference_mut(target) {
                    reference.set_proxy(io_api::Proxy::new(module.path.clone()));
                }
            }
            Ok(())
        }
    }
}

fn retarget(project: &mut model::Project, from: ItemId, to: ItemId) {
    for composition in &mut project.compositions {
        for layer in &mut composition.layers {
            match &mut layer.kind {
                LayerKind::Footage { source }
                | LayerKind::Solid { source }
                | LayerKind::Precomp { source }
                | LayerKind::Placeholder { source } => {
                    if *source == from {
                        *source = to;
                    }
                }
                LayerKind::Null
                | LayerKind::Adjustment
                | LayerKind::Camera { .. }
                | LayerKind::Light { .. }
                | LayerKind::Text
                | LayerKind::Shape
                | LayerKind::Guide => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::RenderTemplate;
    use io_api::OutputFormat;
    use io_media::{encode_still, EncodeOptions, MediaProject, Raster};
    use model::{BitsPerChannel, Composition, Layer, LayerKind, Time};
    use std::path::PathBuf;

    fn comp() -> Composition {
        Composition::new(ItemId::new(9), "Main", 16, 16, 24.0, Time::from_seconds(1.0))
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("openeffect-queue-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn adding_a_comp_creates_one_item_and_defaults_do_not_rewrite_it() {
        let mut queue = ProjectQueue::after_reset();
        let comp = comp();
        queue.add_comp(&comp);
        assert_eq!(queue.items.len(), 1);
        assert_eq!(queue.items[0].settings.template.name(), "Best Settings");
        assert_eq!(queue.items[0].modules[0].template_name, MOVIE_DEFAULT_NAME);
        queue.set_default_template(RenderTemplate::Draft);
        queue.set_default_output("High Quality");
        queue.add_comp(&comp);
        assert_eq!(queue.items[0].settings.template, RenderTemplate::Best);
        assert_eq!(queue.items[1].settings.template, RenderTemplate::Draft);
        assert_eq!(queue.items[1].modules[0].template_name, "High Quality");
        assert_eq!(queue.contract().len(), 2);
    }

    #[test]
    fn output_modules_can_be_added_removed_and_duplicated() {
        let mut queue = ProjectQueue::after_reset();
        let index = queue.add_comp(&comp());
        let mut wav = OutputModule::high_quality("sound.wav");
        wav.set_format(OutputFormat::Wav);
        queue.add_output(index, wav);
        assert_eq!(queue.items[0].modules.len(), 2);
        let removed = queue.remove_output(index, 1).unwrap();
        assert_eq!(removed.format, OutputFormat::Wav);
        queue.duplicate_output(index, 0);
        let original = &queue.items[0].modules[0];
        let copy = &queue.items[0].modules[1];
        assert_eq!(original.format, copy.format);
        assert_eq!(original.depth, copy.depth);
        assert_ne!(original.path, copy.path);
        queue.add_comp(&comp());
        let extra = OutputModule::high_quality("extra.png");
        queue.add_output_to_selected(&[0, 1], &extra);
        assert_eq!(queue.items[0].modules.len(), 3);
        assert_eq!(queue.items[1].modules.len(), 2);
    }

    #[test]
    fn post_render_import_replace_and_proxy() {
        let dir = temp_dir("post");
        let path = dir.join("out.png");
        let raster = Raster::solid(2, 2, BitsPerChannel::Bpc8, [0.0, 1.0, 0.0, 1.0]);
        encode_still(&path, &raster, &EncodeOptions::default()).unwrap();
        let mut media = MediaProject::new(BitsPerChannel::Bpc8);
        let source = media.project.alloc_item();
        media.project.footage.push(model::Footage::file(
            source,
            "source",
            2,
            2,
            Time::from_frames(1.0, 24.0),
            24.0,
        ));
        media.references.push(io_api::FootageReference::linked(source, "source.png"));
        let comp_id = media.project.alloc_item();
        let mut composition = Composition::new(comp_id, "Main", 2, 2, 24.0, Time::from_seconds(1.0));
        let layer_id = media.project.alloc_layer();
        composition.layers.push(Layer::new(layer_id, "clip", LayerKind::Footage { source }));
        media.project.add_composition(composition);

        let mut item = RenderItem {
            comp: comp_id,
            settings: RenderSettings::from_template(RenderTemplate::Best),
            modules: vec![OutputModule::high_quality(&path)],
            status: QueueStatus::Done,
            replace_target: Some(source),
        };
        let before = media.project.footage.len();
        item.modules[0].post_render = PostRenderAction::None;
        apply_post_render(&mut media, &item, 0).unwrap();
        assert_eq!(media.project.footage.len(), before);

        item.modules[0].post_render = PostRenderAction::Import;
        item.modules[0].path = path.clone();
        apply_post_render(&mut media, &item, 0).unwrap();
        assert!(media.project.footage.len() > before);

        item.modules[0].post_render = PostRenderAction::ImportAndReplace;
        apply_post_render(&mut media, &item, 0).unwrap();
        let layer = &media.project.composition(comp_id).unwrap().layers[0];
        match layer.kind {
            LayerKind::Footage { source: next } => assert_ne!(next, source),
            _ => panic!("footage"),
        }

        item.modules[0].post_render = PostRenderAction::SetProxy;
        item.replace_target = Some(source);
        apply_post_render(&mut media, &item, 0).unwrap();
        assert!(media.reference(source).unwrap().proxy.as_ref().is_some_and(|proxy| proxy.in_use));
    }

    #[test]
    fn pre_render_sets_import_and_replace() {
        let mut queue = ProjectQueue::after_reset();
        let index = pre_render(&mut queue, &comp(), ItemId::new(3));
        assert_eq!(queue.items[index].modules[0].post_render, PostRenderAction::ImportAndReplace);
        assert_eq!(queue.items[index].replace_target, Some(ItemId::new(3)));
    }
}
