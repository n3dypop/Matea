use compact_str::CompactString;
use matea_render::{Size, Surface, Text, View};
use matea_style::Theme;
use std::{borrow::Cow, cell::Cell, fmt, rc::Rc};

#[test]
fn standard_inputs_normalize_identically_and_own_their_snapshot() {
    let raw = "抹茶\t\u{1b}[31m\r\n\0e\u{301}\u{85}";
    let expected = "抹茶    �[31m�\n�e\u{301}�";
    let string = raw.to_owned();
    let mut mutable = raw.to_owned();
    let values = [
        Text::from(raw),
        Text::from(string.clone()),
        Text::from(&string),
        Text::from(Cow::Borrowed(raw)),
        Text::from(Cow::Owned(raw.to_owned())),
        Text::from(raw.to_owned().into_boxed_str()),
        Text::from(CompactString::from(raw)),
        Text::from(&CompactString::from(raw)),
        Text::from(&mut mutable),
        Text::from(mutable.as_mut_str()),
        Text::from(format_args!("{raw}")),
    ];
    mutable.clear();
    for text in values {
        assert_eq!(text.as_str(), expected);
        assert_eq!(text.as_ref(), expected);
        assert_eq!(&*text, expected);
        assert_eq!(String::from(text.clone()), expected);
        assert_eq!(Text::from(&text), text);
    }
    assert_eq!(Text::from('\t').as_str(), "    ");
    assert_eq!(Text::from('\n').as_str(), "\n");
    assert_eq!(Text::default().as_str(), "");
    for byte in (0..32).chain(127..160) {
        let c = char::from_u32(byte).unwrap();
        let expected = match c {
            '\n' => "\n",
            '\t' => "    ",
            _ => "�",
        };
        assert_eq!(Text::from(c).as_str(), expected);
    }
}

struct Counted {
    calls: Rc<Cell<usize>>,
    value: String,
}
impl fmt::Display for Counted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.calls.set(self.calls.get() + 1);
        f.write_str(&self.value)
    }
}
#[test]
fn arguments_format_once_and_the_scene_retains_no_ui_borrow() {
    let calls = Rc::new(Cell::new(0));
    let mut value = Counted {
        calls: calls.clone(),
        value: "抹茶".to_owned(),
    };
    let text = Text::from(format_args!("{value} / {}", 42));
    value.value.clear();
    drop(value);
    assert_eq!(calls.get(), 1);
    assert_eq!(text.as_str(), "抹茶 / 42");
    assert_eq!(format!("{:>8.2}", text), format!("{:>8.2}", "抹茶 / 42"));
    let view = View::text(&text);
    for _ in 0..2 {
        assert_eq!(
            view.clone().render(Size::new(24, 1)).unwrap().plain(),
            text.as_str()
        );
    }
    assert_eq!(calls.get(), 1);
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Text>();
}

#[test]
fn typed_views_and_raw_canvas_preserve_cells_styles_and_clipping_at_every_size() {
    for theme in [Theme::default(), Theme::plain()] {
        for focused in [false, true] {
            let style = if focused { theme.focus } else { theme.text };
            for raw in ["抹茶 e\u{301} 👩‍💻", "a\tb\u{1b}[31m\n\0抹茶", "x\n\nend\n"] {
                for size in [
                    Size::new(0, 0),
                    Size::new(1, 1),
                    Size::new(5, 2),
                    Size::new(72, 4),
                ] {
                    let mut expected = Surface::new(size).unwrap();
                    expected.canvas(expected.area()).text(0, 0, raw, style);
                    let actual = View::styled(format_args!("{raw}"), style)
                        .render(size)
                        .unwrap();
                    assert_eq!(actual, expected);
                }
            }
        }
    }
    for n in 0..=40 {
        let raw = "抹".repeat(n);
        assert_eq!(Text::from(raw.clone()).into_string(), raw);
    }
}

struct Bad;
impl fmt::Display for Bad {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Err(fmt::Error)
    }
}
#[test]
fn failed_display_panics_instead_of_returning_partial_text() {
    assert!(std::panic::catch_unwind(|| Text::from(format_args!("prefix {Bad}"))).is_err());
}

#[test]
fn byte_scan_preserves_standard_control_policy_for_every_unicode_scalar() {
    for codepoint in 0..=0x10ffff {
        let Some(c) = char::from_u32(codepoint) else {
            continue;
        };
        let mut utf8 = [0u8; 4];
        let expected = match c {
            '\n' => "\n",
            '\t' => "    ",
            c if c.is_control() => "�",
            c => c.encode_utf8(&mut utf8),
        };
        assert_eq!(Text::from(c).as_str(), expected, "U+{codepoint:04X}");
    }
}

#[test]
fn c2_candidates_in_mixed_utf8_keep_controls_and_valid_latin_distinct() {
    for value in 0x80..=0xbf {
        let candidate = char::from_u32(value).unwrap();
        for before in ["", "ASCII\n", "抹茶👩‍💻", "\u{85}\t"] {
            for after in ["", "ASCII", "抹茶👩‍💻", "\u{a0}\u{9f}"] {
                let raw = format!("{before}{candidate}{after}");
                let expected: String = raw
                    .chars()
                    .flat_map(|c| match c {
                        '\n' => "\n".chars().collect::<Vec<_>>(),
                        '\t' => "    ".chars().collect(),
                        c if c.is_control() => vec!['�'],
                        c => vec![c],
                    })
                    .collect();
                assert_eq!(matea_render::sanitize(&raw), expected);
                assert_eq!(matea_render::sanitized(&raw), expected);
                assert_eq!(Text::from(raw.clone()).as_str(), expected);
                assert_eq!(Text::from(CompactString::from(raw)).as_str(), expected);
            }
        }
    }
}
