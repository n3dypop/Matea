use super::{DeviceIo, Ending, PasswordError, TerminalFile, read_line_buffered};
use matea_render::Size;
use std::{
    io::{self, Read},
    mem::MaybeUninit,
    os::fd::{AsRawFd, BorrowedFd},
};

#[derive(Clone)]
pub(super) struct State(libc::termios);
pub(super) struct Native<'fd> {
    fd: BorrowedFd<'fd>,
}
impl<'fd> Native<'fd> {
    pub(super) fn new(file: &'fd (impl TerminalFile + ?Sized)) -> Self {
        Self { fd: file.as_fd() }
    }
}
impl DeviceIo for Native<'_> {
    type State = State;
    fn state(&self) -> io::Result<State> {
        let mut state = MaybeUninit::<libc::termios>::uninit();
        // SAFETY: borrowed fd remains open; the correctly aligned output points
        // to writable termios storage. POSIX initializes its fields on success.
        let result = unsafe { libc::tcgetattr(self.fd.as_raw_fd(), state.as_mut_ptr()) };
        if result == -1 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: tcgetattr succeeded. We never inspect/serialize padding bytes.
        Ok(State(unsafe { state.assume_init() }))
    }
    fn set_state(&mut self, state: &State) -> io::Result<()> {
        // SAFETY: fd is borrowed/open and state is an initialized termios value.
        let result = unsafe { libc::tcsetattr(self.fd.as_raw_fd(), libc::TCSANOW, &state.0) };
        if result == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
    fn raw_state(&self, state: &State) -> io::Result<State> {
        let mut raw = state.clone();
        // Use the pinned public contract explicitly. Darwin cfmakeraw also
        // changes IGNBRK/IMAXBEL/echo-edit flags; the real PTY regression rejects
        // those extra changes. All edits are on this owned initialized value.
        raw.0.c_iflag &= !(libc::IGNBRK
            | libc::BRKINT
            | libc::PARMRK
            | libc::ISTRIP
            | libc::INLCR
            | libc::IGNCR
            | libc::ICRNL
            | libc::IXON);
        raw.0.c_oflag &= !libc::OPOST;
        raw.0.c_lflag &= !(libc::ECHO | libc::ECHONL | libc::ICANON | libc::ISIG | libc::IEXTEN);
        raw.0.c_cflag = (raw.0.c_cflag & !(libc::CSIZE | libc::PARENB)) | libc::CS8;
        raw.0.c_cc[libc::VMIN] = 1;
        raw.0.c_cc[libc::VTIME] = 0;
        Ok(raw)
    }
    fn password_state(&self, state: &State) -> io::Result<State> {
        let mut password = state.clone();
        password.0.c_lflag &= !libc::ECHO;
        password.0.c_lflag |= libc::ICANON | libc::ISIG;
        password.0.c_iflag |= libc::ICRNL;
        Ok(password)
    }
    fn ending(&self) -> Ending {
        Ending::Lf
    }
    fn password_line(&mut self, maximum: usize) -> Result<Vec<u8>, PasswordError> {
        // Called only after the guard successfully installed ICANON. Canonical
        // read(2) never crosses a line, unlike arbitrary Read implementations.
        read_line_buffered::<256>(self, Ending::Lf, maximum)
    }
    fn size(&self) -> io::Result<Size> {
        let mut size = MaybeUninit::<libc::winsize>::uninit();
        // SAFETY: borrowed fd and correctly aligned writable winsize output;
        // TIOCGWINSZ initializes the structure on successful supported Unix calls.
        let result =
            unsafe { libc::ioctl(self.fd.as_raw_fd(), libc::TIOCGWINSZ, size.as_mut_ptr()) };
        if result == -1 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: checked the successful ioctl before reading initialized fields.
        let size = unsafe { size.assume_init() };
        Ok(Size::new(size.ws_col, size.ws_row))
    }
}
impl Read for Native<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        // SAFETY: fd stays open and the mutable slice is writable for its length.
        // No bytes past the returned length are read by Rust.
        let count =
            unsafe { libc::read(self.fd.as_raw_fd(), bytes.as_mut_ptr().cast(), bytes.len()) };
        if count == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(count as usize)
        }
    }
}
