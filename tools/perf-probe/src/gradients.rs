//! RGB8 CIELAB fixtures: fresh materialization and distinct reuse controls.
use crate::measurement::measure;
use matea::style::{Color, Gradient};
use std::hint::black_box;

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
