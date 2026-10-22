//! Block-level style intent. Resolved cells keep the smaller Copy Style value.
use crate::{Gradient, StylePatch};
use std::{fmt, ops::Range, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Edges<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}
impl<T: Copy> Edges<T> {
    pub const fn all(value: T) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
    pub const fn symmetric(vertical: T, horizontal: T) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
    pub fn map<U>(self, f: impl Fn(T) -> U) -> Edges<U> {
        Edges {
            top: f(self.top),
            right: f(self.right),
            bottom: f(self.bottom),
            left: f(self.left),
        }
    }
}
pub type Insets = Edges<u16>;
impl Insets {
    pub const fn horizontal(self) -> u32 {
        self.left as u32 + self.right as u32
    }
    pub const fn vertical(self) -> u32 {
        self.top as u32 + self.bottom as u32
    }
}
impl Edges<Option<u16>> {
    pub fn resolved(self) -> Insets {
        self.map(|n| n.unwrap_or(0))
    }
}

/// Rounding puts an odd spare cell after the content, consistently on both axes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Alignment {
    #[default]
    Start,
    Center,
    End,
}
impl Alignment {
    pub const fn offset(self, spare: usize) -> usize {
        match self {
            Self::Start => 0,
            Self::Center => spare / 2,
            Self::End => spare,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecorationError {
    ControlInBorder { part: BorderPart },
    ZeroWidthBorder { part: BorderPart },
    WidthOverflow,
}
impl fmt::Display for DecorationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ControlInBorder { part } => {
                write!(f, "border {part:?} contains a control character")
            }
            Self::ZeroWidthBorder { part } => {
                write!(f, "border {part:?} contains a zero-width grapheme")
            }
            Self::WidthOverflow => f.write_str("border glyph width exceeds u16 cells"),
        }
    }
}
impl std::error::Error for DecorationError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BorderPart {
    Top,
    Right,
    Bottom,
    Left,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    MiddleLeft,
    MiddleRight,
    Middle,
    MiddleTop,
    MiddleBottom,
}
impl BorderPart {
    pub const ALL: [Self; 13] = [
        Self::Top,
        Self::Right,
        Self::Bottom,
        Self::Left,
        Self::TopLeft,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomRight,
        Self::MiddleLeft,
        Self::MiddleRight,
        Self::Middle,
        Self::MiddleTop,
        Self::MiddleBottom,
    ];
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BorderPreset {
    None,
    Plain,
    #[default]
    Rounded,
    Ascii,
    Thick,
    Double,
    Block,
    OuterHalfBlock,
    InnerHalfBlock,
    Hidden,
    Markdown,
}
impl BorderPreset {
    fn parts(self) -> [&'static str; 13] {
        match self {
            Self::None => [""; 13],
            Self::Plain => [
                "─", "│", "─", "│", "┌", "┐", "└", "┘", "├", "┤", "┼", "┬", "┴",
            ],
            Self::Rounded => [
                "─", "│", "─", "│", "╭", "╮", "╰", "╯", "├", "┤", "┼", "┬", "┴",
            ],
            Self::Ascii => [
                "-", "|", "-", "|", "+", "+", "+", "+", "+", "+", "+", "+", "+",
            ],
            Self::Thick => [
                "━", "┃", "━", "┃", "┏", "┓", "┗", "┛", "┣", "┫", "╋", "┳", "┻",
            ],
            Self::Double => [
                "═", "║", "═", "║", "╔", "╗", "╚", "╝", "╠", "╣", "╬", "╦", "╩",
            ],
            Self::Block => ["█"; 13],
            Self::OuterHalfBlock => ["▀", "▐", "▄", "▌", "▛", "▜", "▙", "▟", "", "", "", "", ""],
            Self::InnerHalfBlock => ["▄", "▌", "▀", "▐", "▗", "▖", "▝", "▘", "", "", "", "", ""],
            Self::Hidden => [" "; 13],
            Self::Markdown => [
                "-", "|", "-", "|", "|", "|", "|", "|", "|", "|", "|", "|", "|",
            ],
        }
    }
}
#[derive(Clone, Debug)]
struct Glyph {
    range: Range<usize>,
    width: u16,
}
#[derive(Clone, Debug)]
struct Pattern {
    text: Arc<str>,
    glyphs: Box<[Glyph]>,
    width: u16,
}
#[derive(Clone, Debug)]
enum BorderData {
    Preset(BorderPreset),
    Custom(Arc<[Pattern; 13]>),
}

/// Immutable validated glyph patterns. Built-ins allocate nothing; custom
/// patterns own their text/segmentation once and share it on Clone. Corners
/// render the first whole grapheme; side patterns cycle through whole clusters.
#[derive(Clone, Debug)]
pub struct BorderSet(BorderData);
impl Default for BorderSet {
    fn default() -> Self {
        Self::preset(BorderPreset::Rounded)
    }
}
impl From<BorderPreset> for BorderSet {
    fn from(preset: BorderPreset) -> Self {
        Self::preset(preset)
    }
}
impl From<crate::Border> for BorderSet {
    fn from(border: crate::Border) -> Self {
        Self::preset(match border {
            crate::Border::Rounded => BorderPreset::Rounded,
            crate::Border::Plain => BorderPreset::Plain,
            crate::Border::Ascii => BorderPreset::Ascii,
        })
    }
}
impl BorderSet {
    pub const fn preset(preset: BorderPreset) -> Self {
        Self(BorderData::Preset(preset))
    }
    /// Parts use BorderPart::ALL order. Empty parts are valid and render a
    /// space when their side is enabled. Controls and zero-width clusters are
    /// rejected before publishing any object; input strings are never mutated.
    pub fn custom(parts: [&str; 13]) -> Result<Self, DecorationError> {
        // Validate the full borrowed input before allocating owned text/cache.
        for (part, text) in BorderPart::ALL.into_iter().zip(parts) {
            if text.chars().any(char::is_control) {
                return Err(DecorationError::ControlInBorder { part });
            }
            for glyph in text.graphemes(true) {
                if glyph.width() == 0 {
                    return Err(DecorationError::ZeroWidthBorder { part });
                }
                if glyph.width() > usize::from(u16::MAX) {
                    return Err(DecorationError::WidthOverflow);
                }
            }
        }
        let patterns = parts.map(|text| {
            let glyphs: Box<[_]> = text
                .grapheme_indices(true)
                .map(|(start, glyph)| Glyph {
                    range: start..start + glyph.len(),
                    width: glyph.width() as u16,
                })
                .collect();
            let width = glyphs.iter().map(|g| g.width).max().unwrap_or(0);
            Pattern {
                text: Arc::from(text),
                glyphs,
                width,
            }
        });
        Ok(Self(BorderData::Custom(Arc::new(patterns))))
    }
    pub fn part(&self, part: BorderPart) -> &str {
        match &self.0 {
            BorderData::Preset(p) => p.parts()[part as usize],
            BorderData::Custom(parts) => &parts[part as usize].text,
        }
    }
    pub fn part_width(&self, part: BorderPart) -> u16 {
        match &self.0 {
            BorderData::Preset(_) => u16::from(!self.part(part).is_empty()),
            BorderData::Custom(parts) => parts[part as usize].width,
        }
    }
    pub fn is_empty(&self) -> bool {
        BorderPart::ALL.into_iter().all(|p| self.part(p).is_empty())
    }
    pub fn glyph_at(&self, part: BorderPart, index: usize) -> (&str, u16) {
        match &self.0 {
            BorderData::Preset(_) => {
                let text = self.part(part);
                (if text.is_empty() { " " } else { text }, 1)
            }
            BorderData::Custom(parts) => {
                let p = &parts[part as usize];
                if p.glyphs.is_empty() {
                    return (" ", 1);
                }
                let glyph = &p.glyphs[index % p.glyphs.len()];
                (&p.text[glyph.range.clone()], glyph.width)
            }
        }
    }
}

/// Explicit conversion of HT before normalized structured text is created.
/// Arbitrary raw terminal tabs are not stored in display cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabWidth {
    Spaces(u16),
    Remove,
}
impl Default for TabWidth {
    fn default() -> Self {
        Self::Spaces(4)
    }
}

