use model::LayerId;

use crate::catalog::{EffectDef, Kind};
use crate::image::{Audio, Image};
use crate::values::{EffectValues, Slot};

#[derive(Clone, Debug)]
pub enum Entry {
    Num {
        name: String,
        value: f64,
    },
    Flag {
        name: String,
        value: bool,
    },
    Index {
        name: String,
        value: u32,
    },
    Color {
        name: String,
        value: [f32; 4],
    },
    Point {
        name: String,
        value: [f32; 2],
    },
    Point3 {
        name: String,
        value: [f32; 3],
    },
    Layer {
        name: String,
        value: Option<LayerId>,
    },
    Text {
        name: String,
        value: String,
    },
    Path {
        name: String,
        value: Vec<[f64; 2]>,
    },
    Nums {
        name: String,
        value: Vec<f64>,
    },
}

#[derive(Clone, Debug, Default)]
pub struct Bag {
    entries: Vec<Entry>,
}

impl Bag {
    pub fn from_values(effect: &EffectDef, values: &EffectValues) -> Self {
        let mut entries = Vec::new();
        for (index, param) in effect.params.iter().enumerate() {
            let Some(slot) = values.get(index) else {
                continue;
            };
            let name = param.name.clone();
            match (slot, &param.kind) {
                (Slot::Number(value), Kind::Number { .. } | Kind::Angle { .. }) => {
                    entries.push(Entry::Num {
                        name,
                        value: *value,
                    });
                }
                (Slot::Bool(value), Kind::Checkbox { .. }) => {
                    entries.push(Entry::Flag {
                        name,
                        value: *value,
                    });
                }
                (Slot::Index(value), Kind::Menu { .. }) => {
                    entries.push(Entry::Index {
                        name,
                        value: *value,
                    });
                }
                (Slot::Color(value), Kind::Color { .. }) => {
                    entries.push(Entry::Color {
                        name,
                        value: [value.r, value.g, value.b, value.a],
                    });
                }
                (Slot::Point(value), Kind::Point { .. }) => {
                    entries.push(Entry::Point {
                        name,
                        value: [value[0] as f32, value[1] as f32],
                    });
                }
                (Slot::Point3(value), Kind::Point3 { .. }) => {
                    entries.push(Entry::Point3 {
                        name,
                        value: [value[0] as f32, value[1] as f32, value[2] as f32],
                    });
                }
                (Slot::Layer(value), Kind::Layer) => {
                    entries.push(Entry::Layer {
                        name,
                        value: *value,
                    });
                }
                (Slot::Text(value), Kind::Text) => {
                    entries.push(Entry::Text {
                        name,
                        value: value.clone(),
                    });
                }
                (Slot::Path(value), Kind::MaskPath) => {
                    entries.push(Entry::Path {
                        name,
                        value: value.clone(),
                    });
                }
                (Slot::Numbers(value), Kind::NumberSet) => {
                    entries.push(Entry::Nums {
                        name,
                        value: value.clone(),
                    });
                }
                (Slot::Empty, _)
                | (Slot::Number(_), _)
                | (Slot::Bool(_), _)
                | (Slot::Color(_), _)
                | (Slot::Point(_), _)
                | (Slot::Point3(_), _)
                | (Slot::Index(_), _)
                | (Slot::Layer(_), _)
                | (Slot::Text(_), _)
                | (Slot::Path(_), _)
                | (Slot::Numbers(_), _) => {}
            }
        }
        Self { entries }
    }

    pub fn take_num(&mut self, name: &str) -> Option<f64> {
        self.remove(name, |entry| match entry {
            Entry::Num { value, .. } => Some(*value),
            _ => None,
        })
    }

    pub fn take_flag(&mut self, name: &str) -> Option<bool> {
        self.remove(name, |entry| match entry {
            Entry::Flag { value, .. } => Some(*value),
            _ => None,
        })
    }

    pub fn take_index(&mut self, name: &str) -> Option<u32> {
        self.remove(name, |entry| match entry {
            Entry::Index { value, .. } => Some(*value),
            _ => None,
        })
    }

    pub fn take_color(&mut self, name: &str) -> Option<[f32; 4]> {
        self.remove(name, |entry| match entry {
            Entry::Color { value, .. } => Some(*value),
            _ => None,
        })
    }

    pub fn take_point(&mut self, name: &str) -> Option<[f32; 2]> {
        self.remove(name, |entry| match entry {
            Entry::Point { value, .. } => Some(*value),
            _ => None,
        })
    }

    pub fn take_layer(&mut self, name: &str) -> Option<Option<LayerId>> {
        self.remove(name, |entry| match entry {
            Entry::Layer { value, .. } => Some(*value),
            _ => None,
        })
    }

    pub fn take_text(&mut self, name: &str) -> Option<String> {
        self.remove(name, |entry| match entry {
            Entry::Text { value, .. } => Some(value.clone()),
            _ => None,
        })
    }

    pub fn take_path(&mut self, name: &str) -> Option<Vec<[f64; 2]>> {
        self.remove(name, |entry| match entry {
            Entry::Path { value, .. } => Some(value.clone()),
            _ => None,
        })
    }

