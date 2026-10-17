use matea_core::{Effect, Event, Input, Model, Runtime, Subscription, ViewContext};
use matea_render::{Size, View};
use matea_style::Theme;
use std::time::Duration;

struct App {
    invalid: bool,
    calls: Vec<&'static str>,
}
impl Model for App {
    type Message = &'static str;
    fn update(&mut self, event: Event<Self::Message>) -> Effect<Self::Message> {
        match event {
            Event::Input(Input::Focus(value)) => self.invalid = value,
            Event::Message(message) => self.calls.push(message),
            _ => {}
        }
        Effect::none()
    }
    fn subscriptions(&self) -> Vec<Subscription<Self::Message>> {
        let second = if self.invalid { "a" } else { "b" };
        vec![
            Subscription::every("a", Duration::from_secs(1), || "a"),
            Subscription::every(second, Duration::from_secs(1), move || second),
        ]
    }
    fn view(&self, _: ViewContext<'_>) -> View {
        View::text("")
    }
}
#[test]
fn rejected_duplicate_configuration_does_not_lose_existing_deadlines() {
    let mut runtime = Runtime::new(
        App {
            invalid: false,
            calls: vec![],
        },
        Size::new(1, 1),
        Theme::plain(),
    )
    .unwrap();
    runtime.advance(Duration::from_millis(500)).unwrap();
    assert!(runtime.dispatch(Event::Input(Input::Focus(true))).is_err());
    runtime.dispatch(Event::Input(Input::Focus(false))).unwrap();
    runtime.advance(Duration::from_millis(500)).unwrap();
    assert_eq!(runtime.model().calls, ["a", "b"]);
}
