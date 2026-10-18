use matea_core::{
    Component, Input, Key, KeyCode, KeyKind, Modifiers, Mouse, MouseKind, ScrollDirection,
};
use matea_render::Size;
use matea_style::Theme;
use matea_widgets::{Action, Viewport, ViewportKey, key::Binding};

fn key(code: KeyCode) -> Input {
    Input::Key(Key::new(code))
}

#[test]
fn pager_defaults_match_each_documented_chord_and_keep_existing_shift_navigation() {
    let cases = [
        (KeyCode::PageDown, Modifiers::empty(), 12, 8),
        (KeyCode::Char(' '), Modifiers::empty(), 12, 8),
        (KeyCode::Char('f'), Modifiers::empty(), 12, 8),
        (KeyCode::PageUp, Modifiers::empty(), 4, 8),
        (KeyCode::Char('b'), Modifiers::empty(), 4, 8),
        (KeyCode::Char('u'), Modifiers::empty(), 6, 8),
        (KeyCode::Char('u'), Modifiers::CONTROL, 6, 8),
        (KeyCode::Char('d'), Modifiers::empty(), 10, 8),
        (KeyCode::Char('d'), Modifiers::CONTROL, 10, 8),
        (KeyCode::Down, Modifiers::empty(), 9, 8),
        (KeyCode::Char('j'), Modifiers::empty(), 9, 8),
        (KeyCode::Up, Modifiers::empty(), 7, 8),
        (KeyCode::Char('k'), Modifiers::empty(), 7, 8),
        (KeyCode::Left, Modifiers::empty(), 8, 2),
        (KeyCode::Char('h'), Modifiers::empty(), 8, 2),
        (KeyCode::Right, Modifiers::empty(), 8, 14),
        (KeyCode::Char('l'), Modifiers::empty(), 8, 14),
        (KeyCode::Home, Modifiers::empty(), 0, 8),
        (KeyCode::End, Modifiers::empty(), 16, 8),
        (KeyCode::Down, Modifiers::SHIFT, 9, 8),
        (KeyCode::Up, Modifiers::SHIFT, 7, 8),
        (KeyCode::PageDown, Modifiers::SHIFT, 12, 8),
        (KeyCode::PageUp, Modifiers::SHIFT, 4, 8),
        (KeyCode::Home, Modifiers::SHIFT, 0, 8),
        (KeyCode::End, Modifiers::SHIFT, 16, 8),
        (KeyCode::Right, Modifiers::SHIFT, 8, 14),
        (KeyCode::Left, Modifiers::SHIFT, 8, 2),
    ];
    for (code, modifiers, row, column) in cases {
        let mut viewport = Viewport::new(&vec!["X".repeat(40); 20].join("\n"));
        viewport.set_size(5, 4);
        viewport.set_offset(8);
        viewport.set_horizontal_offset(8);
        let event = Key::new(code).with_modifiers(modifiers);
        assert!(!viewport.event(&Input::Key(event), false).consumed);
        assert!(
            !viewport
                .event(&Input::Key(event.with_kind(KeyKind::Release)), true)
                .consumed
        );
        assert_eq!((viewport.offset(), viewport.horizontal_offset()), (8, 8));
        assert_eq!(
            viewport
                .event(&Input::Key(event.with_kind(KeyKind::Repeat)), true)
                .message,
            Some(Action::Changed)
        );
        assert_eq!(
            (viewport.offset(), viewport.horizontal_offset()),
            (row, column),
            "{event:?}"
        );
    }
}

