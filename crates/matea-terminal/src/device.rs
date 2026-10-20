//! Borrowed native terminal handles and owned, exclusive restoration.
use crate::{EXCLUSIVE, session::Lease};
use matea_render::Size;
use std::{
    fmt,
    io::{self, Read},
    marker::PhantomData,
    rc::Rc,
};

#[cfg(unix)]
#[path = "device/unix.rs"]
#[allow(unsafe_code)]
mod native;
#[cfg(windows)]
#[path = "device/windows.rs"]
#[allow(unsafe_code)]
mod native;
#[cfg(not(any(unix, windows)))]
#[path = "device/unsupported.rs"]
mod native;

/// A borrowed native handle. The caller retains its ownership and close policy.
#[cfg(unix)]
pub trait TerminalFile: std::os::fd::AsFd {}
#[cfg(unix)]
impl<T: std::os::fd::AsFd + ?Sized> TerminalFile for T {}
#[cfg(windows)]
pub trait TerminalFile: std::os::windows::io::AsHandle {}
#[cfg(windows)]
impl<T: std::os::windows::io::AsHandle + ?Sized> TerminalFile for T {}
/// Native descriptor access is unavailable on these targets; operations return
/// Unsupported. This does not assert source Plan 9/other-OS feature parity.
#[cfg(not(any(unix, windows)))]
pub trait TerminalFile {}
#[cfg(not(any(unix, windows)))]
impl<T: ?Sized> TerminalFile for T {}

/// Query this exact handle, without taking an exclusive lease or changing it.
pub fn is_terminal(file: &(impl TerminalFile + ?Sized)) -> bool {
    native::Native::new(file).state().is_ok()
}

/// Read-only native state query, allowed while a Matea session owns the device.
pub fn terminal_state(file: &(impl TerminalFile + ?Sized)) -> io::Result<TerminalState> {
    native::Native::new(file).state().map(TerminalState)
}

/// Visible terminal dimensions. Windows requires an output-console handle;
/// an input handle or a pipe can return an OS error rather than a guessed size.
pub fn terminal_size(file: &(impl TerminalFile + ?Sized)) -> io::Result<Size> {
    native::Native::new(file).size()
}

/// Opaque platform state. Kernel-maintained transient flags may change when
/// applying it; equality with a subsequent query is deliberately not promised.
#[derive(Clone)]
pub struct TerminalState(native::State);
impl fmt::Debug for TerminalState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TerminalState(..)")
    }
}

/// Explicit Windows console-state customization. The ordinary portable state
/// remains opaque; console flags have input/output-handle-specific meanings.
/// Applying a snapshot still uses TerminalDevice's checked OS call and owned
/// original-state restoration. Unknown bits are retained for future OS flags.
#[cfg(windows)]
pub mod windows {
    use super::{TerminalState, native};
    use windows_sys::Win32::System::Console::*;

    bitflags::bitflags! {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        pub struct ConsoleMode: u32 {
            const INPUT_PROCESSED = ENABLE_PROCESSED_INPUT;
            const INPUT_LINE = ENABLE_LINE_INPUT;
            const INPUT_ECHO = ENABLE_ECHO_INPUT;
            const INPUT_WINDOW = ENABLE_WINDOW_INPUT;
            const INPUT_MOUSE = ENABLE_MOUSE_INPUT;
            const INPUT_INSERT = ENABLE_INSERT_MODE;
            const INPUT_QUICK_EDIT = ENABLE_QUICK_EDIT_MODE;
            const INPUT_EXTENDED_FLAGS = ENABLE_EXTENDED_FLAGS;
            const INPUT_AUTO_POSITION = ENABLE_AUTO_POSITION;
            const INPUT_VIRTUAL_TERMINAL = ENABLE_VIRTUAL_TERMINAL_INPUT;
            const OUTPUT_PROCESSED = ENABLE_PROCESSED_OUTPUT;
            const OUTPUT_WRAP_AT_EOL = ENABLE_WRAP_AT_EOL_OUTPUT;
            const OUTPUT_VIRTUAL_TERMINAL = ENABLE_VIRTUAL_TERMINAL_PROCESSING;
            const OUTPUT_DISABLE_NEWLINE_AUTO_RETURN = DISABLE_NEWLINE_AUTO_RETURN;
            const OUTPUT_LVB_GRID = ENABLE_LVB_GRID_WORLDWIDE;
        }
    }

