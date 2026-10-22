use matea::prelude::*;
mod support;

struct Gradients {
    colors: Gradient,
    angle: i16,
    focused: bool,
}
impl Model for Gradients {
    type Message = ();
    fn update(&mut self, event: Event<()>) -> Effect<()> {
        if let Event::Input(Input::Key(key)) = event {
            if key.is_char('q') {
                return Effect::quit();
            }
            if key.is_plain() && key.is_down() {
                match key.code {
                    KeyCode::Tab => self.focused = !self.focused,
                    KeyCode::Right if self.focused => self.angle = (self.angle + 30) % 360,
                    KeyCode::Left if self.focused => self.angle = (self.angle + 330) % 360,
                    KeyCode::Esc => return Effect::quit(),
                    _ => {}
                }
            }
        }
        Effect::none()
    }
    fn view(&self, context: ViewContext<'_>) -> View {
        let width = context.size.width.saturating_sub(4).min(48);
        let style = if self.focused {
            context.theme.focus
        } else {
            context.theme.muted
        };
        let mut colors = self.colors.colors(usize::from(width) * 2);
        let stripe = View::row((0..width).map(|_| {
            let foreground = colors.next().expect("two samples per half block");
            let background = colors.next().expect("two samples per half block");
            View::styled("▌", context.theme.surface.fg(foreground).bg(background))
        }));
        let grid = self
            .colors
            .grid(width, 4, f64::from(self.angle))
            .expect("bounded finite color grid");
        let rows = (0..grid.height()).map(|y| {
            View::row((0..grid.width()).map(|x| {
                View::styled(
                    " ",
                    context
                        .theme
                        .surface
                        .bg(grid.color_at(x, y).expect("inside grid")),
                )
            }))
        });
        View::column([
            View::styled(
                format_args!(
                    "{} Angle: {}",
                    if self.focused { ">" } else { " " },
                    self.angle
                ),
                style,
            ),
            View::styled(
                "left/right: rotate | tab: focus | q: quit",
                context.theme.muted,
            ),
            stripe,
            View::column(rows),
        ])
        .panel("Matea gradients", context.theme)
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    support::launch(Gradients {
        colors: Gradient::new(&[[90, 86, 224], [238, 111, 248], [7, 201, 33]])
            .expect("small gradient configuration"),
        angle: 0,
        focused: true,
    })
}
