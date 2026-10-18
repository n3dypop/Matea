use matea_core::{Component, Input, Key, KeyCode};
use matea_render::Size;
use matea_style::Theme;
use matea_widgets::{Action, Viewport};

fn key(code: KeyCode) -> Input {
    Input::Key(Key::new(code))
}

#[test]
fn trailing_empty_lines_are_scrollable_and_crlf_is_structural() {
    let mut viewport = Viewport::new("a\r\nb\n\n");
    viewport.rows = 1;
    viewport.event(&key(KeyCode::End), true);
    assert_eq!(viewport.offset(), 3);
    assert_eq!(
        viewport
            .view(&Theme::plain(), true)
            .render(Size::new(8, 1))
            .unwrap()
            .plain(),
        ""
    );
    viewport.event(&key(KeyCode::Home), true);
    assert_eq!(
        viewport
            .view(&Theme::plain(), true)
            .render(Size::new(8, 1))
            .unwrap()
            .plain(),
        "a"
    );
}

#[test]
fn resize_clamps_observed_position_before_applying_navigation() {
    let mut viewport = Viewport::new("a\nb\nc\nd\ne\nf");
    viewport.rows = 1;
    viewport.event(&key(KeyCode::End), true);
    assert_eq!(viewport.offset(), 5);
    viewport.rows = 4;
    assert_eq!(viewport.offset(), 2);
    assert_eq!(
        viewport.event(&key(KeyCode::Up), true).message,
        Some(Action::Changed)
    );
    assert_eq!(viewport.offset(), 1);
    assert_eq!(
        viewport
            .view(&Theme::plain(), true)
            .render(Size::new(8, 4))
            .unwrap()
            .plain(),
        "b\nc\nd\ne"
    );
}

#[test]
fn ignored_navigation_does_not_commit_a_resize_position_change() {
    let mut viewport = Viewport::new("a\nb\nc\nd\ne\nf");
    viewport.rows = 1;
    viewport.event(&key(KeyCode::End), true);
    viewport.rows = 4;
    assert_eq!(viewport.offset(), 2);
    assert!(!viewport.event(&key(KeyCode::Char('x')), true).consumed);
    viewport.rows = 1;
    assert_eq!(viewport.offset(), 5);
}

#[test]
fn viewport_slices_match_an_independent_line_model() {
    for text in [
        "",
        "\n",
        "\n\n",
        "a",
        "a\n",
        "茶\ne\u{301}\n👩‍💻",
        "a\t\x1b\nb\r\nc\r",
    ] {
        let expected = text
            .replace("\r\n", "\n")
            .chars()
            .fold(String::new(), |mut s, c| {
                match c {
                    '\t' => s.push_str("    "),
                    '\n' => s.push('\n'),
                    c if c.is_control() => s.push('�'),
                    c => s.push(c),
                }
                s
            });
        let lines: Vec<_> = if expected.is_empty() {
            vec![]
        } else {
            expected.split('\n').collect()
        };
        for rows in 0..8 {
            let mut viewport = Viewport::new(text);
            viewport.rows = rows;
            let mut offset = 0usize;
            for code in [
                KeyCode::End,
                KeyCode::Up,
                KeyCode::PageUp,
                KeyCode::Down,
                KeyCode::Home,
                KeyCode::PageDown,
            ] {
                let max = lines.len().saturating_sub(usize::from(rows));
                offset = match code {
                    KeyCode::End => max,
                    KeyCode::Home => 0,
                    KeyCode::Up => offset.saturating_sub(1),
                    KeyCode::Down => offset.saturating_add(1),
                    KeyCode::PageUp => offset.saturating_sub(usize::from(rows)),
                    _ => offset.saturating_add(usize::from(rows)),
                }
                .min(max);
                viewport.event(&key(code), true);
                assert_eq!(viewport.offset(), offset);
                let view = viewport
                    .view(&Theme::plain(), true)
                    .render(Size::new(40, rows))
                    .unwrap();
                let mut visible: Vec<String> = lines
                    .iter()
                    .skip(offset)
                    .take(usize::from(rows))
                    .map(|s| s.trim_end().to_string())
                    .collect();
                visible.resize(usize::from(rows), String::new());
                assert_eq!(
                    view.plain(),
                    visible.join("\n"),
                    "text={text:?} rows={rows} key={code:?}"
                );
            }
        }
    }
}

