use matea_render::{Rect, RichText, Size, Surface, Text, View};
use matea_style::{Color, Style};

fn color(index: u8) -> Style {
    Style::default().fg(Color::Indexed(index))
}

#[test]
fn chunks_normalize_once_and_ranges_follow_normalized_utf8() {
    let red = color(1);
    let blue = color(4);
    let text = RichText::from_spans([
        ("", red),
        ("A\t", red),
        ("\u{85}B", blue),
        ("C", blue),
        ("", red),
    ]);
    assert_eq!(text.as_str(), "A    �BC");
    assert_eq!((text.len(), text.width(), text.line_count()), (10, 8, 1));
    let mut styles = text.styles();
    assert_eq!(styles.len(), 2);
    assert_eq!(styles.next_back(), Some((5..10, blue)));
    assert_eq!(styles.next(), Some((0..5, red)));
    assert!(styles.next().is_none() && styles.next_back().is_none());
    for byte in 0..5 {
        assert_eq!(text.style_at_byte(byte), Some(red));
    }
    for byte in 5..10 {
        assert_eq!(text.style_at_byte(byte), Some(blue));
    }
    assert_eq!(text.style_at_byte(10), None);
    assert_eq!(text.style_at_byte(usize::MAX), None);
    let empty = RichText::from_spans(std::iter::empty::<(&str, Style)>());
    assert!(empty.is_empty());
    assert_eq!((empty.width(), empty.line_count()), (0, 1));
    assert_eq!(empty.styles().len(), 0);
    assert!(empty.graphemes().next().is_none());
    let merged = RichText::from_spans([("A", blue), ("", red), ("B", blue)]);
    assert_eq!(merged.styles().collect::<Vec<_>>(), [(0..2, blue)]);
}

#[test]
fn untrusted_hints_and_irregular_chunks_preserve_canonical_content() {
    struct Hint<I>(I, usize);
    impl<I: Iterator> Iterator for Hint<I> {
        type Item = I::Item;
        fn next(&mut self) -> Option<Self::Item> {
            self.0.next()
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            (self.1, None)
        }
    }
    let chunks = [
        ("", color(1)),
        ("x", color(1)),
        ("茶", color(2)),
        ("", color(3)),
        ("e\u{301}", color(2)),
    ];
    let expected = RichText::from_spans(chunks);
    for hint in [0, 1, 5, 64, usize::MAX] {
        let actual = RichText::from_spans(Hint(chunks.into_iter(), hint));
        assert_eq!(actual.as_str(), expected.as_str());
        assert_eq!(
            actual.styles().collect::<Vec<_>>(),
            expected.styles().collect::<Vec<_>>()
        );
        assert_eq!(actual.width(), expected.width());
    }
    let huge = "x".repeat(64 * 1024 + 1);
    let text = RichText::from_spans(Hint([(huge.as_str(), color(1))].into_iter(), usize::MAX));
    assert_eq!(text.as_str(), huge);
}

#[test]
fn whole_graphemes_cross_style_boundaries_and_use_the_leading_style() {
    let red = color(1);
    let blue = color(4);
    let green = color(2);
    let yellow = color(3);
    let text = RichText::from_spans([
        ("e", red),
        ("\u{301}", blue),
        ("👩", green),
        ("\u{200d}", blue),
        ("💻", yellow),
        ("🇯", red),
        ("🇵", blue),
        ("1", green),
        ("\u{fe0f}\u{20e3}", blue),
        ("X", yellow),
    ]);
    let expected = [
        ("e\u{301}", red, 1),
        ("👩‍💻", green, 2),
        ("🇯🇵", red, 2),
        ("1️⃣", green, 2),
        ("X", yellow, 1),
    ];
    assert_eq!(text.width(), 8);
    assert_eq!(
        text.graphemes()
            .map(|g| (g.symbol, g.style, g.width))
            .collect::<Vec<_>>(),
        expected
    );
    let mut iterator = text.graphemes();
    assert_eq!(iterator.next_back().unwrap().symbol, "X");
    assert_eq!(iterator.next().unwrap().symbol, "e\u{301}");
    drop(iterator);
    let frame = View::rich_text(text).render(Size::new(8, 1)).unwrap();
    assert_eq!(frame.plain(), "e\u{301}👩‍💻🇯🇵1️⃣X");
    for (x, symbol, style, width) in [
        (0, "e\u{301}", red, 1),
        (1, "👩‍💻", green, 2),
        (3, "🇯🇵", red, 2),
        (5, "1️⃣", green, 2),
        (7, "X", yellow, 1),
    ] {
        let cell = &frame.row(0)[x];
        assert_eq!(
            (cell.symbol(), cell.style(), cell.width()),
            (symbol, style, width)
        );
    }
    for x in [2, 4, 6] {
        assert!(frame.row(0)[x].is_continuation());
    }
}