    /// Read a captured or explicitly constructed snapshot without native I/O.
    pub const fn mode(state: &TerminalState) -> ConsoleMode {
        ConsoleMode::from_bits_retain(state.0.mode())
    }

    /// Construct a mode value without touching a console. The OS validates its
    /// flags/handle role when a device applies it; this is not a validity proof.
    pub const fn snapshot(mode: ConsoleMode) -> TerminalState {
        TerminalState(native::State::from_mode(mode.bits()))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn snapshots_retain_every_low_word_and_each_high_bit_without_io() {
            let cases = (0..=u16::MAX)
                .map(u32::from)
                .chain((0..32).flat_map(|shift| [1u32 << shift, !(1u32 << shift)]))
                .chain([u32::MAX]);
            for bits in cases {
                let value = ConsoleMode::from_bits_retain(bits);
                assert_eq!(mode(&snapshot(value)).bits(), bits);
                assert_eq!(
                    mode(&snapshot(value)).symmetric_difference(value),
                    ConsoleMode::empty()
                );
            }
            let unknown = ConsoleMode::from_bits_retain(1 << 31);
            let mut edited = unknown | ConsoleMode::INPUT_MOUSE;
            assert_eq!(edited.bits(), (1 << 31) | 16);
            edited.set(ConsoleMode::INPUT_ECHO, true);
            edited.remove(ConsoleMode::INPUT_MOUSE);
            assert_eq!(edited.bits(), (1 << 31) | 4);
        }
    }
}

/// UI-local, exclusively owned mode restoration over a borrowed native handle.
/// The device and all native cleanup are dropped before the shared lease.
///
/// ```compile_fail
/// fn require_send<T: Send>() {}
/// require_send::<matea_terminal::device::TerminalDevice<'static>>();
/// ```
///
/// ```compile_fail
/// use matea_terminal::device::TerminalDevice;
/// let file = std::fs::File::open("terminal")?;
/// let mut terminal = TerminalDevice::open(&file)?;
/// drop(file); // The native handle must outlive all restoration.
/// terminal.restore()?;
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct TerminalDevice<'fd> {
    inner: Guard<native::Native<'fd>>,
    _lease: Lease<'static>,
    _ui_thread: PhantomData<Rc<()>>,
}
impl<'fd> TerminalDevice<'fd> {
    pub fn open(file: &'fd (impl TerminalFile + ?Sized)) -> io::Result<Self> {
        let lease = Lease::acquire(&EXCLUSIVE)?;
        let inner = Guard::new(native::Native::new(file))?;
        Ok(Self {
            inner,
            _lease: lease,
            _ui_thread: PhantomData,
        })
    }
    /// Capture the original state once and immediately enter raw mode. Unlike
    /// open followed by make_raw, this fused constructor needs no second query.
    /// A failed transition still owns cleanup until the lease is released.
    pub fn open_raw(file: &'fd (impl TerminalFile + ?Sized)) -> io::Result<Self> {
        let lease = Lease::acquire(&EXCLUSIVE)?;
        let inner = Guard::new_raw(native::Native::new(file))?;
        Ok(Self {
            inner,
            _lease: lease,
            _ui_thread: PhantomData,
        })
    }
    pub fn state(&self) -> io::Result<TerminalState> {
        self.inner.io.state().map(TerminalState)
    }
    pub fn set_state(&mut self, state: &TerminalState) -> io::Result<()> {
        self.inner.set_state(&state.0)
    }
    /// Return the previous state, while keeping original-state restoration owned.
    pub fn make_raw(&mut self) -> io::Result<TerminalState> {
        self.inner.make_raw().map(TerminalState)
    }
    pub fn size(&self) -> io::Result<Size> {
        self.inner.io.size()
    }
    /// Restore the original state now. Failure remains retryable, including Drop.
    pub fn restore(&mut self) -> io::Result<()> {
        self.inner.restore()
    }
    /// Read one non-echo line without consuming following input. The limit counts
    /// bytes after byte-backspace editing, not characters. Partial/error values
    /// remain bytes; no UTF-8 replacement or renderer sanitization is performed.
    /// This is a blocking native-device operation; use outside the running TEA
    /// session (whose exclusive lease deliberately prevents concurrent input).
    pub fn read_password(&mut self, maximum_bytes: usize) -> Result<Vec<u8>, PasswordError> {
        self.inner.read_password(maximum_bytes)
    }
}
impl fmt::Debug for TerminalDevice<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerminalDevice").finish_non_exhaustive()
    }
}

