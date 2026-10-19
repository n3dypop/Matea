use matea_ratatui::{
    self as bridge, ImportError, WidgetState,
    buffer::{Buffer, CellDiffOption},
    layout::Rect,
    style::{Color as C, Modifier, Style as RStyle},
    widgets::{StatefulWidget, Widget},
};
use matea_render::{Size, Surface};
use matea_style::{BlinkStyle, Color, TextAttribute, UnderlineStyle};
use std::{cell::Cell, num::NonZeroU16, rc::Rc};

#[test]
fn cached_styles_and_ascii_runs_preserve_chunk_style_unicode_and_row_boundaries() {
    let mut source = Buffer::empty(Rect::new(0, 0, 600, 2));
    let red = RStyle::default().fg(C::Red).add_modifier(Modifier::ITALIC);
    let green = RStyle::reset()
        .fg(C::Green)
        .underline_color(C::Rgb(2, 3, 4));
    source.set_string(0, 0, "a".repeat(513), red);
    source[(255, 0)].set_style(green);
    source.set_string(513, 0, "茶👩‍💻e\u{301}", green);
    source.set_string(518, 0, "end", red);
    source.set_string(0, 1, "b".repeat(600), green);
    let frame = bridge::surface(&source).unwrap();
    for y in 0..2 {
        for x in 0..600 {
            let cell = &frame.row(y)[usize::from(x)];
            if !cell.is_continuation() {
                assert_eq!(cell.symbol(), source[(x, y)].symbol(), "{x},{y}");
                let expected = if (y == 0 && x != 255 && !(513..518).contains(&x)) && x < 521 {
                    (Color::Indexed(1), Color::Default, true)
                } else if y == 1 || x == 255 || (513..518).contains(&x) {
                    (Color::Indexed(2), Color::Rgb(2, 3, 4), false)
                } else {
                    (Color::Default, Color::Default, false)
                };
                assert_eq!(
                    (
                        cell.style().fg,
                        cell.style().underline_color,
                        cell.style().has(TextAttribute::Italic)
                    ),
                    expected,
                    "{x},{y}"
                );
            }
        }
    }
    source[(599, 1)].modifier = Modifier::SLOW_BLINK | Modifier::RAPID_BLINK;
    assert_eq!(
        bridge::surface(&source),
        Err(ImportError::ConflictingBlink { x: 599, y: 1 })
    );
    source[(599, 1)].modifier = Modifier::empty();
    source[(599, 1)].set_symbol("\x1b");
    assert_eq!(
        bridge::surface(&source),
        Err(ImportError::InvalidSymbol { x: 599, y: 1 })
    );
}

