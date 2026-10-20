//! Owned precise color stops, RGB8 output and allocation-free borrowed sampling.
// Altered Rust implementation informed by Lip Gloss blending.go (19f011f1)
// and go-colorful v1.4.1 colors.go. See THIRD_PARTY_NOTICES.md and ADR 0045.
use crate::Color;
use std::{iter::FusedIterator, ops::Range};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientError {
    EmptyStops,
    AllocationFailed,
    NonFiniteAngle,
    AreaOverflow,
    UnresolvedDefaultColor,
    InvalidPremultipliedStop { index: usize },
}
impl std::fmt::Display for GradientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyStops => f.write_str("a gradient requires at least one RGB stop"),
            Self::AllocationFailed => f.write_str("cannot reserve gradient stops"),
            Self::NonFiniteAngle => f.write_str("gradient angle must be finite"),
            Self::AreaOverflow => f.write_str("gradient grid area is not representable"),
            Self::UnresolvedDefaultColor => {
                f.write_str("resolve default terminal colors before blending")
            }
            Self::InvalidPremultipliedStop { index } => write!(
                f,
                "gradient stop {index} has a channel above its nonzero alpha"
            ),
        }
    }
}
impl std::error::Error for GradientError {}

#[derive(Clone, Copy, Debug)]
struct Stop {
    /// Original premultiplied projection for prefix and single-stop paths.
    raw: [u8; 3],
    /// Opaque normalized endpoint for an interpolated segment.
    rgb: [u8; 3],
    lab: [f64; 3],
}

/// Immutable stops with CIELAB coordinates cached for interpolated segments.
/// Sampling uses the reference's segmented distribution and prefix shortcut.
/// RGB8 and precise premultiplied-RGBA16 inputs produce terminal RGB8 output.
/// This is style color math, without image processing or background compositing.
#[derive(Clone, Debug)]
pub struct Gradient {
    stops: Vec<Stop>,
}
impl Gradient {
    pub fn new(stops: &[[u8; 3]]) -> Result<Self, GradientError> {
        Self::from_rgb(stops.iter().copied())
    }

    /// Own explicit premultiplied 16-bit sRGB/alpha stops. For nonzero alpha,
    /// every RGB channel must be at most alpha. Integer unpremultiplication
    /// precedes Lab conversion; interpolation is opaque, not composited over
    /// an unknown terminal background.
    ///
    /// Prefix counts (count <= stop count) and single-stop gradients retain
    /// original premultiplied channel projections, matching the source's
    /// shortcut contract. Multiple-stop interpolation uses normalized opaque
    /// endpoints. Zero alpha follows Lip Gloss's documented lost-alpha rule:
    /// recover opaque RGB8 from the original high bytes before blending.
    /// Unknown/default terminal colors must be resolved separately.
    ///
    /// ```
    /// use matea_style::{Color, Gradient};
    /// let gradient = Gradient::from_premultiplied_rgba16(&[
    ///     [32768, 8192, 16384, 32768], [65535; 4],
    /// ])?;
    /// assert_eq!(gradient.color_at(0, 1), Some(Color::Rgb(128, 32, 64)));
    /// assert_eq!(gradient.color_at(0, 3), Some(Color::Rgb(255, 63, 127)));
    /// # Ok::<(), matea_style::GradientError>(())
    /// ```
    pub fn from_premultiplied_rgba16(stops: &[[u16; 4]]) -> Result<Self, GradientError> {
        if stops.is_empty() {
            return Err(GradientError::EmptyStops);
        }
        for (index, &[r, g, b, alpha]) in stops.iter().enumerate() {
            if alpha != 0 && [r, g, b].iter().any(|&c| c > alpha) {
                return Err(GradientError::InvalidPremultipliedStop { index });
            }
        }
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(stops.len())
            .map_err(|_| GradientError::AllocationFailed)?;
        if let [[r, g, b, _]] = stops {
            let raw = [*r, *g, *b].map(|c| (c >> 8) as u8);
            // A single stop always samples its raw projection, including grids;
            // there is no segment that could read normalized endpoints or Lab.
            owned.push(Stop {
                raw,
                rgb: raw,
                lab: [0.0; 3],
            });
            return Ok(Self { stops: owned });
        }
        owned.extend(stops.iter().map(|&[r, g, b, alpha]| {
            let raw = [r, g, b].map(|c| (c >> 8) as u8);
            if alpha == 0 {
                Stop {
                    raw,
                    rgb: raw,
                    lab: to_lab(raw),
                }
            } else {
                // 65535² fits u32. Validation proves the quotient fits u16.
                // Opaque stops already have the normalized channel values.
                let normalized = if alpha == u16::MAX {
                    [r, g, b]
                } else {
                    [r, g, b].map(|c| (u32::from(c) * 65535 / u32::from(alpha)) as u16)
                };
                Stop {
                    raw,
                    rgb: normalized.map(|c| (c >> 8) as u8),
                    lab: to_lab16(normalized),
                }
            }
        }));
        Ok(Self { stops: owned })
    }

