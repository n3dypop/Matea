use matea_style::{
    AdaptiveColor, Color, ColorError, ColorLevel, PremultipliedRgba16, ProfileColors, Rgba16,
};

#[test]
#[cfg_attr(
    miri,
    ignore = "29,898 pinned observations run natively; Miri covers typed boundary tests"
)]
fn pinned_public_color_operations_and_standard_premultiplication() {
    let mut counts = [0; 6];
    for (line, record) in include_str!("fixtures/color-inputs.tsv")
        .lines()
        .enumerate()
    {
        let fields: Vec<_> = record.split('\t').collect();
        assert_eq!(fields.len(), 8, "line {}", line + 1);
        let [op, argument, parameter, kind] = [fields[0], fields[1], fields[2], fields[3]];
        let expected: Vec<u16> = fields[4..]
            .iter()
            .map(|field| field.parse().unwrap())
            .collect();
        let expected_rgb = Color::Rgb(
            (expected[0] >> 8) as u8,
            (expected[1] >> 8) as u8,
            (expected[2] >> 8) as u8,
        );
        match op {
            "parse" => {
                counts[0] += 1;
                let result = argument.parse::<Color>();
                if kind == "default" {
                    assert!(result.is_err(), "accepted invalid input at {}", line + 1);
                    assert_eq!(result.unwrap_or_default(), Color::Default);
                } else if let Some(index) = kind.strip_prefix("index:") {
                    assert_eq!(result, Ok(Color::Indexed(index.parse().unwrap())));
                    assert_eq!(result.unwrap().rgb_channels(), expected_rgb.rgb_channels());
                } else {
                    assert_eq!(result, Ok(expected_rgb), "line {}", line + 1);
                }
            }
            "complementary" | "darken" | "lighten" => {
                let color: Color = argument.parse().unwrap();
                let result = match op {
                    "complementary" => {
                        counts[1] += 1;
                        color.complementary()
                    }
                    "darken" => {
                        counts[2] += 1;
                        color.darken(parameter.parse().unwrap())
                    }
                    _ => {
                        counts[3] += 1;
                        color.lighten(parameter.parse().unwrap())
                    }
                };
                assert_eq!(result, Ok(expected_rgb), "line {}", line + 1);
            }
            "premultiply" => {
                counts[4] += 1;
                let red: u16 = argument.parse().unwrap();
                let alpha: u16 = parameter.parse().unwrap();
                let color = Rgba16::new(red, 65535 - red, 32768, alpha).premultiplied();
                assert_eq!(color.channels().as_slice(), expected, "line {}", line + 1);
                assert_eq!(PremultipliedRgba16::try_from(color.channels()), Ok(color));
            }
            "source-alpha" => {
                counts[5] += 1;
                let red: u8 = argument.parse().unwrap();
                let color = Color::Rgb(red, 0, 0).rgba16().unwrap();
                let half = color.with_alpha(32767);
                // The pinned source retains the opaque RGB value inside a
                // premultiplied color.RGBA while changing only alpha to 127.
                assert_eq!(expected, [u16::from(red) * 257, 0, 0, 32639]);
                assert_eq!(half.red, u16::from(red) * 257);
                assert!(half.premultiplied().channels()[0] <= half.alpha);
                assert_eq!(half.rgb(), Color::Rgb(red, 0, 0));
            }
            other => panic!("unknown fixture {other}"),
        }
    }
    assert_eq!(counts, [5146, 1285, 11565, 11565, 81, 256]);
}

#[test]
fn invalid_formats_numeric_boundaries_and_unknown_colors_are_explicit() {
    for input in [
        "",
        "#ggg",
        "#abcd",
        "red",
        "\u{1b}[31m",
        "1\n",
        "１２",
        "--1",
    ] {
        assert_eq!(input.parse::<Color>(), Err(ColorError::InvalidFormat));
    }
    for input in [
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
    ] {
        assert_eq!(input.parse::<Color>(), Err(ColorError::OutOfRange));
    }
    for amount in [f64::NAN, f64::NEG_INFINITY, f64::INFINITY] {
        for color in [Color::Default, Color::Rgb(0, 0, 0)] {
            assert_eq!(color.darken(amount), Err(ColorError::NonFiniteAmount));
            assert_eq!(color.lighten(amount), Err(ColorError::NonFiniteAmount));
        }
    }
    assert_eq!(Color::Default.rgb_channels(), None);
    assert_eq!(Color::Default.rgba16(), None);
    assert_eq!(
        Color::Default.complementary(),
        Err(ColorError::UnresolvedDefault)
    );
    assert_eq!(
        Color::Default.darken(0.5),
        Err(ColorError::UnresolvedDefault)
    );
    assert_eq!(
        Color::Default.lighten(0.5),
        Err(ColorError::UnresolvedDefault)
    );
    for error in [
        ColorError::InvalidFormat,
        ColorError::OutOfRange,
        ColorError::UnresolvedDefault,
        ColorError::NonFiniteAmount,
        ColorError::InvalidPremultipliedChannels,
    ] {
        assert!(!error.to_string().is_empty());
        assert!(std::error::Error::source(&error).is_none());
    }
}

