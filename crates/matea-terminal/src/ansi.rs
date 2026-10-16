//! Commands encoded to bytes must never fall through to WinAPI console calls.
use crossterm::Command;
use std::{fmt, io};

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
