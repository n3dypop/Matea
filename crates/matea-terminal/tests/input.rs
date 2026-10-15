use crossterm::event as source;
use matea_core::{
    Input, Key, KeyCode, KeyKind, KeyState, Modifiers, MouseButton, MouseKind, ScrollDirection,
};
use matea_terminal::{TerminalEvent, translate_event};
fn key(event: source::KeyEvent) -> Key {
    let TerminalEvent::Input(Input::Key(key)) = translate_event(source::Event::Key(event)) else {
        panic!("key event lost")
    };
    key
}
#[test]
fn key_kinds_modifiers_and_lock_state_survive_all_combinations() {
    for (kind, expected_kind) in [
        (source::KeyEventKind::Press, KeyKind::Press),
        (source::KeyEventKind::Repeat, KeyKind::Repeat),
        (source::KeyEventKind::Release, KeyKind::Release),
    ] {
        for modifiers in 0..64 {
            for state in 0..8 {
                let event = source::KeyEvent::new_with_kind_and_state(
                    source::KeyCode::F(12),
                    source::KeyModifiers::from_bits(modifiers).unwrap(),
                    kind,
                    source::KeyEventState::from_bits(state).unwrap(),
                );
                let translated = key(event);
                assert_eq!(translated.code, KeyCode::F(12));
                assert_eq!(translated.kind, expected_kind);
                assert_eq!(translated.modifiers.bits(), modifiers);
                assert_eq!(translated.state.bits(), state);
            }
        }
    }
    let modified = key(source::KeyEvent::new(
        source::KeyCode::Char('q'),
        source::KeyModifiers::SUPER | source::KeyModifiers::HYPER | source::KeyModifiers::META,
    ));
    assert!(!modified.is_char('q'));
    assert!(
        modified
            .modifiers
            .contains(Modifiers::SUPER | Modifiers::HYPER | Modifiers::META)
    );
    assert_eq!(
        key(source::KeyEvent::new(
            source::KeyCode::Char('\0'),
            source::KeyModifiers::CONTROL
        ))
        .code,
        KeyCode::Null
    );
    assert_eq!(
        key(source::KeyEvent::new(
            source::KeyCode::Null,
            source::KeyModifiers::NONE
        ))
        .code,
        KeyCode::Null
    );
    assert_eq!(
        Key::new(KeyCode::Tab)
            .with_state(KeyState::NUM_LOCK)
            .focus_direction(),
        Some(1)
    );
}
#[test]
fn every_nonpayload_function_media_and_modifier_code_survives() {
    use source::{KeyCode as C, MediaKeyCode as M, ModifierKeyCode as K};
    let mut codes = vec![
        C::Backspace,
        C::Enter,
        C::Left,
        C::Right,
        C::Up,
        C::Down,
        C::Home,
        C::End,
        C::PageUp,
        C::PageDown,
        C::Tab,
        C::BackTab,
        C::Delete,
        C::Insert,
        C::Null,
        C::Esc,
        C::CapsLock,
        C::ScrollLock,
        C::NumLock,
        C::PrintScreen,
        C::Pause,
        C::Menu,
        C::KeypadBegin,
        C::Char('抹'),
    ];
    codes.extend((1..=35).map(C::F));
    codes.extend(
        [
            M::Play,
            M::Pause,
            M::PlayPause,
            M::Reverse,
            M::Stop,
            M::FastForward,
            M::Rewind,
            M::TrackNext,
            M::TrackPrevious,
            M::Record,
            M::LowerVolume,
            M::RaiseVolume,
            M::MuteVolume,
        ]
        .into_iter()
        .map(C::Media),
    );
    codes.extend(
        [
            K::LeftShift,
            K::LeftControl,
            K::LeftAlt,
            K::LeftSuper,
            K::LeftHyper,
            K::LeftMeta,
            K::RightShift,
            K::RightControl,
            K::RightAlt,
            K::RightSuper,
            K::RightHyper,
            K::RightMeta,
            K::IsoLevel3Shift,
            K::IsoLevel5Shift,
        ]
        .into_iter()
        .map(C::Modifier),
    );
    for code in codes {
        // Backend and transport-neutral code names intentionally match, while
        // their concrete Rust types and ownership are independent.
        assert_eq!(
            format!("{code:?}"),
            format!(
                "{:?}",
                key(source::KeyEvent::new(code, source::KeyModifiers::NONE)).code
            )
        );
    }
}
#[test]
fn mouse_coordinates_buttons_scroll_and_modifiers_are_preserved() {
    use source::{MouseButton as B, MouseEventKind as K};
    let cases = [
        (K::Down(B::Left), MouseKind::Down(MouseButton::Left)),
        (K::Up(B::Right), MouseKind::Up(MouseButton::Right)),
        (K::Drag(B::Middle), MouseKind::Drag(MouseButton::Middle)),
        (K::Moved, MouseKind::Moved),
        (K::ScrollUp, MouseKind::Scroll(ScrollDirection::Up)),
        (K::ScrollDown, MouseKind::Scroll(ScrollDirection::Down)),
        (K::ScrollLeft, MouseKind::Scroll(ScrollDirection::Left)),
        (K::ScrollRight, MouseKind::Scroll(ScrollDirection::Right)),
    ];
    for (kind, expected) in cases {
        let event = source::Event::Mouse(source::MouseEvent {
            kind,
            column: u16::MAX,
            row: 0,
            modifiers: source::KeyModifiers::SHIFT | source::KeyModifiers::ALT,
        });
        let TerminalEvent::Input(Input::Mouse(mouse)) = translate_event(event) else {
            panic!("mouse event lost")
        };
        assert_eq!(mouse.kind, expected);
        assert_eq!((mouse.column, mouse.row), (u16::MAX, 0));
        assert_eq!(mouse.modifiers, Modifiers::SHIFT | Modifiers::ALT);
    }
}
#[test]
fn paste_is_moved_intact_and_focus_resize_are_delivered() {
    let paste = String::from("茶\r\n\x1b[31m");
    let pointer = paste.as_ptr();
    let TerminalEvent::Input(Input::Paste(paste)) = translate_event(source::Event::Paste(paste))
    else {
        panic!("paste lost")
    };
    assert_eq!(paste.as_ptr(), pointer);
    assert_eq!(paste, "茶\r\n\x1b[31m");
    assert!(matches!(
        translate_event(source::Event::FocusGained),
        TerminalEvent::Input(Input::Focus(true))
    ));
    assert!(matches!(
        translate_event(source::Event::FocusLost),
        TerminalEvent::Input(Input::Focus(false))
    ));
    assert!(
        matches!(translate_event(source::Event::Resize(0,65535)),TerminalEvent::Resize(size) if size.width==0&&size.height==65535)
    );
}