#[test]
fn every_channel_and_boundary_alpha_preserves_premultiplied_invariants() {
    let channels: Vec<u16> = if cfg!(miri) {
        vec![0, 1, 255, 256, 257, 32767, 32768, 65534, 65535]
    } else {
        (0..=u16::MAX).collect()
    };
    for channel in channels {
        for alpha in [0, 1, 255, 256, 257, 32767, 32768, 65534, 65535] {
            let raw = Rgba16::new(channel, 65535 - channel, channel, alpha);
            let premultiplied = raw.premultiplied();
            let [r, g, b, a] = premultiplied.channels();
            assert_eq!(a, alpha);
            assert_eq!(u64::from(r), u64::from(channel) * u64::from(alpha) / 65535);
            assert_eq!(
                u64::from(g),
                u64::from(65535 - channel) * u64::from(alpha) / 65535
            );
            assert_eq!(r, b);
            assert!(r <= a && g <= a);
            assert_eq!(
                PremultipliedRgba16::try_from([r, g, b, a]),
                Ok(premultiplied)
            );
            let straight = premultiplied.straight();
            if alpha == 0 {
                assert_eq!(straight, Rgba16::default());
            } else {
                assert_eq!(u64::from(straight.red), u64::from(r) * 65535 / u64::from(a));
                assert_eq!(
                    u64::from(straight.green),
                    u64::from(g) * 65535 / u64::from(a)
                );
            }
        }
    }
    for invalid in [[1, 0, 0, 0], [0, 256, 0, 255], [0, 0, 65535, 65534]] {
        assert_eq!(
            PremultipliedRgba16::try_from(invalid),
            Err(ColorError::InvalidPremultipliedChannels)
        );
    }
    assert_eq!(PremultipliedRgba16::default().straight(), Rgba16::default());
}

#[test]
fn alpha_composition_uses_known_colors_and_independent_rational_model() {
    // All 65536 scalar alpha values: an independent rational model on an
    // opaque background has no dependency on production blend helpers.
    let red = Rgba16::new(65535, 10000, 0, 0);
    let background = Rgba16::new(0, 20000, 65535, 65535);
    let alphas: Vec<u16> = if cfg!(miri) {
        vec![0, 1, 255, 256, 257, 32767, 32768, 65534, 65535]
    } else {
        (0..=u16::MAX).collect()
    };
    for alpha in alphas {
        let result = red.with_alpha(alpha).over(background);
        let expected_green =
            (10000u64 * u64::from(alpha) + 20000u64 * u64::from(65535 - alpha) + 32767) / 65535;
        assert_eq!(
            result.channels(),
            [alpha, expected_green as u16, 65535 - alpha, 65535]
        );
    }
    for front in [0, 1, 255, 32767, 32768, 65534, 65535] {
        for back in [0, 1, 255, 32767, 32768, 65534, 65535] {
            let source = Rgba16::new(12000, 44000, 65535, front);
            let dest = Rgba16::new(63000, 22000, 1000, back);
            let result = source.over(dest);
            if front == 0 && back == 0 {
                assert_eq!(result, source); // retained transparent RGB identity
                continue;
            }
            let a = f64::from(front) / 65535.0;
            let b = f64::from(back) / 65535.0;
            let opacity = a + b * (1.0 - a);
            for (index, (&x, &y)) in source.channels()[..3]
                .iter()
                .zip(&dest.channels()[..3])
                .enumerate()
            {
                let expected =
                    ((f64::from(x) * a + f64::from(y) * b * (1.0 - a)) / opacity).round();
                assert!((f64::from(result.channels()[index]) - expected).abs() <= 1.0);
            }
            assert_eq!(result.alpha, (opacity * 65535.0).round() as u16);
        }
    }
}