fn compare<W: Widget>(size: Size, make: impl Fn() -> W) -> Surface {
    let area = Rect::new(0, 0, size.width, size.height);
    let mut expected = Buffer::empty(area);
    make().render(area, &mut expected);
    let surface = bridge::render(size, make())
        .unwrap()
        .into_surface(size)
        .unwrap();
    for y in 0..size.height {
        let mut x = 0;
        while x < size.width {
            let actual = &surface.row(y)[usize::from(x)];
            let source = &expected[(x, y)];
            assert_eq!(actual.symbol(), source.symbol(), "{x},{y}");
            assert_eq!(
                actual.style().has(TextAttribute::Bold),
                source.modifier.contains(Modifier::BOLD)
            );
            assert_eq!(
                actual.style().has(TextAttribute::Reverse),
                source.modifier.contains(Modifier::REVERSED)
            );
            assert_eq!(
                actual.style().attributes.underline() == UnderlineStyle::Single,
                source.modifier.contains(Modifier::UNDERLINED)
            );
            assert!(actual.width() > 0);
            x += actual.width();
        }
    }
    surface
}
#[test]
fn catalog_widgets_render_through_owned_scenes_without_a_foreign_terminal() {
    use bridge::layout::{Alignment, Constraint};
    use bridge::symbols::Marker;
    use bridge::widgets::canvas::Canvas;
    use bridge::widgets::*;
    let size = Size::new(24, 6);
    assert!(
        compare(size, || Block::bordered().title("Matea"))
            .plain()
            .contains("Matea")
    );
    assert!(
        compare(size, || Paragraph::new("抹茶 / tea\nline two")
            .block(Block::bordered())
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center))
        .plain()
        .contains("抹茶")
    );
    compare(size, || {
        List::new(["one", "茶", "three"]).block(Block::bordered())
    });
    compare(size, || {
        Table::new(
            [Row::new(["tea", "茶"]), Row::new(["two", "2"])],
            [Constraint::Length(6), Constraint::Min(2)],
        )
        .header(Row::new(["name", "value"]))
    });
    compare(size, || Gauge::default().ratio(0.5).label("half"));
    compare(size, || LineGauge::default().ratio(0.5));
    compare(size, || Sparkline::default().data([0, 1, 3, 2, 7, 1]));
    compare(size, || {
        BarChart::default().data(&[("tea", 3), ("water", 7)])
    });
    compare(size, || Tabs::new(["tea", "water"]).select(1));

    compare(size, || Clear);
    compare(size, || Fill::new("."));
    compare(size, || {
        Chart::new(vec![
            Dataset::default()
                .name("tea")
                .data(&[(0.0, 0.0), (1.0, 1.0)])
                .marker(Marker::Dot),
        ])
        .x_axis(Axis::default().bounds([0.0, 1.0]))
        .y_axis(Axis::default().bounds([0.0, 1.0]))
    });
    compare(size, || {
        Canvas::default()
            .x_bounds([0.0, 2.0])
            .y_bounds([0.0, 2.0])
            .paint(|context| {
                context.draw(&bridge::widgets::canvas::Line {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 2.0,
                    y2: 2.0,
                    color: C::Green,
                })
            })
    });
    #[cfg(feature = "calendar")]
    compare(Size::new(30, 9), || {
        bridge::widgets::calendar::Monthly::new(
            bridge::time::Date::from_calendar_date(2026, bridge::time::Month::October, 19).unwrap(),
            bridge::widgets::calendar::CalendarEventStore::default(),
        )
        .show_month_header(RStyle::new().bold())
        .show_weekdays_header(RStyle::new())
    });
}
#[test]
fn all_named_indexed_rgb_colors_and_resolved_modifier_groups_survive_import() {
    let named = [
        C::Reset,
        C::Black,
        C::Red,
        C::Green,
        C::Yellow,
        C::Blue,
        C::Magenta,
        C::Cyan,
        C::Gray,
        C::DarkGray,
        C::LightRed,
        C::LightGreen,
        C::LightYellow,
        C::LightBlue,
        C::LightMagenta,
        C::LightCyan,
        C::White,
    ];
    for (i, value) in named.into_iter().enumerate() {
        let mut buffer = Buffer::empty(Rect::new(13, 17, 1, 1));
        let cell = &mut buffer.content[0];
        cell.set_symbol("x");
        cell.fg = value;
        cell.bg = C::Indexed(251);
        cell.underline_color = C::Rgb(13, 41, 227);
        let surface = bridge::surface(&buffer).unwrap();
        let s = surface.row(0)[0].style();
        assert_eq!(
            s.fg,
            if i == 0 {
                Color::Default
            } else {
                Color::Indexed((i - 1) as u8)
            }
        );
        assert_eq!(s.bg, Color::Indexed(251));
        assert_eq!(s.underline_color, Color::Rgb(13, 41, 227));
    }
    let limit: usize = if cfg!(miri) { 8 } else { 256 };
    for raw_index in 0..limit {
        let index = raw_index as u8;
        for flags in [
            Modifier::empty(),
            Modifier::BOLD
                | Modifier::DIM
                | Modifier::ITALIC
                | Modifier::UNDERLINED
                | Modifier::REVERSED
                | Modifier::HIDDEN
                | Modifier::CROSSED_OUT,
        ] {
            for blink in [
                Modifier::empty(),
                Modifier::SLOW_BLINK,
                Modifier::RAPID_BLINK,
            ] {
                let mut buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
                let cell = &mut buffer.content[0];
                cell.set_symbol("x");
                cell.fg = C::Indexed(index);
                cell.bg = C::Rgb(index, 41, 227);
                cell.modifier = flags | blink;
                let surface = bridge::surface(&buffer).unwrap();
                let s = surface.row(0)[0].style();
                assert_eq!(s.fg, Color::Indexed(index));
                assert_eq!(s.bg, Color::Rgb(index, 41, 227));
                for (a, b) in [
                    (TextAttribute::Bold, Modifier::BOLD),
                    (TextAttribute::Dim, Modifier::DIM),
                    (TextAttribute::Italic, Modifier::ITALIC),
                    (TextAttribute::Reverse, Modifier::REVERSED),
                    (TextAttribute::Hidden, Modifier::HIDDEN),
                    (TextAttribute::Strikethrough, Modifier::CROSSED_OUT),
                ] {
                    assert_eq!(s.has(a), flags.contains(b));
                }
                assert_eq!(
                    s.attributes.underline(),
                    if flags.contains(Modifier::UNDERLINED) {
                        UnderlineStyle::Single
                    } else {
                        UnderlineStyle::None
                    }
                );
                assert_eq!(
                    s.attributes.blink(),
                    if blink == Modifier::SLOW_BLINK {
                        BlinkStyle::Slow
                    } else if blink == Modifier::RAPID_BLINK {
                        BlinkStyle::Rapid
                    } else {
                        BlinkStyle::None
                    }
                );
            }
        }
    }
}
#[test]
fn unsupported_source_cells_and_invalid_buffers_fail_before_returning_a_scene() {
    let make = || Buffer::empty(Rect::new(0, 0, 2, 1));
    for (symbol, error) in [
        ("\x1b[2J", ImportError::InvalidSymbol { x: 0, y: 0 }),
        ("ab", ImportError::InvalidSymbol { x: 0, y: 0 }),
        ("", ImportError::InvalidSymbol { x: 0, y: 0 }),
        ("\u{301}", ImportError::ZeroWidth { x: 0, y: 0 }),
        (
            "ﾊﾟ",
            ImportError::WidthPolicy {
                x: 0,
                y: 0,
                source: 2,
                matea: 1,
            },
        ),
    ] {
        let mut buffer = make();
        buffer.content[0].set_symbol(symbol);
        assert_eq!(bridge::surface(&buffer), Err(error));
    }
    for option in [
        CellDiffOption::Skip,
        CellDiffOption::AlwaysUpdate,
        CellDiffOption::ForcedWidth(NonZeroU16::new(1).unwrap()),
    ] {
        let mut buffer = make();
        buffer.content[0].diff_option = option;
        assert_eq!(
            bridge::surface(&buffer),
            Err(ImportError::DiffOption { x: 0, y: 0, option })
        );
    }
    let mut buffer = make();
    buffer.content.pop();
    assert_eq!(
        bridge::surface(&buffer),
        Err(ImportError::InvalidBufferLength)
    );
    let mut buffer = make();
    buffer.content[0].set_symbol("茶");
    buffer.content[1].set_symbol("X");
    assert_eq!(
        bridge::surface(&buffer),
        Err(ImportError::OverlappingGlyph { x: 1, y: 0 })
    );
    let mut buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
    buffer.content[0].set_symbol("茶");
    assert_eq!(
        bridge::surface(&buffer),
        Err(ImportError::ClippedGlyph { x: 0, y: 0 })
    );
    let mut buffer = make();
    buffer.content[0].modifier = Modifier::SLOW_BLINK | Modifier::RAPID_BLINK;
    assert_eq!(
        bridge::surface(&buffer),
        Err(ImportError::ConflictingBlink { x: 0, y: 0 })
    );
    let mut buffer = make();
    buffer.content[0].modifier = Modifier::from_bits_retain(u16::MAX);
    assert_eq!(
        bridge::surface(&buffer),
        Err(ImportError::UnknownModifiers { x: 0, y: 0 })
    );
    #[allow(deprecated)]
    {
        let mut buffer = make();
        buffer.content[0].skip = true;
        assert_eq!(
            bridge::surface(&buffer),
            Err(ImportError::DeprecatedSkip { x: 0, y: 0 })
        );
    }
}
#[test]
fn source_buffers_drop_without_retaining_borrows_and_wide_placeholders_are_owned() {
    let snapshot = {
        let mut buffer = Buffer::empty(Rect::new(11, 19, 5, 1));
        buffer.set_string(11, 19, "A茶B", RStyle::new().fg(C::Green).bg(C::Blue));
        bridge::view(buffer).unwrap()
    };
    let a = snapshot.render(Size::new(5, 1)).unwrap();
    assert_eq!(a.plain(), "A茶B");
    assert!(a.row(0)[2].is_continuation());
    assert_eq!(a.row(0)[1].style(), a.row(0)[2].style());
    let old = snapshot.clone();
    let mut owned = snapshot.into_surface(Size::new(5, 1)).unwrap();
    owned
        .canvas(owned.area())
        .fill(matea_style::Style::default());
    assert_eq!(old.render(Size::new(5, 1)).unwrap(), a);
}
#[test]
fn stateful_render_updates_scoped_ui_state_without_putting_borrows_in_the_scene() {
    use bridge::layout::Constraint;
    use bridge::widgets::{List, ListState, Row, Table, TableState};
    let mut slot = WidgetState::new(ListState::default().with_selected(Some(3)));
    let mut direct = ListState::default().with_selected(Some(3));
    let size = Size::new(8, 2);
    let make = || List::new(["a", "b", "茶", "last"]).highlight_symbol("> ");
    let mut buffer = Buffer::empty(Rect::new(0, 0, 8, 2));
    StatefulWidget::render(make(), buffer.area, &mut buffer, &mut direct);
    let snapshot = slot.render(size, make()).unwrap();
    assert_eq!(
        snapshot.render(size).unwrap(),
        bridge::surface(&buffer).unwrap()
    );
    assert_eq!(*slot.state().unwrap(), direct);
    let borrow = slot.state().unwrap();
    assert!(matches!(
        slot.render(size, make()),
        Err(ImportError::StateBorrowed)
    ));
    drop(borrow);
    slot.state_mut().select(Some(0));
    assert!(
        slot.render(size, make())
            .unwrap()
            .render(size)
            .unwrap()
            .plain()
            .contains("> a")
    );
    assert!(snapshot.render(size).unwrap().plain().contains("last"));
    let mut scroll = bridge::widgets::ScrollbarState::new(100).position(3);
    bridge::render_stateful(size, bridge::widgets::Scrollbar::default(), &mut scroll).unwrap();
    let mut table = TableState::default().with_selected(Some(2));
    let widget = Table::new(
        [Row::new(["a"]), Row::new(["b"]), Row::new(["c"])],
        [Constraint::Min(1)],
    );
    assert!(
        bridge::render_stateful(size, widget, &mut table)
            .unwrap()
            .render(size)
            .unwrap()
            .plain()
            .contains('c')
    );
}
struct Probe {
    entered: Rc<Cell<bool>>,
    invalid: bool,
}
impl Widget for Probe {
    fn render(self, _: Rect, buffer: &mut Buffer) {
        self.entered.set(true);
        if self.invalid {
            buffer.area.x = 1;
        }
    }
}
#[test]
fn geometry_is_checked_before_widget_entry_and_local_model_state_need_not_be_send() {
    let entered = Rc::new(Cell::new(false));
    assert!(matches!(
        bridge::render(
            Size::new(u16::MAX, u16::MAX),
            Probe {
                entered: entered.clone(),
                invalid: false
            }
        ),
        Err(ImportError::Geometry(_))
    ));
    assert!(!entered.get());
    assert!(matches!(
        bridge::render(
            Size::new(2, 1),
            Probe {
                entered: entered.clone(),
                invalid: true
            }
        ),
        Err(ImportError::WidgetChangedGeometry)
    ));
    assert!(entered.get());
    let state = WidgetState::new(Rc::new(Cell::new(0)));
    assert_eq!(state.state().unwrap().get(), 0);
    for size in [Size::new(0, 0), Size::new(0, 3), Size::new(3, 0)] {
        let view = bridge::render(size, bridge::widgets::Paragraph::new("tea")).unwrap();
        assert_eq!(view.into_surface(size).unwrap().size(), size);
    }
}

