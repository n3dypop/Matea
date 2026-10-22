//! Explicit terminal style color input. No image buffers, codecs or terminal I/O.
use crate::{Color, ColorLevel};
use std::{fmt, str::FromStr};

/// A rejected color input. Invalid user input never silently becomes black.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorError {
    InvalidFormat,
    OutOfRange,
    UnresolvedDefault,
    NonFiniteAmount,
    InvalidPremultipliedChannels,
}
impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidFormat => "expected #RGB, #RRGGBB or a signed decimal color",
            Self::OutOfRange => "decimal color magnitude must fit a signed 64-bit integer",
            Self::UnresolvedDefault => "resolve the terminal default color before color arithmetic",
            Self::NonFiniteAmount => "color adjustment must be finite",
            Self::InvalidPremultipliedChannels => {
                "premultiplied RGB channels must not exceed alpha"
            }
        })
    }
}
impl std::error::Error for ColorError {}

impl FromStr for Color {
    type Err = ColorError;

    /// Parse Lip Gloss's hex/index/packed-decimal notation. Negative decimal
    /// values select their magnitude; values >= 256 select the low 24 RGB bits.
    /// Parsing is independent of the host pointer width. Unlike the reference,
    /// invalid input is an error; use `unwrap_or_default()` for its fallback.
    /// The unrepresentable signed magnitude (i64::MIN) is rejected explicitly.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if let Some(hex) = input.strip_prefix('#') {
            fn digit(byte: u8) -> Result<u8, ColorError> {
                match byte {
                    b'0'..=b'9' => Ok(byte - b'0'),
                    b'a'..=b'f' => Ok(byte - b'a' + 10),
                    b'A'..=b'F' => Ok(byte - b'A' + 10),
                    _ => Err(ColorError::InvalidFormat),
                }
            }
            let rgb = match *hex.as_bytes() {
                [r, g, b] => [digit(r)? * 17, digit(g)? * 17, digit(b)? * 17],
                [r, r2, g, g2, b, b2] => [
                    digit(r)? * 16 + digit(r2)?,
                    digit(g)? * 16 + digit(g2)?,
                    digit(b)? * 16 + digit(b2)?,
                ],
                _ => return Err(ColorError::InvalidFormat),
            };
            return Ok(Self::Rgb(rgb[0], rgb[1], rgb[2]));
        }
        let digits = input.strip_prefix(['+', '-']).unwrap_or(input);
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(ColorError::InvalidFormat);
        }
        let value = input
            .parse::<i64>()
            .map_err(|_| ColorError::OutOfRange)?
            .checked_abs()
            .ok_or(ColorError::OutOfRange)? as u64;
        if value < 256 {
            Ok(Self::Indexed(value as u8))
        } else {
            Ok(Self::Rgb(
                (value >> 16) as u8,
                (value >> 8) as u8,
                value as u8,
            ))
        }
    }
}

impl Color {
    /// Resolve the canonical xterm palette. A terminal's actual ANSI16 palette
    /// may differ. Default is unknown, rather than known opaque black.
    pub const fn rgb_channels(self) -> Option<[u8; 3]> {
        match self {
            Self::Default => None,
            Self::Rgb(r, g, b) => Some([r, g, b]),
            Self::Indexed(index) => {
                let (r, g, b) = crate::palette(index);
                Some([r, g, b])
            }
        }
    }

    /// Known opaque color at explicit 16-bit depth; 255 maps to 65535, not
    /// 65280. Indexed colors use the canonical palette, without terminal I/O.
    pub const fn rgba16(self) -> Option<Rgba16> {
        match self.rgb_channels() {
            Some([r, g, b]) => Some(Rgba16::new(
                r as u16 * 257,
                g as u16 * 257,
                b as u16 * 257,
                u16::MAX,
            )),
            None => None,
        }
    }

    /// Rotate HSV hue by 180 degrees, preserving saturation and value. For
    /// RGB8 this is exactly max(channel)+min(channel)-channel; no float HSV
    /// round trip or allocation is necessary.
    pub fn complementary(self) -> Result<Self, ColorError> {
        let [r, g, b] = self.rgb_channels().ok_or(ColorError::UnresolvedDefault)?;
        let sum = u16::from(r.max(g).max(b)) + u16::from(r.min(g).min(b));
        Ok(Self::Rgb(
            (sum - u16::from(r)) as u8,
            (sum - u16::from(g)) as u8,
            (sum - u16::from(b)) as u8,
        ))
    }

    /// Multiply RGB8 channels by 1-amount, truncating as the source does.
    /// Finite amounts clamp to 0..1; NaN/infinity are explicit errors.
    pub fn darken(self, amount: f64) -> Result<Self, ColorError> {
        self.adjust(amount, |channel, amount| {
            (f64::from(channel) * (1.0 - amount)) as u8
        })
    }

    /// Add 255*amount to RGB8 channels, saturating and truncating. This is the
    /// reference's channel adjustment, rather than HSL lightness adjustment.
    pub fn lighten(self, amount: f64) -> Result<Self, ColorError> {
        self.adjust(amount, |channel, amount| {
            (f64::from(channel) + 255.0 * amount).min(255.0) as u8
        })
    }

