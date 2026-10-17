use matea_core::{Effect, Event, Input, Model, Runtime, Subscription, SubscriptionId, ViewContext};
use matea_render::{Size, View};
use matea_style::Theme;
use std::time::Duration;

struct Alpha;
struct Beta;
struct ScopedApp {
    revision: u64,
    value: u64,
    calls: Vec<u64>,
}
impl Model for ScopedApp {
    type Message = u64;
    fn update(&mut self, event: Event<u64>) -> Effect<u64> {
        match event {
            Event::Input(Input::Focus(restart)) => {
                self.value += 1;
                if restart {
                    self.revision += 1;
                }
            }
            Event::Message(value) => self.calls.push(value),
            _ => {}
        }
        Effect::none()
    }
    fn subscriptions(&self) -> Vec<Subscription<u64>> {
        let value = self.value;
        let id = SubscriptionId::scoped::<Alpha>(7, self.revision, 0);
        vec![
            Subscription::every_with_id(id, Duration::from_secs(1), move || value),
            Subscription::every_with_id(
                SubscriptionId::scoped::<Beta>(7, 0, 0),
                Duration::from_secs(1),
                || 10,
            ),
            Subscription::every(
                SubscriptionId::scoped::<Alpha>(7, 0, 0).to_string(),
                Duration::from_secs(1),
                || 100,
            ),
        ]
    }
    fn view(&self, _: ViewContext<'_>) -> View {
        View::text("")
    }
}
#[test]
fn namespaces_and_named_text_are_distinct_and_revision_controls_callback_replacement() {
    let id = SubscriptionId::scoped::<Alpha>(7, 0, 0);
    assert_eq!(id.clone(), id);
    assert_ne!(id, SubscriptionId::scoped::<Beta>(7, 0, 0));
    assert_ne!(id, SubscriptionId::scoped::<Alpha>(7, 0, 1));
    assert_ne!(id, SubscriptionId::scoped::<Alpha>(8, 0, 0));
    assert_ne!(id, SubscriptionId::named(id.to_string()));
    let mut r = Runtime::new(
        ScopedApp {
            revision: 0,
            value: 1,
            calls: vec![],
        },
        Size::new(1, 1),
        Theme::plain(),
    )
    .unwrap();
    r.advance(Duration::from_millis(500)).unwrap();
    r.dispatch(Event::Input(Input::Focus(false))).unwrap();
    r.advance(Duration::from_millis(500)).unwrap();
    let mut calls = r.model().calls.clone();
    calls.sort_unstable();
    assert_eq!(calls, [1, 10, 100]); // unchanged key retains the original closure
    r.advance(Duration::from_millis(250)).unwrap();
    r.dispatch(Event::Input(Input::Focus(true))).unwrap();
    r.advance(Duration::from_millis(750)).unwrap();
    assert_eq!(r.model().calls.len(), 5); // only the unchanged Beta/named deadlines
    r.advance(Duration::from_millis(250)).unwrap();
    assert_eq!(r.model().calls.last(), Some(&3));
}

struct InvalidApp {
    invalid: bool,
    too_many: bool,
    calls: usize,
}
impl Model for InvalidApp {
    type Message = ();
    fn update(&mut self, event: Event<()>) -> Effect<()> {
        match event {
            Event::Input(Input::Focus(invalid)) => self.invalid = invalid,
            Event::Message(()) => self.calls += 1,
            _ => {}
        }
        Effect::none()
    }
    fn subscriptions(&self) -> Vec<Subscription<()>> {
        if self.invalid && self.too_many {
            return (0..65)
                .map(|i| {
                    Subscription::every_with_id(
                        SubscriptionId::scoped::<Alpha>(i, 0, 0),
                        Duration::from_secs(1),
                        || (),
                    )
                })
                .collect();
        }
        vec![
            Subscription::every("a", Duration::from_secs(1), || ()),
            Subscription::every(
                "b",
                if self.invalid {
                    Duration::ZERO
                } else {
                    Duration::from_secs(1)
                },
                || (),
            ),
        ]
    }
    fn view(&self, _: ViewContext<'_>) -> View {
        View::text("")
    }
}
#[test]
fn zero_period_and_limit_errors_preserve_all_preexisting_deadlines() {
    for too_many in [false, true] {
        let mut r = Runtime::new(
            InvalidApp {
                invalid: false,
                too_many,
                calls: 0,
            },
            Size::new(1, 1),
            Theme::plain(),
        )
        .unwrap();
        r.advance(Duration::from_millis(500)).unwrap();
        let error = r.dispatch(Event::Input(Input::Focus(true))).unwrap_err();
        assert!(
            error
                .to_string()
                .contains(if too_many { "limit" } else { "nonzero" })
        );
        r.dispatch(Event::Input(Input::Focus(false))).unwrap();
        r.advance(Duration::from_millis(500)).unwrap();
        assert_eq!(r.model().calls, 2);
    }
}
