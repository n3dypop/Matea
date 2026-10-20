//! Matched static progress frames and separate native lifecycle/scale controls.
use crate::measurement::measure;
use matea::motion::Motion;
use matea::prelude::*;
use std::hint::black_box;
use std::time::{Duration, Instant};

/// Transport input for an independent native console readback test, without
/// clock calibration or timing records. All selected symbols occupy one BMP
/// cell; supplementary glyph/font appearance is a separate validation domain.
pub(crate) fn console_trace() {
    use crate::measurement::quote;
    use matea::terminal::{Capabilities, FrameEncoder};
    let mut previous = None;
    let mut encoder = FrameEncoder::default();
    let theme = Theme::plain();
    let mut records = Vec::new();
    for (name, full, empty, unicode) in [
        ("ascii", '#', '-', false),
        ("block", '█', '░', true),
        ("half", '▄', '░', true),
    ] {
        for width in [40, 8, 60, 17] {
            for (step, fraction) in [(0, 0.0), (625, 0.625), (1000, 1.0)] {
                let mut model = Progress::new();
                model.set_width(width);
                model.set_glyphs(full, empty).unwrap();
                model.fill = ProgressFill::Solid(Color::Rgb(117, 113, 249));
                let frame = model
                    .view_as(fraction, &theme)
                    .unwrap()
                    .render(Size::new(width, 1))
                    .unwrap();
                let expected: String = frame.row(0).iter().map(|cell| cell.symbol()).collect();
                assert_eq!(expected.chars().count(), usize::from(width));
                let mut bytes = Vec::new();
                encoder
                    .write(
                        &mut bytes,
                        &frame,
                        previous.as_ref(),
                        Capabilities {
                            color: ColorLevel::None,
                            unicode,
                            reduced_motion: true,
                        },
                    )
                    .unwrap();
                records.push(format!(
                    "{{\"case\":{},\"width\":{width},\"ansi\":{},\"expected\":{}}}",
                    quote(&format!("{name}/{width}/{step}")),
                    quote(&String::from_utf8(bytes).unwrap()),
                    quote(&expected)
                ));
                previous = Some(frame);
            }
        }
    }
    println!(
        "{{\"schema\":1,\"kind\":\"native-windows-console-input\",\"read_width\":60,\"frames\":[{}]}}",
        records.join(",")
    );
}

