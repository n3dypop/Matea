use super::{Capabilities, set_style};
use crossterm::{
    cursor::MoveTo,
    queue,
    style::Print,
    terminal::{Clear, ClearType},
};
use matea_render::Surface;
use matea_style::Style;
use std::io::{self, Write};

/// Stateful frame encoding for exclusive terminal writers. Reuse with the actual
/// previous Surface; call invalidate after external output, graphics or cursor
/// movement. The encoder never owns a terminal, hides a cursor, or writes on Drop.
#[derive(Default, Debug)]
pub struct FrameEncoder {
    cursor: Option<(u16, u16)>,
    style: Option<Style>,
    caps: Option<Capabilities>,
    invalidated: bool,
}
impl FrameEncoder {
    /// The next write clears and repaints completely, including empty geometry.
    pub fn invalidate(&mut self) {
        self.cursor = None;
        self.style = None;
        self.caps = None;
        self.invalidated = true;
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
            Some((old_x, old_y))
                if y == old_y && old_x < width && x.checked_add(1) == Some(old_x) =>
            {
                w.write_all(b"\x08")?
            }
            _ => queue!(w, MoveTo(x, y))?,
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
        let previous = if self.invalidated || self.caps.is_some_and(|old| old != caps) {
            None
        } else {
            previous
        };
        let resized = previous.is_some_and(|old| old.size() != frame.size());
        let full = previous.is_none() || resized;
        let mut rows = frame.changed_rows(previous).peekable();
        if rows.peek().is_none() && !full {
            return Ok(());
        }
        if full {
            self.style(w, Style::default())?;
            queue!(w, Clear(ClearType::All))?;
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
                queue!(w, Clear(ClearType::CurrentLine))?;
            }
            for (x, cell) in frame
                .row(y)
                .iter()
                .enumerate()
                .filter(|(_, cell)| !cell.is_continuation())
            {
                let style = cell.style().adapt(caps.color);
                if ascii && old.is_some_and(|old| old.row(y)[x] == *cell) {
                    continue;
                }
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
                queue!(w, Print(symbol))?;
                self.cursor = if cell.width() == 1 && symbol.len() == 1 && symbol.is_ascii() {
                    Some((x as u16 + 1, y))
                } else {
                    None
                };
            }
        }
        w.flush()?;
        self.invalidated = false;
        self.caps = Some(caps);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matea_render::{Size, View};
    use matea_style::ColorLevel;
    fn caps() -> Capabilities {
        Capabilities {
            color: ColorLevel::None,
            unicode: true,
            reduced_motion: true,
        }
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
}
