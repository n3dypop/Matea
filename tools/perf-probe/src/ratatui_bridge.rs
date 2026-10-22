//! Matched source-widget vs managed owned-scene boundary, without terminal I/O.
//! Projection contains dimensions and every visible lead's width, UTF-8 symbol,
//! three canonical colors and all nine source modifiers. Covered continuations
//! have one canonical zero marker; their hidden styles have no visible effect.
use super::measurement::measure;
use matea::{
    prelude::{BlinkStyle, Color, Size, Surface, TextAttribute, UnderlineStyle},
    ratatui::{self as bridge, buffer::Buffer, layout::Rect, style, widgets::*},
};
use std::{fmt::Write, hint::black_box};

fn color(out: &mut Vec<u8>, color: Color) {
    match color {
        Color::Default => out.extend_from_slice(&[0, 0, 0, 0]),
        Color::Indexed(i) => out.extend_from_slice(&[1, i, 0, 0]),
        Color::Rgb(r, g, b) => out.extend_from_slice(&[2, r, g, b]),
    }
}
fn source_color(value: style::Color) -> Color {
    use style::Color as C;
    match value {
        C::Reset => Color::Default,
        C::Black => Color::Indexed(0),
        C::Red => Color::Indexed(1),
        C::Green => Color::Indexed(2),
        C::Yellow => Color::Indexed(3),
        C::Blue => Color::Indexed(4),
        C::Magenta => Color::Indexed(5),
        C::Cyan => Color::Indexed(6),
        C::Gray => Color::Indexed(7),
        C::DarkGray => Color::Indexed(8),
        C::LightRed => Color::Indexed(9),
        C::LightGreen => Color::Indexed(10),
        C::LightYellow => Color::Indexed(11),
        C::LightBlue => Color::Indexed(12),
        C::LightMagenta => Color::Indexed(13),
        C::LightCyan => Color::Indexed(14),
        C::White => Color::Indexed(15),
        C::Indexed(i) => Color::Indexed(i),
        C::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
fn header(size: Size) -> Vec<u8> {
    let mut out = Vec::with_capacity(usize::from(size.width) * usize::from(size.height) * 24 + 4);
    out.extend_from_slice(&size.width.to_le_bytes());
    out.extend_from_slice(&size.height.to_le_bytes());
    out
}
fn lead(out: &mut Vec<u8>, width: u16, symbol: &str, fg: Color, bg: Color, ul: Color, flags: u16) {
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&(symbol.len() as u32).to_le_bytes());
    out.extend_from_slice(symbol.as_bytes());
    color(out, fg);
    color(out, bg);
    color(out, ul);
    out.extend_from_slice(&flags.to_le_bytes());
}
fn source_projection(buffer: &Buffer) -> Vec<u8> {
    use bridge::buffer::CellWidth;
    let size = Size::new(buffer.area.width, buffer.area.height);
    let mut out = header(size);
    for y in 0..size.height {
        let mut covered = 0;
        for x in 0..size.width {
            if covered != 0 {
                out.extend_from_slice(&0u16.to_le_bytes());
                covered -= 1;
                continue;
            }
            let cell = &buffer[(x, y)];
            let width = cell.cell_width();
            assert!(width > 0 && width <= size.width - x);
            lead(
                &mut out,
                width,
                cell.symbol(),
                source_color(cell.fg),
                source_color(cell.bg),
                source_color(cell.underline_color),
                cell.modifier.bits(),
            );
            covered = width - 1;
        }
    }
    out
}
fn native_projection(surface: &Surface) -> Vec<u8> {
    use style::Modifier as M;
    let mut out = header(surface.size());
    for y in 0..surface.size().height {
        for cell in surface.row(y) {
            if cell.is_continuation() {
                out.extend_from_slice(&0u16.to_le_bytes());
                continue;
            }
            let s = cell.style();
            let mut flags = M::empty();
            for (a, b) in [
                (TextAttribute::Bold, M::BOLD),
                (TextAttribute::Dim, M::DIM),
                (TextAttribute::Italic, M::ITALIC),
                (TextAttribute::Reverse, M::REVERSED),
                (TextAttribute::Hidden, M::HIDDEN),
                (TextAttribute::Strikethrough, M::CROSSED_OUT),
            ] {
                flags.set(b, s.has(a));
            }
            flags.set(
                M::UNDERLINED,
                s.attributes.underline() == UnderlineStyle::Single,
            );
            flags.set(M::SLOW_BLINK, s.attributes.blink() == BlinkStyle::Slow);
            flags.set(M::RAPID_BLINK, s.attributes.blink() == BlinkStyle::Rapid);
            lead(
                &mut out,
                cell.width(),
                cell.symbol(),
                s.fg,
                s.bg,
                s.underline_color,
                flags.bits(),
            );
        }
    }
    out
}
fn digest(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(out, "{b:02x}").unwrap();
    }
    out
}
fn draw<W: Widget>(size: Size, widget: W, managed: bool) -> Vec<u8> {
    if managed {
        native_projection(
            &bridge::render(size, widget)
                .unwrap()
                .into_surface(size)
                .unwrap(),
        )
    } else {
        let area = Rect::new(0, 0, size.width, size.height);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer);
        source_projection(&buffer)
    }
}
fn draw_stateful<W: StatefulWidget>(
    size: Size,
    widget: W,
    state: &mut W::State,
    managed: bool,
) -> Vec<u8> {
    if managed {
        native_projection(
            &bridge::render_stateful(size, widget, state)
                .unwrap()
                .into_surface(size)
                .unwrap(),
        )
    } else {
        let area = Rect::new(0, 0, size.width, size.height);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer, state);
        source_projection(&buffer)
    }
}
pub(crate) fn costs(n: usize, managed: bool) {
    const SEMANTICS: &str = "same pinned source Widget/StatefulWidget builder from existing 100-line input, fresh buffer and selected=50 state per operation; source returns owned Buffer, Matea returns owned Surface via managed scene/unique consuming transfer; produce complete dimensions/lead width+UTF8/fg+bg+underline+all nine modifier binary projection with zero continuation markers, then FNV64; includes widget, validation/import where applicable, serialization and drop; excludes terminal I/O";
    for (kind, line) in [("ascii", "matcha tea"), ("unicode", "抹茶 e\u{301} 👩‍💻 tea")] {
        let text = (0..100).map(|i| format!("{line} #{i}")).collect::<Vec<_>>();
        let joined = text.join("\n");
        for size in [Size::new(24, 6), Size::new(80, 24)] {
            for widget in ["paragraph", "list", "table"] {
                let operation = || {
                    let selected = style::Style::default()
                        .fg(style::Color::Rgb(40, 180, 80))
                        .bg(style::Color::Indexed(17))
                        .underline_color(style::Color::LightCyan)
                        .add_modifier(
                            style::Modifier::BOLD
                                | style::Modifier::ITALIC
                                | style::Modifier::UNDERLINED
                                | style::Modifier::REVERSED,
                        );
                    match widget {
                        "paragraph" => draw(
                            size,
                            Paragraph::new(black_box(joined.as_str()))
                                .style(selected)
                                .wrap(Wrap { trim: true })
                                .block(Block::bordered().title("Matea")),
                            managed,
                        ),
                        "list" => draw_stateful(
                            size,
                            List::new(black_box(&text).iter().map(String::as_str))
                                .highlight_style(selected)
                                .highlight_symbol("> ")
                                .block(Block::bordered().title("Matea")),
                            &mut ListState::default().with_selected(Some(50)),
                            managed,
                        ),
                        _ => draw_stateful(
                            size,
                            Table::new(
                                black_box(&text).iter().map(|s| {
                                    let (label, index) = s.rsplit_once(' ').unwrap();
                                    Row::new([index, label])
                                }),
                                [
                                    bridge::layout::Constraint::Length(4),
                                    bridge::layout::Constraint::Min(0),
                                ],
                            )
                            .row_highlight_style(selected)
                            .block(Block::bordered().title("Matea")),
                            &mut TableState::default().with_selected(Some(50)),
                            managed,
                        ),
                    }
                };
                let expected = operation();
                measure(
                    &format!(
                        "ratatui-bridge/{widget}_{kind}_{}x{}",
                        size.width, size.height
                    ),
                    n,
                    || digest(&operation()),
                    &hex(&expected),
                    SEMANTICS,
                );
            }
        }
    }
}
