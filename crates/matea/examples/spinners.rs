//! Independent animations and reduced-motion scheduling, without per-widget tasks.
use matea::prelude::*;
use std::time::Instant;
mod support;
struct Gallery {
    spinners: Vec<(SpinnerPreset, Spinner)>,
    paused: bool,
    reduced: bool,
    resets: u32,
    manual: u32,
    updates: u32,
}
impl Model for Gallery {
    type Message = ClockTick;
    fn init(&mut self) -> Effect<ClockTick> {
        let now = Instant::now();
        for (_, spinner) in &mut self.spinners {
            spinner.start_at(now);
        }
        Effect::none()
    }
    fn update(&mut self, event: Event<ClockTick>) -> Effect<ClockTick> {
        match event {
            Event::Message(tick) => {
                for (_, spinner) in &mut self.spinners {
                    if spinner.update(tick) {
                        self.updates = self.updates.saturating_add(1);
                    }
                }
            }
            Event::Input(Input::Key(key)) => {
                if key.is_char('q') || key.is_plain() && key.code == KeyCode::Esc {
                    return Effect::quit();
                }
                let now = Instant::now();
                if key.is_char('p') {
                    self.paused = !self.paused;
                    for (_, spinner) in &mut self.spinners {
                        if self.paused {
                            spinner.stop_at(now);
                        } else {
                            spinner.start_at(now);
                        }
                    }
                }
                if key.is_char('r') {
                    for (_, spinner) in &mut self.spinners {
                        spinner.reset_at(now);
                    }
                    self.resets = self.resets.saturating_add(1);
                }
                if key.is_char('n') {
                    let theme = Theme {
                        reduced_motion: self.reduced,
                        ..Theme::default()
                    };
                    for (_, spinner) in &mut self.spinners {
                        spinner.tick(&theme);
                    }
                    self.manual = self.manual.saturating_add(1);
                }
                if key.is_char('m') {
                    self.reduced = !self.reduced;
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
        self.spinners
            .iter()
            .filter_map(|(_, s)| s.subscription(&theme, |tick| tick))
            .collect()
    }
    fn view(&self, c: ViewContext<'_>) -> View {
        let mut theme = c.theme.clone();
        theme.reduced_motion |= self.reduced;
        let mut lines = vec![
            View::styled("Matea spinners", theme.accent),
            View::text(format!(
                "Animation: {} | Motion: {}",
                if self.paused { "paused" } else { "running" },
                if self.reduced { "reduced" } else { "normal" }
            )),
            View::text(format!(
                "Resets: {} | Manual: {} | Updates: {}",
                self.resets, self.manual, self.updates
            )),
        ];
        lines.extend(self.spinners.iter().map(|(preset, spinner)| {
            View::styled(
                format!("{preset:?}: {}", spinner.text(&theme)),
                theme.accent,
            )
        }));
        lines.push(View::styled(
            "p: pause | r: reset | n: step | m: motion | q: quit",
            theme.muted,
        ));
        View::column(lines)
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    let now = Instant::now();
    support::launch(Gallery {
        spinners: SpinnerPreset::ALL
            .into_iter()
            .map(|p| (p, Spinner::new_at(p.animation(), now)))
            .collect(),
        paused: false,
        reduced: matea::terminal::Capabilities::detect().reduced_motion,
        resets: 0,
        manual: 0,
        updates: 0,
    })
}
