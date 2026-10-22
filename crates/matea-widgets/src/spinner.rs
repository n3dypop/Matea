//! Validated animation data and a generation-safe, monotonic spinner.
use crate::{Action, ClockId, ClockSchedule, ClockTick, Stopwatch, TickInterval};
use matea_core::{Component, Input, Response, Subscription};
use matea_render::View;
use matea_style::{Style, Theme};
use std::{
    borrow::Cow,
    fmt,
    sync::Arc,
    time::{Duration, Instant},
};

/// Immutable animation data; cloning shares custom frame storage.
#[derive(Clone, Debug)]
pub struct SpinnerAnimation {
    frames: Frames,
    interval: TickInterval,
}
#[derive(Clone, Debug)]
enum Frames {
    Static(&'static [&'static str]),
    Shared(Arc<[String]>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmptySpinnerFrames;
impl fmt::Display for EmptySpinnerFrames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("spinner needs at least one frame")
    }
}
impl std::error::Error for EmptySpinnerFrames {}
impl SpinnerAnimation {
    pub fn new(
        frames: impl IntoIterator<Item = impl Into<String>>,
        interval: TickInterval,
    ) -> Result<Self, EmptySpinnerFrames> {
        let frames: Vec<String> = frames.into_iter().map(Into::into).collect();
        if frames.is_empty() {
            return Err(EmptySpinnerFrames);
        }
        Ok(Self {
            frames: Frames::Shared(frames.into()),
            interval,
        })
    }
    pub fn len(&self) -> usize {
        match &self.frames {
            Frames::Static(frames) => frames.len(),
            Frames::Shared(frames) => frames.len(),
        }
    }
    /// A validated animation always contains at least one frame.
    pub fn is_empty(&self) -> bool {
        false
    }
    #[inline]
    pub fn frame(&self, index: usize) -> Option<&str> {
        match &self.frames {
            Frames::Static(frames) => frames.get(index).copied(),
            Frames::Shared(frames) => frames.get(index).map(String::as_str),
        }
    }
    pub fn interval(&self) -> TickInterval {
        self.interval
    }
}

/// Frame data and periods from pinned Bubbles; see THIRD_PARTY_NOTICES.md.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpinnerPreset {
    Line,
    Dot,
    MiniDot,
    Jump,
    Pulse,
    Points,
    Globe,
    Moon,
    Monkey,
    Meter,
    Hamburger,
    Ellipsis,
}
impl SpinnerPreset {
    pub const ALL: [Self; 12] = [
        Self::Line,
        Self::Dot,
        Self::MiniDot,
        Self::Jump,
        Self::Pulse,
        Self::Points,
        Self::Globe,
        Self::Moon,
        Self::Monkey,
        Self::Meter,
        Self::Hamburger,
        Self::Ellipsis,
    ];
    pub fn animation(self) -> SpinnerAnimation {
        let (frames, fps): (&'static [&'static str], u32) = match self {
            Self::Line => (&["|", "/", "-", "\\"], 10),
            Self::Dot => (&["⣾ ", "⣽ ", "⣻ ", "⢿ ", "⡿ ", "⣟ ", "⣯ ", "⣷ "], 10),
            Self::MiniDot => (&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"], 12),
            Self::Jump => (&["⢄", "⢂", "⢁", "⡁", "⡈", "⡐", "⡠"], 10),
            Self::Pulse => (&["█", "▓", "▒", "░"], 8),
            Self::Points => (&["∙∙∙", "●∙∙", "∙●∙", "∙∙●"], 7),
            Self::Globe => (&["🌍", "🌎", "🌏"], 4),
            Self::Moon => (&["🌑", "🌒", "🌓", "🌔", "🌕", "🌖", "🌗", "🌘"], 8),
            Self::Monkey => (&["🙈", "🙉", "🙊"], 3),
            Self::Meter => (&["▱▱▱", "▰▱▱", "▰▰▱", "▰▰▰", "▰▰▱", "▰▱▱", "▱▱▱"], 7),
            Self::Hamburger => (&["☱", "☲", "☴", "☲"], 3),
            Self::Ellipsis => (&["", ".", "..", "..."], 3),
        };
        SpinnerAnimation {
            frames: Frames::Static(frames),
            interval: TickInterval::new(Duration::from_secs(1) / fps)
                .expect("positive preset period"),
        }
    }
}

