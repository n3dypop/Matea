use matea_core::{
    Component, Effect, Event, Input, Key, KeyCode, Model, Runtime, Subscription, ViewContext,
};
use matea_render::{Size, View};
use matea_style::{Color, Style, Theme};
use matea_widgets::{Spinner, SpinnerAnimation, SpinnerPreset, TickInterval};
use std::{
    borrow::Cow,
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

struct AnimatedApp {
    spinner: Spinner,
    now: Rc<Cell<Instant>>,
    theme: Theme,
    updates: usize,
}
impl Model for AnimatedApp {
    type Message = matea_widgets::ClockTick;
    fn update(&mut self, event: Event<Self::Message>) -> Effect<Self::Message> {
        match event {
            Event::Message(tick) => {
                if self.spinner.update(tick) {
                    self.updates += 1;
                }
            }
            Event::Input(Input::Key(key)) if key.is_char('m') => {
                self.theme.reduced_motion = !self.theme.reduced_motion
            }
            Event::Input(_) => {
                if self.spinner.running() {
                    self.spinner.stop_at(self.now.get());
                } else {
                    self.spinner.start_at(self.now.get());
                }
            }
            _ => {}
        }
        Effect::none()
    }
    fn subscriptions(&self) -> Vec<Subscription<Self::Message>> {
        self.spinner
            .schedule(&self.theme)
            .map(|schedule| {
                let now = self.now.clone();
                schedule.subscription_with_clock(move || now.get(), |tick| tick)
            })
            .into_iter()
            .collect()
    }
    fn view(&self, _: ViewContext<'_>) -> View {
        self.spinner.view(&self.theme, false)
    }
}

#[test]
fn runtime_coalesces_animation_and_removes_paused_or_reduced_motion_schedules() {
    let origin = Instant::now();
    let now = Rc::new(Cell::new(origin));
    let mut spinner = Spinner::new_at(SpinnerPreset::Line.animation(), origin);
    spinner.start_at(origin);
    let mut runtime = Runtime::new(
        AnimatedApp {
            spinner,
            now: now.clone(),
            theme: Theme::default(),
            updates: 0,
        },
        Size::new(8, 1),
        Theme::plain(),
    )
    .unwrap();
    now.set(origin + Duration::from_millis(350));
    runtime.advance(Duration::from_millis(350)).unwrap();
    assert_eq!(runtime.model().spinner.frame_index(), 3);
    assert_eq!(runtime.model().updates, 1);
    let motion = || Event::Input(Input::Key(Key::new(KeyCode::Char('m'))));
    let pause = || Event::Input(Input::Key(Key::new(KeyCode::Char('p'))));
    runtime.dispatch(motion()).unwrap();
    now.set(origin + Duration::from_millis(2350));
    assert!(!runtime.advance(Duration::from_secs(2)).unwrap());
    assert_eq!(runtime.frame().unwrap().plain(), "*");
    runtime.dispatch(motion()).unwrap();
    now.set(origin + Duration::from_millis(2450));
    runtime.advance(Duration::from_millis(100)).unwrap();
    assert_eq!(runtime.model().spinner.frame_index(), 0);
    assert_eq!(runtime.model().updates, 2);
    runtime.dispatch(pause()).unwrap();
    now.set(origin + Duration::from_millis(4450));
    assert!(!runtime.advance(Duration::from_secs(2)).unwrap());
    runtime.dispatch(pause()).unwrap();
    now.set(origin + Duration::from_millis(4550));
    runtime.advance(Duration::from_millis(100)).unwrap();
    assert_eq!(runtime.model().updates, 3);
    assert_eq!(runtime.frame().unwrap().plain(), "/");
}

#[test]
fn all_preset_frames_and_intervals_match_pinned_data() {
    let names = [
        "line",
        "dot",
        "mini_dot",
        "jump",
        "pulse",
        "points",
        "globe",
        "moon",
        "monkey",
        "meter",
        "hamburger",
        "ellipsis",
    ];
    let mut counts = [0; 12];
    for line in include_str!("fixtures/spinners.tsv").lines() {
        let fields: Vec<_> = line.splitn(4, '\t').collect();
        let preset = names.iter().position(|name| *name == fields[0]).unwrap();
        let animation = SpinnerPreset::ALL[preset].animation();
        let index: usize = fields[2].parse().unwrap();
        assert_eq!(animation.frame(index), Some(fields[3]));
        assert_eq!(
            animation.interval().get().as_nanos(),
            fields[1].parse::<u128>().unwrap()
        );
        counts[preset] += 1;
    }
    assert_eq!(counts.iter().sum::<usize>(), 66);
    for (preset, count) in SpinnerPreset::ALL.into_iter().zip(counts) {
        let animation = preset.animation();
        assert_eq!(animation.len(), count);
        assert!(!animation.is_empty());
        assert_eq!(animation.frame(count), None);
    }
}

#[test]
fn custom_frames_are_validated_and_shared_without_cloning_live_state() {
    let interval = TickInterval::new(Duration::from_millis(2)).unwrap();
    assert!(SpinnerAnimation::new(Vec::<String>::new(), interval).is_err());
    let animation = SpinnerAnimation::new(["", "抹茶", "e\u{301}", "👩‍💻"], interval).unwrap();
    let other = animation.clone();
    assert_eq!(other.frame(0), Some(""));
    assert_eq!(other.frame(3), Some("👩‍💻"));
    let a = Spinner::new(animation);
    let b = Spinner::new(other);
    assert_ne!(a.id(), b.id());
    assert!(!a.running());
    assert_eq!(a.animation().interval(), interval);
}

#[test]
fn phase_uses_elapsed_time_and_controls_invalidate_old_schedules() {
    let origin = Instant::now();
    let at = |ms| origin + Duration::from_millis(ms);
    let theme = Theme::default();
    let mut spinner = Spinner::new_at(SpinnerPreset::Line.animation(), origin);
    assert!(spinner.schedule(&theme).is_none());
    assert!(spinner.start_at(origin));
    assert!(!spinner.start_at(origin));
    let first = spinner.schedule(&theme).unwrap();
    let tick = first.tick_at(at(100));
    assert!(spinner.update(tick));
    assert_eq!(spinner.frame_text(&theme), "/");
    assert!(!spinner.update(tick));
    assert!(!spinner.update(first.tick_at(at(99))));
    assert!(spinner.update(first.tick_at(at(650))));
    assert_eq!(spinner.frame_index(), 2);
    assert!(spinner.stop_at(at(675)));
    assert!(!spinner.stop_at(at(675)));
    assert!(spinner.schedule(&theme).is_none());
    assert!(spinner.start_at(at(1000)));
    assert!(!spinner.update(first.tick_at(at(1100))));
    let resumed = spinner.schedule(&theme).unwrap();
    assert!(spinner.update(resumed.tick_at(at(1025))));
    assert_eq!(spinner.frame_index(), 3); // partial pre-pause interval retained
    assert!(!spinner.reset_at(at(1024)));
    assert!(!spinner.set_animation_at(SpinnerPreset::Dot.animation(), at(1024)));
    assert_eq!(spinner.animation().len(), 4);
    assert!(spinner.set_animation_at(SpinnerPreset::Dot.animation(), at(1025)));
    assert!(spinner.running());
    assert_eq!(spinner.frame_index(), 0);
    assert!(!spinner.update(resumed.tick_at(at(1200))));
    let other = Spinner::new_at(SpinnerPreset::Moon.animation(), origin);
    assert_ne!(spinner.id(), other.id());
    let mut other = other;
    other.start_at(origin);
    assert!(!spinner.update(other.schedule(&theme).unwrap().tick_at(at(1300))));
}

#[test]
fn manual_cadence_reduced_motion_labels_and_style_are_independent() {
    let normal = Theme::default();
    let reduced = Theme::plain();
    let mut spinner = Spinner::default();
    assert!(matches!(spinner.text(&normal), Cow::Borrowed("|")));
    spinner.tick(&normal);
    assert_eq!(spinner.frame_text(&normal), "/");
    spinner.tick(&reduced);
    assert_eq!(spinner.frame_index(), 1);
    assert_eq!(spinner.frame_text(&reduced), "*");
    spinner.start();
    assert!(spinner.schedule(&reduced).is_none());
    assert!(spinner.subscription(&normal, |tick| tick).is_some());
    assert!(spinner.subscription(&reduced, |tick| tick).is_none());
    assert!(
        !spinner
            .event(&Input::Key(Key::new(KeyCode::Enter)), true)
            .consumed
    );
    let style = Style::default().fg(Color::Indexed(2));
    for (preset, expected) in [
        (SpinnerPreset::Line, "| Loading"),
        (SpinnerPreset::Dot, "⣾ Loading"),
        (SpinnerPreset::Ellipsis, "Loading"),
    ] {
        let spinner = Spinner::new(preset.animation())
            .with_label("Loading")
            .with_style(style);
        assert_eq!(spinner.text(&normal), expected);
        let surface = spinner
            .view(&normal, false)
            .render(Size::new(20, 1))
            .unwrap();
        assert_eq!(surface.plain(), expected);
        assert_eq!(surface.row(0)[0].style().fg, style.fg);
    }
    let custom =
        Spinner::new(SpinnerAnimation::new(["抹茶\u{1b}[31m"], TickInterval::default()).unwrap());
    let surface = custom.view(&normal, false).render(Size::new(2, 1)).unwrap();
    assert_eq!(surface.plain(), "抹");
    assert!(!surface.plain().contains('\u{1b}'));
}