#[test]
fn pager_overrides_share_help_enable_exact_modifiers_precedence_and_reset() {
    let mut viewport = Viewport::new("a\nb\nc\nd\ne\nf\ng\nh\ni\nj");
    viewport.set_height(4);
    let chord = Key::new(KeyCode::F(9)).with_modifiers(Modifiers::ALT);
    viewport.keys.set(
        ViewportKey::Down,
        Binding::new([chord], "alt+F9", "advance"),
    );
    assert_eq!(viewport.keys.chords(ViewportKey::Down), [chord]);
    assert_eq!(
        viewport.keys.help(ViewportKey::Down),
        Some(("alt+F9", "advance"))
    );
    assert!(!viewport.event(&key(KeyCode::Down), true).consumed);
    assert!(!viewport.event(&key(KeyCode::F(9)), true).consumed);
    assert!(viewport.event(&Input::Key(chord), true).consumed);
    assert_eq!(viewport.offset(), 1);
    let mut disabled = Binding::new([chord], "alt+F9", "advance");
    disabled.enabled = false;
    viewport.keys.set(ViewportKey::Down, disabled);
    assert!(viewport.keys.help(ViewportKey::Down).is_none());
    assert!(!viewport.event(&Input::Key(chord), true).consumed);
    viewport
        .keys
        .set(ViewportKey::Down, Binding::new([], "empty", "inactive"));
    assert!(viewport.keys.help(ViewportKey::Down).is_none());
    assert!(viewport.keys.reset(ViewportKey::Down));
    assert!(!viewport.keys.reset(ViewportKey::Down));
    assert_eq!(viewport.keys.help(ViewportKey::Down), Some(("↓/j", "down")));
    assert!(viewport.event(&key(KeyCode::Down), true).consumed);
    viewport
        .keys
        .set(ViewportKey::PageDown, Binding::new([chord], "F9", "page"));
    viewport
        .keys
        .set(ViewportKey::Up, Binding::new([chord], "F9", "up"));
    let clone = viewport.clone();
    viewport.event(&Input::Key(chord), true);
    assert_eq!(viewport.offset(), 6); // earlier page action wins
    viewport.keys.reset(ViewportKey::PageDown);
    viewport.event(&Input::Key(chord), true);
    assert_eq!(viewport.offset(), 5);
    assert_eq!(clone.keys.help(ViewportKey::PageDown), Some(("F9", "page")));
}

#[test]
fn focused_wheel_navigation_obeys_direction_modifiers_enable_delta_and_step() {
    let mut viewport = Viewport::new(&vec!["X".repeat(40); 20].join("\n"));
    viewport.set_size(5, 4);
    let wheel = |direction, modifiers| {
        Input::Mouse(Mouse {
            kind: MouseKind::Scroll(direction),
            column: 1,
            row: 1,
            modifiers,
        })
    };
    for (direction, modifiers, row, column) in [
        (ScrollDirection::Down, Modifiers::empty(), 11, 8),
        (ScrollDirection::Up, Modifiers::empty(), 5, 8),
        (ScrollDirection::Right, Modifiers::empty(), 8, 14),
        (ScrollDirection::Left, Modifiers::empty(), 8, 2),
        (ScrollDirection::Down, Modifiers::SHIFT, 8, 14),
        (ScrollDirection::Up, Modifiers::SHIFT, 8, 2),
    ] {
        viewport.set_offset(8);
        viewport.set_horizontal_offset(8);
        let input = wheel(direction, modifiers);
        assert!(!viewport.event(&input, false).consumed);
        assert!(viewport.event(&input, true).consumed);
        assert_eq!(
            (viewport.offset(), viewport.horizontal_offset()),
            (row, column)
        );
    }
    viewport.set_offset(8);
    viewport.set_horizontal_offset(8);
    for modifier in [
        Modifiers::CONTROL,
        Modifiers::ALT,
        Modifiers::SHIFT | Modifiers::CONTROL,
    ] {
        assert!(
            !viewport
                .event(&wheel(ScrollDirection::Down, modifier), true)
                .consumed
        );
        assert_eq!((viewport.offset(), viewport.horizontal_offset()), (8, 8));
    }
    viewport.mouse_wheel_enabled = false;
    assert!(
        !viewport
            .event(&wheel(ScrollDirection::Down, Modifiers::empty()), true)
            .consumed
    );
    viewport.mouse_wheel_enabled = true;
    viewport.mouse_wheel_delta = 0;
    assert!(
        !viewport
            .event(&wheel(ScrollDirection::Down, Modifiers::empty()), true)
            .consumed
    );
    assert_eq!((viewport.offset(), viewport.horizontal_offset()), (8, 8));
    viewport.set_horizontal_step(0);
    assert!(
        !viewport
            .event(&wheel(ScrollDirection::Down, Modifiers::SHIFT), true)
            .consumed
    );
    viewport.mouse_wheel_delta = usize::MAX;
    viewport.event(&wheel(ScrollDirection::Down, Modifiers::empty()), true);
    assert_eq!(viewport.offset(), 16);
    viewport.event(&wheel(ScrollDirection::Up, Modifiers::empty()), true);
    assert_eq!(viewport.offset(), 0);
    viewport.set_horizontal_step(usize::MAX);
    viewport.event(&wheel(ScrollDirection::Right, Modifiers::empty()), true);
    assert_eq!(viewport.horizontal_offset(), 35);
    viewport.clear_width();
    assert!(
        !viewport
            .event(&wheel(ScrollDirection::Right, Modifiers::empty()), true)
            .consumed
    );
    let moved = Input::Mouse(Mouse {
        kind: MouseKind::Moved,
        column: 1,
        row: 1,
        modifiers: Modifiers::empty(),
    });
    assert!(!viewport.event(&moved, true).consumed);
}

