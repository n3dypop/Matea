#![deny(unsafe_code)]
//! Exclusive terminal lifecycle and a replaceable I/O backend.
mod ansi;
mod frame;
mod input;
pub use input::translate_event;
mod modes;
mod session;
pub use modes::{InputOptions, KeyboardFlags, KeyboardMode};
use modes::{ModeLeases, apply_mode};
use session::{Exclusive, Session};
pub mod protocol;
pub use frame::FrameEncoder;
// sigaction has no safe standard-library wrapper; this is the only unsafe module.
#[cfg(unix)]
#[allow(unsafe_code)]
mod signals_unix;
use crossterm::{
    event, execute,
    terminal::{self, Clear, ClearType},
};
use matea_core::{Event, Input, KeyCode, Model, Modifiers, Runtime};
use matea_render::{Size, Surface};
use matea_style::{Border, Color, ColorLevel, Style, Theme};
use std::{
    io::{self, IsTerminal, Write},
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capabilities {
    pub color: ColorLevel,
    pub unicode: bool,
    pub reduced_motion: bool,
}
impl Capabilities {
    /// Conservative environment hints; no graphics protocol is claimed or queried.
    pub fn from_environment(get: impl Fn(&str) -> Option<String>) -> Self {
        let term = get("TERM").unwrap_or_default();
        let dumb = term == "dumb";
        let color = if get("NO_COLOR").is_some() || dumb {
            ColorLevel::None
        } else if get("COLORTERM").is_some_and(|v| v == "truecolor" || v == "24bit") {
            ColorLevel::TrueColor
        } else if term.contains("256color") {
            ColorLevel::Ansi256
        } else {
            ColorLevel::Ansi16
        };
        Self {
            color,
            unicode: !dumb,
            reduced_motion: get("MATEA_REDUCED_MOTION").is_some_and(|v| v != "0"),
        }
    }
    pub fn detect() -> Self {
        Self::from_environment(|key| std::env::var(key).ok())
    }
}
#[derive(Clone, Debug)]
pub struct RunOptions {
    pub theme: Theme,
    pub capabilities: Capabilities,
    pub input: InputOptions,
    pub ctrl_c: CtrlCBehavior,
}
impl Default for RunOptions {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            capabilities: Capabilities::detect(),
            input: InputOptions::default(),
            ctrl_c: CtrlCBehavior::Quit,
        }
    }
}
#[derive(Clone, Debug)]
pub enum TerminalEvent {
    Input(Input),
    Resize(Size),
    Interrupt,
}
/// Policy for Ctrl-C key-down. OS signals still trigger restoration and exit;
/// releases are forwarded. Shift is allowed, other command modifiers are not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CtrlCBehavior {
    #[default]
    Quit,
    Deliver,
}
fn ctrl_c(key: matea_core::Key) -> bool {
    key.is_down()
        && key.modifiers.difference(Modifiers::SHIFT) == Modifiers::CONTROL
        && matches!(key.code, KeyCode::Char('c' | 'C'))
}

