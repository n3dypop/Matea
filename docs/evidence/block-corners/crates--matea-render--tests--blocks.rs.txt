use matea_render::{Size, StyledBlock, Surface};
use matea_style::{
    Alignment, BlinkStyle, BlockStyle, BorderPreset, Color, Edges, Insets, Style, StylePatch,
    TabWidth, TextAttribute, UnderlineStyle,
};
use std::fmt::Write;

fn unhex(s: &str) -> String {
    assert!(s.len().is_multiple_of(2));
    let bytes: Vec<u8> = s
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}
fn config(raw: &str) -> BlockStyle {
    let c: Vec<i32> = raw.split(',').map(|s| s.parse().unwrap()).collect();
    assert_eq!(c.len(), 24);
    let mut style = BlockStyle::default()
        .padding(Insets {
            top: c[0] as u16,
            right: c[1] as u16,
            bottom: c[2] as u16,
            left: c[3] as u16,
        })
        .margin(Insets {
            top: c[4] as u16,
            right: c[5] as u16,
            bottom: c[6] as u16,
            left: c[7] as u16,
        })
        .width(c[8] as u16)
        .height(c[9] as u16)
        .max_width(c[10] as u16)
        .max_height(c[11] as u16);
    let align = |n| match n {
        0 => Alignment::Start,
        1 => Alignment::Center,
        2 => Alignment::End,
        _ => panic!("alignment"),
    };
    style = style.align(align(c[12]), align(c[13]));
    if c[14] >= 0 {
        style = style.border(
            [
                BorderPreset::Plain,
                BorderPreset::Rounded,
                BorderPreset::Ascii,
                BorderPreset::Thick,
                BorderPreset::Double,
                BorderPreset::Block,
                BorderPreset::OuterHalfBlock,
                BorderPreset::InnerHalfBlock,
                BorderPreset::Hidden,
                BorderPreset::Markdown,
                BorderPreset::None,
            ][c[14] as usize],
        );
    }
    if c[15] >= 0 {
        style.border_sides = Edges {
            top: Some(c[15] & 1 != 0),
            right: Some(c[15] & 2 != 0),
            bottom: Some(c[15] & 4 != 0),
            left: Some(c[15] & 8 != 0),
        }
    }
    style.inline = Some(c[16] != 0);
    style.tab_width = Some(TabWidth::Spaces(c[17] as u16));
    if c[18] != 0 {
        style.padding_char = char::from_u32(c[18] as u32)
    }
    if c[19] != 0 {
        style.margin_char = char::from_u32(c[19] as u32)
    }
    let flags = c[20];
    style.text = StylePatch::new()
        .attribute(TextAttribute::Bold, flags & 1 != 0)
        .attribute(TextAttribute::Italic, flags & 2 != 0)
        .attribute(TextAttribute::Strikethrough, flags & 8 != 0)
        .attribute(TextAttribute::Reverse, flags & 16 != 0)
        .underline_style(if flags & 4 != 0 {
            UnderlineStyle::Single
        } else {
            UnderlineStyle::None
        });
    if flags & 32 != 0 {
        style.text = style
            .text
            .fg(Color::Rgb(180, 35, 88))
            .bg(Color::Rgb(23, 50, 41));
        style.margin_style = StylePatch::new().bg(Color::Rgb(36, 86, 23));
        style.border_style = Edges::all(
            StylePatch::new()
                .fg(Color::Rgb(67, 82, 97))
                .bg(Color::Rgb(18, 103, 67)),
        );
    }
    if flags & 64 != 0 {
        style.text = style.text.underline_color(Color::Rgb(18, 52, 171))
    }
    style.color_whitespace = Some(c[21] != 0);
    style.underline_spaces = (c[22] >= 0).then_some(c[22] != 0);
    style.strikethrough_spaces = (c[23] >= 0).then_some(c[23] != 0);
    style
}
fn color(c: Color) -> String {
    match c.rgb_channels() {
        None => "-".into(),
        Some([r, g, b]) => format!("{r:02x}{g:02x}{b:02x}"),
    }
}
fn cells(frame: &Surface) -> String {
    let mut out = String::new();
    for y in 0..frame.size().height {
        for c in frame.row(y) {
            if c.is_continuation() {
                out.push_str("0,,-,-,-,0,0;");
                continue;
            }
            let style = c.style();
            let mut hex = String::new();
            for byte in c.symbol().bytes() {
                write!(hex, "{byte:02x}").unwrap()
            }
            let mut flags = 0;
            for (flag, bit) in [
                (TextAttribute::Bold, 1),
                (TextAttribute::Italic, 2),
                (TextAttribute::Reverse, 4),
                (TextAttribute::Strikethrough, 8),
                (TextAttribute::Dim, 16),
            ] {
                if style.has(flag) {
                    flags |= bit
                }
            }
            flags |= match style.attributes.blink() {
                BlinkStyle::None => 0,
                BlinkStyle::Slow => 32,
                BlinkStyle::Rapid => 64,
            };
            write!(
                out,
                "{},{},{},{},{},{},{};",
                c.width(),
                hex,
                color(style.fg),
                color(style.bg),
                color(style.underline_color),
                flags,
                style.attributes.underline() as u8
            )
            .unwrap();
        }
    }
    out
}
// Literal corrected content precedes geometry; this oracle never calls the
// production wrapper, alignment or block renderer. Source observations stay
// intact and are asserted to differ for each explicitly reviewed correction.
fn corrected_model(text: &str, c: &[i32]) -> Option<Surface> {
    use unicode_width::UnicodeWidthStr;
    let target = (c[8] - c[1] - c[3]).max(0);
    let (rows, width, height, left, right, align): (Vec<&str>, usize, usize, usize, usize, i32) =
        if c[16] != 0 {
            let value = match c[17] {
                0 => "abc",
                1 => "a bc",
                2 => "a  bc",
                3 => "a   bc",
                4 => "a    bc",
                _ => return None,
            };
            (vec![value], 10, 5, 0, 0, 0)
        } else if c[18] == '茶' as i32 && c[19] == '茶' as i32 {
            (vec!["", "", "  a", "", ""], 5, 5, 0, 0, 0)
        } else {
            let rows = match (text, target) {
                ("茶e\u{301} 👩‍💻", 1) => vec!["茶", "e\u{301}", "👩‍💻"],
                ("one-two-long", 1) => {
                    vec!["o", "n", "e", "-", "t", "w", "o", "-", "l", "o", "n", "g"]
                }
                ("  a b  ", 1 | 2) => vec!["a", "b"],
                _ => return None,
            };
            let width =
                (rows.iter().map(|s| s.width()).max().unwrap() + c[1] as usize + c[3] as usize)
                    .max(c[8] as usize);
            let height = rows.len();
            (rows, width, height, c[3] as usize, c[1] as usize, c[12])
        };
    let mut frame = Surface::new(Size::new(width as u16, height as u16)).unwrap();
    let area = frame.area();
    for (y, row) in rows.iter().enumerate() {
        let spare = width - row.width() - left - right;
        let gap = match align {
            0 => 0,
            1 => spare / 2,
            2 => spare,
            _ => unreachable!(),
        };
        frame
            .canvas(area)
            .text((left + gap) as u16, y as u16, row, Style::default());
    }
    Some(frame)
}