#[derive(Debug)]
enum Failure {
    Io(io::Error),
    Limit(usize),
}
/// Reading/limit failures retain partial bytes. A simultaneous mode-restoration
/// failure is accessible separately. Debug and Display never include input bytes.
pub struct PasswordError {
    partial: Vec<u8>,
    operation: Option<Failure>,
    restoration: Option<io::Error>,
}
impl PasswordError {
    fn io(error: io::Error) -> Self {
        Self {
            partial: Vec::new(),
            operation: Some(Failure::Io(error)),
            restoration: None,
        }
    }
    pub fn partial(&self) -> &[u8] {
        &self.partial
    }
    pub fn into_partial(self) -> Vec<u8> {
        self.partial
    }
    pub fn io_error(&self) -> Option<&io::Error> {
        match &self.operation {
            Some(Failure::Io(error)) => Some(error),
            _ => None,
        }
    }
    pub fn maximum_bytes(&self) -> Option<usize> {
        match self.operation {
            Some(Failure::Limit(limit)) => Some(limit),
            _ => None,
        }
    }
    pub fn restoration_error(&self) -> Option<&io::Error> {
        self.restoration.as_ref()
    }
}
impl fmt::Debug for PasswordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PasswordError")
            .field("operation", &self.operation)
            .field("restoration", &self.restoration)
            .finish_non_exhaustive()
    }
}
impl fmt::Display for PasswordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.operation {
            Some(Failure::Io(error)) => write!(f, "terminal input failed: {error}")?,
            Some(Failure::Limit(limit)) => {
                write!(f, "terminal input exceeds the {limit}-byte limit")?
            }
            None => f.write_str("terminal input mode restoration failed")?,
        }
        if let Some(error) = &self.restoration {
            write!(f, "; terminal restoration: {error}")?;
        }
        Ok(())
    }
}
impl std::error::Error for PasswordError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.io_error()
            .or(self.restoration.as_ref())
            .map(|e| e as _)
    }
}

