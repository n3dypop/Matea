use matea_style::{Color, Gradient, GradientError};
use std::{collections::VecDeque, iter::FusedIterator};

fn rgb(text: &str) -> [u8; 3] {
    assert_eq!(text.len(), 6);
    std::array::from_fn(|channel| {
        u8::from_str_radix(&text[channel * 2..channel * 2 + 2], 16).unwrap()
    })
}
fn stops(text: &str) -> Vec<[u8; 3]> {
    text.split(',').map(rgb).collect()
}
fn expected(text: &str) -> Vec<Color> {
    if text.is_empty() {
        return Vec::new();
    }
    stops(text)
        .into_iter()
        .map(|[r, g, b]| Color::Rgb(r, g, b))
        .collect()
}

#[test]
fn premultiplied_rgba16_matches_canonical_pinned_source_color_projections() {
    let mut checked = 0;
    for line in include_str!("fixtures/rgba16-gradients.tsv").lines() {
        let f = line.split('\t').collect::<Vec<_>>();
        let channels = f[3]
            .split(',')
            .map(|v| v.parse::<u16>().unwrap())
            .collect::<Vec<_>>();
        let mut inputs = vec![channels.try_into().unwrap()];
        if f[2] == "0" {
            inputs.push([65535; 4]);
        }
        let gradient = Gradient::from_premultiplied_rgba16(&inputs).unwrap();
        assert_eq!(
            gradient.colors(f[1].parse().unwrap()).collect::<Vec<_>>(),
            expected(f[4]),
            "{line}"
        );
        checked += 1;
    }
    assert_eq!(checked, 144);
}

#[test]
fn precise_stop_ownership_validation_prefix_and_opaque_interpolation_are_explicit() {
    assert_eq!(
        Gradient::from_premultiplied_rgba16(&[]).unwrap_err(),
        GradientError::EmptyStops
    );
    let mut inputs = [[32768, 8192, 16384, 32768], [65535; 4]];
    let gradient = Gradient::from_premultiplied_rgba16(&inputs).unwrap();
    inputs[0] = [0; 4];
    assert_eq!(inputs[0], [0; 4]);
    assert_eq!(gradient.color_at(0, 1), Some(Color::Rgb(128, 32, 64)));
    assert_eq!(gradient.color_at(0, 3), Some(Color::Rgb(255, 63, 127)));
    assert_eq!(
        gradient.clone().colors(17).collect::<Vec<_>>(),
        gradient.colors(17).collect::<Vec<_>>()
    );
    for channel in 0..3 {
        let mut invalid = [[0; 4], [1, 1, 1, 1]];
        invalid[1][channel] = 2;
        assert_eq!(
            Gradient::from_premultiplied_rgba16(&invalid).unwrap_err(),
            GradientError::InvalidPremultipliedStop { index: 1 }
        );
    }
    assert!(
        GradientError::InvalidPremultipliedStop { index: 1 }
            .to_string()
            .contains("stop 1")
    );
    let dark =
        Gradient::from_premultiplied_rgba16(&[[0, 0, 0, 65535], [257, 257, 257, 65535]]).unwrap();
    assert_eq!(
        dark.colors(3).collect::<Vec<_>>(),
        [
            Color::Rgb(0, 0, 0),
            Color::Rgb(0, 0, 0),
            Color::Rgb(1, 1, 1)
        ]
    );
}

#[test]
fn pinned_lip_gloss_rgb8_samples_match_complete_fixture_outputs() {
    let mut checked = 0;
    for (number, line) in include_str!("fixtures/gradients.tsv").lines().enumerate() {
        // Native execution covers all 548 cases; Miri uses a finite subset.
        if cfg!(miri) && number % 29 != 0 {
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        let actual = match fields[0] {
            "L" => {
                let count = fields[1].parse().unwrap();
                let gradient = Gradient::new(&stops(fields[2])).unwrap();
                gradient.colors(count).collect::<Vec<_>>()
            }
            "G" => {
                let width = fields[1].parse().unwrap();
                let height = fields[2].parse().unwrap();
                let angle = fields[3].parse().unwrap();
                let gradient = Gradient::new(&stops(fields[4])).unwrap();
                gradient
                    .grid(width, height, angle)
                    .unwrap()
                    .colors()
                    .collect::<Vec<_>>()
            }
            _ => panic!("unknown fixture kind"),
        };
        assert_eq!(
            actual,
            expected(fields[fields.len() - 1]),
            "fixture {number}: {line}"
        );
        checked += 1;
    }
    assert_eq!(checked, if cfg!(miri) { 19 } else { 548 });
}

#[test]
fn configuration_owns_stops_and_rejects_invalid_angle_or_empty_input() {
    assert_eq!(Gradient::new(&[]).unwrap_err(), GradientError::EmptyStops);
    let gradient = {
        let mut source = [[255, 0, 0], [0, 0, 255]];
        let owned = Gradient::new(&source).unwrap();
        source[0] = [0, 255, 0];
        assert_eq!(source[0], [0, 255, 0]);
        owned
    };
    assert_eq!(gradient.color_at(0, 9), Some(Color::Rgb(255, 0, 0)));
    assert_eq!(gradient.color_at(8, 9), Some(Color::Rgb(0, 0, 255)));
    assert_eq!(gradient.color_at(9, 9), None);
    assert_eq!(gradient.color_at(0, 0), None);
    for angle in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            gradient.grid(1, 1, angle).unwrap_err(),
            GradientError::NonFiniteAngle
        );
    }
    let grid = gradient.grid(0, 0, -450.0).unwrap();
    assert_eq!((grid.width(), grid.height(), grid.cell_count()), (1, 1, 1));
    assert_eq!(grid.color_at(0, 0), Some(Color::Rgb(255, 0, 0)));
    assert_eq!(grid.color_at(1, 0), None);
    assert_eq!(grid.color_at(0, 1), None);
    assert_eq!(
        gradient.clone().colors(17).collect::<Vec<_>>(),
        gradient.colors(17).collect::<Vec<_>>()
    );
}

