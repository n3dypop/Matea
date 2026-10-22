//! Finite matched flag operations and attribute-varying color-cache probes.
use crate::measurement::{measure, quote};
use crossterm::{
    Command,
    style::{Attribute, Attributes, SetAttributes},
};
use matea::{
    prelude::*,
    terminal::{Capabilities, FrameEncoder},
};
use std::hint::black_box;

const REFERENCE: [Attribute; 8] = [
    Attribute::Bold,
    Attribute::Dim,
    Attribute::Italic,
    Attribute::Reverse,
    Attribute::Hidden,
    Attribute::CrossedOut,
    Attribute::Fraktur,
    Attribute::OverLined,
];

fn matea_flags(mask: u8) -> u64 {
    let mut attributes = TextAttributes::new();
    for (index, flag) in TextAttribute::ALL.into_iter().enumerate() {
        if mask & (1 << index) != 0 {
            attributes = attributes.with(flag);
        }
    }
    TextAttribute::ALL
        .into_iter()
        .enumerate()
        .fold(0, |bits, (index, flag)| {
            bits | (u64::from(attributes.contains(flag)) << index)
        })
}
fn reference_flags(mask: u8) -> u64 {
    let mut attributes = Attributes::default();
    for (index, flag) in REFERENCE.into_iter().enumerate() {
        if mask & (1 << index) != 0 {
            attributes.set(flag);
        }
    }
    REFERENCE
        .into_iter()
        .enumerate()
        .fold(0, |bits, (index, flag)| {
            bits | (u64::from(attributes.has(flag)) << index)
        })
}

pub fn costs(iterations: usize) {
    let trace = std::env::args().any(|a| a == "--attribute-trace");
    // Every mask is checked independently, outside timing; both workloads build
    // a fresh resolved positive flag set, then query all eight logical flags.
    for mask in 0..=255u8 {
        assert_eq!(matea_flags(mask), u64::from(mask));
        assert_eq!(reference_flags(mask), u64::from(mask));
    }
    for mask in if trace { &[][..] } else { &[0, 1, 85, 255][..] } {
        let mask = *mask;
        for (implementation, operation) in [
            ("matea", matea_flags as fn(u8) -> u64),
            ("crossterm", reference_flags as fn(u8) -> u64),
        ] {
            measure(
                &format!("attributes/flags_{mask}_{implementation}"),
                iterations,
                || operation(black_box(mask)),
                &mask.to_string(),
                "fresh positive flag set from one u8 mask, eight typed flag insertions conditional on bits, then query all eight into the original logical mask; no colors, choice attributes, hashing or encoding",
            );
        }
    }
    for (profile, color, expected_colors) in [
        (
            "ansi16",
            ColorLevel::Ansi16,
            "\x1b[91m\x1b[40m\x1b[58;5;15m",
        ),
        (
            "ansi256",
            ColorLevel::Ansi256,
            "\x1b[91m\x1b[40m\x1b[58;5;15m",
        ),
        (
            "truecolor",
            ColorLevel::TrueColor,
            "\x1b[38;2;255;0;0m\x1b[48;2;0;0;0m\x1b[58;2;255;255;255m",
        ),
    ] {
        for variants in [8, 16] {
            let mut rows = Vec::new();
            let mut expected = String::from("\x1b[?25l\x1b[0m\x1b[2J");
            for group in 0..16 {
                let mask = group % variants;
                let mut style = Style::default()
                    .fg(Color::Rgb(255, 0, 0))
                    .bg(Color::Rgb(0, 0, 0))
                    .underline_color(Color::Rgb(255, 255, 255))
                    .underline_style(UnderlineStyle::Curly);
                let mut reference = Attributes::from(Attribute::Undercurled);
                for (index, flag) in TextAttribute::ALL.into_iter().enumerate() {
                    if mask & (1 << index) != 0 {
                        style = style.attribute(flag, true);
                        reference.set(REFERENCE[index]);
                    }
                }
                rows.push(View::styled("XXXXX", style));
                expected.push_str("\x1b[0m");
                expected.push_str(expected_colors);
                SetAttributes(reference).write_ansi(&mut expected).unwrap();
                if group == 0 {
                    expected.push_str("\x1b[H");
                }
                expected.push_str("XXXXX");
            }
            let frame = View::row(rows).render(Size::new(80, 1)).unwrap();
            let caps = Capabilities {
                color,
                unicode: true,
                reduced_motion: false,
            };
            let mut encoder = FrameEncoder::default();
            let mut sink = Vec::with_capacity(8192);
            encoder.write(&mut sink, &frame, None, caps).unwrap();
            assert_eq!(sink, expected.as_bytes());
            if trace {
                println!(
                    "{{\"case\":{},\"output\":{}}}",
                    quote(&format!("attributes/full_{variants}_{profile}")),
                    quote(&expected)
                );
            } else {
                measure(
                    &format!("attributes/full_{variants}_{profile}"),
                    iterations,
                    || {
                        sink.clear();
                        encoder.invalidate();
                        encoder
                            .write(black_box(&mut sink), black_box(&frame), None, caps)
                            .unwrap();
                        assert_eq!(sink, expected.as_bytes());
                        sink.len() as u64
                    },
                    &expected,
                    "full 80x1 ASCII repaint, 16 five-cell groups, shared RGB foreground/background/underline colors, 8 or 16 flag variants, curly underline; reused bounded palette cache/sink, explicit invalidation; includes complete byte equality against reference SGR and literal color fixture; no rendering allocation or terminal I/O",
                );
            }
        }
    }
}