    /// Resolve RGB and indexed stops using the canonical xterm 256-color
    /// palette. Actual terminal ANSI16 palettes are configurable; use new with
    /// explicitly resolved RGB stops when their exact appearance matters.
    /// Default foreground/background have no known RGB value and are rejected.
    pub fn from_xterm_colors(stops: &[Color]) -> Result<Self, GradientError> {
        if stops.contains(&Color::Default) {
            return Err(GradientError::UnresolvedDefaultColor);
        }
        Self::from_rgb(stops.iter().map(|color| match *color {
            Color::Rgb(r, g, b) => [r, g, b],
            Color::Indexed(index) => {
                let (r, g, b) = crate::palette(index);
                [r, g, b]
            }
            Color::Default => unreachable!("validated immutable stops"),
        }))
    }

    fn from_rgb(stops: impl ExactSizeIterator<Item = [u8; 3]>) -> Result<Self, GradientError> {
        if stops.len() == 0 {
            return Err(GradientError::EmptyStops);
        }
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(stops.len())
            .map_err(|_| GradientError::AllocationFailed)?;
        owned.extend(stops.map(|rgb| Stop {
            raw: rgb,
            rgb,
            lab: to_lab(rgb),
        }));
        Ok(Self { stops: owned })
    }

    /// Sample one position without constructing the preceding colors.
    pub fn color_at(&self, index: usize, count: usize) -> Option<Color> {
        if index >= count {
            return None;
        }
        Some(self.sample(index, count))
    }

    pub fn colors(&self, count: usize) -> GradientColors<'_> {
        GradientColors {
            gradient: self,
            count,
            indices: 0..count,
        }
    }

    /// Row-major rotated color field. Zero dimensions are promoted to one.
    pub fn grid(&self, width: u16, height: u16, angle: f64) -> Result<GradientGrid, GradientError> {
        if !angle.is_finite() {
            return Err(GradientError::NonFiniteAngle);
        }
        let width = width.max(1);
        let height = height.max(1);
        let count = usize::from(width)
            .checked_mul(usize::from(height))
            .ok_or(GradientError::AreaOverflow)?;
        let mut normalized = angle % 360.0;
        if normalized < 0.0 {
            normalized += 360.0;
        }
        let radians = normalized * std::f64::consts::PI / 180.0;
        let (sin, cos) = radians.sin_cos();
        let w = f64::from(width);
        let h = f64::from(height);
        // Interpolate the diagonal once, rather than once per output cell.
        let length = usize::from(width.max(height));
        let mut palette = Vec::new();
        palette
            .try_reserve_exact(length)
            .map_err(|_| GradientError::AllocationFailed)?;
        palette.extend(self.colors(length));
        Ok(GradientGrid {
            palette,
            width,
            height,
            diagonal: (w * w + h * h).sqrt(),
            sin,
            cos,
            count,
        })
    }

    fn sample(&self, index: usize, count: usize) -> Color {
        let lab = if count <= self.stops.len() {
            return rgb_color(self.stops[index].raw);
        } else if self.stops.len() == 1 {
            return rgb_color(self.stops[0].raw);
        } else {
            let segments = self.stops.len() - 1;
            let base = count / segments;
            let extra = count % segments;
            // This product is at most count: extra segments each receive base+1.
            // Avoid base+1 when extra==0 (count may be usize::MAX).
            let extended = if extra == 0 { 0 } else { extra * (base + 1) };
            let (segment, offset, length) = if index < extended {
                (index / (base + 1), index % (base + 1), base + 1)
            } else {
                (
                    extra + (index - extended) / base,
                    (index - extended) % base,
                    base,
                )
            };
            let from = self.stops[segment];
            let to = self.stops[segment + 1];
            if offset == 0 {
                return rgb_color(from.rgb);
            }
            if offset == length - 1 {
                return rgb_color(to.rgb);
            }
            let t = offset as f64 / (length - 1) as f64;
            std::array::from_fn(|channel| {
                from.lab[channel] + t * (to.lab[channel] - from.lab[channel])
            })
        };
        let [r, g, b] = from_lab(lab);
        Color::Rgb(r, g, b)
    }
}

