use matea::{
    prelude::*,
    terminal::{Capabilities, FrameEncoder},
};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

struct App {
    progress: Progress,
    now: Rc<Cell<Instant>>,
    reduced: bool,
    updates: usize,
}
impl App {
    fn theme(&self) -> Theme {
        Theme {
            reduced_motion: self.reduced,
            ..Theme::default()
        }
    }
}
impl Model for App {
    type Message = ClockTick;
    fn update(&mut self, event: Event<ClockTick>) -> Effect<ClockTick> {
        match event {
            Event::Message(tick) => {
                self.updates += usize::from(self.progress.update(tick).unwrap());
            }
            Event::Input(Input::Key(key)) if key.is_char('m') => self.reduced = !self.reduced,
            Event::Input(Input::Key(key)) if key.is_char('r') => {
                self.progress.set_percent_at(0.3, self.now.get()).unwrap();
            }
            _ => {}
        }
        Effect::none()
    }
    fn subscriptions(&self) -> Vec<Subscription<ClockTick>> {
        let now = self.now.clone();
        self.progress
            .schedule(&self.theme())
            .map(|s| s.subscription_with_clock(move || now.get(), |tick| tick))
            .into_iter()
            .collect()
    }
    fn view(&self, _: ViewContext<'_>) -> View {
        self.progress.view(&self.theme())
    }
}
#[test]
fn runtime_retargets_and_cancels_reduced_motion_subscriptions() {
    let origin = Instant::now();
    let now = Rc::new(Cell::new(origin));
    let mut progress = Progress::new_at(origin);
    progress.set_percent_at(0.7, origin).unwrap();
    let app = App {
        progress,
        now: now.clone(),
        reduced: false,
        updates: 0,
    };
    let mut runtime = Runtime::new(app, Size::new(40, 1), Theme::default()).unwrap();
    let key = |c| Event::Input(Input::Key(Key::new(KeyCode::Char(c))));
    now.set(origin + Duration::from_millis(100));
    runtime.advance(Duration::from_millis(100)).unwrap();
    assert_eq!(runtime.model().updates, 1);
    runtime.dispatch(key('r')).unwrap();
    assert_eq!(runtime.model().progress.target(), 0.3);
    now.set(origin + Duration::from_millis(200));
    runtime.advance(Duration::from_millis(100)).unwrap();
    assert_eq!(runtime.model().updates, 2);
    let state = runtime.model().progress.motion();
    runtime.dispatch(key('m')).unwrap();
    now.set(origin + Duration::from_millis(2200));
    assert!(!runtime.advance(Duration::from_secs(2)).unwrap());
    assert_eq!(runtime.model().progress.motion(), state);
    assert!(runtime.frame().unwrap().plain().ends_with("  30%"));
    runtime.dispatch(key('m')).unwrap();
    now.set(origin + Duration::from_millis(2300));
    runtime.advance(Duration::from_millis(100)).unwrap();
    assert_eq!(runtime.model().updates, 3);
}
#[test]
fn structured_progress_preserves_geometry_and_terminal_color_unicode_fallback() {
    let mut progress = Progress::new();
    progress.set_width(17);
    progress.fill = ProgressFill::Gradient {
        gradient: Gradient::new(&[[90, 86, 224], [238, 111, 248]]).unwrap(),
        scaled: true,
    };
    let view = progress.view_as(0.5, &Theme::default()).unwrap();
    for width in [0, 1, 5, 17, 80] {
        let frame = view.render(Size::new(width, 1)).unwrap();
        assert_eq!(frame.size(), Size::new(width, 1));
        for color in [
            ColorLevel::None,
            ColorLevel::Ansi16,
            ColorLevel::Ansi256,
            ColorLevel::TrueColor,
        ] {
            for unicode in [false, true] {
                let mut encoded = Vec::new();
                FrameEncoder::default()
                    .write(
                        &mut encoded,
                        &frame,
                        None,
                        Capabilities {
                            color,
                            unicode,
                            reduced_motion: true,
                        },
                    )
                    .unwrap();
                let text = std::str::from_utf8(&encoded).unwrap();
                if !unicode {
                    assert!(text.is_ascii());
                }
                if color != ColorLevel::TrueColor {
                    assert!(!text.contains("38;2;"));
                }
                if matches!(color, ColorLevel::None | ColorLevel::Ansi16) {
                    assert!(!text.contains("38;5;"));
                }
            }
        }
    }
}
