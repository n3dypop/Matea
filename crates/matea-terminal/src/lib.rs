#![deny(unsafe_code)]
//! Exclusive terminal lifecycle and a replaceable I/O backend.
mod frame;
pub mod protocol;
pub use frame::FrameEncoder;
// sigaction has no safe standard-library wrapper; this is the only unsafe module.
#[cfg(unix)]
#[allow(unsafe_code)]
mod signals_unix;
use crossterm::{
    cursor::{Hide, Show},
    event::{
        self, DisableBracketedPaste, DisableFocusChange, EnableBracketedPaste, EnableFocusChange,
        KeyEventKind, KeyModifiers,
    },
    execute, queue,
    style::{Attribute, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use matea_core::{Event, Input, Key, KeyCode, Model, Runtime};
use matea_render::{Size, Surface};
use matea_style::{Border, Color, ColorLevel, Style, Theme};
use std::{
    io::{self, IsTerminal, Write},
    sync::atomic::{AtomicBool, Ordering},
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
}
impl Default for RunOptions {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            capabilities: Capabilities::detect(),
        }
    }
}
#[derive(Clone, Debug)]
pub enum TerminalEvent {
    Input(Input),
    Resize(Size),
    Interrupt,
}

/// Only the transport owns terminal side effects. One poller per backend/session.
pub trait Backend {
    fn enter(&mut self) -> io::Result<()>;
    fn restore(&mut self) -> io::Result<()>;
    fn size(&self) -> io::Result<Size>;
    fn poll(&mut self, timeout: Duration) -> io::Result<Option<TerminalEvent>>;
    fn draw(&mut self, frame: &Surface, previous: Option<&Surface>) -> io::Result<()>;
}
struct Session<B: Backend> {
    backend: B,
    active: bool,
}
impl<B: Backend> Session<B> {
    fn enter(backend: B) -> io::Result<Self> {
        let mut session = Self {
            backend,
            active: true,
        };
        if let Err(error) = session.backend.enter() {
            return match session.restore() {
                Ok(()) => Err(error),
                Err(cleanup) => Err(io::Error::other(format!(
                    "{error}; terminal restoration also failed: {cleanup}"
                ))),
            };
        }
        Ok(session)
    }
    fn restore(&mut self) -> io::Result<()> {
        if self.active {
            self.backend.restore()?;
            self.active = false;
        }
        Ok(())
    }
}
impl<B: Backend> Drop for Session<B> {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

pub fn run<A: Model>(model: A) -> Result<(), Error> {
    run_with_options(model, RunOptions::default())
}
pub fn run_with_options<A: Model>(model: A, mut options: RunOptions) -> Result<(), Error> {
    options.theme.reduced_motion |= options.capabilities.reduced_motion;
    if !options.capabilities.unicode {
        options.theme.border_kind = Border::Ascii;
    }
    let native = Native::new(options.capabilities)?;
    run_with_backend(model, options.theme, native)
}
/// Drive the same loop with a mock backend in integration tests or an alternate transport.
pub fn run_with_backend<A: Model, B: Backend>(
    model: A,
    theme: Theme,
    backend: B,
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
            if let Some(event) = session.backend.poll(Duration::from_millis(16))? {
                match event {
                    TerminalEvent::Interrupt => break,
                    TerminalEvent::Input(Input::Key(k))
                        if k.ctrl && matches!(k.code, KeyCode::Char('c' | 'C')) =>
                    {
                        break;
                    }
                    TerminalEvent::Input(input) => runtime.dispatch(Event::Input(input))?,
                    TerminalEvent::Resize(size) => runtime.dispatch(Event::Resize(size))?,
                }
                dirty = true;
            } else {
                dirty = false;
            }
            let now = Instant::now();
            dirty |= runtime.advance(now.duration_since(last))?;
            last = now;
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

static EXCLUSIVE: AtomicBool = AtomicBool::new(false);
struct Native {
    out: io::Stdout,
    encoded: Vec<u8>,
    encoder: FrameEncoder,
    caps: Capabilities,
    entered: bool,
    raw: bool,
    #[cfg(unix)]
    signals: Option<signals_unix::SignalGuard>,
}
impl Native {
    fn new(caps: Capabilities) -> io::Result<Self> {
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err(io::Error::other(
                "interactive Matea requires TTY stdin/stdout; use an example's --snapshot option for plain output",
            ));
        }
        if EXCLUSIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(io::Error::other(
                "another Matea terminal session is already active",
            ));
        }
        Ok(Self {
            out: io::stdout(),
            encoded: Vec::new(),
            encoder: FrameEncoder::default(),
            caps,
            entered: false,
            raw: false,
            #[cfg(unix)]
            signals: None,
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
        self.entered = true;
        // Install the resize listener before the first frame is observable.
        // Otherwise a SIGWINCH between first paint and first poll can be lost.
        let _ = event::poll(Duration::ZERO)?;
        execute!(
            self.out,
            EnterAlternateScreen,
            Hide,
            EnableBracketedPaste,
            EnableFocusChange,
            Clear(ClearType::All)
        )
    }
    fn restore(&mut self) -> io::Result<()> {
        let mut error = None;
        if self.entered {
            // Attempt every operation even if an earlier write fails.
            for result in [
                execute!(self.out, ResetColor),
                execute!(self.out, SetAttribute(Attribute::Reset)),
                execute!(self.out, DisableBracketedPaste),
                execute!(self.out, DisableFocusChange),
                execute!(self.out, Show),
                execute!(self.out, LeaveAlternateScreen),
                self.out.flush(),
            ] {
                if let Err(e) = result {
                    error.get_or_insert(e);
                }
            }
            if error.is_none() {
                self.entered = false;
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
        Ok(translate(event::read()?))
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
        EXCLUSIVE.store(false, Ordering::Release);
    }
}

fn translate(event: event::Event) -> Option<TerminalEvent> {
    use event::{Event as E, KeyCode as C};
    Some(match event {
        E::Resize(w, h) => TerminalEvent::Resize(Size::new(w, h)),
        E::Paste(s) => TerminalEvent::Input(Input::Paste(s)),
        E::FocusGained => TerminalEvent::Input(Input::Focus(true)),
        E::FocusLost => TerminalEvent::Input(Input::Focus(false)),
        E::Key(k) if k.kind != KeyEventKind::Release => {
            let code = match k.code {
                C::Char(c) => KeyCode::Char(c),
                C::Enter => KeyCode::Enter,
                C::Esc => KeyCode::Esc,
                C::Backspace => KeyCode::Backspace,
                C::Delete => KeyCode::Delete,
                C::Left => KeyCode::Left,
                C::Right => KeyCode::Right,
                C::Up => KeyCode::Up,
                C::Down => KeyCode::Down,
                C::Home => KeyCode::Home,
                C::End => KeyCode::End,
                C::Tab => KeyCode::Tab,
                C::BackTab => KeyCode::BackTab,
                C::PageUp => KeyCode::PageUp,
                C::PageDown => KeyCode::PageDown,
                _ => return None,
            };
            TerminalEvent::Input(Input::Key(Key {
                code,
                ctrl: k.modifiers.contains(KeyModifiers::CONTROL),
                alt: k.modifiers.contains(KeyModifiers::ALT),
                shift: k.modifiers.contains(KeyModifiers::SHIFT),
            }))
        }
        _ => return None,
    })
}
fn color(c: Color) -> crossterm::style::Color {
    use crossterm::style::Color as C;
    match c {
        Color::Default => C::Reset,
        Color::Rgb(r, g, b) => C::Rgb { r, g, b },
        Color::Indexed(n) => match n {
            0 => C::Black,
            1 => C::DarkRed,
            2 => C::DarkGreen,
            3 => C::DarkYellow,
            4 => C::DarkBlue,
            5 => C::DarkMagenta,
            6 => C::DarkCyan,
            7 => C::Grey,
            8 => C::DarkGrey,
            9 => C::Red,
            10 => C::Green,
            11 => C::Yellow,
            12 => C::Blue,
            13 => C::Magenta,
            14 => C::Cyan,
            15 => C::White,
            _ => C::AnsiValue(n),
        },
    }
}
fn set_style(w: &mut impl Write, s: Style) -> io::Result<()> {
    queue!(w, SetAttribute(Attribute::Reset))?;
    if s.fg != Color::Default {
        queue!(w, SetForegroundColor(color(s.fg)))?;
    }
    if s.bg != Color::Default {
        queue!(w, SetBackgroundColor(color(s.bg)))?;
    }
    if s.bold {
        queue!(w, SetAttribute(Attribute::Bold))?;
    }
    if s.underline {
        queue!(w, SetAttribute(Attribute::Underlined))?;
    }
    if s.reverse {
        queue!(w, SetAttribute(Attribute::Reverse))?;
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
    use matea_core::{Effect, ViewContext};
    use matea_render::View;
    use std::sync::{Arc, Mutex};
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
        fn poll(&mut self, _: Duration) -> io::Result<Option<TerminalEvent>> {
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
            KeyModifiers::NONE,
            KeyEventKind::Release,
        );
        assert!(translate(event::Event::Key(event)).is_none());
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