fn rgb_color([r, g, b]: [u8; 3]) -> Color {
    Color::Rgb(r, g, b)
}

#[derive(Clone, Debug)]
pub struct GradientColors<'a> {
    gradient: &'a Gradient,
    count: usize,
    indices: Range<usize>,
}
impl Iterator for GradientColors<'_> {
    type Item = Color;
    fn next(&mut self) -> Option<Color> {
        self.indices
            .next()
            .map(|i| self.gradient.sample(i, self.count))
    }
    fn nth(&mut self, n: usize) -> Option<Color> {
        self.indices
            .nth(n)
            .map(|i| self.gradient.sample(i, self.count))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indices.size_hint()
    }
}
impl DoubleEndedIterator for GradientColors<'_> {
    fn next_back(&mut self) -> Option<Color> {
        self.indices
            .next_back()
            .map(|i| self.gradient.sample(i, self.count))
    }
    fn nth_back(&mut self, n: usize) -> Option<Color> {
        self.indices
            .nth_back(n)
            .map(|i| self.gradient.sample(i, self.count))
    }
}
impl ExactSizeIterator for GradientColors<'_> {}
impl FusedIterator for GradientColors<'_> {}

#[derive(Clone, Debug)]
pub struct GradientGrid {
    palette: Vec<Color>,
    width: u16,
    height: u16,
    diagonal: f64,
    sin: f64,
    cos: f64,
    count: usize,
}
impl GradientGrid {
    pub fn width(&self) -> u16 {
        self.width
    }
    pub fn height(&self) -> u16 {
        self.height
    }
    pub fn cell_count(&self) -> usize {
        self.count
    }
    /// Borrow a reusable palette; traversal allocates no output array.
    pub fn colors(&self) -> GradientGridColors<'_> {
        GradientGridColors {
            grid: self,
            indices: 0..self.count,
        }
    }
    pub fn color_at(&self, x: u16, y: u16) -> Option<Color> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(self.sample(usize::from(y) * usize::from(self.width) + usize::from(x)))
    }
    fn sample(&self, index: usize) -> Color {
        let x = index % usize::from(self.width);
        let y = index / usize::from(self.width);
        let dx = x as f64 - (f64::from(self.width) - 1.0) / 2.0;
        let dy = y as f64 - (f64::from(self.height) - 1.0) / 2.0;
        let rotated = dx * self.cos - dy * self.sin;
        let position = ((rotated + self.diagonal / 2.0) / self.diagonal).clamp(0.0, 1.0);
        let length = self.palette.len();
        let offset = (position * (length - 1) as f64) as usize;
        self.palette[offset.min(length - 1)]
    }
}
#[derive(Clone, Debug)]
pub struct GradientGridColors<'a> {
    grid: &'a GradientGrid,
    indices: Range<usize>,
}
impl Iterator for GradientGridColors<'_> {
    type Item = Color;
    fn next(&mut self) -> Option<Color> {
        self.indices.next().map(|i| self.grid.sample(i))
    }
    fn nth(&mut self, n: usize) -> Option<Color> {
        self.indices.nth(n).map(|i| self.grid.sample(i))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indices.size_hint()
    }
}
impl DoubleEndedIterator for GradientGridColors<'_> {
    fn next_back(&mut self) -> Option<Color> {
        self.indices.next_back().map(|i| self.grid.sample(i))
    }
    fn nth_back(&mut self, n: usize) -> Option<Color> {
        self.indices.nth_back(n).map(|i| self.grid.sample(i))
    }
}
impl ExactSizeIterator for GradientGridColors<'_> {}
impl FusedIterator for GradientGridColors<'_> {}

