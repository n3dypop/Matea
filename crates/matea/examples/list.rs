use matea::prelude::*;
mod support;
struct App {
    search: TextInput,
    list: List,
    focus: FocusRing,
    status: String,
}
impl Model for App {
    type Message = ();
    fn update(&mut self, event: Event<()>) -> Effect<()> {
        if let Event::Input(input) = event {
            if matches!(&input, Input::Key(k) if k.code == KeyCode::Esc) {
                return Effect::quit();
            }
            if self.focus.handle(&input) {
                return Effect::none();
            }
            let search = self.focus.current() == Some("search");
            if self.search.event(&input, search).message == Some(Action::Changed) {
                self.list.filter(self.search.value());
            }
            if let Some(Action::Selected(index)) = self.list.event(&input, !search).message {
                self.status = format!("Selected item {}", index + 1);
            }
        }
        Effect::none()
    }
    fn view(&self, c: ViewContext<'_>) -> View {
        let search = self.focus.current() == Some("search");
        View::column([
            self.search.view(c.theme, search).panel("Search", c.theme),
            self.list
                .view(c.theme, !search)
                .panel("Tea collection", c.theme),
            View::styled(
                format!("{} | Tab: focus | arrows: select | Esc: quit", self.status),
                c.theme.muted,
            ),
        ])
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    support::launch(App {
        search: TextInput::default(),
        list: List::new(["Matcha", "Sencha", "Hojicha", "Genmaicha", "Oolong", "抹茶"]),
        focus: FocusRing::new(["search", "list"]),
        status: "Enter: choose".into(),
    })
}
