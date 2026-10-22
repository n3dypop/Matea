use matea_render::{RichText, Size, Surface, View};
use matea_style::{Color as MateaColor, Style as MateaStyle};
use ratatui_core::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
};
use std::{fmt::Write as _, hint::black_box};
use unicode_width::UnicodeWidthStr;
#[path = "../../perf-probe/src/measurement.rs"]
mod measurement;
mod patches;

const CELLS: u16 = 192;
const INITIAL_HASH: u64 = 0xcbf29ce484222325;
fn hash(mut value: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        value = (value ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
    }
    value
}
fn atom(value: u64, text: &str, color: u8) -> u64 {
    hash(
        hash(
            hash(value, &(text.len() as u64).to_le_bytes()),
            text.as_bytes(),
        ),
        &[color],
    )
}
fn matea_color(style: MateaStyle) -> u8 {
    match style.fg {
        MateaColor::Default => 0,
        MateaColor::Indexed(index) => index,
        _ => panic!("unexpected corpus color"),
    }
}
fn reference_color(color: Color) -> u8 {
    match color {
        Color::Reset => 0,
        Color::Indexed(index) => index,
        _ => panic!("unexpected corpus color"),
    }
}
fn corpus(unicode: bool) -> Vec<String> {
    (0..12)
        .map(|index| {
            if unicode {
                format!("抹茶{index:02} e\u{301}👩‍💻|")
            } else {
                format!("m{index:02} tea / ")
            }
        })
        .collect()
}
fn rich(parts: &[String]) -> RichText {
    RichText::from_spans(parts.iter().enumerate().map(|(index, part)| {
        (
            part.as_str(),
            MateaStyle::default().fg(MateaColor::Indexed((index % 3 + 1) as u8)),
        )
    }))
}
fn line(parts: &[String]) -> Line<'static> {
    Line::from(
        parts
            .iter()
            .enumerate()
            .map(|(index, part)| {
                Span::styled(
                    part.clone(),
                    Style::default().fg(Color::Indexed((index % 3 + 1) as u8)),
                )
            })
            .collect::<Vec<_>>(),
    )
}
fn rich_document(text: &RichText) -> u64 {
    let mut value = hash(INITIAL_HASH, &(text.width() as u64).to_le_bytes());
    for (range, style) in text.styles() {
        value = atom(value, &text.as_str()[range], matea_color(style));
    }
    value
}
fn line_document(text: &Line<'_>) -> u64 {
    let mut value = hash(INITIAL_HASH, &(text.width() as u64).to_le_bytes());
    for span in &text.spans {
        value = atom(
            value,
            &span.content,
            reference_color(span.style.fg.unwrap_or(Color::Reset)),
        );
    }
    value
}
fn rich_graphemes(text: &RichText) -> u64 {
    text.graphemes().fold(INITIAL_HASH, |value, g| {
        hash(
            atom(value, g.symbol, matea_color(g.style)),
            &(g.width as u64).to_le_bytes(),
        )
    })
}
fn line_graphemes(text: &Line<'_>) -> u64 {
    text.styled_graphemes(Style::default())
        .fold(INITIAL_HASH, |value, g| {
            hash(
                atom(
                    value,
                    g.symbol,
                    reference_color(g.style.fg.unwrap_or(Color::Reset)),
                ),
                &(g.symbol.width() as u64).to_le_bytes(),
            )
        })
}
fn rich_frame(surface: &Surface) -> u64 {
    let mut value = INITIAL_HASH;
    for (x, cell) in surface.row(0).iter().enumerate() {
        if !cell.is_continuation() {
            value = atom(
                hash(value, &(x as u64).to_le_bytes()),
                cell.symbol(),
                matea_color(cell.style()),
            );
        }
    }
    value
}
fn line_frame(buffer: &Buffer) -> u64 {
    let mut value = INITIAL_HASH;
    let mut x = 0;
    while x < CELLS {
        let cell = &buffer[(x, 0)];
        value = atom(
            hash(value, &u64::from(x).to_le_bytes()),
            cell.symbol(),
            reference_color(cell.fg),
        );
        x += cell.symbol().width().max(1) as u16;
    }
    value
}
fn rich_output(text: &RichText) -> String {
    let mut output = format!("width={} bytes={} styles=", text.width(), text.len());
    for (range, style) in text.styles() {
        write!(
            output,
            "{}:{}:{};",
            range.start,
            range.end,
            matea_color(style)
        )
        .unwrap();
    }
    output.push_str(" text=");
    output.push_str(text.as_str());
    output
}
fn line_output(text: &Line<'_>) -> String {
    let len: usize = text.spans.iter().map(|span| span.content.len()).sum();
    let mut output = format!("width={} bytes={len} styles=", text.width());
    let mut offset = 0;
    for span in &text.spans {
        let end = offset + span.content.len();
        write!(
            output,
            "{offset}:{end}:{};",
            reference_color(span.style.fg.unwrap_or(Color::Reset))
        )
        .unwrap();
        offset = end;
    }
    output.push_str(" text=");
    for span in &text.spans {
        output.push_str(&span.content);
    }
    output
}
fn frame_output(surface: &Surface) -> String {
    let mut output = String::new();
    for (x, cell) in surface.row(0).iter().enumerate() {
        if !cell.is_continuation() {
            write!(
                output,
                "{x}:{}:{}:",
                matea_color(cell.style()),
                cell.symbol().len()
            )
            .unwrap();
            output.push_str(cell.symbol());
            output.push(';');
        }
    }
    output
}
fn buffer_output(buffer: &Buffer) -> String {
    let mut output = String::new();
    let mut x = 0;
    while x < CELLS {
        let cell = &buffer[(x, 0)];
        write!(
            output,
            "{x}:{}:{}:",
            reference_color(cell.fg),
            cell.symbol().len()
        )
        .unwrap();
        output.push_str(cell.symbol());
        output.push(';');
        x += cell.symbol().width().max(1) as u16;
    }
    output
}
fn conformance() {
    for unicode in [false, true] {
        let parts = corpus(unicode);
        let rich = rich(&parts);
        let line = line(&parts);
        assert_eq!(rich_output(&rich), line_output(&line));
        assert_eq!(rich_document(&rich), line_document(&line));
        assert_eq!(rich_graphemes(&rich), line_graphemes(&line));
        let frame = View::rich_text(rich.clone())
            .render(Size::new(CELLS, 1))
            .unwrap();
        let area = Rect::new(0, 0, CELLS, 1);
        let mut buffer = Buffer::empty(area);
        (&line).render(area, &mut buffer);
        assert_eq!(frame_output(&frame), buffer_output(&buffer));
        assert_eq!(rich_frame(&frame), line_frame(&buffer));
        // These literal lists also compare every grapheme/style/width without
        // treating a matching rolling digest as collision-free equality.
        let actual = rich
            .graphemes()
            .map(|g| (g.symbol, matea_color(g.style), g.width))
            .collect::<Vec<_>>();
        let expected = line
            .styled_graphemes(Style::default())
            .map(|g| {
                (
                    g.symbol,
                    reference_color(g.style.fg.unwrap_or(Color::Reset)),
                    g.symbol.width(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }
}
fn benchmarks(reference: bool) {
    let n = measurement::iterations(1000);
    for unicode in [false, true] {
        let kind = if unicode { "unicode" } else { "ascii" };
        let parts = corpus(unicode);
        let text = rich(&parts);
        let original = line(&parts);
        let output = if reference {
            line_output(&original)
        } else {
            rich_output(&text)
        };
        let id = |operation| format!("rich-text/{kind}/{operation}");
        let semantics = "12 owned UTF-8 chunks; alternating indexed foreground; grapheme-aligned boundaries; no controls/LF; full content/style/width digest; matching valid text only";
        if reference {
            measurement::measure(
                &id("construct-read"),
                n,
                || line_document(&line(black_box(&parts))),
                &output,
                semantics,
            );
            measurement::measure(
                &id("clone-read"),
                n,
                || line_document(&black_box(&original).clone()),
                &output,
                semantics,
            );
            measurement::measure(
                &id("width"),
                n,
                || black_box(&original).width() as u64,
                &output,
                semantics,
            );
            measurement::measure(
                &id("graphemes"),
                n,
                || line_graphemes(black_box(&original)),
                &output,
                semantics,
            );
        } else {
            measurement::measure(
                &id("construct-read"),
                n,
                || rich_document(&rich(black_box(&parts))),
                &output,
                semantics,
            );
            measurement::measure(
                &id("clone-read"),
                n,
                || rich_document(&black_box(&text).clone()),
                &output,
                semantics,
            );
            measurement::measure(
                &id("width"),
                n,
                || black_box(&text).width() as u64,
                &output,
                semantics,
            );
            measurement::measure(
                &id("graphemes"),
                n,
                || rich_graphemes(black_box(&text)),
                &output,
                semantics,
            );
        }
        let frame = View::rich_text(text.clone())
            .render(Size::new(CELLS, 1))
            .unwrap();
        let output = if reference {
            let area = Rect::new(0, 0, CELLS, 1);
            let mut buffer = Buffer::empty(area);
            (&original).render(area, &mut buffer);
            buffer_output(&buffer)
        } else {
            frame_output(&frame)
        };
        let semantics = "fresh 192x1 frame, complete styled line; digest every leading cell and blank padding; source continuation fillers skipped by whole-grapheme width; view construction/Arc clone included for Matea; no terminal output";
        if reference {
            measurement::measure(
                &id("render-read"),
                n,
                || {
                    let area = Rect::new(0, 0, CELLS, 1);
                    let mut buffer = Buffer::empty(area);
                    black_box(&original).render(area, &mut buffer);
                    line_frame(&buffer)
                },
                &output,
                semantics,
            );
        } else {
            measurement::measure(
                &id("render-read"),
                n,
                || {
                    let frame = View::rich_text(black_box(&text).clone())
                        .render(Size::new(CELLS, 1))
                        .unwrap();
                    rich_frame(&frame)
                },
                &output,
                semantics,
            );
        }
    }
}
fn reference_raw() -> String {
    use std::fmt::Write;
    let mut output = String::new();
    let cases: &[(&str, &[&str])] = &[
        ("combining_split", &["e", "\u{301}", "x"]),
        ("combining_joined", &["e\u{301}", "x"]),
        ("zwj_split", &["👩", "\u{200d}", "💻", "!"]),
        ("zwj_joined", &["👩‍💻", "!"]),
        ("flag_split", &["🇯", "🇵", "!"]),
        ("flag_joined", &["🇯🇵", "!"]),
        ("keycap_split", &["1", "\u{fe0f}\u{20e3}", "X"]),
        ("keycap_joined", &["1️⃣", "X"]),
    ];
    let colors = [Color::Red, Color::Blue, Color::Green, Color::Yellow];
    for (name, parts) in cases {
        let spans = parts
            .iter()
            .enumerate()
            .map(|(index, text)| {
                Span::styled(
                    *text,
                    Style::default().fg(if name.ends_with("joined") && index == 1 {
                        if *name == "zwj_joined" {
                            Color::Yellow
                        } else {
                            Color::Green
                        }
                    } else {
                        colors[index]
                    }),
                )
            })
            .collect::<Vec<_>>();
        let line = Line::from(spans);
        let width = line.width();
        let area = Rect::new(0, 0, 10, 1);
        let mut buffer = Buffer::empty(area);
        line.render(area, &mut buffer);
        for x in 0..10 {
            let cell = &buffer[(x, 0)];
            let hex = cell
                .symbol()
                .bytes()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            writeln!(output, "{name}\t{width}\t{x}\t{hex}\t{:?}", cell.fg).unwrap();
        }
    }
    output
}
fn main() {
    let raw = reference_raw();
    assert_eq!(
        raw,
        include_str!("../../../crates/matea-render/tests/fixtures/ratatui-span-boundaries.tsv")
    );
    conformance();
    let patch_cases = patches::conformance();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args == ["--oracle"] {
        print!("{raw}");
    } else if args == ["--patch-conflicts"] {
        print!("{}", patches::conflicting_controls());
    } else if args.is_empty() || args == ["--check"] {
        println!(
            "8 cases / 80 pinned public reference cells and 2 complete styled corpora match; {patch_cases} style-patch observations match"
        );
    } else if args == ["--matea"] || args == ["--reference"] {
        benchmarks(args == ["--reference"]);
    } else if args == ["--patch-matea"] || args == ["--patch-reference"] {
        patches::costs(args == ["--patch-reference"]);
    } else {
        panic!(
            "expected --check, --oracle, --matea, --reference, --patch-matea or --patch-reference"
        );
    }
}