#[test]
fn public_input_state_matches_source_with_literal_command_wheel_policy_difference() {
    let keys = [
        Key::new(KeyCode::PageDown),
        Key::new(KeyCode::Char(' ')),
        Key::new(KeyCode::Char('f')),
        Key::new(KeyCode::PageUp),
        Key::new(KeyCode::Char('b')),
        Key::new(KeyCode::Char('u')),
        Key::new(KeyCode::Char('u')).with_modifiers(Modifiers::CONTROL),
        Key::new(KeyCode::Char('d')),
        Key::new(KeyCode::Char('d')).with_modifiers(Modifiers::CONTROL),
        Key::new(KeyCode::Down),
        Key::new(KeyCode::Char('j')),
        Key::new(KeyCode::Up),
        Key::new(KeyCode::Char('k')),
        Key::new(KeyCode::Left),
        Key::new(KeyCode::Char('h')),
        Key::new(KeyCode::Right),
        Key::new(KeyCode::Char('l')),
    ];
    let wheels = [
        (ScrollDirection::Down, Modifiers::empty()),
        (ScrollDirection::Up, Modifiers::empty()),
        (ScrollDirection::Right, Modifiers::empty()),
        (ScrollDirection::Left, Modifiers::empty()),
        (ScrollDirection::Down, Modifiers::SHIFT),
        (ScrollDirection::Up, Modifiers::SHIFT),
    ];
    let wheel = |direction, modifiers| {
        Input::Mouse(Mouse {
            kind: MouseKind::Scroll(direction),
            column: 1,
            row: 1,
            modifiers,
        })
    };
    let chord = Key::new(KeyCode::F(9)).with_modifiers(Modifiers::ALT);
    let mut exact = 0;
    let mut policy_difference = 0;
    for row in include_str!("fixtures/viewport-input.tsv").lines() {
        let name = row.split('\t').next().unwrap();
        let mut viewport = Viewport::new(&vec!["X".repeat(40); 20].join("\n"));
        viewport.set_size(5, 4);
        viewport.set_offset(8);
        viewport.set_horizontal_offset(8);
        let input = if let Some(index) = name
            .strip_prefix("key-")
            .and_then(|index| index.parse::<usize>().ok())
        {
            Input::Key(keys[index])
        } else if let Some(index) = name
            .strip_prefix("wheel-")
            .and_then(|index| index.parse::<usize>().ok())
        {
            let (direction, modifiers) = wheels[index];
            wheel(direction, modifiers)
        } else {
            match name {
                "release-down" => Input::Key(Key::new(KeyCode::Down).with_kind(KeyKind::Release)),
                "release-right" => Input::Key(Key::new(KeyCode::Right).with_kind(KeyKind::Release)),
                "ctrl-down" => {
                    Input::Key(Key::new(KeyCode::Down).with_modifiers(Modifiers::CONTROL))
                }
                "wheel-disabled" => {
                    viewport.mouse_wheel_enabled = false;
                    wheel(ScrollDirection::Down, Modifiers::empty())
                }
                "wheel-zero-delta" => {
                    viewport.mouse_wheel_delta = 0;
                    wheel(ScrollDirection::Down, Modifiers::empty())
                }
                "key-zero-step" => {
                    viewport.set_horizontal_step(0);
                    key(KeyCode::Right)
                }
                "custom-down" | "custom-disabled" => {
                    let mut binding = Binding::new([chord], "alt+F9", "down");
                    binding.enabled = name == "custom-down";
                    viewport.keys.set(ViewportKey::Down, binding);
                    Input::Key(chord)
                }
                "custom-empty" => {
                    viewport
                        .keys
                        .set(ViewportKey::Down, Binding::new([], "empty", "none"));
                    key(KeyCode::Down)
                }
                "custom-priority" => {
                    viewport
                        .keys
                        .set(ViewportKey::PageDown, Binding::new([chord], "F9", "page"));
                    viewport
                        .keys
                        .set(ViewportKey::Up, Binding::new([chord], "F9", "up"));
                    Input::Key(chord)
                }
                "ctrl-wheel-policy" => wheel(ScrollDirection::Down, Modifiers::CONTROL),
                _ => panic!("unknown source input {name}"),
            }
        };
        viewport.event(&input, true);
        let actual = format!(
            "{name}\t{}\t{}",
            viewport.horizontal_offset(),
            viewport.offset()
        );
        if name == "ctrl-wheel-policy" {
            assert_eq!(row, "ctrl-wheel-policy\t8\t11");
            assert_eq!(actual, "ctrl-wheel-policy\t8\t8");
            policy_difference += 1;
        } else {
            assert_eq!(actual, row);
            exact += 1;
        }
    }
    assert_eq!((exact, policy_difference), (33, 1));
}

