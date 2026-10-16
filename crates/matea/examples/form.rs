use matea::prelude::*;
mod support;
struct App {
    form: Form,
    status: String,
}
impl Model for App {
    type Message = ();
    fn update(&mut self, event: Event<()>) -> Effect<()> {
        if let Event::Input(input) = event {
            if matches!(&input, Input::Key(k) if (k.is_plain() && k.code == KeyCode::Esc)) {
                return Effect::quit();
            }
            if self.form.event(&input, true).message == Some(Action::Submit) {
                self.status = "Saved. Esc to close.".into();
            }
        }
        Effect::none()
    }
    fn view(&self, c: ViewContext<'_>) -> View {
        View::column([
            self.form.view(c.theme, true),
            View::styled(&self.status, c.theme.muted),
        ])
        .gap(c.theme.gap)
        .panel("Tea tasting registration", c.theme)
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    let mut tea = Field::new("Favorite tea", required);
    tea.input.placeholder = "Matcha, Sencha, Hojicha".into();
    tea.input.show_suggestions = true;
    tea.input.set_suggestions(["Matcha", "Sencha", "Hojicha"]);
    let mut name = Field::new("Name", required);
    if std::env::args().any(|arg| arg == "--native") {
        name.input.set_virtual_cursor(false);
        tea.input.set_virtual_cursor(false);
    }
    support::launch(App {
        form: Form::new(vec![name, tea]),
        status: "Tab: complete/focus | Shift-Tab: back | Enter: next/submit | Esc: quit".into(),
    })
}