#[test]
fn pinned_block_projection() {
    let mut exact = 0;
    let mut corrected = 0;
    for (number, line) in include_str!("fixtures/blocks.tsv").lines().enumerate() {
        if cfg!(miri) && !number.is_multiple_of(131) {
            continue;
        }
        let f: Vec<_> = line.split('\t').collect();
        assert_eq!(f.len(), 7);
        let c: Vec<i32> = f[2].split(',').map(|s| s.parse().unwrap()).collect();
        let input = unhex(f[1]);
        let block = StyledBlock::new(&input, config(f[2])).unwrap();
        let actual = block.render().unwrap();
        let source_size = Size::new(f[3].parse().unwrap(), f[4].parse().unwrap());
        let output = cells(&actual);
        if let Some(expected) = corrected_model(&input, &c) {
            assert!(
                source_size != expected.size() || f[6] != cells(&expected),
                "stale correction {}",
                f[0]
            );
            assert_eq!(actual.size(), expected.size(), "corrected size {}", f[0]);
            assert_eq!(output, cells(&expected), "corrected cells {}", f[0]);
            corrected += 1;
        } else {
            assert_eq!(actual.size(), source_size, "source size {}", f[0]);
            assert_eq!(output, f[6], "source cells {}", f[0]);
            exact += 1;
        }
    }
    eprintln!("{exact} exact source observations; {corrected} independently modeled corrections");
    assert_eq!(
        (exact, corrected),
        if cfg!(miri) { (17, 0) } else { (2123, 42) }
    );
}