/// Only the transport owns terminal side effects. One poller per backend/session.
pub trait Backend {
    fn enter(&mut self) -> io::Result<()>;
    fn restore(&mut self) -> io::Result<()>;
    fn size(&self) -> io::Result<Size>;
    fn poll(&mut self, timeout: Duration) -> io::Result<Option<TerminalEvent>>;
    fn draw(&mut self, frame: &Surface, previous: Option<&Surface>) -> io::Result<()>;
}
pub fn run<A: Model>(model: A) -> Result<(), Error> {
    run_with_options(model, RunOptions::default())
}
pub fn run_with_options<A: Model>(model: A, mut options: RunOptions) -> Result<(), Error> {
    options.theme.reduced_motion |= options.capabilities.reduced_motion;
    if !options.capabilities.unicode {
        options.theme.border_kind = Border::Ascii;
    }
    let native = Native::new(options.capabilities, options.input)?;
    run_with_backend_and_ctrl_c(model, options.theme, native, options.ctrl_c)
}
/// Drive the same loop with a mock backend in integration tests or an alternate transport.
pub fn run_with_backend<A: Model, B: Backend>(
    model: A,
    theme: Theme,
    backend: B,
) -> Result<(), Error> {
    run_with_backend_and_ctrl_c(model, theme, backend, CtrlCBehavior::Quit)
}
/// Configure key policy for another backend; mode acquisition stays with it.
pub fn run_with_backend_and_ctrl_c<A: Model, B: Backend>(
    model: A,
    theme: Theme,
    backend: B,
    behavior: CtrlCBehavior,
) -> Result<(), Error> {
    let mut session = Session::enter(backend)?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), Error> {
        let mut runtime = Runtime::new(model, session.backend.size()?, theme)?;
        let mut previous = None;
        let mut last = Instant::now();
        let mut dirty = true;
        while !runtime.is_quitting() {
            if runtime.take_worker_panic_notice() {
                // A caught worker panic runs the normal stderr hook first. Even
                // canceled results can therefore invalidate cells/cursor/style.
                previous = None;
                dirty = true;
            }
            if dirty {
                let frame = runtime.frame()?;
                session.backend.draw(&frame, previous.as_ref())?;
                previous = Some(frame);
            }
            let timeout =
                subscription_poll_timeout(runtime.next_subscription_delay(), || last.elapsed());
            let pending = match session.backend.poll(timeout)? {
                Some(TerminalEvent::Interrupt) => break,
                Some(TerminalEvent::Input(Input::Key(k)))
                    if behavior == CtrlCBehavior::Quit && ctrl_c(k) =>
                {
                    break;
                }
                pending => pending,
            };
            // Synchronize time before input can create a new subscription.
            // Already-due subscriptions run first; interrupts still take priority.
            let now = Instant::now();
            dirty = runtime.advance(now.duration_since(last))?;
            last = now;
            if let Some(event) = pending {
                match event {
                    TerminalEvent::Interrupt => break,
                    TerminalEvent::Input(input) => runtime.dispatch(Event::Input(input))?,
                    TerminalEvent::Resize(size) => runtime.dispatch(Event::Resize(size))?,
                }
                dirty = true;
            }
            dirty |= runtime.poll_effects()?;
        }
        runtime.shutdown();
        Ok(())
    }));
    let cleanup = session.restore();
    match result {
        Err(panic) => std::panic::resume_unwind(panic),
        Ok(Err(error)) => match cleanup {
            Ok(()) => Err(error),
            Err(cleanup) => Err(io::Error::other(format!(
                "{error}; terminal restoration also failed: {cleanup}"
            ))
            .into()),
        },
        Ok(Ok(())) => {
            cleanup?;
            Ok(())
        }
    }
}

fn subscription_poll_timeout(next: Option<Duration>, spent: impl FnOnce() -> Duration) -> Duration {
    let maximum = Duration::from_millis(16);
    next.map_or(maximum, |delay| delay.saturating_sub(spent()).min(maximum))
}

