use matea_core::{Effect, Event, Model, Runtime, Subscription, ViewContext};
use matea_render::{Size, View};
use matea_style::{Color, Style, TextAttribute, Theme};
use matea_widgets::{ClockTick, CursorMode, TextInput, TickInterval, VirtualCursor};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

fn period(ms: u64) -> TickInterval {
    TickInterval::new(Duration::from_millis(ms)).unwrap()
}

#[test]
fn cursor_text_is_inline_before_grapheme_clipping() {
    let mut cursor = VirtualCursor::new();
    for (input, expected) in [
        ("a\nb", "ab"),
        ("a\r\nb", "ab"),
        ("a\tb", "a    b"),
        ("抹\n茶\r\ne\u{301}\t👩‍💻", "抹茶e\u{301}    👩‍💻"),
        ("\n\r\n", ""),
    ] {
        cursor.set_char(input);
        assert_eq!(cursor.text(), expected);
        let frame = cursor
            .view(&Theme::plain())
            .render(Size::new(24, 2))
            .unwrap();
        assert_eq!(frame.plain(), format!("{expected}\n"));
        assert!(frame.row(1).iter().all(|cell| cell.symbol() == " "));
    }
    cursor.set_char("抹\n茶");
    assert_eq!(
        cursor
            .view(&Theme::plain())
            .render(Size::new(2, 1))
            .unwrap()
            .plain(),
        "抹"
    );
}

#[test]
fn modes_focus_and_styles_follow_the_reference_contract() {
    let normal = Theme::default();
    let reduced = Theme::plain();
    let origin = Instant::now();
    let mut cursor = VirtualCursor::new_at(origin);
    assert_eq!(cursor.interval().get(), Duration::from_millis(530));
    assert_eq!(cursor.mode(), CursorMode::Blink);
    assert!(!cursor.focused());
    assert!(cursor.blinked());
    assert!(cursor.schedule(&normal).is_none());
    assert_eq!(cursor.text(), "");
    let style = Style::default().fg(Color::Indexed(3));
    let text_style = Style::default().underline();
    cursor.style = Some(style);
    cursor.text_style = Some(text_style);
    cursor.set_char("抹e\u{301}");
    for (mode, name) in [
        (CursorMode::Blink, "blink"),
        (CursorMode::Static, "static"),
        (CursorMode::Hidden, "hidden"),
    ] {
        assert_eq!(mode.to_string(), name);
        for focused in [false, true] {
            assert!(cursor.set_mode_at(mode, origin));
            if focused {
                cursor.focus_at(origin);
            } else {
                cursor.blur_at(origin);
            }
            let visible = focused && mode != CursorMode::Hidden;
            assert_eq!(cursor.visible(&normal), visible);
            assert_eq!(cursor.visible(&reduced), visible);
            assert_eq!(cursor.blinked(), !visible);
            assert_eq!(
                cursor.schedule(&normal).is_some(),
                focused && mode == CursorMode::Blink
            );
            assert!(cursor.schedule(&reduced).is_none());
            let frame = cursor.view(&normal).render(Size::new(3, 1)).unwrap();
            assert_eq!(frame.plain(), "抹e\u{301}");
            assert_eq!(
                frame.row(0)[0].style(),
                if visible { style.reverse() } else { text_style }
            );
        }
    }
}

#[test]
fn phase_coalesces_elapsed_periods_and_rejects_duplicate_foreign_and_backward_ticks() {
    let origin = Instant::now();
    let at = |ms| origin + Duration::from_millis(ms);
    let normal = Theme::default();
    let mut cursor = VirtualCursor::new_at(origin);
    cursor.set_interval_at(period(10), origin);
    cursor.focus_at(origin);
    let schedule = cursor.schedule(&normal).unwrap();
    let mut other = VirtualCursor::new_at(origin);
    other.focus_at(origin);
    assert_ne!(cursor.id(), other.id());
    assert!(!cursor.update(other.schedule(&normal).unwrap().tick_at(at(10))));
    for (ms, blinked) in [
        (1, false),
        (9, false),
        (10, true),
        (20, false),
        (59, true),
        (60, false),
    ] {
        let tick = schedule.tick_at(at(ms));
        assert!(cursor.update(tick));
        assert_eq!(cursor.blinked(), blinked, "{ms}");
        assert!(!cursor.update(tick));
        assert!(!cursor.update(schedule.tick_at(at(ms - 1))));
    }
    assert!(!cursor.set_mode_at(CursorMode::Hidden, at(59)));
    assert!(!cursor.set_interval_at(period(7), at(59)));
    assert!(!cursor.focus_at(at(59)));
    assert!(!cursor.blur_at(at(59)));
    assert!(!cursor.restart_at(at(59)));
    assert_eq!(cursor.mode(), CursorMode::Blink);
    assert_eq!(cursor.interval(), period(10));
    assert!(cursor.focused());
    assert!(cursor.update(schedule.tick_at(at(70))));
    assert!(cursor.blinked());
    assert!(cursor.visible(&Theme::plain()));
}