#[allow(clippy::excessive_precision)] // Preserve audited matrix coefficients.
fn to_lab(rgb: [u8; 3]) -> [f64; 3] {
    to_lab_srgb(rgb.map(|c| f64::from(c) / 255.0))
}
fn to_lab16(rgb: [u16; 3]) -> [f64; 3] {
    to_lab_srgb(rgb.map(|c| f64::from(c) / 65535.0))
}
#[allow(clippy::excessive_precision)] // Preserve audited matrix coefficients.
fn to_lab_srgb(rgb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rgb.map(|v| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    });
    let x = 0.41239079926595948 * r + 0.35758433938387796 * g + 0.18048078840183429 * b;
    let y = 0.21263900587151036 * r + 0.71516867876775593 * g + 0.072192315360733715 * b;
    let z = 0.019330818715591851 * r + 0.11919477979462599 * g + 0.95053215224966058 * b;
    fn transform(t: f64) -> f64 {
        if t > 6.0 / 29.0 * 6.0 / 29.0 * 6.0 / 29.0 {
            t.cbrt()
        } else {
            t / 3.0 * 29.0 / 6.0 * 29.0 / 6.0 + 4.0 / 29.0
        }
    }
    let fy = transform(y);
    [
        1.16 * fy - 0.16,
        5.0 * (transform(x / 0.95047) - fy),
        2.0 * (fy - transform(z / 1.08883)),
    ]
}
#[allow(clippy::excessive_precision)] // Preserve audited matrix coefficients.
fn from_lab([l, a, b]: [f64; 3]) -> [u8; 3] {
    fn inverse(t: f64) -> f64 {
        if t > 6.0 / 29.0 {
            t * t * t
        } else {
            3.0 * 6.0 / 29.0 * 6.0 / 29.0 * (t - 4.0 / 29.0)
        }
    }
    let l2 = (l + 0.16) / 1.16;
    let x = 0.95047 * inverse(l2 + a / 5.0);
    let y = inverse(l2);
    let z = 1.08883 * inverse(l2 - b / 2.0);
    let rgb = [
        3.2409699419045214 * x - 1.5373831775700935 * y - 0.49861076029300328 * z,
        -0.96924363628087983 * x + 1.8759675015077207 * y + 0.041555057407175613 * z,
        0.055630079696993609 * x - 0.20397695888897657 * y + 1.0569715142428786 * z,
    ];
    rgb.map(|v| {
        let srgb = if v <= 0.0031308 {
            12.92 * v
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        // Match color.Color.RGBA's 16-bit rounding followed by terminal RGB8.
        (((srgb.clamp(0.0, 1.0) * 65535.0 + 0.5) as u16) >> 8) as u8
    })
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    #[test]
    fn precise_inputs_keep_stop_and_terminal_color_storage_bounded() {
        if cfg!(target_pointer_width = "64") {
            assert_eq!(std::mem::size_of::<Stop>(), 32);
            assert_eq!(std::mem::size_of::<Color>(), 4);
        }
    }
}
