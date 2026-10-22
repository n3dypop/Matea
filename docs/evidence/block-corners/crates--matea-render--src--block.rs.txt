//! Owned block decoration and bounded structured-cell layout.
use crate::{GeometryError, Rect, RichText, Size, Style, Surface, Text, View};
use matea_style::{BlockStyle, BorderPart, Insets, TabWidth, TextAttribute, UnderlineStyle};
use std::{borrow::Cow, fmt};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockError {
    Geometry(GeometryError),
    DimensionOverflow,
    AllocationFailed,
    InvalidFillCharacter,
    TransformOnRichText,
}
impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Geometry(e)=>e.fmt(f),Self::DimensionOverflow=>f.write_str("block dimensions exceed representable cell coordinates; supply a viewport limit"),
            Self::AllocationFailed=>f.write_str("cannot reserve normalized block text"),
            Self::InvalidFillCharacter=>f.write_str("block fill must be a visible non-control character"),
            Self::TransformOnRichText=>f.write_str("transform rich text explicitly before constructing a block to preserve style ownership"),
        }
    }
}
impl std::error::Error for BlockError {}
impl From<GeometryError> for BlockError {
    fn from(e: GeometryError) -> Self {
        Self::Geometry(e)
    }
}

/// Owned text and decoration. Construction transforms/converts tabs before
/// normalization, then shares immutable RichText storage on Clone. Rendering
/// has no terminal I/O; a viewport limit bounds allocation before drawing.
///
/// ```
/// use matea_render::{StyledBlock, Size};
/// use matea_style::{BlockStyle, BorderPreset, Insets};
/// let block = StyledBlock::new("Matea", BlockStyle::default()
///     .border(BorderPreset::Rounded).padding(Insets::symmetric(0, 1)))?;
/// assert_eq!(block.render()?.plain(), "╭───────╮\n│ Matea │\n╰───────╯");
/// assert_eq!(block.render_within(Size::new(4, 2))?.size(), Size::new(4, 2));
/// # Ok::<(), matea_render::BlockError>(())
/// ```
#[derive(Clone, Debug)]
pub struct StyledBlock {
    text: RichText,
    style: BlockStyle,
}
impl StyledBlock {
    pub fn new(text: impl AsRef<str>, style: BlockStyle) -> Result<Self, BlockError> {
        validate_fill(&style)?;
        let text = text.as_ref();
        let transformed = style
            .transform
            .as_ref()
            .map_or(Cow::Borrowed(text), |f| Cow::Owned(f.apply(text)));
        let raw = if transformed.contains("\r\n") {
            Cow::Owned(transformed.replace("\r\n", "\n"))
        } else {
            transformed
        };
        let inline = style.inline == Some(true);
        if !raw.contains('\t') && !(inline && raw.contains('\n')) {
            let text = RichText::styled(Text::from(raw), Style::default());
            return Ok(Self { text, style });
        }
        let tabs = match style.tab_width.unwrap_or_default() {
            TabWidth::Spaces(n) => usize::from(n),
            TabWidth::Remove => 0,
        };
        let capacity = raw
            .len()
            .checked_add(
                raw.bytes()
                    .filter(|&b| b == b'\t')
                    .count()
                    .checked_mul(tabs)
                    .ok_or(BlockError::DimensionOverflow)?,
            )
            .ok_or(BlockError::DimensionOverflow)?;
        let mut normalized = String::new();
        normalized
            .try_reserve(capacity)
            .map_err(|_| BlockError::AllocationFailed)?;
        for c in raw.chars() {
            match c {
                '\t' => normalized.extend(std::iter::repeat_n(' ', tabs)),
                '\n' if style.inline == Some(true) => {}
                c => normalized.push(c),
            }
        }
        let text = RichText::styled(Text::from(normalized), Style::default());
        Ok(Self { text, style })
    }
    pub fn from_rich_text(text: RichText, style: BlockStyle) -> Result<Self, BlockError> {
        validate_fill(&style)?;
        if style.transform.is_some() {
            return Err(BlockError::TransformOnRichText);
        }
        let text = if style.inline == Some(true) && text.as_str().contains('\n') {
            RichText::from_spans(
                text.styles()
                    .map(|(range, style)| (text.as_str()[range].replace('\n', ""), style)),
            )
        } else {
            text
        };
        Ok(Self { text, style })
    }
    pub fn text(&self) -> &RichText {
        &self.text
    }
    pub fn style(&self) -> &BlockStyle {
        &self.style
    }
    /// Natural final block dimensions; minima include padding/border and maxima
    /// crop after margins. Unrepresentable dimensions are an explicit error.
    pub fn size(&self) -> Result<Size, BlockError> {
        self.layout().size(None)
    }
    pub fn render(&self) -> Result<Surface, BlockError> {
        self.render_limit(None)
    }
    pub fn render_within(&self, limit: Size) -> Result<Surface, BlockError> {
        self.render_limit(Some(limit))
    }
    pub fn into_view(self) -> Result<View, BlockError> {
        self.render().map(View::surface)
    }
    pub fn view_within(&self, limit: Size) -> Result<View, BlockError> {
        self.render_within(limit).map(View::surface)
    }
    fn layout(&self) -> Layout<'_> {
        let inline = self.style.inline == Some(true);
        let (padding, margin, border) = if inline {
            (Insets::default(), Insets::default(), Insets::default())
        } else {
            (
                self.style.padding.resolved(),
                self.style.margin.resolved(),
                self.style.border_insets(),
            )
        };
        let border_w = border.horizontal() as usize;
        let wrap = self
            .style
            .width
            .map(usize::from)
            .unwrap_or(0)
            .saturating_sub(border_w + padding.horizontal() as usize);
        let rows = wrap_rows(&self.text, if inline { 0 } else { wrap });
        let content_w = rows.iter().map(|r| r.width).max().unwrap_or(0);
        let inner_w = (content_w + padding.horizontal() as usize).max(
            self.style
                .width
                .map(usize::from)
                .unwrap_or(0)
                .saturating_sub(border_w),
        );
        let inner_h = (rows.len() + padding.vertical() as usize).max(
            self.style
                .height
                .map(usize::from)
                .unwrap_or(0)
                .saturating_sub(border.vertical() as usize),
        );
        Layout {
            rows,
            text: &self.text,
            padding,
            margin,
            border,
            inner_w,
            inner_h,
            style: &self.style,
        }
    }
    fn render_limit(&self, limit: Option<Size>) -> Result<Surface, BlockError> {
        let layout = self.layout();
        let mut surface = Surface::new(layout.size(limit)?)?;
        layout.paint(&mut surface);
        Ok(surface)
    }
}
fn validate_fill(style: &BlockStyle) -> Result<(), BlockError> {
    for c in [style.padding_char, style.margin_char]
        .into_iter()
        .flatten()
    {
        if c.is_control() || c.width().unwrap_or(0) == 0 {
            return Err(BlockError::InvalidFillCharacter);
        }
    }
    Ok(())
}
#[derive(Clone, Copy, Default)]
struct Row {
    start: usize,
    end: usize,
    width: usize,
}
impl Row {
    fn empty(self) -> bool {
        self.start == self.end
    }
    fn add(&mut self, start: usize, end: usize, width: usize) {
        if self.empty() {
            self.start = start
        }
        self.end = end;
        self.width += width;
    }
    fn append(&mut self, other: &mut Self) {
        if !other.empty() {
            self.add(other.start, other.end, other.width)
        }
        *other = Self::default();
    }
}
fn flush_word(rows: &mut Vec<Row>, row: &mut Row, word: &mut Row, spaces: &mut Row, limit: usize) {
    if word.empty() {
        return;
    }
    if limit > 0 && row.width + spaces.width + word.width > limit {
        if !row.empty() {
            newline(rows, row, spaces);
        } else {
            *spaces = Row::default();
        }
    }
    row.append(spaces);
    row.append(word);
}
fn newline(rows: &mut Vec<Row>, row: &mut Row, spaces: &mut Row) {
    rows.push(std::mem::take(row));
    *spaces = Row::default();
}
// Each display row is a range of the shared normalized document, not a copy of
// every styled character. Dropped wrap-boundary whitespace lies outside ranges.
fn wrap_rows(text: &RichText, limit: usize) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut row = Row::default();
    let mut word = Row::default();
    let mut spaces = Row::default();
    for (start, symbol) in text.as_str().grapheme_indices(true) {
        let end = start + symbol.len();
        let width = symbol.width();
        if symbol == "\n" {
            if word.empty() && (limit == 0 || row.width + spaces.width <= limit) {
                row.append(&mut spaces)
            }
            flush_word(&mut rows, &mut row, &mut word, &mut spaces, limit);
            newline(&mut rows, &mut row, &mut spaces);
            continue;
        }
        if width == 0 {
            continue;
        }
        if limit == 0 {
            row.add(start, end, width);
            continue;
        }
        let first = symbol.chars().next().expect("a grapheme is nonempty");
        if first.is_whitespace() && first != '\u{a0}' {
            flush_word(&mut rows, &mut row, &mut word, &mut spaces, limit);
            spaces.add(start, end, width);
            continue;
        }
        if first == '-' {
            if row.width + spaces.width > limit {
                if !row.empty() {
                    newline(&mut rows, &mut row, &mut spaces);
                } else {
                    spaces = Row::default();
                }
            } else {
                row.append(&mut spaces);
            }
            if row.width + word.width + width > limit {
                // Keep a breakpoint with the pending word when it fits on the
                // next row. Flush a full pending word before it can grow past
                // the limit (the reference's equality-only check misses this).
                if word.width + width > limit {
                    flush_word(&mut rows, &mut row, &mut word, &mut spaces, limit);
                }
                word.add(start, end, width);
                if word.width >= limit {
                    if !row.empty() {
                        newline(&mut rows, &mut row, &mut spaces);
                    }
                    flush_word(&mut rows, &mut row, &mut word, &mut spaces, limit);
                }
            } else {
                flush_word(&mut rows, &mut row, &mut word, &mut spaces, limit);
                row.add(start, end, width);
            }
            continue;
        }
        if word.width + width > limit {
            flush_word(&mut rows, &mut row, &mut word, &mut spaces, limit)
        }
        if row.width == limit || row.width + word.width + width + spaces.width > limit {
            // An indivisible wide glyph may exceed a wrap target. Keep it on
            // the empty row instead of creating a spurious leading blank row.
            if !row.empty() || !word.empty() {
                newline(&mut rows, &mut row, &mut spaces)
            } else {
                spaces = Row::default()
            }
        }
        word.add(start, end, width);
        if word.width >= limit {
            flush_word(&mut rows, &mut row, &mut word, &mut spaces, limit)
        }
    }
    if word.empty() && (limit == 0 || row.width + spaces.width <= limit) {
        row.append(&mut spaces)
    }
    flush_word(&mut rows, &mut row, &mut word, &mut spaces, limit);
    rows.push(row);
    rows
}
struct Layout<'a> {
    rows: Vec<Row>,
    text: &'a RichText,
    padding: Insets,
    margin: Insets,
    border: Insets,
    inner_w: usize,
    inner_h: usize,
    style: &'a BlockStyle,
}
impl Layout<'_> {
    fn size(&self, limit: Option<Size>) -> Result<Size, BlockError> {
        let width = self
            .inner_w
            .checked_add(self.border.horizontal() as usize)
            .and_then(|n| n.checked_add(self.margin.horizontal() as usize))
            .ok_or(BlockError::DimensionOverflow)?;
        let height = self
            .inner_h
            .checked_add(self.border.vertical() as usize)
            .and_then(|n| n.checked_add(self.margin.vertical() as usize))
            .ok_or(BlockError::DimensionOverflow)?;
        let width = width
            .min(
                self.style
                    .max_width
                    .filter(|&n| n > 0)
                    .map_or(usize::MAX, usize::from),
            )
            .min(limit.map_or(usize::MAX, |s| usize::from(s.width)));
        let height = height
            .min(
                self.style
                    .max_height
                    .filter(|&n| n > 0)
                    .map_or(usize::MAX, usize::from),
            )
            .min(limit.map_or(usize::MAX, |s| usize::from(s.height)));
        Ok(Size::new(
            width
                .try_into()
                .map_err(|_| BlockError::DimensionOverflow)?,
            height
                .try_into()
                .map_err(|_| BlockError::DimensionOverflow)?,
        ))
    }
    fn paint(&self, s: &mut Surface) {
        let default = self.style.text.resolve(Style::default());
        let whitespace = whitespace_style(default, self.style.color_whitespace.unwrap_or(true));
        let margin_style = self.style.margin_style.resolve(Style::default());
        s.canvas(s.area()).fill(margin_style);
        let (mx, my) = (usize::from(self.margin.left), usize::from(self.margin.top));
        let (ix, iy) = (
            mx + usize::from(self.border.left),
            my + usize::from(self.border.top),
        );
        fill_region(s, ix, iy, self.inner_w, self.inner_h, whitespace);
        let content_y = iy
            + self.padding.top as usize
            + self.style.vertical.unwrap_or_default().offset(
                self.inner_h
                    .saturating_sub(self.rows.len() + self.padding.vertical() as usize),
            );
        for (row_index, row) in self.rows.iter().enumerate() {
            let y = content_y + row_index;
            if y >= usize::from(s.size().height) {
                break;
            }
            let leading = self.style.horizontal.unwrap_or_default().offset(
                self.inner_w
                    .saturating_sub(row.width + self.padding.horizontal() as usize),
            );
            let mut x = ix + leading + self.padding.left as usize;
            fill_char(
                s,
                ix + leading,
                y,
                self.padding.left as usize,
                self.style.padding_char.unwrap_or(' '),
                whitespace,
            );
            let mut cursor = self.text.style_cursor(row.start);
            for (offset, symbol) in self.text.as_str()[row.start..row.end].grapheme_indices(true) {
                if x >= usize::from(s.size().width) {
                    break;
                }
                let mut style = cursor.at(row.start + offset).patched(self.style.text);
                // A cluster keeps its leading scalar's style, including a
                // space followed by combining marks or variation selectors.
                if symbol.chars().next().is_some_and(char::is_whitespace) {
                    style = space_style(style, self.style);
                }
                draw(s, x, y, symbol, style);
                x += symbol.width();
            }
            fill_char(
                s,
                x,
                y,
                self.padding.right as usize,
                self.style.padding_char.unwrap_or(' '),
                whitespace,
            );
        }
        self.paint_border(s, mx, my);
        // Horizontal margins use the custom fill; top/bottom remain spaces,
        // matching source MarginChar's separate horizontal padding contract.
        let box_h = self.inner_h + self.border.vertical() as usize;
        let right = ix + self.inner_w + self.border.right as usize;
        for y in my..(my + box_h).min(usize::from(s.size().height)) {
            fill_char(
                s,
                0,
                y,
                self.margin.left as usize,
                self.style.margin_char.unwrap_or(' '),
                margin_style,
            );
            fill_char(
                s,
                right,
                y,
                self.margin.right as usize,
                self.style.margin_char.unwrap_or(' '),
                margin_style,
            );
        }
    }
    fn paint_border(&self, s: &mut Surface, x: usize, y: usize) {
        let Some(border) = &self.style.border else {
            return;
        };
        let sides = self.style.sides();
        if self.style.inline == Some(true) {
            return;
        }
        let width = self.inner_w + self.border.horizontal() as usize;
        let gradient_count = (width + self.inner_h + 2) * 2;
        let border_style = |base: matea_style::StylePatch, index: usize| {
            let mut style = base.resolve(Style::default());
            if let Some(gradient) = &self.style.border_gradient {
                let offset = i128::from(self.style.border_gradient_offset.unwrap_or(0));
                let index = (index as i128 - offset).rem_euclid(gradient_count as i128) as usize;
                if let Some(color) = gradient.color_at(index, gradient_count) {
                    style.fg = color
                }
            }
            style
        };
        for (enabled, part, left, right, row_y, patch, bottom) in [
            (
                sides.top,
                BorderPart::Top,
                BorderPart::TopLeft,
                BorderPart::TopRight,
                y,
                self.style.border_style.top,
                false,
            ),
            (
                sides.bottom,
                BorderPart::Bottom,
                BorderPart::BottomLeft,
                BorderPart::BottomRight,
                y + self.border.top as usize + self.inner_h,
                self.style.border_style.bottom,
                true,
            ),
        ] {
            if !enabled || row_y >= usize::from(s.size().height) {
                continue;
            }
            let (_, lw) = border.corner(left);
            let (_, rw) = border.corner(right);
            let lw = if sides.left { usize::from(lw) } else { 0 };
            let rw = if sides.right { usize::from(rw) } else { 0 };
            let index = |column: usize| {
                if bottom {
                    width + 2 + self.inner_h + (width + 1 - column)
                } else {
                    column
                }
            };
            if sides.left {
                draw_pattern(s, x, row_y, border.corner_glyphs(left), |column| {
                    border_style(patch, index(column))
                })
            }
            let mut column = lw;
            let mut pattern = 0;
            let end = width.saturating_sub(rw);
            while column < end && x + column < usize::from(s.size().width) {
                let (glyph, glyph_w) = border.glyph_at(part, pattern);
                let w = usize::from(glyph_w);
                if w > end - column {
                    draw(
                        s,
                        x + column,
                        row_y,
                        " ",
                        border_style(patch, index(column)),
                    );
                    column += 1;
                } else {
                    draw(
                        s,
                        x + column,
                        row_y,
                        glyph,
                        border_style(patch, index(column)),
                    );
                    column += w;
                }
                pattern += 1;
            }
            if sides.right {
                let column = width.saturating_sub(rw);
                draw_pattern(
                    s,
                    x + column,
                    row_y,
                    border.corner_glyphs(right),
                    |offset| border_style(patch, index(column + offset)),
                );
            }
        }
        for row in 0..self
            .inner_h
            .min(usize::from(s.size().height).saturating_sub(y + self.border.top as usize))
        {
            let row_y = y + self.border.top as usize + row;
            for (enabled, part, column, reserved, patch, index) in [
                (
                    sides.left,
                    BorderPart::Left,
                    x,
                    self.border.left,
                    self.style.border_style.left,
                    2 * (width + 2) + self.inner_h + (self.inner_h - 1 - row),
                ),
                (
                    sides.right,
                    BorderPart::Right,
                    x + self.border.left as usize + self.inner_w,
                    self.border.right,
                    self.style.border_style.right,
                    width + 2 + row,
                ),
            ] {
                if !enabled {
                    continue;
                }
                let style = border_style(patch, index);
                fill_region(s, column, row_y, reserved as usize, 1, style);
                let (glyph, glyph_width) = border.glyph_at(part, row);
                let offset = if part == BorderPart::Right {
                    usize::from(reserved.saturating_sub(glyph_width))
                } else {
                    0
                };
                draw(s, column + offset, row_y, glyph, style);
            }
        }
    }
}
fn region(s: &Surface, x: usize, y: usize, w: usize, h: usize) -> Rect {
    let (right, bottom) = (usize::from(s.size().width), usize::from(s.size().height));
    Rect::new(
        x.min(right) as u16,
        y.min(bottom) as u16,
        w.min(right.saturating_sub(x)) as u16,
        h.min(bottom.saturating_sub(y)) as u16,
    )
}
fn fill_region(s: &mut Surface, x: usize, y: usize, w: usize, h: usize, style: Style) {
    let r = region(s, x, y, w, h);
    s.canvas(r).fill(style)
}
fn draw(s: &mut Surface, x: usize, y: usize, text: &str, style: Style) {
    if x < usize::from(s.size().width) && y < usize::from(s.size().height) {
        let area = s.area();
        s.canvas(area).text(x as u16, y as u16, text, style);
    }
}
fn draw_pattern<'a>(
    s: &mut Surface,
    x: usize,
    y: usize,
    glyphs: impl IntoIterator<Item = (&'a str, u16)>,
    mut style: impl FnMut(usize) -> Style,
) {
    let mut offset = 0;
    for (glyph, width) in glyphs {
        if x + offset >= usize::from(s.size().width) {
            break;
        }
        draw(s, x + offset, y, glyph, style(offset));
        offset += usize::from(width);
    }
}

