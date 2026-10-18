//! Owned cell geometry, borrowed windows and focused pager navigation.
use crate::{Action, key};
use matea_core::{Component, Input, KeyCode, Response};
use matea_render::View;
use matea_style::Theme;

/// Pager commands used by [`ViewportKeys`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewportKey {
    PageDown,
    PageUp,
    HalfPageUp,
    HalfPageDown,
    Down,
    Up,
    Left,
    Right,
    Top,
    Bottom,
}
const VIEWPORT_KEYS: [ViewportKey; 10] = [
    ViewportKey::PageDown,
    ViewportKey::PageUp,
    ViewportKey::HalfPageUp,
    ViewportKey::HalfPageDown,
    ViewportKey::Down,
    ViewportKey::Up,
    ViewportKey::Left,
    ViewportKey::Right,
    ViewportKey::Top,
    ViewportKey::Bottom,
];
fn viewport_default_keys(action: ViewportKey) -> &'static [matea_core::Key] {
    use matea_core::{Key, Modifiers};
    const DEFAULTS: [&[Key]; 10] = [
        &[
            Key::new(KeyCode::PageDown),
            Key::new(KeyCode::Char(' ')),
            Key::new(KeyCode::Char('f')),
            Key::new(KeyCode::PageDown).with_modifiers(Modifiers::SHIFT),
        ],
        &[
            Key::new(KeyCode::PageUp),
            Key::new(KeyCode::Char('b')),
            Key::new(KeyCode::PageUp).with_modifiers(Modifiers::SHIFT),
        ],
        &[
            Key::new(KeyCode::Char('u')),
            Key::new(KeyCode::Char('u')).with_modifiers(Modifiers::CONTROL),
        ],
        &[
            Key::new(KeyCode::Char('d')),
            Key::new(KeyCode::Char('d')).with_modifiers(Modifiers::CONTROL),
        ],
        &[
            Key::new(KeyCode::Down),
            Key::new(KeyCode::Char('j')),
            Key::new(KeyCode::Down).with_modifiers(Modifiers::SHIFT),
        ],
        &[
            Key::new(KeyCode::Up),
            Key::new(KeyCode::Char('k')),
            Key::new(KeyCode::Up).with_modifiers(Modifiers::SHIFT),
        ],
        &[
            Key::new(KeyCode::Left),
            Key::new(KeyCode::Char('h')),
            Key::new(KeyCode::Left).with_modifiers(Modifiers::SHIFT),
        ],
        &[
            Key::new(KeyCode::Right),
            Key::new(KeyCode::Char('l')),
            Key::new(KeyCode::Right).with_modifiers(Modifiers::SHIFT),
        ],
        &[
            Key::new(KeyCode::Home),
            Key::new(KeyCode::Home).with_modifiers(Modifiers::SHIFT),
        ],
        &[
            Key::new(KeyCode::End),
            Key::new(KeyCode::End).with_modifiers(Modifiers::SHIFT),
        ],
    ];
    DEFAULTS[action as usize]
}
fn viewport_default_help(action: ViewportKey) -> (&'static str, &'static str) {
    match action {
        ViewportKey::PageDown => ("f/pgdn", "page down"),
        ViewportKey::PageUp => ("b/pgup", "page up"),
        ViewportKey::HalfPageUp => ("u", "½ page up"),
        ViewportKey::HalfPageDown => ("d", "½ page down"),
        ViewportKey::Down => ("↓/j", "down"),
        ViewportKey::Up => ("↑/k", "up"),
        ViewportKey::Left => ("←/h", "move left"),
        ViewportKey::Right => ("→/l", "move right"),
        ViewportKey::Top => ("home", "top"),
        ViewportKey::Bottom => ("end", "bottom"),
    }
}
/// Constant pager defaults with owned per-action overrides. Default matching,
/// chord inspection and help require no per-viewport key allocations.
#[derive(Clone, Debug, Default)]
pub struct ViewportKeys {
    overrides: Vec<(ViewportKey, key::Binding)>,
}
impl ViewportKeys {
    fn override_for(&self, action: ViewportKey) -> Option<&key::Binding> {
        self.overrides
            .iter()
            .find(|(key, _)| *key == action)
            .map(|(_, binding)| binding)
    }
    /// Replace one action's defaults, including its enabled state and help.
    pub fn set(&mut self, action: ViewportKey, binding: key::Binding) {
        if let Some((_, previous)) = self.overrides.iter_mut().find(|(key, _)| *key == action) {
            *previous = binding;
        } else {
            self.overrides.push((action, binding));
        }
    }
    /// Restore one action's defaults. Return whether an override existed.
    pub fn reset(&mut self, action: ViewportKey) -> bool {
        if let Some(index) = self.overrides.iter().position(|(key, _)| *key == action) {
            self.overrides.swap_remove(index);
            true
        } else {
            false
        }
    }
    pub fn chords(&self, action: ViewportKey) -> &[matea_core::Key] {
        self.override_for(action)
            .map_or_else(|| viewport_default_keys(action), |binding| &binding.keys)
    }
    pub fn help(&self, action: ViewportKey) -> Option<(&str, &str)> {
        self.override_for(action)
            .map_or_else(|| Some(viewport_default_help(action)), key::Binding::help)
    }
    pub fn matches(&self, action: ViewportKey, event: matea_core::Key) -> bool {
        self.override_for(action).map_or_else(
            || {
                viewport_default_keys(action)
                    .iter()
                    .any(|chord| event.matches_chord(*chord))
            },
            |binding| binding.matches(event),
        )
    }
    fn action_for(&self, event: matea_core::Key) -> Option<ViewportKey> {
        if !event.is_down() {
            return None;
        }
        if !self.overrides.is_empty() {
            return VIEWPORT_KEYS
                .into_iter()
                .find(|action| self.matches(*action, event));
        }
        // Select the one possible default group from the typed code. The
        // canonical chords still validate modifiers and provide public help.
        let action = match event.code {
            KeyCode::PageDown | KeyCode::Char(' ' | 'f') => ViewportKey::PageDown,
            KeyCode::PageUp | KeyCode::Char('b') => ViewportKey::PageUp,
            KeyCode::Char('u') => ViewportKey::HalfPageUp,
            KeyCode::Char('d') => ViewportKey::HalfPageDown,
            KeyCode::Down | KeyCode::Char('j') => ViewportKey::Down,
            KeyCode::Up | KeyCode::Char('k') => ViewportKey::Up,
            KeyCode::Left | KeyCode::Char('h') => ViewportKey::Left,
            KeyCode::Right | KeyCode::Char('l') => ViewportKey::Right,
            KeyCode::Home => ViewportKey::Top,
            KeyCode::End => ViewportKey::Bottom,
            _ => return None,
        };
        self.matches(action, event).then_some(action)
    }
}