#[test]
fn replacements_own_input_and_old_views_keep_their_snapshot() {
    let mut input = String::from("tea\ncoffee\n");
    let mut viewport = Viewport::new("old");
    viewport.set_height(1);
    viewport.set_content(&input);
    viewport.go_to_bottom();
    let previous = viewport.view(&Theme::plain(), true);
    input.clear();
    input.push_str("external mutation");
    assert_eq!(viewport.content(), "tea\ncoffee\n");
    assert_eq!(viewport.offset(), 2);
    viewport.set_content("new");
    assert_eq!(viewport.offset(), 0);
    assert_eq!(viewport.content(), "new");
    assert_eq!(previous.render(Size::new(20, 1)).unwrap().plain(), "");
    assert_eq!(viewport.visible_text(), "new");
}

#[test]
fn owned_replacement_reuses_clean_utf8_and_normalizes_other_input() {
    let owned = String::from("茶\ne\u{301}\n👩‍💻");
    let original = owned.as_ptr();
    let mut viewport = Viewport::from_string(owned);
    assert_eq!(viewport.content().as_ptr(), original);
    let replacement = String::from("replacement\n");
    let original = replacement.as_ptr();
    viewport.set_content_owned(replacement);
    assert_eq!(viewport.content().as_ptr(), original);
    viewport.set_content_owned(String::from("tea\r\n\t\u{85}\r"));
    assert_eq!(viewport.content(), "tea\n    ��");
    assert_eq!(viewport.lines().collect::<Vec<_>>(), ["tea", "    ��"]);
}

#[test]
fn line_inputs_normalize_individually_without_borrowing_or_changing_the_caller() {
    let mut source = vec![
        "tea\r\ncoffee".to_owned(),
        "\r".to_owned(),
        "茶\n".to_owned(),
    ];
    let original = source.clone();
    let mut viewport = Viewport::new("");
    viewport.set_content_lines(&source);
    assert_eq!(source, original);
    assert_eq!(viewport.content(), "tea\ncoffee\n�\n茶\n");
    assert_eq!(viewport.line_count(), 5);
    source[0] = "external mutation".to_owned();
    assert_eq!(viewport.line(0), Some("tea"));
    assert_eq!(viewport.line(4), Some(""));
    assert_eq!(viewport.line(5), None);
    assert_eq!(viewport.line(usize::MAX), None);
    let mut lines = viewport.lines();
    assert_eq!(lines.len(), 5);
    assert_eq!(lines.next_back(), Some(""));
    assert_eq!(lines.next(), Some("tea"));
    assert_eq!(lines.len(), 3);
    assert_eq!(lines.collect::<Vec<_>>(), ["coffee", "�", "茶"]);
    viewport.set_content_lines(std::iter::empty::<&str>());
    assert_eq!((viewport.content(), viewport.line_count()), ("", 0));
    viewport.set_content_lines([""]);
    assert_eq!(viewport.line_count(), 0);
    viewport.set_content_lines(["", ""]);
    assert_eq!(viewport.line_count(), 2);
}

#[test]
fn failed_line_iteration_preserves_content_position_and_derived_index() {
    let mut viewport = Viewport::new("a\nb\nc");
    viewport.set_height(1);
    viewport.go_to_bottom();
    let mut calls = 0;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        viewport.set_content_lines(std::iter::from_fn(|| {
            calls += 1;
            if calls == 1 {
                Some("replacement")
            } else {
                panic!("caller iterator failed")
            }
        }));
    }));
    assert!(result.is_err());
    assert_eq!(viewport.content(), "a\nb\nc");
    assert_eq!(viewport.offset(), 2);
    assert_eq!(viewport.line_count(), 3);
    assert_eq!(viewport.visible_text(), "c");
}

#[test]
fn explicit_resize_commits_clamping_and_zero_movement_does_not() {
    let mut viewport = Viewport::new("a\nb\nc\nd\ne\nf");
    viewport.set_height(1);
    assert!(viewport.go_to_bottom());
    viewport.rows = 4;
    assert!(viewport.is_past_bottom());
    assert_eq!(viewport.offset(), 2);
    assert!(!viewport.scroll_down(0));
    viewport.rows = 1;
    assert_eq!(viewport.offset(), 5);
    viewport.set_height(4);
    assert!(!viewport.is_past_bottom());
    viewport.set_height(1);
    assert_eq!(viewport.offset(), 2);
    viewport.set_content("one\ntwo");
    assert_eq!(viewport.offset(), 1);
    assert!(viewport.is_at_bottom());
    assert_eq!(viewport.scroll_fraction(), 1.0);
    viewport.set_height(0);
    assert_eq!(viewport.visible_line_count(), 0);
    assert_eq!(viewport.visible_text(), "");
    assert_eq!(viewport.scroll_fraction(), 0.5);
    viewport.set_height(8);
    assert!(viewport.is_at_top() && viewport.is_at_bottom());
    assert_eq!(viewport.scroll_fraction(), 1.0);
    viewport.set_content("");
    assert_eq!(viewport.scroll_fraction(), 1.0);
}