pub(crate) const SEMANTICS: &str = "existing configured RGB8 progress bar at fraction 0.625; construct complete styled one-row frame and consume all UTF-8 symbols and foreground/background colors with FNV64; include frame allocation, formatting and rendering; no terminal I/O or animation";
fn configured(mode: usize, width: u16) -> Progress {
    let mut p = Progress::new();
    p.set_width(width);
    p.styles = ProgressStyles {
        full: Some(Style::default()),
        empty: Some(Style::default().fg(Color::Rgb(96, 96, 96))),
        percentage: Some(Style::default()),
    };
    p.fill = match mode {
        0 => ProgressFill::Solid(Color::Rgb(117, 113, 249)),
        1..=3 => ProgressFill::Gradient {
            gradient: Gradient::new(&[[90, 86, 224], [238, 111, 248]]).unwrap(),
            scaled: mode == 3,
        },
        _ => ProgressFill::ColorFn(Box::new(|total, current| {
            Color::Rgb((total * 255.0) as u8, (current * 255.0) as u8, 77)
        })),
    };
    if mode == 1 {
        p.show_percentage = false;
        p.set_glyphs('█', '░').unwrap();
    }
    if mode == 5 {
        p.set_glyphs('#', '-').unwrap();
    }
    p
}
fn feed(mut value: u64, bytes: &[u8]) -> u64 {
    for &byte in bytes {
        value = (value ^ u64::from(byte)).wrapping_mul(1099511628211);
    }
    value
}
fn feed_color(value: u64, color: Color) -> u64 {
    match color {
        Color::Default => feed(value, &[0]),
        Color::Rgb(r, g, b) => feed(value, &[1, r, g, b]),
        _ => panic!("RGB fixture"),
    }
}
pub(crate) fn checksum(surface: &Surface) -> u64 {
    let mut value = 14695981039346656037;
    for cell in surface.row(0) {
        value = feed(value, cell.symbol().as_bytes());
        value = feed_color(value, cell.style().fg);
        value = feed_color(value, cell.style().bg);
    }
    value
}
pub(crate) fn rgb(color: Color) -> String {
    match color {
        Color::Default => "default".into(),
        Color::Rgb(r, g, b) => format!("{r:02x}{g:02x}{b:02x}"),
        _ => panic!("RGB fixture"),
    }
}
pub(crate) fn costs(iterations: usize) {
    let theme = Theme::plain();
    for (mode, width) in [(0, 40), (1, 40), (2, 40), (3, 80), (4, 80), (5, 200)] {
        let progress = configured(mode, width);
        let surface = progress
            .view_as(0.625, &theme)
            .unwrap()
            .render(Size::new(width, 1))
            .unwrap();
        let output = surface
            .row(0)
            .iter()
            .map(|c| format!("{}:{}:{}", c.symbol(), rgb(c.style().fg), rgb(c.style().bg)))
            .collect::<Vec<_>>()
            .join(",");
        let expected = checksum(&surface);
        let fresh_frame = || {
            let surface = black_box(&progress)
                .view_as(black_box(0.625), black_box(&theme))
                .unwrap()
                .render(Size::new(black_box(width), 1))
                .unwrap();
            checksum(black_box(&surface))
        };
        assert_eq!(fresh_frame(), expected);
        measure(
            &format!("progress/frame_{mode}_{width}"),
            (iterations / 100).max(1),
            fresh_frame,
            &output,
            SEMANTICS,
        );
        let cells = || {
            let frame = black_box(&progress)
                .frame_as(black_box(0.625), black_box(&theme))
                .unwrap();
            let mut value = 14695981039346656037;
            for cell in frame.cells() {
                let mut buffer = [0; 4];
                value = feed(value, cell.symbol.encode_utf8(&mut buffer).as_bytes());
                value = feed_color(value, cell.style.fg);
                value = feed_color(value, cell.style.bg);
            }
            for symbol in frame.label().chars() {
                let mut buffer = [0; 4];
                value = feed(value, symbol.encode_utf8(&mut buffer).as_bytes());
                value = feed_color(value, frame.label_style().fg);
                value = feed_color(value, frame.label_style().bg);
            }
            value
        };
        assert_eq!(cells(), expected);
        measure(
            &format!("progress/cells_{mode}_{width}"),
            (iterations / 100).max(1),
            cells,
            &output,
            "existing configured RGB8 progress bar at fraction 0.625; format percentage and stream complete bar cells and label with FNV64; omit View and Surface materialization; no terminal I/O or animation; separate Rust control without upstream ratio",
        );
    }
}