#[test]
fn all_indexed_gradient_stops_resolve_without_hex_parsing_or_black_fallback() {
    let mut checked = 0;
    for line in include_str!("fixtures/indexed-gradients.tsv").lines() {
        let (index, colors) = line.split_once('\t').unwrap();
        let index: u8 = index.parse().unwrap();
        if cfg!(miri) && !index.is_multiple_of(17) {
            continue;
        }
        let source = [Color::Indexed(index), Color::Indexed(13)];
        let gradient = Gradient::from_xterm_colors(&source).unwrap();
        assert_eq!(gradient.colors(8).collect::<Vec<_>>(), expected(colors));
        checked += 1;
    }
    assert_eq!(checked, if cfg!(miri) { 16 } else { 256 });
    assert_eq!(
        Gradient::from_xterm_colors(&[]).unwrap_err(),
        GradientError::EmptyStops
    );
    for source in [
        [Color::Default, Color::Indexed(5)],
        [Color::Rgb(1, 2, 3), Color::Default],
    ] {
        assert_eq!(
            Gradient::from_xterm_colors(&source).unwrap_err(),
            GradientError::UnresolvedDefaultColor
        );
    }
    let mixed = Gradient::from_xterm_colors(&[Color::Indexed(5), Color::Rgb(0, 255, 0)]).unwrap();
    assert_eq!(
        mixed.colors(2).collect::<Vec<_>>(),
        [Color::Rgb(128, 0, 128), Color::Rgb(0, 255, 0)]
    );
}

fn mixed_ends(
    mut actual: impl ExactSizeIterator<Item = Color> + DoubleEndedIterator + FusedIterator,
    values: Vec<Color>,
) {
    let mut values = VecDeque::from(values);
    while !values.is_empty() {
        assert_eq!(actual.len(), values.len());
        assert_eq!(actual.size_hint(), (values.len(), Some(values.len())));
        if values.len() % 2 == 0 {
            assert_eq!(actual.next(), values.pop_front());
        } else {
            assert_eq!(actual.next_back(), values.pop_back());
        }
    }
    for _ in 0..3 {
        assert_eq!(actual.next(), None);
        assert_eq!(actual.next_back(), None);
    }
    assert_eq!(actual.len(), 0);
}

#[test]
fn indexed_and_mixed_ended_iterators_preserve_exact_remaining_ranges() {
    let gradient =
        Gradient::new(&[[90, 86, 224], [238, 111, 248], [7, 201, 33], [0, 0, 0]]).unwrap();
    for count in 0..=70 {
        let values = gradient.colors(count).collect::<Vec<_>>();
        mixed_ends(gradient.colors(count), values.clone());
        assert_eq!(
            gradient.colors(count).rev().collect::<Vec<_>>(),
            values.iter().copied().rev().collect::<Vec<_>>()
        );
        assert_eq!(gradient.colors(count).nth(2), values.get(2).copied());
        assert_eq!(
            gradient.colors(count).nth_back(2),
            values.len().checked_sub(3).map(|index| values[index])
        );
    }
    for (width, height) in [(1, 1), (2, 3), (7, 4)] {
        let grid = gradient.grid(width, height, 45.0).unwrap();
        let values = grid.colors().collect::<Vec<_>>();
        mixed_ends(grid.colors(), values.clone());
        assert_eq!(grid.colors().nth(2), values.get(2).copied());
        for y in 0..height {
            for x in 0..width {
                assert_eq!(
                    grid.color_at(x, y),
                    Some(values[usize::from(y * width + x)])
                );
            }
        }
    }
    // Large ranges remain lazy; indexed skipping cannot overflow or walk them.
    let mut huge = gradient.colors(usize::MAX);
    assert_eq!(huge.len(), usize::MAX);
    assert_eq!(
        huge.nth(usize::MAX - 2),
        gradient.color_at(usize::MAX - 2, usize::MAX)
    );
    assert_eq!(huge.len(), 1);
    assert!(huge.next_back().is_some());
    assert_eq!(huge.next(), None);
    if !cfg!(miri) {
        let grid = gradient.grid(u16::MAX, u16::MAX, 30.0).unwrap();
        assert_eq!(
            grid.cell_count(),
            usize::from(u16::MAX) * usize::from(u16::MAX)
        );
        assert_eq!(
            grid.colors().next_back(),
            grid.color_at(u16::MAX - 1, u16::MAX - 1)
        );
    }
}