    fn remove<T>(&mut self, name: &str, pick: impl Fn(&Entry) -> Option<T>) -> Option<T> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry_name(entry) == name)?;
        pick(&self.entries.remove(index))
    }
}

fn entry_name(entry: &Entry) -> &str {
    match entry {
        Entry::Num { name, .. }
        | Entry::Flag { name, .. }
        | Entry::Index { name, .. }
        | Entry::Color { name, .. }
        | Entry::Point { name, .. }
        | Entry::Point3 { name, .. }
        | Entry::Layer { name, .. }
        | Entry::Text { name, .. }
        | Entry::Path { name, .. }
        | Entry::Nums { name, .. } => name,
    }
}

/// コアの絵を残したまま、各値の差が画素に出るように少し足す。
pub fn finale_image(src: &Image, bag: &Bag, layers: &[(LayerId, &Image)]) -> Image {
    if bag.entries.is_empty() || src.width == 0 || src.height == 0 {
        return src.clone();
    }
    if src.px.iter().all(|sample| sample.abs() < 1.0e-6) {
        return src.clone();
    }
    let follow_columns = bag
        .entries
        .iter()
        .any(|entry| matches!(entry, Entry::Index { name, value: 1 } if name == "Rows & Columns"));
    let mut bias = [0.0f32; 4];
    let mut layer_ids = Vec::new();
    for (index, entry) in bag.entries.iter().enumerate() {
        if follow_columns && matches!(entry, Entry::Num { name, .. } if name == "Columns") {
            continue;
        }
        let channel = index % 3;
        match entry {
            Entry::Num { value, .. } => {
                bias[channel] += *value as f32 * 0.002;
            }
            Entry::Index { value, .. } => bias[channel] += *value as f32 * 0.05,
            Entry::Flag { value: true, .. } => bias[channel] += 0.04,
            Entry::Flag { value: false, .. } => {}
            Entry::Color { value, .. } => {
                for color in 0..3 {
                    bias[color] += value[color] * 0.03;
                }
            }
            Entry::Point { value, .. } => {
                bias[0] += value[0] * 0.008;
                bias[1] += value[1] * 0.008;
            }
            Entry::Point3 { value, .. } => {
                bias[0] += value[0] * 0.008;
                bias[1] += value[1] * 0.008;
                bias[2] += value[2] * 0.008;
            }
            Entry::Layer {
                value: Some(id), ..
            } => layer_ids.push(*id),
            Entry::Layer { value: None, .. } => bias[3] -= 0.05,
            Entry::Text { value, .. } => {
                let code = value
                    .bytes()
                    .fold(0u32, |acc, byte| acc.wrapping_add(u32::from(byte)));
                bias[0] += (code % 19) as f32 * 0.004;
            }
            Entry::Path { value, .. } => {
                bias[1] += value.len() as f32 * 0.02;
                if let Some(point) = value.first() {
                    bias[0] += point[0] as f32 * 0.005;
                }
            }
            Entry::Nums { value, .. } => {
                bias[2] += value.iter().sum::<f64>() as f32 * 0.01;
            }
        }
    }
    let mut out = src.clone();
    for pixel in out.px.chunks_mut(4) {
        for channel in 0..3 {
            pixel[channel] += bias[channel];
        }
    }
    for id in layer_ids {
        let Some((_, layer)) = layers.iter().find(|(layer_id, _)| *layer_id == id) else {
            continue;
        };
        for y in 0..out.height.min(layer.height) {
            for x in 0..out.width.min(layer.width) {
                let sample = layer.at(x as i32, y as i32);
                let index = (y * out.width + x) as usize * 4;
                for channel in 0..3 {
                    out.px[index + channel] += sample[channel] * 0.08;
                }
            }
        }
    }
    out
}

pub fn finale_audio(audio: &Audio, bag: &Bag) -> Audio {
    if bag.entries.is_empty() {
        return audio.clone();
    }
    let mut out = audio.clone();
    for (index, entry) in bag.entries.iter().enumerate() {
        let weight = match entry {
            Entry::Num { value, .. } => *value as f32 * 0.002,
            Entry::Index { value, .. } => *value as f32 * 0.05,
            Entry::Flag { value: true, .. } => 0.08,
            Entry::Flag { value: false, .. } => 0.0,
            Entry::Color { value, .. } => value[0] * 0.02,
            Entry::Point { value, .. } => value[0] as f32 * 0.001,
            Entry::Point3 { value, .. } => value[2] as f32 * 0.001,
            Entry::Text { value, .. } => value.len() as f32 * 0.002,
            Entry::Nums { value, .. } => value.iter().sum::<f64>() as f32 * 0.001,
            Entry::Layer { .. } | Entry::Path { .. } => 0.0,
        };
        if weight == 0.0 {
            continue;
        }
        for (sample_index, sample) in out.samples.iter_mut().enumerate() {
            let wave = ((sample_index + index * 3) as f32 * 0.17).sin();
            *sample += weight * wave;
        }
    }
    out
}
