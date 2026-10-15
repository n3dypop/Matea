use matea::prelude::*;
use std::time::Duration;
mod support;
struct Dashboard {
    table: Table,
    notes: Viewport,
    spinner: Spinner,
    ticks: u64,
    focus: FocusRing,
    motion: bool,
}
impl Model for Dashboard {
    type Message = ();
    fn update(&mut self, event: Event<()>) -> Effect<()> {
        match event {
            Event::Input(input) => {
                if matches!(&input, Input::Key(k) if k.is_char('q') || (k.is_plain() && k.code == KeyCode::Esc))
                {
                    return Effect::quit();
                }
                if !self.focus.handle(&input) {
                    self.table
                        .event(&input, self.focus.current() == Some("table"));
                    self.notes
                        .event(&input, self.focus.current() == Some("notes"));
                }
            }
            Event::Message(()) => {
                self.ticks += 1;
                self.spinner.tick(&Theme::default());
            }
            _ => {}
        }
        Effect::none()
    }
    fn subscriptions(&self) -> Vec<Subscription<()>> {
        if self.motion {
            vec![Subscription::every(
                "spinner",
                Duration::from_millis(150),
                || (),
            )]
        } else {
            vec![]
        }
    }
    fn view(&self, c: ViewContext<'_>) -> View {
        View::column([
            View::styled("Matea / Tea studio", c.theme.accent),
            View::row([
                self.table
                    .view(c.theme, self.focus.current() == Some("table"))
                    .panel("Collection", c.theme),
                self.notes
                    .view(c.theme, self.focus.current() == Some("notes"))
                    .panel("Tasting notes", c.theme),
            ])
            .gap(c.theme.gap),
            self.spinner.view(c.theme, false),
            View::styled(
                "Tab: focus | arrows/PageUp/PageDown: navigate | q: quit",
                c.theme.muted,
            ),
        ])
        .gap(c.theme.gap)
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    let snapshot = std::env::args().any(|s| s == "--snapshot");
    let reduced = matea::terminal::Capabilities::detect().reduced_motion;
    support::launch(Dashboard {
        table: Table::new(
            vec!["Tea".into(), "Origin".into()],
            vec![
                vec!["Matcha".into(), "Uji".into()],
                vec!["Sencha".into(), "Shizuoka".into()],
                vec!["Hojicha".into(), "Kyoto".into()],
            ],
        ),
        notes: Viewport::new(
            "Welcome to Matea.
Keyboard first.
Visible focus.
Consistent spacing.
Unicode: 抹茶
Colors are optional.
Resize safely.
Enjoy your tea.",
        ),
        spinner: Spinner::default(),
        ticks: 0,
        focus: FocusRing::new(["table", "notes"]),
        motion: !snapshot && !reduced,
    })
}
