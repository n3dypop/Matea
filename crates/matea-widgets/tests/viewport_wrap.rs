use matea_core::Component;
use matea_render::Size;
use matea_style::Theme;
use matea_widgets::Viewport;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

fn rows(view: &Viewport) -> Vec<String> {
    view.visible_lines().map(|line| line.into_owned()).collect()
}

#[test]
fn wrapping_uses_display_rows_without_changing_logical_content() {
    let mut view = Viewport::new("abcde\nx\n\nTAIL\n");
    view.set_size(2, 3);
    view.set_soft_wrap(true);
    assert_eq!((view.line_count(), view.display_line_count()), (5, 8));
    assert_eq!(rows(&view), ["ab", "cd", "e"]);
    assert_eq!(view.visible_text(), "abcde");
    assert_eq!(view.line(0), Some("abcde"));
    view.go_to_bottom();
    assert_eq!(view.offset(), 5);
    assert_eq!(rows(&view), ["TA", "IL", ""]);
    assert_eq!(
        view.view(&Theme::plain(), true)
            .render(Size::new(2, 3))
            .unwrap()
            .plain(),
        "TA\nIL\n"
    );
    view.page_up();
    assert_eq!(rows(&view), ["e", "x", ""]);
    assert_eq!(view.scroll_fraction(), 0.4);
    assert!(
        view.visible_lines()
            .all(|line| matches!(line, std::borrow::Cow::Borrowed(_)))
    );
}

#[test]
fn whole_graphemes_fit_rows_and_oversized_clusters_get_one_replacement() {
    let raw = "A茶B👩‍💻C\n🇯🇵e\u{301}\n";
    let mut view = Viewport::new(raw);
    view.set_size(3, 20);
    view.set_soft_wrap(true);
    assert_eq!(rows(&view), ["A茶", "B👩‍💻", "C", "🇯🇵e\u{301}", ""]);
    view.set_width(1);
    assert_eq!(rows(&view), ["A", "�", "B", "�", "C", "�", "e\u{301}", ""]);
    assert_eq!(view.content(), raw);
    assert!(
        view.visible_lines()
            .all(|line| matches!(line, std::borrow::Cow::Borrowed(_)))
    );
    view.set_width(2);
    assert_eq!(
        rows(&view),
        ["A", "茶", "B", "👩‍💻", "C", "🇯🇵", "e\u{301}", ""]
    );
    let frame = view
        .view(&Theme::plain(), false)
        .render(Size::new(2, 20))
        .unwrap();
    assert!(frame.row(1)[1].is_continuation());
    assert_eq!(frame.row(1)[0].symbol(), "茶");
    assert_eq!(frame.row(3)[0].symbol(), "👩‍💻");
}

#[test]
fn fill_height_includes_borrowed_empty_rows_and_respects_empty_geometry() {
    let mut view = Viewport::new("");
    view.rows = 4;
    assert_eq!(view.visible_line_count(), 0);
    view.fill_height = true;
    assert_eq!(rows(&view), ["", "", "", ""]);
    assert_eq!(view.visible_text(), "");
    view.set_content("tea");
    assert_eq!(rows(&view), ["tea", "", "", ""]);
    view.set_size(0, 4);
    assert_eq!(view.visible_line_count(), 0);
    view.set_width(3);
    view.set_height(0);
    assert_eq!(view.visible_line_count(), 0);
    view.set_size(3, 4);
    view.set_soft_wrap(true);
    assert_eq!(rows(&view), ["tea", "", "", ""]);
    view.fill_height = false;
    assert_eq!(rows(&view), ["tea"]);
}

#[test]
fn zero_sizes_resize_and_parent_width_have_defined_position_semantics() {
    let mut view = Viewport::new("abcdef\nghi");
    view.set_soft_wrap(true);
    assert_eq!(view.display_line_count(), 2); // width supplied by parent: suspended
    view.set_size(2, 0);
    assert_eq!(view.display_line_count(), 5);
    view.go_to_bottom();
    assert_eq!(view.offset(), 5);
    assert_eq!(view.visible_text(), "");
    assert_eq!(rows(&view), Vec::<String>::new());
    view.set_height(1);
    assert_eq!(view.offset(), 4);
    view.set_width(0);
    assert_eq!((view.display_line_count(), view.offset()), (0, 0));
    assert_eq!(rows(&view), Vec::<String>::new());
    view.set_size(3, 1);
    view.go_to_bottom();
    assert_eq!((view.display_line_count(), view.offset()), (3, 2));
    view.clear_width();
    assert_eq!((view.display_line_count(), view.offset()), (2, 1));
    assert_eq!(rows(&view), ["ghi"]);
    view.set_soft_wrap(false);
    assert_eq!(rows(&view), ["ghi"]);
}

#[test]
fn caches_clones_and_old_views_keep_independent_content_and_geometry() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Viewport>();
    let mut view = Viewport::new("abcdef\n抹茶");
    view.set_size(2, 20);
    view.set_horizontal_offset(2);
    view.set_soft_wrap(true);
    assert_eq!(view.horizontal_offset(), 0);
    assert!(!view.scroll_right(usize::MAX));
    assert_eq!(view.max_horizontal_offset(), 0);
    assert_eq!(view.display_line_count(), 5);
    let clone = view.clone();
    let old = view.view(&Theme::plain(), false);
    view.set_width(4);
    assert_eq!(view.display_line_count(), 3);
    assert_eq!(clone.display_line_count(), 5);
    view.set_content("new");
    assert_eq!(rows(&view), ["new"]);
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| assert_eq!(rows(&clone), ["ab", "cd", "ef", "抹", "茶"]));
        }
    });
    assert!(
        old.render(Size::new(2, 20))
            .unwrap()
            .plain()
            .starts_with("ab\ncd\nef\n抹\n茶\n")
    );
}