#[test]
fn pinned_border_gradient_perimeter_and_offsets() {
    use matea_style::Gradient;
    use std::sync::Arc;
    let mut checked = 0;
    let mut mismatches = Vec::new();
    for (number, line) in include_str!("fixtures/block-gradients.tsv")
        .lines()
        .enumerate()
    {
        if cfg!(miri) && !number.is_multiple_of(113) {
            continue;
        }
        let f: Vec<_> = line.split('\t').collect();
        assert_eq!(f.len(), 9);
        let mut style = config(f[2]);
        let stops: &[[u8; 3]] = match f[8] {
            "0" => &[[255, 0, 0], [0, 0, 255]],
            "1" => &[[34, 171, 119]],
            "2" => &[[255, 160, 0], [255, 255, 255], [0, 102, 170]],
            _ => panic!(),
        };
        style.border_gradient = Some(Arc::new(Gradient::new(stops).unwrap()));
        style.border_gradient_offset = Some(f[7].parse().unwrap());
        let frame = StyledBlock::new(unhex(f[1]), style)
            .unwrap()
            .render()
            .unwrap();
        assert_eq!(
            frame.size(),
            Size::new(f[3].parse().unwrap(), f[4].parse().unwrap())
        );
        if cells(&frame) != f[6] {
            if mismatches.len() < 3 {
                eprintln!(
                    "gradient {} cfg={} offset={} actual {} source {}",
                    f[0],
                    f[2],
                    f[7],
                    cells(&frame),
                    f[6]
                );
            }
            mismatches.push(f[0]);
        }
        checked += 1;
    }
    assert!(
        mismatches.is_empty(),
        "{} gradient mismatches; first {:?}",
        mismatches.len(),
        &mismatches[..mismatches.len().min(12)]
    );
    assert_eq!(checked, if cfg!(miri) { 9 } else { 960 });
}

#[test]
fn construction_normalizes_after_one_capturing_transform() {
    use matea_style::TextTransform;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let count = Arc::new(AtomicUsize::new(0));
    let captured = count.clone();
    let mut style = BlockStyle::default();
    style.tab_width = Some(TabWidth::Spaces(2));
    style.transform = Some(TextTransform::new(move |text| {
        captured.fetch_add(1, Ordering::Relaxed);
        format!("<{text}>\u{1b}")
    }));
    let block = StyledBlock::new("a\tb\r\nc", style).unwrap();
    assert_eq!(block.text().as_str(), "<a  b\nc>\u{fffd}");
    let clone = block.clone();
    assert_eq!(
        block.text().as_str().as_ptr(),
        clone.text().as_str().as_ptr()
    );
    assert_eq!(
        block.render().unwrap().plain(),
        clone.render().unwrap().plain()
    );
    assert_eq!(count.load(Ordering::Relaxed), 1);
    let mut style = BlockStyle::default();
    style.tab_width = Some(TabWidth::Remove);
    assert_eq!(
        StyledBlock::new("a\tb", style).unwrap().text().as_str(),
        "ab"
    );
    let mut style = BlockStyle::default();
    style.inline = Some(true);
    assert_eq!(
        StyledBlock::new("a\r\nb\nc", style)
            .unwrap()
            .text()
            .as_str(),
        "abc"
    );
}