/// Owned logical text with independent vertical and cell-based horizontal
/// positions. Parent layout sizes the default view; explicit width enables
/// horizontal scrolling without terminal-specific code.
///
/// ```
/// use matea_widgets::Viewport;
/// let mut notes = Viewport::new("Welcome to Matea.\nUnicode: 抹茶");
/// notes.set_size(12, 2);
/// notes.set_horizontal_step(3);
/// notes.scroll_right(3);
/// notes.ensure_visible(1, 9..11)?;
/// # Ok::<(), matea_widgets::ReversedViewportColumns>(())
/// ```
#[derive(Clone, Debug)]
pub struct Viewport {
    content: String,
    line_starts: Vec<usize>,
    offset: usize,
    width: Option<u16>,
    horizontal_offset: usize,
    horizontal_step: usize,
    longest_line_width: std::sync::OnceLock<usize>,
    soft_wrap: bool,
    wrapped_rows: std::sync::OnceLock<std::sync::Arc<WrappedRows>>,
    recycled_rows: RecycledRows,
    pub rows: u16,
    /// Include empty padding rows in the visible iterator, including an empty
    /// document. Explicit zero width/height still yields no padding rows.
    pub fill_height: bool,
    pub keys: ViewportKeys,
    pub mouse_wheel_enabled: bool,
    pub mouse_wheel_delta: usize,
}

#[derive(Debug, Default)]
struct WrappedRows {
    // Only UTF-8 byte starts are cached, not an allocation per displayed row.
    starts: Vec<usize>,
    // Rare oversized EGCs use a replacement row. Normal documents allocate no
    // replacement storage; sorted row numbers permit a binary lookup.
    replacements: Vec<usize>,
}

