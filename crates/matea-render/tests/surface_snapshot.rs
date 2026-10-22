use matea_render::{Cursor, Rect, Size, Surface, View};
use matea_style::{Color, Style, TextFrame, Theme, UnderlineStyle};
use std::sync::Arc;

fn styled_surface(text: &str, width: u16, height: u16) -> Surface {
    View::styled(
        text,
        Style::default()
            .fg(Color::Indexed(2))
            .bg(Color::Indexed(4))
            .bold()
            .underline_style(UnderlineStyle::Curly)
            .text_frame(TextFrame::Framed),
    )
    .render(Size::new(width, height))
    .unwrap()
}
fn reference_paint(dest: &mut Surface, clip: Rect, x: u16, y: u16, source: &Surface) {
    for row in 0..source.size().height {
        let target_y = u32::from(y) + u32::from(row);
        if target_y > u32::from(u16::MAX) {
            continue;
        }
        for (column, cell) in source.row(row).iter().enumerate() {
            if cell.is_continuation() {
                continue;
            }
            let target_x = u32::from(x) + column as u32;
            if target_x <= u32::from(u16::MAX) {
                dest.canvas(clip).text(
                    target_x as u16,
                    target_y as u16,
                    cell.symbol(),
                    cell.style(),
                );
            }
        }
    }
}
fn valid(surface: &Surface) {
    for y in 0..surface.size().height {
        let row = surface.row(y);
        let mut x = 0;
        while x < row.len() {
            let lead = &row[x];
            assert!(lead.width() > 0);
            assert!(!lead.symbol().is_empty());
            let end = x + usize::from(lead.width());
            assert!(end <= row.len());
            for cell in &row[x + 1..end] {
                assert!(cell.is_continuation());
                assert_eq!(cell.style(), lead.style());
                assert_eq!(cell.symbol(), "");
            }
            x = end;
        }
    }
}
#[test]
fn snapshots_share_owned_immutable_cells_and_keep_the_scene_budget() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<View>();
    send_sync::<Surface>();
    assert_eq!(std::mem::size_of::<View>(), 48);
    let mut source = Arc::new(styled_surface("A茶B\ne\u{301}", 5, 2));
    let expected = source.as_ref().clone();
    let snapshot = View::surface(source.clone());
    let copy = snapshot.clone();
    assert_eq!(Arc::strong_count(&source), 3);
    Arc::make_mut(&mut source)
        .canvas(Rect::new(0, 0, 5, 2))
        .fill(Style::default());
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let expected = &expected;
            let snapshot = &snapshot;
            scope.spawn(move || assert_eq!(snapshot.render(Size::new(5, 2)).unwrap(), *expected));
        }
    });
    drop(source);
    assert_eq!(copy.render(Size::new(5, 2)).unwrap(), expected);
    assert_eq!(
        View::surface(expected.clone())
            .render(Size::new(5, 2))
            .unwrap(),
        expected
    );
}
#[test]
fn natural_height_layout_panels_and_clipped_glyphs_have_literal_results() {
    let surface = View::text("A茶B\nx").render(Size::new(4, 2)).unwrap();
    let scene = View::column([View::surface(surface), View::text("tail")]);
    assert_eq!(
        scene.render(Size::new(6, 3)).unwrap().plain(),
        "A茶B\nx\ntail"
    );
    let mut dest = View::text("茶茶").render(Size::new(4, 1)).unwrap();
    let source = View::text("A茶B").render(Size::new(4, 1)).unwrap();
    dest.canvas(Rect::new(0, 0, 2, 1)).surface(0, 0, &source);
    assert_eq!(dest.plain(), "A 茶");
    valid(&dest);
    let old = dest.clone();
    dest.canvas(Rect::new(2, 0, 1, 1)).surface(
        1,
        0,
        &View::text("茶").render(Size::new(2, 1)).unwrap(),
    );
    assert_eq!(dest, old); // only a clipped source continuation is visible
    let mut theme = Theme::plain();
    theme.padding = 0;
    let panel = View::surface(View::text("tea").render(Size::new(3, 1)).unwrap()).panel("", &theme);
    assert_eq!(
        panel.render(Size::new(5, 3)).unwrap().plain(),
        "+---+\n|tea|\n+---+"
    );
}
#[test]
fn row_clone_matches_independent_per_glyph_paint_for_clips_offsets_and_damage() {
    let texts = [
        "A茶B\n🙂xy",
        "a\u{301}b\n🇯🇵",
        "👩‍💻e\n界",
        "abc\ndef",
        "\u{301}x\n",
        "",
    ];
    let sources: Vec<_> = texts
        .into_iter()
        .map(|text| styled_surface(text, 7, 3))
        .collect();
    let mut a = View::text("茶茶茶茶茶茶\n👩‍💻x🙂\nabcdef\n茶abc")
        .render(Size::new(13, 5))
        .unwrap();
    let mut b = a.clone();
    let mut seed = 317u64;
    let steps = if cfg!(miri) { 96 } else { 4096 };
    for _ in 0..steps {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let source = &sources[(seed as usize >> 8) % sources.len()];
        let x = (seed % 17) as u16;
        let y = ((seed >> 8) % 8) as u16;
        let clip = Rect::new(
            ((seed >> 16) % 17) as u16,
            ((seed >> 24) % 8) as u16,
            ((seed >> 32) % 19) as u16,
            ((seed >> 40) % 9) as u16,
        );
        let cursor = Some(Cursor::new(
            ((seed >> 48) % 13) as u16,
            ((seed >> 56) % 5) as u16,
        ));
        a.set_cursor(cursor);
        b.set_cursor(cursor);
        let old = a.clone();
        a.canvas(clip).surface(x, y, source);
        reference_paint(&mut b, clip, x, y, source);
        assert_eq!(a, b, "x={x} y={y} clip={clip:?}");
        valid(&a);
        assert_eq!(
            a.diff(Some(&old)),
            (0..5)
                .filter(|&row| a.row(row) != old.row(row))
                .collect::<Vec<_>>()
        );
    }
}
#[test]
fn copying_cells_invalidates_old_cursors_and_snapshot_metadata_maps_like_view_cursors() {
    let mut dest = View::text("abcdefgh").render(Size::new(8, 1)).unwrap();
    dest.set_cursor(Some(Cursor::new(4, 0)));
    let source = View::text("XYZ").render(Size::new(3, 1)).unwrap();
    dest.canvas(Rect::new(0, 0, 8, 1)).surface(3, 0, &source);
    assert_eq!(dest.cursor(), None);
    dest.set_cursor(Some(Cursor::new(0, 0)));
    dest.canvas(Rect::new(0, 0, 8, 1)).surface(3, 0, &source);
    assert_eq!(dest.cursor(), Some(Cursor::new(0, 0)));
    let source = View::text("A茶B")
        .with_cursor(Cursor::new(2, 0))
        .render(Size::new(4, 1))
        .unwrap();
    assert_eq!(source.cursor(), Some(Cursor::new(1, 0)));
    let placed = View::layers(vec![(Rect::new(3, 1, 4, 1), View::surface(source.clone()))]);
    assert_eq!(
        placed.render(Size::new(8, 3)).unwrap().cursor(),
        Some(Cursor::new(4, 1))
    );
    let clipped = View::surface(source).width(1);
    assert_eq!(clipped.render(Size::new(4, 1)).unwrap().cursor(), None);
    let override_cursor = View::surface(styled_surface("abc", 3, 1)).with_cursor(Cursor::new(2, 0));
    assert_eq!(
        override_cursor.render(Size::new(3, 1)).unwrap().cursor(),
        Some(Cursor::new(2, 0))
    );
}
#[test]
fn extreme_and_empty_geometry_never_wraps_or_publishes_orphan_continuations() {
    let source = styled_surface("茶x", 3, 1);
    let mut dest = Surface::new(Size::new(3, 2)).unwrap();
    let empty = dest.clone();
    for (x, y, clip) in [
        (u16::MAX, 0, dest.area()),
        (0, u16::MAX, dest.area()),
        (0, 0, Rect::new(u16::MAX, 0, u16::MAX, 1)),
        (0, 0, Rect::new(0, 0, 0, 1)),
    ] {
        dest.canvas(clip).surface(x, y, &source);
        assert_eq!(dest, empty);
    }
    for size in [Size::new(0, 0), Size::new(0, 5), Size::new(5, 0)] {
        let source = Surface::new(size).unwrap();
        dest.canvas(dest.area()).surface(0, 0, &source);
        assert_eq!(dest, empty);
        assert_eq!(
            View::surface(source)
                .render(Size::new(0, 2))
                .unwrap()
                .size(),
            Size::new(0, 2)
        );
    }
}
#[test]
fn long_heap_graphemes_and_nondefault_blank_cells_are_owned_exactly() {
    let text = format!("a{}茶", "\u{301}".repeat(40));
    let source = styled_surface(&text, 5, 1);
    let mut dest = Surface::new(Size::new(7, 1)).unwrap();
    dest.canvas(dest.area()).surface(1, 0, &source);
    assert_eq!(
        dest.row(0)[1].symbol(),
        format!("a{}", "\u{301}".repeat(40))
    );
    assert_eq!(dest.row(0)[1].style(), source.row(0)[0].style());
    valid(&dest);
    let mut blank = Surface::new(Size::new(3, 1)).unwrap();
    blank
        .canvas(blank.area())
        .fill(Style::default().bg(Color::Indexed(9)));
    dest.canvas(dest.area()).surface(2, 0, &blank);
    assert!(
        dest.row(0)[2..5]
            .iter()
            .all(|cell| cell.symbol() == " " && cell.style().bg == Color::Indexed(9))
    );
    valid(&dest);
}