static EXCLUSIVE: AtomicBool = AtomicBool::new(false);
struct Native {
    out: io::Stdout,
    encoded: Vec<u8>,
    encoder: FrameEncoder,
    caps: Capabilities,
    input: InputOptions,
    modes: ModeLeases,
    raw: bool,
    #[cfg(unix)]
    signals: Option<signals_unix::SignalGuard>,
}
impl Native {
    fn new(caps: Capabilities, input: InputOptions) -> io::Result<Exclusive<'static, Self>> {
        input.validate()?;
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err(io::Error::other(
                "interactive Matea requires TTY stdin/stdout; use an example's --snapshot option for plain output",
            ));
        }
        Exclusive::acquire(&EXCLUSIVE, || {
            Ok(Self {
                out: io::stdout(),
                encoded: Vec::new(),
                encoder: FrameEncoder::default(),
                caps,
                input,
                modes: ModeLeases::default(),
                raw: false,
                #[cfg(unix)]
                signals: None,
            })
        })
    }
}
impl Backend for Native {
    fn enter(&mut self) -> io::Result<()> {
        if terminal::is_raw_mode_enabled()? {
            return Err(io::Error::other(
                "terminal is already in raw mode; Matea requires exclusive ownership",
            ));
        }
        #[cfg(unix)]
        {
            self.signals = Some(signals_unix::SignalGuard::install()?);
        }
        terminal::enable_raw_mode()?;
        self.raw = true;
        // Install the resize listener before the first frame is observable.
        // Otherwise a SIGWINCH between first paint and first poll can be lost.
        let _ = event::poll(Duration::ZERO)?;
        self.modes.enter(self.input, |mode, enable, flags| {
            apply_mode(&mut self.out, mode, enable, flags)
        })?;
        // Native mode commands have now selected Crossterm's real transport.
        // The frame writer emits ANSI bytes only; never paint them through a
        // known legacy-console fallback. Session rolls acquired modes back.
        #[cfg(windows)]
        if !crossterm::ansi_support::supports_ansi() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Matea native frame output requires ANSI; legacy WinAPI frame rendering is not implemented",
            ));
        }
        execute!(self.out, Clear(ClearType::All))
    }
    fn restore(&mut self) -> io::Result<()> {
        let mut error = None;
        if self.modes.active() {
            // Attempt every operation even if an earlier write fails.
            for result in [
                self.modes
                    .restore(|mode, enable, flags| apply_mode(&mut self.out, mode, enable, flags)),
                self.out.flush(),
            ] {
                if let Err(e) = result {
                    error.get_or_insert(e);
                }
            }
        }
        if self.raw {
            match terminal::disable_raw_mode() {
                Ok(()) => self.raw = false,
                Err(e) => {
                    error.get_or_insert(e);
                }
            }
        }
        #[cfg(unix)]
        if let Some(signals) = &mut self.signals {
            match signals.restore() {
                Ok(()) => self.signals = None,
                Err(e) => {
                    error.get_or_insert(e);
                }
            }
        }
        error.map_or(Ok(()), Err)
    }
    fn size(&self) -> io::Result<Size> {
        let (w, h) = terminal::size()?;
        Ok(Size::new(w, h))
    }
    fn poll(&mut self, timeout: Duration) -> io::Result<Option<TerminalEvent>> {
        #[cfg(unix)]
        if signals_unix::interrupted() {
            return Ok(Some(TerminalEvent::Interrupt));
        }
        if !event::poll(timeout)? {
            return Ok(None);
        }
        Ok(Some(translate_event(event::read()?)))
    }
    fn draw(&mut self, frame: &Surface, previous: Option<&Surface>) -> io::Result<()> {
        self.encoded.clear();
        self.encoder
            .write(&mut self.encoded, frame, previous, self.caps)?;
        if self.encoded.is_empty() {
            return Ok(());
        }
        let mut out = self.out.lock();
        out.write_all(&self.encoded)?;
        out.flush()
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

fn set_style(w: &mut impl Write, s: Style) -> io::Result<()> {
    w.write_all(b"\x1b[0m")?;
    if s.fg != Color::Default {
        ansi::color(w, s.fg, false)?;
    }
    if s.bg != Color::Default {
        ansi::color(w, s.bg, true)?;
    }
    if s.bold {
        w.write_all(b"\x1b[1m")?;
    }
    if s.underline {
        w.write_all(b"\x1b[4m")?;
    }
    if s.reverse {
        w.write_all(b"\x1b[7m")?;
    }
    Ok(())
}
/// Encode damage to a writer. ASCII rows use cell damage; uncertain-width rows
/// clear and repaint atomically. No terminal ownership is acquired here.
pub fn write_frame(
    w: &mut impl Write,
    frame: &Surface,
    previous: Option<&Surface>,
    caps: Capabilities,
) -> io::Result<()> {
    FrameEncoder::default().write(w, frame, previous, caps)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subscription_wait_subtracts_render_time_and_preserves_idle_cap() {
        let maximum = Duration::from_millis(16);
        for spent in [Duration::ZERO, Duration::from_millis(1), Duration::MAX] {
            assert_eq!(subscription_poll_timeout(None, || spent), maximum);
        }
        assert_eq!(
            subscription_poll_timeout(None, || panic!("idle path must not read the clock")),
            maximum
        );
        for delay in [
            Duration::ZERO,
            Duration::from_nanos(1),
            Duration::from_millis(1),
            maximum,
            Duration::MAX,
        ] {
            for spent in [
                Duration::ZERO,
                Duration::from_nanos(1),
                Duration::from_millis(1),
                Duration::from_millis(20),
                Duration::MAX,
            ] {
                let expected = if spent >= delay {
                    Duration::ZERO
                } else {
                    (delay - spent).min(maximum)
                };
                assert_eq!(subscription_poll_timeout(Some(delay), || spent), expected);
            }
        }
    }
    #[test]
    fn fixed_attribute_sequences_match_crossterm_for_every_supported_combination() {
        use crossterm::style::{Attribute, SetAttribute};
        for mask in 0..8 {
            let style = Style {
                bold: mask & 1 != 0,
                underline: mask & 2 != 0,
                reverse: mask & 4 != 0,
                ..Style::default()
            };
            let mut actual = Vec::new();
            set_style(&mut actual, style).unwrap();
            let mut reference = Vec::new();
            ansi::command(&mut reference, SetAttribute(Attribute::Reset)).unwrap();
            for (enabled, attribute) in [
                (style.bold, Attribute::Bold),
                (style.underline, Attribute::Underlined),
                (style.reverse, Attribute::Reverse),
            ] {
                if enabled {
                    ansi::command(&mut reference, SetAttribute(attribute)).unwrap();
                }
            }
            assert_eq!(actual, reference);
        }
    }
    use matea_core::{Effect, ViewContext};
    use matea_render::View;
    use std::sync::{Arc, Mutex};
    #[test]
    fn ctrl_c_policy_delivers_releases_and_other_modifier_chords() {
        use matea_core::{Key, KeyKind};
        use std::collections::VecDeque;
        struct Keys(Arc<Mutex<Vec<Key>>>);
        impl Model for Keys {
            type Message = ();
            fn update(&mut self, event: Event<()>) -> Effect<()> {
                if let Event::Input(Input::Key(key)) = event {
                    self.0.lock().unwrap().push(key);
                    if key.is_char('q') {
                        return Effect::quit();
                    }
                }
                Effect::none()
            }
            fn view(&self, _: ViewContext<'_>) -> View {
                View::text("keys")
            }
        }
        struct Events {
            events: VecDeque<TerminalEvent>,
            restored: Arc<Mutex<bool>>,
        }
        impl Backend for Events {
            fn enter(&mut self) -> io::Result<()> {
                Ok(())
            }
            fn restore(&mut self) -> io::Result<()> {
                *self.restored.lock().unwrap() = true;
                Ok(())
            }
            fn size(&self) -> io::Result<Size> {
                Ok(Size::new(8, 1))
            }
            fn poll(&mut self, _: Duration) -> io::Result<Option<TerminalEvent>> {
                Ok(self.events.pop_front().or(Some(TerminalEvent::Interrupt)))
            }
            fn draw(&mut self, _: &Surface, _: Option<&Surface>) -> io::Result<()> {
                Ok(())
            }
        }
        let c = Key::new(KeyCode::Char('c')).with_modifiers(Modifiers::CONTROL);
        for (behavior, expected) in [(CtrlCBehavior::Quit, 2), (CtrlCBehavior::Deliver, 4)] {
            let keys = Arc::new(Mutex::new(Vec::new()));
            let restored = Arc::new(Mutex::new(false));
            let events = [
                c.with_kind(KeyKind::Release),
                c.with_modifiers(Modifiers::CONTROL | Modifiers::ALT),
                c,
                Key::new(KeyCode::Char('q')),
            ]
            .into_iter()
            .map(|k| TerminalEvent::Input(Input::Key(k)))
            .collect();
            run_with_backend_and_ctrl_c(
                Keys(keys.clone()),
                Theme::plain(),
                Events {
                    events,
                    restored: restored.clone(),
                },
                behavior,
            )
            .unwrap();
            assert_eq!(keys.lock().unwrap().len(), expected);
            assert!(*restored.lock().unwrap());
        }
        assert!(ctrl_c(
            c.with_modifiers(Modifiers::CONTROL | Modifiers::SHIFT)
        ));
        assert!(!ctrl_c(c.with_kind(KeyKind::Release)));
        assert!(!ctrl_c(
            c.with_modifiers(Modifiers::CONTROL | Modifiers::SUPER)
        ));
    }
    struct App {
        panic: bool,
    }
    impl Model for App {
        type Message = ();
        fn update(&mut self, _: Event<()>) -> Effect<()> {
            if self.panic {
                panic!("fixture");
            }
            Effect::None
        }
        fn view(&self, _: ViewContext<'_>) -> View {
            View::text("茶")
        }
    }
    struct Mock {
        log: Arc<Mutex<Vec<&'static str>>>,
        fail: &'static str,
    }
    impl Backend for Mock {
        fn enter(&mut self) -> io::Result<()> {
            self.log.lock().unwrap().push("enter");
            if self.fail == "enter" {
                Err(io::Error::other("enter failed"))
            } else {
                Ok(())
            }
        }
        fn restore(&mut self) -> io::Result<()> {
            self.log.lock().unwrap().push("restore");
            Ok(())
        }
        fn size(&self) -> io::Result<Size> {
            Ok(Size::new(4, 2))
        }
        fn poll(&mut self, timeout: Duration) -> io::Result<Option<TerminalEvent>> {
            if self.fail == "short-poll" {
                assert!(
                    timeout <= Duration::from_millis(1),
                    "1ms subscription postponed by input wait {timeout:?}"
                );
            }
            Ok(Some(TerminalEvent::Interrupt))
        }
        fn draw(&mut self, _: &Surface, _: Option<&Surface>) -> io::Result<()> {
            self.log.lock().unwrap().push("draw");
            if self.fail == "draw" {
                Err(io::Error::other("draw failed"))
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn input_wait_respects_a_short_subscription_deadline() {
        struct ShortClock;
        impl Model for ShortClock {
            type Message = ();
            fn update(&mut self, _: Event<()>) -> Effect<()> {
                Effect::none()
            }
            fn subscriptions(&self) -> Vec<matea_core::Subscription<()>> {
                vec![matea_core::Subscription::every(
                    "short",
                    Duration::from_millis(1),
                    || (),
                )]
            }
            fn view(&self, _: ViewContext<'_>) -> View {
                View::text("tick")
            }
        }
        let log = Arc::new(Mutex::new(Vec::new()));
        run_with_backend(
            ShortClock,
            Theme::plain(),
            Mock {
                log: log.clone(),
                fail: "short-poll",
            },
        )
        .unwrap();
        assert_eq!(log.lock().unwrap().last(), Some(&"restore"));
    }
    #[test]
    fn subscription_started_by_input_does_not_inherit_the_previous_input_wait() {
        struct StartClock(bool);
        impl Model for StartClock {
            type Message = ();
            fn update(&mut self, event: Event<()>) -> Effect<()> {
                match event {
                    Event::Input(_) => self.0 = true,
                    Event::Message(()) => return Effect::quit(),
                    _ => {}
                }
                Effect::none()
            }
            fn subscriptions(&self) -> Vec<matea_core::Subscription<()>> {
                if self.0 {
                    vec![matea_core::Subscription::every(
                        "new-clock",
                        Duration::from_millis(2),
                        || (),
                    )]
                } else {
                    Vec::new()
                }
            }
            fn view(&self, _: ViewContext<'_>) -> View {
                View::text("clock")
            }
        }
        struct StartInput(Arc<Mutex<usize>>);
        impl Backend for StartInput {
            fn enter(&mut self) -> io::Result<()> {
                Ok(())
            }
            fn restore(&mut self) -> io::Result<()> {
                Ok(())
            }
            fn size(&self) -> io::Result<Size> {
                Ok(Size::new(8, 1))
            }
            fn draw(&mut self, _: &Surface, _: Option<&Surface>) -> io::Result<()> {
                Ok(())
            }
            fn poll(&mut self, timeout: Duration) -> io::Result<Option<TerminalEvent>> {
                let mut calls = self.0.lock().unwrap();
                *calls += 1;
                if *calls == 1 {
                    std::thread::sleep(Duration::from_millis(10));
                    Ok(Some(TerminalEvent::Input(Input::Key(
                        matea_core::Key::new(matea_core::KeyCode::Char('s')),
                    ))))
                } else {
                    std::thread::sleep(timeout);
                    Ok(None)
                }
            }
        }
        let calls = Arc::new(Mutex::new(0));
        run_with_backend(StartClock(false), Theme::plain(), StartInput(calls.clone())).unwrap();
        assert!(
            *calls.lock().unwrap() >= 2,
            "new 2ms subscription fired using the 10ms wait before it existed"
        );
    }
    #[test]
    fn restoration_on_success_failure_and_unwind() {
        for (fail, panic) in [("", false), ("enter", false), ("draw", false), ("", true)] {
            let log = Arc::new(Mutex::new(vec![]));
            let backend = Mock {
                log: log.clone(),
                fail,
            };
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_with_backend(App { panic }, Theme::default(), backend)
            }));
            assert_eq!(log.lock().unwrap().last(), Some(&"restore"));
            if panic {
                assert!(result.is_err());
            } else {
                assert_eq!(result.unwrap().is_err(), !fail.is_empty());
            }
        }
    }
    #[test]
    fn conservative_environment_and_key_release() {
        let caps = Capabilities::from_environment(|k| {
            if k == "TERM" {
                Some("dumb".into())
            } else {
                None
            }
        });
        assert_eq!(caps.color, ColorLevel::None);
        assert!(!caps.unicode);
        let event = event::KeyEvent::new_with_kind(
            event::KeyCode::Char('a'),
            event::KeyModifiers::NONE,
            event::KeyEventKind::Release,
        );
        assert!(
            matches!(translate_event(event::Event::Key(event)),TerminalEvent::Input(Input::Key(key)) if key.kind==matea_core::KeyKind::Release)
        );
    }
    #[test]
    fn diff_transport_clears_wide_rows_and_is_idle_when_equal() {
        let a = View::text("茶ab").render(Size::new(4, 1)).unwrap();
        let b = View::text("x").render(Size::new(4, 1)).unwrap();
        let caps = Capabilities {
            color: ColorLevel::None,
            unicode: true,
            reduced_motion: true,
        };
        let mut out = Vec::new();
        write_frame(&mut out, &b, Some(&a), caps).unwrap();
        let s = String::from_utf8(out.clone()).unwrap();
        assert!(s.contains("\x1b[2K"));
        assert!(s.contains('x'));
        assert!(!s.contains('茶'));
        out.clear();
        write_frame(&mut out, &b, Some(&b), caps).unwrap();
        assert!(out.is_empty());
    }
    #[test]
    fn sparse_ascii_damage_preserves_unchanged_cells_and_styles() {
        let a = View::text("aaaa").render(Size::new(4, 1)).unwrap();
        let b = View::text("abaa").render(Size::new(4, 1)).unwrap();
        let caps = Capabilities {
            color: ColorLevel::None,
            unicode: true,
            reduced_motion: true,
        };
        let mut out = Vec::new();
        write_frame(&mut out, &b, Some(&a), caps).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("\x1b[1;2H"));
        assert!(!s.contains("\x1b[2K"));
        assert_eq!(s.chars().filter(|&c| c == 'b').count(), 1);
        assert!(!s.contains('a'));
        let bold = View::styled("abaa", Style::default().bold())
            .render(Size::new(4, 1))
            .unwrap();
        let mut out = Vec::new();
        write_frame(&mut out, &bold, Some(&b), caps).unwrap();
        assert!(String::from_utf8(out).unwrap().contains("\x1b[1m"));
    }
    #[test]
    fn resize_to_empty_still_clears_old_content() {
        let a = View::text("old").render(Size::new(4, 1)).unwrap();
        let b = View::text("").render(Size::new(0, 0)).unwrap();
        let mut out = Vec::new();
        write_frame(
            &mut out,
            &b,
            Some(&a),
            Capabilities {
                color: ColorLevel::None,
                unicode: true,
                reduced_motion: true,
            },
        )
        .unwrap();
        assert!(String::from_utf8(out).unwrap().contains("\x1b[2J"));
    }
}