#[test]
fn every_scalar_split_of_complex_sequences_preserves_complete_cell_output() {
    for raw in ["e\u{301}X", "👩‍💻!", "🇯🇵!", "1️⃣X", "ن\u{651}茶", "A\nB\n"] {
        let expected = View::styled(raw, color(1))
            .render(Size::new(12, 3))
            .unwrap();
        let boundaries = raw
            .char_indices()
            .map(|(index, _)| index)
            .chain([raw.len()]);
        for index in boundaries {
            let text = RichText::from_spans([(&raw[..index], color(1)), (&raw[index..], color(1))]);
            assert_eq!(
                View::rich_text(text).render(Size::new(12, 3)).unwrap(),
                expected,
                "split={index} text={raw:?}"
            );
        }
    }
}

#[test]
fn single_style_matches_existing_text_for_controls_multiline_and_clipping() {
    let style = color(2).bold();
    for raw in [
        "",
        "A",
        "茶 e\u{301} 👩‍💻",
        "\t\x1b[31m\u{85}\nB\n",
        "\u{301}A\u{200b}B",
        "ن\u{651}\n茶",
    ] {
        let rich = RichText::styled(raw, style);
        for width in 0..8 {
            for height in 0..4 {
                let size = Size::new(width, height);
                assert_eq!(
                    View::rich_text(rich.clone()).render(size).unwrap(),
                    View::styled(raw, style).render(size).unwrap(),
                    "{raw:?} {size:?}"
                );
            }
        }
    }
}

#[test]
fn clipped_rows_skip_offscreen_styles_without_losing_byte_positions() {
    let red = color(1);
    let blue = color(4);
    let green = color(2);
    let yellow = color(3);
    let rich = RichText::from_spans([
        ("a", red),
        ("b茶", blue),
        ("X\n", green),
        ("c", yellow),
        ("d界", red),
        ("Y", blue),
    ]);
    let mut both = Surface::new(Size::new(6, 2)).unwrap();
    both.canvas(Rect::new(2, 0, 3, 2)).rich_text(0, 0, &rich);
    assert_eq!(both.plain(), "  茶X\n  界Y");
    assert_eq!(both.row(0)[2].style(), blue);
    assert_eq!(both.row(0)[4].style(), green);
    assert_eq!(both.row(1)[2].style(), red);
    assert_eq!(both.row(1)[4].style(), blue);
    let mut bottom = Surface::new(Size::new(6, 2)).unwrap();
    bottom.canvas(Rect::new(2, 1, 3, 1)).rich_text(0, 0, &rich);
    assert_eq!(bottom.plain(), "\n  界Y");
    assert_eq!(bottom.row(1)[2].style(), red);
    assert_eq!(bottom.row(1)[4].style(), blue);
    let mut narrow = Surface::new(Size::new(1, 1)).unwrap();
    narrow.canvas(Rect::new(0, 0, 1, 1)).rich_text(
        0,
        0,
        &RichText::from_spans([("茶", red), ("X", blue)]),
    );
    assert_eq!(narrow.plain(), "");
}

#[test]
fn rich_layers_clear_complete_old_wide_glyphs_and_keep_layout_height() {
    let rich = RichText::from_spans([("X", color(1)), ("Y\nZ", color(2))]);
    let scene = View::layers(vec![
        (Rect::new(0, 0, 4, 2), View::text("茶茶\n茶茶")),
        (Rect::new(1, 0, 2, 2), View::rich_text(rich)),
    ]);
    let frame = scene.render(Size::new(4, 2)).unwrap();
    assert_eq!(frame.plain(), " XY\n Z茶");
    let rows = View::column([
        View::rich_text(RichText::styled("A\nB\n", color(1))),
        View::text("tail"),
    ]);
    assert_eq!(
        rows.render(Size::new(8, 4)).unwrap().plain(),
        "A\nB\n\ntail"
    );
}

#[test]
fn ownership_sharing_threads_and_old_view_snapshots_are_explicit() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<RichText>();
    let owned = Text::from("owned text allocation / ".repeat(20));
    let pointer = owned.as_ptr();
    let rich = RichText::styled(owned, color(1));
    assert_eq!(rich.as_str().as_ptr(), pointer);
    let mut input = "borrowed tea".to_owned();
    let mut current = RichText::from_spans([(input.as_str(), color(2))]);
    let clone = current.clone();
    assert_eq!(current.as_str().as_ptr(), clone.as_str().as_ptr());
    let old = View::rich_text(clone.clone());
    input.clear();
    input.push_str("external mutation");
    current = RichText::styled("new", color(4));
    assert_eq!(current.as_str(), "new");
    assert_eq!(
        old.render(Size::new(20, 1)).unwrap().plain(),
        "borrowed tea"
    );
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let text = clone.clone();
            scope.spawn(move || {
                assert_eq!(text.as_str(), "borrowed tea");
                assert_eq!(text.width(), 12);
            });
        }
    });
    assert_eq!(std::mem::size_of::<View>(), 48);
    assert_eq!(
        std::mem::size_of::<RichText>(),
        std::mem::size_of::<usize>()
    );
}