#[test]
fn every_control_invalidates_queued_ticks_even_for_same_timestamp_or_value() {
    let normal = Theme::default();
    let origin = Instant::now();
    let later = origin + Duration::from_secs(1);
    let mut cursor = VirtualCursor::new_at(origin);
    cursor.focus_at(origin);
    for control in 0..6 {
        let old = cursor.schedule(&normal).unwrap();
        match control {
            0 => {
                cursor.focus_at(origin);
            }
            1 => {
                cursor.restart_at(origin);
            }
            2 => {
                cursor.set_interval_at(cursor.interval(), origin);
            }
            3 => {
                cursor.set_mode_at(CursorMode::Blink, origin);
            }
            4 => {
                cursor.set_mode_at(CursorMode::Static, origin);
                cursor.set_mode_at(CursorMode::Blink, origin);
            }
            _ => {
                cursor.blur_at(origin);
                cursor.focus_at(origin);
            }
        }
        assert!(!cursor.update(old.tick_at(later)), "control={control}");
        assert!(cursor.visible(&normal));
    }
    cursor.blur_at(origin);
    assert!(!cursor.restart_at(origin));
    assert!(cursor.schedule(&normal).is_none());
    cursor.focus_at(origin);
    cursor.set_mode_at(CursorMode::Static, origin);
    assert!(!cursor.restart_at(origin));
}

#[test]
fn text_input_composition_preserves_graphemes_passwords_suggestions_and_native_cursor() {
    let origin = Instant::now();
    let normal = Theme::default();
    let mut cursor = VirtualCursor::new_at(origin);
    cursor.focus_at(origin);
    for value in ["abc", "抹茶", "e\u{301}👩‍💻"] {
        let mut input = TextInput::new(value);
        input.set_cursor(0);
        for echo in [
            matea_widgets::EchoMode::Normal,
            matea_widgets::EchoMode::Password('*'),
            matea_widgets::EchoMode::Hidden,
        ] {
            input.echo = echo;
            let mut views = vec![];
            for mode in [CursorMode::Static, CursorMode::Hidden] {
                cursor.set_mode_at(mode, origin);
                let frame = input
                    .view_with_cursor(&normal, true, &cursor)
                    .render(Size::new(24, 1))
                    .unwrap();
                assert_eq!(
                    frame.row(0)[0].style(),
                    if mode == CursorMode::Static {
                        input.styles(&normal).cursor
                    } else {
                        input.styles(&normal).focused.text
                    }
                );
                views.push(frame.plain());
            }
            assert_eq!(views[0], views[1]);
        }
        input.set_virtual_cursor(false);
        assert!(
            input
                .view_with_cursor(&normal, true, &cursor)
                .render(Size::new(24, 1))
                .unwrap()
                .cursor()
                .is_some()
        );
    }
    let mut input = TextInput::new("t");
    input.set_suggestions(["tea"]);
    input.show_suggestions = true;
    let mut styles = input.styles(&normal);
    styles.focused.suggestion = Style::default().underline();
    input.set_styles(styles);
    let frame = input
        .view_with_cursor(&normal, true, &cursor)
        .render(Size::new(24, 1))
        .unwrap();
    assert_eq!(frame.plain(), "tea");
    assert_eq!(frame.row(0)[1].style(), styles.focused.suggestion);
    cursor.set_char("a\u{1b}[31m\n抹茶");
    let frame = cursor.view(&normal).render(Size::new(3, 1)).unwrap();
    assert!(!frame.plain().contains('\u{1b}'));
    assert_eq!(frame.size(), Size::new(3, 1));
}