/// Shared, capturing, thread-safe text transformation. The renderer sanitizes
/// its result before segmentation. User panics propagate without a partial grid.
#[derive(Clone)]
pub struct TextTransform(Arc<dyn Fn(&str) -> String + Send + Sync>);
impl TextTransform {
    pub fn new(f: impl Fn(&str) -> String + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }
    pub fn apply(&self, text: &str) -> String {
        (self.0)(text)
    }
}
impl fmt::Debug for TextTransform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TextTransform(..)")
    }
}

/// Decoration intent outside the compact cell Style. Public optional fields
/// also provide getters and explicit unsetting. Width/height include padding
/// and border, exclude margin, and are minima. Maxima crop the final block.
///
/// ```
/// use matea_style::{BlockStyle, Insets, BorderPreset};
/// let style = BlockStyle::default().padding(Insets::symmetric(1, 2))
///     .border(BorderPreset::Rounded).width(20);
/// assert_eq!(style.padding.resolved().horizontal(), 4);
/// assert_eq!(style.width, Some(20));
/// ```
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
#[must_use]
pub struct BlockStyle {
    pub text: StylePatch,
    pub padding: Edges<Option<u16>>,
    pub margin: Edges<Option<u16>>,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub max_width: Option<u16>,
    pub max_height: Option<u16>,
    pub horizontal: Option<Alignment>,
    pub vertical: Option<Alignment>,
    pub border: Option<BorderSet>,
    pub border_sides: Edges<Option<bool>>,
    pub border_style: Edges<StylePatch>,
    pub border_gradient: Option<Arc<Gradient>>,
    pub border_gradient_offset: Option<i64>,
    pub margin_style: StylePatch,
    pub padding_char: Option<char>,
    pub margin_char: Option<char>,
    pub color_whitespace: Option<bool>,
    pub underline_spaces: Option<bool>,
    pub strikethrough_spaces: Option<bool>,
    pub inline: Option<bool>,
    pub tab_width: Option<TabWidth>,
    pub transform: Option<TextTransform>,
}
impl BlockStyle {
    pub fn padding(mut self, insets: Insets) -> Self {
        self.padding = insets.map(Some);
        self
    }
    pub fn margin(mut self, insets: Insets) -> Self {
        self.margin = insets.map(Some);
        self
    }
    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }
    pub fn height(mut self, height: u16) -> Self {
        self.height = Some(height);
        self
    }
    pub fn max_width(mut self, width: u16) -> Self {
        self.max_width = Some(width);
        self
    }
    pub fn max_height(mut self, height: u16) -> Self {
        self.max_height = Some(height);
        self
    }
    pub fn border(mut self, border: impl Into<BorderSet>) -> Self {
        self.border = Some(border.into());
        self
    }
    pub fn align(mut self, horizontal: Alignment, vertical: Alignment) -> Self {
        self.horizontal = Some(horizontal);
        self.vertical = Some(vertical);
        self
    }
    pub fn text_style(mut self, patch: impl Into<StylePatch>) -> Self {
        self.text = patch.into();
        self
    }
    pub fn sides(&self) -> Edges<bool> {
        let explicit = [
            self.border_sides.top,
            self.border_sides.right,
            self.border_sides.bottom,
            self.border_sides.left,
        ]
        .iter()
        .any(Option::is_some);
        if self.border.as_ref().is_none_or(BorderSet::is_empty) {
            return Edges::all(false);
        }
        self.border_sides.map(|v| v.unwrap_or(!explicit))
    }
    /// Child intent wins. Padding/margin amounts are never inherited; characters
    /// and the remaining properties inherit. Explicit false/zero/reset survive.
    pub fn inherit(mut self, parent: &Self) -> Self {
        self.text = parent.text.then(self.text);
        self.margin_style = parent.margin_style.then(self.margin_style);
        if self.margin_style.bg.is_none() {
            self.margin_style.bg = parent.text.bg;
        }
        macro_rules! fill {($($field:ident),*)=>{$(if self.$field.is_none(){self.$field=parent.$field.clone();})*};}
        fill!(
            width,
            height,
            max_width,
            max_height,
            horizontal,
            vertical,
            border,
            border_gradient,
            border_gradient_offset,
            padding_char,
            margin_char,
            color_whitespace,
            underline_spaces,
            strikethrough_spaces,
            inline,
            tab_width,
            transform
        );
        self.border_sides = Edges {
            top: self.border_sides.top.or(parent.border_sides.top),
            right: self.border_sides.right.or(parent.border_sides.right),
            bottom: self.border_sides.bottom.or(parent.border_sides.bottom),
            left: self.border_sides.left.or(parent.border_sides.left),
        };
        self.border_style = Edges {
            top: parent.border_style.top.then(self.border_style.top),
            right: parent.border_style.right.then(self.border_style.right),
            bottom: parent.border_style.bottom.then(self.border_style.bottom),
            left: parent.border_style.left.then(self.border_style.left),
        };
        self
    }
    /// Border extents: top/bottom consume rows; side patterns reserve their
    /// largest grapheme. Disabled sides and an empty BorderSet consume nothing.
    pub fn border_insets(&self) -> Insets {
        let sides = self.sides();
        let Some(border) = &self.border else {
            return Insets::default();
        };
        Insets {
            top: u16::from(sides.top),
            bottom: u16::from(sides.bottom),
            left: if sides.left {
                border
                    .part_width(BorderPart::Left)
                    .max(border.part_width(BorderPart::TopLeft))
                    .max(border.part_width(BorderPart::BottomLeft))
                    .max(1)
            } else {
                0
            },
            right: if sides.right {
                border
                    .part_width(BorderPart::Right)
                    .max(border.part_width(BorderPart::TopRight))
                    .max(border.part_width(BorderPart::BottomRight))
                    .max(1)
            } else {
                0
            },
        }
    }
    /// Full frame overhead, including margins. u32 preserves all u16 sums.
    pub fn frame_size(&self) -> (u32, u32) {
        if self.inline == Some(true) {
            return (0, 0);
        }
        let (p, m, b) = (
            self.padding.resolved(),
            self.margin.resolved(),
            self.border_insets(),
        );
        (
            p.horizontal() + m.horizontal() + b.horizontal(),
            p.vertical() + m.vertical() + b.vertical(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Style, TextAttribute};

    #[test]
    fn explicit_zero_false_and_reset_inherit_without_padding_or_margin() {
        let mut parent = BlockStyle::default()
            .padding(Insets::all(9))
            .margin(Insets::all(7))
            .width(20)
            .height(6)
            .border(BorderPreset::Double)
            .text_style(
                StylePatch::new()
                    .fg(Color::Indexed(2))
                    .bg(Color::Indexed(3))
                    .attribute(TextAttribute::Bold, true),
            );
        parent.inline = Some(true);
        parent.padding_char = Some('.');
        parent.margin_char = Some('!');
        parent.border_sides.left = Some(true);
        parent.border_style.top = StylePatch::new().fg(Color::Indexed(4));
        parent.transform = Some(TextTransform::new(|s| format!("<{s}>")));
        let mut child = BlockStyle::default().width(0).text_style(
            StylePatch::new()
                .fg(Color::Default)
                .attribute(TextAttribute::Bold, false),
        );
        child.inline = Some(false);
        child.border_sides.left = Some(false);
        child.border_style.top = StylePatch::new().bg(Color::Indexed(5));
        let child = child.inherit(&parent);
        assert_eq!(child.width, Some(0));
        assert_eq!(child.height, Some(6));
        assert_eq!(child.inline, Some(false));
        assert_eq!(child.padding, Edges::default());
        assert_eq!(child.margin, Edges::default());
        assert_eq!(child.padding_char, Some('.'));
        assert_eq!(child.margin_char, Some('!'));
        assert_eq!(child.margin_style.bg, Some(Color::Indexed(3)));
        assert_eq!(child.text.resolve(Style::default()).fg, Color::Default);
        assert!(
            !child
                .text
                .resolve(Style::default())
                .has(TextAttribute::Bold)
        );
        let edge = child.border_style.top.resolve(Style::default());
        assert_eq!((edge.fg, edge.bg), (Color::Indexed(4), Color::Indexed(5)));
        assert_eq!(child.sides(), Edges::all(false));
        assert_eq!(child.transform.unwrap().apply("tea"), "<tea>");
        assert_eq!(parent.width, Some(20));
    }

    #[test]
    fn sides_presence_frame_arithmetic_and_unsetting() {
        let mut block = BlockStyle::default()
            .border(BorderPreset::Rounded)
            .padding(Insets::all(u16::MAX))
            .margin(Insets::all(u16::MAX));
        assert_eq!(block.sides(), Edges::all(true));
        assert_eq!(block.frame_size(), (262142, 262142));
        block.border_sides.top = Some(true);
        assert_eq!(
            block.sides(),
            Edges {
                top: true,
                ..Edges::all(false)
            }
        );
        assert_eq!(
            block.border_insets(),
            Insets {
                top: 1,
                ..Insets::default()
            }
        );
        block.border_sides.top = None;
        assert_eq!(block.sides(), Edges::all(true));
        block.inline = Some(true);
        assert_eq!(block.frame_size(), (0, 0));
        block.border = None;
        assert_eq!(block.sides(), Edges::all(false));
        block.border = Some(BorderSet::preset(BorderPreset::None));
        assert_eq!(block.sides(), Edges::all(false));
        for spare in 0..9 {
            assert_eq!(Alignment::Start.offset(spare), 0);
            assert_eq!(Alignment::Center.offset(spare), spare / 2);
            assert_eq!(Alignment::End.offset(spare), spare);
        }
    }

    #[test]
    fn custom_borders_validate_all_parts_and_share_whole_graphemes() {
        let mut parts = [""; 13];
        for part in BorderPart::ALL {
            for bad in ["\n", "\r", "\t", "\u{1b}[31m", "\u{7f}", "\u{85}"] {
                parts[part as usize] = bad;
                assert_eq!(
                    BorderSet::custom(parts).unwrap_err(),
                    DecorationError::ControlInBorder { part }
                );
            }
            for bad in ["\u{301}", "\u{200d}", "\u{fe0f}"] {
                parts[part as usize] = bad;
                assert_eq!(
                    BorderSet::custom(parts).unwrap_err(),
                    DecorationError::ZeroWidthBorder { part }
                );
            }
            parts[part as usize] = "";
        }
        parts[BorderPart::Top as usize] = "茶e\u{301}";
        parts[BorderPart::Left as usize] = "👩‍💻x";
        parts[BorderPart::TopLeft as usize] = "e\u{301}more";
        let border = BorderSet::custom(parts).unwrap();
        assert_eq!(border.glyph_at(BorderPart::Top, 0), ("茶", 2));
        assert_eq!(border.glyph_at(BorderPart::Top, 1), ("e\u{301}", 1));
        assert_eq!(
            border.glyph_at(BorderPart::Top, usize::MAX),
            ("e\u{301}", 1)
        );
        assert_eq!(border.glyph_at(BorderPart::Left, 0), ("👩‍💻", 2));
        assert_eq!(border.glyph_at(BorderPart::TopLeft, 0), ("e\u{301}", 1));
        assert_eq!(border.glyph_at(BorderPart::Bottom, 0), (" ", 1));
        assert_eq!(border.part_width(BorderPart::Top), 2);
        let clone = border.clone();
        let (BorderData::Custom(a), BorderData::Custom(b)) = (&border.0, &clone.0) else {
            panic!()
        };
        assert!(Arc::ptr_eq(a, b));
        assert!(!border.is_empty());
        assert!(BorderSet::custom([""; 13]).unwrap().is_empty());
        let style = BlockStyle {
            border: Some(border),
            ..BlockStyle::default()
        };
        assert_eq!(
            style.border_insets(),
            Insets {
                top: 1,
                right: 1,
                bottom: 1,
                left: 2
            }
        );
        for error in [
            DecorationError::WidthOverflow,
            DecorationError::ZeroWidthBorder {
                part: BorderPart::Top,
            },
            DecorationError::ControlInBorder {
                part: BorderPart::Bottom,
            },
        ] {
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn presets_cover_all_thirteen_parts_and_legacy_border_conversion() {
        let rounded = BorderSet::default();
        assert_eq!(rounded.part(BorderPart::Middle), "┼");
        assert_eq!(rounded.part(BorderPart::MiddleLeft), "├");
        assert_eq!(rounded.part(BorderPart::MiddleRight), "┤");
        assert_eq!(rounded.part(BorderPart::MiddleTop), "┬");
        assert_eq!(rounded.part(BorderPart::MiddleBottom), "┴");
        for (old, preset) in [
            (crate::Border::Plain, BorderPreset::Plain),
            (crate::Border::Rounded, BorderPreset::Rounded),
            (crate::Border::Ascii, BorderPreset::Ascii),
        ] {
            let old = BorderSet::from(old);
            let expected = BorderSet::preset(preset);
            for part in BorderPart::ALL {
                assert_eq!(old.part(part), expected.part(part));
                assert_eq!(old.part_width(part), 1);
            }
        }
        assert_eq!(TabWidth::default(), TabWidth::Spaces(4));
        let suffix = String::from("!");
        let transform = TextTransform::new(move |text| format!("{text}{suffix}"));
        assert_eq!(transform.clone().apply("tea"), "tea!");
        assert_eq!(format!("{transform:?}"), "TextTransform(..)");
        fn send_sync<T: Send + Sync>() {}
        send_sync::<BlockStyle>();
        send_sync::<BorderSet>();
    }
}
