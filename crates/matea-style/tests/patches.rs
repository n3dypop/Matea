use matea_style::{
    BlinkStyle, Color, Style, StylePatch, TextAttribute, TextAttributes, TextFrame, UnderlineStyle,
};

fn base() -> Style {
    Style::default()
        .fg(Color::Indexed(2))
        .bg(Color::Rgb(10, 20, 30))
        .underline_color(Color::Indexed(4))
        .bold()
        .italic()
        .underline_style(UnderlineStyle::Curly)
        .blink(BlinkStyle::Rapid)
        .text_frame(TextFrame::Framed)
}

#[test]
fn inheritance_false_and_terminal_default_are_distinct_intents() {
    let base = base();
    let empty = StylePatch::new();
    assert!(empty.is_empty());
    assert_eq!(empty.resolve(base), base);
    assert_eq!(empty.attribute_intent(TextAttribute::Bold), None);
    let patch = empty
        .fg(Color::Default)
        .attribute(TextAttribute::Bold, false)
        .underline_style(UnderlineStyle::None)
        .blink(BlinkStyle::None)
        .text_frame(TextFrame::None);
    assert!(!patch.is_empty());
    assert_eq!(patch.attribute_intent(TextAttribute::Bold), Some(false));
    assert_eq!(patch.underline(), Some(UnderlineStyle::None));
    assert_eq!(patch.blink_style(), Some(BlinkStyle::None));
    assert_eq!(patch.frame(), Some(TextFrame::None));
    assert_eq!(
        patch.resolve(base),
        base.fg(Color::Default)
            .attribute(TextAttribute::Bold, false)
            .underline_style(UnderlineStyle::None)
            .blink(BlinkStyle::None)
            .text_frame(TextFrame::None)
    );
    assert_eq!(
        patch
            .inherit_fg()
            .inherit_attribute(TextAttribute::Bold)
            .inherit_underline()
            .inherit_blink()
            .inherit_frame(),
        empty
    );
}

#[test]
fn reset_and_complete_override_preserve_no_implicit_inheritance() {
    assert_eq!(StylePatch::reset().resolve(base()), Style::default());
    let other = Style::default().bg(Color::Indexed(9)).overline();
    assert_eq!(StylePatch::from_style(base()).resolve(other), base());
    assert_eq!(StylePatch::from(base()), StylePatch::from_style(base()));
    assert_eq!(
        StylePatch::from(Color::Indexed(7)).resolve(base()),
        base().fg(Color::Indexed(7))
    );
    let inherit_bg = StylePatch::from_style(base()).inherit_bg();
    assert_eq!(inherit_bg.resolve(other).bg, other.bg);
    assert_eq!(inherit_bg.resolve(other).fg, base().fg);
    let mut public_colors = StylePatch::reset();
    public_colors.fg = None;
    public_colors.bg = Some(Color::Rgb(3, 2, 1));
    assert_eq!(public_colors.resolve(base()).fg, base().fg);
    assert_eq!(public_colors.resolve(base()).bg, Color::Rgb(3, 2, 1));
    assert_eq!(
        StylePatch::new()
            .underline_color(Color::Indexed(1))
            .inherit_underline_color()
            .resolve(base()),
        base()
    );
}

#[test]
fn each_boolean_composes_all_inherit_set_clear_sequences() {
    for flag in TextAttribute::ALL {
        for enabled in [false, true] {
            let base = base().attribute(flag, enabled);
            for a in [None, Some(false), Some(true)] {
                for b in [None, Some(false), Some(true)] {
                    for c in [None, Some(false), Some(true)] {
                        let make = |value| match value {
                            None => StylePatch::new(),
                            Some(v) => StylePatch::new().attribute(flag, v),
                        };
                        let combined = make(a).then(make(b)).then(make(c));
                        let expected = c.or(b).or(a).unwrap_or(enabled);
                        assert_eq!(combined.resolve(base), base.attribute(flag, expected));
                        assert_eq!(combined.attribute_intent(flag), c.or(b).or(a));
                    }
                }
            }
        }
    }
}