// These are Rust API controls. Constructor, reset and assertion work is inside
// each operation where used; none receives a cross-language timing ratio.
fn lifecycle_operation(case: usize, origin: Instant, theme: &Theme) -> u64 {
    let mut progress = match case {
        1 => Progress::new(),
        _ => Progress::new_at(black_box(origin)),
    };
    match case {
        0 | 1 => {
            let progress = black_box(progress);
            assert_eq!(progress.width(), 40);
            assert_eq!(progress.target(), 0.0);
            assert_eq!(progress.motion(), Motion::default());
            assert!(progress.schedule(theme).is_none());
            40
        }
        2 => {
            for (frequency, damping) in [(18.0, 1.0), (18.0, 0.1), (18.0, 2.0)] {
                progress
                    .set_spring(black_box(frequency), black_box(damping))
                    .unwrap();
            }
            assert_eq!(black_box(&progress).target(), 0.0);
            3
        }
        3 => {
            let mut errors = 0;
            for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                errors += u64::from(progress.set_percent_at(black_box(invalid), origin).is_err());
                errors += u64::from(progress.set_spring(black_box(invalid), 1.0).is_err());
                errors += u64::from(progress.frame_as(black_box(invalid), theme).is_err());
            }
            assert_eq!(progress.target(), 0.0);
            assert_eq!(progress.motion(), Motion::default());
            assert!(progress.schedule(theme).is_none());
            errors
        }
        4 => {
            for i in 0..64 {
                assert!(
                    progress
                        .set_percent_at(black_box(f64::from(i % 2)), origin)
                        .unwrap()
                );
            }
            assert_eq!(progress.target(), 1.0);
            assert_eq!(progress.motion(), Motion::default());
            assert!(progress.schedule(theme).is_some());
            64
        }
        5 => {
            progress.set_percent_at(1.0, origin).unwrap();
            let mut steps = 0;
            for frame in 1..=120 {
                let Some(schedule) = progress.schedule(theme) else {
                    break;
                };
                let tick = schedule.tick_at(origin + Duration::from_nanos(16_666_666 * frame));
                assert!(progress.update(black_box(tick)).unwrap());
                steps += 1;
            }
            assert_eq!(progress.motion(), Motion::at_rest(1.0).unwrap());
            assert!(!progress.is_animating());
            assert!(steps > 0 && steps < 120);
            1
        }
        6 => {
            progress.set_percent_at(0.75, origin).unwrap();
            let old = progress.schedule(theme).unwrap();
            let tick = old.tick_at(origin + Duration::from_millis(50));
            assert!(progress.update(tick).unwrap());
            let state = progress.motion();
            assert!(!progress.update(black_box(tick)).unwrap());
            assert!(!progress.update(old.tick_at(origin)).unwrap());
            progress.set_percent_at(0.25, tick.at()).unwrap();
            assert!(
                !progress
                    .update(old.tick_at(origin + Duration::from_millis(100)))
                    .unwrap()
            );
            assert_eq!(progress.motion(), state);
            assert_eq!(progress.target(), 0.25);
            3
        }
        7 => {
            progress.set_percent_at(0.75, origin).unwrap();
            let mut present = 0;
            for _ in 0..64 {
                present += u64::from(black_box(&progress).schedule(black_box(theme)).is_some());
            }
            assert!(progress.set_reduced_motion_at(true, origin));
            assert_eq!(progress.motion(), Motion::at_rest(0.75).unwrap());
            assert!(progress.schedule(theme).is_none());
            assert!(!progress.set_reduced_motion_at(true, origin));
            present
        }
        8 => {
            let progress = black_box(configured(3, 80));
            assert_eq!(progress.width(), 80);
            assert!(!progress.is_animating());
            80
        }
        _ => unreachable!(),
    }
}

const SCALES: [(&str, u16, u8); 6] = [
    ("zero", 0, 0),
    ("narrow", 1, 0),
    ("wide", 1024, 0),
    ("maximum", u16::MAX, 0),
    ("long_ascii_label", 80, 1),
    ("long_unicode_control_label", 80, 2),
];

