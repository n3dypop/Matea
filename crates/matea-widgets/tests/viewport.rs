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
