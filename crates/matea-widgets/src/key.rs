//! Reusable key bindings: matching and help share the same enabled binding.
use matea_core::{Key, KeyCode};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub keys: Vec<Key>,
    pub label: String,
    pub description: String,
    pub enabled: bool,
}
impl Binding {
    pub fn new(
        keys: impl IntoIterator<Item = Key>,
        label: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            keys: keys.into_iter().collect(),
            label: label.into(),
            description: description.into(),
            enabled: true,
        }
    }
    pub fn matches(&self, key: Key) -> bool {
        self.enabled && self.keys.contains(&key)
    }
    pub fn help(&self) -> Option<(&str, &str)> {
        self.enabled.then_some((&self.label, &self.description))
    }
}
#[derive(Clone, Debug)]
pub struct NavigationKeys {
    pub up: Binding,
    pub down: Binding,
    pub first: Binding,
    pub last: Binding,
    pub page_up: Binding,
    pub page_down: Binding,
    pub accept: Binding,
}
impl Default for NavigationKeys {
    fn default() -> Self {
        let bind = |keys: Vec<KeyCode>, label, description| {
            Binding::new(keys.into_iter().map(Key::new), label, description)
        };
        Self {
            up: bind(vec![KeyCode::Up, KeyCode::Char('k')], "↑/k", "up"),
            down: bind(vec![KeyCode::Down, KeyCode::Char('j')], "↓/j", "down"),
            first: bind(vec![KeyCode::Home], "home", "first"),
            last: bind(vec![KeyCode::End], "end", "last"),
            page_up: bind(vec![KeyCode::PageUp], "pgup", "previous page"),
            page_down: bind(vec![KeyCode::PageDown], "pgdn", "next page"),
            accept: bind(vec![KeyCode::Enter], "enter", "select"),
        }
    }
}