#[test]
fn ensure_visible_maps_logical_cell_columns_to_wrapped_rows_transactionally() {
    let mut view = Viewport::new("abcdefghij\nKLMNO");
    view.set_size(3, 1);
    view.set_soft_wrap(true);
    view.ensure_visible(0, 7..8).unwrap();
    assert_eq!((view.offset(), rows(&view)), (2, vec!["ghi".to_owned()]));
    view.ensure_visible(1, 4..5).unwrap();
    assert_eq!((view.offset(), rows(&view)), (5, vec!["NO".to_owned()]));
    let before = (view.offset(), rows(&view));
    assert!(
        view.ensure_visible(0, std::ops::Range { start: 8, end: 4 })
            .is_err()
    );
    assert_eq!((view.offset(), rows(&view)), before);
    view.ensure_visible(usize::MAX, usize::MAX..usize::MAX)
        .unwrap();
    assert_eq!(view.offset(), 5);
    view.ensure_visible(0, 0..1).unwrap();
    assert_eq!(view.offset(), 0);
}

#[test]
fn every_wrapped_row_obeys_its_budget_and_preserves_the_complete_glyph_stream() {
    let raw = "\u{301}A茶e\u{301}👩‍💻\u{200b}🇯🇵1️⃣\n👩‍💻tail\n\n";
    for width in 1..=9 {
        let mut view = Viewport::new(raw);
        view.set_size(width, u16::MAX);
        view.set_soft_wrap(true);
        let actual = rows(&view);
        assert_eq!(actual.len(), view.display_line_count());
        let mut visible = Vec::new();
        for row in &actual {
            let cells: usize = row.graphemes(true).map(UnicodeWidthStr::width).sum();
            assert!(cells <= usize::from(width), "{row:?} width={width}");
            visible.extend(
                row.graphemes(true)
                    .filter(|g| g.width() > 0)
                    .map(str::to_owned),
            );
        }
        let expected = raw
            .split('\n')
            .flat_map(|line| line.graphemes(true))
            .filter(|g| g.width() > 0)
            .map(|g| {
                if g.width() > usize::from(width) {
                    "�"
                } else {
                    g
                }
                .to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(visible, expected);
        for height in 0..=8 {
            view.set_height(height);
            for offset in [0, 1, 2, 7, usize::MAX] {
                view.set_offset(offset);
                assert!(view.offset() <= view.max_offset());
                assert!(view.visible_line_count() <= usize::from(height));
                view.view(&Theme::plain(), false)
                    .render(Size::new(width, height))
                    .unwrap();
            }
        }
    }
}

#[test]
fn pinned_source_matrix_has_586_exact_states_and_422_literal_reviewed_deviations() {
    fn decode(text: &str) -> String {
        String::from_utf8(
            (0..text.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
                .collect(),
        )
        .unwrap()
    }
    let source = include_str!("fixtures/viewport-wrap-source.tsv")
        .lines()
        .collect::<Vec<_>>();
    let matea = include_str!("fixtures/viewport-wrap-matea.tsv")
        .lines()
        .collect::<Vec<_>>();
    let mut reviewed = std::collections::BTreeSet::new();
    for row in include_str!("fixtures/viewport-wrap-deviations.tsv").lines() {
        let (index, reason) = row.split_once('\t').unwrap();
        assert!(!reason.is_empty());
        assert!(reviewed.insert(index.parse::<usize>().unwrap()));
    }
    assert_eq!(
        (source.len(), matea.len(), reviewed.len()),
        (1008, 1008, 422)
    );
    let mut exact = 0;
    for (index, (old, expected)) in source.iter().zip(matea).enumerate() {
        let old = old.split('\t').collect::<Vec<_>>();
        let expected = expected.split('\t').collect::<Vec<_>>();
        assert_eq!((old.len(), expected.len()), (13, 11));
        assert_eq!(&old[..6], &expected[..6]);
        assert!(old[12].is_empty(), "source panic in fixture {index}");
        let width = old[2].parse().unwrap();
        let height = old[3].parse().unwrap();
        let mut view = Viewport::new(&decode(old[1]));
        view.set_size(width, height);
        view.set_soft_wrap(true);
        view.fill_height = old[4].parse().unwrap();
        match old[5] {
            "top" => {}
            "bottom" => {
                view.go_to_bottom();
            }
            "page" => {
                view.page_down();
                view.page_down();
            }
            "ensure" => {
                view.ensure_visible(1, 4..5).unwrap();
            }
            _ => panic!("unknown fixture action"),
        }
        let fields = [
            view.display_line_count(),
            view.offset(),
            view.horizontal_offset(),
            view.visible_line_count(),
        ];
        for (value, expected) in fields.into_iter().zip(&expected[6..10]) {
            assert_eq!(value.to_string(), *expected, "case {index}");
        }
        let plain = view
            .view(&Theme::plain(), false)
            .render(Size::new(width, height))
            .unwrap()
            .plain();
        assert_eq!(plain, decode(expected[10]), "case {index}");
        let matched = old[6..10] == expected[6..10] && old[11] == expected[10];
        if reviewed.remove(&index) {
            assert!(!matched, "stale deviation {index}");
        } else {
            assert!(matched, "unreviewed source deviation {index}");
            exact += 1;
        }
    }
    assert_eq!(exact, 586);
    assert!(reviewed.is_empty());
}
