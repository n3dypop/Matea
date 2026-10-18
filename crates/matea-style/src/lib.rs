#![forbid(unsafe_code)]
//! Portable design tokens and deterministic integer layout. No terminal I/O.
mod attributes;
pub use attributes::{
    BlinkStyle, StylePatch, TextAttribute, TextAttributes, TextFrame, UnderlineStyle,
};
mod gradient;
pub use gradient::{Gradient, GradientColors, GradientError, GradientGrid, GradientGridColors};

/// Semantic terminal color. Indexed values use the xterm palette.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Color {
    #[default]
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorLevel {
    None,
    #[default]
    Ansi16,
    Ansi256,
    TrueColor,
}

const ANSI: [(u8, u8, u8); 16] = [
    (0, 0, 0),
    (128, 0, 0),
    (0, 128, 0),
    (128, 128, 0),
    (0, 0, 128),
    (128, 0, 128),
    (0, 128, 128),
    (192, 192, 192),
    (128, 128, 128),
    (255, 0, 0),
    (0, 255, 0),
    (255, 255, 0),
    (0, 0, 255),
    (255, 0, 255),
    (0, 255, 255),
    (255, 255, 255),
];

fn palette(n: u8) -> (u8, u8, u8) {
    match n {
        0..=15 => ANSI[n as usize],
        16..=231 => {
            let n = n - 16;
            let ramp = [0, 95, 135, 175, 215, 255];
            (
                ramp[(n / 36) as usize],
                ramp[(n / 6 % 6) as usize],
                ramp[(n % 6) as usize],
            )
        }
        _ => {
            let g = 8 + (n - 232) * 10;
            (g, g, g)
        }
    }
}

fn distance((r, g, b): (u8, u8, u8), (a, c, d): (u8, u8, u8)) -> u32 {
    let dr = i32::from(r) - i32::from(a);
    let dg = i32::from(g) - i32::from(c);
    let db = i32::from(b) - i32::from(d);
    (dr * dr + dg * dg + db * db) as u32
}

/// Nearest cube channel, breaking exact midpoint ties toward the smaller index.
fn cube_channel(n: u8) -> u8 {
    match n {
        0..=47 => 0,
        48..=115 => 1,
        116..=155 => 2,
        156..=195 => 3,
        196..=235 => 4,
        _ => 5,
    }
}