#[derive(Clone, Copy)]
enum Ending {
    #[cfg_attr(windows, allow(dead_code))]
    Lf,
    #[cfg_attr(not(windows), allow(dead_code))]
    Cr,
}
trait DeviceIo: Read {
    type State: Clone;
    fn state(&self) -> io::Result<Self::State>;
    fn set_state(&mut self, state: &Self::State) -> io::Result<()>;
    fn raw_state(&self, state: &Self::State) -> io::Result<Self::State>;
    fn password_state(&self, state: &Self::State) -> io::Result<Self::State>;
    fn ending(&self) -> Ending;
    fn size(&self) -> io::Result<Size>;
    fn password_line(&mut self, maximum: usize) -> Result<Vec<u8>, PasswordError>
    where
        Self: Sized,
    {
        let ending = self.ending();
        read_line(self, ending, maximum)
    }
}
struct Guard<D: DeviceIo> {
    io: D,
    original: D::State,
    changed: bool,
}
impl<D: DeviceIo> Guard<D> {
    fn new(io: D) -> io::Result<Self> {
        let original = io.state()?;
        Ok(Self {
            io,
            original,
            changed: false,
        })
    }
    fn new_raw(io: D) -> io::Result<Self> {
        let mut owned = Self::new(io)?;
        let raw = owned.io.raw_state(&owned.original)?;
        owned.set_state(&raw)?;
        Ok(owned)
    }
    fn set_state(&mut self, state: &D::State) -> io::Result<()> {
        self.changed = true;
        self.io.set_state(state)
    }
    fn make_raw(&mut self) -> io::Result<D::State> {
        let old = self.io.state()?;
        let raw = self.io.raw_state(&old)?;
        self.set_state(&raw)?;
        Ok(old)
    }
    fn restore(&mut self) -> io::Result<()> {
        if self.changed {
            self.io.set_state(&self.original)?;
            self.changed = false;
        }
        Ok(())
    }
    fn read_password(&mut self, maximum: usize) -> Result<Vec<u8>, PasswordError> {
        let previous = self.io.state().map_err(PasswordError::io)?;
        let password = self
            .io
            .password_state(&previous)
            .map_err(PasswordError::io)?;
        let was_changed = self.changed;
        self.changed = true;
        let mut mode = ModeGuard {
            io: &mut self.io,
            previous,
            active: true,
        };
        let operation = match mode.io.set_state(&password) {
            Ok(()) => mode.io.password_line(maximum),
            Err(error) => Err(PasswordError::io(error)),
        };
        let restoration = mode.restore().err();
        if restoration.is_none() {
            self.changed = was_changed;
        }
        match (operation, restoration) {
            (Ok(bytes), None) => Ok(bytes),
            (Ok(bytes), Some(error)) => Err(PasswordError {
                partial: bytes,
                operation: None,
                restoration: Some(error),
            }),
            (Err(mut error), restoration) => {
                error.restoration = restoration;
                Err(error)
            }
        }
    }
}
impl<D: DeviceIo> Drop for Guard<D> {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
struct ModeGuard<'a, D: DeviceIo> {
    io: &'a mut D,
    previous: D::State,
    active: bool,
}
impl<D: DeviceIo> ModeGuard<'_, D> {
    fn restore(&mut self) -> io::Result<()> {
        if self.active {
            self.io.set_state(&self.previous)?;
            self.active = false;
        }
        Ok(())
    }
}
impl<D: DeviceIo> Drop for ModeGuard<'_, D> {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

fn read_line(
    reader: &mut impl Read,
    ending: Ending,
    maximum: usize,
) -> Result<Vec<u8>, PasswordError> {
    read_line_buffered::<1>(reader, ending, maximum)
}

// N > 1 is only valid for canonical Unix reads: the kernel returns at most one
// line, so the completed line never consumes a following line. Other readers,
// including the Windows console text decoder, keep the one-byte path.
fn read_line_buffered<const N: usize>(
    reader: &mut impl Read,
    ending: Ending,
    maximum: usize,
) -> Result<Vec<u8>, PasswordError> {
    let mut bytes = Vec::new();
    let mut input = [0; N];
    loop {
        // At most one byte beyond the remaining limit: a limit error consumes
        // exactly the triggering byte even when backspaces reduce the prefix.
        let length = N.min(maximum.saturating_sub(bytes.len()).saturating_add(1));
        let count = match reader.read(&mut input[..length]) {
            Ok(0) if !bytes.is_empty() => return Ok(bytes),
            Ok(0) => return Err(PasswordError::io(io::ErrorKind::UnexpectedEof.into())),
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => {
                return Err(PasswordError {
                    partial: bytes,
                    operation: Some(Failure::Io(error)),
                    restoration: None,
                });
            }
        };
        for (index, &byte) in input[..count].iter().enumerate() {
            let failure = match byte {
                b'\x08' => {
                    bytes.pop();
                    continue;
                }
                b'\n' if matches!(ending, Ending::Lf) => return Ok(bytes),
                b'\r' if matches!(ending, Ending::Cr) => return Ok(bytes),
                b'\r' | b'\n' => continue,
                _ if bytes.len() >= maximum => Failure::Limit(maximum),
                value => {
                    let remaining = (count - index).min(maximum - bytes.len());
                    if bytes.len() == bytes.capacity() && bytes.try_reserve(remaining).is_err() {
                        Failure::Io(io::ErrorKind::OutOfMemory.into())
                    } else {
                        bytes.push(value);
                        continue;
                    }
                }
            };
            return Err(PasswordError {
                partial: bytes,
                operation: Some(failure),
                restoration: None,
            });
        }
    }
}

