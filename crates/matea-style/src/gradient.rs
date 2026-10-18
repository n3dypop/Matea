//! Owned RGB8 terminal gradients with allocation-free borrowed sampling.
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
}
impl std::fmt::Display for GradientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::EmptyStops => "a gradient requires at least one RGB stop",
            Self::AllocationFailed => "cannot reserve gradient stops",
            Self::NonFiniteAngle => "gradient angle must be finite",
            Self::AreaOverflow => "gradient grid area is not representable",
        })
    }
}
impl std::error::Error for GradientError {}

#[derive(Clone, Copy, Debug)]
struct Stop {
    rgb: [u8; 3],
    lab: [f64; 3],
}

/// Immutable RGB8 stops with CIELAB coordinates computed once at construction.
/// Sampling uses the reference's segmented distribution and prefix shortcut.
/// Output is quantized to terminal RGB8; it is not a high-depth color container.
#[derive(Clone, Debug)]
pub struct Gradient {
    stops: Vec<Stop>,
}
impl Gradient {
    pub fn new(stops: &[[u8; 3]]) -> Result<Self, GradientError> {
        if stops.is_empty() {
            return Err(GradientError::EmptyStops);
        }
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(stops.len())
            .map_err(|_| GradientError::AllocationFailed)?;
        owned.extend(stops.iter().map(|&rgb| Stop {
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
            return rgb_color(self.stops[index].rgb);
        } else if self.stops.len() == 1 {
            return rgb_color(self.stops[0].rgb);
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
    let [r, g, b] = rgb.map(|c| {
        let v = f64::from(c) / 255.0;
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
