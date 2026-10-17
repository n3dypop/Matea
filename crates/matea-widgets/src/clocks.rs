//! Monotonic UI clocks with cheap, generation-safe scheduling descriptors.
use matea_core::{Subscription, SubscriptionId};
use matea_render::View;
use matea_style::Theme;
use std::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

/// In-process identity shared across both clock types; never a broadcast token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClockId(u64);
impl ClockId {
    pub fn get(self) -> u64 {
        self.0
    }
}
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
fn next_id() -> ClockId {
    let mut id = NEXT_ID.load(Ordering::Relaxed);
    loop {
        let next = id.checked_add(1).expect("clock identity space exhausted");
        match NEXT_ID.compare_exchange_weak(id, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return ClockId(id),
            Err(observed) => id = observed,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZeroTickInterval;
impl fmt::Display for ZeroTickInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("clock tick interval must be nonzero")
    }
}
impl std::error::Error for ZeroTickInterval {}
/// Positive refresh period, independent of the measured elapsed time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickInterval(Duration);
impl TickInterval {
    pub fn new(duration: Duration) -> Result<Self, ZeroTickInterval> {
        if duration.is_zero() {
            Err(ZeroTickInterval)
        } else {
            Ok(Self(duration))
        }
    }
    pub fn get(self) -> Duration {
        self.0
    }
}
impl Default for TickInterval {
    fn default() -> Self {
        Self(Duration::from_secs(1))
    }
}

/// Handle timeout outcomes from controls as well as ticks. Foreign, duplicate,
/// stale and backward-time updates leave the cached value unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockUpdate {
    Ignored,
    Changed,
    TimedOut(ClockId),
}
impl ClockUpdate {
    pub fn changed(self) -> bool {
        self != Self::Ignored
    }
    pub fn timeout(self) -> Option<ClockId> {
        if let Self::TimedOut(id) = self {
            Some(id)
        } else {
            None
        }
    }
}
/// Immutable scheduling-generation timestamp; construct through ClockSchedule.
#[derive(Clone, Copy, Debug)]
pub struct ClockTick {
    id: ClockId,
    generation: u64,
    at: Instant,
}
impl ClockTick {
    pub fn id(self) -> ClockId {
        self.id
    }
    pub fn at(self) -> Instant {
        self.at
    }
}
/// Cheap scheduling intent. The UI-thread subscription adapter currently
/// allocates and does not promise real-time wake precision.
#[derive(Clone, Copy, Debug)]
pub struct ClockSchedule {
    id: ClockId,
    generation: u64,
    interval: TickInterval,
}
struct ClockSubscription;
impl ClockSchedule {
    pub fn id(self) -> ClockId {
        self.id
    }
    pub fn interval(self) -> TickInterval {
        self.interval
    }
    pub fn tick(self) -> ClockTick {
        self.tick_at(Instant::now())
    }
    pub fn tick_at(self, at: Instant) -> ClockTick {
        ClockTick {
            id: self.id,
            generation: self.generation,
            at,
        }
    }
    pub fn subscription<M: 'static>(
        self,
        message: impl Fn(ClockTick) -> M + 'static,
    ) -> Subscription<M> {
        self.subscription_with_clock(Instant::now, message)
    }
    /// Use the same monotonic domain as the clock's _at controls.
    pub fn subscription_with_clock<M: 'static>(
        self,
        now: impl Fn() -> Instant + 'static,
        message: impl Fn(ClockTick) -> M + 'static,
    ) -> Subscription<M> {
        let id = SubscriptionId::scoped::<ClockSubscription>(
            self.id.0,
            self.generation,
            self.interval.0.as_nanos(),
        );
        Subscription::every_with_id(id, self.interval.0, move || message(self.tick_at(now())))
    }
}

