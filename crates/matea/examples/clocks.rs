use matea::prelude::*;
use std::time::{Duration, Instant};

struct Clocks {
    stopwatch: Stopwatch,
    countdown: Countdown,
    timeouts: u32,
}
impl Default for Clocks {
    fn default() -> Self {
        let now = Instant::now();
        let mut stopwatch = Stopwatch::new_at(now);
        let mut countdown = Countdown::new_at(Duration::from_secs(2), now);
        let interval = TickInterval::new(Duration::from_millis(100)).unwrap();
        stopwatch.set_interval_at(interval, now);
        countdown.set_interval_at(interval, now);
        Self {
            stopwatch,
            countdown,
            timeouts: 0,
        }
    }
}
impl Clocks {
    fn observe(&mut self, update: ClockUpdate) {
        if update.timeout().is_some() {
            self.timeouts = self.timeouts.saturating_add(1);
        }
    }
}
impl Model for Clocks {
    type Message = ClockTick;
    fn init(&mut self) -> Effect<ClockTick> {
        let now = Instant::now();
        self.stopwatch.start_at(now);
        self.countdown.set_remaining_at(Duration::from_secs(2), now);
        Effect::none()
    }
    fn update(&mut self, event: Event<ClockTick>) -> Effect<ClockTick> {
        match event {
            Event::Message(tick) => {
                self.stopwatch.update(tick);
                let update = self.countdown.update(tick);
                self.observe(update);
            }
            Event::Input(Input::Key(key)) => {
                let now = Instant::now();
                if key.is_char('q') || key.is_plain() && key.code == KeyCode::Esc {
                    return Effect::quit();
                }
                if key.is_char('s') {
                    self.stopwatch.toggle_at(now);
                }
                if key.is_char('c') {
                    let update = self.countdown.toggle_at(now);
                    self.observe(update);
                }
                if key.is_char('r') {
                    self.stopwatch.reset_at(now);
                    self.countdown.set_remaining_at(Duration::from_secs(2), now);
                    self.countdown.start_at(now);
                }
            }
            _ => {}
        }
        Effect::none()
    }
    fn subscriptions(&self) -> Vec<Subscription<ClockTick>> {
        [
            self.stopwatch.subscription(|tick| tick),
            self.countdown.subscription(|tick| tick),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
    fn view(&self, c: ViewContext<'_>) -> View {
        let state = if self.countdown.timed_out() {
            "expired"
        } else if self.countdown.running() {
            "running"
        } else {
            "paused"
        };
        View::column([
            View::styled("Matea clocks", c.theme.accent),
            View::text(format!(
                "Stopwatch: {} ({})",
                self.stopwatch.text(),
                if self.stopwatch.running() {
                    "running"
                } else {
                    "paused"
                }
            )),
            View::text(format!("Countdown: {} ({state})", self.countdown.text())),
            View::text(format!("Timeouts: {}", self.timeouts)),
            View::styled(
                "s: stopwatch | c: countdown | r: reset | q: quit",
                c.theme.muted,
            ),
        ])
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    if std::env::args().any(|arg| arg == "--snapshot") {
        println!(
            "{}",
            Runtime::new(Clocks::default(), Size::new(64, 8), Theme::plain())?
                .frame()?
                .plain()
                .trim_end()
        );
        Ok(())
    } else {
        matea::run(Clocks::default())
    }
}
