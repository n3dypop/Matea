use matea::prelude::*;
mod support;

struct Attributes {
    count: i32,
    focused: bool,
    blink: bool,
}
impl Model for Attributes {
    type Message = ();
    fn update(&mut self, event: Event<()>) -> Effect<()> {
        if let Event::Input(Input::Key(key)) = event {
            if key.is_char('q') || (key.is_plain() && key.code == KeyCode::Esc && key.is_down()) {
                return Effect::quit();
            }
            if key.is_plain() && key.code == KeyCode::Tab && key.is_down() {
                self.focused = !self.focused;
            }
            if key.is_char('b') {
                self.blink = !self.blink;
            }
            if self.focused {
                if key.is_char('+') || (key.is_plain() && key.code == KeyCode::Up && key.is_down())
                {
                    self.count = self.count.saturating_add(1);
                }
                if key.is_char('-')
                    || (key.is_plain() && key.code == KeyCode::Down && key.is_down())
                {
                    self.count = self.count.saturating_sub(1);
                }
            }
        }
        Effect::none()
    }
    fn view(&self, context: ViewContext<'_>) -> View {
        let base = if self.focused {
            context.theme.focus
        } else {
            context.theme.muted
        };
        let style = base
            .italic()
            .underline_style(UnderlineStyle::Curly)
            .underline_color(context.theme.accent.fg)
            .blink(if self.blink {
                BlinkStyle::Slow
            } else {
                BlinkStyle::None
            });
        let mut rows = vec![
            View::styled(
                format_args!(
                    "{} Count: {}",
                    if self.focused { ">" } else { " " },
                    self.count
                ),
                style,
            ),
            View::styled(
                "+/-: count | tab: focus | b: blink | q: quit",
                context.theme.muted,
            ),
            View::styled(
                "Dim / italic / strike",
                context.theme.text.dim().italic().strikethrough(),
            ),
            View::styled(
                "Encircled / overline",
                context
                    .theme
                    .text
                    .text_frame(TextFrame::Encircled)
                    .overline(),
            ),
        ];
        for underline in UnderlineStyle::ALL {
            rows.push(View::styled(
                format_args!("{underline:?}: Tea 茶"),
                context
                    .theme
                    .text
                    .underline_style(underline)
                    .underline_color(context.theme.accent.fg),
            ));
        }
        View::column(rows).panel("Matea attributes", context.theme)
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    support::launch(Attributes {
        count: 0,
        focused: true,
        blink: false,
    })
}