#[test]
fn validation_and_user_panic_never_publish_partial_blocks() {
    use matea_render::{BlockError, GeometryError, RichText};
    use matea_style::TextTransform;
    for fill in ['\0', '\t', '\n', '\u{1b}', '\u{301}', '\u{200d}'] {
        for margin in [false, true] {
            let mut style = BlockStyle::default();
            if margin {
                style.margin_char = Some(fill)
            } else {
                style.padding_char = Some(fill)
            }
            assert_eq!(
                StyledBlock::new("a", style).unwrap_err(),
                BlockError::InvalidFillCharacter
            );
        }
    }
    let mut style = BlockStyle::default();
    style.transform = Some(TextTransform::new(|_| panic!("user transformation")));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| StyledBlock::new(
            "a",
            style.clone()
        )))
        .is_err()
    );
    assert_eq!(
        StyledBlock::from_rich_text(RichText::default(), style).unwrap_err(),
        BlockError::TransformOnRichText
    );
    let huge =
        StyledBlock::new("a", BlockStyle::default().width(u16::MAX).height(u16::MAX)).unwrap();
    assert!(matches!(huge.render(), Err(BlockError::Geometry(_))));
    assert_eq!(huge.render_within(Size::new(2, 1)).unwrap().plain(), "a");
    let oversized =
        StyledBlock::new("a", BlockStyle::default().padding(Insets::all(u16::MAX))).unwrap();
    assert_eq!(oversized.size(), Err(BlockError::DimensionOverflow));
    assert_eq!(
        oversized.render_within(Size::new(3, 2)).unwrap().size(),
        Size::new(3, 2)
    );
    for error in [
        BlockError::DimensionOverflow,
        BlockError::AllocationFailed,
        BlockError::InvalidFillCharacter,
        BlockError::TransformOnRichText,
        BlockError::Geometry(GeometryError),
    ] {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn rich_styles_survive_inline_cluster_merging_and_block_overrides() {
    use matea_render::RichText;
    let red = Style::default().fg(Color::Indexed(1)).bold();
    let blue = Style::default().fg(Color::Indexed(4));
    let text = RichText::from_spans([("e\n", red), ("\u{301}茶", blue)]);
    let mut style = BlockStyle::default().text_style(StylePatch::new().bg(Color::Indexed(2)));
    style.inline = Some(true);
    let block = StyledBlock::from_rich_text(text, style).unwrap();
    assert_eq!(block.text().as_str(), "e\u{301}茶");
    let frame = block.render().unwrap();
    let row = frame.row(0);
    assert_eq!(row[0].symbol(), "e\u{301}");
    assert_eq!(row[0].style(), red.bg(Color::Indexed(2)));
    assert_eq!(row[1].symbol(), "茶");
    assert_eq!(row[1].style(), blue.bg(Color::Indexed(2)));
    assert!(row[2].is_continuation());
}

#[test]
fn clipping_and_view_composition_preserve_whole_graphemes_and_damage() {
    let block = StyledBlock::new(
        "茶e\u{301}\n👩‍💻",
        BlockStyle::default()
            .border(BorderPreset::Rounded)
            .padding(Insets::symmetric(0, 1)),
    )
    .unwrap();
    let full = block.render().unwrap();
    for width in 0..=full.size().width {
        for height in 0..=full.size().height {
            let size = Size::new(width, height);
            let clipped = block.render_within(size).unwrap();
            let composed = block.view_within(size).unwrap().render(size).unwrap();
            assert_eq!(cells(&clipped), cells(&composed));
            for y in 0..height {
                for (x, cell) in clipped.row(y).iter().enumerate() {
                    if cell.is_continuation() {
                        assert!(x > 0);
                        assert!(!clipped.row(y)[x - 1].is_continuation());
                    } else if cell.width() > 1 {
                        assert!(x + usize::from(cell.width()) <= usize::from(width));
                    }
                }
            }
        }
    }
    assert!(full.diff(Some(&full.clone())).is_empty());
    assert_eq!(
        cells(&block.into_view().unwrap().render(full.size()).unwrap()),
        cells(&full)
    );
}

#[test]
fn generated_wraps_preserve_non_whitespace_and_bound_every_row() {
    let alphabet = ["a", "-", " ", "茶", "e\u{301}"];
    let cases = if cfg!(miri) { 25 } else { 3125 };
    for number in 0..cases {
        let mut code = number;
        let mut text = String::new();
        for _ in 0..5 {
            text.push_str(alphabet[code % 5]);
            code /= 5;
        }
        for width in 1..=6 {
            let block = StyledBlock::new(&text, BlockStyle::default().width(width)).unwrap();
            let frame = block.render().unwrap();
            assert_eq!(
                frame.size().width,
                width.max(if text.contains('茶') { 2 } else { 1 }),
                "size {text:?} target={width}"
            );
            let expected: String = text.chars().filter(|c| !c.is_whitespace()).collect();
            let actual: String = frame
                .plain()
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            assert_eq!(actual, expected, "content {text:?} target={width}");
        }
    }
}

#[test]
fn pinned_custom_border_patterns_and_first_corner() {
    use matea_style::BorderSet;
    let border = BorderSet::custom([
        "-~", "?:", "=~", "|!", "1ab", "2cd", "3ef", "4gh", "5", "6", "7", "8", "9",
    ])
    .unwrap();
    let mut checked = 0;
    for (number, line) in include_str!("fixtures/block-custom.tsv")
        .lines()
        .enumerate()
    {
        if cfg!(miri) && !number.is_multiple_of(13) {
            continue;
        }
        let f: Vec<_> = line.split('\t').collect();
        assert_eq!(f.len(), 7);
        let mut style = config(f[2]);
        style.border = Some(border.clone());
        let frame = StyledBlock::new(unhex(f[1]), style)
            .unwrap()
            .render()
            .unwrap();
        assert_eq!(
            frame.size(),
            Size::new(f[3].parse().unwrap(), f[4].parse().unwrap()),
            "size {}",
            f[0]
        );
        assert_eq!(cells(&frame), f[6], "cells {}", f[0]);
        checked += 1;
    }
    assert_eq!(checked, if cfg!(miri) { 8 } else { 96 });
}

#[test]
fn wide_custom_borders_preserve_clusters_and_fill_unfit_cells() {
    use matea_style::BorderSet;
    let mut style = BlockStyle::default();
    style.border = Some(
        BorderSet::custom([
            "茶",
            "👩‍💻x",
            "茶",
            "e\u{301}",
            "👩‍💻rest",
            "e\u{301}extra",
            "e\u{301}",
            "👩‍💻",
            "",
            "",
            "",
            "",
            "",
        ])
        .unwrap(),
    );
    let frame = StyledBlock::new("ab\nc", style.clone())
        .unwrap()
        .render()
        .unwrap();
    assert_eq!(frame.size(), Size::new(6, 4));
    assert_eq!(
        frame.plain(),
        "👩‍💻茶 e\u{301}\ne\u{301} ab👩‍💻\ne\u{301} c  x\ne\u{301}茶 👩‍💻"
    );
    for width in 1..=6 {
        let cropped = StyledBlock::new("ab\nc", style.clone())
            .unwrap()
            .render_within(Size::new(width, 4))
            .unwrap();
        for y in 0..4 {
            for c in cropped.row(y) {
                assert!(!c.symbol().contains('\u{1b}'));
            }
        }
    }
    let mut side = BlockStyle::default();
    side.border = Some(
        BorderSet::custom([
            "",
            "👩‍💻x",
            "",
            "e\u{301}",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
        ])
        .unwrap(),
    );
    side.border_sides = Edges {
        left: Some(true),
        right: Some(true),
        ..Edges::default()
    };
    let frame = StyledBlock::new("a\nb\nc", side).unwrap().render().unwrap();
    assert_eq!(frame.plain(), "e\u{301}a👩‍💻\ne\u{301}b x\ne\u{301}c👩‍💻");
}

#[test]
fn pinned_inheritance_preserves_explicit_resets_and_child_spacing() {
    let mut checked = 0;
    for (number, line) in include_str!("fixtures/block-inherit.tsv")
        .lines()
        .enumerate()
    {
        if cfg!(miri) && !number.is_multiple_of(13) {
            continue;
        }
        let f: Vec<_> = line.split('\t').collect();
        assert_eq!(f.len(), 8);
        let mode: u8 = f[7].parse().unwrap();
        let parent = config(f[2]);
        let mut child = BlockStyle::default()
            .padding(Insets::all(1))
            .margin(Insets::all(1));
        if mode & 1 != 0 {
            child = child.width(0).height(0);
        }
        if mode & 2 != 0 {
            child.text = StylePatch::new()
                .fg(Color::Default)
                .attribute(TextAttribute::Bold, false)
                .underline_style(UnderlineStyle::None);
        }
        if mode & 4 != 0 {
            child.border_sides.left = Some(false);
            child.margin_style = StylePatch::new().bg(Color::Default);
            child.inline = Some(false);
            child.tab_width = Some(TabWidth::Spaces(0));
            child.padding_char = Some('.');
            child.margin_char = Some('!');
        }
        let frame = StyledBlock::new(unhex(f[1]), child.inherit(&parent))
            .unwrap()
            .render()
            .unwrap();
        assert_eq!(
            frame.size(),
            Size::new(f[3].parse().unwrap(), f[4].parse().unwrap()),
            "size {}",
            f[0]
        );
        assert_eq!(cells(&frame), f[6], "cells {}", f[0]);
        checked += 1;
    }
    assert_eq!(checked, if cfg!(miri) { 5 } else { 64 });
}

#[test]
fn pinned_attribute_groups_and_combining_spaces() {
    let mut checked = 0;
    let mut mismatches = Vec::new();
    for (number, line) in include_str!("fixtures/block-attributes.tsv")
        .lines()
        .enumerate()
    {
        if cfg!(miri) && !number.is_multiple_of(31) {
            continue;
        }
        let f: Vec<_> = line.split('\t').collect();
        assert_eq!(f.len(), 7);
        let c: Vec<i32> = f[2].split(',').map(|v| v.parse().unwrap()).collect();
        let flags = c[20];
        let mut style = config(f[2]);
        style.text = style
            .text
            .attribute(TextAttribute::Dim, flags & 256 != 0)
            .blink(if flags & 128 != 0 {
                BlinkStyle::Slow
            } else {
                BlinkStyle::None
            })
            .underline_style(UnderlineStyle::ALL[((flags >> 10) & 7) as usize]);
        let frame = StyledBlock::new(unhex(f[1]), style)
            .unwrap()
            .render()
            .unwrap();
        assert_eq!(
            frame.size(),
            Size::new(f[3].parse().unwrap(), f[4].parse().unwrap())
        );
        if cells(&frame) != f[6] {
            if mismatches.len() < 3 {
                eprintln!(
                    "attribute {} input {:?} actual {} source {}",
                    f[0],
                    unhex(f[1]),
                    cells(&frame),
                    f[6]
                );
            }
            mismatches.push(f[0]);
        }
        checked += 1;
    }
    assert!(
        mismatches.is_empty(),
        "{} mismatches; first {:?}",
        mismatches.len(),
        &mismatches[..mismatches.len().min(12)]
    );
    assert_eq!(checked, if cfg!(miri) { 10 } else { 288 });
}

#[test]
fn full_corner_patterns_and_wide_right_corners_reserve_a_rectangle() {
    use matea_style::{BorderSet, CornerMode};
    let border = BorderSet::custom([
        " ",
        "│",
        " ",
        "│",
        "┌──",
        "──┐",
        "└──",
        "──┘",
        "",
        "",
        "",
        "",
        "",
    ])
    .unwrap()
    .with_corner_mode(CornerMode::WholePattern)
    .unwrap();
    let full = StyledBlock::new("ab\nc", BlockStyle::default().border(border)).unwrap();
    let frame = full.render().unwrap();
    assert_eq!(frame.size(), Size::new(8, 4));
    assert_eq!(frame.plain(), "┌──  ──┐\n│  ab  │\n│  c   │\n└──  ──┘");
    let border =
        BorderSet::custom(["─", "│", "─", "│", "┌", "⏩", "└", "⏩", "", "", "", "", ""]).unwrap();
    let frame = StyledBlock::new("ab\nc", BlockStyle::default().border(border.clone()))
        .unwrap()
        .render()
        .unwrap();
    assert_eq!(frame.size(), Size::new(5, 4));
    assert_eq!(frame.plain(), "┌──⏩\n│ab │\n│c  │\n└──⏩");
    let empty = StyledBlock::new("", BlockStyle::default().border(border))
        .unwrap()
        .render()
        .unwrap();
    assert_eq!(empty.size(), Size::new(3, 3));
    assert_eq!(empty.plain(), "┌⏩\n│ │\n└⏩");
}

#[test]
fn full_corner_gradients_use_each_cached_cluster_and_signed_offset() {
    use matea_style::{BorderSet, CornerMode, Gradient};
    use std::sync::Arc;
    let border = BorderSet::custom([
        " ",
        "│",
        " ",
        "│",
        "┌──",
        "──┐",
        "└──",
        "──┘",
        "",
        "",
        "",
        "",
        "",
    ])
    .unwrap()
    .with_corner_mode(CornerMode::WholePattern)
    .unwrap();
    let stops: Vec<[u8; 3]> = (0..24).map(|i| [i, 17, 255 - i]).collect();
    let gradient = Arc::new(Gradient::new(&stops).unwrap());
    for offset in [i64::MIN, -49, -1, 0, 1, 49, i64::MAX] {
        let mut style = BlockStyle::default().border(border.clone());
        style.border_gradient = Some(gradient.clone());
        style.border_gradient_offset = Some(offset);
        let frame = StyledBlock::new("ab\nc", style).unwrap().render().unwrap();
        let expected = |index: i128| {
            let i = (index - i128::from(offset)).rem_euclid(24) as u8;
            Color::Rgb(i, 17, 255 - i)
        };
        for x in 0..8 {
            assert_eq!(frame.row(0)[x].style().fg, expected(x as i128));
            assert_eq!(frame.row(3)[x].style().fg, expected(21 - x as i128));
        }
        for y in 1..=2 {
            for x in [0, 1, 2] {
                assert_eq!(frame.row(y)[x].style().fg, expected(24 - y as i128));
            }
            for x in [5, 6, 7] {
                assert_eq!(frame.row(y)[x].style().fg, expected(9 + y as i128));
            }
        }
        for width in 0..=8 {
            let cropped = StyledBlock::new("ab\nc", BlockStyle::default().border(border.clone()))
                .unwrap()
                .render_within(Size::new(width, 4))
                .unwrap();
            assert_eq!(cropped.size(), Size::new(width, 4));
        }
    }
}

#[test]
fn full_corners_clip_whole_clusters_at_each_viewport_edge() {
    use matea_style::{BorderParts, BorderSet, CornerMode};
    let border = BorderSet::custom(BorderParts {
        top: " ",
        bottom: " ",
        left: "|",
        right: "|",
        top_left: "茶e\u{301}",
        top_right: "x👩‍💻",
        bottom_left: "e\u{301}茶",
        bottom_right: "👩‍💻x",
        ..BorderParts::default()
    })
    .unwrap()
    .with_corner_mode(CornerMode::WholePattern)
    .unwrap();
    let block = StyledBlock::new("a", BlockStyle::default().border(border)).unwrap();
    assert_eq!(
        block.render().unwrap().plain(),
        "茶e\u{301} x👩‍💻\n|  a  |\ne\u{301}茶 👩‍💻x"
    );
    assert_eq!(
        block.render_within(Size::new(1, 3)).unwrap().plain(),
        "\n|\ne\u{301}"
    );
    assert_eq!(
        block.render_within(Size::new(2, 3)).unwrap().plain(),
        "茶\n|\ne\u{301}"
    );
    for width in 0..=7 {
        for height in 0..=3 {
            let frame = block.render_within(Size::new(width, height)).unwrap();
            assert_eq!(frame.size(), Size::new(width, height));
            for y in 0..height {
                for (x, c) in frame.row(y).iter().enumerate() {
                    if c.is_continuation() {
                        assert!(x > 0);
                        assert_eq!(frame.row(y)[x - 1].width(), 2);
                    } else if c.width() == 2 {
                        assert!(x + 1 < usize::from(width));
                        assert!(frame.row(y)[x + 1].is_continuation());
                    }
                }
            }
        }
    }
}