/// Initially paused. Use a schedule subscription or explicit tick(theme) calls.
/// Live state is not Clone, so its identity cannot accidentally be duplicated.
#[derive(Debug)]
pub struct Spinner {
    clock: Stopwatch,
    animation: SpinnerAnimation,
    frame: usize,
    manual: usize,
    pub style: Option<Style>,
    label: Option<String>,
}
impl Default for Spinner {
    fn default() -> Self {
        Self::new(SpinnerPreset::Line.animation())
    }
}
impl Spinner {
    pub fn new(animation: SpinnerAnimation) -> Self {
        Self::new_at(animation, Instant::now())
    }
    pub fn new_at(animation: SpinnerAnimation, at: Instant) -> Self {
        let mut clock = Stopwatch::new_at(at);
        clock.set_interval_at(animation.interval, at);
        Self {
            clock,
            animation,
            frame: 0,
            manual: 0,
            style: None,
            label: None,
        }
    }
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn animation(&self) -> &SpinnerAnimation {
        &self.animation
    }
    pub fn frame_index(&self) -> usize {
        self.frame
    }
    pub fn id(&self) -> ClockId {
        self.clock.id()
    }
    pub fn running(&self) -> bool {
        self.clock.running()
    }
    pub fn start(&mut self) -> bool {
        self.start_at(Instant::now())
    }
    pub fn start_at(&mut self, at: Instant) -> bool {
        self.clock.start_at(at).changed()
    }
    pub fn stop(&mut self) -> bool {
        self.stop_at(Instant::now())
    }
    pub fn stop_at(&mut self, at: Instant) -> bool {
        let changed = self.clock.stop_at(at).changed();
        self.synchronize();
        changed
    }
    pub fn reset(&mut self) -> bool {
        self.reset_at(Instant::now())
    }
    pub fn reset_at(&mut self, at: Instant) -> bool {
        if !self.clock.reset_at(at).changed() {
            return false;
        }
        self.frame = 0;
        self.manual = 0;
        true
    }
    pub fn set_animation(&mut self, animation: SpinnerAnimation) -> bool {
        self.set_animation_at(animation, Instant::now())
    }
    /// Reset the phase and invalidate old ticks, preserving running/pause state.
    pub fn set_animation_at(&mut self, animation: SpinnerAnimation, at: Instant) -> bool {
        if !self.reset_at(at) {
            return false;
        }
        self.clock.set_interval_at(animation.interval, at);
        self.animation = animation;
        true
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
        map: impl Fn(ClockTick) -> M + 'static,
    ) -> Option<Subscription<M>> {
        self.schedule(theme)
            .map(|schedule| schedule.subscription(map))
    }
    /// True means a current, later clock observation was accepted, even if it
    /// spans an exact whole cycle and leaves the same visible frame.
    pub fn update(&mut self, tick: ClockTick) -> bool {
        if !self.clock.update(tick).changed() {
            return false;
        }
        self.synchronize();
        true
    }
    fn synchronize(&mut self) {
        let phase = (self.clock.elapsed().as_nanos() / self.animation.interval.get().as_nanos())
            % self.animation.len() as u128;
        self.frame = (phase as usize + self.manual) % self.animation.len();
    }
    /// Explicit owner-controlled one-frame advance, independent of timer cadence.
    pub fn tick(&mut self, theme: &Theme) {
        if !theme.reduced_motion {
            self.manual = (self.manual + 1) % self.animation.len();
            self.synchronize();
        }
    }
    #[inline]
    pub fn frame_text(&self, theme: &Theme) -> &str {
        if theme.reduced_motion {
            "*"
        } else {
            self.animation
                .frame(self.frame)
                .expect("validated spinner phase")
        }
    }
    /// Plain output borrows an unlabeled frame; a label needs owned storage.
    /// Normalize trailing ASCII spaces only at the label boundary.
    #[inline]
    pub fn text(&self, theme: &Theme) -> Cow<'_, str> {
        let frame = self.frame_text(theme);
        match self.label.as_deref().filter(|label| !label.is_empty()) {
            Some(label) => labeled_text(frame, label),
            None => Cow::Borrowed(frame),
        }
    }
}

// Isolate owned formatting from the inline, allocation-free frame lookup.
#[inline(never)]
fn labeled_text<'a>(frame: &'a str, label: &'a str) -> Cow<'a, str> {
    let frame = frame.trim_end_matches(' ');
    if frame.is_empty() {
        Cow::Borrowed(label)
    } else {
        Cow::Owned(format!("{frame} {label}"))
    }
}
impl Component for Spinner {
    type Message = Action;
    fn event(&mut self, _: &Input, _: bool) -> Response<Action> {
        Response::ignored()
    }
    fn view(&self, theme: &Theme, _: bool) -> View {
        View::styled(self.text(theme), self.style.unwrap_or(theme.accent))
    }
}
