//! ソーステキスト。文字範囲の書き込みは範囲内だけを変える。

use crate::enums::{
    AutoKernType, BaselineDirection, BoxAutoFitPolicy, BoxFirstBaselineAlignment,
    BoxVerticalAlignment, ComposerEngine, DigitSet, FontBaselineOption, FontCapsOption,
    LeadingType, LineJoinType, LineOrientation, ParagraphDirection, ParagraphJustification,
};
use crate::error::{ScriptError, ScriptResult};

#[derive(Clone, Debug, PartialEq)]
pub struct FontObject {
    pub post_script_name: String,
    pub family: String,
    pub style: String,
    pub location: String,
    pub locked: bool,
}

impl FontObject {
    pub fn from_postscript(name: impl Into<String>) -> Self {
        let post_script_name = name.into();
        let (family, style) = split_font(&post_script_name);
        Self {
            post_script_name,
            family,
            style,
            location: String::new(),
            locked: false,
        }
    }
}

fn split_font(name: &str) -> (String, String) {
    let trimmed = name.trim_end_matches("MT");
    if let Some((family, style)) = trimmed.rsplit_once('-') {
        (family.to_string(), style.to_string())
    } else if trimmed.is_empty() {
        ("Arial".to_string(), "Regular".to_string())
    } else {
        (trimmed.to_string(), "Regular".to_string())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharStyle {
    pub font: FontObject,
    pub size: f64,
    pub fill: [f64; 3],
    pub apply_fill: bool,
    pub stroke: [f64; 3],
    pub apply_stroke: bool,
    pub stroke_over_fill: bool,
    pub stroke_width: f64,
    pub faux_bold: bool,
    pub faux_italic: bool,
    pub baseline_shift: f64,
    pub horizontal_scale: f64,
    pub vertical_scale: f64,
    pub kerning: f64,
    pub tracking: f64,
    pub tsume: f64,
    pub caps: FontCapsOption,
    pub baseline: FontBaselineOption,
    pub no_break: bool,
    pub ligature: bool,
    pub line_join: LineJoinType,
}

impl Default for CharStyle {
    fn default() -> Self {
        Self {
            font: FontObject::from_postscript("ArialMT"),
            size: 36.0,
            fill: [1.0, 1.0, 1.0],
            apply_fill: true,
            stroke: [0.0, 0.0, 0.0],
            apply_stroke: false,
            stroke_over_fill: false,
            stroke_width: 1.0,
            faux_bold: false,
            faux_italic: false,
            baseline_shift: 0.0,
            horizontal_scale: 100.0,
            vertical_scale: 100.0,
            kerning: 0.0,
            tracking: 0.0,
            tsume: 0.0,
            caps: FontCapsOption::Normal,
            baseline: FontBaselineOption::Normal,
            no_break: false,
            ligature: false,
            line_join: LineJoinType::Miter,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParaStyle {
    pub justification: ParagraphJustification,
    pub start_indent: f64,
    pub end_indent: f64,
    pub first_line_indent: f64,
    pub space_before: f64,
    pub space_after: f64,
    pub auto_hyphenate: bool,
    pub hanging_roman: bool,
    pub leading: f64,
    pub auto_leading: bool,
    pub direction: ParagraphDirection,
    pub leading_type: LeadingType,
    pub every_line_composer: bool,
}

impl Default for ParaStyle {
    fn default() -> Self {
        Self {
            justification: ParagraphJustification::Left,
            start_indent: 0.0,
            end_indent: 0.0,
            first_line_indent: 0.0,
            space_before: 0.0,
            space_after: 0.0,
            auto_hyphenate: false,
            hanging_roman: false,
            leading: 36.0,
            auto_leading: true,
            direction: ParagraphDirection::LeftToRight,
            leading_type: LeadingType::Roman,
            every_line_composer: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextDocument {
    pub text: String,
    pub chars: Vec<CharStyle>,
    pub paragraphs: Vec<ParaStyle>,
    pub box_text: bool,
    pub box_pos: [f64; 2],
    pub box_size: [f64; 2],
    pub vertical: bool,
    pub line_orientation: LineOrientation,
    pub auto_kern: AutoKernType,
    pub baseline_direction: BaselineDirection,
    pub box_auto_fit: BoxAutoFitPolicy,
    pub box_first_baseline: BoxFirstBaselineAlignment,
    pub box_first_baseline_minimum: f64,
    pub box_inset: f64,
    pub box_vertical_alignment: BoxVerticalAlignment,
    pub composer: ComposerEngine,
    pub digit_set: DigitSet,
    pub applied: bool,
}

impl TextDocument {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let mut doc = Self {
            text: String::new(),
            chars: Vec::new(),
            paragraphs: vec![ParaStyle::default()],
            box_text: false,
            box_pos: [0.0, 0.0],
            box_size: [200.0, 100.0],
            vertical: false,
            line_orientation: LineOrientation::Horizontal,
            auto_kern: AutoKernType::Metric,
            baseline_direction: BaselineDirection::Horizontal,
            box_auto_fit: BoxAutoFitPolicy::None,
            box_first_baseline: BoxFirstBaselineAlignment::Ascent,
            box_first_baseline_minimum: 0.0,
            box_inset: 0.0,
            box_vertical_alignment: BoxVerticalAlignment::Top,
            composer: ComposerEngine::LatinCjk,
            digit_set: DigitSet::DefaultDigits,
            applied: false,
        };
        doc.set_text(text);
        doc
    }

    pub fn point(text: impl Into<String>) -> Self {
        let mut doc = Self::new(text);
        doc.box_text = false;
        doc.line_orientation = LineOrientation::Horizontal;
        doc.vertical = false;
        doc
    }

    pub fn box_text(text: impl Into<String>, width: f64, height: f64, vertical: bool) -> Self {
        let mut doc = Self::new(text);
        doc.box_text = true;
        doc.box_size = [width.max(1.0), height.max(1.0)];
        doc.vertical = vertical;
        doc.line_orientation = if vertical {
            LineOrientation::VerticalRightToLeft
        } else {
            LineOrientation::Horizontal
        };
        doc
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        let style = self.chars.first().cloned().unwrap_or_default();
        self.chars.resize(self.text.chars().count(), style);
        let paras = paragraph_spans(&self.text).len().max(1);
        let para = self.paragraphs.first().cloned().unwrap_or_default();
        self.paragraphs.resize(paras, para);
        self.applied = false;
    }

    pub fn point_text(&self) -> bool {
        !self.box_text
    }

    pub fn paragraph_count(&self) -> i32 {
        paragraph_spans(&self.text).len().max(1) as i32
    }

    pub fn composed_lines(&self) -> Vec<(usize, usize)> {
        if !self.applied && self.chars.is_empty() && self.text.is_empty() {
            return vec![];
        }
        let chars: Vec<char> = self.text.chars().collect();
        if chars.is_empty() {
            return vec![(0, 0)];
        }
        let size = self.chars.first().map(|c| c.size).unwrap_or(36.0);
        let scale = self
            .chars
            .first()
            .map(|c| c.horizontal_scale / 100.0)
            .unwrap_or(1.0);
        let advance = (size * scale).max(1.0);
        let width = if self.box_text {
            (self.box_size[0] - self.box_inset * 2.0).max(advance)
        } else {
            f64::MAX
        };
        let mut lines = Vec::new();
        let mut start = 0usize;
        let mut column = 0.0;
        for (index, ch) in chars.iter().enumerate() {
            if *ch == '\n' {
                lines.push((start, index + 1));
                start = index + 1;
                column = 0.0;
                continue;
            }
            if self.box_text && column + advance > width && index > start {
                lines.push((start, index));
                start = index;
                column = 0.0;
            }
            column += advance;
        }
        if start <= chars.len() {
            lines.push((start, chars.len()));
        }
        if lines.is_empty() {
            lines.push((0, chars.len()));
        }
        lines
    }

    pub fn composed_line_count(&self) -> i32 {
        self.composed_lines().len() as i32
    }

    pub fn baseline_locs(&self) -> Vec<f64> {
        let leading = self.paragraphs.first().map(|p| p.leading).unwrap_or(36.0);
        let align = self.box_first_baseline_minimum;
        self.composed_lines()
            .iter()
            .enumerate()
            .map(|(index, _)| align + leading * index as f64)
            .collect()
    }

    pub fn box_overflow(&self) -> bool {
        if !self.box_text {
            return false;
        }
        let lines = self.composed_line_count() as f64;
        let leading = self.paragraphs.first().map(|p| p.leading).unwrap_or(36.0);
        lines * leading > (self.box_size[1] - self.box_inset * 2.0).max(0.0)
    }

    pub fn all_caps(&self) -> bool {
        self.chars.first().is_some_and(|c| {
            matches!(
                c.caps,
                FontCapsOption::AllCaps | FontCapsOption::AllSmallCaps
            )
        })
    }

    pub fn small_caps(&self) -> bool {
        self.chars.first().is_some_and(|c| {
            matches!(
                c.caps,
                FontCapsOption::SmallCaps | FontCapsOption::AllSmallCaps
            )
        })
    }

    pub fn subscript(&self) -> bool {
        self.chars
            .first()
            .is_some_and(|c| c.baseline == FontBaselineOption::FauxSubscript)
    }

    pub fn superscript(&self) -> bool {
        self.chars
            .first()
            .is_some_and(|c| c.baseline == FontBaselineOption::FauxSuperscript)
    }

    pub fn set_subscript(&mut self, on: bool) {
        let baseline = if on {
            FontBaselineOption::FauxSubscript
        } else {
            FontBaselineOption::Normal
        };
        for ch in &mut self.chars {
            ch.baseline = baseline;
        }
    }

    pub fn set_superscript(&mut self, on: bool) {
        let baseline = if on {
            FontBaselineOption::FauxSuperscript
        } else {
            FontBaselineOption::Normal
        };
        for ch in &mut self.chars {
            ch.baseline = baseline;
        }
    }

    fn char_mut_all(&mut self, f: impl Fn(&mut CharStyle)) {
        if self.chars.is_empty() {
            self.chars.push(CharStyle::default());
        }
        for ch in &mut self.chars {
            f(ch);
        }
    }

    fn para_mut_all(&mut self, f: impl Fn(&mut ParaStyle)) {
        if self.paragraphs.is_empty() {
            self.paragraphs.push(ParaStyle::default());
        }
        for para in &mut self.paragraphs {
            f(para);
        }
    }

    pub fn set_fill_color(&mut self, rgb: [f64; 3]) {
        self.char_mut_all(|c| {
            c.fill = rgb;
            c.apply_fill = true;
        });
    }

    pub fn fill_color(&self) -> [f64; 3] {
        self.chars.first().map(|c| c.fill).unwrap_or([1.0, 1.0, 1.0])
    }

    pub fn set_stroke_color(&mut self, rgb: [f64; 3]) {
        self.char_mut_all(|c| {
            c.stroke = rgb;
            c.apply_stroke = true;
        });
    }

    pub fn stroke_color(&self) -> [f64; 3] {
        self.chars.first().map(|c| c.stroke).unwrap_or([0.0, 0.0, 0.0])
    }

    pub fn set_font(&mut self, name: &str) {
        let font = FontObject::from_postscript(name);
        self.char_mut_all(|c| c.font = font.clone());
    }

    pub fn font(&self) -> String {
        self.chars
            .first()
            .map(|c| c.font.post_script_name.clone())
            .unwrap_or_else(|| "ArialMT".to_string())
    }

    pub fn set_font_size(&mut self, size: f64) -> ScriptResult<()> {
        if !(0.1..=1296.0).contains(&size) {
            return Err(ScriptError::range("fontSize"));
        }
        self.char_mut_all(|c| c.size = size);
        Ok(())
    }

    pub fn font_size(&self) -> f64 {
        self.chars.first().map(|c| c.size).unwrap_or(36.0)
    }

    pub fn set_caps(&mut self, caps: FontCapsOption) {
        self.char_mut_all(|c| c.caps = caps);
    }

    pub fn set_faux_bold(&mut self, on: bool) {
        self.char_mut_all(|c| c.faux_bold = on);
    }

    pub fn set_faux_italic(&mut self, on: bool) {
        self.char_mut_all(|c| c.faux_italic = on);
    }

    pub fn set_baseline_shift(&mut self, value: f64) {
        self.char_mut_all(|c| c.baseline_shift = value);
    }

    pub fn set_horizontal_scale(&mut self, value: f64) {
        self.char_mut_all(|c| c.horizontal_scale = value);
    }

    pub fn set_vertical_scale(&mut self, value: f64) {
        self.char_mut_all(|c| c.vertical_scale = value);
    }

    pub fn set_kerning(&mut self, value: f64) {
        self.char_mut_all(|c| c.kerning = value);
    }

    pub fn set_tracking(&mut self, value: f64) {
        self.char_mut_all(|c| c.tracking = value);
    }

    pub fn set_tsume(&mut self, value: f64) {
        self.char_mut_all(|c| c.tsume = value);
    }

    pub fn set_stroke_width(&mut self, value: f64) -> ScriptResult<()> {
        if !(0.0..=1000.0).contains(&value) {
            return Err(ScriptError::range("strokeWidth"));
        }
        self.char_mut_all(|c| c.stroke_width = value);
        Ok(())
    }

    pub fn set_apply_fill(&mut self, on: bool) {
        self.char_mut_all(|c| c.apply_fill = on);
    }

    pub fn set_apply_stroke(&mut self, on: bool) {
        self.char_mut_all(|c| c.apply_stroke = on);
    }

    pub fn set_stroke_over_fill(&mut self, on: bool) {
        self.char_mut_all(|c| c.stroke_over_fill = on);
    }

    pub fn set_no_break(&mut self, on: bool) {
        self.char_mut_all(|c| c.no_break = on);
    }

    pub fn set_ligature(&mut self, on: bool) {
        self.char_mut_all(|c| c.ligature = on);
    }

    pub fn set_line_join(&mut self, join: LineJoinType) {
        self.char_mut_all(|c| c.line_join = join);
    }

    pub fn set_baseline_option(&mut self, option: FontBaselineOption) {
        self.char_mut_all(|c| c.baseline = option);
    }

    pub fn set_justification(&mut self, value: ParagraphJustification) {
        let value = if value == ParagraphJustification::Multiple {
            ParagraphJustification::Center
        } else {
            value
        };
        self.para_mut_all(|p| p.justification = value);
    }

    pub fn justification(&self) -> ParagraphJustification {
        let Some(first) = self.paragraphs.first() else {
            return ParagraphJustification::Left;
        };
        if self.paragraphs.iter().all(|p| p.justification == first.justification) {
            first.justification
        } else {
            ParagraphJustification::Multiple
        }
    }

    pub fn set_start_indent(&mut self, value: f64) {
        self.para_mut_all(|p| p.start_indent = value);
    }

    pub fn set_end_indent(&mut self, value: f64) {
        self.para_mut_all(|p| p.end_indent = value);
    }

    pub fn set_first_line_indent(&mut self, value: f64) {
        self.para_mut_all(|p| p.first_line_indent = value);
    }

    pub fn set_space_before(&mut self, value: f64) {
        self.para_mut_all(|p| p.space_before = value);
    }

    pub fn set_space_after(&mut self, value: f64) {
        self.para_mut_all(|p| p.space_after = value);
    }

    pub fn set_auto_hyphenate(&mut self, on: bool) {
        self.para_mut_all(|p| p.auto_hyphenate = on);
    }

    pub fn set_hanging_roman(&mut self, on: bool) {
        self.para_mut_all(|p| p.hanging_roman = on);
    }

    pub fn set_leading(&mut self, value: f64) {
        self.para_mut_all(|p| {
            p.leading = value;
            p.auto_leading = false;
        });
    }

    pub fn set_auto_leading(&mut self, on: bool) {
        self.para_mut_all(|p| p.auto_leading = on);
    }

    pub fn set_direction(&mut self, direction: ParagraphDirection) {
        self.para_mut_all(|p| p.direction = direction);
    }

    pub fn set_leading_type(&mut self, value: LeadingType) {
        self.para_mut_all(|p| p.leading_type = value);
    }

    pub fn set_every_line_composer(&mut self, on: bool) {
        self.para_mut_all(|p| p.every_line_composer = on);
    }

    pub fn set_line_orientation(&mut self, value: LineOrientation) {
        self.line_orientation = value;
        self.vertical = !matches!(value, LineOrientation::Horizontal);
    }

    pub fn set_box_size(&mut self, size: [f64; 2]) -> ScriptResult<()> {
        if size[0] < 1.0 || size[1] < 1.0 {
            return Err(ScriptError::range("boxTextSize"));
        }
        self.box_size = size;
        Ok(())
    }

    pub fn reset_char_style(&mut self) {
        let len = self.chars.len();
        self.chars = vec![CharStyle::default(); len.max(1)];
        if self.text.is_empty() {
            self.chars.clear();
        }
    }

    pub fn reset_paragraph_style(&mut self) {
        let len = self.paragraphs.len().max(1);
        self.paragraphs = vec![ParaStyle::default(); len];
    }

    pub fn mark_applied(&mut self) {
        self.applied = true;
    }

    pub fn character_range(&self, start: i32, end: Option<i32>) -> ScriptResult<CharacterRange> {
        let len = self.text.chars().count() as i32;
        if start < 0 || start > len {
            return Err(ScriptError::range("characterStart"));
        }
        let end = match end {
            Some(value) if value < 0 => len + value,
            Some(value) => value,
            None => len,
        };
        if end < start || end > len {
            return Err(ScriptError::range("characterEnd"));
        }
        Ok(CharacterRange {
            start: start as usize,
            end: end as usize,
            valid: true,
            doc: self.clone(),
        })
    }

    pub fn composed_line_character_indexes_at(&self, index: i32) -> ScriptResult<(usize, usize)> {
        if index < 0 {
            return Err(ScriptError::range("characterIndex"));
        }
        let index = index as usize;
        for (start, end) in self.composed_lines() {
            if index >= start && index < end.max(start + 1) {
                return Ok((start, end));
            }
        }
        Err(ScriptError::range("characterIndex"))
    }

    pub fn paragraph_character_indexes_at(&self, index: i32) -> ScriptResult<(usize, usize)> {
        if index < 0 {
            return Err(ScriptError::range("characterIndex"));
        }
        let index = index as usize;
        for (start, end) in paragraph_spans(&self.text) {
            if index >= start && index < end.max(start + 1) {
                return Ok((start, end));
            }
        }
        if self.text.is_empty() && index == 0 {
            return Ok((0, 0));
        }
        Err(ScriptError::range("characterIndex"))
    }

    pub fn composed_line_range(&self, start: i32, end: Option<i32>) -> ScriptResult<LineRange> {
        let len = self.composed_line_count();
        range_pair(start, end, len).map(|(s, e)| LineRange {
            start: s,
            end: e,
            valid: s <= e && e <= len as usize,
            doc: self.clone(),
        })
    }

    pub fn paragraph_range(&self, start: i32, end: Option<i32>) -> ScriptResult<ParagraphRange> {
        let len = self.paragraph_count();
        range_pair(start, end, len).map(|(s, e)| ParagraphRange {
            start: s,
            end: e,
            valid: s <= e && e <= len as usize,
            doc: self.clone(),
        })
    }

    pub fn replace_font(&mut self, from: &str, to: &str, no_font_locking: bool) -> bool {
        let mut changed = false;
        for ch in &mut self.chars {
            if ch.font.post_script_name == from && (no_font_locking || !ch.font.locked) {
                ch.font = FontObject::from_postscript(to);
                changed = true;
            }
        }
        changed
    }
}

fn range_pair(start: i32, end: Option<i32>, len: i32) -> ScriptResult<(usize, usize)> {
    if start < 0 {
        return Err(ScriptError::range("range start"));
    }
    let end = match end {
        Some(value) if value < 0 => len + value,
        Some(value) => value,
        None => len,
    };
    if end < start {
        return Err(ScriptError::range("range end"));
    }
    Ok((start as usize, end as usize))
}

pub fn paragraph_spans(text: &str) -> Vec<(usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return vec![(0, 0)];
    }
    let mut spans = Vec::new();
    let mut start = 0usize;
    for (index, ch) in chars.iter().enumerate() {
        if *ch == '\n' {
            spans.push((start, index + 1));
            start = index + 1;
        }
    }
    if start <= chars.len() {
        spans.push((start, chars.len()));
    }
    spans
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterRange {
    pub start: usize,
    pub end: usize,
    pub valid: bool,
    doc: TextDocument,
}

impl CharacterRange {
    pub fn character_start(&self) -> usize {
        self.start
    }

    pub fn character_end(&self) -> usize {
        self.end
    }

    pub fn is_range_valid(&self) -> bool {
        self.valid && self.end >= self.start
    }

    pub fn text(&self) -> String {
        self.doc.text.chars().skip(self.start).take(self.end - self.start).collect()
    }

    pub fn to_string_value(&self) -> String {
        self.text()
    }

    pub fn into_document(self) -> TextDocument {
        self.doc
    }

    pub fn document(&self) -> &TextDocument {
        &self.doc
    }

    fn each_char(&mut self, f: impl Fn(&mut CharStyle)) {
        for ch in self.doc.chars.iter_mut().skip(self.start).take(self.end - self.start) {
            f(ch);
        }
    }

    pub fn fill_color(&self) -> Option<[f64; 3]> {
        uniform(self.doc.chars.iter().skip(self.start).take(self.end - self.start).map(|c| c.fill))
    }

    pub fn set_fill_color(&mut self, rgb: [f64; 3]) {
        self.each_char(|c| {
            c.fill = rgb;
            c.apply_fill = true;
        });
    }

    pub fn stroke_color(&self) -> Option<[f64; 3]> {
        uniform(
            self.doc
                .chars
                .iter()
                .skip(self.start)
                .take(self.end - self.start)
                .map(|c| c.stroke),
        )
    }

    pub fn set_stroke_color(&mut self, rgb: [f64; 3]) {
        self.each_char(|c| {
            c.stroke = rgb;
            c.apply_stroke = true;
        });
    }

    pub fn stroke_over_fill(&self) -> Option<bool> {
        uniform_bool(self.span_chars().map(|c| c.stroke_over_fill))
    }

    pub fn set_stroke_over_fill(&mut self, on: bool) {
        self.each_char(|c| c.stroke_over_fill = on);
    }

    pub fn kerning(&self) -> Option<f64> {
        uniform(self.span_chars().map(|c| c.kerning))
    }

    pub fn set_kerning(&mut self, value: f64) {
        self.each_char(|c| c.kerning = value);
    }

    pub fn all_caps(&self) -> Option<bool> {
        uniform_bool(self.span_chars().map(|c| {
            matches!(c.caps, FontCapsOption::AllCaps | FontCapsOption::AllSmallCaps)
        }))
    }

    pub fn set_all_caps(&mut self, on: bool) {
        let caps = if on {
            FontCapsOption::AllCaps
        } else {
            FontCapsOption::Normal
        };
        self.each_char(|c| c.caps = caps);
    }

    pub fn set_font_size(&mut self, size: f64) -> ScriptResult<()> {
        if !(0.1..=1296.0).contains(&size) {
            return Err(ScriptError::range("fontSize"));
        }
        self.each_char(|c| c.size = size);
        Ok(())
    }

    pub fn font_size_uniform(&self) -> Option<f64> {
        uniform(self.span_chars().map(|c| c.size))
    }

    pub fn set_apply_fill(&mut self, on: bool) {
        self.each_char(|c| c.apply_fill = on);
    }

    pub fn set_apply_stroke(&mut self, on: bool) {
        self.each_char(|c| c.apply_stroke = on);
    }

    pub fn set_faux_bold(&mut self, on: bool) {
        self.each_char(|c| c.faux_bold = on);
    }

    pub fn set_faux_italic(&mut self, on: bool) {
        self.each_char(|c| c.faux_italic = on);
    }

    pub fn set_font(&mut self, name: &str) {
        let font = FontObject::from_postscript(name);
        self.each_char(|c| c.font = font.clone());
    }

    pub fn set_horizontal_scale(&mut self, value: f64) {
        self.each_char(|c| c.horizontal_scale = value);
    }

    pub fn set_vertical_scale(&mut self, value: f64) {
        self.each_char(|c| c.vertical_scale = value);
    }

    pub fn set_tracking(&mut self, value: f64) {
        self.each_char(|c| c.tracking = value);
    }

    pub fn set_tsume(&mut self, value: f64) {
        self.each_char(|c| c.tsume = value);
    }

    pub fn set_stroke_width(&mut self, value: f64) {
        self.each_char(|c| c.stroke_width = value);
    }

    pub fn set_baseline_shift(&mut self, value: f64) {
        self.each_char(|c| c.baseline_shift = value);
    }

    pub fn set_caps(&mut self, caps: FontCapsOption) {
        self.each_char(|c| c.caps = caps);
    }

    pub fn set_baseline_option(&mut self, option: FontBaselineOption) {
        self.each_char(|c| c.baseline = option);
    }

    pub fn set_no_break(&mut self, on: bool) {
        self.each_char(|c| c.no_break = on);
    }

    pub fn set_ligature(&mut self, on: bool) {
        self.each_char(|c| c.ligature = on);
    }

    pub fn set_subscript(&mut self, on: bool) {
        let baseline = if on {
            FontBaselineOption::FauxSubscript
        } else {
            FontBaselineOption::Normal
        };
        self.each_char(|c| c.baseline = baseline);
    }

    pub fn set_superscript(&mut self, on: bool) {
        let baseline = if on {
            FontBaselineOption::FauxSuperscript
        } else {
            FontBaselineOption::Normal
        };
        self.each_char(|c| c.baseline = baseline);
    }

    pub fn paste_from(&mut self, other: &CharacterRange) {
        let Some(style) = other.doc.chars.get(other.start).cloned() else {
            return;
        };
        self.each_char(|c| *c = style.clone());
    }

    pub fn reset_char_style(&mut self) {
        let def = CharStyle::default();
        self.each_char(|c| *c = def.clone());
    }

    pub fn reset_paragraph_style(&mut self) {
        let spans = paragraph_spans(&self.doc.text);
        for (index, (start, end)) in spans.into_iter().enumerate() {
            let overlaps = self.start < end && self.end > start;
            if overlaps {
                if let Some(para) = self.doc.paragraphs.get_mut(index) {
                    *para = ParaStyle::default();
                }
            }
        }
    }

    fn span_chars(&self) -> impl Iterator<Item = &CharStyle> {
        self.doc.chars.iter().skip(self.start).take(self.end - self.start)
    }

    pub fn char_style(&self, offset: usize) -> Option<&CharStyle> {
        self.doc.chars.get(self.start + offset)
    }
}

fn uniform<T: PartialEq + Copy>(mut iter: impl Iterator<Item = T>) -> Option<T> {
    let first = iter.next()?;
    if iter.all(|value| value == first) {
        Some(first)
    } else {
        None
    }
}

fn uniform_bool(iter: impl Iterator<Item = bool>) -> Option<bool> {
    uniform(iter)
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
    pub valid: bool,
    doc: TextDocument,
}

impl LineRange {
    pub fn is_range_valid(&self) -> bool {
        self.valid
    }

    pub fn character_start(&self) -> usize {
        self.doc
            .composed_lines()
            .get(self.start)
            .map(|span| span.0)
            .unwrap_or(0)
    }

    pub fn character_end(&self) -> usize {
        let lines = self.doc.composed_lines();
        if self.end == 0 {
            return 0;
        }
        lines.get(self.end - 1).map(|span| span.1).unwrap_or(0)
    }

    pub fn to_string_value(&self) -> String {
        let start = self.character_start();
        let end = self.character_end();
        self.doc.text.chars().skip(start).take(end.saturating_sub(start)).collect()
    }

    pub fn character_range(&self) -> ScriptResult<CharacterRange> {
        self.doc.character_range(self.character_start() as i32, Some(self.character_end() as i32))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParagraphRange {
    pub start: usize,
    pub end: usize,
    pub valid: bool,
    doc: TextDocument,
}

impl ParagraphRange {
    pub fn is_range_valid(&self) -> bool {
        self.valid
    }

    pub fn character_start(&self) -> usize {
        paragraph_spans(&self.doc.text)
            .get(self.start)
            .map(|span| span.0)
            .unwrap_or(0)
    }

    pub fn character_end(&self) -> usize {
        let spans = paragraph_spans(&self.doc.text);
        if self.end == 0 {
            return 0;
        }
        spans.get(self.end - 1).map(|span| span.1).unwrap_or(0)
    }

    pub fn to_string_value(&self) -> String {
        let start = self.character_start();
        let end = self.character_end();
        self.doc.text.chars().skip(start).take(end.saturating_sub(start)).collect()
    }

    pub fn character_range(&self) -> ScriptResult<CharacterRange> {
        self.doc
            .character_range(self.character_start() as i32, Some(self.character_end() as i32))
    }
}