// A cache miss may recycle a previous allocation without making hot readers
// lock or clone an Arc. Default/unused wrapping never creates/locks this mutex.
// A clone gets a fresh spare slot: only published immutable geometry is shared.
#[derive(Debug, Default)]
struct RecycledRows(Option<std::sync::Mutex<Option<std::sync::Arc<WrappedRows>>>>);
impl Clone for RecycledRows {
    fn clone(&self) -> Self {
        Self::default()
    }
}
impl RecycledRows {
    fn retain(&mut self, rows: std::sync::Arc<WrappedRows>) {
        if let Some(slot) = self.0.as_mut() {
            *slot
                .get_mut()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(rows);
        } else {
            self.0 = Some(std::sync::Mutex::new(Some(rows)));
        }
    }
    fn take(&self) -> Option<std::sync::Arc<WrappedRows>> {
        self.0
            .as_ref()?
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }
}

/// An ensure-visible range must have its end at or after its start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReversedViewportColumns {
    pub start: usize,
    pub end: usize,
}
impl std::fmt::Display for ReversedViewportColumns {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "viewport column range {}..{} is reversed",
            self.start, self.end
        )
    }
}
impl std::error::Error for ReversedViewportColumns {}

impl Viewport {
    pub fn new(text: &str) -> Self {
        let mut viewport = Self {
            content: String::new(),
            line_starts: Vec::new(),
            offset: 0,
            width: None,
            horizontal_offset: 0,
            horizontal_step: 6,
            longest_line_width: std::sync::OnceLock::new(),
            soft_wrap: false,
            wrapped_rows: std::sync::OnceLock::new(),
            recycled_rows: RecycledRows::default(),
            rows: 6,
            fill_height: false,
            keys: ViewportKeys::default(),
            mouse_wheel_enabled: true,
            mouse_wheel_delta: 3,
        };
        viewport.set_content(text);
        viewport
    }
    /// Consume clean owned text without copying its allocation. CRLF and
    /// controls follow the same policy as [`Self::set_content`].
    pub fn from_string(text: String) -> Self {
        let mut viewport = Self::new("");
        viewport.set_content_owned(text);
        viewport
    }
    /// Replace content by copying borrowed text. External input cannot mutate
    /// either the owned content or its cached line index after this call.
    pub fn set_content(&mut self, text: &str) {
        self.install_content(normalize_viewport_text(text).into_owned());
    }
    /// Replace content with owned text, retaining its allocation when already
    /// normalized. Views created earlier keep their own immutable text.
    pub fn set_content_owned(&mut self, text: String) {
        let content = match normalize_viewport_text(&text) {
            std::borrow::Cow::Borrowed(_) => text,
            std::borrow::Cow::Owned(normalized) => normalized,
        };
        self.install_content(content);
    }
    /// Consume line inputs without retaining caller references or creating a
    /// temporary Vec of owned lines. Embedded newlines split into real lines.
    /// Normalize each input before inserting separators, so a bare CR at the
    /// end of an input cannot combine with a generated LF separator.
    pub fn set_content_lines<I, S>(&mut self, lines: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut content = String::new();
        for (index, line) in lines.into_iter().enumerate() {
            let normalized = normalize_viewport_text(line.as_ref());
            if index == 0 {
                // A whole document is commonly passed as a single element.
                // Move its normalized buffer instead of immediately copying it.
                content = normalized.into_owned();
            } else {
                content.push('\n');
                content.push_str(&normalized);
            }
        }
        self.install_content(content);
    }
    fn install_content(&mut self, content: String) {
        let count = if content.is_empty() {
            0
        } else {
            1 + content.bytes().filter(|byte| *byte == b'\n').count()
        };
        // Reserve before changing either field. After this point rebuilding
        // the index invokes no caller code and requires no further allocation.
        self.line_starts
            .reserve(count.saturating_sub(self.line_starts.len()));
        self.line_starts.clear();
        if count != 0 {
            self.line_starts.push(0);
            self.line_starts
                .extend(content.match_indices('\n').map(|(index, _)| index + 1));
        }
        self.content = content;
        self.longest_line_width.take();
        self.wrapped_rows.take();
        self.recycled_rows = RecycledRows::default();
        if self.offset != 0 {
            self.offset = self.offset.min(self.max_offset());
        }
        if self.horizontal_offset != 0 {
            self.horizontal_offset = self.horizontal_offset.min(self.max_horizontal_offset());
        }
    }
    pub fn content(&self) -> &str {
        &self.content
    }
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }
    pub fn line(&self, index: usize) -> Option<&str> {
        let start = *self.line_starts.get(index)?;
        let end = self
            .line_starts
            .get(index + 1)
            .map_or(self.content.len(), |next| next - 1);
        self.content.get(start..end)
    }
    pub fn lines(
        &self,
    ) -> impl ExactSizeIterator<Item = &str> + DoubleEndedIterator + std::iter::FusedIterator {
        (0..self.line_count()).map(|index| self.line(index).expect("owned line index is valid"))
    }
    pub fn height(&self) -> u16 {
        self.rows
    }
    /// Explicit content width in cells, or None to use the parent layout's
    /// available width with horizontal navigation disabled.
    pub fn width(&self) -> Option<u16> {
        self.width
    }
    /// Enable horizontal navigation and commit a position that fits the new
    /// width. Zero means an empty viewport, not an inferred parent width.
    pub fn set_width(&mut self, width: u16) {
        if self.width != Some(width)
            && let Some(previous) = self.wrapped_rows.take()
        {
            self.recycled_rows.retain(previous);
        }
        self.width = Some(width);
        if self.soft_wrap && self.offset != 0 {
            self.offset = self.offset.min(self.max_offset());
        }
        if self.horizontal_offset != 0 {
            self.horizontal_offset = self.horizontal_offset.min(self.max_horizontal_offset());
        }
    }
    /// Return sizing to the parent layout and reset horizontal position.
    pub fn clear_width(&mut self) {
        self.width = None;
        self.horizontal_offset = 0;
        self.wrapped_rows.take();
        self.recycled_rows = RecycledRows::default();
        if self.soft_wrap && self.offset != 0 {
            self.offset = self.offset.min(self.max_offset());
        }
    }
    pub fn set_size(&mut self, width: u16, height: u16) {
        self.set_width(width);
        self.set_height(height);
    }
    /// Wrap complete graphemes once an explicit width is known. Vertical
    /// positions then count display rows; logical line access stays unchanged.
    /// Enabling wrap resets horizontal position. Clear width suspends wrapping
    /// until the caller supplies a width again. Width zero displays no rows.
    pub fn set_soft_wrap(&mut self, enabled: bool) {
        if self.soft_wrap != enabled {
            self.soft_wrap = enabled;
            self.wrapped_rows.take();
            self.recycled_rows = RecycledRows::default();
            if enabled {
                self.horizontal_offset = 0;
            }
            if self.offset != 0 {
                self.offset = self.offset.min(self.max_offset());
            }
        }
    }
    pub fn soft_wrap(&self) -> bool {
        self.soft_wrap
    }
    /// Logical lines may occupy several display rows when wrapping is active.
    pub fn display_line_count(&self) -> usize {
        self.wrap_index()
            .map_or_else(|| self.line_count(), |rows| rows.starts.len())
    }
    fn wrap_index(&self) -> Option<&WrappedRows> {
        if !self.soft_wrap {
            return None;
        }
        let width = usize::from(self.width?);
        Some(self.wrapped_rows.get_or_init(|| {
            if let Some(mut previous) = self.recycled_rows.take() {
                // Exclusive ownership is checked by Arc, never assumed from
                // the application model. Shared clones retain the old index.
                if let Some(rows) = std::sync::Arc::get_mut(&mut previous) {
                    self.build_wrapped_rows(width, rows);
                    return previous;
                }
            }
            let mut rows = WrappedRows::default();
            self.build_wrapped_rows(width, &mut rows);
            std::sync::Arc::new(rows)
        }))
    }
    fn build_wrapped_rows(&self, width: usize, rows: &mut WrappedRows) {
        use unicode_segmentation::UnicodeSegmentation;
        use unicode_width::UnicodeWidthStr;
        rows.starts.clear();
        rows.replacements.clear();
        if width == 0 {
            return;
        }
        rows.starts.reserve(self.line_count());
        for (number, line) in self.lines().enumerate() {
            let start = self.line_starts[number];
            rows.starts.push(start);
            if line.is_ascii() {
                rows.starts.extend(
                    (width..line.len())
                        .step_by(width)
                        .map(|offset| start + offset),
                );
                continue;
            }
            let mut column = 0;
            for (index, grapheme) in line.grapheme_indices(true) {
                let cells = grapheme.width();
                if cells == 0 {
                    continue;
                }
                if column != 0 && cells > width - column {
                    rows.starts.push(start + index);
                    column = 0;
                }
                if cells > width {
                    rows.replacements.push(rows.starts.len() - 1);
                    column = width;
                } else {
                    column += cells;
                }
            }
        }
    }
    fn display_line(&self, index: usize) -> Option<std::borrow::Cow<'_, str>> {
        use std::borrow::Cow;
        let Some(rows) = self.wrap_index() else {
            return self.line(index).map(Cow::Borrowed);
        };
        let start = *rows.starts.get(index)?;
        if rows.replacements.binary_search(&index).is_ok() {
            return Some(Cow::Borrowed("�"));
        }
        let mut end = rows
            .starts
            .get(index + 1)
            .copied()
            .unwrap_or(self.content.len());
        if end > start && self.content.as_bytes()[end - 1] == b'\n' {
            end -= 1;
        }
        Some(Cow::Borrowed(&self.content[start..end]))
    }
    /// The compositor measures each grapheme independently. Cache that same
    /// policy lazily; vertical-only callers do not scan Unicode widths.
    pub fn longest_line_width(&self) -> usize {
        use unicode_segmentation::UnicodeSegmentation;
        use unicode_width::UnicodeWidthStr;
        *self.longest_line_width.get_or_init(|| {
            if self.content.is_ascii() {
                // Byte lengths are cell lengths for normalized ASCII. Reuse
                // the owned LF index rather than rechecking/slicing every row.
                let between = self
                    .line_starts
                    .windows(2)
                    .map(|pair| pair[1] - pair[0] - 1)
                    .max()
                    .unwrap_or(0);
                let tail = self.content.len() - self.line_starts.last().copied().unwrap_or(0);
                return between.max(tail);
            }
            let mut previous = None;
            let mut maximum = 0;
            for line in self.lines() {
                // Equal adjacent lines have equal width. One borrowed slice
                // avoids repeated Unicode work without a hash table or cache
                // allocation; varied lines still take the normal width path.
                if previous == Some(line) {
                    continue;
                }
                previous = Some(line);
                let cells = if line.is_ascii() {
                    line.len()
                } else {
                    line.graphemes(true).map(UnicodeWidthStr::width).sum()
                };
                maximum = maximum.max(cells);
            }
            maximum
        })
    }
    pub fn max_horizontal_offset(&self) -> usize {
        if self.soft_wrap {
            return 0;
        }
        self.width.map_or(0, |width| {
            self.longest_line_width().saturating_sub(usize::from(width))
        })
    }
    pub fn horizontal_offset(&self) -> usize {
        self.horizontal_offset
    }
    /// Set a clamped cell position. Parent-sized viewports cannot scroll
    /// horizontally. Return whether the position changed.
    pub fn set_horizontal_offset(&mut self, offset: usize) -> bool {
        if self.soft_wrap {
            return false;
        }
        let next = if offset == 0 {
            0
        } else {
            offset.min(self.max_horizontal_offset())
        };
        let changed = self.horizontal_offset != next;
        self.horizontal_offset = next;
        changed
    }
    /// A zero step disables the default left/right keys; explicit controls
    /// continue to accept a movement count.
    pub fn set_horizontal_step(&mut self, cells: usize) {
        self.horizontal_step = cells;
    }
    pub fn horizontal_step(&self) -> usize {
        self.horizontal_step
    }
    pub fn scroll_left(&mut self, cells: usize) -> bool {
        if cells == 0 {
            return false;
        }
        self.set_horizontal_offset(self.horizontal_offset.saturating_sub(cells))
    }
    pub fn scroll_right(&mut self, cells: usize) -> bool {
        if cells == 0 {
            return false;
        }
        self.set_horizontal_offset(self.horizontal_offset.saturating_add(cells))
    }
    /// Fraction through the horizontal range. Fitting/empty/parent-sized
    /// content returns 1, including explicit zero width with empty content.
    pub fn horizontal_scroll_fraction(&self) -> f64 {
        let maximum = self.max_horizontal_offset();
        if maximum == 0 {
            1.0
        } else {
            self.horizontal_offset as f64 / maximum as f64
        }
    }
    /// Reveal a logical line and cell range with one configured step of left
    /// context. Inverted ranges return an error without changing positions.
    /// Out-of-document positions are clamped; zero-height views stay empty.
    pub fn ensure_visible(
        &mut self,
        line: usize,
        columns: std::ops::Range<usize>,
    ) -> Result<bool, ReversedViewportColumns> {
        if columns.end < columns.start {
            return Err(ReversedViewportColumns {
                start: columns.start,
                end: columns.end,
            });
        }
        let before = (self.offset(), self.horizontal_offset);
        if let Some(rows) = self.wrap_index() {
            use unicode_segmentation::UnicodeSegmentation;
            use unicode_width::UnicodeWidthStr;
            let logical = line.min(self.line_count().saturating_sub(1));
            let mut byte = self.line_starts.get(logical).copied().unwrap_or(0);
            let mut column = 0usize;
            if let Some(text) = self.line(logical) {
                for grapheme in text.graphemes(true) {
                    if column.saturating_add(grapheme.width()) > columns.start {
                        break;
                    }
                    column += grapheme.width();
                    byte += grapheme.len();
                }
            }
            let target = rows
                .starts
                .partition_point(|start| *start <= byte)
                .saturating_sub(1);
            if target < self.offset()
                || target.saturating_sub(self.offset()) >= usize::from(self.rows)
            {
                self.set_offset(target);
            }
            return Ok(before != (self.offset(), self.horizontal_offset));
        }
        let column = if columns.end <= usize::from(self.width.unwrap_or(0)) {
            0
        } else {
            columns.start.saturating_sub(self.horizontal_step)
        };
        self.set_horizontal_offset(column);
        if line < self.offset() || line.saturating_sub(self.offset()) >= usize::from(self.rows) {
            self.set_offset(line);
        }
        Ok(before != (self.offset(), self.horizontal_offset))
    }
    /// Explicit resize also commits the clamped position. Direct `rows`
    /// assignment keeps the existing lazy observed-clamping behavior.
    pub fn set_height(&mut self, rows: u16) {
        self.rows = rows;
        self.offset = self.offset();
    }
    /// Observed position always fits the current content and height, including
    /// after direct assignment to the public `rows` field.
    pub fn offset(&self) -> usize {
        self.offset.min(self.max_offset())
    }
    pub fn max_offset(&self) -> usize {
        self.display_line_count()
            .saturating_sub(usize::from(self.rows))
    }
    /// Set a clamped position; return whether the observed position changed.
    pub fn set_offset(&mut self, offset: usize) -> bool {
        let before = self.offset();
        self.offset = offset.min(self.max_offset());
        self.offset != before
    }
    pub fn is_at_top(&self) -> bool {
        self.offset() == 0
    }
    pub fn is_at_bottom(&self) -> bool {
        self.offset() == self.max_offset()
    }
    /// Whether a direct `rows` assignment left a stored position past the new
    /// maximum. The observed position remains clamped; navigation can commit it.
    pub fn is_past_bottom(&self) -> bool {
        self.offset > self.max_offset()
    }
    /// Fraction through the scrollable range. Empty/fitting content returns 1;
    /// zero height uses the full line count as its scrollable range.
    pub fn scroll_fraction(&self) -> f64 {
        let maximum = self.max_offset();
        if maximum == 0 {
            1.0
        } else {
            self.offset() as f64 / maximum as f64
        }
    }
    pub fn visible_line_count(&self) -> usize {
        if self.fill_height {
            return if self.width == Some(0) {
                0
            } else {
                usize::from(self.rows)
            };
        }
        self.display_line_count()
            .saturating_sub(self.offset())
            .min(usize::from(self.rows))
    }
    pub fn scroll_down(&mut self, lines: usize) -> bool {
        if lines == 0 {
            return false;
        }
        self.set_offset(self.offset().saturating_add(lines))
    }
    pub fn scroll_up(&mut self, lines: usize) -> bool {
        if lines == 0 {
            return false;
        }
        self.set_offset(self.offset().saturating_sub(lines))
    }
    pub fn page_down(&mut self) -> bool {
        self.scroll_down(usize::from(self.rows))
    }
    pub fn page_up(&mut self) -> bool {
        self.scroll_up(usize::from(self.rows))
    }
    pub fn half_page_down(&mut self) -> bool {
        self.scroll_down(usize::from(self.rows / 2))
    }
    pub fn half_page_up(&mut self) -> bool {
        self.scroll_up(usize::from(self.rows / 2))
    }
    pub fn go_to_top(&mut self) -> bool {
        self.set_offset(0)
    }
    pub fn go_to_bottom(&mut self) -> bool {
        self.set_offset(self.max_offset())
    }
    /// Borrow the original text covered by visible rows. Soft breaks and
    /// fill-height padding are not inserted; use visible_lines for display rows.
    /// The first wrapped geometry lookup can allocate its shared row index.
    pub fn visible_text(&self) -> &str {
        if let Some(rows) = self.wrap_index() {
            let first = self.offset();
            let last = first
                .saturating_add(usize::from(self.rows))
                .min(rows.starts.len());
            if first == last {
                return "";
            }
            let start = rows.starts[first];
            let mut end = rows.starts.get(last).copied().unwrap_or(self.content.len());
            if end > start && self.content.as_bytes()[end - 1] == b'\n' {
                end -= 1;
            }
            return &self.content[start..end];
        }
        let start = self.offset();
        let end = start
            .saturating_add(usize::from(self.rows))
            .min(self.line_starts.len());
        if start == end {
            return "";
        }
        let end_byte = self
            .line_starts
            .get(end)
            .map_or(self.content.len(), |n| n - 1);
        &self.content[self.line_starts[start]..end_byte]
    }
    /// Lazily borrow visible logical lines, clipped to the explicit cell
    /// window. A partial wide grapheme at the left edge becomes spaces to
    /// preserve following cell positions. Only that case needs an owned Cow.
    pub fn visible_lines(
        &self,
    ) -> impl ExactSizeIterator<Item = std::borrow::Cow<'_, str>>
    + DoubleEndedIterator
    + std::iter::FusedIterator {
        let start = self.offset();
        let end = start + self.visible_line_count();
        (start..end).map(|index| {
            if self.soft_wrap && self.width.is_some() {
                return self
                    .display_line(index)
                    .unwrap_or(std::borrow::Cow::Borrowed(""));
            }
            let Some(line) = self.line(index) else {
                return std::borrow::Cow::Borrowed("");
            };
            self.width
                .map_or(std::borrow::Cow::Borrowed(line), |width| {
                    viewport_cell_window(line, self.horizontal_offset, usize::from(width))
                })
        })
    }
}

