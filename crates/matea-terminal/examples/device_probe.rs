//! Machine-readable native-device fixture; never operates on a user's console
//! during automated tests. The driver supplies an isolated PTY/new console.
use matea_terminal::device::{
    TerminalDevice, TerminalFile, is_terminal, terminal_size, terminal_state,
};
use std::io::{self, Read, Write};

fn emit(text: impl std::fmt::Display) -> io::Result<()> {
    let mut output = io::stdout().lock();
    writeln!(output, "{text}")?;
    output.flush()
}
fn barrier() -> io::Result<()> {
    io::stdin().read_exact(&mut [0])
}
#[cfg(windows)]
fn check_console_flag(
    device: &mut TerminalDevice<'_>,
    flag: matea_terminal::device::windows::ConsoleMode,
) -> io::Result<()> {
    use matea_terminal::device::windows::{mode, snapshot};
    let initial = device.state()?;
    let toggled = mode(&initial) ^ flag;
    device.set_state(&snapshot(toggled))?;
    if mode(&device.state()?) != toggled {
        return Err(io::Error::other("custom console mode was not applied"));
    }
    device.set_state(&initial)?;
    if mode(&device.state()?) != mode(&initial) {
        return Err(io::Error::other("custom console mode was not restored"));
    }
    Ok(())
}
fn probe(
    input: &(impl TerminalFile + ?Sized),
    output: &(impl TerminalFile + ?Sized),
) -> io::Result<()> {
    let operation = std::env::args().nth(1).unwrap_or_else(|| "state".into());
    let maximum: usize = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "4096".into())
        .parse()
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    fn rejected(file: &(impl TerminalFile + ?Sized)) -> io::Result<()> {
        if is_terminal(file)
            || terminal_state(file).is_ok()
            || terminal_size(file).is_ok()
            || TerminalDevice::open(file).is_ok()
        {
            return Err(io::Error::other("non-terminal accepted as a terminal"));
        }
        Ok(())
    }
    rejected(&std::fs::File::open(std::env::current_exe()?)?)?;
    rejected(&io::stdout())?; // The fixture driver captures stdout in a pipe.
    #[cfg(windows)]
    {
        use matea_terminal::device::windows::{ConsoleMode, mode};
        let mut console = TerminalDevice::open(output)?;
        let initial = mode(&console.state()?);
        if console.make_raw().is_ok()
            || console.read_password(8).is_ok()
            || mode(&console.state()?) != initial
        {
            return Err(io::Error::other(
                "output handle accepted input-only operations",
            ));
        }
        check_console_flag(&mut console, ConsoleMode::OUTPUT_WRAP_AT_EOL)?;
        console.restore()?;
    }
    let mut device = TerminalDevice::open(input)?;
    let size = terminal_size(output)?;
    let nested = TerminalDevice::open(input).unwrap_err();
    if nested.kind() != io::ErrorKind::WouldBlock
        || !is_terminal(input)
        || terminal_state(input).is_err()
    {
        return Err(io::Error::other(
            "exclusive/read-only device contract failed",
        ));
    }
    #[cfg(windows)]
    check_console_flag(
        &mut device,
        matea_terminal::device::windows::ConsoleMode::INPUT_ECHO,
    )?;
    if operation == "password" {
        emit(format_args!("ready\t{}\t{}", size.width, size.height))?;
        let (kind, bytes, limit, restoration) = match device.read_password(maximum) {
            Ok(bytes) => ("ok".to_owned(), bytes, None, false),
            Err(error) => {
                let kind = error
                    .io_error()
                    .map_or("limit".to_owned(), |e| format!("{:?}", e.kind()));
                let limit = error.maximum_bytes();
                let restoration = error.restoration_error().is_some();
                (kind, error.into_partial(), limit, restoration)
            }
        };
        device.restore()?;
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        emit(format_args!(
            "result\t{kind}\t{hex}\t{}\t{restoration}",
            limit.map_or_else(|| "none".to_owned(), |n| n.to_string())
        ))?;
    } else if operation == "drop" || operation == "unwind" {
        drop(device);
        let device = TerminalDevice::open_raw(input)?;
        emit(format_args!("raw\t{}\t{}", size.width, size.height))?;
        barrier()?;
        if operation == "unwind" {
            std::panic::set_hook(Box::new(|_| {}));
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _owned = device;
                panic!("disposable terminal unwind fixture");
            }));
            if result.is_ok() {
                return Err(io::Error::other("unwind fixture did not unwind"));
            }
        } else {
            drop(device);
        }
        // The same native device can be leased again after cleanup.
        drop(TerminalDevice::open(input)?);
        emit("restored")?;
    } else if operation == "state" {
        let original = device.state()?;
        let prior = device.make_raw()?;
        emit(format_args!("raw\t{}\t{}", size.width, size.height))?;
        barrier()?;
        device.set_state(&prior)?;
        emit("set")?;
        barrier()?;
        device.set_state(&original)?;
        device.restore()?;
        device.restore()?;
        emit("restored")?;
    } else {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    Ok(())
}
fn main() -> io::Result<()> {
    #[cfg(unix)]
    {
        probe(&io::stdin(), &io::stdin())
    }
    #[cfg(windows)]
    {
        let input = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("CONIN$")?;
        let output = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("CONOUT$")?;
        probe(&input, &output)
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(io::ErrorKind::Unsupported.into())
    }
}
