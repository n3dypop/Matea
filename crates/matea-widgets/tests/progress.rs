use matea_core::{Component, Input, Key, KeyCode};
use matea_motion::Motion;
use matea_render::Size;
use matea_style::{Color, Gradient, Style, Theme};
use matea_widgets::{Progress, ProgressError, ProgressFill};
use std::time::{Duration, Instant};

fn theme() -> Theme {
    Theme {
        reduced_motion: false,
        ..Theme::plain()
    }
}
fn rgb(color: Color) -> String {
    match color {
        Color::Default => "default".into(),
        Color::Rgb(r, g, b) => format!("{r:02x}{g:02x}{b:02x}"),
        Color::Indexed(_) => panic!("unexpected indexed fixture"),
    }
}
fn configured(mode: usize, width: u16) -> Progress {
    let mut p = Progress::new();
    p.set_width(width);
    p.styles.full = Some(Style::default());
    p.styles.empty = Some(Style::default().fg(Color::Rgb(96, 96, 96)));
    p.styles.percentage = Some(Style::default());
    p.fill = match mode {
        0 => ProgressFill::Solid(Color::Rgb(117, 113, 249)),
        1..=3 => ProgressFill::Gradient {
            gradient: Gradient::new(&[[90, 86, 224], [238, 111, 248]]).unwrap(),
            scaled: mode == 3,
        },
        4 | 5 => ProgressFill::ColorFn(Box::new(|total, current| {
            Color::Rgb((total * 255.0) as u8, (current * 255.0) as u8, 77)
        })),
        _ => unreachable!(),
    };
    if mode == 1 {
        p.show_percentage = false;
        p.set_glyphs('█', '░').unwrap();
    }
    if mode == 5 {
        p.set_glyphs('#', '-').unwrap();
    }
    p
}
#[test]
fn all_public_reference_view_fixtures_match_cells_and_rgb8_styles() {
    let mut count = 0;
    for line in include_str!("fixtures/progress.tsv").lines() {
        let f: Vec<_> = line.split('\t').collect();
        let width = f[1].parse().unwrap();
        let p = configured(f[0].parse().unwrap(), width);
        let view = p.view_as(f[2].parse().unwrap(), &theme()).unwrap();
        let surface = view.render(Size::new(width, 1)).unwrap();
        let actual = surface
            .row(0)
            .iter()
            .map(|c| format!("{}:{}:{}", c.symbol(), rgb(c.style().fg), rgb(c.style().bg)))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(actual, f[3], "fixture {line}");
        count += 1;
    }
    assert_eq!(count, 120);
}
#[test]
fn negative_velocity_is_not_rest_and_final_state_snaps_exactly() {
    let origin = Instant::now();
    let mut p = Progress::new_at(origin);
    p.set_spring(18.0, 0.1).unwrap();
    p.set_percent_at(0.5, origin).unwrap();
    let mut crossing = false;
    let t = theme();
    for frame in 1..=1000 {
        let Some(schedule) = p.schedule(&t) else {
            break;
        };
        let at = origin + Duration::from_nanos(16_666_666 * frame);
        assert!(p.update(schedule.tick_at(at)).unwrap());
        let state = p.motion();
        if (state.position - p.target()).abs() < 0.001 && state.velocity < -0.01 {
            crossing = true;
            assert!(
                p.is_animating(),
                "signed-velocity early stop at {frame}: {state:?}"
            );
        }
    }
    assert!(crossing);
    assert!(!p.is_animating());
    assert_eq!(
        p.motion(),
        Motion {
            position: 0.5,
            velocity: 0.0
        }
    );
    assert!(p.schedule(&t).is_none());
}
#[test]
fn retargeting_rejects_stale_foreign_duplicate_and_backward_ticks() {
    let origin = Instant::now();
    let mut p = Progress::new_at(origin);
    p.set_percent_at(0.7, origin).unwrap();
    let first = p.schedule(&theme()).unwrap();
    let later = origin + Duration::from_millis(100);
    let tick = first.tick_at(later);
    assert!(p.update(tick).unwrap());
    let state = p.motion();
    assert!(!p.update(tick).unwrap());
    assert!(!p.update(first.tick_at(origin)).unwrap());
    let mut other = Progress::new_at(origin);
    other.set_percent_at(1.0, origin).unwrap();
    assert_ne!(p.id(), other.id());
    assert!(
        !p.update(other.schedule(&theme()).unwrap().tick_at(later))
            .unwrap()
    );
    assert!(!p.set_percent_at(0.1, origin).unwrap());
    assert_eq!(p.target(), 0.7);
    assert_eq!(p.motion(), state);
    p.set_percent_at(0.3, later).unwrap();
    assert!(
        !p.update(first.tick_at(later + Duration::from_millis(50)))
            .unwrap()
    );
    let current = p.schedule(&theme()).unwrap();
    assert!(
        p.update(current.tick_at(later + Duration::from_millis(50)))
            .unwrap()
    );
    // Coalesced time advances only one fixed step, matching source cadence.
    let mut coalesced = Progress::new_at(origin);
    coalesced.set_percent_at(0.7, origin).unwrap();
    coalesced
        .update(
            coalesced
                .schedule(&theme())
                .unwrap()
                .tick_at(origin + Duration::from_secs(100)),
        )
        .unwrap();
    // Separate spring construction involves transcendental floating-point
    // evaluation. Compare one-step behavior tightly, not identical rounding.
    assert!((coalesced.motion().position - state.position).abs() < 1e-12);
    assert!((coalesced.motion().velocity - state.velocity).abs() < 1e-12);
}
#[test]
fn invalid_input_is_transactional_and_finite_values_clamp() {
    let mut p = Progress::new();
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(p.set_percent(invalid), Err(ProgressError::NonFinitePercent));
        assert!(p.increment(invalid).is_err());
        assert!(p.view_as(invalid, &theme()).is_err());
        assert!(p.set_spring(invalid, 1.0).is_err());
        assert!(p.set_spring(18.0, invalid).is_err());
        assert_eq!(p.target(), 0.0);
        assert_eq!(p.motion(), Motion::default());
    }
    for (frequency, damping) in [(0.0, 1.0), (-1.0, 1.0), (18.0, -1.0)] {
        assert!(p.set_spring(frequency, damping).is_err());
    }
    p.set_spring(18.0, 0.0).unwrap(); // Explicitly undamped, not falsely resting.
    p.set_percent(2.0).unwrap();
    assert_eq!(p.target(), 1.0);
    p.increment(-3.0).unwrap();
    assert_eq!(p.target(), 0.0);
    p.decrement(-0.25).unwrap();
    assert_eq!(p.target(), 0.25);
    for c in ['\0', '\n', '\t', '\u{301}', '茶', '👩'] {
        assert_eq!(p.set_glyphs(c, '.'), Err(ProgressError::InvalidGlyph));
        assert_eq!(p.glyphs(), ('▌', '░'));
    }
    p.set_glyphs('#', '-').unwrap();
}
#[test]
fn cells_are_exact_double_ended_and_exhausted_without_hidden_work() {
    let p = configured(3, 17);
    let frame = p.frame_as(0.5, &theme()).unwrap();
    let expected = frame.cells().collect::<Vec<_>>();
    let mut it = frame.cells();
    assert_eq!(it.len(), 12);
    assert_eq!(it.next(), Some(expected[0]));
    assert_eq!(it.next_back(), Some(expected[11]));
    assert_eq!(it.nth(3), Some(expected[4]));
    assert_eq!(it.nth_back(3), Some(expected[7]));
    assert_eq!(it.len(), 2);
    assert_eq!(it.nth(usize::MAX), None);
    assert_eq!(it.next_back(), None);
    assert_eq!(it.next(), None);
    assert!(frame.cell_at(usize::MAX).is_none());
    let mut zero = Progress::new();
    zero.set_width(0);
    let empty = zero.frame(&theme());
    assert_eq!(empty.cells().next(), None);
    assert_eq!(empty.label(), "");
}
#[test]
fn narrow_formatting_is_single_line_sanitized_and_grapheme_clipped() {
    let mut p = Progress::new();
    p.set_percentage_formatter(|_| "e\u{301}茶👩‍💻\n\x1b\t".into());
    for width in 0..=15 {
        p.set_width(width);
        let frame = p.frame_as(0.5, &theme()).unwrap();
        assert!(!frame.label().contains(['\n', '\r', '\t', '\x1b']));
        assert!(unicode_width::UnicodeWidthStr::width(frame.label()) <= usize::from(width));
        assert_eq!(
            frame
                .view()
                .render(Size::new(width, 1))
                .unwrap()
                .size()
                .width,
            width
        );
    }
    p.set_width(3);
    assert_eq!(p.frame_as(1.0, &theme()).unwrap().label(), "e\u{301}茶");
    p.reset_percentage_formatter();
    p.set_width(40);
    assert_eq!(p.frame_as(0.125, &theme()).unwrap().label(), "  12%");
    // Custom formatting exercises owned heap storage, then UTF-8-safe clipping.
    p.set_percentage_formatter(|_| "茶".repeat(32));
    p.set_width(3);
    let frame = p.frame_as(0.5, &theme()).unwrap();
    assert_eq!(frame.label(), "茶");
    assert_eq!(frame.view().render(Size::new(3, 1)).unwrap().plain(), "▌茶");
}
#[test]
fn reduced_motion_uses_target_suppresses_schedules_and_can_snap_state() {
    let origin = Instant::now();
    let mut p = Progress::new_at(origin);
    p.set_percent_at(0.7, origin).unwrap();
    let old = p.schedule(&theme()).unwrap();
    let reduced = Theme {
        reduced_motion: true,
        ..theme()
    };
    assert!(p.schedule(&reduced).is_none());
    assert_eq!(p.frame(&reduced).label(), "  70%");
    assert_eq!(p.motion().position, 0.0); // Rendering remains immutable.
    assert!(p.set_reduced_motion_at(true, origin));
    assert_eq!(
        p.motion(),
        Motion {
            position: 0.7,
            velocity: 0.0
        }
    );
    assert!(!p.is_animating());
    assert!(
        !p.update(old.tick_at(origin + Duration::from_secs(1)))
            .unwrap()
    );
    p.set_percent_at(0.3, origin).unwrap();
    assert_eq!(p.motion().position, 0.3);
    assert!(!p.set_reduced_motion_at(true, origin));
    assert!(p.set_reduced_motion_at(false, origin));
    assert!(p.schedule(&theme()).is_none());
    p.set_percent_at(0.9, origin).unwrap();
    assert!(p.schedule(&theme()).is_some());
    assert!(
        !p.event(&Input::Key(Key::new(KeyCode::Enter)), true)
            .consumed
    );
}

