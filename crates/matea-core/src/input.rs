//! Transport-neutral input values. A terminal can report only a subset; absence
//! of event metadata is not proof that a modifier/lock is physically inactive.
use bitflags::bitflags;

bitflags! {
    #[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Hash)]
    pub struct Modifiers:u8 {
        const SHIFT=1;
        const CONTROL=2;
        const ALT=4;
        const SUPER=8;
        const HYPER=16;
        const META=32;
    }
    #[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Hash)]
    pub struct KeyState:u8 {
        const KEYPAD=1;
        const CAPS_LOCK=2;
        const NUM_LOCK=4;
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyKind {
    #[default]
    Press,
    Repeat,
    Release,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MediaKey {
    Play,
    Pause,
    PlayPause,
    Reverse,
    Stop,
    FastForward,
    Rewind,
    TrackNext,
    TrackPrevious,
    Record,
    LowerVolume,
    RaiseVolume,
    MuteVolume,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModifierKey {
    LeftShift,
    LeftControl,
    LeftAlt,
    LeftSuper,
    LeftHyper,
    LeftMeta,
    RightShift,
    RightControl,
    RightAlt,
    RightSuper,
    RightHyper,
    RightMeta,
    IsoLevel3Shift,
    IsoLevel5Shift,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Char(char),
    Enter,
    Esc,
    Backspace,
    Delete,
    Insert,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Tab,
    BackTab,
    PageUp,
    PageDown,
    F(u8),
    Null,
    CapsLock,
    ScrollLock,
    NumLock,
    PrintScreen,
    Pause,
    Menu,
    KeypadBegin,
    Media(MediaKey),
    Modifier(ModifierKey),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    pub code: KeyCode,
    pub modifiers: Modifiers,
    pub kind: KeyKind,
    pub state: KeyState,
}
impl Key {
    pub const fn new(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: Modifiers::empty(),
            kind: KeyKind::Press,
            state: KeyState::empty(),
        }
    }
    pub const fn with_modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = modifiers;
        self
    }
    pub const fn with_kind(mut self, kind: KeyKind) -> Self {
        self.kind = kind;
        self
    }
    pub const fn with_state(mut self, state: KeyState) -> Self {
        self.state = state;
        self
    }
    pub const fn is_down(self) -> bool {
        matches!(self.kind, KeyKind::Press | KeyKind::Repeat)
    }
    /// Down event without command modifiers. Shift is allowed for case-sensitive
    /// text and shifted symbols; use an exact chord for modifier-sensitive input.
    pub const fn is_plain(self) -> bool {
        self.is_down() && self.modifiers.bits() & !Modifiers::SHIFT.bits() == 0
    }
    pub fn is_char(self, c: char) -> bool {
        self.is_plain() && self.code == KeyCode::Char(c)
    }
    /// Compare a shortcut independent of repeat/release metadata or lock state.
    /// The receiver is an event: release never triggers an ordinary binding.
    pub fn matches_chord(self, chord: Self) -> bool {
        self.is_down() && self.code == chord.code && self.modifiers == chord.modifiers
    }
    /// +1 forward / -1 backward traversal, only for plain Tab variants.
    pub fn focus_direction(self) -> Option<i8> {
        if !self.is_plain() {
            return None;
        }
        match self.code {
            KeyCode::Tab if !self.modifiers.contains(Modifiers::SHIFT) => Some(1),
            KeyCode::Tab | KeyCode::BackTab => Some(-1),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseKind {
    Down(MouseButton),
    Up(MouseButton),
    Drag(MouseButton),
    Moved,
    Scroll(ScrollDirection),
}
/// Zero-based cell coordinates. Pixel coordinates/button identity can be
/// unavailable in a backend; no precision is inferred beyond the source event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Mouse {
    pub kind: MouseKind,
    pub column: u16,
    pub row: u16,
    pub modifiers: Modifiers,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    Key(Key),
    Mouse(Mouse),
    Paste(String),
    Focus(bool),
}
impl Input {
    pub fn key_down(&self) -> Option<Key> {
        match self {
            Self::Key(key) if key.is_down() => Some(*key),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chords_ignore_repeat_and_lock_state_but_preserve_modifiers_and_release() {
        let chord = Key::new(KeyCode::Char('x')).with_modifiers(Modifiers::CONTROL);
        assert!(
            chord
                .with_kind(KeyKind::Repeat)
                .with_state(KeyState::CAPS_LOCK)
                .matches_chord(chord)
        );
        assert!(!chord.with_kind(KeyKind::Release).matches_chord(chord));
        assert!(
            !chord
                .with_modifiers(Modifiers::CONTROL | Modifiers::SUPER)
                .matches_chord(chord)
        );
        for modifiers in [
            Modifiers::CONTROL,
            Modifiers::ALT,
            Modifiers::SUPER,
            Modifiers::HYPER,
            Modifiers::META,
        ] {
            assert!(
                !Key::new(KeyCode::Char('x'))
                    .with_modifiers(modifiers)
                    .is_char('x')
            );
            assert_eq!(
                Key::new(KeyCode::Tab)
                    .with_modifiers(modifiers)
                    .focus_direction(),
                None
            );
        }
        assert_eq!(
            Key::new(KeyCode::Tab)
                .with_kind(KeyKind::Release)
                .focus_direction(),
            None
        );
        assert_eq!(
            Key::new(KeyCode::Tab)
                .with_kind(KeyKind::Repeat)
                .focus_direction(),
            Some(1)
        );
        assert_eq!(
            Key::new(KeyCode::Tab)
                .with_modifiers(Modifiers::SHIFT)
                .focus_direction(),
            Some(-1)
        );
    }
    #[test]
    fn focus_ring_ignores_release_and_modified_tab() {
        let mut focus = crate::FocusRing::new(["first", "last"]);
        for key in [
            Key::new(KeyCode::Tab).with_kind(KeyKind::Release),
            Key::new(KeyCode::Tab).with_modifiers(Modifiers::CONTROL),
            Key::new(KeyCode::BackTab).with_modifiers(Modifiers::ALT),
        ] {
            assert!(!focus.handle(&Input::Key(key)));
            assert_eq!(focus.current(), Some("first"));
        }
        assert!(focus.handle(&Input::Key(
            Key::new(KeyCode::Tab).with_kind(KeyKind::Repeat)
        )));
        assert_eq!(focus.current(), Some("last"));
    }
}
