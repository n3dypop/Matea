//! Validated byte matches with immutable shared geometry and local selection.
use super::Viewport;
use matea_render::{RichText, View};
use matea_style::Style;
use std::{ops::Range, sync::Arc};
use unicode_segmentation::{GraphemeIndices, UnicodeSegmentation};
use unicode_width::UnicodeWidthStr;

/// An invalid match in the iterator supplied to `Viewport::set_highlights`.
/// No content, selection or scroll position is changed on validation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HighlightError {
    Reversed { index: usize },
    OutOfBounds { index: usize },
    NotCharBoundary { index: usize },
    UnsortedOrOverlapping { index: usize },
}
impl std::fmt::Display for HighlightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (index, reason) = match *self {
            Self::Reversed { index } => (index, "reversed range"),
            Self::OutOfBounds { index } => (index, "outside normalized content"),
            Self::NotCharBoundary { index } => (index, "inside a UTF-8 scalar"),
            Self::UnsortedOrOverlapping { index } => (index, "unsorted or overlapping ranges"),
        };
        write!(f, "highlight {index}: {reason}")
    }
}
impl std::error::Error for HighlightError {}

#[derive(Clone, Debug)]
struct Highlight {
    bytes: Range<usize>,
    line: usize,
    columns: Range<usize>,
}
#[derive(Clone, Debug, Default)]
pub(super) struct HighlightState {
    matches: Option<Arc<Vec<Highlight>>>,
    selected: Option<usize>,
    paint: bool,
}
impl HighlightState {
    fn entries(&self) -> &[Highlight] {
        self.matches.as_deref().map_or(&[], Vec::as_slice)
    }
}

// Queries are monotone, but a scalar boundary may fall inside the current EGC.
// Keep that EGC pending until the query passes it. Multiple matches inside one
// combining sequence therefore require no repeated segmentation or rescanning.
struct Positions<'a> {
    graphemes: std::iter::Peekable<GraphemeIndices<'a>>,
    line: usize,
    column: usize,
}
impl<'a> Positions<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            graphemes: text.grapheme_indices(true).peekable(),
            line: 0,
            column: 0,
        }
    }
    fn at(&mut self, byte: usize, round_end: bool) -> (usize, usize) {
        while let Some(&(start, text)) = self.graphemes.peek() {
            if start + text.len() > byte {
                return (
                    self.line,
                    self.column.saturating_add(if round_end && byte > start {
                        text.width()
                    } else {
                        0
                    }),
                );
            }
            self.graphemes.next();
            if text == "\n" {
                self.line += 1;
                self.column = 0;
            } else {
                self.column = self.column.saturating_add(text.width());
            }
        }
        (self.line, self.column)
    }
}

