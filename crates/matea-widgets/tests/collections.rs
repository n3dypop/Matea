//! Exercise collection/form contracts through public component APIs.
use matea_core::{Component, Input, Key, KeyCode, KeyKind, Modifiers};
use matea_render::Size;
use matea_style::Theme;
use matea_widgets::{Action, Field, Form, List, Table, required};
fn key(code: KeyCode) -> Input {
    Input::Key(Key::new(code))
}

#[test]
fn caught_filter_panics_leave_a_valid_visible_prefix_and_selection() {
    for panic_at in 0..4 {
        let mut list = List::new(["zero", "one", "two", "three"]);
        list.event(&key(KeyCode::End), true);
        let mut index = 0;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            list.filter_with("", |_, _| {
                let n = index;
                index += 1;
                assert_ne!(n, panic_at, "predicate fixture");
                true
            });
        }));
        assert!(result.is_err());
        assert_eq!(list.selected(), if panic_at == 0 { None } else { Some(0) });
        let view = list
            .view(&Theme::plain(), true)
            .render(Size::new(20, 4))
            .unwrap()
            .plain();
        assert!(view.starts_with(if panic_at == 0 {
            "(no matches)"
        } else {
            "> zero"
        }));
        list.filter("");
        assert_eq!(list.selected(), Some(0));
        list.event(&key(KeyCode::End), true);
        assert_eq!(list.selected(), Some(3));
    }
}

#[test]
fn list_navigation_clamps_and_preserves_original_filtered_indices() {
    for len in 0usize..12 {
        for rows in 0..8 {
            let mut list = List::new((0..len).map(|i| format!("row{i}")));
            list.rows = rows;
            assert_eq!(list.selected(), if len == 0 { None } else { Some(0) });
            for _ in 0..(len + 3) {
                list.event(&key(KeyCode::Down), true);
            }
            assert_eq!(list.selected(), len.checked_sub(1));
            for input in [
                Input::Focus(false),
                Input::Paste("q".into()),
                Input::Key(Key::new(KeyCode::Home).with_kind(KeyKind::Release)),
                Input::Key(Key::new(KeyCode::Home).with_modifiers(Modifiers::CONTROL)),
            ] {
                assert!(!list.event(&input, true).consumed);
                assert_eq!(list.selected(), len.checked_sub(1));
            }
            let selected = list.selected();
            assert!(!list.event(&key(KeyCode::Home), false).consumed);
            assert_eq!(list.selected(), selected);
            for _ in 0..(len + 3) {
                list.event(&key(KeyCode::Up), true);
            }
            assert_eq!(list.selected(), if len == 0 { None } else { Some(0) });
            list.filter_with("", |item, _| {
                item.strip_prefix("row").unwrap().parse::<usize>().unwrap() & 1 == 1
            });
            assert_eq!(list.selected(), if len > 1 { Some(1) } else { None });
            list.event(&key(KeyCode::End), true);
            let expected = (0..len).rev().find(|i| !i.is_multiple_of(2));
            assert_eq!(list.selected(), expected);
            assert_eq!(
                list.event(&key(KeyCode::Enter), true).message,
                expected.map(Action::Selected)
            );
            list.filter("does not exist");
            assert_eq!(list.selected(), None);
            assert!(!list.event(&key(KeyCode::Enter), true).consumed);
        }
    }
}

#[test]
fn table_ragged_rows_empty_headers_and_navigation_are_bounded() {
    for headers in [vec![], vec!["A".into()], vec!["A".into(), "B".into()]] {
        for len in 0usize..10 {
            let rows = (0..len)
                .map(|i| {
                    if i.is_multiple_of(2) {
                        vec![format!("row{i}"), "extra".into(), "ignored".into()]
                    } else {
                        vec![]
                    }
                })
                .collect();
            let mut table = Table::new(headers.clone(), rows);
            assert_eq!(table.selected(), if len == 0 { None } else { Some(0) });
            table.event(&key(KeyCode::End), true);
            assert_eq!(table.selected(), len.checked_sub(1));
            let text = table
                .view(&Theme::plain(), true)
                .render(Size::new(40, 12))
                .unwrap()
                .plain();
            assert!(!text.contains("ignored"));
            assert!(!table.event(&key(KeyCode::Home), false).consumed);
            assert_eq!(
                table.event(&key(KeyCode::Enter), true).message,
                len.checked_sub(1).map(Action::Selected)
            );
            table.event(&key(KeyCode::Home), true);
            assert_eq!(table.selected(), if len == 0 { None } else { Some(0) });
        }
    }
}

#[test]
fn forms_validate_all_fields_focus_first_failure_and_clear_stale_submission() {
    for len in 0..6 {
        let mut form = Form::new(
            (0..len)
                .map(|i| Field::new(format!("field{i}"), required))
                .collect(),
        );
        if len == 0 {
            assert!(!form.event(&key(KeyCode::Enter), true).consumed);
            assert!(!form.submitted());
            continue;
        }
        // Tab can traverse invalid fields, but submission must revisit all fields.
        for _ in 1..len {
            form.event(&key(KeyCode::Tab), true);
        }
        assert_eq!(form.focus_index(), len - 1);
        assert_eq!(form.event(&key(KeyCode::Enter), true).message, None);
        assert_eq!(form.focus_index(), 0);
        assert!(form.fields().iter().all(|f| f.error().is_some()));
        for i in 0..len {
            assert_eq!(form.focus_index(), i);
            form.event(&Input::Paste("tea".into()), true);
            assert!(form.fields()[i].error().is_none());
            let response = form.event(&key(KeyCode::Enter), true);
            assert_eq!(
                response.message,
                if i + 1 == len {
                    Some(Action::Submit)
                } else {
                    None
                }
            );
        }
        assert!(form.submitted());
        assert!(!form.event(&key(KeyCode::Backspace), false).consumed);
        assert!(form.submitted());
        form.event(&key(KeyCode::Backspace), true);
        assert!(!form.submitted());
        form.event(&key(KeyCode::BackTab), true);
        assert_eq!(form.focus_index(), len.saturating_sub(2));
    }
}
