//! Opt-in facade identity and terminal event-loop probes, without terminal I/O.
#![deny(unsafe_code)]
#[allow(unsafe_code)] // Measurement-only System allocator forwarding.
#[path = "../../../tools/perf-probe/src/measurement.rs"]
mod measurement;
use matea::prelude::*;
use matea_terminal::{Backend, CtrlCBehavior, TerminalEvent};
use measurement::measure;
use std::{any::TypeId, cell::Cell, hint::black_box, io, rc::Rc, time::Duration};

#[derive(Default)]
struct DeadlineStats {
    notifications: Cell<u32>,
    draws: Cell<u32>,
    restores: Cell<u32>,
}
struct DeadlineApp {
    period: Duration,
    stats: Rc<DeadlineStats>,
}
impl Model for DeadlineApp {
    type Message = ();
    fn update(&mut self, event: Event<()>) -> Effect<()> {
        if matches!(event, Event::Message(())) {
            self.stats
                .notifications
                .set(self.stats.notifications.get() + 1);
            Effect::quit()
        } else {
            Effect::none()
        }
    }
    fn subscriptions(&self) -> Vec<Subscription<()>> {
        vec![Subscription::every(
            "first-notification",
            self.period,
            || (),
        )]
    }
    fn view(&self, _: ViewContext<'_>) -> View {
        View::text("tea")
    }
}
struct DeadlineTransport(Rc<DeadlineStats>);
impl Backend for DeadlineTransport {
    fn enter(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn restore(&mut self) -> io::Result<()> {
        self.0.restores.set(self.0.restores.get() + 1);
        Ok(())
    }
    fn size(&self) -> io::Result<Size> {
        Ok(Size::new(8, 2))
    }
    fn poll(&mut self, timeout: Duration) -> io::Result<Option<TerminalEvent>> {
        // Real elapsed time, honoring the driver's requested input wait.
        // No injected elapsed value, native terminal or fabricated wakeup.
        std::thread::sleep(timeout);
        Ok(None)
    }
    fn draw(&mut self, _: &Surface, _: Option<&Surface>) -> io::Result<()> {
        self.0.draws.set(self.0.draws.get() + 1);
        Ok(())
    }
}
fn first_notification(period: Duration) -> u64 {
    let stats = Rc::new(DeadlineStats::default());
    matea_terminal::run_with_backend(
        DeadlineApp {
            period,
            stats: stats.clone(),
        },
        Theme::plain(),
        DeadlineTransport(stats.clone()),
    )
    .unwrap();
    assert_eq!(
        (
            stats.notifications.get(),
            stats.draws.get(),
            stats.restores.get()
        ),
        (1, 1, 1)
    );
    1
}

fn deadlines() {
    for milliseconds in [1, 5, 16, 50] {
        let period = Duration::from_millis(milliseconds);
        measure(
            &format!("terminal/first_notification_{milliseconds}ms"),
            1,
            || first_notification(black_box(period)),
            "notifications=1; initial_frames=1; restorations=1",
            "fresh headless TEA runtime and 8x2 initial frame; one periodic subscription message quits; backend honors requested poll timeout using real sleep; includes setup, wait, dispatch and restoration; no native terminal I/O",
        );
    }
}

#[derive(Clone, Copy, Debug)]
enum Scenario {
    InitQuit,
    Interrupt,
    Keys(u32),
    Idle(u32),
    Resize(u32),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fault {
    None,
    Enter,
    Size,
    Poll,
    Draw,
    Restore,
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct Stats {
    enter: u32,
    restore: u32,
    poll: u32,
    draw: u32,
    updates: u32,
    checksum: u64,
}
struct App<'a> {
    stats: &'a Cell<Stats>,
    init_quit: bool,
}
impl Model for App<'_> {
    type Message = ();
    fn init(&mut self) -> Effect<()> {
        if self.init_quit {
            Effect::quit()
        } else {
            Effect::none()
        }
    }
    fn update(&mut self, event: Event<()>) -> Effect<()> {
        let mut stats = self.stats.get();
        stats.updates += 1;
        self.stats.set(stats);
        if matches!(event, Event::Input(Input::Key(k)) if k.is_char('q')) {
            Effect::quit()
        } else {
            Effect::none()
        }
    }
    fn view(&self, _: ViewContext<'_>) -> View {
        View::text("tea")
    }
}
struct Transport<'a> {
    stats: &'a Cell<Stats>,
    scenario: Scenario,
    fault: Fault,
}
fn fail() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "independent transport fixture")
}
impl Backend for Transport<'_> {
    fn enter(&mut self) -> io::Result<()> {
        let mut s = self.stats.get();
        s.enter += 1;
        self.stats.set(s);
        if self.fault == Fault::Enter {
            Err(fail())
        } else {
            Ok(())
        }
    }
    fn restore(&mut self) -> io::Result<()> {
        let mut s = self.stats.get();
        s.restore += 1;
        self.stats.set(s);
        if self.fault == Fault::Restore {
            Err(fail())
        } else {
            Ok(())
        }
    }
    fn size(&self) -> io::Result<Size> {
        if self.fault == Fault::Size {
            Err(fail())
        } else {
            Ok(Size::new(8, 2))
        }
    }
    fn poll(&mut self, _: Duration) -> io::Result<Option<TerminalEvent>> {
        let mut s = self.stats.get();
        let index = s.poll;
        s.poll += 1;
        self.stats.set(s);
        if self.fault == Fault::Poll {
            return Err(fail());
        }
        Ok(match self.scenario {
            Scenario::Keys(n) => Some(TerminalEvent::Input(Input::Key(Key::new(KeyCode::Char(
                if index < n { 'x' } else { 'q' },
            ))))),
            Scenario::Idle(n) if index < n => None,
            Scenario::Resize(n) if index < n => {
                Some(TerminalEvent::Resize(if index.is_multiple_of(2) {
                    Size::new(16, 4)
                } else {
                    Size::new(8, 2)
                }))
            }
            _ => Some(TerminalEvent::Interrupt),
        })
    }
    fn draw(&mut self, frame: &Surface, _: Option<&Surface>) -> io::Result<()> {
        let mut s = self.stats.get();
        s.draw += 1;
        s.checksum += u64::from(frame.size().width) * u64::from(frame.size().height);
        self.stats.set(s);
        if self.fault == Fault::Draw {
            Err(fail())
        } else {
            Ok(())
        }
    }
}
fn cycle(scenario: Scenario, fault: Fault) -> Stats {
    let stats = Cell::new(Stats::default());
    let result = matea_terminal::run_with_backend_and_ctrl_c(
        App {
            stats: &stats,
            init_quit: matches!(scenario, Scenario::InitQuit),
        },
        Theme::default(),
        Transport {
            stats: &stats,
            scenario,
            fault,
        },
        CtrlCBehavior::Deliver,
    );
    assert_eq!(result.is_err(), fault != Fault::None);
    if let Err(error) = result {
        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::BrokenPipe
        );
    }
    stats.get()
}
fn identity() {
    macro_rules! types {
        ($module:ident: $($name:ident $(<$($arg:tt),+>)?),+ $(,)?) => {$(assert_eq!(TypeId::of::<$name $(<$($arg),+>)?>(),TypeId::of::<$module::$name $(<$($arg),+>)?>());)+};
    }
    use matea_core as direct_core;
    types!(direct_core: Cancellation, Effect<()>, Event<()>, FocusRing, Input, Key, KeyCode, KeyKind, KeyState, MediaKey, ModifierKey, Modifiers, Mouse, MouseButton, MouseKind, Response<()>, ScrollDirection, Subscription<()>, SubscriptionId, Task<()>, ViewContext<'static>);
    // Generic/lifetime type paths cannot be prefixed by the macro above.
    assert_eq!(
        TypeId::of::<Runtime<App<'static>>>(),
        TypeId::of::<matea_core::Runtime<App<'static>>>()
    );
    use matea_render as direct_render;
    types!(direct_render: Cursor, Rect, Size, Surface, Text, View);
    use matea_style as direct_style;
    types!(direct_style: Border, Color, ColorLevel, Style, Theme);
    use matea_widgets as direct_widgets;
    types!(direct_widgets: Action, ClockId, ClockSchedule, ClockText, ClockTick, ClockUpdate, Countdown, EchoMode, EmptySpinnerFrames, Field, Form, Help, HelpItem<'static>, HelpMap<'static>, HelpStyles, InputStyleState, InputStyles, List, PageDisplay, PageKeys, PageNumbers, Pagination, Paginator, Spinner, SpinnerAnimation, SpinnerPreset, Stopwatch, Table, TextEditKeys, TextInput, TickInterval, Viewport);
    assert_eq!(
        TypeId::of::<Binding>(),
        TypeId::of::<matea_widgets::key::Binding>()
    );
    assert_eq!(
        TypeId::of::<NavigationKeys>(),
        TypeId::of::<matea_widgets::key::NavigationKeys>()
    );
    assert!(std::ptr::fn_addr_eq(
        matea::run::<App<'static>> as fn(_) -> _,
        matea_terminal::run::<App<'static>> as fn(_) -> _
    ));
    assert!(std::ptr::fn_addr_eq(
        matea::run_with_options::<App<'static>> as fn(_, _) -> _,
        matea_terminal::run_with_options::<App<'static>> as fn(_, _) -> _
    ));
    assert!(std::ptr::fn_addr_eq(
        required as fn(&str) -> _,
        matea_widgets::required as fn(&str) -> _
    ));
}
fn main() {
    if std::env::args().any(|a| a == "--deadlines") {
        deadlines();
        return;
    }
    identity();
    let n = if std::env::args().any(|a| a == "--measure") {
        std::env::var("MATEA_PERF_ITERATIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000)
    } else {
        0
    };
    for scenario in [
        Scenario::InitQuit,
        Scenario::Interrupt,
        Scenario::Keys(0),
        Scenario::Keys(1),
        Scenario::Keys(64),
        Scenario::Idle(4),
        Scenario::Resize(4),
    ] {
        let actual = cycle(scenario, Fault::None);
        let (poll, draw, updates, checksum) = match scenario {
            Scenario::InitQuit => (0, 0, 0, 0),
            Scenario::Interrupt => (1, 1, 1, 16),
            Scenario::Keys(n) => (n + 1, n + 1, n + 2, 16 * u64::from(n + 1)),
            Scenario::Idle(n) => (n + 1, 1, 1, 16),
            Scenario::Resize(n) => (
                n + 1,
                n + 1,
                n + 1,
                16 + 64 * u64::from(n.div_ceil(2)) + 16 * u64::from(n / 2),
            ),
        };
        assert_eq!(
            actual,
            Stats {
                enter: 1,
                restore: 1,
                poll,
                draw,
                updates,
                checksum
            }
        );
        if n > 0 {
            measure(
                &format!("terminal/event_loop_{scenario:?}"),
                n,
                || {
                    let s = cycle(black_box(scenario), Fault::None);
                    black_box(s).checksum
                },
                &format!("{actual:?}"),
                "complete headless terminal event loop, fresh runtime/model/theme/backend, deterministic input/idle/resize/quit; no sleeps, effects, native I/O or signal delivery",
            );
        }
    }
    for fault in [
        Fault::Enter,
        Fault::Size,
        Fault::Poll,
        Fault::Draw,
        Fault::Restore,
    ] {
        let actual = cycle(Scenario::Interrupt, fault);
        assert_eq!(actual.enter, 1);
        assert_eq!(actual.restore, if fault == Fault::Restore { 2 } else { 1 });
        if n > 0 {
            measure(
                &format!("terminal/fault_{fault:?}"),
                n,
                || u64::from(cycle(Scenario::Interrupt, black_box(fault)).restore),
                &format!("{actual:?}"),
                "complete headless loop with BrokenPipe at one backend boundary; check error kind and explicit/Drop restoration attempts; no terminal I/O",
            );
        }
    }
    for (name, constructor) in [
        (
            "direct",
            (|s: &str| matea_render::View::text(s)) as fn(&str) -> View,
        ),
        (
            "facade",
            (|s: &str| matea::render::View::text(s)) as fn(&str) -> View,
        ),
        ("prelude", (|s: &str| View::text(s)) as fn(&str) -> View),
    ] {
        let expected = constructor("tea").render(Size::new(3, 1)).unwrap().plain();
        assert_eq!(expected, "tea");
        if n > 0 {
            measure(
                &format!("facade/text_{name}"),
                n,
                || {
                    black_box(constructor(black_box("tea")));
                    3
                },
                &expected,
                "same borrowed text construction through direct crate, facade module or prelude; rendering and identity checks outside timing",
            );
        }
    }
}