#[test]
fn theme_conversion_preserves_explicit_false_default_colors_and_every_supported_group() {
    let flags = [
        TextAttribute::Bold,
        TextAttribute::Dim,
        TextAttribute::Italic,
        TextAttribute::Reverse,
        TextAttribute::Hidden,
        TextAttribute::Strikethrough,
    ];
    for mask in 0..64u8 {
        for blink in [BlinkStyle::None, BlinkStyle::Slow, BlinkStyle::Rapid] {
            for underline in [UnderlineStyle::None, UnderlineStyle::Single] {
                let mut native = matea_style::Style::default()
                    .fg(Color::Rgb(19, 41, 227))
                    .bg(Color::Default)
                    .underline_color(Color::Indexed(251))
                    .blink(blink)
                    .underline_style(underline);
                for (i, flag) in flags.into_iter().enumerate() {
                    native = native.attribute(flag, mask & (1 << i) != 0);
                }
                let converted = bridge::to_style(native).unwrap();
                for previous in [
                    RStyle::new(),
                    RStyle::new()
                        .fg(C::Red)
                        .bg(C::Blue)
                        .add_modifier(Modifier::all()),
                ] {
                    let mut buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
                    buffer.content[0].set_style(previous).set_style(converted);
                    assert_eq!(bridge::surface(&buffer).unwrap().row(0)[0].style(), native);
                }
            }
        }
    }
    for token in [
        matea_style::Style::default().fraktur(),
        matea_style::Style::default().overline(),
        matea_style::Style::default().underline_style(UnderlineStyle::Curly),
        matea_style::Style::default().text_frame(matea_style::TextFrame::Framed),
    ] {
        assert!(bridge::to_style(token).is_err());
    }
}
struct MutatingPanic;
impl StatefulWidget for MutatingPanic {
    type State = usize;
    fn render(self, _: Rect, _: &mut Buffer, state: &mut usize) {
        *state += 1;
        panic!("foreign widget panic");
    }
}
#[test]
fn foreign_panics_release_ui_state_borrows_without_claiming_rollback() {
    let state = WidgetState::new(0usize);
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        state.render(Size::new(2, 1), MutatingPanic)
    }));
    assert!(caught.is_err());
    assert_eq!(*state.state().unwrap(), 1);
}
