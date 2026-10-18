//! Original native observation caller; no timing or fixture rewriting.
use matea::prelude::*;
use std::fmt::Write;
use std::hint::black_box;

pub(crate) fn costs(n: usize) {
    use super::{measurement::measure, viewport_digest};
    for (kind, line) in [("ascii", "abcdefgh"), ("unicode", "抹茶抹茶")] {
        let text = std::iter::repeat_n(line, 100)
            .collect::<Vec<_>>()
            .join("\n");
        let make = || {
            let mut view = Viewport::new(black_box(&text));
            view.set_size(4, 3);
            view.set_soft_wrap(true);
            view
        };
        measure(
            &format!("viewport-wrap/construct_{kind}"),
            n,
            || {
                let view = make();
                let total = view.display_line_count();
                let owned = view.content().to_owned();
                viewport_digest(&owned, total, view.offset())
            },
            &format!("total=200;y=0;text={text}"),
            "fresh viewport; install one existing 100-line input, set 4x3 and soft wrap, compute complete display geometry, create independent owned content snapshot, consume all UTF-8 bytes/total/y with FNV64; include drop; no terminal I/O",
        );
        let mut view = make();
        assert_eq!(view.display_line_count(), 200);
        measure(
            &format!("viewport-wrap/count_{kind}"),
            n,
            || black_box(&view).display_line_count() as u64,
            "total=200",
            "read total display row count from existing unchanged soft-wrap document; geometry initialized; no render/terminal I/O",
        );
        view.set_offset(7);
        measure(
            &format!("viewport-wrap/scroll_{kind}"),
            n,
            || {
                view.set_offset(7);
                view.scroll_down(1);
                view.scroll_up(1);
                viewport_digest("", view.horizontal_offset(), view.offset())
            },
            "x=0;y=7",
            "existing 4x3 soft-wrap document; reset y=7, down one/up one, consume final x/y with FNV64; no render/terminal I/O",
        );
        let plain = view
            .view(&Theme::plain(), false)
            .render(Size::new(4, 3))
            .unwrap()
            .plain();
        measure(
            &format!("viewport-wrap/view_{kind}"),
            n,
            || {
                let plain = black_box(&view)
                    .view(&Theme::plain(), false)
                    .render(Size::new(4, 3))
                    .unwrap()
                    .plain();
                viewport_digest(&plain, view.horizontal_offset(), view.offset())
            },
            &plain,
            "existing 4x3 soft-wrap document at x=0,y=7; construct complete owned plain frame, trim only ASCII right-padding per row; consume all bytes/x/y with FNV64; Rust scene/surface/plain or Go View/plain adapter included; no terminal I/O",
        );
        measure(
            &format!("viewport-wrap/borrowed_{kind}"),
            n,
            || {
                let mut value = 14695981039346656037u64;
                for (index, line) in black_box(&view).visible_lines().enumerate() {
                    if index != 0 {
                        value = (value ^ 10).wrapping_mul(1099511628211);
                    }
                    for byte in line.bytes() {
                        value = (value ^ u64::from(byte)).wrapping_mul(1099511628211);
                    }
                }
                for byte in (view.horizontal_offset() as u64)
                    .to_le_bytes()
                    .into_iter()
                    .chain((view.offset() as u64).to_le_bytes())
                {
                    value = (value ^ u64::from(byte)).wrapping_mul(1099511628211);
                }
                value
            },
            &plain,
            "Rust-only borrowed wrapped-row iterator and full UTF-8/x/y digest; initialized geometry, no owned frame or terminal I/O; no matched upstream ratio",
        );
        view.set_offset(0);
        measure(
            &format!("viewport-wrap/resize_{kind}"),
            n,
            || {
                view.set_width(8);
                let wide = view.display_line_count();
                view.set_width(4);
                let narrow = view.display_line_count();
                viewport_digest("", wide, narrow)
            },
            "wide=100;narrow=200",
            "existing soft-wrap document at y=0; set width 8/read full geometry then width 4/read geometry, consume both totals with FNV64; include invalidated cache drop/rebuild; no render/terminal I/O",
        );
    }
}

fn decode(text: &str) -> String {
    String::from_utf8(
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect(),
    )
    .unwrap()
}
pub(crate) fn fixtures() {
    for line in
        include_str!("../../../crates/matea-widgets/tests/fixtures/viewport-wrap-source.tsv")
            .lines()
    {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 13);
        let text = decode(fields[1]);
        let width: u16 = fields[2].parse().unwrap();
        let height: u16 = fields[3].parse().unwrap();
        let mut view = Viewport::new(&text);
        view.set_size(width, height);
        view.set_soft_wrap(true);
        view.fill_height = fields[4].parse().unwrap();
        match fields[5] {
            "top" => {}
            "bottom" => {
                view.go_to_bottom();
            }
            "page" => {
                view.page_down();
                view.page_down();
            }
            "ensure" => {
                view.ensure_visible(1, 4..5).unwrap();
            }
            _ => panic!("unknown fixture action"),
        }
        let plain = view
            .view(&Theme::plain(), false)
            .render(Size::new(width, height))
            .unwrap()
            .plain();
        let mut hex = String::new();
        for byte in plain.bytes() {
            write!(hex, "{byte:02x}").unwrap();
        }
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            fields[0],
            fields[1],
            width,
            height,
            view.fill_height,
            fields[5],
            view.display_line_count(),
            view.offset(),
            view.horizontal_offset(),
            view.visible_line_count(),
            hex
        );
    }
}
