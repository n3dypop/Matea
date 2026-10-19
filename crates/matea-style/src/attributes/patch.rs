//! Explicit style intent, separate from the compact resolved style in cells.
use super::{BlinkStyle, TextAttribute, TextAttributes, TextFrame, UnderlineStyle};
use crate::{Color, Style};
use std::fmt;

const UNDERLINE: u16 = 7 << 8;
const BLINK: u16 = 3 << 11;
const FRAME: u16 = 3 << 13;
const ALL: u16 = (1 << 15) - 1;

/// An owned style difference. None colors and unspecified attributes inherit;
/// Some(Color::Default) resets a color. False and the decoration enums' None
/// variants explicitly disable attributes; absent intent still inherits.
/// Compose with `then`, then resolve against a theme or another resolved Style.
/// Every cell keeps the existing compact Style representation.
///
/// ```
/// use matea_style::{Color, Style, StylePatch, TextAttribute};
/// let base = Style::default().fg(Color::Indexed(2)).bold();
/// let highlight = StylePatch::new().bg(Color::Indexed(4))
///     .attribute(TextAttribute::Bold, false);
/// let selected = highlight.then(StylePatch::new().fg(Color::Default));
/// let style = base.patched(selected);
/// assert_eq!(style.fg, Color::Default);
/// assert_eq!(style.bg, Color::Indexed(4));
/// assert!(!style.has(TextAttribute::Bold));
/// ```
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[must_use = "style patches are values; use the returned patch"]
pub struct StylePatch {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub underline_color: Option<Color>,
    mask: u16,
    values: u16,
}
impl StylePatch {
    #[inline]
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            underline_color: None,
            mask: 0,
            values: 0,
        }
    }
    /// Explicit terminal defaults for all colors and every attribute group.
    #[inline]
    pub const fn reset() -> Self {
        Self {
            fg: Some(Color::Default),
            bg: Some(Color::Default),
            underline_color: Some(Color::Default),
            mask: ALL,
            values: 0,
        }
    }
    /// Treat every property of a resolved style as an explicit override.
    #[inline]
    pub const fn from_style(style: Style) -> Self {
        Self {
            fg: Some(style.fg),
            bg: Some(style.bg),
            underline_color: Some(style.underline_color),
            mask: ALL,
            values: style.attributes.bits,
        }
    }
    pub const fn is_empty(self) -> bool {
        self.fg.is_none() && self.bg.is_none() && self.underline_color.is_none() && self.mask == 0
    }
    pub const fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }
    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }
    pub const fn underline_color(mut self, color: Color) -> Self {
        self.underline_color = Some(color);
        self
    }
    pub const fn inherit_fg(mut self) -> Self {
        self.fg = None;
        self
    }
    pub const fn inherit_bg(mut self) -> Self {
        self.bg = None;
        self
    }
    pub const fn inherit_underline_color(mut self) -> Self {
        self.underline_color = None;
        self
    }
    pub const fn attribute(self, attribute: TextAttribute, enabled: bool) -> Self {
        let bit = 1 << attribute as u8;
        self.group(bit, if enabled { bit } else { 0 })
    }
    pub const fn inherit_attribute(self, attribute: TextAttribute) -> Self {
        self.inherit_group(1 << attribute as u8)
    }
    pub const fn attribute_intent(self, attribute: TextAttribute) -> Option<bool> {
        let bit = 1 << attribute as u8;
        if self.mask & bit == 0 {
            None
        } else {
            Some(self.values & bit != 0)
        }
    }
    pub const fn underline_style(self, style: UnderlineStyle) -> Self {
        self.group(UNDERLINE, (style as u16) << 8)
    }
    pub const fn blink(self, style: BlinkStyle) -> Self {
        self.group(BLINK, (style as u16) << 11)
    }
    pub const fn text_frame(self, style: TextFrame) -> Self {
        self.group(FRAME, (style as u16) << 13)
    }
    pub const fn inherit_underline(self) -> Self {
        self.inherit_group(UNDERLINE)
    }
    pub const fn inherit_blink(self) -> Self {
        self.inherit_group(BLINK)
    }
    pub const fn inherit_frame(self) -> Self {
        self.inherit_group(FRAME)
    }
    pub const fn underline(self) -> Option<UnderlineStyle> {
        if self.mask & UNDERLINE == 0 {
            None
        } else {
            Some(TextAttributes { bits: self.values }.underline())
        }
    }
    pub const fn blink_style(self) -> Option<BlinkStyle> {
        if self.mask & BLINK == 0 {
            None
        } else {
            Some(TextAttributes { bits: self.values }.blink())
        }
    }
    pub const fn frame(self) -> Option<TextFrame> {
        if self.mask & FRAME == 0 {
            None
        } else {
            Some(TextAttributes { bits: self.values }.frame())
        }
    }
    const fn group(mut self, mask: u16, values: u16) -> Self {
        self.mask |= mask;
        self.values = (self.values & !mask) | values;
        self
    }
    const fn inherit_group(mut self, mask: u16) -> Self {
        self.mask &= !mask;
        self.values &= !mask;
        self
    }
    /// Sequential application, with every explicit overlay setting winning.
    /// This composes intent; unspecified settings never erase previous intent.
    #[inline]
    pub const fn then(self, overlay: Self) -> Self {
        Self {
            fg: match overlay.fg {
                Some(_) => overlay.fg,
                None => self.fg,
            },
            bg: match overlay.bg {
                Some(_) => overlay.bg,
                None => self.bg,
            },
            underline_color: match overlay.underline_color {
                Some(_) => overlay.underline_color,
                None => self.underline_color,
            },
            mask: self.mask | overlay.mask,
            values: (self.values & !overlay.mask) | overlay.values,
        }
    }
    /// Produce a resolved cell style. No allocation, terminal I/O or mutation of
    /// either input occurs; typed setters preserve valid packed enum groups.
    #[inline]
    pub const fn resolve(self, base: Style) -> Style {
        Style {
            fg: match self.fg {
                Some(value) => value,
                None => base.fg,
            },
            bg: match self.bg {
                Some(value) => value,
                None => base.bg,
            },
            underline_color: match self.underline_color {
                Some(value) => value,
                None => base.underline_color,
            },
            attributes: TextAttributes {
                bits: (base.attributes.bits & !self.mask) | self.values,
            },
        }
    }
}
impl From<Style> for StylePatch {
    fn from(style: Style) -> Self {
        Self::from_style(style)
    }
}
impl From<Color> for StylePatch {
    fn from(color: Color) -> Self {
        Self::new().fg(color)
    }
}
impl fmt::Debug for StylePatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut value = f.debug_struct("StylePatch");
        if let Some(color) = self.fg {
            value.field("fg", &color);
        }
        if let Some(color) = self.bg {
            value.field("bg", &color);
        }
        if let Some(color) = self.underline_color {
            value.field("underline_color", &color);
        }
        for attribute in TextAttribute::ALL {
            if let Some(intent) = self.attribute_intent(attribute) {
                let name = match attribute {
                    TextAttribute::Bold => "bold",
                    TextAttribute::Dim => "dim",
                    TextAttribute::Italic => "italic",
                    TextAttribute::Reverse => "reverse",
                    TextAttribute::Hidden => "hidden",
                    TextAttribute::Strikethrough => "strikethrough",
                    TextAttribute::Fraktur => "fraktur",
                    TextAttribute::Overline => "overline",
                };
                value.field(name, &intent);
            }
        }
        if let Some(style) = self.underline() {
            value.field("underline", &style);
        }
        if let Some(style) = self.blink_style() {
            value.field("blink", &style);
        }
        if let Some(style) = self.frame() {
            value.field("frame", &style);
        }
        value.finish()
    }
}
