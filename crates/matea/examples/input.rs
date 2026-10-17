//! Inspect typed input without terminal escape strings in application code.
use matea::{
    prelude::*,
    terminal::{CtrlCBehavior, InputOptions, KeyboardFlags, KeyboardMode, RunOptions},
};
use std::collections::VecDeque;
#[derive(Default)]
struct Inspector {
    events: VecDeque<String>,
}
impl Model for Inspector {
    type Message = ();
    fn update(&mut self, event: Event<()>) -> Effect<()> {
        if let Event::Input(input) = event {
            if matches!(&input,Input::Key(key) if key.is_char('q')) {
                return Effect::quit();
            }
            let text = match input {
                Input::Key(key) => format!(
                    "Key {:?} {:?} {:?} {:?}",
                    key.code, key.kind, key.modifiers, key.state
                ),
                Input::Mouse(mouse) => format!(
                    "Mouse {:?} {},{} {:?}",
                    mouse.kind, mouse.column, mouse.row, mouse.modifiers
                ),
                Input::Paste(text) => format!("Paste {} bytes", text.len()),
                Input::Focus(focused) => format!("Focus {focused}"),
            };
            if self.events.len() == 8 {
                self.events.pop_front();
            }
            self.events.push_back(text);
        }
        Effect::none()
    }
    fn view(&self, c: ViewContext<'_>) -> View {
        View::column(
            std::iter::once(View::styled("Matea input | q exits", c.theme.accent))
                .chain(self.events.iter().map(View::text)),
        )
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    if std::env::args().any(|s| s == "--snapshot") {
        println!(
            "{}",
            Runtime::new(Inspector::default(), Size::new(72, 24), Theme::plain())?
                .frame()?
                .plain()
                .trim_end()
        );
        return Ok(());
    }
    let options = RunOptions {
        input: InputOptions {
            mouse: std::env::args().any(|s| s == "--mouse"),
            keyboard: if std::env::args().any(|s| s == "--kitty") {
                KeyboardMode::Kitty(
                    KeyboardFlags::DISAMBIGUATE_ESCAPE_CODES
                        | KeyboardFlags::REPORT_EVENT_TYPES
                        | KeyboardFlags::REPORT_ALTERNATE_KEYS
                        | KeyboardFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES,
                )
            } else {
                KeyboardMode::Legacy
            },
            ..InputOptions::default()
        },
        ctrl_c: if std::env::args().any(|s| s == "--deliver-ctrl-c") {
            CtrlCBehavior::Deliver
        } else {
            CtrlCBehavior::Quit
        },
        ..RunOptions::default()
    };
    matea::run_with_options(Inspector::default(), options)
}
