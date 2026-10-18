//! Opt-in private transport probes; normal all-target tests run correctness only.
#![deny(unsafe_code)]
// Source modules include unit-test helpers under Cargo's bench cfg(test); this
// custom executable runs its own finite smoke fixtures instead of a test harness.
#[allow(dead_code)]
#[path = "../src/ansi.rs"]
mod ansi;
#[allow(unsafe_code)] // Measurement-only, unchanged System allocator forwarding.
#[path = "../../../tools/perf-probe/src/measurement.rs"]
mod measurement;
#[allow(dead_code)]
#[path = "../src/modes.rs"]
mod modes;
#[cfg(unix)]
#[allow(unused_imports)] // Unit-test imports are unused under this custom bench harness.
#[allow(unsafe_code)] // The same isolated production sigaction boundary.
#[path = "../src/signals_unix.rs"]
mod signals_unix;

use crossterm::{Command, cursor, style};
use matea_style::Color;
use measurement::measure;
use modes::{InputOptions, KeyboardFlags, KeyboardMode, ModeLeases};
use std::{hint::black_box, io, io::Write};

#[derive(Default)]
struct Sink {
    bytes: Vec<u8>,
    writes: usize,
    flushes: usize,
}
impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.writes += 1;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        Ok(())
    }
}
fn command<C: Command + Copy>(id: &str, command: C, expected: &str, n: usize) {
    let mut sink = Sink::default();
    ansi::command(&mut sink, command).unwrap();
    assert_eq!(sink.bytes, expected.as_bytes());
    if n != 0 {
        measure(
            &format!("ansi/command_{id}"),
            n,
            || {
                sink.bytes.clear();
                ansi::command(black_box(&mut sink), black_box(command)).unwrap();
                black_box(&sink.bytes).len() as u64
            },
            expected,
            "pure Command::write_ansi bridge into preallocated byte sink; no console dispatch, flush or I/O; include checked write adapter",
        );
    }
}
fn main() {
    let n = if std::env::args().any(|a| a == "--measure") {
        measurement::iterations(20000)
    } else {
        0
    };
    // Independent numeric literals, including every ANSI16 boundary.
    for (color, fg, bg) in [
        (Color::Default, "\x1b[39m", "\x1b[49m"),
        (Color::Indexed(0), "\x1b[30m", "\x1b[40m"),
        (Color::Indexed(7), "\x1b[37m", "\x1b[47m"),
        (Color::Indexed(8), "\x1b[90m", "\x1b[100m"),
        (Color::Indexed(15), "\x1b[97m", "\x1b[107m"),
        (Color::Indexed(16), "\x1b[38;5;16m", "\x1b[48;5;16m"),
        (Color::Indexed(255), "\x1b[38;5;255m", "\x1b[48;5;255m"),
        (
            Color::Rgb(255, 128, 42),
            "\x1b[38;2;255;128;42m",
            "\x1b[48;2;255;128;42m",
        ),
        (Color::Rgb(0, 0, 0), "\x1b[38;2;0;0;0m", "\x1b[48;2;0;0;0m"),
    ] {
        for (background, expected) in [(false, fg), (true, bg)] {
            let mut sink = Sink::default();
            ansi::color(&mut sink, color, background).unwrap();
            assert_eq!(sink.bytes, expected.as_bytes());
            if n != 0 {
                measure(
                    &format!("ansi/color_{color:?}_bg_{background}"),
                    n,
                    || {
                        sink.bytes.clear();
                        ansi::color(
                            black_box(&mut sink),
                            black_box(color),
                            black_box(background),
                        )
                        .unwrap();
                        black_box(&sink.bytes).len() as u64
                    },
                    expected,
                    "explicit color to preallocated ANSI bytes; input already adapted, no environment lookup, allocation, flush or terminal I/O",
                );
            }
        }
    }
    command("origin", cursor::MoveTo(0, 0), "\x1b[1;1H", n);
    command("position", cursor::MoveTo(79, 23), "\x1b[24;80H", n);
    command("hide", cursor::Hide, "\x1b[?25l", n);
    command("show", cursor::Show, "\x1b[?25h", n);
    command(
        "reset",
        style::SetAttribute(style::Attribute::Reset),
        "\x1b[0m",
        n,
    );
    command(
        "bold",
        style::SetAttribute(style::Attribute::Bold),
        "\x1b[1m",
        n,
    );
    command(
        "underline",
        style::SetAttribute(style::Attribute::Underlined),
        "\x1b[4m",
        n,
    );
    command(
        "reverse",
        style::SetAttribute(style::Attribute::Reverse),
        "\x1b[7m",
        n,
    );

    for (name, options, expected) in [
        ("default", InputOptions::default(), 10),
        (
            "minimal",
            InputOptions {
                mouse: false,
                focus: false,
                bracketed_paste: false,
                keyboard: KeyboardMode::Legacy,
            },
            6,
        ),
        (
            "full",
            InputOptions {
                mouse: true,
                keyboard: KeyboardMode::Kitty(KeyboardFlags::DISAMBIGUATE_ESCAPE_CODES),
                ..InputOptions::default()
            },
            14,
        ),
    ] {
        let mut calls = 0;
        let mut leases = ModeLeases::default();
        leases
            .enter(options, |_, _, _| {
                calls += 1;
                Ok(())
            })
            .unwrap();
        assert!(leases.active());
        leases
            .restore(|_, _, _| {
                calls += 1;
                Ok(())
            })
            .unwrap();
        assert!(!leases.active());
        assert_eq!(calls, expected);
        #[cfg(unix)]
        {
            let mut enter = String::from("\x1b[?1049h\x1b[?25l\x1b[0m\x1b[0m");
            let mut exit = String::new();
            if matches!(options.keyboard, KeyboardMode::Kitty(_)) {
                exit.push_str("\x1b[<1u");
            }
            if options.mouse {
                exit.push_str("\x1b[?1006l\x1b[?1015l\x1b[?1003l\x1b[?1002l\x1b[?1000l");
            }
            if options.bracketed_paste {
                enter.push_str("\x1b[?2004h");
            }
            if options.focus {
                enter.push_str("\x1b[?1004h");
                exit.push_str("\x1b[?1004l");
            }
            if options.mouse {
                enter.push_str("\x1b[?1000h\x1b[?1002h\x1b[?1003h\x1b[?1015h\x1b[?1006h");
            }
            if matches!(options.keyboard, KeyboardMode::Kitty(_)) {
                enter.push_str("\x1b[>1u");
            }
            if options.bracketed_paste {
                exit.push_str("\x1b[?2004l");
            }
            exit.push_str("\x1b[0m\x1b[0m\x1b[?25h\x1b[?1049l");
            let expected_bytes = enter + &exit;
            let mut sink = Sink::default();
            let cycle = |sink: &mut Sink| {
                sink.bytes.clear();
                sink.writes = 0;
                sink.flushes = 0;
                let mut leases = ModeLeases::default();
                leases
                    .enter(options, |mode, enable, flags| {
                        modes::apply_mode(sink, mode, enable, flags)
                    })
                    .unwrap();
                sink.flush().unwrap();
                leases
                    .restore(|mode, enable, flags| modes::apply_mode(sink, mode, enable, flags))
                    .unwrap();
                sink.flush().unwrap();
            };
            cycle(&mut sink);
            assert_eq!(sink.bytes, expected_bytes.as_bytes());
            if std::env::args().any(|a| a == "--mode-stats") {
                eprintln!(
                    "{{\"case\":\"modes/native_cycle_{name}\",\"writes\":{},\"flushes\":{},\"bytes\":{}}}",
                    sink.writes,
                    sink.flushes,
                    sink.bytes.len()
                );
            }
            if n != 0 {
                measure(
                    &format!("modes/native_cycle_{name}"),
                    n,
                    || {
                        cycle(black_box(&mut sink));
                        black_box(&sink.bytes).len() as u64
                    },
                    &expected_bytes,
                    "Unix native mode commands with acquisition and restoration phase flushes into preallocated counting sink; fresh leases, no actual terminal or kernel write",
                );
            }
        }
        if n != 0 {
            measure(
                &format!("modes/lease_cycle_{name}"),
                n,
                || {
                    let mut leases = ModeLeases::default();
                    let mut calls = 0;
                    leases
                        .enter(black_box(options), |mode, enable, flags| {
                            black_box((mode, enable, flags));
                            calls += 1;
                            Ok(())
                        })
                        .unwrap();
                    leases
                        .restore(|mode, enable, flags| {
                            black_box((mode, enable, flags));
                            calls += 1;
                            Ok(())
                        })
                        .unwrap();
                    calls
                },
                &expected.to_string(),
                "fresh mode lease acquisition and successful rollback with counting callback; no native commands, syscalls or terminal I/O",
            );
        }
    }
    #[cfg(unix)]
    {
        let mut guard = signals_unix::SignalGuard::install().unwrap();
        assert!(!signals_unix::interrupted());
        guard.restore().unwrap();
        if n != 0 {
            measure(
                "signals/install_restore",
                n / 20 + 1,
                || {
                    let mut guard = signals_unix::SignalGuard::install().unwrap();
                    black_box(signals_unix::interrupted());
                    guard.restore().unwrap();
                    u64::from(!signals_unix::interrupted())
                },
                "SIGINT/SIGTERM previous actions restored",
                "install and explicitly restore two previous sigactions, then drop empty guard; includes native syscalls; isolated standalone process, no signal delivery or terminal I/O",
            );
        }
    }
    // Compile these production paths without invoking a WinAPI console backend
    // on non-interactive CI; command dispatch is covered by Native PTY tests.
    let _: fn(&mut Sink, modes::Mode, bool, KeyboardFlags) -> io::Result<()> = modes::apply_mode;
    assert!(InputOptions::default().validate().is_ok());
}