#[test]
fn horizontal_positions_resize_replace_and_clone_keep_owned_geometry() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Viewport>();
    let mut viewport = Viewport::new("0123456789\n茶ab");
    assert_eq!((viewport.width(), viewport.horizontal_step()), (None, 6));
    assert!(!viewport.scroll_right(usize::MAX));
    assert_eq!(viewport.horizontal_scroll_fraction(), 1.0);
    viewport.set_size(3, 1);
    assert_eq!(viewport.longest_line_width(), 10);
    assert_eq!(viewport.max_horizontal_offset(), 7);
    assert!(viewport.scroll_right(usize::MAX));
    assert_eq!(viewport.horizontal_offset(), 7);
    assert!(!viewport.scroll_right(usize::MAX));
    assert!(viewport.scroll_left(1));
    assert_eq!(viewport.horizontal_scroll_fraction(), 6.0 / 7.0);
    let mut clone = viewport.clone();
    viewport.set_content("茶a");
    assert_eq!(viewport.longest_line_width(), 3);
    assert_eq!(viewport.horizontal_offset(), 0);
    assert_eq!(clone.longest_line_width(), 10);
    assert_eq!(clone.horizontal_offset(), 6);
    clone.set_width(9);
    assert_eq!(clone.horizontal_offset(), 1);
    clone.set_width(0);
    assert_eq!(clone.max_horizontal_offset(), 10);
    assert!(clone.scroll_right(usize::MAX));
    assert_eq!(clone.horizontal_offset(), 10);
    assert!(clone.visible_lines().all(|line| line.is_empty()));
    clone.set_content_owned("short".to_owned());
    assert_eq!(clone.horizontal_offset(), 5);
    clone.clear_width();
    assert_eq!(clone.width(), None);
    assert_eq!(clone.horizontal_offset(), 0);
    assert_eq!(clone.visible_lines().next().as_deref(), Some("short"));
    clone.set_content_lines(["界界", "a"]);
    assert_eq!(clone.longest_line_width(), 4);
    clone.set_content("");
    assert_eq!(clone.longest_line_width(), 0);
    assert_eq!(clone.horizontal_scroll_fraction(), 1.0);
}

#[test]
fn horizontal_controls_match_an_independent_bounded_position_model() {
    for width in [0, 1, 3, 10, 15] {
        let mut viewport = Viewport::new("0123456789\nab");
        viewport.set_width(width);
        let maximum = 10usize.saturating_sub(usize::from(width));
        let mut position = 0usize;
        for count in [0, 1, 7, usize::MAX, 0, usize::MAX, 3, 1] {
            let before = position;
            position = (position as u128 + count as u128).min(maximum as u128) as usize;
            assert_eq!(viewport.scroll_right(count), position != before);
            assert_eq!(viewport.horizontal_offset(), position);
            let before = position;
            position = (position as i128 - count as i128).max(0) as usize;
            assert_eq!(viewport.scroll_left(count), position != before);
            assert_eq!(viewport.horizontal_offset(), position);
            let expected = if maximum == 0 {
                1.0
            } else {
                position as f64 / maximum as f64
            };
            assert_eq!(viewport.horizontal_scroll_fraction(), expected);
        }
    }
}

