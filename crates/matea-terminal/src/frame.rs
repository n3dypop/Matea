use super::{Capabilities, ansi::command, set_style};
use crossterm::{
    cursor::{Hide, MoveDown, MoveLeft, MoveRight, MoveTo, MoveUp, Show},
    terminal::{Clear, ClearType},
};
use matea_render::Surface;
use matea_style::{Color, ColorLevel, Style};
use std::io::{self, Write};

/// Logical, immutable palette results are independent of physical terminal state.
/// Eight entries cover common theme tokens without a growing global color map.
#[derive(Default, Debug)]
struct QuantizedStyles {
    level: Option<ColorLevel>,
    entries: [Option<(Style, Style)>; 8],
    next: usize,
}
impl QuantizedStyles {
    fn adapt(&mut self, source: Style, level: ColorLevel) -> Style {
        if matches!(level, ColorLevel::None | ColorLevel::TrueColor) {
            return source.adapt(level);
        }
        let expensive = |color| match color {
            Color::Rgb(..) => matches!(level, ColorLevel::Ansi16 | ColorLevel::Ansi256),
            Color::Indexed(n) => level == ColorLevel::Ansi16 && n >= 16,
            Color::Default => false,
        };
        if !expensive(source.fg) && !expensive(source.bg) {
            return source.adapt(level);
        }
        if self.level != Some(level) {
            self.entries.fill(None);
            self.next = 0;
            self.level = Some(level);
        }
        if let Some((_, result)) = self
            .entries
            .iter()
            .flatten()
            .find(|(key, _)| *key == source)
        {
            return *result;
        }
        let result = source.adapt(level);
        self.entries[self.next] = Some((source, result));
        self.next = (self.next + 1) % self.entries.len();
        result
    }
}

