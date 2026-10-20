//! Themed static/animated progress, independent of runtime and terminal I/O.
//! Behavior informed by pinned Bubbles progress; see ADR 0046 and notices.
use crate::{ClockId, ClockSchedule, ClockTick, Stopwatch, TickInterval};
use compact_str::{CompactString, format_compact};
use matea_core::{Component, Input, Response, Subscription};
use matea_motion::{Motion, MotionError, Spring};
use matea_render::{Rect, Size, Surface, View, sanitize};
use matea_style::{Color, Gradient, Style, Theme};
use std::{
    fmt,
    iter::FusedIterator,
    ops::Range,
    time::{Duration, Instant},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const FRAME: Duration = Duration::from_nanos(16_666_666);
pub const PROGRESS_HALF_BLOCK: char = '▌';
pub const PROGRESS_FULL_BLOCK: char = '█';
pub const PROGRESS_EMPTY_BLOCK: char = '░';

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressError {
    NonFinitePercent,
    InvalidGlyph,
    Motion(MotionError),
}
impl fmt::Display for ProgressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinitePercent => f.write_str("progress percentage must be finite"),
            Self::InvalidGlyph => {
                f.write_str("progress glyphs must be printable one-cell characters")
            }
            Self::Motion(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for ProgressError {}
impl From<MotionError> for ProgressError {
    fn from(value: MotionError) -> Self {
        Self::Motion(value)
    }
}

/// Exactly one active color policy; assigning a new policy replaces the old one.
pub enum ProgressFill {
    Theme,
    Solid(Color),
    Gradient {
        gradient: Gradient,
        scaled: bool,
    },
    /// Arguments are displayed fraction and horizontal position, both in 0..=1.
    ColorFn(Box<dyn Fn(f64, f64) -> Color>),
}
impl fmt::Debug for ProgressFill {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Theme => f.write_str("Theme"),
            Self::Solid(c) => f.debug_tuple("Solid").field(c).finish(),
            Self::Gradient { gradient, scaled } => f
                .debug_struct("Gradient")
                .field("gradient", gradient)
                .field("scaled", scaled)
                .finish(),
            Self::ColorFn(_) => f.write_str("ColorFn(..)"),
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct ProgressStyles {
    pub full: Option<Style>,
    pub empty: Option<Style>,
    pub percentage: Option<Style>,
}
type PercentageFormatter = Box<dyn Fn(f64) -> String>;

/// A live animation is not Clone; its clock identity cannot be duplicated.
pub struct Progress {
    clock: Stopwatch,
    spring: Spring,
    motion: Motion,
    target: f64,
    width: u16,
    full: char,
    empty: char,
    reduced: bool,
    pub fill: ProgressFill,
    pub styles: ProgressStyles,
    pub show_percentage: bool,
    formatter: Option<PercentageFormatter>,
}
impl fmt::Debug for Progress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Progress")
            .field("clock", &self.clock)
            .field("motion", &self.motion)
            .field("target", &self.target)
            .field("width", &self.width)
            .field("fill", &self.fill)
            .field("styles", &self.styles)
            .field("show_percentage", &self.show_percentage)
            .field("reduced", &self.reduced)
            .finish_non_exhaustive()
    }
}
impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}
impl Progress {
    pub fn new() -> Self {
        Self::new_at(Instant::now())
    }
    pub fn new_at(at: Instant) -> Self {
        let mut clock = Stopwatch::new_at(at);
        clock.set_interval_at(TickInterval::new(FRAME).expect("positive frame period"), at);
        Self {
            clock,
            spring: Spring::new(FRAME, 18.0, 1.0).expect("default spring"),
            motion: Motion::default(),
            target: 0.0,
            width: 40,
            full: PROGRESS_HALF_BLOCK,
            empty: PROGRESS_EMPTY_BLOCK,
            reduced: false,
            fill: ProgressFill::Theme,
            styles: ProgressStyles::default(),
            show_percentage: true,
            formatter: None,
        }
    }
    pub fn id(&self) -> ClockId {
        self.clock.id()
    }
    pub fn width(&self) -> u16 {
        self.width
    }
    pub fn set_width(&mut self, width: u16) {
        self.width = width;
    }
    pub fn target(&self) -> f64 {
        self.target
    }
    /// The unrounded spring state may overshoot 0..=1; rendering clamps it.
    pub fn motion(&self) -> Motion {
        self.motion
    }
    pub fn is_animating(&self) -> bool {
        self.clock.running()
    }
    pub fn glyphs(&self) -> (char, char) {
        (self.full, self.empty)
    }
    pub fn set_glyphs(&mut self, full: char, empty: char) -> Result<(), ProgressError> {
        if [full, empty]
            .iter()
            .any(|c| c.is_control() || c.width() != Some(1))
        {
            return Err(ProgressError::InvalidGlyph);
        }
        (self.full, self.empty) = (full, empty);
        Ok(())
    }
    /// Formatter receives the clamped fraction (not the value multiplied by 100).
    pub fn set_percentage_formatter(&mut self, formatter: impl Fn(f64) -> String + 'static) {
        self.formatter = Some(Box::new(formatter));
    }
    pub fn reset_percentage_formatter(&mut self) {
        self.formatter = None;
    }
    pub fn set_spring(&mut self, frequency: f64, damping: f64) -> Result<(), ProgressError> {
        if frequency <= 0.0 {
            return Err(MotionError::InvalidParameters.into());
        }
        let spring = Spring::new(FRAME, frequency, damping)?;
        self.spring = spring;
        Ok(())
    }
    pub fn set_percent(&mut self, percent: f64) -> Result<bool, ProgressError> {
        self.set_percent_at(percent, Instant::now())
    }
    pub fn set_percent_at(&mut self, percent: f64, at: Instant) -> Result<bool, ProgressError> {
        let target = finite_percent(percent)?;
        let running = !self.reduced && !settled(self.motion, target);
        if !self.clock.reset_running_at(at, running).changed() {
            return Ok(false);
        }
        self.target = target;
        if !running {
            self.motion = Motion::at_rest(target)?;
        }
        Ok(true)
    }
    /// Adds to the target; rejects nonfinite deltas and overflow before mutation.
    pub fn increment(&mut self, delta: f64) -> Result<bool, ProgressError> {
        self.set_percent(self.target + delta)
    }
    pub fn decrement(&mut self, delta: f64) -> Result<bool, ProgressError> {
        self.increment(-delta)
    }
    pub fn set_reduced_motion(&mut self, reduced: bool) -> bool {
        self.set_reduced_motion_at(reduced, Instant::now())
    }
    pub fn set_reduced_motion_at(&mut self, reduced: bool, at: Instant) -> bool {
        if self.reduced == reduced {
            return false;
        }
        let running = !reduced && !settled(self.motion, self.target);
        if !self.clock.reset_running_at(at, running).changed() {
            return false;
        }
        self.reduced = reduced;
        if reduced {
            self.motion = Motion::at_rest(self.target).expect("validated target");
        }
        true
    }
    pub fn schedule(&self, theme: &Theme) -> Option<ClockSchedule> {
        if self.reduced || theme.reduced_motion {
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
    /// One fixed spring step per accepted later observation; no catch-up loop.
    /// Invalid observations and arithmetic errors leave the motion unchanged.
    pub fn update(&mut self, tick: ClockTick) -> Result<bool, ProgressError> {
        if !self.clock.accepts(tick) {
            return Ok(false);
        }
        let next = self.spring.step(self.motion, self.target)?;
        if !self.clock.update(tick).changed() {
            return Ok(false);
        }
        self.motion = next;
        if settled(next, self.target) {
            self.motion = Motion::at_rest(self.target)?;
            self.clock.stop_at(tick.at());
        }
        Ok(true)
    }
    /// Borrow the bar configuration. The default short label uses inline UTF-8
    /// storage; custom formatting or long labels may allocate.
    pub fn frame_as<'a>(
        &'a self,
        percent: f64,
        theme: &Theme,
    ) -> Result<ProgressFrame<'a>, ProgressError> {
        let percent = finite_percent(percent)?;
        let mut label = if !self.show_percentage {
            CompactString::default()
        } else if let Some(format) = &self.formatter {
            format(percent).into()
        } else {
            format_compact!(" {:3.0}%", percent * 100.0)
        };
        if label.contains(['\n', '\r']) {
            label = label.replace(['\n', '\r'], " ").into();
        }
        if label.chars().any(char::is_control) {
            label = sanitize(&label).into();
        }
        let mut label_width = 0;
        let mut end = 0;
        for (i, g) in label.grapheme_indices(true) {
            let next = label_width + g.width();
            if next > usize::from(self.width) {
                break;
            }
            label_width = next;
            end = i + g.len();
        }
        label.truncate(end);
        let width = usize::from(self.width) - label_width;
        let filled = (width as f64 * percent).round() as usize;
        Ok(ProgressFrame {
            progress: self,
            percent,
            width,
            filled,
            label,
            full_style: self.styles.full.unwrap_or(theme.accent),
            empty_style: self.styles.empty.unwrap_or(theme.muted),
            label_style: self.styles.percentage.unwrap_or(theme.text),
        })
    }
    pub fn frame<'a>(&'a self, theme: &Theme) -> ProgressFrame<'a> {
        let percent = if self.reduced || theme.reduced_motion {
            self.target
        } else {
            self.motion.position
        };
        self.frame_as(percent, theme)
            .expect("validated progress state")
    }
    pub fn view_as(&self, percent: f64, theme: &Theme) -> Result<View, ProgressError> {
        Ok(self.frame_as(percent, theme)?.view())
    }
    pub fn view(&self, theme: &Theme) -> View {
        self.frame(theme).view()
    }
}
fn finite_percent(percent: f64) -> Result<f64, ProgressError> {
    if percent.is_finite() {
        Ok(percent.clamp(0.0, 1.0))
    } else {
        Err(ProgressError::NonFinitePercent)
    }
}
fn settled(state: Motion, target: f64) -> bool {
    (state.position - target).abs() < 0.001 && state.velocity.abs() < 0.01
}
impl Component for Progress {
    type Message = crate::Action;
    fn event(&mut self, _: &Input, _: bool) -> Response<Self::Message> {
        Response::ignored()
    }
    fn view(&self, theme: &Theme, _: bool) -> View {
        self.view(theme)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressCell {
    pub symbol: char,
    pub style: Style,
}
#[derive(Debug)]
pub struct ProgressFrame<'a> {
    progress: &'a Progress,
    percent: f64,
    width: usize,
    filled: usize,
    label: CompactString,
    full_style: Style,
    empty_style: Style,
    label_style: Style,
}
impl ProgressFrame<'_> {
    pub fn bar_width(&self) -> usize {
        self.width
    }
    pub fn filled_width(&self) -> usize {
        self.filled
    }
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn label_style(&self) -> Style {
        self.label_style
    }
    pub fn cells(&self) -> ProgressCells<'_, '_> {
        ProgressCells {
            frame: self,
            indices: 0..self.width,
        }
    }
    pub fn cell_at(&self, index: usize) -> Option<ProgressCell> {
        if index >= self.width {
            return None;
        }
        if index >= self.filled {
            return Some(ProgressCell {
                symbol: self.progress.empty,
                style: self.empty_style,
            });
        }
        let half = self.progress.full == PROGRESS_HALF_BLOCK;
        let mut style = self.full_style;
        match &self.progress.fill {
            ProgressFill::Theme => {}
            ProgressFill::Solid(c) => style.fg = *c,
            ProgressFill::Gradient { gradient, scaled } => {
                let multiplier = if half { 2 } else { 1 };
                let count = if *scaled { self.filled } else { self.width } * multiplier;
                style.fg = gradient
                    .color_at(index * multiplier, count)
                    .expect("filled sample");
                if half {
                    style.bg = gradient
                        .color_at(index * multiplier + 1, count)
                        .expect("half sample");
                }
            }
            ProgressFill::ColorFn(color) => {
                let current = index as f64 / self.width as f64;
                style.fg = color(self.percent, current);
                if half {
                    style.bg = color(self.percent, (current + 0.5 / self.width as f64).min(1.0));
                }
            }
        }
        Some(ProgressCell {
            symbol: self.progress.full,
            style,
        })
    }
    pub fn view(&self) -> View {
        if self.filled != 0
            && matches!(
                self.progress.fill,
                ProgressFill::Gradient { .. } | ProgressFill::ColorFn(_)
            )
        {
            // Each colored glyph was already a separate text child. Retain
            // that boundary; the uniform empty run and label keep their own
            // whole-text grapheme segmentation (including regional indicators).
            let mut surface = Surface::new(Size::new(self.progress.width, 1))
                .expect("one u16-width row fits the surface limit");
            for (x, cell) in self.cells().take(self.filled).enumerate() {
                let mut utf8 = [0; 4];
                surface.canvas(Rect::new(x as u16, 0, 1, 1)).text(
                    x as u16,
                    0,
                    cell.symbol.encode_utf8(&mut utf8),
                    cell.style,
                );
            }
            let empty_width = (self.width - self.filled) as u16;
            if empty_width != 0 {
                surface
                    .canvas(Rect::new(self.filled as u16, 0, empty_width, 1))
                    .text(
                        self.filled as u16,
                        0,
                        &repeat(self.progress.empty, usize::from(empty_width)),
                        self.empty_style,
                    );
            }
            if !self.label.is_empty() {
                surface
                    .canvas(Rect::new(
                        self.width as u16,
                        0,
                        self.progress.width - self.width as u16,
                        1,
                    ))
                    .text(self.width as u16, 0, &self.label, self.label_style);
            }
            return View::surface(surface).width(self.progress.width).height(1);
        }
        let mut children = Vec::new();
        if self.filled != 0 {
            if matches!(
                self.progress.fill,
                ProgressFill::Theme | ProgressFill::Solid(_)
            ) {
                let cell = self.cell_at(0).expect("nonempty fill");
                children.push(
                    View::styled(repeat(cell.symbol, self.filled), cell.style)
                        .width(self.filled as u16),
                );
            } else {
                children.extend(self.cells().take(self.filled).map(|c| {
                    let mut utf8 = [0; 4];
                    View::styled(c.symbol.encode_utf8(&mut utf8) as &str, c.style).width(1)
                }));
            }
        }
        if self.width > self.filled {
            children.push(
                View::styled(
                    repeat(self.progress.empty, self.width - self.filled),
                    self.empty_style,
                )
                .width((self.width - self.filled) as u16),
            );
        }
        if !self.label.is_empty() {
            children.push(
                View::styled(self.label.as_str(), self.label_style)
                    .width(self.progress.width - self.width as u16),
            );
        }
        View::row(children).width(self.progress.width).height(1)
    }
}
fn repeat(symbol: char, count: usize) -> String {
    let mut text = String::with_capacity(symbol.len_utf8() * count);
    text.extend(std::iter::repeat_n(symbol, count));
    text
}
#[derive(Clone, Debug)]
pub struct ProgressCells<'f, 'p> {
    frame: &'f ProgressFrame<'p>,
    indices: Range<usize>,
}
impl Iterator for ProgressCells<'_, '_> {
    type Item = ProgressCell;
    fn next(&mut self) -> Option<Self::Item> {
        self.indices.next().and_then(|i| self.frame.cell_at(i))
    }
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.indices.nth(n).and_then(|i| self.frame.cell_at(i))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indices.size_hint()
    }
}
impl DoubleEndedIterator for ProgressCells<'_, '_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.indices.next_back().and_then(|i| self.frame.cell_at(i))
    }
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.indices.nth_back(n).and_then(|i| self.frame.cell_at(i))
    }
}
impl ExactSizeIterator for ProgressCells<'_, '_> {}
impl FusedIterator for ProgressCells<'_, '_> {}