#[test]
fn profile_selection_and_rgb_depth_do_not_guess_or_quantize() {
    let choices = ProfileColors {
        ansi16: Color::Indexed(1),
        ansi256: Color::Indexed(196),
        truecolor: Color::Rgb(255, 20, 30),
    };
    assert_eq!(choices.resolve(ColorLevel::None), Color::Default);
    assert_eq!(choices.resolve(ColorLevel::Ansi16), choices.ansi16);
    assert_eq!(choices.resolve(ColorLevel::Ansi256), choices.ansi256);
    assert_eq!(choices.resolve(ColorLevel::TrueColor), choices.truecolor);
    let adaptive = AdaptiveColor {
        light: Color::Indexed(0),
        dark: Color::Indexed(15),
    };
    assert_eq!(adaptive.resolve(false), adaptive.light);
    assert_eq!(adaptive.resolve(true), adaptive.dark);
    for n in 0..=255u8 {
        let color = Color::Rgb(n, n, n);
        assert_eq!(
            color.rgba16().unwrap().channels(),
            [
                u16::from(n) * 257,
                u16::from(n) * 257,
                u16::from(n) * 257,
                65535
            ]
        );
        assert_eq!(color.rgba16().unwrap().rgb(), color);
        assert_eq!(color.complementary(), Ok(color));
        let indexed = Color::Indexed(n);
        assert_eq!(
            indexed.rgba16().unwrap().rgb().rgb_channels(),
            indexed.rgb_channels()
        );
    }
}

#[test]
fn pinned_color_types_and_selection_distinguish_unknown_and_explicit_depth() {
    let mut counts = [0; 5];
    for row in include_str!("fixtures/color-contracts.tsv").lines() {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 6);
        let n: i32 = fields[1].parse().unwrap();
        let expected: Vec<u16> = fields[2..].iter().map(|s| s.parse().unwrap()).collect();
        let rgb = Color::Rgb(
            (expected[0] >> 8) as u8,
            (expected[1] >> 8) as u8,
            (expected[2] >> 8) as u8,
        );
        match fields[0] {
            "rgbcolor" => {
                counts[0] += 1;
                assert_eq!(
                    expected,
                    [
                        (n as u16) << 8,
                        ((255 - n) as u16) << 8,
                        (n as u16) << 8,
                        65535
                    ]
                );
                // Preserve the raw shift-only source observations. Canonical
                // Rust expands RGB8 across the complete u16 range instead.
                let color = Color::Rgb(n as u8, (255 - n) as u8, n as u8);
                let canonical = color.rgba16().unwrap();
                assert_eq!(canonical.rgb(), rgb);
                assert_eq!(
                    canonical.channels(),
                    [
                        n as u16 * 257,
                        (255 - n) as u16 * 257,
                        n as u16 * 257,
                        65535
                    ]
                );
            }
            "none" => {
                counts[1] += 1;
                assert_eq!(expected, [0, 0, 0, 65535]);
                assert_eq!(Color::Default.rgba16(), None);
            }
            "adaptive" => {
                counts[2] += 1;
                assert_eq!(
                    AdaptiveColor {
                        light: Color::Rgb(18, 171, 52),
                        dark: Color::Rgb(205, 86, 239)
                    }
                    .resolve(n != 0),
                    rgb
                );
            }
            "profile" => {
                counts[3] += 1;
                let profile = match n {
                    3 => ColorLevel::Ansi16,
                    4 => ColorLevel::Ansi256,
                    5 => ColorLevel::TrueColor,
                    _ => ColorLevel::None,
                };
                let color = ProfileColors {
                    ansi16: Color::Rgb(18, 171, 52),
                    ansi256: Color::Rgb(205, 86, 239),
                    truecolor: Color::Rgb(161, 178, 195),
                }
                .resolve(profile);
                assert_eq!(
                    color,
                    if profile == ColorLevel::None {
                        Color::Default
                    } else {
                        rgb
                    }
                );
            }
            "constant" => {
                counts[4] += 1;
                assert_eq!(Color::Indexed(n as u8).rgb_channels(), rgb.rgb_channels());
            }
            other => panic!("unknown fixture {other}"),
        }
    }
    assert_eq!(counts, [256, 1, 2, 7, 16]);
}