fn api_operation(case: usize, origin: Instant, theme: &Theme) -> u64 {
    use matea::core::Component;
    let mut progress = if case == 3 {
        configured(0, 40)
    } else {
        Progress::new_at(black_box(origin))
    };
    match case {
        0 => {
            progress.set_width(u16::MAX);
            assert_eq!(black_box(&progress).width(), u16::MAX);
            progress.set_width(80);
            for glyph in ['\0', '\t', '\u{301}', '茶', '👩'] {
                assert!(progress.set_glyphs(black_box(glyph), '.').is_err());
            }
            progress.set_glyphs('#', '-').unwrap();
            assert_eq!(progress.glyphs(), ('#', '-'));
            let prefix = String::from("v");
            progress.set_percentage_formatter(move |fraction| format!("{prefix}:{fraction:.3}"));
            assert_eq!(progress.frame_as(0.625, theme).unwrap().label(), "v:0.625");
            progress.reset_percentage_formatter();
            assert_eq!(progress.frame_as(0.625, theme).unwrap().label(), "  62%");
            5
        }
        1 => {
            let id = progress.id();
            assert!(progress.set_percent(black_box(0.5)).unwrap());
            assert!(progress.increment(black_box(0.25)).unwrap());
            assert!(progress.decrement(black_box(0.5)).unwrap());
            assert_eq!(progress.target(), 0.25);
            assert!(progress.set_reduced_motion(true));
            assert!(progress.set_percent(black_box(0.625)).unwrap());
            assert!(progress.set_reduced_motion(false));
            assert_eq!(progress.motion(), Motion::at_rest(0.625).unwrap());
            assert_eq!(progress.id(), id);
            // Explicitly order the backward observation even on hosts whose
            // real clock returns equal readings for consecutive controls.
            assert!(
                progress
                    .set_percent_at(0.625, Instant::now() + Duration::from_secs(1))
                    .unwrap()
            );
            assert!(!progress.set_percent_at(1.0, origin).unwrap());
            6
        }
        2 => {
            progress.set_percent_at(1.0, origin).unwrap();
            let subscription = black_box(progress.subscription(theme, |tick: ClockTick| tick));
            assert!(subscription.is_some());
            drop(subscription);
            let reduced = Theme {
                reduced_motion: true,
                ..theme.clone()
            };
            assert!(
                progress
                    .subscription(&reduced, |tick: ClockTick| tick)
                    .is_none()
            );
            assert!(progress.set_reduced_motion_at(true, origin));
            assert!(
                progress
                    .subscription(theme, |tick: ClockTick| tick)
                    .is_none()
            );
            1
        }
        3 => {
            progress.set_reduced_motion(true);
            progress.set_percent(0.625).unwrap();
            let response = black_box(Component::event(&mut progress, &Input::Focus(true), true));
            assert!(!response.consumed && response.message.is_none());
            let surface = Component::view(&progress, theme, true)
                .render(Size::new(40, 1))
                .unwrap();
            checksum(black_box(&surface))
        }
        4 => {
            progress.set_width(17);
            let frame = progress.frame_as(0.5, theme).unwrap();
            assert_eq!(frame.bar_width(), 12);
            assert_eq!(frame.filled_width(), 6);
            assert_eq!(frame.label_style(), theme.text);
            let mut cells = frame.cells();
            assert_eq!(cells.len(), 12);
            assert_eq!(cells.next(), frame.cell_at(0));
            assert_eq!(cells.next_back(), frame.cell_at(11));
            assert_eq!(cells.nth(3), frame.cell_at(4));
            assert_eq!(cells.nth_back(3), frame.cell_at(7));
            assert_eq!(cells.len(), 2);
            assert!(cells.nth(usize::MAX).is_none());
            assert!(cells.next_back().is_none());
            assert!(frame.cell_at(usize::MAX).is_none());
            12
        }
        _ => unreachable!(),
    }
}
pub(crate) fn api_costs(iterations: usize) {
    let origin = Instant::now();
    let theme = Theme {
        reduced_motion: false,
        ..Theme::plain()
    };
    for (case, name, output) in [
        (0, "configure_formatters_glyph_errors", "5"),
        (1, "clocked_fraction_controls", "6"),
        (2, "subscription_construction_reduction", "1"),
        (3, "component_live_frame", "13470074180807873566"),
        (4, "borrowed_iterator_boundaries", "12"),
    ] {
        let operation = || api_operation(black_box(case), origin, &theme);
        assert_eq!(operation(), output.parse::<u64>().unwrap());
        measure(
            &format!("progress/api_{name}"),
            iterations,
            operation,
            output,
            "Rust native public-API control; include fresh construction/drop, all named mutations, clock reads, error validation/assertions, formatter/subscription ownership and live frame when named; no terminal transport or upstream timing ratio",
        );
    }
}
fn scale_progress(width: u16, custom: u8) -> Progress {
    let mut progress = configured(0, width);
    if custom == 1 {
        progress.set_percentage_formatter(|_| "x".repeat(8192));
    } else if custom == 2 {
        progress.set_percentage_formatter(|_| "e\u{301}茶\n\u{1b}".repeat(1024));
    }
    progress
}
pub(crate) fn lifecycle_costs(iterations: usize) {
    let theme = Theme {
        reduced_motion: false,
        ..Theme::plain()
    };
    let origin = Instant::now();
    for (case, name, output) in [
        (0, "new_at", "40"),
        (1, "new_clock", "40"),
        (2, "new_three_springs", "3"),
        (3, "new_nine_errors", "9"),
        (4, "new_retarget_64", "64"),
        (5, "new_animate_to_rest", "1"),
        (6, "new_reject_three_ticks", "3"),
        (7, "new_schedule_64_reduce", "64"),
        (8, "new_gradient", "80"),
    ] {
        let operation = || lifecycle_operation(black_box(case), origin, &theme);
        assert_eq!(operation(), output.parse::<u64>().unwrap());
        measure(
            &format!("progress/lifecycle_{name}"),
            iterations,
            operation,
            output,
            "Rust native control; include fresh owned Progress construction/drop and every named validated operation/assertion; no terminal I/O, subscriptions or upstream timing ratio",
        );
    }
    for (name, width, custom) in SCALES {
        let progress = scale_progress(width, custom);
        let surface = progress
            .view_as(0.625, &theme)
            .unwrap()
            .render(Size::new(width, 1))
            .unwrap();
        let expected = checksum(&surface);
        let output = format!("{width}:{}:{expected:016x}", surface.plain());
        measure(
            &format!("progress/scale_{name}"),
            (iterations / 100).max(1),
            || {
                let surface = black_box(&progress)
                    .view_as(black_box(0.625), &theme)
                    .unwrap()
                    .render(Size::new(black_box(width), 1))
                    .unwrap();
                checksum(black_box(&surface))
            },
            &output,
            "Rust native control; existing solid RGB8 bar, full owned View/Surface render and all-cell FNV; width/long formatter scale, include sanitation/clipping/formatting and requested allocations; no upstream timing ratio",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lifecycle_controls_check_results_and_transactional_state() {
        let theme = Theme {
            reduced_motion: false,
            ..Theme::plain()
        };
        let origin = Instant::now();
        for (case, expected) in [40, 40, 3, 9, 64, 1, 3, 64, 80].into_iter().enumerate() {
            assert_eq!(lifecycle_operation(case, origin, &theme), expected);
        }
    }
    #[test]
    fn supplementary_controls_check_public_ownership_and_adapters() {
        let theme = Theme {
            reduced_motion: false,
            ..Theme::plain()
        };
        let origin = Instant::now();
        for (case, expected) in [5, 6, 1, 13470074180807873566, 12].into_iter().enumerate() {
            assert_eq!(api_operation(case, origin, &theme), expected);
        }
    }
    #[test]
    fn scale_controls_match_independent_complete_cell_digests() {
        let expected = [
            0xcbf29ce484222325,
            0xc41db217cf0f5a57,
            0x1a8feae50a706538,
            0xdcdd3e763e9d4ff8,
            0xab0f676942233465,
            0xb13c9b1700827765,
        ];
        for ((_, width, custom), expected) in SCALES.into_iter().zip(expected) {
            let surface = scale_progress(width, custom)
                .view_as(0.625, &Theme::plain())
                .unwrap()
                .render(Size::new(width, 1))
                .unwrap();
            assert_eq!(checksum(&surface), expected);
        }
    }
}
