//! A separately compiled component using only public Matea contracts.
use matea::prelude::*;
struct Badge {
    enabled: bool,
}
impl Component for Badge {
    type Message = bool;
    fn event(&mut self, input: &Input, focused: bool) -> Response<bool> {
        if focused && matches!(input, Input::Key(k) if k.is_char(' ')) {
            self.enabled = !self.enabled;
            Response::consumed(Some(self.enabled))
        } else {
            Response::ignored()
        }
    }
    fn view(&self, theme: &Theme, focused: bool) -> View {
        View::styled(
            format_args!(
                "[{}] Third-party badge",
                if self.enabled { "x" } else { " " }
            ),
            if focused { theme.focus } else { theme.text },
        )
    }
}
struct App(Badge);
impl Model for App {
    type Message = ();
    fn update(&mut self, event: Event<()>) -> Effect<()> {
        if let Event::Input(input) = event {
            if matches!(&input, Input::Key(k) if k.is_char('q')) {
                return Effect::quit();
            }
            self.0.event(&input, true);
        }
        Effect::none()
    }
    fn view(&self, c: ViewContext<'_>) -> View {
        self.0
            .view(c.theme, true)
            .panel("Independent extension", c.theme)
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    let app = App(Badge { enabled: false });
    if std::env::args().any(|s| s == "--snapshot") {
        let r = Runtime::new(app, Size::new(40, 7), Theme::plain())?;
        println!("{}", r.frame()?.plain());
        Ok(())
    } else {
        matea::run(app)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extension_routes_input_and_renders_without_internals() {
        let mut badge = Badge { enabled: false };
        let space = Input::Key(Key::new(KeyCode::Char(' ')));
        assert!(!badge.event(&space, false).consumed);
        assert_eq!(badge.event(&space, true).message, Some(true));
        assert!(
            badge
                .view(&Theme::plain(), true)
                .render(Size::new(40, 1))
                .unwrap()
                .plain()
                .contains("[x]")
        );
    }
}
