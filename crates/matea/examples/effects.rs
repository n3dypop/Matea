//! Typed background completion, cancellation and timer reconciliation.
use matea::prelude::*;
use std::time::Duration;
mod support;
enum Message {
    Loaded(String),
    Tick,
}
struct App {
    status: String,
    pending: bool,
    spinner: Spinner,
    motion: bool,
}
fn load() -> Effect<Message> {
    Effect::perform("tea", |token| {
        if token.wait(Duration::from_millis(150)) {
            return Err("cancelled".into());
        }
        Ok(Message::Loaded("Tea is ready".into()))
    })
}
impl Model for App {
    type Message = Message;
    fn init(&mut self) -> Effect<Message> {
        load()
    }
    fn update(&mut self, event: Event<Message>) -> Effect<Message> {
        match event {
            Event::Message(Message::Loaded(status)) => {
                self.status = status;
                self.pending = false;
            }
            Event::Message(Message::Tick) => self.spinner.tick(&Theme::default()),
            Event::Failure(f) => {
                self.status = format!("Error: {}", f.message);
                self.pending = false;
            }
            Event::Input(Input::Key(k)) => {
                if k.is_char('q') {
                    return Effect::quit();
                }
                if k.is_char('c') {
                    self.pending = false;
                    self.status = "Cancelled".into();
                    return Effect::cancel("tea");
                }
                if k.is_char('r') {
                    self.pending = true;
                    self.status = "Loading tea...".into();
                    return load();
                }
            }
            _ => {}
        }
        Effect::none()
    }
    fn subscriptions(&self) -> Vec<Subscription<Message>> {
        if self.pending && self.motion {
            vec![Subscription::every(
                "loading",
                Duration::from_millis(75),
                || Message::Tick,
            )]
        } else {
            vec![]
        }
    }
    fn view(&self, c: ViewContext<'_>) -> View {
        let mut lines = vec![View::styled(&self.status, c.theme.accent)];
        if self.pending {
            lines.push(self.spinner.view(c.theme, false));
        }
        lines.push(View::styled(
            "r: reload | c: cancel | q: quit",
            c.theme.muted,
        ));
        View::column(lines).panel("Typed effects", c.theme)
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    support::launch(App {
        status: "Loading tea...".into(),
        pending: true,
        spinner: Spinner::default().with_label("Working"),
        motion: !matea::terminal::Capabilities::detect().reduced_motion,
    })
}
