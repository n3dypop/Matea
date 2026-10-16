//! Grapheme editing and cell-based display. No terminal state or clipboard I/O.
use crate::{Action, key::Binding};
use matea_core::{Component, Input, Key, KeyCode, Modifiers, Response};
use matea_render::{Cursor, View};
use matea_style::{Style, Theme};
use std::{borrow::Cow, fmt, rc::Rc};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EchoMode {
    #[default]
    Normal,
    Password(char),
    Hidden,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputStyleState {
    pub text: Style,
    pub placeholder: Style,
    pub suggestion: Style,
    pub prompt: Style,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputStyles {
    pub focused: InputStyleState,
    pub blurred: InputStyleState,
    pub cursor: Style,
}
impl InputStyles {
    pub fn from_theme(theme: &Theme) -> Self {
        let focused = InputStyleState {
            text: theme.text,
            placeholder: theme.muted,
            suggestion: theme.muted,
            prompt: theme.accent,
        };
        Self {
            focused,
            blurred: InputStyleState {
                text: theme.muted,
                ..focused
            },
            cursor: theme.focus,
        }
    }
}
#[derive(Clone, Debug)]
pub struct TextEditKeys {
    pub forward: Binding,
    pub backward: Binding,
    pub word_forward: Binding,
    pub word_backward: Binding,
    pub delete_word_backward: Binding,
    pub delete_word_forward: Binding,
    pub delete_after: Binding,
    pub delete_before: Binding,
    pub backspace: Binding,
    pub delete: Binding,
    pub start: Binding,
    pub end: Binding,
    pub accept_suggestion: Binding,
    pub next_suggestion: Binding,
    pub previous_suggestion: Binding,
}
impl Default for TextEditKeys {
    fn default() -> Self {
        let plain = Key::new;
        let ctrl = |c| Key::new(KeyCode::Char(c)).with_modifiers(Modifiers::CONTROL);
        let alt = |code| Key::new(code).with_modifiers(Modifiers::ALT);
        let control = |code| Key::new(code).with_modifiers(Modifiers::CONTROL);
        let bind = |keys: Vec<Key>, label, help| Binding::new(keys, label, help);
        Self {
            forward: bind(vec![plain(KeyCode::Right), ctrl('f')], "right", "forward"),
            backward: bind(vec![plain(KeyCode::Left), ctrl('b')], "left", "backward"),
            word_forward: bind(
                vec![
                    alt(KeyCode::Right),
                    control(KeyCode::Right),
                    alt(KeyCode::Char('f')),
                ],
                "alt+right",
                "next word",
            ),
            word_backward: bind(
                vec![
                    alt(KeyCode::Left),
                    control(KeyCode::Left),
                    alt(KeyCode::Char('b')),
                ],
                "alt+left",
                "previous word",
            ),
            delete_word_backward: bind(
                vec![
                    alt(KeyCode::Backspace),
                    ctrl('w'),
                    control(KeyCode::Backspace),
                ],
                "ctrl+w",
                "delete previous word",
            ),
            delete_word_forward: bind(
                vec![
                    alt(KeyCode::Delete),
                    alt(KeyCode::Char('d')),
                    control(KeyCode::Delete),
                ],
                "alt+d",
                "delete next word",
            ),
            delete_after: bind(vec![ctrl('k')], "ctrl+k", "delete to end"),
            delete_before: bind(vec![ctrl('u')], "ctrl+u", "delete to start"),
            backspace: bind(
                vec![plain(KeyCode::Backspace), ctrl('h')],
                "backspace",
                "delete backward",
            ),
            delete: bind(
                vec![plain(KeyCode::Delete), ctrl('d')],
                "delete",
                "delete forward",
            ),
            start: bind(vec![plain(KeyCode::Home), ctrl('a')], "home", "start"),
            end: bind(vec![plain(KeyCode::End), ctrl('e')], "end", "end"),
            accept_suggestion: bind(vec![plain(KeyCode::Tab)], "tab", "complete"),
            next_suggestion: bind(
                vec![plain(KeyCode::Down), ctrl('n')],
                "down",
                "next suggestion",
            ),
            previous_suggestion: bind(
                vec![plain(KeyCode::Up), ctrl('p')],
                "up",
                "previous suggestion",
            ),
        }
    }
}
type Validator = Rc<dyn Fn(&str) -> Result<(), String>>;
#[derive(Clone, Debug)]
struct Suggestions {
    values: Rc<Vec<String>>,
    search: Vec<String>,
    matched: Vec<usize>,
    scratch: Vec<usize>,
    selected: usize,
    query: String,
}
#[derive(Clone)]
pub struct TextInput {
    value: String,
    cursor: usize,
    count: usize,
    ascii: bool,
    pub width: u16,
    /// Extended graphemes, not bytes or scalar values. usize::MAX is unlimited.
    pub limit: usize,
    /// Retained UTF-8 bytes, independently of grapheme count. This bounds even
    /// a single cluster containing many combining marks. Default: 64 KiB.
    pub byte_limit: usize,
    pub prompt: String,
    pub placeholder: String,
    pub echo: EchoMode,
    pub keys: TextEditKeys,
    pub show_suggestions: bool,
    virtual_cursor: bool,
    styles: Option<InputStyles>,
    validator: Option<Validator>,
    error: Option<String>,
    suggestions: Option<Suggestions>,
}
impl fmt::Debug for TextInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextInput")
            .field(
                "value",
                &if self.echo == EchoMode::Normal {
                    self.value.as_str()
                } else {
                    "<redacted>"
                },
            )
            .field("cursor", &self.cursor)
            .field("width", &self.width)
            .field("limit", &self.limit)
            .field("echo", &self.echo)
            .finish_non_exhaustive()
    }
}
impl Default for TextInput {
    fn default() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            count: 0,
            ascii: true,
            width: 24,
            limit: 1024,
            byte_limit: 64 * 1024,
            prompt: String::new(),
            placeholder: String::new(),
            echo: EchoMode::Normal,
            keys: TextEditKeys::default(),
            show_suggestions: false,
            virtual_cursor: true,
            styles: None,
            validator: None,
            error: None,
            suggestions: None,
        }
    }
}
impl TextInput {
    /// Virtual cursors are styled cells. Native cursors become local view hints;
    /// the compositor handles layout offsets and the terminal owns visibility.
    pub fn virtual_cursor(&self) -> bool {
        self.virtual_cursor
    }
    pub fn set_virtual_cursor(&mut self, enabled: bool) {
        self.virtual_cursor = enabled;
    }
    pub fn new(value: impl Into<String>) -> Self {
        let mut result = Self::default();
        result.set_value(value);
        result
    }
    pub fn value(&self) -> &str {
        &self.value
    }
    pub fn grapheme_len(&self) -> usize {
        self.count
    }
    /// Cursor position in extended graphemes. Internally only byte boundaries
    /// known to be whole grapheme boundaries are stored.
    pub fn position(&self) -> usize {
        if self.ascii {
            self.cursor
        } else {
            self.value[..self.cursor].graphemes(true).count()
        }
    }
    pub fn set_cursor(&mut self, position: usize) {
        self.cursor = if self.ascii {
            position.min(self.value.len())
        } else {
            self.value
                .grapheme_indices(true)
                .nth(position)
                .map_or(self.value.len(), |(i, _)| i)
        };
    }
    pub fn cursor_start(&mut self) {
        self.cursor = 0;
    }
    pub fn cursor_end(&mut self) {
        self.cursor = self.value.len();
    }
    pub fn reserve(&mut self, additional_bytes: usize) {
        self.value.reserve(additional_bytes);
    }
    /// None removes this widget's width constraint; parent clipping still applies.
    /// Some(0) intentionally hides the field, unlike the reference's integer API.
    pub fn set_width(&mut self, width: Option<u16>) {
        self.width = width.unwrap_or(u16::MAX);
    }
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
        self.truncate_to_limits();
    }
    pub fn set_byte_limit(&mut self, bytes: usize) {
        self.byte_limit = bytes;
        self.truncate_to_limits();
    }
    fn truncate_to_limits(&mut self) {
        if self.count > self.limit || self.value.len() > self.byte_limit {
            let mut bytes = 0;
            let end = self
                .value
                .grapheme_indices(true)
                .enumerate()
                .find_map(|(n, (i, g))| {
                    if n == self.limit || g.len() > self.byte_limit.saturating_sub(bytes) {
                        Some(i)
                    } else {
                        bytes += g.len();
                        None
                    }
                })
                .unwrap_or(self.value.len());
            self.value.truncate(end);
            self.cursor = self.cursor.min(end);
            self.refresh();
            self.validate();
        }
    }
    pub fn set_value(&mut self, value: impl Into<String>) {
        let incoming = value.into();
        let clean = single_line(&incoming);
        self.value.clear();
        let mut bytes = 0;
        self.value
            .extend(clean.graphemes(true).take(self.limit).take_while(|g| {
                if g.len() > self.byte_limit.saturating_sub(bytes) {
                    return false;
                }
                bytes += g.len();
                true
            }));
        self.cursor = self.value.len();
        self.refresh();
        self.validate();
    }
    pub fn reset(&mut self) {
        self.value.clear();
        self.cursor = 0;
        self.refresh();
        self.validate();
    }
    pub fn with_validator(
        mut self,
        validator: impl Fn(&str) -> Result<(), String> + 'static,
    ) -> Self {
        self.validator = Some(Rc::new(validator));
        self.validate();
        self
    }
    pub fn clear_validator(&mut self) {
        self.validator = None;
        self.error = None;
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub fn set_styles(&mut self, styles: InputStyles) {
        self.styles = Some(styles);
    }
    pub fn styles(&self, theme: &Theme) -> InputStyles {
        self.styles
            .unwrap_or_else(|| InputStyles::from_theme(theme))
    }
    pub fn set_suggestions(&mut self, values: impl IntoIterator<Item = impl Into<String>>) {
        let values: Vec<String> = values
            .into_iter()
            .map(|s| single_line(&s.into()).into_owned())
            .collect();
        let search = values.iter().map(|s| s.to_lowercase()).collect();
        let n = values.len();
        self.suggestions = Some(Suggestions {
            values: Rc::new(values),
            search,
            matched: Vec::with_capacity(n),
            scratch: Vec::with_capacity(n),
            selected: 0,
            query: String::new(),
        });
        self.update_suggestions();
    }
    pub fn available_suggestions(&self) -> &[String] {
        self.suggestions
            .as_ref()
            .map_or(&[], |s| s.values.as_slice())
    }
    pub fn matched_suggestions(&self) -> impl Iterator<Item = &str> {
        self.suggestions
            .iter()
            .flat_map(|s| s.matched.iter().map(|&i| s.values[i].as_str()))
    }
    pub fn current_suggestion_index(&self) -> Option<usize> {
        self.suggestions
            .as_ref()
            .filter(|s| !s.matched.is_empty())
            .map(|s| s.selected)
    }
    pub fn current_suggestion(&self) -> Option<&str> {
        let s = self.suggestions.as_ref()?;
        s.matched.get(s.selected).map(|&i| s.values[i].as_str())
    }
    fn update_suggestions(&mut self) {
        let Some(s) = &mut self.suggestions else {
            return;
        };
        s.scratch.clear();
        s.query.clear();
        if !self.value.is_empty() {
            if self.ascii {
                s.query.push_str(&self.value);
                s.query.make_ascii_lowercase();
            } else {
                // Keep str's contextual Unicode lowercasing (e.g. final sigma),
                // rather than replacing it with scalar-only case conversion.
                s.query = self.value.to_lowercase();
            }
            s.scratch.extend(
                s.search
                    .iter()
                    .enumerate()
                    .filter(|(_, text)| text.starts_with(&s.query))
                    .map(|(i, _)| i),
            );
        }
        if s.scratch != s.matched {
            s.selected = 0;
        }
        std::mem::swap(&mut s.scratch, &mut s.matched);
    }
    pub(crate) fn validate(&mut self) {
        self.error = self.validator.as_ref().and_then(|f| f(&self.value).err());
    }
    fn refresh(&mut self) {
        self.ascii = self.value.is_ascii();
        self.count = if self.ascii {
            self.value.len()
        } else {
            self.value.graphemes(true).count()
        };
        self.cursor = self.cursor.min(self.value.len());
        if !self.ascii {
            self.cursor = self
                .value
                .grapheme_indices(true)
                .map(|(i, _)| i)
                .find(|&i| i >= self.cursor)
                .unwrap_or(self.value.len());
        }
        self.update_suggestions();
    }
    fn previous(&self) -> usize {
        if self.ascii {
            self.cursor.saturating_sub(1)
        } else {
            self.value[..self.cursor]
                .grapheme_indices(true)
                .next_back()
                .map_or(0, |(i, _)| i)
        }
    }
    fn next(&self) -> usize {
        if self.ascii {
            (self.cursor + 1).min(self.value.len())
        } else {
            self.cursor
                + self.value[self.cursor..]
                    .graphemes(true)
                    .next()
                    .map_or(0, str::len)
        }
    }
    fn word_backward(&self) -> usize {
        if self.echo != EchoMode::Normal {
            return 0;
        }
        let mut start = self.cursor;
        let mut word = false;
        for (i, g) in self.value[..self.cursor].grapheme_indices(true).rev() {
            let space = g.chars().next().is_some_and(char::is_whitespace);
            if word && space {
                break;
            }
            word |= !space;
            start = i;
        }
        start
    }
    fn word_forward(&self) -> usize {
        if self.echo != EchoMode::Normal {
            return self.value.len();
        }
        let mut word = false;
        for (i, g) in self.value[self.cursor..].grapheme_indices(true) {
            let space = g.chars().next().is_some_and(char::is_whitespace);
            if word && space {
                return self.cursor + i;
            }
            word |= !space;
        }
        self.value.len()
    }
    fn erase(&mut self, start: usize, end: usize) -> bool {
        if start == end {
            return false;
        }
        self.value.replace_range(start..end, "");
        self.cursor = start;
        if self.ascii {
            self.count = self.value.len();
            self.update_suggestions();
        } else {
            self.refresh();
        }
        true
    }
    fn insert(&mut self, text: &str) -> bool {
        let clean = single_line(text);
        if self.ascii && clean.is_ascii() {
            let count = clean
                .len()
                .min(self.limit.saturating_sub(self.count))
                .min(self.byte_limit.saturating_sub(self.value.len()));
            if count == 0 {
                return false;
            }
            self.value.insert_str(self.cursor, &clean[..count]);
            self.cursor += count;
            self.count += count;
            self.update_suggestions();
            return true;
        }
        let mut bytes = 0;
        let available = self.byte_limit.saturating_sub(self.value.len());
        let byte_end = clean
            .grapheme_indices(true)
            .find_map(|(i, g)| {
                if g.len() > available.saturating_sub(bytes) {
                    Some(i)
                } else {
                    bytes += g.len();
                    None
                }
            })
            .unwrap_or(clean.len());
        let clean = &clean[..byte_end];
        // Include two context candidates so a combining/ZWJ insertion can fit
        // even at the limit. Recount actual joined clusters and roll back only
        // excess inserted graphemes; existing suffix text is never truncated.
        let candidates = self.limit.saturating_sub(self.count).saturating_add(2);
        let mut end = clean
            .grapheme_indices(true)
            .nth(candidates)
            .map_or(clean.len(), |(i, _)| i);
        if end == 0 {
            return false;
        }
        let anchor = self.cursor;
        self.value.insert_str(anchor, &clean[..end]);
        loop {
            let count = self.value.graphemes(true).count();
            if count <= self.limit || end == 0 {
                break;
            }
            let previous = clean[..end]
                .grapheme_indices(true)
                .next_back()
                .map_or(0, |(i, _)| i);
            self.value
                .replace_range(anchor + previous..anchor + end, "");
            end = previous;
        }
        self.cursor = anchor + end;
        self.refresh();
        end > 0
    }
    fn accept_suggestion(&mut self) -> bool {
        let Some(s) = &self.suggestions else {
            return false;
        };
        let Some(&i) = s.matched.get(s.selected) else {
            return false;
        };
        let values = Rc::clone(&s.values);
        let suffix = values[i].len() - self.completion_suffix(&values[i]).len();
        let previous = self.cursor;
        self.cursor_end();
        let changed = self.insert(&values[i][suffix..]);
        if !changed {
            self.cursor = previous;
        }
        changed
    }
    fn completion_suffix<'a>(&self, suggestion: &'a str) -> &'a str {
        // Lowercasing can expand a scalar (İ -> i + combining dot). Map the
        // matched lowercase prefix back to an original UTF-8 scalar boundary,
        // rather than slicing the suggestion by the typed string's byte length.
        let target = self.value.chars().flat_map(char::to_lowercase).count();
        if target == 0 {
            return suggestion;
        }
        let mut count = 0;
        for (i, c) in suggestion.char_indices() {
            count += c.to_lowercase().count();
            if count >= target {
                return &suggestion[i + c.len_utf8()..];
            }
        }
        ""
    }
    fn cycle_suggestion(&mut self, forward: bool) -> bool {
        let Some(s) = &mut self.suggestions else {
            return false;
        };
        if s.matched.is_empty() {
            return false;
        }
        s.selected = if forward {
            (s.selected + 1) % s.matched.len()
        } else {
            (s.selected + s.matched.len() - 1) % s.matched.len()
        };
        true
    }
    fn mask(&self) -> char {
        match self.echo {
            EchoMode::Password(c) if !c.is_control() && c.width().is_some_and(|n| n > 0) => c,
            _ => '*',
        }
    }
}
fn single_line(text: &str) -> Cow<'_, str> {
    if !text.chars().any(char::is_control) {
        return Cow::Borrowed(text);
    }
    Cow::Owned(
        text.chars()
            .filter_map(|c| match c {
                '\r' | '\n' | '\t' => Some(' '),
                c if c.is_control() => None,
                c => Some(c),
            })
            .collect(),
    )
}
impl Component for TextInput {
    type Message = Action;
    fn event(&mut self, input: &Input, focused: bool) -> Response<Action> {
        if !focused {
            return Response::ignored();
        }
        let changed = match input {
            Input::Paste(text) => self.insert(text),
            Input::Key(key) if key.is_down() => {
                let key = *key;
                if self.show_suggestions
                    && self.keys.accept_suggestion.matches(key)
                    && self.current_suggestion().is_some()
                    && self.accept_suggestion()
                {
                    self.validate();
                    return Response::consumed(Some(Action::Changed));
                }
                if self.keys.delete_word_backward.matches(key) {
                    self.erase(self.word_backward(), self.cursor)
                } else if self.keys.backspace.matches(key) {
                    self.erase(self.previous(), self.cursor)
                } else if self.keys.word_backward.matches(key) {
                    self.cursor = self.word_backward();
                    false
                } else if self.keys.backward.matches(key) {
                    self.cursor = self.previous();
                    false
                } else if self.keys.word_forward.matches(key) {
                    self.cursor = self.word_forward();
                    false
                } else if self.keys.forward.matches(key) {
                    self.cursor = self.next();
                    false
                } else if self.keys.start.matches(key) {
                    self.cursor_start();
                    false
                } else if self.keys.delete.matches(key) {
                    self.erase(self.cursor, self.next())
                } else if self.keys.end.matches(key) {
                    self.cursor_end();
                    false
                } else if self.keys.delete_after.matches(key) {
                    self.erase(self.cursor, self.value.len())
                } else if self.keys.delete_before.matches(key) {
                    self.erase(0, self.cursor)
                } else if self.keys.delete_word_forward.matches(key) {
                    self.erase(self.cursor, self.word_forward())
                } else if self.show_suggestions
                    && ((self.keys.next_suggestion.matches(key) && self.cycle_suggestion(true))
                        || (self.keys.previous_suggestion.matches(key)
                            && self.cycle_suggestion(false)))
                {
                    false
                } else if key.is_plain() {
                    match key.code {
                        KeyCode::Char(c) if !c.is_control() => {
                            self.insert(c.encode_utf8(&mut [0; 4]))
                        }
                        KeyCode::Enter => return Response::consumed(Some(Action::Submit)),
                        _ => return Response::ignored(),
                    }
                } else {
                    return Response::ignored();
                }
            }
            _ => return Response::ignored(),
        };
        if changed {
            self.validate();
        }
        Response::consumed(changed.then_some(Action::Changed))
    }
    fn view(&self, theme: &Theme, focused: bool) -> View {
        let styles = self.styles(theme);
        let state = if focused {
            styles.focused
        } else {
            styles.blurred
        };
        let prompt = single_line(&self.prompt);
        let prompt_width = prompt.width().min(u16::MAX as usize) as u16;
        let mut caret_x = 0;
        let field = if self.echo == EchoMode::Hidden {
            View::styled(
                " ",
                if focused && self.virtual_cursor {
                    styles.cursor
                } else {
                    state.text
                },
            )
            .width(self.width)
        } else {
            let placeholder = self.value.is_empty();
            let source = if placeholder {
                single_line(&self.placeholder)
            } else if self.echo == EchoMode::Normal {
                Cow::Borrowed(self.value.as_str())
            } else {
                Cow::Owned(
                    self.mask()
                        .to_string()
                        .repeat(self.count.min(usize::from(self.width) + 1)),
                )
            };
            let cursor = if placeholder {
                0
            } else if self.echo == EchoMode::Normal {
                self.cursor
            } else {
                self.position()
                    .min(self.count.min(usize::from(self.width) + 1))
                    * self.mask().len_utf8()
            };
            let text_style = if placeholder {
                state.placeholder
            } else {
                state.text
            };
            let before = &source[..cursor];
            let after = &source[cursor..];
            let ghost = if focused
                && self.show_suggestions
                && self.echo == EchoMode::Normal
                && self.cursor == self.value.len()
            {
                let suffix = self
                    .current_suggestion()
                    .map(|s| self.completion_suffix(s))
                    .unwrap_or("");
                let mut bytes = 0;
                let end = suffix
                    .grapheme_indices(true)
                    .enumerate()
                    .find_map(|(n, (i, g))| {
                        if n == self.limit.saturating_sub(self.count)
                            || g.len()
                                > self
                                    .byte_limit
                                    .saturating_sub(self.value.len())
                                    .saturating_sub(bytes)
                        {
                            Some(i)
                        } else {
                            bytes += g.len();
                            None
                        }
                    })
                    .unwrap_or(suffix.len());
                let suffix = &suffix[..end];
                // A combining suffix edits an existing cluster; do not preview
                // it as an independent cursor cell. Acceptance still works.
                if suffix
                    .graphemes(true)
                    .next()
                    .is_some_and(|g| g.width() == 0)
                {
                    ""
                } else {
                    suffix
                }
            } else {
                ""
            };
            let under = if ghost.is_empty() {
                after.graphemes(true).next().unwrap_or(" ")
            } else {
                ghost.graphemes(true).next().unwrap()
            };
            let cursor_width = under.width().max(1).min(usize::from(self.width));
            let budget = usize::from(self.width).saturating_sub(cursor_width);
            let mut start = cursor;
            let mut columns = 0;
            for (i, g) in before.grapheme_indices(true).rev() {
                let width = g.width();
                if columns + width > budget {
                    break;
                }
                columns += width;
                start = i;
            }
            caret_x = columns as u16;
            if !focused {
                View::styled(
                    cell_prefix(&source[start..], usize::from(self.width)),
                    text_style,
                )
                .width(self.width)
            } else {
                let glyph = if under.width() > usize::from(self.width) {
                    " "
                } else {
                    under
                };
                let (rest, rest_style) = if !ghost.is_empty() {
                    (&ghost[under.len()..], state.suggestion)
                } else {
                    (
                        &after[after.graphemes(true).next().map_or(0, str::len)..],
                        text_style,
                    )
                };
                let rest = cell_prefix(
                    rest,
                    usize::from(self.width).saturating_sub(columns + glyph.width().max(1)),
                );
                View::row([
                    View::styled(&source[start..cursor], text_style).width(columns as u16),
                    View::styled(
                        glyph,
                        if self.virtual_cursor {
                            styles.cursor
                        } else if ghost.is_empty() {
                            text_style
                        } else {
                            state.suggestion
                        },
                    )
                    .width(glyph.width().max(1).min(usize::from(self.width)) as u16),
                    View::styled(rest, rest_style),
                ])
                .width(self.width)
            }
        };
        let view = if prompt.is_empty() {
            field
        } else {
            View::row([
                View::styled(prompt.into_owned(), state.prompt).width(prompt_width),
                field,
            ])
            .width(prompt_width.saturating_add(self.width))
        };
        if focused && !self.virtual_cursor && self.width > 0 {
            view.with_cursor(Cursor::new(prompt_width.saturating_add(caret_x), 0))
        } else {
            view
        }
    }
}
fn cell_prefix(text: &str, budget: usize) -> &str {
    let mut width = 0;
    for (i, g) in text.grapheme_indices(true) {
        width += g.width();
        if width > budget {
            return &text[..i];
        }
    }
    text
}
#[cfg(test)]
mod tests {
    use super::*;
    use matea_core::KeyKind;
    use matea_render::Size;
    fn key(code: KeyCode) -> Input {
        Input::Key(Key::new(code))
    }
    fn ctrl(c: char) -> Input {
        Input::Key(Key::new(KeyCode::Char(c)).with_modifiers(Modifiers::CONTROL))
    }
    #[test]
    fn native_cursor_uses_visible_cells_prompt_and_focus_without_reverse_style() {
        let theme = Theme::default();
        let mut input = TextInput::new("あいう");
        input.set_virtual_cursor(false);
        input.width = 10;
        input.prompt = "> ".into();
        let frame = input.view(&theme, true).render(Size::new(20, 1)).unwrap();
        assert_eq!(frame.cursor(), Some(Cursor::new(8, 0)));
        assert_ne!(frame.row(0)[8].style(), input.styles(&theme).cursor);
        input.width = 5;
        assert_eq!(
            input
                .view(&theme, true)
                .render(Size::new(20, 1))
                .unwrap()
                .cursor(),
            Some(Cursor::new(6, 0))
        );
        assert_eq!(
            input
                .view(&theme, false)
                .render(Size::new(20, 1))
                .unwrap()
                .cursor(),
            None
        );
        input.echo = EchoMode::Hidden;
        assert_eq!(
            input
                .view(&theme, true)
                .render(Size::new(20, 1))
                .unwrap()
                .cursor(),
            Some(Cursor::new(2, 0))
        );
        input.echo = EchoMode::Password('*');
        assert_eq!(
            input
                .view(&theme, true)
                .render(Size::new(20, 1))
                .unwrap()
                .cursor(),
            Some(Cursor::new(5, 0))
        );
        input.width = 0;
        assert_eq!(
            input
                .view(&theme, true)
                .render(Size::new(20, 1))
                .unwrap()
                .cursor(),
            None
        );
        input.set_virtual_cursor(true);
        assert!(input.virtual_cursor());
        let mut scrolled = TextInput::new("a".repeat(30));
        scrolled.prompt = "> ".into();
        scrolled.width = 20;
        scrolled.set_virtual_cursor(false);
        scrolled.event(&key(KeyCode::Left), true);
        let frame = scrolled
            .view(&theme, true)
            .render(Size::new(24, 1))
            .unwrap();
        assert_eq!(frame.cursor(), Some(Cursor::new(21, 0)));
    }
    fn invariant(input: &TextInput) {
        assert_eq!(input.count, input.value.graphemes(true).count());
        assert_eq!(input.ascii, input.value.is_ascii());
        assert!(input.cursor <= input.value.len());
        assert!(
            input
                .value
                .grapheme_indices(true)
                .map(|(i, _)| i)
                .chain(std::iter::once(input.value.len()))
                .any(|i| i == input.cursor)
        );
    }
    #[test]
    fn edits_preserve_global_grapheme_boundaries_without_splitting_xyz_or_flags() {
        for original in ["", "tea", "e\u{301}茶👩‍💻", "🇺🇸🇨🇦", "🇺X🇸", "\u{301}e"]
        {
            let len = original.graphemes(true).count();
            for cursor in 0..len + 2 {
                for code in [
                    KeyCode::Left,
                    KeyCode::Right,
                    KeyCode::Backspace,
                    KeyCode::Delete,
                    KeyCode::Home,
                    KeyCode::End,
                ] {
                    let mut input = TextInput::new(original);
                    input.set_cursor(cursor);
                    input.event(&key(code), true);
                    invariant(&input);
                }
            }
        }
        let mut flags = TextInput::new("🇺X🇸");
        flags.set_cursor(1);
        flags.event(&key(KeyCode::Delete), true);
        assert_eq!(flags.value(), "🇺🇸");
        assert_eq!(flags.position(), 1);
        flags.event(&key(KeyCode::Backspace), true);
        assert_eq!(flags.value(), "");
    }
    #[test]
    fn context_merging_at_limit_and_paste_truncation_preserve_existing_suffix() {
        let mut input = TextInput::new("e");
        input.set_limit(1);
        assert_eq!(
            input.event(&key(KeyCode::Char('\u{301}')), true).message,
            Some(Action::Changed)
        );
        assert_eq!(input.value(), "e\u{301}");
        assert_eq!(input.grapheme_len(), 1);
        assert_eq!(input.event(&key(KeyCode::Char('x')), true).message, None);
        let mut joined = TextInput::new("👩💻");
        joined.limit = 1;
        joined.set_cursor(1);
        joined.event(&key(KeyCode::Char('\u{200d}')), true);
        assert_eq!(joined.value(), "👩‍💻");
        invariant(&joined);
        let mut middle = TextInput::new("aZ");
        middle.set_limit(4);
        middle.set_cursor(1);
        middle.event(&Input::Paste("e\u{301}👩‍💻茶".into()), true);
        assert_eq!(middle.value(), "ae\u{301}👩‍💻Z");
        invariant(&middle);
        middle.set_limit(0);
        assert_eq!(middle.value(), "");
    }
    #[test]
    fn byte_budget_and_sanitizer_are_independent_of_grapheme_limits() {
        let mut input = TextInput::default();
        input.set_byte_limit(4);
        input.set_value("茶茶");
        assert_eq!(input.value(), "茶");
        input.event(&key(KeyCode::Char('x')), true);
        assert_eq!(input.value(), "茶x");
        input.event(&key(KeyCode::Char('y')), true);
        assert_eq!(input.value(), "茶x");
        input.set_byte_limit(2);
        assert_eq!(input.value(), "");
        input.set_byte_limit(1024);
        input.set_value("a\t\r\n\0\x1b\u{85}�z");
        assert_eq!(input.value(), "a   �z");
        let huge = format!("e{}", "\u{301}".repeat(40000));
        input.set_value(huge);
        assert_eq!(input.value(), "");
        invariant(&input);
    }
    #[test]
    fn words_custom_bindings_and_masks_share_the_editing_contract() {
        let mut input = TextInput::new("tea 茶  coffee");
        input.event(&ctrl('w'), true);
        assert_eq!(input.value(), "tea 茶  ");
        input.event(&ctrl('w'), true);
        assert_eq!(input.value(), "tea ");
        input.event(&ctrl('u'), true);
        assert_eq!(input.value(), "");
        input.set_value("tea coffee");
        input.cursor_start();
        input.event(
            &Input::Key(Key::new(KeyCode::Char('d')).with_modifiers(Modifiers::ALT)),
            true,
        );
        assert_eq!(input.value(), " coffee");
        input.keys.end = Binding::new([Key::new(KeyCode::F(2))], "F2", "end");
        assert!(!input.event(&key(KeyCode::End), true).consumed);
        input.event(&key(KeyCode::F(2)), true);
        assert_eq!(input.position(), 7);
        input.echo = EchoMode::Password('*');
        input.event(&ctrl('w'), true);
        assert_eq!(input.value(), "");
        input.set_value("secret words");
        input.width = 4;
        assert!(
            !input
                .view(&Theme::plain(), true)
                .render(Size::new(4, 1))
                .unwrap()
                .plain()
                .contains("secret")
        );
        assert!(!format!("{input:?}").contains("secret"));
        input.echo = EchoMode::Hidden;
        assert_eq!(
            input
                .view(&Theme::plain(), true)
                .render(Size::new(4, 1))
                .unwrap()
                .plain(),
            ""
        );
    }
    #[test]
    fn completion_handles_empty_selection_unicode_case_expansion_and_combining_marks() {
        let mut input = TextInput {
            show_suggestions: true,
            ..TextInput::default()
        };
        assert!(!input.event(&key(KeyCode::Up), true).consumed);
        assert_eq!(input.current_suggestion(), None);
        assert_eq!(input.current_suggestion_index(), None);
        input.set_suggestions(["Tea", "TEAM", "toast"]);
        input.set_value("te");
        assert_eq!(
            input.matched_suggestions().collect::<Vec<_>>(),
            vec!["Tea", "TEAM"]
        );
        input.event(&key(KeyCode::Down), true);
        assert_eq!(input.current_suggestion(), Some("TEAM"));
        input.event(&key(KeyCode::Tab), true);
        assert_eq!(input.value(), "teAM");
        input.set_suggestions(["İzmir"]);
        input.set_value("i");
        input.event(&key(KeyCode::Tab), true);
        assert_eq!(input.value(), "izmir");
        invariant(&input);
        input.set_suggestions(["e\u{301}"]);
        input.set_value("e");
        input.set_limit(1);
        input.event(&key(KeyCode::Tab), true);
        assert_eq!(input.value(), "e\u{301}");
        invariant(&input);
    }
    #[test]
    fn completions_respect_limits_and_live_validation_and_form_tab_ownership() {
        use crate::{Field, Form, required};
        let mut input = TextInput::new("t").with_validator(|s| {
            if s.len() == 2 {
                Ok(())
            } else {
                Err("two bytes".into())
            }
        });
        input.set_limit(2);
        input.show_suggestions = true;
        input.set_suggestions(["tea"]);
        assert!(input.error().is_some());
        assert_eq!(
            input.event(&key(KeyCode::Tab), true).message,
            Some(Action::Changed)
        );
        assert_eq!(input.value(), "te");
        assert_eq!(input.error(), None);
        assert!(!input.event(&key(KeyCode::Tab), true).consumed);
        let mut first = Field::new("Tea", required);
        first.input = input;
        first.input.set_limit(3);
        let mut form = Form::new(vec![first, Field::new("City", required)]);
        form.event(&key(KeyCode::Tab), true);
        assert_eq!(form.focus_index(), 0);
        assert_eq!(form.fields()[0].input.value(), "tea");
        form.event(&key(KeyCode::Tab), true);
        assert_eq!(form.focus_index(), 1);
    }
    #[test]
    fn placeholders_clip_by_cell_width_without_nuls_and_views_keep_cursor_visible() {
        let mut input = TextInput {
            prompt: "> ".into(),
            placeholder: "输入消息...".into(),
            width: 20,
            ..TextInput::default()
        };
        for focused in [false, true] {
            let frame = input
                .view(&Theme::plain(), focused)
                .render(Size::new(22, 1))
                .unwrap();
            assert_eq!(frame.plain(), "> 输入消息...");
            assert_eq!(frame.row(0)[2].width(), 2);
            assert!(!frame.plain().contains('\0'));
        }
        input.placeholder = "A very long placeholder".into();
        input.width = 10;
        assert_eq!(
            input
                .view(&Theme::plain(), true)
                .render(Size::new(12, 1))
                .unwrap()
                .plain(),
            "> A very lon"
        );
        input.prompt.clear();
        input.set_value("e\u{301}茶👩‍💻abcdef");
        input.width = 5;
        let frame = input
            .view(&Theme::plain(), true)
            .render(Size::new(5, 1))
            .unwrap();
        assert_eq!(frame.plain(), "cdef");
        assert_eq!(frame.row(0)[4].style(), Theme::plain().focus);
        for width in 0..5 {
            input.width = width;
            input.cursor_start();
            input
                .view(&Theme::plain(), true)
                .render(Size::new(width, 1))
                .unwrap();
        }
    }
    #[test]
    fn ignored_events_keep_error_and_state_and_repeat_is_an_edit() {
        let mut input = TextInput::new("t").with_validator(|_| Err("invalid".into()));
        for event in [
            Input::Focus(false),
            Input::Key(Key::new(KeyCode::Char('x')).with_kind(KeyKind::Release)),
            Input::Key(Key::new(KeyCode::Char('x')).with_modifiers(Modifiers::SUPER)),
        ] {
            assert!(!input.event(&event, true).consumed);
            assert_eq!(input.error(), Some("invalid"));
            assert_eq!(input.value(), "t");
        }
        input.event(
            &Input::Key(Key::new(KeyCode::Char('x')).with_kind(KeyKind::Repeat)),
            true,
        );
        assert_eq!(input.value(), "tx");
        input.clear_validator();
        assert_eq!(input.error(), None);
    }
    #[test]
    fn wide_completion_reserves_the_cursor_and_preview_respects_limits() {
        let mut input = TextInput::new("abcd");
        input.show_suggestions = true;
        input.width = 5;
        input.set_suggestions(["abcd茶x"]);
        let frame = input
            .view(&Theme::plain(), true)
            .render(Size::new(5, 1))
            .unwrap();
        assert_eq!(frame.plain(), "bcd茶");
        assert_eq!(frame.row(0)[3].style(), Theme::plain().focus);
        input.set_value("t");
        input.set_limit(2);
        input.set_suggestions(["tea"]);
        assert_eq!(
            input
                .view(&Theme::plain(), true)
                .render(Size::new(5, 1))
                .unwrap()
                .plain(),
            "te"
        );
    }
    #[test]
    fn form_validation_cannot_bypass_the_input_validator() {
        use crate::{Field, Form, required};
        let mut field = Field::new("Tea", required);
        field.input = TextInput::new("t").with_validator(|s| {
            if s.len() == 2 {
                Ok(())
            } else {
                Err("two bytes".into())
            }
        });
        let mut form = Form::new(vec![field]);
        assert_eq!(form.fields()[0].error(), Some("two bytes"));
        form.event(&key(KeyCode::Enter), true);
        assert!(!form.submitted());
        assert!(
            form.view(&Theme::plain(), true)
                .render(Size::new(40, 4))
                .unwrap()
                .plain()
                .contains("Error: two bytes")
        );
        form.event(&key(KeyCode::Char('e')), true);
        assert_eq!(form.fields()[0].error(), None);
        assert_eq!(
            form.event(&key(KeyCode::Enter), true).message,
            Some(Action::Submit)
        );
    }
    #[test]
    fn form_submission_rechecks_validators_that_capture_other_ui_state() {
        use crate::{Field, Form, required};
        use std::cell::Cell;
        let allowed = Rc::new(Cell::new(true));
        let captured = Rc::clone(&allowed);
        let mut field = Field::new("Tea", required);
        field.input = TextInput::new("tea").with_validator(move |_| {
            if captured.get() {
                Ok(())
            } else {
                Err("unavailable".into())
            }
        });
        let mut form = Form::new(vec![field]);
        allowed.set(false);
        assert_eq!(form.event(&key(KeyCode::Enter), true).message, None);
        assert!(!form.submitted());
        assert_eq!(form.fields()[0].error(), Some("unavailable"));
    }
    #[test]
    fn custom_styles_switch_with_focus_without_embedded_terminal_sequences() {
        let mut input = TextInput::new("tea");
        input.set_cursor(1);
        let mut styles = InputStyles::from_theme(&Theme::plain());
        styles.focused.text = Style::default().bold();
        styles.blurred.text = Style::default().underline();
        input.set_styles(styles);
        let active = input
            .view(&Theme::plain(), true)
            .render(Size::new(3, 1))
            .unwrap();
        let inactive = input
            .view(&Theme::plain(), false)
            .render(Size::new(3, 1))
            .unwrap();
        assert_eq!(active.row(0)[0].style(), styles.focused.text);
        assert_eq!(active.row(0)[1].style(), styles.cursor);
        assert_eq!(inactive.row(0)[0].style(), styles.blurred.text);
        assert_eq!(active.plain(), inactive.plain());
        assert_eq!(input.styles(&Theme::plain()), styles);
    }
    #[test]
    fn deleting_at_word_end_never_panics_or_erases_the_next_word() {
        for (value, expected) in [("abc d", "a d"), ("ab c", "a c"), ("ab", "a")] {
            let mut input = TextInput::new(value);
            input.set_cursor(1);
            input.event(
                &Input::Key(Key::new(KeyCode::Char('d')).with_modifiers(Modifiers::ALT)),
                true,
            );
            assert_eq!(input.value(), expected);
            invariant(&input);
        }
        let mut input = TextInput {
            placeholder: "Something".into(),
            ..TextInput::default()
        };
        input.set_width(None);
        assert_eq!(
            input
                .view(&Theme::plain(), false)
                .render(Size::new(20, 1))
                .unwrap()
                .plain(),
            "Something"
        );
        input.set_width(Some(0));
        assert_eq!(
            input
                .view(&Theme::plain(), false)
                .render(Size::new(20, 1))
                .unwrap()
                .plain(),
            ""
        );
    }
}
