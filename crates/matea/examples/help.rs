use matea::prelude::*;

struct Keys {
    help: Help,
    toggle: Binding,
    action: Binding,
    quit: Binding,
}
impl Default for Keys {
    fn default() -> Self {
        Self {
            help: Help::default(),
            toggle: Binding::new([Key::new(KeyCode::Char('?'))], "?", "toggle help"),
            action: Binding::new([Key::new(KeyCode::Char('d'))], "d", "toggle tea action"),
            quit: Binding::new([Key::new(KeyCode::Char('q'))], "q", "quit"),
        }
    }
}
impl Model for Keys {
    type Message = ();
    fn update(&mut self, event: Event<()>) -> Effect<()> {
        match event {
            Event::Resize(size) => self.help.width = Some(size.width),
            Event::Input(Input::Key(key)) if self.quit.matches(key) => return Effect::quit(),
            Event::Input(Input::Key(key)) if self.toggle.matches(key) => {
                self.help.show_all = !self.help.show_all;
            }
            Event::Input(Input::Key(key)) if key.is_char('d') => {
                self.action.enabled = !self.action.enabled;
            }
            _ => {}
        }
        Effect::none()
    }
    fn view(&self, c: ViewContext<'_>) -> View {
        let first = [HelpItem::from(&self.toggle), HelpItem::from(&self.action)];
        let last = [HelpItem::from(&self.quit)];
        let short = [first[0], first[1], last[0]];
        View::column([
            View::styled("Matea key help", c.theme.accent),
            View::styled(
                "? expands help; d changes the enabled binding",
                c.theme.muted,
            ),
            self.help.view(HelpMap {
                short: &short,
                full: &[&first, &last],
            }),
        ])
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    if std::env::args().any(|s| s == "--snapshot") {
        println!(
            "{}",
            Runtime::new(Keys::default(), Size::new(64, 6), Theme::plain())?
                .frame()?
                .plain()
                .trim_end()
        );
        Ok(())
    } else {
        matea::run(Keys::default())
    }
}
