//! Original bounded viewport highlight observation caller.
use matea::prelude::*;
use std::fmt::Write;

pub(crate) fn cells(view: &Viewport) -> String {
    let width = view.width().expect("explicit fixture width");
    let surface = view
        .view(&Theme::plain(), false)
        .render(Size::new(width, view.rows))
        .unwrap();
    let mut out = String::with_capacity(usize::from(width) * usize::from(view.rows) * 32);
    for y in 0..view.rows {
        for cell in surface.row(y) {
            if cell.is_continuation() {
                out.push_str("0,,0,0,0;");
                continue;
            }
            write!(out, "{},", cell.width()).unwrap();
            for byte in cell.symbol().bytes() {
                write!(out, "{byte:02x}").unwrap();
            }
            write!(
                out,
                ",{},{},{};",
                cell.style().has(TextAttribute::Bold),
                cell.style().attributes.underline() as u8,
                cell.style().has(TextAttribute::Reverse)
            )
            .unwrap();
        }
    }
    out
}
fn hex(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        write!(out, "{byte:02x}").unwrap();
    }
    out
}
pub(crate) fn fixtures() {
    let cases = [
        (
            "ascii",
            "one tea two tea\nsecond tea line\nlast",
            vec![4..7, 12..15, 23..26],
        ),
        (
            "mixed",
            "A茶B👩‍💻C\n🇯🇵e\u{301}\nlast茶",
            vec![1..4, 5..16, 34..37],
        ),
        ("split", "e\u{301}👩‍💻🇯🇵", vec![0..1, 1..3, 7..10, 18..22]),
        ("multiline", "ab\ncd\nef", vec![1..4, 6..8]),
        ("empty", "ab\ncd", vec![0..0, 2..2]),
    ];
    for (name, text, ranges) in cases {
        for width in [1, 3, 8] {
            for height in [1, 3] {
                for wrap in [false, true] {
                    for action in ["set", "next", "previous", "clear", "empty", "bottom", "top"] {
                        let mut view = Viewport::new(text);
                        view.set_size(width, height);
                        view.set_soft_wrap(wrap);
                        view.highlight_style = StylePatch::new()
                            .attribute(TextAttribute::Bold, true)
                            .underline_style(UnderlineStyle::Single);
                        view.selected_highlight_style =
                            StylePatch::new().attribute(TextAttribute::Reverse, true);
                        view.set_highlights(ranges.iter().cloned()).unwrap();
                        match action {
                            "next" => {
                                view.highlight_next();
                            }
                            "previous" => {
                                view.highlight_previous();
                            }
                            "clear" => {
                                view.clear_highlights();
                            }
                            "empty" => {
                                view.set_highlights([]).unwrap();
                            }
                            "bottom" => {
                                view.go_to_bottom();
                            }
                            "top" => {
                                view.go_to_top();
                            }
                            _ => {}
                        }
                        println!(
                            "{name}\t{}\t{width}\t{height}\t{wrap}\t{action}\t{}\t{}\t\t{}\t",
                            hex(text),
                            view.horizontal_offset(),
                            view.offset(),
                            hex(&cells(&view))
                        );
                    }
                }
            }
        }
    }
}

pub(crate) fn costs(n: usize) {
    use super::{measurement::measure, viewport_digest};
    use std::hint::black_box;
    const INSTALL: &str = "existing 100-line 24x3 flat viewport; reset y=0, replace all 100 ordered whole-EGC byte matches, reveal first match, produce complete owned lead-cell symbol/width/bold/underline/reverse projection plus x/y FNV64; includes Rust scene/surface or Go View/Ultraviolet ANSI interpretation and canonical serialization; no terminal I/O";
    const VIEW: &str = "existing initialized 100-line 24x3 flat viewport and 100 matches, first selected; produce complete owned lead-cell symbol/width/bold/underline/reverse projection plus x/y FNV64; includes Rust scene/surface or Go View/Ultraviolet ANSI interpretation and canonical serialization; no terminal I/O";
    const CYCLE: &str = "existing initialized 100-line 24x3 flat viewport and 100 matches, first selected; next then previous match, reveal each, produce complete owned lead-cell symbol/width/bold/underline/reverse projection plus x/y FNV64; includes Rust scene/surface or Go View/Ultraviolet ANSI interpretation and canonical serialization; no terminal I/O";
    for (kind, line) in [("ascii", "prefix tea tail"), ("unicode", "抹茶 tea 茶")] {
        let text = (0..100)
            .map(|i| format!("{line} #{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let ranges: Vec<_> = text
            .match_indices("tea")
            .map(|(i, s)| i..i + s.len())
            .collect();
        let mut view = Viewport::new(&text);
        view.set_size(24, 3);
        view.highlight_style = StylePatch::new()
            .attribute(TextAttribute::Bold, true)
            .underline_style(UnderlineStyle::Single);
        view.selected_highlight_style = StylePatch::new().attribute(TextAttribute::Reverse, true);
        view.set_highlights(ranges.iter().cloned()).unwrap();
        let output = format!("x=0;y=0;cells={}", cells(&view));
        for (operation, semantics) in [("install", INSTALL), ("view", VIEW), ("cycle", CYCLE)] {
            measure(
                &format!("viewport-highlights/{operation}_{kind}"),
                n,
                || {
                    if operation == "install" {
                        view.set_offset(0);
                        view.set_highlights(black_box(&ranges).iter().cloned())
                            .unwrap();
                    } else if operation == "cycle" {
                        view.highlight_next();
                        view.highlight_previous();
                    }
                    let projected = cells(black_box(&view));
                    viewport_digest(&projected, view.horizontal_offset(), view.offset())
                },
                &output,
                semantics,
            );
        }
    }
}
