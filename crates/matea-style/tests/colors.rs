use matea_style::{Color, ColorLevel, Style, TextAttribute, UnderlineStyle};

// Independent, literal canonical xterm palette. The production implementation's
// private palette/distance/candidate helpers are intentionally not reused.
fn palette() -> Vec<[i32; 3]> {
    let mut p = vec![
        [0, 0, 0],
        [128, 0, 0],
        [0, 128, 0],
        [128, 128, 0],
        [0, 0, 128],
        [128, 0, 128],
        [0, 128, 128],
        [192, 192, 192],
        [128, 128, 128],
        [255, 0, 0],
        [0, 255, 0],
        [255, 255, 0],
        [0, 0, 255],
        [255, 0, 255],
        [0, 255, 255],
        [255, 255, 255],
    ];
    for r in [0, 95, 135, 175, 215, 255] {
        for g in [0, 95, 135, 175, 215, 255] {
            for b in [0, 95, 135, 175, 215, 255] {
                p.push([r, g, b]);
            }
        }
    }
    for n in 0..24 {
        p.push([8 + 10 * n; 3]);
    }
    p
}

fn closest([r, g, b]: [i32; 3], palette: &[[i32; 3]]) -> u8 {
    let mut best = (i32::MAX, 0);
    for (index, &[pr, pg, pb]) in palette.iter().enumerate() {
        let distance = (r - pr) * (r - pr) + (g - pg) * (g - pg) + (b - pb) * (b - pb);
        if distance < best.0 {
            best = (distance, index as u8);
        }
    }
    best.1
}

#[test]
fn palette_and_midpoint_boundaries_match_independent_search() {
    let p = palette();
    for &[r, g, b] in &p {
        for (level, count) in [(ColorLevel::Ansi16, 16), (ColorLevel::Ansi256, 256)] {
            assert_eq!(
                Color::Rgb(r as u8, g as u8, b as u8).adapt(level),
                Color::Indexed(closest([r, g, b], &p[..count]))
            );
        }
    }
    let values: &[u8] = if cfg!(miri) {
        &[0, 47, 48, 115, 255]
    } else {
        &[
            0, 1, 7, 8, 9, 47, 48, 94, 95, 96, 114, 115, 116, 134, 135, 136, 154, 155, 156, 174,
            175, 176, 194, 195, 196, 214, 215, 216, 234, 235, 236, 254, 255,
        ]
    };
    for &r in values {
        for &g in values {
            for &b in values {
                assert_eq!(
                    Color::Rgb(r, g, b).adapt(ColorLevel::Ansi256),
                    Color::Indexed(closest([r.into(), g.into(), b.into()], &p)),
                    "RGB {r},{g},{b}"
                );
            }
        }
    }
}

#[test]
fn profiles_preserve_indices_attributes_and_first_index_ties() {
    let p = palette();
    for n in 0..=255 {
        let color = Color::Indexed(n);
        assert_eq!(color.adapt(ColorLevel::Ansi256), color);
        assert_eq!(color.adapt(ColorLevel::TrueColor), color);
        assert_eq!(color.adapt(ColorLevel::None), Color::Default);
        let expected = if n < 16 {
            n
        } else {
            closest(p[usize::from(n)], &p[..16])
        };
        assert_eq!(color.adapt(ColorLevel::Ansi16), Color::Indexed(expected));
    }
    for level in [
        ColorLevel::None,
        ColorLevel::Ansi16,
        ColorLevel::Ansi256,
        ColorLevel::TrueColor,
    ] {
        assert_eq!(Color::Default.adapt(level), Color::Default);
        for underline_color in [Color::Default, Color::Rgb(13, 13, 13)] {
            let s = Style::default()
                .fg(Color::Rgb(27, 160, 230))
                .bg(Color::Indexed(200))
                .underline_color(underline_color)
                .bold()
                .underline()
                .reverse();
            let out = s.adapt(level);
            assert_eq!(out.fg, s.fg.adapt(level));
            assert_eq!(out.bg, s.bg.adapt(level));
            let underline = if underline_color == Color::Default {
                Color::Default
            } else {
                match level {
                    ColorLevel::None => Color::Default,
                    ColorLevel::Ansi16 => Color::Indexed(closest([13, 13, 13], &p[..16])),
                    ColorLevel::Ansi256 => Color::Indexed(closest([13, 13, 13], &p)),
                    ColorLevel::TrueColor => Color::Rgb(13, 13, 13),
                }
            };
            assert_eq!(out.underline_color, underline);
            assert!(
                out.has(TextAttribute::Bold)
                    && out.attributes.underline() == UnderlineStyle::Single
                    && out.has(TextAttribute::Reverse)
            );
        }
    }
    assert_eq!(
        Color::Rgb(0, 0, 0).adapt(ColorLevel::Ansi256),
        Color::Indexed(0)
    );
    assert_eq!(
        Color::Rgb(255, 255, 255).adapt(ColorLevel::Ansi256),
        Color::Indexed(15)
    );
    assert_eq!(
        Color::Rgb(13, 13, 13).adapt(ColorLevel::Ansi256),
        Color::Indexed(232)
    );
}

#[test]
#[ignore = "Run explicitly in release: exhaustive 16,777,216 RGB triples against full search"]
fn every_rgb_triple_matches_full_palette_search() {
    let p = palette();
    for r in 0..=255u8 {
        for g in 0..=255u8 {
            for b in 0..=255u8 {
                assert_eq!(
                    Color::Rgb(r, g, b).adapt(ColorLevel::Ansi256),
                    Color::Indexed(closest([r.into(), g.into(), b.into()], &p)),
                    "RGB {r},{g},{b}"
                );
            }
        }
    }
}
