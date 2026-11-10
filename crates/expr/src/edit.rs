use model::{ItemId, LayerId, LayerKind, Property, PropertyBody, PropertyId};

use crate::eval::execute;
use crate::scene::{LayerError, Panel, Reveal, Scene};
use crate::{ExprFailure, Target};

#[derive(Clone, Debug, PartialEq)]
pub struct Banner {
    pub line: String,
    pub full_text: String,
    pub comp: ItemId,
    pub layer: LayerId,
    pub property: PropertyId,
    pub fix: Option<Reveal>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComponentDrop {
    pub index: usize,
    pub dimensions: usize,
}

impl Scene {
    pub fn visible_errors(&self) -> Vec<&LayerError> {
        if !self.show_banners {
            return Vec::new();
        }
        self.errors
            .iter()
            .filter(|error| expression_is_shown(self, error))
            .collect()
    }

    pub fn banners(&self, _panel: Panel, width: usize) -> Vec<Banner> {
        self.visible_errors()
            .into_iter()
            .map(|error| Banner {
                line: banner_line(&error.message, width, self.banner_open),
                full_text: error.message.clone(),
                comp: error.comp,
                layer: error.layer,
                property: error.property,
                fix: error.fix.clone(),
            })
            .collect()
    }

    pub fn current_banner(&self, panel: Panel, width: usize) -> Option<Banner> {
        let banners = self.banners(panel, width);
        banners
            .get(self.banner_index.min(banners.len().saturating_sub(1)))
            .cloned()
    }

    pub fn step_banner(&mut self, delta: i32) {
        let count = self.visible_errors().len();
        if count == 0 {
            self.banner_index = 0;
            return;
        }
        let next = self.banner_index as i32 + delta;
        self.banner_index = next.clamp(0, count as i32 - 1) as usize;
    }

    pub fn reveal_expression(&mut self, index: usize) -> Option<Reveal> {
        let reveal = {
            let error = self.visible_errors().into_iter().nth(index)?;
            Reveal {
                comp: error.comp,
                layer: error.layer,
                property: Some(error.property),
            }
        };
        self.open_comp = Some(reveal.comp);
        self.revealed = Some(reveal.clone());
        Some(reveal)
    }

    pub fn reveal_fix(&mut self, index: usize) -> Option<Reveal> {
        let reveal = {
            let error = self.visible_errors().into_iter().nth(index)?;
            error.fix.clone()?
        };
        self.open_comp = Some(reveal.comp);
        self.revealed = Some(reveal.clone());
        Some(reveal)
    }

    pub fn has_warning(&self, property: PropertyId) -> bool {
        self.errors.iter().any(|error| error.property == property)
    }

    pub fn show_expressions_with_errors(&mut self, layer: LayerId) -> Vec<PropertyId> {
        let ids: Vec<_> = self
            .errors
            .iter()
            .filter(|error| error.layer == layer)
            .map(|error| error.property)
            .collect();
        self.shown_properties = Some(ids.clone());
        ids
    }

    pub fn pickwhip(
        &mut self,
        comp: ItemId,
        layer: LayerId,
        properties: &[PropertyId],
        component: Option<ComponentDrop>,
    ) -> String {
        self.disambiguate_name(comp, layer);
        let reference = self.reference_text(comp, layer, properties);
        match component {
            None => reference,
            Some(component) => {
                let indexed = format!("{reference}[{}]", component.index);
                let parts = vec![indexed; component.dimensions];
                format!("[{}]", parts.join(", "))
            }
        }
    }

    pub fn insert_reference(existing: &str, caret: Option<usize>, reference: &str) -> String {
        match caret {
            None => reference.to_string(),
            Some(index) => {
                let index = index.min(existing.len());
                format!("{}{reference}{}", &existing[..index], &existing[index..])
            }
        }
    }

    pub fn rename_layer(&mut self, comp: ItemId, layer: LayerId, new_name: &str) {
        let old = self
            .layer(comp, layer)
            .map(|layer| layer.name.clone())
            .unwrap_or_default();
        let unique = self
            .project
            .composition(comp)
            .map(|composition| {
                composition
                    .layers
                    .iter()
                    .all(|item| item.id == layer || item.name != new_name)
            })
            .unwrap_or(false);
        if let Some(found) = self.layer_mut(comp, layer) {
            found.name = new_name.to_string();
        }
        if unique {
            let from = format!("layer(\"{old}\")");
            let to = format!("layer(\"{new_name}\")");
            rewrite_expressions(self, &from, &to);
            return;
        }
        let needle = format!("layer(\"{old}\")");
        let hits = expressions_containing(self, &needle);
        for (hit_comp, hit_layer, property, source) in hits {
            let target = Target {
                comp: hit_comp,
                layer: hit_layer,
                property,
                time: 0.0,
            };
            if let Err(error) = execute(self, target, &source, false) {
                self.record_error(target, error.message);
            }
        }
    }

