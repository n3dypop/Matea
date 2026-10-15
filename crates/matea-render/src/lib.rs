#![forbid(unsafe_code)]
//! Structured views, grapheme-safe cell composition, and deterministic row diffs.
use matea_style::{Border, Length, Style, Theme, distribute};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}
impl Size {
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}
impl Rect {
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn right(self) -> u16 {
        self.x.saturating_add(self.width)
    }
    pub fn bottom(self) -> u16 {
        self.y.saturating_add(self.height)
    }
    pub fn intersect(self, other: Self) -> Self {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        Self::new(
            x,
            y,
            self.right().min(other.right()).saturating_sub(x),
            self.bottom().min(other.bottom()).saturating_sub(y),
        )
    }
    pub fn inset(self, n: u16) -> Self {
        Self::new(
            self.x.saturating_add(n).min(self.right()),
            self.y.saturating_add(n).min(self.bottom()),
            self.width.saturating_sub(n.saturating_mul(2)),
            self.height.saturating_sub(n.saturating_mul(2)),
        )
    }
}

/// A leading grapheme or a reserved continuation cell. Mutated only through Canvas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    symbol: String,
    style: Style,
    width: u16,
}
impl Default for Cell {
    fn default() -> Self {
        Self {
            symbol: " ".into(),
            style: Style::default(),
            width: 1,
        }
    }
}
impl Cell {
    pub fn symbol(&self) -> &str {
        &self.symbol
    }
    pub fn style(&self) -> Style {
        self.style
    }
    pub fn width(&self) -> u16 {
        self.width
    }
    pub fn is_continuation(&self) -> bool {
        self.width == 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Surface {
    size: Size,
    cells: Vec<Cell>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeometryError;
impl std::fmt::Display for GeometryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("surface exceeds 1,048,576 cells; reduce viewport dimensions")
    }
}
impl std::error::Error for GeometryError {}
impl Surface {
    pub fn new(size: Size) -> Result<Self, GeometryError> {
        let count = usize::from(size.width) * usize::from(size.height);
        if count > 1_048_576 {
            return Err(GeometryError);
        }
        Ok(Self {
            size,
            cells: vec![Cell::default(); count],
        })
    }
    pub fn size(&self) -> Size {
        self.size
    }
    pub fn area(&self) -> Rect {
        Rect::new(0, 0, self.size.width, self.size.height)
    }
    pub fn row(&self, y: u16) -> &[Cell] {
        if y >= self.size.height {
            return &[];
        }
        let start = usize::from(y) * usize::from(self.size.width);
        &self.cells[start..start + usize::from(self.size.width)]
    }
    pub fn canvas(&mut self, clip: Rect) -> Canvas<'_> {
        let clip = clip.intersect(self.area());
        Canvas {
            surface: self,
            clip,
        }
    }
    /// Plain snapshot, preserving rows and trimming trailing spaces.
    pub fn plain(&self) -> String {
        (0..self.size.height)
            .map(|y| {
                self.row(y)
                    .iter()
                    .map(Cell::symbol)
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    /// Changed rows in top-to-bottom order. Resize forces a complete redraw.
    pub fn diff(&self, previous: Option<&Self>) -> Vec<u16> {
        (0..self.size.height)
            .filter(|&y| previous.is_none_or(|p| p.size != self.size || p.row(y) != self.row(y)))
            .collect()
    }
    fn index(&self, x: u16, y: u16) -> usize {
        usize::from(y) * usize::from(self.size.width) + usize::from(x)
    }
    fn erase_glyph(&mut self, x: u16, y: u16) {
        let mut left = x;
        while left > 0 && self.cells[self.index(left, y)].is_continuation() {
            left -= 1;
        }
        let width = self.cells[self.index(left, y)].width.max(1);
        for col in left..left.saturating_add(width).min(self.size.width) {
            let i = self.index(col, y);
            self.cells[i] = Cell::default();
        }
    }
}

/// Only this API can write cells; control characters cannot reach terminal output.
pub struct Canvas<'a> {
    surface: &'a mut Surface,
    clip: Rect,
}
impl Canvas<'_> {
    pub fn clip(&self) -> Rect {
        self.clip
    }
    pub fn text(&mut self, x: u16, y: u16, text: &str, style: Style) {
        let safe = sanitize(text);
        for (dy, line) in safe.split('\n').enumerate() {
            let Some(row) = y.checked_add(u16::try_from(dy).unwrap_or(u16::MAX)) else {
                break;
            };
            if row >= self.clip.bottom() {
                break;
            }
            let mut col = u32::from(x);
            for g in line.graphemes(true) {
                let w = g.width() as u32;
                if w == 0 {
                    continue;
                }
                if col + w > u32::from(self.clip.right()) {
                    break;
                }
                if col >= u32::from(self.clip.x) && row >= self.clip.y {
                    self.put(col as u16, row, g, style, w as u16);
                }
                col += w;
            }
        }
    }
    fn put(&mut self, x: u16, y: u16, g: &str, style: Style, width: u16) {
        for col in x..x + width {
            self.surface.erase_glyph(col, y);
        }
        let i = self.surface.index(x, y);
        self.surface.cells[i] = Cell {
            symbol: g.into(),
            style,
            width,
        };
        for col in x + 1..x + width {
            let i = self.surface.index(col, y);
            self.surface.cells[i] = Cell {
                symbol: String::new(),
                style,
                width: 0,
            };
        }
    }
    pub fn fill(&mut self, style: Style) {
        for y in self.clip.y..self.clip.bottom() {
            for x in self.clip.x..self.clip.right() {
                self.put(x, y, " ", style, 1);
            }
        }
    }
}

/// Newlines are structural; tabs use four spaces. Escape/C0/C1 controls become U+FFFD.
pub fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\n' => "\n".into(),
            '\t' => "    ".into(),
            c if c.is_control() => "�".into(),
            c => c.to_string(),
        })
        .collect()
}