#[test]
fn public_controls_match_an_independent_bounded_position_model() {
    let operations = [0, 3, 4, 5, 6, 7, 1, 2, 8, 9];
    for height in [0, 1, 2, 4, 8] {
        let mut viewport = Viewport::new("a\nb\nc\nd\ne\nf");
        viewport.set_height(height);
        let maximum = 6usize.saturating_sub(usize::from(height));
        let mut position = 0usize;
        for operation in operations {
            let before = position;
            position = match operation {
                0 => maximum,
                1 => 0,
                2 => maximum,
                3 => position.saturating_sub(usize::from(height)),
                4 => position.saturating_add(usize::from(height)).min(maximum),
                5 => position.saturating_sub(usize::from(height / 2)),
                6 => position
                    .saturating_add(usize::from(height / 2))
                    .min(maximum),
                7 => 0,
                8 => maximum,
                _ => position,
            };
            let changed = match operation {
                0 => viewport.set_offset(usize::MAX),
                1 => viewport.go_to_top(),
                2 => viewport.go_to_bottom(),
                3 => viewport.page_up(),
                4 => viewport.page_down(),
                5 => viewport.half_page_up(),
                6 => viewport.half_page_down(),
                7 => viewport.scroll_up(usize::MAX),
                8 => viewport.scroll_down(usize::MAX),
                _ => viewport.scroll_down(0),
            };
            assert_eq!((changed, viewport.offset()), (position != before, position));
            assert_eq!(viewport.is_at_top(), position == 0);
            assert_eq!(viewport.is_at_bottom(), position == maximum);
            assert_eq!(
                viewport.visible_line_count(),
                (6 - position).min(usize::from(height))
            );
            let expected = if maximum == 0 {
                1.0
            } else {
                position as f64 / maximum as f64
            };
            assert_eq!(viewport.scroll_fraction(), expected);
        }
    }
}

#[test]
fn pinned_public_controls_match_and_crlf_content_deviation_is_explicit() {
    let mut fixture = include_str!("fixtures/viewport-controls.tsv").lines();
    let mut cases = 0;
    let mut crlf_deviations = 0;
    for (index, text) in [
        "",
        "tea",
        "tea\ncoffee\n",
        "tea\r\ncoffee",
        "茶\ne\u{301}\n👩‍💻",
    ]
    .into_iter()
    .enumerate()
    {
        for height in [0, 1, 2, 6] {
            let mut viewport = Viewport::new(text);
            viewport.set_height(height);
            for step in 0..11 {
                match step {
                    1 => {
                        viewport.set_offset(usize::MAX);
                    }
                    2 => {
                        viewport.scroll_up(1);
                    }
                    3 => {
                        viewport.half_page_up();
                    }
                    4 => {
                        viewport.page_up();
                    }
                    5 => {
                        viewport.scroll_down(1);
                    }
                    6 => {
                        viewport.half_page_down();
                    }
                    7 => {
                        viewport.page_down();
                    }
                    8 => {
                        viewport.go_to_top();
                    }
                    9 => {
                        viewport.go_to_bottom();
                    }
                    10 => {
                        viewport.set_content_lines(["new", "tea\r\ncoffee", ""]);
                    }
                    _ => {}
                }
                let hex: String = viewport
                    .content()
                    .bytes()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
                let actual = format!(
                    "{index}\t{height}\t{step}\t{}\t{}\t{}\t{}\t{}\t{:016x}\t{hex}",
                    viewport.offset(),
                    viewport.line_count(),
                    viewport.visible_line_count(),
                    viewport.is_at_top(),
                    viewport.is_at_bottom(),
                    viewport.scroll_fraction().to_bits()
                );
                let expected = fixture.next().unwrap();
                if index == 3 && step < 10 {
                    // SetContent splits at LF before trying per-line CRLF
                    // replacement. The pinned public API therefore keeps CR,
                    // despite its normalization documentation. Preserve the
                    // exact source observation and our existing safe policy.
                    let (actual_state, actual_hex) = actual.rsplit_once('\t').unwrap();
                    let (expected_state, expected_hex) = expected.rsplit_once('\t').unwrap();
                    assert_eq!(actual_state, expected_state);
                    assert_eq!(expected_hex, "7465610d0a636f66666565");
                    assert_eq!(actual_hex, "7465610a636f66666565");
                    crlf_deviations += 1;
                } else {
                    assert_eq!(actual, expected);
                }
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 220);
    assert_eq!(crlf_deviations, 40);
    assert_eq!(fixture.next(), None);
}
