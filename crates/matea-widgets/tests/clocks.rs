use matea_core::{Effect, Event, Input, Key, KeyCode, Model, Runtime, Subscription, ViewContext};
use matea_render::{Size, View};
use matea_style::Theme;
use matea_widgets::{ClockText, ClockTick, ClockUpdate, Countdown, Stopwatch, TickInterval};
use std::{
    cell::Cell,
    collections::HashSet,
    rc::Rc,
    time::{Duration, Instant},
};

#[test]
fn defaults_reject_zero_intervals_and_zero_deadlines_do_not_schedule() {
    let sw = Stopwatch::new();
    assert_eq!(sw.interval().get(), Duration::from_secs(1));
    assert!(!sw.running());
    assert!(sw.schedule().is_none());
    assert!(TickInterval::new(Duration::ZERO).is_err());
    let mut zero = Countdown::new(Duration::ZERO);
    assert!(zero.timed_out());
    assert!(!zero.running());
    assert!(zero.schedule().is_none());
    assert_eq!(zero.start(), ClockUpdate::Ignored);
    let short = Countdown::new(Duration::from_nanos(1));
    assert_eq!(
        short.schedule().unwrap().interval().get(),
        Duration::from_nanos(1)
    );
}

#[test]
fn stopwatch_accounts_partial_intervals_pause_reset_and_invalidates_old_ticks() {
    let origin = Instant::now();
    let at = |ms| origin + Duration::from_millis(ms);
    let mut sw = Stopwatch::new_at(origin);
    assert_eq!(sw.start_at(origin), ClockUpdate::Changed);
    let first = sw.schedule().unwrap();
    assert_eq!(sw.update(first.tick_at(at(750))), ClockUpdate::Changed);
    assert_eq!(sw.elapsed(), Duration::from_millis(750));
    assert_eq!(sw.update(first.tick_at(at(750))), ClockUpdate::Ignored);
    sw.stop_at(at(900));
    assert_eq!(sw.elapsed(), Duration::from_millis(900));
    assert_eq!(sw.update(first.tick_at(at(1500))), ClockUpdate::Ignored);
    sw.start_at(at(2000));
    assert_eq!(sw.update(first.tick_at(at(2100))), ClockUpdate::Ignored);
    let second = sw.schedule().unwrap();
    sw.update(second.tick_at(at(2100)));
    assert_eq!(sw.elapsed(), Duration::from_secs(1));
    sw.reset_at(at(2200));
    assert!(sw.running());
    assert_eq!(sw.update(second.tick_at(at(2300))), ClockUpdate::Ignored);
    let third = sw.schedule().unwrap();
    sw.update(third.tick_at(at(2700)));
    assert_eq!(sw.elapsed(), Duration::from_millis(500));
    assert_eq!(sw.reset_at(at(2600)), ClockUpdate::Ignored);
    sw.set_interval_at(
        TickInterval::new(Duration::from_millis(250)).unwrap(),
        at(2700),
    );
    assert_eq!(sw.update(third.tick_at(at(2800))), ClockUpdate::Ignored);
    sw.update(sw.schedule().unwrap().tick_at(at(3000)));
    assert_eq!(sw.elapsed(), Duration::from_millis(800));
    assert_eq!(sw.elapsed_at(at(4000)), Duration::from_millis(1800));
    assert_eq!(sw.text().to_string(), "800ms");
    assert_eq!(sw.stop_at(at(2900)), ClockUpdate::Ignored);
    assert_eq!(sw.elapsed(), Duration::from_millis(800));
}