fn viewport_cell_window(line: &str, left: usize, width: usize) -> std::borrow::Cow<'_, str> {
    use std::borrow::Cow;
    use unicode_segmentation::UnicodeSegmentation;
    use unicode_width::UnicodeWidthStr;
    if width == 0 {
        return Cow::Borrowed("");
    }
    let right = left.saturating_add(width);
    if line.is_ascii() {
        return Cow::Borrowed(&line[left.min(line.len())..right.min(line.len())]);
    }
    let mut column = 0usize;
    let mut start = None;
    let mut end = 0;
    let mut padding = 0;
    for (index, grapheme) in line.grapheme_indices(true) {
        let next = column.saturating_add(grapheme.width());
        if next > left && next != column {
            if column < left {
                padding = (next - left).min(width);
            } else if next > right {
                break;
            } else {
                start.get_or_insert(index);
                end = index + grapheme.len();
            }
        }
        column = next;
        if column >= right {
            break;
        }
    }
    let slice = start.map_or("", |start| &line[start..end]);
    if padding == 0 {
        Cow::Borrowed(slice)
    } else {
        let mut text = String::with_capacity(padding + slice.len());
        text.extend(std::iter::repeat_n(' ', padding));
        text.push_str(slice);
        Cow::Owned(text)
    }
}