/// Stateful frame encoding for exclusive terminal writers. Reuse with the actual
/// previous Surface; call invalidate after external output, graphics or cursor
/// movement. Encodes cursor visibility and position without acquiring a terminal
/// or writing on Drop. The terminal session owns restoration.
#[derive(Default, Debug)]
pub struct FrameEncoder {
    cursor: Option<(u16, u16)>,
    style: Option<Style>,
    caps: Option<Capabilities>,
    invalidated: bool,
    visible: Option<bool>,
    quantized: QuantizedStyles,
}
impl FrameEncoder {
    /// The next write clears and repaints completely, including empty geometry.
    pub fn invalidate(&mut self) {
        self.cursor = None;
        self.style = None;
        self.caps = None;
        self.invalidated = true;
        self.visible = None;
    }
    pub fn write(
        &mut self,
        w: &mut impl Write,
        frame: &Surface,
        previous: Option<&Surface>,
        caps: Capabilities,
    ) -> io::Result<()> {
        let result = self.encode(w, frame, previous, caps);
        if result.is_err() {
            self.invalidate();
        }
        result
    }
    fn position(&mut self, w: &mut impl Write, x: u16, y: u16, width: u16) -> io::Result<()> {
        if self.cursor == Some((x, y)) {
            return Ok(());
        }
        match self.cursor {
            Some((_, old_y)) if y == old_y && x == 0 => w.write_all(b"\r")?,
            // CR cancels delayed wrap before LF advances one valid surface row.
            // Callers only request in-bounds cells, so this cannot scroll below
            // the viewport; unknown positions still need an absolute anchor.
            Some((_, old_y)) if x == 0 && old_y.checked_add(1) == Some(y) => {
                w.write_all(b"\r\n")?
            }
            Some((old_x, old_y))
                if y == old_y && old_x < width && x.checked_add(1) == Some(old_x) =>
            {
                w.write_all(b"\x08")?
            }
            Some((old_x, old_y))
                if old_x < width && y == old_y && relative_is_shorter(x.abs_diff(old_x), x, y) =>
            {
                if x > old_x {
                    command(w, MoveRight(x - old_x))?;
                } else {
                    command(w, MoveLeft(old_x - x))?;
                }
            }
            Some((old_x, old_y))
                if old_x < width && x == old_x && relative_is_shorter(y.abs_diff(old_y), x, y) =>
            {
                if y > old_y {
                    command(w, MoveDown(y - old_y))?;
                } else {
                    command(w, MoveUp(old_y - y))?;
                }
            }
            _ if x == 0 && y == 0 => w.write_all(b"\x1b[H")?,
            _ => command(w, MoveTo(x, y))?,
        }
        self.cursor = Some((x, y));
        Ok(())
    }
    fn style(&mut self, w: &mut impl Write, style: Style) -> io::Result<()> {
        if self.style != Some(style) {
            set_style(w, style)?;
            self.style = Some(style);
        }
        Ok(())
    }
    fn encode(
        &mut self,
        w: &mut impl Write,
        frame: &Surface,
        previous: Option<&Surface>,
        caps: Capabilities,
    ) -> io::Result<()> {
        // A supplied previous frame describes the actual terminal state, even
        // for the stateless write_frame convenience function's first call.
        if self.caps.is_none() && !self.invalidated {
            self.visible = previous.map(|old| old.cursor().is_some());
            self.cursor = previous.and_then(|old| old.cursor().map(|c| (c.x, c.y)));
        }
        let previous = if self.invalidated || self.caps.is_some_and(|old| old != caps) {
            None
        } else {
            previous
        };
        let resized = previous.is_some_and(|old| old.size() != frame.size());
        let full = previous.is_none() || resized;
        let mut rows = frame.changed_rows(previous).peekable();
        let painting = rows.peek().is_some() || full;
        let hint = frame.cursor();
        let visible = hint.is_some();
        let positioned = hint.is_none_or(|c| self.cursor == Some((c.x, c.y)));
        if !painting && self.visible == Some(visible) && positioned {
            return Ok(());
        }
        if full && self.caps.is_some() {
            self.visible = None;
        }
        if (painting || !visible) && self.visible != Some(false) {
            command(w, Hide)?;
            self.visible = Some(false);
        }
        if full {
            // None is also the driver's recovery signal after external output.
            // Clearing cells alone cannot establish the current SGR/cursor.
            self.style = None;
            self.cursor = None;
            self.style(w, Style::default())?;
            command(w, Clear(ClearType::All))?;
        }
        for y in rows {
            let old = previous.filter(|old| old.size() == frame.size());
            let ascii = frame
                .row(y)
                .iter()
                .chain(old.into_iter().flat_map(|old| old.row(y)))
                .all(|cell| {
                    cell.width() == 1 && cell.symbol().len() == 1 && cell.symbol().is_ascii()
                });
            if !ascii {
                self.style(w, Style::default())?;
                self.position(w, 0, y, frame.size().width)?;
                command(w, Clear(ClearType::CurrentLine))?;
            }
            for (x, cell) in frame
                .row(y)
                .iter()
                .enumerate()
                .filter(|(_, cell)| !cell.is_continuation())
            {
                if ascii && old.is_some_and(|old| old.row(y)[x] == *cell) {
                    continue;
                }
                let style = self.quantized.adapt(cell.style(), caps.color);
                if (!ascii || full) && cell.symbol() == " " && style == Style::default() {
                    continue;
                }
                self.style(w, style)?;
                self.position(w, x as u16, y, frame.size().width)?;
                let symbol = if !caps.unicode && !cell.symbol().is_ascii() {
                    "?"
                } else {
                    cell.symbol()
                };
                w.write_all(symbol.as_bytes())?;
                self.cursor = if cell.width() == 1 && symbol.len() == 1 && symbol.is_ascii() {
                    Some((x as u16 + 1, y))
                } else {
                    None
                };
            }
        }
        if let Some(c) = hint {
            self.position(w, c.x, c.y, frame.size().width)?;
            if self.visible != Some(true) {
                command(w, Show)?;
                self.visible = Some(true);
            }
        }
        w.flush()?;
        self.invalidated = false;
        self.caps = Some(caps);
        Ok(())
    }
}
fn relative_is_shorter(distance: u16, x: u16, y: u16) -> bool {
    fn digits(value: u16) -> u16 {
        match value {
            0..=9 => 1,
            10..=99 => 2,
            100..=999 => 3,
            1000..=9999 => 4,
            _ => 5,
        }
    }
    3 + digits(distance) < 4 + digits(x.saturating_add(1)) + digits(y.saturating_add(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use matea_render::{Cursor, Size, View};
    fn caps() -> Capabilities {
        Capabilities {
            color: ColorLevel::None,
            unicode: true,
            reduced_motion: true,
        }
    }
    #[test]
    fn quantization_remains_correct_after_eviction_level_changes_and_attributes() {
        let mut cache = QuantizedStyles::default();
        let styles: Vec<_> = (0..40)
            .map(|n| Style {
                fg: Color::Rgb(n * 5, 135, 175),
                bg: Color::Indexed(n + 16),
                bold: n % 2 == 0,
                underline: n % 3 == 0,
                reverse: n % 5 == 0,
            })
            .collect();
        for level in [
            ColorLevel::Ansi256,
            ColorLevel::Ansi16,
            ColorLevel::None,
            ColorLevel::TrueColor,
            ColorLevel::Ansi256,
        ] {
            for style in styles.iter().chain(styles.iter().rev()) {
                assert_eq!(cache.adapt(*style, level), style.adapt(level));
            }
        }
    }
    #[test]
    fn logical_color_cache_survives_recovery_without_reusing_physical_pen_state() {
        let frame = View::styled(
            "tea",
            Style::default()
                .fg(Color::Rgb(95, 135, 175))
                .bg(Color::Rgb(175, 95, 135)),
        )
        .render(Size::new(4, 1))
        .unwrap();
        let mut encoder = FrameEncoder::default();
        for color in [
            ColorLevel::Ansi256,
            ColorLevel::Ansi16,
            ColorLevel::None,
            ColorLevel::TrueColor,
            ColorLevel::Ansi256,
        ] {
            let capabilities = Capabilities { color, ..caps() };
            let mut fresh = Vec::new();
            FrameEncoder::default()
                .write(&mut fresh, &frame, None, capabilities)
                .unwrap();
            encoder.invalidate();
            let mut recovered = Vec::new();
            encoder
                .write(&mut recovered, &frame, Some(&frame), capabilities)
                .unwrap();
            assert_eq!(recovered, fresh);
        }
    }
    #[test]
    fn cursor_only_updates_show_move_hide_without_repainting_cells() {
        let a = View::text("tea")
            .with_cursor(Cursor::new(0, 0))
            .render(Size::new(4, 1))
            .unwrap();
        let mut b = a.clone();
        b.set_cursor(Some(Cursor::new(2, 0)));
        let mut encoder = FrameEncoder::default();
        let mut out = Vec::new();
        encoder.write(&mut out, &a, None, caps()).unwrap();
        assert!(out.starts_with(b"\x1b[?25l"));
        assert!(out.ends_with(b"\r\x1b[?25h"));
        out.clear();
        encoder.write(&mut out, &b, Some(&a), caps()).unwrap();
        assert_eq!(out, b"\x1b[2C");
        out.clear();
        encoder.write(&mut out, &b, Some(&b), caps()).unwrap();
        assert!(out.is_empty());
        let mut hidden = b.clone();
        hidden.set_cursor(None);
        encoder.write(&mut out, &hidden, Some(&b), caps()).unwrap();
        assert_eq!(out, b"\x1b[?25l");
    }
    #[test]
    fn visible_cursor_is_hidden_during_damage_then_restored_after_paint() {
        let a = View::text("tea")
            .with_cursor(Cursor::new(1, 0))
            .render(Size::new(4, 1))
            .unwrap();
        let b = View::text("Tea")
            .with_cursor(Cursor::new(1, 0))
            .render(Size::new(4, 1))
            .unwrap();
        let mut encoder = FrameEncoder::default();
        encoder.write(&mut Vec::new(), &a, None, caps()).unwrap();
        let mut out = Vec::new();
        encoder.write(&mut out, &b, Some(&a), caps()).unwrap();
        assert_eq!(out, b"\x1b[?25l\rT\x1b[?25h");
        encoder.invalidate();
        out.clear();
        encoder.write(&mut out, &b, Some(&b), caps()).unwrap();
        assert!(out.starts_with(b"\x1b[?25l"));
        assert!(out.ends_with(b"\x1b[?25h"));
    }
    #[test]
    fn cursor_moves_use_short_relative_sequences_only_from_known_positions() {
        let frame = View::text("").render(Size::new(12, 12)).unwrap();
        let mut encoder = FrameEncoder::default();
        let mut previous = frame.clone();
        previous.set_cursor(Some(Cursor::new(2, 3)));
        encoder
            .write(&mut Vec::new(), &previous, None, caps())
            .unwrap();
        for (cursor, expected) in [
            (Cursor::new(4, 3), &b"\x1b[2C"[..]),
            (Cursor::new(2, 3), &b"\x1b[2D"[..]),
            (Cursor::new(2, 5), &b"\x1b[2B"[..]),
            (Cursor::new(2, 3), &b"\x1b[2A"[..]),
            (Cursor::new(1, 3), &b"\x08"[..]),
            (Cursor::new(0, 3), &b"\r"[..]),
            (Cursor::new(2, 4), &b"\x1b[5;3H"[..]),
        ] {
            let mut next = frame.clone();
            next.set_cursor(Some(cursor));
            let mut out = Vec::new();
            encoder
                .write(&mut out, &next, Some(&previous), caps())
                .unwrap();
            assert_eq!(out, expected);
            previous = next;
        }
        encoder.cursor = Some((12, 3)); // delayed-wrap position must reanchor
        let mut out = Vec::new();
        encoder.position(&mut out, 11, 3, 12).unwrap();
        assert_eq!(out, b"\x1b[4;12H");
        assert!(!relative_is_shorter(65535, 0, 0));
    }
    #[test]
    fn one_cell_updates_are_two_bytes_and_invalidation_repaints() {
        let a = View::text("aaaa").render(Size::new(4, 1)).unwrap();
        let b = View::text("baaa").render(Size::new(4, 1)).unwrap();
        let mut encoder = FrameEncoder::default();
        let mut out = Vec::new();
        encoder.write(&mut out, &a, None, caps()).unwrap();
        for (frame, old, expected) in [(&b, &a, &b"\rb"[..]), (&a, &b, &b"\ra"[..])] {
            out.clear();
            encoder.write(&mut out, frame, Some(old), caps()).unwrap();
            assert_eq!(out, expected);
        }
        encoder.invalidate();
        out.clear();
        encoder.write(&mut out, &a, Some(&a), caps()).unwrap();
        assert!(out.windows(4).any(|w| w == b"aaaa"));
    }
    #[test]
    fn row_advances_cancel_wrap_and_bottom_row_moves_do_not_scroll() {
        let mut encoder = FrameEncoder::default();
        let mut out = Vec::new();
        encoder.position(&mut out, 0, 0, 80).unwrap();
        assert_eq!(out, b"\x1b[H");
        for old_x in [0, 4, 80] {
            encoder.cursor = Some((old_x, 22));
            out.clear();
            encoder.position(&mut out, 0, 23, 80).unwrap();
            assert_eq!(out, b"\r\n");
            encoder.cursor = Some((80, 23));
            out.clear();
            encoder.position(&mut out, 0, 23, 80).unwrap();
            assert_eq!(out, b"\r");
        }
        encoder.cursor = None;
        out.clear();
        encoder.position(&mut out, 0, 23, 80).unwrap();
        assert_eq!(out, b"\x1b[24;1H");
    }
    #[test]
    fn io_failure_invalidates_cache_and_capability_change_repaints() {
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("fixture"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let frame = View::text("茶").render(Size::new(3, 1)).unwrap();
        let mut encoder = FrameEncoder::default();
        assert!(encoder.write(&mut Broken, &frame, None, caps()).is_err());
        let mut out = Vec::new();
        encoder
            .write(&mut out, &frame, Some(&frame), caps())
            .unwrap();
        assert!(out.windows(3).any(|w| w == "茶".as_bytes()));
        out.clear();
        let mut plain = caps();
        plain.unicode = false;
        encoder
            .write(&mut out, &frame, Some(&frame), plain)
            .unwrap();
        assert!(out.contains(&b'?'));
    }
    #[test]
    fn changing_ascii_row_does_not_reemit_unchanged_wide_row() {
        let a = View::text("茶 👩‍💻\na").render(Size::new(10, 2)).unwrap();
        let b = View::text("茶 👩‍💻\nb").render(Size::new(10, 2)).unwrap();
        let mut encoder = FrameEncoder::default();
        let mut out = Vec::new();
        encoder.write(&mut out, &a, None, caps()).unwrap();
        out.clear();
        encoder.write(&mut out, &b, Some(&a), caps()).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(!output.contains('茶'));
        assert!(!output.contains('👩'));
        assert!(output.contains('b'));
    }
    #[test]
    fn missing_previous_forces_unknown_cursor_and_style_reset() {
        let frame = View::text("aaaa").render(Size::new(4, 1)).unwrap();
        let mut encoder = FrameEncoder::default();
        let mut out = Vec::new();
        encoder.write(&mut out, &frame, None, caps()).unwrap();
        out.clear();
        encoder.write(&mut out, &frame, None, caps()).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("\x1b[0m"));
        assert!(output.contains("\x1b[H"));
        assert!(output.contains("\x1b[2J"));
    }
}
