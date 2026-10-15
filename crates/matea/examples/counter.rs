use matea::prelude::*;
mod support;
#[derive(Default)]
struct Counter(i32);
impl Model for Counter {
    type Message = ();
    fn update(&mut self, event: Event<()>) -> Effect<()> {
        if let Event::Input(Input::Key(key)) = event {
            if key.is_char('+') || (key.is_plain() && key.code == KeyCode::Up) {
                self.0 += 1;
            }
            if key.is_char('-') || (key.is_plain() && key.code == KeyCode::Down) {
                self.0 -= 1;
            }
            if key.is_char('q') || (key.is_plain() && key.code == KeyCode::Esc) {
                return Effect::quit();
            }
        }
        Effect::none()
    }
    fn view(&self, c: ViewContext<'_>) -> View {
        View::column([
            View::styled(format!("Count: {}", self.0), c.theme.accent),
            View::styled("+/- or arrows: change | q: quit", c.theme.muted),
        ])
        .gap(c.theme.gap)
        .panel("Matea counter", c.theme)
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    support::launch(Counter::default())
}
