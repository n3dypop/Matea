#![forbid(unsafe_code)]
//! Stateful components sharing input, theme and focus contracts.
mod clocks;
mod cursor;
mod help;
pub mod key;
mod pagination;
mod paginator;
mod progress;
mod spinner;
mod textinput;
pub use clocks::{
    ClockId, ClockSchedule, ClockText, ClockTick, ClockUpdate, Countdown, Stopwatch, TickInterval,
    ZeroTickInterval,
};
pub use cursor::{CursorMode, VirtualCursor};
pub use help::{Help, HelpItem, HelpMap, HelpStyles};
use matea_core::{Component, Input, KeyCode, Response};
use matea_render::View;
use matea_style::Theme;
pub use pagination::Pagination;
pub use paginator::{PageDisplay, PageKeys, PageNumbers, Paginator};
pub use progress::{
    PROGRESS_EMPTY_BLOCK, PROGRESS_FULL_BLOCK, PROGRESS_HALF_BLOCK, Progress, ProgressCell,
    ProgressCells, ProgressError, ProgressFill, ProgressFrame, ProgressStyles,
};
pub use spinner::{EmptySpinnerFrames, Spinner, SpinnerAnimation, SpinnerPreset};
pub use textinput::{EchoMode, InputStyleState, InputStyles, TextEditKeys, TextInput};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Changed,
    Selected(usize),
    Submit,
}

#[derive(Clone, Debug)]
pub struct List {
    items: Vec<String>,
    search_terms: Vec<String>,
    visible: Vec<usize>,
    selected: usize,
    pub rows: u16,
    pub keys: key::NavigationKeys,
}
impl List {
    pub fn new(items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let items: Vec<_> = items.into_iter().map(Into::into).collect();
        let visible = (0..items.len()).collect();
        let search_terms = items.iter().map(|s| s.to_lowercase()).collect();
        Self {
            items,
            search_terms,
            visible,
            selected: 0,
            rows: 6,
            keys: key::NavigationKeys::default(),
        }
    }
    pub fn filter(&mut self, query: &str) {
        let q = query.to_lowercase();
        self.visible.clear();
        self.visible.extend(
            self.search_terms
                .iter()
                .enumerate()
                .filter(|(_, s)| s.contains(&q))
                .map(|(i, _)| i),
        );
        self.selected = 0;
    }
    /// Caller-defined matching for locale/normalization/fuzzy policies. The
    /// default filter is lowercase substring matching, not Unicode case folding.
    /// If the predicate panics, the accepted prefix remains visible and its first
    /// item is selected. An empty prefix has no selection.
    pub fn filter_with(&mut self, query: &str, mut matches: impl FnMut(&str, &str) -> bool) {
        self.selected = 0;
        self.visible.clear();
        self.visible.extend(
            self.items
                .iter()
                .enumerate()
                .filter(|(_, s)| matches(s, query))
                .map(|(i, _)| i),
        );
    }
    pub fn selected(&self) -> Option<usize> {
        self.visible.get(self.selected).copied()
    }
    fn start(&self) -> usize {
        self.selected
            .saturating_sub(usize::from(self.rows.max(1)) - 1)
    }
}
impl Component for List {
    type Message = Action;
    fn event(&mut self, input: &Input, focused: bool) -> Response<Action> {
        if !focused || self.visible.is_empty() {
            return Response::ignored();
        }
        let Input::Key(k) = input else {
            return Response::ignored();
        };
        match *k {
            k if self.keys.up.matches(k) => self.selected = self.selected.saturating_sub(1),
            k if self.keys.down.matches(k) => {
                self.selected = (self.selected + 1).min(self.visible.len() - 1)
            }
            k if self.keys.first.matches(k) => self.selected = 0,
            k if self.keys.last.matches(k) => self.selected = self.visible.len() - 1,
            k if self.keys.page_down.matches(k) => {
                self.selected = (self.selected + usize::from(self.rows)).min(self.visible.len() - 1)
            }
            k if self.keys.page_up.matches(k) => {
                self.selected = self.selected.saturating_sub(usize::from(self.rows))
            }
            k if self.keys.accept.matches(k) => {
                return Response::consumed(self.selected().map(Action::Selected));
            }
            _ => return Response::ignored(),
        }
        Response::consumed(Some(Action::Changed))
    }
    fn view(&self, theme: &Theme, focused: bool) -> View {
        if self.visible.is_empty() {
            return View::styled("(no matches)", theme.muted);
        }
        View::column(
            self.visible
                .iter()
                .enumerate()
                .skip(self.start())
                .take(usize::from(self.rows))
                .map(|(i, &index)| {
                    let selected = i == self.selected;
                    let style = if selected && focused {
                        theme.focus
                    } else {
                        theme.text
                    };
                    View::styled(
                        format!("{} {}", if selected { ">" } else { " " }, self.items[index]),
                        style,
                    )
                }),
        )
    }
}

