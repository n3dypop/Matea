//! RGB8 CIELAB fixtures: fresh materialization and distinct reuse controls.
use crate::measurement::measure;
use matea::prelude::{Progress, ProgressFill, ProgressStyles, Size, Style, Theme};
use matea::style::{Color, Gradient};
use std::hint::black_box;

const PRECISE: [[[u16; 4]; 2]; 2] = [
    [[32768, 8192, 16384, 32768], [65535; 4]],
    [[32768, 8192, 16384, 0], [65535; 4]],
];
pub(crate) const PRECISE_LINEAR_SEMANTICS: &str = "fresh premultiplied RGBA16-stop CIELAB configuration, complete 80-color canonical RGB8 vector and FNV64 checksum; include allocation, integer normalization and all output colors; no formatting or terminal I/O";
pub(crate) const PRECISE_FRAME_SEMANTICS: &str = "existing configured premultiplied RGBA16 progress at fraction 0.625; construct complete styled one-row frame and consume all UTF-8 symbols and foreground/background colors with FNV64; include frame allocation, formatting and rendering; no terminal I/O or animation";

fn precise_progress(mode: usize, width: u16) -> Progress {
    let mut progress = Progress::new();
    progress.set_width(width);
    progress.styles = ProgressStyles {
        full: Some(Style::default()),
        empty: Some(Style::default().fg(Color::Rgb(96, 96, 96))),
        percentage: Some(Style::default()),
    };
    progress.fill = ProgressFill::Gradient {
        gradient: Gradient::from_premultiplied_rgba16(&PRECISE[mode]).unwrap(),
        scaled: false,
    };
    progress
}
fn precise_control(case: usize) -> u64 {
    if case == 4 {
        let invalid = [[0; 4], [2, 1, 1, 1]];
        assert!(matches!(
            Gradient::from_premultiplied_rgba16(black_box(&invalid)),
            Err(matea::style::GradientError::InvalidPremultipliedStop { index: 1 })
        ));
        return 1;
    }
    if case == 5 {
        let mut value = 0;
        for channel in [
            0, 1, 2, 127, 128, 254, 255, 256, 257, 511, 512, 65534, 65535,
        ] {
            let Color::Rgb(r, g, b) =
                Color::rgb16(black_box(channel), black_box(channel), black_box(channel))
            else {
                unreachable!()
            };
            value += u64::from(r) + u64::from(g) + u64::from(b);
        }
        return value;
    }
    let opaque = [[32896, 8224, 16448, 65535], [65535; 4]];
    let stops: &[[u16; 4]] = match case {
        0 => &PRECISE[0],
        1 => &PRECISE[1],
        2 => &PRECISE[0][..1],
        3 => &opaque,
        _ => unreachable!(),
    };
    let gradient = Gradient::from_premultiplied_rgba16(black_box(stops)).unwrap();
    black_box(&gradient);
    checksum(gradient.colors(3).map(rgb))
}
pub(crate) fn precise_costs(iterations: usize) {
    let theme = Theme::plain();
    for (mode, stops) in PRECISE.iter().enumerate() {
        let configured = Gradient::from_premultiplied_rgba16(stops).unwrap();
        let colors = configured.colors(80).map(rgb).collect::<Vec<_>>();
        let output = colors
            .iter()
            .map(|[r, g, b]| format!("{r:02x}{g:02x}{b:02x}"))
            .collect::<Vec<_>>()
            .join(",");
        let expected = checksum(colors);
        let fresh = || {
            let gradient = Gradient::from_premultiplied_rgba16(black_box(stops)).unwrap();
            let colors = black_box(gradient.colors(black_box(80)).map(rgb).collect::<Vec<_>>());
            checksum(colors.iter().copied())
        };
        assert_eq!(fresh(), expected);
        measure(
            &format!("precise/fresh_{mode}_80"),
            iterations,
            fresh,
            &output,
            PRECISE_LINEAR_SEMANTICS,
        );
        let width = if mode == 0 { 40 } else { 80 };
        let progress = precise_progress(mode, width);
        let surface = progress
            .view_as(0.625, &theme)
            .unwrap()
            .render(Size::new(width, 1))
            .unwrap();
        let output = surface
            .row(0)
            .iter()
            .map(|c| {
                format!(
                    "{}:{}:{}",
                    c.symbol(),
                    crate::progress::rgb(c.style().fg),
                    crate::progress::rgb(c.style().bg)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let expected = crate::progress::checksum(&surface);
        let operation = || {
            let surface = black_box(&progress)
                .view_as(black_box(0.625), &theme)
                .unwrap()
                .render(Size::new(black_box(width), 1))
                .unwrap();
            crate::progress::checksum(black_box(&surface))
        };
        assert_eq!(operation(), expected);
        measure(
            &format!("precise/frame_{mode}_{width}"),
            iterations,
            operation,
            &output,
            PRECISE_FRAME_SEMANTICS,
        );
    }
    for (case, name) in [
        "ctor_half",
        "ctor_lost_alpha",
        "ctor_single",
        "ctor_opaque",
        "invalid_alpha",
        "convert_13",
    ]
    .into_iter()
    .enumerate()
    {
        let expected = precise_control(case);
        measure(
            &format!("precise/{name}"),
            iterations,
            || precise_control(black_box(case)),
            &expected.to_string(),
            "Rust API control: fresh precise Gradient construction/drop and three samples (or named validation/13 scalar conversions); no upstream constructor ratio or terminal I/O",
        );
    }
}

const STOPS: [[u8; 3]; 3] = [[90, 86, 224], [238, 111, 248], [7, 201, 33]];
#[derive(Clone, Copy)]
struct Spec {
    name: &'static str,
    count: usize,
    width: u16,
    height: u16,
    angle: f64,
}
const CASES: [Spec; 6] = [
    Spec {
        name: "linear_8",
        count: 8,
        width: 0,
        height: 0,
        angle: 0.0,
    },
    Spec {
        name: "multi_80",
        count: 80,
        width: 0,
        height: 0,
        angle: 0.0,
    },
    Spec {
        name: "multi_1024",
        count: 1024,
        width: 0,
        height: 0,
        angle: 0.0,
    },
    Spec {
        name: "grid_16x8_45",
        count: 0,
        width: 16,
        height: 8,
        angle: 45.0,
    },
    Spec {
        name: "grid_80x24_0",
        count: 0,
        width: 80,
        height: 24,
        angle: 0.0,
    },
    Spec {
        name: "grid_80x24_90",
        count: 0,
        width: 80,
        height: 24,
        angle: 90.0,
    },
];
fn rgb(color: Color) -> [u8; 3] {
    match color {
        Color::Rgb(r, g, b) => [r, g, b],
        _ => panic!("gradient returned a non-RGB color"),
    }
}
fn values(gradient: &Gradient, spec: Spec) -> Vec<[u8; 3]> {
    if spec.width == 0 {
        gradient.colors(spec.count).map(rgb).collect()
    } else {
        gradient
            .grid(spec.width, spec.height, spec.angle)
            .unwrap()
            .colors()
            .map(rgb)
            .collect()
    }
}
fn checksum(colors: impl IntoIterator<Item = [u8; 3]>) -> u64 {
    colors
        .into_iter()
        .flatten()
        .fold(14695981039346656037, |value, channel| {
            (value ^ u64::from(channel)).wrapping_mul(1099511628211)
        })
}
pub(crate) fn costs(iterations: usize) {
    let gradient = Gradient::new(&STOPS).unwrap();
    for spec in CASES {
        let output = values(&gradient, spec)
            .iter()
            .map(|[r, g, b]| format!("{r:02x}{g:02x}{b:02x}"))
            .collect::<Vec<_>>()
            .join(",");
        let expected = checksum(values(&gradient, spec));
        let fresh = || {
            let configured = Gradient::new(black_box(&STOPS)).unwrap();
            let colors = black_box(values(&configured, black_box(spec)));
            checksum(colors.iter().copied())
        };
        assert_eq!(fresh(), expected);
        measure(
            &format!("gradients/fresh_{}", spec.name),
            (iterations / 100).max(1),
            fresh,
            &output,
            "fresh RGB8-stop CIELAB configuration, complete normalized RGB8 vector and FNV64 checksum; include allocation and all output colors; no formatting or terminal I/O",
        );
        let semantics = "reused immutable CIELAB configuration and optional precomputed diagonal palette; stream all RGB8 colors and FNV64 checksum without materializing output; no formatting or terminal I/O";
        if spec.width == 0 {
            let reused = || checksum(black_box(&gradient).colors(black_box(spec.count)).map(rgb));
            assert_eq!(reused(), expected);
            measure(
                &format!("gradients/reused_{}", spec.name),
                (iterations / 100).max(1),
                reused,
                &output,
                semantics,
            );
        } else {
            let grid = gradient.grid(spec.width, spec.height, spec.angle).unwrap();
            let reused = || checksum(black_box(&grid).colors().map(rgb));
            assert_eq!(reused(), expected);
            measure(
                &format!("gradients/reused_{}", spec.name),
                (iterations / 100).max(1),
                reused,
                &output,
                semantics,
            );
        }
    }
}

#[cfg(test)]
mod precise_tests {
    use super::*;
    #[test]
    fn precise_controls_match_independent_source_projections_and_literal_validation() {
        // FNV values derive from pinned public RGBA16 projections, then the
        // explicit canonical 16-to-8-bit conversion; no ANSI depth guessing.
        for (case, expected) in [
            12561013804634252943,
            6340143887728155171,
            14091479019096348767,
            6340143887728155171,
            1,
            1545,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(precise_control(case), expected);
        }
    }
}