enum Message {
    Tick(ClockTick),
    Focus(bool),
    Mode(CursorMode),
    Reduced(bool),
}

struct Editor {
    cursor: VirtualCursor,
    now: Rc<Cell<Instant>>,
    theme: Theme,
    updates: usize,
}
impl Model for Editor {
    type Message = Message;
    fn update(&mut self, event: Event<Message>) -> Effect<Message> {
        match event {
            Event::Message(Message::Tick(tick)) => {
                self.updates += usize::from(self.cursor.update(tick))
            }
            Event::Message(Message::Focus(true)) => {
                self.cursor.focus_at(self.now.get());
            }
            Event::Message(Message::Focus(false)) => {
                self.cursor.blur_at(self.now.get());
            }
            Event::Message(Message::Mode(mode)) => {
                self.cursor.set_mode_at(mode, self.now.get());
            }
            Event::Message(Message::Reduced(reduced)) => self.theme.reduced_motion = reduced,
            _ => {}
        }
        Effect::none()
    }
    fn subscriptions(&self) -> Vec<Subscription<Message>> {
        self.cursor
            .schedule(&self.theme)
            .map(|schedule| {
                let now = self.now.clone();
                schedule.subscription_with_clock(move || now.get(), Message::Tick)
            })
            .into_iter()
            .collect()
    }
    fn view(&self, _: ViewContext<'_>) -> View {
        self.cursor.view(&self.theme)
    }
}

#[test]
fn runtime_reconciles_blur_mode_reduced_motion_and_delayed_notifications() {
    let origin = Instant::now();
    let now = Rc::new(Cell::new(origin));
    let mut cursor = VirtualCursor::new_at(origin);
    cursor.set_char("x");
    cursor.set_interval_at(period(10), origin);
    cursor.focus_at(origin);
    let mut runtime = Runtime::new(
        Editor {
            cursor,
            now: now.clone(),
            theme: Theme::default(),
            updates: 0,
        },
        Size::new(1, 1),
        Theme::default(),
    )
    .unwrap();
    now.set(origin + Duration::from_millis(35));
    runtime.advance(Duration::from_millis(35)).unwrap();
    assert_eq!(runtime.model().updates, 1);
    assert!(
        !runtime.frame().unwrap().row(0)[0]
            .style()
            .has(TextAttribute::Reverse)
    );
    runtime
        .dispatch(Event::Message(Message::Focus(false)))
        .unwrap();
    now.set(origin + Duration::from_millis(45));
    assert!(!runtime.advance(Duration::from_millis(10)).unwrap());
    assert_eq!(runtime.model().updates, 1);
    runtime
        .dispatch(Event::Message(Message::Focus(true)))
        .unwrap();
    now.set(origin + Duration::from_millis(55));
    runtime.advance(Duration::from_millis(10)).unwrap();
    assert_eq!(runtime.model().updates, 2);
    runtime
        .dispatch(Event::Message(Message::Mode(CursorMode::Static)))
        .unwrap();
    now.set(origin + Duration::from_millis(155));
    assert!(!runtime.advance(Duration::from_millis(100)).unwrap());
    assert!(
        runtime.frame().unwrap().row(0)[0]
            .style()
            .has(TextAttribute::Reverse)
    );
    runtime
        .dispatch(Event::Message(Message::Mode(CursorMode::Blink)))
        .unwrap();
    now.set(origin + Duration::from_millis(165));
    runtime.advance(Duration::from_millis(10)).unwrap();
    assert_eq!(runtime.model().updates, 3);
    runtime
        .dispatch(Event::Message(Message::Reduced(true)))
        .unwrap();
    now.set(origin + Duration::from_millis(265));
    assert!(!runtime.advance(Duration::from_millis(100)).unwrap());
    assert!(
        runtime.frame().unwrap().row(0)[0]
            .style()
            .has(TextAttribute::Reverse)
    );
    runtime
        .dispatch(Event::Message(Message::Reduced(false)))
        .unwrap();
    now.set(origin + Duration::from_millis(275));
    runtime.advance(Duration::from_millis(10)).unwrap();
    assert_eq!(runtime.model().updates, 4);
    assert!(
        runtime.frame().unwrap().row(0)[0]
            .style()
            .has(TextAttribute::Reverse)
    );
}