#[derive(Clone, Debug)]
enum Node {
    Text(String, Style),
    Stack {
        horizontal: bool,
        children: Vec<View>,
        gap: u16,
    },
    Panel {
        title: String,
        child: Box<View>,
        theme: Box<Theme>,
    },
    Layers(Vec<(Rect, View)>),
}
/// An owned immutable scene. Views never contain terminal escape sequences.
#[derive(Clone, Debug)]
pub struct View {
    node: Node,
    width: Option<Length>,
    height: Option<Length>,
}
impl View {
    fn new(node: Node) -> Self {
        Self {
            node,
            width: None,
            height: None,
        }
    }
    pub fn text(text: impl Into<String>) -> Self {
        Self::new(Node::Text(text.into(), Style::default()))
    }
    pub fn styled(text: impl Into<String>, style: Style) -> Self {
        Self::new(Node::Text(text.into(), style))
    }
    pub fn column(children: impl IntoIterator<Item = View>) -> Self {
        Self::new(Node::Stack {
            horizontal: false,
            children: children.into_iter().collect(),
            gap: 0,
        })
    }
    pub fn row(children: impl IntoIterator<Item = View>) -> Self {
        Self::new(Node::Stack {
            horizontal: true,
            children: children.into_iter().collect(),
            gap: 0,
        })
    }
    pub fn layers(children: Vec<(Rect, View)>) -> Self {
        Self::new(Node::Layers(children))
    }
    pub fn panel(self, title: impl Into<String>, theme: &Theme) -> Self {
        Self::new(Node::Panel {
            title: title.into(),
            child: Box::new(self),
            theme: Box::new(theme.clone()),
        })
    }
    pub fn gap(mut self, gap: u16) -> Self {
        if let Node::Stack { gap: g, .. } = &mut self.node {
            *g = gap;
        }
        self
    }
    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(Length::Fixed(width));
        self
    }
    pub fn height(mut self, height: u16) -> Self {
        self.height = Some(Length::Fixed(height));
        self
    }
    pub fn fill(mut self) -> Self {
        self.height = Some(Length::Fill(1));
        self
    }
    fn natural_height(&self) -> u16 {
        if let Some(Length::Fixed(n)) = self.height {
            return n;
        }
        match &self.node {
            Node::Text(s, _) => s.split('\n').count().min(u16::MAX as usize) as u16,
            Node::Stack {
                horizontal,
                children,
                gap,
            } => {
                if *horizontal {
                    children.iter().map(Self::natural_height).max().unwrap_or(0)
                } else {
                    children
                        .iter()
                        .fold(0u16, |n, v| n.saturating_add(v.natural_height()))
                        .saturating_add(gap.saturating_mul(
                            children.len().saturating_sub(1).min(u16::MAX as usize) as u16,
                        ))
                }
            }
            Node::Panel { child, theme, .. } => child
                .natural_height()
                .saturating_add(2)
                .saturating_add(theme.padding.saturating_mul(2)),
            Node::Layers(c) => c.iter().map(|(r, _)| r.bottom()).max().unwrap_or(0),
        }
    }
    pub fn render(&self, size: Size) -> Result<Surface, GeometryError> {
        let mut s = Surface::new(size)?;
        let area = s.area();
        self.paint(&mut s, area, area);
        Ok(s)
    }
    fn paint(&self, s: &mut Surface, mut area: Rect, clip: Rect) {
        if let Some(Length::Fixed(n)) = self.width {
            area.width = area.width.min(n);
        }
        if let Some(Length::Fixed(n)) = self.height {
            area.height = area.height.min(n);
        }
        let clip = clip.intersect(area);
        if clip.width == 0 || clip.height == 0 {
            return;
        }
        match &self.node {
            Node::Text(text, style) => s.canvas(clip).text(area.x, area.y, text, *style),
            Node::Stack {
                horizontal,
                children,
                gap,
            } => {
                let total = if *horizontal { area.width } else { area.height };
                let gaps = gap
                    .saturating_mul(children.len().saturating_sub(1).min(u16::MAX as usize) as u16)
                    .min(total);
                let lengths: Vec<_> = children
                    .iter()
                    .map(|v| {
                        if *horizontal {
                            v.width.unwrap_or(Length::Fill(1))
                        } else {
                            v.height.unwrap_or(Length::Fixed(v.natural_height()))
                        }
                    })
                    .collect();
                let sizes = distribute(total - gaps, &lengths);
                let mut offset = 0u16;
                for (v, n) in children.iter().zip(sizes) {
                    let r = if *horizontal {
                        Rect::new(area.x.saturating_add(offset), area.y, n, area.height)
                    } else {
                        Rect::new(area.x, area.y.saturating_add(offset), area.width, n)
                    };
                    v.paint(s, r, clip);
                    offset = offset.saturating_add(n).saturating_add(*gap);
                }
            }
            Node::Panel {
                title,
                child,
                theme,
            } => {
                s.canvas(clip).fill(theme.surface);
                let (tl, tr, bl, br, h, v) = match theme.border_kind {
                    Border::Rounded => ("╭", "╮", "╰", "╯", "─", "│"),
                    Border::Plain => ("┌", "┐", "└", "┘", "─", "│"),
                    Border::Ascii => ("+", "+", "+", "+", "-", "|"),
                };
                let mut c = s.canvas(clip);
                for x in area.x..area.right() {
                    c.text(x, area.y, h, theme.border);
                    c.text(x, area.bottom() - 1, h, theme.border);
                }
                for y in area.y..area.bottom() {
                    c.text(area.x, y, v, theme.border);
                    c.text(area.right() - 1, y, v, theme.border);
                }
                for (x, y, g) in [
                    (area.x, area.y, tl),
                    (area.right() - 1, area.y, tr),
                    (area.x, area.bottom() - 1, bl),
                    (area.right() - 1, area.bottom() - 1, br),
                ] {
                    c.text(x, y, g, theme.border);
                }
                if area.width > 4 {
                    let title_clip =
                        Rect::new(area.x + 2, area.y, area.width - 4, 1).intersect(clip);
                    s.canvas(title_clip)
                        .text(area.x + 2, area.y, title, theme.accent);
                }
                child.paint(
                    s,
                    area.inset(1u16.saturating_add(theme.padding)),
                    clip.intersect(area.inset(1)),
                );
            }
            Node::Layers(layers) => {
                for (r, v) in layers {
                    let r = Rect::new(
                        area.x.saturating_add(r.x),
                        area.y.saturating_add(r.y),
                        r.width,
                        r.height,
                    );
                    v.paint(s, r, clip);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_clipping_and_controls() {
        let s = View::text("e\u{301}茶👩‍💻Z\n\x1b[31m")
            .render(Size::new(5, 2))
            .unwrap();
        assert_eq!(s.plain(), "e\u{301}茶👩‍💻\n�[31m");
        assert_eq!(s.row(0)[1].width(), 2);
        assert!(s.row(0)[2].is_continuation());
        assert_eq!(
            View::text("茶").render(Size::new(1, 1)).unwrap().plain(),
            ""
        );
    }
    #[test]
    fn overlapping_wide_glyph_is_removed_atomically() {
        let s = View::layers(vec![
            (Rect::new(0, 0, 4, 1), View::text("茶ab")),
            (Rect::new(1, 0, 1, 1), View::text("X")),
        ])
        .render(Size::new(4, 1))
        .unwrap();
        assert_eq!(s.plain(), " Xab");
    }
    #[test]
    fn diff_handles_wide_to_narrow_style_and_resize() {
        let a = View::text("茶").render(Size::new(4, 2)).unwrap();
        let b = View::styled("a", Style::default().bold())
            .render(Size::new(4, 2))
            .unwrap();
        assert_eq!(b.diff(Some(&a)), vec![0]);
        assert!(b.diff(Some(&b)).is_empty());
        assert_eq!(a.diff(None), vec![0, 1]);
        assert_eq!(
            a.diff(Some(&View::text("").render(Size::new(2, 1)).unwrap())),
            vec![0, 1]
        );
    }
    #[test]
    fn zero_and_oversized_geometry() {
        assert_eq!(View::text("x").render(Size::new(0, 0)).unwrap().plain(), "");
        assert_eq!(
            Surface::new(Size::new(u16::MAX, u16::MAX)),
            Err(GeometryError)
        );
        for w in 0..8 {
            for h in 0..8 {
                View::text("茶")
                    .panel("title", &Theme::default())
                    .render(Size::new(w, h))
                    .unwrap();
            }
        }
    }
}
