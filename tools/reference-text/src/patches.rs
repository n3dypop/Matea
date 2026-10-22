//! Original caller of the frozen public Style/Cell APIs; std-only profile.
use matea_style::{BlinkStyle, Color, Style, StylePatch, TextAttribute, UnderlineStyle};
use ratatui_core::{
    buffer::Cell,
    style::{Color as ReferenceColor, Modifier, Style as ReferenceStyle},
};

const FLAGS: [(TextAttribute, Modifier); 6] = [
    (TextAttribute::Bold, Modifier::BOLD),
    (TextAttribute::Dim, Modifier::DIM),
    (TextAttribute::Italic, Modifier::ITALIC),
    (TextAttribute::Reverse, Modifier::REVERSED),
    (TextAttribute::Hidden, Modifier::HIDDEN),
    (TextAttribute::Strikethrough, Modifier::CROSSED_OUT),
];
fn color(value: Color) -> ReferenceColor {
    match value {
        Color::Default => ReferenceColor::Reset,
        Color::Indexed(i) => ReferenceColor::Indexed(i),
        Color::Rgb(r, g, b) => ReferenceColor::Rgb(r, g, b),
    }
}
fn patch(mut choices: usize) -> (StylePatch, ReferenceStyle) {
    let mut a = StylePatch::new();
    let mut b = ReferenceStyle::new();
    for (attribute, flag) in FLAGS {
        match choices % 3 {
            0 => {}
            1 => {
                a = a.attribute(attribute, false);
                b = b.remove_modifier(flag);
            }
            _ => {
                a = a.attribute(attribute, true);
                b = b.add_modifier(flag);
            }
        }
        choices /= 3;
    }
    (a, b)
}
fn compare(actual: Style, expected: &Cell) {
    assert_eq!(color(actual.fg), expected.fg);
    assert_eq!(color(actual.bg), expected.bg);
    for (attribute, flag) in FLAGS {
        assert_eq!(actual.has(attribute), expected.modifier.contains(flag));
    }
    assert_eq!(
        actual.attributes.underline() != UnderlineStyle::None,
        expected.modifier.contains(Modifier::UNDERLINED)
    );
    assert_eq!(
        actual.attributes.blink() == BlinkStyle::Slow,
        expected.modifier.contains(Modifier::SLOW_BLINK)
    );
    assert_eq!(
        actual.attributes.blink() == BlinkStyle::Rapid,
        expected.modifier.contains(Modifier::RAPID_BLINK)
    );
}
pub(crate) fn conformance() -> usize {
    let mut cases = 0;
    for bits in 0..64 {
        let mut base = Style::default().fg(Color::Indexed(2)).bg(Color::Indexed(3));
        let mut reference = ReferenceStyle::new()
            .fg(ReferenceColor::Indexed(2))
            .bg(ReferenceColor::Indexed(3));
        for (i, (attribute, flag)) in FLAGS.into_iter().enumerate() {
            base = base.attribute(attribute, bits & (1 << i) != 0);
            if bits & (1 << i) != 0 {
                reference = reference.add_modifier(flag);
            }
        }
        for choices in 0..729 {
            let (a, b) = patch(choices);
            let mut cell = Cell::default();
            cell.set_style(reference).set_style(b);
            compare(a.resolve(base), &cell);
            cases += 1;
        }
    }
    for a in 0..729 {
        let (ma, ra) = patch(a);
        for b in 0..729 {
            let (mb, rb) = patch(b);
            let merged = ma.then(mb);
            let reference = ra.patch(rb);
            for (attribute, flag) in FLAGS {
                let expected = if reference.sub_modifier.contains(flag) {
                    Some(false)
                } else if reference.add_modifier.contains(flag) {
                    Some(true)
                } else {
                    None
                };
                assert_eq!(merged.attribute_intent(attribute), expected);
            }
            cases += 1;
        }
    }
    let colors = [
        Color::Default,
        Color::Indexed(0),
        Color::Indexed(15),
        Color::Indexed(255),
        Color::Rgb(0, 0, 0),
        Color::Rgb(19, 47, 233),
    ];
    for fg in colors {
        for bg in colors {
            for pfg in [None].into_iter().chain(colors.map(Some)) {
                for pbg in [None].into_iter().chain(colors.map(Some)) {
                    let base = Style::default().fg(fg).bg(bg).bold();
                    let mut a = StylePatch::new();
                    a.fg = pfg;
                    a.bg = pbg;
                    let mut b = ReferenceStyle::new();
                    b.fg = pfg.map(color);
                    b.bg = pbg.map(color);
                    let mut cell = Cell::default();
                    cell.set_style(
                        ReferenceStyle::new()
                            .fg(color(fg))
                            .bg(color(bg))
                            .add_modifier(Modifier::BOLD),
                    )
                    .set_style(b);
                    compare(a.resolve(base), &cell);
                    cases += 1;
                }
            }
        }
    }
    for underline in [false, true] {
        for blink in BlinkStyle::ALL {
            let base = Style::default()
                .underline_style(if underline {
                    UnderlineStyle::Single
                } else {
                    UnderlineStyle::None
                })
                .blink(blink);
            let mut reference = ReferenceStyle::new();
            if underline {
                reference = reference.add_modifier(Modifier::UNDERLINED);
            }
            reference = reference.add_modifier(match blink {
                BlinkStyle::None => Modifier::empty(),
                BlinkStyle::Slow => Modifier::SLOW_BLINK,
                BlinkStyle::Rapid => Modifier::RAPID_BLINK,
            });
            for pu in [None, Some(false), Some(true)] {
                for pb in [None].into_iter().chain(BlinkStyle::ALL.map(Some)) {
                    let mut a = StylePatch::new();
                    let mut b = ReferenceStyle::new();
                    if let Some(value) = pu {
                        a = a.underline_style(if value {
                            UnderlineStyle::Single
                        } else {
                            UnderlineStyle::None
                        });
                        b = if value {
                            b.add_modifier(Modifier::UNDERLINED)
                        } else {
                            b.remove_modifier(Modifier::UNDERLINED)
                        };
                    }
                    if let Some(value) = pb {
                        a = a.blink(value);
                        b = b
                            .remove_modifier(Modifier::SLOW_BLINK | Modifier::RAPID_BLINK)
                            .add_modifier(match value {
                                BlinkStyle::None => Modifier::empty(),
                                BlinkStyle::Slow => Modifier::SLOW_BLINK,
                                BlinkStyle::Rapid => Modifier::RAPID_BLINK,
                            });
                    }
                    let mut cell = Cell::default();
                    cell.set_style(reference).set_style(b);
                    compare(a.resolve(base), &cell);
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 579_933);
    conflicting_controls();
    cases
}

pub(crate) fn conflicting_controls() -> String {
    use std::fmt::Write;
    let mut output = String::new();
    for (attribute, flag) in FLAGS {
        // Direct source fields can overlap; both query and Cell::set_style
        // consistently prioritize removal. Do not call this an upstream bug.
        let source = ReferenceStyle {
            add_modifier: flag,
            sub_modifier: flag,
            ..ReferenceStyle::new()
        };
        let mut cell = Cell::default();
        cell.set_style(source);
        assert!(!source.has_modifier(flag));
        assert!(!cell.modifier.contains(flag));
        let enabled = StylePatch::new()
            .attribute(attribute, false)
            .attribute(attribute, true);
        let disabled = StylePatch::new()
            .attribute(attribute, true)
            .attribute(attribute, false);
        assert_eq!(enabled.attribute_intent(attribute), Some(true));
        assert!(enabled.resolve(Style::default()).has(attribute));
        assert_eq!(disabled.attribute_intent(attribute), Some(false));
        assert!(!disabled.resolve(Style::default()).has(attribute));
        writeln!(
            output,
            "{attribute:?}\t{}\t{}\t{}\t{}",
            source.has_modifier(flag),
            cell.modifier.contains(flag),
            enabled.resolve(Style::default()).has(attribute),
            disabled.resolve(Style::default()).has(attribute)
        )
        .unwrap();
    }
    output
}

fn color_record(color: Option<Color>) -> [u8; 5] {
    match color {
        None => [0; 5],
        Some(Color::Default) => [1, 0, 0, 0, 0],
        Some(Color::Indexed(i)) => [2, i, 0, 0, 0],
        Some(Color::Rgb(r, g, b)) => [3, r, g, b, 0],
    }
}
fn from_reference(color: ReferenceColor) -> Color {
    match color {
        ReferenceColor::Reset => Color::Default,
        ReferenceColor::Indexed(i) => Color::Indexed(i),
        ReferenceColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
        _ => panic!("named source color outside benchmark corpus"),
    }
}
fn intent(value: Option<bool>) -> u8 {
    match value {
        None => 0,
        Some(false) => 1,
        Some(true) => 2,
    }
}
fn matea_record(patch: StylePatch) -> [u8; 16] {
    let mut out = [0; 16];
    out[..5].copy_from_slice(&color_record(patch.fg));
    out[5..10].copy_from_slice(&color_record(patch.bg));
    for (i, (flag, _)) in FLAGS.into_iter().enumerate() {
        out[10 + i] = intent(patch.attribute_intent(flag));
    }
    out
}
fn reference_record(style: ReferenceStyle) -> [u8; 16] {
    let mut out = [0; 16];
    out[..5].copy_from_slice(&color_record(style.fg.map(from_reference)));
    out[5..10].copy_from_slice(&color_record(style.bg.map(from_reference)));
    for (i, (_, flag)) in FLAGS.into_iter().enumerate() {
        out[10 + i] = intent(if style.sub_modifier.contains(flag) {
            Some(false)
        } else if style.add_modifier.contains(flag) {
            Some(true)
        } else {
            None
        });
    }
    out
}
#[inline(always)]
fn matea_resolved(style: Style) -> [u8; 16] {
    let mut out = [0; 16];
    out[..5].copy_from_slice(&color_record(Some(style.fg)));
    out[5..10].copy_from_slice(&color_record(Some(style.bg)));
    for (i, (flag, _)) in FLAGS.into_iter().enumerate() {
        out[10 + i] = intent(Some(style.has(flag)));
    }
    out
}
#[inline(always)]
fn reference_resolved(cell: &Cell) -> [u8; 16] {
    let mut out = [0; 16];
    out[..5].copy_from_slice(&color_record(Some(from_reference(cell.fg))));
    out[5..10].copy_from_slice(&color_record(Some(from_reference(cell.bg))));
    for (i, (_, flag)) in FLAGS.into_iter().enumerate() {
        out[10 + i] = intent(Some(cell.modifier.contains(flag)));
    }
    out
}
fn build_matea(mut choices: usize) -> StylePatch {
    let mut patch = StylePatch::new()
        .fg(Color::Indexed(choices as u8))
        .bg(Color::Rgb(19, 47, 233));
    for (flag, _) in FLAGS {
        match choices % 3 {
            0 => {}
            1 => patch = patch.attribute(flag, false),
            _ => patch = patch.attribute(flag, true),
        };
        choices /= 3;
    }
    patch
}
fn build_reference(mut choices: usize) -> ReferenceStyle {
    let mut patch = ReferenceStyle::new()
        .fg(ReferenceColor::Indexed(choices as u8))
        .bg(ReferenceColor::Rgb(19, 47, 233));
    for (_, flag) in FLAGS {
        match choices % 3 {
            0 => {}
            1 => patch = patch.remove_modifier(flag),
            _ => patch = patch.add_modifier(flag),
        };
        choices /= 3;
    }
    patch
}
fn digest(bytes: &[u8]) -> u64 {
    bytes.iter().fold(14695981039346656037u64, |v, b| {
        (v ^ u64::from(*b)).wrapping_mul(1099511628211)
    })
}
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    for b in bytes {
        write!(out, "{b:02x}").unwrap();
    }
    out
}
fn full_record(patch: StylePatch) -> [u8; 25] {
    let mut out = [0; 25];
    out[..16].copy_from_slice(&matea_record(patch));
    out[16..21].copy_from_slice(&color_record(patch.underline_color));
    out[21] = intent(patch.attribute_intent(TextAttribute::Fraktur))
        | (intent(patch.attribute_intent(TextAttribute::Overline)) << 2);
    out[22] = patch.underline().map_or(0, |v| v as u8 + 1);
    out[23] = patch.blink_style().map_or(0, |v| v as u8 + 1);
    out[24] = patch.frame().map_or(0, |v| v as u8 + 1);
    out
}

pub(crate) fn costs(reference: bool) {
    use super::measurement::{iterations, measure};
    use std::hint::black_box;
    let n = iterations(1000);
    let base = Style::default()
        .fg(Color::Indexed(2))
        .bg(Color::Rgb(11, 41, 227))
        .bold()
        .italic();
    let mut cell = Cell::default();
    cell.set_style(
        ReferenceStyle::new()
            .fg(ReferenceColor::Indexed(2))
            .bg(ReferenceColor::Rgb(11, 41, 227))
            .add_modifier(Modifier::BOLD | Modifier::ITALIC),
    );
    let a = build_matea(137);
    let b = build_matea(593);
    let ra = build_reference(137);
    let rb = build_reference(593);
    let output = if reference {
        reference_record(build_reference(137))
    } else {
        matea_record(build_matea(137))
    };
    let semantics = "build FG/BG and six ternary boolean intents from opaque input 137, consume complete 16-byte canonical intent record with FNV64; default std profile, extra Matea groups excluded; no terminal I/O";
    if reference {
        measure(
            "style-patch/build",
            n,
            || digest(&reference_record(build_reference(black_box(137)))),
            &hex(&output),
            semantics,
        );
    } else {
        measure(
            "style-patch/build",
            n,
            || digest(&matea_record(build_matea(black_box(137)))),
            &hex(&output),
            semantics,
        );
    }
    let output = if reference {
        let mut s = cell.clone();
        s.set_style(ra);
        reference_resolved(&s)
    } else {
        matea_resolved(a.resolve(base))
    };
    let semantics = "apply opaque existing FG/BG/six-boolean patch to resolved base style/cell, consume complete 16-byte canonical FG/BG/boolean state with FNV64; Cell clone included for source; no terminal I/O";
    if reference {
        measure(
            "style-patch/resolve",
            n,
            || {
                let mut s = black_box(&cell).clone();
                s.set_style(black_box(ra));
                digest(&reference_resolved(&s))
            },
            &hex(&output),
            semantics,
        );
    } else {
        measure(
            "style-patch/resolve",
            n,
            || digest(&matea_resolved(black_box(a).resolve(black_box(base)))),
            &hex(&output),
            semantics,
        );
    }
    let output = if reference {
        reference_record(ra.patch(rb))
    } else {
        matea_record(a.then(b))
    };
    let semantics = "compose two opaque existing FG/BG/six-boolean intent values, consume all 16 canonical intent bytes with FNV64; last explicit setting wins; no terminal I/O";
    if reference {
        measure(
            "style-patch/compose",
            n,
            || digest(&reference_record(black_box(ra).patch(black_box(rb)))),
            &hex(&output),
            semantics,
        );
    } else {
        measure(
            "style-patch/compose",
            n,
            || digest(&matea_record(black_box(a).then(black_box(b)))),
            &hex(&output),
            semantics,
        );
    }
    let inherit = |mut value: ReferenceStyle| {
        value.fg = None;
        value.add_modifier.remove(Modifier::BOLD);
        value.sub_modifier.remove(Modifier::BOLD);
        value
    };
    let output = if reference {
        reference_record(inherit(ra))
    } else {
        matea_record(a.inherit_fg().inherit_attribute(TextAttribute::Bold))
    };
    let semantics = "copy opaque existing intent value, unset only foreground and bold intent, consume all 16 canonical intent bytes with FNV64; no terminal I/O";
    if reference {
        measure(
            "style-patch/inherit",
            n,
            || digest(&reference_record(inherit(black_box(ra)))),
            &hex(&output),
            semantics,
        );
    } else {
        measure(
            "style-patch/inherit",
            n,
            || {
                digest(&matea_record(
                    black_box(a)
                        .inherit_fg()
                        .inherit_attribute(TextAttribute::Bold),
                ))
            },
            &hex(&output),
            semantics,
        );
    }
    let output = if reference {
        reference_record(ReferenceStyle::reset().patch(cell.style()))
    } else {
        matea_record(StylePatch::from_style(base))
    };
    let semantics = "convert opaque resolved base style/cell to full explicit FG/BG/six-boolean intent and consume all 16 canonical bytes with FNV64; no terminal I/O";
    if reference {
        measure(
            "style-patch/override",
            n,
            || {
                digest(&reference_record(
                    ReferenceStyle::reset().patch(black_box(&cell).style()),
                ))
            },
            &hex(&output),
            semantics,
        );
    } else {
        measure(
            "style-patch/override",
            n,
            || digest(&matea_record(StylePatch::from_style(black_box(base)))),
            &hex(&output),
            semantics,
        );
    }
    let output = if reference {
        let mut value = cell.clone();
        value.set_style(ReferenceStyle::reset());
        reference_resolved(&value)
    } else {
        matea_resolved(StylePatch::reset().resolve(base))
    };
    let semantics = "select explicit reset versus inheritance from opaque true flag and apply to opaque resolved base style/cell, consume all 16 canonical bytes with FNV64; Cell clone included for source; no terminal I/O";
    if reference {
        measure(
            "style-patch/reset",
            n,
            || {
                let patch = if black_box(true) {
                    ReferenceStyle::reset()
                } else {
                    ReferenceStyle::new()
                };
                let mut value = black_box(&cell).clone();
                value.set_style(patch);
                digest(&reference_resolved(&value))
            },
            &hex(&output),
            semantics,
        );
    } else {
        measure(
            "style-patch/reset",
            n,
            || {
                let patch = if black_box(true) {
                    StylePatch::reset()
                } else {
                    StylePatch::new()
                };
                digest(&matea_resolved(patch.resolve(black_box(base))))
            },
            &hex(&output),
            semantics,
        );
    }
    if !reference {
        use matea_style::TextFrame;
        let make = |style: Style| {
            StylePatch::from_style(style)
                .underline_color(Color::Indexed(23))
                .underline_style(UnderlineStyle::Dashed)
                .blink(BlinkStyle::Rapid)
                .text_frame(TextFrame::Encircled)
                .attribute(TextAttribute::Overline, true)
        };
        let output = full_record(make(base));
        measure(
            "style-patch/decoration-control",
            n,
            || digest(&full_record(make(black_box(base)))),
            &hex(&output),
            "Rust-only expanded decoration/underline-color intent, complete 25-byte record; frozen std reference lacks these groups; no matched ratio",
        );
        let original = make(base);
        let output = full_record(
            original
                .inherit_underline()
                .inherit_underline_color()
                .inherit_blink()
                .inherit_frame(),
        );
        measure(
            "style-patch/inherit-decoration-control",
            n,
            || {
                digest(&full_record(
                    black_box(original)
                        .inherit_underline()
                        .inherit_underline_color()
                        .inherit_blink()
                        .inherit_frame(),
                ))
            },
            &hex(&output),
            "Rust-only unset expanded decoration and underline-color intents, complete 25-byte record; no matched ratio",
        );
    }
}
