use matea_core::Component;
use matea_render::{Size, Surface};
use matea_style::{Color, StylePatch, TextAttribute, Theme, UnderlineStyle};
use matea_widgets::{HighlightError, Viewport};

fn frame(view: &Viewport, width: u16, height: u16) -> Surface {
    view.view(&Theme::plain(), true)
        .render(Size::new(width, height))
        .unwrap()
}
fn state(view: &Viewport) -> (Vec<std::ops::Range<usize>>, Option<usize>, usize, usize) {
    (
        view.highlights().collect(),
        view.selected_highlight(),
        view.offset(),
        view.horizontal_offset(),
    )
}
#[test]
fn ordinary_selected_and_explicit_false_intents_reach_cells() {
    let mut view = Viewport::new("one tea two tea");
    view.set_size(15, 1);
    view.highlight_style = StylePatch::new()
        .fg(Color::Indexed(2))
        .attribute(TextAttribute::Bold, true);
    view.selected_highlight_style = StylePatch::new()
        .bg(Color::Indexed(4))
        .attribute(TextAttribute::Bold, false);
    view.set_highlights([4..7, 12..15]).unwrap();
    let surface = frame(&view, 15, 1);
    for (index, cell) in surface.row(0).iter().enumerate() {
        assert_eq!(
            cell.style().fg,
            if (4..7).contains(&index) || (12..15).contains(&index) {
                Color::Indexed(2)
            } else {
                Color::Default
            }
        );
        assert_eq!(
            cell.style().bg,
            if (4..7).contains(&index) {
                Color::Indexed(4)
            } else {
                Color::Default
            }
        );
        assert_eq!(
            cell.style().has(TextAttribute::Bold),
            (12..15).contains(&index)
        );
    }
    assert_eq!(view.highlight_next(), Some(1));
    assert_eq!(frame(&view, 15, 1).row(0)[12].style().bg, Color::Indexed(4));
    assert_eq!(view.highlight_next(), Some(0));
    assert_eq!(view.highlight_previous(), Some(1));
}
#[test]
fn invalid_ranges_and_iterator_panics_are_transactional() {
    let mut view = Viewport::new("茶 one\nlast");
    view.set_size(3, 1);
    view.set_highlights([4..7, 8..12]).unwrap();
    view.select_highlight(1);
    let before = state(&view);
    for (ranges, error) in [
        (
            vec![4..7, std::ops::Range { start: 9, end: 8 }],
            HighlightError::Reversed { index: 1 },
        ),
        (
            vec![4..7, 8..usize::MAX],
            HighlightError::OutOfBounds { index: 1 },
        ),
        (
            std::iter::once(1..3).collect(),
            HighlightError::NotCharBoundary { index: 0 },
        ),
        (
            std::iter::once(0..1).collect(),
            HighlightError::NotCharBoundary { index: 0 },
        ),
        (
            vec![4..7, 6..9],
            HighlightError::UnsortedOrOverlapping { index: 1 },
        ),
        (
            vec![8..12, 4..7],
            HighlightError::UnsortedOrOverlapping { index: 1 },
        ),
    ] {
        assert_eq!(view.set_highlights(ranges), Err(error));
        assert_eq!(state(&view), before);
    }
    for fail_at in 0..=2 {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            view.set_highlights((0..3).map(|i| {
                assert_ne!(i, fail_at, "caller panic");
                if i == 0 { 0..3 } else { 4..7 }
            }))
        }));
        assert!(result.is_err());
        assert_eq!(state(&view), before);
    }
    assert!(!view.select_highlight(usize::MAX));
    assert_eq!(state(&view), before);
    let mut iterator = view.highlights();
    assert_eq!(iterator.len(), 2);
    assert_eq!(iterator.next_back(), Some(8..12));
    assert_eq!(iterator.next(), Some(4..7));
    assert_eq!(iterator.next(), None);
    assert_eq!(iterator.next_back(), None);
}
#[test]
fn combining_and_emoji_matches_cover_complete_intersecting_graphemes() {
    let text = "e\u{301}👩‍💻🇯🇵";
    let mut view = Viewport::new(text);
    view.set_size(8, 1);
    view.highlight_style = StylePatch::new().fg(Color::Indexed(2));
    view.selected_highlight_style = StylePatch::new().bg(Color::Indexed(4));
    // Two disjoint scalar matches share the same EGC. Selection has precedence.
    view.set_highlights([0..1, 1..3, 7..10, 18..22]).unwrap();
    assert!(view.select_highlight(1));
    let surface = frame(&view, 8, 1);
    assert_eq!(surface.row(0)[0].symbol(), "e\u{301}");
    assert_eq!(surface.row(0)[0].style().bg, Color::Indexed(4));
    assert_eq!(surface.row(0)[1].symbol(), "👩‍💻");
    assert_eq!(surface.row(0)[1].style().fg, Color::Indexed(2));
    assert!(surface.row(0)[2].is_continuation());
    assert_eq!(surface.row(0)[3].symbol(), "🇯🇵");
    assert_eq!(surface.row(0)[3].style().fg, Color::Indexed(2));
}
#[test]
fn multiline_empty_matches_clear_and_content_replacement_have_defined_semantics() {
    let mut view = Viewport::new("ab\ncd\nef");
    view.set_size(2, 3);
    view.selected_highlight_style = StylePatch::new();
    view.set_highlights([1..4, 4..4, 6..8]).unwrap();
    let surface = frame(&view, 2, 3);
    for (y, expected) in [[false, true], [true, false], [true, true]]
        .into_iter()
        .enumerate()
    {
        for (x, highlighted) in expected.into_iter().enumerate() {
            assert_eq!(
                surface.row(y as u16)[x].style().attributes.underline(),
                if highlighted {
                    UnderlineStyle::Single
                } else {
                    UnderlineStyle::None
                }
            );
        }
    }
    view.set_highlights([]).unwrap();
    assert_eq!(view.highlights().len(), 0);
    assert_eq!(view.selected_highlight(), None);
    assert_eq!(view.highlight_next(), None);
    assert_eq!(view.highlight_previous(), None);
    assert!(!view.clear_highlights());
    view.set_highlights([0..0, 0..0, 8..8]).unwrap();
    assert_eq!(view.highlight_previous(), Some(2));
    assert!(
        frame(&view, 2, 3)
            .row(0)
            .iter()
            .all(|cell| cell.style().attributes.underline() == UnderlineStyle::None)
    );
    view.set_content_owned("new".to_string());
    assert_eq!(view.highlights().len(), 0);
    assert_eq!(view.selected_highlight(), None);
    view.set_highlights(std::iter::once(0..3)).unwrap();
    assert!(view.clear_highlights());
    let mut empty = Viewport::new("");
    empty.set_highlights(std::iter::once(0..0)).unwrap();
    assert_eq!(empty.highlight_next(), Some(0));
}
#[test]
fn navigation_tracks_first_logical_range_and_wrapped_display_rows() {
    let mut view = Viewport::new("abcdefghij\nKLMNO\nPQRST");
    view.set_size(3, 1);
    view.set_horizontal_step(1);
    view.set_highlights([7..9, 12..14, 17..19]).unwrap();
    assert_eq!(view.selected_highlight(), Some(0));
    assert_eq!(view.horizontal_offset(), 6);
    assert_eq!(view.highlight_next(), Some(1));
    assert_eq!((view.offset(), view.horizontal_offset()), (1, 0));
    view.set_soft_wrap(true);
    assert!(view.select_highlight(0));
    assert_eq!(view.offset(), 2);
    assert_eq!(frame(&view, 3, 1).plain(), "ghi");
    view.go_to_bottom();
    assert_eq!(view.selected_highlight(), None); // all matches start above last row
    assert_eq!(view.highlight_previous(), Some(2)); // no selection starts from last
    assert_eq!(view.offset(), 6);
    view.go_to_top();
    assert_eq!(view.selected_highlight(), Some(0));
    view.set_width(0);
    assert_eq!(view.highlight_next(), Some(1));
    assert_eq!(frame(&view, 0, 1).plain(), "");
}
#[test]
fn shared_geometry_old_views_and_independent_selections_are_safe_across_threads() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Viewport>();
    let mut view = Viewport::new("one tea two tea");
    view.set_size(15, 1);
    view.set_highlights([4..7, 12..15]).unwrap();
    let old = view.view(&Theme::plain(), true);
    let mut clone = view.clone();
    clone.highlight_next();
    view.set_content("new");
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let shared = &clone;
            scope.spawn(move || {
                assert_eq!(shared.selected_highlight(), Some(1));
                assert_eq!(frame(shared, 15, 1).plain(), "one tea two tea");
            });
        }
    });
    let old = old.render(Size::new(15, 1)).unwrap();
    assert!(old.row(0)[4].style().has(TextAttribute::Reverse));
    assert!(!old.row(0)[12].style().has(TextAttribute::Reverse));
    assert!(
        !frame(&clone, 15, 1).row(0)[4]
            .style()
            .has(TextAttribute::Reverse)
    );
    assert!(
        frame(&clone, 15, 1).row(0)[12]
            .style()
            .has(TextAttribute::Reverse)
    );
}
#[test]
fn every_scalar_boundary_range_preserves_plain_geometry_across_clipping_and_wrap() {
    for text in ["A茶B👩‍💻C\n🇯🇵e\u{301}\n", "\u{301}a\n\n茶", "abcdef\nghij"] {
        let boundaries: Vec<_> = text
            .char_indices()
            .map(|(i, _)| i)
            .chain([text.len()])
            .collect();
        for &start in &boundaries {
            for &end in boundaries.iter().filter(|&&end| end >= start) {
                for wrap in [false, true] {
                    for width in 0..=5 {
                        let mut base = Viewport::new(text);
                        base.set_size(width, 3);
                        base.set_soft_wrap(wrap);
                        let mut styled = base.clone();
                        styled.set_highlights(std::iter::once(start..end)).unwrap();
                        // Hold exactly the same view positions after auto-reveal.
                        base.set_offset(styled.offset());
                        base.set_horizontal_offset(styled.horizontal_offset());
                        assert_eq!(
                            frame(&base, width, 3).plain(),
                            frame(&styled, width, 3).plain(),
                            "{text:?} {start}..{end} width={width} wrap={wrap}"
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn clipped_wide_left_padding_and_oversized_replacements_keep_source_styles() {
    let mut view = Viewport::new("茶BC");
    view.set_size(2, 1);
    view.set_highlights(std::iter::once(0..3)).unwrap();
    view.set_horizontal_offset(1);
    let surface = frame(&view, 2, 1);
    assert_eq!(surface.plain(), " B");
    assert_eq!(
        surface.row(0)[0].style().attributes.underline(),
        UnderlineStyle::None
    );
    assert_eq!(
        surface.row(0)[1].style().attributes.underline(),
        UnderlineStyle::None
    );
    view.set_width(1);
    view.set_soft_wrap(true);
    view.select_highlight(0);
    let surface = frame(&view, 1, 1);
    assert_eq!(surface.plain(), "�");
    assert_eq!(
        surface.row(0)[0].style().attributes.underline(),
        UnderlineStyle::Single
    );
    assert!(surface.row(0)[0].style().has(TextAttribute::Reverse));
}

fn decode_hex(text: &str) -> String {
    String::from_utf8(
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect(),
    )
    .unwrap()
}
fn canonical(view: &Viewport, width: u16, height: u16) -> String {
    use std::fmt::Write;
    let mut result = String::new();
    let surface = frame(view, width, height);
    for y in 0..height {
        for cell in surface.row(y) {
            if cell.is_continuation() {
                result.push_str("0,,0,0,0;");
                continue;
            }
            write!(result, "{},", cell.width()).unwrap();
            for byte in cell.symbol().bytes() {
                write!(result, "{byte:02x}").unwrap();
            }
            write!(
                result,
                ",{},{},{};",
                cell.style().has(TextAttribute::Bold),
                cell.style().attributes.underline() as u8,
                cell.style().has(TextAttribute::Reverse)
            )
            .unwrap();
        }
    }
    result
}
#[test]
fn pinned_420_source_states_retain_190_matches_and_230_literal_deviations() {
    let source: Vec<_> = include_str!("fixtures/viewport-highlight-source.tsv")
        .lines()
        .collect();
    let expected: Vec<_> = include_str!("fixtures/viewport-highlight-matea.tsv")
        .lines()
        .collect();
    let deviations: std::collections::BTreeMap<usize, _> =
        include_str!("fixtures/viewport-highlight-deviations.tsv")
            .lines()
            .map(|line| {
                let (index, reason) = line.split_once('\t').unwrap();
                assert!(!reason.is_empty());
                (index.parse().unwrap(), reason)
            })
            .collect();
    assert_eq!(
        (source.len(), expected.len(), deviations.len()),
        (420, 420, 230)
    );
    let mut matched = 0;
    for (row, (source, expected)) in source.into_iter().zip(expected).enumerate() {
        let a: Vec<_> = source.split('\t').collect();
        let b: Vec<_> = expected.split('\t').collect();
        assert_eq!((a.len(), b.len()), (11, 11));
        assert_eq!(a[..6], b[..6]);
        assert!(a[10].is_empty() && b[10].is_empty());
        let mut view = Viewport::new(&decode_hex(a[1]));
        let width = a[2].parse().unwrap();
        let height = a[3].parse().unwrap();
        view.set_size(width, height);
        view.set_soft_wrap(a[4].parse().unwrap());
        view.highlight_style = StylePatch::new()
            .attribute(TextAttribute::Bold, true)
            .underline_style(UnderlineStyle::Single);
        view.selected_highlight_style = StylePatch::new().attribute(TextAttribute::Reverse, true);
        let ranges = match a[0] {
            "ascii" => vec![4..7, 12..15, 23..26],
            "mixed" => vec![1..4, 5..16, 34..37],
            "split" => vec![0..1, 1..3, 7..10, 18..22],
            "multiline" => vec![1..4, 6..8],
            "empty" => vec![0..0, 2..2],
            _ => panic!("unknown corpus"),
        };
        view.set_highlights(ranges).unwrap();
        match a[5] {
            "set" => {}
            "next" => {
                view.highlight_next();
            }
            "previous" => {
                view.highlight_previous();
            }
            "clear" => {
                view.clear_highlights();
            }
            "empty" => {
                view.set_highlights([]).unwrap();
            }
            "bottom" => {
                view.go_to_bottom();
            }
            "top" => {
                view.go_to_top();
            }
            _ => panic!("unknown action"),
        };
        assert_eq!(
            (
                view.horizontal_offset().to_string(),
                view.offset().to_string(),
                canonical(&view, width, height)
            ),
            (b[6].to_string(), b[7].to_string(), decode_hex(b[9])),
            "literal Matea state {row}"
        );
        let equal = [6, 7, 9, 10].into_iter().all(|i| a[i] == b[i]);
        assert_eq!(
            equal,
            !deviations.contains_key(&row),
            "explicit source deviation membership {row}"
        );
        matched += usize::from(equal);
    }
    assert_eq!(matched, 190);
}

#[test]
fn adversarial_size_hints_do_not_control_reservations_or_match_semantics() {
    struct Hints<I> {
        inner: I,
        hint: usize,
    }
    impl<I: Iterator> Iterator for Hints<I> {
        type Item = I::Item;
        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next()
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            (self.hint, Some(self.hint))
        }
    }
    for text in ["abc\ndef\n", "茶e\u{301}\nlast"] {
        for hint in [0, 1, 256, usize::MAX] {
            let mut view = Viewport::new(text);
            view.set_size(3, 2);
            let ranges = [
                0..text.chars().next().unwrap().len_utf8(),
                text.len()..text.len(),
            ];
            view.set_highlights(Hints {
                inner: ranges.clone().into_iter(),
                hint,
            })
            .unwrap();
            assert_eq!(view.highlights().collect::<Vec<_>>(), ranges);
            assert_eq!(view.selected_highlight(), Some(0));
            view.set_highlights(Hints {
                inner: std::iter::empty(),
                hint,
            })
            .unwrap();
            assert_eq!(view.highlights().len(), 0);
            assert_eq!(view.selected_highlight(), None);
        }
    }
}
