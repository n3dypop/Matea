use matea::{
    prelude::*,
    widgets::{PageDisplay, Pagination, Paginator},
};
use std::num::NonZeroUsize;

struct Pages {
    paginator: Paginator,
    teas: [&'static str; 10],
}
impl Default for Pages {
    fn default() -> Self {
        Self {
            paginator: Paginator::new(Pagination::new(10, NonZeroUsize::new(3).unwrap()))
                .with_formatter(|n| format!("Page {} of {}", n.current, n.total)),
            teas: [
                "Matcha",
                "Sencha",
                "Gyokuro",
                "Hojicha",
                "Genmaicha",
                "Kukicha",
                "Bancha",
                "Kabusecha",
                "Tamaryokucha",
                "Shincha",
            ],
        }
    }
}
impl Model for Pages {
    type Message = ();
    fn update(&mut self, event: Event<()>) -> Effect<()> {
        if let Event::Input(input) = event {
            if let Input::Key(key) = &input {
                if key.is_char('q') || key.is_plain() && key.code == KeyCode::Esc {
                    return Effect::quit();
                }
                if key.is_char('d') {
                    self.paginator.display = match self.paginator.display {
                        PageDisplay::Arabic => PageDisplay::Dots,
                        PageDisplay::Dots => PageDisplay::Arabic,
                    };
                }
            }
            self.paginator.event(&input, true);
        }
        Effect::none()
    }
    fn view(&self, c: ViewContext<'_>) -> View {
        View::column([
            View::styled("Matea tea pages", c.theme.accent),
            View::column(
                self.teas[self.paginator.pages.range()]
                    .iter()
                    .copied()
                    .map(View::text),
            ),
            self.paginator.view(c.theme, true),
            View::styled("left/h right/l: page | d: dots | q: exit", c.theme.muted),
        ])
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    if std::env::args().any(|s| s == "--snapshot") {
        println!(
            "{}",
            Runtime::new(Pages::default(), Size::new(72, 24), Theme::plain())?
                .frame()?
                .plain()
                .trim_end()
        );
        Ok(())
    } else {
        matea::run(Pages::default())
    }
}