#[test]
fn enum_groups_never_leak_into_other_attributes_or_each_other() {
    let base = base();
    for current in UnderlineStyle::ALL {
        for left in [None].into_iter().chain(UnderlineStyle::ALL.map(Some)) {
            for right in [None].into_iter().chain(UnderlineStyle::ALL.map(Some)) {
                let make = |choice| match choice {
                    None => StylePatch::new(),
                    Some(v) => StylePatch::new().underline_style(v),
                };
                let value = make(left).then(make(right));
                assert_eq!(
                    value.resolve(base.underline_style(current)),
                    base.underline_style(right.or(left).unwrap_or(current))
                );
                assert_eq!(value.underline(), right.or(left));
                assert_eq!(value.inherit_underline(), StylePatch::new());
            }
        }
    }
    for current in BlinkStyle::ALL {
        for left in [None].into_iter().chain(BlinkStyle::ALL.map(Some)) {
            for right in [None].into_iter().chain(BlinkStyle::ALL.map(Some)) {
                let make = |choice| match choice {
                    None => StylePatch::new(),
                    Some(v) => StylePatch::new().blink(v),
                };
                let value = make(left).then(make(right));
                assert_eq!(
                    value.resolve(base.blink(current)),
                    base.blink(right.or(left).unwrap_or(current))
                );
                assert_eq!(value.blink_style(), right.or(left));
            }
        }
    }
    for current in TextFrame::ALL {
        for left in [None].into_iter().chain(TextFrame::ALL.map(Some)) {
            for right in [None].into_iter().chain(TextFrame::ALL.map(Some)) {
                let make = |choice| match choice {
                    None => StylePatch::new(),
                    Some(v) => StylePatch::new().text_frame(v),
                };
                let value = make(left).then(make(right));
                assert_eq!(
                    value.resolve(base.text_frame(current)),
                    base.text_frame(right.or(left).unwrap_or(current))
                );
                assert_eq!(value.frame(), right.or(left));
            }
        }
    }
}

#[test]
fn composition_is_associative_and_equivalent_to_sequential_resolution() {
    let styles = [
        Style::default(),
        base(),
        Style::default().dim().hidden().overline().fraktur(),
    ];
    let patches = [
        StylePatch::new(),
        StylePatch::reset(),
        StylePatch::from_style(base()),
        StylePatch::new()
            .fg(Color::Default)
            .attribute(TextAttribute::Bold, false),
        StylePatch::new()
            .bg(Color::Rgb(40, 50, 60))
            .underline_style(UnderlineStyle::Double),
        StylePatch::new()
            .blink(BlinkStyle::Slow)
            .text_frame(TextFrame::Encircled),
        StylePatch::new()
            .underline_color(Color::Indexed(255))
            .attribute(TextAttribute::Overline, true),
    ];
    for base in styles {
        for a in patches {
            for b in patches {
                for c in patches {
                    assert_eq!(a.then(b).then(c), a.then(b.then(c)));
                    assert_eq!(
                        a.then(b).then(c).resolve(base),
                        c.resolve(b.resolve(a.resolve(base)))
                    );
                    assert_eq!(a.then(StylePatch::new()), a);
                }
            }
        }
    }
}

#[test]
fn compact_copy_values_work_in_const_contexts_and_across_threads() {
    const BASE: Style = Style {
        fg: Color::Indexed(2),
        bg: Color::Default,
        underline_color: Color::Default,
        attributes: TextAttributes::new().with(TextAttribute::Bold),
    };
    const PATCH: StylePatch = StylePatch::new()
        .fg(Color::Indexed(1))
        .attribute(TextAttribute::Bold, false)
        .then(StylePatch::new().underline_style(UnderlineStyle::Dashed));
    const RESOLVED: Style = BASE.patched(PATCH);
    assert_eq!(
        RESOLVED,
        BASE.fg(Color::Indexed(1))
            .attribute(TextAttribute::Bold, false)
            .underline_style(UnderlineStyle::Dashed)
    );
    assert_eq!(std::mem::size_of::<Style>(), 14);
    assert_eq!(std::mem::size_of::<StylePatch>(), 16);
    fn send_sync<T: Send + Sync>() {}
    send_sync::<StylePatch>();
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let copy = PATCH;
            scope.spawn(move || assert_eq!(copy.resolve(BASE), RESOLVED));
        }
    });
    let debug = format!("{PATCH:?}");
    assert!(debug.contains("bold: false") && debug.contains("Dashed"));
    assert!(!debug.contains("mask") && !debug.contains("values"));
}

#[test]
#[ignore = "finite full flag domain, explicitly executed by native acceptance"]
fn every_boolean_base_and_ternary_patch_matches_a_branching_value_model() {
    for bits in 0u16..256 {
        let mut base = base();
        for (i, flag) in TextAttribute::ALL.into_iter().enumerate() {
            base = base.attribute(flag, bits & (1 << i) != 0);
        }
        for mut choices in 0u32..6561 {
            let mut expected = base;
            let mut patch = StylePatch::new();
            for flag in TextAttribute::ALL {
                match choices % 3 {
                    0 => {}
                    1 => {
                        expected = expected.attribute(flag, false);
                        patch = patch.attribute(flag, false);
                    }
                    _ => {
                        expected = expected.attribute(flag, true);
                        patch = patch.attribute(flag, true);
                    }
                }
                choices /= 3;
            }
            assert_eq!(patch.resolve(base), expected);
        }
    }
}