#[derive(Clone, Debug)]
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    selection: List,
}
impl Table {
    pub fn new(headers: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        let selection = List::new((0..rows.len()).map(|n| n.to_string()));
        Self {
            headers,
            rows,
            selection,
        }
    }
    pub fn selected(&self) -> Option<usize> {
        self.selection.selected()
    }
}
impl Component for Table {
    type Message = Action;
    fn event(&mut self, input: &Input, focused: bool) -> Response<Action> {
        self.selection.event(input, focused)
    }
    fn view(&self, theme: &Theme, focused: bool) -> View {
        let mut lines = vec![View::row(
            self.headers.iter().map(|h| View::styled(h, theme.accent)),
        )];
        for (i, row) in self
            .rows
            .iter()
            .enumerate()
            .skip(self.selection.start())
            .take(usize::from(self.selection.rows))
        {
            let selected = self.selected() == Some(i);
            let style = if selected && focused {
                theme.focus
            } else {
                theme.text
            };
            lines.push(View::row(self.headers.iter().enumerate().map(
                |(col, _)| {
                    View::styled(
                        format!(
                            "{}{}",
                            if col == 0 && selected { "> " } else { "  " },
                            row.get(col).map_or("", String::as_str)
                        ),
                        style,
                    )
                },
            )));
        }
        View::column(lines)
    }
}

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
    pub rows: u16,
    pub keys: ViewportKeys,
    pub mouse_wheel_enabled: bool,
    pub mouse_wheel_delta: usize,
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
            rows: 6,
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
        self.offset = self.offset.min(self.max_offset());
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
        self.width = Some(width);
        if self.horizontal_offset != 0 {
            self.horizontal_offset = self.horizontal_offset.min(self.max_horizontal_offset());
        }
    }
    /// Return sizing to the parent layout and reset horizontal position.
    pub fn clear_width(&mut self) {
        self.width = None;
        self.horizontal_offset = 0;
    }
    pub fn set_size(&mut self, width: u16, height: u16) {
        self.set_width(width);
        self.set_height(height);
    }
    /// The compositor measures each grapheme independently. Cache that same
    /// policy lazily; vertical-only callers do not scan Unicode widths.
    pub fn longest_line_width(&self) -> usize {
        use unicode_segmentation::UnicodeSegmentation;
        use unicode_width::UnicodeWidthStr;
        *self.longest_line_width.get_or_init(|| {
            self.lines()
                .map(|line| {
                    if line.is_ascii() {
                        line.len()
                    } else {
                        line.graphemes(true).map(UnicodeWidthStr::width).sum()
                    }
                })
                .max()
                .unwrap_or(0)
        })
    }
    pub fn max_horizontal_offset(&self) -> usize {
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
        self.line_count().saturating_sub(usize::from(self.rows))
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
        self.line_count()
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
    /// Borrow the current vertical slice. Constructing an owned View is a
    /// separate operation; this getter performs no allocation.
    pub fn visible_text(&self) -> &str {
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
            let line = self.line(index).expect("visible owned line index is valid");
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
            Input::Key(event) => VIEWPORT_KEYS
                .into_iter()
                .find(|action| self.keys.matches(*action, *event)),
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
                if self.width.is_none() || self.horizontal_step == 0 {
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
        let view = if self.horizontal_offset == 0 {
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

pub type Validator = fn(&str) -> Result<(), String>;
#[derive(Clone)]
pub struct Field {
    pub label: String,
    pub input: TextInput,
    pub validator: Validator,
    error: Option<String>,
}
impl Field {
    pub fn new(label: impl Into<String>, validator: Validator) -> Self {
        Self {
            label: label.into(),
            input: TextInput::default(),
            validator,
            error: None,
        }
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref().or_else(|| self.input.error())
    }
}
pub fn required(value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err("A value is required".into())
    } else {
        Ok(())
    }
}
#[derive(Clone)]
pub struct Form {
    fields: Vec<Field>,
    focus: usize,
    submitted: bool,
}
impl Form {
    pub fn new(fields: Vec<Field>) -> Self {
        Self {
            fields,
            focus: 0,
            submitted: false,
        }
    }
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }
    pub fn submitted(&self) -> bool {
        self.submitted
    }
    pub fn focus_index(&self) -> usize {
        self.focus
    }
    fn validate(&mut self) -> bool {
        for f in &mut self.fields {
            f.input.validate();
            f.error = f
                .input
                .error()
                .map(str::to_owned)
                .or_else(|| (f.validator)(f.input.value()).err());
        }
        if let Some(i) = self.fields.iter().position(|f| f.error.is_some()) {
            self.focus = i;
            false
        } else {
            self.submitted = true;
            true
        }
    }
}
impl Component for Form {
    type Message = Action;
    fn event(&mut self, input: &Input, focused: bool) -> Response<Action> {
        if !focused || self.fields.is_empty() {
            return Response::ignored();
        }
        if let Some(k) = input.key_down().filter(|k| k.is_plain()) {
            if k.focus_direction().is_some() {
                let response = self.fields[self.focus].input.event(input, true);
                if response.consumed {
                    self.submitted = false;
                    if response.message == Some(Action::Changed) {
                        self.fields[self.focus].error = None;
                    }
                    return response;
                }
            }
            match k.code {
                KeyCode::Tab if k.focus_direction() == Some(1) => {
                    self.submitted = false;
                    self.focus = (self.focus + 1) % self.fields.len();
                    return Response::consumed(None);
                }
                KeyCode::BackTab | KeyCode::Tab => {
                    self.submitted = false;
                    self.focus = (self.focus + self.fields.len() - 1) % self.fields.len();
                    return Response::consumed(None);
                }
                KeyCode::Enter => {
                    self.submitted = false;
                    if self.focus + 1 == self.fields.len() {
                        return Response::consumed(self.validate().then_some(Action::Submit));
                    }
                    let f = &mut self.fields[self.focus];
                    f.input.validate();
                    f.error = f
                        .input
                        .error()
                        .map(str::to_owned)
                        .or_else(|| (f.validator)(f.input.value()).err());
                    if f.error.is_none() {
                        self.focus += 1;
                    }
                    return Response::consumed(None);
                }
                _ => {}
            }
        }
        let response = self.fields[self.focus].input.event(input, true);
        if response.consumed {
            self.submitted = false;
        }
        if response.message == Some(Action::Changed) {
            self.fields[self.focus].error = None;
        }
        response
    }
    fn view(&self, theme: &Theme, focused: bool) -> View {
        View::column(self.fields.iter().enumerate().flat_map(|(i, f)| {
            let active = focused && i == self.focus;
            let mut rows = vec![
                View::styled(
                    format!("{} {}", if active { ">" } else { " " }, f.label),
                    if active { theme.accent } else { theme.muted },
                ),
                f.input.view(theme, active),
            ];
            if let Some(e) = f.error() {
                rows.push(View::styled(format!("Error: {e}"), theme.error));
            }
            rows
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matea_core::Key;
    use matea_render::Size;
    fn key(k: KeyCode) -> Input {
        Input::Key(Key::new(k))
    }
    #[test]
    fn releases_command_modifiers_and_nontext_events_do_not_edit_or_submit() {
        use matea_core::{KeyKind, Modifiers};
        let mut text = TextInput::new("tea");
        let mut list = List::new(["tea", "coffee"]);
        let mut viewport = Viewport::new("one\ntwo\nthree");
        viewport.rows = 1;
        let mut form = Form::new(vec![Field::new("Name", required)]);
        form.event(&Input::Paste("tea".into()), true);
        form.event(&key(KeyCode::Enter), true);
        assert!(form.submitted());
        for event in [
            Input::Key(Key::new(KeyCode::Char('x')).with_kind(KeyKind::Release)),
            Input::Key(Key::new(KeyCode::Enter).with_kind(KeyKind::Release)),
            Input::Key(Key::new(KeyCode::Tab).with_modifiers(Modifiers::CONTROL)),
            Input::Key(Key::new(KeyCode::Char('q')).with_modifiers(Modifiers::SUPER)),
            Input::Key(Key::new(KeyCode::Char('\0'))),
            Input::Focus(false),
        ] {
            assert!(!text.event(&event, true).consumed);
            assert!(!list.event(&event, true).consumed);
            assert!(!viewport.event(&event, true).consumed);
            assert!(!form.event(&event, true).consumed);
            assert_eq!(text.value(), "tea");
            assert_eq!(list.selected(), Some(0));
            assert_eq!(viewport.offset(), 0);
            assert!(form.submitted());
        }
        let repeat = Input::Key(Key::new(KeyCode::Down).with_kind(KeyKind::Repeat));
        list.event(&repeat, true);
        assert_eq!(list.selected(), Some(1));
        viewport.event(&repeat, true);
        assert_eq!(viewport.offset(), 1);
        text.event(
            &Input::Key(Key::new(KeyCode::Char('!')).with_kind(KeyKind::Repeat)),
            true,
        );
        assert_eq!(text.value(), "tea!");
    }
    #[test]
    fn custom_filter_and_keys_share_enabled_help() {
        let mut list = List::new(["café", "tea", "CAFÉ"]);
        list.filter_with("cafe", |s, q| {
            s.to_lowercase().replace('é', "e").contains(q)
        });
        list.keys.down = key::Binding::new([Key::new(KeyCode::Char('n'))], "n", "next");
        assert!(!list.event(&key(KeyCode::Char('j')), true).consumed);
        assert!(list.event(&key(KeyCode::Char('n')), true).consumed);
        assert_eq!(list.selected(), Some(2));
        assert_eq!(list.keys.down.help(), Some(("n", "next")));
        list.keys.down.enabled = false;
        assert!(list.keys.down.help().is_none());
        assert!(!list.event(&key(KeyCode::Char('n')), true).consumed);
        list.filter("TEA");
        assert_eq!(list.selected(), Some(1));
    }
    #[test]
    fn input_graphemes_focus_paste_and_scroll() {
        let mut t = TextInput::new("e\u{301}茶👩‍💻");
        assert!(!t.event(&key(KeyCode::Backspace), false).consumed);
        t.event(&key(KeyCode::Backspace), true);
        assert_eq!(t.value(), "e\u{301}茶");
        t.event(&key(KeyCode::Left), true);
        t.event(&key(KeyCode::Backspace), true);
        assert_eq!(t.value(), "茶");
        t.event(&Input::Paste("\x1b\nX".into()), true);
        assert!(!t.value().contains('\x1b'));
        t.set_value("abcdefghijklmnop");
        t.width = 5;
        assert_eq!(
            t.view(&Theme::plain(), true)
                .render(Size::new(5, 1))
                .unwrap()
                .plain(),
            "mnop"
        );
    }
    #[test]
    fn list_filter_selection_empty_and_table() {
        let mut l = List::new(["Tea", "Coffee", "Matcha"]);
        l.filter("a");
        l.event(&key(KeyCode::Down), true);
        assert_eq!(l.selected(), Some(2));
        assert_eq!(
            l.event(&key(KeyCode::Enter), true).message,
            Some(Action::Selected(2))
        );
        l.filter("zzz");
        assert_eq!(l.selected(), None);
        assert!(!l.event(&key(KeyCode::Down), true).consumed);
        let mut table = Table::new(vec!["Name".into()], vec![]);
        assert!(!table.event(&key(KeyCode::Down), true).consumed);
    }
    #[test]
    fn form_focus_validation_and_submit() {
        let mut f = Form::new(vec![
            Field::new("Name", required),
            Field::new("City", required),
        ]);
        f.event(&key(KeyCode::Enter), true);
        assert!(f.fields()[0].error().is_some());
        assert_eq!(f.focus_index(), 0);
        f.event(&Input::Paste("Matea".into()), true);
        f.event(&key(KeyCode::Tab), true);
        f.event(&key(KeyCode::Enter), true);
        assert_eq!(f.focus_index(), 1);
        assert!(!f.submitted());
        f.event(&Input::Paste("Kyoto".into()), true);
        assert_eq!(
            f.event(&key(KeyCode::Enter), true).message,
            Some(Action::Submit)
        );
        assert!(f.submitted());
        f.event(&key(KeyCode::BackTab), true);
        assert_eq!(f.focus_index(), 0);
    }
    #[test]
    fn viewport_bounds_and_reduced_motion() {
        let mut v = Viewport::new("1\n2\n3\n4");
        v.rows = 2;
        v.event(&key(KeyCode::End), true);
        assert_eq!(v.offset(), 2);
        v.rows = 10;
        assert!(
            v.view(&Theme::plain(), false)
                .render(Size::new(5, 10))
                .unwrap()
                .plain()
                .starts_with("1\n2")
        );
        let theme = Theme::plain();
        let mut s = Spinner::default();
        s.tick(&theme);
        assert_eq!(s.frame_index(), 0);
    }
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