#[test]
fn horizontal_navigation_obeys_focus_step_and_key_lifecycle() {
    let mut viewport = Viewport::new("0123456789abcdef");
    assert!(!viewport.event(&key(KeyCode::Right), true).consumed);
    viewport.set_size(3, 1);
    assert!(!viewport.event(&key(KeyCode::Right), false).consumed);
    for event in [
        Input::Key(Key::new(KeyCode::Right).with_kind(KeyKind::Release)),
        Input::Key(Key::new(KeyCode::Right).with_modifiers(Modifiers::CONTROL)),
        Input::Key(Key::new(KeyCode::Left).with_modifiers(Modifiers::ALT)),
        Input::Paste("right".to_owned()),
    ] {
        assert!(!viewport.event(&event, true).consumed);
        assert_eq!(viewport.horizontal_offset(), 0);
    }
    assert_eq!(
        viewport.event(&key(KeyCode::Right), true).message,
        Some(Action::Changed)
    );
    assert_eq!(viewport.horizontal_offset(), 6);
    viewport.set_horizontal_step(0);
    assert!(!viewport.event(&key(KeyCode::Left), true).consumed);
    assert!(viewport.scroll_left(2));
    assert_eq!(viewport.horizontal_offset(), 4);
    viewport.set_horizontal_step(usize::MAX);
    assert!(
        viewport
            .event(
                &Input::Key(Key::new(KeyCode::Right).with_kind(KeyKind::Repeat)),
                true
            )
            .consumed
    );
    assert_eq!(viewport.horizontal_offset(), 13);
    assert!(viewport.event(&key(KeyCode::Left), true).consumed);
    assert_eq!(viewport.horizontal_offset(), 0);
}

#[test]
fn ensure_visible_validates_before_mutation_and_handles_extreme_regions() {
    let mut viewport = Viewport::new("0123456789abcdef\nb\nc\nd");
    viewport.set_size(4, 2);
    viewport.set_horizontal_step(2);
    assert_eq!(viewport.ensure_visible(3, 9..11), Ok(true));
    assert_eq!((viewport.offset(), viewport.horizontal_offset()), (2, 7));
    let error = viewport
        .ensure_visible(0, std::ops::Range { start: 5, end: 4 })
        .unwrap_err();
    assert_eq!((error.start, error.end), (5, 4));
    assert_eq!(error.to_string(), "viewport column range 5..4 is reversed");
    assert_eq!((viewport.offset(), viewport.horizontal_offset()), (2, 7));
    assert_eq!(viewport.ensure_visible(2, 0..4), Ok(true));
    assert_eq!((viewport.offset(), viewport.horizontal_offset()), (2, 0));
    assert_eq!(viewport.ensure_visible(2, 0..4), Ok(false));
    assert_eq!(
        viewport.ensure_visible(usize::MAX, usize::MAX..usize::MAX),
        Ok(true)
    );
    assert_eq!((viewport.offset(), viewport.horizontal_offset()), (2, 12));
    viewport.set_size(0, 0);
    assert_eq!(
        viewport.ensure_visible(usize::MAX, usize::MAX..usize::MAX),
        Ok(true)
    );
    assert_eq!((viewport.offset(), viewport.horizontal_offset()), (4, 16));
    assert!(viewport.visible_lines().next().is_none());
}

