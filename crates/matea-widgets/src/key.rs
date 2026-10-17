//! Reusable key bindings: matching and help share the same enabled binding.
use compact_str::CompactString;
use matea_core::{Key, KeyCode};
use std::borrow::Borrow;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding<K = Key> {
    pub keys: Vec<K>,
    pub label: CompactString,
    pub description: CompactString,
    pub enabled: bool,
}
impl<K> Binding<K> {
    pub fn new(
        keys: impl IntoIterator<Item = K>,
        label: impl Into<CompactString>,
        description: impl Into<CompactString>,
    ) -> Self {
        Self {
            keys: keys.into_iter().collect(),
            label: label.into(),
            description: description.into(),
            enabled: true,
        }
    }
    /// Match custom tokens by borrowed equality, with no implicit formatting.
    /// Keyboard events normally use matches, which additionally handles key kind.
    #[inline]
    pub fn matches_value<Q: ?Sized + Eq>(&self, value: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.is_enabled()
            && self.keys.iter().any(|key| {
                let key = key.borrow();
                // Eq guarantees reflexivity. This also handles interned tokens
                // without calling a byte comparator; distinct values still use Eq.
                std::ptr::eq(key, value) || key == value
            })
    }
    #[inline]
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
impl Binding<Key> {
    #[inline]
    pub fn matches(&self, key: Key) -> bool {
        self.is_enabled()
            && key.is_down()
            && self
                .keys
                .iter()
                .any(|chord| key.code == chord.code && key.modifiers == chord.modifiers)
    }
}

/// Match any enabled custom-token binding without allocating a formatted key.
pub fn matches_value<'a, K: Borrow<Q> + 'a, Q: ?Sized + Eq>(
    value: &Q,
    bindings: impl IntoIterator<Item = &'a Binding<K>>,
) -> bool {
    bindings
        .into_iter()
        .any(|binding| binding.matches_value(value))
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
    fn custom_tokens_share_enable_unbind_help_and_borrowed_matching() {
        let mut text = Binding::new(["ctrl+x".to_owned(), "F12".to_owned()], "x/F12", "inspect");
        assert!(text.matches_value("F12"));
        assert!(!text.matches_value("f12"));
        assert!(matches_value("ctrl+x", [&text]));
        text.enabled = false;
        assert!(!text.matches_value("F12"));
        assert!(text.help().is_none());
        text.enabled = true;
        text.keys.clear();
        // Empty retained storage cannot advertise an actionable shortcut.
        assert!(!text.is_enabled());
        text.keys.push("a".into());
        text.label = "a".into();
        text.description = "accept".into();
        assert_eq!(text.help(), Some(("a", "accept")));
        let label = "抹茶".repeat(20);
        let description = "e\u{301}".repeat(20);
        text.label = label.clone().into();
        text.description = description.clone().into();
        assert_eq!(text.help(), Some((label.as_str(), description.as_str())));
        text.unbind();
        assert!(text.keys.is_empty() && text.label.is_empty() && text.description.is_empty());
        #[derive(PartialEq, Eq)]
        enum Command {
            Accept,
            Cancel,
        }
        let commands = Binding::new([Command::Accept], "enter", "accept");
        assert!(commands.matches_value(&Command::Accept));
        assert!(!commands.matches_value(&Command::Cancel));
    }
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
