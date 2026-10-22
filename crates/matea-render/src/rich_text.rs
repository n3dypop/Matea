//! Normalized immutable styled text, segmented across input style boundaries.
use crate::{Text, is_safe_text, write_sanitized};
use compact_str::CompactString;
use matea_style::Style;
use std::{ops::Range, sync::Arc};
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};
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
        let spans = spans.into_iter();
        // A hint is advisory, including for user iterators. Bound speculation;
        // never reserve an untrusted usize::MAX or allocate styles for a
        // uniform document. Unknown/irregular chunk sizes retain normal growth.
        let hint = spans.size_hint().0.min(64);
        let mut text = CompactString::default();
        let mut base = Style::default();
        let mut active = base;
        let mut transitions = Vec::new();
        for (span, style) in spans {
            let raw = span.as_ref();
            let start = text.len();
            if start == 0 && !raw.is_empty() && hint > 1 {
                let capacity = raw.len().saturating_mul(hint).min(64 * 1024);
                if capacity > text.capacity() {
                    text = CompactString::with_capacity(capacity);
                }
            }
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
                if transitions.is_empty() {
                    transitions.reserve_exact(hint.saturating_sub(1).max(1));
                }
                transitions.push(Transition { start, style });
            }
            active = style;
        }
        Self::finish(Text::from_normalized(text), base, transitions)
    }
    fn finish(text: Text, base: Style, transitions: Vec<Transition>) -> Self {
        let mut width = 0;
        let mut lines = 0;
        for line in text.split('\n') {
            width = width.max(cell_width(line));
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

// unicode-width is pinned to 0.2.2. Its whole-string fast path agrees with
// per-EGC width except script ligatures that may cross grapheme boundaries.
// Conservatively fall back for entire relevant script blocks, including Khmer
// and Kirat Rai even when the current segmentation keeps a sequence together.
// LF is split by the caller and CR was normalized, so CRLF is not a case here.
// Audit this list against unicode-width's documented rules when upgrading it.
fn cell_width(line: &str) -> usize {
    if line.is_ascii() {
        return line.len();
    }
    let script = line.chars().any(|c| {
        matches!(c,
            '\u{500}'..='\u{8ff}' | // Hebrew / Arabic and joining extensions
            '\u{1780}'..='\u{17ff}' | // Khmer
            '\u{1a00}'..='\u{1a1f}' | // Buginese
            '\u{2d30}'..='\u{2d7f}' | // Tifinagh
            '\u{a4d0}'..='\u{a4ff}' | // Lisu
            '\u{10c00}'..='\u{10c4f}' | // Old Turkic
            '\u{16d40}'..='\u{16d7f}' // Kirat Rai
        )
    });
    // unicode-width also recognizes keycap/flag/modifier ZWJ combinations
    // that Unicode grapheme segmentation may split. Check those boundaries
    // with the same segmentation engine, without walking every other EGC.
    let split_zwj = line.match_indices('\u{200d}').any(|(index, zwj)| {
        GraphemeCursor::new(index + zwj.len(), line.len(), true).is_boundary(line, 0) != Ok(false)
    });
    if script || split_zwj {
        line.graphemes(true).map(UnicodeWidthStr::width).sum()
    } else {
        line.width()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;
    fn model(text: &str) -> usize {
        text.graphemes(true).map(UnicodeWidthStr::width).sum()
    }
    #[test]
    fn fast_width_preserves_cross_grapheme_script_and_emoji_policies() {
        for text in [
            "لا",
            "ل\u{65f}\u{65e}أ",
            "ڸا",
            "א‍ל",
            "ᨕᨗ‍ᨐ",
            "\u{17d2}ក",
            "\u{16d63}\u{16d67}",
            "ꓹꓼ",
            "𐰲‍𐰃",
            "ⵏ⵿ⴾ",
            "ⵏ‍ⴾ",
            "e\u{301}👩‍💻🇯🇵1️⃣",
            "1️⃣‍💻",
            "🇯🇵‍💻",
            "\u{1f3fb}‍💻",
            "#\u{fe0f}‍💻",
            "👩‍🇯🇵",
            "“\u{fe01}tea”,",
            "\u{301}A\u{200b}",
        ] {
            assert_eq!(cell_width(text), model(text), "{text:?}");
        }
        assert_ne!("لا".width(), model("لا"));
        for first in '\u{2d31}'..='\u{2d65}' {
            for last in '\u{2d31}'..='\u{2d65}' {
                for joiner in ['\u{2d7f}', '\u{200d}'] {
                    let text = format!("{first}{joiner}{last}");
                    assert_eq!(cell_width(&text), model(&text));
                }
            }
        }
    }
    #[test]
    #[ignore = "exhaustive scalar contexts, explicitly executed by native acceptance"]
    fn every_scalar_context_matches_per_grapheme_width() {
        let mut text = String::new();
        for code in 0..=0x10ffff {
            let Some(c) = char::from_u32(code) else {
                continue;
            };
            if c.is_control() {
                continue;
            }
            for position in 0..3 {
                text.clear();
                match position {
                    0 => write!(&mut text, "A{c}e\u{301}👩‍💻茶X"),
                    1 => write!(&mut text, "A{c}‍💻X"),
                    _ => write!(&mut text, "A👩‍{c}X"),
                }
                .unwrap();
                assert_eq!(
                    cell_width(&text),
                    model(&text),
                    "U+{code:04X} context {position}"
                );
            }
        }
    }
}
