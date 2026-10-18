//! Normalized immutable styled text, segmented across input style boundaries.
use crate::{Text, is_safe_text, write_sanitized};
use compact_str::CompactString;
use matea_style::Style;
use std::{ops::Range, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug)]
struct Transition {
    start: usize,
    style: Style,
}
#[derive(Debug)]
struct Data {
    text: Text,
    base: Style,
    transitions: Vec<Transition>,
    width: usize,
    lines: usize,
}

/// Owned immutable display text with portable cell styles. Style chunks are
/// joined before grapheme segmentation; a cluster uses its leading byte's
/// style, even when combining/emoji code points span input chunks.
///
/// ```
/// use matea_render::{RichText, Size, View};
/// use matea_style::{Color, Style};
/// let tea = RichText::from_spans([
///     ("抹茶", Style::default().fg(Color::Indexed(2))),
///     (" / Matea", Style::default()),
/// ]);
/// assert_eq!(View::rich_text(tea).render(Size::new(16, 1))?.plain(), "抹茶 / Matea");
/// # Ok::<(), matea_render::GeometryError>(())
/// ```
#[derive(Clone, Debug)]
pub struct RichText(Arc<Data>);

/// One complete grapheme from a RichText document. Newline is a structural
/// zero-width item; other zero-width items follow the compositor's policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StyledGrapheme<'a> {
    pub symbol: &'a str,
    pub style: Style,
    pub width: usize,
}

impl Default for RichText {
    fn default() -> Self {
        Self::styled("", Style::default())
    }
}
impl RichText {
    /// Own normalized text with one style. Clean owned Text/String inputs can
    /// retain their text storage; cloning a document shares immutable data.
    pub fn styled(text: impl Into<Text>, style: Style) -> Self {
        Self::finish(text.into(), style, Vec::new())
    }
    /// Normalize and join chunks once without retaining caller references or
    /// allocating a Text for every chunk. Empty chunks do not change style.
    /// Adjacent equal styles merge. Iterator/AsRef panics expose no partial doc.
    pub fn from_spans<I, S>(spans: I) -> Self
    where
        I: IntoIterator<Item = (S, Style)>,
        S: AsRef<str>,
    {
        let mut text = CompactString::default();
        let mut base = Style::default();
        let mut active = base;
        let mut transitions = Vec::new();
        for (span, style) in spans {
            let raw = span.as_ref();
            let start = text.len();
            if is_safe_text(raw) {
                text.push_str(raw);
            } else {
                write_sanitized(raw, &mut text).expect("writing normalized text cannot fail");
            }
            if start == text.len() {
                continue;
            }
            if start == 0 {
                base = style;
            } else if style != active {
                transitions.push(Transition { start, style });
            }
            active = style;
        }
        Self::finish(Text::from(text), base, transitions)
    }
    fn finish(text: Text, base: Style, transitions: Vec<Transition>) -> Self {
        let mut width = 0;
        let mut lines = 0;
        for line in text.split('\n') {
            let cells = if line.is_ascii() {
                line.len()
            } else {
                line.graphemes(true).map(UnicodeWidthStr::width).sum()
            };
            width = width.max(cells);
            lines += 1;
        }
        Self(Arc::new(Data {
            text,
            base,
            transitions,
            width,
            lines,
        }))
    }
    pub fn as_str(&self) -> &str {
        self.0.text.as_str()
    }
    pub fn len(&self) -> usize {
        self.0.text.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.text.is_empty()
    }
    /// Maximum row width under the compositor's per-grapheme width policy.
    pub fn width(&self) -> usize {
        self.0.width
    }
    /// Structural rows, including one empty row for empty text and a trailing
    /// empty row after LF, matching an ordinary text View.
    pub fn line_count(&self) -> usize {
        self.0.lines
    }
    /// Style of an existing byte, including interior bytes of a UTF-8 scalar.
    /// End/out-of-range positions return None; this method never slices text.
    pub fn style_at_byte(&self, byte: usize) -> Option<Style> {
        (byte < self.len()).then(|| self.style_cursor(byte).active)
    }
    /// Exact normalized UTF-8 ranges, including structural LF. Ranges cover
    /// the document without gaps/overlap and never split a Unicode scalar.
    pub fn styles(
        &self,
    ) -> impl ExactSizeIterator<Item = (Range<usize>, Style)>
    + DoubleEndedIterator
    + std::iter::FusedIterator {
        let count = if self.is_empty() {
            0
        } else {
            self.0.transitions.len() + 1
        };
        (0..count).map(|index| {
            let (start, style) = if index == 0 {
                (0, self.0.base)
            } else {
                let run = &self.0.transitions[index - 1];
                (run.start, run.style)
            };
            let end = self
                .0
                .transitions
                .get(index)
                .map_or(self.len(), |run| run.start);
            (start..end, style)
        })
    }
    pub fn graphemes(
        &self,
    ) -> impl DoubleEndedIterator<Item = StyledGrapheme<'_>> + std::iter::FusedIterator {
        self.as_str()
            .grapheme_indices(true)
            .fuse()
            .map(|(index, symbol)| StyledGrapheme {
                symbol,
                style: self
                    .style_at_byte(index)
                    .expect("grapheme starts inside owned text"),
                width: symbol.width(),
            })
    }
    pub(crate) fn uniform_style(&self) -> Option<Style> {
        self.0.transitions.is_empty().then_some(self.0.base)
    }
    pub(crate) fn style_cursor(&self, byte: usize) -> StyleCursor<'_> {
        let index = self.0.transitions.partition_point(|run| run.start <= byte);
        let active = if index == 0 {
            self.0.base
        } else {
            self.0.transitions[index - 1].style
        };
        StyleCursor {
            remaining: &self.0.transitions[index..],
            active,
        }
    }
}

pub(crate) struct StyleCursor<'a> {
    remaining: &'a [Transition],
    active: Style,
}
impl StyleCursor<'_> {
    pub(crate) fn at(&mut self, byte: usize) -> Style {
        while let Some((run, rest)) = self.remaining.split_first() {
            if run.start > byte {
                break;
            }
            self.active = run.style;
            self.remaining = rest;
        }
        self.active
    }
}