#[test]
fn countdown_shortens_last_period_and_notifies_timeout_once_without_overshoot() {
    let origin = Instant::now();
    let at = |ms| origin + Duration::from_millis(ms);
    let mut timer = Countdown::new_at(Duration::from_millis(1500), origin);
    let first = timer.schedule().unwrap();
    assert_eq!(first.interval().get(), Duration::from_secs(1));
    timer.update(first.tick_at(at(1200)));
    assert_eq!(timer.remaining(), Duration::from_millis(300));
    assert_eq!(
        timer.schedule().unwrap().interval().get(),
        Duration::from_millis(300)
    );
    assert_eq!(timer.remaining_at(at(1500)), Duration::ZERO);
    let last = timer.schedule().unwrap().tick_at(at(1600));
    assert_eq!(timer.update(last), ClockUpdate::TimedOut(timer.id()));
    assert_eq!(timer.update(last), ClockUpdate::Ignored);
    assert_eq!(timer.remaining(), Duration::ZERO);
    assert_eq!(timer.text().to_string(), "0s");
    assert!(!timer.running() && timer.timed_out());
    assert!(timer.schedule().is_none());
    timer.set_remaining_at(Duration::from_secs(2), at(2000));
    assert!(!timer.running());
    timer.start_at(at(2000));
    assert_eq!(timer.update(first.tick_at(at(2100))), ClockUpdate::Ignored);
    timer.stop_at(at(2100));
    assert_eq!(timer.remaining(), Duration::from_millis(1900));
    assert_eq!(timer.remaining_at(at(5000)), Duration::from_millis(1900));
    timer.set_remaining_at(Duration::from_secs(3), at(5000));
    timer.toggle_at(at(5000));
    assert_eq!(
        timer.set_remaining_at(Duration::ZERO, at(5100)),
        ClockUpdate::TimedOut(timer.id())
    );
    assert_eq!(
        timer.set_remaining_at(Duration::ZERO, at(5200)),
        ClockUpdate::Changed
    );
    assert!(timer.start_at(at(5300)).timeout().is_none());
}

#[test]
fn repeated_start_foreign_ticks_and_backward_controls_cannot_accelerate_clocks() {
    let origin = Instant::now();
    let mut sw = Stopwatch::new_at(origin);
    let mut timer = Countdown::new_at(Duration::from_secs(10), origin);
    sw.start_at(origin);
    let a = sw.schedule().unwrap();
    let b = timer.schedule().unwrap();
    let later = origin + Duration::from_secs(3);
    assert_eq!(sw.start_at(later), ClockUpdate::Ignored);
    assert_eq!(timer.start_at(later), ClockUpdate::Ignored);
    assert_eq!(sw.update(b.tick_at(later)), ClockUpdate::Ignored);
    assert_eq!(timer.update(a.tick_at(later)), ClockUpdate::Ignored);
    sw.update(a.tick_at(later));
    timer.update(b.tick_at(later));
    assert_eq!(sw.elapsed(), Duration::from_secs(3));
    assert_eq!(timer.remaining(), Duration::from_secs(7));
    assert_eq!(timer.update(b.tick_at(later)), ClockUpdate::Ignored);
    assert_eq!(
        timer.set_remaining_at(Duration::from_secs(100), origin),
        ClockUpdate::Ignored
    );
    assert_eq!(
        timer.set_interval_at(TickInterval::new(Duration::from_millis(1)).unwrap(), origin),
        ClockUpdate::Ignored
    );
    assert_eq!(timer.remaining(), Duration::from_secs(7));
    assert_eq!(timer.interval().get(), Duration::from_secs(1));
}

