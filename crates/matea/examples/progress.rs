//! Typed animation scheduling, theme defaults and independent color policies.
use matea::prelude::*;
use std::time::Instant;
mod support;

struct Gallery {
    bars: Vec<Progress>,
    focus: usize,
    reduced: bool,
    width: u16,
}
impl Gallery {
    fn resize(&mut self, width: u16) {
        self.width = width;
        for bar in &mut self.bars {
            bar.set_width(width.saturating_sub(2).min(48));
        }
    }
}
impl Model for Gallery {
    type Message = ClockTick;
    fn update(&mut self, event: Event<ClockTick>) -> Effect<ClockTick> {
        match event {
            Event::Message(tick) => {
                for bar in &mut self.bars {
                    bar.update(tick).expect("validated animation");
                }
            }
            Event::Resize(size) => self.resize(size.width),
            Event::Input(Input::Key(key)) => {
                if key.is_char('q') || key.is_plain() && key.is_down() && key.code == KeyCode::Esc {
                    return Effect::quit();
                }
                if key.is_plain() && key.is_down() && key.code == KeyCode::Tab {
                    self.focus = (self.focus + 1) % self.bars.len();
                }
                let bar = &mut self.bars[self.focus];
                if key.is_plain() && key.is_down() && key.code == KeyCode::Right {
                    bar.increment(0.1).expect("finite target");
                }
                if key.is_plain() && key.is_down() && key.code == KeyCode::Left {
                    bar.decrement(0.1).expect("finite target");
                }
                if key.is_char('r') {
                    bar.set_percent(0.0).expect("finite target");
                }
                if key.is_char('g') {
                    let full = if bar.glyphs().0 == PROGRESS_HALF_BLOCK {
                        PROGRESS_FULL_BLOCK
                    } else {
                        PROGRESS_HALF_BLOCK
                    };
                    bar.set_glyphs(full, PROGRESS_EMPTY_BLOCK)
                        .expect("one-cell glyphs");
                }
                if key.is_char('s')
                    && let ProgressFill::Gradient { scaled, .. } = &mut bar.fill
                {
                    *scaled = !*scaled;
                }
                if key.is_char('m') {
                    self.reduced = !self.reduced;
                    let now = Instant::now();
                    for bar in &mut self.bars {
                        bar.set_reduced_motion_at(self.reduced, now);
                    }
                }
            }
            _ => {}
        }
        Effect::none()
    }
    fn subscriptions(&self) -> Vec<Subscription<ClockTick>> {
        let theme = Theme {
            reduced_motion: self.reduced,
            ..Theme::default()
        };
        self.bars
            .iter()
            .filter_map(|bar| bar.subscription(&theme, |tick| tick))
            .collect()
    }
    fn view(&self, c: ViewContext<'_>) -> View {
        let mut theme = c.theme.clone();
        theme.reduced_motion |= self.reduced;
        let mut lines = vec![
            View::styled("Matea progress", theme.accent),
            View::styled(
                format!(
                    "Width: {} | Motion: {}",
                    self.width,
                    if self.reduced { "reduced" } else { "normal" }
                ),
                theme.muted,
            ),
        ];
        for (index, bar) in self.bars.iter().enumerate() {
            let description = match bar.fill {
                ProgressFill::Theme => "theme",
                ProgressFill::Solid(_) => "solid",
                ProgressFill::Gradient { scaled: true, .. } => "gradient scaled",
                ProgressFill::Gradient { scaled: false, .. } => "gradient unscaled",
                ProgressFill::ColorFn(_) => "callback",
            };
            lines.push(View::styled(
                format!(
                    "{} Target: {:.0}% | {description}",
                    if index == self.focus { ">" } else { " " },
                    bar.target() * 100.0
                ),
                if index == self.focus {
                    theme.focus
                } else {
                    theme.text
                },
            ));
            lines.push(bar.view(&theme));
        }
        lines.push(View::styled(
            "Tab: focus | ←/→: target | g: glyph | s: scale | m: motion | r: reset | q: quit",
            theme.muted,
        ));
        View::column(lines)
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    let reduced = matea::terminal::Capabilities::detect().reduced_motion;
    let now = Instant::now();
    let mut bars: Vec<_> = (0..3).map(|_| Progress::new_at(now)).collect();
    bars[1].fill = ProgressFill::Gradient {
        gradient: Gradient::new(&[[175, 215, 135], [229, 235, 223], [255, 150, 140]])
            .expect("nonempty stops"),
        scaled: false,
    };
    bars[2].fill = ProgressFill::ColorFn(Box::new(|total, current| {
        Color::Rgb(
            (175.0 + 80.0 * total) as u8,
            (215.0 - 65.0 * current) as u8,
            135,
        )
    }));
    for bar in &mut bars {
        bar.set_reduced_motion_at(reduced, now);
        bar.set_percent_at(0.25, now).expect("finite target");
    }
    let mut gallery = Gallery {
        bars,
        focus: 0,
        reduced,
        width: 72,
    };
    gallery.resize(72);
    support::launch(gallery)
}
