use matea::{
    motion::{Motion, Spring},
    prelude::*,
};
use std::time::Duration;
mod support;
struct Animation {
    state: Motion,
    target: f64,
    spring: Spring,
}
impl Model for Animation {
    type Message = ();
    fn subscriptions(&self) -> Vec<Subscription<()>> {
        vec![Subscription::every(
            "motion",
            Duration::from_millis(16),
            || (),
        )]
    }
    fn update(&mut self, event: Event<()>) -> Effect<()> {
        match event {
            Event::Message(()) => {
                self.state = self
                    .spring
                    .step(self.state, self.target)
                    .expect("bounded demo state")
            }
            Event::Input(Input::Key(key)) if key.is_char(' ') => self.target = 20.0 - self.target,
            Event::Input(Input::Key(key)) if key.is_char('q') => return Effect::quit(),
            _ => {}
        }
        Effect::none()
    }
    fn view(&self, c: ViewContext<'_>) -> View {
        let position = if c.theme.reduced_motion {
            self.target
        } else {
            self.state.position
        };
        View::column([
            View::styled(
                format!(
                    "{}●",
                    " ".repeat(position.round().clamp(0.0, 30.0) as usize)
                ),
                c.theme.accent,
            ),
            View::text("space: change target | q: quit"),
        ])
        .panel("Matea motion", c.theme)
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    support::launch(Animation {
        state: Motion::default(),
        target: 20.0,
        spring: Spring::new(Duration::from_millis(16), 12.0, 0.7).expect("valid demo parameters"),
    })
}
