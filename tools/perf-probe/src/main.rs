//! Matched Foundation micro-workloads. No image or GPU processing.
//! Allocation instrumentation is enabled only during the separate allocation pass.
mod measurement;
mod protocol;
use matea::{
    prelude::*,
    style::Length,
    terminal::{Capabilities, FrameEncoder},
};
use measurement::{measure, quote};
use std::{hint::black_box, time::Instant};
use unicode_width::UnicodeWidthStr;
fn trace() {
    let caps = Capabilities {
        color: ColorLevel::None,
        unicode: true,
        reduced_motion: true,
    };
    let mut encoder = FrameEncoder::default();
    let mut previous = None;
    let mut sequences = Vec::new();
    for c in ['a', 'b', 'a'] {
        let frame = View::text(format!("{c}{}", "a".repeat(79)))
            .render(Size::new(80, 24))
            .unwrap();
        let mut bytes = Vec::new();
        encoder
            .write(&mut bytes, &frame, previous.as_ref(), caps)
            .unwrap();
        sequences.push(quote(&String::from_utf8(bytes).unwrap()));
        previous = Some(frame);
    }
    println!(
        "{{\"width\":80,\"height\":24,\"sequences\":[{}]}}",
        sequences.join(",")
    );
}
fn main() {
    if std::env::args().any(|arg| arg == "--fallback-trace") {
        fallback_trace();
        return;
    }
    if std::env::args().any(|arg| arg == "--trace") {
        trace();
        return;
    }
    if std::env::args().any(|arg| arg == "--cursor-trace") {
        cursor_trace();
        return;
    }
    if std::env::args().any(|arg| arg == "--color-trace") {
        colored_frames(0);
        return;
    }
    let iterations = std::env::var("MATEA_PERF_ITERATIONS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(300);
    if std::env::args().any(|arg| arg == "--subscriptions") {
        subscription_costs(iterations);
        return;
    }
    if std::env::args().any(|arg| arg == "--clocks") {
        clock_cases(iterations);
        return;
    }
    if std::env::args().any(|arg| arg == "--virtual-cursor") {
        virtual_cursor_cases(iterations);
        return;
    }
    if std::env::args().any(|arg| arg == "--views") {
        view_construction_costs(iterations);
        return;
    }
    if std::env::args().any(|arg| arg == "--viewport") {
        viewport_costs(iterations);
        return;
    }
    if std::env::args().any(|arg| arg == "--style") {
        style_costs(iterations);
        return;
    }
    if std::env::args().any(|arg| arg == "--input") {
        input_costs(iterations);
        return;
    }
    if std::env::args().any(|arg| arg == "--protocol") {
        protocol::costs(iterations);
        return;
    }
    input_bindings(iterations);
    projectiles(iterations);
    pagination(iterations);
    text_input(iterations);
    cursor_encoding(iterations);
    colored_frames(iterations);
    help_cases(iterations);
    clock_cases(iterations);
    spinner_cases(iterations);
    let size = Size::new(80, 24);
    for (name, text) in [
        ("empty", String::new()),
        (
            "ascii",
            (0..20)
                .map(|_| "Matea tea studio: identical ASCII row")
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        (
            "unicode",
            (0..20)
                .map(|_| "抹茶 e\u{301} 👩‍💻 tea")
                .collect::<Vec<_>>()
                .join("\n"),
        ),
    ] {
        let view = View::text(&text);
        let output = view.render(size).unwrap().plain();
        measure(
            &format!("render/{name}_80x24"),
            iterations,
            || {
                let s = black_box(view.render(size).unwrap());
                black_box(s.row(0));
                u64::from(s.size().width)
            },
            &output,
            "fresh clipped grapheme cell surface, default style, non-CJK ambiguous width",
        );
    }
    let text = "抹茶 e\u{301} 👩‍💻 ".repeat(16);
    let width = text.width();
    measure(
        "unicode/width",
        iterations * 10,
        || black_box(text.as_str()).width() as u64,
        &width.to_string(),
        "grapheme display width, no control characters, non-CJK ambiguous width",
    );
    let frame = View::text("a".repeat(80)).render(size).unwrap();
    let caps = Capabilities {
        color: ColorLevel::None,
        unicode: true,
        reduced_motion: true,
    };
    let mut sink = Vec::with_capacity(8192);
    let mut encoder = FrameEncoder::default();
    encoder.write(&mut sink, &frame, None, caps).unwrap();
    measure(
        "terminal/unchanged_80x24",
        iterations,
        || {
            sink.clear();
            encoder
                .write(&mut sink, &frame, Some(&frame), caps)
                .unwrap();
            sink.len() as u64
        },
        "",
        "encode unchanged existing frame to reused memory sink; no terminal I/O",
    );
    let next = View::text(format!("b{}", "a".repeat(79)))
        .render(size)
        .unwrap();
    let mut flip = false;
    measure(
        "terminal/one_cell_80x24",
        iterations,
        || {
            sink.clear();
            let (a, b) = if flip {
                (&frame, &next)
            } else {
                (&next, &frame)
            };
            encoder.write(&mut sink, a, Some(b), caps).unwrap();
            flip = !flip;
            sink.len() as u64
        },
        "one ASCII cell toggled; independently replay output before claiming equivalence",
        "toggle one existing ASCII cell, encode to reused memory sink; output encoding may differ",
    );
    for damping in [0.2, 1.0, 2.0] {
        use matea::motion::{Motion, Spring};
        let make = || {
            Spring::new(
                std::time::Duration::from_millis(16),
                black_box(12.0),
                black_box(damping),
            )
            .unwrap()
        };
        let spring = make();
        let state = Motion {
            position: 2.0,
            velocity: 0.0,
        };
        let output = spring.step(state, 1.0).unwrap();
        let expected = format!("{:.12},{:.12}", output.position, output.velocity);
        measure(
            &format!("motion/spring_update_{damping}"),
            (iterations * 10).max(30000),
            || {
                let m = spring.sample(black_box(state), black_box(1.0));
                m.position.to_bits() ^ m.velocity.to_bits()
            },
            &expected,
            "cached spring update dt=16ms omega=12 state=(2,0) target=1; finite inputs",
        );
        measure(
            &format!("motion/spring_checked_update_{damping}"),
            (iterations * 10).max(30000),
            || {
                let m = spring.step(black_box(state), black_box(1.0)).unwrap();
                m.position.to_bits() ^ m.velocity.to_bits()
            },
            &expected,
            "cached spring update with finite input/result checks dt=16ms omega=12 state=(2,0) target=1",
        );
        measure(
            &format!("motion/spring_construct_{damping}"),
            (iterations * 10).max(30000),
            || {
                let m = black_box(make())
                    .step(black_box(state), black_box(1.0))
                    .unwrap();
                m.position.to_bits() ^ m.velocity.to_bits()
            },
            &expected,
            "construct spring coefficients and update once dt=16ms omega=12 state=(2,0) target=1; finite inputs",
        );
    }
}
fn input_costs(iterations: usize) {
    use crossterm::event as source;
    use matea::terminal::translate_event;
    let events = vec![
        (
            "ascii",
            source::Event::Key(source::KeyEvent::new(
                source::KeyCode::Char('q'),
                source::KeyModifiers::empty(),
            )),
        ),
        (
            "null",
            source::Event::Key(source::KeyEvent::new(
                source::KeyCode::Char('\0'),
                source::KeyModifiers::empty(),
            )),
        ),
        (
            "modified_repeat",
            source::Event::Key(source::KeyEvent::new_with_kind_and_state(
                source::KeyCode::F(12),
                source::KeyModifiers::all(),
                source::KeyEventKind::Repeat,
                source::KeyEventState::all(),
            )),
        ),
        (
            "release",
            source::Event::Key(source::KeyEvent::new_with_kind(
                source::KeyCode::Left,
                source::KeyModifiers::SHIFT,
                source::KeyEventKind::Release,
            )),
        ),
        (
            "media",
            source::Event::Key(source::KeyEvent::new(
                source::KeyCode::Media(source::MediaKeyCode::PlayPause),
                source::KeyModifiers::empty(),
            )),
        ),
        ("resize", source::Event::Resize(80, 24)),
        ("focus_gained", source::Event::FocusGained),
        ("focus_lost", source::Event::FocusLost),
        ("paste_short", source::Event::Paste("tea".into())),
        ("paste_long", source::Event::Paste("a".repeat(4096))),
        (
            "paste_unicode",
            source::Event::Paste("抹茶 e\u{301} 👩‍💻".repeat(32)),
        ),
    ];
    for (name, event) in events {
        let expected = format!("{:?}", translate_event(event.clone()));
        measure(
            &format!("input/translate_{name}"),
            iterations,
            || {
                black_box(translate_event(black_box(event.clone())));
                1
            },
            &expected,
            "clone an existing Crossterm event, move into transport-neutral translation, drop output; paste includes source String clone; no terminal parsing or I/O",
        );
    }
    for (name, kind) in [
        (
            "down",
            source::MouseEventKind::Down(source::MouseButton::Left),
        ),
        ("up", source::MouseEventKind::Up(source::MouseButton::Right)),
        (
            "drag",
            source::MouseEventKind::Drag(source::MouseButton::Middle),
        ),
        ("moved", source::MouseEventKind::Moved),
        ("scroll_up", source::MouseEventKind::ScrollUp),
        ("scroll_down", source::MouseEventKind::ScrollDown),
        ("scroll_left", source::MouseEventKind::ScrollLeft),
        ("scroll_right", source::MouseEventKind::ScrollRight),
    ] {
        let event = source::Event::Mouse(source::MouseEvent {
            kind,
            column: 17,
            row: 9,
            modifiers: source::KeyModifiers::ALT,
        });
        let expected = format!("{:?}", translate_event(event.clone()));
        measure(
            &format!("input/translate_mouse_{name}"),
            iterations,
            || {
                black_box(translate_event(black_box(event.clone())));
                1
            },
            &expected,
            "clone/translate/drop one existing mouse event, preserving kind/coordinates/modifiers; no terminal parsing or I/O",
        );
    }
    let chord = Key::new(KeyCode::Char('x')).with_modifiers(Modifiers::CONTROL);
    for (name, key) in [
        ("press", chord),
        ("repeat", chord.with_kind(KeyKind::Repeat)),
        ("release", chord.with_kind(KeyKind::Release)),
        (
            "wrong_modifier",
            chord.with_modifiers(Modifiers::CONTROL | Modifiers::META),
        ),
        ("wrong_code", Key::new(KeyCode::Char('z'))),
    ] {
        let expected = key.matches_chord(chord);
        measure(
            &format!("input/chord_{name}"),
            iterations,
            || u64::from(black_box(key).matches_chord(black_box(chord))),
            &expected.to_string(),
            "typed chord comparison including down/repeat/release and exact modifiers; no string conversion or I/O",
        );
    }
    for (name, key) in [
        ("tab", Key::new(KeyCode::Tab)),
        (
            "shift_tab",
            Key::new(KeyCode::Tab).with_modifiers(Modifiers::SHIFT),
        ),
        ("backtab", Key::new(KeyCode::BackTab)),
        (
            "control_tab",
            Key::new(KeyCode::Tab).with_modifiers(Modifiers::CONTROL),
        ),
        (
            "release_tab",
            Key::new(KeyCode::Tab).with_kind(KeyKind::Release),
        ),
    ] {
        let expected = format!("{:?}", key.focus_direction());
        measure(
            &format!("input/focus_direction_{name}"),
            iterations,
            || {
                black_box(black_box(key).focus_direction());
                1
            },
            &expected,
            "derive typed focus traversal direction from one key event; no focus ring, strings or I/O",
        );
    }
    for count in [0usize, 3, 64, 1024] {
        for duplicate in [false, true] {
            let ids: Vec<_> = (0..count)
                .map(|i| format!("field-{}", if duplicate { 0 } else { i }))
                .collect();
            let expected = format!("{:?}", FocusRing::new(ids.iter().cloned()));
            measure(
                &format!(
                    "input/focus_construct_{count}_{}",
                    if duplicate { "duplicate" } else { "unique" }
                ),
                if count > 64 {
                    (iterations / 100).max(50)
                } else {
                    iterations
                },
                || {
                    black_box(FocusRing::new(black_box(&ids).iter().cloned()));
                    1
                },
                &expected,
                "clone owned input IDs, construct/deduplicate/drop ordered focus ring; includes String/Vec allocation; no runtime or terminal I/O",
            );
        }
    }
}

fn fallback_trace() {
    let style = Style::default()
        .fg(Color::Rgb(95, 135, 175))
        .bg(Color::Rgb(175, 95, 135))
        .bold()
        .reverse();
    let frames = [
        View::styled("茶xe\u{301}", style)
            .render(Size::new(8, 2))
            .unwrap(),
        View::text("q").render(Size::new(8, 2)).unwrap(),
        View::layers(vec![(Rect::new(6, 1, 2, 1), View::styled("茶", style))])
            .render(Size::new(8, 2))
            .unwrap(),
        View::styled("a\n茶", style)
            .with_cursor(Cursor::new(0, 1))
            .render(Size::new(8, 2))
            .unwrap(),
    ];
    let mut traces = Vec::new();
    for (profile, color) in [
        ("none", ColorLevel::None),
        ("truecolor", ColorLevel::TrueColor),
    ] {
        let caps = Capabilities {
            color,
            unicode: false,
            reduced_motion: true,
        };
        let mut encoder = FrameEncoder::default();
        let mut previous = None;
        let mut sequences = Vec::new();
        for frame in &frames {
            let mut bytes = Vec::new();
            encoder.write(&mut bytes, frame, previous, caps).unwrap();
            sequences.push(quote(&String::from_utf8(bytes).unwrap()));
            previous = Some(frame);
        }
        traces.push(format!(
            "{{\"profile\":{},\"width\":8,\"height\":2,\"sequences\":[{}]}}",
            quote(profile),
            sequences.join(",")
        ));
    }
    println!(
        "{{\"kind\":\"unicode_fallback\",\"traces\":[{}]}}",
        traces.join(",")
    );
}

fn style_costs(iterations: usize) {
    let inputs = [
        ("default", Color::Default),
        ("indexed_9", Color::Indexed(9)),
        ("indexed_200", Color::Indexed(200)),
        ("rgb_black", Color::Rgb(0, 0, 0)),
        ("rgb_white", Color::Rgb(255, 255, 255)),
        ("rgb_cube", Color::Rgb(95, 135, 175)),
        ("rgb_gray_tie", Color::Rgb(13, 13, 13)),
        ("rgb_arbitrary", Color::Rgb(41, 173, 227)),
    ];
    for (profile, level) in [
        ("none", ColorLevel::None),
        ("ansi16", ColorLevel::Ansi16),
        ("ansi256", ColorLevel::Ansi256),
        ("truecolor", ColorLevel::TrueColor),
    ] {
        for (name, color) in inputs {
            let expected = format!("{:?}", color.adapt(level));
            measure(
                &format!("style/adapt_{name}_{profile}"),
                iterations,
                || {
                    let result = black_box(color).adapt(black_box(level));
                    black_box(result);
                    1
                },
                &expected,
                "adapt one semantic color using squared RGB distance and first-index ties; no global cache, scene or terminal I/O",
            );
        }
        let style = Style::default()
            .fg(Color::Rgb(41, 173, 227))
            .bg(Color::Rgb(13, 13, 13))
            .bold()
            .reverse();
        let expected = format!("{:?}", style.adapt(level));
        measure(
            &format!("style/adapt_pair_{profile}"),
            iterations,
            || {
                black_box(black_box(style).adapt(black_box(level)));
                1
            },
            &expected,
            "adapt foreground/background while preserving attributes; no global cache, scene or terminal I/O",
        );
        // Vary all channels to prevent a single-color cache from defining the
        // workload. The complete 256-result fixture is checked independently.
        let colors: Vec<_> = (0..=255u8)
            .map(|n| Color::Rgb(n, n.wrapping_mul(73), n.wrapping_mul(151)))
            .collect();
        let expected = format!(
            "{:?}",
            colors.iter().map(|c| c.adapt(level)).collect::<Vec<_>>()
        );
        measure(
            &format!("style/adapt_varied_256_{profile}"),
            (iterations / 100).max(50),
            || {
                for c in black_box(&colors) {
                    black_box(black_box(*c).adapt(black_box(level)));
                }
                256
            },
            &expected,
            "adapt 256 distinct RGB triples in fixed order; squared RGB distance, first-index ties, no cache or terminal I/O",
        );
    }
    for count in [0usize, 3, 64, 1024] {
        let lengths: Vec<_> = (0..count)
            .map(|i| match i % 4 {
                0 => Length::Fixed(3),
                1 => Length::Fill(0),
                2 => Length::Fill(1),
                _ => Length::Fill(2),
            })
            .collect();
        let expected = format!("{:?}", matea::style::distribute(65535, &lengths));
        measure(
            &format!("style/layout_{count}"),
            iterations,
            || {
                black_box(matea::style::distribute(
                    black_box(65535),
                    black_box(&lengths),
                ));
                1
            },
            &expected,
            "allocate fixed and zero/weighted-fill widths under 65535-cell budget; construct/drop result Vec",
        );
    }
    for (name, make) in [
        ("default", Theme::default as fn() -> Theme),
        ("plain", Theme::plain),
    ] {
        let expected = format!("{:?}", make());
        measure(
            &format!("style/theme_{name}"),
            iterations,
            || {
                black_box(make());
                1
            },
            &expected,
            "construct/drop explicit theme tokens without environment lookup or terminal I/O",
        );
    }
}

fn viewport_costs(iterations: usize) {
    let theme = Theme::plain();
    for (name, line) in [
        ("ascii", "Matea owns each line"),
        ("unicode", "抹茶 e\u{301} 👩‍💻"),
    ] {
        for rows in [1u16, 6, 24] {
            let text = std::iter::repeat_n(line, 100)
                .collect::<Vec<_>>()
                .join("\n");
            let mut viewport = Viewport::new(&text);
            viewport.rows = rows;
            let size = Size::new(80, rows);
            let output = viewport.view(&theme, false).render(size).unwrap().plain();
            measure(
                &format!("viewport/view_{name}_{rows}"),
                iterations,
                || {
                    black_box(black_box(&viewport).view(black_box(&theme), false));
                    1
                },
                &output,
                "construct and drop an owned viewport scene of first visible rows from 100 lines; no rendering or terminal I/O",
            );
            measure(
                &format!("viewport/view_render_{name}_{rows}"),
                iterations,
                || {
                    black_box(
                        black_box(&viewport)
                            .view(black_box(&theme), false)
                            .render(black_box(size))
                            .unwrap(),
                    );
                    1
                },
                &output,
                "construct, render and drop viewport scene and fresh 80-column surface; no terminal I/O",
            );
        }
    }
}

fn view_construction_costs(iterations: usize) {
    let theme = Theme::default();
    let size = Size::new(72, 4);
    let fixtures = [
        ("short_ascii", "Matea tea"),
        ("short_unicode", "抹茶 e\u{301}"),
        (
            "long_ascii",
            "Matea owns this longer line of terminal-neutral text without retaining a caller borrow.",
        ),
        ("controls", "tea\t\u{1b}[31m\n抹茶"),
    ];
    for (name, text) in fixtures {
        let output = View::text(text).render(size).unwrap().plain();
        measure(
            &format!("views/new_{name}"),
            iterations,
            || {
                black_box(View::text(black_box(text)));
                1
            },
            &output,
            "construct and drop one owned text scene from borrowed UTF-8; no cell render or terminal I/O",
        );
        let view = View::text(text);
        measure(
            &format!("views/clone_{name}"),
            iterations,
            || {
                black_box(black_box(&view).clone());
                1
            },
            &output,
            "clone and drop one owned text scene; no cell render or terminal I/O",
        );
        measure(
            &format!("views/new_render_{name}"),
            iterations,
            || {
                let surface = View::text(black_box(text)).render(black_box(size)).unwrap();
                black_box(surface);
                1
            },
            &output,
            "construct, normalize and render fresh owned scene to 72x4 cells; no terminal I/O",
        );
        measure(
            &format!("views/clone_render_{name}"),
            iterations,
            || {
                let surface = black_box(&view).clone().render(black_box(size)).unwrap();
                black_box(surface);
                1
            },
            &output,
            "clone existing scene and render fresh 72x4 cell surface; no terminal I/O",
        );
    }
    let value = black_box(42u64);
    let make_counter = || View::styled(format_args!("Count: {}", black_box(value)), theme.accent);
    let output = make_counter().render(size).unwrap().plain();
    measure(
        "views/formatted_counter",
        iterations,
        || {
            black_box(make_counter());
            1
        },
        &output,
        "format one numeric counter into an owned styled text scene; no cell render or terminal I/O",
    );
    measure(
        "views/formatted_counter_render",
        iterations,
        || {
            black_box(make_counter().render(black_box(size)).unwrap());
            1
        },
        &output,
        "format numeric counter and render owned styled scene to fresh 72x4 cells; no terminal I/O",
    );
    let panel_size = Size::new(72, 6);
    let output = View::text("抹茶")
        .panel("Tea", &theme)
        .render(panel_size)
        .unwrap()
        .plain();
    measure(
        "views/short_panel",
        iterations,
        || {
            black_box(View::text(black_box("抹茶")).panel(black_box("Tea"), &theme));
            1
        },
        &output,
        "construct and drop owned panel with short text/title and existing theme; no cell render or terminal I/O",
    );
    measure(
        "views/short_panel_render",
        iterations,
        || {
            black_box(
                View::text(black_box("抹茶"))
                    .panel(black_box("Tea"), &theme)
                    .render(black_box(panel_size))
                    .unwrap(),
            );
            1
        },
        &output,
        "construct and render owned panel with short text/title to fresh 72x6 cells; no terminal I/O",
    );
}
fn subscription_costs(iterations: usize) {
    let origin = Instant::now();
    let make_schedule = || {
        let mut clock = Stopwatch::new_at(origin);
        clock.start_at(origin);
        clock.schedule().unwrap()
    };
    let schedule = make_schedule();
    measure(
        "subscriptions/clock_adapter",
        iterations,
        || {
            black_box(black_box(schedule).subscription_with_clock(move || origin, |tick| tick));
            1
        },
        "one clock subscription descriptor; callback not invoked",
        "construct and drop one clock subscription adapter with captured monotonic clock; no runtime, worker or timer wakeup",
    );
    struct App {
        schedules: Vec<ClockSchedule>,
        origin: Instant,
        seen: usize,
    }
    impl Model for App {
        type Message = ClockTick;
        fn update(&mut self, event: Event<ClockTick>) -> Effect<ClockTick> {
            if matches!(event, Event::Message(_)) {
                self.seen += 1;
            }
            Effect::none()
        }
        fn view(&self, _: ViewContext<'_>) -> View {
            View::text("")
        }
        fn subscriptions(&self) -> Vec<Subscription<ClockTick>> {
            let origin = self.origin;
            self.schedules
                .iter()
                .map(|schedule| schedule.subscription_with_clock(move || origin, |tick| tick))
                .collect()
        }
    }
    for count in [0, 1, 12, 64] {
        let app = App {
            schedules: (0..count).map(|_| make_schedule()).collect(),
            origin,
            seen: 0,
        };
        let mut runtime = Runtime::new(app, Size::new(1, 1), Theme::plain()).unwrap();
        measure(
            &format!("subscriptions/reconcile_{count}"),
            iterations,
            || {
                black_box(&mut runtime)
                    .dispatch(Event::Input(Input::Focus(true)))
                    .unwrap();
                black_box(runtime.model().schedules.len()) as u64
            },
            &format!("{count} configured subscriptions; no timer fired"),
            "dispatch ignored focus event and retain unchanged clock subscriptions; callbacks recreated but not invoked; no frame or terminal I/O",
        );
        assert_eq!(runtime.model().seen, 0);
        runtime.advance(std::time::Duration::from_secs(1)).unwrap();
        assert_eq!(runtime.model().seen, count);
    }
}
fn spinner_cases(iterations: usize) {
    let theme = Theme::default();
    let names = [
        "line",
        "dot",
        "mini_dot",
        "jump",
        "pulse",
        "points",
        "globe",
        "moon",
        "monkey",
        "meter",
        "hamburger",
        "ellipsis",
    ];
    for (preset, name) in SpinnerPreset::ALL.into_iter().zip(names) {
        let spinner = Spinner::new(preset.animation());
        measure(
            &format!("spinner/plain_{name}"),
            iterations * 10,
            || black_box(&spinner).text(black_box(&theme)).len() as u64,
            &spinner.text(&theme),
            "plain spinner frame string from existing immutable animation; storage sharing allowed; no label, style, terminal I/O or scheduling",
        );
    }
    let origin = Instant::now();
    let mut spinner = Spinner::new_at(SpinnerPreset::Line.animation(), origin);
    spinner.start_at(origin);
    let mut other = Spinner::default();
    other.start();
    let foreign = other.schedule(&theme).unwrap().tick();
    let output = spinner.text(&theme).into_owned();
    measure(
        "spinner/foreign",
        iterations * 10,
        || {
            assert!(!black_box(&mut spinner).update(black_box(foreign)));
            spinner.text(black_box(&theme)).len() as u64
        },
        &output,
        "ignore foreign spinner tick; return unchanged plain frame; no new scheduling request",
    );
    let stale = spinner
        .schedule(&theme)
        .unwrap()
        .tick_at(origin + std::time::Duration::from_millis(100));
    spinner.reset_at(origin);
    spinner.tick(&theme);
    spinner.tick(&theme);
    let output = spinner.text(&theme).into_owned();
    measure(
        "spinner/stale",
        iterations * 10,
        || {
            assert!(!black_box(&mut spinner).update(black_box(stale)));
            spinner.text(black_box(&theme)).len() as u64
        },
        &output,
        "ignore old positive-generation spinner tick; return unchanged plain frame; no new scheduling request",
    );
}
fn clock_cases(iterations: usize) {
    use std::fmt::Write as _;
    use std::time::Duration;
    for (name, nanos) in [
        ("zero", 0),
        ("nanoseconds", 42),
        ("microseconds", 1500),
        ("seconds", 1_234_567_890),
        ("hours", 3_723_400_000_000),
        ("maximum_go", 9_223_372_036_854_775_807),
    ] {
        let value = Duration::from_nanos(nanos);
        let output = ClockText(value).to_string();
        measure(
            &format!("clocks/format_{name}"),
            iterations,
            || black_box(ClockText(black_box(value)).into_string()).len() as u64,
            &output,
            "fresh compact nonnegative duration string used by clock View; known duration; no timing, scheduling or terminal I/O",
        );
        let mut buffer = String::with_capacity(40);
        write!(&mut buffer, "{}", ClockText(value)).unwrap();
        let output = buffer.clone();
        measure(
            &format!("clocks/reused_format_{name}"),
            iterations,
            || {
                buffer.clear();
                write!(&mut buffer, "{}", ClockText(black_box(value))).unwrap();
                black_box(&buffer).len() as u64
            },
            &output,
            "compact duration appended into cleared caller-owned reusable UTF-8 buffer; known duration; no scheduling or terminal I/O",
        );
    }
    let origin = Instant::now();
    for foreign in [false, true] {
        let mut sw = Stopwatch::new_at(origin);
        sw.set_interval_at(TickInterval::new(Duration::from_nanos(1)).unwrap(), origin);
        sw.start_at(origin);
        let other = Countdown::new_at(Duration::from_secs(10), origin);
        let tick = if foreign {
            other.schedule().unwrap().tick_at(origin)
        } else {
            let tick = sw.schedule().unwrap().tick_at(origin);
            sw.stop_at(origin);
            tick
        };
        let accepted = sw.update(tick).changed();
        let output = format!(
            "value={}ns,running={},accepted={accepted}",
            sw.elapsed().as_nanos(),
            sw.running()
        );
        let name = if foreign {
            "stopwatch_foreign"
        } else {
            "stopwatch_paused"
        };
        measure(
            &format!("clocks/{name}"),
            iterations * 10,
            || black_box(&mut sw).update(black_box(tick)).changed() as u64,
            &output,
            "reject ignored tick, preserve cached clock value/running state, emit no new schedule; prebuilt event; no timer execution",
        );
        let mut timer = Countdown::new_at(Duration::from_secs(10), origin);
        timer.set_interval_at(TickInterval::new(Duration::from_nanos(1)).unwrap(), origin);
        let tick = if foreign {
            sw.schedule().unwrap().tick_at(origin)
        } else {
            let tick = timer.schedule().unwrap().tick_at(origin);
            timer.stop_at(origin);
            tick
        };
        let accepted = timer.update(tick).changed();
        let output = format!(
            "value={}ns,running={},accepted={accepted}",
            timer.remaining().as_nanos(),
            timer.running()
        );
        let name = if foreign {
            "countdown_foreign"
        } else {
            "countdown_paused"
        };
        measure(
            &format!("clocks/{name}"),
            iterations * 10,
            || black_box(&mut timer).update(black_box(tick)).changed() as u64,
            &output,
            "reject ignored tick, preserve cached clock value/running state, emit no new schedule; prebuilt event; no timer execution",
        );
    }
}
fn help_cases(iterations: usize) {
    let binding = |key: &'static str, description: &'static str| HelpItem {
        key,
        description,
        enabled: true,
    };
    let a = [binding("enter", "continue")];
    let b = [binding("esc", "back"), binding("?", "help")];
    let c = [
        binding("H", "home"),
        binding("ctrl+c", "quit"),
        binding("ctrl+l", "log"),
    ];
    let groups = [&a[..], &b[..], &c[..]];
    let short = [a[0], b[0], b[1], c[0], c[1], c[2]];
    let unicode = [
        binding("茶", "e\u{301}を選ぶ"),
        binding("←/→", "移動"),
        binding("q", "終了"),
    ];
    for (name, width, full, items) in [
        ("short_unbounded", None, false, short.as_slice()),
        ("short_40", Some(40), false, short.as_slice()),
        ("full_40", Some(40), true, short.as_slice()),
        ("short_unicode_80", Some(80), false, unicode.as_slice()),
    ] {
        let help = Help {
            width,
            full_separator: " | ".into(),
            ..Help::default()
        };
        let render = || {
            if full {
                help.full_text(&groups)
            } else {
                help.short_text(items)
            }
        };
        let output = render();
        measure(
            &format!("help/{name}"),
            iterations,
            || black_box(render()).len() as u64,
            &output,
            "render fresh plain key help string; existing metadata; no ANSI styling, terminal I/O or binding construction",
        );
    }
}
fn colored_frames(iterations: usize) {
    let mut traces = Vec::new();
    for (profile, level, foreground, background) in [
        (
            "ansi16",
            ColorLevel::Ansi16,
            Color::Rgb(255, 255, 255),
            Color::Rgb(0, 0, 0),
        ),
        (
            "ansi256",
            ColorLevel::Ansi256,
            Color::Rgb(95, 135, 175),
            Color::Rgb(175, 95, 135),
        ),
        (
            "truecolor",
            ColorLevel::TrueColor,
            Color::Rgb(95, 135, 175),
            Color::Rgb(175, 95, 135),
        ),
        (
            "none",
            ColorLevel::None,
            Color::Rgb(95, 135, 175),
            Color::Rgb(175, 95, 135),
        ),
    ] {
        let caps = Capabilities {
            color: level,
            unicode: true,
            reduced_motion: true,
        };
        let text = vec!["a".repeat(80); 20].join("\n");
        let frame = View::styled(text, Style::default().fg(foreground).bg(background))
            .render(Size::new(80, 24))
            .unwrap();
        let mut encoder = FrameEncoder::default();
        let mut output = Vec::with_capacity(8192);
        encoder.write(&mut output, &frame, None, caps).unwrap();
        if iterations == 0 {
            let initial = quote(std::str::from_utf8(&output).unwrap());
            output.clear();
            encoder.invalidate();
            encoder.write(&mut output, &frame, None, caps).unwrap();
            traces.push(format!(
                "{{\"profile\":{},\"width\":80,\"height\":24,\"sequences\":[{},{}]}}",
                quote(profile),
                initial,
                quote(std::str::from_utf8(&output).unwrap())
            ));
        } else {
            measure(
                &format!("terminal/colored_full_{profile}_80x24"),
                (iterations / 20).max(50),
                || {
                    output.clear();
                    encoder.invalidate();
                    encoder.write(&mut output, &frame, None, caps).unwrap();
                    output.len() as u64
                },
                "20 rows of 80 a cells; palette output replayed independently; lower four rows blank/default",
                &format!(
                    "recover unknown cursor/style/visibility with full repaint existing 80x24 frame; 20 ASCII rows, uniform RGB foreground/background; {profile}; hidden cursor; reused memory sink; no terminal I/O"
                ),
            );
        }
    }
    if iterations == 0 {
        println!("{{\"traces\":[{}]}}", traces.join(","));
    }
}
fn cursor_trace() {
    let caps = Capabilities {
        color: ColorLevel::None,
        unicode: true,
        reduced_motion: true,
    };
    let mut encoder = FrameEncoder::default();
    let mut frame = View::text("a".repeat(80))
        .render(Size::new(80, 24))
        .unwrap();
    let mut sequences = Vec::new();
    for (i, cursor) in [
        Some(Cursor::new(2, 3)),
        Some(Cursor::new(4, 3)),
        Some(Cursor::new(2, 3)),
        None,
    ]
    .into_iter()
    .enumerate()
    {
        frame.set_cursor(cursor);
        let mut bytes = Vec::new();
        encoder
            .write(
                &mut bytes,
                &frame,
                if i == 0 { None } else { Some(&frame) },
                caps,
            )
            .unwrap();
        sequences.push(quote(&String::from_utf8(bytes).unwrap()));
    }
    println!(
        "{{\"width\":80,\"height\":24,\"sequences\":[{}]}}",
        sequences.join(",")
    );
}
fn cursor_encoding(iterations: usize) {
    let caps = Capabilities {
        color: ColorLevel::None,
        unicode: true,
        reduced_motion: true,
    };
    let mut frame = View::text("a".repeat(80))
        .render(Size::new(80, 24))
        .unwrap();
    frame.set_cursor(Some(Cursor::new(2, 3)));
    let mut encoder = FrameEncoder::default();
    let mut bytes = Vec::with_capacity(8192);
    encoder.write(&mut bytes, &frame, None, caps).unwrap();
    let mut flip = false;
    measure(
        "cursor/visible_move_80x24",
        iterations,
        || {
            bytes.clear();
            frame.set_cursor(Some(Cursor::new(if flip { 2 } else { 4 }, 3)));
            encoder
                .write(&mut bytes, &frame, Some(&frame), caps)
                .unwrap();
            flip = !flip;
            bytes.len() as u64
        },
        "visible cursor alternates (2,3),(4,3); independently replay output before claiming equivalence",
        "cursor-only move on identical immutable cells in reused 80x24 frame; visible, default style; reused memory sink; no terminal I/O",
    );
}
fn projectiles(iterations: usize) {
    use matea::motion::{Point3, Projectile, TERMINAL_GRAVITY, Vector3, frame_duration};
    use std::{num::NonZeroU32, time::Duration};
    let fps = NonZeroU32::new(60).unwrap();
    measure(
        "motion/frame_duration",
        (iterations * 10).max(300000),
        || frame_duration(black_box(fps)).as_secs_f64().to_bits(),
        &format!("{:.12}", frame_duration(fps).as_secs_f64()),
        "positive fps=60; integer nanosecond period converted to seconds",
    );
    let make = || {
        Projectile::new(
            Duration::from_millis(16),
            black_box(Point3 {
                x: 1.0,
                y: 10.0,
                z: -2.0,
            }),
            black_box(Vector3 {
                x: 2.0,
                y: -3.0,
                z: 4.0,
            }),
            black_box(TERMINAL_GRAVITY),
        )
        .unwrap()
    };
    let initial = make();
    let result = initial.sample();
    let (p, v) = (result.position(), result.velocity());
    let expected = format!(
        "{:.12},{:.12},{:.12},{:.12},{:.12},{:.12}",
        p.x, p.y, p.z, v.x, v.y, v.z
    );
    for checked in [false, true] {
        measure(
            if checked {
                "motion/projectile_checked_update"
            } else {
                "motion/projectile_update"
            },
            (iterations * 10).max(300000),
            || {
                let mut next = black_box(initial);
                if checked {
                    next.step().unwrap();
                } else {
                    next = next.sample();
                }
                let (p, v, a) = (next.position(), next.velocity(), next.acceleration());
                p.x.to_bits()
                    ^ p.y.to_bits()
                    ^ p.z.to_bits()
                    ^ v.x.to_bits()
                    ^ v.y.to_bits()
                    ^ v.z.to_bits()
                    ^ a.y.to_bits()
            },
            &expected,
            if checked {
                "copy initial projectile; forward Euler and six finite result checks; dt=16ms p=(1,10,-2) v=(2,-3,4) a=(0,9.81,0); read all getters"
            } else {
                "copy initial projectile; raw forward Euler; dt=16ms p=(1,10,-2) v=(2,-3,4) a=(0,9.81,0); read all getters"
            },
        );
    }
    measure(
        "motion/projectile_construct",
        (iterations * 10).max(300000),
        || {
            let mut next = black_box(make());
            let p = next.step().unwrap();
            let v = next.velocity();
            p.y.to_bits() ^ v.y.to_bits()
        },
        &expected,
        "construct with nine finite input checks then checked Euler; dt=16ms p=(1,10,-2) v=(2,-3,4) a=(0,9.81,0)",
    );
}
fn pagination(iterations: usize) {
    use matea::widgets::{Pagination, Paginator};
    use std::num::NonZeroUsize;
    let mut pages = Pagination::new(101, NonZeroUsize::new(10).unwrap());
    pages.set_page(10);
    assert_eq!(pages.range(), 100..101);
    measure(
        "pagination/range",
        (iterations * 10).max(300000),
        || {
            let range = black_box(pages).range();
            (range.start ^ range.end) as u64
        },
        "100,101",
        "collection length=101 per_page=10 page=10; slice bounds only",
    );
    pages.set_page(4);
    measure(
        "pagination/navigation",
        (iterations * 10).max(300000),
        || {
            let mut p = black_box(pages);
            p.next();
            p.previous();
            p.page() as u64
        },
        "4",
        "next then previous from page=4 total_pages=11 per_page=10; state helpers only",
    );
    let pager = Paginator::new(pages);
    assert_eq!(pager.label(), "5/11");
    measure(
        "pagination/arabic",
        iterations.max(1000),
        || black_box(&pager).label().len() as u64,
        "5/11",
        "Arabic label only current=5 total=11; no view/render/I/O",
    );
    for count in [32, 256] {
        let mut dots = Pagination::from_pages(count, NonZeroUsize::new(1).unwrap());
        dots.set_page(10);
        let expected = dots.dots("•", "○", count);
        measure(
            &format!("pagination/dots_{count}"),
            iterations.max(1000),
            || black_box(dots).dots("•", "○", black_box(count)).len() as u64,
            &expected,
            "dot label only all pages current=10 zero-based; active=• inactive=○; no view/render/I/O",
        );
    }
    let mut pager = Paginator::new(pages);
    let right = Input::Key(Key::new(KeyCode::Right));
    let left = Input::Key(Key::new(KeyCode::Left));
    measure(
        "pagination/key_dispatch",
        (iterations * 10).max(30000),
        || {
            black_box(black_box(&mut pager).event(black_box(&right), true));
            black_box(black_box(&mut pager).event(black_box(&left), true));
            pager.pages.page() as u64
        },
        "4",
        "next/right then previous/left key dispatch; initial page=4 total_pages=11; no view/render/I/O",
    );
    assert_eq!(pager.pages.page(), 4);
}
fn input_bindings(iterations: usize) {
    use matea::widgets::key::Binding;
    let ctrl_x = Key::new(KeyCode::Char('x')).with_modifiers(Modifiers::CONTROL);
    for (name, event, enabled, expected) in [
        ("ctrl_x", ctrl_x, true, 1),
        ("f12", Key::new(KeyCode::F(12)), true, 1),
        ("miss", Key::new(KeyCode::Char('y')), true, 0),
        ("disabled", ctrl_x, false, 0),
        ("repeat", ctrl_x.with_kind(KeyKind::Repeat), true, 1),
        ("release", ctrl_x.with_kind(KeyKind::Release), true, 0),
    ] {
        let mut binding = Binding::new([ctrl_x, Key::new(KeyCode::F(12))], "ctrl+x/F12", "inspect");
        binding.enabled = enabled;
        assert_eq!(u64::from(binding.matches(event)), expected);
        measure(
            &format!("input/binding_{name}"),
            (iterations * 10).max(30000),
            || u64::from(black_box(&binding).matches(black_box(event))),
            &expected.to_string(),
            "two typed/string shortcuts ctrl+x,F12; down-event policy; matching only; no construction or I/O",
        );
    }
    for (name, value, enabled, expected) in [
        ("hit", "F12", true, 1),
        ("miss", "other", true, 0),
        ("disabled", "F12", false, 0),
    ] {
        let mut binding = Binding::new(["ctrl+x", "F12"], "ctrl+x/F12", "inspect");
        binding.enabled = enabled;
        assert_eq!(u64::from(binding.matches_value(value)), expected);
        measure(
            &format!("binding/value_{name}"),
            (iterations * 10).max(30000),
            || u64::from(black_box(&binding).matches_value(black_box(value))),
            &expected.to_string(),
            "two custom string tokens ctrl+x,F12; borrowed exact case-sensitive equality; enabled policy; no key formatting or I/O",
        );
    }
    measure(
        "binding/construct",
        (iterations * 10).max(30000),
        || {
            let binding = black_box(Binding::new(
                [black_box("ctrl+x"), black_box("F12")],
                black_box("ctrl+x/F12"),
                black_box("inspect"),
            ));
            binding.keys.len() as u64
                + binding
                    .help()
                    .map_or(0, |(key, help)| (key.len() + help.len()) as u64)
        },
        "2,ctrl+x/F12,inspect,true",
        "construct binding with owned two-token vector/slice and short help metadata; retain result through black_box/global sink; borrowed static token text",
    );
    let custom = Binding::new(["ctrl+x", "F12"], "ctrl+x/F12", "inspect");
    let query = String::from("F12");
    assert!(!std::ptr::eq(custom.keys[1], query.as_str()));
    assert!(custom.matches_value(query.as_str()));
    measure(
        "binding/value_hit_owned",
        (iterations * 10).max(30000),
        || u64::from(black_box(&custom).matches_value(black_box(query.as_str()))),
        "1",
        "two custom string tokens ctrl+x,F12; equal F12 query has distinct owned storage; borrowed equality; no query construction, formatting or I/O",
    );
    let binding = Binding::new(["ctrl+x", "F12"], "ctrl+x/F12", "inspect");
    measure(
        "binding/metadata",
        (iterations * 10).max(30000),
        || {
            let b = black_box(&binding);
            let (label, description) = b.help().unwrap();
            b.keys.len() as u64
                + label.len() as u64
                + description.len() as u64
                + u64::from(b.is_enabled())
        },
        "2,ctrl+x/F12,inspect,true",
        "borrow enabled binding keys, help and enabled status; no mutation, formatting or I/O",
    );
}
fn text_input(iterations: usize) {
    for (name, initial, position, first, second) in [
        (
            "left_right_ascii_64",
            "a".repeat(64),
            32,
            Input::Key(Key::new(KeyCode::Left)),
            Input::Key(Key::new(KeyCode::Right)),
        ),
        (
            "insert_end_ascii_64",
            "a".repeat(64),
            64,
            Input::Key(Key::new(KeyCode::Char('x'))),
            Input::Key(Key::new(KeyCode::Backspace)),
        ),
        (
            "insert_middle_ascii_64",
            "a".repeat(64),
            32,
            Input::Key(Key::new(KeyCode::Char('x'))),
            Input::Key(Key::new(KeyCode::Backspace)),
        ),
        (
            "word_roundtrip_ascii_63",
            format!("{}tea", "tea ".repeat(15)),
            63,
            Input::Key(Key::new(KeyCode::Left).with_modifiers(Modifiers::ALT)),
            Input::Key(Key::new(KeyCode::Right).with_modifiers(Modifiers::ALT)),
        ),
        (
            "paste_clear_ascii_64",
            String::new(),
            0,
            Input::Paste("p".repeat(64)),
            Input::Key(Key::new(KeyCode::Char('u')).with_modifiers(Modifiers::CONTROL)),
        ),
        (
            "ignored_focus_ascii_64",
            "a".repeat(64),
            64,
            Input::Focus(false),
            Input::Focus(true),
        ),
    ] {
        let mut input = TextInput::new(&initial);
        input.set_cursor(position);
        input.reserve(128);
        let expected = format!("{},{}", initial.len(), position);
        let semantics = format!(
            "{name}; two events; ASCII editing only, no view, virtual cursor, clipboard, validator or suggestions; steady state with capacity reuse"
        );
        input.event(&first, true);
        input.event(&second, true);
        assert_eq!(input.value(), initial);
        assert_eq!(input.position(), position);
        measure(
            &format!("textinput/{name}"),
            (iterations * 10).max(10000),
            || {
                black_box(black_box(&mut input).event(black_box(&first), true));
                black_box(black_box(&mut input).event(black_box(&second), true));
                (input.value().len() ^ input.position()) as u64
            },
            &expected,
            &semantics,
        );
        assert_eq!(input.value(), initial);
        assert_eq!(input.position(), position);
    }
    let initial = "抹茶 e\u{301} 👩‍💻";
    let mut input = TextInput::new(initial);
    let left = Input::Key(Key::new(KeyCode::Left).with_modifiers(Modifiers::ALT));
    let right = Input::Key(Key::new(KeyCode::Right).with_modifiers(Modifiers::ALT));
    measure(
        "textinput/word_roundtrip_unicode",
        (iterations * 10).max(10000),
        || {
            black_box(black_box(&mut input).event(black_box(&left), true));
            black_box(black_box(&mut input).event(black_box(&right), true));
            input.value().len() as u64
        },
        &format!("{},end", initial.len()),
        "Unicode whitespace word left-right; same value and cursor at end verified outside timing; no view, virtual cursor, clipboard, validator or suggestions",
    );
    assert_eq!(input.value(), initial);
    assert_eq!(input.position(), input.grapheme_len());
    let mut input = TextInput::new("t");
    input.show_suggestions = true;
    input.set_suggestions(["tea", "team", "toast"]);
    input.reserve(16);
    let insert = Input::Key(Key::new(KeyCode::Char('e')));
    let delete = Input::Key(Key::new(KeyCode::Backspace));
    measure(
        "textinput/suggestions_edit_pair",
        (iterations * 10).max(10000),
        || {
            black_box(black_box(&mut input).event(black_box(&insert), true));
            black_box(black_box(&mut input).event(black_box(&delete), true));
            input.value().len() as u64
        },
        "t,3,0",
        "ASCII t -> te -> t with suggestions tea,team,toast; lowercase prefix filtering; no view, virtual cursor, clipboard or validator",
    );
    assert_eq!(input.value(), "t");
    assert_eq!(input.matched_suggestions().count(), 3);
    assert_eq!(input.current_suggestion_index(), Some(0));
}

fn virtual_cursor_cells(cursor: &VirtualCursor, theme: &Theme) -> String {
    let frame = cursor.view(theme).render(Size::new(16, 1)).unwrap();
    format!("{}|{}", frame.plain(), frame.row(0)[0].style().reverse)
}
fn virtual_cursor_cases(n: usize) {
    let theme = Theme::plain();
    let origin = Instant::now();
    for (name, text, mode, focused) in [
        ("visible_ascii", "tea", CursorMode::Static, true),
        ("visible_unicode", "抹e\u{301}👩‍💻", CursorMode::Static, true),
        ("blurred", "tea", CursorMode::Blink, false),
        ("hidden", "tea", CursorMode::Hidden, true),
    ] {
        let mut cursor = VirtualCursor::new_at(origin);
        cursor.set_char(text);
        cursor.set_mode_at(mode, origin);
        if focused {
            cursor.focus_at(origin);
        }
        let output = virtual_cursor_cells(&cursor, &theme);
        measure(
            &format!("virtual_cursor/{name}"),
            n,
            || black_box(virtual_cursor_cells(&cursor, &theme)).len() as u64,
            &output,
            "existing virtual cursor to fresh 16x1 grapheme cells; owned plain text and first-cell reverse flag; no scheduling or terminal I/O",
        );
    }
    for (name, mode) in [
        ("static_controls", CursorMode::Static),
        ("hidden_controls", CursorMode::Hidden),
    ] {
        let mut cursor = VirtualCursor::new_at(origin);
        cursor.set_mode_at(mode, origin);
        measure(
            &format!("virtual_cursor/{name}"),
            n,
            || {
                cursor.focus_at(black_box(origin));
                cursor.blur_at(black_box(origin));
                assert!(!cursor.visible(&theme));
                0
            },
            "false",
            "focus then blur existing nonblinking cursor; final visibility false; no scheduling or terminal I/O",
        );
    }
    let normal = Theme::default();
    let mut cursor = VirtualCursor::new_at(origin);
    cursor.focus_at(origin);
    let mut other = VirtualCursor::new_at(origin);
    other.focus_at(origin);
    let foreign = other
        .schedule(&normal)
        .unwrap()
        .tick_at(origin + std::time::Duration::from_secs(1));
    measure(
        "virtual_cursor/foreign",
        n,
        || {
            assert!(!cursor.update(black_box(foreign)));
            assert!(cursor.visible(&normal));
            1
        },
        "true",
        "ignore foreign tick on focused blinking cursor; final visibility true; no new scheduling request or terminal I/O",
    );
}
