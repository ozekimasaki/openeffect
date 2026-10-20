use std::collections::BTreeMap;

/// 拡張子ごとの初期フレームレート。
/// After Effects の 4 文字コード文法は要確認なので、OpenEffect は `拡張子 fps` の行を読む。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InterpretationRules {
    frame_rates: BTreeMap<String, f64>,
}

impl InterpretationRules {
    pub fn frame_rate(&self, extension: &str) -> Option<f64> {
        self.frame_rates
            .get(&extension.trim_start_matches('.').to_ascii_lowercase())
            .copied()
    }
}

pub fn parse_rules(text: &str) -> InterpretationRules {
    let mut frame_rates = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(extension) = parts.next() else {
            continue;
        };
        let Some(rate) = parts.next().and_then(|value| value.parse::<f64>().ok()) else {
            continue;
        };
        if rate > 0.0 && rate <= crate::sequence::MAX_SEQUENCE_FPS {
            frame_rates.insert(extension.trim_start_matches('.').to_ascii_lowercase(), rate);
        }
    }
    InterpretationRules { frame_rates }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_file_sets_frame_rate_for_new_imports() {
        let rules = parse_rules("# comment\npng 24\n");
        assert_eq!(rules.frame_rate("png"), Some(24.0));
        assert_eq!(rules.frame_rate("PNG"), Some(24.0));
    }
}