#[test]
fn visible_windows_preserve_independently_specified_grapheme_cell_coordinates() {
    // Explicit clusters/widths are independent of the production segmentation
    // and width functions. Zero-width clusters never occupy a surface cell.
    let inputs: &[&[(&str, usize)]] = &[
        &[],
        &[("A", 1), ("B", 1), ("C", 1), ("D", 1)],
        &[("茶", 2), ("a", 1), ("b", 1), ("界", 2), ("c", 1)],
        &[("A", 1), ("e\u{301}", 1), ("👩‍💻", 2), ("B", 1)],
        &[("\u{301}", 0), ("A", 1), ("\u{200b}", 0), ("B", 1)],
        &[("🇯🇵", 2), ("1️⃣", 2), ("Z", 1)],
    ];
    for clusters in inputs {
        let line: String = clusters.iter().map(|(text, _)| *text).collect();
        // This longer second line permits every tested offset, even when the
        // first line ends before the left boundary.
        let mut viewport = Viewport::new(&format!("{line}\n{}", "X".repeat(20)));
        viewport.set_height(1);
        for width in 0..9 {
            viewport.set_width(width);
            for left in 0..10usize {
                viewport.set_horizontal_offset(left);
                assert_eq!(viewport.horizontal_offset(), left);
                let mut expected = String::new();
                let mut column = 0;
                for (grapheme, cells) in *clusters {
                    let end = column + cells;
                    if *cells != 0 {
                        for cell in column..end {
                            if cell >= left && cell < left + usize::from(width) {
                                if column >= left && end <= left + usize::from(width) {
                                    if cell == column {
                                        expected.push_str(grapheme);
                                    }
                                } else {
                                    expected.push(' ');
                                }
                            }
                        }
                    }
                    column = end;
                }
                let mut lines = viewport.visible_lines();
                assert_eq!(lines.len(), 1);
                let actual = lines.next_back().unwrap();
                assert!(lines.next().is_none());
                if line == "\u{301}A\u{200b}B" && left == 0 && width >= 2 {
                    // Borrowed text keeps an internal zero-width byte sequence;
                    // the compositor separately skips its zero-cell cluster.
                    assert_eq!(actual, "A\u{200b}B");
                } else {
                    assert_eq!(
                        actual.trim_end(),
                        expected.trim_end(),
                        "line={line:?} left={left} width={width}"
                    );
                }
                let frame = viewport
                    .view(&Theme::plain(), true)
                    .render(Size::new(width, 1))
                    .unwrap();
                assert_eq!(
                    frame.plain(),
                    expected.trim_end(),
                    "render line={line:?} left={left} width={width}"
                );
            }
        }
    }
}

#[test]
fn visible_lines_borrow_aligned_slices_and_owned_views_keep_their_snapshot() {
    use std::borrow::Cow;
    let mut viewport = Viewport::new("abcd\n茶ab界c");
    viewport.set_size(3, 2);
    viewport.set_horizontal_offset(1);
    let mut lines = viewport.visible_lines();
    assert!(matches!(lines.next(), Some(Cow::Borrowed("bcd"))));
    assert!(matches!(lines.next(), Some(Cow::Owned(text)) if text==" ab"));
    drop(lines);
    let previous = viewport.view(&Theme::plain(), true);
    viewport.set_horizontal_offset(2);
    assert!(
        viewport
            .visible_lines()
            .all(|line| matches!(line, Cow::Borrowed(_)))
    );
    viewport.set_content("replacement");
    assert_eq!(
        previous.render(Size::new(3, 2)).unwrap().plain(),
        "bcd\n ab"
    );
    assert_eq!(viewport.visible_text(), "replacement");
}

