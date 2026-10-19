use matea::prelude::*;
use matea::ratatui::{self, WidgetState, widgets::ListState};
mod support;

const TEAS: [&str; 6] = ["Matcha", "Sencha", "Hojicha", "Genmaicha", "Oolong", "抹茶"];

struct App {
    list: WidgetState<ListState>,
}
impl Model for App {
    type Message = ();
    fn update(&mut self, event: Event<()>) -> Effect<()> {
        if let Event::Input(Input::Key(key)) = event {
            if key.is_char('q') || (key.is_plain() && key.code == KeyCode::Esc) {
                return Effect::quit();
            }
            let state = self.list.state_mut();
            let selected = state.selected().unwrap_or(0);
            if key.is_char('j') || (key.is_plain() && key.code == KeyCode::Down) {
                state.select(Some((selected + 1).min(TEAS.len() - 1)));
            }
            if key.is_char('k') || (key.is_plain() && key.code == KeyCode::Up) {
                state.select(Some(selected.saturating_sub(1)));
            }
        }
        Effect::none()
    }
    fn view(&self, c: ViewContext<'_>) -> View {
        use ratatui::widgets::{Block, List};
        let scene = (|| {
            let text = ratatui::to_style(c.theme.text).map_err(|e| e.to_string())?;
            let focus = ratatui::to_style(c.theme.focus).map_err(|e| e.to_string())?;
            self.list
                .render(
                    Size::new(c.size.width, c.size.height.saturating_sub(2)),
                    List::new(TEAS)
                        .block(Block::bordered().title("Tea collection"))
                        .style(text)
                        .highlight_style(focus)
                        .highlight_symbol("> "),
                )
                .map_err(|e| e.to_string())
        })();
        View::column([
            View::styled("Matea Rust widget bridge", c.theme.accent),
            scene.unwrap_or_else(|e| View::styled(e, c.theme.error)),
            View::styled("arrows / j / k: select | q / Esc: quit", c.theme.muted),
        ])
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    support::launch(App {
        list: WidgetState::new(ListState::default().with_selected(Some(0))),
    })
}