#[derive(Debug)]
struct Clock<const DOWN: bool> {
    id: ClockId,
    generation: u64,
    value: Duration,
    anchor: Instant,
    running: bool,
    interval: TickInterval,
}
impl<const DOWN: bool> Clock<DOWN> {
    fn new(value: Duration, anchor: Instant, running: bool) -> Self {
        Self {
            id: next_id(),
            generation: 0,
            value,
            anchor,
            running,
            interval: TickInterval::default(),
        }
    }
    fn bump(&mut self) {
        self.generation = self
            .generation
            .checked_add(1)
            .expect("clock generation space exhausted");
    }
    fn projected(&self, at: Instant) -> Duration {
        let Some(elapsed) = at
            .checked_duration_since(self.anchor)
            .filter(|_| self.running)
        else {
            return self.value;
        };
        if DOWN {
            self.value.saturating_sub(elapsed)
        } else {
            self.value.saturating_add(elapsed)
        }
    }
    fn advance(&mut self, at: Instant) -> ClockUpdate {
        if !self.running || at <= self.anchor {
            return ClockUpdate::Ignored;
        }
        let value = self.projected(at);
        self.anchor = at;
        if DOWN && value.is_zero() {
            self.value = value;
            self.running = false;
            self.bump();
            ClockUpdate::TimedOut(self.id)
        } else {
            let changed = value != self.value;
            self.value = value;
            if changed {
                ClockUpdate::Changed
            } else {
                ClockUpdate::Ignored
            }
        }
    }
    fn update(&mut self, tick: ClockTick) -> ClockUpdate {
        if tick.id != self.id || tick.generation != self.generation {
            ClockUpdate::Ignored
        } else {
            self.advance(tick.at)
        }
    }
    fn start(&mut self, at: Instant) -> ClockUpdate {
        if self.running || at < self.anchor || DOWN && self.value.is_zero() {
            return ClockUpdate::Ignored;
        }
        self.anchor = at;
        self.running = true;
        self.bump();
        ClockUpdate::Changed
    }
    fn stop(&mut self, at: Instant) -> ClockUpdate {
        if !self.running || at < self.anchor {
            return ClockUpdate::Ignored;
        }
        let update = self.advance(at);
        if update.timeout().is_some() {
            return update;
        }
        self.running = false;
        self.anchor = at;
        self.bump();
        ClockUpdate::Changed
    }
    fn set(&mut self, value: Duration, at: Instant) -> ClockUpdate {
        if at < self.anchor {
            return ClockUpdate::Ignored;
        }
        let timeout = DOWN && value.is_zero() && self.running && !self.value.is_zero();
        self.value = value;
        self.anchor = at;
        self.bump();
        if DOWN && value.is_zero() {
            self.running = false;
        }
        if timeout {
            ClockUpdate::TimedOut(self.id)
        } else {
            ClockUpdate::Changed
        }
    }
    fn set_interval(&mut self, interval: TickInterval, at: Instant) -> ClockUpdate {
        if at < self.anchor {
            return ClockUpdate::Ignored;
        }
        let update = self.advance(at);
        if interval == self.interval {
            return update;
        }
        self.interval = interval;
        self.anchor = at;
        self.bump();
        if update.timeout().is_some() {
            update
        } else {
            ClockUpdate::Changed
        }
    }
    fn schedule(&self) -> Option<ClockSchedule> {
        self.running.then(|| ClockSchedule {
            id: self.id,
            generation: self.generation,
            interval: if DOWN {
                TickInterval(self.interval.0.min(self.value))
            } else {
                self.interval
            },
        })
    }
}

/// Initially paused elapsed-time clock, with a one-second refresh period.
/// Text is a cached snapshot; elapsed_at projects without mutation.
#[derive(Debug)]
pub struct Stopwatch(Clock<false>);
/// Positive countdowns start running. Values clamp to zero; retiming an expired
/// clock preserves pause until explicit start. Text/getters are cached snapshots.
#[derive(Debug)]
pub struct Countdown(Clock<true>);
macro_rules! controls {
    ($name:ident) => {
        impl $name {
            pub fn id(&self) -> ClockId {
                self.0.id
            }
            pub fn running(&self) -> bool {
                self.0.running
            }
            pub fn interval(&self) -> TickInterval {
                self.0.interval
            }
            pub fn schedule(&self) -> Option<ClockSchedule> {
                self.0.schedule()
            }
            pub fn subscription<M: 'static>(
                &self,
                message: impl Fn(ClockTick) -> M + 'static,
            ) -> Option<Subscription<M>> {
                self.schedule()
                    .map(|schedule| schedule.subscription(message))
            }
            pub fn update(&mut self, tick: ClockTick) -> ClockUpdate {
                self.0.update(tick)
            }
            /// Owner-authorized advance, including a shared timestamp broadcast.
            pub fn advance_at(&mut self, at: Instant) -> ClockUpdate {
                self.0.advance(at)
            }
            pub fn start(&mut self) -> ClockUpdate {
                self.start_at(Instant::now())
            }
            pub fn start_at(&mut self, at: Instant) -> ClockUpdate {
                self.0.start(at)
            }
            pub fn stop(&mut self) -> ClockUpdate {
                self.stop_at(Instant::now())
            }
            pub fn stop_at(&mut self, at: Instant) -> ClockUpdate {
                self.0.stop(at)
            }
            pub fn toggle(&mut self) -> ClockUpdate {
                self.toggle_at(Instant::now())
            }
            pub fn toggle_at(&mut self, at: Instant) -> ClockUpdate {
                if self.running() {
                    self.stop_at(at)
                } else {
                    self.start_at(at)
                }
            }
            pub fn set_interval(&mut self, interval: TickInterval) -> ClockUpdate {
                self.set_interval_at(interval, Instant::now())
            }
            pub fn set_interval_at(&mut self, interval: TickInterval, at: Instant) -> ClockUpdate {
                self.0.set_interval(interval, at)
            }
            pub fn text(&self) -> ClockText {
                ClockText(self.0.value)
            }
            pub fn view(&self, theme: &Theme) -> View {
                View::styled(self.text().into_string(), theme.text)
            }
        }
    };
}
controls!(Stopwatch);
controls!(Countdown);
impl Default for Stopwatch {
    fn default() -> Self {
        Self::new()
    }
}
impl Stopwatch {
    pub fn new() -> Self {
        Self::new_at(Instant::now())
    }
    pub fn new_at(at: Instant) -> Self {
        Self(Clock::new(Duration::ZERO, at, false))
    }
    pub fn elapsed(&self) -> Duration {
        self.0.value
    }
    pub fn elapsed_at(&self, at: Instant) -> Duration {
        self.0.projected(at)
    }
    pub fn reset(&mut self) -> ClockUpdate {
        self.reset_at(Instant::now())
    }
    pub fn reset_at(&mut self, at: Instant) -> ClockUpdate {
        self.0.set(Duration::ZERO, at)
    }
}
impl Countdown {
    pub fn new(timeout: Duration) -> Self {
        Self::new_at(timeout, Instant::now())
    }
    pub fn new_at(timeout: Duration, at: Instant) -> Self {
        Self(Clock::new(timeout, at, !timeout.is_zero()))
    }
    pub fn remaining(&self) -> Duration {
        self.0.value
    }
    pub fn remaining_at(&self, at: Instant) -> Duration {
        self.0.projected(at)
    }
    pub fn timed_out(&self) -> bool {
        self.0.value.is_zero()
    }
    /// Preserve pause state. Expired clocks require an explicit start to rearm.
    pub fn set_remaining(&mut self, timeout: Duration) -> ClockUpdate {
        self.set_remaining_at(timeout, Instant::now())
    }
    pub fn set_remaining_at(&mut self, timeout: Duration, at: Instant) -> ClockUpdate {
        self.0.set(timeout, at)
    }
}

