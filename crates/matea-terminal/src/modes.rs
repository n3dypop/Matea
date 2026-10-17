pub use crossterm::event::KeyboardEnhancementFlags as KeyboardFlags;
use crossterm::{
    cursor::{Hide, Show},
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    queue,
    style::ResetColor,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};

/// Explicit keyboard protocol request, not an environment-based capability claim.
/// Kitty mode requires a compatible terminal and is unsupported by the current
/// legacy Windows command backend. Bounded automatic detection remains pending.
/// Include REPORT_ALTERNATE_KEYS for layout-provided shifted text: Crossterm
/// substitutes that character and consumes SHIFT; physical identity is unavailable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyboardMode {
    #[default]
    Legacy,
    Kitty(KeyboardFlags),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputOptions {
    pub mouse: bool,
    pub bracketed_paste: bool,
    pub focus: bool,
    pub keyboard: KeyboardMode,
}
impl Default for InputOptions {
    fn default() -> Self {
        Self {
            mouse: false,
            bracketed_paste: true,
            focus: true,
            keyboard: KeyboardMode::Legacy,
        }
    }
}
impl InputOptions {
    pub(crate) fn validate(self) -> io::Result<()> {
        if let KeyboardMode::Kitty(flags) = self.keyboard
            && !KeyboardFlags::all().contains(flags)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "keyboard flags include capabilities this backend cannot decode",
            ));
        }
        if cfg!(windows) && matches!(self.keyboard, KeyboardMode::Kitty(_)) {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Kitty keyboard requests are not supported by this Crossterm Windows command backend; use Legacy",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Alternate,
    CursorVisibility,
    Style,
    Paste,
    Focus,
    Mouse,
    Keyboard,
}
impl Mode {
    fn bit(self) -> u8 {
        1 << (self as u8)
    }
}
/// No allocation and no global state. A successful release clears only its own
/// lease, so retry after an unrelated failure cannot pop the keyboard stack twice.
#[derive(Debug)]
pub(crate) struct ModeLeases {
    held: u8,
    keyboard: KeyboardFlags,
}
impl Default for ModeLeases {
    fn default() -> Self {
        Self {
            held: 0,
            keyboard: KeyboardFlags::empty(),
        }
    }
}
impl ModeLeases {
    pub(crate) fn active(&self) -> bool {
        self.held != 0
    }
    pub(crate) fn enter(
        &mut self,
        options: InputOptions,
        mut apply: impl FnMut(Mode, bool, KeyboardFlags) -> io::Result<()>,
    ) -> io::Result<()> {
        for mode in [
            Mode::Alternate,
            Mode::CursorVisibility,
            Mode::Style,
            Mode::Paste,
            Mode::Focus,
            Mode::Mouse,
            Mode::Keyboard,
        ] {
            let enabled = match mode {
                Mode::Paste => options.bracketed_paste,
                Mode::Focus => options.focus,
                Mode::Mouse => options.mouse,
                Mode::Keyboard => match options.keyboard {
                    KeyboardMode::Legacy => false,
                    KeyboardMode::Kitty(flags) => {
                        self.keyboard = flags;
                        true
                    }
                },
                _ => true,
            };
            if enabled {
                // A write can partially succeed: retain the attempted lease so
                // cleanup is attempted even if acquisition returns an I/O error.
                self.held |= mode.bit();
                apply(mode, true, self.keyboard)?;
            }
        }
        Ok(())
    }
    pub(crate) fn restore(
        &mut self,
        mut apply: impl FnMut(Mode, bool, KeyboardFlags) -> io::Result<()>,
    ) -> io::Result<()> {
        let mut error = None;
        for mode in [
            Mode::Keyboard,
            Mode::Mouse,
            Mode::Focus,
            Mode::Paste,
            Mode::Style,
            Mode::CursorVisibility,
            Mode::Alternate,
        ] {
            if self.held & mode.bit() == 0 {
                continue;
            }
            // Main/alternate keyboard stacks are independent. A failed pop must
            // be retried while still in the screen on which it was acquired.
            if mode == Mode::Alternate && self.held & Mode::Keyboard.bit() != 0 {
                continue;
            }
            match apply(mode, false, self.keyboard) {
                Ok(()) => self.held &= !mode.bit(),
                Err(e) => {
                    error.get_or_insert(e);
                }
            }
        }
        error.map_or(Ok(()), Err)
    }
}
// Crossterm 0.29 SetAttribute builds an owned SGR String even for Reset.
// Preserve its ANSI bytes and no-op legacy WinAPI behavior without allocation.
struct ResetAttributes;
impl crossterm::Command for ResetAttributes {
    fn write_ansi(&self, out: &mut impl std::fmt::Write) -> std::fmt::Result {
        out.write_str("\x1b[0m")
    }
    #[cfg(windows)]
    fn execute_winapi(&self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn apply_mode(
    out: &mut impl Write,
    mode: Mode,
    enable: bool,
    flags: KeyboardFlags,
) -> io::Result<()> {
    match (mode, enable) {
        (Mode::Alternate, true) => queue!(out, EnterAlternateScreen),
        (Mode::Alternate, false) => queue!(out, LeaveAlternateScreen),
        (Mode::CursorVisibility, true) => queue!(out, Hide),
        (Mode::CursorVisibility, false) => queue!(out, Show),
        (Mode::Style, _) => queue!(out, ResetColor, ResetAttributes),
        (Mode::Paste, true) => queue!(out, EnableBracketedPaste),
        (Mode::Paste, false) => queue!(out, DisableBracketedPaste),
        (Mode::Focus, true) => queue!(out, EnableFocusChange),
        (Mode::Focus, false) => queue!(out, DisableFocusChange),
        (Mode::Mouse, true) => queue!(out, EnableMouseCapture),
        (Mode::Mouse, false) => queue!(out, DisableMouseCapture),
        (Mode::Keyboard, true) => queue!(out, PushKeyboardEnhancementFlags(flags)),
        (Mode::Keyboard, false) => queue!(out, PopKeyboardEnhancementFlags),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn options() -> InputOptions {
        InputOptions {
            mouse: true,
            keyboard: KeyboardMode::Kitty(KeyboardFlags::DISAMBIGUATE_ESCAPE_CODES),
            ..InputOptions::default()
        }
    }
    #[test]
    fn unsupported_keyboard_requests_fail_before_mode_acquisition() {
        let mut options = options();
        options.keyboard = KeyboardMode::Kitty(KeyboardFlags::from_bits_retain(16));
        assert_eq!(
            options.validate().unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        options.keyboard = KeyboardMode::Kitty(KeyboardFlags::DISAMBIGUATE_ESCAPE_CODES);
        assert_eq!(options.validate().is_err(), cfg!(windows));
        options.keyboard = KeyboardMode::Legacy;
        assert!(options.validate().is_ok());
    }
    #[test]
    fn retries_failed_modes_without_repeating_successful_releases() {
        for failure in [
            Mode::Alternate,
            Mode::CursorVisibility,
            Mode::Style,
            Mode::Paste,
            Mode::Focus,
            Mode::Mouse,
            Mode::Keyboard,
        ] {
            let mut lease = ModeLeases::default();
            let mut released = Vec::new();
            lease.enter(options(), |_, _, _| Ok(())).unwrap();
            assert!(
                lease
                    .restore(|mode, _, _| {
                        if mode == failure {
                            Err(io::Error::other("fixture"))
                        } else {
                            released.push(mode);
                            Ok(())
                        }
                    })
                    .is_err()
            );
            assert!(lease.active());
            lease
                .restore(|mode, _, _| {
                    released.push(mode);
                    Ok(())
                })
                .unwrap();
            assert!(!lease.active());
            for mode in [
                Mode::Alternate,
                Mode::CursorVisibility,
                Mode::Style,
                Mode::Paste,
                Mode::Focus,
                Mode::Mouse,
                Mode::Keyboard,
            ] {
                assert_eq!(
                    released.iter().filter(|&&m| m == mode).count(),
                    1,
                    "{failure:?}: {released:?}"
                );
            }
            assert!(
                released.iter().position(|&m| m == Mode::Keyboard).unwrap()
                    < released.iter().position(|&m| m == Mode::Alternate).unwrap()
            );
        }
    }
    #[test]
    fn partial_acquisition_restores_only_attempted_modes_and_is_idempotent() {
        for failure in [
            Mode::Alternate,
            Mode::CursorVisibility,
            Mode::Style,
            Mode::Paste,
            Mode::Focus,
            Mode::Mouse,
            Mode::Keyboard,
        ] {
            let mut lease = ModeLeases::default();
            let mut attempted = Vec::new();
            assert!(
                lease
                    .enter(options(), |mode, _, _| {
                        attempted.push(mode);
                        if mode == failure {
                            Err(io::Error::other("fixture"))
                        } else {
                            Ok(())
                        }
                    })
                    .is_err()
            );
            let mut released = Vec::new();
            lease
                .restore(|mode, _, _| {
                    released.push(mode);
                    Ok(())
                })
                .unwrap();
            assert_eq!(attempted.into_iter().rev().collect::<Vec<_>>(), released);
            lease.restore(|_, _, _| panic!("already restored")).unwrap();
        }
        let mut lease = ModeLeases::default();
        let mut attempted = Vec::new();
        lease
            .enter(
                InputOptions {
                    mouse: false,
                    focus: false,
                    bracketed_paste: false,
                    keyboard: KeyboardMode::Legacy,
                },
                |mode, _, _| {
                    attempted.push(mode);
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(
            attempted,
            vec![Mode::Alternate, Mode::CursorVisibility, Mode::Style]
        );
    }
}
