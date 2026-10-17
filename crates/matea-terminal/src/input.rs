//! Convert backend events without discarding event kinds or modifiers.
use crate::TerminalEvent;
use crossterm::event as source;
use matea_core::{
    Input, Key, KeyCode, KeyKind, KeyState, MediaKey, ModifierKey, Modifiers, Mouse, MouseButton,
    MouseKind, ScrollDirection,
};
use matea_render::Size;
fn modifiers(value: source::KeyModifiers) -> Modifiers {
    let mut result = Modifiers::empty();
    if value.contains(source::KeyModifiers::SHIFT) {
        result |= Modifiers::SHIFT;
    }
    if value.contains(source::KeyModifiers::CONTROL) {
        result |= Modifiers::CONTROL;
    }
    if value.contains(source::KeyModifiers::ALT) {
        result |= Modifiers::ALT;
    }
    if value.contains(source::KeyModifiers::SUPER) {
        result |= Modifiers::SUPER;
    }
    if value.contains(source::KeyModifiers::HYPER) {
        result |= Modifiers::HYPER;
    }
    if value.contains(source::KeyModifiers::META) {
        result |= Modifiers::META;
    }
    result
}
fn key(value: source::KeyEvent) -> Key {
    let code = match value.code {
        source::KeyCode::Char('\0') => KeyCode::Null,
        source::KeyCode::Char(c) => KeyCode::Char(c),
        source::KeyCode::F(number) => KeyCode::F(number),
        source::KeyCode::Backspace => KeyCode::Backspace,
        source::KeyCode::Enter => KeyCode::Enter,
        source::KeyCode::Left => KeyCode::Left,
        source::KeyCode::Right => KeyCode::Right,
        source::KeyCode::Up => KeyCode::Up,
        source::KeyCode::Down => KeyCode::Down,
        source::KeyCode::Home => KeyCode::Home,
        source::KeyCode::End => KeyCode::End,
        source::KeyCode::PageUp => KeyCode::PageUp,
        source::KeyCode::PageDown => KeyCode::PageDown,
        source::KeyCode::Tab => KeyCode::Tab,
        source::KeyCode::BackTab => KeyCode::BackTab,
        source::KeyCode::Delete => KeyCode::Delete,
        source::KeyCode::Insert => KeyCode::Insert,
        source::KeyCode::Null => KeyCode::Null,
        source::KeyCode::Esc => KeyCode::Esc,
        source::KeyCode::CapsLock => KeyCode::CapsLock,
        source::KeyCode::ScrollLock => KeyCode::ScrollLock,
        source::KeyCode::NumLock => KeyCode::NumLock,
        source::KeyCode::PrintScreen => KeyCode::PrintScreen,
        source::KeyCode::Pause => KeyCode::Pause,
        source::KeyCode::Menu => KeyCode::Menu,
        source::KeyCode::KeypadBegin => KeyCode::KeypadBegin,
        source::KeyCode::Media(code) => KeyCode::Media(match code {
            source::MediaKeyCode::Play => MediaKey::Play,
            source::MediaKeyCode::Pause => MediaKey::Pause,
            source::MediaKeyCode::PlayPause => MediaKey::PlayPause,
            source::MediaKeyCode::Reverse => MediaKey::Reverse,
            source::MediaKeyCode::Stop => MediaKey::Stop,
            source::MediaKeyCode::FastForward => MediaKey::FastForward,
            source::MediaKeyCode::Rewind => MediaKey::Rewind,
            source::MediaKeyCode::TrackNext => MediaKey::TrackNext,
            source::MediaKeyCode::TrackPrevious => MediaKey::TrackPrevious,
            source::MediaKeyCode::Record => MediaKey::Record,
            source::MediaKeyCode::LowerVolume => MediaKey::LowerVolume,
            source::MediaKeyCode::RaiseVolume => MediaKey::RaiseVolume,
            source::MediaKeyCode::MuteVolume => MediaKey::MuteVolume,
        }),
        source::KeyCode::Modifier(code) => KeyCode::Modifier(match code {
            source::ModifierKeyCode::LeftShift => ModifierKey::LeftShift,
            source::ModifierKeyCode::LeftControl => ModifierKey::LeftControl,
            source::ModifierKeyCode::LeftAlt => ModifierKey::LeftAlt,
            source::ModifierKeyCode::LeftSuper => ModifierKey::LeftSuper,
            source::ModifierKeyCode::LeftHyper => ModifierKey::LeftHyper,
            source::ModifierKeyCode::LeftMeta => ModifierKey::LeftMeta,
            source::ModifierKeyCode::RightShift => ModifierKey::RightShift,
            source::ModifierKeyCode::RightControl => ModifierKey::RightControl,
            source::ModifierKeyCode::RightAlt => ModifierKey::RightAlt,
            source::ModifierKeyCode::RightSuper => ModifierKey::RightSuper,
            source::ModifierKeyCode::RightHyper => ModifierKey::RightHyper,
            source::ModifierKeyCode::RightMeta => ModifierKey::RightMeta,
            source::ModifierKeyCode::IsoLevel3Shift => ModifierKey::IsoLevel3Shift,
            source::ModifierKeyCode::IsoLevel5Shift => ModifierKey::IsoLevel5Shift,
        }),
    };
    let kind = match value.kind {
        source::KeyEventKind::Press => KeyKind::Press,
        source::KeyEventKind::Repeat => KeyKind::Repeat,
        source::KeyEventKind::Release => KeyKind::Release,
    };
    let mut state = KeyState::empty();
    if value.state.contains(source::KeyEventState::KEYPAD) {
        state |= KeyState::KEYPAD;
    }
    if value.state.contains(source::KeyEventState::CAPS_LOCK) {
        state |= KeyState::CAPS_LOCK;
    }
    if value.state.contains(source::KeyEventState::NUM_LOCK) {
        state |= KeyState::NUM_LOCK;
    }
    Key {
        code,
        kind,
        modifiers: modifiers(value.modifiers),
        state,
    }
}
fn button(value: source::MouseButton) -> MouseButton {
    match value {
        source::MouseButton::Left => MouseButton::Left,
        source::MouseButton::Middle => MouseButton::Middle,
        source::MouseButton::Right => MouseButton::Right,
    }
}
/// Exhaustive conversion of the released Crossterm 0.29 event model. This does
/// not parse bytes or infer metadata absent from the backend. Paste is moved
/// intact; non-paste conversion allocates no strings or heap objects.
pub fn translate_event(event: source::Event) -> TerminalEvent {
    match event {
        source::Event::Resize(w, h) => TerminalEvent::Resize(Size::new(w, h)),
        source::Event::Paste(text) => TerminalEvent::Input(Input::Paste(text)),
        source::Event::FocusGained => TerminalEvent::Input(Input::Focus(true)),
        source::Event::FocusLost => TerminalEvent::Input(Input::Focus(false)),
        source::Event::Key(value) => TerminalEvent::Input(Input::Key(key(value))),
        source::Event::Mouse(value) => TerminalEvent::Input(Input::Mouse(Mouse {
            column: value.column,
            row: value.row,
            modifiers: modifiers(value.modifiers),
            kind: match value.kind {
                source::MouseEventKind::Down(b) => MouseKind::Down(button(b)),
                source::MouseEventKind::Up(b) => MouseKind::Up(button(b)),
                source::MouseEventKind::Drag(b) => MouseKind::Drag(button(b)),
                source::MouseEventKind::Moved => MouseKind::Moved,
                source::MouseEventKind::ScrollUp => MouseKind::Scroll(ScrollDirection::Up),
                source::MouseEventKind::ScrollDown => MouseKind::Scroll(ScrollDirection::Down),
                source::MouseEventKind::ScrollLeft => MouseKind::Scroll(ScrollDirection::Left),
                source::MouseEventKind::ScrollRight => MouseKind::Scroll(ScrollDirection::Right),
            },
        })),
    }
}