#[cfg(any(windows, test))]
#[derive(Default)]
struct Utf16Decoder {
    pending: Option<u16>,
}
#[cfg(any(windows, test))]
impl Utf16Decoder {
    fn next(
        &mut self,
        mut next: impl FnMut() -> io::Result<Option<u16>>,
    ) -> io::Result<Option<char>> {
        let lead = match self.pending.take() {
            Some(unit) => unit,
            None => match next()? {
                Some(unit) => unit,
                None => return Ok(None),
            },
        };
        let value = match lead {
            0xd800..=0xdbff => match next() {
                Ok(Some(low @ 0xdc00..=0xdfff)) => char::from_u32(
                    0x10000 + ((u32::from(lead) - 0xd800) << 10) + u32::from(low) - 0xdc00,
                )
                .expect("bounded surrogate pair"),
                Ok(unit) => {
                    self.pending = unit;
                    char::REPLACEMENT_CHARACTER
                }
                Err(error) => {
                    self.pending = Some(lead);
                    return Err(error);
                }
            },
            0xdc00..=0xdfff => char::REPLACEMENT_CHARACTER,
            unit => char::from_u32(u32::from(unit)).expect("non-surrogate u16"),
        };
        Ok(Some(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::{Cell, RefCell},
        collections::VecDeque,
        io::Cursor,
    };

    #[test]
    fn line_bytes_endings_eof_limits_and_no_overread() {
        let fixtures: &[(&[u8], Ending, &[u8])] = &[
            (b"tea\nnext", Ending::Lf, b"tea"),
            (b"a\rb\n", Ending::Lf, b"ab"),
            (b"a\nb\rnext", Ending::Cr, b"ab"),
            (b"\x08ab\x08c\n", Ending::Lf, b"ac"),
            (
                b"\xff\x00\xe8\x8c\xb6\x08\n",
                Ending::Lf,
                b"\xff\x00\xe8\x8c",
            ),
            (b"tea", Ending::Lf, b"tea"),
            (b"\n", Ending::Lf, b""),
        ];
        for &(input, ending, expected) in fixtures {
            let mut cursor = Cursor::new(input);
            assert_eq!(read_line(&mut cursor, ending, 8).unwrap(), expected);
            if input.ends_with(b"next") {
                assert_eq!(&input[cursor.position() as usize..], b"next");
            }
        }
        let error = read_line(&mut Cursor::new([]), Ending::Lf, 0).unwrap_err();
        assert_eq!(
            error.io_error().unwrap().kind(),
            io::ErrorKind::UnexpectedEof
        );
        assert!(
            read_line(&mut Cursor::new(b"\n"), Ending::Lf, 0)
                .unwrap()
                .is_empty()
        );
        let error = read_line(&mut Cursor::new(b"tea\n"), Ending::Lf, 2).unwrap_err();
        assert_eq!(error.maximum_bytes(), Some(2));
        assert_eq!(error.partial(), b"te");
        assert_eq!(
            read_line(&mut Cursor::new(b"ab\x08c\n"), Ending::Lf, 2).unwrap(),
            b"ac"
        );
    }

    #[test]
    fn canonical_chunks_preserve_limits_backspaces_and_the_following_line() {
        struct Canonical {
            data: Cursor<Vec<u8>>,
            calls: usize,
        }
        impl Read for Canonical {
            fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
                self.calls += 1;
                let remaining = &self.data.get_ref()[self.data.position() as usize..];
                let line = remaining
                    .iter()
                    .position(|&b| b == b'\n')
                    .map_or(remaining.len(), |i| i + 1);
                let length = line.min(output.len());
                self.data.read(&mut output[..length])
            }
        }
        for length in [0, 1, 7, 15, 31, 255, 256, 257, 511, 512] {
            let mut input = vec![b'x'; length];
            input.extend_from_slice(b"\x08!\nNEXT\n");
            for maximum in [0, 1, 2, 7, 16, 255, 256, 512, usize::MAX] {
                let mut reader = Canonical {
                    data: Cursor::new(input.clone()),
                    calls: 0,
                };
                let result = read_line_buffered::<256>(&mut reader, Ending::Lf, maximum);
                let position = reader.data.position() as usize;
                if maximum < length {
                    let error = result.unwrap_err();
                    assert_eq!(error.maximum_bytes(), Some(maximum));
                    assert_eq!(error.partial(), vec![b'x'; maximum]);
                    assert_eq!(position, maximum + 1);
                } else if length == 0 && maximum == 0 {
                    let error = result.unwrap_err();
                    assert_eq!(error.maximum_bytes(), Some(0));
                    assert!(error.partial().is_empty());
                    assert_eq!(position, 2);
                } else {
                    let mut expected = vec![b'x'; length.saturating_sub(1)];
                    expected.push(b'!');
                    assert_eq!(result.unwrap(), expected);
                    assert_eq!(&input[position..], b"NEXT\n");
                    if maximum == usize::MAX {
                        assert_eq!(reader.calls, (length + 3).div_ceil(256));
                    }
                }
            }
        }
    }

    #[test]
    fn input_errors_keep_partial_bytes_without_debugging_them() {
        struct Reader(u8);
        impl Read for Reader {
            fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
                self.0 += 1;
                match self.0 {
                    1 => Err(io::ErrorKind::Interrupted.into()),
                    2 => {
                        out[0] = b'X';
                        Ok(1)
                    }
                    _ => Err(io::ErrorKind::BrokenPipe.into()),
                }
            }
        }
        let error = read_line(&mut Reader(0), Ending::Lf, 20).unwrap_err();
        assert_eq!(error.partial(), b"X");
        assert_eq!(error.io_error().unwrap().kind(), io::ErrorKind::BrokenPipe);
        assert!(!format!("{error:?}").contains('X'));
        assert!(!error.to_string().contains('X'));
    }

