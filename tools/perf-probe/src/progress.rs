//! Complete static progress frame comparisons; no terminal I/O or animation.
use crate::measurement::measure;
use matea::prelude::*;
use std::hint::black_box;

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
fn checksum(surface: &Surface) -> u64 {
    let mut value = 14695981039346656037;
    for cell in surface.row(0) {
        value = feed(value, cell.symbol().as_bytes());
        value = feed_color(value, cell.style().fg);
        value = feed_color(value, cell.style().bg);
    }
    value
}
fn rgb(color: Color) -> String {
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