    fn adjust(self, amount: f64, operation: impl Fn(u8, f64) -> u8) -> Result<Self, ColorError> {
        if !amount.is_finite() {
            return Err(ColorError::NonFiniteAmount);
        }
        let channels = self.rgb_channels().ok_or(ColorError::UnresolvedDefault)?;
        let [r, g, b] = channels.map(|channel| operation(channel, amount.clamp(0.0, 1.0)));
        Ok(Self::Rgb(r, g, b))
    }
}

/// Straight (not premultiplied) 16-bit sRGB channels and alpha. This describes
/// individual style colors, without pixel/image storage. Zero alpha may retain
/// RGB; changing alpha never silently changes the straight RGB channels.
///
/// ```
/// use matea_style::{Color, Rgba16};
/// let red = Rgba16::new(65535, 0, 0, 32768);
/// let blue = Color::Rgb(0, 0, 255).rgba16().unwrap();
/// assert_eq!(red.over(blue).rgb(), Color::Rgb(128, 0, 127));
/// assert_eq!(red.premultiplied().channels(), [32768, 0, 0, 32768]);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rgba16 {
    pub red: u16,
    pub green: u16,
    pub blue: u16,
    pub alpha: u16,
}
impl Rgba16 {
    pub const fn new(red: u16, green: u16, blue: u16, alpha: u16) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
    pub const fn channels(self) -> [u16; 4] {
        [self.red, self.green, self.blue, self.alpha]
    }
    /// Explicitly discard alpha and project straight RGB channels to RGB8.
    /// Use `over` first when the appearance over a background matters.
    pub const fn rgb(self) -> Color {
        Color::rgb16(self.red, self.green, self.blue)
    }
    pub const fn with_alpha(self, alpha: u16) -> Self {
        Self { alpha, ..self }
    }
    /// Canonical Go Color-style premultiplication: truncate at 16-bit depth.
    pub const fn premultiplied(self) -> PremultipliedRgba16 {
        let alpha = self.alpha as u32;
        PremultipliedRgba16([
            (self.red as u32 * alpha / 65535) as u16,
            (self.green as u32 * alpha / 65535) as u16,
            (self.blue as u32 * alpha / 65535) as u16,
            self.alpha,
        ])
    }
    /// Source-over composition in encoded sRGB, rounding to nearest integer.
    /// Both operands are known colors; terminal defaults must be resolved by
    /// the caller. The opaque/transparent identities avoid needless work.
    pub fn over(self, background: Self) -> Self {
        if self.alpha == u16::MAX || background.alpha == 0 {
            return self;
        }
        if self.alpha == 0 {
            return background;
        }
        let front = u64::from(self.alpha) * 65535;
        let back = u64::from(background.alpha) * u64::from(65535 - self.alpha);
        let denominator = front + back;
        let blend = |a: u16, b: u16| {
            ((u64::from(a) * front + u64::from(b) * back + denominator / 2) / denominator) as u16
        };
        Self::new(
            blend(self.red, background.red),
            blend(self.green, background.green),
            blend(self.blue, background.blue),
            ((denominator + 32767) / 65535) as u16,
        )
    }
}

/// Validated premultiplied 16-bit style channels. The private representation
/// ensures alpha is never below a channel, including at zero alpha. Use
/// `Rgba16` for straight or deliberately retained transparent RGB instead.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PremultipliedRgba16([u16; 4]);
impl TryFrom<[u16; 4]> for PremultipliedRgba16 {
    type Error = ColorError;
    fn try_from(channels: [u16; 4]) -> Result<Self, Self::Error> {
        if channels[..3].iter().any(|&channel| channel > channels[3]) {
            Err(ColorError::InvalidPremultipliedChannels)
        } else {
            Ok(Self(channels))
        }
    }
}
impl PremultipliedRgba16 {
    pub const fn channels(self) -> [u16; 4] {
        self.0
    }
    /// Integer unpremultiplication, truncating as Go color conversion does.
    /// At zero alpha no straight RGB can be recovered, so the result is zero.
    pub const fn straight(self) -> Rgba16 {
        let [r, g, b, alpha] = self.0;
        if alpha == 0 {
            return Rgba16::new(0, 0, 0, 0);
        }
        Rgba16::new(
            (r as u32 * 65535 / alpha as u32) as u16,
            (g as u32 * 65535 / alpha as u32) as u16,
            (b as u32 * 65535 / alpha as u32) as u16,
            alpha,
        )
    }
}

/// Explicit theme choice. No global terminal detection or hidden I/O.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct AdaptiveColor {
    pub light: Color,
    pub dark: Color,
}
impl AdaptiveColor {
    pub const fn resolve(self, dark_background: bool) -> Color {
        if dark_background {
            self.dark
        } else {
            self.light
        }
    }
}

/// Author-selected profile colors, distinct from nearest-color quantization.
/// A None profile selects the terminal default, retaining non-color cues.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ProfileColors {
    pub ansi16: Color,
    pub ansi256: Color,
    pub truecolor: Color,
}
impl ProfileColors {
    pub const fn resolve(self, level: ColorLevel) -> Color {
        match level {
            ColorLevel::None => Color::Default,
            ColorLevel::Ansi16 => self.ansi16,
            ColorLevel::Ansi256 => self.ansi256,
            ColorLevel::TrueColor => self.truecolor,
        }
    }
}