#[test]
fn identities_are_unique_across_clock_types_and_concurrent_construction() {
    let threads: Vec<_> = (0..8)
        .map(|_| {
            std::thread::spawn(|| {
                (0..100)
                    .flat_map(|_| {
                        [
                            Stopwatch::new().id().get(),
                            Countdown::new(Duration::ZERO).id().get(),
                        ]
                    })
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let ids: Vec<_> = threads
        .into_iter()
        .flat_map(|thread| thread.join().unwrap())
        .collect();
    let distinct: HashSet<_> = ids.iter().copied().collect();
    assert_eq!(distinct.len(), ids.len());
    assert!(!distinct.contains(&0));
}

#[test]
fn compact_duration_display_matches_fixed_reference_fixtures() {
    for line in include_str!("fixtures/durations.tsv").lines() {
        let (nanos, expected) = line.split_once('\t').unwrap();
        let value = Duration::from_nanos(nanos.parse().unwrap());
        assert_eq!(ClockText(value).to_string(), expected);
        assert_eq!(ClockText(value).into_string(), expected);
        assert_eq!(ClockText(value).into_text().as_str(), expected);
    }
    let maximum = ClockText(Duration::MAX).to_string();
    assert!(maximum.ends_with(".999999999s") && !maximum.contains('-'));
    assert_eq!(ClockText(Duration::MAX).into_text().as_str(), maximum);
    assert_eq!(
        format!("{:>7}", ClockText(Duration::from_nanos(1500))),
        "  1.5µs"
    );
    assert_eq!(
        format!("{} ms", Duration::from_secs(2).as_millis()),
        "2000 ms"
    );
}

#[test]
fn clock_views_keep_compact_unicode_units_and_theme_style() {
    use matea_style::{Color, Style};
    let origin = Instant::now();
    let value = Duration::from_nanos(1500);
    let mut stopwatch = Stopwatch::new_at(origin);
    stopwatch.start_at(origin);
    stopwatch.advance_at(origin + value);
    let countdown = Countdown::new_at(value, origin);
    let mut theme = Theme::plain();
    theme.text = Style::default().fg(Color::Indexed(2)).underline();
    for view in [stopwatch.view(&theme), countdown.view(&theme)] {
        let surface = view.render(Size::new(12, 1)).unwrap();
        assert_eq!(surface.plain(), "1.5µs");
        assert_eq!(surface.row(0)[3].symbol(), "µ");
        assert_eq!(surface.row(0)[3].style(), theme.text);
    }
    let mut live = Stopwatch::new();
    live.start();
    let schedule = live.schedule().unwrap();
    let tick = schedule.tick();
    assert_eq!(tick.id(), live.id());
    assert!(tick.at() <= Instant::now());
    live.update(tick);
    live.stop();
    live.reset();
    assert_eq!(live.elapsed(), Duration::ZERO);
    live.toggle();
    assert!(live.running());
    let mut countdown = Countdown::new(Duration::ZERO);
    countdown.set_remaining(Duration::from_secs(1));
    assert!(!countdown.running());
    countdown.start();
    countdown.set_interval(TickInterval::new(Duration::from_millis(100)).unwrap());
    assert_eq!(countdown.interval().get(), Duration::from_millis(100));
}

struct TimedApp {
    now: Rc<Cell<Instant>>,
    sw: Stopwatch,
    timer: Countdown,
    timeouts: usize,
}
impl Model for TimedApp {
    type Message = ClockTick;
    fn update(&mut self, event: Event<ClockTick>) -> Effect<ClockTick> {
        match event {
            Event::Message(tick) => {
                self.sw.update(tick);
                if self.timer.update(tick).timeout().is_some() {
                    self.timeouts += 1;
                }
            }
            Event::Input(_) => {
                self.sw.stop_at(self.now.get());
            }
            _ => {}
        }
        Effect::none()
    }
    fn subscriptions(&self) -> Vec<Subscription<ClockTick>> {
        [self.sw.schedule(), self.timer.schedule()]
            .into_iter()
            .flatten()
            .map(|schedule| {
                let now = self.now.clone();
                schedule.subscription_with_clock(move || now.get(), |tick| tick)
            })
            .collect()
    }
    fn view(&self, _: ViewContext<'_>) -> View {
        View::text(format!("{} {}", self.sw.text(), self.timer.text()))
    }
}
#[test]
fn runtime_reconciles_short_final_deadlines_and_coalesces_without_losing_elapsed_time() {
    let origin = Instant::now();
    let now = Rc::new(Cell::new(origin));
    let interval = TickInterval::new(Duration::from_millis(250)).unwrap();
    let mut sw = Stopwatch::new_at(origin);
    sw.set_interval_at(interval, origin);
    sw.start_at(origin);
    let mut timer = Countdown::new_at(Duration::from_millis(450), origin);
    timer.set_interval_at(interval, origin);
    let mut runtime = Runtime::new(
        TimedApp {
            now: now.clone(),
            sw,
            timer,
            timeouts: 0,
        },
        Size::new(24, 1),
        Theme::plain(),
    )
    .unwrap();
    now.set(origin + Duration::from_millis(250));
    runtime.advance(Duration::from_millis(250)).unwrap();
    assert_eq!(
        runtime.model().timer.remaining(),
        Duration::from_millis(200)
    );
    now.set(origin + Duration::from_millis(450));
    runtime.advance(Duration::from_millis(200)).unwrap();
    assert_eq!(runtime.model().timeouts, 1);
    assert!(runtime.model().timer.schedule().is_none());
    now.set(origin + Duration::from_secs(2));
    runtime.advance(Duration::from_millis(1550)).unwrap();
    assert_eq!(runtime.model().sw.elapsed(), Duration::from_secs(2));
    assert_eq!(runtime.model().timeouts, 1);
    assert!(!runtime.advance(Duration::ZERO).unwrap());
    now.set(origin + Duration::from_millis(2100));
    runtime
        .dispatch(Event::Input(Input::Key(Key::new(KeyCode::Char(' ')))))
        .unwrap();
    now.set(origin + Duration::from_millis(12100));
    runtime.advance(Duration::from_millis(10100)).unwrap();
    assert_eq!(runtime.model().sw.elapsed(), Duration::from_millis(2100));
    assert_eq!(runtime.frame().unwrap().plain(), "2.1s 0s");
}
