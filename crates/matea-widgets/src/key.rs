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
        self.is_enabled()
            && key.is_down()
            && self
                .keys
                .iter()
                .any(|chord| key.code == chord.code && key.modifiers == chord.modifiers)
    }
    pub fn is_enabled(&self) -> bool {
        self.enabled && !self.keys.is_empty()
    }
    /// Remove shortcuts and help together. Existing allocation capacity can be reused.
    pub fn unbind(&mut self) {
        self.keys.clear();
        self.label.clear();
        self.description.clear();
    }
    pub fn help(&self) -> Option<(&str, &str)> {
        self.is_enabled()
            .then_some((&self.label, &self.description))
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

/// Match any enabled binding without formatting a keystroke into a String.
pub fn matches<'a>(key: Key, bindings: impl IntoIterator<Item = &'a Binding>) -> bool {
    bindings.into_iter().any(|binding| binding.matches(key))
}
#[cfg(test)]
mod tests {
    use super::*;
    use matea_core::KeyKind;
    #[test]
    fn unbinding_disabled_and_repeat_keep_help_and_matching_consistent() {
        let key = Key::new(KeyCode::F(12));
        let mut binding = Binding::new([key], "F12", "inspect");
        assert!(matches(key.with_kind(KeyKind::Repeat), [&binding]));
        assert!(!matches(key.with_kind(KeyKind::Release), [&binding]));
        binding.enabled = false;
        assert!(!binding.matches(key));
        assert!(binding.help().is_none());
        binding.enabled = true;
        binding.unbind();
        assert!(!binding.is_enabled());
        assert!(binding.help().is_none());
        assert!(!binding.matches(key));
        assert!(binding.label.is_empty() && binding.description.is_empty());
        binding.keys.push(key);
        assert!(binding.matches(key));
    }
}