/// Compact nonnegative duration formatting, independent of scheduling and style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockText(pub Duration);
struct DurationText {
    bytes: [u8; 40],
    len: usize,
}
impl DurationText {
    fn push(&mut self, text: &[u8]) {
        let end = self.len + text.len();
        self.bytes[self.len..end].copy_from_slice(text);
        self.len = end;
    }
    fn integer(&mut self, mut value: u64) {
        let mut digits = [0u8; 20];
        let mut start = digits.len();
        loop {
            start -= 1;
            digits[start] = b'0' + (value % 10) as u8;
            value /= 10;
            if value == 0 {
                break;
            }
        }
        self.push(&digits[start..]);
    }
    fn fraction(&mut self, mut value: u32, width: usize) {
        if value == 0 {
            return;
        }
        let mut digits = [b'0'; 9];
        for byte in digits[..width].iter_mut().rev() {
            *byte += (value % 10) as u8;
            value /= 10;
        }
        let end = digits[..width]
            .iter()
            .rposition(|byte| *byte != b'0')
            .unwrap()
            + 1;
        self.push(b".");
        self.push(&digits[..end]);
    }
    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len])
            .expect("clock text uses digits and static UTF-8 units")
    }
}
impl ClockText {
    /// One exact-size owned string allocation; no growing intermediate String.
    #[inline]
    pub fn into_string(self) -> String {
        if self.0.is_zero() {
            return "0s".to_owned();
        }
        String::from(self.formatted().as_str())
    }
    fn formatted(self) -> DurationText {
        // Duration::MAX needs at most 33 bytes: 16 hour digits, unit fields,
        // fractional seconds and suffix. No user-controlled text enters here.
        let mut out = DurationText {
            bytes: [0; 40],
            len: 0,
        };
        let seconds = self.0.as_secs();
        let nanos = self.0.subsec_nanos();
        if seconds == 0 {
            if nanos == 0 {
                out.push(b"0s");
            } else if nanos < 1_000 {
                out.integer(u64::from(nanos));
                out.push(b"ns");
            } else if nanos < 1_000_000 {
                out.integer(u64::from(nanos / 1_000));
                out.fraction(nanos % 1_000, 3);
                out.push("µs".as_bytes());
            } else {
                out.integer(u64::from(nanos / 1_000_000));
                out.fraction(nanos % 1_000_000, 6);
                out.push(b"ms");
            }
            return out;
        }
        let hours = seconds / 3_600;
        let minutes = seconds / 60 % 60;
        if hours > 0 {
            out.integer(hours);
            out.push(b"h");
        }
        if hours > 0 || minutes > 0 {
            out.integer(minutes);
            out.push(b"m");
        }
        out.integer(seconds % 60);
        out.fraction(nanos, 9);
        out.push(b"s");
        out
    }
}
impl fmt::Display for ClockText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_zero() {
            f.pad("0s")
        } else {
            f.pad(self.formatted().as_str())
        }
    }
}