    struct Mock {
        state: Rc<RefCell<(u8, Vec<u8>)>>,
        input: Cursor<Vec<u8>>,
        writes: usize,
        fail: VecDeque<usize>,
        panic_read: bool,
        queries: Rc<Cell<usize>>,
        reject_raw: bool,
    }
    impl Read for Mock {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            assert!(!self.panic_read, "mock read panic");
            self.input.read(out)
        }
    }
    impl DeviceIo for Mock {
        type State = u8;
        fn state(&self) -> io::Result<u8> {
            self.queries.set(self.queries.get() + 1);
            Ok(self.state.borrow().0)
        }
        fn set_state(&mut self, value: &u8) -> io::Result<()> {
            self.writes += 1;
            self.state.borrow_mut().1.push(*value);
            if self.fail.front() == Some(&self.writes) {
                self.fail.pop_front();
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            self.state.borrow_mut().0 = *value;
            Ok(())
        }
        fn raw_state(&self, _: &u8) -> io::Result<u8> {
            if self.reject_raw {
                Err(io::ErrorKind::Unsupported.into())
            } else {
                Ok(2)
            }
        }
        fn password_state(&self, _: &u8) -> io::Result<u8> {
            Ok(3)
        }
        fn ending(&self) -> Ending {
            Ending::Lf
        }
        fn size(&self) -> io::Result<Size> {
            Ok(Size::new(132, 42))
        }
    }
    fn mock(fail: &[usize], panic_read: bool) -> Mock {
        Mock {
            state: Rc::new(RefCell::new((1, Vec::new()))),
            input: Cursor::new(b"tea\n".to_vec()),
            writes: 0,
            fail: fail.iter().copied().collect(),
            panic_read,
            queries: Rc::new(Cell::new(0)),
            reject_raw: false,
        }
    }
    #[test]
    fn fresh_raw_captures_once_and_owns_failed_transition_cleanup() {
        let io = mock(&[], false);
        let (state, queries) = (io.state.clone(), io.queries.clone());
        let owned = Guard::new_raw(io).unwrap();
        assert_eq!(queries.get(), 1);
        assert_eq!(state.borrow().0, 2);
        drop(owned);
        assert_eq!(*state.borrow(), (1, vec![2, 1]));

        let io = mock(&[1], false);
        let state = io.state.clone();
        assert!(
            matches!(Guard::new_raw(io), Err(e) if e.kind() == io::ErrorKind::PermissionDenied)
        );
        assert_eq!(*state.borrow(), (1, vec![2, 1]));

        let mut io = mock(&[], false);
        io.reject_raw = true;
        let state = io.state.clone();
        assert!(matches!(Guard::new_raw(io), Err(e) if e.kind() == io::ErrorKind::Unsupported));
        assert_eq!(*state.borrow(), (1, vec![]));
    }
    #[test]
    fn restore_is_retryable_and_password_restores_the_current_mode() {
        let io = mock(&[2], false);
        let state = io.state.clone();
        let mut guard = Guard::new(io).unwrap();
        assert_eq!(guard.make_raw().unwrap(), 1);
        assert!(guard.restore().is_err());
        assert!(guard.changed);
        guard.restore().unwrap();
        assert!(!guard.changed);
        assert_eq!(guard.make_raw().unwrap(), 1);
        assert_eq!(guard.read_password(8).unwrap(), b"tea");
        assert_eq!(state.borrow().0, 2);
        drop(guard);
        assert_eq!(state.borrow().0, 1);
    }
    #[test]
    fn password_errors_and_unwind_keep_restoration_owned() {
        let io = mock(&[2, 3], false);
        let state = io.state.clone();
        let mut guard = Guard::new(io).unwrap();
        let error = guard.read_password(8).unwrap_err();
        assert_eq!(error.partial(), b"tea");
        assert!(error.restoration_error().is_some());
        assert!(guard.changed);
        drop(guard);
        assert_eq!(state.borrow().0, 1);
        let io = mock(&[], true);
        let state = io.state.clone();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut guard = Guard::new(io).unwrap();
            let _ = guard.read_password(8);
        }));
        assert!(result.is_err());
        assert_eq!(state.borrow().0, 1);
    }
    #[test]
    fn utf16_stream_matches_standard_replacement_without_losing_lookahead() {
        let step = if cfg!(miri) { 257 } else { 1 };
        for unit in (0..=u16::MAX).step_by(step) {
            let input = [unit, b'\r' as u16, b'X' as u16];
            let mut source = input.into_iter();
            let mut decoder = Utf16Decoder::default();
            let mut actual = Vec::new();
            while let Some(c) = decoder.next(|| Ok(source.next())).unwrap() {
                actual.push(c);
            }
            assert_eq!(
                actual,
                char::decode_utf16(input)
                    .map(|v| v.unwrap_or(char::REPLACEMENT_CHARACTER))
                    .collect::<Vec<_>>()
            );
        }
        for input in [[0xd800, 0xdc00], [0xdbff, 0xdfff], [0xd83c, 0xdf75]] {
            let mut decoder = Utf16Decoder::default();
            let mut index = 0;
            assert_eq!(
                decoder
                    .next(|| {
                        let v = input.get(index).copied();
                        index += 1;
                        Ok(v)
                    })
                    .unwrap(),
                char::decode_utf16(input).next().unwrap().ok()
            );
        }
        let mut decoder = Utf16Decoder::default();
        let mut calls = 0;
        assert_eq!(
            decoder
                .next(|| {
                    calls += 1;
                    if calls == 1 {
                        Ok(Some(0xd83c))
                    } else {
                        Err(io::ErrorKind::Interrupted.into())
                    }
                })
                .unwrap_err()
                .kind(),
            io::ErrorKind::Interrupted
        );
        assert_eq!(decoder.next(|| Ok(Some(0xdf75))).unwrap(), Some('🍵'));
    }
}
