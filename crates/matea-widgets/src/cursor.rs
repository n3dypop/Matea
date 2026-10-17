//! A virtual cursor with monotonic phase and generation-safe scheduling.
use crate::{ClockId, ClockSchedule, ClockTick, Stopwatch, TickInterval};
use matea_core::Subscription;
use matea_render::View;
use matea_style::{Style, Theme};
use std::{fmt, time::Instant};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CursorMode {
    #[default]
    Blink,
    Static,
    Hidden,
}
impl fmt::Display for CursorMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Blink => "blink",
            Self::Static => "static",
            Self::Hidden => "hidden",
        })
    }
}

/// Owner-controlled cursor state. A live cursor deliberately does not implement
/// Clone: its identity, phase and pending notifications have one owner.
///
/// Schedule reconciliation removes timers after blur, mode changes or reduced
/// motion. Already queued ticks are harmless after any control changes the
/// generation. No timer, task or context is allocated by the controls themselves.
#[derive(Debug)]
pub struct VirtualCursor {
    clock: Stopwatch,
    mode: CursorMode,
    focused: bool,
    blinked: bool,
    text: String,
    pub style: Option<Style>,
    pub text_style: Option<Style>,
}
impl Default for VirtualCursor {
    fn default() -> Self {
        Self::new()
    }
}
impl VirtualCursor {
    pub fn new() -> Self {
        Self::new_at(Instant::now())
    }
    pub fn new_at(at: Instant) -> Self {
        let mut clock = Stopwatch::new_at(at);
        clock.set_interval_at(
            TickInterval::new(std::time::Duration::from_millis(530)).unwrap(),
            at,
        );
        Self {
            clock,
            mode: CursorMode::Blink,
            focused: false,
            blinked: true,
            text: String::new(),
            style: None,
            text_style: None,
        }
    }
    pub fn id(&self) -> ClockId {
        self.clock.id()
    }
    pub fn mode(&self) -> CursorMode {
        self.mode
    }
    pub fn focused(&self) -> bool {
        self.focused
    }
    /// Cached blink phase, before the theme's reduced-motion override.
    pub fn blinked(&self) -> bool {
        self.blinked
    }
    pub fn interval(&self) -> TickInterval {
        self.clock.interval()
    }
    pub fn set_char(&mut self, text: impl AsRef<str>) {
        self.text.clear();
        self.text.push_str(text.as_ref());
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn focus(&mut self) -> bool {
        self.focus_at(Instant::now())
    }
    /// Focus, reveal and restart the phase, including when already focused.
    /// Backward-time controls return false and leave all state unchanged.
    pub fn focus_at(&mut self, at: Instant) -> bool {
        if !self.clock.reset_at(at).changed() {
            return false;
        }
        self.focused = true;
        self.reconcile(at);
        true
    }
    pub fn blur(&mut self) -> bool {
        self.blur_at(Instant::now())
    }
    pub fn blur_at(&mut self, at: Instant) -> bool {
        if !self.clock.reset_at(at).changed() {
            return false;
        }
        self.focused = false;
        self.reconcile(at);
        true
    }
    pub fn set_mode(&mut self, mode: CursorMode) -> bool {
        self.set_mode_at(mode, Instant::now())
    }
    pub fn set_mode_at(&mut self, mode: CursorMode, at: Instant) -> bool {
        if !self.clock.reset_at(at).changed() {
            return false;
        }
        self.mode = mode;
        self.reconcile(at);
        true
    }
    pub fn set_interval(&mut self, interval: TickInterval) -> bool {
        self.set_interval_at(interval, Instant::now())
    }
    pub fn set_interval_at(&mut self, interval: TickInterval, at: Instant) -> bool {
        if !self.clock.reset_at(at).changed() {
            return false;
        }
        self.clock.set_interval_at(interval, at);
        self.reconcile(at);
        true
    }
    pub fn restart(&mut self) -> bool {
        self.restart_at(Instant::now())
    }
    /// Reveal after editing and invalidate old ticks. Static/hidden/blurred
    /// cursors need no restart. This also replaces the upstream broadcast Blink.
    pub fn restart_at(&mut self, at: Instant) -> bool {
        if !self.focused || self.mode != CursorMode::Blink {
            return false;
        }
        self.focus_at(at)
    }
    fn reconcile(&mut self, at: Instant) {
        self.blinked = !self.focused || self.mode == CursorMode::Hidden;
        if self.focused && self.mode == CursorMode::Blink {
            self.clock.start_at(at);
        } else {
            self.clock.stop_at(at);
        }
    }
    pub fn visible(&self, theme: &Theme) -> bool {
        self.focused
            && self.mode != CursorMode::Hidden
            && (self.mode == CursorMode::Static || theme.reduced_motion || !self.blinked)
    }
    pub fn schedule(&self, theme: &Theme) -> Option<ClockSchedule> {
        if theme.reduced_motion {
            None
        } else {
            self.clock.schedule()
        }
    }
    pub fn subscription<M: 'static>(
        &self,
        theme: &Theme,
        message: impl Fn(ClockTick) -> M + 'static,
    ) -> Option<Subscription<M>> {
        self.schedule(theme)
            .map(|schedule| schedule.subscription(message))
    }
    /// Accept a current, later observation. Elapsed periods determine the phase,
    /// so a delayed/coalesced notification does not slow blinking. Duplicate,
    /// foreign, backward and stale ticks are ignored without allocating.
    pub fn update(&mut self, tick: ClockTick) -> bool {
        if !self.clock.update(tick).changed() {
            return false;
        }
        self.blinked =
            (self.clock.elapsed().as_nanos() / self.interval().get().as_nanos()) % 2 == 1;
        true
    }
    pub fn view(&self, theme: &Theme) -> View {
        let style = if self.visible(theme) {
            self.style.unwrap_or(theme.text).reverse()
        } else {
            self.text_style.unwrap_or(theme.text)
        };
        View::styled(self.text(), style)
    }
}