#[test]
fn caller_iterator_failure_keeps_the_preexisting_document() {
    let mut current = RichText::styled("old", color(1));
    let mut calls = 0;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        current = RichText::from_spans(std::iter::from_fn(|| {
            calls += 1;
            if calls == 1 {
                Some(("replacement", color(2)))
            } else {
                panic!("caller iterator failed")
            }
        }));
    }));
    assert!(result.is_err());
    assert_eq!(current.as_str(), "old");
    assert_eq!(current.styles().collect::<Vec<_>>(), [(0..3, color(1))]);
}

#[test]
fn pinned_public_span_cells_have_four_matches_and_four_literal_cluster_deviations() {
    let cases: &[(&str, &[&str])] = &[
        ("combining_split", &["e", "\u{301}", "x"]),
        ("combining_joined", &["e\u{301}", "x"]),
        ("zwj_split", &["👩", "\u{200d}", "💻", "!"]),
        ("zwj_joined", &["👩‍💻", "!"]),
        ("flag_split", &["🇯", "🇵", "!"]),
        ("flag_joined", &["🇯🇵", "!"]),
        ("keycap_split", &["1", "\u{fe0f}\u{20e3}", "X"]),
        ("keycap_joined", &["1️⃣", "X"]),
    ];
    let colors = [1, 4, 2, 3];
    let mut fixture = include_str!("fixtures/ratatui-span-boundaries.tsv").lines();
    let mut exact = 0;
    let mut different = 0;
    for (name, parts) in cases {
        let rich = RichText::from_spans(parts.iter().enumerate().map(|(index, text)| {
            let foreground = if name.ends_with("joined") && index == 1 {
                if *name == "zwj_joined" { 3 } else { 2 }
            } else {
                colors[index]
            };
            (*text, color(foreground))
        }));
        let width = rich.width();
        let frame = View::rich_text(rich).render(Size::new(10, 1)).unwrap();
        let mut source_atoms = Vec::new();
        let mut source_width = 0;
        for x in 0..10 {
            let fields: Vec<_> = fixture.next().unwrap().split('\t').collect();
            assert_eq!(fields.len(), 5);
            assert_eq!(fields[0], *name);
            assert_eq!(fields[2], x.to_string());
            let next_width = fields[1].parse::<usize>().unwrap();
            if x == 0 {
                source_width = next_width;
            } else {
                assert_eq!(source_width, next_width);
            }
            let symbol = String::from_utf8(
                (0..fields[3].len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&fields[3][i..i + 2], 16).unwrap())
                    .collect(),
            )
            .unwrap();
            if symbol != " " {
                let foreground = match fields[4] {
                    "Red" => 1,
                    "Blue" => 4,
                    "Green" => 2,
                    "Yellow" => 3,
                    value => panic!("unexpected source color {value}"),
                };
                source_atoms.push((x, symbol, foreground));
            } else {
                assert_eq!(fields[4], "Reset");
            }
        }
        // Compare leading visible atoms; the raw source stores blank space
        // continuation cells, while Surface reserves empty continuation cells.
        let actual = frame
            .row(0)
            .iter()
            .enumerate()
            .filter(|(_, cell)| !cell.is_continuation() && cell.symbol() != " ")
            .map(|(x, cell)| {
                let Color::Indexed(foreground) = cell.style().fg else {
                    panic!("expected indexed style")
                };
                (x, cell.symbol().to_owned(), foreground)
            })
            .collect::<Vec<_>>();
        let atoms = |values: &[(usize, &str, u8)]| {
            values
                .iter()
                .map(|(x, s, c)| (*x, (*s).to_owned(), *c))
                .collect::<Vec<_>>()
        };
        let expected = match *name {
            "combining_split" => Some((
                2,
                atoms(&[(0, "e", 1), (1, "x", 2)]),
                2,
                atoms(&[(0, "e\u{301}", 1), (1, "x", 2)]),
            )),
            "zwj_split" => Some((
                5,
                atoms(&[(0, "👩", 1), (2, "💻", 2), (4, "!", 3)]),
                3,
                atoms(&[(0, "👩‍💻", 1), (2, "!", 3)]),
            )),
            "flag_split" => Some((
                3,
                atoms(&[(0, "🇯", 1), (1, "🇵", 4), (2, "!", 2)]),
                3,
                atoms(&[(0, "🇯🇵", 1), (2, "!", 2)]),
            )),
            "keycap_split" => Some((
                2,
                atoms(&[(0, "1", 1), (1, "X", 2)]),
                3,
                atoms(&[(0, "1️⃣", 1), (2, "X", 2)]),
            )),
            _ => None,
        };
        if let Some((old_width, old_atoms, new_width, new_atoms)) = expected {
            assert_eq!((source_width, source_atoms), (old_width, old_atoms));
            assert_eq!((width, actual), (new_width, new_atoms));
            different += 1;
        } else {
            assert_eq!((source_width, source_atoms), (width, actual));
            exact += 1;
        }
    }
    assert_eq!((exact, different), (4, 4));
    assert!(fixture.next().is_none());
}
