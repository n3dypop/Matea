//! A clonable text value composed with independently owned cursor scheduling.
use matea::prelude::*;
mod support;

struct Editor {
    field: TextInput,
    cursor: VirtualCursor,
    theme: Theme,
}
impl Model for Editor {
    type Message = ClockTick;
    fn init(&mut self) -> Effect<ClockTick> {
        self.cursor.focus();
        Effect::none()
    }
    fn update(&mut self, event: Event<ClockTick>) -> Effect<ClockTick> {
        match event {
            Event::Message(tick) => {
                self.cursor.update(tick);
            }
            Event::Input(Input::Key(key)) if key.is_plain() && key.code == KeyCode::Esc => {
                return Effect::quit();
            }
            Event::Input(Input::Key(key))
                if key.matches_chord(
                    Key::new(KeyCode::Char('b')).with_modifiers(Modifiers::CONTROL),
                ) =>
            {
                self.cursor
                    .set_mode(if self.cursor.mode() == CursorMode::Blink {
                        CursorMode::Static
                    } else {
                        CursorMode::Blink
                    });
            }
            Event::Input(Input::Key(key))
                if key.matches_chord(
                    Key::new(KeyCode::Char('r')).with_modifiers(Modifiers::CONTROL),
                ) =>
            {
                self.theme.reduced_motion = !self.theme.reduced_motion
            }
            Event::Input(Input::Focus(focused)) => {
                if focused {
                    self.cursor.focus();
                } else {
                    self.cursor.blur();
                }
            }
            Event::Input(input) if self.field.event(&input, self.cursor.focused()).consumed => {
                self.cursor.restart();
            }
            _ => {}
        }
        Effect::none()
    }
    fn subscriptions(&self) -> Vec<Subscription<ClockTick>> {
        self.cursor
            .subscription(&self.theme, |tick| tick)
            .into_iter()
            .collect()
    }
    fn view(&self, context: ViewContext<'_>) -> View {
        let mut theme = context.theme.clone();
        theme.reduced_motion |= self.theme.reduced_motion;
        View::column([
            View::styled("Matea virtual cursor", theme.accent),
            self.field
                .view_with_cursor(&theme, self.cursor.focused(), &self.cursor),
            View::text(format!(
                "Cursor: {} | Motion: {}",
                self.cursor.mode(),
                if self.theme.reduced_motion {
                    "reduced"
                } else {
                    "normal"
                }
            )),
            View::styled(
                "ctrl+b: blink/static | ctrl+r: motion | Esc: quit",
                theme.muted,
            ),
        ])
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    support::launch(Editor {
        field: TextInput::new("Matcha"),
        cursor: VirtualCursor::new(),
        theme: Theme {
            reduced_motion: matea::terminal::Capabilities::detect().reduced_motion,
            ..Theme::default()
        },
    })
}
