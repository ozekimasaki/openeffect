use crate::color::Rgba;
use crate::mask::MaskMode;
use crate::shape::transform_points;

/// ポイントテキストは折り返さない。段落テキストはボックス幅で折り返す。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextLayout {
    Point,
    Paragraph { width: f64, height: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum TextOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// 横書きの左・中央・右は、縦書きの上・中央・下と同じ値。初期値は要確認。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ParagraphAlignment {
    #[default]
    Left,
    Center,
    Right,
    JustifyLastLeft,
    JustifyLastRight,
    JustifyForced,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlignCommand {
    Start,
    Center,
    End,
    JustifyLastLeft,
    JustifyLastRight,
    JustifyForced,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlignEdge {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointJustify;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kerning {
    Metrics,
    Optical,
    Manual(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextLineJoin {
    Miter,
    Round,
    Bevel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StrokeFillOrder {
    StrokeOverFill,
    FillOverStroke,
    AllStrokesOverAllFills,
    AllFillsOverAllStrokes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LeadingMeasure {
    TopToTop,
    BottomToBottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Composer {
    SingleLine,
    EveryLine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpacingDirection {
    Tighten,
    Expand,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DigitSet {
    pub raw: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BaselineDirection {
    pub raw: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterStyle {
    pub font_family: String,
    pub font_style: String,
    pub size: Option<f64>,
    pub fill: Option<Rgba>,
    pub apply_fill: Option<bool>,
    pub stroke: Option<Rgba>,
    pub apply_stroke: Option<bool>,
    pub stroke_width: Option<f64>,
    pub faux_bold: bool,
    pub faux_italic: bool,
    pub tracking: f64,
    pub kerning: Kerning,
    pub horizontal_scale: f64,
    pub vertical_scale: f64,
    pub baseline_shift: f64,
    pub all_caps: bool,
    pub small_caps: bool,
    pub superscript: bool,
    pub subscript: bool,
    pub tsume: Option<f64>,
    pub no_break: bool,
    pub tate_chu_yoko: bool,
    pub digit_set: DigitSet,
    pub baseline_direction: BaselineDirection,
    pub line_join: Option<TextLineJoin>,
    pub stroke_fill: Option<StrokeFillOrder>,
}

impl CharacterStyle {
    pub fn new() -> Self {
        Self {
            font_family: String::new(),
            font_style: String::new(),
            size: None,
            fill: None,
            apply_fill: None,
            apply_stroke: None,
            stroke: None,
            stroke_width: None,
            faux_bold: false,
            faux_italic: false,
            tracking: 0.0,
            kerning: Kerning::Metrics,
            horizontal_scale: 100.0,
            vertical_scale: 100.0,
            baseline_shift: 0.0,
            all_caps: false,
            small_caps: false,
            superscript: false,
            subscript: false,
            tsume: None,
            no_break: false,
            tate_chu_yoko: false,
            digit_set: DigitSet::default(),
            baseline_direction: BaselineDirection::default(),
            line_join: None,
            stroke_fill: None,
        }
    }
}

impl Default for CharacterStyle {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParagraphStyle {
    pub alignment: ParagraphAlignment,
    pub left_indent: f64,
    pub right_indent: f64,
    pub first_line_indent: f64,
    pub space_before: f64,
    pub space_after: f64,
    pub hanging_roman: bool,
    pub composer: Option<Composer>,
    pub leading: f64,
    pub leading_measure: Option<LeadingMeasure>,
    pub auto_leading: bool,
}

impl ParagraphStyle {
    pub fn new() -> Self {
        Self {
            alignment: ParagraphAlignment::Left,
            left_indent: 0.0,
            right_indent: 0.0,
            first_line_indent: 0.0,
            space_before: 0.0,
            space_after: 0.0,
            hanging_roman: false,
            composer: None,
            leading: 0.0,
            leading_measure: None,
            auto_leading: false,
        }
    }
}

impl Default for ParagraphStyle {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub font_family: String,
    pub font_style: String,
    pub size: f64,
    pub fill: Rgba,
    pub faux_bold: bool,
    pub faux_italic: bool,
    pub alignment: ParagraphAlignment,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: String::new(),
            font_style: String::new(),
            size: 0.0,
            fill: Rgba::white(),
            faux_bold: false,
            faux_italic: false,
            alignment: ParagraphAlignment::Left,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceTextKey {
    pub time_seconds: f64,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PathOptions {
    pub mask_index: Option<usize>,
    pub reverse: Option<bool>,
    pub perpendicular: Option<bool>,
    pub force_alignment: bool,
    pub first_margin: f64,
    pub last_margin: f64,
}

impl PathOptions {
    pub fn new() -> Self {
        Self {
            mask_index: None,
            reverse: None,
            perpendicular: None,
            force_alignment: false,
            first_margin: 0.0,
            last_margin: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnchorGroup {
    Character,
    Word,
    Line,
    Block,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoreOptions {
    pub anchor_group: Option<AnchorGroup>,
    pub group_alignment: [f64; 2],
    pub character_blend_enabled: bool,
    pub fill_and_stroke_enabled: bool,
}

impl MoreOptions {
    pub fn new() -> Self {
        Self {
            anchor_group: None,
            group_alignment: [0.0, 0.0],
            character_blend_enabled: true,
            fill_and_stroke_enabled: true,
        }
    }
}

/// ソーステキスト。書式の直接アニメーションはできず、ホールドのキーで文字と書式を変える。
#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub source: String,
    pub layout: TextLayout,
    pub orientation: TextOrientation,
    pub style: TextStyle,
    pub characters: Vec<CharacterStyle>,
    pub paragraphs: Vec<ParagraphStyle>,
    pub typing: CharacterStyle,
    pub source_keys: Vec<SourceTextKey>,
    pub path_options: PathOptions,
    pub more_options: MoreOptions,
    pub per_character_3d: bool,
    pub layer_is_3d: bool,
    pub smart_quotes: Option<bool>,
}

impl Text {
    pub fn point(source: impl Into<String>) -> Self {
        Self::build(source.into(), TextLayout::Point)
    }

    pub fn paragraph(source: impl Into<String>, width: f64, height: f64) -> Self {
        Self::build(source.into(), TextLayout::Paragraph { width, height })
    }

    fn build(source: String, layout: TextLayout) -> Self {
        let count = source.chars().count();
        Self {
            source,
            layout,
            orientation: TextOrientation::Horizontal,
            style: TextStyle::default(),
            characters: vec![CharacterStyle::new(); count],
            paragraphs: vec![ParagraphStyle::new()],
            typing: CharacterStyle::new(),
            source_keys: Vec::new(),
            path_options: PathOptions::new(),
            more_options: MoreOptions::new(),
            per_character_3d: false,
            layer_is_3d: false,
            smart_quotes: None,
        }
    }

    pub fn enable_per_character_3d(&mut self) {
        self.per_character_3d = true;
        self.layer_is_3d = true;
        self.more_options.character_blend_enabled = false;
        self.more_options.fill_and_stroke_enabled = false;
    }

    pub fn sample_source(&self, time_seconds: f64) -> &str {
        let mut chosen = self.source.as_str();
        let mut chosen_time = f64::NEG_INFINITY;
        for key in &self.source_keys {
            if key.time_seconds <= time_seconds && key.time_seconds >= chosen_time {
                chosen = key.source.as_str();
                chosen_time = key.time_seconds;
            }
        }
        if chosen_time == f64::NEG_INFINITY {
            self.source.as_str()
        } else {
            chosen
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotKeyframable;

pub fn try_keyframe_character_panel() -> Result<(), NotKeyframable> {
    Err(NotKeyframable)
}

pub fn try_keyframe_per_character_3d() -> Result<(), NotKeyframable> {
    Err(NotKeyframable)
}

pub fn point_lines(source: &str) -> Vec<&str> {
    if source.is_empty() {
        return vec![""];
    }
    source.split(['\n', '\r']).collect()
}

pub fn paragraph_pieces(source: &str) -> Vec<&str> {
    source.split('\r').collect()
}

pub fn wrap_lines(
    paragraph: &str,
    box_width: f64,
    advance: f64,
    no_break: &[std::ops::Range<usize>],
) -> Vec<String> {
    let chars: Vec<char> = paragraph.chars().collect();
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut width = 0.0;
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '\n' {
            lines.push(std::mem::take(&mut current));
            width = 0.0;
            index += 1;
            continue;
        }
        let token = no_break
            .iter()
            .find(|range| range.start == index)
            .map(|range| range.end.min(chars.len()))
            .unwrap_or(index + 1);
        let piece: String = chars[index..token].iter().collect();
        let piece_width = advance * piece.chars().count() as f64;
        if !current.is_empty() && width + piece_width > box_width {
            lines.push(std::mem::take(&mut current));
            width = 0.0;
        }
        current.push_str(&piece);
        width += piece_width;
        index = token;
    }
    lines.push(current);
    lines
}

pub fn point_to_paragraph_source(source: &str) -> String {
    let lines = point_lines(source);
    if lines.len() <= 1 {
        return source.to_string();
    }
    lines.join("\r")
}

pub fn paragraph_to_point_source(
    source: &str,
    box_width: f64,
    box_height: f64,
    advance: f64,
    line_height: f64,
) -> String {
    let mut kept = Vec::new();
    let mut y = 0.0;
    for paragraph in paragraph_pieces(source) {
        for line in wrap_lines(paragraph, box_width, advance, &[]) {
            if y + line_height > box_height && !kept.is_empty() {
                return kept.join("\n");
            }
            if y < box_height {
                kept.push(line);
            }
            y += line_height;
        }
    }
    kept.join("\n")
}

pub fn apply_alignment(
    layout: TextLayout,
    command: AlignCommand,
) -> Result<ParagraphAlignment, PointJustify> {
    let justify = matches!(
        command,
        AlignCommand::JustifyLastLeft
            | AlignCommand::JustifyLastRight
            | AlignCommand::JustifyForced
    );
    if justify && matches!(layout, TextLayout::Point) {
        return Err(PointJustify);
    }
    Ok(match command {
        AlignCommand::Start => ParagraphAlignment::Left,
        AlignCommand::Center => ParagraphAlignment::Center,
        AlignCommand::End => ParagraphAlignment::Right,
        AlignCommand::JustifyLastLeft => ParagraphAlignment::JustifyLastLeft,
        AlignCommand::JustifyLastRight => ParagraphAlignment::JustifyLastRight,
        AlignCommand::JustifyForced => ParagraphAlignment::JustifyForced,
    })
}

pub fn alignment_edge(alignment: ParagraphAlignment, orientation: TextOrientation) -> AlignEdge {
    match (alignment, orientation) {
        (ParagraphAlignment::Center, _) => AlignEdge::Center,
        (ParagraphAlignment::Right, TextOrientation::Horizontal) => AlignEdge::End,
        (ParagraphAlignment::Right, TextOrientation::Vertical) => AlignEdge::End,
        (ParagraphAlignment::Left, _)
        | (ParagraphAlignment::JustifyLastLeft, _)
        | (ParagraphAlignment::JustifyLastRight, _)
        | (ParagraphAlignment::JustifyForced, _) => AlignEdge::Start,
    }
}

pub fn aligned_span(box_width: f64, content: f64, alignment: ParagraphAlignment) -> (f64, f64) {
    match alignment {
        ParagraphAlignment::Left | ParagraphAlignment::JustifyLastLeft => (0.0, content),
        ParagraphAlignment::Right | ParagraphAlignment::JustifyLastRight => {
            (box_width - content, box_width)
        }
        ParagraphAlignment::Center => {
            let start = (box_width - content) * 0.5;
            (start, start + content)
        }
        ParagraphAlignment::JustifyForced => (0.0, box_width),
    }
}

pub fn justified_span(
    box_width: f64,
    content: f64,
    alignment: ParagraphAlignment,
    is_last: bool,
) -> (f64, f64) {
    let force_last = alignment == ParagraphAlignment::JustifyForced;
    let expand = !is_last || force_last;
    if expand
        && matches!(
            alignment,
            ParagraphAlignment::JustifyLastLeft
                | ParagraphAlignment::JustifyLastRight
                | ParagraphAlignment::JustifyForced
        )
    {
        return (0.0, box_width);
    }
    if is_last && alignment == ParagraphAlignment::JustifyLastRight {
        return (box_width - content, box_width);
    }
    (0.0, content)
}

pub fn line_start(indent: f64, first_line_indent: f64, line_index: usize) -> f64 {
    if line_index == 0 {
        indent + first_line_indent
    } else {
        indent
    }
}

pub fn second_line_top(
    first_height: f64,
    second_height: f64,
    leading: f64,
    measure: LeadingMeasure,
) -> f64 {
    match measure {
        LeadingMeasure::TopToTop => leading,
        LeadingMeasure::BottomToBottom => leading + first_height - second_height,
    }
}

pub fn apply_auto_leading(style: &mut ParagraphStyle) {
    style.auto_leading = true;
}

pub fn used_leading(style: &ParagraphStyle, font_size: f64) -> f64 {
    if style.auto_leading {
        font_size
    } else {
        style.leading
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontFace {
    pub family: String,
    pub style: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedFont {
    pub used_family: String,
    pub original_name: String,
    pub substituted: bool,
}

pub fn resolve_font(requested: &str, available: &[FontFace], substitute: &str) -> ResolvedFont {
    if available.iter().any(|face| face.family == requested) {
        ResolvedFont {
            used_family: requested.to_string(),
            original_name: requested.to_string(),
            substituted: false,
        }
    } else {
        ResolvedFont {
            used_family: substitute.to_string(),
            original_name: requested.to_string(),
            substituted: true,
        }
    }
}

pub fn font_label(font: &ResolvedFont) -> String {
    if font.substituted {
        format!("{} ({})", font.used_family, font.original_name)
    } else {
        font.used_family.clone()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StyleChoice {
    Real,
    FauxBold,
    FauxItalic,
}

pub fn choose_bold(family_has_bold: bool) -> StyleChoice {
    if family_has_bold {
        StyleChoice::Real
    } else {
        StyleChoice::FauxBold
    }
}

pub fn choose_italic(family_has_italic: bool) -> StyleChoice {
    if family_has_italic {
        StyleChoice::Real
    } else {
        StyleChoice::FauxItalic
    }
}

pub fn appearance_pixels(size: f64, layer_scale_percent: f64) -> f64 {
    size * layer_scale_percent / 100.0
}

pub fn nudge_font_size(size: f64, by_ten: bool, decrease: bool) -> f64 {
    let step = if by_ten { 10.0 } else { 2.0 };
    if decrease {
        size - step
    } else {
        size + step
    }
}

pub fn nudge_tracking_units(by_hundred: bool) -> f64 {
    if by_hundred {
        100.0
    } else {
        20.0
    }
}

pub fn reset_tracking(style: &mut CharacterStyle) {
    style.tracking = 0.0;
}

pub fn reset_horizontal_scale(style: &mut CharacterStyle) {
    style.horizontal_scale = 100.0;
}

pub fn reset_vertical_scale(style: &mut CharacterStyle) {
    style.vertical_scale = 100.0;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UseTracking;

pub fn set_manual_kerning(range_length: usize, insertion_point: bool) -> Result<(), UseTracking> {
    if insertion_point && range_length == 0 {
        Ok(())
    } else {
        Err(UseTracking)
    }
}

pub fn pair_gap(tracking: f64, kerning: Kerning) -> f64 {
    let manual = match kerning {
        Kerning::Manual(value) => f64::from(value),
        Kerning::Metrics | Kerning::Optical => 0.0,
    };
    tracking + manual
}

pub fn fill_rgb(color: Rgba) -> [f64; 3] {
    [
        f64::from(color.r).clamp(0.0, 1.0),
        f64::from(color.g).clamp(0.0, 1.0),
        f64::from(color.b).clamp(0.0, 1.0),
    ]
}

pub fn stroke_sides(width: f64) -> (f64, f64) {
    (width * 0.5, width * 0.5)
}

pub fn stroke_visible(width: f64) -> bool {
    width != 0.0
}

pub fn paint_order(order: StrokeFillOrder) -> [&'static str; 2] {
    match order {
        StrokeFillOrder::StrokeOverFill => ["fill", "stroke"],
        StrokeFillOrder::FillOverStroke => ["stroke", "fill"],
        StrokeFillOrder::AllStrokesOverAllFills => ["fills", "strokes"],
        StrokeFillOrder::AllFillsOverAllStrokes => ["strokes", "fills"],
    }
}

pub fn effective_stroke_fill(
    per_character: Option<StrokeFillOrder>,
    layer: Option<StrokeFillOrder>,
    per_character_3d: bool,
) -> Option<StrokeFillOrder> {
    if per_character_3d {
        per_character
    } else {
        layer.or(per_character)
    }
}

pub fn apply_family(text: &mut Text, selection: Option<std::ops::Range<usize>>, family: &str) {
    match selection {
        None => text.typing.font_family = family.to_string(),
        Some(range) => {
            for style in text
                .characters
                .iter_mut()
                .skip(range.start)
                .take(range.end.saturating_sub(range.start))
            {
                style.font_family = family.to_string();
            }
        }
    }
}

pub fn apply_fill_color(text: &mut Text, selection: Option<std::ops::Range<usize>>, color: Rgba) {
    match selection {
        None => text.typing.fill = Some(color),
        Some(range) => {
            for style in text
                .characters
                .iter_mut()
                .skip(range.start)
                .take(range.end.saturating_sub(range.start))
            {
                style.fill = Some(color);
            }
        }
    }
}

pub fn baseline_offset(orientation: TextOrientation, value: f64) -> [f64; 2] {
    match orientation {
        TextOrientation::Horizontal => [0.0, value],
        TextOrientation::Vertical => [value, 0.0],
    }
}

pub fn nudge_baseline(value: f64, by_ten: bool, decrease: bool) -> f64 {
    let step = if by_ten { 10.0 } else { 2.0 };
    if decrease {
        value - step
    } else {
        value + step
    }
}

pub fn display_case(source: &str, all_caps: bool) -> String {
    if all_caps {
        source.to_uppercase()
    } else {
        source.to_string()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmallCapGlyph {
    pub source: char,
    pub small_cap: bool,
    pub height: f64,
}

pub fn small_cap_glyphs(source: &str, enabled: bool) -> Vec<SmallCapGlyph> {
    source
        .chars()
        .map(|ch| SmallCapGlyph {
            source: ch,
            small_cap: enabled && ch.is_lowercase(),
            height: 1.0,
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BaselineEffect {
    pub smaller: bool,
    pub raised: bool,
}

pub fn superscript_effect() -> BaselineEffect {
    BaselineEffect {
        smaller: true,
        raised: true,
    }
}

pub fn subscript_effect() -> BaselineEffect {
    BaselineEffect {
        smaller: true,
        raised: false,
    }
}

pub const SUPERSCRIPT_SHORTCUT: &str = "Ctrl+Shift+=";
pub const SUBSCRIPT_SHORTCUT: &str = "Ctrl+Alt+Shift+=";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TsumeAboveMax;

pub fn tsume_gap(original_gap: f64, percent: f64) -> Result<f64, TsumeAboveMax> {
    if percent > 100.0 {
        return Err(TsumeAboveMax);
    }
    Ok(original_gap * (1.0 - percent / 100.0))
}

pub fn glyph_width_after_tsume(width: f64) -> f64 {
    width
}

pub fn words(text: &str, no_break: &[std::ops::Range<usize>]) -> Vec<std::ops::Range<usize>> {
    let chars: Vec<char> = text.chars().collect();
    let mut ranges = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if let Some(range) = no_break.iter().find(|range| range.start == index) {
            ranges.push(range.start..range.end.min(chars.len()));
            index = range.end.min(chars.len());
            continue;
        }
        if chars[index].is_whitespace() {
            index += 1;
            continue;
        }
        let start = index;
        while index < chars.len()
            && !chars[index].is_whitespace()
            && no_break.iter().all(|range| range.start != index)
        {
            index += 1;
        }
        ranges.push(start..index);
    }
    ranges
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphPlacement {
    pub x: f64,
    pub y: f64,
    pub horizontal: bool,
}

pub fn layout_vertical(horizontal: &[bool], advance: f64) -> Vec<GlyphPlacement> {
    let mut placements = Vec::new();
    let mut index = 0;
    let mut y = 0.0;
    while index < horizontal.len() {
        if horizontal[index] {
            let row = y;
            let mut x = 0.0;
            while index < horizontal.len() && horizontal[index] {
                placements.push(GlyphPlacement {
                    x,
                    y: row,
                    horizontal: true,
                });
                x += advance;
                index += 1;
            }
            y += advance;
        } else {
            placements.push(GlyphPlacement {
                x: 0.0,
                y,
                horizontal: false,
            });
            y += advance;
            index += 1;
        }
    }
    placements
}

pub fn insert_quote(source: &str, quote: char, smart: bool) -> String {
    if !smart {
        return format!("{source}{quote}");
    }
    let opening = source
        .chars()
        .last()
        .map(|ch| ch.is_whitespace())
        .unwrap_or(true);
    let replacement = match (quote, opening) {
        ('"', true) => '\u{201C}',
        ('"', false) => '\u{201D}',
        ('\'', true) => '\u{2018}',
        ('\'', false) => '\u{2019}',
        _ => quote,
    };
    format!("{source}{replacement}")
}

pub fn hangs_outside(ch: char, enabled: bool) -> bool {
    enabled
        && matches!(
            ch,
            '.' | ',' | '\'' | '"' | '-' | '\u{2014}' | '\u{2013}' | ':' | ';'
        )
}

pub fn line_ink_end(box_width: f64, char_width: f64, last: char, enabled: bool) -> f64 {
    if hangs_outside(last, enabled) {
        box_width + char_width
    } else {
        box_width
    }
}

pub fn single_line_spacing_order() -> [SpacingDirection; 2] {
    [SpacingDirection::Tighten, SpacingDirection::Expand]
}

pub fn fit_gap(
    left: f64,
    right: f64,
    normal_gap: f64,
    min_gap: f64,
    box_width: f64,
) -> Option<f64> {
    if left + min_gap + right <= box_width {
        return Some(min_gap);
    }
    if left + normal_gap + right <= box_width {
        return Some(normal_gap);
    }
    None
}

pub fn break_lines(widths: &[f64], box_width: f64, composer: Composer) -> Vec<usize> {
    match composer {
        Composer::SingleLine => {
            let mut breaks = Vec::new();
            let mut start = 0;
            while start < widths.len() {
                let mut end = start + 1;
                while end < widths.len() && widths[start..=end].iter().sum::<f64>() <= box_width {
                    end += 1;
                }
                breaks.push(end);
                start = end;
            }
            breaks
        }
        Composer::EveryLine => every_line_breaks(widths, box_width),
    }
}

fn every_line_breaks(widths: &[f64], box_width: f64) -> Vec<usize> {
    let count = widths.len();
    let mut cost = vec![f64::MAX; count + 1];
    let mut previous = vec![0; count + 1];
    cost[0] = 0.0;
    for end in 1..=count {
        for start in 0..end {
            let width = widths[start..end].iter().sum::<f64>();
            if width > box_width && end - start > 1 {
                continue;
            }
            let line_cost = if width > box_width {
                1.0e12
            } else {
                let slack = box_width - width;
                slack * slack
            };
            let total = cost[start] + line_cost;
            if total < cost[end] {
                cost[end] = total;
                previous[end] = start;
            }
        }
    }
    let mut breaks = Vec::new();
    let mut cursor = count;
    while cursor > 0 && breaks.len() <= count {
        breaks.push(cursor);
        cursor = previous[cursor];
    }
    breaks.reverse();
    breaks
}

pub fn text_follows_mask(closed: bool, mode: Option<MaskMode>) -> bool {
    if closed {
        mode == Some(MaskMode::None)
    } else {
        true
    }
}

pub fn margins_used(alignment: ParagraphAlignment, force_alignment: bool) -> (bool, bool) {
    let ignore_first = alignment == ParagraphAlignment::Right && !force_alignment;
    let ignore_last = alignment == ParagraphAlignment::Left && !force_alignment;
    (!ignore_first, !ignore_last)
}

pub fn group_alignment_offset(percent: [f64; 2], em: f64) -> [f64; 2] {
    [percent[0] / 100.0 * em, -percent[1] / 100.0 * em]
}

pub fn anchor_for(
    group: AnchorGroup,
    index: usize,
    anchors: &[[f64; 2]],
    word: std::ops::Range<usize>,
) -> [f64; 2] {
    match group {
        AnchorGroup::Character => anchors.get(index).copied().unwrap_or([0.0, 0.0]),
        AnchorGroup::Word | AnchorGroup::Line | AnchorGroup::Block => {
            let slice = &anchors[word.start.min(anchors.len())..word.end.min(anchors.len())];
            if slice.is_empty() {
                [0.0, 0.0]
            } else {
                let sum = slice.iter().fold([0.0, 0.0], |acc, point| {
                    [acc[0] + point[0], acc[1] + point[1]]
                });
                [sum[0] / slice.len() as f64, sum[1] / slice.len() as f64]
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum SelectorMode {
    #[default]
    Add,
    Subtract,
    Intersect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectorShape {
    Square,
    RampUp,
    RampDown,
    Triangle,
    Round,
    Smooth,
}

impl SelectorShape {
    pub const ALL: [Self; 6] = [
        Self::Square,
        Self::RampUp,
        Self::RampDown,
        Self::Triangle,
        Self::Round,
        Self::Smooth,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectorUnit {
    Percent,
    Index,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectorBasedOn {
    Characters,
    CharactersExcludingSpaces,
    Words,
    Lines,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RangeSelector {
    pub mode: SelectorMode,
    pub amount_percent: f64,
    pub unit: SelectorUnit,
    pub based_on: SelectorBasedOn,
    pub start: f64,
    pub end: f64,
    pub offset: f64,
    pub shape: SelectorShape,
    pub ease_high: f64,
    pub ease_low: f64,
    pub randomize: bool,
    pub seed: i32,
}

impl RangeSelector {
    /// モードの初期値は加算。量・単位・開始・終了・形状・イーズの初期値は要確認。
    pub fn new() -> Self {
        Self {
            mode: SelectorMode::Add,
            amount_percent: 0.0,
            unit: SelectorUnit::Percent,
            based_on: SelectorBasedOn::Characters,
            start: 0.0,
            end: 100.0,
            offset: 0.0,
            shape: SelectorShape::Square,
            ease_high: 0.0,
            ease_low: 0.0,
            randomize: false,
            seed: 0,
        }
    }
}

impl Default for RangeSelector {
    fn default() -> Self {
        Self::new()
    }
}

pub fn range_influence(selector: &RangeSelector, index: usize, count: usize) -> f64 {
    if selector.amount_percent == 0.0 && selector.mode == SelectorMode::Add {
        return 0.0;
    }
    let t = if count == 0 {
        0.0
    } else {
        index as f64 / count as f64
    };
    let shape = shape_weight(
        selector.shape,
        t,
        selector.start / 100.0,
        selector.end / 100.0,
        selector.ease_high,
    );
    let scaled = shape * selector.amount_percent / 100.0;
    match selector.mode {
        SelectorMode::Add => scaled,
        SelectorMode::Subtract => 1.0 - scaled,
        SelectorMode::Intersect => scaled,
    }
}

fn shape_weight(shape: SelectorShape, t: f64, start: f64, end: f64, ease_high: f64) -> f64 {
    if end <= start {
        return 0.0;
    }
    let local = ((t - start) / (end - start)).clamp(0.0, 1.0);
    let inside = t >= start && t <= end;
    match shape {
        SelectorShape::Square => {
            if !inside {
                return 0.0;
            }
            let gradual = ((ease_high + 100.0) / 200.0).clamp(0.0, 1.0);
            let knee = 1.0 - 0.5 * gradual;
            if local <= knee {
                1.0
            } else {
                1.0 - (local - knee) / (1.0 - knee).max(1e-6)
            }
        }
        SelectorShape::RampUp => {
            if inside {
                local
            } else {
                0.0
            }
        }
        SelectorShape::RampDown => {
            if inside {
                1.0 - local
            } else {
                0.0
            }
        }
        SelectorShape::Triangle => {
            if inside {
                1.0 - (local * 2.0 - 1.0).abs()
            } else {
                0.0
            }
        }
        SelectorShape::Round => {
            if inside {
                (1.0 - (local * 2.0 - 1.0).powi(2)).sqrt()
            } else {
                0.0
            }
        }
        SelectorShape::Smooth => {
            if inside {
                local * local * (3.0 - 2.0 * local)
            } else {
                0.0
            }
        }
    }
}

pub fn selector_count(text: &str, based_on: SelectorBasedOn) -> usize {
    match based_on {
        SelectorBasedOn::Characters => text.chars().count(),
        SelectorBasedOn::CharactersExcludingSpaces => {
            text.chars().filter(|ch| !ch.is_whitespace()).count()
        }
        SelectorBasedOn::Words => words(text, &[]).len(),
        SelectorBasedOn::Lines => point_lines(text).len(),
    }
}

pub fn randomized_order(seed: i32, group_id: u64, count: usize) -> Vec<usize> {
    let mut state = if seed == 0 {
        group_id.wrapping_mul(0x9E37_79B9_7F4A_7C15)
    } else {
        seed as u64
    };
    let mut items: Vec<usize> = (0..count).collect();
    for index in (1..count).rev() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let swap = (state as usize) % (index + 1);
        items.swap(index, swap);
    }
    items
}

#[derive(Clone, Debug, PartialEq)]
pub struct WigglySelector {
    pub max_amount: f64,
    pub min_amount: f64,
    pub wiggles_per_second: f64,
    pub correlation_percent: f64,
    pub temporal_phase: f64,
    pub spatial_phase: f64,
    pub lock_dimensions: bool,
    pub random_seed: i32,
}

pub fn wiggly_amount(selector: &WigglySelector, index: usize, time: f64) -> f64 {
    let independence = 1.0 - selector.correlation_percent / 100.0;
    let wave = (time * selector.wiggles_per_second
        + selector.temporal_phase
        + selector.spatial_phase
        + index as f64 * independence)
        .sin();
    let mix = (wave + 1.0) * 0.5;
    selector.min_amount + (selector.max_amount - selector.min_amount) * mix
}

pub const DEFAULT_AMOUNT_EXPRESSION: &str = "selectorValue * textIndex/textTotal";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpressionSelector {
    pub amount_expression: String,
}

impl ExpressionSelector {
    pub fn new() -> Self {
        Self {
            amount_expression: DEFAULT_AMOUNT_EXPRESSION.to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExprError {
    Syntax,
    Unsupported,
}

pub fn eval_selector_expression(
    expression: &str,
    selector_value: f64,
    text_index: f64,
    text_total: f64,
) -> Result<f64, ExprError> {
    if expression != DEFAULT_AMOUNT_EXPRESSION {
        return Err(ExprError::Unsupported);
    }
    if text_total == 0.0 {
        return Err(ExprError::Syntax);
    }
    Ok(selector_value * text_index / text_total)
}

pub fn selector_name_outside(name: &str) -> Result<(), ExprError> {
    match name {
        "textIndex" | "textTotal" | "selectorValue" => Err(ExprError::Syntax),
        _ => Ok(()),
    }
}

pub fn text_index_step(count: usize) -> Vec<i32> {
    (0..count as i32).collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OptionalAxis {
    Absent,
    PresentUnset,
    Set(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub enum AnimatorProperty {
    Anchor { xy: [f64; 2], z: OptionalAxis },
    Position { xy: [f64; 2], z: OptionalAxis },
    Scale { xy: [f64; 2], z: OptionalAxis },
    Skew(f64),
    SkewAxis(f64),
    Rotation(f64),
    XRotation(f64),
    YRotation(f64),
    ZRotation(f64),
    Opacity(f64),
    FillColor(String),
    StrokeColor(String),
    StrokeWidth(f64),
    Tracking(f64),
    LineAnchor(f64),
    LineSpacing(f64),
    CharacterOffset(i32),
    CharacterValue(i32),
    CharacterRange(CharacterRangeMode),
    Blur { horizontal: f64, vertical: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharacterRangeMode {
    PreserveCaseAndDigits,
    AllUnicode,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TextSelector {
    Range(RangeSelector),
    Wiggly(WigglySelector),
    Expression(ExpressionSelector),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextAnimator {
    pub name: String,
    pub selector: String,
    pub selectors: Vec<TextSelector>,
    pub properties: Vec<AnimatorProperty>,
    pub per_character_3d: bool,
    pub layer_is_3d: bool,
}

impl TextAnimator {
    pub fn new(name: impl Into<String>, selector: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            selector: selector.into(),
            selectors: Vec::new(),
            properties: Vec::new(),
            per_character_3d: false,
            layer_is_3d: false,
        }
    }

    pub fn with_default_range(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            selector: "Range".to_string(),
            selectors: vec![TextSelector::Range(RangeSelector::new())],
            properties: Vec::new(),
            per_character_3d: false,
            layer_is_3d: false,
        }
    }

    pub fn enable_per_character_3d(&mut self) {
        self.per_character_3d = true;
        self.layer_is_3d = true;
        for property in &mut self.properties {
            if let AnimatorProperty::Rotation(value) = property {
                *property = AnimatorProperty::ZRotation(*value);
            }
            if let AnimatorProperty::Position { z, .. } = property {
                if matches!(z, OptionalAxis::Absent) {
                    *z = OptionalAxis::PresentUnset;
                }
            }
            if let AnimatorProperty::Anchor { z, .. } = property {
                if matches!(z, OptionalAxis::Absent) {
                    *z = OptionalAxis::PresentUnset;
                }
            }
            if let AnimatorProperty::Scale { z, .. } = property {
                if matches!(z, OptionalAxis::Absent) {
                    *z = OptionalAxis::PresentUnset;
                }
            }
        }
    }

    pub fn disable_per_character_keep_layer(&mut self) {
        self.per_character_3d = false;
        self.layer_is_3d = true;
    }

    pub fn return_to_2d(&mut self) {
        self.per_character_3d = false;
        self.layer_is_3d = false;
        self.properties.retain(|property| {
            !matches!(
                property,
                AnimatorProperty::XRotation(_) | AnimatorProperty::YRotation(_)
            )
        });
        for property in &mut self.properties {
            if let AnimatorProperty::ZRotation(value) = property {
                *property = AnimatorProperty::Rotation(*value);
            }
            if let AnimatorProperty::Position { z, .. } = property {
                *z = OptionalAxis::Absent;
            }
        }
    }

    pub fn orient_each_character_independently(&mut self) {
        self.enable_per_character_3d();
    }

    pub fn add_all_transforms(&mut self) {
        self.properties.push(AnimatorProperty::Anchor {
            xy: [0.0, 0.0],
            z: OptionalAxis::Absent,
        });
        self.properties.push(AnimatorProperty::Position {
            xy: [0.0, 0.0],
            z: OptionalAxis::Absent,
        });
        self.properties.push(AnimatorProperty::Scale {
            xy: [0.0, 0.0],
            z: OptionalAxis::Absent,
        });
        self.properties.push(AnimatorProperty::Skew(0.0));
        if self.per_character_3d {
            self.properties.push(AnimatorProperty::ZRotation(0.0));
        } else {
            self.properties.push(AnimatorProperty::Rotation(0.0));
        }
        self.properties.push(AnimatorProperty::Opacity(0.0));
        if self.per_character_3d {
            self.enable_per_character_3d();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuEntry {
    Item(&'static str),
    Submenu(&'static str),
}

pub fn animator_menu(per_character_3d: bool) -> Vec<MenuEntry> {
    let mut menu = vec![
        MenuEntry::Item("Anchor Point"),
        MenuEntry::Item("Position"),
        MenuEntry::Item("Scale"),
        MenuEntry::Item("Skew"),
        MenuEntry::Item("Skew Axis"),
    ];
    if per_character_3d {
        menu.push(MenuEntry::Item("X Rotation"));
        menu.push(MenuEntry::Item("Y Rotation"));
        menu.push(MenuEntry::Item("Z Rotation"));
    } else {
        menu.push(MenuEntry::Item("Rotation"));
    }
    menu.push(MenuEntry::Item("Opacity"));
    menu.push(MenuEntry::Item("All Transform Properties"));
    menu.push(MenuEntry::Submenu("Fill Color"));
    menu.push(MenuEntry::Submenu("Stroke Color"));
    menu.push(MenuEntry::Item("Stroke Width"));
    menu.push(MenuEntry::Item("Tracking"));
    menu.push(MenuEntry::Item("Line Anchor"));
    menu.push(MenuEntry::Item("Line Spacing"));
    menu.push(MenuEntry::Item("Character Offset"));
    menu.push(MenuEntry::Item("Character Value"));
    menu.push(MenuEntry::Item("Blur"));
    menu
}

pub fn can_add_rotation(name: &str, per_character_3d: bool) -> bool {
    match name {
        "Rotation" => !per_character_3d,
        "X Rotation" | "Y Rotation" | "Z Rotation" => per_character_3d,
        _ => false,
    }
}

pub fn drag_position(position: &mut [f64; 2], delta: [f64; 2]) {
    position[0] += delta[0];
    position[1] += delta[1];
}

pub fn scale_by_amount(delta: [f64; 2], amount_percent: f64) -> [f64; 2] {
    [
        delta[0] * amount_percent / 100.0,
        delta[1] * amount_percent / 100.0,
    ]
}

pub fn orbit_character(
    point: [f64; 2],
    anchor: [f64; 2],
    degrees: f64,
    influence: f64,
) -> [f64; 2] {
    transform_points(
        &[point],
        anchor,
        [0.0, 0.0],
        degrees * influence,
        [1.0, 1.0],
    )[0]
}

pub fn scale_point(point: [f64; 3], anchor: [f64; 3], scale: [f64; 3]) -> [f64; 3] {
    [
        anchor[0] + (point[0] - anchor[0]) * scale[0],
        anchor[1] + (point[1] - anchor[1]) * scale[1],
        anchor[2] + (point[2] - anchor[2]) * scale[2],
    ]
}

pub fn skew_point(point: [f64; 2], skew_degrees: f64, axis_degrees: f64) -> [f64; 2] {
    let axis = axis_degrees.to_radians();
    let (sin, cos) = axis.sin_cos();
    let along = point[0] * cos + point[1] * sin;
    let shift = skew_degrees.to_radians().tan() * along;
    [point[0] - shift * sin, point[1] + shift * cos]
}

pub fn reveal_opacity(index: usize, count: usize, end_percent: f64, animator_opacity: f64) -> f64 {
    let revealed = end_percent / 100.0 * count as f64;
    if (index as f64) < revealed {
        1.0
    } else {
        animator_opacity
    }
}

pub fn line_anchor_shift(extra: f64, anchor_percent: f64) -> f64 {
    extra * anchor_percent / 100.0
}

pub fn animated_line_gap(leading: f64, animator_gap: f64, line_count: usize) -> f64 {
    if line_count < 2 {
        leading
    } else {
        leading + animator_gap
    }
}

pub fn offset_text(source: &str, offset: i32, mode: CharacterRangeMode) -> String {
    source
        .chars()
        .map(|ch| offset_char(ch, offset, mode))
        .collect()
}

pub fn replace_with_value(source: &str, value: u32) -> String {
    let replacement = char::from_u32(value).unwrap_or('\u{FFFD}');
    source.chars().map(|_| replacement).collect()
}

pub fn shows_character_range(properties: &[AnimatorProperty]) -> bool {
    properties.iter().any(|property| {
        matches!(
            property,
            AnimatorProperty::CharacterOffset(_) | AnimatorProperty::CharacterValue(_)
        )
    })
}

fn offset_char(ch: char, offset: i32, mode: CharacterRangeMode) -> char {
    match mode {
        CharacterRangeMode::AllUnicode => char::from_u32((ch as i32 + offset) as u32).unwrap_or(ch),
        CharacterRangeMode::PreserveCaseAndDigits => shift_group(ch, offset).unwrap_or(ch),
    }
}

fn shift_group(ch: char, offset: i32) -> Option<char> {
    let (start, len) = if ch.is_ascii_uppercase() {
        ('A' as u32, 26)
    } else if ch.is_ascii_lowercase() {
        ('a' as u32, 26)
    } else if ch.is_ascii_digit() {
        ('0' as u32, 10)
    } else if ('\u{30A1}'..='\u{30FA}').contains(&ch) {
        ('\u{30A1}' as u32, '\u{30FA}' as u32 - '\u{30A1}' as u32 + 1)
    } else {
        return None;
    };
    let index = ch as u32 - start;
    let shifted = (index as i32 + offset).rem_euclid(len as i32) as u32;
    char::from_u32(start + shifted)
}

pub fn blur_amount(horizontal: f64, vertical: f64, influence: f64) -> (f64, f64) {
    (horizontal * influence, vertical * influence)
}

pub fn property_names(properties: &[AnimatorProperty]) -> Vec<&'static str> {
    properties.iter().map(property_name).collect()
}

fn property_name(property: &AnimatorProperty) -> &'static str {
    match property {
        AnimatorProperty::Anchor { .. } => "Anchor Point",
        AnimatorProperty::Position { .. } => "Position",
        AnimatorProperty::Scale { .. } => "Scale",
        AnimatorProperty::Skew(_) => "Skew",
        AnimatorProperty::SkewAxis(_) => "Skew Axis",
        AnimatorProperty::Rotation(_) => "Rotation",
        AnimatorProperty::XRotation(_) => "X Rotation",
        AnimatorProperty::YRotation(_) => "Y Rotation",
        AnimatorProperty::ZRotation(_) => "Z Rotation",
        AnimatorProperty::Opacity(_) => "Opacity",
        AnimatorProperty::FillColor(_) => "Fill Color",
        AnimatorProperty::StrokeColor(_) => "Stroke Color",
        AnimatorProperty::StrokeWidth(_) => "Stroke Width",
        AnimatorProperty::Tracking(_) => "Tracking",
        AnimatorProperty::LineAnchor(_) => "Line Anchor",
        AnimatorProperty::LineSpacing(_) => "Line Spacing",
        AnimatorProperty::CharacterOffset(_) => "Character Offset",
        AnimatorProperty::CharacterValue(_) => "Character Value",
        AnimatorProperty::CharacterRange(_) => "Character Range",
        AnimatorProperty::Blur { .. } => "Blur",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(left: f64, right: f64) -> bool {
        (left - right).abs() < 1e-6
    }

    #[test]
    fn point_text_does_not_wrap_and_keys_change_characters() {
        let lines = point_lines("a very long line without a break");
        assert_eq!(lines.len(), 1);
        assert_eq!(point_lines("one\ntwo").len(), 2);
        let mut text = Text::point("one\ntwo");
        text.source_keys.push(SourceTextKey {
            time_seconds: 1.0,
            source: "next".to_string(),
        });
        assert_eq!(text.sample_source(0.5), "one\ntwo");
        assert_eq!(text.sample_source(1.0), "next");
        assert_eq!(text.sample_source(2.0), "next");
    }

    #[test]
    fn paragraph_wraps_soft_returns_and_drops_overflow_on_point_conversion() {
        assert_eq!(paragraph_pieces("ab\ncd").len(), 1);
        assert_eq!(wrap_lines("ab\ncd", 100.0, 10.0, &[]).len(), 2);
        assert_eq!(
            wrap_lines("abcdef", 25.0, 10.0, &[]),
            vec!["ab", "cd", "ef"]
        );
        let converted = point_to_paragraph_source("ab\ncd\nef");
        assert_eq!(converted, "ab\rcd\ref");
        assert!(!converted.ends_with('\r'));
        let trimmed = paragraph_to_point_source("abcdef", 25.0, 20.0, 10.0, 10.0);
        assert_eq!(trimmed, "ab\ncd");
        assert!(apply_alignment(TextLayout::Point, AlignCommand::JustifyLastLeft).is_err());
        assert!(apply_alignment(
            TextLayout::Paragraph {
                width: 10.0,
                height: 10.0
            },
            AlignCommand::JustifyForced
        )
        .is_ok());
    }

    #[test]
    fn missing_font_keeps_the_original_name() {
        let resolved = resolve_font(
            "Missing",
            &[FontFace {
                family: "Arial".into(),
                style: "Regular".into(),
            }],
            "Arial",
        );
        assert!(resolved.substituted);
        assert!(font_label(&resolved).contains("(Missing)"));
        let mut text = Text::point("abcd");
        apply_family(&mut text, Some(1..3), "Arial");
        assert_eq!(text.characters[0].font_family, "");
        assert_eq!(text.characters[1].font_family, "Arial");
        assert_eq!(text.characters[3].font_family, "");
        apply_family(&mut text, None, "Next");
        assert_eq!(text.typing.font_family, "Next");
        assert_eq!(text.characters[2].font_family, "Arial");
        assert!(try_keyframe_character_panel().is_err());
    }

    #[test]
    fn faux_style_is_used_when_the_family_lacks_it() {
        assert_eq!(choose_bold(true), StyleChoice::Real);
        assert_eq!(choose_bold(false), StyleChoice::FauxBold);
        assert_eq!(choose_italic(true), StyleChoice::Real);
        assert_eq!(choose_italic(false), StyleChoice::FauxItalic);
    }

    #[test]
    fn font_size_matches_comp_pixels_at_uniform_scale() {
        assert!(near(appearance_pixels(10.0, 100.0), 10.0));
        assert!(near(appearance_pixels(10.0, 200.0), 20.0));
        assert!(near(nudge_font_size(10.0, false, false), 12.0));
        assert!(near(nudge_font_size(10.0, true, false), 20.0));
    }

    #[test]
    fn faux_bold_stays_distinct_from_the_style_name() {
        let mut style = CharacterStyle::new();
        style.font_style = "Regular".to_string();
        style.faux_bold = true;
        assert_eq!(style.font_style, "Regular");
        assert!(style.faux_bold);
    }

    #[test]
    fn faux_italic_stays_distinct_from_the_style_name() {
        let mut style = CharacterStyle::new();
        style.font_style = "Regular".to_string();
        style.faux_italic = true;
        assert_eq!(style.font_style, "Regular");
        assert!(style.faux_italic);
        assert!(!style.faux_bold);
    }

    #[test]
    fn leading_changes_only_the_gap_between_measures() {
        let top = second_line_top(20.0, 10.0, 30.0, LeadingMeasure::TopToTop);
        let bottom = second_line_top(20.0, 10.0, 30.0, LeadingMeasure::BottomToBottom);
        assert!(near(top, 30.0));
        assert!(!near(top, bottom));
        assert!(near(20.0, 20.0));
    }

    #[test]
    fn auto_leading_shortcut_computes_from_size() {
        let mut style = ParagraphStyle::new();
        style.leading = 30.0;
        apply_auto_leading(&mut style);
        assert!(style.auto_leading);
        assert!(near(style.leading, 30.0));
        assert!(!near(
            used_leading(&style, 12.0),
            used_leading(&style, 18.0)
        ));
        assert!(!near(used_leading(&style, 12.0), style.leading));
    }

    #[test]
    fn kerning_starts_metric_and_adds_to_tracking() {
        let style = CharacterStyle::new();
        assert_eq!(style.kerning, Kerning::Metrics);
        assert!(set_manual_kerning(4, false).is_err());
        assert!(set_manual_kerning(0, true).is_ok());
        assert!(near(pair_gap(20.0, Kerning::Manual(5)), 25.0));
        assert!(near(nudge_tracking_units(false), 20.0));
        assert!(near(nudge_tracking_units(true), 100.0));
    }

    #[test]
    fn tracking_resets_to_zero_without_touching_kerning() {
        let mut style = CharacterStyle::new();
        assert!(near(style.tracking, 0.0));
        style.tracking = 40.0;
        style.kerning = Kerning::Manual(15);
        reset_tracking(&mut style);
        assert!(near(style.tracking, 0.0));
        assert_eq!(style.kerning, Kerning::Manual(15));
    }

    #[test]
    fn fill_color_follows_the_selection_and_expression_range() {
        let mut text = Text::point("abcd");
        apply_fill_color(&mut text, Some(0..1), Rgba::new(0.2, 0.4, 0.6, 1.0));
        assert_eq!(text.characters[0].fill.unwrap().r, 0.2);
        assert!(text.characters[1].fill.is_none());
        apply_fill_color(&mut text, None, Rgba::new(1.0, 0.0, 0.0, 1.0));
        assert_eq!(text.typing.fill.unwrap().r, 1.0);
        assert!(text.characters[2].fill.is_none());
        let rgb = fill_rgb(Rgba::new(1.2, -0.2, 0.5, 1.0));
        assert_eq!(rgb.len(), 3);
        assert!(rgb.iter().all(|channel| (0.0..=1.0).contains(channel)));
    }

    #[test]
    fn clearing_fill_leaves_the_stroke() {
        let mut style = CharacterStyle::new();
        style.apply_fill = Some(false);
        style.apply_stroke = Some(true);
        assert_eq!(style.apply_fill, Some(false));
        assert_eq!(style.apply_stroke, Some(true));
    }

    #[test]
    fn stroke_sits_on_the_center_of_the_outline() {
        let (inside, outside) = stroke_sides(8.0);
        assert!(near(inside, 4.0));
        assert!(near(outside, 4.0));
        let mut text = Text::point("ab");
        text.characters[0].stroke = Some(Rgba::black());
        assert!(text.characters[1].stroke.is_none());
    }

    #[test]
    fn clearing_stroke_leaves_the_fill() {
        let mut style = CharacterStyle::new();
        style.apply_fill = Some(true);
        style.apply_stroke = Some(false);
        assert_eq!(style.apply_fill, Some(true));
        assert_eq!(style.apply_stroke, Some(false));
    }

    #[test]
    fn stroke_width_zero_is_invisible_and_growth_is_symmetric() {
        assert!(!stroke_visible(0.0));
        assert!(stroke_visible(2.0));
        let narrow = stroke_sides(2.0);
        let wide = stroke_sides(6.0);
        assert!(wide.0 > narrow.0 && wide.1 > narrow.1);
    }

    #[test]
    fn stroke_fill_order_has_four_results_and_3d_ignores_the_layer() {
        let orders = [
            StrokeFillOrder::StrokeOverFill,
            StrokeFillOrder::FillOverStroke,
            StrokeFillOrder::AllStrokesOverAllFills,
            StrokeFillOrder::AllFillsOverAllStrokes,
        ];
        let painted: Vec<_> = orders.into_iter().map(paint_order).collect();
        assert_eq!(
            painted
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            4
        );
        let character = Some(StrokeFillOrder::FillOverStroke);
        let layer = Some(StrokeFillOrder::AllStrokesOverAllFills);
        assert_eq!(effective_stroke_fill(character, layer, false), layer);
        assert_eq!(effective_stroke_fill(character, layer, true), character);
    }

    #[test]
    fn text_line_join_is_stored_apart_from_shape_joins() {
        let text_join = TextLineJoin::Round;
        let shape_join = crate::shape::LineJoin::Miter;
        assert_eq!(text_join, TextLineJoin::Round);
        assert_eq!(shape_join, crate::shape::LineJoin::Miter);
        assert_ne!(
            format!("{text_join:?}"),
            format!("{:?}", TextLineJoin::Bevel)
        );
    }

    #[test]
    fn horizontal_scale_resets_to_100_and_does_not_change_height() {
        let mut style = CharacterStyle::new();
        assert!(near(style.horizontal_scale, 100.0));
        style.horizontal_scale = 80.0;
        style.vertical_scale = 100.0;
        reset_horizontal_scale(&mut style);
        assert!(near(style.horizontal_scale, 100.0));
        assert!(near(style.vertical_scale, 100.0));
    }

    #[test]
    fn vertical_scale_resets_to_100_and_does_not_change_width() {
        let mut style = CharacterStyle::new();
        style.vertical_scale = 70.0;
        style.horizontal_scale = 90.0;
        reset_vertical_scale(&mut style);
        assert!(near(style.vertical_scale, 100.0));
        assert!(near(style.horizontal_scale, 90.0));
    }

    #[test]
    fn baseline_shift_axes_and_shortcut_steps() {
        let up = baseline_offset(TextOrientation::Horizontal, 4.0);
        let right = baseline_offset(TextOrientation::Vertical, 4.0);
        assert!(up[1] > 0.0 && near(up[0], 0.0));
        assert!(right[0] > 0.0 && near(right[1], 0.0));
        assert!(near(nudge_baseline(0.0, false, false), 2.0));
        assert!(near(nudge_baseline(0.0, true, false), 10.0));
    }

    #[test]
    fn all_caps_toggles_back_to_the_source() {
        assert_eq!(display_case("Ab", true), "AB");
        assert_eq!(display_case("Ab", false), "Ab");
    }

    #[test]
    fn small_caps_skip_letters_typed_as_capitals() {
        let plain = small_cap_glyphs("Ab", false);
        let glyphs = small_cap_glyphs("Ab", true);
        assert!(!glyphs[0].small_cap);
        assert!(glyphs[1].small_cap);
        assert!(near(glyphs[0].height, plain[0].height));
    }

    #[test]
    fn superscript_is_smaller_and_raised() {
        let effect = superscript_effect();
        assert!(effect.smaller && effect.raised);
    }

    #[test]
    fn subscript_is_smaller_and_lower_on_another_shortcut() {
        let effect = subscript_effect();
        assert!(effect.smaller && !effect.raised);
        assert_ne!(SUPERSCRIPT_SHORTCUT, SUBSCRIPT_SHORTCUT);
    }

    #[test]
    fn tsume_at_100_closes_the_sidebearing() {
        assert!(near(tsume_gap(8.0, 100.0).unwrap(), 0.0));
        assert!(tsume_gap(8.0, 120.0).is_err());
        assert!(near(glyph_width_after_tsume(12.0), 12.0));
    }

    #[test]
    fn no_break_range_stays_whole_and_counts_as_one_word() {
        let lines = wrap_lines("abcdef", 20.0, 10.0, &[0..4]);
        assert_eq!(lines[0], "abcd");
        let found = words("aa bb cc", &[0..5]);
        assert_eq!(found.len(), 2);
        assert_eq!(&"aa bb cc"[found[0].clone()], "aa bb");
    }

    #[test]
    fn tate_chu_yoko_keeps_a_horizontal_run_editable() {
        let placed = layout_vertical(&[false, true, true, false], 10.0);
        assert!(!placed[0].horizontal);
        assert!(placed[1].horizontal && placed[2].horizontal);
        assert!(near(placed[1].y, placed[2].y));
        assert!(placed[2].x > placed[1].x);
        let mut source = "あ12い".to_string();
        source.push('う');
        assert!(source.ends_with('う'));
    }

    #[test]
    fn smart_quotes_curl_only_while_enabled() {
        assert_eq!(insert_quote("", '"', false).chars().last().unwrap(), '"');
        assert_eq!(
            insert_quote("", '"', true).chars().last().unwrap(),
            '\u{201C}'
        );
        assert_eq!(
            insert_quote("a", '"', true).chars().last().unwrap(),
            '\u{201D}'
        );
    }

    #[test]
    fn digit_set_is_stored_without_listing_unknown_choices() {
        let mut style = CharacterStyle::new();
        style.digit_set.raw = "kept".to_string();
        assert_eq!(style.digit_set.raw, "kept");
    }

    #[test]
    fn baseline_direction_is_a_different_field_from_superscript() {
        let mut style = CharacterStyle::new();
        style.superscript = true;
        style.baseline_direction.raw = "other".to_string();
        assert!(style.superscript);
        assert_eq!(style.baseline_direction.raw, "other");
    }

    #[test]
    fn left_align_applies_to_point_text() {
        assert_eq!(
            apply_alignment(TextLayout::Point, AlignCommand::Start).unwrap(),
            ParagraphAlignment::Left
        );
        let (start, _) = aligned_span(100.0, 40.0, ParagraphAlignment::Left);
        assert!(near(start, 0.0));
    }

    #[test]
    fn center_align_shares_the_line_center() {
        let narrow = aligned_span(100.0, 20.0, ParagraphAlignment::Center);
        let wide = aligned_span(100.0, 40.0, ParagraphAlignment::Center);
        assert!(near((narrow.0 + narrow.1) * 0.5, (wide.0 + wide.1) * 0.5));
    }

    #[test]
    fn right_align_meets_the_right_edge() {
        let (_, end) = aligned_span(100.0, 30.0, ParagraphAlignment::Right);
        assert!(near(end, 100.0));
    }

    #[test]
    fn top_align_uses_the_horizontal_left_command() {
        assert_eq!(
            apply_alignment(TextLayout::Point, AlignCommand::Start).unwrap(),
            ParagraphAlignment::Left
        );
        assert_eq!(
            alignment_edge(ParagraphAlignment::Left, TextOrientation::Vertical),
            AlignEdge::Start
        );
        assert_eq!(
            alignment_edge(ParagraphAlignment::Left, TextOrientation::Horizontal),
            AlignEdge::Start
        );
    }

    #[test]
    fn vertical_center_uses_the_center_command() {
        assert_eq!(
            alignment_edge(ParagraphAlignment::Center, TextOrientation::Vertical),
            AlignEdge::Center
        );
    }

    #[test]
    fn bottom_align_uses_the_horizontal_right_command() {
        assert_eq!(
            apply_alignment(
                TextLayout::Paragraph {
                    width: 1.0,
                    height: 1.0
                },
                AlignCommand::End
            )
            .unwrap(),
            ParagraphAlignment::Right
        );
        assert_eq!(
            alignment_edge(ParagraphAlignment::Right, TextOrientation::Vertical),
            AlignEdge::End
        );
    }

    #[test]
    fn justify_last_left_expands_every_line_but_the_last() {
        assert!(apply_alignment(TextLayout::Point, AlignCommand::JustifyLastLeft).is_err());
        let body = justified_span(100.0, 40.0, ParagraphAlignment::JustifyLastLeft, false);
        let last = justified_span(100.0, 40.0, ParagraphAlignment::JustifyLastLeft, true);
        assert!(near(body.0, 0.0) && near(body.1, 100.0));
        assert!(near(last.0, 0.0) && near(last.1, 40.0));
    }

    #[test]
    fn justify_last_right_keeps_the_last_line_on_the_right() {
        let last = justified_span(100.0, 40.0, ParagraphAlignment::JustifyLastRight, true);
        assert!(near(last.1, 100.0));
        assert!(near(last.0, 60.0));
    }

    #[test]
    fn justify_forced_expands_the_last_line_too() {
        assert!(apply_alignment(TextLayout::Point, AlignCommand::JustifyForced).is_err());
        let last = justified_span(100.0, 40.0, ParagraphAlignment::JustifyForced, true);
        assert!(near(last.0, 0.0) && near(last.1, 100.0));
    }

    #[test]
    fn left_indent_moves_only_the_selected_paragraph() {
        let mut paragraphs = vec![ParagraphStyle::new(), ParagraphStyle::new()];
        paragraphs[0].left_indent = 12.0;
        assert!(near(line_start(paragraphs[0].left_indent, 0.0, 1), 12.0));
        assert!(near(line_start(paragraphs[1].left_indent, 0.0, 1), 0.0));
        assert_eq!(
            alignment_edge(ParagraphAlignment::Left, TextOrientation::Vertical),
            AlignEdge::Start
        );
    }

    #[test]
    fn right_indent_insets_one_paragraph() {
        let mut paragraphs = vec![ParagraphStyle::new(), ParagraphStyle::new()];
        paragraphs[1].right_indent = 8.0;
        assert!(near(100.0 - paragraphs[1].right_indent, 92.0));
        assert!(near(100.0 - paragraphs[0].right_indent, 100.0));
    }

    #[test]
    fn negative_first_line_indent_hangs() {
        let first = line_start(10.0, -4.0, 0);
        let second = line_start(10.0, -4.0, 1);
        assert!(first < 10.0);
        assert!(near(second, 10.0));
    }

    #[test]
    fn space_before_opens_only_that_paragraph() {
        let mut paragraphs = vec![ParagraphStyle::new(), ParagraphStyle::new()];
        paragraphs[1].space_before = 6.0;
        assert!(near(paragraphs[1].space_before, 6.0));
        assert!(near(paragraphs[0].space_before, 0.0));
    }

    #[test]
    fn space_after_opens_only_after_that_paragraph() {
        let mut paragraphs = vec![ParagraphStyle::new(), ParagraphStyle::new()];
        paragraphs[0].space_after = 5.0;
        assert!(near(paragraphs[0].space_after, 5.0));
        assert!(near(paragraphs[1].space_after, 0.0));
    }

    #[test]
    fn roman_punctuation_hangs_and_cjk_punctuation_does_not() {
        for ch in ['.', ',', '\'', '"', '-', '\u{2014}', '\u{2013}', ':', ';'] {
            assert!(hangs_outside(ch, true), "{ch}");
        }
        assert!(!hangs_outside('。', true));
        assert!(!hangs_outside('、', true));
        assert!(line_ink_end(100.0, 4.0, '.', true) > 100.0);
        assert!(near(line_ink_end(100.0, 4.0, '。', true), 100.0));
    }

    #[test]
    fn composers_break_differently_and_single_line_tightens_first() {
        let widths = [4.0, 4.0, 4.0, 4.0, 10.0];
        let single = break_lines(&widths, 12.0, Composer::SingleLine);
        let every = break_lines(&widths, 12.0, Composer::EveryLine);
        assert_ne!(single, every);
        assert_eq!(single_line_spacing_order()[0], SpacingDirection::Tighten);
        assert_eq!(fit_gap(6.0, 4.0, 2.0, 0.0, 10.0), Some(0.0));
    }

    #[test]
    fn path_text_requires_none_on_a_closed_mask_and_ignores_margins() {
        assert!(!text_follows_mask(true, Some(MaskMode::Add)));
        assert!(text_follows_mask(true, Some(MaskMode::None)));
        assert!(text_follows_mask(false, None));
        assert_eq!(
            margins_used(ParagraphAlignment::Right, false),
            (false, true)
        );
        assert_eq!(margins_used(ParagraphAlignment::Left, false), (true, false));
        assert_eq!(margins_used(ParagraphAlignment::Right, true), (true, true));
    }

    #[test]
    fn more_options_move_anchor_groups() {
        let anchors = [[0.0, 0.0], [10.0, 0.0]];
        let character = anchor_for(AnchorGroup::Character, 1, &anchors, 0..2);
        let word = anchor_for(AnchorGroup::Word, 1, &anchors, 0..2);
        assert!(near(character[0], 10.0));
        assert!(near(word[0], 5.0));
        let caps = group_alignment_offset([0.0, -50.0], 100.0);
        let baseline = group_alignment_offset([0.0, 0.0], 100.0);
        let center = 50.0;
        assert!((caps[1] - center).abs() < (baseline[1] - center).abs());
        let raised = group_alignment_offset([10.0, 10.0], 100.0);
        assert!(raised[0] > 0.0 && raised[1] < 0.0);
        let mut options = MoreOptions::new();
        options.character_blend_enabled = false;
        options.fill_and_stroke_enabled = false;
        assert!(!options.character_blend_enabled && !options.fill_and_stroke_enabled);
    }

    #[test]
    fn new_animator_has_one_range_selector() {
        let animator = TextAnimator::with_default_range("Animator 1");
        assert_eq!(animator.selectors.len(), 1);
        let TextSelector::Range(range) = &animator.selectors[0] else {
            panic!("range")
        };
        assert_eq!(range.mode, SelectorMode::Add);
        let mut quiet = range.clone();
        quiet.amount_percent = 0.0;
        assert!(near(range_influence(&quiet, 0, 4), 0.0));
        quiet.amount_percent = 50.0;
        quiet.shape = SelectorShape::Square;
        quiet.start = 0.0;
        quiet.end = 100.0;
        assert!(near(range_influence(&quiet, 0, 4), 0.5));
        assert_eq!(SelectorShape::ALL.len(), 6);
        assert_eq!(selector_count("a b", SelectorBasedOn::Characters), 3);
        assert!(!animator.selectors.is_empty());
        assert_eq!(selector_count("abcd", SelectorBasedOn::Characters), 4);
        assert_ne!(randomized_order(0, 1, 4), randomized_order(0, 2, 4));
        assert_eq!(randomized_order(7, 1, 4), randomized_order(7, 9, 4));
    }

    #[test]
    fn wiggly_amounts_match_at_full_correlation() {
        let mut selector = WigglySelector {
            max_amount: 1.0,
            min_amount: -1.0,
            wiggles_per_second: 1.0,
            correlation_percent: 100.0,
            temporal_phase: 0.2,
            spatial_phase: 0.0,
            lock_dimensions: false,
            random_seed: 1,
        };
        assert!(near(
            wiggly_amount(&selector, 0, 0.3),
            wiggly_amount(&selector, 3, 0.3)
        ));
        selector.correlation_percent = 0.0;
        assert!(!near(
            wiggly_amount(&selector, 0, 0.3),
            wiggly_amount(&selector, 1, 0.3)
        ));
        assert!(!near(
            wiggly_amount(&selector, 0, 0.0),
            wiggly_amount(&selector, 0, 0.4)
        ));
    }

    #[test]
    fn expression_selector_starts_from_the_documented_formula() {
        let selector = ExpressionSelector::new();
        assert_eq!(
            selector.amount_expression,
            "selectorValue * textIndex/textTotal"
        );
        let indices = text_index_step(4);
        assert_eq!(indices[2] - indices[1], 1);
        assert!(near(
            eval_selector_expression(&selector.amount_expression, 8.0, 2.0, 4.0).unwrap(),
            4.0
        ));
        assert_eq!(selector_name_outside("textIndex"), Err(ExprError::Syntax));
        assert_eq!(selector_name_outside("textTotal"), Err(ExprError::Syntax));
        assert_eq!(
            selector_name_outside("selectorValue"),
            Err(ExprError::Syntax)
        );
    }

    #[test]
    fn animator_anchor_rotates_only_selected_characters() {
        let moved = orbit_character([2.0, 0.0], [0.0, 0.0], 90.0, 1.0);
        let still = orbit_character([2.0, 0.0], [0.0, 0.0], 90.0, 0.0);
        assert!(near(moved[0], 0.0) && near(moved[1], -2.0));
        assert!(near(still[0], 2.0) && near(still[1], 0.0));
    }

    #[test]
    fn animator_position_drag_keeps_z_and_half_amount_halves_motion() {
        let mut xy = [1.0, 2.0];
        let z = OptionalAxis::Set(5.0);
        drag_position(&mut xy, [3.0, -1.0]);
        assert!(near(xy[0], 4.0) && near(xy[1], 1.0));
        assert_eq!(z, OptionalAxis::Set(5.0));
        let half = scale_by_amount([4.0, 2.0], 50.0);
        assert!(near(half[0], 2.0) && near(half[1], 1.0));
    }

    #[test]
    fn scale_z_is_invisible_when_anchor_z_is_zero() {
        let flat = scale_point([1.0, 2.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 3.0]);
        assert!(near(flat[2], 0.0));
        let deep = scale_point([1.0, 2.0, 0.0], [0.0, 0.0, 2.0], [1.0, 1.0, 3.0]);
        assert!(!near(deep[2], 0.0));
    }

    #[test]
    fn skew_tilts_the_character() {
        let tilted = skew_point([2.0, 1.0], 20.0, 0.0);
        assert!(!near(tilted[0], 0.0) || !near(tilted[1], 2.0));
        assert!(near(skew_point([1.0, 1.0], 0.0, 30.0)[0], 1.0));
    }

    #[test]
    fn skew_axis_changes_the_tilt_direction() {
        let horizontal = skew_point([2.0, 1.0], 25.0, 0.0);
        let vertical = skew_point([2.0, 1.0], 25.0, 90.0);
        assert_ne!(horizontal, vertical);
    }

    #[test]
    fn rotation_becomes_z_rotation_when_per_character_3d_turns_on() {
        assert!(can_add_rotation("Rotation", false));
        assert!(!can_add_rotation("X Rotation", false));
        let mut animator = TextAnimator::with_default_range("A");
        animator.properties.push(AnimatorProperty::Rotation(15.0));
        animator.enable_per_character_3d();
        assert!(
            matches!(animator.properties[0], AnimatorProperty::ZRotation(value) if near(value, 15.0))
        );
        assert!(can_add_rotation("Z Rotation", true));
        assert!(!can_add_rotation("Rotation", true));
    }

    #[test]
    fn x_rotation_exists_only_for_per_character_3d() {
        assert!(!can_add_rotation("X Rotation", false));
        assert!(can_add_rotation("X Rotation", true));
    }

    #[test]
    fn y_rotation_exists_only_for_per_character_3d() {
        assert!(!can_add_rotation("Y Rotation", false));
        assert!(can_add_rotation("Y Rotation", true));
    }

    #[test]
    fn z_rotation_matches_the_planar_rotation() {
        let planar = orbit_character([2.0, 0.0], [0.0, 0.0], 90.0, 1.0);
        assert!(near(planar[0], 0.0) && near(planar[1], -2.0));
        assert!(can_add_rotation("Z Rotation", true));
    }

    #[test]
    fn opacity_reveal_runs_from_transparent_to_opaque() {
        assert!(near(reveal_opacity(0, 4, 0.0, 0.0), 0.0));
        assert!(near(reveal_opacity(0, 4, 50.0, 0.0), 1.0));
        assert!(near(reveal_opacity(3, 4, 50.0, 0.0), 0.0));
        assert!(near(reveal_opacity(3, 4, 100.0, 0.0), 1.0));
    }

    #[test]
    fn all_transform_properties_match_adding_them_one_by_one() {
        let mut together = TextAnimator::with_default_range("A");
        together.add_all_transforms();
        let separate = [
            "Anchor Point",
            "Position",
            "Scale",
            "Skew",
            "Rotation",
            "Opacity",
        ];
        assert_eq!(property_names(&together.properties), separate);
    }

    #[test]
    fn fill_color_menu_adds_only_the_chosen_child() {
        assert!(animator_menu(false).contains(&MenuEntry::Submenu("Fill Color")));
        let mut animator = TextAnimator::with_default_range("A");
        animator
            .properties
            .push(AnimatorProperty::FillColor("child-a".into()));
        assert_eq!(animator.properties.len(), 1);
        assert!(
            matches!(&animator.properties[0], AnimatorProperty::FillColor(name) if name == "child-a")
        );
    }

    #[test]
    fn stroke_color_menu_is_separate_from_fill() {
        let menu = animator_menu(false);
        assert!(menu.contains(&MenuEntry::Submenu("Stroke Color")));
        assert!(menu.contains(&MenuEntry::Submenu("Fill Color")));
        let mut animator = TextAnimator::with_default_range("A");
        animator
            .properties
            .push(AnimatorProperty::StrokeColor("stroke-child".into()));
        assert!(animator
            .properties
            .iter()
            .all(|property| !matches!(property, AnimatorProperty::FillColor(_))));
    }

    #[test]
    fn animator_stroke_width_is_not_the_character_width() {
        let mut style = CharacterStyle::new();
        style.stroke_width = Some(2.0);
        let animator_width = AnimatorProperty::StrokeWidth(8.0);
        assert!(near(style.stroke_width.unwrap(), 2.0));
        assert!(matches!(animator_width, AnimatorProperty::StrokeWidth(width) if near(width, 8.0)));
    }

    #[test]
    fn animator_tracking_is_independent_of_character_tracking() {
        let style = CharacterStyle::new();
        assert!(near(style.tracking, 0.0));
        let animator = AnimatorProperty::Tracking(25.0);
        assert!(matches!(animator, AnimatorProperty::Tracking(value) if near(value, 25.0)));
        assert!(near(style.tracking, 0.0));
    }

    #[test]
    fn line_anchor_distributes_from_left_center_and_right() {
        assert!(near(line_anchor_shift(10.0, 0.0), 0.0));
        assert!(near(line_anchor_shift(10.0, 50.0), 5.0));
        assert!(near(line_anchor_shift(10.0, 100.0), 10.0));
    }

    #[test]
    fn animator_line_spacing_is_separate_and_affects_multiple_lines() {
        let leading = 16.0;
        assert!(near(animated_line_gap(leading, 4.0, 1), leading));
        assert!(near(animated_line_gap(leading, 4.0, 3), 20.0));
    }

    #[test]
    fn character_offset_shifts_offset_by_five() {
        assert_eq!(
            offset_text("offset", 5, CharacterRangeMode::AllUnicode),
            "tkkxjy"
        );
        let mut animator = TextAnimator::with_default_range("A");
        animator
            .properties
            .push(AnimatorProperty::CharacterOffset(5));
        assert!(shows_character_range(&animator.properties));
    }

    #[test]
    fn character_value_replaces_every_character_with_a() {
        assert_eq!(replace_with_value("value", 65), "AAAAA");
        let mut animator = TextAnimator::with_default_range("A");
        animator
            .properties
            .push(AnimatorProperty::CharacterValue(65));
        assert!(shows_character_range(&animator.properties));
    }

    #[test]
    fn character_range_is_absent_until_offset_or_value_and_keeps_case() {
        let animator = TextAnimator::with_default_range("A");
        assert!(!shows_character_range(&animator.properties));
        assert_eq!(
            offset_text("A", 32, CharacterRangeMode::PreserveCaseAndDigits),
            "G"
        );
        assert!(
            offset_text("A", 32, CharacterRangeMode::PreserveCaseAndDigits)
                .chars()
                .all(|ch| ch.is_uppercase())
        );
    }

    #[test]
    fn blur_can_be_horizontal_only_inside_the_selector() {
        let (horizontal, vertical) = blur_amount(6.0, 0.0, 1.0);
        assert!(horizontal > 0.0 && near(vertical, 0.0));
        let outside = blur_amount(6.0, 4.0, 0.0);
        assert!(near(outside.0, 0.0) && near(outside.1, 0.0));
    }

    #[test]
    fn per_character_3d_drops_y_rotation_when_returning_to_2d() {
        let mut animator = TextAnimator::with_default_range("A");
        animator.properties.push(AnimatorProperty::Position {
            xy: [1.0, 2.0],
            z: OptionalAxis::Absent,
        });
        animator.properties.push(AnimatorProperty::Rotation(12.0));
        animator.properties.push(AnimatorProperty::YRotation(40.0));
        assert!(!animator.per_character_3d);
        animator.enable_per_character_3d();
        assert!(matches!(
            animator.properties[0],
            AnimatorProperty::Position {
                z: OptionalAxis::PresentUnset,
                ..
            }
        ));
        assert!(
            matches!(animator.properties[1], AnimatorProperty::ZRotation(value) if near(value, 12.0))
        );
        animator.disable_per_character_keep_layer();
        assert!(animator.layer_is_3d && !animator.per_character_3d);
        animator.per_character_3d = true;
        animator.return_to_2d();
        assert!(!animator
            .properties
            .iter()
            .any(|property| matches!(property, AnimatorProperty::YRotation(_))));
        animator.enable_per_character_3d();
        assert!(!animator
            .properties
            .iter()
            .any(|property| matches!(property, AnimatorProperty::YRotation(_))));
        assert!(try_keyframe_per_character_3d().is_err());
        let mut text = Text::point("Ab");
        text.more_options.character_blend_enabled = true;
        text.more_options.fill_and_stroke_enabled = true;
        text.enable_per_character_3d();
        assert!(text.per_character_3d);
        assert!(!text.more_options.character_blend_enabled);
        assert!(!text.more_options.fill_and_stroke_enabled);
    }
}