fn normalize_viewport_text(text: &str) -> std::borrow::Cow<'_, str> {
    use std::borrow::Cow;
    if text.contains("\r\n") {
        // CRLF removal cannot increase byte length. One reservation avoids
        // repeated String growth, while the shared sanitizer remains the only
        // authority for C0/C1/tab replacement.
        let mut normalized = String::with_capacity(text.len());
        for segment in text.split_inclusive("\r\n") {
            if let Some(line) = segment.strip_suffix("\r\n") {
                normalized.push_str(line);
                normalized.push('\n');
            } else {
                normalized.push_str(segment);
            }
        }
        match matea_render::sanitized(&normalized) {
            Cow::Borrowed(_) => Cow::Owned(normalized),
            Cow::Owned(clean) => Cow::Owned(clean),
        }
    } else {
        matea_render::sanitized(text)
    }
}
impl Component for Viewport {
    type Message = Action;
    fn event(&mut self, input: &Input, focused: bool) -> Response<Action> {
        if !focused {
            return Response::ignored();
        }
        let action = match input {
            Input::Key(event) => self.keys.action_for(*event),
            Input::Mouse(mouse) => {
                use matea_core::{Modifiers, MouseKind, ScrollDirection};
                if !self.mouse_wheel_enabled
                    || mouse.modifiers.bits() & !Modifiers::SHIFT.bits() != 0
                {
                    return Response::ignored();
                }
                let horizontal = mouse.modifiers.contains(Modifiers::SHIFT);
                match mouse.kind {
                    MouseKind::Scroll(ScrollDirection::Down)
                        if !horizontal && self.mouse_wheel_delta > 0 =>
                    {
                        self.scroll_down(self.mouse_wheel_delta);
                        return Response::consumed(Some(Action::Changed));
                    }
                    MouseKind::Scroll(ScrollDirection::Up)
                        if !horizontal && self.mouse_wheel_delta > 0 =>
                    {
                        self.scroll_up(self.mouse_wheel_delta);
                        return Response::consumed(Some(Action::Changed));
                    }
                    MouseKind::Scroll(ScrollDirection::Down) if horizontal => {
                        Some(ViewportKey::Right)
                    }
                    MouseKind::Scroll(ScrollDirection::Up) if horizontal => Some(ViewportKey::Left),
                    MouseKind::Scroll(ScrollDirection::Right) => Some(ViewportKey::Right),
                    MouseKind::Scroll(ScrollDirection::Left) => Some(ViewportKey::Left),
                    _ => None,
                }
            }
            _ => None,
        };
        match action {
            Some(ViewportKey::Left | ViewportKey::Right) => {
                if self.soft_wrap || self.width.is_none() || self.horizontal_step == 0 {
                    return Response::ignored();
                }
                if action == Some(ViewportKey::Left) {
                    self.scroll_left(self.horizontal_step);
                } else {
                    self.scroll_right(self.horizontal_step);
                }
            }
            Some(ViewportKey::PageDown) => {
                self.page_down();
            }
            Some(ViewportKey::PageUp) => {
                self.page_up();
            }
            Some(ViewportKey::HalfPageDown) => {
                self.half_page_down();
            }
            Some(ViewportKey::HalfPageUp) => {
                self.half_page_up();
            }
            Some(ViewportKey::Down) => {
                self.scroll_down(1);
            }
            Some(ViewportKey::Up) => {
                self.scroll_up(1);
            }
            Some(ViewportKey::Top) => {
                self.go_to_top();
            }
            Some(ViewportKey::Bottom) => {
                self.go_to_bottom();
            }
            None => return Response::ignored(),
        }
        Response::consumed(Some(Action::Changed))
    }
    fn view(&self, theme: &Theme, _: bool) -> View {
        let view = if self.horizontal_offset == 0
            && !(self.soft_wrap && self.width.is_some())
            && !self.fill_height
        {
            View::styled(self.visible_text(), theme.text)
        } else {
            let mut text = compact_str::CompactString::default();
            for (index, line) in self.visible_lines().enumerate() {
                if index != 0 {
                    text.push('\n');
                }
                text.push_str(&line);
            }
            View::styled(text, theme.text)
        };
        let view = if let Some(width) = self.width {
            view.width(width)
        } else {
            view
        };
        view.height(self.rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matea_render::Size;
    #[test]
    fn viewport_vertical_work_does_not_initialize_width_cache() {
        let mut viewport = Viewport::new("tea\n茶\ncoffee");
        viewport.set_height(1);
        viewport.go_to_bottom();
        viewport.visible_text();
        viewport
            .view(&Theme::plain(), true)
            .render(Size::new(8, 1))
            .unwrap();
        assert!(viewport.longest_line_width.get().is_none());
        viewport.set_width(2);
        assert!(viewport.longest_line_width.get().is_none());
        assert_eq!(viewport.longest_line_width(), 6);
        assert_eq!(viewport.longest_line_width.get(), Some(&6));
        viewport.set_content("new");
        assert!(viewport.longest_line_width.get().is_none());
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| assert_eq!(viewport.longest_line_width(), 3));
            }
        });
    }
}
