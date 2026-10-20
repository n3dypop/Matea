use super::{DeviceIo, Ending, TerminalFile, Utf16Decoder};
use matea_render::Size;
use std::{
    cell::Cell,
    io::{self, Read},
    mem::MaybeUninit,
    os::windows::io::{AsRawHandle, BorrowedHandle},
};
use windows_sys::Win32::System::Console::*;

#[derive(Clone)]
pub(super) struct State(u32);
impl State {
    pub(super) const fn mode(&self) -> u32 {
        self.0
    }
    pub(super) const fn from_mode(mode: u32) -> Self {
        Self(mode)
    }
}
pub(super) struct Native<'fd> {
    handle: BorrowedHandle<'fd>,
    input_validated: Cell<bool>,
    decoder: Utf16Decoder,
    utf8: [u8; 4],
    start: usize,
    end: usize,
}
impl<'fd> Native<'fd> {
    pub(super) fn new(file: &'fd (impl TerminalFile + ?Sized)) -> Self {
        Self {
            handle: file.as_handle(),
            input_validated: Cell::new(false),
            decoder: Utf16Decoder::default(),
            utf8: [0; 4],
            start: 0,
            end: 0,
        }
    }
    fn ensure_input(&self) -> io::Result<()> {
        if self.input_validated.get() {
            return Ok(());
        }
        let mut count = 0;
        // SAFETY: handle is borrowed/open and count is a writable DWORD output.
        if unsafe { GetNumberOfConsoleInputEvents(self.handle.as_raw_handle(), &mut count) } == 0 {
            return Err(io::Error::last_os_error());
        }
        self.input_validated.set(true);
        Ok(())
    }
}
fn console_unit(handle: BorrowedHandle<'_>) -> io::Result<Option<u16>> {
    let mut unit = 0u16;
    let mut count = 0;
    // SAFETY: the borrowed console handle remains open. The output holds one
    // UTF-16 unit and count is writable; no optional input-control pointer is used.
    let result = unsafe {
        ReadConsoleW(
            handle.as_raw_handle(),
            (&mut unit as *mut u16).cast(),
            1,
            &mut count,
            std::ptr::null(),
        )
    };
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok((count != 0).then_some(unit))
    }
}
impl DeviceIo for Native<'_> {
    type State = State;
    fn state(&self) -> io::Result<State> {
        let mut mode = 0;
        // SAFETY: handle is borrowed/open and mode is a writable DWORD output.
        if unsafe { GetConsoleMode(self.handle.as_raw_handle(), &mut mode) } == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(State(mode))
        }
    }
    fn set_state(&mut self, state: &State) -> io::Result<()> {
        // SAFETY: handle stays open; mode is a scalar captured/derived console state.
        if unsafe { SetConsoleMode(self.handle.as_raw_handle(), state.0) } == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
    fn raw_state(&self, state: &State) -> io::Result<State> {
        self.ensure_input()?;
        Ok(State(
            (state.0 & !(ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT | ENABLE_LINE_INPUT))
                | ENABLE_VIRTUAL_TERMINAL_INPUT,
        ))
    }
    fn password_state(&self, state: &State) -> io::Result<State> {
        self.ensure_input()?;
        Ok(State(
            (state.0 & !(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT)) | ENABLE_PROCESSED_INPUT,
        ))
    }
    fn ending(&self) -> Ending {
        Ending::Cr
    }
    fn size(&self) -> io::Result<Size> {
        let mut info = MaybeUninit::<CONSOLE_SCREEN_BUFFER_INFO>::uninit();
        // SAFETY: open borrowed output handle and correctly aligned output
        // structure; GetConsoleScreenBufferInfo initializes it on success.
        if unsafe { GetConsoleScreenBufferInfo(self.handle.as_raw_handle(), info.as_mut_ptr()) }
            == 0
        {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: verified successful initialization before reading fields.
        let info = unsafe { info.assume_init() };
        let width =
            u16::try_from(i32::from(info.srWindow.Right) - i32::from(info.srWindow.Left) + 1)
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
        let height =
            u16::try_from(i32::from(info.srWindow.Bottom) - i32::from(info.srWindow.Top) + 1)
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
        Ok(Size::new(width, height))
    }
}
impl Read for Native<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        if self.start == self.end {
            self.ensure_input()?;
            let handle = self.handle;
            let Some(value) = self.decoder.next(|| console_unit(handle))? else {
                return Ok(0);
            };
            self.start = 0;
            self.end = value.encode_utf8(&mut self.utf8).len();
        }
        bytes[0] = self.utf8[self.start];
        self.start += 1;
        Ok(1)
    }
}