fn fill_char(s: &mut Surface, x: usize, y: usize, count: usize, c: char, style: Style) {
    if c == ' ' {
        fill_region(s, x, y, count, 1, style);
        return;
    }
    let mut encoded = [0; 4];
    let glyph = c.encode_utf8(&mut encoded);
    let width = c.width().unwrap_or(1);
    let mut offset = 0;
    while offset < count && x + offset < usize::from(s.size().width) {
        if width > count - offset {
            draw(s, x + offset, y, " ", style);
            offset += 1
        } else {
            draw(s, x + offset, y, glyph, style);
            offset += width
        }
    }
}
fn whitespace_style(body: Style, color: bool) -> Style {
    let mut out = Style::default();
    if color {
        out.bg = body.bg;
        out.underline_color = body.underline_color
    }
    if body.has(TextAttribute::Reverse) {
        out = out.reverse().fg(body.fg)
    }
    out
}
fn space_style(body: Style, block: &BlockStyle) -> Style {
    let underline = body.attributes.underline() != UnderlineStyle::None;
    let strike = body.has(TextAttribute::Strikethrough);
    let underline_spaces = block.underline_spaces.unwrap_or(underline);
    let strike_spaces = block.strikethrough_spaces.unwrap_or(strike);
    if !(underline || strike || underline_spaces || strike_spaces) {
        return body;
    }
    let mut out = Style::default().fg(body.fg).bg(body.bg);
    out.underline_color = body.underline_color;
    if underline_spaces {
        out = out.underline()
    }
    if strike_spaces {
        out = out.strikethrough()
    }
    out
}
