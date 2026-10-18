#![forbid(unsafe_code)]
//! Structured views, grapheme-safe cell composition, and deterministic row diffs.
use compact_str::CompactString;
use matea_style::{Border, Length, Style, Theme, distribute};
use std::{borrow::Cow, fmt};
mod text;
pub use text::Text;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}
/// Zero-based cell position. Layout translates local hints into frame coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cursor {
    pub x: u16,
    pub y: u16,
}
impl Cursor {
    pub const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
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
    symbol: CompactString,
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
    cursor: Option<Cursor>,
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
            cursor: None,
        })
    }
    pub fn size(&self) -> Size {
        self.size
    }
    pub fn area(&self) -> Rect {
        Rect::new(0, 0, self.size.width, self.size.height)
    }
    pub fn cursor(&self) -> Option<Cursor> {
        self.cursor
    }
    /// Invalid positions hide the cursor. Continuation cells normalize to the
    /// grapheme lead, rather than allowing a caret inside a wide glyph.
    pub fn set_cursor(&mut self, cursor: Option<Cursor>) {
        self.cursor = cursor.and_then(|mut c| {
            if c.x >= self.size.width || c.y >= self.size.height {
                return None;
            }
            while c.x > 0 && self.cells[self.index(c.x, c.y)].is_continuation() {
                c.x -= 1;
            }
            Some(c)
        });
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
        let mut text = String::with_capacity(self.cells.len());
        for y in 0..self.size.height {
            let start = text.len();
            for cell in self.row(y) {
                text.push_str(cell.symbol());
            }
            let end = start + text[start..].trim_end().len();
            text.truncate(end);
            if y + 1 < self.size.height {
                text.push('\n');
            }
        }
        text
    }
    /// Changed rows in top-to-bottom order. Resize forces a complete redraw.
    pub fn diff(&self, previous: Option<&Self>) -> Vec<u16> {
        self.changed_rows(previous).collect()
    }
    /// Borrowed damage iterator; reusing the same immutable surface is O(1).
    pub fn changed_rows<'a>(
        &'a self,
        previous: Option<&'a Self>,
    ) -> impl Iterator<Item = u16> + 'a {
        let height = if previous.is_some_and(|p| std::ptr::eq(self, p)) {
            0
        } else {
            self.size.height
        };
        (0..height).filter(move |&y| {
            previous.is_none_or(|p| p.size != self.size || p.row(y) != self.row(y))
        })
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
        if self
            .cursor
            .is_some_and(|c| c.y == y && c.x >= left && c.x < left.saturating_add(width))
        {
            self.cursor = None;
        }
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
        let safe = sanitized(text);
        self.draw_text(x, y, &safe, style);
    }
    fn text_owned(&mut self, x: u16, y: u16, text: &Text, style: Style) {
        self.draw_text(x, y, text.as_str(), style);
    }
    fn draw_text(&mut self, x: u16, y: u16, text: &str, style: Style) {
        for (dy, line) in text.split('\n').enumerate() {
            let Some(row) = y.checked_add(u16::try_from(dy).unwrap_or(u16::MAX)) else {
                break;
            };
            if row >= self.clip.bottom() {
                break;
            }
            let mut col = u32::from(x);
            if line.is_ascii() {
                for (offset, _) in line.bytes().enumerate() {
                    if col >= u32::from(self.clip.right()) {
                        break;
                    }
                    if col >= u32::from(self.clip.x) && row >= self.clip.y {
                        self.put(col as u16, row, &line[offset..offset + 1], style, 1);
                    }
                    col += 1;
                }
                continue;
            }
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
                symbol: CompactString::default(),
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
    sanitized(s).into_owned()
}
/// Borrow safe input; allocate only when control replacement is necessary.
pub fn sanitized(s: &str) -> Cow<'_, str> {
    if is_safe_text(s) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    write_sanitized(s, &mut out).expect("writing normalized String cannot fail");
    Cow::Owned(out)
}
fn is_safe_text(s: &str) -> bool {
    // str already guarantees valid UTF-8. C0/DEL bytes cannot occur inside a
    // multibyte scalar; C1 controls are exactly the pairs C2 80..9F.
    // Fold ASCII-control/high-bit flags together so clean ASCII can vectorize.
    let mut flags = 0u8;
    for byte in s.bytes() {
        let invalid = ((byte < 0x20) & (byte != b'\n')) | (byte == 0x7f);
        flags |= u8::from(invalid) | (byte & 0x80);
    }
    if flags & 1 != 0 {
        return false;
    }
    flags & 0x80 == 0
        || !s
            .as_bytes()
            .windows(2)
            .any(|pair| pair[0] == 0xc2 && pair[1] < 0xa0)
}
fn write_sanitized(s: &str, out: &mut impl fmt::Write) -> fmt::Result {
    for c in s.chars() {
        match c {
            '\n' => out.write_char('\n')?,
            '\t' => out.write_str("    ")?,
            c if c.is_control() => out.write_char('�')?,
            c => out.write_char(c)?,
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
enum Node {
    Text(Text, Style),
    Stack {
        horizontal: bool,
        children: Vec<View>,
        gap: u16,
    },
    Panel {
        title: Text,
        child: Box<View>,
        theme: Box<Theme>,
    },
    Layers(Vec<(Rect, View)>),
    Cursor {
        child: Box<View>,
        cursor: Cursor,
    },
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
    pub fn text(text: impl Into<Text>) -> Self {
        Self::new(Node::Text(text.into(), Style::default()))
    }
    pub fn styled(text: impl Into<Text>, style: Style) -> Self {
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
    /// Declare a local cursor; clipping, offsets and later paint are composed by
    /// the renderer. Only the transport controls terminal visibility/position.
    pub fn with_cursor(self, cursor: Cursor) -> Self {
        let (width, height) = (self.width, self.height);
        Self {
            node: Node::Cursor {
                child: Box::new(self),
                cursor,
            },
            width,
            height,
        }
    }
    pub fn panel(self, title: impl Into<Text>, theme: &Theme) -> Self {
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
            Node::Cursor { child, .. } => child.natural_height(),
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
            Node::Text(text, style) => s.canvas(clip).text_owned(area.x, area.y, text, *style),
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
            Node::Cursor { child, cursor } => {
                child.paint(s, area, clip);
                let absolute = Cursor::new(
                    area.x.saturating_add(cursor.x),
                    area.y.saturating_add(cursor.y),
                );
                if absolute.x >= clip.x
                    && absolute.x < clip.right()
                    && absolute.y >= clip.y
                    && absolute.y < clip.bottom()
                {
                    s.set_cursor(Some(absolute));
                    // Normalization must not move outside the inherited clip.
                    if s.cursor().is_some_and(|c| c.x < clip.x) {
                        s.set_cursor(None);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extended_attributes_keep_the_cell_budget_and_follow_whole_graphemes() {
        use matea_style::{BlinkStyle, TextFrame, UnderlineStyle};
        if cfg!(target_pointer_width = "64") {
            assert_eq!(
                std::mem::size_of::<Cell>(),
                40,
                "resolved styling must not grow every cell"
            );
        }
        let style = Style::default()
            .italic()
            .dim()
            .strikethrough()
            .overline()
            .blink(BlinkStyle::Slow)
            .text_frame(TextFrame::Encircled)
            .underline_style(UnderlineStyle::Dashed)
            .underline_color(matea_style::Color::Indexed(15));
        let view = View::styled("茶e\u{301}🙂", style);
        let frame = view.render(Size::new(6, 1)).unwrap();
        assert_eq!(frame.row(0)[0].symbol(), "茶");
        assert!(frame.row(0)[1].is_continuation());
        assert_eq!(frame.row(0)[2].symbol(), "e\u{301}");
        assert_eq!(frame.row(0)[3].symbol(), "🙂");
        assert!(frame.row(0)[4].is_continuation());
        for cell in &frame.row(0)[..5] {
            assert_eq!(cell.style(), style);
        }
        assert_eq!(frame.row(0)[5].style(), Style::default());
        let clipped = view.render(Size::new(4, 1)).unwrap();
        assert_eq!(clipped.plain(), "茶e\u{301}");
        assert_eq!(clipped.row(0)[3].style(), Style::default());
        assert_eq!(view.render(Size::new(0, 0)).unwrap().plain(), "");
    }
    #[test]
    fn cursor_composes_through_layout_panel_and_layers() {
        let theme = Theme::default();
        let child = View::column([
            View::text("label"),
            View::row([
                View::text("prefix").width(3),
                View::text("tea").with_cursor(Cursor::new(2, 0)),
            ])
            .gap(1),
        ])
        .gap(1)
        .panel("input", &theme);
        let frame = View::layers(vec![(Rect::new(5, 3, 30, 12), child)])
            .render(Size::new(40, 20))
            .unwrap();
        let inset = 1 + theme.padding;
        assert_eq!(
            frame.cursor(),
            Some(Cursor::new(5 + inset + 6, 3 + inset + 2))
        );
    }
    #[test]
    fn cursor_clips_normalizes_wide_cells_and_obeys_paint_order() {
        let wide = || View::text("茶x").with_cursor(Cursor::new(1, 0));
        assert_eq!(
            wide().render(Size::new(3, 1)).unwrap().cursor(),
            Some(Cursor::new(0, 0))
        );
        assert_eq!(
            View::text("a")
                .with_cursor(Cursor::new(3, 0))
                .render(Size::new(3, 1))
                .unwrap()
                .cursor(),
            None
        );
        assert_eq!(wide().render(Size::new(0, 0)).unwrap().cursor(), None);
        for overlay in [View::text("X"), View::text("").panel("", &Theme::default())] {
            let frame = View::layers(vec![
                (Rect::new(0, 0, 3, 1), wide()),
                (Rect::new(1, 0, 1, 1), overlay),
            ])
            .render(Size::new(3, 1))
            .unwrap();
            assert_eq!(frame.cursor(), None);
        }
        let frame = View::layers(vec![
            (Rect::new(0, 0, 3, 1), wide()),
            (
                Rect::new(2, 0, 1, 1),
                View::text("z").with_cursor(Cursor::new(0, 0)),
            ),
        ])
        .render(Size::new(3, 1))
        .unwrap();
        assert_eq!(frame.cursor(), Some(Cursor::new(2, 0)));
    }
    #[test]
    fn cursor_metadata_does_not_damage_cells_and_invalid_positions_hide() {
        let a = View::text("tea").render(Size::new(4, 1)).unwrap();
        let mut b = a.clone();
        b.set_cursor(Some(Cursor::new(2, 0)));
        assert!(b.diff(Some(&a)).is_empty());
        assert_ne!(a, b);
        for cursor in [
            Cursor::new(4, 0),
            Cursor::new(0, 1),
            Cursor::new(u16::MAX, u16::MAX),
        ] {
            b.set_cursor(Some(cursor));
            assert_eq!(b.cursor(), None);
        }
    }
    #[test]
    fn sanitization_borrows_safe_input_and_handles_all_controls() {
        assert!(matches!(sanitized("ASCII 茶\ne\u{301}"), Cow::Borrowed(_)));
        assert_eq!(sanitized("\t\x00\x1b\u{85}\r"), "    ����");
        let long = format!("e{}", "\u{301}".repeat(40));
        let s = View::text(&long).render(Size::new(2, 1)).unwrap();
        assert_eq!(s.row(0)[0].symbol(), long);
        assert_eq!(s.clone(), s);
        assert_eq!(s.plain(), long);
    }
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
