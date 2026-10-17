#![forbid(unsafe_code)]
//! Stateful components sharing input, theme and focus contracts.
mod clocks;
mod help;
pub mod key;
mod pagination;
mod paginator;
mod spinner;
mod textinput;
pub use clocks::{
    ClockId, ClockSchedule, ClockText, ClockTick, ClockUpdate, Countdown, Stopwatch, TickInterval,
    ZeroTickInterval,
};
pub use help::{Help, HelpItem, HelpMap, HelpStyles};
use matea_core::{Component, Input, KeyCode, Response};
use matea_render::{View, sanitize};
use matea_style::Theme;
pub use pagination::Pagination;
pub use paginator::{PageDisplay, PageKeys, PageNumbers, Paginator};
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
    pub fn filter_with(&mut self, query: &str, mut matches: impl FnMut(&str, &str) -> bool) {
        self.visible.clear();
        self.visible.extend(
            self.items
                .iter()
                .enumerate()
                .filter(|(_, s)| matches(s, query))
                .map(|(i, _)| i),
        );
        self.selected = 0;
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

#[derive(Clone, Debug)]
pub struct Viewport {
    lines: Vec<String>,
    offset: usize,
    pub rows: u16,
}
impl Viewport {
    pub fn new(text: &str) -> Self {
        Self {
            lines: sanitize(text).lines().map(str::to_owned).collect(),
            offset: 0,
            rows: 6,
        }
    }
    pub fn offset(&self) -> usize {
        self.offset
    }
}
impl Component for Viewport {
    type Message = Action;
    fn event(&mut self, input: &Input, focused: bool) -> Response<Action> {
        if !focused {
            return Response::ignored();
        }
        let Input::Key(k) = input else {
            return Response::ignored();
        };
        if !k.is_plain() {
            return Response::ignored();
        }
        let max = self.lines.len().saturating_sub(usize::from(self.rows));
        match k.code {
            KeyCode::Down => self.offset = self.offset.saturating_add(1),
            KeyCode::Up => self.offset = self.offset.saturating_sub(1),
            KeyCode::PageDown => self.offset = self.offset.saturating_add(usize::from(self.rows)),
            KeyCode::PageUp => self.offset = self.offset.saturating_sub(usize::from(self.rows)),
            KeyCode::Home => self.offset = 0,
            KeyCode::End => self.offset = max,
            _ => return Response::ignored(),
        }
        self.offset = self.offset.min(max);
        Response::consumed(Some(Action::Changed))
    }
    fn view(&self, theme: &Theme, _: bool) -> View {
        let offset = self
            .offset
            .min(self.lines.len().saturating_sub(usize::from(self.rows)));
        View::styled(
            self.lines
                .iter()
                .skip(offset)
                .take(usize::from(self.rows))
                .cloned()
                .collect::<Vec<_>>()
                .join("\n"),
            theme.text,
        )
        .height(self.rows)
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
}