#[test]
fn callback_policy_replacement_styles_and_owned_view_are_independent() {
    use std::{cell::RefCell, rc::Rc};
    let calls = Rc::new(RefCell::new(Vec::new()));
    let captured = calls.clone();
    let mut p = Progress::new();
    p.set_width(4);
    p.show_percentage = false;
    p.styles.full = Some(Style::default().bold().bg(Color::Rgb(1, 2, 3)));
    p.styles.empty = Some(Style::default().italic().fg(Color::Indexed(2)));
    p.fill = ProgressFill::ColorFn(Box::new(move |total, current| {
        captured.borrow_mut().push((total, current));
        Color::Rgb(4, 5, 6)
    }));
    let view = p.view_as(0.5, &theme()).unwrap();
    assert_eq!(
        &*calls.borrow(),
        &[(0.5, 0.0), (0.5, 0.125), (0.5, 0.25), (0.5, 0.375)]
    );
    let frame = view.render(Size::new(4, 1)).unwrap();
    assert!(
        frame.row(0)[0]
            .style()
            .has(matea_style::TextAttribute::Bold)
    );
    assert_eq!(frame.row(0)[0].style().fg, Color::Rgb(4, 5, 6));
    assert_eq!(frame.row(0)[0].style().bg, Color::Rgb(4, 5, 6));
    assert!(
        frame.row(0)[3]
            .style()
            .has(matea_style::TextAttribute::Italic)
    );
    assert_eq!(frame.row(0)[3].style().fg, Color::Indexed(2));
    p.fill = ProgressFill::Gradient {
        gradient: Gradient::new(&[[0, 0, 0], [255, 255, 255]]).unwrap(),
        scaled: true,
    };
    p.styles = Default::default();
    p.set_width(2);
    p.view_as(1.0, &theme()).unwrap();
    assert_eq!(
        calls.borrow().len(),
        4,
        "replaced callback cannot retain precedence"
    );
    assert_eq!(
        view.render(Size::new(4, 1)).unwrap(),
        frame,
        "owned view cannot observe later model changes"
    );
}