impl Color {
    /// Quantize by RGB distance; actual ANSI16 palette remains terminal configurable.
    pub fn adapt(self, level: ColorLevel) -> Self {
        if level == ColorLevel::None {
            return Self::Default;
        }
        if self == Self::Default || level == ColorLevel::TrueColor {
            return self;
        }
        if let Self::Indexed(n) = self
            && (level == ColorLevel::Ansi256 || n < 16)
        {
            return self;
        }
        let rgb = match self {
            Self::Rgb(r, g, b) => (r, g, b),
            Self::Indexed(n) => palette(n),
            Self::Default => unreachable!(),
        };
        // The configurable ANSI16 prefix must participate, including duplicates
        // of cube/gray colors. Strict improvement preserves the first-index tie.
        let mut nearest = 0u8;
        let mut best = u32::MAX;
        for (n, &candidate) in ANSI.iter().enumerate() {
            let d = distance(rgb, candidate);
            if d < best {
                best = d;
                nearest = n as u8;
            }
        }
        if level == ColorLevel::Ansi16 || best == 0 {
            return Self::Indexed(nearest);
        }
        // Squared RGB distance separates by channel, so only one of the 216
        // cube points can improve the winner (ties choose the lower channel).
        let (r, g, b) = rgb;
        let cube = 16 + 36 * cube_channel(r) + 6 * cube_channel(g) + cube_channel(b);
        let d = distance(rgb, palette(cube));
        if d < best {
            best = d;
            nearest = cube;
        }
        if best != 0 {
            // Minimize (r-v)^2+(g-v)^2+(b-v)^2 for v=8+10k.
            // k is nearest to (r+g+b-24)/30; +14 rounds exact ties down.
            let sum = u16::from(r) + u16::from(g) + u16::from(b);
            let gray = 232 + (sum.saturating_sub(10) / 30).min(23) as u8;
            if distance(rgb, palette(gray)) < best {
                nearest = gray;
            }
        }
        Self::Indexed(nearest)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[must_use = "styles are values; use the returned style"]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub underline_color: Color,
    pub attributes: TextAttributes,
}

impl Style {
    /// Apply explicit settings while preserving properties the patch inherits.
    pub const fn patched(self, patch: StylePatch) -> Self {
        patch.resolve(self)
    }
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = color;
        self
    }
    pub fn bold(mut self) -> Self {
        self.attributes = self.attributes.with(TextAttribute::Bold);
        self
    }
    pub fn underline(mut self) -> Self {
        self.attributes = self.attributes.with_underline(UnderlineStyle::Single);
        self
    }
    pub fn reverse(mut self) -> Self {
        self.attributes = self.attributes.with(TextAttribute::Reverse);
        self
    }
    #[inline]
    pub fn attribute(mut self, attribute: TextAttribute, enabled: bool) -> Self {
        self.attributes = self.attributes.set(attribute, enabled);
        self
    }
    #[inline]
    pub fn has(self, attribute: TextAttribute) -> bool {
        self.attributes.contains(attribute)
    }
    pub fn dim(self) -> Self {
        self.attribute(TextAttribute::Dim, true)
    }
    pub fn italic(self) -> Self {
        self.attribute(TextAttribute::Italic, true)
    }
    pub fn hidden(self) -> Self {
        self.attribute(TextAttribute::Hidden, true)
    }
    pub fn strikethrough(self) -> Self {
        self.attribute(TextAttribute::Strikethrough, true)
    }
    pub fn fraktur(self) -> Self {
        self.attribute(TextAttribute::Fraktur, true)
    }
    pub fn overline(self) -> Self {
        self.attribute(TextAttribute::Overline, true)
    }
    pub fn underline_style(mut self, style: UnderlineStyle) -> Self {
        self.attributes = self.attributes.with_underline(style);
        self
    }
    pub fn blink(mut self, style: BlinkStyle) -> Self {
        self.attributes = self.attributes.with_blink(style);
        self
    }
    pub fn text_frame(mut self, style: TextFrame) -> Self {
        self.attributes = self.attributes.with_frame(style);
        self
    }
    pub fn underline_color(mut self, color: Color) -> Self {
        self.underline_color = color;
        self
    }
    #[inline]
    pub fn adapt(mut self, level: ColorLevel) -> Self {
        match level {
            ColorLevel::None => {
                self.fg = Color::Default;
                self.bg = Color::Default;
                self.underline_color = Color::Default;
            }
            ColorLevel::TrueColor => {}
            _ => {
                self.fg = self.fg.adapt(level);
                self.bg = self.bg.adapt(level);
                self.underline_color = self.underline_color.adapt(level);
            }
        }
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Border {
    #[default]
    Rounded,
    Plain,
    Ascii,
}

/// Explicit design system passed through views and components.
#[derive(Clone, Debug)]
pub struct Theme {
    pub text: Style,
    pub muted: Style,
    pub accent: Style,
    pub surface: Style,
    pub border: Style,
    pub error: Style,
    pub focus: Style,
    pub gap: u16,
    pub padding: u16,
    pub border_kind: Border,
    pub reduced_motion: bool,
}

impl Default for Theme {
    fn default() -> Self {
        let base = Style::default().bg(Color::Rgb(23, 31, 24));
        let text = base.fg(Color::Rgb(229, 235, 223));
        Self {
            text,
            muted: base.fg(Color::Rgb(160, 176, 150)),
            accent: base.fg(Color::Rgb(175, 215, 135)).bold(),
            surface: text,
            border: base.fg(Color::Rgb(116, 145, 92)),
            error: base.fg(Color::Rgb(255, 150, 140)).underline(),
            focus: text.reverse().bold(),
            gap: 1,
            padding: 1,
            border_kind: Border::Rounded,
            reduced_motion: false,
        }
    }
}

impl Theme {
    pub fn plain() -> Self {
        let s = Style::default();
        Self {
            text: s,
            muted: s,
            accent: s.bold(),
            surface: s,
            border: s,
            error: s.underline(),
            focus: s.reverse(),
            border_kind: Border::Ascii,
            reduced_motion: true,
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Length {
    Fixed(u16),
    Fill(u16),
}

/// Allocate fixed cells first, then weighted fill. Rounding remainder goes left to right.
pub fn distribute(total: u16, lengths: &[Length]) -> Vec<u16> {
    let mut remaining = total;
    let mut out = vec![0; lengths.len()];
    for (i, l) in lengths.iter().enumerate() {
        if let Length::Fixed(n) = l {
            out[i] = (*n).min(remaining);
            remaining -= out[i];
        }
    }
    let mut weight: u64 = lengths
        .iter()
        .map(|l| match l {
            Length::Fill(n) => *n as u64,
            _ => 0,
        })
        .sum();
    for (i, l) in lengths.iter().enumerate() {
        if let Length::Fill(n) = l {
            if weight > 0 {
                out[i] = (u64::from(remaining) * u64::from(*n)).div_ceil(weight) as u16;
            }
            remaining -= out[i];
            weight -= u64::from(*n);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_rgb_tokens_have_readable_contrast() {
        fn luminance(c: Color) -> f64 {
            let Color::Rgb(r, g, b) = c else {
                panic!("expected explicit RGB token")
            };
            fn linear(n: u8) -> f64 {
                let c = f64::from(n) / 255.0;
                if c <= 0.04045 {
                    c / 12.92
                } else {
                    ((c + 0.055) / 1.055).powf(2.4)
                }
            }
            0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
        }
        let t = Theme::default();
        for style in [t.text, t.muted, t.accent, t.error] {
            let ratio = (luminance(style.fg) + 0.05) / (luminance(style.bg) + 0.05);
            assert!(ratio >= 4.5, "token contrast {ratio}");
        }
    }
    #[test]
    fn fallback_is_bounded_and_no_color_keeps_focus() {
        assert_eq!(
            Color::Rgb(255, 0, 0).adapt(ColorLevel::Ansi16),
            Color::Indexed(9)
        );
        assert_eq!(Color::Indexed(200).adapt(ColorLevel::None), Color::Default);
        assert!(
            Theme::default()
                .focus
                .adapt(ColorLevel::None)
                .has(TextAttribute::Reverse)
        );
        for n in 0..=255 {
            assert!(matches!(
                Color::Indexed(n).adapt(ColorLevel::Ansi16),
                Color::Indexed(0..=15)
            ));
        }
    }
    #[test]
    fn layout_does_not_overflow_at_small_sizes() {
        use Length::*;
        assert_eq!(distribute(8, &[Fixed(3), Fill(1), Fill(1)]), vec![3, 3, 2]);
        assert_eq!(distribute(2, &[Fixed(3), Fill(1)]), vec![2, 0]);
        for size in 0..100 {
            assert_eq!(
                distribute(size, &[Fill(2), Fill(1), Fill(0)])
                    .iter()
                    .sum::<u16>(),
                size
            );
        }
    }
}