impl Viewport {
    /// Replace matches against the **normalized** `content()` UTF-8 bytes.
    /// Ranges must be ordered, disjoint, in bounds and on scalar boundaries.
    /// A match inside a combining/emoji cluster styles the entire intersecting
    /// cluster. Empty ranges remain navigable but paint nothing; an empty
    /// iterator clears matches. Iterator panics expose no partial replacement.
    ///
    /// ```
    /// use matea_widgets::Viewport;
    /// let mut view = Viewport::new("抹茶 / tea");
    /// view.set_highlights([0..6, 9..12])?;
    /// assert_eq!(view.selected_highlight(), Some(0));
    /// assert_eq!(view.highlight_next(), Some(1));
    /// # Ok::<(), matea_widgets::HighlightError>(())
    /// ```
    pub fn set_highlights<I>(&mut self, ranges: I) -> Result<(), HighlightError>
    where
        I: IntoIterator<Item = Range<usize>>,
    {
        let ranges = ranges.into_iter();
        // Hints are untrusted. Reserve only after the first valid match, and
        // cap speculative storage independently of the caller's reported size.
        let hint = ranges.size_hint().0.min(256);
        let mut entries = Vec::new();
        let ascii = self.content.is_ascii();
        let mut positions = Positions::new(&self.content);
        let mut previous_end = 0;
        let mut paint = false;
        for (index, bytes) in ranges.enumerate() {
            if bytes.start > bytes.end {
                return Err(HighlightError::Reversed { index });
            }
            if bytes.end > self.content.len() {
                return Err(HighlightError::OutOfBounds { index });
            }
            if !self.content.is_char_boundary(bytes.start)
                || !self.content.is_char_boundary(bytes.end)
            {
                return Err(HighlightError::NotCharBoundary { index });
            }
            if bytes.start < previous_end {
                return Err(HighlightError::UnsortedOrOverlapping { index });
            }
            let (line, start) = if ascii {
                let line = self
                    .line_starts
                    .partition_point(|start| *start <= bytes.start)
                    .saturating_sub(1);
                (
                    line,
                    bytes.start - self.line_starts.get(line).copied().unwrap_or(0),
                )
            } else {
                positions.at(bytes.start, false)
            };
            let line_end = self
                .line_starts
                .get(line + 1)
                .map_or(self.content.len(), |next| next - 1);
            let end = if ascii {
                bytes.end.min(line_end) - self.line_starts.get(line).copied().unwrap_or(0)
            } else {
                positions.at(bytes.end.min(line_end), true).1
            };
            previous_end = bytes.end;
            paint |= !bytes.is_empty();
            if entries.is_empty() && hint != 0 {
                entries.reserve_exact(hint);
            }
            entries.push(Highlight {
                bytes,
                line,
                columns: start..end,
            });
        }
        self.highlights = HighlightState {
            matches: (!entries.is_empty()).then(|| Arc::new(entries)),
            selected: None,
            paint,
        };
        self.select_nearest_highlight();
        self.reveal_highlight();
        Ok(())
    }
    pub fn highlights(
        &self,
    ) -> impl ExactSizeIterator<Item = Range<usize>> + DoubleEndedIterator + std::iter::FusedIterator
    {
        self.highlights
            .entries()
            .iter()
            .map(|entry| entry.bytes.clone())
    }
    pub fn selected_highlight(&self) -> Option<usize> {
        self.highlights.selected
    }
    /// Clear geometry and selection. Return whether matches existed.
    pub fn clear_highlights(&mut self) -> bool {
        let changed = self.highlights.matches.is_some();
        self.highlights = HighlightState::default();
        changed
    }
    /// Select a valid match and reveal its first logical line/cell range.
    /// Invalid indices return false without changing any state.
    pub fn select_highlight(&mut self, index: usize) -> bool {
        if index >= self.highlights.entries().len() {
            return false;
        }
        self.highlights.selected = Some(index);
        self.reveal_highlight();
        true
    }
    pub fn highlight_next(&mut self) -> Option<usize> {
        let len = self.highlights.entries().len();
        if len == 0 {
            return None;
        }
        let index = self
            .highlights
            .selected
            .map_or(0, |index| if index + 1 == len { 0 } else { index + 1 });
        self.select_highlight(index);
        Some(index)
    }
    pub fn highlight_previous(&mut self) -> Option<usize> {
        let len = self.highlights.entries().len();
        if len == 0 {
            return None;
        }
        let index =
            self.highlights.selected.map_or(
                len - 1,
                |index| if index == 0 { len - 1 } else { index - 1 },
            );
        self.select_highlight(index);
        Some(index)
    }
    pub(super) fn select_nearest_highlight(&mut self) {
        let entries = self.highlights.entries();
        if entries.is_empty() {
            return;
        }
        let offset = self.offset();
        let rows = self.wrap_index();
        let index = entries.partition_point(|entry| {
            let row = rows.map_or(entry.line, |rows| {
                rows.starts
                    .partition_point(|start| *start <= entry.bytes.start)
                    .saturating_sub(1)
            });
            row < offset
        });
        self.highlights.selected = (index < entries.len()).then_some(index);
    }
    fn reveal_highlight(&mut self) {
        if let Some(index) = self.highlights.selected {
            let entry = &self.highlights.entries()[index];
            let (line, columns) = (entry.line, entry.columns.clone());
            self.ensure_visible(line, columns)
                .expect("validated cell columns are ordered");
        }
    }
    fn range_style(&self, bytes: Range<usize>, base: Style) -> Style {
        let entries = self.highlights.entries();
        let first = entries.partition_point(|entry| entry.bytes.end <= bytes.start);
        let mut matched = false;
        let mut selected = false;
        for (offset, entry) in entries[first..].iter().enumerate() {
            if entry.bytes.start >= bytes.end {
                break;
            }
            if !entry.bytes.is_empty() {
                matched = true;
                selected |= self.highlights.selected == Some(first + offset);
            }
        }
        let style = if matched {
            self.highlight_style.resolve(base)
        } else {
            base
        };
        if selected {
            self.selected_highlight_style.resolve(style)
        } else {
            style
        }
    }
    fn append_styled<'a>(
        &'a self,
        spans: &mut Vec<(&'a str, Style)>,
        text: &'a str,
        byte: usize,
        base: Style,
    ) {
        if text.is_ascii() {
            // In normalized ASCII every non-LF byte is a complete one-cell
            // grapheme. Walk only intersecting intervals rather than searching
            // and segmenting again for each byte of every visible row.
            let entries = self.highlights.entries();
            let first = entries.partition_point(|entry| entry.bytes.end <= byte);
            let end = byte + text.len();
            let mut cursor = byte;
            for (offset, entry) in entries[first..].iter().enumerate() {
                if entry.bytes.start >= end {
                    break;
                }
                if entry.bytes.is_empty() {
                    continue;
                }
                let start = entry.bytes.start.max(byte);
                if cursor < start {
                    spans.push((&text[cursor - byte..start - byte], base));
                }
                let stop = entry.bytes.end.min(end);
                let style = self.highlight_style.resolve(base);
                let style = if self.highlights.selected == Some(first + offset) {
                    self.selected_highlight_style.resolve(style)
                } else {
                    style
                };
                spans.push((&text[start - byte..stop - byte], style));
                cursor = stop;
            }
            if cursor < end {
                spans.push((&text[cursor - byte..], base));
            }
            return;
        }
        let mut start = 0;
        let mut active = base;
        for (index, cluster) in text.grapheme_indices(true) {
            let style = self.range_style(byte + index..byte + index + cluster.len(), base);
            if index == 0 {
                active = style;
            } else if style != active {
                spans.push((&text[start..index], active));
                start = index;
                active = style;
            }
        }
        if !text.is_empty() {
            spans.push((&text[start..], active));
        }
    }
    pub(super) fn highlighted_view(&self, base: Style) -> Option<View> {
        if !self.highlights.paint
            || (self.highlight_style.is_empty() && self.selected_highlight_style.is_empty())
        {
            return None;
        }
        // Bound speculation for large caller-provided heights. The common
        // three-row search window needs eleven spans, without repeated growth.
        let mut spans = Vec::with_capacity(self.visible_line_count().min(16) * 4);
        let wrapped = self.wrap_index();
        let offset = self.offset();
        for row in 0..self.visible_line_count() {
            if row != 0 {
                spans.push(("\n", base));
            }
            let index = offset + row;
            if let Some(rows) = wrapped {
                let Some(&start) = rows.starts.get(index) else {
                    continue;
                };
                let mut end = rows
                    .starts
                    .get(index + 1)
                    .copied()
                    .unwrap_or(self.content.len());
                if end > start && self.content.as_bytes()[end - 1] == b'\n' {
                    end -= 1;
                }
                if rows.replacements.binary_search(&index).is_ok() {
                    spans.push(("�", self.range_style(start..end, base)));
                } else {
                    self.append_styled(&mut spans, &self.content[start..end], start, base);
                }
            } else if let Some(line) = self.line(index) {
                let start = self.line_starts[index];
                if let Some(width) = self.width {
                    let (source_start, source_end, padding) = super::viewport_cell_bounds(
                        line,
                        self.horizontal_offset,
                        usize::from(width),
                    );
                    for _ in 0..padding {
                        spans.push((" ", base));
                    }
                    self.append_styled(
                        &mut spans,
                        &line[source_start..source_end],
                        start + source_start,
                        base,
                    );
                } else {
                    self.append_styled(&mut spans, line, start, base);
                }
            }
        }
        Some(View::rich_text(RichText::from_spans(spans)))
    }
}
