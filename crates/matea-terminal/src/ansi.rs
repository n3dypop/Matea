//! Commands encoded to bytes must never fall through to WinAPI console calls.
use crossterm::Command;
use matea_style::Color;
use std::{fmt, io};

/// Color policy was resolved by Capabilities; never consult Crossterm's global
/// NO_COLOR/force_color_output state while encoding a deterministic frame.
pub(crate) fn color(w: &mut impl io::Write, color: Color, background: bool) -> io::Result<()> {
    let prefix = if background { 48 } else { 38 };
    match color {
        Color::Default => write!(w, "\x1b[{}m", if background { 49 } else { 39 }),
        Color::Indexed(n @ 0..=7) => write!(w, "\x1b[{}m", (if background { 40 } else { 30 }) + n),
        Color::Indexed(n @ 8..=15) => {
            write!(w, "\x1b[{}m", (if background { 100 } else { 90 }) + n - 8)
        }
        Color::Indexed(n) => write!(w, "\x1b[{prefix};5;{n}m"),
        Color::Rgb(r, g, b) => write!(w, "\x1b[{prefix};2;{r};{g};{b}m"),
    }
}

pub(crate) fn underline_color(w: &mut impl io::Write, color: Color) -> io::Result<()> {
    match color {
        Color::Default => w.write_all(b"\x1b[59m"),
        Color::Indexed(n) => write!(w, "\x1b[58;5;{n}m"),
        Color::Rgb(r, g, b) => write!(w, "\x1b[58;2;{r};{g};{b}m"),
    }
}

pub(crate) fn command(w: &mut impl io::Write, command: impl Command) -> io::Result<()> {
    struct Adapter<'a, W> {
        writer: &'a mut W,
        error: Option<io::Error>,
    }
    impl<W: io::Write> fmt::Write for Adapter<'_, W> {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            self.writer.write_all(text.as_bytes()).map_err(|error| {
                self.error = Some(error);
                fmt::Error
            })
        }
    }
    let mut adapter = Adapter {
        writer: w,
        error: None,
    };
    command.write_ansi(&mut adapter).map_err(|_| {
        adapter
            .error
            .unwrap_or_else(|| io::Error::other("ANSI command formatting failed"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn underline_reset_and_partial_writer_failure_preserve_the_io_contract() {
        let mut bytes = Vec::new();
        underline_color(&mut bytes, Color::Default).unwrap();
        assert_eq!(bytes, b"\x1b[59m");
        struct StopsAfter(usize);
        impl io::Write for StopsAfter {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if self.0 == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "underline fixture",
                    ));
                }
                let count = self.0.min(bytes.len());
                self.0 -= count;
                Ok(count)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        for color in [Color::Default, Color::Indexed(255), Color::Rgb(13, 41, 227)] {
            let error = underline_color(&mut StopsAfter(3), color).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
            assert_eq!(error.to_string(), "underline fixture");
        }
    }
    struct ConsoleOnly;
    impl Command for ConsoleOnly {
        fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
            f.write_str("encoded")
        }
        #[cfg(windows)]
        fn execute_winapi(&self) -> io::Result<()> {
            panic!("pure encoding must not execute a console command")
        }
        #[cfg(windows)]
        fn is_ansi_code_supported(&self) -> bool {
            false
        }
    }
    #[test]
    fn encoding_does_not_consult_console_support_or_execute_winapi() {
        let mut bytes = Vec::new();
        command(&mut bytes, ConsoleOnly).unwrap();
        assert_eq!(bytes, b"encoded");
    }
    #[test]
    fn preserves_original_io_error() {
        struct Broken;
        impl io::Write for Broken {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "original fixture",
                ))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let error = command(&mut Broken, ConsoleOnly).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
        assert_eq!(error.to_string(), "original fixture");
    }
}