#[test]
fn consuming_unique_roots_transfers_cells_while_shared_and_constrained_scenes_remain_independent() {
    let source = styled_surface("a茶", 5, 2);
    let address = source.row(0).as_ptr();
    let expected = source.clone();
    let result = View::surface(source).into_surface(Size::new(5, 2)).unwrap();
    assert_eq!(result.row(0).as_ptr(), address);
    assert_eq!(result, expected);
    let shared = Arc::new(expected);
    let address = shared.row(0).as_ptr();
    let mut result = View::surface(shared.clone())
        .into_surface(Size::new(5, 2))
        .unwrap();
    assert_ne!(result.row(0).as_ptr(), address);
    assert_eq!(result, *shared);
    result.canvas(result.area()).fill(Style::default());
    assert_ne!(result, *shared);
    for size in [Size::new(1, 1), Size::new(5, 2), Size::new(8, 4)] {
        for scene in [
            View::surface(shared.clone()),
            View::surface(shared.clone()).width(1),
            View::surface(shared.clone()).height(1),
            View::column([View::surface(shared.clone()), View::text("tail")]),
        ] {
            let expected = scene.render(size).unwrap();
            assert_eq!(scene.into_surface(size).unwrap(), expected);
        }
    }
    assert!(
        View::surface(shared)
            .into_surface(Size::new(u16::MAX, u16::MAX))
            .is_err()
    );
}
