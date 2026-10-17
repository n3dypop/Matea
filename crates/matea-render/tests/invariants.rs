use matea_render::{Cursor, Rect, Size, Surface};
use matea_style::{Color, Style};

#[test]
fn bounded_paint_sequences_preserve_glyph_ownership_and_determinism() {
    let mut a = Surface::new(Size::new(23, 7)).unwrap();
    let mut b = a.clone();
    let texts = [
        "",
        "abc",
        "茶",
        "👩‍💻",
        "e\u{301}",
        "\u{301}",
        "a\tb\x1b",
        "a\nb",
        "مرحبا",
    ];
    let mut seed = 17u64;
    let steps = if cfg!(miri) { 32 } else { 4096 };
    for step in 0..steps {
        let previous = a.clone();
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let x = (seed % 28) as u16;
        let y = ((seed >> 8) % 10) as u16;
        let clip = Rect::new(
            ((seed >> 16) % 25) as u16,
            ((seed >> 24) % 8) as u16,
            ((seed >> 32) % 30) as u16,
            ((seed >> 40) % 10) as u16,
        );
        let text = texts[(seed >> 48) as usize % texts.len()];
        let style = Style::default().fg(Color::Indexed(seed as u8));
        for s in [&mut a, &mut b] {
            s.set_cursor(Some(Cursor::new(x, y)));
            let mut canvas = s.canvas(clip);
            if step % 7 == 0 {
                canvas.fill(style)
            } else {
                canvas.text(x, y, text, style)
            }
        }
        assert_eq!(a, b, "step {step}");
        let expected: Vec<_> = (0..7).filter(|&y| a.row(y) != previous.row(y)).collect();
        assert_eq!(a.diff(Some(&previous)), expected);
        for y in 0..7 {
            let row = a.row(y);
            let mut x = 0;
            while x < row.len() {
                let lead = &row[x];
                assert!(lead.width() > 0, "orphan continuation at {x},{y}");
                let end = x + usize::from(lead.width());
                assert!(end <= row.len());
                assert!(!lead.symbol().is_empty());
                assert!(!lead.symbol().chars().any(char::is_control));
                for cell in &row[x + 1..end] {
                    assert!(cell.is_continuation());
                    assert_eq!(cell.symbol(), "");
                    assert_eq!(cell.style(), lead.style());
                }
                x = end;
            }
        }
    }
}

#[test]
fn extreme_empty_clips_never_write_or_wrap_coordinates() {
    let mut s = Surface::new(Size::new(3, 2)).unwrap();
    let blank = s.clone();
    for clip in [
        Rect::new(u16::MAX, 0, u16::MAX, 1),
        Rect::new(0, u16::MAX, 1, u16::MAX),
        Rect::new(1, 1, 0, 0),
    ] {
        s.canvas(clip).text(0, 0, "茶\nabc", Style::default());
        s.canvas(clip).fill(Style::default().bold());
        assert_eq!(s, blank);
    }
}