    pub fn precompose(&mut self, comp: ItemId, layers: &[LayerId], name: &str) -> ItemId {
        let (width, height, fps, seconds) = self
            .project
            .composition(comp)
            .map(|composition| {
                (
                    composition.width,
                    composition.height,
                    composition.frame_rate,
                    composition.duration.seconds(),
                )
            })
            .unwrap_or((1920, 1080, 25.0, 10.0));
        let moved_names: Vec<String> = layers
            .iter()
            .filter_map(|layer| self.layer(comp, *layer).map(|layer| layer.name.clone()))
            .collect();
        let mut moved = Vec::new();
        if let Some(composition) = self.project.composition_mut(comp) {
            let mut stay = Vec::new();
            for layer in composition.layers.drain(..) {
                if layers.contains(&layer.id) {
                    moved.push(layer);
                } else {
                    stay.push(layer);
                }
            }
            composition.layers = stay;
        }
        let new_comp = self.add_comp(name, width, height, fps, seconds);
        if let Some(composition) = self.project.composition_mut(new_comp) {
            composition.layers = moved;
        }
        for layer in layers {
            self.layer_comp.insert(*layer, new_comp);
        }
        let _ = self.add_layer(comp, name, LayerKind::Precomp { source: new_comp });
        for moved_name in &moved_names {
            let needle = format!("layer(\"{moved_name}\")");
            let hits = expressions_containing(self, &needle);
            for (hit_comp, hit_layer, property, source) in hits {
                if hit_comp != comp {
                    continue;
                }
                let target = Target {
                    comp: hit_comp,
                    layer: hit_layer,
                    property,
                    time: 0.0,
                };
                if let Err(ExprFailure { message }) = execute(self, target, &source, false) {
                    self.record_error(target, message);
                    if let Some(moved) = layers.first().copied() {
                        self.set_fix(
                            property,
                            Reveal {
                                comp: new_comp,
                                layer: moved,
                                property: None,
                            },
                        );
                    }
                }
            }
        }
        new_comp
    }

    fn disambiguate_name(&mut self, comp: ItemId, layer: LayerId) {
        let Some(name) = self.layer(comp, layer).map(|layer| layer.name.clone()) else {
            return;
        };
        let duplicates = self
            .project
            .composition(comp)
            .map(|composition| {
                composition
                    .layers
                    .iter()
                    .filter(|item| item.name == name)
                    .count()
            })
            .unwrap_or(0);
        if duplicates < 2 {
            return;
        }
        let mut suffix = 2u32;
        let renamed = loop {
            let candidate = format!("{name} {suffix}");
            let taken = self
                .project
                .composition(comp)
                .map(|composition| composition.layers.iter().any(|item| item.name == candidate))
                .unwrap_or(false);
            if !taken {
                break candidate;
            }
            suffix += 1;
        };
        if let Some(found) = self.layer_mut(comp, layer) {
            found.name = renamed;
        }
    }

    fn reference_text(&self, comp: ItemId, layer: LayerId, properties: &[PropertyId]) -> String {
        let name = self
            .layer(comp, layer)
            .map(|layer| layer.name.clone())
            .unwrap_or_default();
        let mut text = format!("thisComp.layer(\"{name}\")");
        for id in properties {
            let Some(property) = self.property(comp, layer, *id) else {
                continue;
            };
            if self.compact_english {
                text.push('.');
                text.push_str(&property.match_name);
            } else {
                text.push_str(&format!("(\"{}\")", property.name));
            }
        }
        text
    }
}

fn banner_line(message: &str, width: usize, open: bool) -> String {
    if !open {
        return String::new();
    }
    let one_line: String = message.chars().filter(|ch| *ch != '\n').collect();
    one_line.chars().take(width).collect()
}

fn expression_is_shown(scene: &Scene, error: &LayerError) -> bool {
    match scene.expression_of(error.comp, error.layer, error.property) {
        Some((_, false)) => false,
        Some((_, true)) | None => true,
    }
}

fn rewrite_expressions(scene: &mut Scene, from: &str, to: &str) {
    let comp_ids: Vec<_> = scene
        .project
        .compositions
        .iter()
        .map(|comp| comp.id)
        .collect();
    for comp in comp_ids {
        let layer_ids: Vec<_> = scene
            .project
            .composition(comp)
            .map(|composition| composition.layers.iter().map(|layer| layer.id).collect())
            .unwrap_or_default();
        for layer in layer_ids {
            let props = value_properties(
                scene
                    .layer(comp, layer)
                    .map(|layer| layer.properties.as_slice()),
            );
            for id in props {
                if let Some(property) = scene.property_mut(comp, layer, id) {
                    if let PropertyBody::Value { expression, .. } = &mut property.body {
                        if let Some(source) = expression.as_mut() {
                            *source = source.replace(from, to);
                        }
                    }
                }
            }
        }
    }
}

fn expressions_containing(
    scene: &Scene,
    needle: &str,
) -> Vec<(ItemId, LayerId, PropertyId, String)> {
    let mut hits = Vec::new();
    for composition in &scene.project.compositions {
        for layer in &composition.layers {
            for id in value_properties(Some(&layer.properties)) {
                if let Some((source, _)) = scene.expression_of(composition.id, layer.id, id) {
                    if source.contains(needle) {
                        hits.push((composition.id, layer.id, id, source));
                    }
                }
            }
        }
    }
    hits
}

fn value_properties(properties: Option<&[Property]>) -> Vec<PropertyId> {
    let Some(properties) = properties else {
        return Vec::new();
    };
    let mut ids = Vec::new();
    for property in properties {
        match &property.body {
            PropertyBody::Value { .. } => ids.push(property.id),
            PropertyBody::Group { children } => ids.extend(value_properties(Some(children))),
        }
    }
    ids
}