#[test]
fn horizontal_public_controls_match_all_pinned_observations() {
    let mut fixture = include_str!("fixtures/viewport-horizontal.tsv").lines();
    let mut cases = 0;
    for (index, text) in [
        "",
        "tea",
        "0123456789\nab\n",
        "茶ab茶c\n甲乙丙丁",
        "Ae\u{301}👩‍💻B\né茶",
        "\u{301}A\u{200b}BC",
    ]
    .into_iter()
    .enumerate()
    {
        for width in [0, 1, 2, 3, 6, 20] {
            for height in [0, 1, 2] {
                let mut viewport = Viewport::new(text);
                viewport.set_size(width, height);
                for step in 0..13 {
                    match step {
                        1 => {
                            viewport.set_horizontal_offset(isize::MAX as usize);
                        }
                        2 => {
                            viewport.scroll_left(1);
                        }
                        3 => {
                            viewport.scroll_right(1);
                        }
                        4 => {
                            viewport.set_horizontal_offset(0);
                        }
                        5 => {
                            viewport.scroll_right(2);
                        }
                        6 => {
                            viewport.set_horizontal_step(0);
                            viewport.scroll_right(1);
                        }
                        7 => {
                            viewport.set_horizontal_step(2);
                            viewport.ensure_visible(1, 5..8).unwrap();
                        }
                        8 => {
                            viewport.ensure_visible(0, 0..1).unwrap();
                        }
                        9 => {
                            viewport.scroll_left(isize::MAX as usize);
                        }
                        10 => {
                            viewport.scroll_right(isize::MAX as usize);
                        }
                        11 => {
                            viewport.set_offset(isize::MAX as usize);
                        }
                        12 => {
                            viewport.ensure_visible(0, 1..2).unwrap();
                        }
                        _ => {}
                    }
                    let actual = format!(
                        "{index}\t{width}\t{height}\t{step}\t{}\t{}\t{:016x}",
                        viewport.horizontal_offset(),
                        viewport.offset(),
                        viewport.horizontal_scroll_fraction().to_bits()
                    );
                    assert_eq!(actual, fixture.next().unwrap());
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 1404);
    assert!(fixture.next().is_none());
}

#[test]
fn pinned_cut_outputs_match_or_have_literal_reviewed_cell_policy_deviations() {
    use std::collections::BTreeMap;
    fn unhex(value: &str) -> String {
        assert_eq!(value.len() % 2, 0);
        let bytes = (0..value.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&value[index..index + 2], 16).unwrap())
            .collect::<Vec<_>>();
        String::from_utf8(bytes).unwrap()
    }
    let mut deviations = BTreeMap::new();
    for row in include_str!("fixtures/viewport-cut-deviations.tsv").lines() {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 5);
        assert!(
            deviations
                .insert(fields[..3].join("\t"), (fields[3], unhex(fields[4])))
                .is_none()
        );
    }
    assert_eq!(deviations.len(), 39);
    let mut fixture = include_str!("fixtures/viewport-cuts.tsv").lines();
    let mut exact = 0;
    let mut different = 0;
    for (index, text) in [
        "ABCD",
        "茶ab界c",
        "Ae\u{301}👩‍💻B",
        "\u{301}A\u{200b}BC",
        "🇯🇵1️⃣Z",
    ]
    .into_iter()
    .enumerate()
    {
        let mut viewport = Viewport::new(&format!("{text}\n{}", "X".repeat(20)));
        viewport.set_height(1);
        for width in [1, 2, 3, 4, 6] {
            viewport.set_width(width);
            for left in 0..8 {
                viewport.set_horizontal_offset(left);
                let row = fixture.next().unwrap();
                let fields: Vec<_> = row.split('\t').collect();
                assert_eq!(fields.len(), 4);
                let key = format!("{index}\t{width}\t{left}");
                assert_eq!(fields[..3].join("\t"), key);
                let raw_source = unhex(fields[3]);
                // Surface::plain trims padding on each row. Preserve extra
                // rows and all zero-width/emoji bytes in the source comparison.
                let source = raw_source
                    .split('\n')
                    .map(|line| line.trim_end_matches(' '))
                    .collect::<Vec<_>>()
                    .join("\n");
                let actual = viewport
                    .view(&Theme::plain(), true)
                    .render(Size::new(width, 1))
                    .unwrap()
                    .plain();
                if let Some((source_hex, expected)) = deviations.remove(&key) {
                    assert_eq!(fields[3], source_hex);
                    assert_eq!(actual, expected, "literal cell window {key}");
                    assert_ne!(source, expected);
                    different += 1;
                } else {
                    assert_eq!(source, actual, "unreviewed difference {key}");
                    exact += 1;
                }
            }
        }
    }
    assert_eq!((exact, different), (161, 39));
    assert!(fixture.next().is_none() && deviations.is_empty());
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

#[test]
fn source_resize_replace_overflow_edges_have_explicit_bounded_rust_outcomes() {
    let mut exact_nonnegative = 0;
    let mut signed_only = 0;
    let mut improved = 0;
    for row in include_str!("fixtures/viewport-geometry-edges.tsv").lines() {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 9);
        assert!(fields[8].is_empty(), "source panic in {}", fields[0]);
        let source = String::from_utf8(
            (0..fields[7].len())
                .step_by(2)
                .map(|index| u8::from_str_radix(&fields[7][index..index + 2], 16).unwrap())
                .collect(),
        )
        .unwrap();
        for bits in &fields[5..7] {
            assert!(f64::from_bits(u64::from_str_radix(bits, 16).unwrap()).is_finite());
        }
        let mut viewport = Viewport::new("0123456789");
        viewport.set_size(3, 1);
        viewport.set_horizontal_offset(7);
        let expected_plain = match fields[0] {
            "width-grow" => {
                assert_eq!(
                    (&fields[1..7], source.as_str()),
                    (
                        &["20", "1", "7", "0", "3ff0000000000000", "3ff0000000000000"][..],
                        "789                 "
                    )
                );
                viewport.set_width(20);
                assert_eq!(viewport.horizontal_offset(), 0);
                improved += 1;
                "0123456789".to_owned()
            }
            "replace-short" => {
                assert_eq!(
                    (&fields[1..7], source.as_str()),
                    (
                        &["3", "1", "7", "0", "3ff0000000000000", "3ff0000000000000"][..],
                        "   "
                    )
                );
                viewport.set_content("tea");
                assert_eq!(viewport.horizontal_offset(), 0);
                improved += 1;
                "tea".to_owned()
            }
            "positive-overflow" => {
                assert_eq!(
                    (&fields[1..7], source.as_str()),
                    (
                        &["3", "1", "0", "0", "3ff0000000000000", "0000000000000000"][..],
                        "012"
                    )
                );
                viewport.set_horizontal_offset(1);
                viewport.scroll_right(isize::MAX as usize);
                assert_eq!(viewport.horizontal_offset(), 7);
                improved += 1;
                "789".to_owned()
            }
            "height--2" | "height--1" => {
                // The reported older signed-height panic is absent in this
                // exact pinned sequence. Negative sizes cannot enter u16 APIs.
                assert_eq!(source, "   ");
                assert_eq!(fields[4], if fields[0] == "height--2" { "6" } else { "5" });
                signed_only += 1;
                continue;
            }
            name if name.starts_with("height-") => {
                let height = fields[2].parse::<u16>().unwrap();
                viewport.set_horizontal_offset(0);
                viewport.set_content("a\nb\nc\nd");
                viewport.go_to_bottom();
                viewport.set_height(height);
                viewport.go_to_bottom();
                let state = format!(
                    "{}\t{}\t{}\t{}\t{:016x}\t{:016x}",
                    viewport.width().unwrap(),
                    viewport.height(),
                    viewport.horizontal_offset(),
                    viewport.offset(),
                    viewport.scroll_fraction().to_bits(),
                    viewport.horizontal_scroll_fraction().to_bits()
                );
                assert_eq!(state, fields[1..7].join("\t"));
                exact_nonnegative += 1;
                source
                    .split('\n')
                    .map(|line| line.trim_end_matches(' '))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            name => panic!("unknown source edge {name}"),
        };
        let actual = viewport
            .view(&Theme::plain(), true)
            .render(Size::new(viewport.width().unwrap(), viewport.height()))
            .unwrap()
            .plain();
        assert_eq!(actual, expected_plain, "{}", fields[0]);
    }
    assert_eq!((exact_nonnegative, signed_only, improved), (5, 2, 3));
}

#[test]
fn longest_width_handles_repeated_varied_ascii_indexes_and_replacement() {
    let mut viewport = Viewport::new("");
    assert_eq!(viewport.longest_line_width(), 0);
    for empty in ["\n", "\n\n", "\n\n\n"] {
        viewport.set_content(empty);
        assert_eq!(viewport.longest_line_width(), 0);
    }
    let ascii = [
        "".to_owned(),
        "a".to_owned(),
        "X".repeat(1024),
        "tail".to_owned(),
        "".to_owned(),
    ]
    .join("\n");
    viewport.set_content(&ascii);
    assert_eq!(viewport.longest_line_width(), 1024);
    let repeated = std::iter::repeat_n("茶e\u{301}👩‍💻", 100)
        .collect::<Vec<_>>()
        .join("\n");
    viewport.set_content(&repeated);
    assert_eq!(viewport.longest_line_width(), 5);
    let mut expected = 0;
    let varied = (0..97usize)
        .map(|index| {
            let digits = index.to_string();
            expected = expected.max(2 * (index % 13) + 1 + digits.len());
            format!("{}e\u{301}{digits}", "茶".repeat(index % 13))
        })
        .collect::<Vec<_>>()
        .join("\n");
    viewport.set_content(&varied);
    assert_eq!(viewport.longest_line_width(), expected);
    assert_eq!(expected, 27);
    viewport.set_content(&format!("{repeated}\n{ascii}\n{varied}\n{repeated}"));
    assert_eq!(viewport.longest_line_width(), 1024);
    viewport.set_content("X\n");
    assert_eq!(viewport.longest_line_width(), 1);
}
